//! A global allocator for test binaries that looks for a test secret in
//! memory the program gives back or keeps (`docs/SECRET-LEAKS.md`). A
//! test binary declares it as its allocator; a test arms its own thread
//! with the forms of the secret it planted ([`arm`]), runs the flow, and
//! reads what was found ([`Armed::finish`], [`Armed::scan_live`]).
//!
//! Every call goes to the system allocator. The extra work is done on the
//! thread that makes the call, and only on an armed thread (or one armed
//! from it, [`Armed::handle`]), so tests running side by side in one
//! process see only their own blocks. A disarmed thread pays one
//! thread-local read per call. On an armed thread new memory that is not
//! zeroed is filled with a fixed byte, so a block is reported for what its
//! owner wrote and not for what an earlier owner left in it.
//!
//! ```
//! #[global_allocator]
//! static ALLOC: faraday_leak_scan::LeakScan = faraday_leak_scan::LeakScan;
//!
//! let secret = faraday_leak_scan::Form::new("secret", b"a long unusual test secret".to_vec());
//! let armed = faraday_leak_scan::arm(&[secret]);
//! let copy = std::hint::black_box(b"a long unusual test secret".to_vec());
//! drop(copy);
//! assert!(!armed.finish().freed.is_empty());
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::backtrace::Backtrace;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// The most hits kept; the rest are counted ([`Report::dropped`]).
const MAX_HITS: usize = 300;

/// One form a secret can be held in: a name and the bytes searched for.
#[derive(Clone)]
pub struct Form {
    name: String,
    bytes: Vec<u8>,
}

impl Form {
    /// A form called `name` that reads as `bytes`. An empty form is
    /// never searched for.
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Form {
        Form {
            name: name.into(),
            bytes,
        }
    }

    /// The form's name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// When a block with a form in it was seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// Freed, or moved by a reallocation, with the form in it.
    Freed,
    /// Still live when [`Armed::scan_live`] looked.
    Live,
}

/// A block that held a form.
pub struct Hit {
    /// The form's name.
    pub form: String,
    /// Where in the block the form begins.
    pub offset: usize,
    /// The block's size.
    pub size: usize,
    /// Freed or live.
    pub when: When,
    trace: Option<Backtrace>,
}

impl Hit {
    /// The backtrace of the call that freed the block, one frame a line
    /// (empty for a live block). Costs symbol resolution: call it only
    /// for a hit being reported.
    pub fn backtrace(&self) -> String {
        self.trace
            .as_ref()
            .map(|b| b.to_string())
            .unwrap_or_default()
    }
}

impl fmt::Debug for Hit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: form {:?} at offset {} of a block of {} bytes",
            self.when, self.form, self.offset, self.size
        )
    }
}

/// What an armed thread's run found.
#[derive(Debug, Default)]
pub struct Report {
    /// Blocks freed (or moved) holding a form.
    pub freed: Vec<Hit>,
    /// Hits past the most kept, not recorded.
    pub dropped: usize,
}

struct Session {
    forms: Vec<Form>,
    shortest: usize,
    open: AtomicBool,
    /// The live blocks of every thread armed with this session: address
    /// and size. Its own memory is taken while the thread is marked busy,
    /// so it never lists itself.
    live: Mutex<HashMap<usize, usize>>,
    hits: Mutex<Vec<Hit>>,
    dropped: AtomicUsize,
}

/// The sessions still open: a block one of them lists may be freed by a
/// thread that is not armed with it (a thread of a pool, one that was
/// never armed), and that free must take it off the list, or the list
/// holds an address that is reused and scanned as a block the test keeps.
static OPEN: Mutex<Vec<Arc<Session>>> = Mutex::new(Vec::new());

/// How many sessions [`OPEN`] holds: the one read a free makes when no
/// test is running.
static OPEN_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Takes the block at `ptr`, given back by a thread this allocator does
/// not scan for (not armed, or inside its own bookkeeping), off every
/// open session's list of live blocks.
fn forget(ptr: *mut u8) {
    if OPEN_COUNT.load(Ordering::Relaxed) == 0 {
        return;
    }
    // A thread already in the bookkeeping may hold a list's lock.
    if BUSY.try_with(|b| b.replace(true)).unwrap_or(true) {
        return;
    }
    {
        let open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
        for s in open.iter() {
            s.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&(ptr as usize));
        }
    }
    let _ = BUSY.try_with(|b| b.set(false));
}

thread_local! {
    /// Whether this thread is armed: the one read a disarmed call makes.
    static ON: Cell<bool> = const { Cell::new(false) };
    /// Whether this thread is inside the allocator's own bookkeeping: its
    /// calls go straight to the system allocator and are not listed.
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static SESSION: RefCell<Option<Arc<Session>>> = const { RefCell::new(None) };
}

