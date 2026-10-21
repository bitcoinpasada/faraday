//! The backup's plan (`docs/WALLETS.md` §5): the answers a person gives
//! (where the seeds go, how many places keep paper, where the wallet
//! description goes, for which software), the map they draw (what each
//! place keeps on paper and on each of its sticks, each vault, a stick of
//! files and the watch-only software hold), the check computed from it,
//! and the checklist of only what it needs.
//!
//! The check is found the way [`crate::backup::audit`] finds what a split
//! achieves: by trying each place lost and each place found, not by
//! trusting arithmetic. Nothing here knows a place's name: names live in
//! the vault alone and are put on screen by the caller.

use osk_crypto::work::Compute;

use crate::backup::split_plan;
use crate::strings::EN;
use crate::strings::fill;

/// Where the seeds go, by row of the seeds question.
pub mod seeds {
    /// On paper, the words by hand.
    pub const WORDS: usize = 0;
    /// On paper, the SeedQR by hand.
    pub const SEEDQR: usize = 1;
    /// Into vaults, a vault per seed unless more are ticked into one.
    pub const VAULT: usize = 2;
    /// As a file, unprotected.
    pub const FILE: usize = 3;
}

/// Where the wallet description goes, by row of its question.
pub mod wallet {
    /// A printed sheet, or share, in each place.
    pub const PAPER: usize = 0;
    /// Into every vault made.
    pub const VAULT: usize = 1;
    /// Into watch-only software.
    pub const SOFTWARE: usize = 2;
    /// As files on a stick.
    pub const FILES: usize = 3;
}

/// The watch-only software, by row of its question.
pub mod software {
    /// Sparrow.
    pub const SPARROW: usize = 0;
    /// Coldcard, Keystone, Passport.
    pub const HARDWARE: usize = 1;
    /// Nunchuk.
    pub const NUNCHUK: usize = 2;
    /// Bitcoin Core.
    pub const CORE: usize = 3;
    /// Not sure: the descriptor, which most read.
    pub const NOT_SURE: usize = 4;
}

/// The form the public files take, by row.
pub mod form {
    /// A QR picture.
    pub const QR: usize = 0;
    /// Text.
    pub const TEXT: usize = 1;
}

/// The plans a backup once started from, kept as plan builders
/// (`docs/BACKUP-FLOW.md` §19.21, F53): The standard way is the plan a new
/// backup opens on; the others are offered nowhere, and are what a saved
/// plan's `preset` line and the tests' plans by presses
/// (`testkit::backup_as`) name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// The standard way (`docs/BACKUP-FLOW.md` §19.6, §19.21 item 12):
    /// paper at one place for a single key, at a place per seed for a
    /// multisig, each holding its seed's words and the whole description;
    /// no SeedQR, no vault, no stick. The board opens on it for a new
    /// plan.
    Standard,
    /// Paper in each place: since §19.21 item 12 the same plan as
    /// [`Preset::Standard`], which a saved plan's `preset 1` named.
    Paper,
    /// Paper, and the seeds and the wallet in the vault.
    PaperVault,
    /// Paper, the vault, and the wallet in watch-only software.
    PaperVaultSoftware,
}

impl Preset {
    /// Every preset, The standard way first.
    pub const ALL: [Preset; 4] = [
        Preset::Standard,
        Preset::Paper,
        Preset::PaperVault,
        Preset::PaperVaultSoftware,
    ];
}

/// One seed of the wallet, as the plan sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    /// Its fingerprint, or the share's number.
    pub name: String,
    /// Loaded here from words: it can go on paper, into the vault or a
    /// file. A seed not here is on its own device.
    pub here: bool,
    /// Loaded with a BIP-39 passphrase, which has its own places.
    pub passphrase: bool,
}

/// What the plan is for: the wallet's quorum, its seeds, and whether its
/// description splits into shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// Signatures needed.
    pub m: usize,
    /// Keys in the description, which the shares split.
    pub keys: usize,
    /// The seeds, one each.
    pub seeds: Vec<Seed>,
    /// The description splits into shares (a multisig).
    pub splits: bool,
    /// How many vaults were set up before the keys were made
    /// (`docs/BACKUP-FLOW.md` §10.2): the plan has at least this many
    /// vaults to put seeds in, however many seeds there are.
    pub vaults: usize,
}

impl Shape {
    /// No seed of it is here: nothing to ask about seeds.
    pub fn watch_only(&self) -> bool {
        !self.seeds.iter().any(|s| s.here)
    }

    /// More than one seed signs.
    pub fn multi(&self) -> bool {
        self.seeds.len() > 1
    }

    /// The seeds loaded here, by place among the wallet's.
    pub fn here(&self) -> Vec<usize> {
        (0..self.seeds.len())
            .filter(|&i| self.seeds[i].here)
            .collect()
    }

    /// How many vaults the plan has a section for: one per seed loaded
    /// here, and one for the description alone when none is; never fewer
    /// than the vaults set up before the keys.
    pub fn vault_rows(&self) -> usize {
        self.here().len().max(1).max(self.vaults)
    }
}

/// The answers to the plan's questions. Each list is a multi-choice:
/// more than one row may be ticked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answers {
    /// Where the seeds go, by [`seeds`] row.
    pub seeds: [bool; 4],
    /// How many places keep paper.
    pub places: usize,
    /// For a multisig: each place its own share, rather than the whole
    /// wallet sheet.
    pub split: bool,
    /// For vault `v`, which seeds it holds, a flag per seed of the
    /// wallet: one vault per seed loaded here.
    pub vaults: Vec<Vec<bool>>,
    /// For vault `v`, which places keep a stick with it on: a row per
    /// vault of [`Answers::vaults`], at least one.
    pub sticks: Vec<Vec<bool>>,
    /// Where the wallet description goes, by [`wallet`] row.
    pub wallet: [bool; 4],
    /// The watch-only software, by [`software`] row.
    pub software: [bool; 5],
    /// The public files' form, by [`form`] row.
    pub form: [bool; 2],
    /// For each seed with a passphrase, where the passphrase is kept: a
    /// flag per place, then one for the vault. Empty for other seeds.
    pub pass: Vec<Vec<bool>>,
    /// Keys left off each share.
    pub omit: usize,
    /// The map as edited on the wallet's chart (`docs/NEW-WALLET.md`
    /// §9.7): from the first edit there, the plan itself, which the rest
    /// of the answers no longer spread by rule. `None` while the answers
    /// draw it.
    pub map: Option<Vec<Spot>>,
    /// Places that keep no paper copy of a seed: the place a default
    /// gives a passphrase when every other place keeps a copy of its seed
    /// (`docs/BACKUP-FLOW.md` §10.3).
    pub bare: Vec<usize>,
    /// The board has been opened for changes by **Customize backup**
    /// (§19.6): until then it shows the placement read-only. Kept with
    /// the plan, so a plan customised once reopens customisable.
    pub custom: bool,
    /// The SeedQR copies dropped for the words that stay on the map,
    /// drawn in part (`docs/NEW-WALLET.md` §14.7 item 3), as (place,
    /// seed): each a secret at its place for "One place found", never a
    /// copy that rebuilds, until I destroyed this copy takes it off.
    pub part: Vec<(usize, usize)>,
}

impl Default for Answers {
    /// Paper only, in one place, for a wallet not yet known.
    fn default() -> Answers {
        Answers {
            seeds: [true, false, false, false],
            places: 1,
            split: false,
            vaults: Vec::new(),
            sticks: vec![vec![false]],
            wallet: [true, false, false, false],
            software: [false, false, false, false, true],
            form: [true, false],
            pass: Vec::new(),
            omit: 0,
            map: None,
            bare: Vec::new(),
            custom: false,
            part: Vec::new(),
        }
    }
}

impl Answers {
    /// The wallet's defaults: one key, paper words, one place (the most a
    /// single key has, `docs/BACKUP-FLOW.md` §19.19), the sheet there, the descriptor as a QR picture; a multisig, a
    /// place per seed, each with its seed and the whole wallet sheet; no
    /// seed here, no seed on paper. Should the seeds go into vaults, a
    /// vault per seed here, vault `v` holding the `v`th.
    pub fn defaults(shape: &Shape) -> Answers {
        let places = if shape.multi() { shape.seeds.len() } else { 1 };
        let mut a = Answers {
            seeds: [!shape.watch_only(), false, false, false],
            places,
            split: false,
            vaults: shape
                .here()
                .into_iter()
                .map(|i| (0..shape.seeds.len()).map(|j| j == i).collect())
                .chain((shape.here().len()..shape.vaults).map(|_| vec![false; shape.seeds.len()]))
                .collect(),
            sticks: vec![vec![false; places]; shape.vault_rows()],
            wallet: [true, false, false, false],
            software: [false, false, false, false, true],
            form: [true, false],
            pass: Vec::new(),
            omit: shape.m.saturating_sub(1),
            map: None,
            bare: Vec::new(),
            custom: false,
            part: Vec::new(),
        };
        a.pass = shape
            .seeds
            .iter()
            .map(|s| {
                if s.here && s.passphrase {
                    vec![false; places + 1]
                } else {
                    Vec::new()
                }
            })
            .collect();
        a.place_passphrases(shape);
        a
    }

    /// Costs a map of the plan (through `map`). The board's start
    /// (`docs/BACKUP-FLOW.md` §19.6): one empty place, nothing kept; the
    /// seeds not here on their own devices, as every plan has them. The
    /// rest of the answers are the wallet's defaults, which a preset reads.
    pub fn empty(c: &Compute, shape: &Shape) -> Answers {
        let mut a = Answers::defaults(shape);
        let away: Vec<Spot> = map(c, shape, &a)
            .into_iter()
            .filter(|s| s.at == At::Away)
            .collect();
        let mut m = vec![Spot {
            at: At::Place(0),
            holds: Vec::new(),
        }];
        m.extend(away);
        a.set_map(m);
        a
    }

    /// Whether place `p` keeps a copy of seed `i` in any form: its words
    /// or SeedQR on paper, or the stick of a vault that holds it.
    pub fn seed_at(&self, shape: &Shape, i: usize, p: usize) -> bool {
        self.words_at(shape, p).contains(&i)
            || self
                .vaults_made(shape)
                .into_iter()
                .any(|v| self.stick_at(v, p) && self.vault_seeds(shape, v).contains(&i))
    }

    /// Puts each seed's passphrase at the first place that keeps no copy
    /// of that seed in any form, its vault's stick included; where there
    /// is none, a place is added for it (`docs/BACKUP-FLOW.md` §10.3). A
    /// passphrase the plan already puts somewhere stays.
    pub fn place_passphrases(&mut self, shape: &Shape) {
        self.place_passphrases_up_to(shape, max_places(shape));
    }

    /// [`Answers::place_passphrases`], adding no place past `most`.
    fn place_passphrases_up_to(&mut self, shape: &Shape, most: usize) {
        for i in 0..shape.seeds.len() {
            if !(shape.seeds[i].here && shape.seeds[i].passphrase) {
                continue;
            }
            if self.pass.len() <= i {
                self.pass.resize(i + 1, Vec::new());
            }
            if self.pass[i].is_empty() {
                self.pass[i] = vec![false; self.places + 1];
            }
            if self.pass[i].iter().take(self.places).any(|&b| b) {
                continue;
            }
            let free = (0..self.places).find(|&p| !self.seed_at(shape, i, p));
            // At the most places a plan has (§19.19), the passphrase goes
            // into the vault where the seeds go into one, else beside its
            // seed, which the board warns of.
            let full = self.places >= most;
            if free.is_none() && full {
                let vault = self.seeds[seeds::VAULT];
                let row = &mut self.pass[i];
                match row.last_mut() {
                    Some(v) if vault => *v = true,
                    _ => {
                        if let Some(first) = row.first_mut() {
                            *first = true;
                        }
                    }
                }
                continue;
            }
            let p = match free {
                Some(p) => p,
                None => {
                    let p = self.places;
                    self.set_places(shape, p + 1);
                    // The new place keeps no paper copy.
                    if !self.bare.contains(&p) {
                        self.bare.push(p);
                    }
                    p
                }
            };
            if let Some(slot) = self.pass[i].get_mut(p) {
                *slot = true;
            }
        }
    }

    /// The answers a preset fills in, from the wallet's defaults.
    pub fn preset(shape: &Shape, preset: Preset) -> Answers {
        // A place per key, one for a single key; its passphrase's place is
        // added below, up to two (`docs/BACKUP-FLOW.md` §19.19).
        let base = if shape.multi() { shape.seeds.len() } else { 1 };
        Answers::preset_at(shape, preset, base)
    }

    /// The answers a preset fills in over `base` places: [`Answers::preset`]
    /// at a place per key, or as a plan saved before plans kept at most one
    /// place per key had it (two for a single key).
    pub fn preset_at(shape: &Shape, preset: Preset, base: usize) -> Answers {
        let mut a = Answers::defaults(shape);
        // The passphrase's own place is found again below, once the
        // vault's stick has its place.
        if a.places != base {
            a.set_places(shape, base);
        }
        a.bare.clear();
        // The words on paper and no SeedQR, The standard way too: a SeedQR
        // is added on the board (`docs/BACKUP-FLOW.md` §19.21 item 12).
        if !matches!(preset, Preset::Paper | Preset::Standard) {
            a.seeds[seeds::VAULT] = !shape.watch_only();
            a.wallet[wallet::VAULT] = true;
            a.place_new_sticks(shape, &[]);
        }
        if preset == Preset::PaperVaultSoftware {
            a.wallet[wallet::SOFTWARE] = true;
        }
        // The passphrase's place keeps no copy of its seed, the vault's
        // stick included.
        for row in a.pass.iter_mut().filter(|r| !r.is_empty()) {
            row.iter_mut().for_each(|b| *b = false);
        }
        // A plan over more places than one per key is one saved before
        // (§19.19): its passphrase finds a place as it did then.
        let most = if base > max_places(shape) {
            usize::MAX
        } else {
            max_places(shape)
        };
        a.place_passphrases_up_to(shape, most);
        a
    }

    /// Brings answers made for a wallet's keys-to-be to the shape it turned
    /// out to have: a seed not loaded here is in no vault and keeps no
    /// passphrase, a vault left holding no seed here goes, and a key that
    /// has a row for it (`docs/BACKUP-FLOW.md` §10.2). Nothing is placed:
    /// a default puts a passphrase apart at preset time only, and a plan
    /// the person made keeps its places. Returns, for each vault of the
    /// refitted answers, the vault it was (`usize::MAX` for one added), so
    /// that what is kept by vault number follows.
    pub fn refit(&mut self, shape: &Shape) -> Vec<usize> {
        let n = shape.seeds.len();
        let here = |i: usize| shape.seeds.get(i).is_some_and(|s| s.here);
        let mut kept_rows: Vec<usize> = (0..self.vaults.len()).collect();
        if self.map.is_none() {
            // A vault set up before the keys stays, whatever it holds.
            let keep: Vec<usize> = (0..self.vaults.len())
                .filter(|&v| {
                    v < shape.vaults
                        || self.vaults[v]
                            .iter()
                            .enumerate()
                            .any(|(i, &t)| t && here(i))
                })
                .collect();
            let mut vaults: Vec<Vec<bool>> = keep
                .iter()
                .map(|&v| {
                    (0..n)
                        .map(|i| self.vaults[v].get(i) == Some(&true) && here(i))
                        .collect()
                })
                .collect();
            let mut sticks: Vec<Vec<bool>> = keep
                .iter()
                .map(|&v| {
                    let mut row = self.sticks.get(v).cloned().unwrap_or_default();
                    row.resize(self.places, false);
                    row
                })
                .collect();
            let rows = shape.vault_rows();
            vaults.resize_with(rows, || vec![false; n]);
            sticks.resize_with(rows, || vec![false; self.places]);
            self.vaults = vaults;
            self.sticks = sticks;
            kept_rows = keep;
            kept_rows.resize(rows, usize::MAX);
        }
        if shape.watch_only() {
            self.seeds[seeds::VAULT] = false;
        }
        let old = std::mem::take(&mut self.pass);
        self.pass = (0..n)
            .map(|i| {
                if shape.seeds[i].here && shape.seeds[i].passphrase {
                    let mut row = old.get(i).cloned().unwrap_or_default();
                    if row.len() != self.places + 1 {
                        row = vec![false; self.places + 1];
                    }
                    row
                } else {
                    Vec::new()
                }
            })
            .collect();
        self.place_new_sticks(shape, &[]);
        kept_rows
    }

    /// Moves the seeds of the answers to where they turned out to be:
    /// `map[old]` is the new place of old seed `old` among `new_n` seeds.
    /// A wallet's keys-to-be are numbered by slot; its seeds are numbered
    /// as its description lists them (FROST's by share).
    pub fn remap_seeds(&mut self, map: &[usize], new_n: usize) {
        let to = |old: usize| map.get(old).copied().filter(|&n| n < new_n);
        for row in &mut self.vaults {
            let mut moved = vec![false; new_n];
            for (old, &t) in row.iter().enumerate() {
                if t && let Some(n) = to(old) {
                    moved[n] = true;
                }
            }
            *row = moved;
        }
        let old = std::mem::take(&mut self.pass);
        self.pass = vec![Vec::new(); new_n];
        for (o, row) in old.into_iter().enumerate() {
            if let Some(n) = to(o) {
                self.pass[n] = row;
            }
        }
    }

    /// Costs what `of` costs. The seeds the plan keeps as a file on the
    /// stick of files of a plan saved before the places kept files
    /// (`At::Files`), which the checklist's seed files item makes.
    pub fn seeds_as_files(&self, c: &Compute, shape: &Shape) -> Vec<usize> {
        Mapped::of(c, shape, self).seeds_as_files()
    }

    /// Costs what `of` costs. The seeds the plan keeps as a file on a stick
    /// at a place: see [`Mapped::seeds_on_stick_files`].
    pub fn seeds_on_stick_files(&self, c: &Compute, shape: &Shape) -> Vec<usize> {
        Mapped::of(c, shape, self).seeds_on_stick_files(shape)
    }
}

