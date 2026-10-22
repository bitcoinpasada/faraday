//! The wallet at a glance (`docs/history/NEW-WALLET.md` §6): one chart of a
//! loaded wallet, its keys and its backup, on the wallet's card.
//!
//! [`of`] gathers what the chart shows from the wallet, the session's keys
//! and the backup's plan: the plan of the backup under way once its
//! checklist is made, else the one this power-on recorded, else the one an
//! open vault keeps. [`draw`] lays it out as a structure chart
//! (`docs/DESIGN.md` §4.16): three rows of nodes, wrapped where the card
//! is too narrow, joined by orthogonal lines that run down, across and
//! down through the bands and gaps between nodes and never through one.
//! [`draw_column`] is the small panel's page: the same nodes in one
//! column, with no lines.
//!
//! [`of_vault`] charts a wallet an open vault holds from that vault's own
//! plan record, for the vault's view (§7.2), which draws it the other way
//! up: the backup on top, through the keys, to the wallet at the foot.

pub mod sheet;

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_draw::{Color, tokens};

use crate::drawing::View;
use crate::plan::{self, At, Tag, What};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::{Session, fp_text};
use crate::wallet_backup::Tone;
use crate::{Action, Faraday};
use faraday_keys::vault::{RecordView, VaultId};

/// What the chart, its sheet and the checklist's titles read of the app
/// that a frame keeps: through [`View`]'s memo and kept accessors while a
/// frame is drawn, and worked out from the state as it is by [`Now`] for
/// a decision (`docs/COMPUTE.md` §4). Each says what it reads.
pub(crate) trait Reads {
    /// The app, read only.
    fn app(&self) -> &Faraday;
    /// What loaded wallet `w`'s plan is for ([`Faraday::plan_shape_now`]).
    fn plan_shape(&self, w: usize) -> plan::Shape;
    /// The keys here that are held ([`Faraday::held_keys_now`]).
    fn held_keys(&self) -> Vec<faraday_bip::keys::Fingerprint>;
    /// Every wallet known and where its backup is
    /// ([`Faraday::backups_now`]).
    fn backups(&self) -> Vec<crate::backups::Entry>;
    /// Whether open vaults keep a seed without a passphrase
    /// ([`Faraday::vault_passphrase_seeds_now`]).
    fn passphrase_seeds(&self) -> bool;
    /// Where the seed of wallet slot `slot` is in the open vaults
    /// ([`Faraday::vault_key_for`]).
    fn vault_key_for(&self, slot: &crate::wallet::Slot) -> Option<(usize, usize)>;
    /// The plan an open vault keeps for `wallet` ([`Faraday::plan_load_of`]).
    fn plan_load_of(
        &self,
        wallet: &crate::wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)>;
    /// The plan open vault `v` keeps for `wallet` ([`Faraday::plan_in_vault`]).
    fn plan_in_vault(
        &self,
        v: usize,
        wallet: &crate::wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)>;
    /// The notes the vaults keep on `wallet` ([`Faraday::plan_notes`]).
    fn plan_notes(&self, wallet: &crate::wallet::Wallet, v: Option<usize>) -> Vec<plan::Note>;
    /// A vault key record's fingerprint
    /// ([`crate::vault::screen::key_fingerprint`]).
    fn key_fingerprint(&self, r: RecordView<'_>) -> Option<String>;
    /// The wallet description record `r` of open vault `id` holds
    /// ([`crate::vault::Vaults::described`]).
    fn described(&self, id: VaultId, r: usize) -> Option<&crate::vault::Described>;
    /// A plan's map ([`Faraday::plan_mapped`]).
    fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> std::rc::Rc<plan::Mapped>;
    /// A plan's check on its map ([`plan::Mapped::check`]).
    fn plan_check(&self, shape: &plan::Shape, a: &plan::Answers) -> plan::Check;
    /// Whether a checklist item is done ([`Faraday::backup_item_done_now`]).
    fn backup_item_done(&self, item: plan::Item) -> bool;
    /// The public files of loaded wallet `wi` under answers `a`
    /// ([`Faraday::backup_public_names_for_now`]).
    fn backup_public_names_for(&self, wi: usize, a: &plan::Answers) -> Vec<String>;
    /// The open vault the backup's vault `v` is saved in
    /// ([`Faraday::backup_vault_fits`]).
    fn backup_vault_fits(&self, v: usize) -> Option<String>;
    /// The sticks the wallet being backed up is written to
    /// ([`Faraday::wallet_stick_rows_now`]).
    fn wallet_stick_rows(&self) -> Vec<crate::drawing::StickRow>;
    /// The backup's checklist ([`Faraday::backup_items_now`]).
    fn backup_items(&self) -> Vec<u8>;
    /// Wallet `w`'s seeds ([`Faraday::backup_seed_list_now`]).
    fn backup_seed_list(&self, w: usize) -> Vec<(String, Option<usize>)>;
    /// An address of `wl` as the screen shows it
    /// ([`crate::wallet::Session::address_shown`]), kept on the wallet.
    fn address_shown(&self, wl: &crate::wallet::Wallet, change: bool, index: u32) -> String;
    /// The Inbox PSBT loaded wallet `w` spends from ([`Faraday::psbt_for`]).
    fn psbt_for(&self, w: usize) -> Option<usize>;
}

/// The app as a decision reads it: every answer worked out from the
/// state as it is, with the caller's token ([`Reads`]).
pub(crate) struct Now<'a> {
    /// The app.
    pub app: &'a Faraday,
    /// The caller's token.
    pub c: &'a Compute,
}

impl Reads for Now<'_> {
    fn app(&self) -> &Faraday {
        self.app
    }
    fn plan_shape(&self, w: usize) -> plan::Shape {
        self.app.plan_shape_now(w)
    }
    fn held_keys(&self) -> Vec<faraday_bip::keys::Fingerprint> {
        self.app.held_keys_now(self.c)
    }
    fn backups(&self) -> Vec<crate::backups::Entry> {
        self.app.backups_now(self.c)
    }
    fn passphrase_seeds(&self) -> bool {
        !self.app.vault_passphrase_seeds_now(self.c).is_empty()
    }
    fn vault_key_for(&self, slot: &crate::wallet::Slot) -> Option<(usize, usize)> {
        self.app.vault_key_for(self.c, slot)
    }
    fn plan_load_of(
        &self,
        wallet: &crate::wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        self.app.plan_load_of(self.c, wallet, shape)
    }
    fn plan_in_vault(
        &self,
        v: usize,
        wallet: &crate::wallet::Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        self.app.plan_in_vault(self.c, v, wallet, shape)
    }
    fn plan_notes(&self, wallet: &crate::wallet::Wallet, v: Option<usize>) -> Vec<plan::Note> {
        self.app.plan_notes(self.c, wallet, v)
    }
    fn key_fingerprint(&self, r: RecordView<'_>) -> Option<String> {
        crate::vault::screen::key_fingerprint(self.c, self.app, r)
    }
    fn described(&self, id: VaultId, r: usize) -> Option<&crate::vault::Described> {
        self.app.vaults.described(self.c, id, r)
    }
    fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> std::rc::Rc<plan::Mapped> {
        self.app.plan_mapped(self.c, shape, a)
    }
    fn plan_check(&self, shape: &plan::Shape, a: &plan::Answers) -> plan::Check {
        self.app
            .plan_mapped(self.c, shape, a)
            .check(self.c, shape, a)
    }
    fn backup_item_done(&self, item: plan::Item) -> bool {
        self.app.backup_item_done_now(self.c, item)
    }
    fn backup_public_names_for(&self, wi: usize, a: &plan::Answers) -> Vec<String> {
        self.app.backup_public_names_for_now(self.c, wi, a)
    }
    fn backup_vault_fits(&self, v: usize) -> Option<String> {
        self.app.backup_vault_fits(self.c, v)
    }
    fn wallet_stick_rows(&self) -> Vec<crate::drawing::StickRow> {
        self.app.wallet_stick_rows_now(self.c)
    }
    fn backup_items(&self) -> Vec<u8> {
        self.app.backup_items_now(self.c)
    }
    fn backup_seed_list(&self, w: usize) -> Vec<(String, Option<usize>)> {
        self.app.backup_seed_list_now(w)
    }
    fn address_shown(&self, wl: &crate::wallet::Wallet, change: bool, index: u32) -> String {
        self.app.session.address_shown(self.c, wl, change, index)
    }
    fn psbt_for(&self, w: usize) -> Option<usize> {
        self.app.psbt_for(self.c, w)
    }
}

