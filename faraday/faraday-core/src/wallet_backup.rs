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

use crate::copy_check::{PIN_MAX, Typed, WordCheck, WordsUse, copy_key};
use crate::forms;
use crate::plan::{CopyChecks, CopyForm, CopyForms};
use crate::wallet::{Kind, Session, Wallet, fp_text, wallet_name};
use crate::{
    Action, BStage, Code, Command, Faraday, Later, PlanPress, ScanPurpose, ScanState, Screen,
    Sheet, board, bstep, copy_check, file_stem, flow, paper, plan, plan_cards_on, qrow, qstep, ui,
    vault,
};
use faraday_bip::bip39::Language;
use faraday_shell_api::Key as KeyIn;
use faraday_ui::keyboard::OskPress;
use zeroize::Zeroize;

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

/// The shape of a wallet that Faraday's split sheets and Coldcard config
/// rebuild: what [`crate::restore::merge`] and
/// [`faraday_bip::multisig_config::parse`] write back out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rebuilt {
    /// `sortedmulti` in a wrapper, as the config's `Format:` names it.
    Config(&'static str),
    /// `tr(NUMS, sortedmulti_a(…))`.
    Taproot,
}

/// What a wallet's split sheets and config rebuild it as, where they
/// rebuild it at all: a `sortedmulti` in `sh`, `wsh` or `sh(wsh)`, or a
/// `tr` whose internal key is the NUMS point over one `sortedmulti_a`
/// leaf, its keys at `/<0;1>/*` as every wallet policy's are. The
/// rebuild always sorts the keys and always writes the NUMS point, so a
/// `multi`, a `multi_a` or another internal key would come back as
/// another wallet, and Restore refuses two keys of one seed
/// ([`crate::restore::repeats_a_seed`]): those have none.
fn rebuilt_as(wallet: &Wallet) -> Option<Rebuilt> {
    use faraday_bip::policy::{Template, Wrapper};
    let p = wallet.policy();
    if crate::restore::repeats_a_seed(p) {
        return None;
    }
    match Kind::of(p) {
        Kind::Multi(wrapper) if matches!(p.template(), Template::Multi { sorted: true, .. }) => {
            Some(Rebuilt::Config(match wrapper {
                Wrapper::Sh => "P2SH",
                Wrapper::ShWsh => "P2SH-P2WSH",
                Wrapper::Wsh => "P2WSH",
            }))
        }
        Kind::TapMulti if p.tapscript_sorted() && p.key_path_unspendable() => {
            Some(Rebuilt::Taproot)
        }
        _ => None,
    }
}

/// The multisig config Coldcard writes and Sparrow imports as "Coldcard
/// Multisig"; SeedSigner recognises it by "multisig setup file" in its
/// first line. `only` keeps those key lines alone, for a split sheet.
/// `None` for a wallet the config does not rebuild: every coordinator
/// that reads it sorts the keys, so only a `sortedmulti` wallet has one.
pub fn multisig_config(wallet: &Wallet, only: Option<&[usize]>) -> Option<String> {
    let Some(Rebuilt::Config(format)) = rebuilt_as(wallet) else {
        return None;
    };
    Some(config(
        wallet,
        only,
        format,
        "# Coldcard Multisig setup file (created by Faraday)\n",
    ))
}

/// Whether a wallet's backup can be split between its signers: a
/// `sortedmulti` in `wsh`, `sh` or `sh(wsh)`, or Taproot's
/// `tr(NUMS, sortedmulti_a)`, the wallets the sheets rebuild.
pub fn splits(wallet: &Wallet) -> bool {
    rebuilt_as(wallet).is_some()
}

