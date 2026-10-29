//! The host's strip and menus (`docs/DESKTOP-HOST.md` §2, §3), drawn in
//! the host's own look: the app's theme turned inside out (Material's
//! inverse roles, and the opposite mode's scheme for what stands on them),
//! so a light app has a dark host and a dark app a light one; never the
//! app's surface or its components. The always-there controls first (the
//! stick pills, the boot stick first and "Sticks · N" holding those that
//! do not fit; Samples; Size), then the screen's demo aids in a section of
//! their own colour under "Practice", those with no room in More; the
//! whole row centred, scrolling sideways where even folded it is wider
//! than the window. Drawing reads the app's state and what its last frame
//! noted; it works nothing out but where a control goes.

use faraday_draw::Color;
use faraday_draw::geom::Rect;
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_ui::demo::DemoItem;
use faraday_ui::{Mode, Role, Theme};

use super::{Area, Host, HostMenu as Menu, HostPress};
use crate::fresh::roller::Act as DieAct;
use crate::samples::{PASSPHRASE, Pick, Roller};
use crate::strings::{EN, fill};
use crate::{Action, Faraday};

/// The host's colours for the app's scheme: the strip in the app's
/// inverse roles, and what stands on it, round it and over it in the
/// scheme of the app's theme in the opposite mode at the same contrast
/// level, so each pair is one Material made to read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    /// The opposite mode's scheme: the computer's die is drawn in it.
    pub theme: Theme,
    /// The space round a preview's frame: that scheme's surface.
    pub surround: Color,
    /// The strip: the app's inverse surface.
    strip: Color,
    /// The strip's text and outlines' ink: the app's inverse on-surface.
    ink: Color,
    /// A control whose menu is open: the app's inverse primary, its
    /// label in the strip's own colour.
    accent: Color,
    /// A control's outline on the strip, and the preview frame's edge.
    edge: Color,
    /// The section of the screen's own aids: a tertiary container.
    section: Color,
    /// Text and outlines in that section.
    on_section: Color,
    /// The step's main press in that section: tertiary.
    section_main: Color,
    /// Its label: on-tertiary.
    on_section_main: Color,
    /// A menu's or Samples' sheet's surface, raised over the strip:
    /// Material's surface container.
    raised: Color,
    /// Text on it.
    on_raised: Color,
    /// Secondary text on it: a heading, a size, a sample's kind.
    on_raised_dim: Color,
    /// A menu's and the sheet's edge, and the rule over the sheet's foot.
    rule: Color,
    /// The sheet's main press and a tick: primary.
    main: Color,
    /// Its label and the tick's check: on-primary.
    on_main: Color,
    /// A stick that is in: the OK colour.
    stick_in: Color,
    /// A stick that is out: the outline.
    stick_out: Color,
    /// A stick the flow asks for, and the sheet's line asking for one out.
    caution: Color,
}

impl Look {
    /// The host's colours for the app's scheme `app`. A lookup.
    pub(crate) fn of(app: Theme) -> Look {
        let mode = match app.mode {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        };
        let h = Theme::new(app.name, mode, app.contrast);
        Look {
            theme: h,
            surround: h.role(Role::Surface),
            strip: app.role(Role::InverseSurface),
            ink: app.role(Role::InverseOnSurface),
            accent: app.role(Role::InversePrimary),
            edge: h.role(Role::Outline),
            section: h.role(Role::TertiaryContainer),
            on_section: h.role(Role::OnTertiaryContainer),
            section_main: h.role(Role::Tertiary),
            on_section_main: h.role(Role::OnTertiary),
            raised: h.role(Role::SurfaceContainer),
            on_raised: h.role(Role::OnSurface),
            on_raised_dim: h.role(Role::OnSurfaceVariant),
            rule: h.role(Role::OutlineVariant),
            main: h.role(Role::Primary),
            on_main: h.role(Role::OnPrimary),
            stick_in: h.role(Role::Ok),
            stick_out: h.role(Role::Outline),
            caution: h.role(Role::Warning),
        }
    }

    /// The host's colours for the app as it stands. A lookup.
    pub(crate) fn now(host: &Host) -> Look {
        Look::of(host.app.theme())
    }
}

/// `ink` at `opacity`: Material's state layer, or what is not available
/// now.
fn layer(ink: Color, opacity: f32) -> Color {
    ink.with_alpha(t::alpha_of(opacity))
}

type HostUi<'a> = faraday_ui::Ui<'a, HostPress>;

/// What a control on the strip is.
#[derive(Clone, PartialEq)]
enum Kind {
    /// A stick's pill, its dot lit while it is in.
    Stick { inserted: bool },
    /// A button.
    Button,
    /// The step's main press: filled.
    Primary,
    /// A button that takes no press now.
    Off,
    /// A button that opens a menu, its chevron pointing up.
    Opens(Menu),
}

/// One control on the strip.
#[derive(Clone)]
struct Ctl {
    /// Marked in primary, pulsing: a stick the app asks for, or Sticks · N
    /// holding one (`docs/DESKTOP-HOST.md` §3).
    mark: bool,
    label: String,
    press: HostPress,
    kind: Kind,
}

/// The padding each side of a control's label: Material's button's.
const CTL_PAD: f32 = t::PAD;
/// The room a stick's dot takes before its label.
const DOT_ROOM: f32 = t::GAP + t::GAP_SMALL;
/// The room a menu's chevron takes after its label.
const CHEVRON_ROOM: f32 = t::ICON_SMALL;
/// The room inside the aids' section at either end.
const SECTION_PAD: f32 = t::GAP;

fn ctl_w(ui: &HostUi, c: &Ctl) -> f32 {
    let extra = match c.kind {
        Kind::Stick { .. } => DOT_ROOM,
        Kind::Opens(_) => CHEVRON_ROOM,
        _ => 0.0,
    };
    ui.measure_as(t::LABEL_L, &c.label) + 2.0 * CTL_PAD + extra
}

/// The stick pills' controls: every stick with its state, the boot stick
/// first.
fn sticks(app: &Faraday) -> Vec<Ctl> {
    if !app.shell_sticks {
        return Vec::new();
    }
    app.stick_pills()
        .into_iter()
        .map(|p| {
            let state = if p.inserted {
                EN.stick_pill_inserted
            } else {
                EN.stick_pill_out
            };
            Ctl {
                mark: p.asked,
                label: fill(EN.stick_pill_state, &[&p.label, state]),
                press: HostPress::App(Action::StickPill(p.n as u8)),
                kind: Kind::Stick {
                    inserted: p.inserted,
                },
            }
        })
        .collect()
}

/// The computer's dice roller the page shows, New key's dice or Create a
/// vault's Generate with dice, where the "Demo only" card would hold it.
/// A lookup.
pub(crate) fn roller_shown(app: &Faraday) -> Option<Roller> {
    app.demo_hosted_now().and_then(|h| h.roller)
}

