//! A wallet's backup, in two parts kept differently (`docs/WALLETS.md`
//! §5):
//!
//! - the seeds, copied by hand from the screen: words with their BIP-39
//!   indices, and the SeedQR drawn as a ruled grid. A copy of the words is
//!   checked by typing the words back, and a SeedQR by scanning the drawn
//!   grid, each compared by [`compare_words`] (`docs/history/NEW-WALLET.md`
//!   §14.7). Never a printer.
//!   After the copy by hand, a further copy: into the open vault as the
//!   key record Vaults saves, or as a file only through the secret sheet
//!   (`crate::secrets`), past a warning: the words or the SeedQR as a
//!   labelled PNG, never the BIP-39 passphrase. Paper stays first; a
//!   file is the least safe of the three.
//! - the wallet in public keys: the descriptor as a QR code and as files,
//!   a multisig config, and for a multisig the split plan.
//!
//! The sheets ([`sheet_wallet`], [`sheet_blank`]) hold nothing secret;
//! they go to the Outbox as PDFs ([`crate::pdf`]), printed wherever the
//! stick goes next.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::Compute;

use crate::wallet::{Kind, Session, Wallet, fp_text};
use crate::{
    Action, BStage, Command, Faraday, Later, PlanPress, ScanPurpose, ScanState, Screen, Sheet,
    board, bstep, copy_check, file_stem, flow, paper, plan, qrow, qstep, ui, vault,
};

/// The public files of wallet `w` that are under Public files' More
/// options (F64, `docs/INTEROP.md` §5 I4), as [`Faraday::public_out`]
/// numbers them, each with its file's name: Bitcoin Core before 29, the
/// Electrum wallet file, the Caravan config and the Wasabi file, where
/// the wallet has each.
pub(crate) fn public_more_files(w: &Wallet) -> Vec<(u8, String)> {
    let stem = file_stem(&w.name);
    let p = w.policy();
    [
        (
            9,
            faraday_bip::core_import::writes_pair(p),
            "bitcoin-core-before-29",
        ),
        (10, faraday_intake::electrum::writes(p), "electrum"),
        (11, faraday_intake::caravan::writes(p), "caravan"),
        (13, faraday_intake::single_key::wasabi_writes(p), "wasabi"),
    ]
    .into_iter()
    .filter(|(_, writes, _)| *writes)
    .map(|(what, _, file)| (what, format!("{stem}-{file}.json")))
    .collect()
}

/// Which keys each sheet of a split backup carries: sheet `i` carries `d`
/// keys starting at key `i`, wrapping, where `d` is `n` less the keys each
/// sheet leaves off (at most `m − 1`). The split plan.
pub fn split_plan(n: usize, m: usize, omit: usize) -> Vec<Vec<usize>> {
    let max_omit = m.saturating_sub(1).min(n.saturating_sub(1));
    let drop = omit.min(max_omit);
    let d = n - drop;
    (0..n)
        .map(|i| {
            let mut row: Vec<usize> = (0..d).map(|j| (i + j) % n).collect();
            row.sort_unstable();
            row
        })
        .collect()
}

/// What a split plan achieves, found by trying every set of sheets rather
/// than trusting the arithmetic. The split audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audit {
    /// Every `m` sheets together hold every key.
    pub every_quorum_rebuilds: bool,
    /// The fewest sheets that hold every key.
    pub smallest_rebuild: usize,
    /// Keys on each sheet.
    pub keys_per_sheet: usize,
    /// The fewest sheets any one key is on.
    pub copies_per_key: usize,
}

/// Audits a plan for an `m`-of-`n` wallet.
pub fn audit(plan: &[Vec<usize>], m: usize) -> Audit {
    let n = plan.len();
    let masks: Vec<u32> = plan
        .iter()
        .map(|row| row.iter().fold(0u32, |acc, k| acc | (1 << k)))
        .collect();
    let all = (1u32 << n) - 1;
    let mut every = true;
    let mut smallest = n;
    for mask in 1..=all {
        let size = mask.count_ones() as usize;
        let cover = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .fold(0u32, |acc, i| acc | masks[i]);
        let rebuilds = cover == all;
        if size == m && !rebuilds {
            every = false;
        }
        if rebuilds && size < smallest {
            smallest = size;
        }
    }
    Audit {
        every_quorum_rebuilds: every,
        smallest_rebuild: smallest,
        keys_per_sheet: plan.first().map(Vec::len).unwrap_or(0),
        copies_per_key: (0..n)
            .map(|k| plan.iter().filter(|r| r.contains(&k)).count())
            .min()
            .unwrap_or(0),
    }
}

