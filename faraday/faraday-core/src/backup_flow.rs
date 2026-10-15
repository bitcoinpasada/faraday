//! The shared guided backup flow (`docs/BACKUP-FLOW.md`, F31): one flow
//! that decides where a key will be kept, brings in each stick the plan
//! names with nothing secret open, unlocks and makes the vaults, makes the
//! key straight into them, locks once and writes each stick by place.
//!
//! This file is the flow's state ([`BackupFlow`], §7), its plan, what is
//! kept of it across a lock, and the gate signing waits at (§8). The steps
//! are `backup_flow_steps.rs`; the cards are `backup_flow_screen.rs`.
//!
//! The plan is a plain [`Plan`]: places, each keeping the sticks of some
//! vaults, and the vaults. A key's own plan fills it for the APK key, GPG
//! keys and Secure Boot keys; a Bitcoin wallet's plan fills it from its
//! map (batch B1b). Only places that keep a stick take part in the stick
//! steps.

use faraday_vault::Record;
use faraday_vault::records::kind as rkind;

use crate::Faraday;
use crate::flow;

/// The fewest places a key that cannot be made again from seed words is
/// planned at (`docs/BACKUP-FLOW.md` §1).
pub const MIN_PLACES: usize = 2;

/// What the flow makes. A Nostr key and a Bitcoin wallet's keys join it
/// in their own batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An APK signing key.
    Apk,
    /// A GPG key.
    Gpg,
    /// A set of Secure Boot keys.
    SecureBoot,
}

impl Kind {
    /// Its code in the kept state.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Kind::Apk => "apk",
            Kind::Gpg => "gpg",
            Kind::SecureBoot => "sb",
        }
    }

    pub(crate) fn from_code(s: &str) -> Option<Kind> {
        Some(match s {
            "apk" => Kind::Apk,
            "gpg" => Kind::Gpg,
            "sb" => Kind::SecureBoot,
            _ => return None,
        })
    }

    /// The vault category its keys are listed in (`vaults::CATEGORIES`).
    pub fn category(self) -> usize {
        match self {
            Kind::Apk => 6,
            Kind::Gpg => 4,
            Kind::SecureBoot => 5,
        }
    }

    /// The kind whose keys a vault category lists, when the flow makes
    /// them.
    pub fn of_category(c: usize) -> Option<Kind> {
        match c {
            6 => Some(Kind::Apk),
            4 => Some(Kind::Gpg),
            5 => Some(Kind::SecureBoot),
            _ => None,
        }
    }

    /// The record type its keys are kept as.
    pub fn record_kind(self) -> u8 {
        match self {
            Kind::Apk => rkind::APK_KEY,
            Kind::Gpg => rkind::GPG,
            Kind::SecureBoot => rkind::SECURE_BOOT,
        }
    }

    /// What the key is known by, as its page labels it.
    pub fn subject_label(self) -> &'static str {
        match self {
            Kind::Apk => crate::strings::EN.apk_certificate_sha_256,
            Kind::Gpg => crate::strings::EN.vault_fingerprint,
            Kind::SecureBoot => crate::strings::EN.vault_owner_guid,
        }
    }

    /// The fewest places with a stick.
    pub fn min_places(self) -> usize {
        MIN_PLACES
    }

    /// Its name, for the flow's heading and Home's lead.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Apk => crate::strings::EN.backup_flow_kind_apk,
            Kind::Gpg => crate::strings::EN.backup_flow_kind_gpg,
            Kind::SecureBoot => crate::strings::EN.backup_flow_kind_sb,
        }
    }
}

/// Where the flow is. The numbers are the cards' (§5); a card a plan skips
/// is not drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// 1: where it will be kept.
    Where,
    /// 2: bring in the sticks.
    Sticks,
    /// 3: unlock and make the vaults.
    Vaults,
    /// 4: make the key.
    Make,
    /// 5: lock once.
    Lock,
    /// 6: write each stick by place.
    Write,
    /// 7: done.
    Done,
}

impl Step {
    pub(crate) const ALL: [Step; 7] = [
        Step::Where,
        Step::Sticks,
        Step::Vaults,
        Step::Make,
        Step::Lock,
        Step::Write,
        Step::Done,
    ];

    /// Its place in [`Step::ALL`].
    pub fn n(self) -> u8 {
        self as u8
    }

    pub(crate) fn of(n: u8) -> Option<Step> {
        Step::ALL.get(usize::from(n)).copied()
    }
}

/// How a vault is kept at a place's stick, decided at step 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// Written beside what the stick holds, under a free name.
    New,
    /// The copy of the vault the stick holds, which step 6 replaces.
    Over(String),
}

/// How the person confirmed a stick's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    /// A tap: it reads differently from every stick already bound.
    Tap,
    /// A hold: it reads like a stick bound to another place.
    Hold,
}

/// One vault file a stick holds, as step 2 read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRead {
    /// Its name on the stick.
    pub name: String,
    /// Its length.
    pub len: usize,
    /// SHA-256 of its bytes.
    pub sha: [u8; 32],
    /// The vault's salt: what tells one vault from another.
    pub salt: [u8; 32],
}

