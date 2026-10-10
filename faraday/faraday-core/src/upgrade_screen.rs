//! Upgrade a Faraday stick: the three steps, each with what it found.
//! Labels and values only; what the upgrade is and how Secure Boot goes
//! through it are on its Learn page.

use crate::screens::title;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::upgrade::{Fit, Step, UpgradeState, version};
use crate::{Action, BootPart, Faraday, Screen};

/// A row of the list of sticks to upgrade.
const ROW: f32 = 44.0;

/// The disk a boot partition is on, as the kernel names it: `sdb` of
/// `sdb1@sdb#7`.
fn disk(part: &BootPart) -> &str {
    part.id
        .split_once('@')
        .map_or(part.id.as_str(), |(_, d)| d.split('#').next().unwrap_or(d))
}

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(760.0))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = if compact { 14.0 } else { 36.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        title(ui, x, y, app.medium.upgrade());
        y += 56.0;
    }
    let empty = UpgradeState::default();
    let u = app.upgrade.as_ref().unwrap_or(&empty);
    let step = u.step();
    y += source(app, ui, u, step, x, y, w) + 20.0;
    y += target(app, ui, u, step, x, y, w) + 20.0;
    y += done(app, ui, u, step, x, y, w);
    ui.c.pop_clip();
    app.content_h.set(y - top + 24.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}

/// A step's number, or a tick once it is behind, and its label. Returns
/// its height.
fn heading(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    n: u8,
    label: &str,
    (now, past): (bool, bool),
) -> f32 {
    let (fill, ink) = if past {
        (OK.with_alpha(40), OK)
    } else if now {
        (ACCENT, BG)
    } else {
        (INNER, DIM)
    };
    ui.fill(x, y, 26.0, 26.0, 13.0, fill);
    if past {
        ui.icon(x, y, 26.0, osk_ui::widgets::Icon::Check, 12.0, ink);
    } else {
        let s = n.to_string();
        let tw = ui.measure(13.0, W::S, &s);
        ui.text_mid(x + 13.0 - tw / 2.0, y, 26.0, 13.0, W::S, ink, &s);
    }
    let tone = if now { TEXT } else { MUTED };
    let th = ui.wrap(x + 38.0, y + 3.0, w - 38.0, 16.0, W::S, tone, label);
    th.max(26.0) + 12.0
}

/// A label and its value on one row, the value at the right.
fn value_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    key: &str,
    value: &str,
    tone: osk_ui::Color,
) -> f32 {
    if ui.compact {
        return crate::compact::kv(ui, x, y, w, key, value, tone);
    }
    let kw = ui.measure(13.0, W::R, key) + 24.0;
    ui.text_mid(x, y, 36.0, 13.0, W::R, MUTED, key);
    let v = ui.fit(14.0, W::R, value, w - kw);
    ui.text_right(x + w, y, 36.0, 14.0, W::R, tone, &v);
    ui.rule(x, y + 36.0, w, INNER);
    38.0
}

/// Step 1: the stick Faraday started from. Returns its height.
fn source(
    app: &Faraday,
    ui: &mut Ui,
    u: &UpgradeState,
    step: Step,
    x: f32,
    y0: f32,
    w: f32,
) -> f32 {
    let mut y = y0;
    let label = format!("Insert the {} Faraday started from", app.medium.noun());
    y += heading(
        ui,
        x,
        y,
        w,
        1,
        &label,
        (step == Step::Source, step != Step::Source),
    );
    let ix = x + 38.0;
    let iw = w - 38.0;
    if let Some((_, release, _)) = &u.source {
        y += value_row(ui, ix, y, iw, "Read", &version(Some(release)), TEXT);
    } else if u.reading {
        y += value_row(ui, ix, y, iw, "Reading", "", MUTED);
    } else if let Some(why) = &u.refused {
        y += ui.wrap(ix, y, iw, 13.0, W::S, ERR, why) + 8.0;
    }
    y - y0
}