/// The screen's demo aids, the lines saying what Practice puts in place of
/// the person's own, and whether Play's options are offered: Make codex32
/// shares' entry's from its state, every other page's and sheet's from
/// what its last frame offered for the "Demo only" card, the computer's
/// die's controls first. A lookup.
fn aids(app: &Faraday) -> (Vec<Ctl>, Vec<String>, bool) {
    let mut ctls: Vec<Ctl> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut walks = false;
    let roller = app.roller_offered();
    if let Some(m) = app
        .keygen()
        .filter(|k| k.open == Some(crate::keygen::kstep::CODEX32))
        .and_then(|k| k.codex32.as_ref())
    {
        for (label, on, primary, act) in crate::codex32::walk_screen::host_aids(m, roller) {
            let kind = match (on, primary) {
                (false, _) => Kind::Off,
                (true, true) => Kind::Primary,
                (true, false) => Kind::Button,
            };
            ctls.push(Ctl {
                mark: false,
                label: label.to_string(),
                press: HostPress::App(Action::Codex32(act)),
                kind,
            });
        }
        walks = crate::codex32::walk_screen::host_walks(m, roller);
        if walks {
            let die = fill(EN.codex32_die_size, &[&m.dice_choice().faces.to_string()]);
            lines.push(fill(EN.codex32_rolls_die, &[&die]));
        }
    }
    for item in app.sample_area.aids() {
        match item {
            DemoItem::Button(label, action) => {
                let press = HostPress::App(action);
                if !ctls.iter().any(|c| c.press == press) {
                    ctls.push(Ctl {
                        mark: false,
                        label: label.to_string(),
                        press,
                        kind: Kind::Button,
                    });
                }
            }
            DemoItem::Line(text) => lines.push(text),
        }
    }
    if let Some(hosted) = app.demo_hosted_now() {
        lines.extend(hosted.lines);
        if let Some(which) = hosted.roller {
            ctls.splice(0..0, roller_ctls(app, which));
        }
    }
    lines.dedup();
    (ctls, lines, walks)
}

/// The die, its press for each of the roller's own, whether it may roll.
type DieOf<'a> = (&'a crate::fresh::roller::Roller, fn(DieAct) -> Action, bool);

/// The computer's die `which` the page shows: the roller, its press for
/// each of the roller's own, and whether it may roll now (the entries
/// have room). A lookup.
fn roller_of(app: &Faraday, which: Roller) -> Option<DieOf<'_>> {
    match which {
        Roller::KeyGen => app.keygen().map(|k| {
            (
                &k.roller,
                Action::KRoller as fn(DieAct) -> Action,
                k.roller_room() > 0,
            )
        }),
        Roller::Vault => {
            let act: fn(DieAct) -> Action = |a| Action::Vault(crate::vault::VaultAction::Roller(a));
            Some((&app.vaults.roller, act, crate::vault::dice::room(app) > 0))
        }
    }
}

/// The die's controls in the strip: **Roll** (**Stop** while a click
/// rolls), then **Rolls per click · N**, which opens its choices. A
/// lookup.
fn roller_ctls(app: &Faraday, which: Roller) -> Vec<Ctl> {
    let Some((r, act, room)) = roller_of(app, which) else {
        return Vec::new();
    };
    let roll = if r.rolling() {
        Ctl {
            mark: false,
            label: EN.roller_stop.to_string(),
            press: HostPress::App(act(DieAct::Stop)),
            kind: Kind::Button,
        }
    } else {
        Ctl {
            mark: false,
            label: EN.roller_roll.to_string(),
            press: HostPress::App(act(DieAct::Roll)),
            kind: if room { Kind::Primary } else { Kind::Off },
        }
    };
    let per = crate::fresh::roller::screen::per_choices()
        .into_iter()
        .find(|(_, p)| *p == r.per())
        .map_or_else(
            || {
                if r.typed().is_empty() {
                    EN.roller_number.to_string()
                } else {
                    r.typed().to_string()
                }
            },
            |(l, _)| l.to_string(),
        );
    vec![
        roll,
        Ctl {
            mark: false,
            label: fill(EN.host_rolls_per_click, &[&per]),
            press: HostPress::Menu(Menu::Rolls),
            kind: Kind::Opens(Menu::Rolls),
        },
    ]
}

/// The preview sizes the Size menu lists, each its name and its size in
/// dp; `None` is the window itself.
fn sizes() -> [(&'static str, Option<(u16, u16)>); 10] {
    let s = t::PREVIEW_SIZES;
    [
        (EN.settings_preview_this_window, None),
        (EN.settings_preview_pi, Some(s[0])),
        (EN.settings_preview_narrow, Some(s[1])),
        (EN.settings_preview_half, Some(s[2])),
        (EN.settings_preview_7in, Some(s[3])),
        (EN.settings_preview_7in_portrait, Some(s[4])),
        (EN.settings_preview_10in, Some(s[5])),
        (EN.settings_preview_10in_portrait, Some(s[6])),
        (EN.settings_preview_desktop, Some(s[7])),
        (EN.settings_preview_monitor, Some(s[8])),
    ]
}

/// The name of the size shown.
fn size_name(preview: Option<(u16, u16)>) -> &'static str {
    sizes()
        .into_iter()
        .find(|(_, s)| *s == preview)
        .map_or(EN.settings_preview_this_window, |(n, _)| n)
}

/// What the strip holds this frame, fitted to its width: the sticks shown
/// (the rest are in the Sticks menu), the aids shown and those in More.
struct Fitted {
    sticks: Vec<Ctl>,
    right: Vec<Ctl>,
    /// "Practice" heads the aids, the die after it; else none, and More
    /// reads "Practice".
    head: bool,
    /// The section's head and narrowest More: the mode's name, "Practice"
    /// or "Testnet".
    head_label: &'static str,
    demo: Vec<Ctl>,
    demo_more: Vec<Ctl>,
    lines: Vec<String>,
}

/// Size's control: by its size's name, or `short`, "Size" alone.
fn size_ctl(host: &Host, short: bool) -> Ctl {
    Ctl {
        mark: false,
        label: if short {
            EN.host_size_short.to_string()
        } else {
            fill(EN.host_size, &[size_name(host.preview)])
        },
        press: HostPress::Menu(Menu::Size),
        kind: Kind::Opens(Menu::Size),
    }
}

/// "Sticks · N", opening the menu of every stick, marked where one of
/// those it holds is asked for.
fn sticks_ctl(n: usize, mark: bool) -> Ctl {
    Ctl {
        mark,
        label: fill(EN.stick_pill_more, &[&n.to_string()]),
        press: HostPress::Menu(Menu::Sticks),
        kind: Kind::Opens(Menu::Sticks),
    }
}

/// More, opening the aids with no room; narrowest (`!head`), it reads
/// `label`, the mode's name.
fn more_ctl(head: bool, label: &'static str) -> Ctl {
    Ctl {
        mark: false,
        label: if head { EN.host_more } else { label }.to_string(),
        press: HostPress::Menu(Menu::More),
        kind: Kind::Opens(Menu::More),
    }
}

/// The screen's demo aids with Play options after them where the walk
/// is offered, and the lines. A lookup.
fn demo_ctls(app: &Faraday) -> (Vec<Ctl>, Vec<String>) {
    let (mut demo, lines, walks) = aids(app);
    if walks {
        demo.push(Ctl {
            mark: false,
            label: EN.host_play_options.to_string(),
            press: HostPress::Menu(Menu::Play),
            kind: Kind::Opens(Menu::Play),
        });
    }
    (demo, lines)
}

