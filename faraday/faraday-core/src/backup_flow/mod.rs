//! The shared guided backup flow (`docs/BACKUP-FLOW.md`, F31): one flow
//! that decides where a key will be kept, brings in each stick the plan
//! names with nothing secret open, unlocks and makes the vaults, makes the
//! key straight into them, locks once and writes each stick by place.
//!
//! This file is the flow's state ([`BackupFlow`], §7), its plan, what is
//! kept of it across a lock, and the gate signing waits at (§8). The steps
//! are `backup_flow/steps.rs`; the cards are `backup_flow/screen.rs`.
//!
//! The plan is a plain [`Plan`]: places, each keeping the sticks of some
//! vaults, and the vaults. A key's own plan fills it for GPG keys and
//! Secure Boot keys; a Bitcoin wallet's plan fills it from its
//! map (batch B1b). Only places that keep a stick take part in the stick
//! steps.

mod area;
pub(crate) mod screen;
pub mod steps;
pub(crate) mod wallet;
pub(crate) mod wallet_files;
pub(crate) use wallet_files::wallet_share_record;
pub(crate) mod wallet_screen;

pub(crate) use area::BackupFlowArea;

use faraday_crypto::work::Compute;
use faraday_keys::vault::RecordView;
use faraday_vault::records::kind as rkind;

use crate::flow;
use crate::strings::{EN as FEN, fill};

/// The fewest places a key that cannot be made again from seed words is
/// planned at (`docs/BACKUP-FLOW.md` §1).
pub const MIN_PLACES: usize = 2;

/// What the flow makes. A Nostr key and a Bitcoin wallet's keys join it
/// in their own batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A GPG key.
    Gpg,
    /// A set of Secure Boot keys.
    SecureBoot,
    /// A Bitcoin wallet's keys, with the backup plan of its checklist
    /// (`docs/BACKUP-FLOW.md` §10).
    Wallet,
}