/// What step 2 kept of a stick (§4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// Its volume label.
    pub label: String,
    /// Its FAT volume serial.
    pub serial: Option<u32>,
    /// Its files, names and sizes.
    pub listing: Vec<(String, u64)>,
    /// The vault files among them.
    pub vaults: Vec<VaultRead>,
    /// Its free space, bytes, when the shell said.
    pub free: Option<u64>,
    /// When it was read, Unix seconds.
    pub at: Option<u64>,
    /// How its place was confirmed.
    pub confirm: Confirm,
    /// For each vault the stick keeps, how.
    pub picks: Vec<(usize, Pick)>,
    /// The vaults whose copy on this stick the person declined in favour
    /// of another copy of the same vault: the stick's copy is written over
    /// only after a hold (§4.2).
    pub declined: Vec<usize>,
}

/// What a written stick holds (§7 `written`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The stick's id when it was written (not kept across a lock, since a
    /// stick put back is another id).
    pub stick: String,
    /// The stick's label.
    pub label: String,
    /// The stick's FAT volume serial: with the label and the files, what
    /// a wipe or a delete is matched on.
    pub serial: Option<u32>,
    /// When, Unix seconds.
    pub time: Option<u64>,
    /// Each file's name as written and its SHA-256.
    pub files: Vec<(String, [u8; 32])>,
}

/// What a stick is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The stick for all the vaults a place keeps: where a place keeps one
    /// vault, or where the person chose **One stick for both**.
    Place(usize),
    /// The stick of one vault of a place that keeps several, each on a
    /// stick of its own (§4.2: full optionality).
    PlaceVault(usize, usize),
    /// The stick of files (a wallet's seed files).
    Files,
    /// The stick of public files.
    Public,
}

impl Target {
    pub(crate) fn code(self) -> String {
        match self {
            Target::Place(p) => format!("p{p}"),
            Target::PlaceVault(p, v) => format!("q{p}.{v}"),
            Target::Files => "f".into(),
            Target::Public => "u".into(),
        }
    }

    pub(crate) fn from_code(s: &str) -> Option<Target> {
        match s {
            "f" => Some(Target::Files),
            "u" => Some(Target::Public),
            _ => {
                if let Some(rest) = s.strip_prefix('q') {
                    let (p, v) = rest.split_once('.')?;
                    return Some(Target::PlaceVault(p.parse().ok()?, v.parse().ok()?));
                }
                s.strip_prefix('p')?.parse().ok().map(Target::Place)
            }
        }
    }

    /// The place whose stick it is.
    pub fn place(self) -> Option<usize> {
        match self {
            Target::Place(p) | Target::PlaceVault(p, _) => Some(p),
            _ => None,
        }
    }

    /// Sort key: places and their vaults in order, then the extras.
    fn key(self) -> (u8, usize, usize) {
        match self {
            Target::Place(p) => (0, p, 0),
            Target::PlaceVault(p, v) => (0, p, v + 1),
            Target::Files => (1, 0, 0),
            Target::Public => (2, 0, 0),
        }
    }
}

/// One stick the flow asks for, in order (§4.2, §4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// What it is for.
    pub target: Target,
    /// Other files it will hold, by Outbox name: a wallet's, in B1b.
    pub files: Vec<String>,
    /// What step 2 (or step 6, for a stick named later) read.
    pub bound: Option<Bound>,
    /// What was written to it, read back and matched.
    pub written: Option<Written>,
}

/// A vault of the plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultPlan {
    /// The plan gives it as a vault that exists already, not a new one.
    pub existing: bool,
    /// Its file in Files once chosen (existing) or made (new).
    pub file: Option<String>,
    /// Its salt, once it has one.
    pub salt: Option<[u8; 32]>,
    /// Its length.
    pub len: usize,
    /// The place and the stick's file the existing vault was chosen from.
    pub from: Option<(usize, String)>,
}

/// Where the key will be kept: the places, each with the vaults whose
/// stick it keeps (none: no stick there), and the vaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Each place's vaults, by index into `vaults`.
    pub places: Vec<Vec<usize>>,
    /// The vaults.
    pub vaults: Vec<VaultPlan>,
    /// The places that keep several vaults on one stick (**One stick for
    /// both**). A place not listed keeps each vault on a stick of its own.
    pub together: Vec<usize>,
    /// The place whose stick carries the key's public files.
    pub public_at: usize,
}

impl Plan {
    /// The default of a key that cannot be made again: one new vault at
    /// two places (`docs/BACKUP-FLOW.md` §2).
    pub fn key_default() -> Plan {
        Plan {
            places: vec![vec![0], vec![0]],
            vaults: vec![VaultPlan::default()],
            together: Vec::new(),
            public_at: 0,
        }
    }

    /// Whether the place keeps everything on one stick: it keeps one
    /// vault, or the person chose one stick for both.
    pub fn one_stick(&self, p: usize) -> bool {
        self.places.get(p).is_some_and(|vs| vs.len() <= 1) || self.together.contains(&p)
    }

    /// The sticks the plan names for its places, in order.
    pub fn place_targets(&self) -> Vec<Target> {
        let mut out = Vec::new();
        for p in self.stick_places() {
            if self.one_stick(p) {
                out.push(Target::Place(p));
            } else if let Some(vs) = self.places.get(p) {
                out.extend(vs.iter().map(|&v| Target::PlaceVault(p, v)));
            }
        }
        out
    }

    /// The places that keep a stick.
    pub fn stick_places(&self) -> Vec<usize> {
        (0..self.places.len())
            .filter(|&p| !self.places[p].is_empty())
            .collect()
    }