/// Costs a text measure per control. Fits the strip's controls in `room`,
/// the die taking `die_w` before the aids: every stick and every aid
/// where all fit; else the sticks fold into "Sticks · N" after the boot
/// stick and as many as fit; then Size reads "Size" alone; then the aids
/// that do not fit go into More, the step's main press staying; then the
/// sticks stand as "Sticks · N" alone; then Samples joins More;
/// narrowest, no "Practice" head and no die.
fn fit(host: &Host, ui: &HostUi, room: f32, die_w: f32) -> Fitted {
    let app = &host.app;
    let all_sticks = sticks(app);
    let samples = app.samples_on().then(|| Ctl {
        mark: false,
        label: EN.samples_pill_short.to_string(),
        press: HostPress::Menu(Menu::Samples),
        kind: Kind::Button,
    });
    let size = |short: bool| size_ctl(host, short);
    let (mut demo, lines) = demo_ctls(app);
    let head_label = app.mode().name();
    let more_ctl = more_ctl(true, head_label);
    let sum = |v: &[Ctl]| -> f32 { v.iter().map(|c| ctl_w(ui, c) + t::GAP).sum() };
    // The section's own room at its ends.
    let divider = 2.0 * SECTION_PAD;
    let head = |any: bool| {
        if any {
            divider + ui.measure_as(t::TITLE_S, head_label) + t::GAP + die_w
        } else {
            0.0
        }
    };
    let n = all_sticks.len();
    // The last `k` sticks fold into the pill; it is marked where one of
    // them is asked for.
    let sticks_more = |k: usize| sticks_ctl(k, all_sticks[n - k.min(n)..].iter().any(|s| s.mark));
    let any_aid = !demo.is_empty() || die_w > 0.0;
    let mut right: Vec<Ctl> = samples.iter().cloned().chain([size(false)]).collect();
    let fixed = |right: &[Ctl], any: bool| sum(right) + head(any);
    let demo_w = sum(&demo);
    // The sticks: all, else folded beside every aid.
    let mut shown = all_sticks.clone();
    if n > 1 && sum(&shown) + fixed(&right, any_aid) + demo_w > room {
        let mut kept = vec![all_sticks[0].clone()];
        for s in &all_sticks[1..n - 1] {
            let mut next = kept.clone();
            next.push(s.clone());
            next.push(sticks_more(n - next.len()));
            if sum(&next) + fixed(&right, any_aid) + demo_w > room {
                break;
            }
            kept.push(s.clone());
        }
        let rest = n - kept.len();
        kept.push(sticks_more(rest));
        shown = kept;
    }
    // Then Size gives up its size's name, which its menu shows checked,
    // before an aid leaves the strip.
    if sum(&shown) + fixed(&right, any_aid) + demo_w > room {
        right = samples.iter().cloned().chain([size(true)]).collect();
    }
    // The aids that do not fit go into More; where even More does not
    // fit beside the folded sticks, the sticks stand as one pill, then
    // Size shortens and Samples joins More.
    let mut demo_more: Vec<Ctl> = Vec::new();
    let more_w = ctl_w(ui, &more_ctl) + t::GAP;
    let mut head = any_aid;
    if sum(&shown) + fixed(&right, any_aid) + demo_w > room {
        if n > 0 && sum(&shown) + fixed(&right, any_aid) + more_w > room {
            shown = vec![sticks_more(n)];
        }
        if sum(&shown) + fixed(&right, any_aid) + more_w > room {
            right = vec![size(true)];
            demo_more.extend(samples.iter().cloned());
        }
        // Narrowest: no "Practice" head and no die; the sticks' menu
        // reads "Sticks" and More itself "Practice".
        if sum(&shown) + fixed(&right, true) + more_w > room {
            head = false;
            if n > 0 {
                shown = vec![Ctl {
                    label: EN.host_sticks_short.to_string(),
                    ..sticks_more(n)
                }];
            }
        }
        let left = room - sum(&shown) - fixed(&right, true);
        demo = keep_aids(ui, demo, &mut demo_more, left, more_w);
    }
    Fitted {
        sticks: shown,
        right,
        head,
        head_label,
        demo,
        demo_more,
        lines,
    }
}

/// Costs a text measure per aid. The aids that stay in the strip in
/// `left` dp, More's `more_w` counted: the step's main press (Next,
/// Roll) first, then the others in their order while they fit; the rest
/// go into `more`.
fn keep_aids(ui: &HostUi, demo: Vec<Ctl>, more: &mut Vec<Ctl>, left: f32, more_w: f32) -> Vec<Ctl> {
    let main = |c: &Ctl| c.kind == Kind::Primary;
    let main_w: f32 = demo
        .iter()
        .filter(|c| main(c))
        .map(|c| ctl_w(ui, c) + t::GAP)
        .sum();
    let mains_stay = more_w + main_w <= left;
    let mut used = more_w + if mains_stay { main_w } else { 0.0 };
    let mut open = more
        .iter()
        .all(|m| m.press == HostPress::Menu(Menu::Samples));
    let mut kept = Vec::new();
    for c in demo {
        if mains_stay && main(&c) {
            kept.push(c);
            continue;
        }
        let w = ctl_w(ui, &c) + t::GAP;
        if open && used + w <= left {
            used += w;
            kept.push(c);
        } else {
            open = false;
            more.push(c);
        }
    }
    kept
}

/// What the host drew.
pub(crate) struct Drawn {
    /// What the strip's controls say, left to right, then the open menu's
    /// rows or Samples' sheet's texts.
    pub labels: Vec<String>,
    /// The open menu or Samples' sheet: where it stands in the window's
    /// pixels and the furthest its rows scroll, dp.
    pub sheet: Option<(Rect, f32)>,
    /// The furthest the strip's row scrolls sideways, dp: 0 where it fits.
    pub strip_max: f32,
}

/// One piece of the strip's row.
enum Piece<'a> {
    /// A control.
    Ctl(&'a Ctl),
    /// The mode's name ("Practice", "Testnet"), heading the section.
    Head(&'static str),
    /// The computer's die.
    Die(Roller),
    /// What Practice puts in place of the person's own, cut to fit.
    Lines(String),
}

/// The row as it is laid out: the always-there controls, then the aids'
/// section where there is one.
struct Row<'a> {
    always: Vec<Piece<'a>>,
    section: Vec<Piece<'a>>,
}

/// A piece's width. A text measure.
fn piece_w(ui: &HostUi, p: &Piece) -> f32 {
    match p {
        Piece::Ctl(c) => ctl_w(ui, c),
        Piece::Head(label) => ui.measure_as(t::TITLE_S, label),
        Piece::Die(_) => t::ROLLER_DIE,
        Piece::Lines(s) => ui.measure_as(t::BODY_M, s),
    }
}

/// Pieces' widths with a gap between each two. Text measures.
fn run_w(ui: &HostUi, ps: &[Piece]) -> f32 {
    let n = ps.len() as f32;
    ps.iter().map(|p| piece_w(ui, p)).sum::<f32>() + (n - 1.0).max(0.0) * t::GAP
}

impl Row<'_> {
    /// The whole row's width: the section's ends included. Text measures.
    fn width(&self, ui: &HostUi) -> f32 {
        let a = run_w(ui, &self.always);
        if self.section.is_empty() {
            return a;
        }
        let between = if self.always.is_empty() { 0.0 } else { t::GAP };
        a + between + 2.0 * SECTION_PAD + run_w(ui, &self.section)
    }
}