/// Runs `f` with the thread's session, if it is armed and not already in
/// the allocator's bookkeeping. Everything `f` allocates is unlisted.
fn with_session<T>(f: impl FnOnce(&Session) -> T) -> Option<T> {
    if !ON.try_with(Cell::get).unwrap_or(false) {
        return None;
    }
    if BUSY.try_with(|b| b.replace(true)).unwrap_or(true) {
        return None;
    }
    let out = SESSION
        .try_with(|s| {
            let s = s.try_borrow().ok()?;
            let s = s.as_ref()?;
            s.open.load(Ordering::Relaxed).then(|| f(s))
        })
        .ok()
        .flatten();
    let _ = BUSY.try_with(|b| b.set(false));
    out
}

/// Runs `f` marked busy, so what it allocates is not listed or scanned.
fn busy<T>(f: impl FnOnce() -> T) -> T {
    let was = BUSY.try_with(|b| b.replace(true)).unwrap_or(true);
    let out = f();
    if !was {
        let _ = BUSY.try_with(|b| b.set(false));
    }
    out
}

/// The offset of each form's first appearance in `block`.
fn find_forms(s: &Session, block: &[u8]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    if block.len() < s.shortest {
        return found;
    }
    for (i, form) in s.forms.iter().enumerate() {
        let needle = form.bytes.as_slice();
        if needle.is_empty() || block.len() < needle.len() {
            continue;
        }
        let last = block.len() - needle.len();
        let mut at = 0;
        while at <= last {
            // The first byte is found fast; the rest is compared there.
            match block[at..=last].iter().position(|b| *b == needle[0]) {
                None => break,
                Some(p) => {
                    at += p;
                    if block[at..].starts_with(needle) {
                        found.push((i, at));
                        break;
                    }
                    at += 1;
                }
            }
        }
    }
    found
}

fn record(s: &Session, found: &[(usize, usize)], size: usize, when: When, trace: bool) {
    if found.is_empty() {
        return;
    }
    let mut hits = s.hits.lock().unwrap_or_else(|e| e.into_inner());
    for &(i, offset) in found {
        if hits.len() >= MAX_HITS {
            s.dropped.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        hits.push(Hit {
            form: s.forms[i].name.clone(),
            offset,
            size,
            when,
            trace: trace.then(Backtrace::force_capture),
        });
    }
}

/// Reads the `size` bytes at `ptr`, a block this allocator handed out and
/// has not yet given back.
#[allow(unsafe_code)]
fn block<'a>(ptr: *const u8, size: usize) -> &'a [u8] {
    // SAFETY: callers pass a block that is allocated now: one being freed
    // or moved by the call in progress, or one still in the live list,
    // which a free on any armed thread removes before the block is given
    // back. It was allocated as `size` bytes.
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

/// What an armed thread's new, uninitialised memory is filled with: a
/// block reused from the heap would otherwise carry what an earlier owner
/// left in it (the test's own work before it armed, another thread's), and
/// be reported for a copy its owner never made.
const FRESH: u8 = 0xA5;

/// Fills `len` bytes from `ptr`, memory just handed out and not yet seen
/// by the caller.
#[allow(unsafe_code)]
fn fresh(ptr: *mut u8, len: usize) {
    // SAFETY: `ptr` is the start of `len` writable bytes the allocator has
    // just made, and nothing else holds them yet.
    unsafe { ptr.write_bytes(FRESH, len) }
}

fn on_alloc(ptr: *mut u8, size: usize, uninit: bool) {
    if ptr.is_null() {
        return;
    }
    with_session(|s| {
        if uninit {
            fresh(ptr, size);
        }
        s.live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(ptr as usize, size);
    });
}

fn on_free(ptr: *mut u8, size: usize) {
    let seen = with_session(|s| {
        s.live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(ptr as usize));
        let found = find_forms(s, block(ptr, size));
        record(s, &found, size, When::Freed, true);
    });
    if seen.is_none() {
        forget(ptr);
    }
}

/// The allocator: the system's, with the scan of an armed thread's blocks.
pub struct LeakScan;

#[allow(unsafe_code)]
// SAFETY: every call is passed to `System` unchanged; the scan reads
// blocks the caller is giving back, and the bookkeeping allocates only
// while the thread is marked busy, so it never re-enters this allocator.
unsafe impl GlobalAlloc for LeakScan {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc` is `System`'s.
        let p = unsafe { System.alloc(layout) };
        on_alloc(p, layout.size(), true);
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`.
        let p = unsafe { System.alloc_zeroed(layout) };
        on_alloc(p, layout.size(), false);
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        on_free(ptr, layout.size());
        // SAFETY: `ptr` came from `System` through this allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // What the old block holds is looked for before it can move; it is
        // a hit only if it did move.
        let seen = with_session(|s| {
            s.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&(ptr as usize));
            find_forms(s, block(ptr, layout.size()))
        });
        if seen.is_none() {
            forget(ptr);
        }
        let found = seen.unwrap_or_default();
        // SAFETY: `ptr` came from `System` through this allocator.
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        let size = if new.is_null() {
            layout.size()
        } else {
            new_size
        };
        let at = if new.is_null() { ptr } else { new };
        with_session(|s| {
            if new != ptr && !new.is_null() {
                record(s, &found, layout.size(), When::Freed, true);
            }
            if !new.is_null() && new_size > layout.size() {
                fresh(new.wrapping_add(layout.size()), new_size - layout.size());
            }
            s.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(at as usize, size);
        });
        new
    }
}

