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

    /// The seeds vault `v` holds: those loaded here and ticked into it,
    /// while the seeds go into vaults.
    pub fn vault_seeds(&self, shape: &Shape, v: usize) -> Vec<usize> {
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
        let with: Vec<usize> = (0..self.vaults.len())
            .filter(|&v| !self.vault_seeds(shape, v).is_empty())
            .collect();
        if with.is_empty() && self.wallet[wallet::VAULT] {
            vec![0]
        } else {
            with
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

    /// Whether place `p` keeps a stick with vault `v` on it.
    pub fn stick_at(&self, v: usize, p: usize) -> bool {
        self.sticks
            .get(v)
            .and_then(|r| r.get(p))
            .copied()
            .unwrap_or(false)
    }

    /// Puts vault `v`'s stick where it adds no second key to a place: the
    /// first place that keeps no seed's words and no other vault's stick;
    /// failing that, the last place that keeps only its own seeds' words
    /// and no other vault's stick, which for a seed kept in more than one
    /// place is the one with its further copy; failing that, the last
    /// place with its own seeds' words. Where none is, it is left with no
    /// place.
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
        if !(self.wallet[wallet::PAPER] && self.split && shape.splits) {
            return Vec::new();
        }
        spread(shape.keys, self.places, p)
    }

    /// Whether place `p` keeps the whole wallet sheet.
    pub fn sheet_at(&self, shape: &Shape, _p: usize) -> bool {
        self.wallet[wallet::PAPER] && !(self.split && shape.splits)
    }

    /// Whether seed `i`'s passphrase is kept in place `p`.
    pub fn pass_at(&self, i: usize, p: usize) -> bool {
        self.pass
            .get(i)
            .and_then(|r| r.get(p))
            .copied()
            .unwrap_or(false)
    }

    /// Whether seed `i`'s passphrase goes into the vault with it: into
    /// whichever vault holds the seed.
    pub fn pass_in_vault(&self, i: usize) -> bool {
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
        out
    }

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
        let fits = seen >= 8
            && (1..=shape.max_places()).contains(&a.places)
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

/// Where a thing may be lost or found: each place, a stick of files, and
/// each vault's own stick when no place keeps one.
fn spots(shape: &Shape, a: &Answers) -> Vec<At> {
    let mut v: Vec<At> = (0..a.places).map(At::Place).collect();
    if (a.seeds[seeds::FILE] && !shape.watch_only()) || a.wallet[wallet::FILES] {
        v.push(At::Files);
    }
    for u in a.vaults_made(shape) {
        if !(0..a.places).any(|p| a.stick_at(u, p)) {
            v.push(At::Vault(u));
        }
    }
    v
}

/// Computes the check: every place lost in turn, and every place found.
/// Each vault is read, with its passphrase, from a place that keeps its
/// stick, or from its own stick.
pub fn check(shape: &Shape, a: &Answers) -> Check {
    let boxes = map(shape, a);
    let holds = |at: At| -> Vec<What> {
        boxes
            .iter()
            .filter(|s| s.at == at)
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect()
    };
    let made = a.vaults_made(shape);
    // The vaults a spot can be read from: the ones whose stick a place
    // keeps, or a vault's own stick.
    let reads = |at: At| -> Vec<usize> {
        match at {
            At::Place(p) => made.iter().copied().filter(|&v| a.stick_at(v, p)).collect(),
            At::Vault(v) if made.contains(&v) => vec![v],
            _ => Vec::new(),
        }
    };
    let opened = |have: &Have, vaults: &[usize]| {
        let mut with = have.clone();
        for &v in vaults {
            holds(At::Vault(v)).into_iter().for_each(|w| with.add(w));
        }
        with
    };
    let places = spots(shape, a);
    let mut lost = Lost::Yes;
    for &gone in &places {
        let mut have = Have::none(shape);
        let mut readable: Vec<usize> = Vec::new();
        for &at in places.iter().filter(|&&at| at != gone) {
            if !matches!(at, At::Vault(_)) {
                holds(at).into_iter().for_each(|w| have.add(w));
            }
            readable.extend(reads(at));
        }
        holds(At::Software).into_iter().for_each(|w| have.add(w));
        holds(At::Away).into_iter().for_each(|w| have.add(w));
        let verdict = if have.rebuilds(shape, a) {
            Lost::Yes
        } else if !readable.is_empty() && opened(&have, &readable).rebuilds(shape, a) {
            Lost::WithVault
        } else {
            Lost::No
        };
        lost = lost.min(verdict);
    }
    let mut spend = Found::No;
    let mut balance = Found::No;
    for &at in &places {
        let mut have = Have::none(shape);
        if !matches!(at, At::Vault(_)) {
            holds(at).into_iter().for_each(|w| have.add(w));
        }
        let with = opened(&have, &reads(at));
        let found = |ok: &dyn Fn(&Have) -> bool| {
            if ok(&have) {
                Found::Yes
            } else if ok(&with) {
                Found::OnlyWithVault
            } else {
                Found::No
            }
        };
        spend = spend.max(found(&|h| {
            h.seeds_ok(shape) >= shape.m.max(1) && (!shape.multi() || h.wallet_ok(shape, a))
        }));
        balance = balance.max(found(&|h| h.sees(shape, a)));
    }
    Check {
        lost,
        spend,
        balance,
    }
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
    let here: Vec<usize> = (0..shape.seeds.len())
        .filter(|&i| shape.seeds[i].here)
        .collect();
    let mut out = Vec::new();
    if a.paper_seeds() && !here.is_empty() {
        out.push(Item::Templates);
        out.extend(here.iter().map(|&i| Item::Copy(i)));
    }
    out.extend(a.vaults_made(shape).into_iter().map(Item::Vault));
    if a.seeds[seeds::FILE] && !here.is_empty() {
        out.push(Item::SeedFiles);
    }
    if a.wallet[wallet::PAPER] {
        out.push(Item::Sheets);
    }
    if a.wallet[wallet::FILES] || (a.wallet[wallet::SOFTWARE] && a.form[form::TEXT]) {
        out.push(Item::PublicFiles);
    }
    if a.wallet[wallet::SOFTWARE] && a.form[form::QR] {
        out.push(Item::ShowDescriptor);
    }
    out.push(Item::Envelopes);
    out
}

/// How many blank templates the plan prints: one per paper copy of a
/// seed, in every place.
pub fn templates(shape: &Shape, a: &Answers) -> usize {
    (0..a.places).map(|p| a.words_at(shape, p).len()).sum()
}
