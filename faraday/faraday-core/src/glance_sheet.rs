//! Acting on the chart (`docs/NEW-WALLET.md` §9): a press on a node of
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

use osk_ui::widgets::Icon;
use osk_ui::{Color, tokens};

use crate::chart_edit::OFF;
use crate::glance::{self, Backup, BackupNode, Glance, Press};
use crate::plan::{self, At, Edit, Item, Mark, What};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vaults::VaultAction as V;
use crate::{Action, BStage, Faraday, Screen, Sheet, bstep};

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
    /// Change the plan, which replaces the changes made on the chart.
    Replan,
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
    /// Change the plan, the changes on the chart replaced.
    Replan(Press),
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
    pub facts: Vec<(&'static str, String, Color)>,
    pub lines: Vec<(String, Color)>,
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

/// The chart a press was made on.
fn chart(app: &Faraday, press: Press) -> Option<Glance> {
    match press {
        Press::Loaded(w) => glance::of(app, w),
        Press::Vault(v, r) => glance::of_vault(app, v, r),
    }
}

/// Where backup node `n` of the chart of `press` is.
pub(crate) fn node_at(app: &Faraday, press: Press, n: u8) -> Option<At> {
    match chart(app, press)?.backup {
        Backup::Plan { nodes, .. } => nodes.get(usize::from(n)).map(|x| x.at),
        _ => None,
    }
}

/// How a check line reads before and after a change, and its tone: a
/// line made worse in `WARN`.
fn check_facts(before: plan::Check, after: plan::Check) -> Vec<(&'static str, String, Color)> {
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
                if worse { WARN } else { TEXT },
            )
        })
        .collect()
}