/// One sheet of a split backup, holding the keys `keep` names: the
/// multisig config's lines for a `wsh` or `sh` multisig; for a Taproot
/// multisig the same lines with `Format: P2TR`, which Faraday rebuilds
/// as `tr(NUMS, sortedmulti_a(…))` (`restore::merge`). Coldcard's own
/// config has no Taproot format, so a Taproot wallet has no whole config.
pub fn split_share(wallet: &Wallet, keep: &[usize]) -> Option<String> {
    match rebuilt_as(wallet)? {
        Rebuilt::Config(_) => multisig_config(wallet, Some(keep)),
        Rebuilt::Taproot => Some(config(
            wallet,
            Some(keep),
            "P2TR",
            "# Faraday split backup sheet: Taproot multisig, tr(NUMS, sortedmulti_a)\n",
        )),
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
    out.push_str(&format!("name: {}\n", wallet_name(&wallet.name)));
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
    out.push_str(&format!("name: {}\n", wallet_name(&wallet.name)));
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
        out.push_str(&format!(
            "name: {}\nshape: {}\n",
            wallet_name(&w.name),
            Session::shape(w)
        ));
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
    /// Kept: the ok role.
    Ok,
    /// Not kept yet: warning.
    Warn,
    /// A copy in the clear: error.
    Err,
    /// Not known here: on-surface-variant.
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
    /// The block of the grid marked for copying, in reading order; the
    /// row, where it is marked by rows (`docs/BACKUP-FLOW.md` §17.24).
    pub pin: usize,
    /// The grid is marked row by row rather than block by block (the
    /// default).
    pub rows: bool,
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
    /// Work done that a later plan dropped, as what it is ("vault.fvlt
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
    /// The codex32 item's state, for the seed it was last open on
    /// (`docs/CODEX32.md` §9.4).
    pub c32: Option<crate::codex32_backup::Strings>,
    /// The codex32 strings typed back the same, or skipped, in this
    /// backup: each seed and index.
    pub c32_checked: Vec<(usize, usize)>,
    /// The open item, and whether it was done when last looked at: an
    /// item that becomes done moves on to the next.
    pub seen: Option<(u8, bool)>,
}

/// The wallet backup checklist (`Screen::Backup`): the backup under way,
/// while it is open. Its state is read through [`Faraday::backup`] and
/// changed only through the methods below.
#[derive(Default)]
pub(crate) struct WalletBackupArea {
    state: Option<BackupState>,
}

impl WalletBackupArea {
    /// The backup under way, while it is open.
    pub(crate) fn state(&self) -> Option<&BackupState> {
        self.state.as_ref()
    }

    /// The backup under way, for the tests' kit to set what it holds.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn state_mut(&mut self) -> Option<&mut BackupState> {
        self.state.as_mut()
    }

    /// Opens the checklist on `state`.
    pub(crate) fn open(&mut self, state: BackupState) {
        self.state = Some(state);
    }

    /// Drops the backup under way.
    pub(crate) fn close(&mut self) {
        self.state = None;
    }

    /// Drops the backup under way, returning it.
    pub(crate) fn take(&mut self) -> Option<BackupState> {
        self.state.take()
    }

    /// The column's scroll offset, to move it.
    pub(crate) fn scroll_y_mut(&mut self) -> Option<&mut f32> {
        self.state.as_mut().map(|b| &mut b.scroll.y)
    }

    /// Sets the column's scroll, as the drawing pass noted it.
    pub(crate) fn set_scroll(&mut self, scroll: flow::Scroll) {
        if let Some(b) = self.state.as_mut() {
            b.scroll = scroll;
        }
    }

    /// What the board holds between presses, to change.
    pub(crate) fn board_mut(&mut self) -> Option<&mut board::BoardState> {
        self.state.as_mut().map(|b| &mut b.board)
    }

    /// The board's empty sticks, menu and focus let go, where it is not
    /// on screen.
    pub(crate) fn board_tidy(&mut self) {
        if let Some(b) = self.state.as_mut() {
            b.board.empty.clear();
            b.board.menu = None;
            b.board.focus = None;
        }
    }

    /// The open typed check, for the word entry that types into it.
    pub(crate) fn check_mut(&mut self) -> Option<&mut copy_check::WordCheck> {
        self.state.as_mut().and_then(|b| b.check.as_mut())
    }

    /// A press that changes only the checklist's own state: a question
    /// opened, the wallet list, a seed, its words shown, its paper form
    /// hidden, the SeedQR's kind, a pinned row, the template's words.
    pub(crate) fn edit(&mut self, action: Action) {
        let Some(b) = self.state.as_mut() else {
            return;
        };
        match action {
            Action::BQ(n) => {
                b.q = if b.q == Some(n) { None } else { Some(n) };
                b.scroll.follow = true;
                b.naming = None;
            }
            Action::BWallets => b.pick = !b.pick,
            Action::BKey(k) => {
                b.key = k;
                b.reveal = false;
                b.check = None;
                b.scanned = None;
                b.paper = None;
            }
            Action::BReveal => {
                // Show the seed ends the check and wipes what was typed;
                // the check starts again from word 1.
                b.reveal = b.check.is_some() || !b.reveal;
                b.check = None;
            }
            Action::BPaperHide => {
                b.paper = None;
            }
            Action::BCompact(c) => {
                b.compact = c;
                b.pin = 0;
            }
            Action::BPin(r) => b.pin = r.min(PIN_MAX),
            Action::BRows(r) => {
                b.rows = r;
                b.pin = 0;
            }
            Action::BWords(n) => b.words = n,
            _ => {}
        }
    }

    /// Next on question `n`: the question after it in `qs` opens.
    pub(crate) fn next_question(&mut self, qs: &[u8], n: u8) {
        if let Some(b) = self.state.as_mut() {
            b.q = qs.iter().skip_while(|&&q| q != n).nth(1).copied();
            b.scroll.follow = true;
            b.naming = None;
        }
    }

    /// Row `row` of question `q` toggled in the plan of shape `shape`.
    pub(crate) fn answer(&mut self, shape: &plan::Shape, q: plan::Question, row: usize) {
        if let Some(b) = self.state.as_mut() {
            b.answers.toggle(shape, q, row);
            // A question that no longer applies closes.
            b.naming = b.naming.filter(|&p| p < b.answers.places);
        }
    }

    /// A press on place `p`'s name, with a vault open to keep it: its
    /// field opens, holding the name shown, or closes. Whether it opened.
    pub(crate) fn name_press(&mut self, p: usize) -> bool {
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        if b.names.len() <= p {
            b.names.resize(p + 1, String::new());
        }
        if b.naming == Some(p) {
            b.naming = None;
            return false;
        }
        // The field holds the name shown, "Place 1" until one is typed,
        // so the press selects all of it (`docs/BACKUP-FLOW.md` §17.18).
        if b.names[p].trim().is_empty() {
            b.names[p] = fill(EN.place_name_place, &[&(p + 1).to_string()]);
        }
        b.naming = Some(p);
        true
    }

    /// A key typed while a place's name is open, or on a copy item's
    /// grid: the row pinned moves, the name takes the key. Whether the
    /// key was taken.
    pub(crate) fn name_key(&mut self, key: KeyIn) -> bool {
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        // A codex32 string typed back, or in, on the keyboard.
        if b.open.is_some_and(bstep::is_codex32)
            && b.c32.as_ref().is_some_and(|c| {
                c.at.is_some() || !matches!(c.set, crate::codex32_backup::SetStep::None)
            })
        {
            let press = match key {
                KeyIn::Char(c) => OskPress::Char(c),
                KeyIn::Backspace => OskPress::Back,
                _ => return false,
            };
            return self.c32_key(press);
        }
        match key {
            // The grid's mark: row by row, or block by block in reading
            // order, Left and Right too (`docs/BACKUP-FLOW.md` §17.24).
            KeyIn::Down | KeyIn::Right if copy_key(b, key) => b.pin = (b.pin + 1).min(PIN_MAX),
            KeyIn::Up | KeyIn::Left if copy_key(b, key) => b.pin = b.pin.saturating_sub(1),
            // A place's name, typed: kept in the vault alone.
            KeyIn::Char(c) if b.naming.is_some() => {
                if let Some(name) = b.naming.and_then(|p| b.names.get_mut(p))
                    && name.chars().count() < 32
                    && !c.is_control()
                {
                    name.push(c);
                }
            }
            KeyIn::Backspace if b.naming.is_some() => {
                if let Some(name) = b.naming.and_then(|p| b.names.get_mut(p)) {
                    name.pop();
                }
            }
            KeyIn::Enter | KeyIn::Escape if b.naming.is_some() => b.naming = None,
            _ => return false,
        }
        true
    }

    /// A key of the codex32 keyboard on the open string: typed in, and a
    /// string that holds counted as checked (`docs/CODEX32.md` §9.4).
    /// Whether it was taken.
    pub(crate) fn c32_key(&mut self, press: OskPress) -> bool {
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        let Some(st) = b.c32.as_mut() else {
            return false;
        };
        let Some(form) = b.answers.c32(st.seed) else {
            return false;
        };
        let seed = st.seed;
        let Some(j) = st.at else {
            // A string typed in for a set action (`docs/CODEX32.md` §9.7).
            if let Some(whole) = st.type_in(form, press) {
                st.held.push(zeroize::Zeroizing::new(whole));
            }
            return true;
        };
        if let Some(whole) = st.key(form, press) {
            st.held.push(zeroize::Zeroizing::new(whole));
            if !b.c32_checked.contains(&(seed, j)) {
                b.c32_checked.push((seed, j));
            }
        }
        true
    }

    /// The codex32 item's state, to change.
    pub(crate) fn c32_mut(&mut self) -> Option<&mut crate::codex32_backup::Strings> {
        self.state.as_mut()?.c32.as_mut()
    }

    /// The codex32 item, its seed's form and the strings checked or
    /// skipped past, to change.
    pub(crate) fn c32_parts(&mut self) -> Option<crate::codex32_backup::Parts<'_>> {
        let b = self.state.as_mut()?;
        let seed = b.c32.as_ref()?.seed;
        Some((b.c32.as_mut()?, b.answers.c32(seed)?, &mut b.c32_checked))
    }

    /// Codex32 string `j` of seed `i` to be checked again: made anew.
    pub(crate) fn c32_unchecked(&mut self, i: usize, j: usize) {
        if let Some(b) = self.state.as_mut() {
            b.c32_checked.retain(|&x| x != (i, j));
        }
    }

    /// Seed `i`'s set replaced: none of its strings checked, nothing kept
    /// of the old ones (`docs/CODEX32.md` §9.7, Rotate the set).
    pub(crate) fn c32_reset(&mut self, i: usize, checked: Vec<(usize, usize)>) {
        if let Some(b) = self.state.as_mut() {
            b.c32_checked = checked;
            if let Some(st) = b.c32.as_mut().filter(|st| st.seed == i) {
                *st = crate::codex32_backup::Strings::new(i);
            }
        }
    }

    /// The open item as last seen, and whether it was done then.
    pub(crate) fn set_seen(&mut self, seen: Option<(u8, bool)>) {
        if let Some(b) = self.state.as_mut() {
            b.seen = seen;
        }
    }

    /// Place `p` removed from the plan: its name goes, and the places
    /// after are numbered anew where their names were their own.
    pub(crate) fn place_removed(&mut self, p: usize) {
        if let Some(b) = self.state.as_mut()
            && p < b.names.len()
        {
            // A name left as its place's own ("Place 3", put in the field
            // by a press on it) is no name: the places after are numbered
            // anew.
            for (q, n) in b.names.iter_mut().enumerate() {
                if *n == fill(EN.place_name_place, &[&(q + 1).to_string()]) {
                    n.clear();
                }
            }
            b.names.remove(p);
            b.naming = None;
        }
    }

    /// The plan made into the checklist, from the top.
    pub(crate) fn checklist(&mut self) {
        if let Some(b) = self.state.as_mut() {
            b.stage = BStage::Checklist;
            b.naming = None;
            b.scroll = flow::Scroll::default();
        }
    }

    /// The checklist as a chart opens it: made, with no question or item
    /// open, from the top. Nothing changes where it was made already.
    pub(crate) fn checklist_closed(&mut self) {
        if let Some(b) = self.state.as_mut()
            && b.stage != BStage::Checklist
        {
            b.stage = BStage::Checklist;
            b.q = None;
            b.naming = None;
            b.open = None;
            b.scroll = flow::Scroll::default();
        }
    }

    /// Back to the plan, question `q` open, from the top.
    pub(crate) fn back_to_plan(&mut self, q: Option<u8>) {
        if let Some(b) = self.state.as_mut() {
            b.stage = BStage::Plan;
            b.q = q;
            b.check = None;
            b.scroll = flow::Scroll::default();
        }
    }

    /// The plan a chart shows put into the backup, where its checklist is
    /// not made yet: its answers, and its names where none are typed.
    /// Opened from a vault's view when `from_vault`. False with no backup
    /// open.
    pub(crate) fn chart_plan(
        &mut self,
        from_vault: bool,
        known: Option<(plan::Answers, Vec<String>)>,
    ) -> bool {
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        b.from_vault = from_vault;
        if b.stage != BStage::Checklist
            && let Some((answers, names)) = known
        {
            b.answers = answers;
            if b.names.iter().all(|n| n.trim().is_empty()) {
                b.names = names;
            }
            b.q = None;
        }
        true
    }

    /// Opened from a chart in an open vault's view, or not.
    pub(crate) fn set_from_vault(&mut self, from_vault: bool) {
        if let Some(b) = self.state.as_mut() {
            b.from_vault = from_vault;
        }
    }

    /// Opened from Create's Back up card.
    pub(crate) fn set_from_create(&mut self) {
        if let Some(b) = self.state.as_mut() {
            b.from_create = true;
        }
    }

    /// The list of loaded wallets under the wallet chip closed.
    pub(crate) fn close_pick(&mut self) {
        if let Some(b) = self.state.as_mut() {
            b.pick = false;
        }
    }

    /// The envelopes (`bstep::ENVELOPE`) or the blank templates (any
    /// other item) marked done by their press.
    pub(crate) fn mark_done(&mut self, n: u8) {
        if let Some(b) = self.state.as_mut() {
            if n == bstep::ENVELOPE {
                b.envelopes = true;
            } else {
                b.printed = true;
            }
        }
    }

    /// Opens checklist item `n`, or closes them all, showing seed `key`
    /// where it has one; the public files item's More options open when
    /// `more`.
    pub(crate) fn open_item(&mut self, n: Option<u8>, more: bool, key: Option<usize>) {
        if let Some(b) = self.state.as_mut() {
            // A passphrase item opens, and closes, with it hidden.
            if n.is_some_and(bstep::is_passphrase) || b.open.is_some_and(bstep::is_passphrase) {
                b.reveal = false;
            }
            b.open = n;
            b.public_more = more;
            // The codex32 item opens on its seed's strings, its list.
            if let Some(i) = n.and_then(bstep::item).and_then(|it| match it {
                plan::Item::Codex32(i) => Some(i),
                _ => None,
            }) {
                match b.c32.as_mut() {
                    Some(st) if st.seed == i => st.list(),
                    _ => b.c32 = Some(crate::codex32_backup::Strings::new(i)),
                }
            }
            b.scroll.follow = true;
            b.check = None;
            if let Some(k) = key
                && b.key != k
            {
                b.key = k;
                b.reveal = false;
                b.scanned = None;
                b.paper = None;
            }
        }
    }

    /// The seeds' files item shows seed `k`, where it is open.
    pub(crate) fn file_key(&mut self, k: usize) {
        if let Some(b) = self.state.as_mut()
            && b.open == Some(bstep::FILES)
        {
            b.key = k;
        }
    }

    /// The keys each share leaves off.
    pub(crate) fn set_omit(&mut self, n: usize) {
        if let Some(b) = self.state.as_mut() {
            b.answers.omit = n;
        }
    }

    /// The plan's answers replaced.
    #[cfg(test)]
    pub(crate) fn set_answers(&mut self, answers: plan::Answers) {
        if let Some(b) = self.state.as_mut() {
            b.answers = answers;
        }
    }

    /// Wallet `w`'s flow changed its plan: the checklist's answers
    /// follow, where it is open on that wallet.
    pub(crate) fn follow_answers(&mut self, w: usize, answers: plan::Answers) {
        if let Some(b) = self.state.as_mut().filter(|b| b.wallet == w) {
            b.answers = answers;
        }
    }

    /// The plan's answers changed by `f`, in the plan of shape `shape`;
    /// a place's name open past the last place closes.
    pub(crate) fn change_answers(
        &mut self,
        shape: &plan::Shape,
        f: &dyn Fn(&mut plan::Answers, &plan::Shape),
    ) {
        if let Some(b) = self.state.as_mut() {
            f(&mut b.answers, shape);
            let places = b.answers.places;
            b.naming = b.naming.filter(|&p| p < places);
        }
    }

    /// The plan's answers and the places' names, as a chart's edit left
    /// them. Returns the work a later plan dropped.
    pub(crate) fn chart_edit(&mut self, answers: &plan::Answers, names: &[String]) -> Vec<String> {
        let Some(b) = self.state.as_mut() else {
            return Vec::new();
        };
        b.answers = answers.clone();
        b.names = names.to_vec();
        b.extras.clone()
    }

    /// Keep the words only: the plan's answers as it leaves them, the
    /// scan and the check gone.
    pub(crate) fn keep_words(&mut self, answers: plan::Answers) {
        if let Some(b) = self.state.as_mut() {
            b.answers = answers;
            b.scanned = None;
            b.check = None;
        }
    }

    /// Files of wallet `wallet` just put in the Outbox, for the summary,
    /// where its backup is the one under way.
    pub(crate) fn sent(&mut self, wallet: usize, names: &[String]) {
        if let Some(b) = self.state.as_mut()
            && b.wallet == wallet
        {
            for n in names {
                if !b.sent.contains(n) {
                    b.sent.push(n.clone());
                }
            }
        }
    }

    /// The descriptor of wallet `wallet` was shown as a code: marked
    /// where its backup is the one under way. Whether it was.
    pub(crate) fn shown(&mut self, wallet: usize) -> bool {
        match self.state.as_mut() {
            Some(b) if b.wallet == wallet => {
                b.shown = true;
                true
            }
            _ => false,
        }
    }

    /// Public files' More options opened, where wallet `wallet`'s backup
    /// is the one under way.
    pub(crate) fn public_more(&mut self, wallet: usize) {
        if let Some(b) = self.state.as_mut()
            && b.wallet == wallet
        {
            b.public_more = true;
        }
    }

    /// Seed `fp`'s files on a stick let out unprotected past the secret
    /// sheet's warning.
    pub(crate) fn files_warned(&mut self, fp: String) {
        if let Some(b) = self.state.as_mut()
            && !b.files_warned.contains(&fp)
        {
            b.files_warned.push(fp);
        }
    }

    /// The vault item's **Load at unlock** turned off or on for plan
    /// vault `v`. False where the plan keeps no wallet there.
    pub(crate) fn load_toggle(&mut self, v: usize) -> bool {
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        if !b.answers.wallet_in_vault(v) {
            return false;
        }
        if let Some(k) = b.no_load.iter().position(|&x| x == v) {
            b.no_load.remove(k);
        } else {
            b.no_load.push(v);
        }
        true
    }

    /// What the vault of salt `salt` held before this backup first saved
    /// into it.
    pub(crate) fn note_before(&mut self, salt: [u8; 32], held: Vec<String>) {
        if let Some(b) = self.state.as_mut() {
            b.before.push((salt, held));
        }
    }

    /// The vaults the checklist had filled, as Change the plan finds them.
    pub(crate) fn set_prior(&mut self, prior: Vec<(String, Vec<String>)>) {
        if let Some(b) = self.state.as_mut() {
            b.prior = prior;
        }
    }

    /// The work a later plan dropped, the prior vaults taken into it.
    pub(crate) fn set_extras(&mut self, extras: Vec<String>) {
        if let Some(b) = self.state.as_mut() {
            b.extras = extras;
            b.prior.clear();
        }
    }

    /// Seed `fp`'s copy matched in form `form`.
    pub(crate) fn note_checked(&mut self, fp: [u8; 4], form: plan::CopyForm) {
        if let Some(b) = self.state.as_mut()
            && !b.checked.contains(&(fp, form))
        {
            b.checked.push((fp, form));
        }
    }

    /// What scanning the copy on screen found.
    pub(crate) fn set_scanned(&mut self, found: CopyCheck) {
        if let Some(b) = self.state.as_mut() {
            b.scanned = Some(found);
        }
    }

    /// A scan of seed `fp`'s copy closed without a match: counted once
    /// for the backup under way.
    pub(crate) fn copy_missed(&mut self, fp: [u8; 4]) {
        let Some(b) = self.state.as_mut() else {
            return;
        };
        match b.misses.iter_mut().find(|(f, _)| *f == fp) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => b.misses.push((fp, 1)),
        }
    }

    /// Another paper form of the key, on screen.
    pub(crate) fn set_paper(&mut self, form: paper::PaperForm) {
        if let Some(b) = self.state.as_mut() {
            b.paper = Some(form);
        }
    }

    /// Check my copy opened on `check`, the seed hidden.
    pub(crate) fn check_start(&mut self, check: Option<copy_check::WordCheck>) {
        if let Some(b) = self.state.as_mut() {
            b.reveal = false;
            b.check = check;
        }
    }

    /// Clear: the open check starts again as `check`.
    pub(crate) fn check_restart(&mut self, check: Option<copy_check::WordCheck>) {
        if let Some(b) = self.state.as_mut()
            && b.check.is_some()
        {
            b.check = check;
        }
    }

    /// The check closed, what was typed wiped.
    pub(crate) fn check_close(&mut self) {
        if let Some(b) = self.state.as_mut() {
            b.check = None;
        }
    }

    /// A key typed into the open check; closed by its Escape.
    pub(crate) fn check_key(&mut self, key: KeyIn) -> copy_check::Typed {
        let Some(c) = self.check_mut() else {
            return copy_check::Typed::Nothing;
        };
        let typed = c.key(key);
        if typed == copy_check::Typed::Close {
            self.check_close();
        }
        typed
    }

    /// The sample the check shows is dismissed.
    pub(crate) fn check_sample_gone(&mut self) {
        if let Some(c) = self.check_mut() {
            c.sample_gone = true;
        }
    }

    /// Word `k` of the words typed back differs, as `line` says: the
    /// words before it stay and typing resumes at it.
    pub(crate) fn check_wrong(&mut self, k: usize, line: String) {
        if let Some(c) = self.check_mut() {
            c.wrong(k, line);
        }
    }

    /// The passphrase of seed `fp` typed back matched: the check closes
    /// and the item is done.
    pub(crate) fn passphrase_matched(&mut self, fp: [u8; 4]) {
        if let Some(b) = self.state.as_mut() {
            b.check = None;
            if !b.checked.contains(&(fp, plan::CopyForm::Passphrase)) {
                b.checked.push((fp, plan::CopyForm::Passphrase));
            }
        }
    }

    /// The passphrase typed back differs: wiped, and said.
    pub(crate) fn passphrase_differs(&mut self) {
        if let Some(c) = self.check_mut() {
            c.typed.clear();
            c.error = Some(EN.copy_check_passphrase_differs.to_string());
        }
    }
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
                        Some((mut a, names)) => {
                            plan::codex32::sync(&mut a, &shape, &self.c32_sets(i));
                            (a, names, None)
                        }
                        // A key with a codex32 set places that set: one
                        // string per place (`docs/CODEX32.md` §9.5).
                        None => (
                            plan::codex32::standard(c, &shape, &self.c32_sets(i)),
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
                    self.backup.open(BackupState {
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
            Action::BQ(_)
            | Action::BWallets
            | Action::BKey(_)
            | Action::BReveal
            | Action::BPaperHide
            | Action::BCompact(_)
            | Action::BPin(_)
            | Action::BRows(_)
            | Action::BWords(_) => self.backup.edit(action),
            Action::BQNext(n) => {
                let qs = self.backup_questions(c);
                self.backup.next_question(&qs, n);
            }
            Action::BAnswer(list, row) => {
                // Back up's plan asks the rest place by place.
                if !matches!(list, qrow::SOFTWARE | qrow::FORM) {
                    return;
                }
                if let Some(i) = self.backup_locked_flow() {
                    return self.flow_wallet_answer(i, list, row);
                }
                let Some(w) = self.backup.state().map(|b| b.wallet) else {
                    return;
                };
                let shape = self.plan_shape_now(w);
                if let Some(q) = qrow::question(list) {
                    self.backup.answer(&shape, q, usize::from(row));
                }
            }
            Action::BName(p8) => {
                let open = self.vaults.open.get(self.vaults.current).is_some();
                if open && self.backup.name_press(usize::from(p8)) {
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
                        .state()
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
                self.backup.checklist();
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
                // On the board, the first card, at the top: as a new plan
                // opens (`docs/BACKUP-FLOW.md` §17.21 item 11).
                self.backup.back_to_plan(Some(qstep::BOARD));
            }
            Action::BStep(n) => {
                let open = self.backup.state().and_then(|b| b.open);
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
                    self.backup.mark_done(n);
                    items.iter().copied().find(|&i| {
                        bstep::item(i).is_some_and(|it| !self.backup_item_done_now(c, it))
                    })
                } else {
                    items.iter().skip_while(|&&i| i != n).nth(1).copied()
                };
                self.backups_marked();
                self.backup_item_open(next);
            }
            Action::BXor(n) => self.paper_xor(n),
            Action::BC32(p) => self.backup_c32(p, c),
            Action::BCheck => self.copy_check_open(),
            Action::BScan => {
                let fp = self
                    .backup
                    .state()
                    .and_then(|b| self.session.keys.info(b.key))
                    .map(|k| k.fingerprint);
                if let Some(fp) = fp {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::CheckCopy(fp);
                    self.camera.open(scan);
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
                let next = self.backup.state().and_then(|b| {
                    let stem = file_stem(&self.session.wallets.get(b.wallet)?.name);
                    self.backup_keys(b.wallet)
                        .into_iter()
                        .find(|&k| !self.seed_file_made(&stem, &self.session.keys[k]))
                });
                if let Some(k) = next {
                    self.backup.file_key(k);
                }
                self.offer_seed();
            }
            Action::BOmit(n) => {
                if let Some(i) = self.backup_locked_flow() {
                    return self
                        .flow_plan_press(i, PlanPress::Omit(u8::try_from(n).unwrap_or(u8::MAX)));
                }
                let remake = self.backup_remake(4);
                self.backup.set_omit(n);
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

impl Faraday {
    /// The wallet backup checklist under way, while it is open.
    pub fn backup(&self) -> Option<&BackupState> {
        self.backup.state()
    }

    /// The word entry the on-screen keys and the candidates press into.
    pub(crate) fn words_use(&mut self) -> WordsUse<'_> {
        if self.screen == Screen::Backup
            && let Some(c) = self.backup.check_mut().filter(|c| !c.passphrase)
        {
            return WordsUse {
                keys: &mut c.keys,
                shift: &mut c.shift,
                error: &mut c.error,
                lang: c.lang,
                in_check: true,
            };
        }
        let e = self.entry.state_mut();
        let lang = e.language();
        WordsUse {
            keys: &mut e.keys,
            shift: &mut e.shift,
            error: &mut e.error,
            lang,
            in_check: false,
        }
    }

    /// Costs what `plan_mapped` costs. The forms of the copies of seed `i`
    /// the backup's map keeps, read from the plan's map
    /// (`Kept::plan_mapped`: once a frame while drawing).
    pub(crate) fn copy_forms(&self, c: &Compute, b: &BackupState, i: usize) -> CopyForms {
        let shape = self.plan_shape_now(b.wallet);
        plan::copy_forms_on(&self.kept.plan_mapped(c, &shape, &b.answers), i)
    }

    /// Costs what `copy_forms` costs. What checking seed `i`'s copies asks
    /// for with the cameras listed now.
    pub(crate) fn copy_checks(
        &self,
        c: &Compute,
        b: &BackupState,
        i: usize,
        key: &faraday_keys::KeyInfo,
    ) -> CopyChecks {
        self.copy_forms(c, b, i).checks(
            !self.cameras().is_empty(),
            key.language == Language::English,
        )
    }

    /// Costs what `copy_checks` and `copy_forms` cost. Whether every check
    /// seed `i`'s copy item asks for has matched in this backup. A scan of
    /// the drawn grid also stands for the words of a seed with no copy of
    /// its words.
    pub(crate) fn copy_checked(
        &self,
        c: &Compute,
        b: &BackupState,
        i: usize,
        key: &faraday_keys::KeyInfo,
    ) -> bool {
        let fp = key.fingerprint.0;
        let checks = self.copy_checks(c, b, i, key);
        let has = |f: CopyForm| b.checked.contains(&(fp, f));
        (checks.typed || checks.scan)
            && (!checks.typed
                || has(CopyForm::Words)
                || (!self.copy_forms(c, b, i).words && has(CopyForm::SeedQr)))
            && (!checks.scan || has(CopyForm::SeedQr))
    }

    /// Costs what `copy_checked` costs. Whether seed `fp`'s copy item is
    /// done in the backup under way; a seed the backup has no item for is
    /// checked by any match.
    pub(crate) fn seed_copy_checked(&self, c: &Compute, fp: [u8; 4]) -> bool {
        let Some(b) = self.backup.state() else {
            return false;
        };
        let at = self
            .backup_seed_list_now(b.wallet)
            .iter()
            .enumerate()
            .find_map(|(i, (_, k))| {
                let key = self.session.keys.info((*k)?)?;
                (key.fingerprint.0 == fp).then_some((i, key))
            });
        match at {
            Some((i, key)) => self.copy_checked(c, b, i, key),
            None => b
                .checked
                .iter()
                .any(|(f, form)| *f == fp && *form != CopyForm::Passphrase),
        }
    }

    /// Check my copy: opens the typed check, with nothing typed and the
    /// seed hidden. With a passphrase's check open, Compare.
    pub(crate) fn copy_check_open(&mut self) {
        let Some(b) = self.backup.state() else {
            return;
        };
        if let Some(c) = b.check.as_ref() {
            if c.passphrase {
                self.passphrase_check_try();
            }
            return;
        }
        let check = self.copy_check_new();
        self.backup.check_start(check);
    }

    /// Clear: the check starts again from word 1.
    pub(crate) fn copy_check_clear(&mut self) {
        let check = self.copy_check_new();
        self.backup.check_restart(check);
    }

    fn copy_check_new(&self) -> Option<WordCheck> {
        let b = self.backup.state()?;
        if b.open.is_some_and(crate::bstep::is_passphrase) {
            return Some(WordCheck::passphrase());
        }
        let key = self.session.keys.info(b.key)?;
        let count = usize::from(key.word_count?);
        let keyed = forms::on_screen(key.language) || self.compact;
        Some(WordCheck::new(key.language, count, keyed))
    }

    /// One key of the open check, typed through Add a key's word entry,
    /// with a word's four-digit number accepted.
    pub(crate) fn copy_check_key(&mut self, key: KeyIn) {
        match self.backup.check_key(key) {
            Typed::Compare => self.passphrase_check_try(),
            Typed::Words => self.copy_check_after(),
            Typed::Nothing | Typed::Close => {}
        }
    }

    /// Compares the words with the seed once the last is taken, not
    /// before. A match is the item's words check, done: the words are
    /// wiped and the check closes. A word that differs is named; the
    /// words before it stay and typing resumes at it.
    pub(crate) fn copy_check_after(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(chk) = b.check.as_ref() else {
            return;
        };
        if chk.passphrase || chk.taken() < chk.count {
            return;
        }
        let Some(key) = self.session.keys.info(b.key) else {
            return;
        };
        let fp = key.fingerprint.0;
        let mut typed = chk.indices();
        let Some(found) = self.session.keys.words_match(&c, b.key, &typed) else {
            typed.zeroize();
            return;
        };
        let off_list = match found {
            CopyCheck::WrongWord(k) => typed.get(k - 1) == Some(&u16::MAX),
            CopyCheck::Matches => false,
        };
        typed.zeroize();
        match found {
            CopyCheck::Matches => {
                self.backup.check_close();
                self.copy_matched(fp, CopyForm::Words);
            }
            CopyCheck::WrongWord(k) => {
                let line = if off_list {
                    fill(
                        EN.forms_word_not_in_list,
                        &[&k.to_string(), forms::language_name(self.check_language())],
                    )
                } else {
                    fill(EN.copy_check_word_differs, &[&k.to_string()])
                };
                self.backup.check_wrong(k, line);
            }
        }
    }

    /// The list of the seed under check.
    fn check_language(&self) -> Language {
        self.backup
            .state()
            .and_then(|b| b.check.as_ref())
            .map_or(Language::English, |c| c.lang)
    }

    /// Keep the words only, on the open copy item: its seed's SeedQR
    /// copies become copies of the words, the grid kept on the map drawn
    /// in part ([`plan::keep_words`]), and the plan saved. Items done stay
    /// done; the item now asks for typed words, and with those already
    /// checked it is done at once and the checklist moves on.
    pub(crate) fn keep_words_only(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(n) = b.open.filter(|&n| crate::bstep::is_copy(n)) else {
            return;
        };
        let i = usize::from(n - crate::bstep::COPY);
        if !self.copy_forms(&c, b, i).seedqr {
            return;
        }
        let shape = self.plan_shape_now(b.wallet);
        let answers = plan::keep_words(&shape, &b.answers, i);
        self.backup.keep_words(answers);
        self.plan_save();
        self.backups_sync();
        if crate::bstep::item(n).is_some_and(|it| self.backup_item_done_now(&c, it)) {
            self.act(crate::Action::BNext(n));
        }
    }

    /// Compare: the passphrase typed against the one the key was loaded
    /// with, whole, exactly and in constant time. A match is the item
    /// done; either way what was typed is wiped. A key that keeps no
    /// passphrase matches nothing.
    pub(crate) fn passphrase_check_try(&mut self) {
        let Some(b) = self.backup.state() else {
            return;
        };
        let Some(chk) = b.check.as_ref().filter(|c| c.passphrase) else {
            return;
        };
        let Some(key) = self.session.keys.info(b.key) else {
            return;
        };
        let fp = key.fingerprint.0;
        let matched = self.session.keys.passphrase_matches(b.key, &chk.typed);
        if matched {
            self.backup.passphrase_matched(fp);
            self.backups_passphrase_checked(fp);
        } else {
            self.backup.passphrase_differs();
        }
    }

    /// Whether the key the backup shows has words to make a paper form of.
    fn paper_key(&self) -> bool {
        self.backup
            .state()
            .and_then(|b| self.session.keys.info(b.key))
            .is_some_and(|k| k.has_words())
    }

    /// Splits the key's words into `parts` Seed XOR parts.
    pub(crate) fn paper_xor(&mut self, parts: u8) {
        let c = self.compute();
        // Fresh parts are a new secret: none in the demo.
        if !self.makes_secrets() {
            return;
        }
        if !self.paper_key() {
            return self.toast(EN.paper_no_words_to_split);
        }
        let Some(draw) = self.fresh_bytes(b"seed xor", faraday_keys::seed_xor_random_len(parts))
        else {
            return self.toast(EN.paper_no_randomness);
        };
        let Some(b) = self.backup.state() else {
            return;
        };
        let Ok(lines) =
            self.session
                .keys
                .seed_xor(&c, b.key, parts, &draw, faraday_keys::Reveal::PaperForm)
        else {
            return;
        };
        self.backup.set_paper(paper::PaperForm::Xor(lines));
    }
}

impl Faraday {
    /// Costs what `vault_has_wallet`, `vault_key` and `seed_copy_checked`
    /// cost. Where the wallet being backed up and each of its keys are
    /// kept, as the small panel's copy page lists them: in which open
    /// vault, whether the copy by hand was checked, whether a file of it is
    /// in the Outbox unprotected, or not here at all.
    pub fn backup_kept(&self, c: &Compute) -> Option<Kept> {
        use crate::vault::screen::{vault_has_wallet, vault_key};
        let b = self.backup.state()?;
        let wallet = self.session.wallets.get(b.wallet)?;
        let open = 0..self.vaults.open.len();
        // A locked vault cannot be looked into.
        let unknown = self.vaults.open.is_empty() && !self.vault_files().is_empty();
        let not_known = || (EN.backup_kept_not_checked.to_string(), Tone::Dim);
        let wallet_line = match open.clone().find(|&v| vault_has_wallet(c, self, v, wallet)) {
            Some(v) => (
                fill(
                    EN.backup_kept_wallet_vault,
                    &[&self.vaults.open[v].name.to_string()],
                ),
                Tone::Ok,
            ),
            None if unknown => not_known(),
            None => (EN.backup_kept_wallet.to_string(), Tone::Warn),
        };
        let stem = file_stem(&wallet.name);
        let here = self.backup_keys(b.wallet);
        let seed_lines = |key: &faraday_keys::KeyInfo| -> Vec<(String, Tone)> {
            let fp = key.fingerprint;
            let mut lines = Vec::new();
            lines.push(
                match open
                    .clone()
                    .find_map(|v| vault_key(c, self, v, key).map(|p| (v, p)))
                {
                    Some((v, with)) => (
                        fill(
                            EN.backup_kept_vault,
                            &[
                                &self.vaults.open[v].name.to_string(),
                                &(if with {
                                    format!(" {}", EN.backup_with_its_passphrase)
                                } else {
                                    String::new()
                                }),
                            ],
                        ),
                        Tone::Ok,
                    ),
                    None if unknown => not_known(),
                    None => (EN.backup_kept_no_vault.to_string(), Tone::Warn),
                },
            );
            lines.push(if self.seed_copy_checked(c, fp.0) {
                (EN.backup_kept_paper_copy.to_string(), Tone::Ok)
            } else {
                (EN.backup_kept_paper_unchecked.to_string(), Tone::Warn)
            });
            let fps = fp_text(fp).to_lowercase();
            let files = ["words.txt", "seedqr.png", "compactseedqr.png"]
                .map(|end| format!("{stem}-{fps}-{end}"));
            if self
                .outbox
                .iter()
                .any(|i| i.secret && files.contains(&i.name))
            {
                lines.push((
                    fill(EN.backup_kept_file_unprotected, &[self.medium.for_the()]),
                    Tone::Err,
                ));
            }
            lines
        };
        let away = || vec![(EN.backup_kept_not_here.to_string(), Tone::Dim)];
        let mut seeds = Vec::new();
        let mut fps: Vec<faraday_bip::keys::Fingerprint> = Vec::new();
        for f in wallet
            .policy()
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint())
        {
            if !fps.contains(&f) {
                fps.push(f);
            }
        }
        for fp in fps {
            let key = here
                .iter()
                .map(|&k| &self.session.keys[k])
                .find(|k| k.fingerprint == fp);
            seeds.push(KeptSeed {
                name: fp_text(fp),
                lines: key.map_or_else(away, seed_lines),
            });
        }
        Some(Kept {
            wallet: wallet_line,
            seeds,
        })
    }

    /// Costs what `plan_mapped` costs. The cards of the backup's plan, in
    /// order (`docs/BACKUP-FLOW.md` §15.1): the board, the software where
    /// the description goes into software or as files, and The plan.
    pub fn backup_questions(&self, c: &Compute) -> Vec<u8> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        // The board replaces the cards from Places to Where the
        // description goes (`docs/BACKUP-FLOW.md` §17.1); the software and
        // The plan stay. Start from is gone (§19.21 item 1).
        let shape = self.plan_shape_now(b.wallet);
        plan_cards_on(&self.kept.plan_mapped(c, &shape, &b.answers))
    }

    /// Costs what `plan_mapped` costs. The backup's checklist, by
    /// [`bstep`]: only what the plan needs. Works out the plan's shape
    /// (kept for the frame while one is drawn).
    ///
    /// Works out [`crate::drawing::View::backup_items`] from the state as it is.
    pub fn backup_items_now(&self, c: &Compute) -> Vec<u8> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        plan::checklist_on(
            &shape,
            &b.answers,
            &self.kept.plan_mapped(c, &shape, &b.answers),
        )
        .into_iter()
        .map(bstep::of)
        .collect()
    }

    /// Opens checklist item `n`, or closes them all. A seed's copy shows
    /// that seed.
    pub(crate) fn backup_item_open(&mut self, n: Option<u8>) {
        let shape = self.backup.state().map(|b| self.plan_shape_now(b.wallet));
        let key = n
            .and_then(bstep::item)
            .and_then(|it| match it {
                plan::Item::Copy(i) | plan::Item::Passphrase(i) | plan::Item::Codex32(i) => Some(i),
                // A vault's item shows its first seed.
                plan::Item::Vault(v) => {
                    let b = self.backup.state()?;
                    b.answers.vault_seeds(shape.as_ref()?, v).first().copied()
                }
                _ => None,
            })
            .and_then(|i| {
                let w = self.backup.state()?.wallet;
                self.backup_seed_list_now(w).get(i)?.1
            });
        // More options opens closed, unless a file under it is already
        // For the stick.
        let more = n == Some(bstep::PUBLIC)
            && self.backup.state().is_some_and(|b| {
                self.session.wallets.get(b.wallet).is_some_and(|w| {
                    public_more_files(w)
                        .iter()
                        .any(|(_, f)| self.outbox.iter().any(|i| i.name == *f))
                })
            });
        self.backup.open_item(n, more, key);
    }

    /// The public files the plan's software calls for, as
    /// [`Faraday::public_out`] numbers their text: Sparrow the wallet
    /// file, Coldcard, Keystone and Passport the multisig config, Nunchuk
    /// the BSMS record, Bitcoin Core its import, each the descriptor where
    /// the wallet has no such file; "Not sure" the descriptor.
    pub fn backup_public(&self) -> Vec<u8> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        Self::backup_public_for(w, &b.answers)
    }

    /// Costs what `backup_public_names_for_now` costs. The files
    /// [`Faraday::backup_public`] makes, in the form chosen: the text, the
    /// labelled picture, or both; a file with no picture as its text.
    pub(crate) fn backup_public_names(&self, c: &Compute) -> Vec<String> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        self.backup_public_names_for_now(c, b.wallet, &b.answers)
    }

    /// Costs what `copy_checked`, `backup_vault_fits`,
    /// `backup_public_names` and the rest it calls cost. Whether a
    /// checklist item is done, by what it does (it names files and may read
    /// the outbox, so kept for the frame while one is drawn): its file For
    /// the stick or written, its seeds or the wallet in the open vault, the
    /// copy matched, the descriptor shown; the envelopes alone by a press.
    ///
    /// Works out [`crate::drawing::View::backup_item_done`] from the state as it is.
    pub fn backup_item_done_now(&self, c: &Compute, item: plan::Item) -> bool {
        let Some(b) = self.backup.state() else {
            return false;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return false;
        };
        let stem = file_stem(&w.name);
        // For the stick, or written by the last visit, or by the flow.
        let out = |name: &str| {
            self.outbox.iter().any(|i| i.name == name)
                || self.visit.receipt().as_ref().is_some_and(|r| r.wrote(name))
                || self.wallet_wrote(name)
        };
        let seeds = self.backup_seed_list_now(b.wallet);
        let here: Vec<(usize, &faraday_keys::KeyInfo)> = seeds
            .iter()
            .enumerate()
            .filter_map(|(i, (_, k))| Some((i, self.session.keys.info((*k)?)?)))
            .collect();
        match item {
            // Printed from Faraday Starter: no file is made here.
            plan::Item::Templates => b.printed,
            // Every string placed on paper checked, typed back or
            // skipped (`docs/CODEX32.md` §9.4).
            plan::Item::Codex32(i) => {
                let shape = self.plan_shape_now(b.wallet);
                let mapped = self.kept.plan_mapped(c, &shape, &b.answers);
                let paper = plan::codex32::paper_strings(&b.answers, &mapped, i);
                !paper.is_empty() && paper.iter().all(|&j| b.c32_checked.contains(&(i, j)))
            }
            plan::Item::Copy(i) => here
                .iter()
                .find(|(j, _)| *j == i)
                .is_some_and(|(_, k)| self.copy_checked(c, b, i, k)),
            plan::Item::Passphrase(i) => {
                here.iter().find(|(j, _)| *j == i).is_some_and(|(_, k)| {
                    b.checked
                        .contains(&(k.fingerprint.0, plan::CopyForm::Passphrase))
                })
            }
            plan::Item::Vault(v) => self.backup_vault_fits(c, v).is_some(),
            plan::Item::SeedFiles => {
                !here.is_empty() && here.iter().all(|(_, k)| self.seed_file_made(&stem, k))
            }
            plan::Item::Sheets => {
                if b.answers.split && splits(w) {
                    let (_, n) = Session::quorum(w);
                    out(&format!("{stem}-share-1-of-{n}.pdf"))
                } else {
                    out(&format!("{stem}-backup.pdf"))
                }
            }
            plan::Item::PublicFiles => self.backup_public_names(c).iter().all(|n| out(n)),
            plan::Item::ShowDescriptor => b.shown,
            plan::Item::Envelopes => b.envelopes,
            // Every stick of the plan written, inside the flow.
            plan::Item::Sticks => self.wallet_sticks_done(c),
        }
    }

    /// The fingerprints of the seeds the plan puts into vault `v`, and of
    /// those it puts into another vault and not this one.
    pub(crate) fn backup_vault_keys(
        &self,
        v: usize,
    ) -> (
        Vec<faraday_bip::keys::Fingerprint>,
        Vec<faraday_bip::keys::Fingerprint>,
    ) {
        let Some(b) = self.backup.state() else {
            return (Vec::new(), Vec::new());
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        let fp = |i: usize| {
            let k = list.get(i)?.1?;
            Some(self.session.keys.info(k)?.fingerprint)
        };
        let own = b.answers.vault_seeds(&shape, v);
        let mut others: Vec<usize> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter(|&u| u != v)
            .flat_map(|u| b.answers.vault_seeds(&shape, u))
            .filter(|i| !own.contains(i))
            .collect();
        others.sort_unstable();
        others.dedup();
        (
            own.into_iter().filter_map(fp).collect(),
            others.into_iter().filter_map(fp).collect(),
        )
    }

    /// The session keys of the seeds the plan puts into vault `v`, each
    /// with whether its passphrase goes in with it, and those of the seeds
    /// it puts into another vault and not this one.
    pub(crate) fn backup_vault_seeds(&self, v: usize) -> (Vec<(usize, bool)>, Vec<usize>) {
        let Some(b) = self.backup.state() else {
            return (Vec::new(), Vec::new());
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        let own = b.answers.vault_seeds(&shape, v);
        let mut others: Vec<usize> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter(|&u| u != v)
            .flat_map(|u| b.answers.vault_seeds(&shape, u))
            .filter(|i| !own.contains(i))
            .filter_map(|i| list.get(i)?.1)
            .collect();
        others.sort_unstable();
        others.dedup();
        let own = own
            .into_iter()
            .filter_map(|i| {
                let k = list.get(i)?.1?;
                let with = shape.seeds[i].passphrase && b.answers.pass_in_vault(i);
                Some((k, with))
            })
            .collect();
        (own, others)
    }

    /// Costs what `backup_vault_shares` and `backup_vault_passphrases`
    /// cost. Whether the plan puts anything in vault `v`.
    pub(crate) fn backup_vault_wanted(&self, c: &Compute, v: usize) -> Option<()> {
        let b = self.backup.state()?;
        self.session.wallets.get(b.wallet)?;
        let (own_seeds, _) = self.backup_vault_seeds(v);
        let shares = !self.backup_vault_shares(c, v).is_empty()
            || !self.backup_vault_strings(c, v).is_empty();
        let passphrases = !self.backup_vault_passphrases(c, v).is_empty();
        (!own_seeds.is_empty() || b.answers.wallet_in_vault(v) || shares || passphrases)
            .then_some(())
    }

    /// Costs what `plan_mapped` costs. The session keys whose passphrase
    /// the plan keeps alone in vault `v`, as a passphrase record
    /// (`docs/BACKUP-FLOW.md` §17.12 item 4): a key with a passphrase only.
    pub(crate) fn backup_vault_passphrases(&self, c: &Compute, v: usize) -> Vec<usize> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let list = self.backup_seed_list_now(b.wallet);
        self.kept
            .plan_mapped(c, &shape, &b.answers)
            .vault_passphrases(v)
            .into_iter()
            .filter_map(|i| list.get(i)?.1)
            .filter(|&k| {
                self.session
                    .keys
                    .info(k)
                    .is_some_and(|key| key.has_passphrase)
            })
            .collect()
    }

    /// Costs what `plan_mapped` costs. The shares the plan keeps in vault
    /// `v`, each with its text and how many shares there are
    /// (`docs/BACKUP-FLOW.md` §15.6).
    pub(crate) fn backup_vault_shares(&self, c: &Compute, v: usize) -> Vec<(usize, String, usize)> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(b.wallet);
        let (m, n) = Session::quorum(w);
        let plan = split_plan(n, m, b.answers.omit);
        self.kept
            .plan_mapped(c, &shape, &b.answers)
            .vault_shares(v)
            .into_iter()
            .filter_map(|j| Some((j, split_share(w, plan.get(j)?)?, plan.len())))
            .collect()
    }

    /// Costs what `backup_vault_wanted`, `vault_holds_seed`,
    /// `vault_has_wallet` and the rest it calls cost. Whether open vault
    /// `o` holds what the plan puts in vault `v`, no seed only another
    /// vault is to hold, and no codex32 string of the plan's seeds the plan
    /// does not keep there (of another set, or at another index).
    pub(crate) fn backup_open_fits(&self, c: &Compute, v: usize, o: usize) -> bool {
        use crate::vault::screen::vault_has_wallet;
        let Some(b) = self.backup.state() else {
            return false;
        };
        let Some(w) = self.session.wallets.get(b.wallet) else {
            return false;
        };
        if self.backup_vault_wanted(c, v).is_none() {
            return false;
        }
        let (own_seeds, other_seeds) = self.backup_vault_seeds(v);
        let wallet_too = b.answers.wallet_in_vault(v);
        own_seeds
            .iter()
            .all(|&(k, with)| self.vault_holds_seed(c, o, k, with))
            && !other_seeds
                .iter()
                .any(|&k| self.vault_holds_seed(c, o, k, false))
            && (!wallet_too || vault_has_wallet(c, self, o, w))
            && self
                .backup_vault_shares(c, v)
                .iter()
                .all(|(j, text, _)| self.vault_holds_share(c, o, w.checksum(), *j, text))
            && self
                .backup_vault_passphrases(c, v)
                .iter()
                .all(|&k| self.vault_holds_passphrase(o, k))
            && self
                .backup_vault_strings(c, v)
                .iter()
                .all(|&(k, index, _)| self.vault_holds_string(o, k, index))
            && self.backup_vault_c32_strays(c, o, v).is_empty()
    }

    /// Costs what `backup_vault_shares`, `backup_vault_passphrases` and
    /// `backup_vault_open` cost. The vault that does the plan's vault `v`:
    /// an open vault, or one locked since this power-on by what it was seen
    /// to hold, that holds every seed the plan puts into `v` (with its
    /// passphrase where the plan puts it there), the wallet where the plan
    /// puts it there, and no seed only another vault is to hold. Its name.
    pub(crate) fn backup_vault_fits(&self, c: &Compute, v: usize) -> Option<String> {
        let b = self.backup.state()?;
        let w = self.session.wallets.get(b.wallet)?;
        let wallet_too = b.answers.wallet_in_vault(v);
        let (own_seeds, _) = self.backup_vault_seeds(v);
        let shares = !self.backup_vault_shares(c, v).is_empty()
            || !self.backup_vault_strings(c, v).is_empty();
        let passphrases = !self.backup_vault_passphrases(c, v).is_empty();
        if own_seeds.is_empty() && !wallet_too && !shares && !passphrases {
            return None;
        }
        if let Some(o) = self.backup_vault_open(c, v) {
            return Some(self.vaults.open[o].name.clone());
        }
        // A locked vault's summary lists no shares and no passphrases: a
        // vault that is to hold one is done only while it is open.
        if shares || passphrases {
            return None;
        }
        let (own, others) = self.backup_vault_keys(v);
        let sum = w.checksum().to_string();
        // Each seed told by its whole master id, the session key's else
        // the backup record's (F65); one that cannot be told is not done.
        let record = self.backups_area.record(&sum);
        let has = |s: &crate::vault::VaultSummary, f: &faraday_bip::keys::Fingerprint| {
            crate::backups::summary_has_seed(s, f.0, self.seed_id(record, f.0))
        };
        let salts: Vec<[u8; 32]> = self.vaults.open.iter().map(|o| o.header().salt).collect();
        self.vaults
            .summaries
            .iter()
            .filter(|s| !salts.contains(&s.salt))
            .find(|s| {
                own.iter().all(|f| has(s, f) == Some(true))
                    && others.iter().all(|f| has(s, f) == Some(false))
                    && (!wallet_too || s.sums.contains(&sum))
            })
            .map(|s| s.file.clone())
    }

    /// What place `p` is called on screen: its name, kept in the vault, or
    /// "Place 1", "Place 2".
    pub fn place_name(&self, p: usize) -> String {
        self.backup
            .state()
            .and_then(|b| b.names.get(p))
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .map_or_else(
                || fill(EN.place_name_place, &[&(p + 1).to_string()]),
                str::to_string,
            )
    }

    pub(crate) fn backup_out(&mut self, what: u8) {
        let Some(wallet) = self.backup.state().map(|b| b.wallet) else {
            return;
        };
        self.public_out(wallet, what);
    }

    /// The backup under way's wallet, when its checklist is showing and
    /// file `what` is For the stick: that file is made again when what it
    /// depends on changes. The old one goes now.
    pub(crate) fn backup_remake(&mut self, what: u8) -> Option<usize> {
        let b = self.backup.state()?;
        if b.stage != BStage::Checklist {
            return None;
        }
        let w = b.wallet;
        let made = self.public_names(w, what);
        if !self.outbox.iter().any(|i| made.contains(&i.name)) {
            return None;
        }
        self.public_remove(w, what);
        Some(w)
    }

    /// Costs what `public_files` costs, and for the multisig config (2)
    /// or the split sheets (4) what `restore::rebuilds` costs per set
    /// tried. The files [`Faraday::public_out`] makes, with their bytes;
    /// the config or the sheets only where they rebuild the wallet, and
    /// otherwise none, which a toast says.
    pub(crate) fn public_made(&mut self, wallet: usize, what: u8) -> Vec<(String, Vec<u8>)> {
        let c = self.compute();
        let net = self.session.network();
        let omit = self.backup.state().map(|b| b.answers.omit);
        let refused = self.session.wallets.get(wallet).and_then(|w| match what {
            2 if multisig_config(w, None).is_some()
                && crate::restore::checked_config(&c, net, w).is_none() =>
            {
                Some(EN.backup_config_rebuilds_another)
            }
            4 if splits(w) => {
                let (m, n) = Session::quorum(w);
                let plan = split_plan(n, m, omit.unwrap_or(m.saturating_sub(1)));
                (!crate::restore::sheets_rebuild(&c, net, w, &plan))
                    .then_some(EN.backup_sheets_rebuild_another)
            }
            _ => None,
        });
        if let Some(why) = refused {
            self.toast(why);
            return Vec::new();
        }
        self.public_files(wallet, what, true)
    }

    /// Making the checklist makes every public file it calls for, For
    /// the stick: the blank template, the sheet or the shares, and the
    /// files for the software chosen. Made again, each replaces its
    /// namesake.
    pub(crate) fn backup_files_make(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let w = b.wallet;
        let (qr, text) = (
            b.answers.form[plan::form::QR],
            b.answers.form[plan::form::TEXT],
        );
        let split = self
            .session
            .wallets
            .get(w)
            .is_some_and(|wl| b.answers.split && splits(wl));
        let items: Vec<plan::Item> = self
            .backup_items_now(&c)
            .into_iter()
            .filter_map(bstep::item)
            .collect();
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for item in items {
            match item {
                plan::Item::Sheets => files.extend(self.public_made(w, if split { 4 } else { 3 })),
                plan::Item::PublicFiles => {
                    // In the form chosen, as `backup_public_names` lists
                    // them: the picture, the text, or both.
                    for what in self.backup_public() {
                        let picture: Vec<(String, Vec<u8>)> = match what {
                            _ if !qr => Vec::new(),
                            1 => self.public_made(w, 8),
                            2 | 6 => {
                                let code = if what == 2 {
                                    Code::MultisigConfig(w)
                                } else {
                                    Code::Bsms(w)
                                };
                                self.code_view(&c, code)
                                    .ok()
                                    .and_then(|v| v.png(&c))
                                    .into_iter()
                                    .collect()
                            }
                            _ => Vec::new(),
                        };
                        if picture.is_empty() || text {
                            files.extend(self.public_made(w, what));
                        }
                        files.extend(picture);
                    }
                    // Each key's account key files go with them.
                    files.extend(self.account_key_files(&c, w));
                }
                _ => {}
            }
        }
        let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
        for (name, bytes) in files {
            self.put_outbox(&name, bytes);
        }
        self.backup_sent(w, &names);
        self.toast_out_files(&names);
    }

    /// Files of wallet `wallet` just put in the Outbox, for the summary
    /// of its backup when one is under way.
    pub(crate) fn backup_sent(&mut self, wallet: usize, names: &[String]) {
        self.backup.sent(wallet, names);
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