/// Costs a text measure per piece. The strip's row from what was fitted,
/// the lines cut to the room `room` leaves, or left out where it leaves
/// less than a touch target's width.
fn lay_out<'a>(ui: &HostUi, fitted: &'a Fitted, shown: Option<Roller>, room: f32) -> Row<'a> {
    let mut row = Row {
        always: fitted
            .sticks
            .iter()
            .chain(&fitted.right)
            .map(Piece::Ctl)
            .collect(),
        section: Vec::new(),
    };
    let group = !fitted.demo.is_empty() || !fitted.demo_more.is_empty() || shown.is_some();
    if group {
        if fitted.head {
            row.section.push(Piece::Head(fitted.head_label));
            if let Some(r) = shown {
                row.section.push(Piece::Die(r));
            }
        }
        row.section.extend(fitted.demo.iter().map(Piece::Ctl));
    }
    // The lines in the room left, More (made by the caller) counted.
    if !fitted.lines.is_empty() {
        let text = fitted.lines.join(" · ");
        let more = if fitted.demo_more.is_empty() {
            0.0
        } else {
            ctl_w(ui, &more_ctl(fitted.head, fitted.head_label)) + t::GAP
        };
        // Where no aid stands, the section opens to hold the lines.
        let opens = if row.section.is_empty() {
            t::GAP + 2.0 * SECTION_PAD
        } else {
            t::GAP
        };
        let left = room - row.width(ui) - more - opens;
        if left > t::TOUCH {
            row.section
                .push(Piece::Lines(ui.fit_as(t::BODY_M, &text, left)));
        }
    }
    row
}

/// Draws the host round the app's frame at `area`: the preview's edge,
/// the strip and its row of controls, centred, or scrolled `scroll` dp
/// where it is wider than the window, and the menu or Samples' sheet open
/// or closing.
pub(crate) fn draw(host: &Host, ui: &mut HostUi, area: Option<Area>, scroll: f32) -> Drawn {
    let look = Look::now(host);
    // What the strip draws by role (a focus ring, a state layer, the
    // High contrast outline) is in the host's scheme, not the app's.
    ui.theme = look.theme;
    let f = ui.f;
    let (w, h) = (ui.c.width() as f32 / f, ui.c.height() as f32 / f);
    if host.preview.is_some()
        && let Some(a) = area
    {
        // Primary while the pointer or the keys reach the app, else
        // outline-variant, in the host's scheme it stands on.
        edge(ui, a, look.rule.mix(look.main, host.edge_shown()));
    }
    let sh = host.strip_dp();
    let top = h - sh;
    ui.fill(0.0, top, w, sh, 0.0, look.strip);
    let shown = roller_shown(&host.app);
    let die_w = shown.map_or(0.0, |_| t::ROLLER_DIE + t::GAP);
    let room = w - 2.0 * t::PAD;
    let fitted = fit(host, ui, room, die_w);
    let more = (!fitted.demo_more.is_empty()).then(|| more_ctl(fitted.head, fitted.head_label));
    let mut row = lay_out(ui, &fitted, shown, room);
    // More ends the aids, before the lines.
    if let Some(m) = &more {
        let at = row
            .section
            .iter()
            .position(|p| matches!(p, Piece::Lines(_)))
            .unwrap_or(row.section.len());
        row.section.insert(at, Piece::Ctl(m));
    }
    let row_w = row.width(ui);
    // Centred where the row fits; else from the left edge, scrolled.
    let strip_max = (row_w - room).max(0.0);
    let mut x = if strip_max > 0.0 {
        // Pulled past an end by a finger, it stretches.
        t::PAD - scroll.clamp(0.0, strip_max) - host.strip_stretch()
    } else {
        (w - row_w) / 2.0
    };
    let cy = top + (sh - t::HOST_CONTROL_H) / 2.0;
    let mut anchors: Vec<(Menu, f32)> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for p in &row.always {
        x += piece(
            host,
            ui,
            p,
            (x, cy, top, sh),
            false,
            &mut anchors,
            &mut labels,
        ) + t::GAP;
    }
    if !row.section.is_empty() {
        let sw = 2.0 * SECTION_PAD + run_w(ui, &row.section);
        let sy = top + t::GAP_SMALL;
        let sh2 = sh - 2.0 * t::GAP_SMALL;
        ui.fill(x, sy, sw, sh2, t::SHAPE_L, look.section);
        // Edged, so it stands apart from the strip where the two
        // containers meet in tone.
        ui.stroke(x, sy, sw, sh2, t::SHAPE_L, look.edge);
        x += SECTION_PAD;
        for p in &row.section {
            x += piece(
                host,
                ui,
                p,
                (x, cy, top, sh),
                true,
                &mut anchors,
                &mut labels,
            ) + t::GAP;
        }
    }
    if let Some(m) = &more {
        // What More holds follows its own label.
        let at = labels
            .iter()
            .position(|l| *l == m.label)
            .map_or(labels.len(), |i| i + 1);
        let rest: Vec<String> = fitted.demo_more.iter().map(|c| c.label.clone()).collect();
        labels.splice(at..at, rest);
    }
    // The lines, each whole, however short they were cut.
    labels.extend(fitted.lines.iter().cloned());
    if strip_max > 0.0 {
        indicator(
            ui,
            &look,
            (top, sh, room),
            (row_w, scroll.clamp(0.0, strip_max), strip_max),
        );
    }
    let (rows, sheet) = menus(host, ui, &fitted, &anchors, (area, top, w));
    labels.extend(rows);
    Drawn {
        labels,
        sheet,
        strip_max,
    }
}

/// Draws one piece of the row at (`x`, `cy`), in the strip at `top`, `sh`
/// tall, in the aids' section where `in_section`. Notes a menu's control
/// in `anchors` and what it says in `labels`. Returns its width.
fn piece(
    host: &Host,
    ui: &mut HostUi,
    p: &Piece,
    (x, cy, top, sh): (f32, f32, f32, f32),
    in_section: bool,
    anchors: &mut Vec<(Menu, f32)>,
    labels: &mut Vec<String>,
) -> f32 {
    let look = &Look::now(host);
    match p {
        Piece::Ctl(c) => {
            labels.push(c.label.clone());
            if let Kind::Opens(m) = c.kind {
                anchors.push((m, x));
            }
            control(host, ui, c, (x, cy), on_strip(look, in_section))
        }
        Piece::Head(label) => {
            labels.push(label.to_string());
            let ink = look.on_section;
            ui.text_mid_as((x, cy, t::HOST_CONTROL_H), t::TITLE_S, ink, label)
        }
        Piece::Die(r) => {
            let y = top + (sh - crate::fresh::roller::screen::ROW_H) / 2.0;
            die(host, ui, look, *r, (x, y));
            t::ROLLER_DIE
        }
        Piece::Lines(s) => {
            let ink = look.on_section;
            ui.text_mid_as((x, cy, t::HOST_CONTROL_H), t::BODY_M, ink, s)
        }
    }
}

/// The strip's scroll indicator along its foot, in a strip at `top`, `sh`
/// tall, its row `room` wide seen of `row_w`, scrolled `at` of `max`.
fn indicator(
    ui: &mut HostUi,
    look: &Look,
    (top, sh, room): (f32, f32, f32),
    (row_w, at, max): (f32, f32, f32),
) {
    let bar = t::HOST_INDICATOR;
    let tw = (room * room / row_w).max(t::TOUCH);
    let tx = t::PAD + (room - tw) * at / max;
    let y = top + sh - t::GAP_SMALL - bar;
    ui.fill(tx, y, tw, bar, bar / 2.0, look.edge);
}

