//! Bring in the sticks (`docs/STICK-SETUP.md` §3): a figure for each stick
//! of the layout, in order, wrapping into rows, each with its name, the
//! vaults it will hold and its state; which waiting stick a stick put in
//! goes to; and the boot stick offered to any one of them. The stick
//! boxes are the backup board's (`board/screen.rs`).

use crate::backup_flow::steps::FlowAction as F;
use crate::backup_flow::{BackupFlow, Purpose, Step};
use crate::board::lines::stick_word;
use crate::board::screen as board_screen;
use crate::drawing::View;
use crate::strings::{EN as FEN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, menu};
use faraday_draw::tokens::{
    self, BOARD_EDGE, BOARD_MARK, BOARD_STICK_HEAD, BOARD_STICK_PAD, BODY_S, GAP, GAP_SMALL,
    LABEL_M, STICK_FIGURE_W, TOUCH,
};
use faraday_draw::widgets::Icon;

fn fa(a: F) -> Action {
    Action::Flow(a)
}

impl BackupFlow {
    /// The stick a stick put in goes to: the waiting one the person
    /// pressed last, else the first one waiting (§3.2).
    pub(crate) fn wanted_to_bind(&self) -> Option<usize> {
        self.live
            .want
            .filter(|&s| self.needs_bind(s))
            .or_else(|| (0..self.slots.len()).find(|&s| self.needs_bind(s)))
    }

    /// A press on the figure of slot `s`: the next stick is for it.
    pub(crate) fn want(&mut self, s: usize) {
        if self.step == Step::Sticks && self.needs_bind(s) {
            self.live.want = Some(s);
        }
    }

    /// A press on slot `s`'s ⋮.
    pub(crate) fn toggle_fig_menu(&mut self, s: usize) {
        self.live.fig_menu = (self.live.fig_menu != Some(s)).then_some(s);
    }

    /// What reading a stick for `slot` is for: a stick bound already is
    /// checked at the write, any other is bound.
    pub(crate) fn bind_purpose(&self, slot: usize) -> Purpose {
        if self.step == Step::Write && self.slots[slot].bound.is_some() {
            Purpose::Check
        } else {
            Purpose::Bind
        }
    }
}

/// How a figure stands.
enum State {
    Waiting,
    Reading,
    Checked,
    Problem(String),
}

/// One thing under a figure's head.
enum Item {
    /// A line of text.
    Text(String, Role),
    /// The state: a tick, if any, and its words.
    State(Option<Icon>, String, Role),
    /// The boot stick offered, as a button.
    Boot(String),
    /// The ⋮ menu's row, open.
    Menu(String),
}

/// The figure of slot `k`: its name, what it holds, its state.
struct Figure {
    k: usize,
    name: String,
    state: State,
    items: Vec<Item>,
    dots: bool,
}

fn state_of(app: &View, f: &BackupFlow, k: usize) -> State {
    let r = f.live.reading.as_ref().filter(|r| r.slot == k);
    if let Some(p) = r.and_then(|r| r.problem.as_ref()) {
        return State::Problem(app.flow_problem_text(p));
    }
    if f.slots[k].bound.is_some() {
        State::Checked
    } else if r.is_some() {
        State::Reading
    } else {
        State::Waiting
    }
}

/// The figures of the flow's slots.
fn figures(app: &View, f: &BackupFlow) -> Vec<Figure> {
    let n = f.slots.len();
    let reading = f.live.reading.as_ref();
    // The slot the boot stick holds or is being read for.
    let boot = (0..n).find(|&j| f.boot_holds(j) || reading.is_some_and(|r| r.boot && r.slot == j));
    let boot_in = app.stick.iter().any(|s| s.boot);
    let m = app.medium;
    (0..n)
        .map(|k| {
            let state = state_of(app, f, k);
            let name = if boot == Some(k) {
                m.boot().to_string()
            } else {
                let before = (0..k).filter(|&j| boot != Some(j)).count();
                fill(stick_word(m), &[&(before + 2).to_string()])
            };
            let mut items = Vec::new();
            let vaults = f.vaults_of(k);
            if !vaults.is_empty() {
                let names: Vec<String> =
                    vaults.iter().map(|&v| BackupFlow::vault_name(v)).collect();
                items.push(Item::Text(
                    fill(FEN.backup_flow_figure_holds, &[&names.join(", ")]),
                    Role::OnSurfaceVariant,
                ));
            }
            if matches!(state, State::Checked) {
                for &v in &vaults {
                    let vp = &f.plan.vaults[v];
                    if let (true, Some((_, file))) = (vp.existing, vp.from.as_ref()) {
                        items.push(Item::Text(
                            fill(
                                FEN.backup_flow_figure_file,
                                &[&BackupFlow::vault_name(v), file],
                            ),
                            Role::OnSurfaceVariant,
                        ));
                    }
                }
            }
            items.push(match &state {
                State::Waiting => Item::State(
                    None,
                    FEN.backup_flow_figure_waiting.to_string(),
                    Role::OnSurfaceVariant,
                ),
                State::Reading => Item::State(
                    None,
                    FEN.backup_flow_figure_reading.to_string(),
                    Role::Primary,
                ),
                State::Checked => Item::State(
                    Some(Icon::Check),
                    FEN.backup_flow_figure_checked.to_string(),
                    Role::Ok,
                ),
                State::Problem(t) => Item::Text(t.clone(), Role::Error),
            });
            if let Some(mark) = app.flow_mark_line(k) {
                items.push(Item::Text(mark, Role::Primary));
            }
            // The stick the machine started from, read already: any
            // waiting figure may take it, one figure at most (§3.3).
            let offer =
                boot_in && boot.is_none() && reading.is_none() && matches!(state, State::Waiting);
            if offer {
                items.push(Item::Boot(m.use_boot().to_string()));
                if f.live.fig_menu == Some(k) {
                    items.push(Item::Menu(m.use_boot().to_string()));
                }
            }
            Figure {
                k,
                name,
                state,
                items,
                dots: offer,
            }
        })
        .collect()
}

