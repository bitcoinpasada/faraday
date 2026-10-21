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

use crate::strings::EN;
use crate::strings::fill;
use faraday_vault::records::{field, kind};
use osk_crypto::work::Compute;

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
    /// The plan's answers, as [`plan::Answers::to_text`] writes them:
    /// what the wallet's card draws its chart from once the backup is
    /// left. Empty in a record kept before it was written.
    pub answers: String,
    /// Each seed's [`crate::wallet::master_id`], by fingerprint, as the
    /// session held it when the record was made or brought up to date:
    /// public, and what a locked vault's summary is compared with, where
    /// the fingerprint only finds a candidate (F65). Empty in a record
    /// kept before it was written.
    pub ids: Vec<([u8; 4], [u8; 32])>,
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
    /// The seeds whose copy by hand it keeps, by fingerprint; for a
    /// vault, the seeds sealed in it.
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
        // The record keeps places: a stick is its place's (`plan::by_place`).
        At::Place(p) | At::Stick(p, _) => format!("place{p}"),
        At::Vault(0) => "vault".into(),
        At::Vault(v) => format!("vault{v}"),
        At::Files => "files".into(),
        At::Software => "software".into(),
        At::Away => "away".into(),
    }
}

fn at_read(s: &str) -> Option<At> {
    Some(match s {
        "vault" => At::Vault(0),
        "files" => At::Files,
        "software" => At::Software,
        "away" => At::Away,
        v if v.starts_with("vault") => At::Vault(v.strip_prefix("vault")?.parse().ok()?),
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

pub(crate) fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn read_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Whether locked vault summary `s` holds seed `fp` whose master id is
/// `id`: `Some(true)` when an entry of that fingerprint has that id,
/// `Some(false)` when no entry has the fingerprint or every one has
/// another id, `None` when an entry of the fingerprint has no id or `id`
/// is not known, so nothing is verified (F65). A lookup per key.
pub(crate) fn summary_has_seed(
    s: &crate::vaults::VaultSummary,
    fp: [u8; 4],
    id: Option<[u8; 32]>,
) -> Option<bool> {
    let text = fp_text(osk_bip::keys::Fingerprint(fp));
    let mut unknown = false;
    for (i, k) in s.keys.iter().enumerate() {
        if *k != text {
            continue;
        }
        match (s.ids.get(i).copied().flatten(), id) {
            (Some(a), Some(b)) if a == b => return Some(true),
            (Some(_), Some(_)) => {}
            _ => unknown = true,
        }
    }
    if unknown { None } else { Some(false) }
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
        for (fp, id) in &r.ids {
            out.push_str(&format!("id\t{}\t{}\n", hex4(*fp), hex32(id)));
        }
        if !r.answers.is_empty() {
            out.push_str(&format!(
                "plan\t{}\n",
                kept_field(&r.answers.trim_end().replace('\n', ";"))
            ));
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
            ["plan", answers] => {
                if let Some(r) = out.last_mut() {
                    r.answers = answers.replace(';', "\n");
                }
            }
            ["id", fp, id] => {
                if let (Some(fp), Some(id), Some(r)) =
                    (read_hex4(fp), read_hex32(id), out.last_mut())
                {
                    r.ids.push((fp, id));
                }
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
    /// The [`crate::wallet::master_id`] the record keeps for seed `fp`.
    /// A lookup.
    fn id_of(&self, fp: [u8; 4]) -> Option<[u8; 32]> {
        self.ids.iter().find(|(f, _)| *f == fp).map(|(_, id)| *id)
    }

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
        c: &Compute,
        wallet: &Wallet,
        wi: Option<usize>,
        a: &plan::Answers,
    ) -> Vec<Spot> {
        let shape = self.plan_shape_of(wallet);
        let seeds = self.backup_seed_list_of(wallet);
        let fp_of = |i: usize| -> Option<[u8; 4]> {
            let k = seeds.get(i)?.1?;
            Some(self.session.keys.get(k)?.fingerprint().0)
        };
        let stem = file_stem(&wallet.name);
        let public = wi.map_or_else(Vec::new, |wi| self.backup_public_names_for_now(c, wi, a));
        // A place with what its sticks keep.
        self.plan_mapped(c, &shape, a)
            .places
            .iter()
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
                        What::Words(i)
                        | What::SeedQr(i)
                        | What::Seed(i)
                        | What::SeedPassphrase(i)
                            if !matches!(spot.at, At::Away) =>
                        {
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
                        // The stick of files of a plan saved before keeps
                        // the checklist's seed files; a stick at a place
                        // its own (`docs/BACKUP-FLOW.md` §19.12 item 6).
                        What::SeedFile(i) if spot.at == At::Files => {
                            if let Some(fp) = fp_of(i) {
                                let fps = fp_text(osk_bip::keys::Fingerprint(fp)).to_lowercase();
                                out.secret.push(format!("{stem}-{fps}-"));
                            }
                        }
                        What::SeedFile(i) | What::SeedQrFile(i) | What::PassphraseFile(i) => {
                            if let Some(fp) = fp_of(i) {
                                let name = crate::backup_flow_wallet::seed_stick_name(
                                    *what,
                                    &fp_text(osk_bip::keys::Fingerprint(fp)),
                                );
                                out.secret.extend(name);
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

    /// Seed `fp`'s [`crate::wallet::master_id`]: the session key's, else
    /// the one record `r` kept. A lookup.
    pub(crate) fn seed_id(&self, r: Option<&Record>, fp: [u8; 4]) -> Option<[u8; 32]> {
        self.session
            .keys
            .iter()
            .find(|k| k.fingerprint().0 == fp)
            .map(|k| k.master_id())
            .or_else(|| r?.id_of(fp))
    }

    /// The [`crate::wallet::master_id`] of each seed `spots` keep that a
    /// session key holds, by fingerprint. A lookup per key.
    fn seed_ids(&self, spots: &[Spot]) -> Vec<([u8; 4], [u8; 32])> {
        let mut out: Vec<([u8; 4], [u8; 32])> = Vec::new();
        for fp in spots.iter().flat_map(|s| &s.copies) {
            if out.iter().any(|(f, _)| f == fp) {
                continue;
            }
            if let Some(k) = self.session.keys.iter().find(|k| k.fingerprint().0 == *fp) {
                out.push((*fp, k.master_id()));
            }
        }
        out
    }

    /// The record of the backup under way, made or brought up to date
    /// from its plan: when its checklist is made. What was checked is
    /// kept.
    pub(crate) fn backups_sync(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let Some(wallet) = self.session.wallets.get(b.wallet) else {
            return;
        };
        let spots = self.backup_spots(&c, wallet, Some(b.wallet), &b.answers);
        let ids = self.seed_ids(&spots);
        let sum = wallet.checksum().to_string();
        let name = wallet.name.clone();
        let shown = b.shown;
        let answers = b.answers.to_text();
        match self.backup_records.iter_mut().find(|r| r.sum == sum) {
            Some(r) => {
                // An id the session no longer holds the key of is kept.
                for (fp, id) in ids {
                    match r.ids.iter_mut().find(|(f, _)| *f == fp) {
                        Some(e) => e.1 = id,
                        None => r.ids.push((fp, id)),
                    }
                }
                r.spots = spots;
                r.name = name;
                r.shown |= shown;
                r.answers = answers;
            }
            None => self.backup_records.push(Record {
                sum,
                name,
                spots,
                checks: Vec::new(),
                shown,
                answers,
                ids,
            }),
        }
    }

    /// A copy by hand of seed `fp` matched in the backup under way in
    /// `form`: that form is checked. Once the seed is checked in every
    /// form its copies ask for, one more of its copies is checked in the
    /// record.
    pub(crate) fn copy_matched(&mut self, fp: [u8; 4], form: crate::plan::CopyForm) {
        let c = self.compute();
        if let Some(b) = self.backup.as_mut()
            && !b.checked.contains(&(fp, form))
        {
            b.checked.push((fp, form));
        }
        if !self.seed_copy_checked(&c, fp) {
            return;
        }
        let Some(sum) = self
            .backup
            .as_ref()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.checksum().to_string())
        else {
            return;
        };
        if !self.backup_records.iter().any(|r| r.sum == sum) {
            self.backups_sync();
        }
        if let Some(r) = self.backup_records.iter_mut().find(|r| r.sum == sum) {
            // One count per copy on paper the plan keeps of the seed:
            // checks past them count no further copy (tracker #156).
            let copies = r
                .spots
                .iter()
                .filter(|s| matches!(s.at, At::Place(_)) && s.copies.contains(&fp))
                .count()
                .max(1) as u32;
            match r.checks.iter_mut().find(|(f, _)| *f == fp) {
                Some((_, n)) => *n = (*n + 1).min(copies),
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
            .map(|w| w.checksum())
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
        let Some(sum) = self.session.wallets.get(i).map(|w| w.checksum()) else {
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
    /// locked one, once each by descriptor checksum, in that order. Kept
    /// in the drawing memo while a frame is drawn (`docs/SPEED.md` §3.3).
    /// Reads each open vault's wallet records as it keeps them read.
    ///
    /// Works out [`crate::drawing::View::backups`], from each open vault's wallet
    /// records as it keeps them read ([`crate::vaults::VaultRecords`]).
    pub fn backups_now(&self, c: &Compute) -> Vec<Entry> {
        let mut out: Vec<Entry> = Vec::new();
        let seen = |out: &[Entry], sum: &str| out.iter().any(|e| e.sum == sum);
        for (i, w) in self.session.wallets.iter().enumerate() {
            let sum = w.checksum().to_string();
            if seen(&out, &sum) {
                continue;
            }
            out.push(Entry {
                lines: self.backup_lines(c, w, Some(i), &sum),
                sum,
                name: w.name.clone(),
                shape: Session::shape(w),
                wallet: Some(i),
            });
        }
        for v in &self.vaults.open {
            for (_, r, d) in v.contents.described_of(c, kind::WALLET) {
                let sum = d.checksum.clone();
                if seen(&out, &sum) {
                    continue;
                }
                let w = Wallet::new(
                    r.text(field::WALLET_NAME)
                        .unwrap_or(EN.backups_wallet)
                        .to_string(),
                    d.policy.clone(),
                    EN.backups_vault.to_string(),
                    crate::wallet::Origin::Vault,
                );
                out.push(Entry {
                    lines: self.backup_lines(c, &w, None, &sum),
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
                    Some(r) => Some(self.record_lines(c, r, &[])),
                    None if s.planned.contains(sum) => Some(vec![Line {
                        place: EN.backups_plan.to_string(),
                        holds: String::new(),
                        tag: None,
                        state: fill(EN.backups_in_locked, &[&s.name.to_string()]),
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
    fn backup_lines(
        &self,
        c: &Compute,
        w: &Wallet,
        wi: Option<usize>,
        sum: &str,
    ) -> Option<Vec<Line>> {
        let shape = self.plan_shape_of(w);
        let kept = self.plan_load_of(c, w, &shape);
        // The places' names: the backup under way's, else the vault's.
        let names = match self.backup.as_ref() {
            Some(b) if wi.is_some_and(|i| i == b.wallet) => b.names.clone(),
            _ => kept.as_ref().map(|(_, n)| n.clone()).unwrap_or_default(),
        };
        if let Some(r) = self.backup_records.iter().find(|r| r.sum == sum) {
            return Some(self.record_lines(c, r, &names));
        }
        let (a, _) = kept?;
        let spots = self.backup_spots(c, w, wi, &a);
        let r = Record {
            sum: sum.to_string(),
            name: w.name.clone(),
            ids: self.seed_ids(&spots),
            spots,
            checks: Vec::new(),
            shown: false,
            answers: String::new(),
        };
        Some(self.record_lines(c, &r, &names))
    }

    /// A record's lines, each place named and given what was seen of it.
    fn record_lines(&self, c: &Compute, r: &Record, names: &[String]) -> Vec<Line> {
        // One vault is "Vault"; several are numbered.
        let vaults = r
            .spots
            .iter()
            .filter(|s| matches!(s.at, At::Vault(_)))
            .count();
        r.spots
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let place = match s.at {
                    At::Place(p) | At::Stick(p, _) => names
                        .get(p)
                        .map(|n| n.trim())
                        .filter(|n| !n.is_empty())
                        .map_or_else(
                            || fill(EN.backups_place, &[&(p + 1).to_string()]),
                            str::to_string,
                        ),
                    At::Vault(_) if vaults == 1 => EN.backups_vault.to_string(),
                    At::Vault(v) => fill(EN.backups_vault_n, &[&(v + 1).to_string()]),
                    At::Files => fill(EN.backups_of_files, &[self.medium.cap()]),
                    At::Software => EN.backups_watch_only.to_string(),
                    At::Away => EN.backups_own_devices.to_string(),
                };
                let (state, tone) = match s.at {
                    At::Place(_) if !s.copies.is_empty() => {
                        if r.place_checked(i) {
                            (EN.backups_checked.to_string(), Tone::Ok)
                        } else {
                            (EN.backups_not_checked.to_string(), Tone::Warn)
                        }
                    }
                    At::Vault(_) => self.vault_state(c, r, &s.copies),
                    At::Away => (EN.backups_not_here.to_string(), Tone::Dim),
                    _ if !s.secret.is_empty() && self.secret_out(&s.secret) => (
                        fill(EN.backups_unprotected, &[self.medium.for_box()]),
                        Tone::Err,
                    ),
                    _ if !s.files.is_empty() => self.files_state(&s.files),
                    At::Software if r.shown => (EN.backups_shown.to_string(), Tone::Ok),
                    At::Software => (EN.backups_not_shown.to_string(), Tone::Warn),
                    _ => (EN.backups_not_seen.to_string(), Tone::Dim),
                };
                Line {
                    place,
                    holds: if s.holds.is_empty() {
                        EN.backups_nothing.to_string()
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
            return (on_stick(&r.label, r.time, self.tz_min), Tone::Ok);
        }
        if names
            .iter()
            .all(|n| written(n) || self.outbox.iter().any(|i| &i.name == n))
        {
            return (self.medium.for_box().to_string(), Tone::Warn);
        }
        (EN.backups_not_made.to_string(), Tone::Warn)
    }

    /// The vault that holds the seeds `seeds` of record `r`, or with
    /// none, its wallet, and where its file stands: an open vault holding
    /// them, else a locked one remembered to. A seed is kept in a vault
    /// when it is that very key, told by all 32 bytes of its master id
    /// (F65): an open vault's record against the session's key, a locked
    /// vault's summary against the id the session or the record keeps.
    pub(crate) fn vault_state(&self, c: &Compute, r: &Record, seeds: &[[u8; 4]]) -> (String, Tone) {
        let sum = r.sum.as_str();
        let files = self.vault_files();
        let holds = |o: &crate::vaults::OpenVault| {
            if !seeds.is_empty() {
                // Each seed as the session holds it, and a record of that
                // very key: the fingerprint finds it, all 32 bytes of the
                // master key's id say it is the one (F65). A seed not held
                // here cannot be told apart, so it is not said kept.
                return seeds.iter().all(|f| {
                    self.session
                        .keys
                        .iter()
                        .find(|k| k.fingerprint().0 == *f)
                        .is_some_and(|k| {
                            o.contents.of(kind::KEY).any(|(_, rec)| {
                                crate::vault_screens::record_is_key(c, self, rec, k)
                            })
                        })
                });
            }
            o.contents
                .described_of(c, kind::WALLET)
                .any(|(_, _, d)| d.checksum == sum)
        };
        let id_of = |f: [u8; 4]| self.seed_id(Some(r), f);
        // Whether locked vault summary `s` holds every seed: `Some(true)`
        // told by the ids, `Some(false)` not, `None` when a seed's
        // fingerprint is there and its id cannot be compared.
        let summary_holds = |s: &crate::vaults::VaultSummary| -> Option<bool> {
            if seeds.is_empty() {
                return Some(s.sums.iter().any(|c| c == sum));
            }
            let mut told = true;
            for f in seeds {
                match summary_has_seed(s, *f, id_of(*f)) {
                    Some(true) => {}
                    Some(false) => return Some(false),
                    None => told = false,
                }
            }
            if told { Some(true) } else { None }
        };
        // The locked vault file summary `s` is of.
        let locked = |s: &crate::vaults::VaultSummary| {
            files
                .iter()
                .find(|f| f.open.is_none() && f.header.salt == s.salt)
        };
        let open_file = self
            .vaults
            .open
            .iter()
            .position(holds)
            .and_then(|v| files.iter().find(|f| f.open == Some(v)));
        let file = open_file.or_else(|| {
            // A locked vault, as remembered when it was open.
            self.vaults
                .summaries
                .iter()
                .filter(|s| summary_holds(s) == Some(true))
                .find_map(&locked)
        });
        // A locked vault whose summary names the seed by fingerprint
        // alone, from before the summaries kept master ids, or a seed no
        // id is kept for: nothing is verified.
        if file.is_none()
            && self
                .vaults
                .summaries
                .iter()
                .any(|s| summary_holds(s).is_none() && locked(s).is_some())
        {
            return (EN.backups_a_vault_is.to_string(), Tone::Dim);
        }
        let Some(f) = file else {
            // A locked vault seen nowhere this power-on may hold it.
            let unknown = files
                .iter()
                .any(|f| f.open.is_none() && self.vault_summary(f).is_none());
            return if unknown {
                (EN.backups_a_vault_is.to_string(), Tone::Dim)
            } else {
                (EN.backups_not_saved_yet.to_string(), Tone::Warn)
            };
        };
        let (state, tone) = match self.currency(c, f) {
            Some(Currency::Current(label)) => {
                let time = self
                    .receipt
                    .as_ref()
                    .filter(|r| r.label == label && r.wrote(&f.name))
                    .and_then(|r| r.time);
                (lower_first(&on_stick(&label, time, self.tz_min)), Tone::Ok)
            }
            Some(Currency::NeverWritten) => (self.medium.for_the().to_string(), Tone::Warn),
            Some(Currency::Changed) => (EN.backups_changed_since.to_string(), Tone::Warn),
            Some(Currency::Unchanged) | None => (EN.backups_unchanged.to_string(), Tone::Ok),
        };
        (format!("{} · {state}", f.name), tone)
    }

    /// Costs what [`Faraday::backups_now`] costs: the wallet card's backup
    /// line, for a decision ([`Faraday::backup_line_in`]).
    pub fn backup_line(&self, c: &Compute, i: usize) -> String {
        self.backup_line_in(i, self.backups_now(c))
    }

    /// The wallet card's backup line (`docs/SIMPLIFY.md` §5.2): "Backup:
    /// paper ×2 checked · vault.ofv · for the stick", or "Not backed up",
    /// from `backups`, every wallet known and where its backup is. Cheap.
    pub(crate) fn backup_line_in(&self, i: usize, backups: Vec<Entry>) -> String {
        let Some(sum) = self.session.wallets.get(i).map(|w| w.checksum()) else {
            return String::new();
        };
        let entry = backups.into_iter().find(|e| e.sum == sum);
        let Some(lines) = entry.and_then(|e| e.lines) else {
            return EN.backups_not_backed.to_string();
        };
        let mut parts: Vec<String> = Vec::new();
        let paper: Vec<&Line> = lines.iter().filter(|l| l.paper).collect();
        if !paper.is_empty() {
            let n = paper.len();
            let k = paper.iter().filter(|l| l.tone == Tone::Ok).count();
            parts.push(if k == n {
                fill(EN.backups_paper_checked, &[&n.to_string()])
            } else {
                fill(
                    EN.backups_paper_of_checked,
                    &[&k.to_string(), &n.to_string()],
                )
            });
        }
        for l in &lines {
            match l.at {
                Some(At::Vault(_)) => parts.push(l.state.clone()),
                Some(At::Files) => parts.push(fill(
                    EN.backups_files,
                    &[&lower_first(&l.state).to_string()],
                )),
                _ => {}
            }
        }
        if parts.is_empty() {
            parts.push(EN.backups_planned.to_string());
        }
        fill(EN.backups_backup, &[&parts.join(" · ").to_string()])
    }
}

/// "On STICK, 14:02", or without the time when the clock is not known.
fn on_stick(label: &str, time: Option<u64>, tz_min: i16) -> String {
    match time {
        Some(t) => fill(
            EN.backups_on_2,
            &[
                label,
                &crate::vaults::time_of_day(crate::vaults::local_secs(t, tz_min)).to_string(),
            ],
        ),
        None => fill(EN.backups_on, &[label]),
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