/// The multisig config Coldcard writes and Sparrow imports as "Coldcard
/// Multisig"; SeedSigner recognises it by "multisig setup file" in its
/// first line. `only` keeps those key lines alone, for a split sheet.
pub fn multisig_config(wallet: &Wallet, only: Option<&[usize]>) -> Option<String> {
    let format = match Kind::of(wallet.policy()) {
        Kind::Multi(faraday_bip::policy::Wrapper::Sh) => "P2SH",
        Kind::Multi(faraday_bip::policy::Wrapper::ShWsh) => "P2SH-P2WSH",
        Kind::Multi(_) => "P2WSH",
        _ => return None,
    };
    Some(config(
        wallet,
        only,
        format,
        "# Coldcard Multisig setup file (created by Faraday)\n",
    ))
}

/// Whether a wallet's backup can be split between its signers: a
/// multisig, `wsh`, `sh` or Taproot's `sortedmulti_a`.
pub fn splits(wallet: &Wallet) -> bool {
    matches!(Kind::of(wallet.policy()), Kind::Multi(_) | Kind::TapMulti)
}

/// One sheet of a split backup, holding the keys `keep` names: the
/// multisig config's lines for a `wsh` or `sh` multisig; for a Taproot
/// multisig the same lines with `Format: P2TR`, which Faraday rebuilds
/// as `tr(NUMS, sortedmulti_a(…))` (`restore::merge`). Coldcard's own
/// config has no Taproot format, so a Taproot wallet has no whole config.
pub fn split_share(wallet: &Wallet, keep: &[usize]) -> Option<String> {
    match Kind::of(wallet.policy()) {
        Kind::Multi(_) => multisig_config(wallet, Some(keep)),
        Kind::TapMulti => Some(config(
            wallet,
            Some(keep),
            "P2TR",
            "# Faraday split backup sheet: Taproot multisig, tr(NUMS, sortedmulti_a)\n",
        )),
        _ => None,
    }
}

fn config(wallet: &Wallet, only: Option<&[usize]>, format: &str, head: &str) -> String {
    let (m, n) = Session::quorum(wallet);
    let mut out = String::from(head);
    if let Some(keep) = only {
        out.push_str(&format!(
            "# Partial: {} of {n} xpubs. Any {m} sheets together rebuild the wallet.\n",
            keep.len()
        ));
    }
    out.push_str(&format!(
        "#\nName: {}\nPolicy: {m} of {n}\nFormat: {format}\n",
        sanitize(&wallet.name)
    ));
    for (i, key) in wallet.policy().keys().iter().enumerate() {
        if only.is_some_and(|keep| !keep.contains(&i)) {
            continue;
        }
        let fp = key
            .fingerprint()
            .map(fp_text)
            .unwrap_or_else(|| "00000000".into());
        let path = key
            .path()
            .map(|p| {
                format!(
                    "m/{}",
                    p.to_string().trim_start_matches("m/").replace('h', "'")
                )
            })
            .unwrap_or_else(|| "m".into());
        out.push_str(&format!(
            "\nDerivation: {path}\n{}: {}\n",
            fp.to_uppercase(),
            key.xpub()
        ));
    }
    out
}

/// A name Coldcard accepts: up to twenty printable ASCII characters.
/// The wallet as the JSON file Specter Desktop writes and Sparrow imports
/// ("Specter Desktop" in its import list): a label, the block height to
/// scan from, and the descriptor. The descriptor and the label carry no
/// character JSON needs escaped: a descriptor is ASCII without quotes or
/// backslashes, and the label is sanitized.
pub fn wallet_json(name: &str, policy: &faraday_bip::policy::WalletPolicy) -> String {
    let label: String = sanitize(name)
        .chars()
        .filter(|c| *c != '"' && *c != '\\')
        .collect();
    format!(
        "{{\n  \"label\": \"{label}\",\n  \"blockheight\": 0,\n  \"descriptor\": \"{}\"\n}}\n",
        policy.to_descriptor_checksummed()
    )
}