/// Step 2: the stick to upgrade, its version and the one to be written,
/// and Upgrade. Returns its height.
fn target(
    app: &Faraday,
    ui: &mut Ui,
    u: &UpgradeState,
    step: Step,
    x: f32,
    y0: f32,
    w: f32,
) -> f32 {
    let mut y = y0;
    let label = format!("Remove it. Insert the {} to upgrade", app.medium.noun());
    y += heading(
        ui,
        x,
        y,
        w,
        2,
        &label,
        (step == Step::Target, step == Step::Done),
    );
    if step != Step::Target {
        return y - y0;
    }
    let ix = x + 38.0;
    let iw = w - 38.0;
    let targets = u.targets();
    let shown = u.target();
    // More than one in: which one.
    if targets.len() > 1 {
        ui.rule(ix, y, iw, INNER);
        for (i, p) in targets.iter().enumerate() {
            let action = Action::UpgradePick(i as u8);
            let on = shown.is_some_and(|s| s.id == p.id);
            if on || ui.is_pressed(action) {
                ui.fill(ix - 8.0, y, iw + 16.0, ROW, 8.0, INNER);
            }
            let name = format!("{} {}", app.medium.cap(), disk(p));
            ui.text_mid(ix, y, ROW, 13.0, W::S, if on { TEXT } else { MUTED }, &name);
            let v = version(p.release.as_deref());
            ui.text_right(ix + iw, y, ROW, 12.0, W::R, MUTED, &v);
            ui.hit(ix - 8.0, y, iw + 16.0, ROW, action);
            y += ROW;
            ui.rule(ix, y, iw, INNER);
        }
        y += 12.0;
    }
    let Some(t) = shown else {
        return y - y0;
    };
    let release = u.source.as_ref().map(|s| s.1.as_str());
    y += value_row(ui, ix, y, iw, app.medium.cap(), disk(t), TEXT);
    if let Some(data) = app.upgrade_data_of(t) {
        let n = data.files.len();
        let files = format!("{n} {}", if n == 1 { "file" } else { "files" });
        y += value_row(ui, ix, y, iw, "Its data partition", &files, TEXT);
    }
    y += value_row(ui, ix, y, iw, "On it", &version(t.release.as_deref()), TEXT);
    y += value_row(ui, ix, y, iw, "To be written", &version(release), TEXT);
    let fit = u.fit(t);
    let note = match &fit {
        Fit::Fits => None,
        Fit::Newer => Some(("Newer than this Faraday".to_string(), WARN)),
        Fit::Same => Some(("Already this version".to_string(), MUTED)),
        Fit::TooSmall(has, needs) => {
            Some((format!("Boot partition {has} MB · needs {needs} MB"), ERR))
        }
    };
    y += 8.0;
    if let Some((line, tone)) = note {
        y += ui.wrap(ix, y, iw, 13.0, W::S, tone, &line) + 8.0;
    }
    if let Some((why, pulled)) = &u.failed {
        let line = if *pulled {
            "Removed during the write: it does not start until upgraded again · its data \
             partition was not written"
                .to_string()
        } else {
            format!("Not written: {why}")
        };
        y += ui.wrap(ix, y, iw, 13.0, W::S, ERR, &line) + 8.0;
    }
    let blocked = matches!(fit, Fit::TooSmall(..) | Fit::Same);
    let (label, style) = if u.writing.is_some() {
        ("Writing", Style::Disabled)
    } else if blocked {
        ("Upgrade", Style::Disabled)
    } else {
        ("Upgrade", Style::Primary)
    };
    y += 4.0;
    ui.button(
        ix,
        y,
        Some(160.0f32.min(iw)),
        44.0,
        label,
        style,
        Action::UpgradeWrite,
    );
    y + 44.0 - y0
}

/// Step 3: written, and the stick out. Returns its height.
fn done(app: &Faraday, ui: &mut Ui, u: &UpgradeState, step: Step, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += heading(ui, x, y, w, 3, "Done", (step == Step::Done, false));
    let Some((id, release)) = &u.done else {
        return y - y0;
    };
    let ix = x + 38.0;
    let iw = w - 38.0;
    y += value_row(
        ui,
        ix,
        y,
        iw,
        "Written, read back and matched",
        &version(Some(release)),
        OK,
    );
    let still_in = u.parts.iter().any(|p| &p.id == id);
    if still_in {
        let line = format!("Remove the {}", app.medium.noun());
        y += value_row(ui, ix, y, iw, &line, "", MUTED);
    }
    y += 12.0;
    let again = format!("Upgrade another {}", app.medium.noun());
    let items = [
        (again.as_str(), Style::Secondary, Action::UpgradeAgain),
        ("Done", Style::Primary, Action::Nav(Screen::Settings)),
    ];
    if ui.compact {
        y += crate::compact::buttons(ui, ix, y, iw, &items);
    } else {
        let mut bx = ix;
        for (label, style, action) in items {
            bx += ui.button(bx, y, None, 44.0, label, style, action) + 10.0;
        }
        y += 44.0;
    }
    y - y0
}