/// The chart of one wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glance {
    /// The wallet: its name.
    pub name: String,
    /// "2 of 3 · native SegWit".
    pub shape: String,
    /// The descriptor's checksum.
    pub checksum: String,
    /// Its keys, numbered as the wallet's keys are.
    pub keys: Vec<KeyNode>,
    /// Its backup.
    pub backup: Backup,
    /// Where the chart is drawn, which says what its presses do.
    pub press: Press,
    /// A threshold wallet, whose keys are shares and not account keys.
    pub threshold: bool,
    /// The key whose lines are drawn bold, the rest dimmed: **Where it
    /// is** on its sheet (`docs/history/NEW-WALLET.md` §9.3).
    pub focus: Option<usize>,
    /// A key of it made here waits for its backup: the wallet node says
    /// "Back up before you receive" (`docs/history/NEW-WALLET.md` §14.3).
    pub held: bool,
    /// A vault is open to keep names, dates and marks in (§9.5).
    pub keep: bool,
    /// The plan charted, with the shape it is read against.
    pub plan: Option<(plan::Shape, plan::Answers)>,
}

/// What every address of a wallet with a key made here and waiting for
/// its backup carries, in `WARN` (`docs/history/NEW-WALLET.md` §14.3).
pub const RECEIVE_WARNING: &str = EN.glance_back_up_before;

/// A key node's state while its key, made here, waits for its backup.
pub const WAITING: &str = EN.glance_waiting_for;

/// Where a chart is drawn, and so what its sheets' actions act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// On the card of loaded wallet n.
    Loaded(usize),
    /// In open vault v, for its wallet record r: each action loads the
    /// wallet, and its seeds in the vault, first.
    Vault(usize, usize),
}

/// One key of the wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyNode {
    /// Its number in the wallet, from one.
    pub number: usize,
    /// Its fingerprint, or "no origin".
    pub fingerprint: String,
    /// Its label, when it has one other than its fingerprint.
    pub label: Option<String>,
    /// Where it is: "Can sign here", "Cosigner · xpub only".
    pub state: String,
    /// Held here: its lines go to whatever holds its seed.
    pub here: bool,
    /// Loaded in this session, where it can sign.
    pub held: bool,
    /// Its master fingerprint, when the wallet names one.
    pub fp: Option<[u8; 4]>,
    /// Its seed among the plan's, by place.
    pub seed: Option<usize>,
    /// For a key the quorum still needs that an open vault holds, its way
    /// in: a press on that line.
    pub action: Option<Action>,
    /// That way in, by name: "Load from vault".
    pub way_in: Option<&'static str>,
    /// Loaded, or kept, with a BIP-39 passphrase.
    pub passphrase: bool,
    /// Who holds it, for a key not held here, as the vault keeps it.
    pub holder: Option<String>,
    /// When its holder last confirmed its backup, as the vault keeps it.
    pub checked: Option<String>,
}

/// The wallet's backup, as the chart's third row shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backup {
    /// A plan is known: a node per spot of its map, and its check.
    Plan {
        /// The nodes, in the map's order.
        nodes: Vec<BackupNode>,
        /// The plan's check.
        check: plan::Check,
        /// The plan's checklist: the flows that make each thing.
        items: Vec<plan::Item>,
        /// The map was edited on the chart, and is the plan (§9.7).
        edited: bool,
    },
    /// The plan is in a locked vault and not remembered: its name, and
    /// the press that unlocks it.
    Locked {
        /// The vault's name.
        vault: String,
        /// Unlock it, and come back.
        unlock: Action,
    },
    /// No plan is known.
    None,
}

/// One spot of the backup's map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupNode {
    /// Where.
    pub at: At,
    /// "Place 1", or the place's name; "Vault 1".
    pub name: String,
    /// The most exposed of what it holds.
    pub tag: Tag,
    /// What it holds: "Key 1 words", "Wallet sheet", "Vault 2 stick".
    pub holds: Vec<String>,
    /// What each of [`BackupNode::holds`] is, on the plan's map; empty
    /// for a node that holds nothing.
    pub whats: Vec<What>,
    /// What it holds, naming the keys a vault's stick opens: "Key 1
    /// words · Key 2 in the vault", for the small panel.
    pub keys_held: String,
    /// For a vault: where its stick is kept.
    pub stick: Option<String>,
    /// What this device saw of it, as the Backups screen says it.
    pub state: Option<(String, Tone)>,
    /// What it gives whoever finds it alone; a vault reads "Sealed".
    pub alone: Option<String>,
    /// What [`BackupNode::alone`] reads as, for the chart's colour
    /// ([`alone_color`]), set beside the text where it is produced so the
    /// colour reads this, not the words. `Some` exactly when `alone` is.
    pub alone_kind: Option<plan::AloneKind>,
    /// The keys whose seed it holds, by index into [`Glance::keys`]: a
    /// line is drawn to it from each of them held here.
    pub keys: Vec<usize>,
    /// Each of [`BackupNode::whats`] marked lost or exposed, as the vault
    /// keeps it (§9.6).
    pub marks: Vec<Option<plan::Mark>>,
    /// When each of [`BackupNode::whats`] was last checked here, as the
    /// vault keeps it.
    pub checked: Vec<Option<String>>,
}

/// The plan the chart draws for loaded wallet `w`, with the places'
/// names: the backup under way once its checklist is made, else this
/// power-on's record of it, else an open vault's. Drawing's form, through
/// the memo and the kept values.
pub fn plan_of(v: &View, w: usize) -> Option<(plan::Answers, Vec<String>)> {
    plan_on(v, w)
}

/// Costs what an open vault's plan records cost to read the first time:
/// [`plan_of`] for a decision, worked out from the state as it is.
pub fn plan_of_now(c: &Compute, app: &Faraday, w: usize) -> Option<(plan::Answers, Vec<String>)> {
    plan_on(&Now { app, c }, w)
}

/// [`plan_of`] through `r`.
fn plan_on(r: &impl Reads, w: usize) -> Option<(plan::Answers, Vec<String>)> {
    let app = r.app();
    let wallet = app.session.wallets.get(w)?;
    let shape = r.plan_shape(w);
    if let Some(b) = app.backup.as_ref()
        && b.wallet == w
        && b.stage == crate::BStage::Checklist
    {
        return Some((b.answers.clone(), b.names.clone()));
    }
    let kept = r.plan_load_of(wallet, &shape);
    let sum = wallet.checksum();
    let recorded = app
        .backup_records
        .iter()
        .find(|r| r.sum == sum && !r.answers.is_empty())
        .and_then(|r| plan::Answers::from_text(&shape, &r.answers));
    match (recorded, kept) {
        (Some(a), kept) => Some((a, kept.map(|(_, n)| n).unwrap_or_default())),
        (None, kept) => kept,
    }
}

/// The chart of loaded wallet `w`; `None` for a silent payments wallet,
/// which has no quorum or plan to draw. Drawing's form, through the memo
/// and the kept values.
pub fn of(v: &View, w: usize) -> Option<Glance> {
    of_on(v, w)
}

/// Costs what the open vaults' records and the plan's map cost to read
/// the first time: [`of`] for a decision, worked out from the state as it
/// is.
pub fn of_now(c: &Compute, app: &Faraday, w: usize) -> Option<Glance> {
    of_on(&Now { app, c }, w)
}

/// [`of`] through `r`.
pub(crate) fn of_on(r: &impl Reads, w: usize) -> Option<Glance> {
    let app = r.app();
    let wallet = app.session.wallets.get(w)?;
    let plan = plan_on(r, w);
    let none = locked(app, wallet.checksum());
    let notes = r.plan_notes(wallet, None);
    build(
        r,
        wallet,
        r.plan_shape(w),
        &[],
        plan,
        &notes,
        none,
        Press::Loaded(w),
    )
}

/// The chart of the wallet that record `r` of open vault `v` holds, from
/// that vault's plan for it, whether or not the wallet is loaded: a seed
/// the vault keeps counts as here. `None` for a record that is not a
/// wallet, or a silent payments wallet. Drawing's form, through the memo
/// and the kept values.
pub fn of_vault(view: &View, v: usize, r: usize) -> Option<Glance> {
    of_vault_on(view, v, r)
}

/// Costs what the vault's records and the plan's map cost to read the
/// first time: [`of_vault`] for a decision, worked out from the state as
/// it is.
pub fn of_vault_now(c: &Compute, app: &Faraday, v: usize, r: usize) -> Option<Glance> {
    of_vault_on(&Now { app, c }, v, r)
}