fn sanitize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(20)
        .collect()
}

/// One split sheet as a sheet file to print: which sheet it is, the
/// quorum, and the share's lines, which its QR code holds too.
pub fn sheet_share(wallet: &Wallet, part: usize, of: usize, share: &str) -> String {
    let (m, n) = Session::quorum(wallet);
    let mut out = String::from("faraday-sheet 1\nkind: share\n");
    out.push_str(&format!("name: {}\n", wallet.name));
    out.push_str(&format!("shape: {}\n", Session::shape(wallet)));
    out.push_str(&format!("part: {part} of {of}\n"));
    out.push_str(&format!("quorum: {m} of {n}\n"));
    for line in share.lines() {
        out.push_str(&format!("line: {line}\n"));
    }
    out
}

/// Costs an address derivation (through `address`). The sheet file for
/// a wallet's backup page: everything on it is public. The desktop app
/// turns it into a PDF; the format is lines of `field: value` after a
/// first line naming it.
pub fn sheet_wallet(c: &Compute, session: &Session, wallet: &Wallet) -> String {
    let mut out = String::from("faraday-sheet 1\nkind: wallet\n");
    out.push_str(&format!("network: {}\n", session.network().name()));
    out.push_str(&format!("name: {}\n", wallet.name));
    out.push_str(&format!("shape: {}\n", Session::shape(wallet)));
    out.push_str(&format!(
        "descriptor: {}\n",
        wallet.policy().to_descriptor_checksummed()
    ));
    for (i, slot) in session.slots(wallet).iter().enumerate() {
        let key = &wallet.policy().keys()[i];
        out.push_str(&format!(
            "key: {} {} {}\n",
            i + 1,
            slot.fingerprint
                .map(fp_text)
                .unwrap_or_else(|| "--------".into()),
            key.key_text()
        ));
    }
    for (label, change, index) in [
        ("0/0", false, 0),
        ("0/1", false, 1),
        ("0/2", false, 2),
        ("1/0", true, 0),
    ] {
        out.push_str(&format!(
            "address: {label} {}\n",
            session.address(c, wallet, change, index)
        ));
    }
    out
}

/// What the blank template has to copy a seed onto: the word lines, the
/// SeedQR grid, or both (`docs/history/NEW-WALLET.md` §14.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankCopy {
    /// The word lines.
    Words,
    /// The SeedQR grid.
    SeedQr,
    /// Both on one sheet.
    Both,
}

impl BlankCopy {
    /// Every choice, in the order a Choice lists them.
    pub const ALL: [BlankCopy; 3] = [BlankCopy::Words, BlankCopy::SeedQr, BlankCopy::Both];

    /// The sheet file's `copy:` value.
    pub fn line(self) -> String {
        match self {
            BlankCopy::Words => "words".into(),
            BlankCopy::SeedQr => "seedqr".into(),
            BlankCopy::Both => [BlankCopy::Words.line(), BlankCopy::SeedQr.line()].join(" "),
        }
    }
}

/// The blank template: numbered word lines, the SeedQR grid's fixed
/// squares, or both as `copy` says, for a seed of `words` words, and a
/// page of `places` empty lines, "Place ____ holds ____", to fill in by
/// hand. It holds no secret and nothing of the plan, and may go to any
/// printer. `wallet` adds the path, script and network to fill in.
pub fn sheet_blank(
    words: usize,
    copy: BlankCopy,
    wallet: Option<&Wallet>,
    network: Network,
    places: usize,
) -> String {
    let mut out = format!(
        "faraday-sheet 1\nkind: blank\nwords: {words}\ncopy: {}\nnetwork: {}\nplaces: {places}\n",
        copy.line(),
        network.name()
    );
    if let Some(w) = wallet {
        out.push_str(&format!("name: {}\nshape: {}\n", w.name, Session::shape(w)));
    }
    out
}