impl Mapped {
    /// [`Answers::seeds_as_files`] on this map.
    pub fn seeds_as_files(&self) -> Vec<usize> {
        let mut out: Vec<usize> = holds_in(&self.map, At::Files)
            .into_iter()
            .filter_map(|w| match w {
                What::SeedFile(i) => Some(i),
                _ => None,
            })
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The seeds the plan keeps as a file on a stick at a place (their
    /// words, their SeedQR picture or their passphrase's file), in order:
    /// each made only past the secret sheet's warning that a seed's file
    /// on a stick is not protected (`docs/BACKUP-FLOW.md` §19.12 item 6).
    pub fn seeds_on_stick_files(&self, shape: &Shape) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .map
            .iter()
            .filter(|s| matches!(s.at, At::Stick(..)))
            .flat_map(|s| s.holds.iter())
            .filter_map(|(w, _)| match *w {
                What::SeedFile(i) | What::SeedQrFile(i) | What::PassphraseFile(i) => Some(i),
                _ => None,
            })
            .filter(|&i| shape.seeds.get(i).is_some_and(|x| x.here))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

impl Answers {
    /// Costs what `of` costs. Whether the plan keeps seed `i` on a stick:
    /// in a vault, or as a file (its words, its SeedQR picture, or its
    /// passphrase's file) on the stick of files or a stick at a place. Such
    /// a seed is held until that stick is written (F6, F31).
    pub fn seed_on_stick(&self, c: &Compute, shape: &Shape, i: usize) -> bool {
        Mapped::of(c, shape, self).seed_on_stick(i)
    }
}

impl Mapped {
    /// [`Answers::seed_on_stick`] on this map.
    pub fn seed_on_stick(&self, i: usize) -> bool {
        self.map.iter().any(|s| match s.at {
            At::Vault(_) => s
                .holds
                .iter()
                .any(|(w, _)| *w == What::Seed(i) || *w == What::SeedPassphrase(i)),
            At::Files | At::Stick(..) => s.holds.iter().any(|(w, _)| {
                matches!(*w, What::SeedFile(j) | What::SeedQrFile(j) | What::PassphraseFile(j) if j == i)
            }),
            _ => false,
        })
    }
}

impl Answers {
    /// Costs what `of` costs. The places that keep a stick of files: a
    /// stick with no vault on it, holding a seed's file, the description's
    /// files, a share's or the software's (`docs/BACKUP-FLOW.md` §17.2,
    /// §19.4).
    pub fn file_places(&self, c: &Compute, shape: &Shape) -> Vec<usize> {
        Mapped::of(c, shape, self).file_places()
    }

    /// Costs what `of` costs. The files the stick `s` at place `p` keeps,
    /// in order.
    pub fn stick_files(&self, c: &Compute, shape: &Shape, p: usize, s: usize) -> Vec<What> {
        Mapped::of(c, shape, self).stick_files(p, s)
    }
}

impl Mapped {
    /// [`Answers::file_places`] on this map.
    pub fn file_places(&self) -> Vec<usize> {
        let boxes: &[Spot] = &self.map;
        let mut out: Vec<usize> = boxes
            .iter()
            .filter_map(|s| match s.at {
                At::Stick(p, k)
                    if stick_vault(boxes, p, k).is_none()
                        && s.holds.iter().any(|(w, _)| is_file(*w)) =>
                {
                    Some(p)
                }
                _ => None,
            })
            .collect();
        out.dedup();
        out
    }

    /// [`Answers::stick_files`] on this map.
    pub fn stick_files(&self, p: usize, s: usize) -> Vec<What> {
        holds_in(&self.map, At::Stick(p, s))
            .into_iter()
            .filter(|w| is_file(*w))
            .collect()
    }
}

impl Answers {
    /// Sets how many places keep paper, at least one and as many as the
    /// person adds; the lists by place grow or shrink with it.
    pub fn set_places(&mut self, shape: &Shape, places: usize) {
        let places = places.max(1);
        self.places = places;
        self.bare.retain(|&p| p < places);
        self.part.retain(|&(p, _)| p < places);
        // A vault whose only stick was at a place that goes is given
        // another.
        let lost: Vec<usize> = self
            .vaults_made(shape)
            .into_iter()
            .filter(|&v| {
                self.sticks
                    .get(v)
                    .is_some_and(|r| r.iter().any(|&s| s) && !r.iter().take(places).any(|&s| s))
            })
            .collect();
        for row in &mut self.sticks {
            row.resize(places, false);
        }
        for v in lost {
            self.place_stick(shape, v);
        }
        for row in self.pass.iter_mut().filter(|r| !r.is_empty()) {
            let vault = row.last().copied().unwrap_or(false);
            row.truncate(places.min(row.len() - 1));
            row.resize(places, false);
            row.push(vault);
        }
        // A passphrase never shares a place with its words.
        for i in 0..self.pass.len() {
            for p in 0..places {
                if self.pass[i].get(p) == Some(&true) && self.words_at(shape, p).contains(&i) {
                    self.pass[i][p] = false;
                }
            }
        }
    }

    /// Paper seeds are kept: words, a SeedQR, or both.
    pub fn paper_seeds(&self) -> bool {
        self.seeds[seeds::WORDS] || self.seeds[seeds::SEEDQR]
    }

    /// A vault is part of the plan.
    pub fn vault(&self, shape: &Shape) -> bool {
        !self.vaults_made(shape).is_empty()
    }

    /// What spot `at` of the map edited on the chart holds, a place with
    /// its sticks; `None` while the answers draw the map.
    fn edited_at(&self, at: At) -> Option<Vec<What>> {
        let m = self.map.as_ref()?;
        Some(
            m.iter()
                .filter(|s| match (at, s.at) {
                    (At::Place(p), At::Stick(q, _)) => p == q,
                    (a, b) => a == b,
                })
                .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
                .collect(),
        )
    }

    /// The seeds vault `v` holds: those loaded here and ticked into it,
    /// while the seeds go into vaults.
    pub fn vault_seeds(&self, shape: &Shape, v: usize) -> Vec<usize> {
        if let Some(held) = self.edited_at(At::Vault(v)) {
            return held
                .into_iter()
                .filter_map(|w| match w {
                    What::Seed(i) | What::SeedPassphrase(i) => Some(i),
                    _ => None,
                })
                .collect();
        }
        if !self.seeds[seeds::VAULT] || shape.watch_only() {
            return Vec::new();
        }
        let Some(row) = self.vaults.get(v) else {
            return Vec::new();
        };
        (0..shape.seeds.len())
            .filter(|&i| shape.seeds[i].here && row.get(i) == Some(&true))
            .collect()
    }

    /// The vaults made, by number: each with a seed ticked into it; with
    /// none, one for the wallet description when it goes into a vault.
    pub fn vaults_made(&self, shape: &Shape) -> Vec<usize> {
        if let Some(m) = &self.map {
            // A vault no place keeps a stick of is out of the plan
            // (`docs/BACKUP-FLOW.md` §19.16 item 3).
            let kept = kept_vaults(m);
            return m
                .iter()
                .filter_map(|s| match s.at {
                    At::Vault(v) if kept.contains(&v) => Some(v),
                    _ => None,
                })
                .collect();
        }
        let with: Vec<usize> = (0..self.vaults.len())
            .filter(|&v| !self.vault_seeds(shape, v).is_empty())
            .collect();
        if with.is_empty() && self.wallet[wallet::VAULT] {
            vec![0]
        } else {
            with
        }
    }

    /// Whether vault `v` holds the wallet description: every vault made,
    /// when it goes into the vault; on a map edited on the chart, the
    /// vaults that hold it there.
    pub fn wallet_in_vault(&self, v: usize) -> bool {
        match self.edited_at(At::Vault(v)) {
            Some(held) => held.contains(&What::Wallet),
            None => self.wallet[wallet::VAULT],
        }
    }

    /// Vault `v` as the map and the checklist name it: "Vault 1 ·
    /// 9A6A2580", its number and the seeds it holds.
    pub fn vault_name(&self, shape: &Shape, v: usize) -> String {
        let names: Vec<&str> = self
            .vault_seeds(shape, v)
            .into_iter()
            .map(|i| shape.seeds[i].name.as_str())
            .collect();
        if names.is_empty() {
            fill(EN.plan_vault, &[&(v + 1).to_string()])
        } else {
            fill(
                EN.plan_vault_2,
                &[&(v + 1).to_string(), &names.join(", ").to_string()],
            )
        }
    }

    /// The first vault whose stick no place keeps that exists already
    /// (set up before the keys): a warning, "Vault n's stick needs a
    /// place" (`docs/BACKUP-FLOW.md` §19.16 item 3). On a map edited on
    /// the board it is out of the plan; a vault the board made has no
    /// warning at no place.
    pub fn stickless(&self, shape: &Shape) -> Option<usize> {
        if let Some(m) = &self.map {
            let kept = kept_vaults(m);
            return m.iter().find_map(|s| match s.at {
                At::Vault(v) if v < shape.vaults && !kept.contains(&v) => Some(v),
                _ => None,
            });
        }
        self.vaults_made(shape)
            .into_iter()
            .find(|&v| !(0..self.places).any(|p| self.stick_at(v, p)))
    }

    /// Whether place `p` keeps a stick with vault `v` on it.
    pub fn stick_at(&self, v: usize, p: usize) -> bool {
        if let Some(held) = self.edited_at(At::Place(p)) {
            return held.contains(&What::VaultStick(v));
        }
        self.sticks
            .get(v)
            .and_then(|r| r.get(p))
            .copied()
            .unwrap_or(false)
    }

    /// The place the answers put vault `v`'s stick, before any edit on
    /// the chart: a stick there is the vault's own, one at any other
    /// place a copy.
    pub fn first_stick(&self, v: usize) -> Option<usize> {
        self.sticks.get(v)?.iter().position(|&s| s)
    }

    /// Puts vault `v`'s stick where it adds no second key to a place: the
    /// first place that keeps no seed's words and no other vault's stick;
    /// failing that, the last place that keeps only its own seeds' words
    /// and no other vault's stick, which for a seed kept in more than one
    /// place is the one with its further copy; failing that, the last
    /// place with its own seeds' words; last, the place holding the
    /// fewest keys. Every vault's stick has a place.
    fn place_stick(&mut self, shape: &Shape, v: usize) {
        let made = self.vaults_made(shape);
        let own = self.vault_seeds(shape, v);
        let other_stick = |a: &Answers, p: usize| made.iter().any(|&u| u != v && a.stick_at(u, p));
        let words = |p: usize| self.words_at(shape, p);
        let pick = (0..self.places)
            .find(|&p| words(p).is_empty() && !other_stick(self, p))
            .or_else(|| {
                (0..self.places).rev().find(|&p| {
                    let w = words(p);
                    !w.is_empty() && w.iter().all(|i| own.contains(i)) && !other_stick(self, p)
                })
            })
            .or_else(|| {
                (0..self.places)
                    .rev()
                    .find(|&p| words(p).iter().any(|i| own.contains(i)))
            })
            .or_else(|| {
                // Last, the place holding the fewest keys: its words and
                // the seeds of the vaults whose sticks it keeps.
                (0..self.places).min_by_key(|&p| {
                    let mut keys = words(p);
                    for &u in made.iter().filter(|&&u| u != v && self.stick_at(u, p)) {
                        keys.extend(self.vault_seeds(shape, u));
                    }
                    keys.sort_unstable();
                    keys.dedup();
                    keys.len()
                })
            });
        if self.sticks.len() <= v {
            self.sticks.resize(v + 1, vec![false; self.places]);
        }
        let row = &mut self.sticks[v];
        row.resize(self.places, false);
        row.iter_mut().for_each(|s| *s = false);
        if let Some(p) = pick {
            row[p] = true;
        }
    }

    /// Gives each vault made now and not in `before` a place for its
    /// stick, when none is ticked.
    fn place_new_sticks(&mut self, shape: &Shape, before: &[usize]) {
        for v in self.vaults_made(shape) {
            let none = !self.sticks.get(v).is_some_and(|r| r.iter().any(|&s| s));
            if !before.contains(&v) && none {
                self.place_stick(shape, v);
            }
        }
    }

    /// The seeds whose paper copy place `p` keeps: seed `i` goes to place
    /// `i` mod the places, and a place past the last seed keeps another
    /// copy of seed `p` mod the seeds. Seeds not here get no paper.
    pub fn words_at(&self, shape: &Shape, p: usize) -> Vec<usize> {
        if let Some(held) = self.edited_at(At::Place(p)) {
            let mut out: Vec<usize> = held
                .into_iter()
                .filter_map(|w| match w {
                    What::Words(i) | What::SeedQr(i) => Some(i),
                    _ => None,
                })
                .collect();
            out.sort_unstable();
            out.dedup();
            return out;
        }
        if !self.paper_seeds() || self.bare.contains(&p) {
            return Vec::new();
        }
        spread(shape.seeds.len(), self.places, p)
            .into_iter()
            .filter(|&i| shape.seeds[i].here)
            .collect()
    }

    /// The shares place `p` keeps, spread as the seeds are.
    pub fn shares_at(&self, shape: &Shape, p: usize) -> Vec<usize> {
        if let Some(held) = self.edited_at(At::Place(p)) {
            return held
                .into_iter()
                .filter_map(|w| match w {
                    What::Share(j) => Some(j),
                    _ => None,
                })
                .collect();
        }
        if !(self.wallet[wallet::PAPER] && self.split && shape.splits) {
            return Vec::new();
        }
        spread(shape.keys, self.places, p)
    }

    /// Whether place `p` keeps the whole wallet sheet.
    pub fn sheet_at(&self, shape: &Shape, p: usize) -> bool {
        if let Some(held) = self.edited_at(At::Place(p)) {
            return held.contains(&What::Sheet);
        }
        self.wallet[wallet::PAPER] && !(self.split && shape.splits)
    }

    /// Whether seed `i`'s passphrase is kept in place `p`.
    pub fn pass_at(&self, i: usize, p: usize) -> bool {
        if let Some(held) = self.edited_at(At::Place(p)) {
            return held.contains(&What::Passphrase(i));
        }
        self.pass
            .get(i)
            .and_then(|r| r.get(p))
            .copied()
            .unwrap_or(false)
    }

    /// Whether seed `i`'s passphrase goes into the vault with it: into
    /// whichever vault holds the seed.
    pub fn pass_in_vault(&self, i: usize) -> bool {
        if let Some(m) = &self.map {
            return m.iter().any(|s| {
                matches!(s.at, At::Vault(_))
                    && s.holds.iter().any(|(w, _)| *w == What::SeedPassphrase(i))
            });
        }
        self.seeds[seeds::VAULT]
            && self.vaults.iter().any(|r| r.get(i) == Some(&true))
            && self
                .pass
                .get(i)
                .and_then(|r| r.last())
                .copied()
                .unwrap_or(false)
    }

    /// Ticks or unticks a row of a question; a passphrase row that would
    /// share a place with its words stays unticked.
    pub fn toggle(&mut self, shape: &Shape, q: Question, row: usize) {
        // With the map the plan (`docs/BACKUP-FLOW.md` §17.2), it is edited
        // on the plan's cards alone: of the questions, only the software
        // and its form, which are no part of the map, are answered here.
        if self.map.is_some() && !matches!(q, Question::Software | Question::Form) {
            return;
        }
        // An answer changed replaces the map edited on the chart: asked
        // before the questions open (`docs/NEW-WALLET.md` §9.7). The
        // software and the form are no part of the map.
        if !matches!(q, Question::Software | Question::Form) {
            self.map = None;
        }
        let before = self.vaults_made(shape);
        match q {
            Question::Seeds => flip(&mut self.seeds, row),
            Question::Wallet => flip(&mut self.wallet, row),
            Question::Software => flip(&mut self.software, row),
            Question::Form => {
                flip(&mut self.form, row);
                self.files_follow_form(row);
            }
            Question::Split => self.split = row == 1,
            Question::Places => self.set_places(shape, row),
            Question::Vault(v) => {
                let here = shape.seeds.get(row).is_some_and(|s| s.here);
                if let Some(s) = self.vaults.get_mut(v).and_then(|r| r.get_mut(row))
                    && here
                {
                    *s = !*s;
                }
            }
            Question::Sticks(v) => {
                if let Some(s) = self.sticks.get_mut(v).and_then(|r| r.get_mut(row)) {
                    *s = !*s;
                }
            }
            Question::Passphrase(i) => {
                let words = row < self.places && self.words_at(shape, row).contains(&i);
                if let Some(v) = self.pass.get_mut(i).and_then(|r| r.get_mut(row))
                    && !words
                {
                    *v = !*v;
                }
            }
        }
        // A vault left with no stick is given a place, unless the person
        // just unticked its last one.
        let before = if matches!(q, Question::Sticks(_)) {
            before
        } else {
            Vec::new()
        };
        self.place_new_sticks(shape, &before);
    }

    /// The description's files and the shares' on the map take form `f`
    /// as the Software card's form answer now says (`docs/BACKUP-FLOW.md`
    /// §19.3): turned on, each such file gains its form `f` on the same
    /// stick; turned off, each loses it where it keeps another form. The
    /// form answer asks for every file at once until the board asks file
    /// by file.
    fn files_follow_form(&mut self, f: usize) {
        let on = self.form.get(f).copied().unwrap_or(false);
        let Some(m) = self.map.as_mut() else {
            return;
        };
        for s in m.iter_mut().filter(|s| matches!(s.at, At::Stick(..))) {
            let had: Vec<What> = s.holds.iter().map(|(w, _)| *w).collect();
            let twin = |w: What, g: usize| match w {
                What::WalletFile(_) => Some(What::WalletFile(g)),
                What::ShareFile(j, _) => Some(What::ShareFile(j, g)),
                _ => None,
            };
            for w in had {
                let (Some(this), Some(other)) = (twin(w, f), twin(w, 1 - f.min(1))) else {
                    continue;
                };
                let here = |x: What, holds: &[(What, Tag)]| holds.iter().any(|(y, _)| *y == x);
                if on && !here(this, &s.holds) {
                    s.holds.push((this, tag_of(s.at, this)));
                } else if !on && here(other, &s.holds) {
                    s.holds.retain(|(y, _)| *y != this);
                }
            }
        }
    }

    /// The answers as the vault keeps them: a line each, `name value`.
    pub fn to_text(&self) -> String {
        let bits = |v: &[bool]| {
            v.iter()
                .map(|&b| if b { '1' } else { '0' })
                .collect::<String>()
        };
        let mut out = format!(
            "seeds {}\nplaces {}\nsplit {}\nwallet {}\nsoftware {}\nform {}\nomit {}\n",
            bits(&self.seeds),
            self.places,
            u8::from(self.split),
            bits(&self.wallet),
            bits(&self.software),
            bits(&self.form),
            self.omit
        );
        for (v, row) in self.vaults.iter().enumerate() {
            out.push_str(&format!("vault {v} {}\n", bits(row)));
        }
        for (v, row) in self.sticks.iter().enumerate() {
            out.push_str(&format!("sticks {v} {}\n", bits(row)));
        }
        for (i, row) in self.pass.iter().enumerate() {
            if !row.is_empty() {
                out.push_str(&format!("pass {i} {}\n", bits(row)));
            }
        }
        if !self.bare.is_empty() {
            let list: Vec<String> = self.bare.iter().map(|p| p.to_string()).collect();
            out.push_str(&format!("bare {}\n", list.join(",")));
        }
        // The map edited on the chart, a `spot` line per spot: where, and
        // what it holds. Never a name, a date or a mark.
        for spot in self.map.iter().flatten() {
            out.push_str("spot ");
            out.push_str(&at_code(spot.at));
            for (w, _) in &spot.holds {
                out.push(' ');
                out.push_str(&what_code(*w));
            }
            out.push('\n');
        }
        // Whether the board was opened for changes (§19.6).
        if self.custom {
            out.push_str("custom 1\n");
        }
        // A SeedQR drawn in part, at its place (§14.7 item 3).
        for (p, i) in &self.part {
            out.push_str(&format!("part {p} {i}\n"));
        }
        out
    }

    /// Makes `m` the plan: the map edited on the chart (§9.7). The
    /// places are the map's; the lists by place follow their count. A
    /// vault's stick or a file `m` gives a place itself goes on a stick
    /// there, as a plan saved before the sticks reads
    /// (`docs/BACKUP-FLOW.md` §19.4).
    pub fn set_map(&mut self, m: Vec<Spot>) {
        let m = tidy_sticks(m);
        let places = m
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .count()
            .max(1);
        self.places = places;
        self.part.retain(|&(p, _)| p < places);
        for row in &mut self.sticks {
            row.resize(places, false);
        }
        for row in self.pass.iter_mut().filter(|r| !r.is_empty()) {
            let vault = row.last().copied().unwrap_or(false);
            row.truncate(row.len() - 1);
            row.resize(places, false);
            row.push(vault);
        }
        self.map = Some(m);
    }
}

/// A spot as a plan's `spot` line writes it: `place0`, `stick0.1` (stick
/// 1 at place 0), `vault1`, `files`, `software`, `away`.
pub fn at_code(at: At) -> String {
    match at {
        At::Place(p) => format!("place{p}"),
        At::Stick(p, s) => format!("stick{p}.{s}"),
        At::Vault(v) => format!("vault{v}"),
        At::Files => "files".to_string(),
        At::Software => "software".to_string(),
        At::Away => "away".to_string(),
    }
}

/// [`at_code`] read back.
pub fn at_read(s: &str) -> Option<At> {
    Some(match s {
        "files" => At::Files,
        "software" => At::Software,
        "away" => At::Away,
        _ => {
            if let Some(rest) = s.strip_prefix("stick") {
                let (p, k) = rest.split_once('.')?;
                return Some(At::Stick(p.parse().ok()?, k.parse().ok()?));
            }
            match s.strip_prefix("place") {
                Some(p) => At::Place(p.parse().ok()?),
                None => At::Vault(s.strip_prefix("vault")?.parse().ok()?),
            }
        }
    })
}

/// A thing as a plan's `spot` line writes it: `w0` seed 0's words, `q0`
/// its SeedQR, `p0` its passphrase, `sheet`, `share1`, `stick0` vault 0's
/// stick, `seed0`, `seedp0` with its passphrase, `wallet`, `file0` its
/// words file, `qrfile0` its SeedQR picture, `passfile0` its passphrase
/// file, `wallettxt` and `walletqr` the description's text and picture,
/// `sharetxt1` and `shareqr1` share 1's, `softwarefile` the software's.
pub fn what_code(w: What) -> String {
    let form = |f: usize| if f == form::TEXT { "txt" } else { "qr" };
    match w {
        What::Words(i) => format!("w{i}"),
        What::SeedQr(i) => format!("q{i}"),
        What::Passphrase(i) => format!("p{i}"),
        What::Sheet => "sheet".to_string(),
        What::Share(j) => format!("share{j}"),
        What::VaultStick(v) => format!("stick{v}"),
        What::Seed(i) => format!("seed{i}"),
        What::SeedPassphrase(i) => format!("seedp{i}"),
        What::PassphraseSealed(i) => format!("vpass{i}"),
        What::Wallet => "wallet".to_string(),
        What::SeedFile(i) => format!("file{i}"),
        What::SeedQrFile(i) => format!("qrfile{i}"),
        What::PassphraseFile(i) => format!("passfile{i}"),
        What::WalletFile(f) => format!("wallet{}", form(f)),
        What::ShareFile(j, f) => format!("share{}{j}", form(f)),
        What::SoftwareFile => "softwarefile".to_string(),
    }
}

/// [`what_code`] read back. A plan saved before the forms
/// (`docs/BACKUP-FLOW.md` §19.3) wrote `walletfile` and `sharefile1`,
/// which [`Answers::from_text`] reads as every form its `form` answers
/// had on; read alone (a note on the chart), each is its QR picture, the
/// form those answers start with.
pub fn what_read(s: &str) -> Option<What> {
    let n = |p: &str| -> Option<usize> { s.strip_prefix(p)?.parse().ok() };
    Some(match s {
        "sheet" => What::Sheet,
        "wallet" => What::Wallet,
        "wallettxt" => What::WalletFile(form::TEXT),
        "walletqr" | "walletfile" => What::WalletFile(form::QR),
        "softwarefile" => What::SoftwareFile,
        _ if s.starts_with("sharefile") => What::ShareFile(n("sharefile")?, form::QR),
        _ if s.starts_with("sharetxt") => What::ShareFile(n("sharetxt")?, form::TEXT),
        _ if s.starts_with("shareqr") => What::ShareFile(n("shareqr")?, form::QR),
        _ if s.starts_with("seedp") => What::SeedPassphrase(n("seedp")?),
        _ if s.starts_with("vpass") => What::PassphraseSealed(n("vpass")?),
        _ if s.starts_with("seed") => What::Seed(n("seed")?),
        _ if s.starts_with("share") => What::Share(n("share")?),
        _ if s.starts_with("stick") => What::VaultStick(n("stick")?),
        _ if s.starts_with("qrfile") => What::SeedQrFile(n("qrfile")?),
        _ if s.starts_with("passfile") => What::PassphraseFile(n("passfile")?),
        _ if s.starts_with("file") => What::SeedFile(n("file")?),
        _ if s.starts_with('w') => What::Words(n("w")?),
        _ if s.starts_with('q') => What::SeedQr(n("q")?),
        _ if s.starts_with('p') => What::Passphrase(n("p")?),
        _ => return None,
    })
}

/// How a thing kept at a spot may be read: anything in a vault, and a
/// vault's stick, sealed; the description, a sheet, a share and the
/// software's file public; the rest secret.
pub fn tag_of(at: At, what: What) -> Tag {
    match (at, what) {
        (At::Vault(_), _) | (_, What::VaultStick(_)) => Tag::Sealed,
        (
            _,
            What::Sheet
            | What::Share(_)
            | What::Wallet
            | What::WalletFile(_)
            | What::ShareFile(..)
            | What::SoftwareFile,
        ) => Tag::Public,
        _ => Tag::Secret,
    }
}

/// The fingerprints a plan's text lists as made here with their backup
/// pending, one `held` line each.
pub fn held_in(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("held "))
        .map(|f| f.trim().to_string())
        .collect()
}

impl Answers {
    /// Reads [`Answers::to_text`] back for `shape`: None when it is not
    /// for a wallet of this shape. A plan saved before vaults were one per
    /// seed, with one `sticks` line and no `vault` line, reads as one
    /// vault holding every seed here, its stick at the places ticked.
    pub fn from_text(shape: &Shape, text: &str) -> Option<Answers> {
        fn bits<const N: usize>(s: &str) -> Option<[bool; N]> {
            let v = flags(s)?;
            v.try_into().ok()
        }
        fn flags(s: &str) -> Option<Vec<bool>> {
            s.chars()
                .map(|c| match c {
                    '1' => Some(true),
                    '0' => Some(false),
                    _ => None,
                })
                .collect()
        }
        let mut a = Answers::defaults(shape);
        a.bare = Vec::new();
        let mut seen = 0;
        let mut one_stick: Option<Vec<bool>> = None;
        let mut sticks: Vec<(usize, Vec<bool>)> = Vec::new();
        let mut vaults: Vec<(usize, Vec<bool>)> = Vec::new();
        // Each `spot` line's place and things, read once every answer is:
        // a thing written before the forms takes the `form` answers'.
        let mut lines: Vec<(At, Vec<&str>)> = Vec::new();
        let numbered = |value: &str| -> Option<(usize, Vec<bool>)> {
            let (v, row) = value.split_once(' ')?;
            Some((v.parse().ok()?, flags(row)?))
        };
        for line in text.lines() {
            let (name, value) = line.split_once(' ')?;
            match name {
                "seeds" => a.seeds = bits(value)?,
                "places" => a.places = value.parse().ok()?,
                "split" => a.split = value == "1",
                "sticks" if value.contains(' ') => sticks.push(numbered(value)?),
                "sticks" => one_stick = Some(flags(value)?),
                "vault" => vaults.push(numbered(value)?),
                "wallet" => a.wallet = bits(value)?,
                "software" => a.software = bits(value)?,
                "form" => a.form = bits(value)?,
                "omit" => a.omit = value.parse().ok()?,
                "bare" => {
                    a.bare = value
                        .split(',')
                        .map(|p| p.parse().ok())
                        .collect::<Option<Vec<usize>>>()?;
                }
                "pass" => {
                    let (i, row) = value.split_once(' ')?;
                    let i: usize = i.parse().ok()?;
                    *a.pass.get_mut(i)? = flags(row)?;
                }
                // A key made here whose backup is pending: read by the
                // caller (`docs/NEW-WALLET.md` §14.3), not an answer.
                "held" => continue,
                // The access layers' count and routes, written only by test
                // builds before the layers were withdrawn (F38, F46): read
                // past, never refused.
                "layers" | "route" => continue,
                // The preset a plan saved before F53 started from: read,
                // never refused, and no longer written. A plan from Paper
                // only, Paper and vault, or Paper, vault and watch-only
                // software reopens as it was, customisable
                // (`docs/BACKUP-FLOW.md` §19.21 item 7).
                "preset" => {
                    if value != "0" {
                        a.custom = true;
                    }
                    continue;
                }
                "custom" => {
                    a.custom = value == "1";
                    continue;
                }
                // A SeedQR drawn in part (§14.7 item 3).
                "part" => {
                    let (p, i) = value.split_once(' ')?;
                    let pi = (p.parse().ok()?, i.parse().ok()?);
                    if !a.part.contains(&pi) {
                        a.part.push(pi);
                    }
                    continue;
                }
                // The map edited on the chart (§9.7).
                "spot" => {
                    let mut parts = value.split(' ');
                    let at = at_read(parts.next()?)?;
                    lines.push((at, parts.filter(|p| !p.is_empty()).collect()));
                    continue;
                }
                _ => return None,
            }
            seen += 1;
        }
        // A plan kept for more vaults than this shape has rows for (the
        // vaults set up before the keys) reads with all of them.
        let seen_rows = |l: &[(usize, Vec<bool>)]| l.iter().map(|(v, _)| v + 1).max().unwrap_or(0);
        if seen_rows(&vaults).max(seen_rows(&sticks)) > 1024 {
            return None;
        }
        if seen_rows(&vaults) > a.vaults.len() {
            a.vaults
                .resize_with(seen_rows(&vaults), || vec![false; shape.seeds.len()]);
        }
        let rows = shape.vault_rows().max(seen_rows(&sticks));
        a.sticks = vec![vec![false; a.places]; rows];
        match one_stick {
            // Saved with one vault: it holds every seed here.
            Some(row) if vaults.is_empty() && sticks.is_empty() => {
                for (v, r) in a.vaults.iter_mut().enumerate() {
                    r.iter_mut()
                        .enumerate()
                        .for_each(|(i, s)| *s = v == 0 && shape.seeds[i].here);
                }
                a.sticks[0] = row;
            }
            Some(_) => return None,
            None => {
                for (v, row) in sticks {
                    *a.sticks.get_mut(v)? = row;
                }
                for (v, row) in vaults {
                    *a.vaults.get_mut(v)? = row;
                }
            }
        }
        let mut edited: Vec<Spot> = Vec::new();
        for (at, parts) in lines {
            let mut holds: Vec<(What, Tag)> = Vec::new();
            for part in parts {
                // A vault's password on paper, placed only by those same
                // test builds: read past.
                if part.starts_with("vpw") {
                    continue;
                }
                // The description's or a share's files saved before the
                // forms: every form the `form` answers had on
                // (`docs/BACKUP-FLOW.md` §19.3).
                let whats: Vec<What> = match part {
                    "walletfile" => a.file_forms().into_iter().map(What::WalletFile).collect(),
                    _ if part.starts_with("sharefile") => {
                        let j: usize = part.strip_prefix("sharefile")?.parse().ok()?;
                        a.file_forms()
                            .into_iter()
                            .map(|f| What::ShareFile(j, f))
                            .collect()
                    }
                    _ => vec![what_read(part)?],
                };
                for w in whats {
                    if !holds.iter().any(|(x, _)| *x == w) {
                        holds.push((w, tag_of(at, w)));
                    }
                }
            }
            edited.push(Spot { at, holds });
        }
        if !edited.is_empty() {
            // Its places numbered in order, each stick at one of them with
            // at most one vault on it and nothing but stick forms, and
            // nothing in it a seed, a share or a key the wallet does not
            // have.
            let places: Vec<usize> = edited
                .iter()
                .filter_map(|s| match s.at {
                    At::Place(p) => Some(p),
                    _ => None,
                })
                .collect();
            let fits_shape = |w: What| match w {
                What::Words(i)
                | What::SeedQr(i)
                | What::Passphrase(i)
                | What::Seed(i)
                | What::SeedPassphrase(i)
                | What::PassphraseSealed(i)
                | What::SeedFile(i)
                | What::SeedQrFile(i)
                | What::PassphraseFile(i) => i < shape.seeds.len(),
                What::Share(j) => j < shape.keys.max(1),
                What::ShareFile(j, f) => j < shape.keys.max(1) && f <= form::TEXT,
                What::WalletFile(f) => f <= form::TEXT,
                What::VaultStick(v) => v < 1024,
                _ => true,
            };
            let sticks_ok = edited.iter().all(|s| match s.at {
                At::Stick(p, k) => {
                    p < places.len()
                        && k < 1024
                        && s.holds.iter().all(|(w, _)| on_stick(*w))
                        && s.holds
                            .iter()
                            .filter(|(w, _)| matches!(w, What::VaultStick(_)))
                            .count()
                            <= 1
                }
                _ => true,
            });
            let ok = places.iter().enumerate().all(|(n, &p)| n == p)
                && sticks_ok
                && edited
                    .iter()
                    .all(|s| !matches!(s.at, At::Vault(v) if v >= 1024))
                && edited
                    .iter()
                    .all(|s| s.holds.iter().all(|(w, _)| fits_shape(*w)));
            if !ok {
                return None;
            }
            a.set_map(edited);
        }
        let fits = seen >= 8
            && a.places >= 1
            && a.sticks.iter().all(|r| r.len() == a.places)
            && a.vaults.iter().all(|r| {
                r.len() == shape.seeds.len()
                    && r.iter().zip(&shape.seeds).all(|(&t, s)| !t || s.here)
            })
            && a.pass
                .iter()
                .zip(&shape.seeds)
                .all(|(r, s)| r.is_empty() || (s.here && s.passphrase && r.len() == a.places + 1));
        fits.then_some(a)
    }
}

fn flip<const N: usize>(v: &mut [bool; N], row: usize) {
    if let Some(b) = v.get_mut(row) {
        *b = !*b;
    }
}

/// Of `things` spread over `places`: what place `p` keeps. Thing `i` goes
/// to place `i` mod `places`; a place past the last thing keeps thing
/// `p` mod `things` again.
fn spread(things: usize, places: usize, p: usize) -> Vec<usize> {
    if things == 0 || places == 0 {
        return Vec::new();
    }
    let mut out: Vec<usize> = (0..things).filter(|i| i % places == p).collect();
    if p >= things {
        out.push(p % things);
    }
    out
}

/// A question of the plan, as its rows are ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Where the seeds go.
    Seeds,
    /// How many places keep paper: the row is the count.
    Places,
    /// The whole sheet in each place (row 0) or each place its own share
    /// (row 1).
    Split,
    /// Which seeds vault `v` holds: a row per seed of the wallet.
    Vault(usize),
    /// Which places keep a stick with vault `v`.
    Sticks(usize),
    /// Where the wallet description goes.
    Wallet,
    /// The watch-only software.
    Software,
    /// The public files' form.
    Form,
    /// Where seed `i`'s passphrase is kept: a row per place, then the
    /// vault.
    Passphrase(usize),
}

/// Where something is kept, on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum At {
    /// A place, by number from zero: what it keeps on paper.
    Place(usize),
    /// Stick `s` (from zero) at place `p` (`docs/BACKUP-FLOW.md` §19.4):
    /// at most one vault's stick, and any of the stick forms beside it.
    Stick(usize, usize),
    /// Vault `v`, by number from zero.
    Vault(usize),
    /// A stick of unprotected files.
    Files,
    /// Watch-only software.
    Software,
    /// The seeds not here, each on its own device.
    Away,
}

