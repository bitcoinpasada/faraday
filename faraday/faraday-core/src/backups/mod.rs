//! The Backups screen's content (`docs/history/SIMPLIFY.md` §5): every wallet the
//! app knows, once each by descriptor checksum, and under each the backup
//! map as the plan draws it, each place with what this device saw of it.
//!
//! What a plan holds is recorded here when its checklist is made, and
//! each check of a copy by hand as it matches: a paper place reads
//! checked only when a check of that copy happened on this device. The
//! records are kept across a lock in the kept state and gone at
//! power-off, as the checked wallets are. A place's name is never in
//! them: names are kept in the vault alone.

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_vault::records::{field, kind};

use crate::plan::{self, At, Tag, What};
use crate::vault::Currency;
use crate::wallet::{Session, Wallet, fp_text};
use crate::wallet_backup::Tone;
use crate::{Action, Faraday, Screen, file_stem};

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
    /// The blank templates were marked printed in this wallet's backup.
    /// Kept with the record, as the lock drops the backup and nothing else
    /// marks the item; kept in step by [`Faraday::backups_marked`].
    pub printed: bool,
    /// The envelopes were marked closed in this wallet's backup. Kept
    /// like [`Record::printed`].
    pub envelopes: bool,
    /// The seeds whose passphrase was typed back and matched in this
    /// wallet's backup, by fingerprint. Kept like [`Record::printed`].
    pub passphrases: Vec<[u8; 4]>,
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