/// Whether module (x, y) of a SeedQR of side `n` is fixed by the QR
/// standard rather than by the seed: finders and their separators, timing,
/// the alignment square, the format areas and the dark module. These are
/// the same for every seed of the same length, so the blank template can
/// carry them and the copy-by-hand grid draws them grey.
pub fn structural(x: usize, y: usize, n: usize) -> bool {
    let finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    let timing = x == 6 || y == 6;
    let format = (y == 8 && (x < 9 || x >= n - 8)) || (x == 8 && (y < 9 || y >= n - 8));
    let version = (n - 17) / 4;
    let align = version >= 2 && {
        let c = n - 7;
        x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2
    };
    finder || timing || format || align
}

/// The squares fixed for every SeedQR of side `n`: finders and their
/// separators, timing and the alignment square. The format areas are left
/// out: they depend on the mask the seed's code was given.
pub fn structural_fixed(x: usize, y: usize, n: usize) -> bool {
    let finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    let format = (y == 8 && (x < 9 || x >= n - 8)) || (x == 8 && (y < 9 || y >= n - 8));
    let version = (n - 17) / 4;
    let align = version >= 2 && {
        let c = n - 7;
        x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2
    };
    (finder && !format) || ((x == 6 || y == 6) && !format) || align
}

/// What comparing a copy of a seed with the seed says
/// ([`faraday_keys::compare_words`]).
pub use faraday_keys::{CopyCheck, compare_words};

/// How a line of the backup's per-seed lines are coloured: kept,
/// wanting, out in the clear, or not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Kept: the theme's OK.
    Ok,
    /// Not kept yet: WARN.
    Warn,
    /// A copy in the clear: ERR.
    Err,
    /// Not known here: DIM.
    Dim,
}

/// One of the wallet's keys on the backup's panel: its fingerprint (or,
/// for a share not here, its number) and where it is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptSeed {
    /// The key, by fingerprint.
    pub name: String,
    /// Where it is kept, a line each.
    pub lines: Vec<(String, Tone)>,
}

/// Where the wallet and each of its seeds are kept, as the backup's
/// panel lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// The wallet: in a vault or not.
    pub wallet: (String, Tone),
    /// Every key of the wallet, in its order.
    pub seeds: Vec<KeptSeed>,
}

/// What the camera found of a copy, as the copy item says it.
pub fn copy_scan_line(found: &CopyCheck) -> String {
    match found {
        CopyCheck::WrongWord(k) => fill(EN.backup_your_copy_scans_but, &[&k.to_string()]),
        _ => EN.backup_your_copy_scans.to_string(),
    }
}

/// The envelope: which sheet goes with which seed, one line per key.
pub fn envelope(session: &Session, wallet: &Wallet, plan: Option<&[Vec<usize>]>) -> Vec<String> {
    let slots = session.slots(wallet);
    slots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let fp = s
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| "--------".into());
            let sheet = match plan {
                Some(p) => fill(
                    EN.backup_envelope_share_count,
                    &[&(i + 1).to_string(), &p.len().to_string()],
                ),
                None => EN.backup_the_backup_sheet.to_string(),
            };
            fill(
                EN.backup_envelope_line,
                &[
                    &(i + 1).to_string(),
                    &(i + 1).to_string(),
                    &fp.to_string(),
                    &sheet.to_string(),
                ],
            )
        })
        .collect()
}

/// The fingerprint a key line on a sheet carries.
pub fn fp_of(fp: Option<Fingerprint>) -> String {
    fp.map(fp_text).unwrap_or_else(|| "--------".into())
}

