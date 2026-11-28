//! `Ctx`, `View`, the facts, `Show`, the memo and `Derived` (03 §5; 04 §3, §6, §13).

use core::any::{Any, TypeId};
use core::cell::RefCell;
use core::hash::Hash;
use std::collections::HashMap;
use std::rc::Rc;

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_port::bytes::EntropySource;
use faraday_port::camera::CameraInfo;
use faraday_port::event::{Abilities, Machine};
use faraday_port::storage::{BootMedium, StickId, StickInfo};
use faraday_port::system::{Computer, Concern, DevImage, Disk, Installed};
use faraday_port::values::{Display, Medium};
use faraday_strings::Strings;
use faraday_text::Text;

use crate::effects::{JobKind, Note};
use crate::message::Public;
use crate::shared::{Settings, session, vaults};

/// What `refuses`, `update` and `present` read. Shared borrows only; no `Compute` and no
/// wording.
pub struct View<'a> {
    /// The session owner's state.
    pub session: &'a dyn session::Read,
    /// The vault composite's state.
    pub vaults: &'a dyn vaults::Read,
    /// The settings owner's state.
    pub settings: &'a Settings,
    /// Kept by the runtime from port facts.
    pub facts: &'a Facts,
    /// Fixed for one build of the scope (§11).
    pub mode: Mode,
    /// The adapter's time and the person's offset.
    pub clock: Clock,
    /// Jobs on the pool now, for busy lines.
    pub running: &'a [Running],
    /// How this build of the scope began (05 §7.5).
    pub started: Started,
}

/// How this build of the scope began.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Started {
    Power,
    Lock,
    /// In process after a failed keep (05 §7.6).
    Rebuilt,
}

/// What `update` reads: the view, and the token for costly work.
pub struct Ctx<'a> {
    pub view: View<'a>,
    /// `faraday-crypto`'s; minted by the runtime (R2).
    pub compute: &'a Compute,
}

pub struct Facts {
    pub display: Display,
    pub medium: Medium,
    pub abilities: Abilities,
    pub machine: Machine,
    pub root_read_only: Option<bool>,
    /// Attached, listed, boot medium, problems.
    pub sticks: Sticks,
    pub cameras: Vec<CameraInfo>,
    /// `HostFact::Downloads`, latest.
    pub downloads: Option<Downloads>,
    pub entropy_source: Option<EntropySource>,
    pub selftest: SelfTest,
    /// The runtime's clean marker (05, R14).
    pub clean: bool,
    /// The Starter's system facts, latest of each.
    pub system: SystemFacts,
}

/// The sticks the runtime knows of (`SticksFact`).
pub struct Sticks {
    pub attached: bool,
    pub listed: Vec<StickInfo>,
    pub boot_medium: Option<BootMedium>,
    /// A stick the backup flow cannot use, and whether it is read-only.
    pub problems: Vec<(StickId, bool)>,
}

/// `HostFact::Downloads`: the folder and its files with their sizes.
pub struct Downloads {
    pub dir: Text,
    pub files: Vec<(Text, u64)>,
}

/// The self-test's result.
pub enum SelfTest {
    Passed,
    /// The name of the check that failed.
    Failed(&'static str),
    NotRun,
}

/// The latest of each `port::SystemFact` but `WriteProgress`.
pub struct SystemFacts {
    pub computer: Option<Computer>,
    pub disks: Vec<Disk>,
    pub no_writer: bool,
    pub installed: Option<Installed>,
    pub dev_image: Option<DevImage>,
    pub images_wanted: Vec<Text>,
    pub concerns: Vec<Concern>,
    pub other_encryption: Option<Text>,
    pub boot_entry: Option<bool>,
}

/// Which fact of `View::facts` changed (`Notice::Facts`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactsChanged {
    Sticks,
    Display,
    Cameras,
    Time,
    Downloads,
    System,
}

pub struct Clock {
    pub now_ms: u64,
    pub unix_secs: u64,
    pub offset_secs: i64,
    pub set: bool,
    pub last_input_ms: u64,
}

pub struct Running {
    pub kind: JobKind,
    pub began_ms: u64,
    pub done: u32,
    pub of: u32,
}

/// Fixed for one build of the lock scope.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Mode {
    pub network: Network,
    pub practice: bool,
    pub online: bool,
}

impl Mode {
    /// The mode as a number the settings owner carries: 0 Practice, 1 Mainnet, 2 Testnet,
    /// 3 Signet, 4 Regtest (`main`'s `modes::Mode`, whose Practice is Testnet's public set).
    #[must_use]
    pub fn index(&self) -> u8 {
        if self.practice {
            return 0;
        }
        match self.network {
            Network::Mainnet => 1,
            Network::Testnet => 2,
            Network::Signet => 3,
            Network::Regtest => 4,
        }
    }
}

/// What present has beside `View`. Built only by the runtime.
pub struct Show<'a> {
    strings: &'static Strings,
    memo: &'a Memo,
    note: Option<NoteUp<'a>>,
}

impl<'a> Show<'a> {
    pub fn strings(&self) -> &'static Strings {
        self.strings
    }

    pub fn memo(&self) -> &'a Memo {
        self.memo
    }

    pub fn note(&self) -> Option<NoteUp<'a>> {
        self.note
    }

    /// The runtime's and test support's only (clippy `disallowed-methods`).
    pub fn new(strings: &'static Strings, memo: &'a Memo, note: Option<NoteUp<'a>>) -> Self {
        Show {
            strings,
            memo,
            note,
        }
    }
}