/// What is kept somewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// Seed `i`'s words, by hand.
    Words(usize),
    /// Seed `i`'s SeedQR, by hand.
    SeedQr(usize),
    /// Seed `i`'s passphrase.
    Passphrase(usize),
    /// The whole wallet sheet.
    Sheet,
    /// Share `j`.
    Share(usize),
    /// A stick with vault `v` on it.
    VaultStick(usize),
    /// Seed `i` in the vault, or on its own device.
    Seed(usize),
    /// Seed `i` in the vault with its passphrase.
    SeedPassphrase(usize),
    /// Seed `i`'s passphrase alone in a vault that does not hold the seed:
    /// a passphrase record (`docs/VAULT.md` §7 type 14,
    /// `docs/BACKUP-FLOW.md` §19.12 item 4).
    PassphraseSealed(usize),
    /// The wallet description.
    Wallet,
    /// Seed `i`'s words as a text file: on the stick of files, or on a
    /// stick at a place (`docs/BACKUP-FLOW.md` §19.3).
    SeedFile(usize),
    /// Seed `i`'s SeedQR as a picture, on a stick at a place.
    SeedQrFile(usize),
    /// Seed `i`'s passphrase as a text file, on a stick at a place.
    PassphraseFile(usize),
    /// The wallet description in [`form`] `f` (text, or a QR picture), on
    /// a stick at a place.
    WalletFile(usize),
    /// Share `j` in [`form`] `f`, on a stick at a place.
    ShareFile(usize, usize),
    /// The chosen watch-only software's own file, on a stick.
    SoftwareFile,
}

/// How what is kept may be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    /// Spends, read as it is.
    Secret,
    /// Read only with the vault's passphrase.
    Sealed,
    /// Spends nothing.
    Public,
}

impl Tag {
    /// The tag as the map writes it.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Secret => "secret",
            Tag::Sealed => "sealed",
            Tag::Public => "public",
        }
    }

    /// The tag as a screen shows it.
    pub fn label(self) -> &'static str {
        match self {
            Tag::Secret => EN.plan_tag_secret,
            Tag::Sealed => EN.plan_tag_sealed,
            Tag::Public => EN.plan_tag_public,
        }
    }
}

/// One box of the map: where, and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    /// Where.
    pub at: At,
    /// What it holds, each with its tag.
    pub holds: Vec<(What, Tag)>,
}

impl What {
    /// What it is, as the map and the envelopes list it.
    pub fn label(self, shape: &Shape, medium: crate::Medium) -> String {
        let seed = |i: usize| {
            shape
                .seeds
                .get(i)
                .map_or("?", |s| s.name.as_str())
                .to_string()
        };
        match self {
            What::Words(i) => fill(EN.plan_seed_words, &[&seed(i).to_string()]),
            What::SeedQr(i) => fill(EN.plan_seed_seedqr, &[&seed(i).to_string()]),
            What::Passphrase(i) => fill(EN.plan_passphrase_of, &[&seed(i).to_string()]),
            What::Sheet => EN.plan_wallet_sheet.to_string(),
            What::Share(j) => fill(
                EN.plan_share_of,
                &[&(j + 1).to_string(), &shape.keys.to_string()],
            ),
            What::VaultStick(v) => fill(EN.plan_with_vault, &[medium.cap(), &(v + 1).to_string()]),
            What::Seed(i) => fill(EN.plan_seed, &[&seed(i).to_string()]),
            What::SeedPassphrase(i) => fill(EN.plan_seed_with_its, &[&seed(i).to_string()]),
            What::PassphraseSealed(i) => fill(EN.plan_passphrase_of, &[&seed(i).to_string()]),
            What::Wallet => EN.plan_wallet_description.to_string(),
            What::SeedFile(i) => fill(EN.plan_seed_file, &[&seed(i).to_string()]),
            What::SeedQrFile(i) => fill(EN.plan_seedqr_file, &[&seed(i).to_string()]),
            What::PassphraseFile(i) => fill(EN.plan_passphrase_file, &[&seed(i).to_string()]),
            What::WalletFile(f) => fill(EN.plan_wallet_file, &[form_name(f)]),
            What::ShareFile(j, f) => fill(
                EN.plan_share_file,
                &[&(j + 1).to_string(), &shape.keys.to_string(), form_name(f)],
            ),
            What::SoftwareFile => EN.plan_software_file.to_string(),
        }
    }
}

/// A public file's form as a label names it: "Text file", "QR picture".
pub fn form_name(f: usize) -> &'static str {
    if f == form::TEXT {
        EN.plan_form_text_file
    } else {
        EN.plan_form_qr_picture
    }
}

/// Costs a map of the plan: the map as stored ([`stored_map`]) without a
/// vault no place keeps a stick of, which stays on the board's list and
/// out of the plan (`docs/BACKUP-FLOW.md` §19.16 item 3). The checks, the
/// checklist and the flows read this one.
pub fn map(_c: &Compute, shape: &Shape, a: &Answers) -> Vec<Spot> {
    crate::work::count(crate::work::Work::PlanMap);
    in_plan(stored_map(shape, a))
}

/// The plan mapped once: its map, the map by place, and its check, for
/// the helpers that read them (`docs/SPEED.md` §3.3). A frame gets one
/// through the drawing memo ([`crate::Faraday::plan_mapped`]) and a
/// press makes its own from the state as it is; either way the plan is
/// mapped once and each helper reads the map it is given, never mapping
/// again. It describes the shape and answers it was made from and is not
/// kept past them.
#[derive(Debug)]
pub struct Mapped {
    /// The plan's map ([`map`]).
    pub map: Vec<Spot>,
    /// The map as places keep it ([`place_map`]).
    pub places: Vec<Spot>,
    check: std::cell::OnceCell<Check>,
}

impl Mapped {
    /// Costs a map of the plan: the plan mapped.
    pub fn of(c: &Compute, shape: &Shape, a: &Answers) -> Mapped {
        let map = map(c, shape, a);
        let places = by_place(&map);
        Mapped {
            map,
            places,
            check: std::cell::OnceCell::new(),
        }
    }

    /// Costs a search over the places the first time: the check of this
    /// map ([`check`]), worked out the first time it is asked, a lookup
    /// after.
    pub fn check(&self, _c: &Compute, shape: &Shape, a: &Answers) -> Check {
        *self.check.get_or_init(|| {
            crate::work::count(crate::work::Work::PlanMap);
            check_of(shape, a, &self.places)
        })
    }
}

impl Mapped {
    /// [`Answers::vault_shares`] on this map.
    pub fn vault_shares(&self, v: usize) -> Vec<usize> {
        holds_in(&self.map, At::Vault(v))
            .into_iter()
            .filter_map(|w| match w {
                What::Share(j) => Some(j),
                _ => None,
            })
            .collect()
    }

    /// [`Answers::vault_passphrases`] on this map.
    pub fn vault_passphrases(&self, v: usize) -> Vec<usize> {
        holds_in(&self.map, At::Vault(v))
            .into_iter()
            .filter_map(|w| match w {
                What::PassphraseSealed(i) => Some(i),
                _ => None,
            })
            .collect()
    }