/// [`of_vault`] through `reads`.
pub(crate) fn of_vault_on(reads: &impl Reads, v: usize, r: usize) -> Option<Glance> {
    use faraday_vault::records::{field, kind};
    let app = reads.app();
    let open = app.vaults.open.get(v)?;
    let rec = app
        .vaults
        .records(open.id)
        .get(r)
        .filter(|x| x.kind() == kind::WALLET)?;
    let wallet = crate::wallet::Wallet::new(
        rec.public_text(field::WALLET_NAME)
            .unwrap_or(EN.glance_wallet)
            .to_string(),
        reads.described(open.id, r)?.policy.clone(),
        String::new(),
        crate::wallet::Origin::Named,
    );
    // The seeds this vault keeps: their fingerprints, and whether each
    // was kept with a passphrase.
    let kept: Vec<(String, bool)> = app
        .vaults
        .records(open.id)
        .of(kind::KEY)
        .filter_map(|(_, k)| {
            let fp = reads.key_fingerprint(k)?;
            Some((fp, k.has(field::KEY_PASSPHRASE)))
        })
        .collect();
    let in_vault: Vec<String> = kept.iter().map(|(f, _)| f.clone()).collect();
    let mut shape = app.plan_shape_of(&wallet);
    let slots = app.session.slots(&wallet);
    let threshold = wallet.policy().record().is_some();
    for (n, slot) in slots.iter().enumerate() {
        let Some(fp) = slot.fingerprint.map(fp_text) else {
            continue;
        };
        let Some((_, pass)) = kept.iter().find(|(f, _)| *f == fp) else {
            continue;
        };
        let i = if threshold {
            (n < shape.seeds.len()).then_some(n)
        } else {
            shape.seeds.iter().position(|s| s.name == fp)
        };
        if let Some(i) = i {
            shape.seeds[i].here = true;
            shape.seeds[i].passphrase |= *pass;
        }
    }
    let plan = reads.plan_in_vault(v, &wallet, &shape);
    let notes = reads.plan_notes(&wallet, Some(v));
    build(
        reads,
        &wallet,
        shape,
        &in_vault,
        plan,
        &notes,
        Backup::None,
        Press::Vault(v, r),
    )
}

/// The chart of `wallet`, read against plan shape `shape`, with the plan
/// and place names `plan` (else `none` as its backup row) and the notes
/// the vault keeps on it; `in_vault` names the seeds the open vault
/// drawing it keeps, which count as here.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn build(
    r: &impl Reads,
    wallet: &crate::wallet::Wallet,
    shape: plan::Shape,
    in_vault: &[String],
    plan: Option<(plan::Answers, Vec<String>)>,
    notes: &[plan::Note],
    none: Backup,
    press: Press,
) -> Option<Glance> {
    if wallet.policy().silent().is_some() {
        return None;
    }
    let app = r.app();
    let slots = app.session.slots(wallet);
    let kept = |slot: &crate::wallet::Slot| {
        slot.fingerprint
            .is_some_and(|f| in_vault.contains(&fp_text(f)))
    };
    let any_here = slots.iter().any(|s| s.held_by.is_some() || kept(s));
    let quorum_here =
        slots.iter().filter(|s| s.held_by.is_some()).count() >= crate::wallet::needed(wallet);
    let may = app.may_load_keys();
    // Asked once for the chart (F62).
    let loaded_w = match press {
        Press::Loaded(w) => Some(w),
        _ => None,
    };
    let passphrase_key = slots
        .iter()
        .any(|s| s.held_by.is_none() && s.fingerprint.is_some())
        && r.passphrase_seeds();
    let threshold = wallet.policy().record().is_some();
    // Each slot's seed in the plan: a share by its number, else by
    // fingerprint.
    let seed_of = |n: usize, slot: &crate::wallet::Slot| -> Option<usize> {
        if threshold {
            return (n < shape.seeds.len()).then_some(n);
        }
        let fp = fp_text(slot.fingerprint?);
        shape.seeds.iter().position(|s| s.name == fp)
    };
    let slot_of: Vec<Option<usize>> = (0..shape.seeds.len())
        .map(|i| (0..slots.len()).find(|&n| seed_of(n, &slots[n]) == Some(i)))
        .collect();
    let held_keys = r.held_keys();
    let keys: Vec<KeyNode> = slots
        .iter()
        .enumerate()
        .map(|(n, slot)| {
            let fingerprint = slot
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| EN.glance_no_origin.to_string());
            let passphrase = seed_of(n, slot).is_some_and(|i| shape.seeds[i].passphrase);
            let held = slot.held_by.is_some();
            // Made here, its backup not done (`docs/history/NEW-WALLET.md` §14.3).
            let waits = held && slot.fingerprint.is_some_and(|f| held_keys.contains(&f));
            let state = match (held, kept(slot), passphrase, any_here) {
                _ if waits => WAITING,
                (true, _, true, _) => EN.glance_can_sign_passphrase,
                (true, _, false, _) => EN.glance_can_sign_here,
                (false, true, true, _) => EN.glance_in_this_vault_passphrase,
                (false, true, false, _) => EN.glance_in_this_vault,
                // Short of keys, one not here says so: the one Scan a key
                // is the signatures bar's (`docs/history/SCAN-TO-SIGN.md` §2.3).
                (false, false, _, _) if !quorum_here && matches!(press, Press::Loaded(_)) => {
                    EN.glance_not_here
                }
                (false, false, _, true) => EN.glance_cosigner_xpub,
                (false, false, _, false) => EN.glance_on_its_own_device,
            };
            // A key an open vault holds loads from it, while short.
            let (action, way_in) = match slot
                .fingerprint
                .filter(|_| !held && !quorum_here && r.vault_key_for(slot).is_some())
            {
                Some(f) => (
                    Some(Action::Vault(crate::vault::VaultAction::LoadKeyOf(
                        f.0,
                        match press {
                            Press::Loaded(w) => crate::vault::SlotOf::Loaded(w, n),
                            Press::Vault(v, r) => crate::vault::SlotOf::Vault(v, r, n),
                        },
                    ))),
                    Some(EN.glance_load_from_vault),
                ),
                // A seed an open vault keeps without the passphrase this
                // wallet may use it with (F62), short or not.
                None if !held && passphrase_key && loaded_w.is_some() => (
                    loaded_w.map(|w| Action::Vault(crate::vault::VaultAction::PassphraseKey(w))),
                    Some(EN.vault_load_with_a_passphrase),
                ),
                None => (None, None),
            };
            KeyNode {
                number: n + 1,
                label: slot
                    .held_by
                    .as_ref()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case(&fingerprint)),
                fingerprint,
                state: state.to_string(),
                here: held || kept(slot),
                held,
                fp: slot.fingerprint.map(|f| f.0),
                seed: seed_of(n, slot),
                action: action.filter(|_| may),
                way_in,
                passphrase,
                holder: slot.fingerprint.and_then(|f| {
                    notes.iter().find_map(|n| match n {
                        plan::Note::Holder(fp, name) if *fp == f.0 => Some(name.clone()),
                        _ => None,
                    })
                }),
                checked: slot.fingerprint.and_then(|f| {
                    notes.iter().find_map(|n| match n {
                        plan::Note::KeyChecked(fp, d) if *fp == f.0 => Some(d.clone()),
                        _ => None,
                    })
                }),
            }
        })
        .collect();
    let sum = wallet.checksum().to_string();
    let charted = plan.as_ref().map(|(a, _)| (shape.clone(), a.clone()));
    let backup = match plan {
        Some((a, names)) => backup_nodes(
            r, &sum, &shape, &a, &names, notes, &slot_of, &keys, any_here,
        ),
        None => none,
    };
    let focus = app.chart_focus.filter(|(p, _)| *p == press).map(|(_, k)| k);
    let held = slots
        .iter()
        .any(|s| s.held_by.is_some() && s.fingerprint.is_some_and(|f| held_keys.contains(&f)));
    Some(Glance {
        name: wallet.name.clone(),
        shape: Session::shape(wallet),
        checksum: sum,
        keys,
        backup,
        press,
        threshold,
        focus,
        held,
        keep: matches!(press, Press::Vault(..))
            || app.vaults.open.get(app.vaults.current).is_some(),
        plan: charted,
    })
}

/// A plan kept in a locked vault seen this power-on, else no plan.
fn locked(app: &Faraday, sum: &str) -> Backup {
    let Some(s) = app
        .vaults
        .summaries
        .iter()
        .find(|s| s.planned.iter().any(|c| c == sum))
    else {
        return Backup::None;
    };
    let back = crate::Screen::Wallets;
    let unlock = app
        .vault_files()
        .iter()
        .position(|f| f.open.is_none() && f.header.salt == s.salt)
        .map_or(crate::vault::VaultAction::ListFrom(back), |k| {
            crate::vault::VaultAction::OpenFrom(k, back)
        });
    Backup::Locked {
        vault: s.name.clone(),
        unlock: Action::Vault(unlock),
    }
}