impl Kind {
    /// Its code in the kept state.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Kind::Gpg => "gpg",
            Kind::SecureBoot => "sb",
            Kind::Wallet => "w",
        }
    }

    pub(crate) fn from_code(s: &str) -> Option<Kind> {
        Some(match s {
            "gpg" => Kind::Gpg,
            "sb" => Kind::SecureBoot,
            "w" => Kind::Wallet,
            _ => return None,
        })
    }

    /// Where Back from its flow lands when nothing before it on the way
    /// back is shown: Tools for the keys Tools starts, Home for a
    /// wallet's.
    pub(crate) fn left_for(self) -> crate::Screen {
        if self.is_wallet() {
            crate::Screen::Home
        } else {
            crate::Screen::Catalog
        }
    }

    /// The vault category its keys are listed in (`vault::CATEGORIES`).
    pub fn category(self) -> usize {
        match self {
            Kind::Gpg => 4,
            Kind::SecureBoot => 5,
            // A wallet's keys are listed under Keys.
            Kind::Wallet => 0,
        }
    }

    /// The kind whose keys a vault category lists, when the flow makes
    /// them.
    pub fn of_category(c: usize) -> Option<Kind> {
        match c {
            4 => Some(Kind::Gpg),
            5 => Some(Kind::SecureBoot),
            _ => None,
        }
    }

    /// The record type its keys are kept as.
    pub fn record_kind(self) -> u8 {
        match self {
            Kind::Gpg => rkind::GPG,
            Kind::SecureBoot => rkind::SECURE_BOOT,
            Kind::Wallet => rkind::KEY,
        }
    }

    /// The fewest places with a stick: two for a key that cannot be made
    /// again from seed words, one for a wallet (§1).
    pub fn min_places(self) -> usize {
        match self {
            Kind::Wallet => 1,
            _ => MIN_PLACES,
        }
    }

    /// A wallet's keys.
    pub fn is_wallet(self) -> bool {
        self == Kind::Wallet
    }

    /// Its name, for the flow's heading and Home's lead.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Gpg => crate::strings::EN.backup_flow_kind_gpg,
            Kind::SecureBoot => crate::strings::EN.backup_flow_kind_sb,
            Kind::Wallet => crate::strings::EN.backup_flow_kind_wallet,
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
    /// The stick is the medium the machine started from: its slot is
    /// Stick 1, the Boot stick (§18).
    pub boot: bool,
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
    /// The stick of files a place keeps: a wallet's seed files and public
    /// files kept there (`docs/BACKUP-FLOW.md` §15.2). Named after step 2
    /// and written at step 6 in its place's turn, as the stick of files is.
    PlaceFiles(usize),
    /// A further stick at place `p` like an earlier one there
    /// (`docs/BACKUP-FLOW.md` §17.4, §17.12 item 7): another stick of
    /// vault `v` (`Some(v)`), or another stick of files (`None`), the
    /// `n`th of its kind there counting from 0 (so `n` ≥ 1; the first is
    /// [`Target::PlaceVault`] or [`Target::PlaceFiles`]). A wallet's map
    /// may keep a vault on two sticks at one place, and files on two
    /// sticks; each is a stick of its own and a write of its own.
    Another(usize, Option<usize>, usize),
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
            Target::PlaceFiles(p) => format!("r{p}"),
            Target::Another(p, Some(v), n) => format!("a{p}.{v}.{n}"),
            Target::Another(p, None, n) => format!("a{p}.-.{n}"),
        }
    }

    pub(crate) fn from_code(s: &str) -> Option<Target> {
        match s {
            "f" => Some(Target::Files),
            "u" => Some(Target::Public),
            _ => {
                if let Some(rest) = s.strip_prefix('a') {
                    let mut it = rest.split('.');
                    let (p, v, n) = (it.next()?, it.next()?, it.next()?);
                    if it.next().is_some() {
                        return None;
                    }
                    let v = match v {
                        "-" => None,
                        v => Some(v.parse().ok()?),
                    };
                    let n: usize = n.parse().ok()?;
                    return (n >= 1).then_some(Target::Another(p.parse().ok()?, v, n));
                }
                if let Some(rest) = s.strip_prefix('r') {
                    return rest.parse().ok().map(Target::PlaceFiles);
                }
                if let Some(rest) = s.strip_prefix('q') {
                    let (p, v) = rest.split_once('.')?;
                    return Some(Target::PlaceVault(p.parse().ok()?, v.parse().ok()?));
                }
                s.strip_prefix('p')?.parse().ok().map(Target::Place)
            }
        }
    }

    /// The place whose stick of files it is.
    pub fn files_place(self) -> Option<usize> {
        match self {
            Target::PlaceFiles(p) | Target::Another(p, None, _) => Some(p),
            _ => None,
        }
    }

    /// The place whose stick it is: a vault's stick there. A place's stick
    /// of files is [`Target::files_place`].
    pub fn place(self) -> Option<usize> {
        match self {
            Target::Place(p) | Target::PlaceVault(p, _) | Target::Another(p, Some(_), _) => Some(p),
            _ => None,
        }
    }

    /// Whether it is a stick that carries a vault at a place.
    pub fn vault_stick(self) -> bool {
        self.place().is_some()
    }

    /// Sort key: places and their vaults in order, each vault's sticks
    /// there in order, the place's sticks of files after them, then the
    /// extras.
    pub(crate) fn key(self) -> (u8, usize, usize, usize) {
        match self {
            Target::Place(p) => (0, p, 0, 0),
            Target::PlaceVault(p, v) => (0, p, v + 1, 0),
            Target::Another(p, Some(v), n) => (0, p, v + 1, n),
            Target::PlaceFiles(p) => (0, p, usize::MAX, 0),
            Target::Another(p, None, n) => (0, p, usize::MAX, n),
            Target::Files => (1, 0, 0, 0),
            Target::Public => (2, 0, 0, 0),
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
    /// The plan names a stick of files: a wallet's seed files, or the
    /// wallet kept as files (§10.2).
    pub files: bool,
    /// The plan names a stick of public files of its own.
    pub public: bool,
    /// The places that keep a stick of files of their own
    /// (`docs/BACKUP-FLOW.md` §15.2).
    pub place_files: Vec<usize>,
    /// Which stick carries the public files.
    pub public_on: PublicOn,
    /// The stick of place `public_at` that carries the public files, where
    /// the plan names one: a wallet's stick there that keeps no secret
    /// file (`docs/BACKUP-FLOW.md` §10.2). None: the place's first stick.
    pub public_stick: Option<Target>,
}

/// Which stick carries a plan's public files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PublicOn {
    /// The stick of the place `public_at` names.
    #[default]
    Place,
    /// The stick of files.
    Files,
    /// A stick of public files of its own.
    Own,
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
            files: false,
            public: false,
            place_files: Vec::new(),
            public_on: PublicOn::Place,
            public_stick: None,
        }
    }

    /// The sticks the plan names besides its places', in order.
    pub fn extra_targets(&self) -> Vec<Target> {
        // A place named twice keeps two sticks of files.
        let mut out: Vec<Target> = Vec::new();
        for (k, &p) in self.place_files.iter().enumerate() {
            if p >= self.places.len() {
                continue;
            }
            let n = self.place_files[..k].iter().filter(|&&q| q == p).count();
            out.push(if n == 0 {
                Target::PlaceFiles(p)
            } else {
                Target::Another(p, None, n)
            });
        }
        if self.files {
            out.push(Target::Files);
        }
        if self.public {
            out.push(Target::Public);
        }
        out
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
                // A vault named twice at a place is kept there on two
                // sticks (a wallet's map, `docs/BACKUP-FLOW.md` §17.12
                // item 7).
                for (k, &v) in vs.iter().enumerate() {
                    let n = vs[..k].iter().filter(|&&u| u == v).count();
                    out.push(if n == 0 {
                        Target::PlaceVault(p, v)
                    } else {
                        Target::Another(p, Some(v), n)
                    });
                }
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

/// Costs what `of` costs. The flow's plan for a wallet, from the map
/// its answers draw (`docs/BACKUP-FLOW.md` §2, §10.2, §17.4): a place
/// for each place of the map, keeping a stick for each stick there that
/// carries a vault (a vault named once per stick of it, so a place may
/// keep one vault on two sticks); a vault for each vault number the map
/// names (numbered as the answers number them, a number no map uses
/// left empty); a place's stick of files for each stick there that
/// carries files and no vault; the stick of files where the map keeps
/// seeds or the wallet as files; and the stick that carries the public
/// files and sheets. Every stick of the map is one stick of the flow
/// ([`stick_target`]).
pub fn plan_of(c: &Compute, shape: &crate::plan::Shape, a: &crate::plan::Answers) -> Plan {
    plan_of_on(a, &crate::plan::Mapped::of(c, shape, a))
}

/// [`plan_of`] on the plan's map `m`.
pub fn plan_of_on(a: &crate::plan::Answers, m: &crate::plan::Mapped) -> Plan {
    use crate::plan::{At, What};
    let sticks = &m.map;
    // A place with its sticks folded in: what the place keeps.
    let boxes = &m.places;
    let mut places: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut place_files: Vec<usize> = Vec::new();
    let mut nv = 0usize;
    let mut with_wallet: Option<usize> = None;
    for s in boxes {
        match s.at {
            At::Place(p) => {
                let mut vs: Vec<usize> = Vec::new();
                for k in 0..crate::plan::sticks_at(sticks, p) {
                    match crate::plan::stick_vault(sticks, p, k) {
                        Some(v) => vs.push(v),
                        None => place_files.push(p),
                    }
                }
                nv = nv.max(vs.iter().map(|v| v + 1).max().unwrap_or(0));
                if !vs.is_empty()
                    && with_wallet.is_none()
                    && s.holds.iter().any(|(w, _)| *w == What::Wallet)
                {
                    with_wallet = Some(p);
                }
                places.push((p, vs));
            }
            At::Vault(v) => nv = nv.max(v + 1),
            _ => {}
        }
    }
    places.sort_by_key(|(p, _)| *p);
    let places: Vec<Vec<usize>> = places.into_iter().map(|(_, v)| v).collect();
    let files_spot = boxes.iter().find(|s| s.at == At::Files);
    let seed_files =
        files_spot.is_some_and(|s| s.holds.iter().any(|(w, _)| matches!(w, What::SeedFile(_))));
    let wallet_files = files_spot.is_some_and(|s| s.holds.iter().any(|(w, _)| *w == What::Wallet));
    let public_any = crate::plan::public_stick_on(a, m);
    // The stick of public files is read on an online computer: a secret
    // file never shares it, on the stick of files or at a place, so a plan
    // accepted at the lock can be written (`docs/BACKUP-FLOW.md` §10.2).
    // The public files go on the wallet's place's first stick that keeps
    // no secret file, else on the stick of files where it keeps the
    // wallet and no seed, else on a stick of their own. Public files
    // beside an encrypted vault copy are allowed.
    let clean_stick =
        with_wallet.and_then(|p| wallet_files::public_stick_at(sticks, p).map(|t| (p, t)));
    let (public_on, public_at, public_stick) = match clean_stick {
        Some((p, t)) if public_any => (PublicOn::Place, p, Some(t)),
        _ if wallet_files && !seed_files => (PublicOn::Files, 0, None),
        _ => (PublicOn::Own, 0, None),
    };
    Plan {
        places,
        vaults: vec![VaultPlan::default(); nv],
        together: Vec::new(),
        public_at,
        files: seed_files || (public_on == PublicOn::Files && public_any),
        public: public_on == PublicOn::Own && public_any,
        place_files,
        public_on,
        public_stick,
    }
}

/// The flow's stick for stick `s` at place `p` of a wallet's map
/// `sticks` (`docs/BACKUP-FLOW.md` §17.4), as [`plan_of`] names it: the
/// place's only stick with a vault, [`Target::Place`]; else its vault's
/// first stick there, [`Target::PlaceVault`], and a later one
/// [`Target::Another`]; a stick with no vault, the place's first stick of
/// files [`Target::PlaceFiles`] and a later one [`Target::Another`].
pub fn stick_target(sticks: &[crate::plan::Spot], p: usize, s: usize) -> Option<Target> {
    use crate::plan::{stick_vault, sticks_at};
    let n = sticks_at(sticks, p);
    if s >= n {
        return None;
    }
    let vault = stick_vault(sticks, p, s);
    let before = (0..s)
        .filter(|&k| stick_vault(sticks, p, k) == vault)
        .count();
    Some(match vault {
        Some(v) => {
            let vault_sticks = (0..n)
                .filter(|&k| stick_vault(sticks, p, k).is_some())
                .count();
            if vault_sticks <= 1 {
                Target::Place(p)
            } else if before == 0 {
                Target::PlaceVault(p, v)
            } else {
                Target::Another(p, Some(v), before)
            }
        }
        None if before == 0 => Target::PlaceFiles(p),
        None => Target::Another(p, None, before),
    })
}

/// The stick of a wallet's map `sticks` that the flow's stick `t` is
/// ([`stick_target`] read back); `None` for a stick no place keeps.
pub fn target_stick(sticks: &[crate::plan::Spot], t: Target) -> Option<crate::plan::At> {
    let p = match t {
        Target::Place(p) | Target::PlaceVault(p, _) | Target::PlaceFiles(p) => p,
        Target::Another(p, _, _) => p,
        Target::Files | Target::Public => return None,
    };
    (0..crate::plan::sticks_at(sticks, p))
        .find(|&s| stick_target(sticks, p, s) == Some(t))
        .map(|s| crate::plan::At::Stick(p, s))
}

/// What the key is known by once made (§3): the public value, as hex, and
/// the name it is shown under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The public value, hex: the GPG primary
    /// key's fingerprint, the Secure Boot owner GUID.
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
    /// An `FARADAYBOOT` partition.
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
    /// This file, written again once, still did not read back as it was
    /// meant (`docs/BACKUP-FLOW.md` §17.12 item 10).
    NoReadBack(String),
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
    /// This reading first bound the stick to its slot: its card says to
    /// mark it with its pill's number until it is pulled (§18).
    pub first: bool,
    /// The person pressed **Use {label}** for it: that press says which
    /// stick it is, so it is bound once read with no second tap, unless
    /// it reads like a stick bound to another place (§4.2).
    pub chosen: bool,
}

