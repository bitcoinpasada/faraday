//! Acting on the chart (`docs/history/NEW-WALLET.md` §9): a press on a node of
//! the wallet at a glance, or on a line inside one, opens a sheet for it:
//! what it is, where it is, what this device saw of it, when it was last
//! checked here, and its actions. Each action leads into the flow that
//! already does it: the backup's checklist at the item that makes that
//! thing, the wallet's QR sheet, a public file For the stick, the copy's
//! scan, the vault way. Opening a sheet changes nothing and puts nothing
//! For the stick.
//!
//! A change to where something is kept (a move, another copy, a thing
//! added, destroyed or a place removed) is asked first, with the check's
//! three lines before and after, and once made the map is the plan
//! (§9.7, [`crate::chart_edit`]). Names, dates and marks are kept in the
//! vault alone (§9.5); with none open, those rows lead to one. A thing
//! marked lost or exposed says what follows, in order (§9.6).
//!
//! In an open vault's view (§7.2) the wallet may not be loaded: an
//! action loads it first, with its seeds in that vault, as the chart's
//! presses did before.

use crate::strings::EN;
use crate::strings::fill;
use crate::ui::Paint;
use faraday_crypto::work::Compute;
use faraday_draw::tokens;
use faraday_draw::widgets::Icon;

use crate::chart_edit::OFF;
use crate::glance::{self, Backup, BackupNode, Glance, Press};
use crate::plan::{self, At, Edit, Item, Mark, What};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Screen, Sheet, bstep};
use faraday_draw::tokens::{BODY_L, BODY_S, FIELD_GAP, FIELD_H, FIELD_PAD, LABEL_L, LABEL_M};
use faraday_keys::Reveal;

/// What on the chart a sheet is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The wallet's node.
    Wallet,
    /// Key k, by place among the chart's keys.
    Key(u8),
    /// Backup node n, by place in the backup row.
    Node(u8),
    /// Line j of backup node n: one thing it holds.
    Line(u8, u8),
    /// Where line j of backup node n moves to: a place.
    MoveTo(u8, u8),
    /// Where another copy of a thing goes: a place.
    CopyTo(What),
    /// What can be added at place node n.
    AddAt(u8),
    /// A change asked for: the check before and after, and Confirm.
    Confirm,
    /// What follows from the mark on line j of node n, or on the node.
    Marked(u8, Option<u8>),
    /// Removing place node n: where each thing in it goes.
    Remove(u8),
    /// The name of key k's holder, typed.
    Holder(u8),
    /// The name of vault node n, typed.
    VaultName(u8),
    /// Key k's passphrase, typed to be checked; from line j of node n, or
    /// from the key ([`NO_LINE`]).
    PassCheck(u8, u8, u8),
    /// Key k's passphrase shown, to copy at line j of node n.
    PassShow(u8, u8, u8),
    /// The wallet's first addresses, as Create's Check shows them.
    Addresses,
}

/// [`Target::PassCheck`] opened from the key, on no line.
pub const NO_LINE: u8 = u8::MAX;

/// What an action of a sheet does with the wallet, loaded first in a
/// vault's view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The wallet's QR sheet.
    Qr,
    /// One of the wallet's public files For the stick, by
    /// [`Faraday::public_out`]'s number.
    Out(u8),
    /// The account key of slot k as a QR code.
    KeyQr(u8),
    /// The account key of slot k as a file, For the stick.
    KeyFile(u8),
    /// The backup's checklist, at item n by [`bstep`].
    Item(u8),
    /// Scan a copy of the seed with this fingerprint, and back to the
    /// chart.
    Scan([u8; 4]),
    /// Type the words of a copy of the seed with this fingerprint, at the
    /// checklist's copy item by [`bstep`], with the check open.
    Type([u8; 4], u8),
    /// The backup's plan questions, with the plan the chart shows.
    Plan,
    /// Type a new name for place p: the plan's Places question.
    RenamePlace(u8),
    /// Type a new name for the wallet, on its card.
    Rename,
    /// The wallet's first addresses, as Create's Check shows them.
    Addresses,
    /// The account key of slot k, held elsewhere, as a QR code: the
    /// wallet's descriptor holds it.
    CosignerQr(u8),
    /// What goes in each place's envelope, a PDF For the stick.
    Envelopes,
}

/// A press on the chart or on one of its sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartAction {
    /// Open the sheet for this thing on this chart.
    Open(Press, Target),
    /// Do this with the chart's wallet.
    Do(Press, Step),
    /// Draw key k's lines bold and the rest dimmed: **Where it is**.
    Where(Press, u8),
    /// Ask for a change to where something is kept: its check before and
    /// after.
    Edit(Press, Edit),
    /// Make the change asked for.
    Confirm(Press),
    /// Mark line j of node n, or the whole node, lost or exposed.
    Mark(Press, u8, Option<u8>, Mark),
    /// Every thing in node n checked here today.
    Checked(Press, u8),
    /// Key k's holder confirmed its backup today.
    KeyChecked(Press, u8),
    /// Thing j of the place being removed goes to place p, or [`OFF`].
    Dest(u8, u8),
    /// Keep the name typed.
    Save(Press),
    /// Compare the passphrase typed.
    PassTry(Press),
    /// Show the passphrase to copy.
    Reveal,
    /// Move the money to a new wallet.
    MoveMoney(Press),
}

/// One action row of a sheet.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub label: String,
    pub sub: String,
    pub action: Option<Action>,
}