    /// The places that keep vault `v`.
    pub fn places_of(&self, v: usize) -> Vec<usize> {
        (0..self.places.len())
            .filter(|&p| self.places[p].contains(&v))
            .collect()
    }

    /// The vaults some place keeps, in order.
    pub fn used_vaults(&self) -> Vec<usize> {
        (0..self.vaults.len())
            .filter(|&v| self.places.iter().any(|p| p.contains(&v)))
            .collect()
    }

    /// Drops the vaults no place keeps and renumbers the rest. Returns the
    /// vaults dropped, by their old index.
    pub(crate) fn tidy(&mut self) -> Vec<usize> {
        let used = self.used_vaults();
        let dropped: Vec<usize> = (0..self.vaults.len())
            .filter(|v| !used.contains(v))
            .collect();
        if dropped.is_empty() {
            return dropped;
        }
        let map = |v: usize| used.iter().position(|&u| u == v).unwrap_or(0);
        for p in &mut self.places {
            for v in p.iter_mut() {
                *v = map(*v);
            }
        }
        self.vaults = used.iter().map(|&u| self.vaults[u].clone()).collect();
        dropped
    }
}

/// What the key is known by once made (§3): the public value, as hex, and
/// the name it is shown under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The public value, hex: the APK certificate's SHA-256, the GPG
    /// primary key's fingerprint, the Secure Boot owner GUID.
    pub id: String,
    /// The name the key is called.
    pub name: String,
}

/// The SHA-256 of a sealing of a vault, which a stick is written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    /// The plan's vault.
    pub vault: usize,
    /// The Outbox file's name.
    pub name: String,
    /// Its length.
    pub len: usize,
    /// SHA-256 of its bytes.
    pub sha: [u8; 32],
}

/// Why a stick cannot be used, or is not the one wanted (§4.2, §4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// An `OSKBOOT` partition.
    Boot,
    /// Not a FAT32 stick, as Faraday reads sticks.
    NotFat,
    /// The stick cannot be read (its room could not be asked for).
    Unreadable,
    /// Cannot be written.
    ReadOnly,
    /// Too little room: needed and free, bytes.
    NoRoom(u64, u64),
    /// The plan gives an existing vault and the stick holds none.
    NoVault(usize),
    /// Two files of one vault: the vault, and the two names.
    TwoCopies(usize, String, String),
    /// Reads like the stick bound to this target.
    BoundTo(Target),
    /// Not the stick read for this target at step 2.
    NotRead(Target),
    /// Holds the first copy of a vault the flow wrote.
    HasFirst,
    /// Holds a file the flow wrote for another stick.
    HasFilesOf(Target),
    /// The copy did not check.
    NotChecked,
    /// Pulled before the copy was checked.
    Pulled,
}

/// What a stick read is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Step 2 (or step 6, for a stick named later): usable? which vault?
    /// which place?
    Bind,
    /// Step 6: is it the stick read at step 2? does it hold a copy
    /// already?
    Check,
}

/// A stick being read.
#[derive(Debug, Clone)]
pub struct Reading {
    /// The shell's id for it.
    pub id: String,
    /// The slot it is read for.
    pub slot: usize,
    /// What for.
    pub purpose: Purpose,
    /// Its label, serial and listing.
    pub label: String,
    /// Its serial.
    pub serial: Option<u32>,
    /// Its files.
    pub listing: Vec<(String, u64)>,
    /// Files still being read.
    pub pending: Vec<String>,
    /// The space is still being asked for.
    pub space_pending: bool,
    /// Free bytes.
    pub free: Option<u64>,
    /// The vault files read, with their bytes.
    pub files: Vec<(VaultRead, Vec<u8>)>,
    /// Why it cannot be used, or is refused.
    pub problem: Option<Problem>,
    /// How each vault the stick keeps is kept on it, as chosen.
    pub picks: Vec<(usize, Pick)>,
    /// Bound: waits for the stick to be pulled.
    pub bound: bool,
    /// The second-stick hold is still needed (step 6).
    pub hold: bool,
    /// The vault whose copy on this stick differs from one already in
    /// Files: which copy to unlock is asked before the place (§4.2).
    pub differs: Option<(usize, String)>,
    /// The vaults whose copy on this stick was declined.
    pub declined: Vec<usize>,
    /// The stick is the medium the machine started from: it stays in, so
    /// nothing waits for it to be pulled.
    pub boot: bool,
    /// The copy was refused until the hold is done.
    pub checked: bool,
}

/// A write under way (step 6).
#[derive(Debug, Clone)]
pub struct Writing {
    /// The stick's id.
    pub id: String,
    /// The slot.
    pub slot: usize,
    /// Files sent and not yet answered: the name sent and its SHA-256.
    pub pending: Vec<(String, [u8; 32])>,
    /// Files answered, read back and matched: the name written as.
    pub done: Vec<(String, [u8; 32])>,
    /// The stick's label.
    pub label: String,
    /// The stick's FAT volume serial.
    pub serial: Option<u32>,
}

/// What the screen keeps of a flow; none of it is kept across a lock.
#[derive(Debug, Clone, Default)]
pub struct Live {
    /// The card open.
    pub open: Option<u8>,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The stick being read.
    pub reading: Option<Reading>,
    /// Sticks (by id) left alone until they are pulled.
    pub ignored: Vec<String>,
    /// A write under way.
    pub writing: Option<Writing>,
    /// A line over the card: why the last stick was refused.
    pub problem: Option<Problem>,
    /// The vault whose "I do not have its passphrase" is open.
    pub no_pass: Option<usize>,
    /// The vault being unlocked.
    pub unlocking: Option<usize>,
    /// The vault being made, while its form shows.
    pub creating: Option<usize>,
    /// The key is being made.
    pub making: bool,
    /// Two sticks are in.
    pub crowd: bool,
}

