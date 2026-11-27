//! `Ctx`, `View`, the facts, `Show`, the memo and `Derived` (03 §5; 04 §3, §6, §13).

use core::cell::RefCell;
use core::hash::Hash;
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

/// The three owners' states read for an owner's page, built by the runtime from its `Tops`;
/// `None` outside the Starter's lock scope.
pub struct TopsView<'a> {
    pub settings: &'a Settings,
    pub session: Option<&'a dyn session::Read>,
    pub vaults: Option<&'a dyn vaults::Read>,
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
// Spec: 03 §5 gives `Sticks` as "attached, listed, boot medium, problems" without a shape.
pub struct Sticks {
    pub attached: bool,
    pub listed: Vec<StickInfo>,
    pub boot_medium: Option<BootMedium>,
    /// A stick the backup flow cannot use, and whether it is read-only.
    pub problems: Vec<(StickId, bool)>,
}

/// `HostFact::Downloads`: the folder and its files with their sizes.
// Spec: 03 §5 names `Downloads` as `port::HostFact::Downloads`, latest.
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
// Spec: 03 §5 gives `SystemFacts` without a shape; one field per fact is the plainest.
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
// Spec: 03 §2.4 names `FactsChanged` and lists "sticks, display, cameras, time".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactsChanged {
    Sticks,
    Display,
    Cameras,
    Time,
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

/// What present has beside `View`. Built only by the runtime.
pub struct Show<'a> {
    strings: &'static Strings,
    memo: &'a Memo,
    note: Option<NoteUp<'a>>,
}

impl<'a> Show<'a> {
    pub fn strings(&self) -> &'static Strings {
        todo!()
    }

    pub fn memo(&self) -> &'a Memo {
        todo!()
    }

    pub fn note(&self) -> Option<NoteUp<'a>> {
        todo!()
    }

    /// The runtime's and test support's only (clippy `disallowed-methods`).
    pub fn new(_strings: &'static Strings, _memo: &'a Memo, _note: Option<NoteUp<'a>>) -> Self {
        todo!()
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
pub struct Memo(RefCell<()>);

/// "<feature>.<what>", a constant of the feature's crate.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub &'static str);

impl Memo {
    /// The value kept for (`slot`, `V`, `key`), or `fill(input)` kept and returned. `fill` is a
    /// function, not a closure: it sees only what `input` gives it.
    #[cfg(not(debug_assertions))]
    pub fn get<K, I, V>(&self, _slot: Slot, _key: K, _input: I, _fill: fn(I) -> V) -> Rc<V>
    where
        K: Public + Eq + Hash,
        I: Public,
        V: Public + 'static,
    {
        todo!()
    }

    /// In debug builds `get` fills again on every call and asserts the value equals the kept one.
    #[cfg(debug_assertions)]
    pub fn get<K, I, V>(&self, _slot: Slot, _key: K, _input: I, _fill: fn(I) -> V) -> Rc<V>
    where
        K: Public + Eq + Hash,
        I: Public,
        V: Public + PartialEq + 'static,
    {
        todo!()
    }
}

/// How many times an owner or a feature has changed what it holds.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Edits(u64);

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