/// The sheet for `target` on the chart of `press`; `None` when it is no
/// longer there.
pub(crate) fn view(app: &Faraday, press: Press, target: Target) -> Option<View> {
    let g = chart(app, press)?;
    let chart_do = |a: ChartAction| Some(Action::Chart(a));
    let doing = |s: Step| chart_do(ChartAction::Do(press, s));
    let open = |t: Target| chart_do(ChartAction::Open(press, t));
    let edit = |e: Edit| chart_do(ChartAction::Edit(press, e));
    let (nodes, items, edited): (&[BackupNode], &[Item], bool) = match &g.backup {
        Backup::Plan {
            nodes,
            items,
            edited,
            ..
        } => (nodes, items, *edited),
        _ => (&[], &[], false),
    };
    // The checklist item that makes `it`, where the plan has one.
    let item = |it: Item| {
        items
            .contains(&it)
            .then(|| doing(Step::Item(bstep::of(it))))
            .flatten()
    };
    let holds = |f: &dyn Fn(What) -> bool| nodes.iter().any(|n| n.whats.iter().any(|w| f(*w)));
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
                "Open the vault",
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
                "Open a vault to keep this",
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
    // Change the plan: with the map edited on the chart, it says first
    // that those changes are replaced.
    let change_plan = || {
        if edited {
            open(Target::Replan)
        } else {
            doing(Step::Plan)
        }
    };
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
                    "Open in Wallets",
                    "",
                    Some(Action::Vault(V::OpenWalletOf(v, r))),
                ));
            }
            rows.push(row("Show wallet QR", "The descriptor", doing(Step::Qr)));
            rows.push(row(
                "Check addresses",
                "The first three",
                doing(Step::Addresses),
            ));
            if let Some(a) = item(Item::PublicFiles) {
                rows.push(row("Public files", "For the software", Some(a)));
            }
            if matches!(g.backup, Backup::Plan { .. }) {
                let (label, sub, what) = if split {
                    ("Print the shares again", "PDFs, text and pictures", 4)
                } else {
                    ("Print the wallet sheet again", "PDF", 3)
                };
                rows.push(row(label, sub, doing(Step::Out(what))));
            }
            rows.push(row("Rename the wallet", "", doing(Step::Rename)));
            rows.push(row("Change the plan", "", change_plan()));
            let mut v = View::new(Icon::Wallet, g.name.clone());
            v.facts = vec![
                ("Shape", g.shape.clone(), TEXT),
                ("Checksum", format!("#{}", g.checksum), TEXT),
            ];
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Key(k) => {
            let key = g.keys.get(usize::from(k))?;
            let mut facts = vec![(
                "Where",
                key.state.clone(),
                if key.here { OK } else { MUTED },
            )];
            if let Some(l) = &key.label {
                facts.insert(0, ("Label", l.clone(), TEXT));
            }
            if let Some(h) = &key.holder {
                facts.push(("Holder", h.clone(), TEXT));
            }
            if let Some(d) = &key.checked {
                facts.push(("Last checked", glance::checked_text(d), OK));
            }
            let seed = key.seed;
            let of_seed = |w: What| {
                seed.is_some_and(|s| {
                    matches!(w, What::Words(i) | What::SeedQr(i) | What::Seed(i)
                        | What::SeedPassphrase(i) | What::SeedFile(i) if i == s)
                })
            };
            let at: Vec<&str> = nodes
                .iter()
                .filter(|n| n.whats.iter().any(|w| of_seed(*w)))
                .map(|n| n.name.as_str())
                .collect();
            if !at.is_empty() {
                facts.push(("Its seed is at", at.join(" · "), TEXT));
            }
            let mut groups = Vec::new();
            if key.here {
                let mut public = Vec::new();
                if !g.threshold {
                    public.push(row("Show xpub QR", "", doing(Step::KeyQr(k))));
                    public.push(row("Xpub file", "For the stick", doing(Step::KeyFile(k))));
                }
                if key.held
                    && let Some(fp) = key.fp
                {
                    public.push(row("Explore this key", "", Some(Action::ExploreKey(fp))));
                }
                if !app.is_compact() {
                    public.push(row(
                        "Where it is",
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
                        "Words, by hand",
                        "Then a place",
                        open(Target::CopyTo(What::Words(s))),
                    ));
                    make.push(row(
                        "SeedQR, by hand",
                        "Then a place",
                        open(Target::CopyTo(What::SeedQr(s))),
                    ));
                    if let Some(a) = item(Item::Copy(s)) {
                        make.push(row("Seed XOR or codex32", "By hand", Some(a)));
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
                                make.push(row(format!("Into {}", n.name), "", a));
                            }
                        }
                    }
                    let a = if holds(&|w| w == What::SeedFile(s)) {
                        item(Item::SeedFiles)
                    } else {
                        edit(Edit::Add(At::Files, What::SeedFile(s)))
                    };
                    make.push(row("As a file", "For the stick", a));
                }
                if !make.is_empty() {
                    groups.push(("Make another backup of this seed".to_string(), make));
                }
                let mut check = Vec::new();
                let by_hand =
                    seed.is_some_and(|s| holds(&|w| w == What::Words(s) || w == What::SeedQr(s)));
                if by_hand && let Some(fp) = key.fp {
                    check.push(row("Scan my copy", "", doing(Step::Scan(fp))));
                    if let Some(a) = seed.and_then(|s| item(Item::Copy(s))) {
                        check.push(row("Type it back", "", Some(a)));
                    }
                }
                if !check.is_empty() {
                    groups.push(("Check a copy of this seed".to_string(), check));
                }
                if key.passphrase
                    && key.held
                    && let Some(s) = seed
                {
                    groups.push((
                        "Its passphrase".to_string(),
                        vec![
                            row(
                                "Check it",
                                "By the key it makes",
                                open(Target::PassCheck(k, NO_LINE, NO_LINE)),
                            ),
                            row(
                                "Another copy",
                                "Then a place",
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
                    rows.push(row("Show its xpub", "", doing(Step::CosignerQr(k))));
                }
                rows.push(keeping("Name its holder", open(Target::Holder(k))));
                rows.push(keeping(
                    "Mark checked",
                    chart_do(ChartAction::KeyChecked(press, k)),
                ));
                if let Some(a) = key.action {
                    rows.push(row("Load this key", key.way_in.unwrap_or(""), Some(a)));
                }
                groups.push((String::new(), rows));
            }
            let mut v = View::new(
                Icon::Fingerprint,
                format!("Key {} · {}", key.number, key.fingerprint),
            );
            v.facts = facts;
            v.groups = groups;
            Some(v)
        }
        Target::Node(n) => {
            let node = nodes.get(usize::from(n))?;
            let mut facts = Vec::new();
            if let Some(s) = &node.stick {
                facts.push(("Stick", s.clone(), TEXT));
            }
            if let Some(s) = &node.alone {
                facts.push(("Found alone", s.clone(), TEXT));
            }
            if let Some((s, t)) = &node.state {
                facts.push(("Seen here", s.clone(), tone_color(*t)));
            }
            if matches!(node.at, At::Place(_)) && !node.whats.is_empty() {
                // The oldest of its things' dates, when each was checked.
                let dates: Option<Vec<&String>> = node.checked.iter().map(Option::as_ref).collect();
                facts.push(match dates.and_then(|d| d.into_iter().min()) {
                    Some(d) => ("Last checked here", glance::checked_text(d), OK),
                    None => ("Last checked here", "Not yet".to_string(), MUTED),
                });
            }
            let lines: Vec<Row> = node
                .holds
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < node.whats.len())
                .map(|(j, h)| {
                    let h = match node.marks.get(j).copied().flatten() {
                        Some(m) => format!("{h} · {}", m.name()),
                        None => h.clone(),
                    };
                    row(h, "", open(Target::Line(n, j as u8)))
                })
                .collect();
            let mut rows = Vec::new();
            let icon = match node.at {
                At::Place(p) => {
                    rows.push(if vault_open {
                        row("Rename", "", doing(Step::RenamePlace(p as u8)))
                    } else {
                        let way = app.vault_way(back).map(|(_, a)| a);
                        row("Rename", "Open a vault to keep this", way)
                    });
                    if !node.whats.is_empty() {
                        rows.push(keeping(
                            "Mark checked",
                            chart_do(ChartAction::Checked(press, n)),
                        ));
                    }
                    if let Some(a) = item(Item::Envelopes) {
                        rows.push(row("What goes in its envelope", "", Some(a)));
                    }
                    rows.push(row(
                        "The envelope list",
                        "PDF, for the stick",
                        doing(Step::Envelopes),
                    ));
                    rows.push(row("Add here", "", open(Target::AddAt(n))));
                    if !node.whats.is_empty() {
                        rows.push(keeping(
                            "Mark lost",
                            chart_do(ChartAction::Mark(press, n, None, Mark::Lost)),
                        ));
                        rows.push(keeping(
                            "Mark exposed",
                            chart_do(ChartAction::Mark(press, n, None, Mark::Exposed)),
                        ));
                    }
                    if places.len() > 1 {
                        rows.push(row("Remove the place", "", open(Target::Remove(n))));
                    }
                    Icon::House
                }
                At::Vault(v) => {
                    rows.extend(vault_way());
                    rows.push(keeping("Rename", open(Target::VaultName(n))));
                    if let Some(a) = item(Item::Vault(v)) {
                        rows.push(row("Save its seeds into it", "", Some(a)));
                    }
                    rows.push(row(
                        "Copy to another stick",
                        "Then a place",
                        open(Target::CopyTo(What::VaultStick(v))),
                    ));
                    Icon::Lock
                }
                At::Files => {
                    if node.whats.iter().any(|w| matches!(w, What::SeedFile(_)))
                        && let Some(a) = item(Item::SeedFiles)
                    {
                        rows.push(row("Write the seed files again", "", Some(a)));
                    }
                    if node.whats.contains(&What::Wallet)
                        && let Some(a) = item(Item::PublicFiles)
                    {
                        rows.push(row("Write the public files again", "", Some(a)));
                    }
                    rows.push(row(
                        "Remove from the plan",
                        "Destroy its copies by hand",
                        edit(Edit::Clear(At::Files)),
                    ));
                    Icon::File
                }
                At::Software => {
                    rows.push(row("Show the descriptor QR again", "", doing(Step::Qr)));
                    if let Some(a) = item(Item::PublicFiles) {
                        rows.push(row("Public files for this software", "", Some(a)));
                    }
                    Icon::Eye
                }
                At::Away => Icon::Keys,
            };
            let mut groups = Vec::new();
            if !lines.is_empty() {
                groups.push(("In it".to_string(), lines));
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
                            format!("Key {} seed", key.number),
                            if key.passphrase {
                                "With its passphrase"
                            } else {
                                ""
                            },
                            edit(Edit::Add(node.at, what)),
                        ));
                    }
                }
                if !node.whats.contains(&What::Wallet) {
                    more.push(row(
                        "The description",
                        "",
                        edit(Edit::Add(node.at, What::Wallet)),
                    ));
                }
                if !more.is_empty() {
                    groups.push(("Save more into it".to_string(), more));
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
            let mut facts = vec![("Kept at", node.name.clone(), TEXT)];
            if let Some((s, t)) = &node.state {
                facts.push(("Seen here", s.clone(), tone_color(*t)));
            }
            if let Some(d) = node.checked.get(usize::from(j)).cloned().flatten() {
                facts.push(("Last checked here", glance::checked_text(&d), OK));
            }
            if let Some(m) = mark {
                let m = match m {
                    Mark::Lost => "Lost",
                    Mark::Exposed => "Exposed",
                };
                facts.push(("Marked", m.to_string(), ERR));
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
                            rows.push(row("Load this key", key.way_in.unwrap_or(""), Some(a)));
                        }
                    }
                }
            };
            let at_place = matches!(node.at, At::Place(_));
            let mut rows = Vec::new();
            match what {
                What::Words(s) | What::SeedQr(s) => secret(s, &mut rows, &|rows, fp| {
                    rows.push(row(
                        "Scan my copy",
                        "Check this copy",
                        doing(Step::Scan(fp)),
                    ));
                    if let Some(a) = item(Item::Copy(s)) {
                        rows.push(row("Type it back", "Check this copy", Some(a)));
                    }
                    rows.push(row(
                        "Make another copy",
                        "Elsewhere",
                        open(Target::CopyTo(what)),
                    ));
                }),
                What::Seed(s) | What::SeedPassphrase(s) => secret(s, &mut rows, &|_, _| {}),
                What::SeedFile(_) => {
                    if let Some(a) = item(Item::SeedFiles) {
                        rows.push(row("Write it again", "For the stick", Some(a)));
                    }
                }
                What::Passphrase(s) => {
                    if let Some(k) = key_of_seed(s)
                        && g.keys.get(k).is_some_and(|key| key.held)
                    {
                        rows.push(row(
                            "Check it",
                            "By the key it makes",
                            open(Target::PassCheck(k as u8, n, j)),
                        ));
                    }
                    rows.push(row("Another copy", "Elsewhere", open(Target::CopyTo(what))));
                }
                What::Sheet => {
                    rows.push(row("Print it again", "PDF", doing(Step::Out(3))));
                    rows.push(row("As a QR picture", "PNG", doing(Step::Out(8))));
                    rows.push(row("As text", "The descriptor", doing(Step::Out(1))));
                    rows.push(row("Another copy", "Elsewhere", open(Target::CopyTo(what))));
                }
                What::Share(_) => {
                    rows.push(row(
                        "Print the shares again",
                        "PDFs, text and pictures",
                        doing(Step::Out(4)),
                    ));
                    if let Some(a) = item(Item::Sheets) {
                        rows.push(row("Keys left off each share", "", Some(a)));
                    }
                }
                What::VaultStick(_) => {
                    rows.extend(vault_way());
                    rows.push(row(
                        "Copy the vault to another stick",
                        "Then a place",
                        open(Target::CopyTo(what)),
                    ));
                    if app.vault_changed().is_some() {
                        rows.push(row(
                            "Write it out again",
                            "Changed since written",
                            Some(Action::WriteAsk),
                        ));
                    }
                }
                What::Wallet => rows.push(row("Show wallet QR", "The descriptor", doing(Step::Qr))),
            }
            // Where it is kept: moved, destroyed, lost or exposed.
            if at_place {
                rows.push(row("Move", "To another place", open(Target::MoveTo(n, j))));
                rows.push(match mark {
                    Some(_) => row("Remove from the plan", "", edit(Edit::Drop(node.at, what))),
                    None => row("I destroyed this copy", "", edit(Edit::Drop(node.at, what))),
                });
                if mark.is_none() {
                    rows.push(keeping(
                        "Lost",
                        chart_do(ChartAction::Mark(press, n, Some(j), Mark::Lost)),
                    ));
                    rows.push(keeping(
                        "Exposed",
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
                "A new place",
                "",
                edit(Edit::Move(node.at, what, At::Place(new_place))),
            ));
            let mut v = View::new(Icon::House, format!("Move {label} to"));
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
                "A new place",
                "",
                edit(Edit::Add(At::Place(new_place), what)),
            ));
            let mut v = View::new(
                Icon::House,
                format!("Another copy of {}", glance::thing_label(Some(&g), what)),
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
                    (What::Words(s), format!("Key {} words", key.number)),
                    (What::SeedQr(s), format!("Key {} SeedQR", key.number)),
                    (What::Passphrase(s), format!("Passphrase {}", key.number)),
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
                            format!("Share {}", j + 1),
                            "",
                            edit(Edit::Add(node.at, what)),
                        ));
                    }
                }
            } else if !node.whats.contains(&What::Sheet) {
                rows.push(row(
                    "Wallet sheet",
                    "",
                    edit(Edit::Add(node.at, What::Sheet)),
                ));
            }
            for x in nodes {
                if let At::Vault(v) = x.at
                    && !node.whats.contains(&What::VaultStick(v))
                {
                    rows.push(row(
                        format!("{} stick", x.name),
                        "A copy of the vault",
                        edit(Edit::Add(node.at, What::VaultStick(v))),
                    ));
                }
            }
            let mut v = View::new(Icon::House, format!("Add at {}", node.name));
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Confirm => {
            let ask = app.chart_work.ask.as_ref()?;
            let mut v = View::new(Icon::House, ask.title.clone());
            v.facts = check_facts(ask.before, ask.after);
            if let Edit::Drop(..) | Edit::Clear(_) | Edit::RemovePlace(_) = ask.edit {
                v.lines
                    .push(("Destroy what goes off the plan by hand".to_string(), MUTED));
            }
            v.go = Some(("Confirm".to_string(), chart_do(ChartAction::Confirm(press))));
            Some(v)
        }
        Target::Marked(n, line) => {
            let node = nodes.get(usize::from(n))?;
            let (shape, answers) = g.plan.as_ref()?;
            let f = plan::follows(shape, answers, &marks);
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
                plan::Found::Yes => v.lines.push(("Someone can now spend".to_string(), ERR)),
                plan::Found::OnlyWithVault => v.lines.push((
                    "Someone could spend with a vault's passphrase".to_string(),
                    ERR,
                )),
                plan::Found::No => {}
            }
            if f.found.spend != plan::Found::No {
                rows.push(row(
                    "Move the money",
                    "New wallet",
                    chart_do(ChartAction::MoveMoney(press)),
                ));
            }
            if f.any_lost && f.rebuilds == plan::Lost::No {
                v.lines.push((
                    "The wallet can no longer be rebuilt from what is left".to_string(),
                    ERR,
                ));
                rows.push(row("Back up again now", "", change_plan()));
            }
            if f.found.spend == plan::Found::No && f.found.balance != plan::Found::No {
                v.lines
                    .push(("Someone can see the balance".to_string(), WARN));
            }
            if f.any_lost && f.elsewhere {
                v.lines
                    .push(("Nothing lost that is not elsewhere".to_string(), OK));
                let what = here.iter().find(|(_, m)| *m == Mark::Lost).map(|(w, _)| *w);
                if let Some(what) = what {
                    rows.push(row(
                        "Make a replacement copy",
                        "",
                        open(Target::CopyTo(what)),
                    ));
                }
            }
            if v.lines.is_empty() && exposed {
                v.lines.push((
                    "Nothing it holds spends or shows the balance".to_string(),
                    OK,
                ));
            }
            v.groups = vec![(String::new(), rows)];
            Some(v)
        }
        Target::Remove(n) => {
            let node = nodes.get(usize::from(n))?;
            let At::Place(p) = node.at else {
                return None;
            };
            let mut v = View::new(Icon::House, format!("Remove {}", node.name));
            for (j, label) in node.holds.iter().enumerate().take(node.whats.len()) {
                let chosen = app.chart_work.dest.get(j).copied().flatten();
                let mut rows: Vec<Row> = places
                    .iter()
                    .filter(|(q, _)| *q != p)
                    .map(|(q, x)| {
                        row(
                            x.name.clone(),
                            if chosen == Some(*q as u8) {
                                "Chosen"
                            } else {
                                ""
                            },
                            chart_do(ChartAction::Dest(j as u8, *q as u8)),
                        )
                    })
                    .collect();
                rows.push(row(
                    "Nowhere",
                    if chosen == Some(OFF) {
                        "Chosen"
                    } else {
                        "Destroyed"
                    },
                    chart_do(ChartAction::Dest(j as u8, OFF)),
                ));
                v.groups.push((format!("{label} goes to"), rows));
            }
            let all = (0..node.whats.len())
                .all(|j| app.chart_work.dest.get(j).copied().flatten().is_some());
            v.go = Some((
                format!("Remove {}", node.name),
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
                format!("Key {} · {}", key.number, key.fingerprint),
            );
            let t = &app.chart_work.text;
            v.field = Some(if t.is_empty() {
                ("Holder", "Name".to_string(), true)
            } else {
                ("Holder", t.clone(), false)
            });
            v.go = Some(("Keep".to_string(), chart_do(ChartAction::Save(press))));
            Some(v)
        }
        Target::VaultName(n) => {
            let node = nodes.get(usize::from(n))?;
            let mut v = View::new(Icon::Lock, node.name.clone());
            let t = &app.chart_work.text;
            v.field = Some(if t.is_empty() {
                ("Name", "Name".to_string(), true)
            } else {
                ("Name", t.clone(), false)
            });
            v.go = Some(("Keep".to_string(), chart_do(ChartAction::Save(press))));
            Some(v)
        }
        Target::PassCheck(k, ..) => {
            let key = g.keys.get(usize::from(k))?;
            let mut v = View::new(Icon::Passphrase, format!("Passphrase {}", key.number));
            let typed = app.chart_work.secret.chars().count();
            v.field = Some(if typed == 0 {
                ("Passphrase", "Type it".to_string(), true)
            } else {
                ("Passphrase", "•".repeat(typed), false)
            });
            match app.chart_work.result {
                Some(true) => v
                    .lines
                    .push((format!("Matches: it makes key {}", key.fingerprint), OK)),
                Some(false) => v.lines.push(("Does not match".to_string(), ERR)),
                None => {}
            }
            v.go = Some((
                "Check".to_string(),
                (typed > 0).then_some(Action::Chart(ChartAction::PassTry(press))),
            ));
            Some(v)
        }
        Target::PassShow(k, n, j) => {
            let key = g.keys.get(usize::from(k))?;
            let mut v = View::new(Icon::Passphrase, format!("Passphrase {}", key.number));
            let shown = key.fp.and_then(|fp| {
                app.session
                    .keys
                    .iter()
                    .find(|x| x.master.fingerprint().0 == fp)
                    .and_then(|x| x.passphrase.as_ref())
            });
            match (shown, app.chart_work.reveal) {
                (Some(p), true) => v.lines.push((p.to_string(), TEXT)),
                (Some(_), false) => v.groups.push((
                    String::new(),
                    vec![row(
                        "Show it",
                        "To copy by hand",
                        chart_do(ChartAction::Reveal),
                    )],
                )),
                (None, _) => v
                    .lines
                    .push(("Load this key with its passphrase".to_string(), MUTED)),
            }
            v.go = Some((
                "Check my copy".to_string(),
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
        Target::Replan => {
            let mut v = View::new(Icon::Wallet, "Change the plan");
            v.lines
                .push(("Your changes on the chart are replaced".to_string(), WARN));
            v.go = Some((
                "Change the plan".to_string(),
                chart_do(ChartAction::Replan(press)),
            ));
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

fn tone_color(t: crate::backup::Tone) -> Color {
    use crate::backup::Tone;
    match t {
        Tone::Ok => OK,
        Tone::Warn => WARN,
        Tone::Err => ERR,
        Tone::Dim => DIM,
    }
}

/// The sheet over the chart. On a small panel it is the page itself, and
/// Back returns to the chart.
pub(crate) fn draw(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some((press, target)) = app.chart else {
        return;
    };
    let Some(v) = view(app, press, target) else {
        return;
    };
    let (place, margin, pad) = if ui.compact {
        ((0.0, w), 0.0, tokens::PAD)
    } else {
        let sw = (w - 2.0 * tokens::TOUCH).min(tokens::CHART_SHEET_WIDTH);
        (((w - sw) / 2.0, sw), tokens::PAD, 2.0 * tokens::PAD)
    };
    let close = if ui.compact { "Back" } else { "Close" };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(ui, x, cy, iw, v.icon, ACCENT, &v.title);
        for (label, value, tone) in &v.facts {
            cy += crate::compact::kv(ui, x, cy, iw, label, value, *tone);
        }
        for (line, tone) in &v.lines {
            cy += ui.wrap(x, cy, iw, tokens::LABEL, W::S, *tone, line) + tokens::GAP_SMALL;
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
                cy += ui.wrap(x, cy, iw, tokens::CAPTION, W::S, MUTED, heading) + tokens::GAP;
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
        buttons.push((close, Style::Secondary, Action::Cancel));
        cy += crate::compact::buttons(ui, x, cy, iw, &buttons);
        if let Some((label, None)) = &v.go {
            cy += tokens::GAP;
            cy += ui.wrap(x, cy, iw, tokens::CAPTION, W::R, DIM, label);
        }
        cy - y
    });
}

/// A field being typed: its label over a box with what it shows and the
/// caret. Returns its height.
fn field(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, text: &str, hint: bool) -> f32 {
    let lh = ui.wrap(x, y, w, tokens::CAPTION, W::S, MUTED, label) + tokens::GAP_SMALL;
    let h = tokens::CHART_SHEET_ROW;
    let by = y + lh;
    ui.fill(x, by, w, h, tokens::RADIUS_SMALL, BG);
    ui.stroke(x, by, w, h, tokens::RADIUS_SMALL, ACCENT.with_alpha(110));
    let shown = ui.fit(tokens::LABEL, W::R, text, w - 2.0 * tokens::PAD);
    let tw = ui.text_mid(
        x + tokens::PAD,
        by,
        h,
        tokens::LABEL,
        W::R,
        if hint { DIM } else { TEXT },
        &shown,
    );
    let cx = x + tokens::PAD + if hint { 0.0 } else { tw };
    ui.caret(cx, by + (h - tokens::LABEL) / 2.0, tokens::LABEL);
    lh + h
}

/// One action row: its label, its value at the right in the caption
/// type, a chevron where it can be pressed, dimmed where it cannot.
/// Returns its height.
fn menu_row(ui: &mut Ui, x: f32, y: f32, w: f32, r: &Row) -> f32 {
    let h = tokens::CHART_SHEET_ROW;
    let pressed = r.action.is_some_and(|a| ui.is_pressed(a));
    ui.fill(
        x,
        y,
        w,
        h,
        tokens::RADIUS_SMALL,
        if pressed { INNER } else { SURFACE },
    );
    ui.stroke(x, y, w, h, tokens::RADIUS_SMALL, LINE);
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
        ui.measure(tokens::CAPTION, W::R, &r.sub).min(room / 2.0)
    };
    let label_room = room - sub_w - if sub_w > 0.0 { tokens::GAP } else { 0.0 };
    let label = ui.fit(tokens::LABEL, W::S, &r.label, label_room);
    let tone = if r.action.is_some() { TEXT } else { DIM };
    ui.text_mid(x + pad, y, h, tokens::LABEL, W::S, tone, &label);
    if sub_w > 0.0 {
        let sub = ui.fit(tokens::CAPTION, W::R, &r.sub, sub_w);
        ui.text_right(
            x + w - pad - chevron,
            y,
            h,
            tokens::CAPTION,
            W::R,
            MUTED,
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
            DIM,
        );
        ui.hit(x, y, w, h, a);
    }
    h
}

impl Faraday {
    /// What a press on the chart or its sheets does.
    pub(crate) fn chart_act(&mut self, a: ChartAction) {
        match a {
            ChartAction::Open(press, target) => {
                // A sheet that types or chooses starts from what is kept.
                match target {
                    Target::Remove(n) => {
                        let things = chart(self, press)
                            .and_then(|g| match g.backup {
                                Backup::Plan { nodes, .. } => {
                                    nodes.get(usize::from(n)).map(|x| x.whats.len())
                                }
                                _ => None,
                            })
                            .unwrap_or(0);
                        self.chart_work.dest = vec![None; things];
                    }
                    Target::Holder(k) => {
                        self.chart_work.text = chart(self, press)
                            .and_then(|g| g.keys.get(usize::from(k)).and_then(|k| k.holder.clone()))
                            .unwrap_or_default();
                    }
                    Target::VaultName(n) => {
                        self.chart_work.text = chart(self, press)
                            .and_then(|g| match g.backup {
                                Backup::Plan { nodes, .. } => {
                                    nodes.get(usize::from(n)).map(|x| x.name.clone())
                                }
                                _ => None,
                            })
                            .filter(|name| !name.starts_with("Vault "))
                            .unwrap_or_default();
                    }
                    Target::PassCheck(..) => {
                        self.chart_work.secret.clear();
                        self.chart_work.result = None;
                    }
                    Target::PassShow(..) => self.chart_work.reveal = false,
                    _ => {}
                }
                if target != Target::Confirm {
                    self.chart_work.ask = None;
                }
                self.chart = Some((press, target));
                self.chart_focus = None;
                self.sheet = Some(Sheet::Chart);
                self.sheet_scroll = (0.0, None);
            }
            ChartAction::Where(press, k) => {
                self.close_chart();
                self.chart_focus = Some((press, usize::from(k)));
            }
            ChartAction::Edit(press, e) => self.chart_ask(press, e),
            ChartAction::Confirm(_) => self.chart_confirm(),
            ChartAction::Mark(press, n, j, m) => self.chart_mark(press, n, j, m),
            ChartAction::Checked(press, n) => self.chart_checked(press, n),
            ChartAction::KeyChecked(press, k) => self.chart_key_checked(press, k),
            ChartAction::Dest(j, p) => {
                let j = usize::from(j);
                if self.chart_work.dest.len() <= j {
                    self.chart_work.dest.resize(j + 1, None);
                }
                self.chart_work.dest[j] = Some(p);
            }
            ChartAction::Save(press) => self.chart_save_name(press),
            ChartAction::PassTry(press) => self.chart_pass_try(press),
            ChartAction::Reveal => self.chart_work.reveal = true,
            ChartAction::Replan(press) => {
                self.close_chart();
                let Some(w) = self.press_wallet(press) else {
                    return;
                };
                let from_vault = matches!(press, Press::Vault(..));
                let screen = self.screen;
                if self.chart_backup(w, from_vault) {
                    self.screen = screen;
                    // What the checklist has put in vaults so far, should
                    // the new plan drop it.
                    self.backup_prior();
                }
                self.chart_plan(w, from_vault, None);
                let shape = self.plan_shape(w);
                if let Some(b) = self.backup.as_mut() {
                    let places = b.answers.places;
                    b.answers.map = None;
                    b.answers.set_places(&shape, places);
                    b.names.truncate(b.answers.places);
                }
            }
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
                        let k = self
                            .session
                            .keys
                            .iter()
                            .position(|k| k.master.fingerprint().0 == fp);
                        if let Some(k) = k
                            && self.chart_checklist(w, from_vault)
                        {
                            self.act(Action::BKey(k));
                            self.act(Action::BScan);
                        }
                    }
                    Step::Plan => self.chart_plan(w, from_vault, None),
                    Step::RenamePlace(p) => {
                        self.chart_plan(w, from_vault, Some(crate::qstep::PLACES));
                        self.act(Action::BName(p));
                    }
                    Step::Rename => {
                        self.act(Action::OpenWallet(w));
                        self.act(Action::Rename);
                    }
                    Step::Addresses => {
                        self.chart = Some((Press::Loaded(w), Target::Addresses));
                        self.sheet = Some(Sheet::Chart);
                        self.sheet_scroll = (0.0, None);
                    }
                    Step::Envelopes => self.envelopes_out(w),
                }
            }
        }
    }

    fn close_chart(&mut self) {
        if self.sheet == Some(Sheet::Chart) {
            self.sheet = None;
        }
        self.chart = None;
    }

    /// The backup of wallet `w` with the plan its chart shows, at its
    /// checklist; nothing is made For the stick, each item offering what
    /// it makes. False when no plan is known.
    pub(crate) fn chart_checklist(&mut self, w: usize, from_vault: bool) -> bool {
        if !self.chart_backup(w, from_vault) {
            return false;
        }
        if let Some(b) = self.backup.as_mut()
            && b.stage != BStage::Checklist
        {
            b.stage = BStage::Checklist;
            b.q = None;
            b.naming = None;
            b.open = None;
            b.scroll = crate::flow::Scroll::default();
        }
        self.backups_sync();
        true
    }

    /// The backup of wallet `w` at its plan's questions, question `q`
    /// open, with the plan its chart shows.
    fn chart_plan(&mut self, w: usize, from_vault: bool, q: Option<u8>) {
        if !self.chart_backup(w, from_vault) {
            self.act(Action::Backup(w));
            if let Some(b) = self.backup.as_mut() {
                b.from_vault = from_vault;
            }
        }
        if let Some(b) = self.backup.as_mut() {
            b.stage = BStage::Plan;
            b.q = q;
            b.checking = false;
            b.scroll = crate::flow::Scroll::default();
        }
        self.screen = Screen::Backup;
    }

    /// The backup state for wallet `w`, holding the plan its chart shows:
    /// the backup under way when it is this wallet's checklist, else that
    /// plan put into it. False when no plan is known and no backup of it
    /// is under way.
    pub(crate) fn chart_backup(&mut self, w: usize, from_vault: bool) -> bool {
        let known = glance::plan_of(self, w);
        let mine = self.backup.as_ref().is_some_and(|b| b.wallet == w);
        if !mine {
            if known.is_none() {
                return false;
            }
            let screen = self.screen;
            self.act(Action::Backup(w));
            self.screen = screen;
        }
        let Some(b) = self.backup.as_mut() else {
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
}