/// The plan's map as the chart's nodes, with its check.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn backup_nodes(
    r: &impl Reads,
    sum: &str,
    shape: &plan::Shape,
    a: &plan::Answers,
    names: &[String],
    notes: &[plan::Note],
    slot_of: &[Option<usize>],
    keys: &[KeyNode],
    any_here: bool,
) -> Backup {
    let app = r.app();
    let number = |i: usize| slot_of.get(i).copied().flatten().map_or(i + 1, |n| n + 1);
    // A vault of the plan by the name the vault keeps for it, else its
    // number.
    let vault_name = |v: usize| {
        notes
            .iter()
            .find_map(|n| match n {
                plan::Note::VaultName(u, name) if *u == v && !name.trim().is_empty() => {
                    Some(name.trim().to_string())
                }
                _ => None,
            })
            .unwrap_or_else(|| fill(EN.glance_vault, &[&(v + 1).to_string()]))
    };
    let place = |p: usize| {
        names
            .get(p)
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .map_or_else(
                || fill(EN.glance_place, &[&(p + 1).to_string()]),
                str::to_string,
            )
    };
    let label = |what: What| match what {
        What::Words(i) => fill(EN.glance_key_words, &[&number(i).to_string()]),
        What::SeedQr(i) => fill(EN.glance_key_seedqr, &[&number(i).to_string()]),
        What::Passphrase(i) => fill(EN.glance_passphrase, &[&number(i).to_string()]),
        What::Sheet => EN.glance_wallet_sheet.to_string(),
        What::Share(j) => fill(
            EN.glance_share_of,
            &[&(j + 1).to_string(), &shape.keys.to_string()],
        ),
        What::VaultStick(v) => fill(EN.glance_stick, &[&vault_name(v).to_string()]),
        What::Seed(i) => fill(EN.glance_key_seed, &[&number(i).to_string()]),
        What::SeedPassphrase(i) => fill(EN.glance_key_seed_passphrase, &[&number(i).to_string()]),
        What::PassphraseSealed(i) => fill(EN.glance_passphrase, &[&number(i).to_string()]),
        What::Wallet => EN.glance_description.to_string(),
        What::SeedFile(i) => fill(EN.glance_key_file, &[&number(i).to_string()]),
        What::SeedQrFile(i) => fill(EN.glance_key_seedqr_file, &[&number(i).to_string()]),
        What::PassphraseFile(i) => fill(EN.glance_passphrase_file, &[&number(i).to_string()]),
        What::WalletFile(f) => fill(EN.plan_wallet_file, &[plan::form_name(f)]),
        What::ShareFile(j, f) => fill(
            EN.plan_share_file,
            &[
                &(j + 1).to_string(),
                &shape.keys.to_string(),
                plan::form_name(f),
            ],
        ),
        What::SoftwareFile => EN.plan_software_file.to_string(),
    };
    // The small panel names the keys a vault's stick opens.
    let keys_label = |what: What| match what {
        What::VaultStick(v) => {
            let inside: Vec<String> = a
                .vault_seeds(shape, v)
                .into_iter()
                .map(|i| fill(EN.glance_key, &[&number(i).to_string()]))
                .collect();
            if inside.is_empty() {
                label(what)
            } else {
                fill(EN.glance_in_the_vault, &[&inside.join(", ").to_string()])
            }
        }
        _ => label(what),
    };
    let lines = r
        .backups()
        .into_iter()
        .find(|e| e.sum == sum)
        .and_then(|e| e.lines)
        .unwrap_or_default();
    let worst = |t: &[(What, Tag)]| {
        let rank = |t: Tag| match t {
            Tag::Public => 0,
            Tag::Sealed => 1,
            Tag::Secret => 2,
        };
        t.iter()
            .map(|(_, t)| *t)
            .max_by_key(|t| rank(*t))
            .unwrap_or(Tag::Public)
    };
    // The chart draws places: each with what its sticks keep.
    let mapped = r.plan_mapped(shape, a);
    let boxes = &mapped.places;
    // A vault's stick at more than one place: the first is its stick, each
    // further one a copy, "Vault 1 · copy" (§9.4).
    let stick_at = |at: At, v: usize| {
        boxes
            .iter()
            .any(|s| s.at == at && s.holds.iter().any(|(w, _)| *w == What::VaultStick(v)))
    };
    let copy_of = |at: At, v: usize| match a.first_stick(v).map(At::Place) {
        Some(first) if stick_at(first, v) => at != first,
        _ => boxes
            .iter()
            .take_while(|s| s.at != at)
            .any(|s| s.holds.iter().any(|(w, _)| *w == What::VaultStick(v))),
    };
    let note_of = |at: At, w: What| {
        // A SeedQR drawn in part is marked so by the plan itself.
        let mark = if plan::is_part(a, at, w) {
            Some(plan::Mark::Part)
        } else {
            notes.iter().find_map(|n| match n {
                plan::Note::Marked(x, y, m) if *x == at && *y == w => Some(*m),
                _ => None,
            })
        };
        let checked = notes.iter().find_map(|n| match n {
            plan::Note::Checked(x, y, d) if *x == at && *y == w => Some(d.clone()),
            _ => None,
        });
        (mark, checked)
    };
    let nodes = boxes
        .iter()
        // A cosigner's seed is theirs to keep: no node of this backup.
        .filter(|spot| !(spot.at == At::Away && any_here))
        .map(|spot| {
            // A place with its SeedQR copies drawn in part, which stay on
            // the map until destroyed (`docs/history/NEW-WALLET.md` §14.7 item 3).
            let parted = plan::with_parts(std::slice::from_ref(spot), a);
            let spot = &parted[0];
            let label = |w: What| match w {
                What::VaultStick(v) if copy_of(spot.at, v) => {
                    fill(EN.glance_copy, &[&vault_name(v).to_string()])
                }
                w => label(w),
            };
            let name = match spot.at {
                At::Place(p) | At::Stick(p, _) => place(p),
                At::Vault(v) => vault_name(v),
                At::Files => fill(EN.glance_of_files, &[app.medium.cap()]),
                At::Software => EN.glance_watch_only_software.to_string(),
                At::Away => EN.glance_on_its_own_device.to_string(),
            };
            let stick = match spot.at {
                At::Vault(v) => {
                    let at: Vec<String> = boxes
                        .iter()
                        .filter_map(|s| match s.at {
                            At::Place(p)
                                if s.holds.iter().any(|(w, _)| *w == What::VaultStick(v)) =>
                            {
                                Some(place(p))
                            }
                            _ => None,
                        })
                        .collect();
                    Some(if at.is_empty() {
                        EN.glance_its_own_stick.to_string()
                    } else {
                        fill(EN.glance_stick_at, &[&at.join(", ").to_string()])
                    })
                }
                _ => None,
            };
            let mut seeds: Vec<usize> = Vec::new();
            for (what, _) in &spot.holds {
                if let What::Words(i)
                | What::SeedQr(i)
                | What::Seed(i)
                | What::SeedPassphrase(i)
                | What::SeedFile(i) = *what
                    && let Some(k) = slot_of.get(i).copied().flatten()
                    && keys.get(k).is_some_and(|k| k.here)
                    && !seeds.contains(&k)
                {
                    seeds.push(k);
                }
            }
            let state = match spot.at {
                At::Away => Some((EN.glance_not_here.to_string(), Tone::Dim)),
                at => lines
                    .iter()
                    .find(|l| l.at == Some(at))
                    .map(|l| (l.state.clone(), l.tone)),
            };
            let (alone, alone_kind) = match spot.at {
                At::Vault(_) => (
                    Some(EN.glance_sealed.to_string()),
                    Some(plan::AloneKind::Sealed),
                ),
                At::Away => (None, None),
                at => {
                    let found = plan::alone_on(shape, a, &mapped, at);
                    (Some(found.text().to_string()), Some(found.kind()))
                }
            };
            let (marks, checked): (Vec<_>, Vec<_>) =
                spot.holds.iter().map(|(w, _)| note_of(spot.at, *w)).unzip();
            BackupNode {
                marks,
                checked,
                whats: spot.holds.iter().map(|(w, _)| *w).collect(),
                at: spot.at,
                name,
                tag: if matches!(spot.at, At::Vault(_)) {
                    Tag::Sealed
                } else {
                    worst(&spot.holds)
                },
                holds: if spot.holds.is_empty() {
                    vec![EN.glance_nothing.to_string()]
                } else {
                    spot.holds.iter().map(|(w, _)| label(*w)).collect()
                },
                keys_held: if spot.holds.is_empty() {
                    EN.glance_nothing.to_string()
                } else {
                    spot.holds
                        .iter()
                        .map(|(w, _)| match note_of(spot.at, *w).0 {
                            Some(m) => format!("{} ({})", keys_label(*w), m.label()),
                            None => keys_label(*w),
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                },
                stick,
                state,
                alone,
                alone_kind,
                keys: seeds,
            }
        })
        .collect();
    Backup::Plan {
        nodes,
        check: r.plan_check(shape, a),
        items: plan::checklist_on(shape, a, &mapped),
        edited: a.map.is_some(),
    }
}

// ---------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------

/// A line of text in a node, and what a press on that line opens.
struct Text {
    text: String,
    size: f32,
    weight: W,
    color: Color,
    press: Option<Action>,
    /// Struck through: a thing marked lost or exposed (§9.6).
    struck: bool,
}

impl Text {
    fn new(text: impl Into<String>, size: f32, weight: W, color: Color) -> Text {
        Text {
            text: text.into(),
            size,
            weight,
            color,
            press: None,
            struck: false,
        }
    }

    /// The line, struck through.
    fn struck(self) -> Text {
        Text {
            struck: true,
            ..self
        }
    }

    /// The line, pressed on its own.
    fn pressed(self, press: Option<Action>) -> Text {
        Text { press, ..self }
    }
}

/// A node as drawn: its title, the tag at its right, the lines under it,
/// a button at its foot, and what a press on it does.
struct Node {
    title: String,
    tag: Option<(&'static str, Color)>,
    lines: Vec<Text>,
    button: Option<(String, Style, Action)>,
    action: Option<Action>,
}

impl Node {
    /// The width its title and lines ask for, with its padding.
    fn natural(&self, ui: &Ui) -> f32 {
        let tag = self.tag.map_or(0.0, |(t, _)| {
            ui.measure(tokens::CAPTION, W::R, t) + tokens::GAP
        });
        let title = ui.measure(tokens::LABEL, W::S, &self.title) + tag;
        let lines = self
            .lines
            .iter()
            .map(|l| ui.measure(l.size, l.weight, &l.text))
            .fold(title, f32::max);
        let button = self.button.as_ref().map_or(0.0, |(l, ..)| {
            ui.measure(tokens::CAPTION, W::S, l) + 2.0 * tokens::PAD
        });
        lines.max(button) + 2.0 * tokens::CHART_NODE_PAD
    }

    /// Its height at width `w`.
    /// Whether its tag goes under its title, the two not fitting side by
    /// side at width `w`.
    fn tag_under(&self, ui: &Ui, w: f32) -> bool {
        self.tag.is_some_and(|(t, _)| {
            let inner = w - 2.0 * tokens::CHART_NODE_PAD;
            ui.measure(tokens::LABEL, W::S, &self.title)
                + tokens::GAP
                + ui.measure(tokens::CAPTION, W::R, t)
                > inner
        })
    }

    fn height(&self, ui: &Ui, w: f32) -> f32 {
        let inner = w - 2.0 * tokens::CHART_NODE_PAD;
        let mut h = 2.0 * tokens::CHART_NODE_PAD + ui.line(tokens::LABEL, W::S);
        if self.tag_under(ui, w) {
            h += ui.line(tokens::CAPTION, W::R);
        }
        for l in &self.lines {
            h += ui.wrap_lines(l.size, l.weight, &l.text, inner) as f32 * ui.line(l.size, l.weight);
        }
        if self.button.is_some() {
            h += tokens::GAP + tokens::CHART_BUTTON;
        }
        h
    }

    fn draw(&self, ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) {
        let pad = tokens::CHART_NODE_PAD;
        let pressed = self.action.is_some_and(|a| ui.is_pressed(a));
        ui.fill(x, y, w, h, tokens::RADIUS, if pressed { INNER } else { BG });
        ui.stroke(x, y, w, h, tokens::RADIUS, INNER);
        let inner = w - 2.0 * pad;
        let mut cy = y + pad;
        let under = self.tag_under(ui, w);
        let tag_w = match self.tag {
            Some(_) if under => 0.0,
            Some((t, c)) => {
                let tw = ui.measure(tokens::CAPTION, W::R, t);
                let lh = ui.line(tokens::LABEL, W::S);
                ui.text_right(x + w - pad, cy, lh, tokens::CAPTION, W::R, c, t);
                tw + tokens::GAP
            }
            None => 0.0,
        };
        // The node's own press first: a line pressed on its own is found
        // over it.
        if let Some(a) = self.action {
            ui.hit(x, y, w, h, a);
        }
        let title = ui.fit(tokens::LABEL, W::S, &self.title, inner - tag_w);
        ui.text(x + pad, cy, tokens::LABEL, W::S, TEXT, &title);
        cy += ui.line(tokens::LABEL, W::S);
        if let Some((t, c)) = self.tag.filter(|_| under) {
            ui.text(x + pad, cy, tokens::CAPTION, W::R, c, t);
            cy += ui.line(tokens::CAPTION, W::R);
        }
        for l in &self.lines {
            let lh = ui.wrap(x + pad, cy, inner, l.size, l.weight, l.color, &l.text);
            if l.struck {
                let one = ui.line(l.size, l.weight);
                let tw = ui.measure(l.size, l.weight, &l.text).min(inner);
                ui.rule(x + pad, cy + one / 2.0, tw, l.color);
            }
            if let Some(a) = l.press {
                ui.hit(x + pad, cy, inner, lh, a);
            }
            cy += lh;
        }
        if let Some((label, style, action)) = &self.button {
            let label = ui.fit(tokens::CAPTION, W::S, label, inner);
            ui.button(
                x + pad,
                cy + tokens::GAP,
                Some(inner),
                tokens::CHART_BUTTON,
                &label,
                *style,
                *action,
            );
        }
    }
}

/// A date a thing was checked here, as a line: "Checked 2026-10-10", or
/// "Checked" when the clock was not known.
pub(crate) fn checked_text(d: &str) -> String {
    if d.is_empty() {
        EN.glance_checked.to_string()
    } else {
        fill(EN.glance_checked_n, &[d])
    }
}

fn tag_color(t: Tag) -> Color {
    match t {
        Tag::Secret => WARN,
        Tag::Sealed => ACCENT,
        Tag::Public => MUTED,
    }
}

fn tone_color(t: Tone) -> Color {
    match t {
        Tone::Ok => OK,
        Tone::Warn => WARN,
        Tone::Err => ERR,
        Tone::Dim => DIM,
    }
}

/// How what a spot gives alone reads: spending in the clear is a warning,
/// a vault's seal the accent, nothing at all is good.
pub(crate) fn alone_color(k: plan::AloneKind) -> Color {
    match k {
        plan::AloneKind::CanSpend => WARN,
        plan::AloneKind::Nothing => OK,
        plan::AloneKind::Sealed | plan::AloneKind::NeedsVault => ACCENT,
        plan::AloneKind::SeesBalance => MUTED,
    }
}

/// The three rows of nodes: the wallet, its keys, its backup. Each node
/// pressed opens its sheet (`docs/history/NEW-WALLET.md` §9); on the wide chart
/// each line of what a backup node holds opens that line's, and a key's
/// way in is its own line's press.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn rows(g: &Glance, compact: bool) -> [Vec<Node>; 3] {
    use crate::glance::sheet::{ChartAction as C, Target};
    let open = |t: Target| Some(Action::Chart(C::Open(g.press, t)));
    let mut wallet_lines = vec![Text::new(
        format!("{} · #{}", g.shape, g.checksum),
        tokens::CAPTION,
        W::R,
        MUTED,
    )];
    if g.held {
        wallet_lines.push(Text::new(
            RECEIVE_WARNING.to_string(),
            tokens::CAPTION,
            W::S,
            WARN,
        ));
    }
    let wallet = vec![Node {
        title: g.name.clone(),
        tag: None,
        lines: wallet_lines,
        button: None,
        action: open(Target::Wallet),
    }];
    let keys = g
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut lines = Vec::new();
            if let Some(l) = &k.label {
                lines.push(Text::new(l.clone(), tokens::CAPTION, W::R, MUTED));
            }
            if let Some(h) = &k.holder {
                lines.push(Text::new(h.clone(), tokens::CAPTION, W::R, TEXT));
            }
            if let Some(d) = &k.checked {
                lines.push(Text::new(checked_text(d), tokens::CAPTION, W::R, OK));
            }
            lines.push(Text::new(
                k.state.clone(),
                tokens::CAPTION,
                W::S,
                if k.state == WAITING {
                    WARN
                } else if k.here {
                    OK
                } else {
                    MUTED
                },
            ));
            if let Some(way) = k.way_in {
                let tone = if k.action.is_some() { ACCENT } else { DIM };
                lines.push(Text::new(way, tokens::CAPTION, W::S, tone).pressed(k.action));
            }
            Node {
                title: fill(
                    EN.glance_key_2,
                    &[&k.number.to_string(), &k.fingerprint.to_string()],
                ),
                tag: None,
                lines,
                button: None,
                action: open(Target::Key(i as u8)),
            }
        })
        .collect();
    let backup = match &g.backup {
        Backup::Plan { nodes, .. } => nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let mut lines: Vec<Text> = if compact {
                    vec![Text::new(n.keys_held.clone(), tokens::CAPTION, W::R, TEXT)]
                } else {
                    n.holds
                        .iter()
                        .enumerate()
                        .map(|(j, h)| {
                            let line = (j < n.whats.len())
                                .then(|| open(Target::Line(i as u8, j as u8)))
                                .flatten();
                            match n.marks.get(j).copied().flatten() {
                                Some(m) => Text::new(
                                    format!("{h} · {}", m.label()),
                                    tokens::CAPTION,
                                    W::R,
                                    ERR,
                                )
                                .struck()
                                .pressed(line),
                                None => {
                                    Text::new(h.clone(), tokens::CAPTION, W::R, TEXT).pressed(line)
                                }
                            }
                        })
                        .collect()
                };
                if let Some(s) = &n.stick {
                    lines.push(Text::new(s.clone(), tokens::CAPTION, W::R, MUTED));
                }
                if let Some((s, t)) = &n.state {
                    lines.push(Text::new(s.clone(), tokens::CAPTION, W::R, tone_color(*t)));
                }
                if let (Some(s), Some(k)) = (&n.alone, n.alone_kind) {
                    lines.push(Text::new(s.clone(), tokens::CAPTION, W::S, alone_color(k)));
                }
                Node {
                    title: n.name.clone(),
                    tag: Some((n.tag.label(), tag_color(n.tag))),
                    lines,
                    button: None,
                    action: open(Target::Node(i as u8)),
                }
            })
            .collect(),
        Backup::Locked { vault, unlock } => vec![Node {
            title: fill(EN.glance_plan_in_locked, &[&vault.to_string()]),
            tag: None,
            lines: Vec::new(),
            button: Some((EN.glance_unlock.to_string(), Style::Secondary, *unlock)),
            action: None,
        }],
        Backup::None => vec![match g.press {
            Press::Loaded(w) => Node {
                title: EN.glance_no_backup_plan_2.to_string(),
                tag: None,
                lines: Vec::new(),
                button: Some((
                    EN.glance_back_up.to_string(),
                    Style::Secondary,
                    Action::Backup(w),
                )),
                action: None,
            },
            Press::Vault(..) => Node {
                title: EN.glance_no_backup_plan.to_string(),
                tag: None,
                lines: Vec::new(),
                button: None,
                action: None,
            },
        }],
    };
    [wallet, keys, backup]
}