    /// [`Answers::vault_pass`] on this map.
    pub fn vault_pass(&self, v: usize) -> Vec<usize> {
        holds_in(&self.map, At::Vault(v))
            .into_iter()
            .filter_map(|w| match w {
                What::SeedPassphrase(i) => Some(i),
                _ => None,
            })
            .collect()
    }

    /// [`Answers::stick_elsewhere`] on this map.
    pub fn stick_elsewhere(&self, v: usize, p: usize) -> Vec<usize> {
        sticks_of(&self.map, v)
            .into_iter()
            .filter(|&q| q != p)
            .collect()
    }

    /// [`Answers::old_files`] on this map.
    pub fn old_files(&self) -> Vec<What> {
        holds_in(&self.map, At::Files)
    }

    /// [`Answers::software_on`] on this map.
    pub fn software_on(&self) -> bool {
        self.map.iter().any(|s| s.at == At::Software)
    }

    /// [`Answers::whole_at`] on this map.
    pub fn whole_at(&self) -> Vec<usize> {
        let boxes: &[Spot] = &self.places;
        boxes
            .iter()
            .filter_map(|s| match s.at {
                At::Place(p)
                    if s.holds
                        .iter()
                        .any(|(w, _)| matches!(w, What::Sheet | What::WalletFile(_))) =>
                {
                    Some(p)
                }
                _ => None,
            })
            .collect()
    }

    /// [`Answers::row_seeds`] on this map.
    pub fn row_seeds(&self, shape: &Shape, p: usize, row: Row) -> Option<Vec<usize>> {
        let boxes: &[Spot] = &self.places;
        if let Row::Vault(v) = row {
            if !holds_in(boxes, At::Place(p)).contains(&What::VaultStick(v)) {
                return None;
            }
            return Some(vault_seeds_in(boxes, v));
        }
        let held = holds_in(boxes, At::Place(p));
        let out: Vec<usize> = (0..shape.seeds.len())
            .filter(|&i| row.what(i).is_some_and(|w| held.contains(&w)))
            .collect();
        (!out.is_empty()).then_some(out)
    }

    /// [`Answers::shares_here`] on this map.
    pub fn shares_here(&self, p: usize) -> Vec<usize> {
        let boxes: &[Spot] = &self.places;
        let mut out: Vec<usize> = holds_in(boxes, At::Place(p))
            .into_iter()
            .filter_map(|w| match w {
                What::Share(j) | What::ShareFile(j, _) => Some(j),
                _ => None,
            })
            .collect();
        for v in forms_vaults(boxes, p) {
            out.extend(
                holds_in(boxes, At::Vault(v))
                    .into_iter()
                    .filter_map(|w| match w {
                        What::Share(j) => Some(j),
                        _ => None,
                    }),
            );
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// [`Answers::form_at`] on this map.
    pub fn form_at(&self, p: usize, form: Form) -> bool {
        let boxes: &[Spot] = &self.places;
        let held = holds_in(boxes, At::Place(p));
        match form {
            Form::Paper => held
                .iter()
                .any(|w| matches!(w, What::Sheet | What::Share(_))),
            Form::File => held
                .iter()
                .any(|w| matches!(w, What::WalletFile(_) | What::ShareFile(..))),
            Form::Vault(v) => {
                held.contains(&What::VaultStick(v)) && forms_vaults(boxes, p).contains(&v)
            }
        }
    }
}

/// The vaults whose stick place `p` keeps and that hold the
/// description, whole or a share of it.
fn forms_vaults(boxes: &[Spot], p: usize) -> Vec<usize> {
    holds_in(boxes, At::Place(p))
        .into_iter()
        .filter_map(|w| match w {
            What::VaultStick(v)
                if holds_in(boxes, At::Vault(v))
                    .iter()
                    .any(|w| matches!(w, What::Wallet | What::Share(_))) =>
            {
                Some(v)
            }
            _ => None,
        })
        .collect()
}

/// The vaults of the map `m` that a place keeps a stick of, each once,
/// in order.
pub fn kept_vaults(m: &[Spot]) -> Vec<usize> {
    let mut out: Vec<usize> = m
        .iter()
        .filter(|s| matches!(s.at, At::Place(_) | At::Stick(..)))
        .flat_map(|s| s.holds.iter())
        .filter_map(|(w, _)| match w {
            What::VaultStick(v) => Some(*v),
            _ => None,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The map `m` without a vault no place keeps a stick of.
fn in_plan(mut m: Vec<Spot>) -> Vec<Spot> {
    let kept = kept_vaults(&m);
    m.retain(|s| !matches!(s.at, At::Vault(v) if !kept.contains(&v)));
    m
}

/// The map as stored, what the board reads and edits and the plan's text
/// keeps: one box per place, each followed by its sticks (a vault's
/// stick each, in vault order), then each vault, a stick of files, the
/// watch-only software and the seeds on their own devices, each where
/// the plan uses it. A vault no place keeps a stick of is here too.
pub fn stored_map(shape: &Shape, a: &Answers) -> Vec<Spot> {
    if let Some(m) = &a.map {
        return m.clone();
    }
    let made = a.vaults_made(shape);
    let mut out = Vec::new();
    for p in 0..a.places {
        let mut holds = Vec::new();
        for i in a.words_at(shape, p) {
            if a.seeds[seeds::WORDS] {
                holds.push((What::Words(i), Tag::Secret));
            }
            if a.seeds[seeds::SEEDQR] {
                holds.push((What::SeedQr(i), Tag::Secret));
            }
        }
        for i in 0..shape.seeds.len() {
            if a.pass_at(i, p) {
                holds.push((What::Passphrase(i), Tag::Secret));
            }
        }
        if a.sheet_at(shape, p) {
            holds.push((What::Sheet, Tag::Public));
        }
        for j in a.shares_at(shape, p) {
            holds.push((What::Share(j), Tag::Public));
        }
        for &v in made.iter().filter(|&&v| a.stick_at(v, p)) {
            holds.push((What::VaultStick(v), Tag::Sealed));
        }
        out.push(Spot {
            at: At::Place(p),
            holds,
        });
    }
    for &v in &made {
        let mut holds = Vec::new();
        for i in a.vault_seeds(shape, v) {
            let what = if shape.seeds[i].passphrase && a.pass_in_vault(i) {
                What::SeedPassphrase(i)
            } else {
                What::Seed(i)
            };
            holds.push((what, Tag::Sealed));
        }
        if a.wallet[wallet::VAULT] {
            holds.push((What::Wallet, Tag::Sealed));
        }
        out.push(Spot {
            at: At::Vault(v),
            holds,
        });
    }
    let mut files = Vec::new();
    if a.seeds[seeds::FILE] {
        for (i, _) in shape.seeds.iter().enumerate().filter(|(_, s)| s.here) {
            files.push((What::SeedFile(i), Tag::Secret));
        }
    }
    if a.wallet[wallet::FILES] {
        files.push((What::Wallet, Tag::Public));
    }
    if !files.is_empty() {
        out.push(Spot {
            at: At::Files,
            holds: files,
        });
    }
    if a.wallet[wallet::SOFTWARE] {
        out.push(Spot {
            at: At::Software,
            holds: vec![(What::Wallet, Tag::Public)],
        });
    }
    let away: Vec<(What, Tag)> = (0..shape.seeds.len())
        .filter(|&i| !shape.seeds[i].here)
        .map(|i| (What::Seed(i), Tag::Secret))
        .collect();
    if !away.is_empty() {
        out.push(Spot {
            at: At::Away,
            holds: away,
        });
    }
    tidy_sticks(out)
}

/// "Any one place lost: the rest rebuild the wallet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lost {
    /// Some place lost leaves too little.
    No,
    /// Every place lost leaves enough, some only with the vault's
    /// passphrase.
    WithVault,
    /// Every place lost leaves enough.
    Yes,
}

/// "One place found: can spend" and "sees the balance".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Found {
    /// No one place gives it.
    No,
    /// Some place gives it, but only with the vault's passphrase.
    OnlyWithVault,
    /// Some place gives it.
    Yes,
}

impl Lost {
    /// The value as the check writes it.
    pub fn text(self) -> &'static str {
        match self {
            Lost::Yes => EN.plan_yes,
            Lost::WithVault => EN.plan_yes_with_the_vault,
            Lost::No => EN.plan_no,
        }
    }
}

impl Found {
    /// The value as the check writes it.
    pub fn text(self) -> &'static str {
        match self {
            Found::Yes => EN.plan_yes,
            Found::OnlyWithVault => EN.plan_only_with_the,
            Found::No => EN.plan_no,
        }
    }
}

/// The check under the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    /// Any one place lost: the rest rebuild the wallet.
    pub lost: Lost,
    /// One place found: can spend.
    pub spend: Found,
    /// One place found: sees the balance.
    pub balance: Found,
    /// The place that keeps every secret of the wallet, or a seed with its
    /// passphrase (`docs/BACKUP-FLOW.md` §10.3): a line of fact, never a
    /// block.
    pub one_place: Option<usize>,
    /// The plan has a single place: "Any one place lost" is No by the
    /// nature of it, and reads so, muted (`docs/BACKUP-FLOW.md` §19.19).
    pub single: bool,
    /// A single key with no passphrase: a passphrase kept at another
    /// place than the seed is recommended.
    pub passphrase_apart: bool,
}

/// The check's three lines, labelled.
pub const CHECK_LINES: [&str; 3] = [
    EN.plan_any_one_place,
    EN.plan_one_place_found_can,
    EN.plan_one_place_found,
];

/// What can be read at once: seeds, passphrases, the whole description,
/// and shares.
#[derive(Clone)]
struct Have {
    seeds: Vec<bool>,
    pass: Vec<bool>,
    wallet: bool,
    shares: Vec<bool>,
}

impl Have {
    fn none(shape: &Shape) -> Have {
        Have {
            seeds: vec![false; shape.seeds.len()],
            pass: vec![false; shape.seeds.len()],
            wallet: false,
            shares: vec![false; shape.keys],
        }
    }

    fn add(&mut self, what: What) {
        match what {
            What::Words(i)
            | What::SeedQr(i)
            | What::Seed(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i) => {
                self.seeds[i] = true;
            }
            What::SeedPassphrase(i) => {
                self.seeds[i] = true;
                self.pass[i] = true;
            }
            What::Passphrase(i) | What::PassphraseFile(i) | What::PassphraseSealed(i) => {
                self.pass[i] = true;
            }
            What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile => {
                self.wallet = true;
            }
            What::Share(j) | What::ShareFile(j, _) => {
                if let Some(s) = self.shares.get_mut(j) {
                    *s = true;
                }
            }
            What::VaultStick(_) => {}
        }
    }

    fn seeds_ok(&self, shape: &Shape) -> usize {
        (0..shape.seeds.len())
            .filter(|&i| self.seeds[i] && (!shape.seeds[i].passphrase || self.pass[i]))
            .count()
    }

    /// The description, whole or from shares that together hold every key;
    /// a seed kept beside a share gives its own key, the one a share left
    /// off (`docs/BACKUP-FLOW.md` §19.5).
    fn wallet_ok(&self, shape: &Shape, a: &Answers) -> bool {
        if self.wallet {
            return true;
        }
        if !shape.splits || !self.shares.iter().any(|&s| s) {
            return false;
        }
        let plan = split_plan(shape.keys, shape.m, a.omit);
        (0..shape.keys).all(|k| {
            self.key_from_seed(shape, k)
                || plan
                    .iter()
                    .enumerate()
                    .any(|(j, row)| self.shares[j] && row.contains(&k))
        })
    }

    /// Key `k` of the description is given by its own seed kept here,
    /// with its passphrase where it has one.
    fn key_from_seed(&self, shape: &Shape, k: usize) -> bool {
        shape.seeds.get(k).is_some_and(|s| {
            self.seeds.get(k) == Some(&true) && (!s.passphrase || self.pass.get(k) == Some(&true))
        })
    }

    /// The wallet rebuilds: a quorum of seeds, and for more than one seed
    /// the description; one seed rebuilds its own wallet.
    fn rebuilds(&self, shape: &Shape, a: &Answers) -> bool {
        self.seeds_ok(shape) >= shape.m.max(1) && (!shape.multi() || self.wallet_ok(shape, a))
    }

    /// The balance can be seen: the description, or the one seed of a
    /// one-key wallet.
    fn sees(&self, shape: &Shape, a: &Answers) -> bool {
        self.wallet_ok(shape, a) || (!shape.multi() && self.seeds_ok(shape) >= 1)
    }
}

/// Where a thing may be lost or found, on the map `boxes`: each place, a
/// stick of files, and each vault's own stick when no place keeps one.
fn spots(boxes: &[Spot]) -> Vec<At> {
    let mut v: Vec<At> = boxes
        .iter()
        .filter(|s| matches!(s.at, At::Place(_)))
        .map(|s| s.at)
        .collect();
    if boxes.iter().any(|s| s.at == At::Files) {
        v.push(At::Files);
    }
    for s in boxes {
        if let At::Vault(u) = s.at
            && sticks_of(boxes, u).is_empty()
        {
            v.push(s.at);
        }
    }
    v
}

/// The places that keep a stick with vault `v` on it, on the map `boxes`,
/// each once.
fn sticks_of(boxes: &[Spot], v: usize) -> Vec<usize> {
    let mut out: Vec<usize> = boxes
        .iter()
        .filter_map(|s| match s.at {
            At::Place(p) | At::Stick(p, _)
                if s.holds.iter().any(|(w, _)| *w == What::VaultStick(v)) =>
            {
                Some(p)
            }
            _ => None,
        })
        .collect();
    out.dedup();
    out
}

/// What one spot gives whoever finds it alone: whether it spends, and
/// whether it sees the balance, each outright or only with the vault's
/// passphrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alone {
    /// It spends.
    pub spend: Found,
    /// It sees the balance.
    pub balance: Found,
}

/// How an [`Alone`] line reads for the chart's colour, kept beside the
/// text itself ([`Alone::parts`]) so the two cannot drift apart: a case
/// added to one must be added to the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AloneKind {
    /// A vault's own seal ("Sealed"), not a spot's reach: the vault's
    /// own accent. Not an [`Alone`] case; a vault's spot never computes
    /// one (`glance::backup_nodes`).
    Sealed,
    /// Spends outright, in the clear: a warning.
    CanSpend,
    /// Nothing at all: good.
    Nothing,
    /// Sees the balance outright, without the vault: neutral.
    SeesBalance,
    /// Needs the vault's passphrase, to spend or to see the balance:
    /// the vault's own accent.
    NeedsVault,
}

impl Alone {
    /// The text and its [`AloneKind`] together, so a new case cannot add
    /// one without the other.
    fn parts(self) -> (&'static str, AloneKind) {
        match (self.spend, self.balance) {
            (Found::Yes, _) => (EN.plan_can_spend, AloneKind::CanSpend),
            (Found::OnlyWithVault, Found::Yes) => {
                (EN.plan_sees_the_balance_can, AloneKind::NeedsVault)
            }
            (Found::OnlyWithVault, _) => (EN.plan_can_spend_with, AloneKind::NeedsVault),
            (Found::No, Found::Yes) => (EN.plan_sees_the_balance_2, AloneKind::SeesBalance),
            (Found::No, Found::OnlyWithVault) => (EN.plan_sees_the_balance, AloneKind::NeedsVault),
            (Found::No, Found::No) => (EN.plan_nothing, AloneKind::Nothing),
        }
    }

    /// The line the wallet's chart writes under a spot: "Cannot see
    /// balance", "Sees the balance", "Can spend", "Can spend with the
    /// vault's passphrase".
    pub fn text(self) -> &'static str {
        self.parts().0
    }

    /// The balance alone, as a place's header tag reads it beside its
    /// spend tag ([`Reach::tag`], `docs/BACKUP-FLOW.md` §19.22).
    pub fn balance_tag(self) -> (&'static str, AloneKind) {
        match self.balance {
            Found::Yes => (EN.plan_sees_the_balance_2, AloneKind::SeesBalance),
            Found::OnlyWithVault => (EN.plan_sees_the_balance, AloneKind::NeedsVault),
            Found::No => (EN.plan_nothing, AloneKind::Nothing),
        }
    }

    /// What [`Alone::text`] reads as, for the chart's colour
    /// ([`crate::glance::alone_color`]), without the colour reading the
    /// words themselves.
    pub fn kind(self) -> AloneKind {
        self.parts().1
    }
}

/// Costs what `of` costs. What spot `at` gives whoever finds it alone:
/// what it holds, and each vault whose stick it keeps, read with that
/// vault's passphrase. The check's "One place found" lines are the most
/// any spot gives. A place is read with its sticks; a stick alone, as
/// itself.
pub fn alone(c: &Compute, shape: &Shape, a: &Answers, at: At) -> Alone {
    alone_on(shape, a, &Mapped::of(c, shape, a), at)
}

/// [`alone`] on the map `m`.
pub fn alone_on(shape: &Shape, a: &Answers, m: &Mapped, at: At) -> Alone {
    match at {
        At::Stick(..) => alone_in(shape, a, &m.map, at),
        _ => alone_in(shape, a, &with_parts(&m.places, a), at),
    }
}

/// The map `boxes` with each SeedQR drawn in part ([`Answers::part`]) at
/// its place, a secret there: what whoever finds the place finds. Not a
/// copy that rebuilds, so "Any one place lost" never reads this.
pub fn with_parts<'a>(boxes: &'a [Spot], a: &Answers) -> std::borrow::Cow<'a, [Spot]> {
    if a.part.is_empty() {
        return std::borrow::Cow::Borrowed(boxes);
    }
    let mut out = boxes.to_vec();
    for s in out.iter_mut().filter(|s| matches!(s.at, At::Place(_))) {
        for w in parts_at(a, s.at) {
            if !s.holds.iter().any(|(h, _)| *h == w) {
                s.holds.push((w, Tag::Secret));
            }
        }
    }
    std::borrow::Cow::Owned(out)
}

/// The SeedQR copies drawn in part at spot `at` ([`Answers::part`]).
pub fn parts_at(a: &Answers, at: At) -> Vec<What> {
    a.part
        .iter()
        .filter(|&&(p, _)| at == At::Place(p))
        .map(|&(_, i)| What::SeedQr(i))
        .collect()
}

/// Whether `what` at `at` is a SeedQR drawn in part ([`Answers::part`]).
pub fn is_part(a: &Answers, at: At, what: What) -> bool {
    matches!((at, what), (At::Place(p), What::SeedQr(i)) if a.part.contains(&(p, i)))
}

/// How near place `p` brings a spend, counting seeds alone: the
/// description is public, so it is never what stands in the way
/// (`docs/BACKUP-FLOW.md` §19.22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach {
    /// The fewest other places that, with this one, hold enough seeds to
    /// sign. 0: this place spends alone.
    pub more: usize,
    /// This place gets there only with its vaults' passphrases; without
    /// them it needs more places, or cannot.
    pub vault: bool,
    /// The seeds this place gives to sign with, its vaults read, at most
    /// the quorum.
    pub seeds: usize,
}

impl Reach {
    /// The place's spend tag and how it reads: spending alone in the
    /// clear is a warning, with a vault's passphrase the vault's accent,
    /// the rest neutral.
    pub fn tag(self) -> (String, AloneKind) {
        let kind = match (self.more, self.vault) {
            (0, false) => AloneKind::CanSpend,
            (_, true) => AloneKind::NeedsVault,
            _ => AloneKind::SeesBalance,
        };
        let text = match (self.more, self.vault) {
            (0, false) => EN.board_spend_alone.to_string(),
            (0, true) => EN.board_spend_alone_vault.to_string(),
            (1, false) => EN.board_spend_more_one.to_string(),
            (1, true) => EN.board_spend_vault_more_one.to_string(),
            (k, false) => fill(EN.board_spend_more, &[&k.to_string()]),
            (k, true) => fill(EN.board_spend_vault_more, &[&k.to_string()]),
        };
        (text, kind)
    }

    /// Whether `self` is nearer a spend than `was`: fewer places, or as
    /// many without the vault's passphrase.
    pub fn nearer_than(self, was: Option<Reach>) -> bool {
        was.is_none_or(|w| (self.more, self.vault) < (w.more, w.vault))
    }
}

/// Costs a map of the plan and a search over the places, at most the
/// quorum deep. What place `p` brings to a spend: `None` when it keeps
/// no seed, or no set of places adds up to a spend. Every other place
/// counts with its vaults read.
pub fn reach(c: &Compute, shape: &Shape, a: &Answers, p: usize) -> Option<Reach> {
    reach_on(shape, &Mapped::of(c, shape, a), p)
}

