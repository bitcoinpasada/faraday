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
use crate::vault::{Currency, VaultSummary, redact_place_name, wallet_name_in};
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
    /// Each seed's Krux code made in this wallet's backup, by fingerprint:
    /// the names of its files and whether it was opened back. Nothing of
    /// the passphrase or the seed is in it (`docs/KEF.md` §15.1). Empty in
    /// a record kept before it was written.
    pub kefs: Vec<([u8; 4], KefMark)>,
    /// Copies checked this power-on, dated, and held here until an unlock
    /// of a vault that keeps the same plan (and is the newest copy seen)
    /// writes them into its notes (`docs/BACKUP-FLOW.md` §18.3). RAM only:
    /// never written into the kept records, nor carried across a lock, so
    /// a lock or a power-off loses what was not written.
    pub dated: Vec<Dated>,
}

/// A copy checked, as a record holds it until the vault keeps it: the
/// place by number, the thing by kind, the date. No names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dated {
    /// The place, by number from zero.
    pub place: usize,
    /// The thing checked.
    pub what: plan::What,
    /// The date (`YYYY-MM-DD`), empty when the clock was not checked.
    pub date: String,
    /// The vault's plan notes hold this date.
    pub written: bool,
    /// The place's own line in the plan the check was made under
    /// ([`place_line`]: its name, where it is and what it holds, no mark
    /// of what is loaded now). Only a vault whose plan has, at this number,
    /// a place with the same line that keeps the thing takes the date.
    pub line: String,
    /// The copy a stick check read, where there is one: the stick's own
    /// vault takes the date only in that copy or one this device sealed
    /// from it, and a current vault of another id that keeps the plan takes
    /// it (`docs/BACKUP-FLOW.md` §18.3).
    pub copy: Option<CopyRead>,
}

/// A Krux code made in a wallet's backup, as its record keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KefMark {
    /// The picture's name in the Outbox, when the code fits one.
    pub png: Option<String>,
    /// The `.kef` file's name in the Outbox.
    pub file: String,
    /// The code was opened back with its passphrase and held the seed.
    pub checked: bool,
    /// The SHA-256 of the `.kef` file's bytes: a file of that name with
    /// other bytes is not this code.
    pub sha: [u8; 32],
    /// The picture was drawn at low error correction.
    pub low: bool,
    /// The picture carries a typed id's line.
    pub label: bool,
    /// The drawn copy on Paper was scanned and held the code.
    pub scanned: bool,
    /// The code's side in modules, 0 where the picture does not fit one
    /// code.
    pub side: u16,
    /// The code was also on Paper at a place when the record was kept.
    pub paper: bool,
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

/// Place `p`'s own line in a plan, as a dated check carries it: the name
/// it was given (`names[p]`, as the plan record keeps it) and the place's
/// spot line as [`plan::text::at_code`] and [`plan::text::what_code`]
/// write it, with no mark of what is loaded now. None where `places` (the
/// plan's map as places keep it) has no place `p` that keeps `what`.
pub(crate) fn place_line(
    places: &[plan::Spot],
    names: &[String],
    p: usize,
    what: What,
) -> Option<String> {
    let spot = places.iter().find(|s| s.at == At::Place(p))?;
    if !spot.holds.iter().any(|(w, _)| *w == what) {
        return None;
    }
    let mut line = format!(
        "{}\n{}",
        names.get(p).map_or("", |n| n.trim()),
        plan::text::at_code(spot.at)
    );
    for (w, _) in &spot.holds {
        line.push(' ');
        line.push_str(&plan::text::what_code(*w));
    }
    Some(line)
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
    faraday_crypto::hex::encode_string(&b)
}

/// A `kef` line's fields into the last record `out` holds; a field that
/// does not read drops the line.
fn read_kef(out: &mut [Record], f: [&str; 10]) {
    let [
        fp,
        png,
        file,
        checked,
        sha,
        low,
        label,
        scanned,
        side,
        paper,
    ] = f;
    if let (Some(fp), Some(sha), Some(r)) = (read_hex4(fp), read_hex32(sha), out.last_mut())
        && !file.is_empty()
        && let (Some(checked), Some(low), Some(label), Some(scanned), Ok(side), Some(paper)) = (
            read_flag(checked),
            read_flag(low),
            read_flag(label),
            read_flag(scanned),
            side.parse::<u16>(),
            read_flag(paper),
        )
    {
        r.kefs.push((
            fp,
            KefMark {
                png: (!png.is_empty()).then(|| png.to_string()),
                file: file.to_string(),
                checked,
                sha,
                low,
                label,
                scanned,
                side,
                paper,
            },
        ));
    }
}

