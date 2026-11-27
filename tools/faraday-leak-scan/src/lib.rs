//! A tool or test-support crate: the leak allocator (09 §8; 07 §6). It lists the live blocks of
//! the armed threads and reads freed ones only inside `drop_checked` (07 §6.1).
//!
//! A test binary installs it (`#[global_allocator] static LEAK: Leak = Leak::new();`), names the
//! forms of its planted secret with [`Leak::watch`], arms the threads whose blocks it wants
//! looked at with [`Leak::arm`], and asks [`Leak::live`] after the run. The table of live blocks
//! is fixed in the binary and touches nothing the allocator itself would allocate.

#![expect(
    unsafe_code,
    reason = "a global allocator is an unsafe impl, and a scan reads the blocks it holds (U2, 07 §6.1)"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// How many live blocks of the armed threads the table holds at once.
const SLOTS: usize = 1 << 20;

/// An empty slot, and one whose block was freed.
const EMPTY: usize = 0;
const FREED: usize = usize::MAX;

struct Slot {
    ptr: AtomicUsize,
    size: AtomicUsize,
    owner: AtomicUsize,
}

impl Slot {
    const fn new() -> Slot {
        Slot {
            ptr: AtomicUsize::new(EMPTY),
            size: AtomicUsize::new(0),
            owner: AtomicUsize::new(0),
        }
    }
}

static TABLE: [Slot; SLOTS] = [const { Slot::new() }; SLOTS];
/// Held while the table is read or changed.
static LOCK: AtomicBool = AtomicBool::new(false);
/// The blocks in the table now.
static HELD: AtomicUsize = AtomicUsize::new(0);
/// A block of an armed thread found the table full: `live` then refuses to answer.
static FULL: AtomicBool = AtomicBool::new(false);
/// The next thread's number.
static NEXT: AtomicUsize = AtomicUsize::new(1);
/// The forms watched for: a secret's bytes in each form the test names.
static FORMS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

thread_local! {
    /// This thread's number while it is armed; 0 when it is not.
    static ARMED: Cell<usize> = const { Cell::new(0) };
    /// Set while `drop_checked` drops its value on this thread.
    static CHECKING: Cell<bool> = const { Cell::new(false) };
    /// A watched form was in a block freed while `CHECKING`.
    static FOUND: Cell<bool> = const { Cell::new(false) };
}

fn locked<T>(then: impl FnOnce() -> T) -> T {
    while LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
    let out = then();
    LOCK.store(false, Ordering::Release);
    out
}

fn home(ptr: usize) -> usize {
    (ptr >> 4).wrapping_mul(0x9E37_79B9_7F4A_7C15) % SLOTS
}

fn record(ptr: usize, size: usize, owner: usize) {
    locked(|| {
        let mut at = home(ptr);
        for _ in 0..SLOTS {
            let slot = &TABLE[at];
            let taken = slot.ptr.load(Ordering::Relaxed);
            if taken == EMPTY || taken == FREED {
                slot.size.store(size, Ordering::Relaxed);
                slot.owner.store(owner, Ordering::Relaxed);
                slot.ptr.store(ptr, Ordering::Relaxed);
                HELD.fetch_add(1, Ordering::Relaxed);
                return;
            }
            at = (at + 1) % SLOTS;
        }
        FULL.store(true, Ordering::Relaxed);
    });
}