/// Which way the chart reads: the wallet on top, its keys under it and
/// its backup at the foot, as the wallet's card draws it; or the backup on
/// top, through the keys, to the wallet at the foot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The wallet, then its keys, then its backup.
    WalletFirst,
    /// The backup, then the keys, then the wallet.
    BackupFirst,
}

impl Direction {
    /// The rows top down, by number: 0 the wallet, 1 its keys, 2 its
    /// backup.
    pub fn rows(self) -> [usize; 3] {
        match self {
            Direction::WalletFirst => [0, 1, 2],
            Direction::BackupFirst => [2, 1, 0],
        }
    }
}

/// Where a node is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Its left edge.
    pub x: f32,
    /// Its top edge.
    pub y: f32,
    /// Its width.
    pub w: f32,
    /// Its height: its line's tallest node's.
    pub h: f32,
    /// Its line of nodes, counted down the whole chart.
    pub line: usize,
}

/// A source of lines: a node, by row and place in it, and the nodes its
/// lines go to.
pub type Source = ((usize, usize), Vec<(usize, usize)>);

/// One line: its source, its upper node and its lower node.
type Edge = (usize, (usize, usize), (usize, usize));

/// The chart laid out: each row's nodes, by row number (0 the wallet, 1
/// its keys, 2 its backup), and each line drawn, by its source and its
/// points, every run between two of them across or down.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Each row's nodes.
    pub nodes: [Vec<Placed>; 3],
    /// Each line: its source, by place in the sources given, and its
    /// points.
    pub paths: Vec<(usize, Vec<(f32, f32)>)>,
    /// How tall the chart is.
    pub height: f32,
}