/// A record's marks, as a backup opened again takes them up.
#[derive(Default)]
pub(crate) struct Marks {
    pub printed: bool,
    pub envelopes: bool,
    pub shown: bool,
    pub passphrases: Vec<[u8; 4]>,
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
    s: &crate::vault::VaultSummary,
    fp: [u8; 4],
    id: Option<[u8; 32]>,
) -> Option<bool> {
    let text = fp_text(faraday_bip::keys::Fingerprint(fp));
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
        if r.printed || r.envelopes {
            out.push_str(&format!(
                "marks\t{}\t{}\n",
                u8::from(r.printed),
                u8::from(r.envelopes)
            ));
        }
        for fp in &r.passphrases {
            out.push_str(&format!("pass\t{}\n", hex4(*fp)));
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
            ["pass", fp] => {
                if let (Some(fp), Some(r)) = (read_hex4(fp), out.last_mut()) {
                    r.passphrases.push(fp);
                }
            }
            ["marks", printed, envelopes] => {
                if let Some(r) = out.last_mut() {
                    r.printed = printed == "1";
                    r.envelopes = envelopes == "1";
                }
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

/// The Backups area: the wallet the screen shows, and what this power-on
/// saw of each wallet's backup, one record per descriptor checksum. The
/// records are kept across a lock by `memory.rs`'s glue and gone at
/// power-off.
#[derive(Default)]
pub(crate) struct BackupsArea {
    at: usize,
    records: Vec<Record>,
}

impl BackupsArea {
    /// The wallet Backups shows, a page each on a small panel.
    pub(crate) fn at(&self) -> usize {
        self.at
    }

    /// Shows the wallet at `at`.
    pub(crate) fn set_at(&mut self, at: usize) {
        self.at = at;
    }

    /// Every record.
    pub(crate) fn records(&self) -> &[Record] {
        &self.records
    }

    /// The record of the wallet of descriptor checksum `sum`.
    pub(crate) fn record(&self, sum: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.sum == sum)
    }

    /// Takes in the records the previous process kept.
    pub(crate) fn set_records(&mut self, records: Vec<Record>) {
        self.records = records;
    }

    fn record_mut(&mut self, sum: &str) -> Option<&mut Record> {
        self.records.iter_mut().find(|r| r.sum == sum)
    }

    /// Makes or brings up to date the record of wallet `sum` from its
    /// plan. What was checked is kept.
    pub(crate) fn sync(&mut self, rec: Synced) {
        let Synced {
            sum,
            name,
            spots,
            ids,
            shown,
            answers,
        } = rec;
        match self.record_mut(&sum) {
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
            None => self.records.push(Record {
                sum,
                name,
                spots,
                checks: Vec::new(),
                shown,
                printed: false,
                envelopes: false,
                passphrases: Vec::new(),
                answers,
                ids,
            }),
        }
    }

    /// Whether wallet `sum` has a record.
    pub(crate) fn has(&self, sum: &str) -> bool {
        self.records.iter().any(|r| r.sum == sum)
    }

    /// One more copy by hand of seed `fp` is checked in wallet `sum`'s
    /// record: one count per copy on paper the plan keeps of the seed,
    /// and checks past them count no further copy.
    pub(crate) fn copy_checked(&mut self, sum: &str, fp: [u8; 4]) {
        if let Some(r) = self.record_mut(sum) {
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

    /// The descriptor was shown in wallet `sum`'s backup.
    pub(crate) fn shown(&mut self, sum: &str) {
        if let Some(r) = self.record_mut(sum) {
            r.shown = true;
        }
    }

    /// The templates or the envelopes of wallet `sum`'s backup were
    /// marked done.
    pub(crate) fn marked(&mut self, sum: &str, printed: bool, envelopes: bool) {
        if let Some(r) = self.record_mut(sum) {
            r.printed |= printed;
            r.envelopes |= envelopes;
        }
    }

    /// A passphrase of seed `fp` typed back matched in wallet `sum`'s
    /// backup.
    pub(crate) fn passphrase_checked(&mut self, sum: &str, fp: [u8; 4]) {
        if let Some(r) = self.record_mut(sum)
            && !r.passphrases.contains(&fp)
        {
            r.passphrases.push(fp);
        }
    }

    /// What wallet `sum`'s backup was marked with that nothing else holds.
    pub(crate) fn marks(&self, sum: &str) -> Marks {
        self.record(sum).map_or_else(Marks::default, |r| Marks {
            printed: r.printed,
            envelopes: r.envelopes,
            shown: r.shown,
            passphrases: r.passphrases.clone(),
        })
    }

    /// The seeds of wallet `sum` whose copy was checked this power-on.
    pub(crate) fn checked(&self, sum: &str) -> Vec<[u8; 4]> {
        self.record(sum)
            .map(|r| {
                r.checks
                    .iter()
                    .filter(|(_, n)| *n > 0)
                    .map(|(f, _)| *f)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// What a plan gives its wallet's record when the checklist is made.
pub(crate) struct Synced {
    pub sum: String,
    pub name: String,
    pub spots: Vec<Spot>,
    pub ids: Vec<([u8; 4], [u8; 32])>,
    pub shown: bool,
    pub answers: String,
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
    /// Costs what `backup_public_names_for_now` and `plan_mapped` cost. The
    /// map of wallet `wallet` under answers `a`, as a record keeps it. `wi`
    /// is the loaded wallet, which names the public files.
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
            Some(self.session.keys.info(k)?.fingerprint.0)
        };
        let stem = file_stem(&wallet.name);
        let public = wi.map_or_else(Vec::new, |wi| self.backup_public_names_for_now(c, wi, a));
        // A place with what its sticks keep.
        self.kept
            .plan_mapped(c, &shape, a)
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
                        // its own (`docs/BACKUP-FLOW.md` §17.12 item 6).
                        What::SeedFile(i) if spot.at == At::Files => {
                            if let Some(fp) = fp_of(i) {
                                let fps =
                                    fp_text(faraday_bip::keys::Fingerprint(fp)).to_lowercase();
                                out.secret.push(format!("{stem}-{fps}-"));
                            }
                        }
                        What::SeedFile(i) | What::SeedQrFile(i) | What::PassphraseFile(i) => {
                            if let Some(fp) = fp_of(i) {
                                let name = crate::backup_flow::wallet::seed_stick_name(
                                    *what,
                                    &fp_text(faraday_bip::keys::Fingerprint(fp)),
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
            .all()
            .find(|k| k.fingerprint.0 == fp)
            .map(|k| k.id.0)
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
            if let Some(k) = self.session.keys.all().find(|k| k.fingerprint.0 == *fp) {
                out.push((*fp, k.id.0));
            }
        }
        out
    }

    /// The record of the backup under way, made or brought up to date
    /// from its plan: when its checklist is made. What was checked is
    /// kept.
    pub(crate) fn backups_sync(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
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
        self.backups_area.sync(Synced {
            sum,
            name,
            spots,
            ids,
            shown,
            answers,
        });
    }

    /// A copy by hand of seed `fp` matched in the backup under way in
    /// `form`: that form is checked. Once the seed is checked in every
    /// form its copies ask for, one more of its copies is checked in the
    /// record.
    pub(crate) fn copy_matched(&mut self, fp: [u8; 4], form: crate::plan::CopyForm) {
        let c = self.compute();
        self.backup.note_checked(fp, form);
        if !self.seed_copy_checked(&c, fp) {
            return;
        }
        let Some(sum) = self
            .backup
            .state()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.checksum().to_string())
        else {
            return;
        };
        if !self.backups_area.has(&sum) {
            self.backups_sync();
        }
        self.backups_area.copy_checked(&sum, fp);
    }

    /// The descriptor was shown in the backup under way.
    pub(crate) fn backups_shown(&mut self) {
        let Some(sum) = self
            .backup
            .state()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.checksum())
        else {
            return;
        };
        self.backups_area.shown(sum);
    }

    /// The templates or the envelopes of the backup under way were marked
    /// done by their press: the record keeps it, for a backup opened again
    /// after a lock. The record was made when the checklist was.
    pub(crate) fn backups_marked(&mut self) {
        let Some((sum, printed, envelopes)) = self.backup.state().and_then(|b| {
            let w = self.session.wallets.get(b.wallet)?;
            Some((w.checksum().to_string(), b.printed, b.envelopes))
        }) else {
            return;
        };
        self.backups_area.marked(&sum, printed, envelopes);
    }

    /// A passphrase of seed `fp` typed back matched in the backup under
    /// way: the record keeps it, for a backup opened again after a lock.
    pub(crate) fn backups_passphrase_checked(&mut self, fp: [u8; 4]) {
        let Some(sum) = self
            .backup
            .state()
            .and_then(|b| self.session.wallets.get(b.wallet))
            .map(|w| w.checksum().to_string())
        else {
            return;
        };
        self.backups_area.passphrase_checked(&sum, fp);
    }

    /// What loaded wallet `i`'s backup was marked with that nothing else
    /// holds, for a backup of it opened again: the templates, the
    /// envelopes, the descriptor shown, the passphrases checked.
    pub(crate) fn backups_marks(&self, i: usize) -> Marks {
        let Some(sum) = self.session.wallets.get(i).map(|w| w.checksum()) else {
            return Marks::default();
        };
        self.backups_area.marks(sum)
    }

    /// The seeds of loaded wallet `i` whose copy was checked this
    /// power-on, for a backup of it opened again.
    pub(crate) fn backups_checked(&self, i: usize) -> Vec<[u8; 4]> {
        let Some(sum) = self.session.wallets.get(i).map(|w| w.checksum()) else {
            return Vec::new();
        };
        self.backups_area.checked(sum)
    }

    /// Costs what `backup_lines`, `described_of` and `record_lines` cost.
    /// Every wallet known: loaded, in an open vault, or remembered of a
    /// locked one, once each by descriptor checksum, in that order. Kept in
    /// the drawing memo while a frame is drawn (`docs/SPEED.md` §3.3).
    /// Reads each open vault's wallet records as it keeps them read.
    ///
    /// Works out [`crate::drawing::View::backups`], from each open vault's wallet
    /// records as it keeps them read ([`crate::vault::Vaults::described`]).
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
            for (_, r, d) in self.vaults.described_of(c, v.id, kind::WALLET) {
                let sum = d.checksum.clone();
                if seen(&out, &sum) {
                    continue;
                }
                let w = Wallet::new(
                    r.public_text(field::WALLET_NAME)
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
                let lines = match self.backups_area.record(sum) {
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

    /// Costs what `plan_load_of`, `record_lines` and `backup_spots` cost. A
    /// wallet's map with what was seen of each place: from its record, else
    /// from the plan an open vault keeps for it; `None` with neither.
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
        let names = match self.backup.state() {
            Some(b) if wi.is_some_and(|i| i == b.wallet) => b.names.clone(),
            _ => kept.as_ref().map(|(_, n)| n.clone()).unwrap_or_default(),
        };
        if let Some(r) = self.backups_area.record(sum) {
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
            printed: false,
            envelopes: false,
            passphrases: Vec::new(),
            answers: String::new(),
        };
        Some(self.record_lines(c, &r, &names))
    }

    /// Costs what `vault_state` costs. A record's lines, each place named
    /// and given what was seen of it.
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
                .visit
                .receipt()
                .as_ref()
                .is_some_and(|r| r.files.iter().any(|f| f.verified && named(&f.name)))
    }

    /// Where public files stand: on the stick the last visit wrote them
    /// to, For the stick, or not made.
    fn files_state(&self, names: &[String]) -> (String, Tone) {
        let written = |n: &String| self.visit.receipt().as_ref().is_some_and(|r| r.wrote(n));
        if let Some(r) = self.visit.receipt().as_ref()
            && names.iter().all(written)
        {
            return (on_stick(&r.label, r.time, self.tz_min()), Tone::Ok);
        }
        if names
            .iter()
            .all(|n| written(n) || self.outbox.iter().any(|i| &i.name == n))
        {
            return (self.medium.for_box().to_string(), Tone::Warn);
        }
        (EN.backups_not_made.to_string(), Tone::Warn)
    }

    /// Costs what `record_is_key`, `described_of` and `currency` cost. The
    /// vault that holds the seeds `seeds` of record `r`, or with none, its
    /// wallet, and where its file stands: an open vault holding them, else
    /// a locked one remembered to. A seed is kept in a vault when it is
    /// that very key, told by all 32 bytes of its master id (F65): an open
    /// vault's record against the session's key, a locked vault's summary
    /// against the id the session or the record keeps.
    pub(crate) fn vault_state(&self, c: &Compute, r: &Record, seeds: &[[u8; 4]]) -> (String, Tone) {
        let sum = r.sum.as_str();
        let files = self.vault_files();
        let holds = |o: &crate::vault::OpenVault| {
            if !seeds.is_empty() {
                // Each seed as the session holds it, and a record of that
                // very key: the fingerprint finds it, all 32 bytes of the
                // master key's id say it is the one (F65). A seed not held
                // here cannot be told apart, so it is not said kept.
                return seeds.iter().all(|f| {
                    self.session
                        .keys
                        .all()
                        .find(|k| k.fingerprint.0 == *f)
                        .is_some_and(|k| {
                            self.vaults.records(o.id).of(kind::KEY).any(|(_, rec)| {
                                crate::vault::screen::record_is_key(c, self, rec, k)
                            })
                        })
                });
            }
            self.vaults
                .described_of(c, o.id, kind::WALLET)
                .any(|(_, _, d)| d.checksum == sum)
        };
        let id_of = |f: [u8; 4]| self.seed_id(Some(r), f);
        // Whether locked vault summary `s` holds every seed: `Some(true)`
        // told by the ids, `Some(false)` not, `None` when a seed's
        // fingerprint is there and its id cannot be compared.
        let summary_holds = |s: &crate::vault::VaultSummary| -> Option<bool> {
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
        let locked = |s: &crate::vault::VaultSummary| {
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
            return (EN.backups_vault_locked.to_string(), Tone::Dim);
        }
        let Some(f) = file else {
            // A locked vault seen nowhere this power-on may hold it.
            let unknown = files
                .iter()
                .any(|f| f.open.is_none() && self.vault_summary(f).is_none());
            return if unknown {
                (EN.backups_vault_locked.to_string(), Tone::Dim)
            } else {
                (EN.backups_not_saved_yet.to_string(), Tone::Warn)
            };
        };
        let (state, tone) = match self.currency(c, f) {
            Some(Currency::Current(label)) => {
                let time = self
                    .visit
                    .receipt()
                    .as_ref()
                    .filter(|r| r.label == label && r.wrote(&f.name))
                    .and_then(|r| r.time);
                (
                    lower_first(&on_stick(&label, time, self.tz_min())),
                    Tone::Ok,
                )
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

    /// The wallet card's backup line (`docs/history/SIMPLIFY.md` §5.2): "Backup:
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
            EN.backups_on_stick_timed,
            &[
                label,
                &crate::vault::time_of_day(crate::vault::local_secs(t, tz_min)).to_string(),
            ],
        ),
        None => fill(EN.backups_on_stick, &[label]),
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

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Backups screen's actions (`BackupsOf`, `BackupsPage`).
    pub(crate) fn backups_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::BackupsOf(i) => {
                let sum = self
                    .session
                    .wallets
                    .get(i)
                    .map(|w| w.checksum().to_string());
                let at = self
                    .backups_now(c)
                    .iter()
                    .position(|e| Some(&e.sum) == sum.as_ref())
                    .unwrap_or(0);
                self.act(Action::Nav(Screen::Backups));
                self.backups_area.set_at(at);
                if !self.compact {
                    self.list_offset = crate::backups::screen::offset_of_now(c, self, at);
                }
            }
            Action::BackupsPage(i) => {
                self.backups_area.set_at(i);
                self.list_offset = 0.0;
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------
// The checklist's codex32 item (`docs/CODEX32.md` §9.2, §9.4, §9.7): the
// glue between its state (`crate::codex32_backup`), the plan and the keys.
// ---------------------------------------------------------------------

impl Faraday {
    /// Costs what the press costs: a split of the seed (Make them, a new
    /// set), a QR encoding (a string's Codex32QR), an interpolation of the
    /// strings typed (the rebuild check, a share derived). A press on the
    /// checklist's codex32 item.
    pub(crate) fn backup_c32(&mut self, p: crate::codex32_backup::Press, c: &Compute) {
        use crate::codex32_backup::Press;
        if matches!(p, Press::PaperOpen | Press::PaperK(_) | Press::PaperN(_)) {
            return self.backup.paper_c32(p);
        }
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(i) = b.c32.as_ref().map(|s| s.seed) else {
            return;
        };
        let Some(key) = self.backup_seed_list_now(b.wallet).get(i).and_then(|x| x.1) else {
            return;
        };
        match p {
            Press::Make => self.c32_make(c, i, key),
            Press::Open(j) => {
                if let Some(st) = self.backup.c32_mut() {
                    st.open(usize::from(j));
                }
            }
            Press::Key(k) => {
                self.backup.c32_key(k);
            }
            Press::Qr => self.c32_qr(c),
            Press::Next => self.c32_next(c, i, key),
            Press::Skip => {
                let Some(st) = self.backup.c32_mut() else {
                    return;
                };
                let Some(j) = st.at else {
                    return;
                };
                if st.skip_ask {
                    self.backup.c32_checked(i, j);
                    self.c32_next(c, i, key);
                } else {
                    st.skip_ask = true;
                }
            }
            Press::List => {
                if let Some(st) = self.backup.c32_mut() {
                    st.list();
                }
            }
            Press::Set(s) => self.c32_set(c, s, i, key),
            Press::PaperOpen | Press::PaperK(_) | Press::PaperN(_) => {}
        }
    }

    /// Costs a split of the seed. **Make them**: the set the plan makes
    /// for seed `i` (session key `key`), from this device's randomness;
    /// its identifier is the key's (`identifier_for` its fingerprint), its
    /// form fixed from now on, and its facts kept with the key
    /// (`docs/CODEX32.md` §9.4). Opens the first string on paper.
    fn c32_make(&mut self, c: &Compute, i: usize, key: usize) {
        use crate::plan::codex32::index_of;
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(form) = b.answers.c32(i).cloned().filter(|f| !f.fixed()) else {
            return;
        };
        // A split is a fresh secret: none in the demo (`docs/DEMO.md` §5.3).
        if form.k >= 2 && !self.makes_secrets() {
            return;
        }
        let Some(fp) = self.session.keys.info(key).map(|k| k.fingerprint.0) else {
            return;
        };
        let Some(draw) = self.fresh(b"codex32 plan") else {
            return self.toast(EN.paper_no_randomness);
        };
        let n = u8::try_from(form.indices.len()).unwrap_or(0);
        let made = match self.session.keys.codex32_shares(
            c,
            key,
            form.k,
            n,
            &draw,
            faraday_keys::Reveal::PaperForm,
        ) {
            Ok(strings) => strings,
            Err(faraday_keys::Paper::Refused(e)) => {
                return self.toast(&fill(EN.paper_codex32, &[&e]));
            }
            Err(_) => return,
        };
        let list: Vec<(usize, zeroize::Zeroizing<String>)> = made
            .iter()
            .filter_map(|s| Some((index_of(*s.as_bytes().get(8)?)?, s.clone())))
            .collect();
        let id = faraday_bip::codex32::identifier_for(fp);
        let length = list
            .first()
            .and_then(|(_, s)| u8::try_from(s.len()).ok())
            .unwrap_or(form.length);
        self.c32_fix(c, i, key, (form.k, id, length), list);
    }

    /// Costs a map of the plan. Seed `i`'s set, now made: its form fixed
    /// to `k`, `id` and `length` at the indices of `list`, the strings
    /// kept with the item and its facts with key `key`, each placed where
    /// the plan has it on paper (`docs/CODEX32.md` §9.4). Opens the first
    /// string on paper.
    fn c32_fix(
        &mut self,
        c: &Compute,
        i: usize,
        key: usize,
        (k, id, length): (u8, [u8; 4], u8),
        list: Vec<(usize, zeroize::Zeroizing<String>)>,
    ) {
        use faraday_vault::codex32::{SetFacts, State};
        let Some(b) = self.backup.state() else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
        let mut answers = b.answers.clone();
        let fixed = plan::codex32::C32 {
            seed: i,
            k,
            length,
            indices: list.iter().map(|(j, _)| *j).collect(),
            identifier: Some(id),
            unknown: 0,
        };
        answers.set_c32(&shape, i, Some(fixed.clone()));
        let mapped = plan::Mapped::of(c, &shape, &answers);
        let paper = plan::codex32::paper_strings(&answers, &mapped, i);
        let strings = fixed
            .indices
            .iter()
            .map(|&j| {
                let state = if paper.contains(&j) {
                    State::Placed
                } else {
                    State::Made
                };
                (plan::codex32::CHARSET[j], state)
            })
            .collect();
        self.session.keys.set_codex32(
            key,
            Some(SetFacts {
                threshold: k,
                identifier: id,
                length,
                unknown: 0,
                strings,
            }),
        );
        self.backup
            .change_answers(&shape, &|a, _| *a = answers.clone());
        if let Some(st) = self.backup.c32_mut() {
            st.keep(list);
            if let Some(&j) = paper.first() {
                st.open(j);
            }
        }
    }

    /// Costs a QR encoding. **String** · **Codex32QR**: the open string
    /// made here, at 48 characters, as its Codex32QR (upper case, byte
    /// mode, version 3, level L, `docs/CODEX32.md` §10.2), or back to the
    /// string.
    fn c32_qr(&mut self, c: &Compute) {
        use faraday_codec::qr::{Ecc, Payload, encode_bounded};
        let Some(st) = self.backup.c32_mut() else {
            return;
        };
        if st.qr.take().is_some() {
            return;
        }
        let Some(s) = st.at.and_then(|j| st.made(j)) else {
            return;
        };
        if s.len() != 48 {
            return;
        }
        let mut upper = zeroize::Zeroizing::new(s.as_bytes().to_vec());
        upper.make_ascii_uppercase();
        st.qr = encode_bounded(c, Payload::Bytes(&upper), Ecc::Low, 3).ok();
    }

    /// Costs an interpolation of the strings typed, once enough are.
    /// **Continue** on a string: it must be typed back the same, or
    /// skipped; then, for a set that exists, whether the strings typed so
    /// far rebuild this seed, once enough are; then the next string on
    /// paper not yet checked, or the list once every one is.
    fn c32_next(&mut self, c: &Compute, i: usize, key: usize) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(form) = b.answers.c32(i).cloned() else {
            return;
        };
        let checked = b.c32_checked.clone();
        let shape = self.plan_shape_now(b.wallet);
        let mapped = self.kept.plan_mapped(c, &shape, &b.answers);
        let paper = plan::codex32::paper_strings(&b.answers, &mapped, i);
        let Some(st) = b.c32.as_ref() else {
            return;
        };
        let Some(j) = st.at else {
            return;
        };
        if !checked.contains(&(i, j)) {
            if let Some(st) = self.backup.c32_mut() {
                st.note = Some(EN.codex32_type_it_back.to_string());
            }
            return;
        }
        let rebuilds = (st.rebuilds.is_none() && st.held.len() >= form.need()).then(|| {
            let parts: Vec<faraday_bip::codex32::Codex32> = st
                .held
                .iter()
                .filter_map(|s| faraday_bip::codex32::Codex32::parse(s).ok())
                .collect();
            self.session.keys.codex32_rebuilds(c, key, &parts)
        });
        let next = paper.iter().copied().find(|x| !checked.contains(&(i, *x)));
        if let Some(st) = self.backup.c32_mut() {
            if let Some(r) = rebuilds.flatten() {
                st.rebuilds = Some(r);
            }
            match next {
                Some(x) => st.open(x),
                None => st.list(),
            }
        }
    }
}

impl Faraday {
    /// Costs what the press costs: an interpolation of `k` strings (a
    /// share derived) or a split of the seed (a new set). The set's
    /// actions on seed `i`'s codex32 item (`docs/CODEX32.md` §9.7), key
    /// `key` its session key.
    fn c32_set(&mut self, c: &Compute, p: crate::codex32_backup::SetPress, i: usize, key: usize) {
        use crate::codex32_backup::{SetPress, SetStep, next_index, rotation};
        use crate::plan::codex32::C32;
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(form) = b.answers.c32(i).cloned().filter(C32::fixed) else {
            return;
        };
        let Some(st) = self.backup.c32_mut() else {
            return;
        };
        match p {
            SetPress::Leave => {
                st.set = SetStep::None;
                st.held.clear();
                st.typed.clear();
                st.found = None;
                st.note = None;
            }
            SetPress::Add | SetPress::Replace(_) => {
                let index = match p {
                    SetPress::Replace(j) => Some(j),
                    _ => next_index(&form).and_then(|j| u8::try_from(j).ok()),
                };
                if index.is_none() {
                    st.note = Some(EN.c32_set_full.to_string());
                    return;
                }
                st.list();
                st.held.clear();
                st.note = None;
                st.set = SetStep::Typing { index };
            }
            SetPress::Rotate => {
                st.list();
                st.note = None;
                st.set = SetStep::Marking(form.indices.iter().map(|&j| (j, 0)).collect());
            }
            SetPress::Mark(j, m) => {
                if let SetStep::Marking(marks) = &mut st.set
                    && let Some(x) = marks.iter_mut().find(|(x, _)| *x == usize::from(j))
                {
                    x.1 = m.min(2);
                }
            }
            SetPress::Counted => {
                let SetStep::Marking(marks) = &st.set else {
                    return;
                };
                match rotation(&form, marks) {
                    Err(line) => st.note = Some(line),
                    Ok((held, line)) => {
                        let out = marks.len() - held.len();
                        st.note = Some(line);
                        st.set = if held.is_empty() {
                            SetStep::Scheme(form.k, u8::try_from(form.n()).unwrap_or(form.k))
                        } else {
                            SetStep::Destroying {
                                held: held.into_iter().map(|j| (j, false)).collect(),
                                out,
                            }
                        };
                    }
                }
            }
            SetPress::Destroyed(j) => st.destroyed(&form, usize::from(j)),
            SetPress::K(d) => st.step_scheme(true, d),
            SetPress::N(d) => st.step_scheme(false, d),
            SetPress::MakeNew => {
                let set = std::mem::take(&mut st.set);
                let held = st.held.clone();
                let typed = st.typed.clone();
                if let Err(line) = self.c32_make_new(c, (i, key), &form, &set, &held, &typed)
                    && let Some(st) = self.backup.c32_mut()
                {
                    st.set = set;
                    st.note = line;
                }
            }
        }
    }

    /// Costs an interpolation of `k` strings, or a split of the seed.
    /// **Make share E** (a share added or replaced from the strings typed
    /// in, `held`) or **Make the new set** (a rotation, its identifier
    /// `typed`), for seed `i` with session key `key` and its form `form`
    /// (`docs/CODEX32.md` §9.7). `Err`: what to say, the step kept.
    fn c32_make_new(
        &mut self,
        c: &Compute,
        (i, key): (usize, usize),
        form: &crate::plan::codex32::C32,
        set: &crate::codex32_backup::SetStep,
        held: &[zeroize::Zeroizing<String>],
        typed: &str,
    ) -> Result<(), Option<String>> {
        use crate::codex32_backup::SetStep;
        use crate::plan::codex32::index_of;
        match set {
            SetStep::Typing { index } => {
                let j = index.map(usize::from).ok_or(None)?;
                let parts: Vec<faraday_bip::codex32::Codex32> = held
                    .iter()
                    .filter_map(|s| faraday_bip::codex32::Codex32::parse(s).ok())
                    .collect();
                if parts.len() < form.need() {
                    return Err(Some(fill(
                        EN.c32_set_type_more,
                        &[&form.need().to_string()],
                    )));
                }
                let made = self
                    .session
                    .keys
                    .codex32_derive(
                        c,
                        key,
                        &parts,
                        plan::codex32::CHARSET[j],
                        faraday_keys::Reveal::PaperForm,
                    )
                    .map_err(|_| Some(EN.c32_set_not_this_seed.to_string()))?;
                let s = made.clone();
                self.c32_added(i, key, j, s);
                Ok(())
            }
            SetStep::Scheme(k, n) => {
                let id = <[u8; 4]>::try_from(typed.as_bytes())
                    .map_err(|_| Some(EN.c32_set_identifier.to_string()))?;
                if Some(id) == form.identifier {
                    return Err(Some(EN.c32_set_identifier_old.to_string()));
                }
                // A new set is a fresh secret: none in the demo.
                if !self.makes_secrets() {
                    return Err(None);
                }
                let Some(draw) = self.fresh(b"codex32 rotation") else {
                    return Err(Some(EN.paper_no_randomness.to_string()));
                };
                let made = self
                    .session
                    .keys
                    .codex32_new_set(c, key, (*k, *n), id, &draw, faraday_keys::Reveal::PaperForm)
                    .map_err(|e| match e {
                        faraday_keys::Paper::Refused(e) => Some(fill(EN.paper_codex32, &[&e])),
                        _ => None,
                    })?;
                let list: Vec<(usize, zeroize::Zeroizing<String>)> = made
                    .iter()
                    .filter_map(|s| Some((index_of(*s.as_bytes().get(8)?)?, s.clone())))
                    .collect();
                let others: Vec<(usize, usize)> = self
                    .backup
                    .state()
                    .map(|b| {
                        b.c32_checked
                            .iter()
                            .copied()
                            .filter(|&(x, _)| x != i)
                            .collect()
                    })
                    .unwrap_or_default();
                self.backup.c32_reset(i, others);
                self.c32_fix(c, i, key, (*k, id, form.length), list);
                Ok(())
            }
            _ => Err(None),
        }
    }

    /// String `s` at index `j`, derived for seed
    /// `i`'s set: kept with the item, the set's form and facts gaining
    /// the index (a lost one replaced keeps its own), the string opened to
    /// write and type back (`docs/CODEX32.md` §9.7).
    fn c32_added(&mut self, i: usize, key: usize, j: usize, s: zeroize::Zeroizing<String>) {
        use faraday_vault::codex32::State;
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(mut form) = b.answers.c32(i).cloned() else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
        let mut answers = b.answers.clone();
        if !form.indices.contains(&j) {
            form.indices.push(j);
        }
        answers.set_c32(&shape, i, Some(form));
        self.backup
            .change_answers(&shape, &|a, _| *a = answers.clone());
        if let Some(mut facts) = self.session.keys.info(key).and_then(|k| k.codex32.clone()) {
            let ch = plan::codex32::CHARSET[j];
            facts.strings.retain(|&(x, _)| x != ch);
            facts.strings.push((ch, State::Made));
            self.session.keys.set_codex32(key, Some(facts));
        }
        if let Some(st) = self.backup.c32_mut() {
            st.keep(vec![(j, s)]);
            st.set = crate::codex32_backup::SetStep::None;
            st.held.clear();
            st.open(j);
        }
        self.backup.c32_unchecked(i, j);
    }
}

impl Faraday {
    /// Costs what `backup_item_done_now` costs. An item that has just
    /// become done moves on to the next, as its Continue did
    /// (`docs/BACKUP-FLOW.md` §17.23): a copy matched, a passphrase compared,
    /// a vault saved into, files or sheets made, the descriptor shown,
    /// codex32 strings checked. The blank templates, the envelopes and the
    /// sticks stay a deliberate press. An item opened already done stays
    /// open. Called after every input.
    pub(crate) fn backup_move_on(&mut self, c: &Compute) {
        let Some(b) = self.backup.state() else {
            return;
        };
        if b.stage != crate::BStage::Checklist {
            return;
        }
        let Some(n) = b.open else {
            return self.backup.set_seen(None);
        };
        let Some(item) = crate::bstep::item(n) else {
            return;
        };
        let moves = !matches!(
            item,
            plan::Item::Templates | plan::Item::Envelopes | plan::Item::Sticks
        );
        let done = self.backup_item_done_now(c, item);
        let was = b.seen.filter(|&(m, _)| m == n).map(|(_, d)| d);
        // A codex32 string still open waits for its Continue.
        let typing = b.c32.as_ref().is_some_and(|st| st.at.is_some());
        // A codex32 string open counts as not done yet, so its Continue
        // is what moves the item on.
        let done = done && !typing;
        self.backup.set_seen(Some((n, done)));
        if moves && done && was == Some(false) {
            self.backup_act(Action::BNext(n), c);
            let open = self.backup.state().and_then(|b| b.open);
            let now = open
                .and_then(crate::bstep::item)
                .is_some_and(|it| self.backup_item_done_now(c, it));
            self.backup.set_seen(open.map(|m| (m, now)));
        }
    }

    /// Each seed of wallet `w`'s codex32 set, where its key is here and
    /// has one (`docs/CODEX32.md` §9.4): what the plan places. A lookup.
    pub(crate) fn c32_sets(&self, w: usize) -> Vec<Option<faraday_vault::codex32::SetFacts>> {
        self.backup_seed_list_now(w)
            .into_iter()
            .map(|(_, k)| self.session.keys.info(k?)?.codex32.clone())
            .collect()
    }
}
