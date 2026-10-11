//! The backup's plan (`docs/WALLETS.md` §5): the answers a person gives
//! (where the seeds go, how many places keep paper, where the wallet
//! description goes, for which software), the map they draw (what each
//! place, each vault, a stick of files and the watch-only software hold),
//! the check computed from it, and the checklist of only what it needs.
//!
//! The check is found the way [`crate::backup::audit`] finds what a split
//! achieves: by trying each place lost and each place found, not by
//! trusting arithmetic. Nothing here knows a place's name: names live in
//! the vault alone and are put on screen by the caller.

use crate::backup::split_plan;

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

/// The three ways to start, one tap each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Paper in each place.
    Paper,
    /// Paper, and the seeds and the wallet in the vault.
    PaperVault,
    /// Paper, the vault, and the wallet in watch-only software.
    PaperVaultSoftware,
}

impl Preset {
    /// Every preset, in the order offered.
    pub const ALL: [Preset; 3] = [
        Preset::Paper,
        Preset::PaperVault,
        Preset::PaperVaultSoftware,
    ];

    /// What the preset is called.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Paper => "Paper only",
            Preset::PaperVault => "Paper and vault",
            Preset::PaperVaultSoftware => "Paper, vault and watch-only software",
        }
    }
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

    /// The most places paper may be kept in: one per seed, and at least
    /// three.
    pub fn max_places(&self) -> usize {
        self.seeds.len().max(3)
    }

    /// The seeds loaded here, by place among the wallet's.
    pub fn here(&self) -> Vec<usize> {
        (0..self.seeds.len())
            .filter(|&i| self.seeds[i].here)
            .collect()
    }

    /// How many vaults the plan has a section for: one per seed loaded
    /// here, and one for the description alone when none is.
    pub fn vault_rows(&self) -> usize {
        self.here().len().max(1)
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
        }
    }
}