fn item_h(ui: &Ui, it: &Item, w: f32) -> f32 {
    match it {
        Item::Text(t, _) => ui.wrap_height_as(BODY_S, t, w),
        Item::State(..) => BOARD_STICK_HEAD,
        Item::Boot(_) | Item::Menu(_) => TOUCH,
    }
}

fn figure_h(ui: &Ui, fg: &Figure, w: f32) -> f32 {
    let top = if fg.dots { TOUCH } else { BOARD_STICK_HEAD };
    let inner = w - 2.0 * BOARD_STICK_PAD;
    let body: f32 = fg
        .items
        .iter()
        .map(|it| item_h(ui, it, inner) + GAP_SMALL)
        .sum();
    (top + body + BOARD_STICK_PAD).max(TOUCH)
}

/// The figures from (x, y) in `w`. Returns their height.
pub(crate) fn draw(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let figs = figures(app, f);
    if figs.is_empty() {
        return 0.0;
    }
    let cols = (((w + GAP) / (STICK_FIGURE_W + GAP)) as usize).clamp(1, figs.len());
    let fw = (w - (cols - 1) as f32 * GAP) / cols as f32;
    let fr = board_screen::card_frame(app, ui, None, None, None);
    let mut cy = y;
    for row in figs.chunks(cols) {
        let rh = row
            .iter()
            .map(|fg| figure_h(ui, fg, fw))
            .fold(0.0, f32::max);
        for (c, fg) in row.iter().enumerate() {
            let fx = x + c as f32 * (fw + GAP);
            draw_one(ui, &fr, fg, (fx, cy, fw, rh));
        }
        cy += rh + GAP;
    }
    cy - y
}

fn draw_one(ui: &mut Ui, fr: &board_screen::CardFrame, fg: &Figure, r: (f32, f32, f32, f32)) {
    let (x, y, w, h) = r;
    let head = board_screen::card_stick(ui, fr, &fg.name, r, fa(F::Want(fg.k)), false);
    match fg.state {
        State::Reading => menu::outline(ui, r, GAP, BOARD_EDGE, Role::Primary.paint()),
        State::Checked => menu::outline(ui, r, GAP, BOARD_EDGE, Role::Ok.paint()),
        State::Problem(_) => menu::outline(ui, r, GAP, BOARD_EDGE, Role::Error.paint()),
        State::Waiting => {}
    }
    let sp = BOARD_STICK_PAD;
    let inner = w - 2.0 * sp;
    let mut cy = y + if fg.dots { TOUCH } else { head };
    if fg.dots {
        menu::button(ui, x + w - TOUCH, y, TOUCH, fa(F::FigMenu(fg.k)), false);
    }
    for it in &fg.items {
        let ih = item_h(ui, it, inner);
        match it {
            Item::Text(t, role) => {
                ui.wrap_as((x + sp, cy, inner), BODY_S, *role, t);
            }
            Item::State(icon, t, role) => {
                let tx = match icon {
                    Some(i) => {
                        ui.icon(
                            x + sp,
                            cy + (ih - BOARD_MARK) / 2.0,
                            BOARD_MARK,
                            *i,
                            BOARD_MARK,
                            *role,
                        );
                        x + sp + BOARD_MARK + tokens::BOARD_CHIP_GAP
                    }
                    None => x + sp,
                };
                ui.text_mid_as((tx, cy, ih), LABEL_M, *role, t);
            }
            Item::Boot(t) => {
                ui.button(
                    x + sp,
                    cy,
                    Some(inner),
                    TOUCH,
                    t,
                    Style::Secondary,
                    fa(F::UseBootAt(fg.k)),
                );
            }
            Item::Menu(t) => {
                ui.button(
                    x + sp,
                    cy,
                    Some(inner),
                    TOUCH,
                    t,
                    Style::Ghost,
                    fa(F::UseBootAt(fg.k)),
                );
            }
        }
        cy += ih + GAP_SMALL;
    }
    debug_assert!(cy <= y + h + 1.0);
}