/// One flow (§7).
#[derive(Debug, Clone)]
pub struct BackupFlow {
    /// What it makes.
    pub kind: Kind,
    /// Where it is.
    pub step: Step,
    /// The plan.
    pub plan: Plan,
    /// Each stick, in order.
    pub slots: Vec<Slot>,
    /// The key is made and is in the vaults, in memory or sealed.
    pub made: bool,
    /// The vaults the key is in, by plan index.
    pub made_in: Vec<usize>,
    /// What the key is known by.
    pub subject: Option<Subject>,
    /// What each vault was sealed as.
    pub sealing: Vec<Sealed>,
    /// The key's public files made with it (the Outbox names): written to
    /// the stick of the place the plan names.
    pub public: Vec<String>,
    /// The extra round for a vault added after the key was made (§4.8).
    pub adding: bool,
    /// The flow was on screen when it was last kept.
    pub on_screen: bool,
    /// The screen's state.
    pub live: Live,
}

impl BackupFlow {
    /// A new flow at step 1 with `kind`'s own plan.
    pub fn new(kind: Kind) -> BackupFlow {
        BackupFlow::with_plan(kind, Plan::key_default())
    }

    /// A new flow at step 1 with this plan. Batch B1b makes a wallet's
    /// from its map.
    pub fn with_plan(kind: Kind, plan: Plan) -> BackupFlow {
        let mut f = BackupFlow {
            kind,
            step: Step::Where,
            plan,
            slots: Vec::new(),
            made: false,
            made_in: Vec::new(),
            subject: None,
            sealing: Vec::new(),
            public: Vec::new(),
            adding: false,
            on_screen: true,
            live: Live::default(),
        };
        f.live.open = Some(Step::Where.n());
        f.sync_slots();
        f
    }

    /// The slots follow the plan: a stick per place that keeps one (or per
    /// vault of a place that keeps several apart), in order, then the
    /// extras. A place that left takes its slot, and its public files go
    /// to the first stick.
    pub(crate) fn sync_slots(&mut self) {
        if self.plan.public_at >= self.plan.places.len() {
            self.plan.public_at = 0;
        }
        let want = self.plan.place_targets();
        let mut orphaned: Vec<String> = Vec::new();
        self.slots.retain(|s| {
            let keep = match s.target {
                Target::Place(_) | Target::PlaceVault(..) => want.contains(&s.target),
                _ => true,
            };
            if !keep {
                orphaned.extend(s.files.iter().cloned());
            }
            keep
        });
        for t in want {
            if !self.slots.iter().any(|s| s.target == t) {
                let at = self
                    .slots
                    .iter()
                    .position(|s| s.target.key() > t.key())
                    .unwrap_or(self.slots.len());
                self.slots.insert(
                    at,
                    Slot {
                        target: t,
                        files: Vec::new(),
                        bound: None,
                        written: None,
                    },
                );
            }
        }
        if !orphaned.is_empty()
            && let Some(first) = self.slots.first_mut()
        {
            for n in orphaned {
                if !first.files.contains(&n) {
                    first.files.push(n);
                }
            }
        }
    }

    /// The slots of place `p`.
    pub(crate) fn slots_at(&self, p: usize) -> Vec<usize> {
        (0..self.slots.len())
            .filter(|&k| self.slots[k].target.place() == Some(p))
            .collect()
    }

    /// The slot that carries the key's public files: the first stick of the
    /// place the plan names, else the first.
    pub(crate) fn public_slot(&self) -> Option<usize> {
        self.slots_at(self.plan.public_at)
            .first()
            .copied()
            .or(if self.slots.is_empty() { None } else { Some(0) })
    }

    /// How many sticks are written, and how many there are.
    pub fn written_of(&self) -> (usize, usize) {
        (
            self.slots.iter().filter(|s| s.written.is_some()).count(),
            self.slots.len(),
        )
    }

    /// Every stick the plan gives the key is written.
    pub fn all_written(&self) -> bool {
        self.made && !self.slots.is_empty() && self.slots.iter().all(|s| s.written.is_some())
    }

    /// The key is made and waiting for its writes: signing waits.
    pub fn waiting(&self) -> bool {
        self.made && !self.all_written()
    }

    /// The files the write of `slot` carries beyond its own: a Secure Boot
    /// key's enrolment files go in the last write of step 6, once every
    /// other stick is written, so that the backup is done when they are
    /// put out (`docs/BACKUP-FLOW.md` §3).
    pub(crate) fn last_write_files(&self, slot: usize) -> Vec<String> {
        if self.kind == Kind::SecureBoot
            && self
                .slots
                .iter()
                .enumerate()
                .all(|(k, s)| k == slot || s.written.is_some())
        {
            self.public.clone()
        } else {
            Vec::new()
        }
    }

    /// The vaults the stick of `slot` keeps.
    pub(crate) fn vaults_of(&self, slot: usize) -> Vec<usize> {
        match self.slots.get(slot).map(|s| s.target) {
            Some(Target::Place(p)) => self.plan.places.get(p).cloned().unwrap_or_default(),
            Some(Target::PlaceVault(_, v)) => vec![v],
            _ => Vec::new(),
        }
    }