/// A sheet as shown: its icon and title, its facts (label, value, tone),
/// lines of what follows, its rows under their headings, a field being
/// typed, its own button, and a wallet's first addresses.
#[derive(Debug, Clone)]
pub(crate) struct View {
    pub icon: Icon,
    pub title: String,
    pub facts: Vec<(&'static str, String, Paint)>,
    pub lines: Vec<(String, Paint)>,
    /// Lines that are a secret shown (a passphrase), wiped on drop,
    /// drawn after `lines`.
    pub secret_lines: Vec<(zeroize::Zeroizing<String>, Paint)>,
    pub groups: Vec<(String, Vec<Row>)>,
    /// A field: its label, what it shows, and whether that is a hint.
    pub field: Option<(&'static str, String, bool)>,
    /// The sheet's own button; `None` as its action while it waits.
    pub go: Option<(String, Option<Action>)>,
    /// The loaded wallet whose first addresses are shown.
    pub addresses: Option<usize>,
}

impl View {
    fn new(icon: Icon, title: impl Into<String>) -> View {
        View {
            icon,
            title: title.into(),
            facts: Vec::new(),
            lines: Vec::new(),
            secret_lines: Vec::new(),
            groups: Vec::new(),
            field: None,
            go: None,
            addresses: None,
        }
    }
}

fn row(label: impl Into<String>, sub: impl Into<String>, action: Option<Action>) -> Row {
    Row {
        label: label.into(),
        sub: sub.into(),
        action,
    }
}

/// The chart a press was made on, read through `r` ([`glance::Reads`]).
fn chart(r: &impl glance::Reads, press: Press) -> Option<Glance> {
    match press {
        Press::Loaded(w) => glance::of_on(r, w),
        Press::Vault(v, i) => glance::of_vault_on(r, v, i),
    }
}

/// Costs what the chart costs to read the first time: where backup node
/// `n` of the chart of `press` is, for a decision, from the state as it
/// is.
pub(crate) fn node_at_now(c: &Compute, app: &Faraday, press: Press, n: u8) -> Option<At> {
    match chart(&glance::Now { app, c }, press)?.backup {
        Backup::Plan { nodes, .. } => nodes.get(usize::from(n)).map(|x| x.at),
        _ => None,
    }
}

/// How a check line reads before and after a change, and its tone: a
/// line made worse in `warning`.
fn check_facts(before: plan::Check, after: plan::Check) -> Vec<(&'static str, String, Paint)> {
    let lines = [
        (
            before.lost.text(),
            after.lost.text(),
            after.lost < before.lost,
        ),
        (
            before.spend.text(),
            after.spend.text(),
            after.spend > before.spend,
        ),
        (
            before.balance.text(),
            after.balance.text(),
            after.balance > before.balance,
        ),
    ];
    plan::CHECK_LINES
        .iter()
        .zip(lines)
        .map(|(label, (b, a, worse))| {
            (
                *label,
                if a == b {
                    a.to_string()
                } else {
                    format!("{b} → {a}")
                },
                if worse {
                    Role::Warning.paint()
                } else {
                    Role::OnSurface.paint()
                },
            )
        })
        .collect()
}

/// The sheet for `target` on the chart of `press`; `None` when it is no
/// longer there. Drawing's form: the chart through the memo and the kept
/// values. Only drawing asks for it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn view(app: &crate::drawing::View, press: Press, target: Target) -> Option<View> {
    let g = chart(app, press)?;
    let chart_do = |a: ChartAction| Some(Action::Chart(a));
    let doing = |s: Step| chart_do(ChartAction::Do(press, s));
    let open = |t: Target| chart_do(ChartAction::Open(press, t));
    let edit = |e: Edit| chart_do(ChartAction::Edit(press, e));
    let (nodes, items): (&[BackupNode], &[Item]) = match &g.backup {
        Backup::Plan { nodes, items, .. } => (nodes, items),
        _ => (&[], &[]),
    };
    // The checklist item that makes `it`, where the plan has one.
    let item = |it: Item| {
        items
            .contains(&it)
            .then(|| doing(Step::Item(bstep::of(it))))
            .flatten()
    };
    let holds = |f: &dyn Fn(What) -> bool| nodes.iter().any(|n| n.whats.iter().any(|w| f(*w)));
    // What checking a copy of seed `s`, with fingerprint `fp`, in the
    // `forms` it takes asks for (`docs/history/NEW-WALLET.md` §14.7): the rows a
    // key or a line offers, one per check.
    let check_rows = |forms: plan::CopyForms, s: usize, fp: [u8; 4]| {
        let english = app
            .session
            .keys
            .all()
            .find(|k| k.fingerprint.0 == fp)
            .is_none_or(|k| k.language == faraday_bip::bip39::Language::English);
        let checks = forms.checks(!app.cameras().is_empty(), english);
        let mut rows = Vec::new();
        if checks.scan {
            rows.push((EN.glance_sheet_scan_my, doing(Step::Scan(fp))));
        }
        if checks.typed && items.contains(&Item::Copy(s)) {
            let n = bstep::of(Item::Copy(s));
            rows.push((EN.glance_sheet_type_my, doing(Step::Type(fp, n))));
        }
        rows
    };
    let split = holds(&|w| matches!(w, What::Share(_)));
    // A vault: open it, or the way to one, back to this chart.
    let back = match press {
        Press::Loaded(_) => Screen::Wallets,
        Press::Vault(..) => Screen::VaultContents,
    };
    let vault_open = app.vaults.open.get(app.vaults.current).is_some();
    let vault_way = || match app.vault_way(back) {
        Some((label, a)) => Some(row(label, "", Some(a))),
        None => matches!(press, Press::Loaded(_)).then(|| {
            row(
                EN.glance_sheet_open_vault,
                "",
                Some(Action::Nav(Screen::VaultContents)),
            )
        }),
    };
    // A row that keeps a name, a date or a mark: in the vault, or the way
    // to one (§9.1).
    let keeping = |label: &str, a: Option<Action>| {
        if g.keep {
            row(label, "", a)
        } else {
            row(
                label,
                EN.glance_sheet_open_vault_to_keep,
                app.vault_way(back).map(|(_, a)| a),
            )
        }
    };
    // The places, by node: their number and name.
    let places: Vec<(usize, &BackupNode)> = nodes
        .iter()
        .filter_map(|n| match n.at {
            At::Place(p) => Some((p, n)),
            _ => None,
        })
        .collect();
    let new_place = places.len();
    let key_of_seed = |s: usize| g.keys.iter().position(|k| k.seed == Some(s));
    // Change the plan: the plan's cards show the map as it is, the
    // chart's edits included, so nothing is replaced and nothing is asked
    // (`docs/BACKUP-FLOW.md` §15.2).
    let change_plan = || doing(Step::Plan);
    let marks: Vec<(At, What, Mark)> = nodes
        .iter()
        .flat_map(|n| {
            n.whats
                .iter()
                .zip(&n.marks)
                .filter_map(|(w, m)| m.map(|m| (n.at, *w, m)))
        })
        .collect();
    match target {
        Target::Wallet => {
            let mut rows = Vec::new();
            if let Press::Vault(v, r) = press {
                rows.push(row(
                    EN.glance_sheet_open_wallets,
                    "",
                    Some(Action::Vault(V::OpenWalletOf(v, r))),
                ));
            }
            rows.push(row(
                EN.glance_sheet_show_wallet,
                EN.glance_sheet_the_descriptor,
                doing(Step::Qr),
            ));
            rows.push(row(
                EN.glance_sheet_check_addresses,
                EN.glance_sheet_the_first,
                doing(Step::Addresses),
            ));
            if let Some(a) = item(Item::PublicFiles) {
                rows.push(row(
                    EN.glance_sheet_public_files,
                    EN.glance_sheet_for_the_software,
                    Some(a),
                ));
            }
            if matches!(g.backup, Backup::Plan { .. }) {
                let (label, sub, what) = if split {
                    (EN.glance_sheet_print, EN.glance_sheet_pdfs_text, 4)
                } else {
                    (EN.glance_sheet_reprint_wallet_sheet, "PDF", 3)
                };
                rows.push(row(label, sub, doing(Step::Out(what))));
            }
            rows.push(row(EN.glance_sheet_rename_wallet, "", doing(Step::Rename)));
            rows.push(row(EN.glance_sheet_change, "", change_plan()));
            let mut v = View::new(Icon::Wallet, g.name.clone());
            v.facts = vec![
                (
                    EN.glance_sheet_shape,
                    g.shape.clone(),
                    Role::OnSurface.paint(),
                ),
                (
                    EN.glance_sheet_checksum,
                    format!("#{}", g.checksum),
                    Role::OnSurface.paint(),
                ),
            ];
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Key(k) => {
            let key = g.keys.get(usize::from(k))?;
            let mut facts = vec![(
                EN.glance_sheet_where_label,
                key.state.clone(),
                if key.here {
                    Role::Ok.paint()
                } else {
                    Role::OnSurfaceVariant.paint()
                },
            )];
            if let Some(l) = &key.label {
                facts.insert(
                    0,
                    (EN.glance_sheet_label, l.clone(), Role::OnSurface.paint()),
                );
            }
            if let Some(h) = &key.holder {
                facts.push((EN.glance_sheet_holder, h.clone(), Role::OnSurface.paint()));
            }
            if let Some(d) = &key.checked {
                facts.push((
                    EN.glance_sheet_last_checked_label,
                    glance::checked_text(d),
                    Role::Ok.paint(),
                ));
            }
            let seed = key.seed;
            let of_seed = |w: What| {
                seed.is_some_and(|s| {
                    matches!(w, What::Words(i) | What::SeedQr(i) | What::Seed(i)
                        | What::SeedPassphrase(i) | What::SeedFile(i)
                        | What::SeedQrFile(i) if i == s)
                })
            };
            let at: Vec<&str> = nodes
                .iter()
                .filter(|n| n.whats.iter().any(|w| of_seed(*w)))
                .map(|n| n.name.as_str())
                .collect();
            if !at.is_empty() {
                facts.push((
                    EN.glance_sheet_its_seed,
                    at.join(" · "),
                    Role::OnSurface.paint(),
                ));
            }
            let mut groups = Vec::new();
            if key.here {
                let mut public = Vec::new();
                if !g.threshold {
                    public.push(row(EN.glance_sheet_show_xpub, "", doing(Step::KeyQr(k))));
                    public.push(row(
                        EN.glance_sheet_xpub_file,
                        EN.glance_sheet_stick_detail,
                        doing(Step::KeyFile(k)),
                    ));
                }
                if key.held
                    && let Some(fp) = key.fp
                {
                    public.push(row(
                        EN.glance_sheet_explore,
                        "",
                        Some(Action::ExploreKey(fp)),
                    ));
                }
                if !app.is_compact() {
                    public.push(row(
                        EN.glance_sheet_where,
                        "",
                        Some(Action::Chart(ChartAction::Where(press, k))),
                    ));
                }
                groups.push((String::new(), public));
                let mut make = Vec::new();
                if let Some(s) = seed {
                    // By hand, at a place: the plan gains the copy, and
                    // the copy's flow makes and checks it.
                    make.push(row(
                        EN.glance_sheet_words,
                        EN.glance_sheet_then_place,
                        open(Target::CopyTo(What::Words(s))),
                    ));
                    make.push(row(
                        EN.glance_sheet_seedqr,
                        EN.glance_sheet_then_place,
                        open(Target::CopyTo(What::SeedQr(s))),
                    ));
                    if let Some(a) = item(Item::Copy(s)) {
                        // Seed XOR's parts are fresh randomness: dimmed
                        // in the demo (`docs/DEMO.md` §4).
                        make.push(row(
                            EN.glance_sheet_seed_xor,
                            EN.glance_sheet_by_hand,
                            app.makes_secrets().then_some(a),
                        ));
                    }
                    let sealed = if key.passphrase {
                        What::SeedPassphrase(s)
                    } else {
                        What::Seed(s)
                    };
                    for n in nodes {
                        if let At::Vault(v) = n.at {
                            let a = if n.whats.iter().any(|w| of_seed(*w)) {
                                item(Item::Vault(v))
                            } else {
                                edit(Edit::Add(n.at, sealed))
                            };
                            if a.is_some() {
                                make.push(row(
                                    fill(EN.glance_sheet_into, &[&n.name.to_string()]),
                                    "",
                                    a,
                                ));
                            }
                        }
                    }
                    let a = if holds(&|w| w == What::SeedFile(s)) {
                        item(Item::SeedFiles)
                    } else {
                        edit(Edit::Add(At::Files, What::SeedFile(s)))
                    };
                    make.push(row(
                        EN.glance_sheet_as_a_file,
                        EN.glance_sheet_stick_detail,
                        a,
                    ));
                }
                if !make.is_empty() {
                    groups.push((EN.glance_sheet_make_another_backup.to_string(), make));
                }
                let mut check = Vec::new();
                if let Some(s) = seed
                    && let Some(fp) = key.fp
                {
                    let forms = plan::CopyForms {
                        words: holds(&|w| w == What::Words(s)),
                        seedqr: holds(&|w| w == What::SeedQr(s)),
                    };
                    for (label, a) in check_rows(forms, s, fp) {
                        check.push(row(label, "", a));
                    }
                }
                if !check.is_empty() {
                    groups.push((EN.glance_sheet_check_seed_copy.to_string(), check));
                }
                if key.passphrase
                    && key.held
                    && let Some(s) = seed
                {
                    groups.push((
                        EN.glance_sheet_its_passphrase.to_string(),
                        vec![
                            row(
                                EN.glance_sheet_check_row,
                                EN.glance_sheet_by_derived_key,
                                open(Target::PassCheck(k, NO_LINE, NO_LINE)),
                            ),
                            row(
                                EN.glance_sheet_another_copy,
                                EN.glance_sheet_then_place,
                                open(Target::CopyTo(What::Passphrase(s))),
                            ),
                        ],
                    ));
                }
            } else {
                // A key not held here: public things, its holder, and the
                // date its holder confirmed its backup (§9.3).
                let mut rows = Vec::new();
                if !g.threshold {
                    rows.push(row(
                        EN.glance_sheet_show_key_xpub,
                        "",
                        doing(Step::CosignerQr(k)),
                    ));
                }
                rows.push(keeping(
                    EN.glance_sheet_name_holder,
                    open(Target::Holder(k)),
                ));
                rows.push(keeping(
                    EN.glance_sheet_mark_checked,
                    chart_do(ChartAction::KeyChecked(press, k)),
                ));
                if let Some(a) = key.action {
                    rows.push(row(
                        EN.glance_sheet_load_this_key,
                        key.way_in.unwrap_or(""),
                        Some(a),
                    ));
                }
                groups.push((String::new(), rows));
            }
            let mut v = View::new(
                Icon::Fingerprint,
                fill(
                    EN.glance_sheet_key,
                    &[&key.number.to_string(), &key.fingerprint.to_string()],
                ),
            );
            v.facts = facts;
            v.groups = groups;
            Some(v)
        }
        Target::Node(n) => {
            let node = nodes.get(usize::from(n))?;
            let mut facts = Vec::new();
            if let Some(s) = &node.stick {
                facts.push((EN.glance_sheet_stick, s.clone(), Role::OnSurface.paint()));
            }
            if let Some(s) = &node.alone {
                facts.push((EN.glance_sheet_found, s.clone(), Role::OnSurface.paint()));
            }
            if let Some((s, t)) = &node.state {
                facts.push((EN.glance_sheet_seen_here, s.clone(), tone_color(*t)));
            }
            if matches!(node.at, At::Place(_)) && !node.whats.is_empty() {
                // The oldest of its things' dates, when each was checked.
                let dates: Option<Vec<&String>> = node.checked.iter().map(Option::as_ref).collect();
                facts.push(match dates.and_then(|d| d.into_iter().min()) {
                    Some(d) => (
                        EN.glance_sheet_last_checked,
                        glance::checked_text(d),
                        Role::Ok.paint(),
                    ),
                    None => (
                        EN.glance_sheet_last_checked,
                        EN.glance_sheet_not_yet.to_string(),
                        Role::OnSurfaceVariant.paint(),
                    ),
                });
            }
            let lines: Vec<Row> = node
                .holds
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < node.whats.len())
                .map(|(j, h)| {
                    let h = match node.marks.get(j).copied().flatten() {
                        Some(m) => format!("{h} · {}", m.label()),
                        None => h.clone(),
                    };
                    row(h, "", open(Target::Line(n, j as u8)))
                })
                .collect();
            let mut rows = Vec::new();
            let icon = match node.at {
                // The chart's nodes are places, their sticks folded in.
                At::Place(p) | At::Stick(p, _) => {
                    rows.push(if vault_open {
                        row(
                            EN.glance_sheet_rename,
                            "",
                            doing(Step::RenamePlace(p as u8)),
                        )
                    } else {
                        let way = app.vault_way(back).map(|(_, a)| a);
                        row(
                            EN.glance_sheet_rename,
                            EN.glance_sheet_open_vault_to_keep,
                            way,
                        )
                    });
                    if !node.whats.is_empty() {
                        rows.push(keeping(
                            EN.glance_sheet_mark_checked,
                            chart_do(ChartAction::Checked(press, n)),
                        ));
                    }
                    if let Some(a) = item(Item::Envelopes) {
                        rows.push(row(EN.glance_sheet_what_goes, "", Some(a)));
                    }
                    rows.push(row(
                        EN.glance_sheet_the_envelope,
                        EN.glance_sheet_envelope_pdf,
                        doing(Step::Envelopes),
                    ));
                    rows.push(row(EN.glance_sheet_add_here, "", open(Target::AddAt(n))));
                    if !node.whats.is_empty() {
                        rows.push(keeping(
                            EN.glance_sheet_mark_lost,
                            chart_do(ChartAction::Mark(press, n, None, Mark::Lost)),
                        ));
                        rows.push(keeping(
                            EN.glance_sheet_mark_exposed,
                            chart_do(ChartAction::Mark(press, n, None, Mark::Exposed)),
                        ));
                    }
                    if places.len() > 1 {
                        rows.push(row(
                            EN.glance_sheet_remove_place,
                            "",
                            open(Target::Remove(n)),
                        ));
                    }
                    Icon::House
                }
                At::Vault(v) => {
                    rows.extend(vault_way());
                    rows.push(keeping(EN.glance_sheet_rename, open(Target::VaultName(n))));
                    if let Some(a) = item(Item::Vault(v)) {
                        rows.push(row(EN.glance_sheet_save_seeds, "", Some(a)));
                    }
                    rows.push(row(
                        EN.glance_sheet_copy_to_stick,
                        EN.glance_sheet_then_place,
                        open(Target::CopyTo(What::VaultStick(v))),
                    ));
                    Icon::Lock
                }
                At::Files => {
                    if node.whats.iter().any(|w| matches!(w, What::SeedFile(_)))
                        && let Some(a) = item(Item::SeedFiles)
                    {
                        rows.push(row(EN.glance_sheet_write_the_seed, "", Some(a)));
                    }
                    if node.whats.contains(&What::Wallet)
                        && let Some(a) = item(Item::PublicFiles)
                    {
                        rows.push(row(EN.glance_sheet_rewrite_public, "", Some(a)));
                    }
                    rows.push(row(
                        EN.glance_sheet_remove_from_plan,
                        EN.glance_sheet_destroy_copies,
                        edit(Edit::Clear(At::Files)),
                    ));
                    Icon::File
                }
                At::Software => {
                    rows.push(row(
                        EN.glance_sheet_reshow_descriptor_qr,
                        "",
                        doing(Step::Qr),
                    ));
                    if let Some(a) = item(Item::PublicFiles) {
                        rows.push(row(EN.glance_sheet_public, "", Some(a)));
                    }
                    Icon::Eye
                }
                At::Away => Icon::Keys,
            };
            let mut groups = Vec::new();
            if !lines.is_empty() {
                groups.push((EN.glance_sheet_contents_group.to_string(), lines));
            }
            if !rows.is_empty() {
                groups.push((String::new(), rows));
            }
            // A vault takes more of what is held here (§9.4).
            if let At::Vault(_) = node.at {
                let mut more = Vec::new();
                for key in g.keys.iter().filter(|k| k.here) {
                    let Some(s) = key.seed else {
                        continue;
                    };
                    let what = if key.passphrase {
                        What::SeedPassphrase(s)
                    } else {
                        What::Seed(s)
                    };
                    if !node.whats.contains(&What::Seed(s))
                        && !node.whats.contains(&What::SeedPassphrase(s))
                    {
                        more.push(row(
                            fill(EN.glance_sheet_key_seed, &[&key.number.to_string()]),
                            if key.passphrase {
                                EN.glance_sheet_with_passphrase
                            } else {
                                ""
                            },
                            edit(Edit::Add(node.at, what)),
                        ));
                    }
                }
                if !node.whats.contains(&What::Wallet) {
                    more.push(row(
                        EN.glance_sheet_the_description,
                        "",
                        edit(Edit::Add(node.at, What::Wallet)),
                    ));
                }
                if !more.is_empty() {
                    groups.push((EN.glance_sheet_save_more.to_string(), more));
                }
            }
            let mut v = View::new(icon, node.name.clone());
            v.facts = facts;
            v.groups = groups;
            Some(v)
        }
        Target::Line(n, j) => {
            let node = nodes.get(usize::from(n))?;
            let what = *node.whats.get(usize::from(j))?;
            let label = node.holds.get(usize::from(j))?.clone();
            let mark = node.marks.get(usize::from(j)).copied().flatten();
            let mut facts = vec![(
                EN.glance_sheet_kept_at_label,
                node.name.clone(),
                Role::OnSurface.paint(),
            )];
            if let Some((s, t)) = &node.state {
                facts.push((EN.glance_sheet_seen_here, s.clone(), tone_color(*t)));
            }
            if let Some(d) = node.checked.get(usize::from(j)).cloned().flatten() {
                facts.push((
                    EN.glance_sheet_last_checked,
                    glance::checked_text(&d),
                    Role::Ok.paint(),
                ));
            }
            if let Some(m) = mark {
                let m = match m {
                    Mark::Lost => EN.glance_sheet_lost,
                    Mark::Exposed => EN.glance_sheet_exposed,
                    Mark::Part => EN.glance_sheet_drawn_in_part,
                };
                facts.push((EN.glance_sheet_marked, m.to_string(), Role::Error.paint()));
            }
            // A SeedQR drawn in part (`docs/history/NEW-WALLET.md` §14.7 item 3):
            // nothing to check or copy; I destroyed this copy takes it off.
            if mark == Some(Mark::Part) {
                let mut v = View::new(Icon::Keys, label);
                v.facts = facts;
                v.groups = vec![(
                    String::new(),
                    vec![row(
                        EN.glance_sheet_i_destroyed,
                        "",
                        edit(Edit::Drop(node.at, what)),
                    )],
                )];
                return Some(v);
            }
            // A secret's actions, with its seed here; else the way to it.
            let secret = |s: usize, rows: &mut Vec<Row>, with: &dyn Fn(&mut Vec<Row>, [u8; 4])| {
                let Some(key) = key_of_seed(s).and_then(|k| g.keys.get(k)) else {
                    return;
                };
                match (key.here, key.fp) {
                    (true, Some(fp)) => with(rows, fp),
                    _ => {
                        if let Some(a) = key.action {
                            rows.push(row(
                                EN.glance_sheet_load_this_key,
                                key.way_in.unwrap_or(""),
                                Some(a),
                            ));
                        }
                    }
                }
            };
            let at_place = matches!(node.at, At::Place(_));
            let mut rows = Vec::new();
            match what {
                What::Words(s) | What::SeedQr(s) => secret(s, &mut rows, &|rows, fp| {
                    let forms = plan::CopyForms {
                        words: matches!(what, What::Words(_)),
                        seedqr: matches!(what, What::SeedQr(_)),
                    };
                    for (label, a) in check_rows(forms, s, fp) {
                        rows.push(row(label, EN.glance_sheet_check_copy, a));
                    }
                    rows.push(row(
                        EN.glance_sheet_make_another,
                        EN.glance_sheet_elsewhere,
                        open(Target::CopyTo(what)),
                    ));
                }),
                What::Seed(s) | What::SeedPassphrase(s) => secret(s, &mut rows, &|_, _| {}),
                // A passphrase record in a vault: nothing to do from here.
                What::PassphraseSealed(_) => {}
                // A file at a place is made and written with its stick
                // (`docs/BACKUP-FLOW.md` §17.12 item 6); a seed's file on
                // the stick of files of a plan saved before, by the seed
                // files item.
                What::SeedFile(_)
                | What::SeedQrFile(_)
                | What::PassphraseFile(_)
                | What::WalletFile(_)
                | What::ShareFile(..)
                | What::SoftwareFile => {
                    let it = if at_place {
                        Item::Sticks
                    } else {
                        Item::SeedFiles
                    };
                    if let Some(a) = item(it) {
                        rows.push(row(
                            EN.glance_sheet_rewrite,
                            EN.glance_sheet_stick_detail,
                            Some(a),
                        ));
                    }
                }
                What::Passphrase(s) => {
                    if let Some(k) = key_of_seed(s)
                        && g.keys.get(k).is_some_and(|key| key.held)
                    {
                        rows.push(row(
                            EN.glance_sheet_check_row,
                            EN.glance_sheet_by_derived_key,
                            open(Target::PassCheck(k as u8, n, j)),
                        ));
                    }
                    rows.push(row(
                        EN.glance_sheet_another_copy,
                        EN.glance_sheet_elsewhere,
                        open(Target::CopyTo(what)),
                    ));
                }
                // A codex32 string: its checklist item checks it, and
                // holds the set's actions (`docs/CODEX32.md` §9.7).
                What::Codex32Share(s, _)
                | What::Codex32QrPaper(s, _)
                | What::Codex32ShareFile(s, _)
                | What::Codex32QrFile(s, _) => {
                    if let Some(a) = item(Item::Codex32(s)) {
                        rows.push(row(EN.c32_glance_check, EN.c32_glance_strings, Some(a)));
                        rows.push(row(EN.c32_glance_set, "", Some(a)));
                    }
                }
                What::Sheet => {
                    rows.push(row(EN.glance_sheet_reprint, "PDF", doing(Step::Out(3))));
                    rows.push(row(EN.glance_sheet_as_a_qr, "PNG", doing(Step::Out(8))));
                    rows.push(row(
                        EN.glance_sheet_as_text,
                        EN.glance_sheet_the_descriptor,
                        doing(Step::Out(1)),
                    ));
                    rows.push(row(
                        EN.glance_sheet_another_copy,
                        EN.glance_sheet_elsewhere,
                        open(Target::CopyTo(what)),
                    ));
                }
                What::Share(_) => {
                    rows.push(row(
                        EN.glance_sheet_print,
                        EN.glance_sheet_pdfs_text,
                        doing(Step::Out(4)),
                    ));
                    if let Some(a) = item(Item::Sheets) {
                        rows.push(row(EN.glance_sheet_keys_left, "", Some(a)));
                    }
                }
                What::VaultStick(_) => {
                    rows.extend(vault_way());
                    rows.push(row(
                        EN.glance_sheet_copy_vault,
                        EN.glance_sheet_then_place,
                        open(Target::CopyTo(what)),
                    ));
                    if app.vault_changed().is_some() {
                        rows.push(row(
                            EN.glance_sheet_write,
                            EN.glance_sheet_changed,
                            Some(Action::WriteAsk),
                        ));
                    }
                }
                What::Wallet => rows.push(row(
                    EN.glance_sheet_show_wallet,
                    EN.glance_sheet_the_descriptor,
                    doing(Step::Qr),
                )),
            }
            // Where it is kept: moved, destroyed, lost or exposed.
            if at_place {
                rows.push(row(
                    EN.glance_sheet_move,
                    EN.glance_sheet_to_another,
                    open(Target::MoveTo(n, j)),
                ));
                rows.push(match mark {
                    Some(_) => row(
                        EN.glance_sheet_remove_from_plan,
                        "",
                        edit(Edit::Drop(node.at, what)),
                    ),
                    None => row(
                        EN.glance_sheet_i_destroyed,
                        "",
                        edit(Edit::Drop(node.at, what)),
                    ),
                });
                if mark.is_none() {
                    rows.push(keeping(
                        EN.glance_sheet_lost,
                        chart_do(ChartAction::Mark(press, n, Some(j), Mark::Lost)),
                    ));
                    rows.push(keeping(
                        EN.glance_sheet_exposed,
                        chart_do(ChartAction::Mark(press, n, Some(j), Mark::Exposed)),
                    ));
                }
            }
            let mut v = View::new(
                match what {
                    What::VaultStick(_) | What::Seed(_) | What::SeedPassphrase(_) => Icon::Lock,
                    What::Passphrase(_) => Icon::Passphrase,
                    What::Words(_) | What::SeedQr(_) => Icon::Keys,
                    _ => Icon::File,
                },
                label,
            );
            v.facts = facts;
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::MoveTo(n, j) => {
            let node = nodes.get(usize::from(n))?;
            let what = *node.whats.get(usize::from(j))?;
            let label = node.holds.get(usize::from(j))?.clone();
            let mut rows: Vec<Row> = places
                .iter()
                .filter(|(_, x)| x.at != node.at && !x.whats.contains(&what))
                .map(|(p, x)| {
                    row(
                        x.name.clone(),
                        "",
                        edit(Edit::Move(node.at, what, At::Place(*p))),
                    )
                })
                .collect();
            rows.push(row(
                EN.glance_sheet_a_new,
                "",
                edit(Edit::Move(node.at, what, At::Place(new_place))),
            ));
            let mut v = View::new(
                Icon::House,
                fill(EN.glance_sheet_move_title, &[&label.to_string()]),
            );
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::CopyTo(what) => {
            let mut rows: Vec<Row> = places
                .iter()
                .filter(|(_, x)| !x.whats.contains(&what) && fits_with(x, what))
                .map(|(p, x)| row(x.name.clone(), "", edit(Edit::Add(At::Place(*p), what))))
                .collect();
            rows.push(row(
                EN.glance_sheet_a_new,
                "",
                edit(Edit::Add(At::Place(new_place), what)),
            ));
            let mut v = View::new(
                Icon::House,
                fill(
                    EN.glance_sheet_another,
                    &[&glance::thing_label(Some(&g), what).to_string()],
                ),
            );
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::AddAt(n) => {
            let node = nodes.get(usize::from(n))?;
            let mut rows = Vec::new();
            for key in g.keys.iter().filter(|k| k.here) {
                let Some(s) = key.seed else {
                    continue;
                };
                for (what, label) in [
                    (
                        What::Words(s),
                        fill(EN.glance_sheet_key_words, &[&key.number.to_string()]),
                    ),
                    (
                        What::SeedQr(s),
                        fill(EN.glance_sheet_key_seedqr, &[&key.number.to_string()]),
                    ),
                    (
                        What::Passphrase(s),
                        fill(EN.glance_sheet_passphrase_n, &[&key.number.to_string()]),
                    ),
                ] {
                    if matches!(what, What::Passphrase(_)) && !(key.passphrase && key.held) {
                        continue;
                    }
                    if !node.whats.contains(&what) && fits_with(node, what) {
                        rows.push(row(label, "", edit(Edit::Add(node.at, what))));
                    }
                }
            }
            if split {
                for j in 0..g.keys.len() {
                    let what = What::Share(j);
                    if !node.whats.contains(&what) {
                        rows.push(row(
                            fill(EN.glance_sheet_share, &[&(j + 1).to_string()]),
                            "",
                            edit(Edit::Add(node.at, what)),
                        ));
                    }
                }
            } else if !node.whats.contains(&What::Sheet) {
                rows.push(row(
                    EN.glance_sheet_wallet,
                    "",
                    edit(Edit::Add(node.at, What::Sheet)),
                ));
            }
            for x in nodes {
                if let At::Vault(v) = x.at
                    && !node.whats.contains(&What::VaultStick(v))
                {
                    rows.push(row(
                        fill(EN.glance_sheet_stick_n, &[&x.name.to_string()]),
                        EN.glance_sheet_a_copy,
                        edit(Edit::Add(node.at, What::VaultStick(v))),
                    ));
                }
            }
            let mut v = View::new(
                Icon::House,
                fill(EN.glance_sheet_add_title, &[&node.name.to_string()]),
            );
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Confirm => {
            let ask = app.glance.work().ask.as_ref()?;
            let mut v = View::new(Icon::House, ask.title.clone());
            v.facts = check_facts(ask.before, ask.after);
            if let Edit::Drop(..) | Edit::Clear(_) | Edit::RemovePlace(_) = ask.edit {
                v.lines.push((
                    EN.glance_sheet_destroy.to_string(),
                    Role::OnSurfaceVariant.paint(),
                ));
            }
            v.go = Some((
                EN.glance_sheet_confirm.to_string(),
                chart_do(ChartAction::Confirm(press)),
            ));
            Some(v)
        }
        Target::Marked(n, line) => {
            let node = nodes.get(usize::from(n))?;
            let (shape, answers) = g.plan.as_ref()?;
            let f = plan::follows_on(shape, answers, &app.plan_mapped(shape, answers), &marks);
            let title = match line {
                Some(j) => node.holds.get(usize::from(j))?.clone(),
                None => node.name.clone(),
            };
            let mut v = View::new(Icon::Warning, title);
            let mut rows = Vec::new();
            // What this thing, or this node's things, are marked: what
            // follows from losing them, from their being found, or both.
            let here: Vec<(What, Mark)> = node
                .whats
                .iter()
                .zip(&node.marks)
                .enumerate()
                .filter(|(i, _)| line.is_none_or(|j| usize::from(j) == *i))
                .filter_map(|(_, (w, m))| m.map(|m| (*w, m)))
                .collect();
            let lost = here.iter().any(|(_, m)| *m == Mark::Lost);
            let exposed = here.iter().any(|(_, m)| *m == Mark::Exposed);
            let f = plan::Follows {
                any_lost: f.any_lost && lost,
                found: if exposed {
                    f.found
                } else {
                    plan::Alone {
                        spend: plan::Found::No,
                        balance: plan::Found::No,
                    }
                },
                ..f
            };
            // In order of what to do (§9.6).
            match f.found.spend {
                plan::Found::Yes => v
                    .lines
                    .push((EN.glance_sheet_someone_can.to_string(), Role::Error.paint())),
                plan::Found::OnlyWithVault => v.lines.push((
                    EN.glance_sheet_someone_could.to_string(),
                    Role::Error.paint(),
                )),
                plan::Found::No => {}
            }
            if f.found.spend != plan::Found::No {
                rows.push(row(
                    EN.glance_sheet_move_money,
                    EN.glance_sheet_new_wallet,
                    chart_do(ChartAction::MoveMoney(press)),
                ));
            }
            if f.any_lost && f.rebuilds == plan::Lost::No {
                v.lines
                    .push((EN.glance_sheet_the_wallet.to_string(), Role::Error.paint()));
                rows.push(row(EN.glance_sheet_back_up, "", change_plan()));
            }
            if f.found.spend == plan::Found::No && f.found.balance != plan::Found::No {
                v.lines
                    .push((EN.glance_sheet_someone.to_string(), Role::Warning.paint()));
            }
            if f.any_lost && f.elsewhere {
                v.lines
                    .push((EN.glance_sheet_nothing_lost.to_string(), Role::Ok.paint()));
                let what = here.iter().find(|(_, m)| *m == Mark::Lost).map(|(w, _)| *w);
                if let Some(what) = what {
                    rows.push(row(
                        EN.glance_sheet_make_replacement,
                        "",
                        open(Target::CopyTo(what)),
                    ));
                }
            }
            if v.lines.is_empty() && exposed {
                v.lines
                    .push((EN.glance_sheet_nothing.to_string(), Role::Ok.paint()));
            }
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Remove(n) => {
            let node = nodes.get(usize::from(n))?;
            let At::Place(p) = node.at else {
                return None;
            };
            let mut v = View::new(
                Icon::House,
                fill(EN.glance_sheet_remove, &[&node.name.to_string()]),
            );
            for (j, label) in node.holds.iter().enumerate().take(node.whats.len()) {
                let chosen = app.glance.work().dest.get(j).copied().flatten();
                let mut rows: Vec<Row> = places
                    .iter()
                    .filter(|(q, _)| *q != p)
                    .map(|(q, x)| {
                        row(
                            x.name.clone(),
                            if chosen == Some(*q as u8) {
                                EN.glance_sheet_chosen
                            } else {
                                ""
                            },
                            chart_do(ChartAction::Dest(j as u8, *q as u8)),
                        )
                    })
                    .collect();
                rows.push(row(
                    EN.glance_sheet_nowhere,
                    if chosen == Some(OFF) {
                        EN.glance_sheet_chosen
                    } else {
                        EN.glance_sheet_destroyed
                    },
                    chart_do(ChartAction::Dest(j as u8, OFF)),
                ));
                v.groups.push((
                    fill(EN.glance_sheet_goes_to_group, &[&label.to_string()]),
                    rows,
                ));
            }
            let all = (0..node.whats.len())
                .all(|j| app.glance.work().dest.get(j).copied().flatten().is_some());
            v.go = Some((
                fill(EN.glance_sheet_remove, &[&node.name.to_string()]),
                all.then_some(Action::Chart(ChartAction::Edit(
                    press,
                    Edit::RemovePlace(p),
                ))),
            ));
            Some(v)
        }
        Target::Holder(k) => {
            let key = g.keys.get(usize::from(k))?;
            let mut v = View::new(
                Icon::Fingerprint,
                fill(
                    EN.glance_sheet_key,
                    &[&key.number.to_string(), &key.fingerprint.to_string()],
                ),
            );
            let t = &app.glance.work().text;
            v.field = Some(if t.is_empty() {
                (
                    EN.glance_sheet_holder,
                    EN.glance_sheet_name.to_string(),
                    true,
                )
            } else {
                (EN.glance_sheet_holder, t.clone(), false)
            });
            v.go = Some((
                EN.glance_sheet_keep.to_string(),
                chart_do(ChartAction::Save(press)),
            ));
            Some(v)
        }
        Target::VaultName(n) => {
            let node = nodes.get(usize::from(n))?;
            let mut v = View::new(Icon::Lock, node.name.clone());
            let t = &app.glance.work().text;
            v.field = Some(if t.is_empty() {
                (EN.glance_sheet_name, EN.glance_sheet_name.to_string(), true)
            } else {
                (EN.glance_sheet_name, t.clone(), false)
            });
            v.go = Some((
                EN.glance_sheet_keep.to_string(),
                chart_do(ChartAction::Save(press)),
            ));
            Some(v)
        }
        Target::PassCheck(k, ..) => {
            let key = g.keys.get(usize::from(k))?;
            let mut v = View::new(
                Icon::Passphrase,
                fill(EN.glance_sheet_passphrase_n, &[&key.number.to_string()]),
            );
            let typed = app.glance.work().secret.chars().count();
            v.field = Some(if typed == 0 {
                (
                    EN.glance_sheet_passphrase,
                    EN.glance_sheet_type_passphrase.to_string(),
                    true,
                )
            } else {
                (EN.glance_sheet_passphrase, "•".repeat(typed), false)
            });
            match app.glance.work().result {
                Some(true) => v.lines.push((
                    fill(EN.glance_sheet_matches, &[&key.fingerprint.to_string()]),
                    Role::Ok.paint(),
                )),
                Some(false) => v
                    .lines
                    .push((EN.glance_sheet_mismatch.to_string(), Role::Error.paint())),
                None => {}
            }
            v.go = Some((
                EN.glance_sheet_check_button.to_string(),
                (typed > 0).then_some(Action::Chart(ChartAction::PassTry(press))),
            ));
            Some(v)
        }
        Target::PassShow(k, n, j) => {
            let key = g.keys.get(usize::from(k))?;
            let mut v = View::new(
                Icon::Passphrase,
                fill(EN.glance_sheet_passphrase_n, &[&key.number.to_string()]),
            );
            let shown = key.fp.and_then(|fp| {
                app.session
                    .keys
                    .position_of(fp)
                    .filter(|&x| app.session.keys[x].has_passphrase)
            });
            match (shown, app.glance.work().reveal) {
                (Some(x), true) => v.secret_lines.push((
                    zeroize::Zeroizing::new(app.reveal(x, Reveal::Passphrase).to_string()),
                    Role::OnSurface.paint(),
                )),
                (Some(_), false) => v.groups.push((
                    String::new(),
                    vec![row(
                        EN.glance_sheet_show_row,
                        EN.glance_sheet_to_copy,
                        chart_do(ChartAction::Reveal),
                    )],
                )),
                (None, _) => v.lines.push((
                    EN.glance_sheet_load_with_passphrase.to_string(),
                    Role::OnSurfaceVariant.paint(),
                )),
            }
            v.go = Some((
                EN.glance_sheet_check.to_string(),
                open(Target::PassCheck(k, n, j)),
            ));
            Some(v)
        }
        Target::Addresses => {
            let Press::Loaded(w) = press else {
                return None;
            };
            let mut v = View::new(Icon::Wallet, g.name.clone());
            v.addresses = Some(w);
            Some(v)
        }
    }
}

/// Whether `what` may go at place node `node`: a passphrase never shares
/// a place with its words, nor its words with it.
fn fits_with(node: &BackupNode, what: What) -> bool {
    match what {
        What::Passphrase(i) => !node
            .whats
            .iter()
            .any(|w| *w == What::Words(i) || *w == What::SeedQr(i)),
        What::Words(i) | What::SeedQr(i) => !node.whats.contains(&What::Passphrase(i)),
        _ => true,
    }
}

fn tone_color(t: crate::wallet_backup::Tone) -> Paint {
    use crate::wallet_backup::Tone;
    match t {
        Tone::Ok => Role::Ok.paint(),
        Tone::Warn => Role::Warning.paint(),
        Tone::Err => Role::Error.paint(),
        Tone::Dim => Role::OnSurfaceVariant.paint(),
    }
}

/// Whether a chart's sheet stands at the window's right as Material's
/// standard side sheet: on the wide layout, beside the expanded
/// navigation rail (`docs/DESIGN.md` §4.16). On the rail's and the bar's
/// windows, where the chart is a column with no lines, it is the
/// centred sheet or the small panel's page. A lookup.
pub(crate) fn side(app: &Faraday) -> bool {
    !app.is_compact() && !app.held
}

/// The side sheet to draw this frame: its chart and target, how far in
/// it stands (0 to 1), and whether it is the open one, which takes
/// presses, or one leaving. `None` where none stands. A lookup.
pub(crate) fn side_now(app: &Faraday) -> Option<((Press, Target), f32, bool)> {
    use crate::motion::{SIDE_SHEET_IN_MS, SIDE_SHEET_OUT_MS, ease, ease_open, progress};
    if !side(app) {
        return None;
    }
    let m = app.glance.side();
    let reduce = app.reduce_motion();
    if app.sheet == Some(Sheet::Chart)
        && let Some(open) = app.glance.chart()
    {
        // Opened since the last tick: just coming in.
        let shown = match m.shown {
            _ if reduce => 1.0,
            Some((_, _, Some(at))) => ease_open(progress(Some(at), app.now_ms, SIDE_SHEET_IN_MS)),
            _ => 0.0,
        };
        return Some((open, shown, true));
    }
    if reduce {
        return None;
    }
    // Closed since the last tick: still all the way in.
    let (seen, gone) = match (m.shown, m.leaving) {
        (Some(s), _) => (s, 0.0),
        (None, Some(l)) => (
            l,
            ease(
                tokens::EASE_EMPHASIZED_ACCELERATE,
                progress(l.2, app.now_ms, SIDE_SHEET_OUT_MS),
            ),
        ),
        (None, None) => return None,
    };
    (seen.1 == app.screen && gone < 1.0).then_some((seen.0, 1.0 - gone, false))
}

/// The sheet over the chart. On a small panel it is the page itself, and
/// Back returns to the chart.
pub(crate) fn draw(app: &crate::drawing::View, ui: &mut Ui, w: f32, h: f32) {
    let Some((press, target)) = app.glance.chart() else {
        return;
    };
    let Some(v) = app.glance_sheet(press, target) else {
        return;
    };
    let (place, margin, pad) = if ui.compact {
        ((0.0, w), 0.0, tokens::PAD)
    } else {
        let sw = (w - 2.0 * tokens::TOUCH).min(tokens::CHART_SHEET_WIDTH);
        (((w - sw) / 2.0, sw), tokens::PAD, 2.0 * tokens::PAD)
    };
    let close = if ui.compact {
        EN.glance_sheet_back
    } else {
        EN.glance_sheet_close
    };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let head = crate::compact::sheet_head(ui, x, y, iw, v.icon, Role::Primary, &v.title);
        head + body(app, ui, &v, (x, y + head, iw), Some(close))
    });
}

/// The sheet beside the chart ([`side_now`]): Material's standard side
/// sheet at the window's right, `w` by `h`, from under the top-right
/// strip to the foot, the chart and its lines in view beside it and
/// still taking presses. One leaving takes none.
pub(crate) fn draw_side(
    app: &crate::drawing::View,
    ui: &mut Ui,
    ((press, target), shown, live): ((Press, Target), f32, bool),
    w: f32,
    h: f32,
) {
    let Some(v) = app.glance_sheet(press, target) else {
        return;
    };
    let mark = (
        ui.hits.len(),
        ui.reach.len(),
        ui.scrolled.clone(),
        ui.offset,
    );
    if live {
        ui.in_sheet = true;
        ui.offset = app.sheet_offset;
    } else {
        ui.offset = 0.0;
    }
    let top = tokens::SUPPORT_PANE_TOP;
    faraday_ui::sheet::side_sheet(
        ui,
        (w, top, tokens::SIDE_SHEET_W, h - top),
        shown,
        (v.icon.into(), Role::Primary.paint(), &v.title),
        (EN.glance_sheet_close, Action::Cancel),
        &mut |ui, x, y, iw| body(app, ui, &v, (x, y, iw), None),
    );
    ui.in_sheet = false;
    if !live {
        ui.hits.truncate(mark.0);
        ui.reach.truncate(mark.1);
        ui.scrolled = mark.2;
        ui.offset = mark.3;
    }
}

/// What a chart's sheet holds under its head, at (x, y) in `iw`: its
/// facts, lines, field, addresses and rows, then its own button and
/// `close` (none on the side sheet, whose header holds it). Returns its
/// height.
fn body(
    app: &crate::drawing::View,
    ui: &mut Ui,
    v: &View,
    (x, y, iw): (f32, f32, f32),
    close: Option<&str>,
) -> f32 {
    let mut cy = y;
    for (label, value, tone) in &v.facts {
        cy += crate::compact::kv(ui, x, cy, iw, label, value, *tone);
    }
    for (line, tone) in &v.lines {
        cy += ui.wrap_as((x, cy, iw), LABEL_L, *tone, line) + tokens::GAP_SMALL;
    }
    for (line, tone) in &v.secret_lines {
        cy += ui.wrap_as((x, cy, iw), LABEL_L, *tone, line) + tokens::GAP_SMALL;
    }
    if let Some((label, text, hint)) = &v.field {
        cy += tokens::GAP;
        cy += field(ui, x, cy, iw, label, text, *hint);
    }
    if let Some(wi) = v.addresses {
        if let Some(wl) = app.session.wallets.get(wi) {
            cy += crate::screens::descriptor_row(ui, x, cy, iw, wl, Action::QrWallet(wi));
        }
        cy += crate::screens::first_addresses(app, ui, wi, x, cy, iw);
    }
    for (heading, rows) in &v.groups {
        if rows.is_empty() {
            continue;
        }
        cy += tokens::GAP;
        if !heading.is_empty() {
            cy += ui.wrap_as((x, cy, iw), LABEL_M, Role::OnSurfaceVariant, heading) + tokens::GAP;
        }
        for r in rows {
            cy += menu_row(ui, x, cy, iw, r) + tokens::GAP_SMALL;
        }
    }
    cy += tokens::GAP;
    let mut buttons: Vec<(&str, Style, Action)> = Vec::new();
    if let Some((label, Some(a))) = &v.go {
        buttons.push((label.as_str(), Style::Primary, *a));
    }
    if let Some(close) = close {
        buttons.push((close, Style::Secondary, Action::Cancel));
    }
    cy += crate::compact::buttons(ui, x, cy, iw, &buttons);
    if let Some((label, None)) = &v.go {
        cy += tokens::GAP;
        cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurfaceVariant, label);
    }
    cy - y
}

/// A field being typed: its label over a box with what it shows and the
/// caret. Returns its height.
fn field(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, text: &str, hint: bool) -> f32 {
    let lh = ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, label) + FIELD_GAP;
    let h = FIELD_H;
    let by = y + lh;
    ui.field_box((x, by, w), true, None);
    let shown = ui.fit_as(BODY_L, text, w - 2.0 * FIELD_PAD);
    let tw = ui.text_mid_as(
        (x + FIELD_PAD, by, h),
        BODY_L,
        if hint {
            Role::OnSurfaceVariant
        } else {
            Role::OnSurface
        },
        &shown,
    );
    let cx = x + FIELD_PAD + if hint { 0.0 } else { tw };
    ui.caret(cx, by + (h - BODY_L.size) / 2.0, BODY_L.size);
    lh + h
}

/// One action row: its label, its value at the right in the caption
/// type, a chevron where it can be pressed, dimmed where it cannot.
/// Returns its height.
fn menu_row(ui: &mut Ui, x: f32, y: f32, w: f32, r: &Row) -> f32 {
    let h = tokens::CHART_SHEET_ROW;
    let pressed = r.action.is_some_and(|a| ui.is_pressed(a));
    crate::ui::fill_pressed(
        ui,
        (x, y, w, h, tokens::RADIUS_SMALL),
        Role::SurfaceContainerLow,
        pressed,
    );
    ui.stroke(x, y, w, h, tokens::RADIUS_SMALL, Role::OutlineVariant);
    let pad = tokens::PAD;
    let chevron = if r.action.is_some() {
        tokens::ICON_SMALL
    } else {
        0.0
    };
    let room = w - 2.0 * pad - chevron;
    let sub_w = if r.sub.is_empty() {
        0.0
    } else {
        ui.measure_as(BODY_S, &r.sub).min(room / 2.0)
    };
    let label_room = room - sub_w - if sub_w > 0.0 { tokens::GAP } else { 0.0 };
    let label = ui.fit_as(LABEL_L, &r.label, label_room);
    let tone = if r.action.is_some() {
        Role::OnSurface
    } else {
        Role::OnSurfaceVariant
    };
    ui.text_mid_as((x + pad, y, h), LABEL_L, tone, &label);
    if sub_w > 0.0 {
        let sub = ui.fit_as(BODY_S, &r.sub, sub_w);
        ui.text_right_as(
            (x + w - pad - chevron, y, h),
            BODY_S,
            Role::OnSurfaceVariant,
            &sub,
        );
    }
    if let Some(a) = r.action {
        let iy = y + (h - tokens::ICON_SMALL) / 2.0;
        ui.icon(
            x + w - pad - chevron,
            iy,
            tokens::ICON_SMALL,
            Icon::ChevronRight,
            tokens::GRID,
            Role::OnSurfaceVariant,
        );
        ui.hit(x, y, w, h, a);
    }
    h
}

impl Faraday {
    /// Whether the wallet's chart is open as its own page.
    pub fn glance(&self) -> bool {
        self.glance.open()
    }

    /// The chart and the thing on it whose sheet is open.
    pub fn chart(&self) -> Option<(Press, Target)> {
        self.glance.chart()
    }

    /// What the chart's sheets hold between presses.
    pub fn chart_work(&self) -> &crate::chart_edit::Work {
        self.glance.work()
    }

    /// What a press on the chart or its sheets does.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn chart_act(&mut self, a: ChartAction) {
        let c = self.compute();
        match a {
            ChartAction::Open(press, target) => {
                // A sheet that types or chooses starts from what is kept.
                match target {
                    Target::Remove(n) => {
                        let things = chart(&glance::Now { app: self, c: &c }, press)
                            .and_then(|g| match g.backup {
                                Backup::Plan { nodes, .. } => {
                                    nodes.get(usize::from(n)).map(|x| x.whats.len())
                                }
                                _ => None,
                            })
                            .unwrap_or(0);
                        self.glance.work_mut().dest = vec![None; things];
                    }
                    Target::Holder(k) => {
                        self.glance.work_mut().text =
                            chart(&glance::Now { app: self, c: &c }, press)
                                .and_then(|g| {
                                    g.keys.get(usize::from(k)).and_then(|k| k.holder.clone())
                                })
                                .unwrap_or_default();
                    }
                    Target::VaultName(n) => {
                        self.glance.work_mut().text =
                            chart(&glance::Now { app: self, c: &c }, press)
                                .and_then(|g| match g.backup {
                                    Backup::Plan { nodes, .. } => {
                                        nodes.get(usize::from(n)).map(|x| x.name.clone())
                                    }
                                    _ => None,
                                })
                                .filter(|name| {
                                    !name.starts_with(
                                        EN.glance_vault.split("{}").next().unwrap_or_default(),
                                    )
                                })
                                .unwrap_or_default();
                    }
                    Target::PassCheck(..) => {
                        self.glance.work_mut().secret.clear();
                        self.glance.work_mut().result = None;
                    }
                    Target::PassShow(..) => self.glance.work_mut().reveal = false,
                    _ => {}
                }
                if target != Target::Confirm {
                    self.glance.work_mut().ask = None;
                }
                // Beside the chart, a sheet newly open slides in from now;
                // another node's replaces it where it stands.
                if side(self) && self.sheet != Some(Sheet::Chart) {
                    self.glance.set_side(glance::SideMotion {
                        shown: Some(((press, target), self.screen, Some(self.now_ms))),
                        leaving: None,
                    });
                }
                self.glance.set_chart(Some((press, target)));
                self.glance.set_focus(None);
                self.sheet = Some(Sheet::Chart);
                self.scroll.reset_sheet();
            }
            ChartAction::Where(press, k) => {
                // Beside the chart the sheet stays: the lines show
                // beside it (`docs/DESIGN.md` §4.16).
                if !side(self) {
                    self.close_chart();
                }
                self.glance.set_focus(Some((press, usize::from(k))));
            }
            ChartAction::Edit(press, e) => self.chart_ask(press, e),
            ChartAction::Confirm(_) => self.chart_confirm(),
            ChartAction::Mark(press, n, j, m) => self.chart_mark(press, n, j, m),
            ChartAction::Checked(press, n) => self.chart_checked(press, n),
            ChartAction::KeyChecked(press, k) => self.chart_key_checked(press, k),
            ChartAction::Dest(j, p) => {
                let j = usize::from(j);
                if self.glance.work_mut().dest.len() <= j {
                    self.glance.work_mut().dest.resize(j + 1, None);
                }
                self.glance.work_mut().dest[j] = Some(p);
            }
            ChartAction::Save(press) => self.chart_save_name(press),
            ChartAction::PassTry(press) => self.chart_pass_try(press),
            ChartAction::Reveal => self.glance.work_mut().reveal = true,
            ChartAction::MoveMoney(press) => self.chart_move_money(press),
            ChartAction::Do(press, step) => {
                self.close_chart();
                let Some(w) = self.press_wallet(press) else {
                    return;
                };
                let from_vault = matches!(press, Press::Vault(..));
                match step {
                    Step::Qr => self.act(Action::QrWallet(w)),
                    Step::Out(what) => self.public_out(w, what),
                    Step::KeyQr(k) => self.act(Action::ShowCode(crate::Code::WalletKey(w, k))),
                    Step::CosignerQr(k) => {
                        self.act(Action::ShowCode(crate::Code::CosignerKey(w, k)))
                    }
                    Step::KeyFile(k) => self.act(Action::WalletKeyOut(w, k)),
                    Step::Item(n) => {
                        if self.chart_checklist(w, from_vault) {
                            self.backup_item_open(Some(n));
                            self.screen = Screen::Backup;
                        }
                    }
                    Step::Scan(fp) => {
                        // The copy is checked against the seed, and the
                        // chart stays under the camera.
                        let k = self.session.keys.all().position(|k| k.fingerprint.0 == fp);
                        if let Some(k) = k
                            && self.chart_checklist(w, from_vault)
                        {
                            self.act(Action::BKey(k));
                            self.act(Action::BScan);
                        }
                    }
                    Step::Type(fp, n) => {
                        // The words of the copy typed back, at its item.
                        let k = self.session.keys.all().position(|k| k.fingerprint.0 == fp);
                        if let Some(k) = k
                            && self.chart_checklist(w, from_vault)
                        {
                            self.backup_item_open(Some(n));
                            self.act(Action::BKey(k));
                            self.act(Action::BCheck);
                            self.screen = Screen::Backup;
                        }
                    }
                    Step::Plan => self.chart_plan(w, from_vault, None),
                    Step::RenamePlace(p) => {
                        self.chart_plan(w, from_vault, Some(crate::qstep::BOARD));
                        self.act(Action::BName(p));
                    }
                    Step::Rename => {
                        self.act(Action::OpenWallet(w));
                        self.act(Action::Rename);
                    }
                    Step::Addresses => {
                        self.glance
                            .set_chart(Some((Press::Loaded(w), Target::Addresses)));
                        self.sheet = Some(Sheet::Chart);
                        self.scroll.reset_sheet();
                    }
                    Step::Envelopes => self.envelopes_out(w),
                }
            }
        }
    }

    /// Keeps the side sheet's coming and going ([`side_now`]) in step
    /// with the chart's open sheet. Returns whether a frame is wanted for
    /// it. A lookup.
    pub(crate) fn chart_side_tick(&mut self, now: u64) -> bool {
        use crate::motion::{SIDE_SHEET_IN_MS, SIDE_SHEET_OUT_MS};
        let open = if side(self) && self.sheet == Some(Sheet::Chart) {
            self.glance.chart()
        } else {
            None
        };
        let mut m = self.glance.side();
        let reduce = self.reduce_motion();
        match (m.shown, open) {
            (None, Some(c)) => {
                m.shown = Some((c, self.screen, Some(now)));
                m.leaving = None;
            }
            (Some((_, s, at)), Some(c)) => m.shown = Some((c, s, at.or(Some(now)))),
            (Some((c, s, _)), None) => {
                m.shown = None;
                m.leaving = (!reduce).then_some((c, s, Some(now)));
            }
            (None, None) => {}
        }
        let leaving = m.leaving.is_some();
        if m.leaving
            .is_some_and(|l| l.2.is_some_and(|at| now >= at + SIDE_SHEET_OUT_MS))
        {
            m.leaving = None;
        }
        let coming = m.shown.is_some_and(|s| {
            s.2.is_some_and(|at| now < at + SIDE_SHEET_IN_MS + tokens::MOTION_SHORT1)
        });
        self.glance.set_side(m);
        !reduce && (coming || leaving)
    }

    fn close_chart(&mut self) {
        if self.sheet == Some(Sheet::Chart) {
            self.sheet = None;
        }
        self.glance.set_chart(None);
    }

    /// The backup of wallet `w` with the plan its chart shows, at its
    /// checklist; nothing is made For the stick, each item offering what
    /// it makes. False when no plan is known.
    pub(crate) fn chart_checklist(&mut self, w: usize, from_vault: bool) -> bool {
        if !self.chart_backup(w, from_vault) {
            return false;
        }
        self.backup.checklist_closed();
        self.backups_sync();
        true
    }

    /// The backup of wallet `w` at its plan's questions, question `q`
    /// open, with the plan its chart shows.
    fn chart_plan(&mut self, w: usize, from_vault: bool, q: Option<u8>) {
        if !self.chart_backup(w, from_vault) {
            self.act(Action::Backup(w));
            self.backup.set_from_vault(from_vault);
        }
        self.backup.back_to_plan(q);
        self.screen = Screen::Backup;
    }

    /// The backup state for wallet `w`, holding the plan its chart shows:
    /// the backup under way when it is this wallet's checklist, else that
    /// plan put into it. False when no plan is known and no backup of it
    /// is under way.
    pub(crate) fn chart_backup(&mut self, w: usize, from_vault: bool) -> bool {
        let c = self.compute();
        let known = glance::plan_of_now(&c, self, w);
        let mine = self.backup.state().is_some_and(|b| b.wallet == w);
        if !mine {
            if known.is_none() {
                return false;
            }
            let screen = self.screen;
            self.act(Action::Backup(w));
            self.screen = screen;
        }
        self.backup.chart_plan(from_vault, known)
    }
}
