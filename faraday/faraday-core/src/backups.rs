//! The Backups screen's content (`docs/SIMPLIFY.md` §5): every wallet the
//! app knows, once each by descriptor checksum, and under each the backup
//! map as the plan draws it, each place with what this device saw of it.
//!
//! What a plan holds is recorded here when its checklist is made, and
//! each check of a copy by hand as it matches: a paper place reads
//! checked only when a check of that copy happened on this device. The
//! records are kept across a lock in the kept state and gone at
//! power-off, as the checked wallets are. A place's name is never in
//! them: names are kept in the vault alone.

use faraday_vault::records::{field, kind};

use crate::backup::Tone;
use crate::plan::{self, At, Tag, What};
use crate::vaults::Currency;
use crate::wallet::{Session, Wallet, fp_text};
use crate::{Faraday, file_stem};

/// What this power-on saw of one wallet's backup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// The wallet's descriptor checksum.
    pub sum: String,
    /// The wallet's name.
    pub name: String,
    /// The map as the plan drew it.
    pub spots: Vec<Spot>,
    /// Each seed's copies by hand checked here, by fingerprint: one
    /// count per check that matched.
    pub checks: Vec<([u8; 4], u32)>,
    /// The descriptor was shown to the watch-only software as a code.
    pub shown: bool,
}

/// One place of the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    /// Where.
    pub at: At,
    /// What it holds, as the map names it.
    pub holds: Vec<String>,
    /// How what it holds may be read: the most exposed of its things.
    pub tag: Tag,
    /// The seeds whose copy by hand it keeps, by fingerprint.
    pub copies: Vec<[u8; 4]>,
    /// The public files it holds, by name.
    pub files: Vec<String>,
    /// The start of the names of the seed files it holds.
    pub secret: Vec<String>,
}

/// A wallet as the Backups screen lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its descriptor checksum.
    pub sum: String,
    /// Its name.
    pub name: String,
    /// Its shape: "2 of 3 · native SegWit".
    pub shape: String,
    /// The loaded wallet, by index into the session, when it is loaded.
    pub wallet: Option<usize>,
    /// Its map, a line per place; `None` when it has no plan.
    pub lines: Option<Vec<Line>>,
}

/// One place of a wallet's map on the Backups screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Where: "Place 1", "Vault", "Stick of files".
    pub place: String,
    /// What it holds.
    pub holds: String,
    /// Secret, sealed or public.
    pub tag: Option<Tag>,
    /// What this device saw of it.
    pub state: String,
    /// How the state reads.
    pub tone: Tone,
    /// Where it is: which part of the map.
    pub at: Option<At>,
    /// A place that keeps a copy by hand of a seed.
    pub paper: bool,
}

/// The most exposed of two tags.
fn worse(a: Tag, b: Tag) -> Tag {
    let rank = |t: Tag| match t {
        Tag::Public => 0,
        Tag::Sealed => 1,
        Tag::Secret => 2,
    };
    if rank(b) > rank(a) { b } else { a }
}

/// A field of the kept records: no tab, line break or separator.
fn kept_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r', '\u{1f}'], " ")
}

fn at_text(at: At) -> String {
    match at {
        At::Place(p) => format!("place{p}"),
        At::Vault => "vault".into(),
        At::Files => "files".into(),
        At::Software => "software".into(),
        At::Away => "away".into(),
    }
}

fn at_read(s: &str) -> Option<At> {
    Some(match s {
        "vault" => At::Vault,
        "files" => At::Files,
        "software" => At::Software,
        "away" => At::Away,
        p => At::Place(p.strip_prefix("place")?.parse().ok()?),
    })
}

fn tag_read(s: &str) -> Option<Tag> {
    [Tag::Secret, Tag::Sealed, Tag::Public]
        .into_iter()
        .find(|t| t.name() == s)
}