/// Places `n` nodes in lines across `w` from `x`: as many on a line as
/// fit at [`tokens::CHART_NODE_MIN`], each line centred. Returns each
/// node's x, width and line within the row.
fn arrange(n: usize, x: f32, w: f32) -> Vec<(f32, f32, usize)> {
    if n == 0 {
        return Vec::new();
    }
    let gap = tokens::CHART_NODE_GAP;
    let per_line = (((w + gap) / (tokens::CHART_NODE_MIN + gap)).floor() as usize).max(1);
    let node_w = if n <= per_line {
        ((w - (n - 1) as f32 * gap) / n as f32).min(tokens::CHART_NODE_MAX)
    } else {
        (w - (per_line - 1) as f32 * gap) / per_line as f32
    };
    let mut out = Vec::new();
    for i in 0..n {
        let line = i / per_line;
        let on_line = (n - line * per_line).min(per_line);
        let line_w = on_line as f32 * node_w + (on_line - 1) as f32 * gap;
        let start = x + (w - line_w) / 2.0;
        out.push((start + (i % per_line) as f32 * (node_w + gap), node_w, line));
    }
    out
}

/// Lays the chart out at (x, y), `width` wide, reading `direction`: the
/// wallet's node `wallet_w` wide and centred, `counts` nodes in each row
/// (by row number), each `height(row, i, width)` tall, and the lines of
/// `sources`. A line leaves the middle of the upper node's bottom edge
/// (or, with more sources there, its own point along it), runs down into
/// the band under that line of nodes, across, down through the gap
/// between two nodes of each line it passes, and into the lower node's
/// top edge: across or down, never through a node.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn layout(
    direction: Direction,
    x: f32,
    y: f32,
    width: f32,
    wallet_w: f32,
    counts: [usize; 3],
    height: &dyn Fn(usize, usize, f32) -> f32,
    sources: &[Source],
) -> Layout {
    let mut placed: [Vec<Placed>; 3] = Default::default();
    let mut line = 0;
    for r in direction.rows() {
        let spots = if r == 0 {
            let nw = wallet_w.max(tokens::CHART_NODE_MIN).min(width);
            (0..counts[0])
                .map(|_| (x + (width - nw) / 2.0, nw, 0))
                .collect()
        } else {
            arrange(counts[r], x, width)
        };
        let lines = spots.iter().map(|s| s.2 + 1).max().unwrap_or(0);
        placed[r] = spots
            .iter()
            .enumerate()
            .map(|(i, &(nx, nw, l))| Placed {
                x: nx,
                y: 0.0,
                w: nw,
                h: height(r, i, nw),
                line: line + l,
            })
            .collect();
        line += lines;
    }
    let lines = line;
    // Every line, from its upper node to its lower one.
    let edges: Vec<Edge> = sources
        .iter()
        .enumerate()
        .flat_map(|(s, (from, to))| {
            let placed = &placed;
            to.iter().filter_map(move |&t| {
                let (a, b) = (placed[from.0].get(from.1)?, placed[t.0].get(t.1)?);
                Some(if a.line <= b.line {
                    (s, *from, t)
                } else {
                    (s, t, *from)
                })
            })
        })
        .collect();
    let at = |p: (usize, usize)| placed[p.0][p.1];
    // The sources each band between two lines of nodes carries, in order.
    let band_of: Vec<Vec<usize>> = (0..lines.saturating_sub(1))
        .map(|b| {
            let mut v: Vec<usize> = edges
                .iter()
                .filter(|(_, u, d)| at(*u).line <= b && b < at(*d).line)
                .map(|(s, ..)| *s)
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();
    let band_h: Vec<f32> = band_of
        .iter()
        .map(|v| match v.len() {
            0 => tokens::CHART_NODE_GAP,
            n => 2.0 * tokens::CHART_BAND + (n - 1) as f32 * tokens::CHART_LANE,
        })
        .collect();
    // Each line's top and height.
    let mut line_h = vec![0.0f32; lines];
    for p in placed.iter().flatten() {
        line_h[p.line] = line_h[p.line].max(p.h);
    }
    let mut line_y = vec![y; lines];
    for l in 1..lines {
        line_y[l] = line_y[l - 1] + line_h[l - 1] + band_h[l - 1];
    }
    for p in placed.iter_mut().flatten() {
        p.y = line_y[p.line];
        p.h = line_h[p.line];
    }
    let at = |p: (usize, usize)| placed[p.0][p.1];
    let lane_y = |b: usize, s: usize| {
        let slot = band_of[b].iter().position(|&x| x == s).unwrap_or(0);
        line_y[b] + line_h[b] + tokens::CHART_BAND + slot as f32 * tokens::CHART_LANE
    };
    // Where a line runs down through line `l`: the gap between two of its
    // nodes, or beside them, nearest both ends.
    let gap_x = |l: usize, from: f32, to: f32| -> f32 {
        let mut on: Vec<Placed> = placed
            .iter()
            .flatten()
            .copied()
            .filter(|p| p.line == l)
            .collect();
        on.sort_by(|a, b| a.x.total_cmp(&b.x));
        let half = tokens::CHART_NODE_GAP / 2.0;
        let mut ways: Vec<f32> = on
            .windows(2)
            .map(|p| (p[0].x + p[0].w + p[1].x) / 2.0)
            .collect();
        if let Some(first) = on.first()
            && first.x - x >= half
        {
            ways.push(first.x - half);
        }
        if let Some(last) = on.last()
            && x + width - (last.x + last.w) >= half
        {
            ways.push(last.x + last.w + half);
        }
        ways.into_iter()
            .min_by(|a, b| {
                ((a - from).abs() + (a - to).abs()).total_cmp(&((b - from).abs() + (b - to).abs()))
            })
            .unwrap_or(to)
    };
    // A node's point along its bottom (`below`) or top edge for source
    // `s`: one per source meeting it there, spread evenly.
    let port = |n: (usize, usize), below: bool, s: usize| -> f32 {
        let mut meet: Vec<usize> = edges
            .iter()
            .filter(|(_, u, d)| if below { *u == n } else { *d == n })
            .map(|(s, ..)| *s)
            .collect();
        meet.sort_unstable();
        meet.dedup();
        let j = meet.iter().position(|&i| i == s).unwrap_or(0);
        let p = at(n);
        p.x + p.w * (j + 1) as f32 / (meet.len() + 1) as f32
    };
    let mut paths = Vec::new();
    for &(s, u, d) in &edges {
        let (up, down) = (at(u), at(d));
        let sx = port(u, true, s);
        let tx = port(d, false, s);
        let mut pts = vec![(sx, up.y + up.h)];
        let mut cx = sx;
        for (b, carried) in band_of.iter().enumerate().take(down.line).skip(up.line) {
            let ly = lane_y(b, s);
            pts.push((cx, ly));
            if b + 1 < down.line {
                let n = carried.len() as f32;
                let slot = carried.iter().position(|&i| i == s).unwrap_or(0) as f32;
                cx = gap_x(b + 1, cx, tx) + (slot - (n - 1.0) / 2.0) * tokens::CHART_LANE;
                pts.push((cx, ly));
                pts.push((cx, lane_y(b + 1, s)));
            } else {
                cx = tx;
                pts.push((cx, ly));
            }
        }
        pts.push((tx, down.y));
        pts.dedup();
        paths.push((s, pts));
    }
    let bottom = (0..lines).map(|l| line_y[l] + line_h[l]).fold(y, f32::max);
    Layout {
        nodes: placed,
        paths,
        height: bottom - y,
    }
}

/// Draws the chart of `g` at (x, y), `width` wide, reading `direction`,
/// with the plan's check strip above it, either way. Returns its height.
pub(crate) fn draw(
    ui: &mut Ui,
    g: &Glance,
    direction: Direction,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    draw_with(ui, g, direction, x, y, width, true)
}

/// [`draw`], with the check strip only when `check`: the end of the
/// backup draws its own strip above the done card.
pub(crate) fn draw_with(
    ui: &mut Ui,
    g: &Glance,
    direction: Direction,
    x: f32,
    y: f32,
    width: f32,
    check: bool,
) -> f32 {
    let rows = rows(g, false);
    let mut top = y;
    // The check leads, read either way (`docs/history/NEW-WALLET.md` §14.6).
    if check && let Backup::Plan { check, .. } = &g.backup {
        top += check_lines(ui, x, top, width, *check, true, &names_of(g));
        top += tokens::PAD;
    }
    // The lines' sources: the wallet to each key, each key held here to
    // every node that holds its seed, each in its colour and dashes.
    let mut sources: Vec<Source> = vec![((0, 0), (0..g.keys.len()).map(|k| (1, k)).collect())];
    let mut looks: Vec<(Color, [f32; 4], f32)> =
        vec![(BORDER, tokens::CHART_DASHES[0], tokens::CHART_STROKE)];
    if let Backup::Plan { nodes, .. } = &g.backup {
        for (held, (k, _)) in g
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.here)
            .enumerate()
        {
            let to: Vec<(usize, usize)> = nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.keys.contains(&k))
                .map(|(i, _)| (2, i))
                .collect();
            sources.push(((1, k), to));
            // Where it is: this key's lines bold, every other dimmed.
            let (color, stroke) = match g.focus {
                Some(f) if f == k => (KEY_LINES[held % KEY_LINES.len()], tokens::CHART_STROKE_BOLD),
                Some(_) => (INNER, tokens::CHART_STROKE),
                None => (KEY_LINES[held % KEY_LINES.len()], tokens::CHART_STROKE),
            };
            looks.push((
                color,
                tokens::CHART_DASHES[held % tokens::CHART_DASHES.len()],
                stroke,
            ));
        }
    }
    if g.focus.is_some() {
        looks[0].0 = INNER;
    }
    let wallet_w = rows[0].iter().map(|n| n.natural(ui)).fold(0.0, f32::max);
    let counts = [rows[0].len(), rows[1].len(), rows[2].len()];
    let laid = {
        let ui: &Ui = ui;
        let height = |r: usize, i: usize, nw: f32| rows[r][i].height(ui, nw);
        layout(
            direction, x, top, width, wallet_w, counts, &height, &sources,
        )
    };
    // The lines, under the nodes.
    for (s, pts) in &laid.paths {
        let (color, dash, stroke) = looks[*s];
        for seg in pts.windows(2) {
            segment(ui, seg[0], seg[1], color, dash, stroke);
        }
    }
    for (r, nodes) in rows.iter().enumerate() {
        for (node, p) in nodes.iter().zip(&laid.nodes[r]) {
            node.draw(ui, p.x, p.y, p.w, p.h);
        }
    }
    let cy = top + laid.height;
    cy - y
}