/// [`reach`] on the map `mapped`. Costs a search over the places, at most
/// the quorum deep.
pub fn reach_on(shape: &Shape, mapped: &Mapped, p: usize) -> Option<Reach> {
    let boxes: &[Spot] = &mapped.places;
    let m = shape.m.max(1);
    let n = shape.seeds.len().min(64);
    let need: u64 = (0..n)
        .filter(|&i| shape.seeds[i].passphrase)
        .fold(0, |acc, i| acc | 1 << i);
    // A place as two masks: the seeds it keeps, the passphrases.
    let masks = |h: &Have| -> (u64, u64) {
        let bits = |v: &[bool]| {
            (0..n)
                .filter(|&i| v.get(i) == Some(&true))
                .fold(0u64, |acc, i| acc | 1 << i)
        };
        (bits(&h.seeds), bits(&h.pass))
    };
    let signs = |(s, ps): (u64, u64)| (s & (ps | !need)).count_ones() as usize;
    let read = |q: usize, vaults: bool| -> (u64, u64) {
        let at = At::Place(q);
        let mut have = Have::none(shape);
        holds_in(boxes, at).into_iter().for_each(|w| have.add(w));
        if vaults {
            have = opened_in(boxes, &have, &reads_in(boxes, at));
        }
        masks(&have)
    };
    let places: Vec<usize> = spots(boxes)
        .into_iter()
        .filter_map(|at| match at {
            At::Place(q) => Some(q),
            _ => None,
        })
        .collect();
    if !places.contains(&p) {
        return None;
    }
    let open = read(p, true);
    if open.0 == 0 {
        return None;
    }
    let others: Vec<(u64, u64)> = places
        .iter()
        .filter(|&&q| q != p)
        .map(|&q| read(q, true))
        .collect();
    let fewest = |start: (u64, u64)| -> Option<usize> {
        // Only places that add something can help.
        let useful: Vec<(u64, u64)> = others
            .iter()
            .copied()
            .filter(|&(s, ps)| s & !start.0 != 0 || ps & !start.1 != 0)
            .collect();
        fn search(
            from: &[(u64, u64)],
            have: (u64, u64),
            left: usize,
            signs: &dyn Fn((u64, u64)) -> usize,
            m: usize,
        ) -> bool {
            if signs(have) >= m {
                return true;
            }
            if left == 0 {
                return false;
            }
            (0..from.len()).any(|i| {
                let (s, ps) = from[i];
                search(
                    &from[i + 1..],
                    (have.0 | s, have.1 | ps),
                    left - 1,
                    signs,
                    m,
                )
            })
        }
        let most = useful.len().min(m);
        (0..=most).find(|&k| search(&useful, start, k, &signs, m))
    };
    let with = fewest(open)?;
    let plain = read(p, false);
    let without = if plain == open {
        Some(with)
    } else {
        fewest(plain)
    };
    Some(Reach {
        more: with,
        vault: without != Some(with),
        seeds: signs(open).min(m),
    })
}

/// What spot `at` holds on the map `boxes`, in order.
pub fn holds_at(boxes: &[Spot], at: At) -> Vec<What> {
    holds_in(boxes, at)
}

/// What a spot holds, on the map `boxes`.
fn holds_in(boxes: &[Spot], at: At) -> Vec<What> {
    boxes
        .iter()
        .filter(|s| s.at == at)
        .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
        .collect()
}

/// The vaults a spot can be read from: the ones whose stick a place keeps,
/// or a vault's own stick.
fn reads_in(boxes: &[Spot], at: At) -> Vec<usize> {
    let made = |v: usize| boxes.iter().any(|s| s.at == At::Vault(v));
    match at {
        At::Vault(v) if made(v) => vec![v],
        At::Vault(_) => Vec::new(),
        at => holds_in(boxes, at)
            .into_iter()
            .filter_map(|w| match w {
                What::VaultStick(v) if made(v) => Some(v),
                _ => None,
            })
            .collect(),
    }
}

/// `have`, with what `vaults` hold read too.
fn opened_in(boxes: &[Spot], have: &Have, vaults: &[usize]) -> Have {
    let mut with = have.clone();
    for &v in vaults {
        holds_in(boxes, At::Vault(v))
            .into_iter()
            .for_each(|w| with.add(w));
    }
    with
}

fn alone_in(shape: &Shape, a: &Answers, boxes: &[Spot], at: At) -> Alone {
    let mut have = Have::none(shape);
    if !matches!(at, At::Vault(_)) {
        holds_in(boxes, at).into_iter().for_each(|w| have.add(w));
    }
    let with = opened_in(boxes, &have, &reads_in(boxes, at));
    let found = |ok: &dyn Fn(&Have) -> bool| {
        if ok(&have) {
            Found::Yes
        } else if ok(&with) {
            Found::OnlyWithVault
        } else {
            Found::No
        }
    };
    Alone {
        // Seeds alone spend: the description is public (§19.22).
        spend: found(&|h| h.seeds_ok(shape) >= shape.m.max(1)),
        balance: found(&|h| h.sees(shape, a)),
    }
}

/// Costs a map of the plan and a search over its places: computes the
/// check, every place lost in turn, and every place found. Each vault is
/// read, with its passphrase, from a place that keeps its stick, or from
/// its own stick. A place is lost or found with its sticks.
pub fn check(c: &Compute, shape: &Shape, a: &Answers) -> Check {
    Mapped::of(c, shape, a).check(c, shape, a)
}

/// The check of the map `boxes`.
fn check_of(shape: &Shape, a: &Answers, boxes: &[Spot]) -> Check {
    let places = spots(boxes);
    let lost = places
        .iter()
        .map(|&gone| lost_one(shape, a, boxes, &places, Some(gone)))
        .min()
        .unwrap_or(Lost::Yes);
    let mut spend = Found::No;
    let mut balance = Found::No;
    // A SeedQR drawn in part is a secret at its place when the place is
    // found, and no copy when another is lost (§14.7 item 3).
    let found_in = with_parts(boxes, a);
    for &at in &places {
        let found = alone_in(shape, a, &found_in, at);
        spend = spend.max(found.spend);
        balance = balance.max(found.balance);
    }
    Check {
        lost,
        spend,
        balance,
        // A single key has one place at most: its every secret there is
        // no warning (§19.19).
        one_place: one_place_in(shape, &found_in).filter(|_| max_places(shape) > 1),
        single: places.len() == 1,
        passphrase_apart: shape.seeds.len() == 1
            && shape.seeds[0].here
            && !shape.seeds[0].passphrase,
    }
}

/// Costs what `of` costs. The place that holds every secret of the
/// wallet: every seed of the wallet, loaded here or not, in any form
/// (words, SeedQR, a seed file on the place's stick of files, a vault
/// whose stick is there; a seed file on the stick of files, and a seed
/// kept on its own device, are at no place), and every passphrase of
/// those seeds (on paper there, or in a vault whose stick is there). It
/// is also the place that holds a seed with its own passphrase,
/// whatever else is elsewhere (`docs/BACKUP-FLOW.md` §10.3).
pub fn all_at_one_place(c: &Compute, shape: &Shape, a: &Answers) -> Option<usize> {
    all_at_one_place_on(shape, &Mapped::of(c, shape, a))
}

/// [`all_at_one_place`] on the map `m`.
pub fn all_at_one_place_on(shape: &Shape, m: &Mapped) -> Option<usize> {
    one_place_in(shape, &m.places)
}

/// [`all_at_one_place`] on the map `boxes`.
fn one_place_in(shape: &Shape, boxes: &[Spot]) -> Option<usize> {
    if shape.watch_only() {
        return None;
    }
    let seeds: Vec<usize> = (0..shape.seeds.len()).collect();
    let places: Vec<usize> = boxes
        .iter()
        .filter_map(|s| match s.at {
            At::Place(p) => Some(p),
            _ => None,
        })
        .collect();
    let holds = |s: &Spot, f: &dyn Fn(What) -> bool| s.holds.iter().any(|(w, _)| f(*w));
    // A vault holding `f`, whose stick is at place `p`.
    let in_vault_at = |p: usize, f: &dyn Fn(What) -> bool| {
        boxes.iter().any(|s| match s.at {
            At::Vault(v) => holds(s, f) && sticks_of(boxes, v).contains(&p),
            _ => false,
        })
    };
    let seed_at = |i: usize, p: usize| {
        boxes
            .iter()
            .any(|s| s.at == At::Place(p) && holds(s, &|w| seed_form(w) == Some(i)))
            || in_vault_at(p, &|w| w == What::Seed(i) || w == What::SeedPassphrase(i))
    };
    let pass_at = |i: usize, p: usize| {
        boxes.iter().any(|s| {
            s.at == At::Place(p)
                && holds(s, &|w| {
                    w == What::Passphrase(i) || w == What::PassphraseFile(i)
                })
        }) || in_vault_at(p, &|w| {
            w == What::SeedPassphrase(i) || w == What::PassphraseSealed(i)
        })
    };
    for &p in &places {
        if seeds
            .iter()
            .all(|&i| seed_at(i, p) && (!shape.seeds[i].passphrase || pass_at(i, p)))
        {
            return Some(p);
        }
    }
    places.iter().copied().find(|&p| {
        seeds
            .iter()
            .any(|&i| shape.seeds[i].passphrase && seed_at(i, p) && pass_at(i, p))
    })
}

/// What is left with spot `gone` lost (none: with nothing lost), of
/// `places` on the map `boxes`.
fn lost_one(shape: &Shape, a: &Answers, boxes: &[Spot], places: &[At], gone: Option<At>) -> Lost {
    let mut have = Have::none(shape);
    let mut readable: Vec<usize> = Vec::new();
    for &at in places.iter().filter(|&&at| Some(at) != gone) {
        if !matches!(at, At::Vault(_)) {
            holds_in(boxes, at).into_iter().for_each(|w| have.add(w));
        }
        readable.extend(reads_in(boxes, at));
    }
    holds_in(boxes, At::Software)
        .into_iter()
        .for_each(|w| have.add(w));
    holds_in(boxes, At::Away)
        .into_iter()
        .for_each(|w| have.add(w));
    if have.rebuilds(shape, a) {
        Lost::Yes
    } else if !readable.is_empty() && opened_in(boxes, &have, &readable).rebuilds(shape, a) {
        Lost::WithVault
    } else {
        Lost::No
    }
}

/// The seed a copy outside a vault is of: its words or SeedQR by hand,
/// its words file or its SeedQR picture.
fn seed_form(w: What) -> Option<usize> {
    match w {
        What::Words(i) | What::SeedQr(i) | What::SeedFile(i) | What::SeedQrFile(i) => Some(i),
        _ => None,
    }
}

/// Where seed `i` is kept on the map `boxes` (by place): each place with
/// its words, SeedQR or a file of it on a stick there, the stick of files,
/// each vault that holds it.
fn seed_spots(boxes: &[Spot], i: usize) -> Vec<At> {
    let has = |s: &Spot, f: &dyn Fn(What) -> bool| s.holds.iter().any(|(w, _)| f(*w));
    let mut out: Vec<At> = boxes
        .iter()
        .filter(|s| matches!(s.at, At::Place(_)) && has(s, &|w| seed_form(w) == Some(i)))
        .map(|s| s.at)
        .collect();
    if boxes
        .iter()
        .any(|s| s.at == At::Files && has(s, &|w| w == What::SeedFile(i)))
    {
        out.push(At::Files);
    }
    out.extend(
        boxes
            .iter()
            .filter(|s| {
                matches!(s.at, At::Vault(_))
                    && has(s, &|w| w == What::Seed(i) || w == What::SeedPassphrase(i))
            })
            .map(|s| s.at),
    );
    out
}

/// Where seed `i`'s passphrase is kept on the map `boxes` (by place):
/// each place that keeps it written or as a file on a stick there, each
/// vault that holds it with its seed.
fn pass_spots(boxes: &[Spot], i: usize) -> Vec<At> {
    boxes
        .iter()
        .filter(|s| {
            s.holds.iter().any(|(w, _)| match s.at {
                At::Place(_) => *w == What::Passphrase(i) || *w == What::PassphraseFile(i),
                At::Vault(_) => *w == What::SeedPassphrase(i) || *w == What::PassphraseSealed(i),
                _ => false,
            })
        })
        .map(|s| s.at)
        .collect()
}

/// Costs what `of` costs. Why "Any one place lost" reads No, as a line
/// under the check: the first spot whose loss leaves too little, and
/// the seed or passphrase kept only there. `name` names a place, `noun`
/// is the medium ("stick"). None when the check does not read No.
pub fn lost_why(
    c: &Compute,
    shape: &Shape,
    a: &Answers,
    name: &dyn Fn(usize) -> String,
    noun: &str,
) -> Option<String> {
    lost_why_on(shape, a, &Mapped::of(c, shape, a), name, noun)
}

/// [`lost_why`] on the map `m`.
pub fn lost_why_on(
    shape: &Shape,
    a: &Answers,
    m: &Mapped,
    name: &dyn Fn(usize) -> String,
    noun: &str,
) -> Option<String> {
    let boxes: &[Spot] = &m.places;
    let places = spots(boxes);
    // A single place: "No: a single place" says why (§19.19); a seed kept
    // nowhere still says so.
    if places.len() == 1 {
        return kept_nowhere_on(shape, m, &shape.here());
    }
    let gone = places
        .iter()
        .copied()
        .find(|&g| lost_one(shape, a, boxes, &places, Some(g)) == Lost::No)?;
    let stick_text = |v: usize| -> String {
        let at: Vec<String> = sticks_of(boxes, v).into_iter().map(name).collect();
        if at.is_empty() {
            EN.plan_at_no_place.to_string()
        } else {
            fill(EN.plan_at, &[&at.join(", ").to_string()])
        }
    };
    // A thing kept at `at` is lost with `gone`: there, or in a vault whose
    // every stick is there.
    let with_gone = |at: At| -> bool {
        at == gone
            || matches!(at, At::Vault(v) if sticks_of(boxes, v)
                .into_iter()
                .all(|p| At::Place(p) == gone))
    };
    let only = |list: &[At]| -> Option<String> {
        match list {
            [At::Vault(v)] => Some(fill(
                EN.plan_vault_whose_is,
                &[&(v + 1).to_string(), noun, &stick_text(*v).to_string()],
            )),
            [At::Place(p)] => Some(name(*p)),
            [At::Files] => Some(fill(EN.plan_the_of_files, &[noun])),
            _ => None,
        }
    };
    for i in shape.here() {
        let seed = &shape.seeds[i].name;
        let kept = seed_spots(boxes, i);
        if kept.is_empty() {
            return Some(fill(EN.plan_seed_is_kept_nowhere, &[&seed.to_string()]));
        }
        if kept.iter().all(|&at| with_gone(at)) {
            let line = match only(&kept) {
                Some(w) if matches!(kept[0], At::Vault(_)) => fill(EN.plan_in, &[&w.to_string()]),
                Some(w) if kept[0] == At::Files => fill(EN.plan_on, &[&w.to_string()]),
                Some(w) => fill(EN.plan_at, &[&w.to_string()]),
                None => fill(EN.plan_with, &[&name_at(gone, name, noun).to_string()]),
            };
            return Some(fill(
                EN.plan_seed_is_kept_only,
                &[&seed.to_string(), &line.to_string()],
            ));
        }
        if shape.seeds[i].passphrase {
            let pass = pass_spots(boxes, i);
            if pass.is_empty() {
                return Some(fill(EN.plan_the_passphrase, &[&seed.to_string()]));
            }
            if pass.iter().all(|&at| with_gone(at)) {
                return Some(fill(
                    EN.plan_the_passphrase_of,
                    &[&seed.to_string(), &name_at(gone, name, noun).to_string()],
                ));
            }
        }
    }
    // The seeds are there without it, the description is not (§19.19
    // item 5): the line names the description.
    let mut have = Have::none(shape);
    let mut readable: Vec<usize> = Vec::new();
    for &at in places.iter().filter(|&&at| at != gone) {
        if !matches!(at, At::Vault(_)) {
            holds_in(boxes, at).into_iter().for_each(|w| have.add(w));
        }
        readable.extend(reads_in(boxes, at));
    }
    for at in [At::Software, At::Away] {
        holds_in(boxes, at).into_iter().for_each(|w| have.add(w));
    }
    let have = opened_in(boxes, &have, &readable);
    if have.seeds_ok(shape) >= shape.m.max(1) && !have.wallet_ok(shape, a) {
        return Some(fill(
            EN.plan_without_no_description,
            &[&name_at(gone, name, noun).to_string()],
        ));
    }
    Some(fill(
        EN.plan_without_the_rest,
        &[&name_at(gone, name, noun).to_string()],
    ))
}

/// A spot as the why line names it.
fn name_at(at: At, name: &dyn Fn(usize) -> String, noun: &str) -> String {
    match at {
        At::Place(p) | At::Stick(p, _) => name(p),
        At::Vault(v) => fill(EN.plan_vault, &[&(v + 1).to_string()]),
        At::Files => fill(EN.plan_the_of_files, &[noun]),
        At::Software => EN.plan_the_software.to_string(),
        At::Away => EN.plan_the_devices.to_string(),
    }
}

/// Costs what `of` costs. The first of `seeds` (made here) the plan
/// keeps no copy of, or whose passphrase it keeps nowhere: the line
/// said in place of Make the checklist (`docs/NEW-WALLET.md` §14.4). A
/// copy on a stick counts as kept: a seed's file or SeedQR picture, its
/// passphrase's file, a vault on a stick (F31: the hold lasts until
/// that stick is written).
pub fn kept_nowhere(c: &Compute, shape: &Shape, a: &Answers, seeds: &[usize]) -> Option<String> {
    kept_nowhere_on(shape, &Mapped::of(c, shape, a), seeds)
}

/// [`kept_nowhere`] on the map `m`.
pub fn kept_nowhere_on(shape: &Shape, m: &Mapped, seeds: &[usize]) -> Option<String> {
    let boxes: &[Spot] = &m.places;
    for &i in seeds {
        let Some(seed) = shape.seeds.get(i).filter(|s| s.here) else {
            continue;
        };
        if seed_spots(boxes, i).is_empty() {
            return Some(fill(
                EN.plan_seed_is_kept_nowhere,
                &[&seed.name.to_string()],
            ));
        }
        if seed.passphrase && pass_spots(boxes, i).is_empty() {
            return Some(fill(EN.plan_the_passphrase, &[&seed.name.to_string()]));
        }
    }
    None
}

/// Costs what `of` costs. Whether the plan needs the stick of public
/// files that goes to an online computer (`docs/BACKUP-FLOW.md` §10.2):
/// sheets or shares on paper to print, the description as files on the
/// stick of files, or the software given its text form. Files kept at a
/// place go on a stick there instead (§17.2, §19.4).
pub fn public_stick(c: &Compute, shape: &Shape, a: &Answers) -> bool {
    public_stick_on(a, &Mapped::of(c, shape, a))
}

/// [`public_stick`] on the map `m`.
pub fn public_stick_on(a: &Answers, m: &Mapped) -> bool {
    let boxes: &[Spot] = &m.map;
    let software = boxes.iter().any(|s| s.at == At::Software);
    boxes.iter().any(|s| {
        (matches!(s.at, At::Place(_))
            && s.holds
                .iter()
                .any(|(w, _)| matches!(w, What::Sheet | What::Share(_))))
            || (s.at == At::Files && s.holds.iter().any(|(w, _)| *w == What::Wallet))
    }) || (software && a.form[form::TEXT])
}

/// Whether `w` is a file: kept on a stick, never on paper or in a vault.
pub fn is_file(w: What) -> bool {
    matches!(
        w,
        What::SeedFile(_)
            | What::SeedQrFile(_)
            | What::PassphraseFile(_)
            | What::WalletFile(_)
            | What::ShareFile(..)
            | What::SoftwareFile
    )
}

/// One thing the checklist asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Print the blank templates: one per paper seed copy.
    Templates,
    /// Copy seed `i` by hand and check it.
    Copy(usize),
    /// Write seed `i`'s passphrase on paper and check it
    /// (`docs/NEW-WALLET.md` §14.7 item 5).
    Passphrase(usize),
    /// Make vault `v` and save into it its seeds, and the wallet where
    /// the plan puts it there.
    Vault(usize),
    /// The seeds as files on the stick of files of a plan saved before
    /// the places kept files, past the secret sheet.
    SeedFiles,
    /// The wallet sheet or the shares, on paper.
    Sheets,
    /// The public files for the software and form chosen.
    PublicFiles,
    /// Show the descriptor to the watch-only software.
    ShowDescriptor,
    /// One envelope per place.
    Envelopes,
    /// Write to sticks: one lock, then each stick by place
    /// (`docs/BACKUP-FLOW.md` §10.2), each with the files the plan keeps
    /// on it, made then (§19.12 item 6).
    Sticks,
}