/// The backup flow's state.
#[derive(Default)]
pub struct BackupState {
    /// The wallet being backed up.
    pub wallet: usize,
    /// The plan, or the checklist made from it.
    pub stage: BStage,
    /// The open question of the plan, by [`qstep`].
    pub q: Option<u8>,
    /// The plan's answers.
    pub answers: plan::Answers,
    /// What each place is called, kept only in a vault; empty for
    /// "Place 1", "Place 2".
    pub names: Vec<String>,
    /// The place whose name is being typed.
    pub naming: Option<usize>,
    /// The open item of the checklist, by [`bstep`].
    pub open: Option<u8>,
    /// The public files item shows its files under More options (F64):
    /// pressed, or one of them already For the stick when it opened.
    pub public_more: bool,
    /// The envelopes closed as done: the one item nothing else marks.
    pub envelopes: bool,
    /// The blank templates were printed from Faraday Starter: the item is
    /// done by its press.
    pub printed: bool,
    /// The descriptor was shown as a code in this backup.
    pub shown: bool,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The session key whose seed is shown, by index into the session.
    pub key: usize,
    /// The words are shown.
    pub reveal: bool,
    /// Compact SeedQR rather than standard.
    pub compact: bool,
    /// The grid row pinned for copying.
    pub pin: usize,
    /// The typed check of a copy of the words, while it is open
    /// (`docs/history/NEW-WALLET.md` §14.7 item 2).
    pub check: Option<copy_check::WordCheck>,
    /// What scanning the copy of the seed on screen found.
    pub scanned: Option<CopyCheck>,
    /// The seeds, by fingerprint, whose copy by hand matched in this
    /// backup, with the form that matched: the words typed back, or the
    /// drawn SeedQR scanned.
    pub checked: Vec<([u8; 4], plan::CopyForm)>,
    /// The scans of each seed's SeedQR copy without a match in this
    /// backup, by fingerprint: each opening of the camera on the copy
    /// that closed without one (`docs/history/NEW-WALLET.md` §14.7 item 3). Goes
    /// with a lock, which drops the backup.
    pub misses: Vec<([u8; 4], u8)>,
    /// Template word count.
    pub words: usize,
    /// What went to the Outbox, for the summary.
    pub sent: Vec<String>,
    /// Another paper form of the key, on screen.
    pub paper: Option<paper::PaperForm>,
    /// The list of loaded wallets is open under the wallet chip.
    pub pick: bool,
    /// Open from Create's Back up card: the chip says the way on is
    /// the wallet card.
    pub from_create: bool,
    /// Open from a chart in an open vault's view: the way back is the
    /// vault.
    pub from_vault: bool,
    /// The board's chip held, by a tap or a drag
    /// (`docs/BACKUP-FLOW.md` §17.2).
    pub board: board::BoardState,
    /// Work done that a later plan dropped, as what it is ("vault.ofv
    /// holds seed 9A6A2580"): the copy exists until it is destroyed.
    pub extras: Vec<String>,
    /// The vaults the checklist had filled when Change the plan was
    /// pressed, by name, with the seeds each holds.
    pub prior: Vec<(String, Vec<String>)>,
    /// What each vault held, by its salt, before this backup first saved
    /// into it ([`Faraday::vault_tokens`]): the flow's round never takes
    /// it out (`docs/BACKUP-FLOW.md` §15). Never a secret.
    pub before: Vec<([u8; 32], Vec<String>)>,
    /// The seeds, by fingerprint as text, whose files on a stick the
    /// person has let out unprotected past the secret sheet's warning:
    /// **Lock and write** makes a seed's file only once it is here
    /// (`docs/BACKUP-FLOW.md` §17.12 item 6). Never a secret.
    pub files_warned: Vec<String>,
    /// The plan's vaults, by number, whose record of the wallet is set
    /// not to load at unlock: the vault item's **Load at unlock** turned
    /// off (F62). Every other starts on.
    pub no_load: Vec<usize>,
}

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the wallet backup flow's actions (`Backup*`, `BQ*`, `B*`).
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn backup_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::BackupChecklist(i) => {
                // The plan the chart shows, at its first item not done;
                // nothing is made For the stick by opening it.
                if self.chart_checklist(i, false) {
                    let first = self.backup_items_now(c).into_iter().find(|&n| {
                        bstep::item(n).is_some_and(|it| !self.backup_item_done_now(c, it))
                    });
                    self.backup_item_open(first);
                    self.screen = Screen::Backup;
                }
            }
            Action::Backup(i) => {
                if i < self.session.wallets.len() {
                    let keys = self.backup_keys(i);
                    let first_key = keys.first().copied().unwrap_or(0);
                    let shape = self.plan_shape_now(i);
                    // The plan this wallet's last backup saved into an
                    // open vault, as it was; else the board open on The
                    // standard way, read-only until Customize backup
                    // (`docs/BACKUP-FLOW.md` §17.6).
                    let (answers, names, q) = match self.plan_load(c, i, &shape) {
                        Some((a, names)) => (a, names, None),
                        None => (
                            plan::Answers::preset(&shape, plan::Preset::Standard),
                            Vec::new(),
                            Some(qstep::BOARD),
                        ),
                    };
                    // A copy checked this power-on stays checked.
                    let marks = self.backups_marks(i);
                    let checked = self
                        .backups_checked(i)
                        .into_iter()
                        .flat_map(|fp| [(fp, plan::CopyForm::Words), (fp, plan::CopyForm::SeedQr)])
                        .chain(
                            marks
                                .passphrases
                                .iter()
                                .map(|&fp| (fp, plan::CopyForm::Passphrase)),
                        )
                        .collect();
                    // The template offered first is the one the seeds to
                    // copy fit: 12 words when none has more, else 24.
                    let counts: Vec<Option<u8>> = self
                        .backup_keys(i)
                        .into_iter()
                        .map(|k| self.session.keys[k].word_count)
                        .collect();
                    let words = if !counts.is_empty()
                        && counts.iter().all(|n| n.is_some_and(|n| n <= 12))
                    {
                        12
                    } else {
                        24
                    };
                    self.backup = Some(BackupState {
                        wallet: i,
                        printed: marks.printed,
                        envelopes: marks.envelopes,
                        shown: marks.shown,
                        q,
                        answers,
                        names,
                        checked,
                        key: first_key,
                        words,
                        ..BackupState::default()
                    });
                    self.screen = Screen::Backup;
                }
            }
            Action::BQ(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.q = if b.q == Some(n) { None } else { Some(n) };
                    b.scroll.follow = true;
                    b.naming = None;
                }
            }
            Action::BQNext(n) => {
                let qs = self.backup_questions(c);
                if let Some(b) = self.backup.as_mut() {
                    b.q = qs.iter().skip_while(|&&q| q != n).nth(1).copied();
                    b.scroll.follow = true;
                    b.naming = None;
                }
            }
            Action::BAnswer(list, row) => {
                // Back up's plan asks the rest place by place.
                if !matches!(list, qrow::SOFTWARE | qrow::FORM) {
                    return;
                }
                if let Some(i) = self.backup_locked_flow() {
                    return self.flow_wallet_answer(i, list, row);
                }
                let Some(w) = self.backup.as_ref().map(|b| b.wallet) else {
                    return;
                };
                let shape = self.plan_shape_now(w);
                if let (Some(q), Some(b)) = (qrow::question(list), self.backup.as_mut()) {
                    b.answers.toggle(&shape, q, usize::from(row));
                    // A question that no longer applies closes.
                    b.naming = b.naming.filter(|&p| p < b.answers.places);
                }
            }
            Action::BName(p8) => {
                let open = self.vaults.open.get(self.vaults.current).is_some();
                let mut focused = false;
                if let Some(b) = self.backup.as_mut()
                    && open
                {
                    let p = usize::from(p8);
                    if b.names.len() <= p {
                        b.names.resize(p + 1, String::new());
                    }
                    if b.naming == Some(p) {
                        b.naming = None;
                    } else {
                        // The field holds the name shown, "Place 1" until
                        // one is typed, so the press selects all of it
                        // (`docs/BACKUP-FLOW.md` §17.18).
                        if b.names[p].trim().is_empty() {
                            b.names[p] = fill(EN.place_name_place, &[&(p + 1).to_string()]);
                        }
                        b.naming = Some(p);
                        focused = true;
                    }
                }
                if focused {
                    self.typing.set_edit(ui::Edit {
                        field: Some(Action::BName(p8)),
                        caret: None,
                        anchor: Some(0),
                    });
                }
            }
            Action::BChecklist => {
                // A plan that keeps no copy of a key made here, or leaves
                // a vault's stick at no place, is not made into a
                // checklist (`docs/history/NEW-WALLET.md` §14.2, §14.4).
                // A backup past its lock keeps its plan until its writes
                // are done (`docs/BACKUP-FLOW.md` §15.8).
                if self.backup_locked_flow().is_some() {
                    return self.toast(EN.backup_flow_past_lock);
                }
                // A vault's stick at no place is a warning only
                // (`docs/BACKUP-FLOW.md` §17.16 item 3).
                if self.backup_kept_nowhere(c).is_some()
                    || self
                        .backup
                        .as_ref()
                        .and_then(|b| b.answers.empty_vault())
                        .is_some()
                {
                    return;
                }
                self.backup_extras();
                self.plan_save();
                self.backup_files_make();
                let items = self.backup_items_now(c);
                let first = items
                    .iter()
                    .copied()
                    .find(|&n| bstep::item(n).is_some_and(|it| !self.backup_item_done_now(c, it)));
                if let Some(b) = self.backup.as_mut() {
                    b.stage = BStage::Checklist;
                    b.naming = None;
                    b.scroll = flow::Scroll::default();
                }
                self.backup_item_open(first);
                self.backups_sync();
            }
            Action::BPlan => {
                // The cards show the map as it is, a chart's edits
                // included (`docs/BACKUP-FLOW.md` §15.2): nothing is
                // replaced, so nothing is asked.
                // What the checklist has put in vaults so far, should the
                // new plan drop it.
                self.backup_prior();
                if let Some(b) = self.backup.as_mut() {
                    b.stage = BStage::Plan;
                    // On the board, the first card, at the top: as a new
                    // plan opens (`docs/BACKUP-FLOW.md` §17.21 item 11).
                    b.q = Some(qstep::BOARD);
                    b.check = None;
                    b.scroll = flow::Scroll::default();
                }
            }
            Action::BWallets => {
                if let Some(b) = self.backup.as_mut() {
                    b.pick = !b.pick;
                }
            }
            Action::BStep(n) => {
                let open = self.backup.as_ref().and_then(|b| b.open);
                // An item opens once every item before it is done
                // (`docs/history/NEW-WALLET.md` §14.1).
                if open == Some(n) {
                    self.backup_item_open(None);
                } else if self.backup_reachable(c, n) {
                    self.backup_item_open(Some(n));
                }
            }
            Action::BNext(n) => {
                // No way past an item but doing it; the envelopes and the
                // blank templates, printed from Faraday Starter, are done
                // by this press.
                if n != bstep::ENVELOPE
                    && n != bstep::BLANK
                    && !bstep::item(n).is_some_and(|it| self.backup_item_done_now(c, it))
                {
                    return;
                }
                let items = self.backup_items_now(c);
                let next = if n == bstep::ENVELOPE || n == bstep::BLANK {
                    if let Some(b) = self.backup.as_mut() {
                        if n == bstep::ENVELOPE {
                            b.envelopes = true;
                        } else {
                            b.printed = true;
                        }
                    }
                    items.iter().copied().find(|&i| {
                        bstep::item(i).is_some_and(|it| !self.backup_item_done_now(c, it))
                    })
                } else {
                    items.iter().skip_while(|&&i| i != n).nth(1).copied()
                };
                self.backups_marked();
                self.backup_item_open(next);
            }
            Action::BKey(k) => {
                if let Some(b) = self.backup.as_mut() {
                    b.key = k;
                    b.reveal = false;
                    b.check = None;
                    b.scanned = None;
                    b.paper = None;
                }
            }
            Action::BReveal => {
                // Show the seed ends the check and wipes what was typed;
                // the check starts again from word 1.
                if let Some(b) = self.backup.as_mut() {
                    b.reveal = b.check.is_some() || !b.reveal;
                    b.check = None;
                }
            }
            Action::BXor(n) => self.paper_xor(n),
            Action::BCodex32(k, n) => self.paper_codex32(k, n),
            Action::BPaperHide => {
                if let Some(b) = self.backup.as_mut() {
                    b.paper = None;
                }
            }
            Action::BCompact(c) => {
                if let Some(b) = self.backup.as_mut() {
                    b.compact = c;
                    b.pin = 0;
                }
            }
            Action::BPin(r) => {
                if let Some(b) = self.backup.as_mut() {
                    b.pin = r;
                }
            }
            Action::BCheck => self.copy_check_open(),
            Action::BScan => {
                let fp = self
                    .backup
                    .as_ref()
                    .and_then(|b| self.session.keys.info(b.key))
                    .map(|k| k.fingerprint);
                if let Some(fp) = fp {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::CheckCopy(fp);
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.out.commands.push_back(Command::CameraOn);
                }
            }
            Action::BCheckClear => self.copy_check_clear(),
            Action::BKeepWords => self.keep_words_only(),
            Action::BVaultSave(v) => self.backup_vault_save(usize::from(v)),
            Action::BVaultLoad(v) => self.backup_vault_load_toggle(usize::from(v)),
            Action::BFilesMake => {
                self.backup_files_make();
                self.backups_sync();
            }
            Action::BackupFirst(w) => self.backup_first(w),
            Action::BNewVault(v) => {
                let v = usize::from(v);
                if let Some(t) = self.backup_vault_target(v) {
                    if self.backup_vault_set_up(v) == Some(t) {
                        // The vault set up for this plan vault keeps its
                        // stick and its place in the setup: it is not
                        // locked away from here.
                        let name = self.vaults.open[t].name.to_string();
                        self.toast(&fill(EN.backup_flow_set_up_holds_other, &[&name]));
                    } else if self.vault_lock_one(t) {
                        self.vault_act(vault::VaultAction::CreateFrom(Screen::Backup));
                    }
                }
            }
            Action::BFile => {
                // The seeds' files item makes the first one not yet made.
                let next = self.backup.as_ref().and_then(|b| {
                    let stem = file_stem(&self.session.wallets.get(b.wallet)?.name);
                    self.backup_keys(b.wallet)
                        .into_iter()
                        .find(|&k| !self.seed_file_made(&stem, &self.session.keys[k]))
                });
                if let (Some(k), Some(b)) = (next, self.backup.as_mut())
                    && b.open == Some(bstep::FILES)
                {
                    b.key = k;
                }
                self.offer_seed();
            }
            Action::BWords(n) => {
                if let Some(b) = self.backup.as_mut() {
                    b.words = n;
                }
            }
            Action::BOmit(n) => {
                if let Some(i) = self.backup_locked_flow() {
                    return self
                        .flow_plan_press(i, PlanPress::Omit(u8::try_from(n).unwrap_or(u8::MAX)));
                }
                let remake = self.backup_remake(4);
                if let Some(b) = self.backup.as_mut() {
                    b.answers.omit = n;
                }
                // The shares' files and pictures are made again once the
                // slider is let go and a frame drawn (`docs/SPEED.md` §9).
                if let Some(w) = remake {
                    self.later = Some(Later::PublicOut(w, 4));
                    self.later_drawn = false;
                }
            }
            Action::BOut(what) => self.backup_out(what),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The audit, for the shapes the backup is made in: a 3-of-5 split
    /// with two keys left off each sheet rebuilds from any three sheets,
    /// and no single sheet holds every key.
    #[test]
    fn any_quorum_of_split_sheets_rebuilds_the_wallet() {
        for (n, m) in [(3, 2), (5, 3), (7, 4), (15, 8)] {
            let plan = split_plan(n, m, m - 1);
            let a = audit(&plan, m);
            assert!(a.every_quorum_rebuilds, "{m} of {n}");
            assert!(
                a.keys_per_sheet < n,
                "{m} of {n}: one sheet holds every key"
            );
        }
    }

    /// Every test wallet written as a wallet .json reads back as the same
    /// wallet.
    #[test]
    fn a_wallet_json_reads_back() {
        let c = crate::testkit::compute();
        for kit in crate::testkit::kits() {
            let policy = crate::wallet::read_wallet(&c, &kit.descriptor).expect("kit");
            let json = wallet_json(kit.name, &policy);
            let back = crate::wallet::read_wallet(&c, &json).expect(kit.id);
            assert_eq!(
                back.to_descriptor_checksummed(),
                policy.to_descriptor_checksummed(),
                "{}",
                kit.id
            );
        }
    }

    /// A copy that differs names the word, counting from one, and one of
    /// another length the first word one has and the other lacks.
    #[test]
    fn a_wrong_word_is_named() {
        let seed = [1, 2, 3, 4];
        assert_eq!(compare_words(&[1, 2, 3, 4], &seed), CopyCheck::Matches);
        assert_eq!(compare_words(&[1, 2, 9, 4], &seed), CopyCheck::WrongWord(3));
        assert_eq!(compare_words(&[1, 2, 3], &seed), CopyCheck::WrongWord(4));
    }
}
