//! The pick's rows (`docs/WORKSPACE.md` §3, "The pick"): what a flow's
//! first step offers to choose, as data, and the one drawer of them. A
//! row is a Material single-choice list row: the radio at the start, a
//! tonal container behind it, the chosen one in the secondary container,
//! the state layer on hover and press. A way in is the same row with no
//! radio. A row the step cannot take is dimmed, keeps its reason as its
//! line and takes no press, by pointer or keyboard. Labels only: no row
//! holds or reads a secret.

use crate::Action;
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{
    BODY_L, BODY_M, DISABLED_CONTENT, GAP, GAP_SMALL, ICON_GLYPH, ICON_GLYPH_BUTTON, LABEL_L,
    LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, PICK_RADIO_DOT_INSET, ROW_CHEVRON_ROOM, SHAPE_S,
};
use faraday_draw::widgets::Icon;

/// What a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Something loaded the flow takes: a radio, ticked when `chosen`.
    Pick { chosen: bool },
    /// Something loaded a step that takes several uses: a checkbox, ticked
    /// when `ticked`; one button of the step uses the ticked ones.
    Tick { ticked: bool },
    /// A way to get what the flow takes (Add a key): no radio.
    WayIn,
    /// An item whose press leads on from it rather than chooses it (Spend
    /// from a backup's wallets): no radio, the chevron at the end.
    Opens,
}

/// One row of the pick.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PickRow {
    pub icon: Icon,
    pub head: String,
    /// The second line; the reason when the row is dimmed.
    pub line: String,
    pub kind: Kind,
    /// Dimmed: the row stays and takes no press.
    pub dimmed: bool,
    /// The second line is drawn in the error role.
    pub error: bool,
    pub action: Action,
}

impl PickRow {
    /// A loaded item the flow takes.
    pub(crate) fn pick(icon: Icon, head: String, line: String, chosen: bool, a: Action) -> Self {
        PickRow {
            icon,
            head,
            line,
            kind: Kind::Pick { chosen },
            dimmed: false,
            error: false,
            action: a,
        }
    }

    /// A loaded item a step that takes several uses: a checkbox.
    pub(crate) fn tick(icon: Icon, head: String, line: String, ticked: bool, a: Action) -> Self {
        PickRow {
            kind: Kind::Tick { ticked },
            ..PickRow::pick(icon, head, line, false, a)
        }
    }

    /// A loaded item a press opens: no radio, a chevron.
    pub(crate) fn opens(icon: Icon, head: String, line: String, a: Action) -> Self {
        PickRow {
            kind: Kind::Opens,
            ..PickRow::pick(icon, head, line, false, a)
        }
    }

    /// A way in, which has no second line.
    pub(crate) fn way_in(icon: Icon, head: &str, action: Action) -> Self {
        PickRow {
            icon,
            head: head.to_string(),
            line: String::new(),
            kind: Kind::WayIn,
            dimmed: false,
            error: false,
            action,
        }
    }

    /// The row as it is when the step cannot take it, with its `reason`.
    pub(crate) fn dimmed_for(mut self, reason: &str) -> Self {
        self.dimmed = true;
        self.line = reason.to_string();
        self
    }
}

/// What a flow says of one item it may take: whether it is the chosen one,
/// what a press does and, when the step cannot take it, why. The item's
/// row, its icon and its line are the pane's.
pub(crate) struct Take {
    pub chosen: bool,
    pub action: Action,
    /// The step cannot take it: the row stays, dimmed, with this line.
    pub refusal: Option<String>,
    /// The flow's own second line, in place of the pane's.
    pub line: Option<String>,
    /// The step takes several: a checkbox in the radio's place, `chosen`
    /// meaning ticked.
    pub several: bool,
    /// A press opens the item and chooses nothing: the chevron, no radio.
    pub opens: bool,
}

impl Take {
    /// An item the step takes.
    pub(crate) fn of(chosen: bool, action: Action) -> Self {
        Take {
            chosen,
            action,
            refusal: None,
            line: None,
            several: false,
            opens: false,
        }
    }