/// A preview frame's focus border round `a`, `HOST_FOCUS_EDGE` wide, just
/// outside it.
fn edge(ui: &mut HostUi, a: Area, color: Color) {
    let e = super::edge_px(ui.f);
    let (x, y, w) = (a.x - e, a.y - e, a.w + 2 * e);
    for r in [
        Rect::new(x, y, w, e),
        Rect::new(x, a.y + a.h, w, e),
        Rect::new(x, a.y, e, a.h),
        Rect::new(a.x + a.w, a.y, e, a.h),
    ] {
        ui.c.fill_rect(r, color);
    }
}

/// The menu or sheet closing and the one open, each over its control at
/// `anchors`, the app's frame at `area`, the strip's top at `top` in a
/// window `w` wide. Returns the open menu's rows or the sheet's texts,
/// and where the menu or the sheet stands with the furthest its rows
/// scroll.
fn menus(
    host: &Host,
    ui: &mut HostUi,
    fitted: &Fitted,
    anchors: &[(Menu, f32)],
    (area, top, w): (Option<Area>, f32, f32),
) -> (Vec<String>, Option<(Rect, f32)>) {
    let anchor = |m: Menu| anchors.iter().find(|(a, _)| *a == m).map(|(_, x)| *x);
    if let Some((m, at)) = host.leaving {
        let p = crate::motion::progress(Some(at), host.now, crate::motion::MENU_OUT_MS);
        let shown = 1.0 - crate::motion::ease(t::EASE_STANDARD_ACCELERATE, p);
        if shown > 0.0 {
            let mark = ui.presses_mark();
            if m == Menu::Samples {
                samples_sheet(host, ui, (area, anchor(m), top, w), shown);
            } else {
                menu(host, ui, fitted, m, (anchor(m), top, w), shown);
            }
            ui.presses_rewind(mark);
        }
    }
    if let Some((m, at)) = host.menu {
        let shown = match at {
            Some(at) if host.app.fades() => {
                let p = crate::motion::progress(Some(at), host.now, crate::motion::MENU_IN_MS);
                crate::motion::ease(t::EASE_STANDARD_DECELERATE, p)
            }
            // At rest, under Reduce motion or with fades off: whole.
            _ => 1.0,
        };
        if m == Menu::Samples {
            return samples_sheet(host, ui, (area, anchor(m), top, w), shown);
        }
        let place = menu(host, ui, fitted, m, (anchor(m), top, w), shown);
        let rows = rows(host, fitted, m).into_iter().map(|r| r.label).collect();
        return (rows, place);
    }
    (Vec::new(), None)
}

/// A control's colours where it stands: its label's ink, its outline,
/// the fill of the step's main press and that press's label.
#[derive(Clone, Copy)]
struct Inks {
    ink: Color,
    outline: Color,
    main: Color,
    on_main: Color,
}

/// The inks of a control on the strip, or in the aids' section.
fn on_strip(look: &Look, in_section: bool) -> Inks {
    if in_section {
        Inks {
            ink: look.on_section,
            outline: look.on_section,
            main: look.section_main,
            on_main: look.on_section_main,
        }
    } else {
        Inks {
            ink: look.ink,
            outline: look.edge,
            main: look.accent,
            on_main: look.strip,
        }
    }
}

/// The inks of a control on a menu's or Samples' sheet's surface.
fn on_raised(look: &Look) -> Inks {
    Inks {
        ink: look.on_raised,
        outline: look.edge,
        main: look.main,
        on_main: look.on_main,
    }
}

/// One control at (`x`, `y`) in `inks`: its outline, its state layer, its
/// dot or chevron and its label, 48 dp tall, its press its whole box.
/// Returns its width.
fn control(host: &Host, ui: &mut HostUi, c: &Ctl, (x, y): (f32, f32), inks: Inks) -> f32 {
    let look = Look::now(host);
    let w = ctl_w(ui, c);
    let h = t::HOST_CONTROL_H;
    // Material's buttons are round at their ends (F108).
    let r = t::shape_full(h);
    let open = matches!(c.kind, Kind::Opens(m) if host.menu.map(|o| o.0) == Some(m));
    let off = layer(inks.ink, t::DISABLED_CONTENT);
    // (label, outline, fill)
    let (ink, edge, body) = match &c.kind {
        Kind::Primary => (inks.on_main, inks.main, Some(inks.main)),
        Kind::Off => (off, layer(inks.ink, t::DISABLED_CONTAINER), None),
        _ if open => (inks.on_main, inks.main, Some(inks.main)),
        _ => (inks.ink, inks.outline, None),
    };
    if let Some(b) = body {
        ui.fill(x, y, w, h, r, b);
    }
    if c.kind != Kind::Off {
        if ui.is_pressed(c.press) {
            ui.wash(x, y, w, h, r, layer(ink, t::STATE_PRESS));
        } else if ui.is_hovered(c.press) {
            ui.wash(x, y, w, h, r, layer(ink, t::STATE_HOVER));
        }
    }
    ui.stroke(x, y, w, h, r, edge);
    if c.mark {
        // The app asks for this stick: a primary layer, pulsing while the
        // pulse runs, under a primary outline.
        ui.wash(x, y, w, h, r, layer(inks.main, host.mark_layer()));
        // `HOST_MARK_EDGE` wide: a dp-wide outline each dp inward.
        for k in 0..t::HOST_MARK_EDGE as u8 {
            let k = f32::from(k);
            ui.stroke(x + k, y + k, w - 2.0 * k, h - 2.0 * k, r - k, inks.main);
        }
    }
    let mut tx = x + CTL_PAD;
    if let Kind::Stick { inserted, .. } = c.kind {
        let tone = if inserted {
            look.stick_in
        } else {
            look.stick_out
        };
        ui.dot(tx + t::GAP_SMALL, y + h / 2.0, t::GAP_SMALL, tone);
        tx += DOT_ROOM;
    }
    ui.text_mid_as((tx, y, h), t::LABEL_L, ink, &c.label);
    if let Kind::Opens(_) = c.kind {
        let ix = x + w - CTL_PAD - CHEVRON_ROOM + t::GAP_SMALL;
        // The menu opens upward from the strip.
        let iy = y + (h - t::ICON_SMALL) / 2.0;
        ui.icon(ix, iy, t::ICON_SMALL, Icon::ChevronUp, t::TITLE_M.size, ink);
    }
    if c.kind != Kind::Off {
        ui.hit_ringed((x, y, w, h, r), (x, y, w, h), c.press);
    }
    w
}

/// One row of a menu.
struct MenuRow {
    label: String,
    note: String,
    press: Option<HostPress>,
    chosen: bool,
    dot: Option<Color>,
}