/// An orthogonal run from `a` to `b`, in `dash`, `stroke` wide.
fn segment(ui: &mut Ui, a: (f32, f32), b: (f32, f32), color: Color, dash: [f32; 4], stroke: f32) {
    let across = (a.1 - b.1).abs() < f32::EPSILON;
    let (start, end) = if across {
        (a.0.min(b.0), a.0.max(b.0))
    } else {
        (a.1.min(b.1), a.1.max(b.1))
    };
    let half = stroke / 2.0;
    let run = |from: f32, to: f32, ui: &mut Ui| {
        if to <= from {
            return;
        }
        match (across, stroke > tokens::CHART_STROKE) {
            (true, false) => ui.rule(from, a.1 - half, to - from, color),
            (false, false) => ui.vrule(a.0 - half, from, to - from, color),
            (true, true) => ui.fill(from, a.1 - half, to - from, stroke, 0.0, color),
            (false, true) => ui.fill(a.0 - half, from, stroke, to - from, 0.0, color),
        }
    };
    if dash[1] <= 0.0 {
        run(start, end, ui);
        return;
    }
    let mut at = start;
    let mut k = 0;
    while at < end {
        let len = dash[k % 4];
        if k % 2 == 0 {
            run(at, (at + len).min(end), ui);
        }
        at += len;
        k += 1;
    }
}

/// What the plan's places are called, as the chart's nodes name them.
pub(crate) fn names_of(g: &Glance) -> impl Fn(usize) -> String + '_ {
    move |p| match &g.backup {
        Backup::Plan { nodes, .. } => nodes
            .iter()
            .find(|n| n.at == At::Place(p))
            .map(|n| n.name.clone())
            .unwrap_or_else(|| crate::backup_flow::BackupFlow::place_name(p)),
        _ => crate::backup_flow::BackupFlow::place_name(p),
    }
}