    /// Which copy of vault `v` the stick of `slot` takes: 0 the first.
    pub(crate) fn copy_number(&self, v: usize, slot: usize) -> usize {
        let Some(p) = self.slots.get(slot).and_then(|s| s.target.place()) else {
            return 0;
        };
        self.plan
            .places_of(v)
            .iter()
            .position(|&q| q == p)
            .unwrap_or(0)
    }

    /// The sealing of vault `v` the writes use: the last made.
    pub(crate) fn sealed_of(&self, v: usize) -> Option<&Sealed> {
        self.sealing.iter().rev().find(|s| s.vault == v)
    }

    /// "Vault 1".
    pub fn vault_name(v: usize) -> String {
        crate::strings::fill(
            crate::strings::EN.backup_flow_vault_n,
            &[&(v + 1).to_string()],
        )
    }

    /// "Place 1".
    pub fn place_name(p: usize) -> String {
        crate::strings::fill(
            crate::strings::EN.backup_flow_place_n,
            &[&(p + 1).to_string()],
        )
    }
}

/// A field of the kept state: no tab or line break.
fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn hex(h: &[u8; 32]) -> String {
    faraday_apk::hex(h)
}

/// The kept state's version: a flow written by another version is not
/// read.
const VERSION: &str = "2";
/// The most places, vaults and sticks a kept flow may name; more is a
/// corrupt state.
const MAX_PLACES: usize = 64;
const MAX_SLOTS: usize = 256;

/// The flows as the kept state holds them: a version line, then a line
/// each, split by tabs. Nothing in it is secret: names, sizes, salts,
/// hashes, labels and public values.
pub fn encode(flows: &[BackupFlow]) -> Vec<u8> {
    let mut out = format!("version\t{VERSION}\n");
    for f in flows {
        let (id, name) = f
            .subject
            .as_ref()
            .map_or((String::new(), String::new()), |s| {
                (s.id.clone(), clean(&s.name))
            });
        out.push_str(&format!(
            "flow\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            f.kind.code(),
            f.step.n(),
            u8::from(f.made),
            u8::from(f.adding),
            id,
            name,
            u8::from(f.on_screen),
        ));
        let together: Vec<String> = f.plan.together.iter().map(|p| p.to_string()).collect();
        out.push_str(&format!(
            "plan\t{}\t{}\n",
            f.plan.public_at,
            together.join(",")
        ));
        for name in &f.public {
            out.push_str(&format!("pub\t{}\n", clean(name)));
        }
        for (p, vs) in f.plan.places.iter().enumerate() {
            let vs: Vec<String> = vs.iter().map(|v| v.to_string()).collect();
            out.push_str(&format!("place\t{p}\t{}\n", vs.join(",")));
        }
        for (v, vp) in f.plan.vaults.iter().enumerate() {
            let (fp, fname) = vp
                .from
                .as_ref()
                .map_or((String::new(), String::new()), |(p, n)| {
                    (p.to_string(), clean(n))
                });
            out.push_str(&format!(
                "vault\t{v}\t{}\t{}\t{}\t{}\t{fp}\t{fname}\t{}\n",
                u8::from(vp.existing),
                clean(vp.file.as_deref().unwrap_or("")),
                vp.salt.as_ref().map_or(String::new(), hex),
                vp.len,
                u8::from(f.made_in.contains(&v)),
            ));
        }
        for (k, s) in f.slots.iter().enumerate() {
            out.push_str(&format!("slot\t{k}\t{}\n", s.target.code()));
            for file in &s.files {
                out.push_str(&format!("sfile\t{k}\t{}\n", clean(file)));
            }
            if let Some(b) = &s.bound {
                out.push_str(&format!(
                    "bound\t{k}\t{}\t{}\t{}\t{}\t{}\n",
                    clean(&b.label),
                    b.serial.map_or(String::new(), |s| s.to_string()),
                    b.free.map_or(String::new(), |s| s.to_string()),
                    b.at.map_or(String::new(), |s| s.to_string()),
                    match b.confirm {
                        Confirm::Tap => "t",
                        Confirm::Hold => "h",
                    },
                ));
                for (n, size) in &b.listing {
                    out.push_str(&format!("list\t{k}\t{}\t{size}\n", clean(n)));
                }
                for r in &b.vaults {
                    out.push_str(&format!(
                        "vread\t{k}\t{}\t{}\t{}\t{}\n",
                        clean(&r.name),
                        r.len,
                        hex(&r.sha),
                        hex(&r.salt)
                    ));
                }
                for (v, p) in &b.picks {
                    let over = match p {
                        Pick::New => String::new(),
                        Pick::Over(n) => clean(n),
                    };
                    out.push_str(&format!(
                        "pick\t{k}\t{v}\t{over}\t{}\n",
                        u8::from(b.declined.contains(v))
                    ));
                }
            }
            if let Some(w) = &s.written {
                out.push_str(&format!(
                    "wrote\t{k}\t{}\t{}\t{}\n",
                    clean(&w.label),
                    w.time.map_or(String::new(), |t| t.to_string()),
                    w.serial.map_or(String::new(), |t| t.to_string()),
                ));
                for (n, sha) in &w.files {
                    out.push_str(&format!("wfile\t{k}\t{}\t{}\n", clean(n), hex(sha)));
                }
            }
        }
        for s in &f.sealing {
            out.push_str(&format!(
                "seal\t{}\t{}\t{}\t{}\n",
                s.vault,
                clean(&s.name),
                s.len,
                hex(&s.sha)
            ));
        }
    }
    out.into_bytes()
}