fn hex4(b: [u8; 4]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn read_hex4(s: &str) -> Option<[u8; 4]> {
    if s.len() != 8 {
        return None;
    }
    let mut out = [0u8; 4];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The records as the kept state holds them: a `backup` line per
/// wallet, then its `spot` and `check` lines, fields split by tabs.
pub(crate) fn encode(all: &[Record]) -> Vec<u8> {
    let list = |v: &[String]| {
        v.iter()
            .map(|s| kept_field(s))
            .collect::<Vec<_>>()
            .join("\u{1f}")
    };
    let mut out = String::new();
    for r in all {
        out.push_str(&format!(
            "backup\t{}\t{}\t{}\n",
            kept_field(&r.sum),
            kept_field(&r.name),
            u8::from(r.shown)
        ));
        for s in &r.spots {
            let copies: Vec<String> = s.copies.iter().map(|c| hex4(*c)).collect();
            out.push_str(&format!(
                "spot\t{}\t{}\t{}\t{}\t{}\t{}\n",
                at_text(s.at),
                s.tag.name(),
                list(&s.holds),
                copies.join(","),
                list(&s.files),
                list(&s.secret)
            ));
        }
        for (fp, n) in &r.checks {
            out.push_str(&format!("check\t{}\t{n}\n", hex4(*fp)));
        }
    }
    out.into_bytes()
}

/// The records read back; a line that does not read is skipped.
pub(crate) fn decode(bytes: &[u8]) -> Vec<Record> {
    let text = String::from_utf8_lossy(bytes);
    let list = |s: &str| -> Vec<String> {
        s.split('\u{1f}')
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect()
    };
    let mut out: Vec<Record> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[..] {
            ["backup", sum, name, shown] => out.push(Record {
                sum: sum.to_string(),
                name: name.to_string(),
                shown: shown == "1",
                ..Record::default()
            }),
            ["spot", at, tag, holds, copies, files, secret] => {
                let (Some(at), Some(tag), Some(r)) = (at_read(at), tag_read(tag), out.last_mut())
                else {
                    continue;
                };
                r.spots.push(Spot {
                    at,
                    holds: list(holds),
                    tag,
                    copies: copies.split(',').filter_map(read_hex4).collect(),
                    files: list(files),
                    secret: list(secret),
                });
            }
            ["check", fp, n] => {
                if let (Some(fp), Ok(n), Some(r)) = (read_hex4(fp), n.parse(), out.last_mut()) {
                    r.checks.push((fp, n));
                }
            }
            _ => {}
        }
    }
    out
}

impl Record {
    /// How many copies of seed `fp` were checked here.
    fn checked(&self, fp: [u8; 4]) -> u32 {
        self.checks
            .iter()
            .find(|(f, _)| *f == fp)
            .map_or(0, |(_, n)| *n)
    }

    /// Whether every copy by hand place `i` keeps was checked here: the
    /// first check of a seed counts for the first place that keeps it,
    /// the second for the second.
    fn place_checked(&self, i: usize) -> bool {
        let Some(spot) = self.spots.get(i) else {
            return false;
        };
        spot.copies.iter().all(|fp| {
            let nth = self.spots[..i]
                .iter()
                .filter(|s| matches!(s.at, At::Place(_)) && s.copies.contains(fp))
                .count() as u32;
            self.checked(*fp) > nth
        })
    }
}

impl Faraday {
    /// The map of wallet `wallet` under answers `a`, as a record keeps it.
    /// `wi` is the loaded wallet, which names the public files.
    pub(crate) fn backup_spots(
        &self,
        wallet: &Wallet,
        wi: Option<usize>,
        a: &plan::Answers,
    ) -> Vec<Spot> {
        let shape = self.plan_shape_of(wallet);
        let seeds = self.backup_seed_list_of(wallet);
        let fp_of = |i: usize| -> Option<[u8; 4]> {
            let k = seeds.get(i)?.1?;
            Some(self.session.keys.get(k)?.master.fingerprint().0)
        };
        let stem = file_stem(&wallet.name);
        let public = wi.map_or_else(Vec::new, |wi| self.backup_public_names_for(wi, a));
        plan::map(&shape, a)
            .into_iter()
            .map(|spot| {
                let mut out = Spot {
                    at: spot.at,
                    holds: spot
                        .holds
                        .iter()
                        .map(|(h, _)| h.label(&shape, self.medium))
                        .collect(),
                    tag: spot.holds.iter().map(|(_, t)| *t).fold(Tag::Public, worse),
                    copies: Vec::new(),
                    files: Vec::new(),
                    secret: Vec::new(),
                };
                for (what, _) in &spot.holds {
                    match *what {
                        What::Words(i) | What::SeedQr(i) => {
                            if let Some(fp) = fp_of(i)
                                && !out.copies.contains(&fp)
                            {
                                out.copies.push(fp);
                            }
                        }
                        What::Sheet => out.files.push(format!("{stem}-backup.pdf")),
                        What::Share(j) => {
                            out.files
                                .push(format!("{stem}-share-{}-of-{}.pdf", j + 1, shape.keys))
                        }
                        What::SeedFile(i) => {
                            if let Some(fp) = fp_of(i) {
                                let fps = fp_text(osk_bip::keys::Fingerprint(fp)).to_lowercase();
                                out.secret.push(format!("{stem}-{fps}-"));
                            }
                        }
                        What::Wallet if spot.at == At::Files => out.files.extend(public.clone()),
                        What::Wallet if spot.at == At::Software && a.form[plan::form::TEXT] => {
                            out.files.extend(public.clone())
                        }
                        _ => {}
                    }
                }
                out
            })
            .collect()
    }

    /// The record of the backup under way, made or brought up to date
    /// from its plan: when its checklist is made. What was checked is
    /// kept.
    pub(crate) fn backups_sync(&mut self) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let Some(wallet) = self.session.wallets.get(b.wallet) else {
            return;
        };
        let spots = self.backup_spots(wallet, Some(b.wallet), &b.answers);
        let sum = wallet.policy.checksum();
        let name = wallet.name.clone();
        let shown = b.shown;
        match self.backup_records.iter_mut().find(|r| r.sum == sum) {
            Some(r) => {
                r.spots = spots;
                r.name = name;
                r.shown |= shown;
            }
            None => self.backup_records.push(Record {
                sum,
                name,
                spots,
                checks: Vec::new(),
                shown,
            }),
        }
    }

    /// A copy by hand of seed `fp` matched in the backup under way: the
    /// seed is checked in it, and one more of its copies in the record.
    pub(crate) fn copy_matched(&mut self, fp: [u8; 4]) {
        if let Some(b) = self.backup.as_mut()
            && !b.checked.contains(&fp)
        {
            b.checked.push(fp);
        }
        let Some(sum) = self
            .backup
            .as_ref()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.policy.checksum())
        else {
            return;
        };
        if !self.backup_records.iter().any(|r| r.sum == sum) {
            self.backups_sync();
        }
        if let Some(r) = self.backup_records.iter_mut().find(|r| r.sum == sum) {
            match r.checks.iter_mut().find(|(f, _)| *f == fp) {
                Some((_, n)) => *n += 1,
                None => r.checks.push((fp, 1)),
            }
        }
    }

    /// The descriptor was shown in the backup under way.
    pub(crate) fn backups_shown(&mut self) {
        let Some(sum) = self
            .backup
            .as_ref()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.policy.checksum())
        else {
            return;
        };
        if let Some(r) = self.backup_records.iter_mut().find(|r| r.sum == sum) {
            r.shown = true;
        }
    }

    /// The seeds of loaded wallet `i` whose copy was checked this
    /// power-on, for a backup of it opened again.
    pub(crate) fn backups_checked(&self, i: usize) -> Vec<[u8; 4]> {
        let Some(sum) = self.session.wallets.get(i).map(|w| w.policy.checksum()) else {
            return Vec::new();
        };
        self.backup_records
            .iter()
            .find(|r| r.sum == sum)
            .map(|r| {
                r.checks
                    .iter()
                    .filter(|(_, n)| *n > 0)
                    .map(|(f, _)| *f)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every wallet known: loaded, in an open vault, or remembered of a
    /// locked one, once each by descriptor checksum, in that order.
    pub fn backups(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = Vec::new();
        let seen = |out: &[Entry], sum: &str| out.iter().any(|e| e.sum == sum);
        for (i, w) in self.session.wallets.iter().enumerate() {
            let sum = w.policy.checksum();
            if seen(&out, &sum) {
                continue;
            }
            out.push(Entry {
                lines: self.backup_lines(w, Some(i), &sum),
                sum,
                name: w.name.clone(),
                shape: Session::shape(w),
                wallet: Some(i),
            });
        }
        for v in &self.vaults.open {
            for (_, r) in v.contents.of(kind::WALLET) {
                let Some(policy) = r
                    .text(field::WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
                else {
                    continue;
                };
                let sum = policy.checksum();
                if seen(&out, &sum) {
                    continue;
                }
                let w = Wallet {
                    name: r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
                    policy,
                    source: "Vault".to_string(),
                };
                out.push(Entry {
                    lines: self.backup_lines(&w, None, &sum),
                    sum,
                    shape: Session::shape(&w),
                    name: w.name,
                    wallet: None,
                });
            }
        }
        for s in &self.vaults.summaries {
            for (k, (name, shape)) in s.wallets.iter().enumerate() {
                let Some(sum) = s.sums.get(k).filter(|c| !c.is_empty()) else {
                    continue;
                };
                if seen(&out, sum) {
                    continue;
                }
                let lines = match self.backup_records.iter().find(|r| &r.sum == sum) {
                    Some(r) => Some(self.record_lines(r, sum, &[])),
                    None if s.planned.contains(sum) => Some(vec![Line {
                        place: "Plan".to_string(),
                        holds: String::new(),
                        tag: None,
                        state: format!("In {} · locked", s.name),
                        tone: Tone::Dim,
                        at: None,
                        paper: false,
                    }]),
                    None => None,
                };
                out.push(Entry {
                    sum: sum.clone(),
                    name: name.clone(),
                    shape: shape.clone(),
                    wallet: None,
                    lines,
                });
            }
        }
        out
    }

    /// A wallet's map with what was seen of each place: from its record,
    /// else from the plan an open vault keeps for it; `None` with neither.
    fn backup_lines(&self, w: &Wallet, wi: Option<usize>, sum: &str) -> Option<Vec<Line>> {
        let shape = self.plan_shape_of(w);
        let kept = self.plan_load_of(w, &shape);
        // The places' names: the backup under way's, else the vault's.
        let names = match self.backup.as_ref() {
            Some(b) if wi.is_some_and(|i| i == b.wallet) => b.names.clone(),
            _ => kept.as_ref().map(|(_, n)| n.clone()).unwrap_or_default(),
        };
        if let Some(r) = self.backup_records.iter().find(|r| r.sum == sum) {
            return Some(self.record_lines(r, sum, &names));
        }
        let (a, _) = kept?;
        let r = Record {
            sum: sum.to_string(),
            name: w.name.clone(),
            spots: self.backup_spots(w, wi, &a),
            checks: Vec::new(),
            shown: false,
        };
        Some(self.record_lines(&r, sum, &names))
    }

    /// A record's lines, each place named and given what was seen of it.
    fn record_lines(&self, r: &Record, sum: &str, names: &[String]) -> Vec<Line> {
        r.spots
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let place = match s.at {
                    At::Place(p) => names
                        .get(p)
                        .map(|n| n.trim())
                        .filter(|n| !n.is_empty())
                        .map_or_else(|| format!("Place {}", p + 1), str::to_string),
                    At::Vault => "Vault".to_string(),
                    At::Files => format!("{} of files", self.medium.cap()),
                    At::Software => "Watch-only software".to_string(),
                    At::Away => "Own devices".to_string(),
                };
                let (state, tone) = match s.at {
                    At::Place(_) if !s.copies.is_empty() => {
                        if r.place_checked(i) {
                            ("Checked".to_string(), Tone::Ok)
                        } else {
                            ("Not checked".to_string(), Tone::Warn)
                        }
                    }
                    At::Vault => self.vault_state(sum),
                    At::Away => ("Not here".to_string(), Tone::Dim),
                    _ if !s.secret.is_empty() && self.secret_out(&s.secret) => {
                        (format!("{}, unprotected", self.medium.for_box()), Tone::Err)
                    }
                    _ if !s.files.is_empty() => self.files_state(&s.files),
                    At::Software if r.shown => ("Shown".to_string(), Tone::Ok),
                    At::Software => ("Not shown".to_string(), Tone::Warn),
                    _ => ("Not seen".to_string(), Tone::Dim),
                };
                Line {
                    place,
                    holds: if s.holds.is_empty() {
                        "Nothing".to_string()
                    } else {
                        s.holds.join(", ")
                    },
                    tag: Some(s.tag),
                    state,
                    tone,
                    at: Some(s.at),
                    paper: matches!(s.at, At::Place(_)) && !s.copies.is_empty(),
                }
            })
            .collect()
    }

    /// Whether a seed file starting with one of `prefixes` is For the
    /// stick or was written.
    fn secret_out(&self, prefixes: &[String]) -> bool {
        let named = |n: &str| prefixes.iter().any(|p| n.starts_with(p.as_str()));
        self.outbox.iter().any(|i| i.secret && named(&i.name))
            || self
                .receipt
                .as_ref()
                .is_some_and(|r| r.files.iter().any(|f| f.verified && named(&f.name)))
    }

    /// Where public files stand: on the stick the last visit wrote them
    /// to, For the stick, or not made.
    fn files_state(&self, names: &[String]) -> (String, Tone) {
        let written = |n: &String| self.receipt.as_ref().is_some_and(|r| r.wrote(n));
        if let Some(r) = self.receipt.as_ref()
            && names.iter().all(written)
        {
            return (on_stick(&r.label, r.time), Tone::Ok);
        }
        if names
            .iter()
            .all(|n| written(n) || self.outbox.iter().any(|i| &i.name == n))
        {
            return (self.medium.for_box().to_string(), Tone::Warn);
        }
        ("Not made".to_string(), Tone::Warn)
    }

    /// The vault that holds wallet `sum` and where its file stands: an
    /// open vault holding it, else a locked one remembered to.
    pub(crate) fn vault_state(&self, sum: &str) -> (String, Tone) {
        let files = self.vault_files();
        let holds = |o: &crate::vaults::OpenVault| {
            o.contents.of(kind::WALLET).any(|(_, r)| {
                r.text(field::WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
                    .is_some_and(|p| p.checksum() == sum)
            })
        };
        let file = self
            .vaults
            .open
            .iter()
            .position(holds)
            .and_then(|v| files.iter().find(|f| f.open == Some(v)))
            .or_else(|| {
                let s = self
                    .vaults
                    .summaries
                    .iter()
                    .find(|s| s.sums.iter().any(|c| c == sum))?;
                files.iter().find(|f| f.header.salt == s.salt)
            });
        let Some(f) = file else {
            // A locked vault seen nowhere this power-on may hold it.
            let unknown = files
                .iter()
                .any(|f| f.open.is_none() && self.vault_summary(f).is_none());
            return if unknown {
                ("A vault is locked".to_string(), Tone::Dim)
            } else {
                ("Not in a vault".to_string(), Tone::Warn)
            };
        };
        let (state, tone) = match self.currency(f) {
            Some(Currency::Current(label)) => {
                let time = self
                    .receipt
                    .as_ref()
                    .filter(|r| r.label == label && r.wrote(&f.name))
                    .and_then(|r| r.time);
                (lower_first(&on_stick(&label, time)), Tone::Ok)
            }
            Some(Currency::NeverWritten) => (self.medium.for_the().to_string(), Tone::Warn),
            Some(Currency::Changed) => ("changed since written".to_string(), Tone::Warn),
            Some(Currency::Unchanged) | None => ("unchanged".to_string(), Tone::Ok),
        };
        (format!("{} · {state}", f.name), tone)
    }

    /// The wallet card's backup line (`docs/SIMPLIFY.md` §5.2): "Backup:
    /// paper ×2 checked · vault.ofv · for the stick", or "Not backed up".
    pub fn backup_line(&self, i: usize) -> String {
        let Some(sum) = self.session.wallets.get(i).map(|w| w.policy.checksum()) else {
            return String::new();
        };
        let entry = self.backups().into_iter().find(|e| e.sum == sum);
        let Some(lines) = entry.and_then(|e| e.lines) else {
            return "Not backed up".to_string();
        };
        let mut parts: Vec<String> = Vec::new();
        let paper: Vec<&Line> = lines.iter().filter(|l| l.paper).collect();
        if !paper.is_empty() {
            let n = paper.len();
            let k = paper.iter().filter(|l| l.tone == Tone::Ok).count();
            parts.push(if k == n {
                format!("paper ×{n} checked")
            } else {
                format!("paper {k} of {n} checked")
            });
        }
        for l in &lines {
            match l.at {
                Some(At::Vault) => parts.push(l.state.clone()),
                Some(At::Files) => parts.push(format!("files {}", lower_first(&l.state))),
                _ => {}
            }
        }
        if parts.is_empty() {
            parts.push("planned".to_string());
        }
        format!("Backup: {}", parts.join(" · "))
    }
}

/// "On STICK, 14:02", or without the time when the clock is not known.
fn on_stick(label: &str, time: Option<u64>) -> String {
    match time {
        Some(t) => format!("On {label}, {}", crate::vaults::time_of_day(t)),
        None => format!("On {label}"),
    }
}

/// The text with its first letter in lower case, to stand mid-line.
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}