/// A write under way (step 6).
#[derive(Debug, Clone)]
pub struct Writing {
    /// The stick's id.
    pub id: String,
    /// The slot.
    pub slot: usize,
    /// The stick it is for: its result is counted for this target,
    /// wherever its slot is by then.
    pub target: Target,
    /// Files sent and not yet answered: the name sent and its SHA-256.
    pub pending: Vec<(String, [u8; 32])>,
    /// Files answered, read back and matched: the name written as.
    pub done: Vec<(String, [u8; 32])>,
    /// The files sent that are not vaults, by the name sent: each may be
    /// written again once when it does not read back (§19.12 item 10).
    pub plain: Vec<String>,
    /// The files written again, by the name the flow meant them under.
    pub retried: Vec<String>,
    /// A name a file was sent again under, with the name the flow meant
    /// it under.
    pub aliases: Vec<(String, String)>,
    /// Pictures written whose codes are being read back: the name written
    /// as, the name meant, and the file's SHA-256.
    pub qr: Vec<(String, String, [u8; 32])>,
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
    /// A wallet's plan card open, by [`crate::qstep`]
    /// (`docs/BACKUP-FLOW.md` §15).
    pub plan_q: Option<u8>,
    /// The board of the plan card: the chip held and how
    /// (`docs/BACKUP-FLOW.md` §17.12 item 1).
    pub board: crate::board::BoardState,
    /// The codex32 string the round's last **Copy the key** stopped on,
    /// not held since the power-on (`docs/CODEX32.md` §9.5): offered to
    /// be typed in place. Set by that press alone and cleared at the next,
    /// and once the string is typed.
    pub c32_wanted: Option<crate::plan::codex32::Wanted>,
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
    /// A wallet's part: its shape, its answers and what it holds back.
    pub wallet: Option<WalletFlow>,
    /// The sticks read for the vaults set up before the keys were made, by
    /// vault: each goes to the place the wallet's plan gives that vault's
    /// stick. Not kept across a lock, which wipes the open vaults too.
    pub parked: Vec<(usize, Bound)>,
    /// The screen's state.
    pub live: Live,
}