/// Read back what [`encode`] kept, and how many flows were dropped: a
/// state of another version, one that does not read, or one whose parts do
/// not agree (an index out of range, a stick that the plan does not name,
/// a step past the key with nothing sealed) is dropped whole.
pub fn decode(bytes: &[u8]) -> (Vec<BackupFlow>, usize) {
    use crate::apk::hex_array;
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines();
    if lines.next().map(|l| l.split('\t').collect::<Vec<_>>()) != Some(vec!["version", VERSION]) {
        return (Vec::new(), usize::from(!bytes.is_empty()));
    }
    // A flow being read; `None` once a line of it did not read.
    let mut flows: Vec<Option<BackupFlow>> = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        if let ["flow", kind, step, made, adding, id, name, on] = f[..] {
            flows.push(
                Kind::from_code(kind)
                    .zip(step.parse().ok().and_then(Step::of))
                    .map(|(kind, step)| BackupFlow {
                        kind,
                        step,
                        plan: Plan::default(),
                        slots: Vec::new(),
                        made: made == "1",
                        made_in: Vec::new(),
                        subject: (!id.is_empty()).then(|| Subject {
                            id: id.to_string(),
                            name: name.to_string(),
                        }),
                        sealing: Vec::new(),
                        public: Vec::new(),
                        adding: adding == "1",
                        on_screen: on == "1",
                        live: Live::default(),
                    }),
            );
            continue;
        }
        let Some(slot_f) = flows.last_mut() else {
            continue;
        };
        // A line that does not read spoils its flow.
        let ok = match slot_f.as_mut() {
            Some(fl) => read_line(fl, &f, hex_array),
            None => true,
        };
        if !ok {
            *slot_f = None;
        }
    }
    let total = flows.len();
    let kept: Vec<BackupFlow> = flows
        .into_iter()
        .flatten()
        .filter_map(|mut f| {
            if !f.valid() {
                return None;
            }
            f.live.open = Some(f.step.n());
            Some(f)
        })
        .collect();
    let dropped = total - kept.len();
    (kept, dropped)
}

/// One line of a flow. False when it does not read or names what is not
/// there.
fn read_line(fl: &mut BackupFlow, f: &[&str], hex_array: fn(&str) -> Option<[u8; 32]>) -> bool {
    let slot = |fl: &mut BackupFlow, k: &str| -> Option<usize> {
        let k: usize = k.parse().ok()?;
        (k < fl.slots.len()).then_some(k)
    };
    match *f {
        ["plan", public_at, together] => {
            let Ok(p) = public_at.parse() else {
                return false;
            };
            fl.plan.public_at = p;
            fl.plan.together = together
                .split(',')
                .filter(|x| !x.is_empty())
                .filter_map(|x| x.parse().ok())
                .collect();
            true
        }
        ["pub", name] => {
            fl.public.push(name.to_string());
            true
        }
        ["place", p, vs] => {
            let Ok(p) = p.parse::<usize>() else {
                return false;
            };
            if p >= MAX_PLACES || p != fl.plan.places.len() {
                return false;
            }
            fl.plan
                .places
                .push(vs.split(',').filter_map(|v| v.parse().ok()).collect());
            true
        }
        ["vault", v, existing, file, salt, len, from_p, from_n, held] => {
            let Ok(v) = v.parse::<usize>() else {
                return false;
            };
            if v >= MAX_PLACES || v != fl.plan.vaults.len() {
                return false;
            }
            fl.plan.vaults.push(VaultPlan {
                existing: existing == "1",
                file: (!file.is_empty()).then(|| file.to_string()),
                salt: hex_array(salt),
                len: len.parse().unwrap_or(0),
                from: from_p.parse().ok().map(|p| (p, from_n.to_string())),
            });
            if held == "1" {
                fl.made_in.push(v);
            }
            true
        }
        ["slot", k, target] => {
            let (Ok(k), Some(t)) = (k.parse::<usize>(), Target::from_code(target)) else {
                return false;
            };
            if k >= MAX_SLOTS || k != fl.slots.len() {
                return false;
            }
            fl.slots.push(Slot {
                target: t,
                files: Vec::new(),
                bound: None,
                written: None,
            });
            true
        }
        ["sfile", k, name] => match slot(fl, k).and_then(|k| fl.slots.get_mut(k)) {
            Some(s) => {
                s.files.push(name.to_string());
                true
            }
            None => false,
        },
        ["bound", k, label, serial, free, at, confirm] => {
            match slot(fl, k).and_then(|k| fl.slots.get_mut(k)) {
                Some(s) => {
                    s.bound = Some(Bound {
                        label: label.to_string(),
                        serial: serial.parse().ok(),
                        listing: Vec::new(),
                        vaults: Vec::new(),
                        free: free.parse().ok(),
                        at: at.parse().ok(),
                        confirm: if confirm == "h" {
                            Confirm::Hold
                        } else {
                            Confirm::Tap
                        },
                        picks: Vec::new(),
                        declined: Vec::new(),
                    });
                    true
                }
                None => false,
            }
        }
        ["list", k, name, size] => {
            match slot(fl, k)
                .and_then(|k| fl.slots.get_mut(k))
                .and_then(|s| s.bound.as_mut())
            {
                Some(b) => {
                    b.listing
                        .push((name.to_string(), size.parse().unwrap_or(0)));
                    true
                }
                None => false,
            }
        }
        ["vread", k, name, len, sha, salt] => {
            match (
                slot(fl, k)
                    .and_then(|k| fl.slots.get_mut(k))
                    .and_then(|s| s.bound.as_mut()),
                hex_array(sha),
                hex_array(salt),
            ) {
                (Some(b), Some(sha), Some(salt)) => {
                    b.vaults.push(VaultRead {
                        name: name.to_string(),
                        len: len.parse().unwrap_or(0),
                        sha,
                        salt,
                    });
                    true
                }
                _ => false,
            }
        }
        ["pick", k, v, over, declined] => {
            let (Some(b), Ok(v)) = (
                slot(fl, k)
                    .and_then(|k| fl.slots.get_mut(k))
                    .and_then(|s| s.bound.as_mut()),
                v.parse::<usize>(),
            ) else {
                return false;
            };
            b.picks.push((
                v,
                if over.is_empty() {
                    Pick::New
                } else {
                    Pick::Over(over.to_string())
                },
            ));
            if declined == "1" {
                b.declined.push(v);
            }
            true
        }
        ["wrote", k, label, time, serial] => match slot(fl, k).and_then(|k| fl.slots.get_mut(k)) {
            Some(s) => {
                s.written = Some(Written {
                    stick: String::new(),
                    label: label.to_string(),
                    serial: serial.parse().ok(),
                    time: time.parse().ok(),
                    files: Vec::new(),
                });
                true
            }
            None => false,
        },
        ["wfile", k, name, sha] => {
            match (
                slot(fl, k)
                    .and_then(|k| fl.slots.get_mut(k))
                    .and_then(|s| s.written.as_mut()),
                hex_array(sha),
            ) {
                (Some(w), Some(sha)) => {
                    w.files.push((name.to_string(), sha));
                    true
                }
                _ => false,
            }
        }
        ["seal", v, name, len, sha] => match (v.parse::<usize>(), hex_array(sha)) {
            (Ok(vault), Some(sha)) => {
                fl.sealing.push(Sealed {
                    vault,
                    name: name.to_string(),
                    len: len.parse().unwrap_or(0),
                    sha,
                });
                true
            }
            _ => false,
        },
        _ => false,
    }
}

