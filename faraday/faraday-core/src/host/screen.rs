//! The host's strip and menus (`docs/DESKTOP-HOST.md` §2, §3), drawn in
//! the host's own look: the dark surface round a preview's frame, small
//! type, outlined controls in light ink, never the app's surface, colours
//! or components. Left to right: the stick pills (the boot stick first;
//! "Sticks · N" holding those that do not fit), Samples, Size, then the
//! screen's demo aids under "Practice", those with no room in More. Drawing
//! reads the app's state and what its last frame noted; it works nothing
//! out but where a control goes.

use faraday_draw::Color;
use faraday_draw::geom::Rect;
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_ui::W;
use faraday_ui::demo::DemoItem;

use super::{Area, Host, HostMenu as Menu, HostPress};
use crate::fresh::roller::Act as DieAct;
use crate::samples::{PASSPHRASE, Pick, Roller};
use crate::strings::{EN, fill};
use crate::{Action, Faraday};

/// The dark space round a preview's frame: the window's own.
pub(crate) const SURROUND: Color = Color::rgb(0x11, 0x16, 0x1c);
/// The strip's surface, a shade darker than the space above it.
const STRIP: Color = Color::rgb(0x0b, 0x0f, 0x14);
/// The rule along the strip's top and between its groups.
const RULE: Color = Color::rgb(0x2a, 0x33, 0x3d);
/// A control's outline, and the preview frame's edge.
const EDGE: Color = Color::rgb(0x3a, 0x46, 0x52);
/// Text and icons.
const INK: Color = Color::rgb(0xd5, 0xdc, 0xe4);
/// Secondary text: a menu row's size, the demo's lines.
const INK_DIM: Color = Color::rgb(0x8b, 0x96, 0xa3);
/// A control that takes no press now.
const INK_OFF: Color = Color::rgb(0x55, 0x60, 0x6c);
/// A menu's surface, raised over the strip and the frame.
const RAISED: Color = Color::rgb(0x1a, 0x21, 0x29);
/// Hovered: a light state layer.
const HOVER: Color = Color::rgba(255, 255, 255, 20);
/// Pressed: a stronger state layer.
const PRESS: Color = Color::rgba(255, 255, 255, 40);
/// A stick that is in.
const STICK_IN: Color = Color::rgb(0x3f, 0xb9, 0x50);
/// A stick that is out.
const STICK_OUT: Color = Color::rgb(0x6e, 0x76, 0x81);
/// The demo's mark: its head and a stick the flow asks for.
const DEMO: Color = Color::rgb(0xd2, 0x99, 0x22);

type HostUi<'a> = faraday_ui::Ui<'a, HostPress>;

/// What a control on the strip is.
#[derive(Clone, PartialEq)]
enum Kind {
    /// A stick's pill, its dot lit while it is in, edged while the flow
    /// asks for it.
    Stick { inserted: bool, asked: bool },
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
    label: String,
    press: HostPress,
    kind: Kind,
}

/// The padding each side of a control's label.
const CTL_PAD: f32 = t::GAP + t::GAP_SMALL;
/// The room a stick's dot takes before its label.
const DOT_ROOM: f32 = t::GAP + t::GAP_SMALL;
/// The room a menu's chevron takes after its label.
const CHEVRON_ROOM: f32 = t::ICON_SMALL;

fn ctl_w(ui: &HostUi, c: &Ctl) -> f32 {
    let extra = match c.kind {
        Kind::Stick { .. } => DOT_ROOM,
        Kind::Opens(_) => CHEVRON_ROOM,
        _ => 0.0,
    };
    ui.measure(t::CAPTION, W::S, &c.label) + 2.0 * CTL_PAD + extra
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
                label: fill(EN.stick_pill_state, &[&p.label, state]),
                press: HostPress::App(Action::StickPill(p.n as u8)),
                kind: Kind::Stick {
                    inserted: p.inserted,
                    asked: p.asked,
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
            label: EN.roller_stop.to_string(),
            press: HostPress::App(act(DieAct::Stop)),
            kind: Kind::Button,
        }
    } else {
        Ctl {
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
            label: fill(EN.host_rolls_per_click, &[&per]),
            press: HostPress::Menu(Menu::Rolls),
            kind: Kind::Opens(Menu::Rolls),
        },
    ]
}