/// The rows of menu `m`.
fn rows(host: &Host, fitted: &Fitted, m: Menu) -> Vec<MenuRow> {
    let app = &host.app;
    let look = Look::now(host);
    let row = |label: String, press: HostPress, chosen: bool| MenuRow {
        label,
        note: String::new(),
        press: Some(press),
        chosen,
        dot: None,
    };
    match m {
        Menu::Sticks => sticks(app)
            .into_iter()
            .map(|c| {
                let dot = match c.kind {
                    Kind::Stick { inserted: true, .. } => look.stick_in,
                    _ => look.stick_out,
                };
                MenuRow {
                    dot: Some(dot),
                    ..row(c.label, c.press, false)
                }
            })
            .collect(),
        Menu::Size => sizes()
            .into_iter()
            .map(|(name, s)| MenuRow {
                note: s.map_or_else(String::new, |(w, h)| {
                    fill(EN.settings_preview_dp, &[&w.to_string(), &h.to_string()])
                }),
                ..row(name.to_string(), HostPress::Size(s), host.preview == s)
            })
            .collect(),
        Menu::Play => {
            let Some(m) = app.keygen().and_then(|k| k.codex32.as_ref()) else {
                return Vec::new();
            };
            let mut v = Vec::new();
            for (head, items) in crate::codex32::walk_screen::host_play_options(m) {
                v.push(MenuRow {
                    label: head.to_string(),
                    note: String::new(),
                    press: None,
                    chosen: false,
                    dot: None,
                });
                for (name, on, act) in items {
                    v.push(row(
                        name.to_string(),
                        HostPress::App(Action::Codex32(act)),
                        on,
                    ));
                }
            }
            v
        }
        Menu::Rolls => {
            let Some((r, act, _)) = roller_shown(app).and_then(|w| roller_of(app, w)) else {
                return Vec::new();
            };
            let typed = r.per() == crate::fresh::roller::PerClick::Typed;
            crate::fresh::roller::screen::per_choices()
                .into_iter()
                .map(|(l, p)| {
                    row(
                        l.to_string(),
                        HostPress::App(act(DieAct::Per(p))),
                        r.per() == p,
                    )
                })
                .chain([row(
                    EN.roller_number.to_string(),
                    HostPress::App(act(DieAct::Count)),
                    typed,
                )])
                .collect()
        }
        // Samples' sheet is drawn by `samples_sheet`.
        Menu::Samples => Vec::new(),
        Menu::More => fitted
            .demo_more
            .iter()
            .filter(|c| c.kind != Kind::Off)
            .map(|c| row(c.label.clone(), c.press, false))
            .collect(),
    }
}

/// Costs a copy of the menu's pixels while it fades. Menu `m` over its
/// control at `x`, over the strip's top `top`, in a window `w` wide,
/// `shown` of the way faded in: Material's menu, fading in on the
/// standard decelerate and out on the standard accelerate, in the host's
/// look; at most as tall as the room above the strip, its rows scrolling
/// inside it where they are taller. Returns where it stands and the
/// furthest its rows scroll, dp.
fn menu(
    host: &Host,
    ui: &mut HostUi,
    fitted: &Fitted,
    m: Menu,
    (x, top, w): (Option<f32>, f32, f32),
    shown: f32,
) -> Option<(Rect, f32)> {
    let look = Look::now(host);
    let rows = rows(host, fitted, m);
    if rows.is_empty() {
        return None;
    }
    let row_h = |r: &MenuRow| {
        if r.press.is_none() {
            t::HOST_LINE
        } else {
            t::HOST_MENU_ROW
        }
    };
    let whole = rows.iter().map(row_h).sum::<f32>() + 2.0 * t::GAP;
    let mw = t::HOST_MENU_W.min(w - 2.0 * t::PAD);
    let mx = x.unwrap_or(t::PAD).min(w - t::PAD - mw).max(t::PAD);
    let bottom = top - t::GAP_SMALL;
    let mh = whole.min(bottom - t::PAD).max(t::HOST_MENU_ROW);
    let max = (whole - mh).max(0.0);
    let my = bottom - mh;
    // Material's menu: extra-small corners at elevation 2, an outline in
    // place of the shadow at High contrast (`docs/THEMES.md` §7, §9).
    raised(ui, (mx, my, mw, mh), shown);
    let fading = Fading::begin(ui, (mx, my, mw, mh), shown);
    ui.fill(mx, my, mw, mh, t::SHAPE_XS, look.raised);
    if ui.theme.is_high_contrast() {
        ui.stroke(mx, my, mw, mh, t::SHAPE_XS, look.edge);
    }
    // A press on the menu itself is the menu's: it closes nothing, and
    // shows no state of its own.
    ui.hit_quiet(mx, my, mw, mh, HostPress::Sheet);
    let place = ui.rect(mx, my, mw, mh);
    ui.c.push_clip(place);
    let mut ry = my + t::GAP - host.list_shown(max);
    for r in &rows {
        let rh = row_h(r);
        match r.press {
            None => {
                let dim = look.on_raised_dim;
                ui.text_mid_as((mx + t::PAD, ry, rh), t::TITLE_S, dim, &r.label);
            }
            Some(p) => {
                // The chosen item in secondary-container (Material's
                // menu).
                let ink = if r.chosen {
                    let h = look.theme;
                    ui.fill(mx, ry, mw, rh, 0.0, h.role(Role::SecondaryContainer));
                    h.role(Role::OnSecondaryContainer)
                } else {
                    look.on_raised
                };
                if ui.is_pressed(p) {
                    ui.wash(mx, ry, mw, rh, 0.0, layer(ink, t::STATE_PRESS));
                } else if ui.is_hovered(p) {
                    ui.wash(mx, ry, mw, rh, 0.0, layer(ink, t::STATE_HOVER));
                }
                let mut tx = mx + t::PAD;
                if let Some(d) = r.dot {
                    ui.dot(tx + t::GAP_SMALL, ry + rh / 2.0, t::GAP_SMALL, d);
                    tx += DOT_ROOM;
                }
                if r.chosen {
                    ui.icon(
                        tx - t::GAP_SMALL,
                        ry + (rh - t::ICON_SMALL) / 2.0,
                        t::ICON_SMALL,
                        Icon::Check,
                        t::TITLE_M.size,
                        ink,
                    );
                }
                let tx = tx + t::ICON_SMALL;
                ui.text_mid_as((tx, ry, rh), t::LABEL_L, ink, &r.label);
                if !r.note.is_empty() {
                    let nw = ui.measure_as(t::BODY_M, &r.note);
                    let nx = mx + mw - t::PAD - nw;
                    let dim = look.on_raised_dim;
                    ui.text_mid_as((nx, ry, rh), t::BODY_M, dim, &r.note);
                }
                ui.hit_ringed((mx, ry, mw, rh, 0.0), (mx, ry, mw, rh), p);
            }
        }
        ry += rh;
    }
    ui.c.pop_clip();
    fading.end(ui);
    Some((place, max))
}

/// The shadow under a menu or Samples' sheet at (x, y, w, h), `shown`
/// of the way faded in: Material's menu's, level 2, none at High
/// contrast.
fn raised(ui: &mut HostUi, (x, y, w, h): (f32, f32, f32, f32), shown: f32) {
    ui.shadow_faded((x, y, w, h, t::SHAPE_XS), t::ELEVATION_MENU, shown);
}

/// A menu or Samples' sheet fading in or out: what it stands on, kept to
/// fade from.
struct Fading {
    rect: Rect,
    under: Vec<u8>,
    shown: f32,
}

impl Fading {
    /// Costs a copy of the rectangle's pixels where `shown` is short of
    /// whole. Keeps what the rectangle (`x`, `y`, `w`, `h`, dp) stands on,
    /// a pixel more each side for its edge.
    fn begin(ui: &HostUi, (x, y, w, h): (f32, f32, f32, f32), shown: f32) -> Fading {
        let px = 1.0 / ui.f;
        let rect = ui.rect(x - px, y - px, w + 2.0 * px, h + 2.0 * px);
        let mut under = Vec::new();
        if shown < 1.0 {
            ui.c.copy_region(rect, &mut under);
        }
        Fading { rect, under, shown }
    }