/// Costs a map of the plan: the checklist, only what the plan needs, in
/// the order to do it. A stick's files are no item of their own: they are
/// made and written with their stick (`docs/BACKUP-FLOW.md` §19.12 item
/// 6).
pub fn checklist(c: &Compute, shape: &Shape, a: &Answers) -> Vec<Item> {
    checklist_on(shape, a, &Mapped::of(c, shape, a))
}

/// [`checklist`] on the map `m`.
pub fn checklist_on(shape: &Shape, a: &Answers, m: &Mapped) -> Vec<Item> {
    let boxes: &[Spot] = &m.map;
    let at_places = |f: &dyn Fn(What) -> bool| {
        boxes
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .any(|s| s.holds.iter().any(|(w, _)| f(*w)))
    };
    let mut out = Vec::new();
    // Each seed here: its copy by hand, then its passphrase on paper.
    let mut paper = Vec::new();
    for i in shape.here() {
        if at_places(&|w| w == What::Words(i) || w == What::SeedQr(i)) {
            paper.push(Item::Copy(i));
        }
        if at_places(&|w| w == What::Passphrase(i)) {
            paper.push(Item::Passphrase(i));
        }
    }
    if paper.iter().any(|i| matches!(i, Item::Copy(_))) {
        out.push(Item::Templates);
    }
    out.extend(paper);
    out.extend(boxes.iter().filter_map(|s| match s.at {
        At::Vault(v) => Some(Item::Vault(v)),
        _ => None,
    }));
    let files = holds_in(boxes, At::Files);
    if files.iter().any(|w| matches!(w, What::SeedFile(_))) {
        out.push(Item::SeedFiles);
    }
    if at_places(&|w| matches!(w, What::Sheet | What::Share(_))) {
        out.push(Item::Sheets);
    }
    let software = boxes.iter().any(|s| s.at == At::Software);
    if files.contains(&What::Wallet) || (software && a.form[form::TEXT]) {
        out.push(Item::PublicFiles);
    }
    if software && a.form[form::QR] {
        out.push(Item::ShowDescriptor);
    }
    out.push(Item::Envelopes);
    // A stick to write: a stick at a place, the stick of files, or the
    // public files and sheets to take to an online computer.
    let stick = boxes
        .iter()
        .any(|s| matches!(s.at, At::Files | At::Stick(..)))
        || out
            .iter()
            .any(|i| matches!(i, Item::Sheets | Item::PublicFiles));
    if stick {
        out.push(Item::Sticks);
    }
    out
}

/// A form of copy by hand, as a check of it is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyForm {
    /// The words, checked by typing them back.
    Words,
    /// The SeedQR, checked by scanning the drawn grid.
    SeedQr,
    /// The passphrase on paper, checked by typing it back
    /// (`docs/NEW-WALLET.md` §14.7 item 5): not a copy of the seed.
    Passphrase,
}

/// The forms seed `i`'s copies by hand take on the map: the words, a
/// SeedQR, or both (`docs/NEW-WALLET.md` §14.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyForms {
    /// A copy of the words at a place.
    pub words: bool,
    /// A copy of the SeedQR at a place.
    pub seedqr: bool,
}

/// What checking seed `i`'s copies asks for (`docs/NEW-WALLET.md` §14.7
/// item 1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyChecks {
    /// The words are typed back.
    pub typed: bool,
    /// The drawn SeedQR is scanned.
    pub scan: bool,
}

impl CopyForms {
    /// What checking asks for. A scan needs a camera and a SeedQR the
    /// seed's list has (`english`: the list is the English one); a SeedQR
    /// copy checked another way is checked by its typed words.
    pub fn checks(self, camera: bool, english: bool) -> CopyChecks {
        let scan = self.seedqr && camera && english;
        CopyChecks {
            typed: self.words || (self.seedqr && !scan),
            scan,
        }
    }
}

/// Costs what `of` costs. The forms of seed `i`'s copies at the places
/// of the plan's map: the map's `Words(i)` and `SeedQr(i)`, whether
/// from the seeds question or from the board's placed chips and the
/// chart's edits. Builds the map.
pub fn copy_forms(c: &Compute, shape: &Shape, a: &Answers, i: usize) -> CopyForms {
    copy_forms_on(&Mapped::of(c, shape, a), i)
}

/// [`copy_forms`] on the map `m`.
pub fn copy_forms_on(m: &Mapped, i: usize) -> CopyForms {
    let boxes: &[Spot] = &m.map;
    let at = |w: What| {
        boxes
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .any(|s| s.holds.iter().any(|(h, _)| *h == w))
    };
    CopyForms {
        words: at(What::Words(i)),
        seedqr: at(What::SeedQr(i)),
    }
}

/// Costs what `of` costs. How many blank templates the plan prints: one
/// per paper copy of a seed, in every place.
pub fn templates(c: &Compute, shape: &Shape, a: &Answers) -> usize {
    templates_on(shape, &Mapped::of(c, shape, a))
}

/// [`templates`] on the map `m`.
pub fn templates_on(shape: &Shape, m: &Mapped) -> usize {
    m.map
        .iter()
        .filter(|s| matches!(s.at, At::Place(_)))
        .map(|s| {
            (0..shape.seeds.len())
                .filter(|&i| {
                    s.holds
                        .iter()
                        .any(|(w, _)| *w == What::Words(i) || *w == What::SeedQr(i))
                })
                .count()
        })
        .sum()
}

/// **Keep the words only** for seed `i` (`docs/NEW-WALLET.md` §14.7 item
/// 3): each place's SeedQR of the seed becomes its words, not added where
/// the place keeps them already, and stays on the map drawn in part
/// ([`Answers::part`]): the device cannot tell how much of the grid was
/// drawn. A plan with no edits, where no other seed keeps a SeedQR,
/// stays its answers: the seeds question's SeedQR row unticked and its
/// words row ticked. Else the map is edited and is the plan from then on
/// (§9.7). Costs a map of the plan.
pub fn keep_words(shape: &Shape, a: &Answers, i: usize) -> Answers {
    let mut out = a.clone();
    let stored = stored_map(shape, a);
    let at: Vec<usize> = stored
        .iter()
        .filter_map(|s| match s.at {
            At::Place(p) if s.holds.iter().any(|(w, _)| *w == What::SeedQr(i)) => Some(p),
            _ => None,
        })
        .collect();
    if at.is_empty() {
        return out;
    }
    let others = stored.iter().any(|s| {
        s.holds
            .iter()
            .any(|(w, _)| matches!(w, What::SeedQr(j) if *j != i))
    });
    if a.map.is_none() && !others {
        out.seeds[seeds::SEEDQR] = false;
        out.seeds[seeds::WORDS] = true;
    } else {
        let mut m = stored;
        for &p in &at {
            m = without(&m, At::Place(p), What::SeedQr(i));
            m = with_added(&m, At::Place(p), What::Words(i));
        }
        out.set_map(m);
    }
    for p in at {
        if !out.part.contains(&(p, i)) {
            out.part.push((p, i));
        }
    }
    out
}

// ---------------------------------------------------------------------
// Edits on the chart (`docs/NEW-WALLET.md` §9.7)
// ---------------------------------------------------------------------

/// A change to where things are kept, made on the wallet's chart. Each
/// makes the map itself the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// Add a thing at a spot: another copy, or a thing saved into a
    /// vault. A place past the last is a new place.
    Add(At, What),
    /// Move a thing from one spot to another; a place past the last is
    /// a new place.
    Move(At, What, At),
    /// Take a thing off the map: the copy destroyed, or removed from the
    /// plan.
    Drop(At, What),
    /// Remove place `p`, each thing in it moved to the place
    /// [`Edit::RemovePlace`]'s caller chose, or off the map.
    RemovePlace(usize),
    /// Take a stick of files, or the software, off the plan with all it
    /// holds.
    Clear(At),
}

/// The number of places on the map `boxes`.
pub fn places_of(boxes: &[Spot]) -> usize {
    boxes
        .iter()
        .filter(|s| matches!(s.at, At::Place(_)))
        .count()
}

/// Whether `w` is kept on a stick rather than on paper: a vault's stick,
/// or a file (`docs/BACKUP-FLOW.md` §19.3).
pub fn on_stick(w: What) -> bool {
    matches!(w, What::VaultStick(_)) || is_file(w)
}

/// The number of sticks place `p` keeps on the map `boxes`.
pub fn sticks_at(boxes: &[Spot], p: usize) -> usize {
    boxes
        .iter()
        .filter(|s| matches!(s.at, At::Stick(q, _) if q == p))
        .count()
}

/// The vault stick `s` at place `p` carries, on the map `boxes`.
pub fn stick_vault(boxes: &[Spot], p: usize, s: usize) -> Option<usize> {
    holds_in(boxes, At::Stick(p, s))
        .into_iter()
        .find_map(|w| match w {
            What::VaultStick(v) => Some(v),
            _ => None,
        })
}

/// The map `boxes` with each place's sticks folded into the place: what
/// a place keeps, on paper or on any stick there, each thing once. The
/// checks read a place this way: whoever finds a place finds its sticks.
pub fn by_place(boxes: &[Spot]) -> Vec<Spot> {
    let mut out: Vec<Spot> = boxes
        .iter()
        .filter(|s| !matches!(s.at, At::Stick(..)))
        .cloned()
        .collect();
    for s in boxes {
        let At::Stick(p, _) = s.at else {
            continue;
        };
        if let Some(place) = out.iter_mut().find(|x| x.at == At::Place(p)) {
            for &(w, t) in &s.holds {
                if !place.holds.iter().any(|(x, _)| *x == w) {
                    place.holds.push((w, t));
                }
            }
        }
    }
    out
}

/// Costs a map of the plan (through `map`). The map as places keep it:
/// [`map`] with each place's sticks folded into it ([`by_place`]).
pub fn place_map(c: &Compute, shape: &Shape, a: &Answers) -> Vec<Spot> {
    by_place(&map(c, shape, a))
}

/// The map `boxes` with its sticks in order (`docs/BACKUP-FLOW.md`
/// §19.4): a vault's stick or a file held by a place itself, as a plan
/// saved before the sticks keeps them, goes on a stick there, each vault
/// on a stick of its own in vault order and the files on a stick of files
/// after them; a stick that holds nothing goes; each place's sticks are
/// numbered from 0 in order and follow their place; a stick at no place
/// goes.
fn tidy_sticks(boxes: Vec<Spot>) -> Vec<Spot> {
    // Each stick's place and what it holds, in order: the sticks the map
    // has, then those a place itself held.
    let mut sticks: Vec<(usize, Holds)> = Vec::new();
    let mut moved: Vec<(usize, Holds)> = Vec::new();
    let mut ordered: Vec<(usize, Spot)> = boxes.into_iter().enumerate().collect();
    // Sticks in their own order, whatever order the spots came in.
    ordered.sort_by_key(|(n, s)| match s.at {
        At::Stick(_, k) => (1, k, *n),
        _ => (0, 0, *n),
    });
    let mut rest: Vec<Spot> = Vec::new();
    for (_, mut s) in ordered {
        match s.at {
            At::Stick(p, _) => sticks.push((p, s.holds)),
            At::Place(p) => {
                let (on, paper): (Holds, Holds) =
                    s.holds.into_iter().partition(|(w, _)| on_stick(*w));
                s.holds = paper;
                let (mut vaults, files): (Holds, Holds) = on
                    .into_iter()
                    .partition(|(w, _)| matches!(w, What::VaultStick(_)));
                vaults.sort_by_key(|(w, _)| match w {
                    What::VaultStick(v) => *v,
                    _ => 0,
                });
                moved.extend(vaults.into_iter().map(|v| (p, vec![v])));
                moved.push((p, files));
                rest.push(s);
            }
            _ => rest.push(s),
        }
    }
    sticks.extend(moved);
    sticks.retain(|(_, holds)| !holds.is_empty());
    let mut out = Vec::with_capacity(rest.len() + sticks.len());
    for s in rest {
        let at = s.at;
        out.push(s);
        if let At::Place(p) = at {
            let mine = sticks.iter().filter(|(q, _)| *q == p);
            for (k, (_, holds)) in mine.enumerate() {
                out.push(Spot {
                    at: At::Stick(p, k),
                    holds: holds.clone(),
                });
            }
        }
    }
    out
}

/// What a spot holds, each with its tag.
type Holds = Vec<(What, Tag)>;

/// Where at place `p` of the map `boxes` a thing kept on a stick goes
/// when it is added to the place itself: nowhere when a stick there
/// keeps it already; a vault's stick on a stick of its own; a file on
/// the place's first stick with no vault (its stick of files), or a new
/// one.
fn stick_for(boxes: &[Spot], p: usize, w: What) -> Option<usize> {
    let n = sticks_at(boxes, p);
    if (0..n).any(|s| holds_in(boxes, At::Stick(p, s)).contains(&w)) {
        return None;
    }
    if matches!(w, What::VaultStick(_)) {
        return Some(n);
    }
    Some(
        (0..n)
            .find(|&s| stick_vault(boxes, p, s).is_none())
            .unwrap_or(n),
    )
}

/// The map `boxes` with `what` added at `at`: into its spot, or a new
/// spot made for it (a new place after the last, a stick, a stick of
/// files, a vault). Nothing is added twice to one spot. A thing kept on a
/// stick added to a place goes on a stick there ([`stick_for`]); a paper
/// form added to a stick goes to its place; a vault's stick added to a
/// stick that carries another vault goes on a new stick there.
pub fn with_added(boxes: &[Spot], at: At, what: What) -> Vec<Spot> {
    // A passphrase in a vault (`docs/BACKUP-FLOW.md` §19.12 item 4): with
    // its seed, in the key record; without it, a passphrase record. A seed
    // added where its passphrase is alone takes it into its record.
    if let At::Vault(_) = at {
        let held = holds_in(boxes, at);
        match what {
            What::Passphrase(i) | What::PassphraseFile(i) | What::PassphraseSealed(i) => {
                if held.contains(&What::SeedPassphrase(i)) {
                    return boxes.to_vec();
                }
                if held.contains(&What::Seed(i)) {
                    let m = without(boxes, at, What::Seed(i));
                    return with_added(&m, at, What::SeedPassphrase(i));
                }
                if what != What::PassphraseSealed(i) {
                    return with_added(boxes, at, What::PassphraseSealed(i));
                }
            }
            What::Seed(i) if held.contains(&What::PassphraseSealed(i)) => {
                let m = without(boxes, at, What::PassphraseSealed(i));
                return with_added(&m, at, What::SeedPassphrase(i));
            }
            _ => {}
        }
    }
    let mut out = boxes.to_vec();
    let places = places_of(&out);
    let at = match at {
        At::Place(p) if p >= places => At::Place(places),
        At::Stick(p, _) if p >= places => At::Stick(places, 0),
        at => at,
    };
    let at = match (at, what) {
        (At::Place(p), w) if on_stick(w) => {
            ensure_place(&mut out, p);
            match stick_for(&out, p, w) {
                Some(s) => At::Stick(p, s),
                None => return out,
            }
        }
        (At::Stick(p, _), w) if !on_stick(w) => At::Place(p),
        (At::Stick(p, s), What::VaultStick(v)) => {
            let n = sticks_at(&out, p);
            match stick_vault(&out, p, s.min(n)) {
                Some(u) if u != v => At::Stick(p, n),
                _ => At::Stick(p, s.min(n)),
            }
        }
        (At::Stick(p, s), _) => At::Stick(p, s.min(sticks_at(&out, p))),
        (at, _) => at,
    };
    if let At::Place(p) | At::Stick(p, _) = at {
        ensure_place(&mut out, p);
    }
    let pos = match out.iter().position(|s| s.at == at) {
        Some(i) => i,
        None => {
            // A new spot: a place after the last place, a stick after its
            // place's, anything else after the places and vaults.
            let after = out
                .iter()
                .rposition(|s| match at {
                    At::Place(_) => matches!(s.at, At::Place(_) | At::Stick(..)),
                    At::Stick(p, _) => matches!(s.at, At::Place(q) | At::Stick(q, _) if q == p),
                    _ => matches!(s.at, At::Place(_) | At::Stick(..) | At::Vault(_)),
                })
                .map_or(0, |i| i + 1);
            out.insert(
                after,
                Spot {
                    at,
                    holds: Vec::new(),
                },
            );
            after
        }
    };
    if !out[pos].holds.iter().any(|(w, _)| *w == what) {
        out[pos].holds.push((what, tag_of(at, what)));
    }
    tidy_sticks(out)
}

/// Place `p` on the map `boxes`, made (empty, after the last place) when
/// it is the next place.
fn ensure_place(boxes: &mut Vec<Spot>, p: usize) {
    if p == places_of(boxes) {
        let after = boxes
            .iter()
            .rposition(|s| matches!(s.at, At::Place(_) | At::Stick(..)))
            .map_or(0, |i| i + 1);
        boxes.insert(
            after,
            Spot {
                at: At::Place(p),
                holds: Vec::new(),
            },
        );
    }
}

/// The map `boxes` with vault `v`'s stick put at place `p` as the board
/// drops it (`docs/BACKUP-FLOW.md` §19.2, §19.12 item 7): on the place's
/// first stick with no vault, or on a new stick when every stick there
/// carries one, its own included; so a place that keeps the vault already
/// gets a second stick of it.
pub fn with_vault_stick(boxes: &[Spot], p: usize, v: usize) -> Vec<Spot> {
    let p = p.min(places_of(boxes));
    let n = sticks_at(boxes, p);
    let s = (0..n)
        .find(|&s| stick_vault(boxes, p, s).is_none())
        .unwrap_or(n);
    with_added(boxes, At::Stick(p, s), What::VaultStick(v))
}

/// The map `boxes` with `what` taken off spot `at`. A thing kept on a
/// stick taken off a place goes from every stick there. A stick, a stick
/// of files or software left holding nothing goes; a place stays, empty.
pub fn without(boxes: &[Spot], at: At, what: What) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    for s in out.iter_mut() {
        let here = match (at, s.at) {
            (At::Place(p), At::Stick(q, _)) => p == q && on_stick(what),
            (a, b) => a == b,
        };
        if here {
            s.holds.retain(|(w, _)| *w != what);
        }
    }
    out.retain(|s| !(s.holds.is_empty() && matches!(s.at, At::Files | At::Software | At::Away)));
    tidy_sticks(out)
}

/// The map `boxes` with place `p` removed, its sticks with it: each thing
/// it held goes to the place `dest` names for it (by the place's number
/// before the removal), or off the map; the places after it move down
/// one.
pub fn without_place(boxes: &[Spot], p: usize, dest: &[(What, Option<usize>)]) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    for &(what, to) in dest {
        if let Some(to) = to.filter(|&to| to != p) {
            out = with_added(&out, At::Place(to), what);
        }
    }
    out.retain(|s| !matches!(s.at, At::Place(q) | At::Stick(q, _) if q == p));
    for s in &mut out {
        match s.at {
            At::Place(q) if q > p => s.at = At::Place(q - 1),
            At::Stick(q, k) if q > p => s.at = At::Stick(q - 1, k),
            _ => {}
        }
    }
    tidy_sticks(out)
}

/// The map `boxes` with `edit` made; for [`Edit::RemovePlace`], `dest`
/// says where each thing goes.
pub fn edited(boxes: &[Spot], edit: Edit, dest: &[(What, Option<usize>)]) -> Vec<Spot> {
    match edit {
        Edit::Add(at, what) => with_added(boxes, at, what),
        Edit::Move(from, what, to) => with_added(&without(boxes, from, what), to, what),
        Edit::Drop(at, what) => without(boxes, at, what),
        Edit::RemovePlace(p) => without_place(boxes, p, dest),
        Edit::Clear(at) => tidy_sticks(boxes.iter().filter(|s| s.at != at).cloned().collect()),
    }
}

// ---------------------------------------------------------------------
// The plan by place (`docs/BACKUP-FLOW.md` §17)
// ---------------------------------------------------------------------

/// A row of a place's card: what the place keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// Seed words by hand.
    Words,
    /// SeedQR by hand.
    SeedQr,
    /// Seed as a file, unprotected, on the place's stick of files.
    File,
    /// A seed's passphrase.
    Passphrase,
    /// The stick of vault `v`.
    Vault(usize),
}