/// What a wallet's flow knows of the wallet (`docs/BACKUP-FLOW.md` §10).
/// The plan's answers are not secret, and are kept across a lock so that
/// **Change the plan** works at every step.
#[derive(Debug, Clone)]
pub struct WalletFlow {
    /// The wallet's shape: its quorum, its seeds by name.
    pub shape: crate::plan::Shape,
    /// The answers the plan is drawn from.
    pub answers: crate::plan::Answers,
    /// The wallet's descriptor checksum once the wallet exists.
    pub checksum: Option<String>,
    /// The wallet's name.
    pub name: String,
    /// The fingerprints, as text, of the keys that spend nothing until
    /// the last stick is written (F6's hold, moved here at the lock).
    pub held: Vec<String>,
    /// The seeds each vault of the plan was sealed with, by vault: what a
    /// plan changed after the lock is compared with (§4.8).
    pub sealed: Vec<(usize, Vec<usize>)>,
    /// What each vault held before this flow first put anything in it, by
    /// the vault's salt: `k{fp}` a key's record of its words alone,
    /// `p{fp}` one with its passphrase, `w{sum}` a
    /// wallet's description, `s{sum}:{n}` a wallet's share. The round
    /// takes none of it out (`docs/BACKUP-FLOW.md` §15). Never a secret.
    pub before: Vec<([u8; 32], Vec<String>)>,
    /// The keys, by fingerprint, another wallet's record in each vault
    /// names, by the vault's salt: the round takes none of them out.
    pub others: Vec<([u8; 32], Vec<String>)>,
    /// At the lock the plan kept the description, a sheet or a share, on
    /// paper or as files: whole or in shares, and the keys left off each
    /// share, are fixed from then on (`docs/BACKUP-FLOW.md` §15).
    pub pieces: bool,
    /// Each seed's fingerprint without its passphrase, as text, for the
    /// round to find a record of the same words; empty where unknown.
    pub base: Vec<String>,
    /// Which Make a new wallet or Restore the flow belongs to while the
    /// wallet is being made; not kept across a lock.
    pub owner: u64,
    /// The flow sets up the vaults of a wallet whose keys are not made
    /// yet: its plan is a vault at a place each, which the answers do not
    /// draw.
    pub setup: bool,
    /// What the Make a new wallet or Restore that asked for the setup was
    /// about, kept across the lock that makes the process clean: nothing
    /// secret, so the setup is picked up where it was.
    pub again: Option<Again>,
    /// Each picture the write step puts on a stick, by its name, with what
    /// it is read back against (`docs/BACKUP-FLOW.md` §17.12 item 10).
    /// Made at the lock, where the pictures are. Nothing in it is derived
    /// from a seed or a passphrase: a SeedQR is named by its key's
    /// fingerprint only.
    pub readback: Vec<(String, ReadBack)>,
}

/// What a picture written to a stick must read back as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadBack {
    /// A public picture (the description, a share, or a part of one): the
    /// SHA-256 of what its codes hold.
    Codes([u8; 32]),
    /// The SeedQR of the seed with this fingerprint (as text): its codes
    /// must hold the digits of the key's words while the key is loaded,
    /// and the picture must be the Outbox's byte for byte. Nothing of the
    /// seed is kept for it.
    Seed(String),
    /// The Codex32QR of the codex32 string at this share index (a bech32
    /// character, lower case) of the set of the key with this fingerprint
    /// (as text): its codes must hold the string while the key is loaded
    /// and holds it, and the picture must be the Outbox's byte for byte.
    /// Nothing of the string is kept for it.
    Codex32(String, u8),
}

/// The non-secret state of Make a new wallet or Restore, kept while its
/// vaults are set up across a lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Again {
    /// Restore, not Make a new wallet.
    pub restore: bool,
    /// The kind, written by its [`crate::create::NewKind::code`].
    pub kind: crate::create::NewKind,
    /// Signatures needed.
    pub m: usize,
    /// Keys.
    pub n: usize,
}

impl BackupFlow {
    /// Costs what `with_plan` costs. A new flow at step 1 with `kind`'s own
    /// plan.
    pub fn new(c: &Compute, kind: Kind) -> BackupFlow {
        BackupFlow::with_plan(c, kind, Plan::key_default())
    }