/// A thread armed with a test's forms. [`Armed::finish`] disarms it;
/// dropping it does too.
pub struct Armed {
    session: Arc<Session>,
}

/// Arms the calling thread with `forms`: from now on a block this thread
/// frees, or moves by a reallocation, is searched for each of them, and
/// the blocks it allocates are listed for [`Armed::scan_live`]. Build what
/// the test must keep (the secrets themselves, the files) before this call
/// and drop it after [`Armed::finish`]: a block allocated before it is
/// searched when freed, but never listed.
pub fn arm(forms: &[Form]) -> Armed {
    let forms: Vec<Form> = forms
        .iter()
        .filter(|f| !f.bytes.is_empty())
        .cloned()
        .collect();
    let shortest = forms
        .iter()
        .map(|f| f.bytes.len())
        .min()
        .unwrap_or(usize::MAX);
    let session = busy(|| {
        Arc::new(Session {
            forms,
            shortest,
            open: AtomicBool::new(true),
            live: Mutex::new(HashMap::new()),
            hits: Mutex::new(Vec::new()),
            dropped: AtomicUsize::new(0),
        })
    });
    busy(|| {
        OPEN.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Arc::clone(&session));
        OPEN_COUNT.fetch_add(1, Ordering::Relaxed);
    });
    let armed = Armed { session };
    armed.handle().arm_this_thread();
    armed
}

impl Armed {
    /// What another thread needs to be armed with this test's forms, for
    /// the worker thread of a job this test starts.
    pub fn handle(&self) -> Handle {
        Handle {
            session: Arc::clone(&self.session),
        }
    }

    /// Searches every block this test's threads have allocated and not
    /// freed, now (after Lock, nothing should hold a form). Call it with
    /// the worker idle: it reads blocks another thread could be writing.
    pub fn scan_live(&self) -> Vec<Hit> {
        busy(|| {
            let live = self.session.live.lock().unwrap_or_else(|e| e.into_inner());
            let mut out = Vec::new();
            for (&addr, &size) in live.iter() {
                for (i, offset) in find_forms(&self.session, block(addr as *const u8, size)) {
                    out.push(Hit {
                        form: self.session.forms[i].name.clone(),
                        offset,
                        size,
                        when: When::Live,
                        trace: None,
                    });
                }
            }
            out
        })
    }

    /// Disarms every thread of this test and gives what freed blocks held.
    pub fn finish(self) -> Report {
        self.disarm()
    }

    fn disarm(&self) -> Report {
        busy(|| {
            self.session.open.store(false, Ordering::SeqCst);
            let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
            let before = open.len();
            open.retain(|s| !Arc::ptr_eq(s, &self.session));
            OPEN_COUNT.fetch_sub(before - open.len(), Ordering::Relaxed);
            drop(open);
            let _ = ON.try_with(|o| o.set(false));
            self.session
                .live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            let freed =
                std::mem::take(&mut *self.session.hits.lock().unwrap_or_else(|e| e.into_inner()));
            Report {
                freed,
                dropped: self.session.dropped.load(Ordering::Relaxed),
            }
        })
    }
}

impl Drop for Armed {
    fn drop(&mut self) {
        if self.session.open.load(Ordering::SeqCst) {
            drop(self.disarm());
        }
    }
}

/// A test's forms, to arm another thread with.
#[derive(Clone)]
pub struct Handle {
    session: Arc<Session>,
}

impl Handle {
    /// Arms the calling thread with the test's forms.
    pub fn arm_this_thread(&self) {
        busy(|| {
            let _ = SESSION.try_with(|s| *s.borrow_mut() = Some(Arc::clone(&self.session)));
            let _ = ON.try_with(|o| o.set(true));
        });
    }
}