/// The plan's three check lines as a strip (`docs/history/NEW-WALLET.md` §14.6):
/// each line in a box with its value large in its colour, side by side
/// with `beside` where the width allows, else one box under another. The
/// wallet card, the open vault, the end of the backup and the backup's
/// map panel all draw it. Returns its height.
pub(crate) fn check_lines(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    c: plan::Check,
    beside: bool,
    place: &dyn Fn(usize) -> String,
) -> f32 {
    let h = check_boxes(ui, x, y, w, c, beside);
    h + check_facts(ui, x, y + h, w, c, place)
}

/// The two lines of fact under the three (`docs/BACKUP-FLOW.md` §10.3): a
/// wallet whose secrets are all at one place says so, in `WARN`; a single
/// key with no passphrase says one kept at another place than the seed is
/// recommended, with **Add a passphrase**. Neither blocks anything.
/// Returns their height, with the gap above them.
fn check_facts(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    c: plan::Check,
    place: &dyn Fn(usize) -> String,
) -> f32 {
    let mut cy = y;
    if let Some(p) = c.one_place {
        cy += tokens::GAP;
        let line = fill(EN.plan_all_at_one_place, &[&place(p)]);
        cy += ui.wrap(x, cy, w, tokens::LABEL, W::R, WARN, &line);
    }
    if c.passphrase_apart {
        cy += tokens::GAP;
        cy += ui.wrap(
            x,
            cy,
            w,
            tokens::LABEL,
            W::R,
            MUTED,
            EN.plan_passphrase_apart,
        );
        cy += tokens::GAP;
        let label = EN.plan_add_passphrase;
        let bw = ui.measure(tokens::LABEL, W::S, label) + 32.0;
        ui.button(
            x,
            cy,
            Some(bw),
            34.0,
            label,
            Style::Secondary,
            Action::PlanAddPass,
        );
        cy += 34.0;
    }
    cy - y
}

/// The three check lines as boxes.
fn check_boxes(ui: &mut Ui, x: f32, y: f32, w: f32, c: plan::Check, beside: bool) -> f32 {
    use crate::plan::{CHECK_LINES, Found, Lost};
    let lost = match c.lost {
        Lost::Yes => OK,
        Lost::WithVault => ACCENT,
        Lost::No => ERR,
    };
    let found = |f: Found| match f {
        Found::Yes => WARN,
        Found::OnlyWithVault => ACCENT,
        Found::No => OK,
    };
    // A single place: No by the nature of it, said so, muted (§19.19).
    let (lost_text, lost) = if c.single && c.lost == Lost::No {
        (EN.plan_no_single_place, MUTED)
    } else {
        (c.lost.text(), lost)
    };
    let lines = [
        (CHECK_LINES[0], lost_text, lost),
        (CHECK_LINES[1], c.spend.text(), found(c.spend)),
        (CHECK_LINES[2], c.balance.text(), found(c.balance)),
    ];
    let gap = tokens::CHART_NODE_GAP;
    let pad = tokens::CHART_NODE_PAD;
    let col_w = (w - 2.0 * gap) / lines.len() as f32;
    let side = beside && col_w >= tokens::CHART_NODE_MIN;
    let box_w = if side { col_w } else { w };
    // Each box's height, measured before it is drawn.
    let measure = |ui: &mut Ui, label: &str, value: &str| -> f32 {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let mut h = ui.wrap(
            0.0,
            0.0,
            box_w - 2.0 * pad,
            tokens::CAPTION,
            W::R,
            MUTED,
            label,
        );
        h += tokens::GAP_SMALL;
        h += ui.wrap(
            0.0,
            0.0,
            box_w - 2.0 * pad,
            tokens::TITLE,
            W::S,
            MUTED,
            value,
        );
        ui.c.pop_clip();
        h + 2.0 * pad
    };
    let heights: Vec<f32> = lines
        .iter()
        .map(|(label, value, _)| measure(ui, label, value))
        .collect();
    let tallest = heights.iter().copied().fold(0.0, f32::max);
    let mut cy = y;
    for (i, (label, value, tone)) in lines.into_iter().enumerate() {
        let (bx, by, bh) = if side {
            (x + i as f32 * (col_w + gap), y, tallest)
        } else {
            (x, cy, heights[i])
        };
        ui.fill(bx, by, box_w, bh, tokens::RADIUS_SMALL, tone.with_alpha(18));
        ui.stroke(bx, by, box_w, bh, tokens::RADIUS_SMALL, tone.with_alpha(90));
        let tx = bx + pad;
        let tw = box_w - 2.0 * pad;
        let lh = ui.wrap(tx, by + pad, tw, tokens::CAPTION, W::R, MUTED, label);
        ui.wrap(
            tx,
            by + pad + lh + tokens::GAP_SMALL,
            tw,
            tokens::TITLE,
            W::S,
            tone,
            value,
        );
        if !side {
            cy += bh + tokens::GAP;
        }
    }
    if side { tallest } else { cy - y - tokens::GAP }
}

/// The small panel's page: the wallet, its keys and its backup in one
/// column, with no lines; each backup node names the keys it holds. Then
/// the check; read backup first, the check, the backup, the keys and the
/// wallet. Returns its height.
pub(crate) fn draw_column(
    ui: &mut Ui,
    g: &Glance,
    direction: Direction,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    draw_column_with(ui, g, direction, x, y, width, true)
}

/// [`draw_column`], with the check rows only when `with_check`.
pub(crate) fn draw_column_with(
    ui: &mut Ui,
    g: &Glance,
    direction: Direction,
    x: f32,
    y: f32,
    width: f32,
    with_check: bool,
) -> f32 {
    let [wallet, keys, backup] = rows(g, true);
    let mut cy = y;
    let check = match &g.backup {
        Backup::Plan { check, .. } if with_check => Some(*check),
        _ => None,
    };
    // The check leads, as three rows (`docs/history/NEW-WALLET.md` §14.6).
    if let Some(c) = check {
        cy += check_lines(ui, x, cy, width, c, false, &names_of(g)) + tokens::PAD;
    }
    let order = match direction {
        Direction::WalletFirst => [
            ("", wallet),
            (EN.glance_keys, keys),
            (EN.glance_backup, backup),
        ],
        Direction::BackupFirst => [
            (EN.glance_backup, backup),
            (EN.glance_keys, keys),
            (EN.glance_wallet, wallet),
        ],
    };
    for (heading, nodes) in order {
        if !heading.is_empty() {
            cy += crate::compact::screen::heading(ui, x, cy, heading);
        }
        for node in nodes {
            let h = node.height(ui, width);
            node.draw(ui, x, cy, width, h);
            cy += h + tokens::GAP;
        }
        cy += tokens::GAP;
    }
    cy - y
}

/// The line of the chart `g` that says what `what` is at `at`: "Key 1
/// words", "Vault 1 · copy".
pub(crate) fn label_of(g: &Glance, at: At, what: What) -> Option<String> {
    let Backup::Plan { nodes, .. } = &g.backup else {
        return None;
    };
    let node = nodes.iter().find(|n| n.at == at)?;
    let j = node.whats.iter().position(|w| *w == what)?;
    node.holds.get(j).cloned()
}

/// A thing as the chart names it, wherever it is: "Key 1 words", "Wallet
/// sheet", "Vault 1 stick".
pub(crate) fn thing_label(g: Option<&Glance>, what: What) -> String {
    let number = |i: usize| {
        g.and_then(|g| g.keys.iter().find(|k| k.seed == Some(i)))
            .map_or(i + 1, |k| k.number)
    };
    match what {
        What::Words(i) => fill(EN.glance_key_words, &[&number(i).to_string()]),
        What::SeedQr(i) => fill(EN.glance_key_seedqr, &[&number(i).to_string()]),
        What::Passphrase(i) => fill(EN.glance_passphrase, &[&number(i).to_string()]),
        What::Sheet => EN.glance_wallet_sheet.to_string(),
        What::Share(j) => fill(EN.glance_share, &[&(j + 1).to_string()]),
        What::VaultStick(v) => fill(EN.glance_vault_copy, &[&(v + 1).to_string()]),
        What::Seed(i) => fill(EN.glance_key_seed, &[&number(i).to_string()]),
        What::SeedPassphrase(i) => fill(EN.glance_key_seed_passphrase, &[&number(i).to_string()]),
        What::Wallet => EN.glance_the_description.to_string(),
        What::SeedFile(i) | What::SeedQrFile(i) => fill(EN.glance_key, &[&number(i).to_string()]),
        What::PassphraseFile(i) | What::PassphraseSealed(i) => {
            fill(EN.glance_passphrase, &[&number(i).to_string()])
        }
        What::WalletFile(f) => fill(EN.plan_wallet_file, &[plan::form_name(f)]),
        What::ShareFile(j, _) => fill(EN.glance_share, &[&(j + 1).to_string()]),
        What::SoftwareFile => EN.plan_software_file.to_string(),
    }
}