    /// Costs what `sync_slots` costs. A new flow at step 1 with this plan.
    /// Batch B1b makes a wallet's from its map.
    pub fn with_plan(c: &Compute, kind: Kind, plan: Plan) -> BackupFlow {
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
            wallet: None,
            parked: Vec::new(),
            live: Live::default(),
        };
        f.live.open = Some(Step::Where.n());
        f.sync_slots(c);
        f
    }

    /// Costs what `plan_of` and `with_wallet` cost. A new flow for a
    /// wallet, at step 1, with the plan its answers draw.
    pub fn new_wallet(
        c: &Compute,
        shape: crate::plan::Shape,
        answers: crate::plan::Answers,
        name: &str,
    ) -> BackupFlow {
        let plan = plan_of(c, &shape, &answers);
        BackupFlow::with_wallet(c, plan, shape, answers, name)
    }

    /// Costs what `with_wallet` costs. [`BackupFlow::new_wallet`] on the
    /// plan's map `m` of `shape` and `answers`, mapped already.
    pub fn new_wallet_on(
        c: &Compute,
        shape: crate::plan::Shape,
        answers: crate::plan::Answers,
        name: &str,
        m: &crate::plan::Mapped,
    ) -> BackupFlow {
        let plan = plan_of_on(&answers, m);
        BackupFlow::with_wallet(c, plan, shape, answers, name)
    }

    /// Costs what `with_wallet` costs. A new flow that sets up `count`
    /// vaults for a wallet whose keys are not made yet: it starts at step
    /// 2, with a stick for each vault, and asks nothing about places, paper
    /// or which key goes where.
    pub fn new_setup(
        c: &Compute,
        mut shape: crate::plan::Shape,
        answers: crate::plan::Answers,
        count: usize,
    ) -> BackupFlow {
        shape.vaults = count;
        let mut f = BackupFlow::with_wallet(c, BackupFlow::setup_plan(count), shape, answers, "");
        if let Some(w) = f.wallet.as_mut() {
            w.setup = true;
        }
        f.step = Step::Sticks;
        f.live.open = Some(Step::Sticks.n());
        f
    }

    /// Costs what `with_plan` costs.
    fn with_wallet(
        c: &Compute,
        plan: Plan,
        shape: crate::plan::Shape,
        answers: crate::plan::Answers,
        name: &str,
    ) -> BackupFlow {
        let mut f = BackupFlow::with_plan(c, Kind::Wallet, plan);
        f.wallet = Some(WalletFlow {
            shape,
            answers,
            checksum: None,
            name: name.to_string(),
            held: Vec::new(),
            sealed: Vec::new(),
            before: Vec::new(),
            others: Vec::new(),
            pieces: false,
            base: Vec::new(),
            owner: 0,
            setup: false,
            again: None,
            readback: Vec::new(),
        });
        f
    }

    /// What is kept by vault number follows the answers' rows: after
    /// [`crate::plan::Answers::refit`] renumbered them, vault `v` is what
    /// `keep[v]` was (`usize::MAX`: a new one). Called before the plan is
    /// drawn again.
    pub(crate) fn remap_vaults(&mut self, keep: &[usize]) {
        let old = std::mem::take(&mut self.plan.vaults);
        let new_len = keep.len().max(
            self.plan
                .places
                .iter()
                .flatten()
                .map(|v| v + 1)
                .max()
                .unwrap_or(0),
        );
        self.plan.vaults = (0..new_len)
            .map(|v| {
                keep.get(v)
                    .copied()
                    .and_then(|o| old.get(o).cloned())
                    .unwrap_or_default()
            })
            .collect();
        let to = |o: usize| keep.iter().position(|&k| k == o);
        for s in &mut self.slots {
            if let Some(b) = s.bound.as_mut() {
                b.picks = b
                    .picks
                    .iter()
                    .filter_map(|(v, p)| to(*v).map(|n| (n, p.clone())))
                    .collect();
                b.declined = b.declined.iter().filter_map(|&v| to(v)).collect();
            }
        }
        self.made_in = self.made_in.iter().filter_map(|&v| to(v)).collect();
    }

    /// Costs what `of` and `wallet_replan_on` cost. The plan follows the
    /// wallet's answers again, keeping what the flow knows of each vault
    /// (which file it is, its salt) and which places keep their vaults on
    /// one stick. A wallet not made yet has the plan of its setup (a vault
    /// at a place each), which its answers do not draw.
    pub(crate) fn wallet_replan(&mut self, c: &Compute) {
        let Some(w) = self.wallet.as_ref() else {
            return;
        };
        let m = crate::plan::Mapped::of(c, &w.shape, &w.answers);
        self.wallet_replan_on(c, &m);
    }

    /// Costs what `sync_slots` costs. [`BackupFlow::wallet_replan`] on its
    /// plan's map `m`, mapped already.
    pub(crate) fn wallet_replan_on(&mut self, c: &Compute, m: &crate::plan::Mapped) {
        let Some(w) = self.wallet.as_ref() else {
            return;
        };
        if self.is_setup() {
            return;
        }
        let mut plan = plan_of_on(&w.answers, m);
        // The vaults the flow knows stay, whether or not the plan names
        // them yet: the vaults set up before the keys are the plan's to
        // put seeds in.
        if plan.vaults.len() < self.plan.vaults.len() {
            plan.vaults
                .resize(self.plan.vaults.len(), VaultPlan::default());
        }
        for (v, vp) in plan.vaults.iter_mut().enumerate() {
            if let Some(old) = self.plan.vaults.get(v) {
                *vp = old.clone();
            }
        }
        plan.together = self
            .plan
            .together
            .iter()
            .copied()
            .filter(|&p| plan.places.get(p).is_some_and(|vs| vs.len() > 1))
            .collect();
        self.plan = plan;
        self.sync_slots(c);
        self.place_parked();
    }

    /// The wallet's keys are not made yet: the flow is the setup of its
    /// vaults, one stick and one vault each, before any key exists.
    pub fn is_setup(&self) -> bool {
        self.wallet.as_ref().is_some_and(|w| w.setup)
    }

    /// The vaults a wallet's setup made before its keys: the plan's while
    /// the flow is the setup, its first vaults once the wallet is made
    /// (the plan keeps them first, [`crate::plan::Shape::vaults`]).
    pub fn setup_vaults(&self) -> Vec<usize> {
        match self.wallet.as_ref() {
            Some(w) if !w.setup => (0..w.shape.vaults.min(self.plan.vaults.len())).collect(),
            _ => self.plan.used_vaults(),
        }
    }

    /// The sticks a wallet's setup read, and how many it has: the slots
    /// while the flow is the setup, the sticks it read, parked for the
    /// plan, once the wallet is made.
    pub fn setup_sticks_read(&self) -> (usize, usize) {
        match self.wallet.as_ref() {
            Some(w) if !w.setup => (self.parked.len(), w.shape.vaults),
            _ => (
                self.slots.iter().filter(|s| s.bound.is_some()).count(),
                self.slots.len(),
            ),
        }
    }

    /// The setup: `count` new vaults, each kept on a stick of its own.
    pub(crate) fn setup_plan(count: usize) -> Plan {
        Plan {
            places: (0..count).map(|v| vec![v]).collect(),
            vaults: vec![VaultPlan::default(); count],
            together: Vec::new(),
            public_at: 0,
            files: false,
            public: false,
            place_files: Vec::new(),
            public_on: PublicOn::Place,
            public_stick: None,
        }
    }

    /// Costs what `sync_slots` costs. The setup has another number of
    /// vaults: the last ones go, or new ones are added after the others,
    /// and every stick and vault already read or opened stays as it is.
    pub(crate) fn resize_setup(&mut self, c: &Compute, count: usize) {
        self.plan.places.truncate(count);
        self.plan.vaults.truncate(count);
        while self.plan.places.len() < count {
            let v = self.plan.places.len();
            self.plan.places.push(vec![v]);
            self.plan.vaults.push(VaultPlan::default());
        }
        if let Some(w) = self.wallet.as_mut() {
            w.shape.vaults = count;
            w.answers = crate::plan::Answers::defaults(&w.shape);
            w.answers.custom = true;
        }
        self.made_in.retain(|&v| v < count);
        self.sync_slots(c);
        for s in &mut self.slots {
            if let Some(b) = s.bound.as_mut() {
                b.picks.retain(|(v, _)| *v < count);
            }
        }
        self.step = Step::Sticks;
        self.live.open = Some(Step::Sticks.n());
    }

    /// The sticks read in the setup are kept by vault, so that the plan
    /// the wallet's backup draws puts each where its vault's stick goes.
    pub(crate) fn park_setup(&mut self) {
        self.parked = self
            .slots
            .iter()
            .filter_map(|s| match (s.target, s.bound.as_ref()) {
                (Target::Place(p), Some(b)) => self
                    .plan
                    .places
                    .get(p)
                    .and_then(|vs| vs.first())
                    .map(|&v| (v, b.clone())),
                _ => None,
            })
            .collect();
    }

    /// Each stick read in the setup is bound to the first stick the plan
    /// gives its vault, unless that stick is read already or written. A
    /// stick the plan no longer gives its vault there is let go.
    fn place_parked(&mut self) {
        if self.parked.is_empty() {
            return;
        }
        let parked = self.parked.clone();
        for s in &mut self.slots {
            if s.written.is_none() && parked.iter().any(|(_, b)| s.bound.as_ref() == Some(b)) {
                s.bound = None;
            }
        }
        for (v, b) in parked {
            let Some(&p) = self.plan.places_of(v).first() else {
                continue;
            };
            let target = if self.plan.one_stick(p) {
                Target::Place(p)
            } else {
                Target::PlaceVault(p, v)
            };
            let taken = self.slots.iter().any(|s| s.bound.as_ref() == Some(&b));
            if taken {
                continue;
            }
            if let Some(s) = self
                .slots
                .iter_mut()
                .find(|s| s.target == target && s.bound.is_none() && s.written.is_none())
            {
                s.bound = Some(b);
            }
        }
    }

    /// Costs what `seed_file_ok` costs. The slots follow the plan: a stick
    /// per place that keeps one (or per vault of a place that keeps several
    /// apart), in order, then the extras. A place that left takes its slot,
    /// and its public files go to the first stick.
    pub(crate) fn sync_slots(&mut self, c: &Compute) {
        if self.plan.public_at >= self.plan.places.len() {
            self.plan.public_at = 0;
        }
        let want = self.plan.place_targets();
        let extras = self.plan.extra_targets();
        let mut orphaned: Vec<String> = Vec::new();
        self.slots.retain(|s| {
            let keep = match s.target {
                Target::Place(_) | Target::PlaceVault(..) | Target::Another(_, Some(_), _) => {
                    want.contains(&s.target)
                }
                Target::Files
                | Target::Public
                | Target::PlaceFiles(_)
                | Target::Another(_, None, _) => extras.contains(&s.target),
            };
            if !keep {
                orphaned.extend(s.files.iter().cloned());
            }
            keep
        });
        for t in want.into_iter().chain(extras) {
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
        // A seed's file goes nowhere but its own stick of files: a stick
        // that leaves the plan takes it off the plan with it.
        orphaned.retain(|n| !self.seed_file_ok(c, n));
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

    /// The slot that carries the key's public files: the stick of files or
    /// the stick of public files where the plan says so, else the stick of
    /// the place the plan names for them, else that place's first stick,
    /// else the first.
    pub(crate) fn public_slot(&self) -> Option<usize> {
        let by = |t: Target| self.slots.iter().position(|s| s.target == t);
        match self.plan.public_on {
            PublicOn::Files => return by(Target::Files),
            PublicOn::Own => return by(Target::Public),
            PublicOn::Place => {}
        }
        if let Some(k) = self
            .plan
            .public_stick
            .filter(|t| t.place().or(t.files_place()) == Some(self.plan.public_at))
            .and_then(by)
        {
            return Some(k);
        }
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

    /// Slot `k` is held by the stick the machine started from.
    pub fn boot_holds(&self, k: usize) -> bool {
        self.slots
            .get(k)
            .and_then(|s| s.bound.as_ref())
            .is_some_and(|b| b.boot)
    }

    /// The number of slot `k`'s stick, as its pill and every line naming
    /// it read (§18): 1 where the boot stick holds it, else from 2 in slot
    /// order, skipping the slots the boot stick holds.
    pub fn stick_number(&self, k: usize) -> usize {
        if self.boot_holds(k) {
            return 1;
        }
        2 + (0..k).filter(|&j| !self.boot_holds(j)).count()
    }

    /// How many of the slots are sticks of their own: the boot stick's
    /// not counted.
    pub fn own_sticks(&self) -> usize {
        (0..self.slots.len())
            .filter(|&k| !self.boot_holds(k))
            .count()
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
            Some(Target::PlaceVault(_, v) | Target::Another(_, Some(v), _)) => vec![v],
            _ => Vec::new(),
        }
    }

    /// Which copy of vault `v` the stick of `slot` takes: 0 the first. The
    /// sticks that carry it are its copies in slot order: one per place
    /// that keeps it, or more where a place keeps it on several sticks.
    pub(crate) fn copy_number(&self, v: usize, slot: usize) -> usize {
        if self
            .slots
            .get(slot)
            .and_then(|s| s.target.place())
            .is_none()
        {
            return 0;
        }
        (0..slot.min(self.slots.len()))
            .filter(|&k| self.vaults_of(k).contains(&v))
            .count()
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
    hex_text(h)
}

/// A 32-byte value from 64 hex digits.
pub(crate) fn hex_array(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The kept state's version: a flow written by another version is not
/// read.
const VERSION: &str = "3";
/// The most places, vaults and sticks a kept flow may name; more is a
/// corrupt state.
const MAX_PLACES: usize = 10_000;
const MAX_SLOTS: usize = 20_000;

/// The flows as the kept state holds them: a version line, then a line
/// each, split by tabs. Nothing in it is secret: names, sizes, salts,
/// hashes, labels and public values.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
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
            "plan\t{}\t{}\t{}\t{}\t{}\n",
            f.plan.public_at,
            together.join(","),
            u8::from(f.plan.files),
            u8::from(f.plan.public),
            match f.plan.public_on {
                PublicOn::Place => "p",
                PublicOn::Files => "f",
                PublicOn::Own => "o",
            },
        ));
        if !f.plan.place_files.is_empty() {
            let list: Vec<String> = f.plan.place_files.iter().map(|p| p.to_string()).collect();
            out.push_str(&format!("pfiles\t{}\n", list.join(",")));
        }
        if let Some(t) = f.plan.public_stick {
            out.push_str(&format!("pstick\t{}\n", t.code()));
        }
        if let Some(w) = &f.wallet {
            out.push_str(&format!(
                "wallet\t{}\t{}\t{}\n",
                clean(w.checksum.as_deref().unwrap_or("")),
                clean(&w.name),
                w.held.join(",")
            ));
            if !w.base.is_empty() {
                out.push_str(&format!("wbase\t{}\n", w.base.join(",")));
            }
            for (v, seeds) in &w.sealed {
                let list: Vec<String> = seeds.iter().map(|s| s.to_string()).collect();
                out.push_str(&format!("wsealed\t{v}\t{}\n", list.join(",")));
            }
            if w.pieces {
                out.push_str("wpieces\t1\n");
            }
            for (name, back) in &w.readback {
                match back {
                    ReadBack::Codes(sha) => {
                        out.push_str(&format!("wqr\t{}\t{}\n", clean(name), hex_text(sha)));
                    }
                    ReadBack::Seed(fp) => {
                        out.push_str(&format!("wqs\t{}\t{}\n", clean(name), clean(fp)));
                    }
                    ReadBack::Codex32(fp, index) => {
                        out.push_str(&format!(
                            "wqc\t{}\t{}\t{}\n",
                            clean(name),
                            clean(fp),
                            char::from(*index)
                        ));
                    }
                }
            }
            for (salt, held) in &w.before {
                out.push_str(&format!(
                    "wbefore\t{}\t{}\n",
                    hex_text(salt),
                    held.join(",")
                ));
            }
            for (salt, fps) in &w.others {
                out.push_str(&format!("wothers\t{}\t{}\n", hex_text(salt), fps.join(",")));
            }
            out.push_str(&format!(
                "wshape\t{}\t{}\t{}\t{}\n",
                w.shape.m,
                w.shape.keys,
                u8::from(w.shape.splits),
                w.shape.vaults
            ));
            for sd in &w.shape.seeds {
                out.push_str(&format!(
                    "wseed\t{}\t{}\t{}\n",
                    clean(&sd.name),
                    u8::from(sd.here),
                    u8::from(sd.passphrase)
                ));
            }
            if w.setup {
                out.push_str("wsetup\t1\n");
            }
            if let Some(a) = &w.again {
                out.push_str(&format!(
                    "wagain\t{}\t{}\t{}\t{}\n",
                    u8::from(a.restore),
                    a.kind.code(),
                    a.m,
                    a.n
                ));
            }
            out.push_str(&format!(
                "wans\t{}\n",
                clean(&w.answers.to_text().replace('\n', "|"))
            ));
        }
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
                if b.boot {
                    out.push_str(&format!("bootstick\t{k}\n"));
                }
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

/// Costs what `valid` costs. Read back what [`encode`] kept, and how
/// many flows were dropped: a state of another version, one that does
/// not read, or one whose parts do not agree (an index out of range, a
/// stick that the plan does not name, a step past the key with nothing
/// sealed) is dropped whole.
pub fn decode(c: &Compute, bytes: &[u8]) -> (Vec<BackupFlow>, usize) {
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
                        wallet: None,
                        parked: Vec::new(),
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
            if !f.valid(c) {
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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn read_line(fl: &mut BackupFlow, f: &[&str], hex_array: fn(&str) -> Option<[u8; 32]>) -> bool {
    let slot = |fl: &mut BackupFlow, k: &str| -> Option<usize> {
        let k: usize = k.parse().ok()?;
        (k < fl.slots.len()).then_some(k)
    };
    match *f {
        ["plan", public_at, together, files, public, on] => {
            let Ok(p) = public_at.parse() else {
                return false;
            };
            fl.plan.public_at = p;
            fl.plan.together = together
                .split(',')
                .filter(|x| !x.is_empty())
                .filter_map(|x| x.parse().ok())
                .collect();
            fl.plan.files = files == "1";
            fl.plan.public = public == "1";
            fl.plan.public_on = match on {
                "f" => PublicOn::Files,
                "o" => PublicOn::Own,
                _ => PublicOn::Place,
            };
            true
        }
        ["pstick", t] => {
            fl.plan.public_stick = Target::from_code(t);
            fl.plan.public_stick.is_some()
        }
        ["pfiles", list] => {
            let Some(places) = list
                .split(',')
                .map(|x| x.parse().ok())
                .collect::<Option<Vec<usize>>>()
            else {
                return false;
            };
            fl.plan.place_files = places;
            true
        }
        ["wallet", checksum, name, held] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.checksum = (!checksum.is_empty()).then(|| checksum.to_string());
            w.name = name.to_string();
            w.held = held
                .split(',')
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            true
        }
        ["wbase", list] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.base = list.split(',').map(str::to_string).collect();
            true
        }
        ["wqr", name, sha] => {
            let Some(sha) = hex_array(sha) else {
                return false;
            };
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.readback.push((name.to_string(), ReadBack::Codes(sha)));
            true
        }
        ["wqs", name, fp] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.readback
                .push((name.to_string(), ReadBack::Seed(fp.to_string())));
            true
        }
        ["wqc", name, fp, index] => {
            let [index] = index.as_bytes() else {
                return false;
            };
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.readback
                .push((name.to_string(), ReadBack::Codex32(fp.to_string(), *index)));
            true
        }
        ["wpieces", on] => {
            fl.wallet.get_or_insert_with(WalletFlow::blank).pieces = on == "1";
            true
        }
        ["wbefore" | "wothers", salt, list] => {
            let Some(salt) = hex_array(salt) else {
                return false;
            };
            let items: Vec<String> = list
                .split(',')
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            if f[0] == "wbefore" {
                w.before.push((salt, items));
            } else {
                w.others.push((salt, items));
            }
            true
        }
        ["wsealed", v, seeds] => {
            let Ok(v) = v.parse::<usize>() else {
                return false;
            };
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.sealed.push((
                v,
                seeds
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .filter_map(|x| x.parse().ok())
                    .collect(),
            ));
            true
        }
        ["wshape", m, keys, splits, vaults] => {
            let (Ok(m), Ok(keys), Ok(vaults)) = (m.parse(), keys.parse(), vaults.parse()) else {
                return false;
            };
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.shape.m = m;
            w.shape.keys = keys;
            w.shape.splits = splits == "1";
            w.shape.vaults = vaults;
            true
        }
        ["wseed", name, here, pass] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.shape.seeds.push(crate::plan::Seed {
                name: name.to_string(),
                here: here == "1",
                passphrase: pass == "1",
            });
            true
        }
        ["wagain", restore, kind, m, n] => {
            let (Ok(kind), Ok(m), Ok(n)) = (kind.parse::<u8>(), m.parse(), n.parse()) else {
                return false;
            };
            let Some(kind) = crate::create::NewKind::from_code(kind) else {
                return false;
            };
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.again = Some(Again {
                restore: restore == "1",
                kind,
                m,
                n,
            });
            true
        }
        ["wsetup", on] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            w.setup = on == "1";
            true
        }
        ["wans", text] => {
            let w = fl.wallet.get_or_insert_with(WalletFlow::blank);
            let text = text.replace('|', "\n");
            match crate::plan::Answers::from_text(&w.shape, &text) {
                Some(a) => {
                    w.answers = a;
                    true
                }
                None => false,
            }
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
                        boot: false,
                    });
                    true
                }
                None => false,
            }
        }
        ["bootstick", k] => match slot(fl, k)
            .and_then(|k| fl.slots.get_mut(k))
            .and_then(|s| s.bound.as_mut())
        {
            Some(b) => {
                b.boot = true;
                true
            }
            None => false,
        },
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