/// The note on show, with the id its rise and fade are keyed by.
#[derive(Clone, Copy)]
pub struct NoteUp<'a> {
    pub note: &'a Note,
    pub id: u32,
}

/// Entries keyed by (slot, the value's type, key); interior mutability, the runtime its only
/// owner.
pub struct Memo(RefCell<Store>);

/// One bucket for each (slot, value type, key type), so a key of any type is kept as itself.
type Store = HashMap<(Slot, TypeId, TypeId), Box<dyn Bucket>>;

trait Bucket {
    /// Drops the entries no frame used since the last call, and marks the rest unused.
    fn end_frame(&mut self);
    fn as_any(&mut self) -> &mut dyn Any;
    fn is_empty(&self) -> bool;
}

struct Entries<K, V> {
    map: HashMap<K, (Rc<V>, bool)>,
}

impl<K: Eq + Hash + 'static, V: 'static> Bucket for Entries<K, V> {
    fn end_frame(&mut self) {
        self.map.retain(|_, (_, used)| core::mem::take(used));
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// "<feature>.<what>", a constant of the feature's crate.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub &'static str);

impl Memo {
    /// The runtime's only: the one store, empty. No `Default`, which would make one anywhere.
    #[expect(
        clippy::new_without_default,
        reason = "only the runtime makes the memo (01 R1)"
    )]
    pub fn new() -> Memo {
        Memo(RefCell::new(HashMap::new()))
    }

    /// The runtime's only, at a frame's end: every entry that frame did not use is dropped.
    pub fn end_frame(&self) {
        let mut store = self.0.borrow_mut();
        for bucket in store.values_mut() {
            bucket.end_frame();
        }
        store.retain(|_, bucket| !bucket.is_empty());
    }

    /// The runtime's only, at every lock and every end: every entry dropped.
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    /// The value kept for (`slot`, `V`, `key`), or `fill(input)` kept and returned. `fill` is a
    /// function, not a closure: it sees only what `input` gives it.
    #[cfg(not(debug_assertions))]
    pub fn get<K, I, V>(&self, slot: Slot, key: K, input: I, fill: fn(I) -> V) -> Rc<V>
    where
        K: Public + Eq + Hash,
        I: Public,
        V: Public + 'static,
    {
        if let Some(kept) = self.hit::<K, V>(slot, &key) {
            return kept;
        }
        self.keep(slot, key, fill(input))
    }

    /// In debug builds `get` fills again on every call and asserts the value equals the kept one.
    #[cfg(debug_assertions)]
    pub fn get<K, I, V>(&self, slot: Slot, key: K, input: I, fill: fn(I) -> V) -> Rc<V>
    where
        K: Public + Eq + Hash,
        I: Public,
        V: Public + PartialEq + 'static,
    {
        match self.hit::<K, V>(slot, &key) {
            Some(kept) => {
                assert!(
                    *kept == fill(input),
                    "memo entry {slot:?} is not what its input fills"
                );
                kept
            }
            None => self.keep(slot, key, fill(input)),
        }
    }

    /// The kept value, marked used.
    fn hit<K: Eq + Hash + 'static, V: 'static>(&self, slot: Slot, key: &K) -> Option<Rc<V>> {
        let mut store = self.0.borrow_mut();
        let bucket = store.get_mut(&(slot, TypeId::of::<V>(), TypeId::of::<K>()))?;
        let entries = bucket.as_any().downcast_mut::<Entries<K, V>>()?;
        let (value, used) = entries.map.get_mut(key)?;
        *used = true;
        Some(Rc::clone(value))
    }

    fn keep<K: Eq + Hash + 'static, V: 'static>(&self, slot: Slot, key: K, value: V) -> Rc<V> {
        let value = Rc::new(value);
        let mut store = self.0.borrow_mut();
        let bucket = store
            .entry((slot, TypeId::of::<V>(), TypeId::of::<K>()))
            .or_insert_with(|| {
                Box::new(Entries::<K, V> {
                    map: HashMap::new(),
                })
            });
        if let Some(entries) = bucket.as_any().downcast_mut::<Entries<K, V>>() {
            entries.map.insert(key, (Rc::clone(&value), true));
        }
        value
    }
}

/// How many times an owner or a feature has changed what it holds.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Edits(u64);

impl Edits {
    /// The count of a part that has changed nothing: what the empty readers answer (05 R26).
    pub const NONE: Edits = Edits(0);

    /// The count after one more change; only an owner calls it.
    #[must_use]
    pub fn after(self) -> Edits {
        Edits(self.0.wrapping_add(1))
    }
}

/// A costly value kept on a state by the inputs it came from. Public, so a state field of it is
/// marked `#[lock(public)]`.
#[derive(Public)]
pub struct Derived<K: Public + Eq, V: Public> {
    key: Option<K>,
    value: Option<V>,
}

impl<K: Public + Eq, V: Public> Derived<K, V> {
    /// Present reads; `None` when missing or stale.
    pub fn get(&self, _key: &K) -> Option<&V> {
        todo!()
    }

    /// Update: worked out with `cx`'s `Compute`, or a job's answer.
    pub fn put(&mut self, _key: K, _value: V) {
        todo!()
    }
}