    /// Costs a pass over the rectangle's pixels where it fades. What was
    /// drawn since [`Fading::begin`] shown `shown` of the way.
    fn end(self, ui: &mut HostUi) {
        if self.shown < 1.0 {
            ui.c.fade_region(self.rect, &self.under, self.shown);
        }
    }
}

/// The computer's die `which` with its top-left at (`x`, `y`), drawn by
/// the roller's own code (`fresh/roller/screen.rs`) in the host's scheme,
/// the app's theme in the opposite mode; its controls are the strip's.
fn die(host: &Host, ui: &mut HostUi, look: &Look, which: Roller, (x, y): (f32, f32)) {
    let app = &host.app;
    let Some((r, _, _)) = roller_of(app, which) else {
        return;
    };
    let mut hits: Vec<(faraday_draw::geom::Rect, Action)> = Vec::new();
    let f = ui.f;
    let mut aui = crate::ui::Ui::new(&mut *ui.c, f, &mut hits, None);
    aui.theme = look.theme;
    crate::fresh::roller::screen::die_alone(&mut aui, (x, y), r);
}

/// Where Samples' sheet stands and how tall it may grow, dp: its left
/// edge, its width and the highest its top may reach (`docs/DESKTOP-HOST.md`
/// §6). Beside a preview's frame where the host's space there holds at least
/// a menu's width, the host's own space, the side of Samples first; else
/// up from Samples at `anchor`, under the frame's foot where it fits
/// there, else over the app's foot. Arithmetic.
fn sheet_place(
    host: &Host,
    f: f32,
    (area, anchor, top, w): (Option<Area>, Option<f32>, f32, f32),
    want: f32,
) -> (f32, f32, f32) {
    let sw = t::HOST_SHEET_W.min(w - 2.0 * t::PAD);
    let up = |limit: f32| {
        let x = anchor.unwrap_or(t::PAD).min(w - t::PAD - sw).max(t::PAD);
        (x, sw, limit)
    };
    let Some(a) = area.filter(|_| host.preview.is_some()) else {
        return up(t::PAD);
    };
    // The frame's border stands just outside it.
    let e = super::edge_px(f);
    let (left, right, foot) = (
        (a.x - e) as f32 / f,
        w - (a.x + a.w + e) as f32 / f,
        (a.y + a.h + e) as f32 / f,
    );
    let gutter =
        |g: f32| Some(t::HOST_SHEET_W.min(g - 2.0 * t::PAD)).filter(|s| *s >= t::HOST_MENU_W);
    if let Some(s) = gutter(left) {
        return (t::PAD, s, t::PAD);
    }
    if let Some(s) = gutter(right) {
        return (w - t::PAD - s, s, t::PAD);
    }
    if want <= top - t::GAP_SMALL - (foot + t::GAP) {
        return up(foot + t::GAP);
    }
    up(t::PAD)
}

/// Costs a text measure per row, and a copy of the sheet's pixels while
/// it fades. Load samples' sheet in the host's look, `shown` of the way
/// faded in, standing on the strip's top `top`
/// (`docs/DESKTOP-HOST.md` §6): the title; the samples' passphrase, a line
/// where a stick is in, Select all or Unselect all, the sample wallets and
/// keys with their tick boxes (one loaded ticked and dimmed), scrolling
/// where the sheet has no room for them; Not now and Load ticked at its
/// foot. A tick and Select all keep it open; Not now, Load ticked and a
/// press beside it close it. Returns its texts, and where it stands with
/// the furthest its lists scroll.
fn samples_sheet(
    host: &Host,
    ui: &mut HostUi,
    (area, anchor, top, w): (Option<Area>, Option<f32>, f32, f32),
    shown: f32,
) -> (Vec<String>, Option<(Rect, f32)>) {
    let app = &host.app;
    let look = Look::now(host);
    if app.samples.is_none() {
        return (Vec::new(), None);
    }
    let state = app.sample_area.sheet();
    let warn = (app.selftest_passed() && !app.stick.is_empty()).then(|| {
        fill(
            EN.pull_medium_to_reason,
            &[app.medium.noun(), EN.pull_what_load_samples],
        )
    });
    let (row, line) = (t::HOST_MENU_ROW, t::HOST_LINE);
    let n = state.wallets.len() + state.keys.len();
    let body =
        line + warn.as_ref().map_or(0.0, |_| line) + row + 2.0 * line + n as f32 * row + t::GAP;
    let head = t::GAP + row;
    let foot = t::HOST_CONTROL_H + 2.0 * t::GAP;
    let want = head + body + foot;
    let (mx, mw, limit) = sheet_place(host, ui.f, (area, anchor, top, w), want);
    let bottom = top - t::GAP_SMALL;
    let mh = want.min(bottom - limit).max(head + foot);
    let my = bottom - mh;
    let view = (mh - head - foot).max(0.0);
    let max = (body - view).max(0.0);
    let scroll = host.list_shown(max);
    raised(ui, (mx, my, mw, mh), shown);
    let fading = Fading::begin(ui, (mx, my, mw, mh), shown);
    ui.fill(mx, my, mw, mh, t::SHAPE_XS, look.raised);
    if ui.theme.is_high_contrast() {
        ui.stroke(mx, my, mw, mh, t::SHAPE_XS, look.edge);
    }
    // A press on the sheet itself is the sheet's: it closes nothing, and
    // shows no state of its own.
    ui.hit_drawn(mx, my, mw, mh, HostPress::Sheet);
    let x = mx + t::PAD;
    let title = EN.samples_title;
    ui.text_mid_as(
        (x, my + t::GAP, row),
        t::sheet_head(false),
        look.on_raised,
        title,
    );
    let mut texts = vec![EN.samples_title.to_string()];
    // The lists, under the title and over the foot.
    let lists = ui.rect(mx, my + head, mw, view);
    ui.c.push_clip(lists);
    sheet_lists(host, ui, (mx, mw, my + head - scroll), warn, &mut texts);
    ui.c.pop_clip();
    sheet_foot(host, ui, (mx, mw, my + mh - foot), &mut texts);
    fading.end(ui);
    (texts, Some((ui.rect(mx, my, mw, mh), max)))
}