impl WalletFlow {
    /// An empty wallet part, filled by the lines that follow.
    pub(crate) fn blank() -> WalletFlow {
        WalletFlow {
            shape: crate::plan::Shape {
                m: 1,
                keys: 1,
                seeds: Vec::new(),
                splits: false,
                vaults: 0,
            },
            answers: crate::plan::Answers::default(),
            checksum: None,
            name: String::new(),
            held: Vec::new(),
            sealed: Vec::new(),
            before: Vec::new(),
            others: Vec::new(),
            pieces: false,
            base: Vec::new(),
            owner: 0,
            setup: false,
            again: None,
            readback: Vec::new(),
        }
    }
}

impl BackupFlow {
    /// Costs what `sync_slots` costs. Whether what was read back agrees
    /// with itself, and brings the slots to the plan. A flow past the key
    /// must have made it and sealed a copy of each vault.
    fn valid(&mut self, c: &Compute) -> bool {
        // A made wallet flow with no wallet part, or none that names the
        // wallet, is not one: its gate entry holds its keys instead.
        if self.kind.is_wallet()
            && (self.wallet.is_none()
                || (self.made && self.wallet.as_ref().is_some_and(|w| w.checksum.is_none())))
        {
            return false;
        }
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
        self.sync_slots(c);
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
            let held = f
                .wallet
                .as_ref()
                .map_or(String::new(), |w| w.held.join(","));
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                f.kind.code(),
                s.id,
                clean(&s.name),
                held
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

/// Costs a lookup a record. The place of the record of `subject` in open
/// vault `o` of `vaults`, to copy it across.
pub(crate) fn record_in(
    vaults: &crate::vault::Vaults,
    o: usize,
    kind: Kind,
    subject: &str,
) -> Option<usize> {
    let open = vaults.open.get(o)?;
    vaults
        .records(open.id)
        .iter()
        .find(|(_, r)| {
            r.kind() == kind.record_kind() && subject_of(kind, *r).as_deref() == Some(subject)
        })
        .map(|(i, _)| i)
}

/// Bytes as lower-case hex.
pub(crate) fn hex_text(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The public value a record of `kind` is known by, as hex.
pub(crate) fn subject_of(kind: Kind, r: RecordView<'_>) -> Option<String> {
    Some(match kind {
        Kind::Gpg => hex_text(&faraday_keys::vault::gpg_facts(r)?.fingerprint),
        Kind::SecureBoot => hex_text(r.public(crate::secure_boot::OWNER_FIELD)?),
        // A wallet is known by its descriptor's checksum, not by a record.
        Kind::Wallet => return None,
    })
}

/// Costs a picture. The SeedQR `mx`, of a seed of `words` words, as a
/// labelled picture titled with its fingerprint `fp`.
pub(crate) fn seedqr_picture(
    c: &Compute,
    mx: &faraday_codec::qr::QrMatrix,
    words: usize,
    fp: &str,
) -> zeroize::Zeroizing<Vec<u8>> {
    let title = crate::strings::fill(crate::strings::EN.secrets_seed, &[fp]);
    let lines = [crate::strings::fill(
        crate::strings::EN.secrets_words_secret,
        &[&words.to_string()],
    )];
    zeroize::Zeroizing::new(crate::picture::labelled_png(c, mx, &title, &lines))
}

/// The name seed form `what` of the seed with fingerprint `fp` is written
/// as on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6): `seed-{fp}.txt`,
/// `seedqr-{fp}.png`, `passphrase-{fp}.txt`, the fingerprint in lower case.
pub(crate) fn seed_stick_name(what: crate::plan::What, fp: &str) -> Option<String> {
    let fp = fp.to_lowercase();
    match what {
        crate::plan::What::SeedFile(_) => Some(format!("seed-{fp}.txt")),
        crate::plan::What::SeedQrFile(_) => Some(format!("seedqr-{fp}.png")),
        crate::plan::What::PassphraseFile(_) => Some(format!("passphrase-{fp}.txt")),
        _ => None,
    }
}

/// Whether `name` is a file on a stick of one of the seeds `fps`, in any
/// form ([`seed_stick_name`]).
pub(crate) fn seed_stick_file_of(fps: &[String], name: &str) -> bool {
    fps.iter().any(|fp| {
        [
            crate::plan::What::SeedFile(0),
            crate::plan::What::SeedQrFile(0),
            crate::plan::What::PassphraseFile(0),
        ]
        .into_iter()
        .any(|w| seed_stick_name(w, fp).as_deref() == Some(name))
    })
}

// ---- what a stick is called ----

impl BackupFlow {
    /// "Vault 1 (Place 1)", "the stick of files".
    pub fn target_short(&self, t: &Target, medium: &str) -> String {
        if self.is_setup() {
            return self.target_name(t, medium);
        }
        match t {
            Target::Place(p) => {
                let vs = self.plan.places.get(*p).cloned().unwrap_or_default();
                let names: Vec<String> = vs.iter().map(|&v| BackupFlow::vault_name(v)).collect();
                format!("{} ({})", names.join(", "), BackupFlow::place_name(*p))
            }
            Target::PlaceVault(p, v) | Target::Another(p, Some(v), _) => format!(
                "{} ({})",
                BackupFlow::vault_name(*v),
                BackupFlow::place_name(*p)
            ),
            t => self.target_name(t, medium),
        }
    }

    /// What a stick is for, with no "the stick": "Vault 1 (Place 1)",
    /// "Files", "Public files"; after its number on a device (§18).
    pub fn target_what(&self, t: &Target, medium: &str) -> String {
        match t {
            t if t.vault_stick() && self.is_setup() => self.target_vaults(t),
            t if t.vault_stick() => self.target_short(t, medium),
            Target::Files => FEN.backup_flow_for_files.to_string(),
            Target::Public => FEN.backup_flow_for_public.to_string(),
            _ => self.target_name(t, medium),
        }
    }

    /// "the stick for Vault 1 (Place 1)", "the stick of files".
    pub fn target_name(&self, t: &Target, medium: &str) -> String {
        match t {
            // Before the keys, a stick is a vault's: no place is asked.
            t if t.vault_stick() && self.is_setup() => fill(
                FEN.backup_flow_stick_for_vault,
                &[medium, &self.target_vaults(t)],
            ),
            t if t.vault_stick() => fill(
                FEN.backup_flow_vault_stick,
                &[medium, &self.target_vaults(t), &self.target_place(t)],
            ),
            Target::Files => fill(FEN.backup_flow_files_stick, &[medium]),
            Target::Public => fill(FEN.backup_flow_public_stick, &[medium]),
            // A place's sticks of files, each named alike: the pill's
            // number tells them apart (§18).
            Target::PlaceFiles(p) | Target::Another(p, None, _) => {
                fill(FEN.backup_flow_place_files, &[&BackupFlow::place_name(*p)])
            }
            _ => String::new(),
        }
    }

    fn target_vaults(&self, t: &Target) -> String {
        match t {
            Target::Place(p) => self
                .plan
                .places
                .get(*p)
                .map(|vs| {
                    vs.iter()
                        .map(|&v| BackupFlow::vault_name(v))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            Target::PlaceVault(_, v) | Target::Another(_, Some(v), _) => BackupFlow::vault_name(*v),
            _ => String::new(),
        }
    }

    fn target_place(&self, t: &Target) -> String {
        t.place().map(BackupFlow::place_name).unwrap_or_default()
    }
}