impl Answers {
    /// The wallet's defaults: one key, paper words, two places, the
    /// sheet in each, the descriptor as a QR picture; a multisig, a
    /// place per seed, each with its seed and the whole wallet sheet; no
    /// seed here, no seed on paper. Should the seeds go into vaults, a
    /// vault per seed here, vault `v` holding the `v`th.
    pub fn defaults(shape: &Shape) -> Answers {
        let places = if shape.multi() { shape.seeds.len() } else { 2 };
        let mut a = Answers {
            seeds: [!shape.watch_only(), false, false, false],
            places,
            split: false,
            vaults: shape
                .here()
                .into_iter()
                .map(|i| (0..shape.seeds.len()).map(|j| j == i).collect())
                .collect(),
            sticks: vec![vec![false; places]; shape.vault_rows()],
            wallet: [true, false, false, false],
            software: [false, false, false, false, true],
            form: [true, false],
            pass: Vec::new(),
            omit: shape.m.saturating_sub(1),
            map: None,
        };
        a.pass = shape
            .seeds
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if !(s.here && s.passphrase) {
                    return Vec::new();
                }
                let mut row = vec![false; places + 1];
                if let Some(p) = (0..places).find(|&p| !a.words_at(shape, p).contains(&i)) {
                    row[p] = true;
                }
                row
            })
            .collect();
        a
    }

    /// The answers a preset fills in, from the wallet's defaults.
    pub fn preset(shape: &Shape, preset: Preset) -> Answers {
        let mut a = Answers::defaults(shape);
        if preset != Preset::Paper {
            a.seeds[seeds::VAULT] = !shape.watch_only();
            a.wallet[wallet::VAULT] = true;
            a.place_new_sticks(shape, &[]);
        }
        if preset == Preset::PaperVaultSoftware {
            a.wallet[wallet::SOFTWARE] = true;
        }
        a
    }

    /// Sets how many places keep paper, within what the shape allows; the
    /// lists by place grow or shrink with it.
    pub fn set_places(&mut self, shape: &Shape, places: usize) {
        let places = places.clamp(1, shape.max_places());
        self.places = places;
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

    /// What spot `at` of the map edited on the chart holds; `None` while
    /// the answers draw the map.
    fn edited_at(&self, at: At) -> Option<Vec<What>> {
        let m = self.map.as_ref()?;
        Some(
            m.iter()
                .filter(|s| s.at == at)
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
            return m
                .iter()
                .filter_map(|s| match s.at {
                    At::Vault(v) => Some(v),
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
            format!("Vault {}", v + 1)
        } else {
            format!("Vault {} · {}", v + 1, names.join(", "))
        }
    }

    /// The first vault made whose stick no place keeps: the plan waits
    /// on Places until it has one (`docs/NEW-WALLET.md` §14.2).
    pub fn stickless(&self, shape: &Shape) -> Option<usize> {
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
        if !self.paper_seeds() {
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
        // An answer changed replaces the map edited on the chart: asked
        // before the questions open (`docs/NEW-WALLET.md` §9.7).
        self.map = None;
        let before = self.vaults_made(shape);
        match q {
            Question::Seeds => flip(&mut self.seeds, row),
            Question::Wallet => flip(&mut self.wallet, row),
            Question::Software => flip(&mut self.software, row),
            Question::Form => flip(&mut self.form, row),
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
        out
    }

    /// Makes `m` the plan: the map edited on the chart (§9.7). The
    /// places are the map's; the lists by place follow their count.
    pub fn set_map(&mut self, m: Vec<Spot>) {
        let places = m
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .count()
            .max(1);
        self.places = places;
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

/// A spot as a plan's `spot` line writes it: `place0`, `vault1`,
/// `files`, `software`, `away`.
pub fn at_code(at: At) -> String {
    match at {
        At::Place(p) => format!("place{p}"),
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
        _ => match s.strip_prefix("place") {
            Some(p) => At::Place(p.parse().ok()?),
            None => At::Vault(s.strip_prefix("vault")?.parse().ok()?),
        },
    })
}

/// A thing as a plan's `spot` line writes it: `w0` seed 0's words, `q0`
/// its SeedQR, `p0` its passphrase, `sheet`, `share1`, `stick0` vault 0's
/// stick, `seed0`, `seedp0` with its passphrase, `wallet`, `file0`.
pub fn what_code(w: What) -> String {
    match w {
        What::Words(i) => format!("w{i}"),
        What::SeedQr(i) => format!("q{i}"),
        What::Passphrase(i) => format!("p{i}"),
        What::Sheet => "sheet".to_string(),
        What::Share(j) => format!("share{j}"),
        What::VaultStick(v) => format!("stick{v}"),
        What::Seed(i) => format!("seed{i}"),
        What::SeedPassphrase(i) => format!("seedp{i}"),
        What::Wallet => "wallet".to_string(),
        What::SeedFile(i) => format!("file{i}"),
    }
}

/// [`what_code`] read back.
pub fn what_read(s: &str) -> Option<What> {
    let n = |p: &str| -> Option<usize> { s.strip_prefix(p)?.parse().ok() };
    Some(match s {
        "sheet" => What::Sheet,
        "wallet" => What::Wallet,
        _ if s.starts_with("seedp") => What::SeedPassphrase(n("seedp")?),
        _ if s.starts_with("seed") => What::Seed(n("seed")?),
        _ if s.starts_with("share") => What::Share(n("share")?),
        _ if s.starts_with("stick") => What::VaultStick(n("stick")?),
        _ if s.starts_with("file") => What::SeedFile(n("file")?),
        _ if s.starts_with('w') => What::Words(n("w")?),
        _ if s.starts_with('q') => What::SeedQr(n("q")?),
        _ if s.starts_with('p') => What::Passphrase(n("p")?),
        _ => return None,
    })
}

/// How a thing kept at a spot may be read: anything in a vault, and a
/// vault's stick, sealed; the description, a sheet and a share public;
/// the rest secret.
pub fn tag_of(at: At, what: What) -> Tag {
    match (at, what) {
        (At::Vault(_), _) | (_, What::VaultStick(_)) => Tag::Sealed,
        (_, What::Sheet | What::Share(_) | What::Wallet) => Tag::Public,
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
        let mut seen = 0;
        let mut one_stick: Option<Vec<bool>> = None;
        let mut sticks: Vec<(usize, Vec<bool>)> = Vec::new();
        let mut vaults: Vec<(usize, Vec<bool>)> = Vec::new();
        let mut edited: Vec<Spot> = Vec::new();
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
                "pass" => {
                    let (i, row) = value.split_once(' ')?;
                    let i: usize = i.parse().ok()?;
                    *a.pass.get_mut(i)? = flags(row)?;
                }
                // A key made here whose backup is pending: read by the
                // caller (`docs/NEW-WALLET.md` §14.3), not an answer.
                "held" => continue,
                // The map edited on the chart (§9.7).
                "spot" => {
                    let mut parts = value.split(' ');
                    let at = at_read(parts.next()?)?;
                    let holds = parts
                        .filter(|p| !p.is_empty())
                        .map(|p| what_read(p).map(|w| (w, tag_of(at, w))))
                        .collect::<Option<Vec<_>>>()?;
                    edited.push(Spot { at, holds });
                    continue;
                }
                _ => return None,
            }
            seen += 1;
        }
        a.sticks = vec![vec![false; a.places]; shape.vault_rows()];
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
        if !edited.is_empty() {
            // Its places numbered in order, and nothing in it a seed, a
            // share or a key the wallet does not have.
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
                | What::SeedFile(i) => i < shape.seeds.len(),
                What::Share(j) => j < shape.keys.max(1),
                _ => true,
            };
            let ok = places.iter().enumerate().all(|(n, &p)| n == p)
                && edited
                    .iter()
                    .all(|s| s.holds.iter().all(|(w, _)| fits_shape(*w)));
            if !ok {
                return None;
            }
            a.set_map(edited);
        }
        let fits = seen >= 8
            && (a.places >= 1 && (a.map.is_some() || a.places <= shape.max_places()))
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
    /// A place that keeps paper, by number from zero.
    Place(usize),
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
    /// The wallet description.
    Wallet,
    /// Seed `i` as a file.
    SeedFile(usize),
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
            What::Words(i) => format!("Seed {}: words", seed(i)),
            What::SeedQr(i) => format!("Seed {}: SeedQR", seed(i)),
            What::Passphrase(i) => format!("Passphrase of {}", seed(i)),
            What::Sheet => "Wallet sheet".to_string(),
            What::Share(j) => format!("Share {} of {}", j + 1, shape.keys),
            What::VaultStick(v) => format!("{} with Vault {}", medium.cap(), v + 1),
            What::Seed(i) => format!("Seed {}", seed(i)),
            What::SeedPassphrase(i) => format!("Seed {} with its passphrase", seed(i)),
            What::Wallet => "Wallet description".to_string(),
            What::SeedFile(i) => format!("Seed {}: file", seed(i)),
        }
    }
}

/// The map: one box per place, then each vault made, a stick of files,
/// the watch-only software and the seeds on their own devices, each where
/// the plan uses it.
pub fn map(shape: &Shape, a: &Answers) -> Vec<Spot> {
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
    out
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
            Lost::Yes => "Yes",
            Lost::WithVault => "Yes, with the vault's passphrase",
            Lost::No => "No",
        }
    }
}

