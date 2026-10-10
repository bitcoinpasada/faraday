//! Acting on the chart (`docs/NEW-WALLET.md` §9): a press on a node of
//! the wallet at a glance, or on a line inside one, opens a sheet for it:
//! what it is, where it is, what this device saw of it, and its actions.
//! Each action leads into the flow that already does it: the backup's
//! checklist at the item that makes that thing, the wallet's QR sheet, a
//! public file For the stick, the copy's scan, the vault way. Opening a
//! sheet changes nothing and puts nothing For the stick.
//!
//! In an open vault's view (§7.2) the wallet may not be loaded: an
//! action loads it first, with its seeds in that vault, as the chart's
//! presses did before.

use osk_ui::widgets::Icon;
use osk_ui::{Color, tokens};

use crate::glance::{self, Backup, BackupNode, Glance, Press};
use crate::plan::{At, Item, What};
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
}

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
}

/// One action row of a sheet.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub label: String,
    pub sub: String,
    pub action: Option<Action>,
}

/// A sheet as shown: its icon and title, its facts (label, value, tone),
/// and its rows under their headings.
#[derive(Debug, Clone)]
pub(crate) struct View {
    pub icon: Icon,
    pub title: String,
    pub facts: Vec<(&'static str, String, Color)>,
    pub groups: Vec<(&'static str, Vec<Row>)>,
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

/// The sheet for `target` on the chart of `press`; `None` when it is no
/// longer there.
pub(crate) fn view(app: &Faraday, press: Press, target: Target) -> Option<View> {
    let g = chart(app, press)?;
    let doing = |s: Step| Some(Action::Chart(ChartAction::Do(press, s)));
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
            rows.push(row("Change the plan", "", doing(Step::Plan)));
            Some(View {
                icon: Icon::Wallet,
                title: g.name.clone(),
                facts: vec![
                    ("Shape", g.shape.clone(), TEXT),
                    ("Checksum", format!("#{}", g.checksum), TEXT),
                ],
                groups: vec![("", rows)],
            })
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
                groups.push(("", public));
                let mut make = Vec::new();
                if let Some(s) = seed {
                    if let Some(a) = item(Item::Copy(s)) {
                        make.push(row(
                            "By hand",
                            "Words · SeedQR · Seed XOR · codex32",
                            Some(a),
                        ));
                    }
                    for n in nodes {
                        if let At::Vault(v) = n.at
                            && n.whats.iter().any(|w| of_seed(*w))
                            && let Some(a) = item(Item::Vault(v))
                        {
                            make.push(row(format!("Into {}", n.name), "", Some(a)));
                        }
                    }
                    if holds(&|w| w == What::SeedFile(s))
                        && let Some(a) = item(Item::SeedFiles)
                    {
                        make.push(row("As a file", "For the stick", Some(a)));
                    }
                }
                if !make.is_empty() {
                    groups.push(("Make another backup of this seed", make));
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
                    groups.push(("Check a copy of this seed", check));
                }
            } else if let Some(a) = key.action {
                groups.push((
                    "",
                    vec![row("Load this key", key.way_in.unwrap_or(""), Some(a))],
                ));
            }
            Some(View {
                icon: Icon::Fingerprint,
                title: format!("Key {} · {}", key.number, key.fingerprint),
                facts,
                groups,
            })
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
            let lines: Vec<Row> = node
                .holds
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < node.whats.len())
                .map(|(j, h)| {
                    row(
                        h.clone(),
                        "",
                        Some(Action::Chart(ChartAction::Open(
                            press,
                            Target::Line(n, j as u8),
                        ))),
                    )
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
                    if let Some(a) = item(Item::Envelopes) {
                        rows.push(row("What goes in its envelope", "", Some(a)));
                    }
                    Icon::House
                }
                At::Vault(v) => {
                    rows.extend(vault_way());
                    if let Some(a) = item(Item::Vault(v)) {
                        rows.push(row("Save its seeds into it", "", Some(a)));
                    }
                    rows.push(row("Copy to another stick", "", Some(Action::WriteAsk)));
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
                groups.push(("In it", lines));
            }
            if !rows.is_empty() {
                groups.push(("", rows));
            }
            Some(View {
                icon,
                title: node.name.clone(),
                facts,
                groups,
            })
        }
        Target::Line(n, j) => {
            let node = nodes.get(usize::from(n))?;
            let what = *node.whats.get(usize::from(j))?;
            let label = node.holds.get(usize::from(j))?.clone();
            let mut facts = vec![("Kept at", node.name.clone(), TEXT)];
            if let Some((s, t)) = &node.state {
                facts.push(("Seen here", s.clone(), tone_color(*t)));
            }
            let key_of = |s: usize| g.keys.iter().position(|k| k.seed == Some(s));
            // A secret's actions, with its seed here; else the way to it.
            let secret = |s: usize, rows: &mut Vec<Row>, with: &dyn Fn(&mut Vec<Row>, [u8; 4])| {
                let Some(key) = key_of(s).and_then(|k| g.keys.get(k)) else {
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
                        rows.push(row("Make another copy", "", Some(a)));
                    }
                }),
                What::Seed(s) | What::SeedPassphrase(s) => secret(s, &mut rows, &|_, _| {}),
                What::SeedFile(_) => {
                    if let Some(a) = item(Item::SeedFiles) {
                        rows.push(row("Write it again", "For the stick", Some(a)));
                    }
                }
                What::Passphrase(_) => {}
                What::Sheet => {
                    rows.push(row("Print it again", "PDF", doing(Step::Out(3))));
                    rows.push(row("As a QR picture", "PNG", doing(Step::Out(8))));
                    rows.push(row("As text", "The descriptor", doing(Step::Out(1))));
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
                        "",
                        Some(Action::WriteAsk),
                    ));
                }
                What::Wallet => rows.push(row("Show wallet QR", "The descriptor", doing(Step::Qr))),
            }
            Some(View {
                icon: match what {
                    What::VaultStick(_) | What::Seed(_) | What::SeedPassphrase(_) => Icon::Lock,
                    What::Passphrase(_) => Icon::Passphrase,
                    What::Words(_) | What::SeedQr(_) => Icon::Keys,
                    _ => Icon::File,
                },
                title: label,
                facts,
                groups: vec![("", rows)],
            })
        }
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
        cy += crate::compact::buttons(ui, x, cy, iw, &[(close, Style::Secondary, Action::Cancel)]);
        cy - y
    });
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
                self.chart = Some((press, target));
                self.chart_focus = None;
                self.sheet = Some(Sheet::Chart);
                self.sheet_scroll = (0.0, None);
            }
            ChartAction::Where(press, k) => {
                self.close_chart();
                self.chart_focus = Some((press, usize::from(k)));
            }
            ChartAction::Do(press, step) => {
                self.close_chart();
                let w = match press {
                    Press::Loaded(w) => Some(w).filter(|&w| w < self.session.wallets.len()),
                    Press::Vault(v, r) => self.vault_load_wallet(v, r),
                };
                let Some(w) = w else {
                    return;
                };
                let from_vault = matches!(press, Press::Vault(..));
                match step {
                    Step::Qr => self.act(Action::QrWallet(w)),
                    Step::Out(what) => self.public_out(w, what),
                    Step::KeyQr(k) => self.act(Action::ShowCode(crate::Code::WalletKey(w, k))),
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
    fn chart_backup(&mut self, w: usize, from_vault: bool) -> bool {
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