    /// A press opens the item or leads on from it: the chevron.
    pub(crate) fn opens(mut self) -> Self {
        self.opens = true;
        self
    }

    /// The step takes several items: a checkbox, ticked when chosen.
    pub(crate) fn several(mut self) -> Self {
        self.several = true;
        self
    }

    /// The row's second line as the flow says it, in place of the pane's
    /// (a flow whose rows name what the item makes).
    pub(crate) fn line(mut self, line: impl Into<String>) -> Self {
        self.line = Some(line.into());
        self
    }

    /// The same item, refused for `reason`.
    pub(crate) fn refused(mut self, reason: impl Into<String>) -> Self {
        self.refusal = Some(reason.into());
        self
    }
}

/// A flow's pick as it declares it: which kinds it takes, which item is
/// chosen and which is refused (each through [`Take`]), and its ways in,
/// the only rows the flow makes itself. The pane lists the keys, wallets
/// and files, in its order (Keys, Wallets, Files; in a pick opened from
/// Check, Check's own: Files, Keys, Wallets),
/// and no flow lists `session.keys` or `session.wallets` for a
/// row of its own (`lint-components`).
pub(crate) struct Decl {
    question: &'static str,
    keys: Vec<PickRow>,
    wallets: Vec<PickRow>,
    files: Vec<PickRow>,
    /// Whether the pick was opened from Check, which puts Files first.
    from_check: bool,
    ways: Vec<PickRow>,
}

impl Decl {
    /// A pick under `question`, taking nothing yet.
    pub(crate) fn new(question: &'static str) -> Self {
        Decl {
            question,
            keys: Vec::new(),
            wallets: Vec::new(),
            files: Vec::new(),
            from_check: false,
            ways: Vec::new(),
        }
    }

    /// Takes keys: `take` answers for each loaded key (its index and facts),
    /// `None` for one of a kind the flow never takes (left out). Each row's
    /// head is the fingerprint and its line the key row's of the pane.
    pub(crate) fn keys(
        self,
        app: &crate::Faraday,
        take: impl Fn(usize, &faraday_keys::KeyInfo) -> Option<Take>,
    ) -> Self {
        self.keys_many(app, |i, k| {
            let fp = crate::wallet::fp_text(k.fingerprint);
            take(i, k)
                .map(|t| (String::from(&*fp), t))
                .into_iter()
                .collect()
        })
    }