fn forget(ptr: usize) {
    if HELD.load(Ordering::Relaxed) == 0 {
        return;
    }
    locked(|| {
        let mut at = home(ptr);
        for _ in 0..SLOTS {
            let slot = &TABLE[at];
            match slot.ptr.load(Ordering::Relaxed) {
                EMPTY => return,
                found if found == ptr => {
                    slot.ptr.store(FREED, Ordering::Relaxed);
                    HELD.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
                _ => at = (at + 1) % SLOTS,
            }
        }
    });
}

/// Whether any watched form is in the `size` bytes at `ptr`.
///
/// # Safety
/// The block is allocated, or is being freed by the caller, so the bytes are mapped.
unsafe fn holds(ptr: usize, size: usize) -> bool {
    let Ok(forms) = FORMS.try_lock() else {
        return false;
    };
    // SAFETY: the caller's promise: the block is mapped for `size` bytes. The bytes may be
    // uninitialised; they are only compared.
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
    forms
        .iter()
        .any(|form| !form.is_empty() && bytes.windows(form.len()).any(|at| at == form.as_slice()))
}

/// A global allocator that finds a form of a secret in a live block, and in a block that
/// `drop_checked` frees; it lists live blocks and reads freed ones only there (07 §6.1).
pub struct Leak {
    _private: (),
}

impl Leak {
    #[must_use]
    pub const fn new() -> Leak {
        Leak { _private: () }
    }

    /// Watches for `form`, a secret's bytes in any form the test names.
    pub fn watch(form: &[u8]) {
        let was = ARMED.with(|armed| armed.replace(0));
        if let Ok(mut forms) = FORMS.lock() {
            forms.push(form.to_vec());
        }
        ARMED.with(|armed| armed.set(was));
    }

    /// Lists this thread's blocks from now on, as live ones to look at. A thread the test spawns
    /// arms itself the same way.
    pub fn arm() {
        ARMED.with(|armed| {
            if armed.get() == 0 {
                armed.set(NEXT.fetch_add(1, Ordering::Relaxed));
            }
        });
    }

    /// Stops listing this thread's new blocks; the ones listed stay until they are freed.
    pub fn disarm() {
        ARMED.with(|armed| armed.set(0));
    }

    /// Whether a watched form is in a live block of the armed threads. Reads only blocks listed
    /// since a thread armed; call it when the other armed threads are idle or ended.
    #[must_use]
    pub fn live() -> bool {
        assert!(
            !FULL.load(Ordering::Relaxed),
            "the leak table filled: its answer would not be whole"
        );
        locked(|| {
            TABLE.iter().any(|slot| {
                let ptr = slot.ptr.load(Ordering::Relaxed);
                if ptr == EMPTY || ptr == FREED || slot.owner.load(Ordering::Relaxed) == 0 {
                    return false;
                }
                // SAFETY: a block in the table is allocated, and freeing it waits for the lock.
                unsafe { holds(ptr, slot.size.load(Ordering::Relaxed)) }
            })
        })
    }

    /// Whether a watched form is in a block that dropping `value` freed: it takes the one value
    /// it drops, so it checks that value's blocks and no other.
    #[must_use]
    pub fn drop_checked<T>(value: T) -> bool {
        FOUND.with(|found| found.set(false));
        CHECKING.with(|checking| checking.set(true));
        drop(value);
        CHECKING.with(|checking| checking.set(false));
        FOUND.with(Cell::get)
    }
}

impl Default for Leak {
    fn default() -> Leak {
        Leak::new()
    }
}

// SAFETY: every call is the system allocator's, with the same layout; the table is bookkeeping
// beside it and allocates nothing.
unsafe impl GlobalAlloc for Leak {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's layout, passed on.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() && layout.size() > 0 {
            let owner = ARMED.try_with(Cell::get).unwrap_or(0);
            if owner != 0 {
                record(ptr as usize, layout.size(), owner);
            }
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's layout, passed on.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() && layout.size() > 0 {
            let owner = ARMED.try_with(Cell::get).unwrap_or(0);
            if owner != 0 {
                record(ptr as usize, layout.size(), owner);
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if CHECKING.try_with(Cell::get).unwrap_or(false) && layout.size() > 0 {
            // SAFETY: the block is the caller's, still allocated until the free below.
            if unsafe { holds(ptr as usize, layout.size()) } {
                let _ = FOUND.try_with(|found| found.set(true));
            }
        }
        forget(ptr as usize);
        // SAFETY: the caller's pointer and layout, passed on.
        unsafe { System.dealloc(ptr, layout) };
    }
}