impl Found {
    /// The value as the check writes it.
    pub fn text(self) -> &'static str {
        match self {
            Found::Yes => "Yes",
            Found::OnlyWithVault => "Only with the vault's passphrase",
            Found::No => "No",
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
}

/// The check's three lines, labelled.
pub const CHECK_LINES: [&str; 3] = [
    "Any one place lost: the rest rebuild the wallet",
    "One place found: can spend",
    "One place found: sees the balance",
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
            What::Words(i) | What::SeedQr(i) | What::Seed(i) | What::SeedFile(i) => {
                self.seeds[i] = true;
            }
            What::SeedPassphrase(i) => {
                self.seeds[i] = true;
                self.pass[i] = true;
            }
            What::Passphrase(i) => self.pass[i] = true,
            What::Sheet | What::Wallet => self.wallet = true,
            What::Share(j) => self.shares[j] = true,
            What::VaultStick(_) => {}
        }
    }

    fn seeds_ok(&self, shape: &Shape) -> usize {
        (0..shape.seeds.len())
            .filter(|&i| self.seeds[i] && (!shape.seeds[i].passphrase || self.pass[i]))
            .count()
    }

    /// The description, whole or from shares that together hold every key.
    fn wallet_ok(&self, shape: &Shape, a: &Answers) -> bool {
        if self.wallet {
            return true;
        }
        if !shape.splits || !self.shares.iter().any(|&s| s) {
            return false;
        }
        let plan = split_plan(shape.keys, shape.m, a.omit);
        (0..shape.keys).all(|k| {
            plan.iter()
                .enumerate()
                .any(|(j, row)| self.shares[j] && row.contains(&k))
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

/// The places that keep a stick with vault `v` on it, on the map `boxes`.
fn sticks_of(boxes: &[Spot], v: usize) -> Vec<usize> {
    boxes
        .iter()
        .filter_map(|s| match s.at {
            At::Place(p) if s.holds.iter().any(|(w, _)| *w == What::VaultStick(v)) => Some(p),
            _ => None,
        })
        .collect()
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

impl Alone {
    /// The line the wallet's chart writes under a spot: "Nothing", "Sees
    /// the balance", "Can spend", "Can spend with the vault's passphrase".
    pub fn text(self) -> &'static str {
        match (self.spend, self.balance) {
            (Found::Yes, _) => "Can spend",
            (Found::OnlyWithVault, Found::Yes) => {
                "Sees the balance · can spend with the vault's passphrase"
            }
            (Found::OnlyWithVault, _) => "Can spend with the vault's passphrase",
            (Found::No, Found::Yes) => "Sees the balance",
            (Found::No, Found::OnlyWithVault) => "Sees the balance with the vault's passphrase",
            (Found::No, Found::No) => "Nothing",
        }
    }
}

/// What spot `at` gives whoever finds it alone: what it holds, and each
/// vault whose stick it keeps, read with that vault's passphrase. The
/// check's "One place found" lines are the most any spot gives.
pub fn alone(shape: &Shape, a: &Answers, at: At) -> Alone {
    alone_in(shape, a, &map(shape, a), at)
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
        spend: found(&|h| {
            h.seeds_ok(shape) >= shape.m.max(1) && (!shape.multi() || h.wallet_ok(shape, a))
        }),
        balance: found(&|h| h.sees(shape, a)),
    }
}

/// Computes the check: every place lost in turn, and every place found.
/// Each vault is read, with its passphrase, from a place that keeps its
/// stick, or from its own stick.
pub fn check(shape: &Shape, a: &Answers) -> Check {
    check_of(shape, a, &map(shape, a))
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
    for &at in &places {
        let found = alone_in(shape, a, boxes, at);
        spend = spend.max(found.spend);
        balance = balance.max(found.balance);
    }
    Check {
        lost,
        spend,
        balance,
    }
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

/// Where seed `i` is kept on the map `boxes`: each place with its words
/// or SeedQR, the stick of files, each vault that holds it.
fn seed_spots(boxes: &[Spot], i: usize) -> Vec<At> {
    let has = |s: &Spot, f: &dyn Fn(What) -> bool| s.holds.iter().any(|(w, _)| f(*w));
    let mut out: Vec<At> = boxes
        .iter()
        .filter(|s| {
            matches!(s.at, At::Place(_)) && has(s, &|w| w == What::Words(i) || w == What::SeedQr(i))
        })
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

/// Where seed `i`'s passphrase is kept on the map `boxes`: each place
/// that keeps it written, each vault that holds it with its seed.
fn pass_spots(boxes: &[Spot], i: usize) -> Vec<At> {
    boxes
        .iter()
        .filter(|s| {
            s.holds.iter().any(|(w, _)| match s.at {
                At::Place(_) => *w == What::Passphrase(i),
                At::Vault(_) => *w == What::SeedPassphrase(i),
                _ => false,
            })
        })
        .map(|s| s.at)
        .collect()
}

/// Why "Any one place lost" reads No, as a line under the check: the
/// first spot whose loss leaves too little, and the seed or passphrase
/// kept only there. `name` names a place, `noun` is the medium ("stick").
/// None when the check does not read No.
pub fn lost_why(
    shape: &Shape,
    a: &Answers,
    name: &dyn Fn(usize) -> String,
    noun: &str,
) -> Option<String> {
    let boxes = map(shape, a);
    let places = spots(&boxes);
    let gone = places
        .iter()
        .copied()
        .find(|&g| lost_one(shape, a, &boxes, &places, Some(g)) == Lost::No)?;
    let stick_text = |v: usize| -> String {
        let at: Vec<String> = sticks_of(&boxes, v).into_iter().map(name).collect();
        if at.is_empty() {
            "at no place".to_string()
        } else {
            format!("at {}", at.join(", "))
        }
    };
    // A thing kept at `at` is lost with `gone`: there, or in a vault whose
    // every stick is there.
    let with_gone = |at: At| -> bool {
        at == gone
            || matches!(at, At::Vault(v) if sticks_of(&boxes, v)
                .into_iter()
                .all(|p| At::Place(p) == gone))
    };
    let only = |list: &[At]| -> Option<String> {
        match list {
            [At::Vault(v)] => Some(format!(
                "Vault {}, whose {noun} is {}",
                v + 1,
                stick_text(*v)
            )),
            [At::Place(p)] => Some(name(*p)),
            [At::Files] => Some(format!("the {noun} of files")),
            _ => None,
        }
    };
    for i in shape.here() {
        let seed = &shape.seeds[i].name;
        let kept = seed_spots(&boxes, i);
        if kept.is_empty() {
            return Some(format!("Seed {seed} is kept nowhere"));
        }
        if kept.iter().all(|&at| with_gone(at)) {
            let line = match only(&kept) {
                Some(w) if matches!(kept[0], At::Vault(_)) => format!("in {w}"),
                Some(w) if kept[0] == At::Files => format!("on {w}"),
                Some(w) => format!("at {w}"),
                None => format!("with {}", name_at(gone, name, noun)),
            };
            return Some(format!("Seed {seed} is kept only {line}"));
        }
        if shape.seeds[i].passphrase {
            let pass = pass_spots(&boxes, i);
            if pass.is_empty() {
                return Some(format!("The passphrase of {seed} is kept nowhere"));
            }
            if pass.iter().all(|&at| with_gone(at)) {
                return Some(format!(
                    "The passphrase of {seed} is kept only with {}",
                    name_at(gone, name, noun)
                ));
            }
        }
    }
    Some(format!(
        "Without {}, the rest do not rebuild the wallet",
        name_at(gone, name, noun)
    ))
}

/// A spot as the why line names it.
fn name_at(at: At, name: &dyn Fn(usize) -> String, noun: &str) -> String {
    match at {
        At::Place(p) => name(p),
        At::Vault(v) => format!("Vault {}", v + 1),
        At::Files => format!("the {noun} of files"),
        At::Software => "the software".to_string(),
        At::Away => "the devices".to_string(),
    }
}

/// The first of `seeds` (made here) the plan keeps no copy of, or whose
/// passphrase it keeps nowhere: the line said in place of Make the
/// checklist (`docs/NEW-WALLET.md` §14.4).
pub fn kept_nowhere(shape: &Shape, a: &Answers, seeds: &[usize]) -> Option<String> {
    let boxes = map(shape, a);
    for &i in seeds {
        let Some(seed) = shape.seeds.get(i).filter(|s| s.here) else {
            continue;
        };
        if seed_spots(&boxes, i).is_empty() {
            return Some(format!("Seed {} is kept nowhere", seed.name));
        }
        if seed.passphrase && pass_spots(&boxes, i).is_empty() {
            return Some(format!("The passphrase of {} is kept nowhere", seed.name));
        }
    }
    None
}

/// One thing the checklist asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Print the blank templates: one per paper seed copy.
    Templates,
    /// Copy seed `i` by hand and check it.
    Copy(usize),
    /// Make vault `v` and save into it its seeds, and the wallet where
    /// the plan puts it there.
    Vault(usize),
    /// The seeds as files, past the secret sheet.
    SeedFiles,
    /// The wallet sheet or the shares.
    Sheets,
    /// The public files for the software and form chosen.
    PublicFiles,
    /// Show the descriptor to the watch-only software.
    ShowDescriptor,
    /// One envelope per place.
    Envelopes,
}

/// The checklist: only what the plan needs, in the order to do it.
pub fn checklist(shape: &Shape, a: &Answers) -> Vec<Item> {
    let boxes = map(shape, a);
    let at_places = |f: &dyn Fn(What) -> bool| {
        boxes
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .any(|s| s.holds.iter().any(|(w, _)| f(*w)))
    };
    let mut out = Vec::new();
    let paper: Vec<usize> = shape
        .here()
        .into_iter()
        .filter(|&i| at_places(&|w| w == What::Words(i) || w == What::SeedQr(i)))
        .collect();
    if !paper.is_empty() {
        out.push(Item::Templates);
        out.extend(paper.into_iter().map(Item::Copy));
    }
    out.extend(boxes.iter().filter_map(|s| match s.at {
        At::Vault(v) => Some(Item::Vault(v)),
        _ => None,
    }));
    let files = holds_in(&boxes, At::Files);
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
    out
}

/// How many blank templates the plan prints: one per paper copy of a
/// seed, in every place.
pub fn templates(shape: &Shape, a: &Answers) -> usize {
    map(shape, a)
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

/// The map `boxes` with `what` added at `at`: into its spot, or a new
/// spot made for it (a new place after the last, a stick of files, a
/// vault). Nothing is added twice to one spot.
pub fn with_added(boxes: &[Spot], at: At, what: What) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    let at = match at {
        At::Place(p) if p >= places_of(&out) => At::Place(places_of(&out)),
        at => at,
    };
    let pos = match out.iter().position(|s| s.at == at) {
        Some(i) => i,
        None => {
            // A new spot: a place after the last place, anything else
            // after the places and vaults.
            let after = out
                .iter()
                .rposition(|s| match at {
                    At::Place(_) => matches!(s.at, At::Place(_)),
                    _ => matches!(s.at, At::Place(_) | At::Vault(_)),
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
    out
}

/// The map `boxes` with `what` taken off spot `at`. A stick of files or
/// software left holding nothing goes; a place stays, empty.
pub fn without(boxes: &[Spot], at: At, what: What) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    for s in out.iter_mut().filter(|s| s.at == at) {
        s.holds.retain(|(w, _)| *w != what);
    }
    out.retain(|s| !(s.holds.is_empty() && matches!(s.at, At::Files | At::Software | At::Away)));
    out
}

/// The map `boxes` with place `p` removed: each thing it held goes to
/// the place `dest` names for it (by the place's number before the
/// removal), or off the map; the places after it move down one.
pub fn without_place(boxes: &[Spot], p: usize, dest: &[(What, Option<usize>)]) -> Vec<Spot> {
    let mut out = boxes.to_vec();
    for &(what, to) in dest {
        if let Some(to) = to.filter(|&to| to != p) {
            out = with_added(&out, At::Place(to), what);
        }
    }
    out.retain(|s| s.at != At::Place(p));
    for s in &mut out {
        if let At::Place(q) = s.at
            && q > p
        {
            s.at = At::Place(q - 1);
        }
    }
    out
}

/// The map `boxes` with `edit` made; for [`Edit::RemovePlace`], `dest`
/// says where each thing goes.
pub fn edited(boxes: &[Spot], edit: Edit, dest: &[(What, Option<usize>)]) -> Vec<Spot> {
    match edit {
        Edit::Add(at, what) => with_added(boxes, at, what),
        Edit::Move(from, what, to) => with_added(&without(boxes, from, what), to, what),
        Edit::Drop(at, what) => without(boxes, at, what),
        Edit::RemovePlace(p) => without_place(boxes, p, dest),
        Edit::Clear(at) => boxes.iter().filter(|s| s.at != at).cloned().collect(),
    }
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
}

impl Mark {
    /// The mark as the chart writes it.
    pub fn name(self) -> &'static str {
        match self {
            Mark::Lost => "lost",
            Mark::Exposed => "exposed",
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

/// What follows on the plan `a` from the things `marks` lists: the lost
/// ones gone, the exposed ones in one finder's hands.
pub fn follows(shape: &Shape, a: &Answers, marks: &[(At, What, Mark)]) -> Follows {
    let boxes = map(shape, a);
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
    let left = without_all(&boxes, &lost);
    let rebuilds = lost_one(shape, a, &left, &spots(&left), None);
    // Whoever found the exposed things holds them as one place would.
    let finder = At::Place(usize::MAX);
    let mut with = boxes.clone();
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
        (What::Passphrase(i), What::Passphrase(j) | What::SeedPassphrase(j)) => i == j,
        (
            What::Words(i)
            | What::SeedQr(i)
            | What::Seed(i)
            | What::SeedPassphrase(i)
            | What::SeedFile(i),
            What::Words(j)
            | What::SeedQr(j)
            | What::Seed(j)
            | What::SeedPassphrase(j)
            | What::SeedFile(j),
        ) => i == j,
        (What::Sheet | What::Wallet, What::Sheet | What::Wallet) => true,
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