impl Row {
    /// The row as an action carries it: 0 to 3, then 16 and up for the
    /// vaults.
    pub fn code(self) -> u8 {
        match self {
            Row::Words => 0,
            Row::SeedQr => 1,
            Row::File => 2,
            Row::Passphrase => 3,
            Row::Vault(v) => 16u8.saturating_add(v.min(239) as u8),
        }
    }

    /// [`Row::code`] read back.
    pub fn from_code(c: u8) -> Option<Row> {
        Some(match c {
            0 => Row::Words,
            1 => Row::SeedQr,
            2 => Row::File,
            3 => Row::Passphrase,
            c if c >= 16 => Row::Vault(usize::from(c - 16)),
            _ => return None,
        })
    }

    /// What the row holds of seed `i` at a place.
    fn what(self, i: usize) -> Option<What> {
        match self {
            Row::Words => Some(What::Words(i)),
            Row::SeedQr => Some(What::SeedQr(i)),
            Row::File => Some(What::SeedFile(i)),
            Row::Passphrase => Some(What::Passphrase(i)),
            Row::Vault(_) => None,
        }
    }
}

/// A form the wallet description takes at a place
/// (`docs/BACKUP-FLOW.md` §17.1, Where the description goes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// The sheet, or the shares, on paper.
    Paper,
    /// The public files, or the shares' files, on the place's stick of
    /// files.
    File,
    /// In vault `v`, whose stick the place keeps.
    Vault(usize),
}

impl Form {
    /// The form as an action carries it: 0, 1, then 16 and up.
    pub fn code(self) -> u8 {
        match self {
            Form::Paper => 0,
            Form::File => 1,
            Form::Vault(v) => 16u8.saturating_add(v.min(239) as u8),
        }
    }

    /// [`Form::code`] read back.
    pub fn from_code(c: u8) -> Option<Form> {
        Some(match c {
            0 => Form::Paper,
            1 => Form::File,
            c if c >= 16 => Form::Vault(usize::from(c - 16)),
            _ => return None,
        })
    }
}

impl Answers {
    /// The map the cards edit: from the first answer on a card after the
    /// presets, the plan itself (`docs/BACKUP-FLOW.md` §17.2).
    pub fn ensure_map(&mut self, shape: &Shape) {
        if self.map.is_none() {
            let m = stored_map(shape, self);
            self.set_map(m);
        }
    }

    /// The map with `f` made to it, as the plan.
    fn edit_map(&mut self, shape: &Shape, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
        self.ensure_map(shape);
        let m = f(self.map.take().unwrap_or_default());
        self.set_map(m);
    }

    /// The vaults a place's card offers: one per vault of the plan
    /// (`Shape::vault_rows`), and any further one the map names.
    pub fn vault_count(&self, shape: &Shape) -> usize {
        let named = stored_map(shape, self)
            .iter()
            .filter_map(|s| match s.at {
                At::Vault(v) => Some(v + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        shape.vault_rows().max(named)
    }

    /// Costs what `of` costs. What row `row` of place `p`'s card holds: the
    /// seeds, by place among the wallet's; `None` while it is not ticked. A
    /// vault's row is ticked while the place keeps its stick, and holds the
    /// vault's seeds.
    pub fn row_seeds(&self, c: &Compute, shape: &Shape, p: usize, row: Row) -> Option<Vec<usize>> {
        Mapped::of(c, shape, self).row_seeds(shape, p, row)
    }

    /// Costs what `of` costs. The shares vault `v` keeps
    /// (`docs/BACKUP-FLOW.md` §17.6).
    pub fn vault_shares(&self, c: &Compute, shape: &Shape, v: usize) -> Vec<usize> {
        Mapped::of(c, shape, self).vault_shares(v)
    }

    /// Costs what `of` costs. The seeds whose passphrase vault `v` keeps
    /// alone, without the seed: each a passphrase record
    /// (`docs/BACKUP-FLOW.md` §19.12 item 4).
    pub fn vault_passphrases(&self, c: &Compute, shape: &Shape, v: usize) -> Vec<usize> {
        Mapped::of(c, shape, self).vault_passphrases(v)
    }

    /// Costs what `of` costs. The seeds vault `v` keeps with their
    /// passphrase.
    pub fn vault_pass(&self, c: &Compute, shape: &Shape, v: usize) -> Vec<usize> {
        Mapped::of(c, shape, self).vault_pass(v)
    }

    /// Costs what `of` costs. The places other than `p` that keep vault
    /// `v`'s stick.
    pub fn stick_elsewhere(&self, c: &Compute, shape: &Shape, v: usize, p: usize) -> Vec<usize> {
        Mapped::of(c, shape, self).stick_elsewhere(v, p)
    }

    /// Row `row` of place `p`'s card as its sheet's **Done** leaves it:
    /// `seeds` kept there in that form, and for a vault `pass`, the seeds
    /// it keeps with their passphrase. A vault is one vault wherever its
    /// stick is kept: what it holds is set for every place. No seed: the
    /// row unticked ([`Answers::clear_row`]); a vault's row stays ticked,
    /// its stick at the place, the vault holding no seed (what goes into
    /// a vault is the person's choice: the description, shares, or
    /// nothing yet).
    pub fn set_row(&mut self, shape: &Shape, p: usize, row: Row, seeds: &[usize], pass: &[usize]) {
        let here = |i: &usize| shape.seeds.get(*i).is_some_and(|s| s.here);
        let seeds: Vec<usize> = seeds.iter().copied().filter(here).collect();
        if seeds.is_empty() && !matches!(row, Row::Vault(_)) {
            return self.clear_row(shape, p, row);
        }
        if let Row::Vault(v) = row {
            self.keep_vault_at(shape, p, v);
        }
        let at = At::Place(p);
        self.edit_map(shape, |mut m| {
            match row {
                Row::Vault(v) => {
                    let old: Vec<What> = holds_in(&m, At::Vault(v))
                        .into_iter()
                        .filter(|w| matches!(w, What::Seed(_) | What::SeedPassphrase(_)))
                        .collect();
                    for w in old {
                        m = without(&m, At::Vault(v), w);
                    }
                    for &i in &seeds {
                        let with = pass.contains(&i) && shape.seeds[i].passphrase;
                        let w = if with {
                            What::SeedPassphrase(i)
                        } else {
                            What::Seed(i)
                        };
                        m = with_added(&m, At::Vault(v), w);
                    }
                    m = with_added(&m, at, What::VaultStick(v));
                }
                row => {
                    for i in 0..shape.seeds.len() {
                        if let Some(w) = row.what(i) {
                            m = without(&m, at, w);
                        }
                    }
                    let held = holds_in(&by_place(&m), at);
                    for &i in &seeds {
                        // A passphrase never shares a place with its
                        // words, nor its words with it, on paper or on a
                        // stick there: the one kept there first stays, and
                        // the other is not added.
                        let words = held.iter().any(|w| seed_form(*w) == Some(i));
                        let pass = held.contains(&What::Passphrase(i))
                            || held.contains(&What::PassphraseFile(i));
                        match row {
                            Row::Passphrase if words => continue,
                            Row::Words | Row::SeedQr if pass => continue,
                            _ => {}
                        }
                        if let Some(w) = row.what(i) {
                            m = with_added(&m, at, w);
                        }
                    }
                }
            }
            m
        });
    }

    /// Row `row` of place `p`'s card unticked: what it held at the place
    /// goes. For a vault, its stick at this place; the vault keeps its
    /// seeds while another place keeps its stick, and goes with its last.
    pub fn clear_row(&mut self, shape: &Shape, p: usize, row: Row) {
        let at = At::Place(p);
        self.edit_map(shape, |mut m| {
            match row {
                Row::Vault(v) => {
                    m = without(&m, at, What::VaultStick(v));
                    if sticks_of(&m, v).is_empty() {
                        m.retain(|s| s.at != At::Vault(v));
                    }
                }
                row => {
                    for i in 0..shape.seeds.len() {
                        if let Some(w) = row.what(i) {
                            m = without(&m, at, w);
                        }
                    }
                }
            }
            m
        });
    }

    /// Vault `v`'s stick kept at place `p`, the vault made where the plan
    /// has none yet.
    pub fn keep_vault_at(&mut self, shape: &Shape, p: usize, v: usize) {
        self.edit_map(shape, |m| {
            with_added(&with_vault(m, v), At::Place(p), What::VaultStick(v))
        });
    }

    /// Vault `v`'s stick dropped on place `p` as the board drops it
    /// ([`with_vault_stick`]): on the place's first stick with no vault,
    /// else a new stick, so a place may keep two sticks of one vault
    /// (`docs/BACKUP-FLOW.md` §19.12 item 7). The vault is made where the
    /// plan has none yet.
    pub fn add_vault_stick(&mut self, shape: &Shape, p: usize, v: usize) {
        self.edit_map(shape, |m| with_vault_stick(&with_vault(m, v), p, v));
    }

    /// Vault `v` taken off the plan, every stick of it with it
    /// (`docs/BACKUP-FLOW.md` §19.12 item 7): a stick that carried only
    /// the vault goes, one with files beside it stays a stick of files.
    pub fn remove_vault(&mut self, shape: &Shape, v: usize) {
        self.edit_map(shape, |m| {
            let places = places_of(&m);
            let mut out: Vec<Spot> = m.into_iter().filter(|s| s.at != At::Vault(v)).collect();
            for p in 0..places {
                out = without(&out, At::Place(p), What::VaultStick(v));
            }
            out
        });
    }

    /// The first vault of the map that holds nothing: no seed, no
    /// description, no share. It holds The plan until it holds something
    /// or goes.
    pub fn empty_vault(&self) -> Option<usize> {
        let m = self.map.as_ref()?;
        // Only a vault kept at a place (`docs/BACKUP-FLOW.md` §19.16
        // item 3): one just added by **+ vault** holds nothing back.
        let kept = kept_vaults(m);
        m.iter().find_map(|s| match s.at {
            At::Vault(v) if s.holds.is_empty() && kept.contains(&v) => Some(v),
            _ => None,
        })
    }

    /// Another place, empty, after the last.
    pub fn add_place(&mut self, shape: &Shape) {
        self.edit_map(shape, |mut m| {
            let n = places_of(&m);
            let after = m
                .iter()
                .rposition(|s| matches!(s.at, At::Place(_)))
                .map_or(0, |i| i + 1);
            m.insert(
                after,
                Spot {
                    at: At::Place(n),
                    holds: Vec::new(),
                },
            );
            m
        });
    }

    /// Place `p` removed with what it held, its sticks with it
    /// (`Edit::RemovePlace` with no destination); the places after it
    /// move down one. One place stays (`docs/BACKUP-FLOW.md` §19.1).
    pub fn remove_place(&mut self, shape: &Shape, p: usize) {
        self.ensure_map(shape);
        let n = self.map.as_deref().map_or(0, places_of);
        if n > 1 && p < n {
            self.edit_map(shape, |m| edited(&m, Edit::RemovePlace(p), &[]));
        }
    }

    /// A vault more, holding nothing and kept at no place yet: the board's
    /// **+ vault** (`docs/BACKUP-FLOW.md` §19.1). Its number is the one
    /// after the plan's last vault and the vaults set up before the keys.
    pub fn add_vault(&mut self, shape: &Shape) -> usize {
        let named = stored_map(shape, self)
            .iter()
            .filter_map(|s| match s.at {
                At::Vault(v) => Some(v + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let v = named.max(shape.vaults);
        self.edit_map(shape, |m| with_vault(m, v));
        v
    }

    /// The last place removed, with what it held; one place stays.
    pub fn remove_last_place(&mut self, shape: &Shape) {
        self.ensure_map(shape);
        let n = self.map.as_deref().map_or(0, places_of);
        if n > 1 {
            self.edit_map(shape, |m| without_place(&m, n - 1, &[]));
        }
    }

    /// The description whole (`false`) or in shares: what each place and
    /// vault keeps of it follows, the sheet becoming the shares the place
    /// would keep, and back.
    pub fn set_split(&mut self, shape: &Shape, split: bool) {
        if split && !shape.splits {
            return;
        }
        self.ensure_map(shape);
        if self.split != split {
            let keys = shape.keys.max(1);
            self.edit_map(shape, |m| {
                let places = places_of(&m);
                let mut out = m.clone();
                for s in &m {
                    // The place a vault's shares are spread as: its first
                    // stick's.
                    let p = match s.at {
                        At::Place(p) | At::Stick(p, _) => p,
                        At::Vault(v) => sticks_of(&m, v).first().copied().unwrap_or(0),
                        _ => continue,
                    };
                    for (w, _) in &s.holds {
                        let to: Vec<What> = match (split, *w) {
                            (true, What::Sheet | What::Wallet) => spread(keys, places, p)
                                .into_iter()
                                .map(What::Share)
                                .collect(),
                            (true, What::WalletFile(f)) => spread(keys, places, p)
                                .into_iter()
                                .map(|j| What::ShareFile(j, f))
                                .collect(),
                            (false, What::Share(_)) if matches!(s.at, At::Vault(_)) => {
                                vec![What::Wallet]
                            }
                            (false, What::Share(_)) => vec![What::Sheet],
                            (false, What::ShareFile(_, f)) => vec![What::WalletFile(f)],
                            _ => continue,
                        };
                        out = without(&out, s.at, *w);
                        for t in to {
                            out = with_added(&out, s.at, t);
                        }
                    }
                }
                out
            });
        }
        self.split = split;
    }

    /// Costs what `of` costs. The shares place `p` keeps, in any form: on
    /// paper, as files, or in a vault whose stick it keeps.
    pub fn shares_here(&self, c: &Compute, shape: &Shape, p: usize) -> Vec<usize> {
        Mapped::of(c, shape, self).shares_here(p)
    }

    /// Costs what `of` costs. Whether place `p` keeps the description in
    /// form `form`.
    pub fn form_at(&self, c: &Compute, shape: &Shape, p: usize, form: Form) -> bool {
        Mapped::of(c, shape, self).form_at(p, form)
    }

    /// The forms the description's files take where Where the
    /// description goes adds them (`docs/BACKUP-FLOW.md` §19.3): each the
    /// `form` answers have on, the QR picture when none is.
    fn file_forms(&self) -> Vec<usize> {
        let on: Vec<usize> = [form::QR, form::TEXT]
            .into_iter()
            .filter(|&f| self.form[f])
            .collect();
        if on.is_empty() { vec![form::QR] } else { on }
    }

    /// What form `form` holds at place `p`, in shares: share `j` there, a
    /// file in each of [`Answers::file_forms`].
    fn form_share(&self, at: At, form: Form, j: usize) -> Vec<(At, What)> {
        match form {
            Form::Paper => vec![(at, What::Share(j))],
            Form::File => self
                .file_forms()
                .into_iter()
                .map(|f| (at, What::ShareFile(j, f)))
                .collect(),
            Form::Vault(v) => vec![(At::Vault(v), What::Share(j))],
        }
    }

    /// Costs what `form_at` and `shares_here` cost. Ticks or unticks form
    /// `form` at place `p`. Whole, the form holds the description; in
    /// shares, every share ticked there, or with none ticked the share the
    /// place would keep.
    pub fn toggle_form(&mut self, c: &Compute, shape: &Shape, p: usize, form: Form) {
        let on = !self.form_at(c, shape, p, form);
        let shares = self.split && shape.splits;
        let mut here = self.shares_here(c, shape, p);
        if here.is_empty() {
            let places = self.places.max(1);
            here = spread(shape.keys.max(1), places, p);
        }
        let at = At::Place(p);
        let files: Vec<usize> = if on {
            self.file_forms()
        } else {
            vec![form::QR, form::TEXT]
        };
        let adds: Vec<(At, What)> = here
            .iter()
            .flat_map(|&j| self.form_share(at, form, j))
            .collect();
        self.edit_map(shape, |mut m| {
            if !shares {
                let things: Vec<(At, What)> = match form {
                    Form::Paper => vec![(at, What::Sheet)],
                    Form::File => files.iter().map(|&f| (at, What::WalletFile(f))).collect(),
                    Form::Vault(v) => vec![(At::Vault(v), What::Wallet)],
                };
                for (spot, w) in things {
                    m = if on {
                        with_added(&m, spot, w)
                    } else {
                        without(&m, spot, w)
                    };
                }
                return m;
            }
            if on {
                for (spot, w) in adds {
                    m = with_added(&m, spot, w);
                }
            } else {
                let spot = match form {
                    Form::Vault(v) => At::Vault(v),
                    _ => at,
                };
                let gone: Vec<What> = holds_in(&by_place(&m), spot)
                    .into_iter()
                    .filter(|w| match form {
                        Form::File => matches!(w, What::ShareFile(..)),
                        _ => matches!(w, What::Share(_)),
                    })
                    .collect();
                for w in gone {
                    m = without(&m, spot, w);
                }
            }
            m
        });
    }

    /// Costs what `shares_here` and `form_at` cost. Ticks or unticks share
    /// `j` at place `p`: in every form ticked there, or on paper while none
    /// is.
    pub fn toggle_share(&mut self, c: &Compute, shape: &Shape, p: usize, j: usize) {
        if j >= shape.keys {
            return;
        }
        let on = !self.shares_here(c, shape, p).contains(&j);
        let mut forms: Vec<Form> = [Form::Paper, Form::File]
            .into_iter()
            .chain((0..self.vault_count(shape)).map(Form::Vault))
            .filter(|&f| self.form_at(c, shape, p, f))
            .collect();
        if forms.is_empty() {
            forms.push(Form::Paper);
        }
        let at = At::Place(p);
        let adds: Vec<(At, What)> = forms
            .iter()
            .flat_map(|&f| self.form_share(at, f, j))
            .collect();
        self.edit_map(shape, |mut m| {
            if on {
                for (spot, w) in adds {
                    m = with_added(&m, spot, w);
                }
            } else {
                m = without(&m, at, What::Share(j));
                m = without(&m, at, What::ShareFile(j, form::QR));
                m = without(&m, at, What::ShareFile(j, form::TEXT));
                for f in forms {
                    if let Form::Vault(v) = f {
                        m = without(&m, At::Vault(v), What::Share(j));
                    }
                }
            }
            m
        });
    }

    /// Costs what `of` costs. What a plan saved before the cards keeps on
    /// the stick of files (`At::Files`), in its order.
    pub fn old_files(&self, c: &Compute, shape: &Shape) -> Vec<What> {
        Mapped::of(c, shape, self).old_files()
    }

    /// Costs what `old_files` costs. The `k`th thing of
    /// [`Answers::old_files`] off the map; the stick of files goes with the
    /// last.
    pub fn drop_old_file(&mut self, c: &Compute, shape: &Shape, k: usize) {
        if let Some(&w) = self.old_files(c, shape).get(k) {
            self.edit_map(shape, |m| without(&m, At::Files, w));
        }
    }

    /// Costs what `of` costs. Whether the description goes into watch-only
    /// software.
    pub fn software_on(&self, c: &Compute, shape: &Shape) -> bool {
        Mapped::of(c, shape, self).software_on()
    }

    /// Costs what `software_on` costs. Into watch-only software, ticked or
    /// unticked.
    pub fn toggle_software(&mut self, c: &Compute, shape: &Shape) {
        let on = !self.software_on(c, shape);
        self.edit_map(shape, |m| {
            if on {
                with_added(&m, At::Software, What::Wallet)
            } else {
                edited(&m, Edit::Clear(At::Software), &[])
            }
        });
    }

    /// Costs what `of` costs. The software and the files' form are asked:
    /// the description goes into watch-only software, or as files
    /// somewhere.
    pub fn asks_software(&self, c: &Compute, shape: &Shape) -> bool {
        Mapped::of(c, shape, self).asks_software()
    }
}

impl Mapped {
    /// [`Answers::asks_software`] on this map.
    pub fn asks_software(&self) -> bool {
        self.map.iter().any(|s| {
            s.at == At::Software
                || s.holds.iter().any(|(w, _)| {
                    matches!(
                        w,
                        What::WalletFile(_) | What::ShareFile(..) | What::SoftwareFile
                    ) || (s.at == At::Files && *w == What::Wallet)
                })
        })
    }
}

/// The map `m` with vault `v` in it, empty where it had none.
pub fn with_vault(mut m: Vec<Spot>, v: usize) -> Vec<Spot> {
    if !m.iter().any(|s| s.at == At::Vault(v)) {
        let after = m
            .iter()
            .rposition(|s| matches!(s.at, At::Place(_) | At::Stick(..) | At::Vault(_)))
            .map_or(0, |i| i + 1);
        m.insert(
            after,
            Spot {
                at: At::Vault(v),
                holds: Vec::new(),
            },
        );
    }
    m
}

/// The seeds vault `v` holds on the map `boxes`.
fn vault_seeds_in(boxes: &[Spot], v: usize) -> Vec<usize> {
    holds_in(boxes, At::Vault(v))
        .into_iter()
        .filter_map(|w| match w {
            What::Seed(i) | What::SeedPassphrase(i) => Some(i),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// What is kept with the plan, in the vault alone (§9.5)
// ---------------------------------------------------------------------

/// What a thing of the map is marked: lost, or found by someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Lost: gone.
    Lost,
    /// Exposed: someone has seen it, or has it.
    Exposed,
    /// A SeedQR drawn in part and dropped for the words
    /// (`docs/NEW-WALLET.md` §14.7 item 3): kept by the plan's answers
    /// ([`Answers::part`]), not as a note.
    Part,
}

impl Mark {
    /// The mark as the chart writes it.
    pub fn name(self) -> &'static str {
        match self {
            Mark::Lost => "lost",
            Mark::Exposed => "exposed",
            Mark::Part => "part",
        }
    }

    /// The mark as a screen shows it.
    pub fn label(self) -> &'static str {
        match self {
            Mark::Lost => EN.plan_mark_lost,
            Mark::Exposed => EN.plan_mark_exposed,
            Mark::Part => EN.plan_mark_drawn_in_part,
        }
    }
}

/// One note the plan's record keeps on a thing of its map, in the vault
/// alone: a date it was checked here, a mark, a key's holder, a vault's
/// name. Never a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// The thing at a spot was last checked here on this date
    /// (`YYYY-MM-DD`, empty when the clock was not known).
    Checked(At, What, String),
    /// The key with this fingerprint was confirmed backed up by its
    /// holder on this date.
    KeyChecked([u8; 4], String),
    /// The thing at a spot is marked.
    Marked(At, What, Mark),
    /// Who holds the key with this fingerprint: "Alice's Coldcard".
    Holder([u8; 4], String),
    /// What the plan's vault `v` is called.
    VaultName(usize, String),
}

fn fp_code(fp: [u8; 4]) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

fn fp_read(s: &str) -> Option<[u8; 4]> {
    if s.len() != 8 {
        return None;
    }
    let mut out = [0u8; 4];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

impl Note {
    /// The note as the plan's record keeps it, a line: `checked place0 w0
    /// 2026-10-10`, `lost place0 w0`, `holder 9a6a2580 Alice's Coldcard`.
    pub fn to_text(&self) -> String {
        let clean = |s: &str| s.replace(['\n', '\r', '\t'], " ");
        match self {
            Note::Checked(at, w, d) => {
                format!("checked {} {} {}", at_code(*at), what_code(*w), clean(d))
                    .trim_end()
                    .to_string()
            }
            Note::KeyChecked(fp, d) => format!("keychecked {} {}", fp_code(*fp), clean(d))
                .trim_end()
                .to_string(),
            Note::Marked(at, w, m) => format!("{} {} {}", m.name(), at_code(*at), what_code(*w)),
            Note::Holder(fp, n) => format!("holder {} {}", fp_code(*fp), clean(n)),
            Note::VaultName(v, n) => format!("vaultname {v} {}", clean(n)),
        }
    }

    /// [`Note::to_text`] read back.
    pub fn from_text(s: &str) -> Option<Note> {
        let mut parts = s.splitn(4, ' ');
        let kind = parts.next()?;
        let a = parts.next()?;
        let b = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        let after = |n: usize| s.splitn(n + 1, ' ').nth(n).unwrap_or("").to_string();
        Some(match kind {
            "checked" => Note::Checked(at_read(a)?, what_read(b)?, rest.to_string()),
            "keychecked" => Note::KeyChecked(fp_read(a)?, after(2)),
            "lost" => Note::Marked(at_read(a)?, what_read(b)?, Mark::Lost),
            "exposed" => Note::Marked(at_read(a)?, what_read(b)?, Mark::Exposed),
            "holder" => Note::Holder(fp_read(a)?, after(2)),
            "vaultname" => Note::VaultName(a.parse().ok()?, after(2)),
            _ => return None,
        })
    }

    /// The thing of the map it is on, when it is on one.
    pub fn on(&self) -> Option<(At, What)> {
        match self {
            Note::Checked(at, w, _) | Note::Marked(at, w, _) => Some((*at, *w)),
            _ => None,
        }
    }
}

/// The notes as they follow an edit of the map: a thing dropped loses
/// its notes, a thing moved takes them with it, and a place removed
/// takes its things' notes to where they went, the places after it
/// numbered down one.
pub fn notes_after(notes: &[Note], edit: Edit, dest: &[(What, Option<usize>)]) -> Vec<Note> {
    let moved = |at: At, w: What| -> Option<At> {
        match edit {
            Edit::Drop(a, x) if a == at && x == w => None,
            Edit::Clear(a) if a == at => None,
            Edit::Move(a, x, to) if a == at && x == w => Some(to),
            Edit::RemovePlace(p) => match at {
                At::Place(q) if q == p => dest
                    .iter()
                    .find(|(x, _)| *x == w)
                    .and_then(|(_, to)| *to)
                    .filter(|&to| to != p)
                    .map(|to| At::Place(if to > p { to - 1 } else { to })),
                At::Place(q) if q > p => Some(At::Place(q - 1)),
                at => Some(at),
            },
            _ => Some(at),
        }
    };
    notes
        .iter()
        .filter_map(|n| match n {
            Note::Checked(at, w, d) => Some(Note::Checked(moved(*at, *w)?, *w, d.clone())),
            Note::Marked(at, w, m) => Some(Note::Marked(moved(*at, *w)?, *w, *m)),
            other => Some(other.clone()),
        })
        .collect()
}

// ---------------------------------------------------------------------
// Lost and exposed (§9.6)
// ---------------------------------------------------------------------

/// What follows from what is marked lost and exposed on a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Follows {
    /// What whoever has every exposed thing can do with it.
    pub found: Alone,
    /// Whether the wallet rebuilds from what is not lost.
    pub rebuilds: Lost,
    /// Every lost thing has a copy elsewhere on the map.
    pub elsewhere: bool,
    /// Something is lost.
    pub any_lost: bool,
}

/// The map `boxes` without `gone`; a vault every stick of which is gone
/// goes with them.
fn without_all(boxes: &[Spot], gone: &[(At, What)]) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    for s in &mut out {
        s.holds.retain(|(w, _)| !gone.contains(&(s.at, *w)));
    }
    let lost_vaults: Vec<usize> = boxes
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) if !sticks_of(boxes, v).is_empty() && sticks_of(&out, v).is_empty() => {
                Some(v)
            }
            _ => None,
        })
        .collect();
    out.retain(|s| !matches!(s.at, At::Vault(v) if lost_vaults.contains(&v)));
    out
}

/// Costs what `of` costs. What follows on the plan `a` from the things
/// `marks` lists: the lost ones gone, the exposed ones in one finder's
/// hands.
pub fn follows(c: &Compute, shape: &Shape, a: &Answers, marks: &[(At, What, Mark)]) -> Follows {
    follows_on(shape, a, &Mapped::of(c, shape, a), marks)
}

/// [`follows`] on the map `m`.
pub fn follows_on(
    shape: &Shape,
    a: &Answers,
    mapped: &Mapped,
    marks: &[(At, What, Mark)],
) -> Follows {
    let boxes: &[Spot] = &mapped.places;
    let lost: Vec<(At, What)> = marks
        .iter()
        .filter(|m| m.2 == Mark::Lost)
        .map(|m| (m.0, m.1))
        .collect();
    let exposed: Vec<(What, Tag)> = marks
        .iter()
        .filter(|m| m.2 == Mark::Exposed)
        .map(|m| (m.1, tag_of(m.0, m.1)))
        .collect();
    let left = without_all(boxes, &lost);
    let rebuilds = lost_one(shape, a, &left, &spots(&left), None);
    // Whoever found the exposed things holds them as one place would.
    let finder = At::Place(usize::MAX);
    let mut with = boxes.to_vec();
    with.push(Spot {
        at: finder,
        holds: exposed
            .iter()
            .filter(|(w, _)| !matches!(w, What::Seed(_) | What::SeedPassphrase(_)))
            .copied()
            .collect(),
    });
    let mut found = alone_in(shape, a, &with, finder);
    // A seed sealed in a vault, exposed: the vault's passphrase guards it.
    let sealed: Vec<What> = exposed
        .iter()
        .map(|(w, _)| *w)
        .filter(|w| matches!(w, What::Seed(_) | What::SeedPassphrase(_)))
        .collect();
    if !sealed.is_empty() {
        let mut have = Have::none(shape);
        holds_in(&with, finder)
            .into_iter()
            .chain(sealed)
            .for_each(|w| have.add(w));
        let spend =
            have.seeds_ok(shape) >= shape.m.max(1) && (!shape.multi() || have.wallet_ok(shape, a));
        if spend && found.spend == Found::No {
            found.spend = Found::OnlyWithVault;
        }
        if have.sees(shape, a) && found.balance == Found::No {
            found.balance = Found::OnlyWithVault;
        }
    }
    let same = |x: What, y: What| match (x, y) {
        (
            What::Passphrase(i) | What::PassphraseFile(i) | What::PassphraseSealed(i),
            What::Passphrase(j)
            | What::PassphraseFile(j)
            | What::PassphraseSealed(j)
            | What::SeedPassphrase(j),
        ) => i == j,
        (
            What::Words(i)
            | What::SeedQr(i)
            | What::Seed(i)
            | What::SeedPassphrase(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i),
            What::Words(j)
            | What::SeedQr(j)
            | What::Seed(j)
            | What::SeedPassphrase(j)
            | What::SeedFile(j)
            | What::SeedQrFile(j),
        ) => i == j,
        (
            What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile,
            What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile,
        ) => true,
        (What::Share(i) | What::ShareFile(i, _), What::Share(j) | What::ShareFile(j, _)) => i == j,
        (x, y) => x == y,
    };
    let elsewhere = lost.iter().all(|&(_, w)| {
        left.iter()
            .any(|s| s.holds.iter().any(|(x, _)| same(w, *x)))
    });
    Follows {
        found,
        rebuilds,
        elsewhere,
        any_lost: !lost.is_empty(),
    }
}

/// The most places a plan for `shape` has (`docs/BACKUP-FLOW.md` §19.19):
/// one per key of the wallet, one for a single key, two for a single key
/// whose seed has a passphrase (the second place is for the passphrase
/// alone; owner, 2026-10-19). Fewer are allowed.
pub fn max_places(shape: &Shape) -> usize {
    let keys = shape.keys.max(shape.seeds.len()).max(1);
    let passphrase = shape.seeds.len() == 1 && shape.seeds[0].here && shape.seeds[0].passphrase;
    if keys == 1 && passphrase { 2 } else { keys }
}

/// How the description's shares are handed out to a plan's places
/// (`docs/BACKUP-FLOW.md` §19.19 items 2 and 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandOut {
    /// The keys each share leaves off.
    pub omit: usize,
    /// The share each place keeps, by place.
    pub shares: Vec<usize>,
    /// Every group of places that holds enough seeds to spend, and every
    /// group of `m` places, can rebuild the description.
    pub rebuilds: bool,
    /// The places that can see the balance from their own share and
    /// seeds though they cannot spend: none where the search could avoid
    /// it.
    pub sees: Vec<usize>,
}

