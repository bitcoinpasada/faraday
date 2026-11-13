//! A checked stick's sheet at Bring in the sticks (`docs/STICK-SETUP.md`
//! §5): what tells the stick apart, read when the stick was listed and
//! checked. Material 3 Bottom sheets on a small panel, a dialog on a wide
//! one ([`crate::compact::dialog`]); nothing in it is a press but
//! **Close**. Reads only.

use crate::backup_flow::BackupFlow;
use crate::backup_flow::figures::name_of;
use crate::drawing::View;
use crate::strings::{EN as FEN, fill};
use crate::ui::Style;
use crate::ui::Ui;
use crate::ui::pal::*;
use crate::{Action, Medium};
use faraday_draw::tokens::{self, BODY_M, BODY_S, GAP, GAP_SMALL};
use faraday_draw::widgets::Icon;

/// The volume serial as Windows shows it: two groups of four hex digits.
fn serial_text(n: u32) -> String {
    format!("{:04X}-{:04X}", n >> 16, n & 0xFFFF)
}

/// The sheet's rows, each a label and its lines, where the stick tells.
fn rows(f: &BackupFlow, k: usize) -> Vec<(&'static str, Vec<String>)> {
    let Some(b) = f.slots.get(k).and_then(|s| s.bound.as_ref()) else {
        return Vec::new();
    };
    let mut rows: Vec<(&'static str, Vec<String>)> = Vec::new();
    let mut add = |label: &'static str, lines: Vec<String>| {
        if !lines.is_empty() {
            rows.push((label, lines));
        }
    };
    add(
        FEN.backup_flow_sheet_name,
        Some(String::from(&*b.label))
            .filter(|l| !l.is_empty())
            .into_iter()
            .collect(),
    );
    add(
        FEN.backup_flow_sheet_make,
        b.make.who().into_iter().collect(),
    );
    add(
        FEN.backup_flow_sheet_size,
        b.make.size_text().into_iter().collect(),
    );
    add(
        FEN.backup_flow_sheet_serial,
        b.serial.map(serial_text).into_iter().collect(),
    );
    add(
        FEN.backup_flow_sheet_free,
        b.free
            .map(|n| String::from(&*crate::screens::size_text(n)))
            .into_iter()
            .collect(),
    );
    add(
        FEN.backup_flow_sheet_files,
        b.listing
            .iter()
            .map(|(n, size)| {
                fill(
                    FEN.backup_flow_figure_file,
                    &[n, &crate::screens::size_text(*size)],
                )
            })
            .collect(),
    );
    add(
        FEN.backup_flow_sheet_vaults,
        b.vaults.iter().map(|v| String::from(&*v.name)).collect(),
    );
    rows
}

/// The sheet of the stick whose figure was pressed.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(f) = app.flow() else {
        return;
    };
    let Some(k) = f.live.stick_sheet.filter(|&k| k < f.slots.len()) else {
        return;
    };
    let rows = rows(f, k);
    let icon = match app.medium {
        Medium::Stick => Icon::Drive,
        Medium::SdCard => Icon::SdCard,
    };
    let title = name_of(app, f, k);
    let actions = [(FEN.backup_flow_sheet_close, Style::Primary, Action::Cancel)];
    crate::compact::dialog(
        ui,
        (w, h, tokens::DIALOG_MAX),
        (icon.into(), Role::Primary.paint(), &title),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (label, lines) in &rows {
                cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurfaceVariant, label);
                for line in lines {
                    cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, line);
                }
                cy += GAP_SMALL;
            }
            cy - y + GAP - GAP_SMALL
        },
        &actions,
    );
}