impl BackupFlow {
    /// Whether what was read back agrees with itself, and brings the slots
    /// to the plan. A flow past the key must have made it and sealed a copy
    /// of each vault.
    fn valid(&mut self) -> bool {
        let nv = self.plan.vaults.len();
        let np = self.plan.places.len();
        if self.plan.places.iter().flatten().any(|&v| v >= nv)
            || self.made_in.iter().any(|&v| v >= nv)
            || self.sealing.iter().any(|s| s.vault >= nv)
            || self.plan.together.iter().any(|&p| p >= np)
        {
            return false;
        }
        for vp in &mut self.plan.vaults {
            if vp.from.as_ref().is_some_and(|(p, _)| *p >= np) {
                vp.from = None;
            }
        }
        // Every stick named is one the plan gives, in order; the plan's own
        // that are missing are added.
        let want = self.plan.place_targets();
        let named: Vec<Target> = self
            .slots
            .iter()
            .map(|s| s.target)
            .filter(|t| t.place().is_some())
            .collect();
        if named.iter().any(|t| !want.contains(t)) {
            return false;
        }
        let before = self.slots.len();
        self.sync_slots();
        // A slot's bound picks must name vaults of the plan.
        for s in &self.slots {
            if let Some(b) = &s.bound
                && b.picks.iter().any(|(v, _)| *v >= nv)
            {
                return false;
            }
        }
        let _ = before;
        if self.made && self.subject.is_none() {
            return false;
        }
        if self.adding && !self.made {
            return false;
        }
        if self.step >= Step::Lock && !self.made {
            return false;
        }
        // A made key is never dropped for what is wrong with its steps: the
        // gate would open. It is repaired, and goes back to where it can be
        // finished.
        if self.step >= Step::Write {
            let sealed = self
                .plan
                .used_vaults()
                .into_iter()
                .all(|v| self.sealed_of(v).is_some());
            if !self.made {
                return false;
            }
            if !sealed {
                self.step = Step::Vaults;
                self.adding = true;
            }
        }
        if self.step == Step::Done && !self.slots.is_empty() && !self.all_written() {
            if !self.made {
                return false;
            }
            self.step = Step::Write;
        }
        true
    }
}

/// The keys that wait for their backup, kept apart from the flows so that a
/// flow that does not read cannot open the gate: one line each, the kind,
/// the key's id and its name.
pub(crate) fn gate_encode(flows: &[BackupFlow]) -> Vec<u8> {
    let mut out = String::new();
    for f in flows.iter().filter(|f| f.waiting()) {
        if let Some(s) = &f.subject {
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                f.kind.code(),
                s.id,
                clean(&s.name)
            ));
        }
    }
    out.into_bytes()
}

/// What a key's page says of its backup (§8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRow {
    /// "Not written", "Written to 1 of 2 places", "Written to every place".
    pub line: String,
    /// The backup is not done: the page offers Continue the backup.
    pub waiting: bool,
}

