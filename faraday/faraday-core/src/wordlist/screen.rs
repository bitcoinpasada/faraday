//! The word-list sheet over [`crate::wordlist`]: a tab for each list,
//! Back, the search field, and the words of the list on show that the
//! search finds (every word while it is empty) in rows of number, code
//! and word, in the mono face, the marked word lit. A press beside the
//! sheet closes it, as Back does.

use crate::Action;
use crate::drawing::View;
use crate::screens::sheet_box;
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wordlist::{LISTS, WordListAction as WL};
use faraday_draw::tokens as T;

/// A row's height, in design units.
const ROW: f32 = 28.0;

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(st) = app.wordlist.state() else {
        return;
    };
    let compact = ui.compact;
    let (sw, sh, pad) = if compact {
        (w - 16.0, h - 16.0, 14.0)
    } else {
        ((w - 80.0).min(860.0), h - 64.0, 28.0)
    };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    ui.hit_around(x, y, sw, sh, Action::WordList(WL::Outside));
    // A tab for each list, and Back; on a small panel the tabs wrap.
    let back_w = if compact { 64.0 } else { 96.0 };
    ui.button(
        x + sw - pad - back_w,
        y + 14.0,
        Some(back_w),
        34.0,
        EN.wordlist_back,
        Style::Secondary,
        Action::WordList(WL::Close),
    );
    let tabs: Vec<(String, Style, Action)> = LISTS
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let style = if *l == st.list {
                Style::Primary
            } else {
                Style::Secondary
            };
            (
                l.name().to_string(),
                style,
                Action::WordList(WL::List(i as u8)),
            )
        })
        .collect();
    let tabs_h =
        crate::screens::button_rows(ui, x + pad, y + 14.0, sw - 2.0 * pad - back_w - 8.0, &tabs)
            - 44.0
            + 34.0;
    let words = st.list.words();
    let (bx, bw) = (x + pad + 4.0, sw - 2.0 * pad - 8.0);
    // The search field at the row's start, and what the list holds or
    // what the search found at its end.
    let mut cy = y + 14.0 + tabs_h + 18.0;
    let about = match st.mark {
        _ if !st.find.is_empty() => fill(EN.wordlist_found, &[&st.found.len().to_string()]),
        Some(m) => fill(
            EN.wordlist_words_word,
            &[
                &words.len().to_string(),
                &(m + 1).to_string(),
                &words.len().to_string(),
            ],
        ),
        None => fill(EN.wordlist_words, &[&words.len().to_string()]),
    };
    // No keyboard on a small panel: no field to type in.
    let row = if !compact || !st.find.is_empty() {
        let fw = search_field(ui, (bx, cy, bw), st);
        let room = bw - fw - T::GAP;
        let about = ui.fit(T::LABEL, W::R, &about, room);
        let ah = T::TEXT_FIELD;
        ui.text_right(
            bx + bw,
            cy,
            ah,
            T::LABEL,
            W::R,
            Role::OnSurfaceVariant,
            &about,
        );
        T::TEXT_FIELD + T::GAP
    } else {
        ui.text(bx, cy, T::LABEL, W::R, Role::OnSurfaceVariant, &about);
        T::LABEL + T::PAD
    };
    cy += row;
    // The columns: number, code, word.
    let num_w = ui.measure(14.0, W::M, "0000");
    let code_w = ui.measure(14.0, W::M, &st.list.code(0));
    let (nx, cx, wx) = (bx + 12.0, bx + 24.0 + num_w, bx + 48.0 + num_w + code_w);
    ui.text(
        nx,
        cy,
        12.0,
        W::S,
        Role::OnSurfaceVariant,
        EN.wordlist_number_column,
    );
    ui.text(
        cx,
        cy,
        12.0,
        W::S,
        Role::OnSurfaceVariant,
        st.list.code_heading(),
    );
    ui.text(wx, cy, 12.0, W::S, Role::OnSurfaceVariant, EN.wordlist_word);
    cy += 22.0;
    ui.rule(bx, cy - 4.0, bw, Role::OutlineVariant);
    let top = cy;
    let view = (y + sh - 20.0 - top).max(ROW);
    let found = &st.found;
    let content = found.len() as f32 * ROW + 12.0;
    let max = (content - view).max(0.0);
    let mut scroll = st.scroll;
    if st.follow
        && let Some(m) = st.mark
        && let Some(at) = found.iter().position(|&i| usize::from(i) == m)
    {
        scroll = at as f32 * ROW - (view - ROW) / 2.0;
    }
    let scroll = scroll.clamp(0.0, max);
    app.wordlist_scroll(max, scroll);
    let clip = ui.rect(x, top, sw, view);
    ui.c.push_clip(clip);
    let first = (scroll / ROW).floor() as usize;
    let last = (((scroll + view) / ROW).ceil() as usize).min(found.len());
    for (r, &i) in found.iter().enumerate().take(last).skip(first) {
        let (i, word) = (
            usize::from(i),
            words.get(usize::from(i)).copied().unwrap_or(""),
        );
        let ry = top + 4.0 + r as f32 * ROW - scroll;
        let marked = st.mark == Some(i);
        if marked {
            ui.fill(bx, ry, bw, ROW - 2.0, 6.0, Role::SecondaryContainer);
        }
        let number = (i + 1).to_string();
        let nw = ui.measure(14.0, W::M, &number);
        let tone = crate::ui::pal::on_selected(marked, Role::OnSurfaceVariant);
        ui.text_mid(nx + num_w - nw, ry, ROW - 2.0, 14.0, W::M, tone, &number);
        ui.text_mid(cx, ry, ROW - 2.0, 14.0, W::M, tone, &st.list.code(i));
        ui.text_mid(
            wx,
            ry,
            ROW - 2.0,
            14.0,
            W::M,
            crate::ui::pal::on_selected(marked, Role::OnSurface),
            word,
        );
        // A press marks the word; only the part of the row in view takes
        // it.
        let (hy, hb) = (ry.max(top), (ry + ROW - 2.0).min(top + view));
        if hb > hy {
            ui.hit(bx, hy, bw, hb - hy, Action::WordList(WL::Mark(i as u16)));
        }
    }
    ui.c.pop_clip();
    // The overlay scrollbar, held and dragged as on any page.
    ui.report_scroll(clip, max);
}

/// The search field at (x, y), at most half the row `w` wide: Material's
/// outlined text field, edged in the accent since every key typed on the
/// open sheet goes into it, with the list's placeholder until something
/// is typed, then the search and the caret. Returns its width.
fn search_field(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    st: &crate::wordlist::WordListState,
) -> f32 {
    let fw = (w / 2.0).min(T::SEARCH_FIELD_MAX);
    let h = T::TEXT_FIELD;
    ui.fill(x, y, fw, h, T::FIELD_RADIUS, Role::SurfaceContainerHighest);
    ui.stroke(x, y, fw, h, T::FIELD_RADIUS, Role::Primary);
    let inner = fw - 2.0 * T::PAD;
    if st.find.is_empty() {
        let hint = ui.fit(T::LABEL, W::R, st.list.search_hint(), inner);
        ui.text_mid(
            x + T::PAD,
            y,
            h,
            T::LABEL,
            W::R,
            Role::OnSurfaceVariant,
            &hint,
        );
    } else {
        let shown = format!("{}{}", st.find, ui.caret_char());
        let shown = ui.fit(T::LABEL, W::M, &shown, inner);
        ui.text_mid(x + T::PAD, y, h, T::LABEL, W::M, Role::OnSurface, &shown);
    }
    fw
}