/// A flag as the record writes it: `0` or `1`.
fn read_flag(s: &str) -> Option<bool> {
    match s {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn read_hex4(s: &str) -> Option<[u8; 4]> {
    faraday_crypto::hex::decode_exact(s)
}

pub(crate) fn hex32(b: &[u8; 32]) -> String {
    faraday_crypto::hex::encode_string(b)
}

pub(crate) fn read_hex32(s: &str) -> Option<[u8; 32]> {
    faraday_crypto::hex::decode_exact(s)
}

/// Whether locked vault summary `s` is the vault that does a plan's
/// vault: each of `own` (the seeds the plan puts there, told by
/// [`summary_has_seed`]) is held, each of `others` (seeds only another
/// vault is to hold) is not, and the wallet's checksum `sum` is listed
/// when `wallet_too`. A seed that cannot be told is not done.
pub(crate) fn summary_fits(
    s: &crate::vault::VaultSummary,
    sum: &str,
    wallet_too: bool,
    own: &[Option<bool>],
    others: &[Option<bool>],
) -> bool {
    own.iter().all(|h| *h == Some(true))
        && others.iter().all(|h| *h == Some(false))
        && (!wallet_too || s.sums.iter().any(|x| x == sum))
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
        for (fp, k) in &r.kefs {
            out.push_str(&format!(
                "kef\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                hex4(*fp),
                kept_field(k.png.as_deref().unwrap_or("")),
                kept_field(&k.file),
                u8::from(k.checked),
                hex32(&k.sha),
                u8::from(k.low),
                u8::from(k.label),
                u8::from(k.scanned),
                k.side,
                u8::from(k.paper)
            ));
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

/// A `kef` line's fields after its tag, of any length kept so far: eight
/// fields (kept before the drawn copy and the side were) read as not
/// scanned, side unknown; ten (kept before Paper was) as not on Paper;
/// any other count drops the line.
fn read_kef_line(out: &mut [Record], rest: &[&str]) {
    if matches!(rest.len(), 7 | 9 | 10) {
        let mut f = ["0"; 10];
        f[..rest.len()].copy_from_slice(rest);
        read_kef(out, f);
    }
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
            // A flag other than 0 or 1 drops the line too. A line without
            // the hash and the options (kept before they were) is dropped:
            // the code reads Not made.
            ["kef", ref rest @ ..] => read_kef_line(&mut out, rest),
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
    /// Vault sticks read from an older copy of their vault this power-on:
    /// the wallet's checksum, the place, the stick. RAM only, never kept
    /// (`docs/BACKUP-FLOW.md` §18.3).
    older: Vec<(String, usize, plan::What)>,
    /// Stick checks made with the clock not checked, of wallets with no
    /// record this power-on: dated at **Set the time**.
    undated: Vec<Held>,
}

/// The copy of a vault a stick check was made from: the vault's
/// identifier and the digest of the copy the stick held.
pub(crate) type CopyRead = ([u8; 32], [u8; 32]);

/// A stick check held for the clock: the wallet's checksum, the place, the
/// thing, the place's line, and the copy the stick held. Only that copy,
/// or one this device sealed from it, takes the date.
pub(crate) type Held = (String, usize, plan::What, String, CopyRead);

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
                kefs: Vec::new(),
                dated: Vec::new(),
            }),
        }
    }

    /// Seed `fp`'s Krux code in wallet `sum`'s record: kept, or (`None`)
    /// not made.
    pub(crate) fn kef_set(&mut self, sum: &str, fp: [u8; 4], mark: Option<KefMark>) {
        let Some(r) = self.record_mut(sum) else {
            return;
        };
        r.kefs.retain(|(f, _)| *f != fp);
        if let Some(m) = mark {
            r.kefs.push((fp, m));
        }
    }

    /// Seed `fp`'s Krux code as wallet `sum`'s record keeps it.
    pub(crate) fn kef_of(&self, sum: &str, fp: [u8; 4]) -> Option<&KefMark> {
        self.record(sum)?
            .kefs
            .iter()
            .find(|(f, _)| *f == fp)
            .map(|(_, m)| m)
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

    /// The vault stick `what` at place `p` of wallet `sum`'s plan was read
    /// from an older copy of its vault.
    pub(crate) fn older_mark(&mut self, sum: &str, p: usize, what: plan::What) {
        if !self.is_older(sum, p, what) {
            self.older.push((sum.to_string(), p, what));
        }
    }

    /// The mark of [`BackupsArea::older_mark`] is gone: the stick was read
    /// from a copy that is not older.
    pub(crate) fn older_clear(&mut self, sum: &str, p: usize, what: plan::What) {
        self.older
            .retain(|(s, q, w)| !(s == sum && *q == p && *w == what));
    }

    /// Whether a vault stick at place `p` of the wallet `sum` was read from
    /// an older copy of its vault: the chart's rule, a vault stick only.
    pub(crate) fn is_older_at(&self, sum: &str, p: usize) -> bool {
        self.older
            .iter()
            .any(|(s, q, w)| s == sum && *q == p && matches!(w, plan::What::VaultStick(_)))
    }

    /// Whether the stick was read from an older copy of its vault.
    pub(crate) fn is_older(&self, sum: &str, p: usize, what: plan::What) -> bool {
        self.older
            .iter()
            .any(|(s, q, w)| s == sum && *q == p && *w == what)
    }

    /// A copy checked and dated, held in wallet `sum`'s record: one entry
    /// per place and thing, the latest.
    pub(crate) fn dated_add(&mut self, sum: &str, d: Dated) {
        if let Some(r) = self.record_mut(sum) {
            r.dated
                .retain(|x| !(x.place == d.place && x.what == d.what));
            r.dated.push(d);
        }
    }

    /// The dated checks of wallet `sum`'s record at these places and
    /// things are in a vault's notes.
    pub(crate) fn dated_written(&mut self, sum: &str, done: &[(usize, plan::What)]) {
        if let Some(r) = self.record_mut(sum) {
            for d in &mut r.dated {
                if done.contains(&(d.place, d.what)) {
                    d.written = true;
                }
            }
        }
    }

    /// A stick check with no date, of a wallet with no record, made from
    /// `copy`: held until the clock is set.
    pub(crate) fn undated_hold(
        &mut self,
        sum: &str,
        p: usize,
        what: plan::What,
        line: &str,
        copy: CopyRead,
    ) {
        let held = (sum.to_string(), p, what, line.to_string(), copy);
        if !self.undated.contains(&held) {
            self.undated.push(held);
        }
    }

    /// The stick checks of [`BackupsArea::undated_hold`], taken.
    pub(crate) fn undated_take(&mut self) -> Vec<Held> {
        std::mem::take(&mut self.undated)
    }

    /// The dated checks with no date take `today`, to be written again;
    /// the checksums of the records that had one.
    pub(crate) fn date_undated(&mut self, today: &str) -> Vec<String> {
        let mut sums = Vec::new();
        for r in &mut self.records {
            let mut any = false;
            for d in r.dated.iter_mut().filter(|d| d.date.is_empty()) {
                d.date = today.to_string();
                d.written = false;
                any = true;
            }
            if any {
                sums.push(r.sum.clone());
            }
        }
        sums
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

    /// Whether the vault's notes or the checks held this power-on date
    /// anything at place `p`.
    fn dated_at(&self, notes: &[plan::Note], p: usize) -> bool {
        self.dated.iter().any(|d| d.place == p)
            || notes.iter().any(
                |n| matches!(n, plan::Note::Checked(At::Place(q) | At::Stick(q, _), ..) if *q == p),
            )
    }

    /// Each of the `things` at place `p` as the chart reads it: the date
    /// kept for it (empty: no date) from the vault's notes and the checks
    /// held this power-on, `None` for one never checked.
    fn place_checks(&self, notes: &[plan::Note], p: usize, things: usize) -> Vec<Option<String>> {
        let kept = notes.iter().filter_map(|n| match n {
            plan::Note::Checked(At::Place(q) | At::Stick(q, _), w, d) if *q == p => Some((*w, d)),
            _ => None,
        });
        let held = self
            .dated
            .iter()
            .filter(|d| d.place == p)
            .map(|d| (d.what, &d.date));
        let mut dates: Vec<(plan::What, String)> = Vec::new();
        for (w, d) in kept.chain(held) {
            match dates.iter_mut().find(|(x, _)| *x == w) {
                Some((_, old)) if old.is_empty() || (!d.is_empty() && d < old) => {
                    *old = d.clone();
                }
                Some(_) => {}
                None => dates.push((w, d.clone())),
            }
        }
        let n = dates.len();
        let mut out: Vec<Option<String>> = dates.into_iter().map(|(_, d)| Some(d)).collect();
        out.resize(things.max(n), None);
        out
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

/// A codex32 string the plan keeps: its seed and its share index.
type Pair = (usize, usize);

impl Faraday {
    /// Costs what `described_of` costs. The plan open vault `v` keeps for
    /// `wallet`, read as the record says (no Krux code dropped for a key
    /// loaded now): the shape `shape_for` makes from the record's own
    /// passphrase flags ([`plan::Answers::pass_seeds`]), the answers read
    /// under it and the places' names.
    pub(crate) fn plan_in_vault_stored(
        &self,
        c: &Compute,
        v: usize,
        wallet: &Wallet,
        shape_for: &dyn Fn(&[bool]) -> plan::Shape,
    ) -> Option<(plan::Shape, plan::Answers, Vec<String>)> {
        let want = crate::wallet::same_wallet(wallet.policy());
        let o = self.vaults.open.get(v)?;
        self.vaults
            .described_of(c, o.id, kind::PLAN)
            .find_map(|(_, r, d)| {
                if d.same != want {
                    return None;
                }
                let text = r.public_text(field::PLAN_ANSWERS)?;
                let shape = shape_for(&plan::Answers::pass_seeds(text));
                let a = plan::Answers::from_text(&shape, text)?;
                let names = r
                    .public_all(field::PLAN_PLACE)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .collect();
                Some((shape, a, names))
            })
    }

    /// Costs what `plan_in_vault` costs. The first plan the open vaults
    /// keep for `wallet` that parses, read as the record says, with the
    /// shape `shape_for` makes from that same record's passphrase flags.
    pub(crate) fn plan_load_stored(
        &self,
        c: &Compute,
        wallet: &Wallet,
        shape_for: &dyn Fn(&[bool]) -> plan::Shape,
    ) -> Option<(plan::Shape, plan::Answers, Vec<String>)> {
        (0..self.vaults.open.len()).find_map(|v| self.plan_in_vault_stored(c, v, wallet, shape_for))
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
                        // A codex32 string's file is a share of a seed: a
                        // secret, as a seed's words are.
                        What::Codex32ShareFile(..) | What::Codex32QrFile(..) => out
                            .secret
                            .extend(crate::codex32_backup::stick_names(&shape, a, *what)),
                        // A Krux code is an extra copy, not a copy of the
                        // seed, and its files are named when made
                        // (`docs/KEF.md` §15.1).
                        What::KefCode(i) => {
                            if let Some(m) = fp_of(i)
                                .and_then(|fp| self.backups_area.kef_of(wallet.checksum(), fp))
                            {
                                out.files.extend(m.png.clone());
                                out.files.push(m.file.clone());
                            }
                        }
                        What::KefCodePaper(_) => {}
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
    /// `form`: that form is checked, and the copy at its place is dated in
    /// the plan's notes (`docs/BACKUP-FLOW.md` §18.3): the place a chart's
    /// line named, the plan's one place for that form, or, where the plan
    /// keeps it at more than one, the one the match result asks for. Once
    /// the seed is checked in every form its copies ask for, one more of
    /// its copies is checked in the record.
    pub(crate) fn copy_matched(&mut self, fp: [u8; 4], form: crate::plan::CopyForm) {
        let c = self.compute();
        self.backup.note_checked(fp, form);
        self.copy_dating(&c, fp, form);
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

    /// Costs what `plan_mapped` costs. Dates the copy of seed `fp` that
    /// matched in `form`, or asks which copy it was.
    fn copy_dating(&mut self, c: &Compute, fp: [u8; 4], form: crate::plan::CopyForm) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let seed = self
            .backup_seed_list_now(b.wallet)
            .iter()
            .position(|(_, k)| {
                k.and_then(|k| self.session.keys.info(k))
                    .is_some_and(|k| k.fingerprint.0 == fp)
            });
        let (Some(i), at) = (seed, b.check_at) else {
            return;
        };
        let what = match form {
            plan::CopyForm::Words => What::Words(i),
            plan::CopyForm::SeedQr => What::SeedQr(i),
            plan::CopyForm::Passphrase => return,
        };
        self.backup.check_at_take();
        self.what_dating(c, what, at);
    }

    /// Costs what `plan_mapped` costs. Dates the copy of `what` that
    /// matched at `at`, or at the one place the plan keeps it, or asks
    /// which copy it was.
    fn what_dating(&mut self, c: &Compute, what: What, at: Option<At>) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let (wallet, answers) = (b.wallet, b.answers.clone());
        // A question still open was for an earlier match: that one is not
        // dated.
        self.backup.which_take();
        let shape = self.plan_shape_now(wallet);
        let places: Vec<usize> = self
            .kept
            .plan_mapped(c, &shape, &answers)
            .places
            .iter()
            .filter_map(|s| match s.at {
                At::Place(p) if s.holds.iter().any(|(w, _)| *w == what) => Some(p),
                _ => None,
            })
            .collect();
        // The place a chart named is dated where the plan keeps this
        // thing there; where it does not, the checklist's way.
        if let Some(At::Place(p)) = at
            && places.contains(&p)
        {
            self.dated_add(c, p, what);
            return;
        }
        match places[..] {
            [] => {}
            [p] => self.dated_add(c, p, what),
            _ => self.backup.ask_which(what, places),
        }
    }

    /// Costs what `plan_mapped` costs. Dates each codex32 string that
    /// became checked since `was` (a Check my copy that held).
    fn c32_dating(&mut self, c: &Compute, was: &[(usize, usize)]) {
        let new: Vec<(usize, usize)> = self
            .backup
            .state()
            .map(|b| {
                b.c32_checked
                    .iter()
                    .copied()
                    .filter(|x| !was.contains(x))
                    .collect()
            })
            .unwrap_or_default();
        for (i, j) in new {
            self.what_dating(c, What::Codex32Share(i, j), None);
        }
    }

    /// Costs what `described_of` costs. The policies of the loaded wallets
    /// and of the WALLET records of open vault `id`.
    fn wallet_policies_with(
        &self,
        c: &Compute,
        id: faraday_keys::vault::VaultId,
    ) -> Vec<faraday_bip::policy::WalletPolicy> {
        let mut policies: Vec<faraday_bip::policy::WalletPolicy> = self
            .session
            .wallets
            .iter()
            .map(|w| w.policy().clone())
            .collect();
        policies.extend(
            self.vaults
                .described_of(c, id, faraday_vault::records::kind::WALLET)
                .map(|(_, _, d)| d.policy.clone()),
        );
        policies
    }

    /// Costs what `plan_mapped` costs, for each wallet the open vaults
    /// keep a plan for. Open vault `o`, just unlocked from the file with
    /// hash `sha`: when that file was read from a stick on a stick visit,
    /// the plan's one vault it does (held as a vault holds its seeds, the
    /// wallet and no seed only another vault holds, as a locked vault's
    /// summary says) has its stick dated at the one place the plan keeps
    /// it (`docs/BACKUP-FLOW.md` §18.3). An unlock of an older copy dates
    /// nothing: the stick's line reads "Older copy", held in RAM; a copy
    /// sealed as many times with other bytes dates nothing and marks
    /// nothing. The wallets need not be loaded.
    pub(crate) fn vault_stick_dated(&mut self, o: usize, sha: Option<[u8; 32]>) {
        let from_stick = sha.is_some_and(|h| self.visit.vault_from_find(|x| x.1 == h).is_some());
        let Some((open_id, salt)) = self
            .vaults
            .open
            .get(o)
            .filter(|_| from_stick)
            .map(|v| (v.id, v.vault_id()))
        else {
            return;
        };
        let (older, diverged) = self.vaults.open.get(o).map_or((false, false), |v| {
            (v.is_older_copy(), v.is_not_current() && !v.is_older_copy())
        });
        if diverged {
            return;
        }
        let Some(summary) = self.vaults.summaries.iter().find(|s| s.id == salt).cloned() else {
            return;
        };
        let copy = self
            .vaults
            .open
            .get(o)
            .map(|v| (v.vault_id(), v.copy_digest()));
        let Some(copy) = copy else {
            return;
        };
        let c = self.compute();
        let policies = self.wallet_policies_with(&c, open_id);
        let mut seen: Vec<String> = Vec::new();
        for policy in policies {
            let sum = policy.checksum().to_string();
            if seen.contains(&sum) {
                continue;
            }
            seen.push(sum.clone());
            let wallet = crate::wallet::Wallet::new(
                String::new(),
                policy,
                String::new(),
                crate::wallet::Origin::Named,
            );
            let Some((shape, answers, _)) = self.plan_load_stored(&c, &wallet, &|pass| {
                let mut shape = self.plan_shape_of(&wallet);
                for (i, seed) in shape.seeds.iter_mut().enumerate() {
                    seed.here |= summary.keys.contains(&seed.name);
                    seed.passphrase = pass.get(i).copied().unwrap_or(false);
                }
                shape
            }) else {
                continue;
            };
            let slots = self.session.slots(&wallet);
            let fp_of = |i: usize| {
                let name = &shape.seeds.get(i)?.name;
                slots
                    .iter()
                    .find_map(|s| s.fingerprint.filter(|f| fp_text(*f) == *name))
                    .map(|f| f.0)
            };
            let record = self.backups_area.record(&sum);
            let has = |fp: [u8; 4]| summary_has_seed(&summary, fp, self.seed_id(record, fp));
            let made = answers.vaults_made(&shape);
            let fits = |pv: usize| {
                let own = answers.vault_seeds(&shape, pv);
                let wallet_too = answers.wallet_in_vault(pv);
                if own.is_empty() && !wallet_too {
                    return false;
                }
                let told = |i: usize| fp_of(i).and_then(has);
                let others: Vec<Option<bool>> = made
                    .iter()
                    .filter(|&&u| u != pv)
                    .flat_map(|&u| answers.vault_seeds(&shape, u))
                    .filter(|i| !own.contains(i))
                    .map(told)
                    .collect();
                let own: Vec<Option<bool>> = own.iter().map(|&i| told(i)).collect();
                summary_fits(&summary, &sum, wallet_too, &own, &others)
            };
            let [pv] = made
                .iter()
                .copied()
                .filter(|&pv| fits(pv))
                .collect::<Vec<_>>()[..]
            else {
                continue;
            };
            let places = self.stick_places(&c, &wallet, &answers, pv);
            if let [p] = places[..] {
                if older {
                    self.backups_area.older_mark(&sum, p, What::VaultStick(pv));
                } else if let Some(line) =
                    self.plan_place_line(&c, &wallet, p, What::VaultStick(pv))
                {
                    self.dated_add_to(&sum, p, What::VaultStick(pv), line, Some(copy));
                }
            }
        }
    }

    /// The shape a dated check's place line is built under, the same
    /// whatever keys are in memory: every seed the plan names counts as
    /// loaded (with a passphrase where the plan as stored gives the seed
    /// one, `pass`, not where the key has one) and so every vault it names counts as
    /// made, and no vault is set up before the keys. Both the check's side
    /// and the vault's read a place's line under it
    /// (`docs/BACKUP-FLOW.md` §18.3).
    pub(crate) fn plan_shape_made(
        &self,
        wallet: &crate::wallet::Wallet,
        pass: &[bool],
    ) -> plan::Shape {
        let mut shape = self.plan_shape_of(wallet);
        for (i, seed) in shape.seeds.iter_mut().enumerate() {
            seed.here = true;
            seed.passphrase = pass.get(i).copied().unwrap_or(false);
        }
        shape.vaults = 0;
        shape
    }

    /// Costs what `plan_mapped` costs. The places of the plan (`answers`)
    /// that keep vault `pv`'s stick, under the one shape a place line is
    /// read under ([`Faraday::plan_shape_made`]), not the one that names
    /// the vault the stick is of.
    fn stick_places(
        &self,
        c: &Compute,
        wallet: &crate::wallet::Wallet,
        answers: &plan::Answers,
        pv: usize,
    ) -> Vec<usize> {
        let shape = self.plan_shape_made(wallet, &answers.pass_flags());
        self.kept
            .plan_mapped(c, &shape, answers)
            .places
            .iter()
            .filter_map(|s| match s.at {
                At::Place(p) if s.holds.iter().any(|(x, _)| *x == What::VaultStick(pv)) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Costs what `plan_mapped` costs. Place `p`'s [`place_line`] in the
    /// plan the open vaults keep for `wallet`, read as the vault's own
    /// records are read, under [`Faraday::plan_shape_made`]: the line does
    /// not depend on which keys are loaded.
    fn plan_place_line(
        &self,
        c: &Compute,
        wallet: &crate::wallet::Wallet,
        p: usize,
        what: What,
    ) -> Option<String> {
        let (shape, answers, names) =
            self.plan_load_stored(c, wallet, &|pass| self.plan_shape_made(wallet, pass))?;
        let mapped = self.kept.plan_mapped(c, &shape, &answers);
        place_line(&mapped.places, &names, p, what)
    }

    /// **Which copy was this?**: the `n`th place offered is the one dated,
    /// where the backup's plan now still keeps the thing there.
    pub(crate) fn copy_which(&mut self, n: usize) {
        let Some((what, places)) = self.backup.which_take() else {
            return;
        };
        if let Some(&p) = places.get(n) {
            let c = self.compute();
            self.dated_add(&c, p, what);
        }
    }

    /// Costs what `plan_mapped` costs. The copy of `what` at place `p` of
    /// the backup under way checked today: held on the wallet's record
    /// and written into the plan's notes of an open vault that keeps the
    /// plan. With the clock not checked the date is empty, and **Set the
    /// time** dates it. Nothing where the backup's plan has no place `p`
    /// that keeps `what`.
    fn dated_add(&mut self, c: &Compute, p: usize, what: What) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(sum) = self
            .session
            .wallets
            .get(b.wallet)
            .map(|w| w.checksum().to_string())
        else {
            return;
        };
        let (wallet, answers, names) = (b.wallet, b.answers.clone(), b.names.clone());
        let Some(w) = self.session.wallets.get(wallet) else {
            return;
        };
        let shape = self.plan_shape_made(w, &answers.pass_flags());
        let mapped = self.kept.plan_mapped(c, &shape, &answers);
        let Some(line) = place_line(&mapped.places, &names, p, what) else {
            return;
        };
        self.dated_add_to(&sum, p, what, line, None);
    }

    /// Costs what `dated_into` costs. [`Faraday::dated_add`] on the record
    /// of the wallet with checksum `sum`, the check made at a place whose
    /// line (a [`place_line`]) is `line`. A stick read (`copy` is the copy
    /// the stick held) dates the stick's own vault only in that copy, or one
    /// this device sealed from it (`Vaults::copy_descends`), and a current
    /// vault of another id that keeps the plan takes it
    /// (`docs/BACKUP-FLOW.md` §18.3); it is held for the clock as such.
    fn dated_add_to(
        &mut self,
        sum: &str,
        p: usize,
        what: What,
        line: String,
        copy: Option<CopyRead>,
    ) {
        let sum = sum.to_string();
        if !self.backups_area.has(&sum) {
            self.backups_sync();
        }
        let date = self.today();
        // **Set the time** is offered on the backup of this wallet only.
        if date.is_empty()
            && self
                .backup
                .state()
                .and_then(|b| self.session.wallets.get(b.wallet))
                .is_some_and(|w| w.checksum() == sum)
        {
            self.backup.set_undated(true);
        }
        self.backups_area.older_clear(&sum, p, what);
        if !self.backups_area.has(&sum) {
            // No backup of this wallet was under way this power-on (its
            // vault was just unlocked and the wallet is not loaded): the
            // date goes straight into the notes of the open vaults that
            // keep its plan; with no date, it waits for the clock.
            let c = self.compute();
            // With no date the note is written empty ("Checked · no
            // date") and held, so Set the time dates it.
            if date.is_empty() {
                for v in 0..self.vaults.open.len() {
                    if self.copy_takes(v, copy) {
                        self.dated_into(
                            &c,
                            v,
                            &sum,
                            &[(At::Place(p), what, String::new(), line.clone())],
                        );
                    }
                }
                if let Some(copy) = copy {
                    self.backups_area.undated_hold(&sum, p, what, &line, copy);
                }
                return;
            }
            let takers: Vec<usize> = (0..self.vaults.open.len())
                .filter(|&v| self.copy_takes(v, copy))
                .collect();
            for v in takers {
                self.dated_into(
                    &c,
                    v,
                    &sum,
                    &[(At::Place(p), what, date.clone(), line.clone())],
                );
            }
            return;
        }
        self.backups_area.dated_add(
            &sum,
            Dated {
                place: p,
                what,
                date,
                written: false,
                line,
                copy,
            },
        );
        self.dated_write(&sum);
    }

    /// Costs what a wallet description's read costs, and `plan_mapped`.
    /// The places of the plan a vault's record keeps for wallet `sum`
    /// (answers `text`), read as the check's side reads its plan (the same
    /// parse, under [`Faraday::plan_shape_made`], with no codex32 set of a
    /// loaded key put into the answers and no Krux code dropped for a
    /// loaded key: the strings a place keeps are the
    /// ones the plan record says, as the check's side reads the backup's
    /// answers): as places keep the map. The wallet is the loaded one
    /// or the one any open vault describes; None where the text does not
    /// read or the wallet is not known.
    fn plan_places_as_read(&self, c: &Compute, sum: &str, text: &str) -> Option<Vec<plan::Spot>> {
        let loaded = self
            .session
            .wallets
            .iter()
            .position(|w| w.checksum() == sum);
        let built;
        let wallet = match loaded {
            Some(i) => &self.session.wallets[i],
            None => {
                let policy = self
                    .vaults
                    .open
                    .iter()
                    .flat_map(|o| {
                        self.vaults
                            .described_of(c, o.id, faraday_vault::records::kind::WALLET)
                    })
                    .map(|(_, _, d)| d.policy.clone())
                    .find(|p| p.checksum() == sum)?;
                built = crate::wallet::Wallet::new(
                    String::new(),
                    policy,
                    String::new(),
                    crate::wallet::Origin::Named,
                );
                &built
            }
        };
        let shape = self.plan_shape_made(wallet, &plan::Answers::pass_seeds(text));
        let a = plan::Answers::from_text(&shape, text)?;
        Some(self.kept.plan_mapped(c, &shape, &a).places.clone())
    }

    /// Costs what `dated_into` costs. Writes the dated checks of wallet
    /// `sum`'s record not yet in a vault into the plan's notes of each
    /// open vault that keeps the plan.
    fn dated_write(&mut self, sum: &str) {
        type Pending = (At, What, String, String, Option<CopyRead>);
        let pending: Vec<Pending> = self.backups_area.record(sum).map_or_else(Vec::new, |r| {
            r.dated
                .iter()
                .filter(|d| !d.written)
                .map(|d| {
                    (
                        At::Place(d.place),
                        d.what,
                        d.date.clone(),
                        d.line.clone(),
                        d.copy,
                    )
                })
                .collect()
        });
        if pending.is_empty() {
            return;
        }
        let c = self.compute();
        let mut done = vec![false; pending.len()];
        for v in 0..self.vaults.open.len() {
            let mine: Vec<usize> = (0..pending.len())
                .filter(|&n| self.copy_takes(v, pending[n].4))
                .collect();
            let one: Vec<(At, What, String, String)> = mine
                .iter()
                .map(|&n| {
                    let (a, w, d, l, _) = &pending[n];
                    (*a, *w, d.clone(), l.clone())
                })
                .collect();
            if one.is_empty() {
                continue;
            }
            for (&n, kept) in mine.iter().zip(self.dated_into(&c, v, sum, &one)) {
                done[n] |= kept;
            }
        }
        let done: Vec<(usize, What)> = pending
            .iter()
            .zip(done)
            .filter_map(|((at, w, ..), d)| match at {
                At::Place(p) if d => Some((*p, *w)),
                _ => None,
            })
            .collect();
        self.backups_area.dated_written(sum, &done);
    }

    /// Costs what `dated_write` costs, once per vault. Writes the unwritten
    /// dated checks of every wallet's record, at an unlock of a vault, and
    /// the stick checks held for the clock once it is checked.
    pub(crate) fn dated_unlock(&mut self) {
        let sums: Vec<String> = self
            .backups_area
            .records()
            .iter()
            .filter(|r| r.dated.iter().any(|d| !d.written))
            .map(|r| r.sum.clone())
            .collect();
        for sum in sums {
            self.dated_write(&sum);
        }
        let today = self.today();
        if !today.is_empty() {
            self.undated_written(&today);
        }
    }

    /// The clock was just checked: a dated check with no date takes
    /// today's, in the record and in the notes of an open vault.
    pub(crate) fn dated_clock_set(&mut self) {
        self.backup.set_undated(false);
        let today = self.today();
        if today.is_empty() {
            return;
        }
        let sums = self.backups_area.date_undated(&today);
        for sum in sums {
            self.dated_write(&sum);
        }
        self.undated_written(&today);
    }

    /// Costs what `dated_into` costs. The held stick checks take `today`
    /// in each open vault that takes them; one no vault took stays held.
    fn undated_written(&mut self, today: &str) {
        let held = self.backups_area.undated_take();
        if held.is_empty() {
            return;
        }
        let c = self.compute();
        for (sum, p, what, line, copy) in held {
            let mut taken = false;
            let takers: Vec<usize> = (0..self.vaults.open.len())
                .filter(|&v| self.copy_takes(v, Some(copy)))
                .collect();
            for v in takers {
                let one = [(At::Place(p), what, today.to_string(), line.clone())];
                taken |= self.dated_into(&c, v, &sum, &one).contains(&true);
            }
            if !taken {
                self.backups_area.undated_hold(&sum, p, what, &line, copy);
            }
        }
    }

    /// Whether open vault `v` may take a check made from `copy` (None: any
    /// vault may). Of a copy of the vault the stick held, only that copy
    /// or one this device sealed from it; any other vault takes it where
    /// its own plan has the place's line.
    fn copy_takes(&self, v: usize, copy: Option<CopyRead>) -> bool {
        copy.is_none_or(|(id, digest)| match self.vaults.open.get(v) {
            Some(o) if o.vault_id() == id => self.vaults.copy_descends(v, &id, &digest),
            Some(_) => true,
            None => false,
        })
    }

    /// Costs what a plan record's description costs. Sets the notes
    /// `checked <place> <thing> <date>` in the plan records of open vault
    /// `v` that are wallet `sum`'s, for each of `dated` (a place, a thing,
    /// a date, the plan it was checked under): true for each that was
    /// written. Nothing is written into a vault that is not the newest
    /// copy seen, nor a check made under another plan than the vault's.
    fn dated_into(
        &mut self,
        c: &Compute,
        v: usize,
        sum: &str,
        dated: &[(At, What, String, String)],
    ) -> Vec<bool> {
        use crate::plan::Note;
        let none = vec![false; dated.len()];
        let Some((id, current)) = self.vaults.open.get(v).map(|o| (o.id, !o.is_not_current()))
        else {
            return none;
        };
        if !current {
            return none;
        }
        let mut want = None;
        let mut ours = Vec::new();
        for (i, _, d) in self.vaults.described_of(c, id, kind::PLAN) {
            if d.checksum == sum {
                want.get_or_insert_with(|| d.same.clone());
                ours.push(i);
            }
        }
        let Some(want) = want else {
            return none;
        };
        // Each record's places, read as the vault's own records are read,
        // with the names it keeps.
        let kept: Vec<(Vec<plan::Spot>, Vec<String>)> = ours
            .iter()
            .filter_map(|i| self.vaults.records(id).get(*i))
            .filter_map(|r| {
                let places =
                    self.plan_places_as_read(c, sum, r.public_text(field::PLAN_ANSWERS)?)?;
                let names = r
                    .public_all(field::PLAN_PLACE)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .collect();
                Some((places, names))
            })
            .collect();
        let fits: Vec<bool> = dated
            .iter()
            .map(|(at, what, _, line)| {
                let At::Place(p) = at else {
                    return false;
                };
                kept.iter()
                    .any(|(pl, names)| place_line(pl, names, *p, *what).as_ref() == Some(line))
            })
            .collect();
        if !fits.contains(&true) {
            return none;
        }
        // Every record of the plan holds the same notes.
        let mut notes: Vec<Note> = self
            .vaults
            .described_of(c, id, kind::PLAN)
            .filter(|(_, _, d)| d.same == want)
            .flat_map(|(_, r, _)| {
                r.public_all(field::PLAN_NOTE)
                    .filter_map(|b| Note::from_text(&String::from_utf8_lossy(b), usize::MAX))
                    .collect::<Vec<_>>()
            })
            .collect();
        for ((at, what, date, _), _) in dated.iter().zip(&fits).filter(|(_, f)| **f) {
            notes.retain(|x| !matches!(x, Note::Checked(a, w, _) if a == at && w == what));
            notes.push(Note::Checked(*at, *what, date.clone()));
        }
        let texts: Vec<String> = notes.iter().map(Note::to_text).collect();
        let values: Vec<&[u8]> = texts.iter().map(|t| t.as_bytes()).collect();
        let c = self.compute();
        if self
            .vaults
            .edit(id, |s| {
                s.set_public_all_in(&c, id, &ours, field::PLAN_NOTE, &values)
            })
            .is_err()
        {
            return none;
        }
        self.vaults.changed(id);
        fits
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

    /// Seed `i`'s Krux code of the backup under way as the record keeps
    /// it: the names, the hash of its `.kef` file and whether it was
    /// checked. Whether the files are still there is
    /// `wallet_backup::kef`'s to say.
    pub(crate) fn kef_mark(&self, i: usize) -> Option<KefMark> {
        let b = self.backup.state()?;
        let sum = self.session.wallets.get(b.wallet)?.checksum();
        let fp = self.kef_fp(b.wallet, i)?;
        self.backups_area.kef_of(sum, fp).cloned()
    }

    /// The fingerprint of seed `i` of wallet `w`'s backup, when its key is
    /// loaded.
    fn kef_fp(&self, w: usize, i: usize) -> Option<[u8; 4]> {
        let k = self.backup_seed_list_now(w).get(i)?.1?;
        Some(self.session.keys.info(k)?.fingerprint.0)
    }

    /// Seed `i`'s Krux code of the backup under way is `mark` (`None`:
    /// not made): the record keeps it, for a backup opened again after a
    /// lock, and the plan's places list its files.
    pub(crate) fn backups_kef_set(&mut self, i: usize, mark: Option<KefMark>) {
        let Some(w) = self.backup.state().map(|b| b.wallet) else {
            return;
        };
        let (Some(sum), Some(fp)) = (
            self.session
                .wallets
                .get(w)
                .map(|x| x.checksum().to_string()),
            self.kef_fp(w, i),
        ) else {
            return;
        };
        if self.backups_area.kef_of(&sum, fp) == mark.as_ref() {
            return;
        }
        self.backups_area.kef_set(&sum, fp, mark);
        self.backups_sync();
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
                    crate::vault::wallet_name_in(r, field::WALLET_NAME, EN.backups_wallet)
                        .into_owned(),
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
                    Some(r) => Some(self.record_lines(c, r, &[], &[], Default::default())),
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

    /// Costs what `plan_load_of`, `plan_notes` (read for each wallet),
    /// `record_lines` and `backup_spots` cost. A wallet's map with what was seen of each place: from its record, else
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
        let iv = crate::glance::age::Interval::of(w.waits().shortest.map(|s| s.margin));
        // The dates the vault keeps for its copies.
        let notes = if self.vaults.open.is_empty() {
            Vec::new()
        } else {
            self.plan_notes(c, w, None)
        };
        // The places' names: the backup under way's, else the vault's.
        let names = match self.backup.state() {
            Some(b) if wi.is_some_and(|i| i == b.wallet) => b.names.clone(),
            _ => kept.as_ref().map(|(_, n)| n.clone()).unwrap_or_default(),
        };
        if let Some(r) = self.backups_area.record(sum) {
            return Some(self.record_lines(c, r, &names, &notes, iv));
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
            kefs: Vec::new(),
            dated: Vec::new(),
        };
        Some(self.record_lines(c, &r, &names, &notes, iv))
    }

    /// Costs what `vault_state` costs. A record's lines, each place named
    /// and given what was seen of it.
    fn record_lines(
        &self,
        c: &Compute,
        r: &Record,
        names: &[String],
        notes: &[plan::Note],
        iv: crate::glance::age::Interval,
    ) -> Vec<Line> {
        let today = self.today();
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
                    // A stick there read from an older copy, dated or not,
                    // with or without a paper copy there.
                    At::Place(p) if self.backups_area.is_older_at(&r.sum, p) => {
                        (EN.glance_older_copy.to_string(), Tone::Warn)
                    }
                    // Nothing dated there: what was checked this power-on.
                    At::Place(p) if !s.copies.is_empty() && !r.dated_at(notes, p) => {
                        if r.place_checked(i) {
                            (EN.backups_checked.to_string(), Tone::Ok)
                        } else {
                            (EN.backups_not_checked.to_string(), Tone::Warn)
                        }
                    }
                    At::Place(p) if !s.copies.is_empty() => {
                        let checks = r.place_checks(notes, p, s.holds.len());
                        let age = crate::glance::age::NodeAge::of(&checks, false, &today, iv);
                        match age {
                            // Nothing there was dated: what was checked
                            // here this power-on.
                            Some(a) if checks.iter().any(Option::is_some) => {
                                if a.never {
                                    (EN.backups_not_checked.to_string(), Tone::Warn)
                                } else if let Some(d) = &a.oldest {
                                    let when = a.text.clone().unwrap_or_else(|| d.clone());
                                    if a.due {
                                        (fill(EN.backups_checked_due, &[&when]), Tone::Warn)
                                    } else {
                                        (fill(EN.backups_checked_n, &[&when]), Tone::Ok)
                                    }
                                } else {
                                    if a.due {
                                        (EN.glance_to_check.to_string(), Tone::Warn)
                                    } else {
                                        (EN.backups_checked.to_string(), Tone::Ok)
                                    }
                                }
                            }
                            _ if r.place_checked(i) => (EN.backups_checked.to_string(), Tone::Ok),
                            _ => (EN.backups_not_checked.to_string(), Tone::Warn),
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
                .find(|f| f.open.is_none() && f.sha.is_some_and(|d| s.copies.contains(&d)))
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

    /// Costs what `backup_map_of` costs: the wallet card's backup line, for
    /// a decision ([`Faraday::backup_line_from`]).
    pub fn backup_line(&self, c: &Compute, i: usize) -> String {
        if self.session.wallets.get(i).is_none() {
            return String::new();
        }
        self.backup_line_from(self.backup_map_of(c, i))
    }

    /// Costs what `backup_lines` costs. Loaded wallet `i`'s map as its
    /// entry of [`Faraday::backups_now`] has it, worked out for that one
    /// wallet: the first loaded wallet with its descriptor checksum stands
    /// for it there, so it does here.
    pub(crate) fn backup_map_of(&self, c: &Compute, i: usize) -> Option<Vec<Line>> {
        let sum = self.session.wallets.get(i)?.checksum().to_string();
        let (first, w) = self
            .session
            .wallets
            .iter()
            .enumerate()
            .find(|(_, w)| w.checksum() == sum)?;
        self.backup_lines(c, w, Some(first), &sum)
    }

    /// Costs what `plan_load_of` costs. How many wallets are known and how
    /// many of them have no map, the two counts of Home's Backups tile: the
    /// length of [`Faraday::backups_now`] and its entries with no lines,
    /// worked out without the lines.
    pub fn backup_counts_now(&self, c: &Compute) -> (usize, usize) {
        let mut seen: Vec<String> = Vec::new();
        let mut without = 0;
        let mut note = |sum: &str, mapped: &dyn Fn() -> bool| {
            if seen.iter().any(|s| s == sum) {
                return;
            }
            seen.push(sum.to_string());
            if !mapped() {
                without += 1;
            }
        };
        for w in &self.session.wallets {
            let sum = w.checksum().to_string();
            note(&sum, &|| self.wallet_has_map(c, w, &sum));
        }
        for v in &self.vaults.open {
            for (_, r, d) in self.vaults.described_of(c, v.id, kind::WALLET) {
                note(&d.checksum, &|| {
                    let w = Wallet::new(
                        crate::vault::wallet_name_in(r, field::WALLET_NAME, EN.backups_wallet)
                            .into_owned(),
                        d.policy.clone(),
                        EN.backups_vault.to_string(),
                        crate::wallet::Origin::Vault,
                    );
                    self.wallet_has_map(c, &w, &d.checksum)
                });
            }
        }
        for s in &self.vaults.summaries {
            for sum in s
                .sums
                .iter()
                .take(s.wallets.len())
                .filter(|x| !x.is_empty())
            {
                note(sum, &|| {
                    self.backups_area.record(sum).is_some() || s.planned.contains(sum)
                });
            }
        }
        (seen.len(), without)
    }

    /// Costs what `plan_load_of` costs. Whether [`Faraday::backup_lines`]
    /// has a map for `w`: a record of its backup, or a plan an open vault
    /// keeps for it.
    fn wallet_has_map(&self, c: &Compute, w: &Wallet, sum: &str) -> bool {
        self.backups_area.record(sum).is_some()
            || self.plan_load_of(c, w, &self.plan_shape_of(w)).is_some()
    }

    /// The wallet card's backup line (`docs/history/SIMPLIFY.md` §5.2): "Backup:
    /// paper ×2 checked · vault.kdbx · for the stick", or "Not backed up",
    /// from the wallet's map ([`Faraday::backup_map_of`]). Cheap.
    pub(crate) fn backup_line_from(&self, map: Option<Vec<Line>>) -> String {
        let Some(lines) = map else {
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
    /// Costs what the press costs: a split of the seed (a new set), a QR encoding (a string's Codex32QR), an interpolation of the
    /// strings typed (the rebuild check, a share derived). A press on the
    /// checklist's codex32 item.
    pub(crate) fn backup_c32(&mut self, p: crate::codex32_backup::Press, c: &Compute) {
        use crate::codex32_backup::Press;
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
            Press::Open(j) => {
                if let Some(st) = self.backup.c32_mut() {
                    st.open(usize::from(j));
                }
                self.c32_show_drawn(c, i);
            }
            Press::Key(k) => {
                let was = self.backup.state().map(|b| b.c32_checked.clone());
                self.backup.c32_key(k);
                self.c32_dating(c, &was.unwrap_or_default());
                self.c32_keep_held(key);
                // A string of a set that exists, typed from its paper:
                // its Codex32QR to draw, where the plan draws one.
                self.c32_show_drawn(c, i);
            }
            Press::Qr => self.c32_qr(c),
            Press::BlockAt(b) => {
                if let Some(st) = self.backup.c32_mut() {
                    st.mark(b);
                }
            }
            Press::Rows(r) => {
                if let Some(st) = self.backup.c32_mut() {
                    st.mark_rows(r);
                }
            }
            Press::ScanCopy => self.c32_scan_open(),
            Press::Next => {
                let was = self.backup.state().map(|b| b.c32_checked.clone());
                self.c32_next(c, i, key);
                self.c32_dating(c, &was.unwrap_or_default());
                self.c32_show_drawn(c, i);
            }
            Press::Skip => {
                let Some(st) = self.backup.c32_mut() else {
                    return;
                };
                let Some(j) = st.at else {
                    return;
                };
                if st.skip_ask {
                    if let Some((_, _, checked)) = self.backup.c32_parts() {
                        crate::codex32_backup::mark_checked(checked, i, j);
                    }
                    self.c32_next(c, i, key);
                    self.c32_show_drawn(c, i);
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
            Press::Exist(d) => self.c32_exist(i, key, d),
        }
    }

    /// "How many strings of w0za are there?" (`docs/CODEX32.md` §9.4)
    /// stepped by `d` for seed `i` (session key `key`): the strings not
    /// known by index, kept on the plan's form and in the key's set
    /// facts. A lookup.
    fn c32_exist(&mut self, i: usize, key: usize, d: i8) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(mut form) = b.answers.c32(i).cloned() else {
            return;
        };
        // The count changes the checklist's answers: past a flow's lock
        // of a wallet holding the seed it is refused as a set's action is.
        if let Some(line) = self.c32_lock_refusal(key) {
            if let Some(st) = self.backup.c32_mut() {
                st.note = Some(line);
            }
            return;
        }
        let known = form.indices.len();
        let lo = known.max(usize::from(form.k)) as i16;
        let total =
            (form.n() as i16 + i16::from(d)).clamp(lo, i16::from(faraday_bip::codex32::MAX_SHARES));
        form.unknown = u8::try_from(total as usize - known).unwrap_or(0);
        let shape = self.plan_shape_now(b.wallet);
        let mut answers = b.answers.clone();
        answers.set_c32(&shape, i, Some(form.clone()));
        self.backup
            .change_answers(&shape, &|a, _| *a = answers.clone());
        if let Some(mut facts) = self.session.keys.info(key).and_then(|k| k.codex32.clone()) {
            facts.unknown = form.unknown;
            self.session.keys.set_codex32(key, Some(facts));
        }
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
        let wallet_i = b.wallet;
        let shape = self.plan_shape_now(b.wallet);
        let mut answers = b.answers.clone();
        let fixed = plan::codex32::C32 {
            seed: i,
            k,
            length,
            indices: list.iter().map(|(j, _)| *j).collect(),
            identifier: id,
            unknown: 0,
        };
        answers.rotate_c32(&shape, i, fixed.clone());
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
        // The plan record is saved with the set made, into the vaults that
        // keep the plan, so that the record and the answers agree and a
        // string typed back is dated (`docs/BACKUP-FLOW.md` §18.3).
        let kept = self
            .session
            .wallets
            .get(wallet_i)
            .is_some_and(|w| self.plan_vault_of(c, w).is_some());
        if kept {
            self.plan_save();
        }
        let texts: Vec<&str> = list.iter().map(|(_, s)| s.as_str()).collect();
        self.session.keys.keep_codex32(c, key, &texts);
        if let Some(st) = self.backup.c32_mut() {
            st.keep(list);
            if let Some(&j) = paper.first() {
                st.open(j);
            }
        }
    }

    /// Costs a QR encoding. **String** · **Codex32QR**: the open string,
    /// made here or typed from its paper, at 48 characters, as its
    /// Codex32QR (`faraday_codec::qr::codex32qr`, `docs/CODEX32.md`
    /// §10.2), or back to the string.
    fn c32_qr(&mut self, c: &Compute) {
        let Some(st) = self.backup.c32_mut() else {
            return;
        };
        if st.qr.take().is_some() {
            return;
        }
        let Some(s) = st.at.and_then(|j| st.known(j)) else {
            return;
        };
        st.qr = faraday_codec::qr::codex32qr(c, s);
        st.block = 0;
    }

    /// Costs a map of the plan, and a QR encoding where it shows one. The
    /// string just opened on seed `i`'s item, where the plan draws its
    /// Codex32QR by hand at a place and it was made here: opened on its
    /// Codex32QR, the grid to copy (`docs/CODEX32.md` §9.5).
    fn c32_show_drawn(&mut self, c: &Compute, i: usize) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(st) = b.c32.as_ref() else {
            return;
        };
        let Some(j) = st.at.filter(|&j| st.qr.is_none() && st.known(j).is_some()) else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
        let mapped = self.kept.plan_mapped(c, &shape, &b.answers);
        if plan::codex32::paper_copy(&mapped, i, j).is_some_and(plan::codex32::PaperCopy::drawn) {
            self.c32_qr(c);
        }
    }

    /// Costs what [`Faraday::c32_show_drawn`] costs, once a string holds.
    /// A key typed on the computer's keyboard on Back up, or on a wallet
    /// flow's plan card while a place is named (`docs/BACKUP-FLOW.md`
    /// §15.7): a place's name, a codex32 string typed back, or the copy
    /// grid's mark moved (§17.24), kept on the SeedQR shown. A codex32
    /// string of a set that exists, typed whole from its paper, then
    /// shows its Codex32QR to draw where the plan draws one. Whether the
    /// key was taken.
    pub(crate) fn backup_name_key(&mut self, key: faraday_shell_api::Key) -> bool {
        let naming_in_flow = self.screen == Screen::BackupFlow
            && self.backup.state().is_some_and(|b| b.naming.is_some());
        if !(self.screen == Screen::Backup || naming_in_flow) || !self.backup.name_key(key) {
            return false;
        }
        let pin = self.backup.state().and_then(|b| {
            let words = self.session.keys.info(b.key)?.word_count?;
            let n = crate::copy_check::seedqr_side(words, b.compact);
            Some(crate::copy_check::pin_on(b, n))
        });
        if let Some(p) = pin {
            self.backup.edit(Action::BPin(p));
        }
        if let Some(i) = self.backup.state().and_then(|b| Some(b.c32.as_ref()?.seed)) {
            let c = self.compute();
            self.c32_show_drawn(&c, i);
        }
        true
    }

    /// **Scan my copy** on the string open: the camera on its Codex32QR
    /// drawn by hand, read as that string typed back (`docs/CODEX32.md`
    /// §9.5). A lookup.
    fn c32_scan_open(&mut self) {
        if self
            .backup
            .state()
            .and_then(|b| b.c32.as_ref()?.at)
            .is_none()
        {
            return;
        }
        let mut scan = crate::ScanState::default();
        scan.purpose = crate::ScanPurpose::CheckCodex32;
        self.camera.open(scan);
        self.sheet = Some(crate::Sheet::Scan);
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOn);
    }

    /// What the camera read off the Codex32QR drawn of the string open:
    /// checked as that string typed back whole. Once it holds, the string
    /// counts as checked and towards the rebuild, and the camera closes;
    /// otherwise the scanner says what differs (`docs/CODEX32.md` §9.5).
    pub(crate) fn c32_copy_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).into_owned());
        drop(bytes);
        let key = self.backup.state().and_then(|b| {
            let i = b.c32.as_ref()?.seed;
            self.backup_seed_list_now(b.wallet).get(i).and_then(|x| x.1)
        });
        let was = self.backup.state().map(|b| b.c32_checked.clone());
        let found = self.backup.c32_scanned(&text);
        let c = self.compute();
        self.c32_dating(&c, &was.unwrap_or_default());
        if let Some(key) = key {
            self.c32_keep_held(key);
        }
        match found {
            Some(crate::codex32_backup::Found::Holds(_)) => {
                self.camera.close();
                self.sheet = None;
                self.out
                    .commands
                    .push_back(faraday_shell_api::Command::CameraOff);
            }
            Some(crate::codex32_backup::Found::Differs(line)) => {
                if let Some(scan) = self.camera.scan_mut() {
                    scan.note = Some(line);
                }
            }
            None => {}
        }
    }

    /// Costs an interpolation of the strings typed, once enough are.
    /// **Continue** on a string: it must be typed back the same, or
    /// skipped; then, for a set that exists, whether the strings typed so
    /// far rebuild this seed, once enough are; then the next string on
    /// paper not yet checked, or the list once every one is.
    fn c32_next(&mut self, c: &Compute, i: usize, key: usize) {
        // The strings typed on a keyboard, or by the item's keys, kept
        // with the key before the item moves on.
        self.c32_keep_held(key);
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
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(form) = b.answers.c32(i).cloned() else {
            return;
        };
        let past_lock = self.c32_lock_refusal(key);
        let Some(st) = self.backup.c32_mut() else {
            return;
        };
        // Past a flow's lock the set's actions are refused at their start,
        // before a string is typed or destroyed.
        if let Some(line) = past_lock
            && matches!(p, SetPress::Add | SetPress::Replace(_) | SetPress::Rotate)
        {
            st.note = Some(line);
            return;
        }
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
        // A backup past its lock keeps its plan until its writes are done
        // (`docs/BACKUP-FLOW.md` §15.8): a set made or a share added
        // changes it, so neither is made then.
        if let Some(line) = self.c32_lock_refusal(key) {
            return Err(Some(line));
        }
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
                let id = self.c32_rotation_refusal(i, key, form, typed)?;
                let Some(draw) =
                    self.fresh_bytes(b"codex32 rotation", faraday_keys::codex32_random_len(*k))
                else {
                    return Err(Some(EN.no_randomness.to_string()));
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

    /// What refuses a set's action past a flow's lock: a backup past its
    /// lock keeps its plan until its writes are done
    /// (`docs/BACKUP-FLOW.md` §15.8). The set is the key's, shared by
    /// every wallet that holds its seed, so a lock of any of them
    /// refuses it, not only the open wallet's.
    fn c32_lock_refusal(&self, key: usize) -> Option<String> {
        // By the flow's own seed names, so a wallet not loaded (the usual
        // case after a lock) still counts.
        let fp = fp_text(self.session.keys[key].fingerprint);
        let locked = self.backup_locked_flow().is_some()
            || self.flows.iter().any(|f| {
                f.made
                    && f.step != crate::backup_flow::Step::Done
                    && f.wallet.as_ref().is_some_and(|wf| {
                        wf.shape
                            .seeds
                            .iter()
                            .any(|s| s.name.eq_ignore_ascii_case(&fp))
                    })
            });
        locked.then(|| EN.backup_flow_past_lock.to_string())
    }

    /// The new set's identifier for seed `i`, or what refuses the rotation
    /// before it draws: `Err` says what to tell (`None`: nothing, as in
    /// the demo). Used by the press and by the wait for the seed, so a
    /// refused press never waits.
    fn c32_rotation_refusal(
        &self,
        i: usize,
        key: usize,
        form: &crate::plan::codex32::C32,
        typed: &str,
    ) -> Result<[u8; 4], Option<String>> {
        if let Some(line) = self.c32_lock_refusal(key) {
            return Err(Some(line));
        }
        let id = <[u8; 4]>::try_from(typed.as_bytes())
            .map_err(|_| Some(EN.c32_set_identifier.to_string()))?;
        let was = self
            .backup
            .state()
            .is_some_and(|b| b.answers.rotated_id(i, id));
        if id == form.identifier || was {
            return Err(Some(EN.c32_set_identifier_old.to_string()));
        }
        // A new set is a fresh secret: none in the demo.
        if !self.makes_secrets() {
            return Err(None);
        }
        Ok(id)
    }

    /// Whether Make the new set, pressed now, goes on to draw the system's
    /// randomness: the codex32 item is open on its Scheme step and the
    /// rotation is not refused.
    pub(crate) fn c32_make_new_draws(&self) -> bool {
        let Some(b) = self.backup.state() else {
            return false;
        };
        let Some(st) = b.c32.as_ref() else {
            return false;
        };
        let i = st.seed;
        let Some(key) = self.backup_seed_list_now(b.wallet).get(i).and_then(|x| x.1) else {
            return false;
        };
        let Some(form) = b.answers.c32(i) else {
            return false;
        };
        matches!(st.set, crate::codex32_backup::SetStep::Scheme(..))
            && self.c32_rotation_refusal(i, key, form, &st.typed).is_ok()
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
        let wallet_i = b.wallet;
        self.backup
            .change_answers(&shape, &|a, _| *a = answers.clone());
        // The plan record is saved with the share added, so that the record
        // and the answers agree and the string typed back is dated.
        let c = self.compute();
        let kept = self
            .session
            .wallets
            .get(wallet_i)
            .is_some_and(|w| self.plan_vault_of(&c, w).is_some());
        if kept {
            self.plan_save();
        }
        if let Some(mut facts) = self.session.keys.info(key).and_then(|k| k.codex32.clone()) {
            let ch = plan::codex32::CHARSET[j];
            facts.strings.retain(|&(x, _)| x != ch);
            facts.strings.push((ch, State::Made));
            self.session.keys.set_codex32(key, Some(facts));
        }
        let c = self.compute();
        self.session.keys.keep_codex32(&c, key, &[s.as_str()]);
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
        // A match that asks which copy it was waits for the answer; the
        // question goes when the item is left.
        let done = done && b.which.is_none();
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

    /// The strings of the codex32 item's set typed back, or typed in,
    /// that hold: kept with key `key` for this session, so that a file of
    /// one or a vault's share record can be written from it
    /// (`docs/CODEX32.md` §9.5). A parse of each.
    fn c32_keep_held(&mut self, key: usize) {
        let Some(st) = self.backup.state().and_then(|b| b.c32.as_ref()) else {
            return;
        };
        let texts: Vec<zeroize::Zeroizing<String>> = st.held.clone();
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let c = self.compute();
        self.session.keys.keep_codex32(&c, key, &refs);
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

// ---------------------------------------------------------------------
// Codex32 strings kept beyond paper (`docs/CODEX32.md` §9.5): a stick's
// files, a vault's share records, the strings gathered at unlock.
// ---------------------------------------------------------------------

impl Faraday {
    /// Costs a QR encoding and a picture for a Codex32QR. The bytes of
    /// codex32 string `j` of session key `k`'s set in form `what`, for its
    /// file on a stick (`docs/CODEX32.md` §9.5): the string, one line; or
    /// its Codex32QR (upper case, byte mode, version 3, level L, §10.2) as
    /// a labelled picture. `None` where this session does not hold the
    /// string.
    pub(crate) fn codex32_stick_bytes(
        &self,
        c: &Compute,
        k: usize,
        what: plan::What,
        j: usize,
    ) -> Option<zeroize::Zeroizing<Vec<u8>>> {
        use faraday_keys::{Part, Reveal};
        let index = plan::codex32::CHARSET[j];
        let s = self
            .session
            .keys
            .codex32_string(k, index, Reveal::BackupStick(Part::Codex32))?;
        match what {
            plan::What::Codex32ShareFile(..) => {
                let mut v = zeroize::Zeroizing::new(Vec::with_capacity(s.len() + 1));
                v.extend_from_slice(s.as_bytes());
                v.push(b'\n');
                Some(v)
            }
            plan::What::Codex32QrFile(..) => {
                let mx = faraday_codec::qr::codex32qr(c, &s)?;
                let id = s.get(4..8).unwrap_or("");
                let share = char::from(index).to_ascii_uppercase().to_string();
                let title = fill(EN.c32_picture_title, &[&share, id]);
                let lines = [EN.c32_picture_line.to_string()];
                Some(zeroize::Zeroizing::new(crate::picture::labelled_png(
                    c, &mx, &title, &lines,
                )))
            }
            _ => None,
        }
    }

    /// Costs what `plan_mapped` costs. The first codex32 string the plan
    /// keeps as a file on a stick that this session does not hold
    /// (`docs/CODEX32.md` §9.5): its share index and its set's identifier.
    /// **Lock and write** passes only once there is none. Computes from
    /// the state as it is.
    pub fn backup_stick_string_missing(&self, c: &Compute) -> Option<(char, String)> {
        let b = self.backup.state()?;
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        self.plan_mapped(c, &shape, &b.answers)
            .stick_strings(&b.answers)
            .into_iter()
            .find_map(|(i, j)| {
                let k = list.get(i)?.1?;
                let index = plan::codex32::CHARSET[j];
                let form = b.answers.c32(i)?;
                if self.session_holds_c32(k, (form.k, form.identifier), index) {
                    return None;
                }
                let id = form.id_text();
                Some((char::from(index).to_ascii_uppercase(), id))
            })
    }

    /// Costs a map of the plan, and a copy of each record staged. What the
    /// flow's round does to open vault `o`, its plan's vault `v`, so it
    /// holds the codex32 strings the plan `(shape, answers)` keeps
    /// there, of the plan's set (`docs/CODEX32.md` §9.5): into `add`, each
    /// such string the vault and `add` lack, as a type 15 record, from an
    /// open vault's record of it or the string this session holds; into
    /// `out`, the places in `o` of the share records of the plan's codex32
    /// seeds the plan keeps in another vault and not there. `Err`: the first string that is
    /// not here, to be typed in place (`Faraday::c32_round_type`).
    pub(crate) fn c32_round_records(
        &self,
        c: &Compute,
        (shape, answers): (&plan::Shape, &plan::Answers),
        v: usize,
        o: usize,
        (add, out): (&mut Vec<faraday_keys::vault::Staged>, &mut Vec<usize>),
    ) -> Result<(), plan::codex32::Wanted> {
        use faraday_keys::vault::{RecordView, Staged, codex32_share_is};
        let fp_of = |s: usize| {
            shape
                .seeds
                .get(s)
                .and_then(|x| read_hex4(&x.name.to_lowercase()))
        };
        let mapped = plan::Mapped::of(c, shape, answers);
        let strings = mapped.vault_strings(answers, v);
        let elsewhere = Self::c32_elsewhere(&mapped, shape, answers, v);
        out.extend(self.c32_strays(answers, o, &fp_of, (&strings, &elsewhere)));
        for (s, j) in strings {
            let (Some(f), Some(form)) = (fp_of(s), answers.c32(s)) else {
                continue;
            };
            let index = plan::codex32::CHARSET[j];
            let set = (form.k, form.identifier);
            let here = |r: RecordView<'_>| codex32_share_is(r, f, set, Some(index));
            if self
                .vaults
                .records(self.vaults.open[o].id)
                .iter()
                .any(|(_, r)| here(r))
                || add.iter().any(|a| here(a.view()))
            {
                continue;
            }
            let found = self
                .vaults
                .open
                .iter()
                .find_map(|open| {
                    let (k, _) = self
                        .vaults
                        .records(open.id)
                        .iter()
                        .find(|(_, r)| here(*r))?;
                    self.vaults.store().stage(open.id, k, &[])
                })
                .or_else(|| {
                    let k = self.session.keys.position_of(f)?;
                    self.session
                        .keys
                        .codex32_share_record(
                            k,
                            index,
                            faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Codex32),
                        )
                        .map(Staged::Held)
                        .filter(|r| here(r.view()))
                });
            match found {
                Some(r) => add.push(r),
                None => {
                    return Err(plan::codex32::Wanted {
                        key: f,
                        k: form.k,
                        identifier: form.identifier,
                        length: form.length,
                        index,
                    });
                }
            }
        }
        Ok(())
    }

    /// Costs what `flow_copy_key` costs. The round's **Copy the key**:
    /// the string it stopped on last is forgotten, and the copy runs
    /// again, naming the first string it lacks (`c32_round_wanted`).
    pub(crate) fn c32_round_copy(&mut self, i: usize) {
        self.flows.set_c32_wanted(i, None);
        self.flow_copy_key(i);
    }

    /// The round of flow `i` stopped on `want`, a codex32 string this
    /// session does not hold (`docs/CODEX32.md` §9.5): its Make card names
    /// it and offers typing it in place.
    pub(crate) fn c32_round_wanted(&mut self, i: usize, want: plan::codex32::Wanted) {
        let line = fill(EN.c32_round_not_here, &[&want.share(), &want.id_text()]);
        self.flows.set_c32_wanted(i, Some(want));
        self.toast(&line);
    }

    /// **Type share D from its copy**: Add a key opens on its codex32 form
    /// for the string flow `i`'s round stopped on, and goes back to the
    /// flow (`docs/CODEX32.md` §9.5).
    pub(crate) fn c32_round_type(&mut self, i: usize) {
        let Some(want) = self.flows[i].live.c32_wanted.clone() else {
            return;
        };
        self.act(Action::Entry(None));
        if self.screen != Screen::Entry {
            return;
        }
        let codex32 = crate::forms::Form::ALL
            .iter()
            .position(|f| *f == crate::forms::Form::Codex32)
            .unwrap_or(0);
        self.act(Action::EntryForm(codex32 as u8));
        let e = self.entry.state_mut();
        e.other_forms = false;
        e.back = Some(Screen::BackupFlow);
        e.c32_round = Some(want);
    }

    /// Costs a parse of the string and what `codex32_of_seed` costs, twice
    /// where it is not refused by name: once to tell the refusal and once
    /// in `keep_codex32`, which proves it again. The
    /// string typed on Add a key for the round (`EntryState::c32_round`):
    /// kept with its key where it is the share the plan keeps there, by
    /// its set (identifier, threshold, length), its index and, once `k`
    /// strings of the set are here to rebuild it, its seed's fingerprint;
    /// refused by name and dropped otherwise, nothing written. Once kept
    /// the round's **Copy the key** goes on.
    pub(crate) fn c32_round_take(&mut self) {
        let c = self.compute();
        let e = self.entry.state();
        let (Some(want), Some(s)) = (e.c32_round.clone(), e.parts.codex32.last()) else {
            return;
        };
        let (share, id) = (want.share(), want.id_text());
        let k = self.session.keys.position_of(want.key);
        let same_set = s.identifier() == want.identifier
            && s.threshold() == want.k
            && s.encode().as_str().len() == usize::from(want.length);
        let refusal = if !same_set {
            Some(fill(
                EN.c32_round_other_set,
                &[&crate::forms::codex32_what(s), &id],
            ))
        } else if s.share_index() != want.index {
            Some(fill(
                EN.c32_round_other_share,
                &[&crate::forms::codex32_what(s), &share, &id],
            ))
        } else {
            match k {
                None => Some(fill(
                    EN.c32_round_no_key,
                    &[&fp_text(faraday_bip::keys::Fingerprint(want.key))],
                )),
                Some(k) => (self
                    .vaults
                    .store()
                    .codex32_of_seed(&c, &self.session.keys, k, s)
                    == Some(false))
                .then(|| {
                    fill(
                        EN.c32_round_other_seed,
                        &[
                            &share,
                            &id,
                            &fp_text(faraday_bip::keys::Fingerprint(want.key)),
                        ],
                    )
                }),
            }
        };
        let kept = match (refusal.is_none(), k) {
            (true, Some(k)) => self
                .vaults
                .store()
                .keep_codex32(&c, &mut self.session.keys, k, s),
            _ => false,
        };
        if !kept {
            // Not refused by name, so not proved: fewer than `k` strings
            // are here to rebuild the seed, or the key's own facts of its
            // set refuse the index.
            let line = refusal.unwrap_or_else(|| fill(EN.c32_round_unproved, &[&share, &id]));
            let e = self.entry.state_mut();
            e.parts.codex32.clear();
            e.error = Some(line);
            return;
        }
        self.entry.reset();
        self.screen = Screen::BackupFlow;
        if let Some(i) = self.flows.on() {
            self.c32_round_copy(i);
        }
    }

    /// A lookup per record. The places in open vault `o` of the codex32
    /// share records of a seed with a codex32 form on the plan `answers`
    /// (its key's fingerprint by `fp_of`) that the save takes out
    /// (`docs/CODEX32.md` §9.5): a string of `elsewhere`, the (seed,
    /// index) the plan keeps in its other vaults, of the plan's set, that
    /// is not among `strings`, the ones it keeps in this vault; and any
    /// share of a set of the seed that the plan records as rotated away
    /// (`answers.rotated`), whatever the vault. A share the plan lists in
    /// no vault, of a set it never rotated away or at another index, is
    /// not the plan's own and stays.
    pub(crate) fn c32_strays(
        &self,
        answers: &plan::Answers,
        o: usize,
        fp_of: &dyn Fn(usize) -> Option<[u8; 4]>,
        sets: (&[Pair], &[Pair]),
    ) -> Vec<usize> {
        self.c32_strays_all(answers, o, fp_of, sets)
            .into_iter()
            .filter_map(|(k, takes)| takes.then_some(k))
            .collect()
    }

    /// A lookup per record. As [`Faraday::c32_strays`], each with whether
    /// the save takes it out: a share of a set rotated away always goes;
    /// one the plan keeps elsewhere goes only when this session holds the
    /// string or another open vault holds it, so that the string is not
    /// lost with the save (`docs/CODEX32.md` §9.5).
    pub(crate) fn c32_strays_all(
        &self,
        answers: &plan::Answers,
        o: usize,
        fp_of: &dyn Fn(usize) -> Option<[u8; 4]>,
        (strings, elsewhere): (&[Pair], &[Pair]),
    ) -> Vec<(usize, bool)> {
        use faraday_keys::vault::codex32_share_is;
        let Some(open) = self.vaults.open.get(o) else {
            return Vec::new();
        };
        self.vaults
            .records(open.id)
            .of(kind::CODEX32_SHARE)
            .filter_map(|(k, r)| {
                // A rotated set counts only where the plan has a made form of
                // that seed with another identifier: a line naming a seed
                // with no form is read, and takes nothing out.
                let old = answers.rotated.iter().any(|&(x, k, id)| {
                    answers
                        .c32(x)
                        .is_some_and(|f| !f.identifier.eq_ignore_ascii_case(&id))
                        && fp_of(x).is_some_and(|fp| codex32_share_is(r, fp, (k, id), None))
                });
                if old {
                    return Some((k, true));
                }
                elsewhere.iter().find_map(|&(x, j)| {
                    if strings.contains(&(x, j)) {
                        return None;
                    }
                    let (fp, f) = (fp_of(x)?, answers.c32(x)?);
                    let index = plan::codex32::CHARSET[j];
                    codex32_share_is(r, fp, (f.k, f.identifier), Some(index))
                        .then(|| (k, self.c32_placeable(fp, (f.k, f.identifier), index, o)))
                })
            })
            .collect()
    }

    /// A lookup. Whether this session holds the string at share index
    /// `index` of the set `set` (threshold and identifier) of session key
    /// `k`: the key's facts must name that set, as papers of another set
    /// loaded as the key hold strings that are not it.
    pub(crate) fn session_holds_c32(&self, k: usize, set: (u8, [u8; 4]), index: u8) -> bool {
        self.session
            .keys
            .info(k)
            .and_then(|i| i.codex32.as_ref())
            .is_some_and(|f| (f.threshold, f.identifier) == set)
            && self.session.keys.holds_codex32(k, index)
    }

    /// A lookup. Whether the codex32 string at share index `index` of the
    /// set `set` of the key with fingerprint `fp` is held by this session
    /// or by an open vault other than `o`.
    fn c32_placeable(&self, fp: [u8; 4], set: (u8, [u8; 4]), index: u8, o: usize) -> bool {
        self.session
            .keys
            .position_of(fp)
            .is_some_and(|k| self.session_holds_c32(k, set, index))
            || self.vaults.open.iter().enumerate().any(|(w, open)| {
                w != o && self.vaults.store().holds_codex32(open.id, fp, set, index)
            })
    }

    /// Costs what `vault_strings` costs. The codex32 strings of `mapped`
    /// that the plan keeps in its vaults other than `v`.
    pub(crate) fn c32_elsewhere(
        mapped: &plan::Mapped,
        shape: &plan::Shape,
        answers: &plan::Answers,
        v: usize,
    ) -> Vec<(usize, usize)> {
        (0..answers.vault_count(shape))
            .filter(|&w| w != v)
            .flat_map(|w| mapped.vault_strings(answers, w))
            .collect()
    }

    /// Costs a QR encoding and a picture for a Codex32QR. Codex32 string
    /// `what` of a seed of `list` (each seed's name and session key), made
    /// for its file on a stick under `names` at **Lock and write**, a
    /// secret let out unprotected (`docs/CODEX32.md` §9.5). Returns what a
    /// picture must read back as.
    pub(crate) fn c32_stick_file(
        &mut self,
        c: &Compute,
        list: &[(String, Option<usize>)],
        what: plan::What,
        names: &[String],
    ) -> Vec<(String, crate::backup_flow::ReadBack)> {
        let Some((i, j)) = plan::codex32::string_of(what) else {
            return Vec::new();
        };
        let (Some(k), Some(name)) = (list.get(i).and_then(|(_, k)| *k), names.first()) else {
            return Vec::new();
        };
        let Some(bytes) = self.codex32_stick_bytes(c, k, what, j) else {
            return Vec::new();
        };
        let mut expect = Vec::new();
        if matches!(what, plan::What::Codex32QrFile(..)) {
            let fp = crate::wallet::fp_text(self.session.keys[k].fingerprint);
            expect.push((
                name.clone(),
                crate::backup_flow::ReadBack::Codex32(fp, plan::codex32::CHARSET[j]),
            ));
        }
        self.outbox.retain(|it| it.name != *name);
        let item = crate::Item::new(c, name, bytes.to_vec());
        self.outbox_secret(crate::secrets::OutReason::StickBackup, item);
        expect
    }

    /// Costs what `plan_mapped` costs. The codex32 strings the plan keeps
    /// in vault `v`, each as a share record (`docs/CODEX32.md` §9.5): the
    /// session key of its seed, its share index (a bech32 character,
    /// lower case) and the seed by its place among the wallet's.
    pub(crate) fn backup_vault_strings(&self, c: &Compute, v: usize) -> Vec<(usize, u8, usize)> {
        let Some(b) = self
            .backup
            .state()
            .filter(|b| !b.answers.codex32.is_empty())
        else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        self.kept
            .plan_mapped(c, &shape, &b.answers)
            .vault_strings(&b.answers, v)
            .into_iter()
            .filter_map(|(i, j)| Some((list.get(i)?.1?, plan::codex32::CHARSET[j], i)))
            .collect()
    }

    /// Costs what `plan_mapped` costs. The places in open vault `o` of the
    /// codex32 share records Back up's plan keeps in another vault than
    /// its vault `v` and not in `v`, and those of a set the plan records
    /// as rotated away ([`Faraday::c32_strays`]).
    pub(crate) fn backup_vault_c32_strays(&self, c: &Compute, o: usize, v: usize) -> Vec<usize> {
        self.backup_vault_c32_strays_all(c, o, v)
            .into_iter()
            .filter_map(|(k, takes)| takes.then_some(k))
            .collect()
    }

    /// Costs what `backup_vault_c32_strays_all` costs. The share index
    /// (upper case) and the set's identifier of each codex32 share record
    /// in open vault `o` that the plan keeps in another vault and that a
    /// save leaves, as nothing else holds the string.
    pub(crate) fn backup_vault_c32_left(
        &self,
        c: &Compute,
        o: usize,
        v: usize,
    ) -> Vec<(char, String)> {
        let Some(open) = self.vaults.open.get(o) else {
            return Vec::new();
        };
        let records = self.vaults.records(open.id);
        self.backup_vault_c32_strays_all(c, o, v)
            .into_iter()
            .filter(|&(_, takes)| !takes)
            .filter_map(|(k, _)| {
                let (_, id, j) =
                    faraday_keys::vault::codex32_share_header(records.iter().nth(k)?.1)?;
                Some((
                    char::from(j).to_ascii_uppercase(),
                    String::from_utf8_lossy(&id).into_owned(),
                ))
            })
            .collect()
    }

    /// Costs what `backup_vault_c32_strays` costs. Each of its places with
    /// whether the save takes the record out ([`Faraday::c32_strays_all`]).
    pub(crate) fn backup_vault_c32_strays_all(
        &self,
        c: &Compute,
        o: usize,
        v: usize,
    ) -> Vec<(usize, bool)> {
        let Some(b) = self
            .backup
            .state()
            .filter(|b| !b.answers.codex32.is_empty())
        else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let mapped = self.kept.plan_mapped(c, &shape, &b.answers);
        let strings = mapped.vault_strings(&b.answers, v);
        let elsewhere = Self::c32_elsewhere(&mapped, &shape, &b.answers, v);
        let fp_of = |s: usize| {
            shape
                .seeds
                .get(s)
                .and_then(|x| read_hex4(&x.name.to_lowercase()))
        };
        self.c32_strays_all(&b.answers, o, &fp_of, (&strings, &elsewhere))
    }

    /// Whether open vault `o` keeps the codex32 share record at index
    /// `index` of session key `k`'s set, as the key's facts state it (its
    /// threshold and identifier): a string of another set of the key is
    /// not it. A lookup.
    pub(crate) fn vault_holds_string(&self, o: usize, k: usize, index: u8) -> bool {
        let Some(info) = self.session.keys.info(k) else {
            return false;
        };
        let Some(set) = info.codex32.as_ref().map(|f| (f.threshold, f.identifier)) else {
            return false;
        };
        let fp = info.fingerprint.0;
        self.vaults
            .open
            .get(o)
            .is_some_and(|open| self.vaults.store().holds_codex32(open.id, fp, set, index))
    }

    /// What is remembered of a vault file, when it was seen open since
    /// power-on.
    pub fn vault_summary(&self, f: &crate::vault::VaultFile) -> Option<&VaultSummary> {
        self.vaults
            .summaries
            .iter()
            .find(|s| f.sha.is_some_and(|d| s.copies.contains(&d)))
    }

    /// Writes each open vault's summary over the one kept for it: on
    /// unlock, after every change made on a vault screen, and as the
    /// session seals.
    pub(crate) fn vault_summaries_refresh(&mut self) {
        let c = self.compute();
        let seen = self.clock_now();
        let mut fresh = Vec::new();
        for v in &self.vaults.open {
            let wallets = self
                .vaults
                .records(v.id)
                .of(kind::WALLET)
                .map(|(i, r)| {
                    (
                        wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet).into_owned(),
                        self.vaults
                            .described(&c, v.id, i)
                            .map(|d| d.shape.clone())
                            .unwrap_or_default(),
                    )
                })
                .collect();
            let (keys, ids): (Vec<String>, Vec<Option<[u8; 32]>>) = self
                .vaults
                .records(v.id)
                .of(kind::KEY)
                .filter_map(|(_, r)| crate::vault::screen::key_identity(&c, self, r))
                .map(|(fp, id)| (fp, Some(id)))
                .unzip();
            let sum_of = |i: usize| {
                self.vaults
                    .described(&c, v.id, i)
                    .map(|d| d.checksum.clone())
            };
            let sums = self
                .vaults
                .records(v.id)
                .of(kind::WALLET)
                .map(|(i, _)| sum_of(i).unwrap_or_default())
                .collect();
            let planned = self
                .vaults
                .records(v.id)
                .of(kind::PLAN)
                .filter_map(|(i, _)| sum_of(i))
                .collect();
            let map = self
                .vaults
                .records(v.id)
                .of(kind::PLAN)
                .flat_map(|(_, r)| {
                    let names: Vec<String> = r
                        .public_all(field::PLAN_PLACE)
                        .map(|b| String::from_utf8_lossy(b).into_owned())
                        .collect();
                    r.public_all(field::PLAN_HOLDS)
                        .map(|b| redact_place_name(&String::from_utf8_lossy(b), &names))
                        .collect::<Vec<_>>()
                })
                .collect();
            fresh.push(VaultSummary {
                file: v.name.clone(),
                id: v.vault_id(),
                copies: Vec::new(),
                name: v.label(self.vaults.store()),
                wallets,
                sums,
                planned,
                keys,
                ids,
                entries: self.vaults.records(v.id).of(kind::ENTRY).count(),
                map,
                seen,
                seal: None,
            });
        }
        self.vaults.merge_summaries(fresh);
    }

    /// Costs what `vault_summaries_refresh` costs.
    /// Open vault `v`, just unlocked: what the app remembers of the vaults
    /// refreshed, and the slot's seal count against what was seen, with
    /// the other copies in Files read with the key just opened
    /// (`docs/VAULT.md` §11). An older copy, or one
    /// sealed as many times with other bytes, is held at the older-copy
    /// dialog, nothing of it loaded, and this says true: the unlock goes
    /// on only from the dialog's **Open** (`docs/VAULT.md` §11, owner 2026-10-29).
    pub(crate) fn vault_unlock_held(
        &mut self,
        v: usize,
        sha: Option<[u8; 32]>,
        for_import: bool,
    ) -> bool {
        let before = self.vaults.summary_before(v);
        self.vault_summaries_refresh();
        self.vaults.seals_unlocked(v);
        // The bytes opened are a copy of the vault seen.
        if let (Some(o), Some(sha)) = (self.vaults.open.get(v), sha) {
            self.vaults.copy_seen(&o.vault_id(), sha);
        }
        let held = self.vaults.older_hold(v, (for_import, self.sheet), before);
        self.vault_stick_dated(v, sha);
        if !held {
            return false;
        }
        self.sheet = Some(crate::Sheet::OlderCopy);
        true
    }

    /// Costs what `vault_unlock_go_on` costs, on **Open**.
    /// The older-copy dialog's actions: **Open** goes on with the unlock
    /// as it would have gone on with no warning (its Load sheet, its
    /// flow); **Keep locked** closes the copy, its key wiped, and shows
    /// the sheet it was unlocked on again.
    pub(crate) fn vault_older_act(&mut self, a: crate::vault::VaultAction) {
        let Some(h) = self.vaults.older_take() else {
            return;
        };
        self.sheet = h.sheet;
        if a != crate::vault::VaultAction::OlderOpen {
            self.vaults.older_close(h);
            crate::vault::copy::drop_wait(self);
            self.flow_unlock_given_up();
            return;
        }
        if self.vaults.open.iter().any(|o| o.id == h.id) {
            self.vault_unlock_go_on(h.for_import);
            self.vault_unlock_flow();
        }
    }

    /// A lookup. After every press and tick: the older-copy dialog put away while
    /// its vault is held (Escape, a press beside it, a sheet that came
    /// over it and closed) is **Keep locked**; the held vault locked
    /// another way ends the hold. True when it closed the vault or the dialog.
    pub(crate) fn vault_older_settle(&mut self) -> bool {
        let Some(id) = self.vaults.older.as_ref().map(|h| h.id) else {
            return false;
        };
        if !self.vaults.open.iter().any(|o| o.id == id) {
            // Locked another way: the hold ends, and its dialog with it.
            self.vaults.older_take();
            if self.sheet == Some(crate::Sheet::OlderCopy) {
                self.sheet = None;
            }
            return true;
        }
        if self.sheet.is_some() {
            return false;
        }
        self.vault_older_act(crate::vault::VaultAction::OlderBack);
        true
    }

    /// Costs what `gather_codex32` costs. The codex32 share records of
    /// every open vault gathered towards their keys (`docs/CODEX32.md`
    /// §9.5): a key held keeps its strings, and a key `k` strings of one
    /// set make across the open vaults loads, its fingerprint said. Only
    /// where keys may load.
    pub(crate) fn vault_gather_codex32(&mut self) {
        let c = self.compute();
        if !self.may_load_keys() {
            return;
        }
        let loaded = self
            .vaults
            .store()
            .gather_codex32(&c, &mut self.session.keys);
        if loaded.is_empty() {
            return;
        }
        let fps: Vec<String> = loaded.into_iter().map(crate::wallet::fp_text).collect();
        self.refresh_spend();
        self.toast(&fill(EN.c32_gathered, &[&fps.join(", ")]));
    }

    /// Costs what `parse_words` costs for a SeedQR. Whether a secret
    /// picture written as `meant` read back right (`docs/BACKUP-FLOW.md`
    /// §17.12 item 10): its `bytes` the Outbox's byte for byte, and its one
    /// code, while the key with fingerprint `fp` is loaded, the digits of
    /// its words (a SeedQR) or its codex32 string at share index `index`
    /// (a Codex32QR, `docs/CODEX32.md` §9.5).
    pub(crate) fn secret_picture_reads(
        &self,
        c: &Compute,
        meant: &str,
        (bytes, codes): (&[u8], &[Vec<u8>]),
        fp: &str,
        index: Option<u8>,
    ) -> bool {
        let made = self
            .outbox
            .iter()
            .find(|it| it.name == meant)
            .is_some_and(|it| it.bytes == bytes);
        let key = (0..self.session.keys.len())
            .find(|&k| crate::wallet::fp_text(self.session.keys[k].fingerprint) == fp);
        let reads = match (key, codes) {
            (_, []) => false,
            (Some(k), [one]) => match index {
                None => self.seedqr_reads_as(c, k, one),
                Some(j) => self.session.keys.codex32_reads_as(k, j, one),
            }
            .unwrap_or(true),
            (Some(_), _) => false,
            (None, _) => true,
        };
        made && reads
    }
}