/// The preview sizes the Size menu lists, each its name and its size in
/// dp; `None` is the window itself.
fn sizes() -> [(&'static str, Option<(u16, u16)>); 8] {
    let s = t::PREVIEW_SIZES;
    [
        (EN.settings_preview_this_window, None),
        (EN.settings_preview_pi, Some(s[0])),
        (EN.settings_preview_narrow, Some(s[1])),
        (EN.settings_preview_half, Some(s[2])),
        (EN.settings_preview_7in, Some(s[3])),
        (EN.settings_preview_10in, Some(s[4])),
        (EN.settings_preview_desktop, Some(s[5])),
        (EN.settings_preview_monitor, Some(s[6])),
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
    /// "Practice" heads the aids, the die after it, rules between the
    /// groups; else none, and More reads "Practice".
    head: bool,
    demo: Vec<Ctl>,
    demo_more: Vec<Ctl>,
    lines: Vec<String>,
}

/// Size's control: by its size's name, or `short`, "Size" alone.
fn size_ctl(host: &Host, short: bool) -> Ctl {
    Ctl {
        label: if short {
            EN.host_size_short.to_string()
        } else {
            fill(EN.host_size, &[size_name(host.preview)])
        },
        press: HostPress::Menu(Menu::Size),
        kind: Kind::Opens(Menu::Size),
    }
}

/// "Sticks · N", opening the menu of every stick.
fn sticks_ctl(n: usize) -> Ctl {
    Ctl {
        label: fill(EN.stick_pill_more, &[&n.to_string()]),
        press: HostPress::Menu(Menu::Sticks),
        kind: Kind::Opens(Menu::Sticks),
    }
}

/// More, opening the aids with no room; narrowest (`!head`), it reads
/// "Practice".
fn more_ctl(head: bool) -> Ctl {
    Ctl {
        label: if head { EN.host_more } else { EN.host_demo }.to_string(),
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
/// stick and as many as fit; then the aids that do not fit go into More;
/// then the sticks stand as "Sticks · N" alone; narrowest, Size by its
/// name alone and Samples in More.
fn fit(host: &Host, ui: &HostUi, room: f32, die_w: f32) -> Fitted {
    let app = &host.app;
    let all_sticks = sticks(app);
    let samples = app.samples_on().then(|| Ctl {
        label: EN.samples_pill_short.to_string(),
        press: HostPress::Menu(Menu::Samples),
        kind: Kind::Button,
    });
    let size = |short: bool| size_ctl(host, short);
    let (mut demo, lines) = demo_ctls(app);
    let more_ctl = more_ctl(true);
    let sum = |v: &[Ctl]| -> f32 { v.iter().map(|c| ctl_w(ui, c) + t::GAP).sum() };
    let divider = 2.0 * t::GAP + 1.0;
    let head = |any: bool| {
        if any {
            divider + ui.measure(t::CAPTION, W::S, EN.host_demo) + t::GAP + die_w
        } else {
            0.0
        }
    };
    let sticks_more = sticks_ctl;
    let n = all_sticks.len();
    let any_aid = !demo.is_empty() || die_w > 0.0;
    let mut right: Vec<Ctl> = samples.iter().cloned().chain([size(false)]).collect();
    let fixed = |right: &[Ctl], any: bool| sum(right) + divider + head(any);
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
        // Narrowest: no "Practice" head, no die and no rules; the sticks'
        // menu reads "Sticks" and More itself "Practice".
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
        let mut used = more_w;
        let mut kept = Vec::new();
        for c in demo {
            let w = ctl_w(ui, &c) + t::GAP;
            if demo_more
                .iter()
                .all(|m| m.press == HostPress::Menu(Menu::Samples))
                && used + w <= left
            {
                used += w;
                kept.push(c);
            } else {
                demo_more.push(c);
            }
        }
        demo = kept;
    }
    Fitted {
        sticks: shown,
        right,
        head,
        demo,
        demo_more,
        lines,
    }
}

/// What the host drew: what the strip's controls say, left to right, then
/// the open menu's rows or Samples' sheet's texts; and Samples' sheet,
/// where it stands in the window's pixels and the furthest its lists
/// scroll, dp.
pub(crate) type Drawn = (Vec<String>, Option<(Rect, f32)>);

/// Draws the host round the app's frame at `area`: the preview's edge,
/// the strip and its controls, and the menu or Samples' sheet open or
/// closing.
pub(crate) fn draw(host: &Host, ui: &mut HostUi, area: Option<Area>) -> Drawn {
    let f = ui.f;
    let (w, h) = (ui.c.width() as f32 / f, ui.c.height() as f32 / f);
    if host.preview.is_some()
        && let Some(a) = area
    {
        edge(ui, a);
    }
    let sh = host.strip_dp();
    let top = h - sh;
    ui.fill(0.0, top, w, sh, 0.0, STRIP);
    ui.fill(0.0, top, w, 1.0 / f, 0.0, RULE);
    let shown = roller_shown(&host.app);
    let die_w = shown.map_or(0.0, |_| t::ROLLER_DIE + t::GAP);
    let fitted = fit(host, ui, w - 2.0 * t::PAD, die_w);
    let cy = top + (sh - t::HOST_CONTROL_H) / 2.0;
    let mut x = t::PAD;
    let mut anchors: Vec<(Menu, f32)> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut put = |ui: &mut HostUi, c: &Ctl, x: &mut f32, labels: &mut Vec<String>| {
        labels.push(c.label.clone());
        if let Kind::Opens(m) = c.kind {
            anchors.push((m, *x));
        }
        *x += control(host, ui, c, (*x, cy)) + t::GAP;
    };
    for c in &fitted.sticks {
        put(ui, c, &mut x, &mut labels);
    }
    if !fitted.sticks.is_empty() && fitted.head {
        x = rule(ui, x, top, sh);
    }
    for c in &fitted.right {
        put(ui, c, &mut x, &mut labels);
    }
    let demo_group = !fitted.demo.is_empty() || !fitted.demo_more.is_empty() || shown.is_some();
    if demo_group {
        if fitted.head {
            x = rule(ui, x, top, sh);
            let hw = ui.text_mid(
                x,
                cy,
                t::HOST_CONTROL_H,
                t::CAPTION,
                W::S,
                DEMO,
                EN.host_demo,
            );
            x += hw + t::GAP;
            labels.push(EN.host_demo.to_string());
            if let Some(r) = shown {
                let y = top + (sh - crate::fresh::roller::screen::ROW_H) / 2.0;
                die(host, ui, r, (x, y));
                x += t::ROLLER_DIE + t::GAP;
            }
        }
        for c in &fitted.demo {
            put(ui, c, &mut x, &mut labels);
        }
        if !fitted.demo_more.is_empty() {
            put(ui, &more_ctl(fitted.head), &mut x, &mut labels);
            labels.extend(fitted.demo_more.iter().map(|c| c.label.clone()));
        }
    }
    // What Practice puts in place of the person's own, in the room left.
    let room = w - t::PAD - x;
    if !fitted.lines.is_empty() && room > t::TOUCH {
        let text = fitted.lines.join(" · ");
        let text = ui.fit(t::CAPTION, W::R, &text, room);
        ui.text_mid(x, cy, t::HOST_CONTROL_H, t::CAPTION, W::R, INK_DIM, &text);
    }
    labels.extend(fitted.lines.iter().cloned());
    let (rows, sheet) = menus(host, ui, &fitted, &anchors, (area, top, w));
    labels.extend(rows);
    (labels, sheet)
}

/// A preview frame's pixel-wide edge round `a`.
fn edge(ui: &mut HostUi, a: Area) {
    let f = ui.f;
    let (x, y) = (a.x as f32 / f - 1.0 / f, a.y as f32 / f - 1.0 / f);
    let (aw, ah) = (a.w as f32 / f + 2.0 / f, a.h as f32 / f + 2.0 / f);
    ui.stroke(x, y, aw, ah, 0.0, EDGE);
}

/// The menu or sheet closing and the one open, each over its control at
/// `anchors`, the app's frame at `area`, the strip's top at `top` in a
/// window `w` wide. Returns the open menu's rows or the sheet's texts,
/// and the sheet where it is open.
fn menus(
    host: &Host,
    ui: &mut HostUi,
    fitted: &Fitted,
    anchors: &[(Menu, f32)],
    (area, top, w): (Option<Area>, f32, f32),
) -> Drawn {
    let anchor = |m: Menu| anchors.iter().find(|(a, _)| *a == m).map(|(_, x)| *x);
    if let Some((m, at)) = host.leaving {
        let p = crate::motion::progress(Some(at), host.now, crate::motion::MENU_OUT_MS);
        let shown = 1.0 - crate::motion::ease(t::EASE_EMPHASIZED_ACCELERATE, p);
        if shown > 0.0 {
            let mark = ui.hits.len();
            if m == Menu::Samples {
                samples_sheet(host, ui, (area, anchor(m), top, w), shown);
            } else {
                menu(host, ui, fitted, m, (anchor(m), top, w), shown);
            }
            ui.hits.truncate(mark);
        }
    }
    if let Some((m, at)) = host.menu {
        let shown = match at {
            Some(at) if !host.app.reduce_motion() => {
                let p = crate::motion::progress(Some(at), host.now, crate::motion::MENU_IN_MS);
                crate::motion::ease_open(p)
            }
            // At rest, or under Reduce motion: whole.
            _ => 1.0,
        };
        if m == Menu::Samples {
            return samples_sheet(host, ui, (area, anchor(m), top, w), shown);
        }
        menu(host, ui, fitted, m, (anchor(m), top, w), shown);
        let rows = rows(host, fitted, m).into_iter().map(|r| r.label).collect();
        return (rows, None);
    }
    (Vec::new(), None)
}

/// A rule between two groups at `x`. Returns where the next group starts.
fn rule(ui: &mut HostUi, x: f32, top: f32, sh: f32) -> f32 {
    let rh = t::HOST_CONTROL_H - t::GAP;
    ui.fill(x, top + (sh - rh) / 2.0, 1.0 / ui.f, rh, 0.0, RULE);
    x + 1.0 / ui.f + t::GAP + t::GAP
}

/// One control at (`x`, `y`): its outline, its state layer, its dot or
/// chevron and its label; its press over the strip's height, Material's
/// 48 dp target. Returns its width.
fn control(host: &Host, ui: &mut HostUi, c: &Ctl, (x, y): (f32, f32)) -> f32 {
    let w = ctl_w(ui, c);
    let h = t::HOST_CONTROL_H;
    let r = match c.kind {
        Kind::Stick { .. } => h / 2.0,
        _ => t::GAP,
    };
    let open = matches!(c.kind, Kind::Opens(m) if host.menu.map(|o| o.0) == Some(m));
    let (ink, edge, body) = match &c.kind {
        Kind::Primary => (STRIP, INK, Some(INK)),
        Kind::Off => (INK_OFF, Color::rgb(0x2a, 0x33, 0x3d), None),
        Kind::Stick { asked: true, .. } => (INK, DEMO, None),
        _ if open => (INK, INK_DIM, Some(Color::rgba(255, 255, 255, 28))),
        _ => (INK, EDGE, None),
    };
    if let Some(b) = body {
        ui.fill(x, y, w, h, r, b);
    }
    if c.kind != Kind::Off {
        if ui.is_pressed(c.press) {
            ui.fill(x, y, w, h, r, PRESS);
        } else if ui.is_hovered(c.press) {
            ui.fill(x, y, w, h, r, HOVER);
        }
    }
    ui.stroke(x, y, w, h, r, edge);
    let mut tx = x + CTL_PAD;
    if let Kind::Stick { inserted, .. } = c.kind {
        let tone = if inserted { STICK_IN } else { STICK_OUT };
        ui.dot(tx + t::GAP_SMALL, y + h / 2.0, t::GAP_SMALL, tone);
        tx += DOT_ROOM;
    }
    ui.text_mid(tx, y, h, t::CAPTION, W::S, ink, &c.label);
    if let Kind::Opens(_) = c.kind {
        let ix = x + w - CTL_PAD - CHEVRON_ROOM + t::GAP_SMALL;
        // The menu opens upward from the strip.
        let iy = y + (h - t::ICON_SMALL) / 2.0;
        ui.icon(ix, iy, t::ICON_SMALL, Icon::ChevronUp, t::CAPTION, ink);
    }
    if c.kind != Kind::Off {
        let top = y - (t::TOUCH - h) / 2.0;
        ui.hit(x, top, w, t::TOUCH, c.press);
    }
    w
}

/// One row of a menu.
struct Row {
    label: String,
    note: String,
    press: Option<HostPress>,
    chosen: bool,
    dot: Option<Color>,
}

/// The rows of menu `m`.
fn rows(host: &Host, fitted: &Fitted, m: Menu) -> Vec<Row> {
    let app = &host.app;
    let row = |label: String, press: HostPress, chosen: bool| Row {
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
                    Kind::Stick { inserted: true, .. } => STICK_IN,
                    _ => STICK_OUT,
                };
                Row {
                    dot: Some(dot),
                    ..row(c.label, c.press, false)
                }
            })
            .collect(),
        Menu::Size => sizes()
            .into_iter()
            .map(|(name, s)| Row {
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
                v.push(Row {
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

/// Menu `m` opening up from its control at `x`, over the strip's top
/// `top`, in a window `w` wide, `shown` of the way open: Material's menu,
/// growing from its control on the emphasized decelerate and leaving on
/// the emphasized accelerate, in the host's look.
fn menu(
    host: &Host,
    ui: &mut HostUi,
    fitted: &Fitted,
    m: Menu,
    (x, top, w): (Option<f32>, f32, f32),
    shown: f32,
) {
    let rows = rows(host, fitted, m);
    if rows.is_empty() {
        return;
    }
    let row_h = |r: &Row| {
        if r.press.is_none() {
            t::HOST_CONTROL_H
        } else {
            t::HOST_MENU_ROW
        }
    };
    let mh = rows.iter().map(row_h).sum::<f32>() + 2.0 * t::GAP;
    let mw = t::HOST_MENU_W;
    let mx = x.unwrap_or(t::PAD).min(w - t::PAD - mw).max(t::PAD);
    let bottom = top - t::GAP_SMALL;
    let my = bottom - mh;
    let seen = mh * shown.clamp(0.0, 1.0);
    let clip = ui.rect(mx, bottom - seen, mw, seen);
    ui.c.push_clip(clip);
    ui.fill(mx, my, mw, mh, t::GAP_SMALL, RAISED);
    ui.stroke(mx, my, mw, mh, t::GAP_SMALL, EDGE);
    let mut ry = my + t::GAP;
    for r in &rows {
        let rh = row_h(r);
        match r.press {
            None => {
                ui.text_mid(mx + t::PAD, ry, rh, t::CAPTION, W::S, INK_DIM, &r.label);
            }
            Some(p) => {
                if ui.is_pressed(p) {
                    ui.fill(mx, ry, mw, rh, 0.0, PRESS);
                } else if ui.is_hovered(p) {
                    ui.fill(mx, ry, mw, rh, 0.0, HOVER);
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
                        t::LABEL,
                        INK,
                    );
                }
                let tx = tx + t::ICON_SMALL;
                ui.text_mid(tx, ry, rh, t::LABEL, W::R, INK, &r.label);
                if !r.note.is_empty() {
                    let nw = ui.measure(t::CAPTION, W::R, &r.note);
                    let nx = mx + mw - t::PAD - nw;
                    ui.text_mid(nx, ry, rh, t::CAPTION, W::R, INK_DIM, &r.note);
                }
                ui.hit(mx, ry, mw, rh, p);
            }
        }
        ry += rh;
    }
    ui.c.pop_clip();
}

/// The computer's die `which` with its top-left at (`x`, `y`), drawn by
/// the roller's own code (`fresh/roller/screen.rs`) in the dark scheme of
/// the person's theme; its controls are the strip's.
fn die(host: &Host, ui: &mut HostUi, which: Roller, (x, y): (f32, f32)) {
    let app = &host.app;
    let Some((r, _, _)) = roller_of(app, which) else {
        return;
    };
    let mut hits: Vec<(faraday_draw::geom::Rect, Action)> = Vec::new();
    let f = ui.f;
    let mut aui = crate::ui::Ui::new(&mut *ui.c, f, &mut hits, None);
    let theme = app.theme();
    aui.theme = faraday_ui::Theme::new(theme.name, faraday_ui::Mode::Dark, theme.contrast);
    crate::fresh::roller::screen::die_alone(&mut aui, (x, y), r);
}

/// Where Samples' sheet stands and how tall it may grow, dp: its left
/// edge, its width and the highest its top may reach (`docs/DESKTOP-HOST.md`
/// §6). Beside a preview's frame where the dark space there holds at least
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
    // The frame's edge stands a pixel outside it.
    let (left, right, foot) = (
        (a.x - 1) as f32 / f,
        w - (a.x + a.w + 1) as f32 / f,
        (a.y + a.h + 1) as f32 / f,
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

/// Costs a text measure per row. Load samples' sheet in the host's look,
/// `shown` of the way open, rising from the strip's top `top`
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
) -> Drawn {
    let app = &host.app;
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
    let (row, line) = (t::HOST_MENU_ROW, t::HOST_CONTROL_H);
    let n = state.wallets.len() + state.keys.len();
    let body =
        line + warn.as_ref().map_or(0.0, |_| line) + row + 2.0 * line + n as f32 * row + t::GAP;
    let head = t::GAP + row;
    let foot = row + 2.0 * t::GAP;
    let want = head + body + foot;
    let (mx, mw, limit) = sheet_place(host, ui.f, (area, anchor, top, w), want);
    let bottom = top - t::GAP_SMALL;
    let mh = want.min(bottom - limit).max(head + foot);
    let my = bottom - mh;
    let view = (mh - head - foot).max(0.0);
    let max = (body - view).max(0.0);
    let scroll = host.sheet_scroll.min(max);
    let seen = mh * shown.clamp(0.0, 1.0);
    let clip = ui.rect(mx, bottom - seen, mw, seen);
    ui.c.push_clip(clip);
    ui.fill(mx, my, mw, mh, t::GAP_SMALL, RAISED);
    ui.stroke(mx, my, mw, mh, t::GAP_SMALL, EDGE);
    // A press on the sheet itself is the sheet's: it closes nothing.
    ui.hit(mx, my, mw, mh, HostPress::Sheet);
    let x = mx + t::PAD;
    ui.text_mid(x, my + t::GAP, row, t::LABEL, W::S, INK, EN.samples_title);
    let mut texts = vec![EN.samples_title.to_string()];
    // The lists, under the title and over the foot.
    let lists = ui.rect(mx, my + head, mw, view);
    ui.c.push_clip(lists);
    sheet_lists(host, ui, (mx, mw, my + head - scroll), warn, &mut texts);
    ui.c.pop_clip();
    sheet_foot(host, ui, (mx, mw, my + mh - foot), &mut texts);
    ui.c.pop_clip();
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
    let Some(set) = app.samples else {
        return;
    };
    let state = app.sample_area.sheet();
    let (row, line) = (t::HOST_MENU_ROW, t::HOST_CONTROL_H);
    let (x, iw) = (mx + t::PAD, mw - 2.0 * t::PAD);
    let mut cy = y;
    let code_w = ui.measure(t::CAPTION, W::M, PASSPHRASE);
    let lead = ui.fit(t::CAPTION, W::R, EN.samples_password, iw - code_w - t::GAP);
    let lead_w = ui.text_mid(x, cy, line, t::CAPTION, W::R, INK_DIM, &lead);
    ui.text_mid(
        x + lead_w + t::GAP,
        cy,
        line,
        t::CAPTION,
        W::M,
        INK,
        PASSPHRASE,
    );
    texts.push(format!("{} {PASSPHRASE}", EN.samples_password));
    cy += line;
    if let Some(warn) = warn {
        let shown = ui.fit(t::CAPTION, W::S, &warn, iw);
        ui.text_mid(x, cy, line, t::CAPTION, W::S, DEMO, &shown);
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
    let link_w = ui.measure(t::LABEL, W::S, link) + 2.0 * t::GAP;
    let ly = cy + (row - line) / 2.0;
    if ui.is_pressed(all) {
        ui.fill(x - t::GAP, ly, link_w, line, t::GAP, PRESS);
    } else if ui.is_hovered(all) {
        ui.fill(x - t::GAP, ly, link_w, line, t::GAP, HOVER);
    }
    ui.text_mid(x, cy, row, t::LABEL, W::S, INK, link);
    ui.hit(x - t::GAP, cy, link_w, row, all);
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
        (mx, mw, cy),
        EN.samples_wallets,
        (&state.wallets, &wallets),
        Pick::Wallet,
        texts,
    );
    tick_list(
        ui,
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
    (mx, mw, y): (f32, f32, f32),
    heading: &str,
    (rows, names): (&[crate::samples::Row], &[String]),
    pick: fn(u8) -> Pick,
    texts: &mut Vec<String>,
) -> f32 {
    let x = mx + t::PAD;
    let mut cy = y;
    ui.text_mid(x, cy, t::HOST_CONTROL_H, t::CAPTION, W::S, INK_DIM, heading);
    texts.push(heading.to_string());
    cy += t::HOST_CONTROL_H;
    for (i, (r, name)) in rows.iter().zip(names).enumerate() {
        let Ok(at) = u8::try_from(i) else {
            break;
        };
        let press = HostPress::App(Action::SampleTick(pick(at)));
        tick_row(ui, (mx, mw, cy), r, name, press);
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
    ui.fill(mx, fy, mw, 1.0 / ui.f, 0.0, RULE);
    let load = Ctl {
        label: EN.samples_load.to_string(),
        press: HostPress::App(Action::SamplesLoad),
        kind: if app.sample_area.sheet().any_ticked() && app.selftest_passed() {
            Kind::Primary
        } else {
            Kind::Off
        },
    };
    let not_now = Ctl {
        label: EN.samples_not_now.to_string(),
        press: HostPress::Menu(Menu::Samples),
        kind: Kind::Button,
    };
    let by = fy + t::GAP + (t::HOST_MENU_ROW - t::HOST_CONTROL_H) / 2.0;
    let lx = mx + mw - t::PAD - ctl_w(ui, &load);
    let nx = lx - t::GAP - ctl_w(ui, &not_now);
    control(host, ui, &not_now, (nx, by));
    control(host, ui, &load, (lx, by));
    texts.push(not_now.label);
    texts.push(load.label);
}

/// One sample's row on Samples' sheet at `y`, the sheet at `x` and `w`
/// wide: its tick box, ticked once it is loaded too and then dimmed and
/// taking no press, its name, and what it is at the row's end.
fn tick_row(
    ui: &mut HostUi,
    (x, w, y): (f32, f32, f32),
    r: &crate::samples::Row,
    label: &str,
    press: HostPress,
) {
    let row = t::HOST_MENU_ROW;
    let on = !r.loaded;
    if on {
        if ui.is_pressed(press) {
            ui.fill(x, y, w, row, 0.0, PRESS);
        } else if ui.is_hovered(press) {
            ui.fill(x, y, w, row, 0.0, HOVER);
        }
    }
    let ink = if on { INK } else { INK_OFF };
    let b = t::ICON_SMALL;
    let (bx, by) = (x + t::PAD, y + (row - b) / 2.0);
    if r.ticked || r.loaded {
        ui.fill(bx, by, b, b, t::GAP_SMALL, ink);
        ui.icon(bx, by, b, Icon::Check, t::LABEL, RAISED);
    } else {
        ui.stroke(bx, by, b, b, t::GAP_SMALL, INK_DIM);
    }
    let tx = bx + b + t::GAP + t::GAP_SMALL;
    let room = x + w - t::PAD - tx;
    let name = ui.fit(t::LABEL, W::R, label, room);
    let name_w = ui.text_mid(tx, y, row, t::LABEL, W::R, ink, &name);
    let note_room = room - name_w - t::GAP;
    if note_room > t::TOUCH {
        let note = ui.fit(t::CAPTION, W::R, &r.kind, note_room);
        let nw = ui.measure(t::CAPTION, W::R, &note);
        ui.text_mid(
            x + w - t::PAD - nw,
            y,
            row,
            t::CAPTION,
            W::R,
            INK_DIM,
            &note,
        );
    }
    if on {
        ui.hit(x, y, w, row, press);
    }
}