impl Faraday {
    /// The flow that made the key `subject`, when it is kept.
    pub(crate) fn flow_of(&self, kind: Kind, subject: &[u8]) -> Option<&BackupFlow> {
        let id = hex_text(subject);
        self.backup_flows
            .iter()
            .find(|f| f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == id))
    }

    /// The writes the key `subject` waits for: how many are done and how
    /// many there are. `None` once every place the plan gives it is
    /// written, and for a key no flow made here, which signs at once
    /// (§8).
    pub fn backup_waits(&self, kind: Kind, subject: &[u8]) -> Option<(usize, usize)> {
        self.flow_of(kind, subject)
            .filter(|f| f.waiting())
            .map(|f| f.written_of())
    }

    /// The key page's Backup row: `None` for a key unlocked from a stick.
    pub fn backup_row(&self, kind: Kind, subject: &[u8]) -> Option<BackupRow> {
        let f = self.flow_of(kind, subject)?;
        let (n, m) = f.written_of();
        let en = &crate::strings::EN;
        let line = if f.all_written() {
            en.backup_flow_row_all.to_string()
        } else if n == 0 {
            en.backup_flow_row_none.to_string()
        } else {
            crate::strings::fill(en.backup_flow_row_some, &[&n.to_string(), &m.to_string()])
        };
        Some(BackupRow {
            line,
            waiting: !f.all_written(),
        })
    }

    /// Reads back what the previous process kept. A flow that was on screen
    /// opens on its step (`docs/BACKUP-FLOW.md` §5).
    pub(crate) fn flow_restore(&mut self, bytes: &[u8]) {
        let (flows, dropped) = decode(bytes);
        self.backup_flows = flows;
        if dropped > 0 {
            self.toast(crate::strings::EN.backup_flow_dropped);
        }
        let on = self.backup_flows.iter().position(|f| f.on_screen);
        self.flow_on = None;
        if let Some(i) = on {
            self.flow_show(i);
        }
    }

    /// Reads the keys that wait. One whose flow did not read comes back as
    /// a flow of its own at Where, made and unwritten, so it still waits.
    pub(crate) fn flow_gate_restore(&mut self, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            let mut p = line.splitn(3, '\t');
            let (Some(k), Some(id), Some(name)) = (p.next(), p.next(), p.next()) else {
                continue;
            };
            let Some(kind) = Kind::from_code(k) else {
                continue;
            };
            let known = self.backup_flows.iter().any(|f| {
                f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == id)
            });
            if known {
                continue;
            }
            let mut f = BackupFlow::new(kind);
            f.made = true;
            f.on_screen = false;
            f.subject = Some(Subject {
                id: id.to_string(),
                name: name.to_string(),
            });
            self.backup_flows.push(f);
        }
    }

    /// Opens the flow that made the key `subject` on its step: what a
    /// waiting action, **Continue the backup**, does.
    pub(crate) fn backup_continue(&mut self, kind: Kind, subject: &[u8]) {
        let id = hex_text(subject);
        if let Some(i) = self
            .backup_flows
            .iter()
            .position(|f| f.kind == kind && f.subject.as_ref().is_some_and(|s| s.id == id))
        {
            self.flow_show(i);
        }
    }

    /// The flow Home leads with and the Power sheet lists: made, and not
    /// every place written.
    pub fn backup_pending(&self) -> Option<(usize, &BackupFlow)> {
        self.backup_flows
            .iter()
            .enumerate()
            .find(|(_, f)| f.waiting())
    }

    /// What the Power sheet says of each backup not finished (§9).
    pub(crate) fn backup_power_lines(&self) -> Vec<String> {
        self.backup_flows
            .iter()
            .filter(|f| f.waiting())
            .map(|f| {
                let (n, m) = f.written_of();
                crate::strings::fill(
                    crate::strings::EN.backup_flow_power,
                    &[
                        &f.subject
                            .as_ref()
                            .map_or(f.kind.name().to_string(), |s| s.name.clone()),
                        &n.to_string(),
                        &m.to_string(),
                    ],
                )
            })
            .collect()
    }

    /// A flow not finished and not yet at the key made, of this kind: what
    /// the kind's tile resumes (**Continue**).
    pub(crate) fn flow_resumable(&self, kind: Kind) -> Option<usize> {
        self.backup_flows
            .iter()
            .position(|f| f.kind == kind && !f.made && f.step != Step::Done)
    }

    /// The record of `subject` in open vault `o`, to copy it across.
    pub(crate) fn flow_record_in(&self, o: usize, kind: Kind, subject: &str) -> Option<Record> {
        let open = self.vaults.open.get(o)?;
        open.contents
            .records
            .iter()
            .find(|r| {
                r.kind == kind.record_kind() && subject_of(kind, r).as_deref() == Some(subject)
            })
            .cloned()
    }
}

/// Bytes as lower-case hex.
pub(crate) fn hex_text(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The public value a record of `kind` is known by, as hex.
pub(crate) fn subject_of(kind: Kind, r: &Record) -> Option<String> {
    Some(match kind {
        Kind::Apk => hex_text(&crate::apk::fingerprint_of(r)?),
        Kind::Gpg => hex_text(&crate::gpg::key_of(r)?.fingerprint()),
        Kind::SecureBoot => hex_text(r.field(crate::secureboot::OWNER_FIELD)?),
    })
}
