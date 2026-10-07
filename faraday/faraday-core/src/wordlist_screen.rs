//! The word-list sheet over [`crate::wordlist`]: a tab for each list,
//! Back, and every word of the list on show in rows of number, code and
//! word, in the mono face, the marked word lit.

use crate::Action;
use crate::Faraday;
use crate::screens::sheet_box;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wordlist::{LISTS, WordListAction as WL};

/// A row's height, in design units.
const ROW: f32 = 28.0;

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(st) = app.wordlist.as_mut() else {
        return;
    };
    let sw = (w - 80.0).min(860.0);
    let sh = h - 64.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    // A tab for each list, and Back.
    let mut tx = x + 28.0;
    for (i, l) in LISTS.iter().enumerate() {
        let style = if *l == st.list {
            Style::Primary
        } else {
            Style::Secondary
        };
        tx += ui.button(
            tx,
            y + 22.0,
            None,
            34.0,
            l.name(),
            style,
            Action::WordList(WL::List(i as u8)),
        ) + 6.0;
    }
    ui.button(
        x + sw - 28.0 - 96.0,
        y + 22.0,
        Some(96.0),
        34.0,
        "Back",
        Style::Secondary,
        Action::WordList(WL::Close),
    );
    let words = st.list.words();
    let (bx, bw) = (x + 32.0, sw - 64.0);
    // What the list is, and what is typed to find a word.
    let mut cy = y + 74.0;
    let about = match st.mark {
        Some(m) => format!(
            "{} words · word {} of {} marked",
            words.len(),
            m + 1,
            words.len()
        ),
        None => format!("{} words", words.len()),
    };
    ui.text(bx, cy, 13.0, W::R, MUTED, &about);
    let find = if st.find.is_empty() {
        "Type to find a word".to_string()
    } else {
        format!("Find: {}", st.find)
    };
    ui.text_right(bx + bw, cy - 6.0, 28.0, 13.0, W::M, DIM, &find);
    cy += 28.0;
    // The columns: number, code, word.
    let num_w = ui.measure(14.0, W::M, "0000");
    let code_w = ui.measure(14.0, W::M, &st.list.code(0));
    let (nx, cx, wx) = (bx + 12.0, bx + 24.0 + num_w, bx + 48.0 + num_w + code_w);
    ui.text(nx, cy, 12.0, W::S, MUTED, "No.");
    ui.text(cx, cy, 12.0, W::S, MUTED, st.list.code_heading());
    ui.text(wx, cy, 12.0, W::S, MUTED, "Word");
    cy += 22.0;
    ui.rule(bx, cy - 4.0, bw, INNER);
    let top = cy;
    let view = (y + sh - 20.0 - top).max(ROW);
    let content = words.len() as f32 * ROW + 12.0;
    st.max = (content - view).max(0.0);
    if st.follow {
        st.follow = false;
        if let Some(m) = st.mark {
            st.scroll = m as f32 * ROW - (view - ROW) / 2.0;
        }
    }
    st.scroll = st.scroll.clamp(0.0, st.max);
    let clip = ui.rect(x, top, sw, view);
    ui.c.push_clip(clip);
    let first = (st.scroll / ROW).floor() as usize;
    let last = (((st.scroll + view) / ROW).ceil() as usize).min(words.len());
    for (i, word) in words.iter().enumerate().take(last).skip(first) {
        let ry = top + 4.0 + i as f32 * ROW - st.scroll;
        let marked = st.mark == Some(i);
        if marked {
            ui.fill(bx, ry, bw, ROW - 2.0, 6.0, ACCENT.with_alpha(40));
            ui.stroke(bx, ry, bw, ROW - 2.0, 6.0, ACCENT);
        }
        let number = (i + 1).to_string();
        let nw = ui.measure(14.0, W::M, &number);
        let tone = if marked { TEXT } else { MUTED };
        ui.text_mid(nx + num_w - nw, ry, ROW - 2.0, 14.0, W::M, tone, &number);
        ui.text_mid(cx, ry, ROW - 2.0, 14.0, W::M, tone, &st.list.code(i));
        ui.text_mid(
            wx,
            ry,
            ROW - 2.0,
            14.0,
            W::M,
            if marked { ACCENT } else { TEXT },
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
    ui.report_scroll_own_bar(clip, st.max);
    if st.max > 0.0 {
        let track = view - 8.0;
        let thumb = (track * view / content).max(30.0);
        let at = (track - thumb) * st.scroll / st.max;
        ui.fill(x + sw - 14.0, top + 4.0 + at, 4.0, thumb, 2.0, BORDER);
    }
}