/// The keys place `p` gives by its seeds on the map `boxes` (by place,
/// [`by_place`]), as a bit per key: a seed's words, SeedQR or file there,
/// or in a vault whose stick it keeps.
fn seed_keys(boxes: &[Spot], p: usize) -> u64 {
    let at = At::Place(p);
    let mut held = holds_in(boxes, at);
    for v in reads_in(boxes, at) {
        held.extend(holds_in(boxes, At::Vault(v)));
    }
    held.iter().fold(0u64, |acc, w| match w {
        What::Words(i)
        | What::SeedQr(i)
        | What::Seed(i)
        | What::SeedPassphrase(i)
        | What::SeedFile(i)
        | What::SeedQrFile(i)
            if *i < 64 =>
        {
            acc | (1 << i)
        }
        _ => acc,
    })
}

/// Costs a map of the plan (through `place_map`). Works out which share
/// goes where (`docs/BACKUP-FLOW.md` §19.19 item 3): tries the ways of
/// handing one share to each place, as many keys left off as can be
/// (from `M − 1` down to 1), and keeps the first in a fixed order where
/// every group of places that holds enough seeds to spend, and every
/// group of `M` places, rebuilds the description, and no place that
/// cannot spend sees the balance alone; failing that, the first where
/// the groups rebuild. Every group of places is tried for each way: a
/// search, bounded by a step budget, once, on the press.
pub fn hand_out(c: &Compute, shape: &Shape, a: &Answers) -> HandOut {
    let boxes = place_map(c, shape, a);
    let places = places_of(&boxes).max(1);
    let n = shape.keys.clamp(1, 63);
    let m = shape.m.max(1);
    let all: u64 = (1u64 << n) - 1;
    let seeds: Vec<u64> = (0..places).map(|p| seed_keys(&boxes, p)).collect();
    let most = m.saturating_sub(1).min(n.saturating_sub(1)).max(1);
    // The groups that must rebuild: by bit mask of places, up to 16
    // places tried in full.
    let groups: Vec<u64> = if places <= 16 {
        (1u64..(1 << places))
            .filter(|&g| {
                let keys = (0..places)
                    .filter(|p| g & (1 << p) != 0)
                    .fold(0u64, |k, p| k | seeds[p]);
                g.count_ones() as usize >= m || keys.count_ones() as usize >= m
            })
            .collect()
    } else {
        Vec::new()
    };
    let masks = |omit: usize| -> Vec<u64> {
        split_plan(n, m, omit)
            .iter()
            .map(|row| row.iter().fold(0u64, |k, &i| k | (1 << i)))
            .collect()
    };
    let can_spend = |p: usize| seeds[p].count_ones() as usize >= m;
    let rebuilds = |shares: &[u64], pick: &[usize]| {
        groups.iter().all(|&g| {
            (0..places)
                .filter(|p| g & (1 << p) != 0)
                .fold(0u64, |k, p| k | seeds[p] | shares[pick[p]])
                == all
        })
    };
    let sees_alone = |shares: &[u64], pick: &[usize]| -> Vec<usize> {
        (0..places)
            .filter(|&p| !can_spend(p) && (seeds[p] | shares[pick[p]]) == all)
            .collect()
    };
    let mut budget: u32 = 20_000;
    for strict in [true, false] {
        for omit in (1..=most).rev() {
            let shares = masks(omit);
            let mut pick: Vec<usize> = Vec::with_capacity(places);
            let found = search(
                &shares,
                places,
                &mut pick,
                &mut budget,
                &|p, j| !(strict && !can_spend(p) && (seeds[p] | shares[j]) == all),
                &|pick| rebuilds(&shares, pick),
            );
            if found {
                return HandOut {
                    omit,
                    sees: sees_alone(&shares, &pick),
                    shares: pick,
                    rebuilds: true,
                };
            }
        }
    }
    // Nothing rebuilds: one share each in order, a key left off each.
    let shares = masks(1);
    let pick: Vec<usize> = (0..places).map(|p| p % shares.len().max(1)).collect();
    HandOut {
        omit: 1,
        sees: sees_alone(&shares, &pick),
        shares: pick,
        rebuilds: false,
    }
}

/// [`hand_out`]'s search: a share for each place in turn, each tried from
/// the place's own number on and none twice while there are shares
/// enough, `ok` deciding a place's share and `done` the whole. Returns
/// whether it found one, leaving it in `pick`.
fn search(
    shares: &[u64],
    places: usize,
    pick: &mut Vec<usize>,
    budget: &mut u32,
    ok: &dyn Fn(usize, usize) -> bool,
    done: &dyn Fn(&[usize]) -> bool,
) -> bool {
    let p = pick.len();
    if p == places {
        *budget = budget.saturating_sub(1);
        return done(pick);
    }
    let n = shares.len().max(1);
    for k in 0..n {
        if *budget == 0 {
            return false;
        }
        let j = (p + k) % n;
        if (places <= n && pick.contains(&j)) || !ok(p, j) {
            continue;
        }
        pick.push(j);
        if search(shares, places, pick, budget, ok, done) {
            return true;
        }
        pick.pop();
    }
    false
}

/// Costs what `of` costs. Whether everything the plan keeps, anywhere,
/// puts the description back together (`docs/BACKUP-FLOW.md` §19.19
/// item 5): the whole of it, or shares and seeds that hold every key.
pub fn description_kept(c: &Compute, shape: &Shape, a: &Answers) -> bool {
    description_kept_on(shape, a, &Mapped::of(c, shape, a))
}

/// [`description_kept`] on the map `m`.
pub fn description_kept_on(shape: &Shape, a: &Answers, m: &Mapped) -> bool {
    let mut have = Have::none(shape);
    for s in &m.places {
        s.holds.iter().for_each(|(w, _)| have.add(*w));
    }
    have.wallet_ok(shape, a)
}

/// Costs what `of` costs. The shares the plan keeps, each with how many
/// places, vaults and the software keep it, in share order.
pub fn shares_kept(c: &Compute, shape: &Shape, a: &Answers) -> Vec<(usize, usize)> {
    shares_kept_on(shape, &Mapped::of(c, shape, a))
}

/// [`shares_kept`] on the map `m`.
pub fn shares_kept_on(shape: &Shape, m: &Mapped) -> Vec<(usize, usize)> {
    let boxes = &m.places;
    (0..shape.keys)
        .map(|j| {
            let n = boxes
                .iter()
                .filter(|s| {
                    s.holds.iter().any(
                        |(w, _)| matches!(w, What::Share(k) | What::ShareFile(k, _) if *k == j),
                    )
                })
                .count();
            (j, n)
        })
        .filter(|&(_, n)| n > 0)
        .collect()
}

impl Answers {
    /// Costs a map of the plan (through `hand_out`).
    ///
    /// **Add partial descriptions to this backup** (`docs/BACKUP-FLOW.md`
    /// §19.19 item 1): any share kept taken off, then one share of the
    /// description on each place's paper, handed out by [`hand_out`], the
    /// keys left off each as it found. The whole description stays where
    /// it is. Works out the hand-out, a search.
    pub fn add_partials(&mut self, c: &Compute, shape: &Shape) -> HandOut {
        self.edit_map(shape, |m| without_shares(&m));
        let out = hand_out(c, shape, self);
        self.omit = out.omit;
        let shares = out.shares.clone();
        self.edit_map(shape, |m| {
            shares
                .iter()
                .enumerate()
                .fold(m, |m, (p, &j)| with_added(&m, At::Place(p), What::Share(j)))
        });
        self.split = shape.splits;
        out
    }

    /// **Remove partial descriptions**: every share of the description
    /// taken off, wherever it is kept.
    pub fn remove_partials(&mut self, shape: &Shape) {
        self.edit_map(shape, |m| without_shares(&m));
        self.split = false;
    }

    /// The whole description taken off every place, on paper and on its
    /// sticks (`docs/BACKUP-FLOW.md` §19.19 item 4): vaults and the
    /// software keep theirs.
    pub fn whole_off_places(&mut self, shape: &Shape) {
        self.edit_map(shape, |mut m| {
            for s in m.iter_mut() {
                if matches!(s.at, At::Place(_) | At::Stick(..)) {
                    s.holds
                        .retain(|(w, _)| !matches!(w, What::Sheet | What::WalletFile(_)));
                }
            }
            tidy_sticks(m)
        });
    }

    /// Costs what `of` costs. The places that keep the whole description,
    /// on paper or on a stick there.
    pub fn whole_at(&self, c: &Compute, shape: &Shape) -> Vec<usize> {
        Mapped::of(c, shape, self).whole_at()
    }

    /// Partial descriptions are on: a share of the description is kept.
    pub fn partials_on(&self) -> bool {
        self.split
    }
}

/// The map `m` with every share of the description taken off.
fn without_shares(m: &[Spot]) -> Vec<Spot> {
    let mut out = m.to_vec();
    for s in out.iter_mut() {
        s.holds
            .retain(|(w, _)| !matches!(w, What::Share(_) | What::ShareFile(..)));
    }
    out.retain(|s| !(s.holds.is_empty() && matches!(s.at, At::Files | At::Software | At::Away)));
    tidy_sticks(out)
}