    /// Takes keys with several rows each (Lightning's two nodes of a key):
    /// `take` gives each row's head and what the flow says of it. The line
    /// is the key row's of the pane.
    pub(crate) fn keys_many<H: AsRef<str>>(
        mut self,
        app: &crate::Faraday,
        take: impl Fn(usize, &faraday_keys::KeyInfo) -> Vec<(H, Take)>,
    ) -> Self {
        let session = &app.session;
        self.keys = session
            .keys
            .all()
            .enumerate()
            .flat_map(|(i, k)| {
                take(i, k)
                    .into_iter()
                    .map(|(head, t)| {
                        let line = || super::rows::key_line(session, k);
                        row_of(Icon::Keys, head.as_ref().to_owned(), line, t)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        self
    }

    /// Takes wallets: `take` answers for the wallet at an index and gives
    /// the line a taken row carries (a refused one carries its reason).
    pub(crate) fn wallets<L: std::ops::Deref<Target = str>>(
        mut self,
        app: &crate::Faraday,
        take: impl Fn(usize, &crate::wallet::Wallet) -> Option<(Take, L)>,
    ) -> Self {
        self.wallets = app
            .session
            .wallets
            .iter()
            .enumerate()
            .filter_map(|(i, w)| {
                let (t, line) = take(i, w)?;
                Some(row_of(
                    Icon::Wallet,
                    String::from(&*w.name),
                    || String::from(&*line),
                    t,
                ))
            })
            .collect();
        self
    }

    /// Takes held files: `take` answers for the file at an index of the
    /// Inbox, `None` for one of a kind the flow never takes (hidden).
    pub(crate) fn files(
        self,
        app: &crate::Faraday,
        take: impl Fn(usize, &crate::item::Item) -> Option<Take>,
    ) -> Self {
        self.files_each(app, |i, f| take(i, f))
    }

    /// [`Decl::files`] where a file may hold several things a step takes: a
    /// row for each take, under the file's name.
    pub(crate) fn files_each<T: IntoIterator<Item = Take>>(
        mut self,
        app: &crate::Faraday,
        take: impl Fn(usize, &crate::item::Item) -> T,
    ) -> Self {
        self.from_check = app.opened_from_screen() == Some(crate::Screen::Check);
        self.files = app
            .inbox
            .iter()
            .enumerate()
            .flat_map(|(i, f)| {
                (take(i, f).into_iter())
                    .map(move |t| row_of(Icon::File, f.name.to_string(), String::new, t))
            })
            .collect();
        self
    }

    /// Rows of its own kind of key that is held but is not in the session's
    /// keys (the aezeed Convert reads), standing last among the keys.
    pub(crate) fn also_keys(mut self, rows: Vec<PickRow>) -> Self {
        self.keys.extend(rows);
        self
    }

    /// Its ways in, in the order they stand.
    pub(crate) fn ways(mut self, rows: Vec<PickRow>) -> Self {
        self.ways = rows;
        self
    }

    /// The groups, each under its heading, empty ones left out; the ways
    /// in last, headed "Other ways in" while anything loaded is taken.
    pub(crate) fn groups(self) -> Vec<crate::flow::PickGroup> {
        let loaded = !(self.keys.is_empty() && self.wallets.is_empty() && self.files.is_empty());
        let mut order = [
            (EN.session_group_keys, self.keys),
            (EN.session_group_wallets, self.wallets),
            (EN.session_group_files, self.files),
        ];
        // Files moves to the front; the other two keep their order.
        if self.from_check {
            order.rotate_right(1);
        }
        let mut groups: Vec<_> = order
            .into_iter()
            .filter(|(_, rows)| !rows.is_empty())
            .map(|(head, rows)| crate::flow::PickGroup { head, rows })
            .collect();
        if !self.ways.is_empty() {
            groups.push(crate::flow::PickGroup {
                head: ways_in_head(loaded),
                rows: self.ways,
            });
        }
        groups
    }

    /// The flow's [`crate::flow::Pick`].
    pub(crate) fn pick(self) -> crate::flow::Pick {
        let question = self.question;
        crate::flow::Pick {
            question,
            groups: self.groups(),
        }
    }
}

/// The pane's line is worked out only where the flow gives neither its own
/// line nor a refusal.
fn row_of(icon: Icon, head: String, line: impl FnOnce() -> String, t: Take) -> PickRow {
    let line = if t.refusal.is_some() {
        String::new()
    } else {
        t.line.unwrap_or_else(line)
    };
    let row = if t.opens {
        PickRow::opens(icon, head, line, t.action)
    } else if t.several {
        PickRow::tick(icon, head, line, t.chosen, t.action)
    } else {
        PickRow::pick(icon, head, line, t.chosen, t.action)
    };
    match t.refusal {
        Some(reason) => row.dimmed_for(&reason),
        None => row,
    }
}

/// Draws a pick's groups from `y`, each under its heading, one under
/// another: the pane's and the narrow card's alike. Returns the height used.
pub(crate) fn draw_groups(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    groups: &[crate::flow::PickGroup],
) -> f32 {
    let mut cy = y;
    for g in groups {
        cy += head(ui, x, cy, w, g.head);
        cy += rows(ui, x, cy, w, &g.rows);
    }
    cy - y
}

/// The ways-in group's heading: "Other ways in", or "Ways in" when
/// nothing loaded is taken.
pub(crate) fn ways_in_head(loaded: bool) -> &'static str {
    if loaded {
        EN.pick_other_ways_in
    } else {
        EN.pick_ways_in
    }
}

/// A group's heading at `y`. Returns the height used.
pub(crate) fn head(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    let h = ui.advance_as(LABEL_L, ICON_GLYPH + GAP);
    let text = ui.fit_as(LABEL_L, text, w);
    ui.text_mid_as((x, y, h), LABEL_L, Role::OnSurfaceVariant, &text);
    h
}

/// The rows from `y`, one under another. Returns the height used.
pub(crate) fn rows(ui: &mut Ui, x: f32, y: f32, w: f32, rows: &[PickRow]) -> f32 {
    let mut cy = y;
    for r in rows {
        cy += row(ui, (x, cy, w), r) + GAP_SMALL;
    }
    cy - y
}

fn row(ui: &mut Ui, (x, y, w): (f32, f32, f32), r: &PickRow) -> f32 {
    let two = !r.line.is_empty();
    let radio = if matches!(r.kind, Kind::Pick { .. } | Kind::Tick { .. }) {
        ICON_GLYPH_BUTTON + GAP
    } else {
        0.0
    };
    let opens = r.kind == Kind::Opens;
    let chevron = if opens { ROW_CHEVRON_ROOM } else { 0.0 };
    let room = w - LIST_PAD - chevron - (LIST_PAD + radio + ICON_GLYPH + GAP);
    // Both lines wrap, so a long head or verdict is read whole; each is
    // broken once and drawn from that.
    let head_laid = ui.wrapped_as(BODY_L, &r.head, room);
    let line_laid = two.then(|| ui.wrapped_as(BODY_M, &r.line, room));
    let (lh, vh) = (ui.line_as(BODY_L), ui.line_as(BODY_M));
    let head_lines = (head_laid.h / lh).round() as usize;
    let line_lines = line_laid
        .as_ref()
        .map_or(0, |l| (l.h / vh).round() as usize);
    let lines = head_laid.h + line_laid.as_ref().map_or(0.0, |l| l.h);
    let mut h = ui.row_height(
        if two { LIST_2 } else { LIST_1 },
        if two { lh + vh } else { lh },
        LIST_PAD_V,
    );
    if head_lines + line_lines > if two { 2 } else { 1 } {
        h = h.max(lines + 2.0 * LIST_PAD_V);
    }
    let on = matches!(
        r.kind,
        Kind::Pick { chosen: true } | Kind::Tick { ticked: true }
    );
    let ink = |role: Role| {
        if r.dimmed {
            Role::OnSurface.at(DISABLED_CONTENT)
        } else {
            on_selected(on, role)
        }
    };
    ui.fill(
        x,
        y,
        w,
        h,
        SHAPE_S,
        selected_or(on, Role::SurfaceContainerHighest),
    );
    let mut tx = x + LIST_PAD;
    if let Kind::Pick { chosen } = r.kind {
        let d = ICON_GLYPH_BUTTON / 2.0;
        let (cx, cy) = (tx + d, y + h / 2.0);
        ui.dot(cx, cy, d, ink(Role::OnSurfaceVariant));
        ui.dot(
            cx,
            cy,
            d - 2.0,
            selected_or(on, Role::SurfaceContainerHighest),
        );
        if chosen {
            ui.dot(cx, cy, d - PICK_RADIO_DOT_INSET, ink(Role::OnSurface));
        }
        tx += ICON_GLYPH_BUTTON + GAP;
    }
    if let Kind::Tick { ticked } = r.kind {
        ui.checkbox(tx, y + (h - ICON_GLYPH_BUTTON) / 2.0, ticked, !r.dimmed);
        tx += ICON_GLYPH_BUTTON + GAP;
    }
    ui.icon(
        tx,
        y + (h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        r.icon,
        ICON_GLYPH,
        ink(Role::OnSurfaceVariant),
    );
    tx += ICON_GLYPH + GAP;
    if opens {
        ui.row_chevron(x + w - LIST_PAD, y, h, ink(Role::OnSurfaceVariant));
    }
    let top = y + (h - lines) / 2.0;
    ui.draw_wrapped((tx, top), ink(Role::OnSurface), &head_laid);
    if let Some(laid) = &line_laid {
        let tone = line_role(r.error, on, r.dimmed);
        ui.draw_wrapped((tx, top + head_laid.h), tone, laid);
    }
    // A dimmed row takes no press: nothing is registered for it.
    if !r.dimmed {
        ui.press_box(x, y, w, h, r.action);
    }
    h
}

/// What a row's second line is written in: a warned row stays red when
/// chosen, in on-error-container, which reads on the selected fill in every
/// theme; a dimmed row is dimmed whatever it says.
fn line_role(error: bool, on: bool, dimmed: bool) -> Paint {
    if dimmed {
        return Role::OnSurface.at(DISABLED_CONTENT);
    }
    match (error, on) {
        (true, true) => Role::OnErrorContainer.into(),
        (true, false) => Role::Error.into(),
        _ => on_selected(on, Role::OnSurfaceVariant),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    /// The presses a drawn pick registers, for pointer and keyboard alike.
    fn presses(rows_: &[PickRow]) -> Vec<Action> {
        let d = DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let mut canvas = faraday_draw::Canvas::new(&d);
        let f = canvas.scale().factor();
        let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
        {
            let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
            rows(&mut ui, 0.0, 0.0, 300.0, rows_);
        }
        hits.into_iter().map(|(_, a)| a).collect()
    }

    /// The height `rows` takes in a column 300 wide.
    fn height(rows_: &[PickRow]) -> f32 {
        let d = DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let mut canvas = faraday_draw::Canvas::new(&d);
        let f = canvas.scale().factor();
        let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        rows(&mut ui, 0.0, 0.0, 300.0, rows_)
    }

    #[test]
    fn a_long_line_is_wrapped_whole_not_cut() {
        let one = |head: &str, line: &str| {
            PickRow::pick(
                Icon::Keys,
                head.into(),
                line.into(),
                false,
                Action::PKey([1; 4]),
            )
        };
        let long = "1 signature · Valid · not the nonce this key makes: do not broadcast";
        assert!(height(&[one("Input 0", long)]) > height(&[one("Input 0", "1 signature")]));
    }

    #[test]
    fn a_long_head_is_wrapped_whole_not_cut() {
        let one = |head: &str| PickRow::way_in(Icon::File, head, Action::PKey([1; 4]));
        let long = "a-very-long-name-of-a-transaction-file-from-the-coordinator-signed-twice.psbt";
        assert!(height(&[one(long)]) > height(&[one("a.psbt")]));
    }

    /// A row whose texts fit is the height the list's formula gives, as
    /// before rows wrapped (one line, and a head over a line).
    #[test]
    fn a_row_whose_texts_fit_keeps_its_height() {
        let d = DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let mut canvas = faraday_draw::Canvas::new(&d);
        let f = canvas.scale().factor();
        let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        let (lh, vh) = (ui.line_as(BODY_L), ui.line_as(BODY_M));
        let one = ui.row_height(LIST_1, lh, LIST_PAD_V);
        let two = ui.row_height(LIST_2, lh + vh, LIST_PAD_V);
        let way = PickRow::way_in(Icon::File, "a.psbt", Action::PKey([1; 4]));
        let pick = PickRow::pick(
            Icon::Keys,
            "Key".into(),
            "Seed".into(),
            false,
            Action::PKey([1; 4]),
        );
        assert_eq!(rows(&mut ui, 0.0, 0.0, 300.0, &[way]), one + GAP_SMALL);
        assert_eq!(rows(&mut ui, 0.0, 0.0, 300.0, &[pick]), two + GAP_SMALL);
    }

    #[test]
    fn a_dimmed_row_takes_no_press() {
        let row = |fp: u8| {
            PickRow::pick(
                Icon::Keys,
                "key".into(),
                "line".into(),
                false,
                Action::PKey([fp; 4]),
            )
        };
        let got = presses(&[row(1), row(2).dimmed_for("No seed held"), row(3)]);
        assert_eq!(got, [Action::PKey([1; 4]), Action::PKey([3; 4])]);
    }

    #[test]
    fn a_chosen_warned_row_stays_red_and_a_dimmed_row_stays_dimmed() {
        let dim = Role::OnSurface.at(DISABLED_CONTENT);
        assert_eq!(
            line_role(true, true, false),
            Paint::role(Role::OnErrorContainer)
        );
        assert_eq!(line_role(true, false, false), Paint::role(Role::Error));
        assert_eq!(
            line_role(false, true, false),
            Paint::role(Role::OnSecondaryContainer)
        );
        assert_eq!(
            line_role(false, false, false),
            Paint::role(Role::OnSurfaceVariant)
        );
        for (error, on) in [(true, true), (true, false), (false, true), (false, false)] {
            assert_eq!(line_role(error, on, true), dim);
        }
    }

    #[test]
    fn a_step_that_takes_several_offers_checkbox_rows() {
        let take = |on: bool, a: Action| Take::of(on, a).several();
        let row = |t: Take| row_of(Icon::Keys, "key".into(), || "line".into(), t);
        let ticked = row(take(true, Action::PKey([1; 4])));
        let open = row(take(false, Action::PKey([2; 4])));
        let held = row(take(false, Action::PKey([3; 4])).refused("Already in the wallet"));
        assert_eq!(ticked.kind, Kind::Tick { ticked: true });
        assert_eq!(open.kind, Kind::Tick { ticked: false });
        assert!(held.dimmed && held.line == "Already in the wallet");
        // A step that takes one stays a radio.
        let one = row(Take::of(true, Action::PKey([4; 4])));
        assert_eq!(one.kind, Kind::Pick { chosen: true });
        let got = presses(&[ticked, open, held]);
        assert_eq!(got, [Action::PKey([1; 4]), Action::PKey([2; 4])]);
    }

    #[test]
    fn a_row_that_opens_has_no_radio_and_a_press_opens() {
        let row = |t: Take| row_of(Icon::Wallet, "wallet".into(), || "line".into(), t);
        let opens = row(Take::of(false, Action::OpenWallet(0)).opens());
        let ready = row(Take::of(false, Action::OpenWallet(1)));
        assert_eq!(opens.kind, Kind::Opens);
        assert_eq!(ready.kind, Kind::Pick { chosen: false });
        assert_eq!(
            presses(&[opens, ready]),
            [Action::OpenWallet(0), Action::OpenWallet(1)]
        );
    }
}

#[cfg(all(test, feature = "testkit"))]
mod order_tests {
    use super::{Decl, EN, Take};
    use crate::{Action, Faraday, Screen};

    /// The headings of a pick that takes every key, wallet and file, as
    /// declared in the tool on show.
    fn heads(app: &Faraday) -> Vec<&'static str> {
        Decl::new("")
            .keys(app, |_, _| Some(Take::of(false, Action::Scan)))
            .wallets(app, |_, _| {
                Some((Take::of(false, Action::Scan), String::new()))
            })
            .files(app, |_, _| Some(Take::of(false, Action::Scan)))
            .groups()
            .iter()
            .map(|g| g.head)
            .collect()
    }

    /// Decode, opened by presses from `opener` in a session holding a key
    /// and a file.
    fn decode_from(opener: Screen) -> Faraday {
        let mut app = crate::testkit::started();
        app.press(Action::Entry(None));
        for c in "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".chars() {
            faraday_shell_api::App::event(
                &mut app,
                faraday_shell_api::Event::Key(faraday_shell_api::Key::Char(c)),
            );
        }
        app.press(Action::EntryAdd);
        crate::testkit::keep_published(&mut app);
        crate::testkit::inbox_file(&mut app, "note.txt", b"a note".to_vec());
        app.press(Action::Nav(opener));
        app.press(Action::Nav(Screen::Decode));
        assert_eq!(app.screen, Screen::Decode);
        app
    }

    #[test]
    fn a_pick_opened_from_check_puts_files_first_and_from_elsewhere_keys() {
        let (keys, files) = (EN.session_group_keys, EN.session_group_files);
        assert_eq!(heads(&decode_from(Screen::Check)), [files, keys]);
        assert_eq!(heads(&decode_from(Screen::Catalog)), [keys, files]);
    }
}