/// Costs a text measure per row. Samples' sheet's lists from `y`, the
/// sheet at `x` and `w` wide: the samples' passphrase, `warn` where a
/// stick is in, Select all or Unselect all, then the sample wallets and
/// keys under their headings. Adds what it says to `texts`.
fn sheet_lists(
    host: &Host,
    ui: &mut HostUi,
    (mx, mw, y): (f32, f32, f32),
    warn: Option<String>,
    texts: &mut Vec<String>,
) {
    let app = &host.app;
    let look = Look::now(host);
    let Some(set) = app.samples else {
        return;
    };
    let state = app.sample_area.sheet();
    let (row, line) = (t::HOST_MENU_ROW, t::HOST_LINE);
    let (x, iw) = (mx + t::PAD, mw - 2.0 * t::PAD);
    let mut cy = y;
    let code_w = ui.measure_as(t::BODY_M.mono(), PASSPHRASE);
    let lead = ui.fit_as(t::BODY_M, EN.samples_password, iw - code_w - t::GAP);
    let dim = look.on_raised_dim;
    let lead_w = ui.text_mid_as((x, cy, line), t::BODY_M, dim, &lead);
    let ink = look.on_raised;
    ui.text_mid_as(
        (x + lead_w + t::GAP, cy, line),
        t::BODY_M.mono(),
        ink,
        PASSPHRASE,
    );
    texts.push(format!("{} {PASSPHRASE}", EN.samples_password));
    cy += line;
    if let Some(warn) = warn {
        let shown = ui.fit_as(t::LABEL_L, &warn, iw);
        ui.text_mid_as((x, cy, line), t::LABEL_L, look.caution, &shown);
        texts.push(warn);
        cy += line;
    }
    // Unselect all while any is ticked; Select all once none is.
    let link = if state.any_ticked() {
        EN.samples_unselect_all
    } else {
        EN.samples_select_all
    };
    let all = HostPress::App(Action::SamplesAll);
    let link_w = ui.measure_as(t::LABEL_L, link) + 2.0 * t::GAP;
    let ly = cy + (row - line) / 2.0;
    if ui.is_pressed(all) {
        ui.wash(
            x - t::GAP,
            ly,
            link_w,
            line,
            t::GAP,
            layer(ink, t::STATE_PRESS),
        );
    } else if ui.is_hovered(all) {
        ui.wash(
            x - t::GAP,
            ly,
            link_w,
            line,
            t::GAP,
            layer(ink, t::STATE_HOVER),
        );
    }
    ui.text_mid_as((x, cy, row), t::LABEL_L, ink, link);
    ui.hit_ringed(
        (x - t::GAP, ly, link_w, line, t::GAP),
        (x - t::GAP, cy, link_w, row),
        all,
    );
    texts.push(link.to_string());
    cy += row;
    let wallets: Vec<String> = state
        .wallets
        .iter()
        .map(|r| crate::samples::label(set.wallets[r.sample].name, "", 0).unwrap_or_default())
        .collect();
    let keys: Vec<String> = state
        .keys
        .iter()
        .map(|r| set.keys[r.sample].label())
        .collect();
    cy += tick_list(
        ui,
        &look,
        (mx, mw, cy),
        EN.samples_wallets,
        (&state.wallets, &wallets),
        Pick::Wallet,
        texts,
    );
    tick_list(
        ui,
        &look,
        (mx, mw, cy),
        EN.samples_keys,
        (&state.keys, &keys),
        Pick::Key,
        texts,
    );
}

/// Costs a text measure per row. One list on Samples' sheet at `y`: its
/// heading, then each row with its name from `names`, its tick pressing
/// `pick` of its place. Adds what it says to `texts`; returns its height.
fn tick_list(
    ui: &mut HostUi,
    look: &Look,
    (mx, mw, y): (f32, f32, f32),
    heading: &str,
    (rows, names): (&[crate::samples::Row], &[String]),
    pick: fn(u8) -> Pick,
    texts: &mut Vec<String>,
) -> f32 {
    let x = mx + t::PAD;
    let mut cy = y;
    let dim = look.on_raised_dim;
    ui.text_mid_as((x, cy, t::HOST_LINE), t::TITLE_S, dim, heading);
    texts.push(heading.to_string());
    cy += t::HOST_LINE;
    for (i, (r, name)) in rows.iter().zip(names).enumerate() {
        let Ok(at) = u8::try_from(i) else {
            break;
        };
        let press = HostPress::App(Action::SampleTick(pick(at)));
        tick_row(ui, look, (mx, mw, cy), r, name, press);
        texts.push(name.clone());
        cy += t::HOST_MENU_ROW;
    }
    cy + t::GAP - y
}

/// Samples' sheet's foot from `y`, the sheet at `x` and `w` wide: a rule,
/// then Not now and Load ticked at its right, Load pressable once one is
/// ticked. Adds what it says to `texts`.
fn sheet_foot(
    host: &Host,
    ui: &mut HostUi,
    (mx, mw, fy): (f32, f32, f32),
    texts: &mut Vec<String>,
) {
    let app = &host.app;
    let look = Look::now(host);
    ui.fill(mx, fy, mw, 1.0 / ui.f, 0.0, look.rule);
    let load = Ctl {
        mark: false,
        label: EN.samples_load.to_string(),
        press: HostPress::App(Action::SamplesLoad),
        kind: if app.sample_area.sheet().any_ticked() && app.selftest_passed() {
            Kind::Primary
        } else {
            Kind::Off
        },
    };
    let not_now = Ctl {
        mark: false,
        label: EN.samples_not_now.to_string(),
        press: HostPress::Menu(Menu::Samples),
        kind: Kind::Button,
    };
    let by = fy + t::GAP;
    let lx = mx + mw - t::PAD - ctl_w(ui, &load);
    let nx = lx - t::GAP - ctl_w(ui, &not_now);
    control(host, ui, &not_now, (nx, by), on_raised(&look));
    control(host, ui, &load, (lx, by), on_raised(&look));
    texts.push(not_now.label);
    texts.push(load.label);
}

/// One sample's row on Samples' sheet at `y`, the sheet at `x` and `w`
/// wide: its tick box, ticked once it is loaded too and then dimmed and
/// taking no press, its name, and what it is at the row's end.
fn tick_row(
    ui: &mut HostUi,
    look: &Look,
    (x, w, y): (f32, f32, f32),
    r: &crate::samples::Row,
    label: &str,
    press: HostPress,
) {
    let row = t::HOST_MENU_ROW;
    let on = !r.loaded;
    let ink = look.on_raised;
    if on {
        if ui.is_pressed(press) {
            ui.wash(x, y, w, row, 0.0, layer(ink, t::STATE_PRESS));
        } else if ui.is_hovered(press) {
            ui.wash(x, y, w, row, 0.0, layer(ink, t::STATE_HOVER));
        }
    }
    let (ink, tick) = if on {
        (ink, look.main)
    } else {
        let off = layer(ink, t::DISABLED_CONTENT);
        (off, off)
    };
    let b = t::ICON_SMALL;
    let (bx, by) = (x + t::PAD, y + (row - b) / 2.0);
    if r.ticked || r.loaded {
        ui.wash(bx, by, b, b, t::GAP_SMALL, tick);
        ui.icon(bx, by, b, Icon::Check, t::BODY_M.size, look.on_main);
    } else {
        ui.stroke(bx, by, b, b, t::GAP_SMALL, look.on_raised_dim);
    }
    let tx = bx + b + t::GAP + t::GAP_SMALL;
    let room = x + w - t::PAD - tx;
    let name = ui.fit_as(t::BODY_L, label, room);
    let name_w = ui.text_mid_as((tx, y, row), t::BODY_L, ink, &name);
    let note_room = room - name_w - t::GAP;
    if note_room > t::TOUCH {
        let note = ui.fit_as(t::BODY_M, &r.kind, note_room);
        let nw = ui.measure_as(t::BODY_M, &note);
        let dim = if on { look.on_raised_dim } else { ink };
        ui.text_mid_as((x + w - t::PAD - nw, y, row), t::BODY_M, dim, &note);
    }
    if on {
        ui.hit_ringed((x, y, w, row, 0.0), (x, y, w, row), press);
    }
}
