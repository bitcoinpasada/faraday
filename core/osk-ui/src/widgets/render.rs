//! Walks a tree and its layout together and draws every widget.

use crate::canvas::Canvas;
use crate::color::Theme;
use crate::geom::Rect;
use crate::layout::{Kind, Layout, Node};
use crate::state::UiState;

/// Draws `root` using `layout` (which must have been solved from `root`).
/// The caller clears the canvas first.
pub fn draw_tree(c: &mut Canvas, root: &Node, layout: &Layout, theme: &Theme, state: &UiState) {
    let damage = c.bounds();
    draw_tree_within(c, root, layout, theme, state, damage);
}

/// Draws the part of `root` that falls inside `damage`, clipped to it:
/// what a caller that has kept the rest of the frame — the pixels of a
/// scroll region moved in place — needs drawn again. The caller fills
/// `damage` with the background first, as [`draw_tree`]'s caller clears
/// the canvas.
pub fn draw_tree_within(
    c: &mut Canvas,
    root: &Node,
    layout: &Layout,
    theme: &Theme,
    state: &UiState,
    damage: Rect,
) {
    let mut index = 0;
    walk(c, root, layout, theme, state, damage, &mut index);
}

#[allow(clippy::too_many_arguments)]
fn walk(
    c: &mut Canvas,
    node: &Node,
    layout: &Layout,
    theme: &Theme,
    state: &UiState,
    damage: Rect,
    index: &mut usize,
) {
    let Some(placed) = layout.items.get(*index).copied() else {
        return;
    };
    *index += 1;
    if let Kind::Widget(w) = &node.kind {
        let clip = placed.clip.intersect(&damage);
        if !clip.intersect(&placed.rect).is_empty() {
            c.push_clip(clip);
            w.draw(c, placed.rect, &layout.ctx, theme, state);
            c.pop_clip();
        }
    }
    match node.kind {
        Kind::Scroll => {
            if let Some(child) = node.children.first() {
                walk(c, child, layout, theme, state, damage, index);
            }
            // The scrollbar is drawn over the content, and only while
            // there is content the viewport cannot show (UX.md §6).
            if let Some(info) = placed.scroll
                && let Some(thumb) = crate::widgets::scrollbar_thumb(
                    placed.rect,
                    info.content_h,
                    info.offset,
                    &layout.ctx,
                )
                && !thumb.intersect(&damage).is_empty()
            {
                c.push_clip(placed.clip.intersect(&damage));
                c.fill_rounded_rect(thumb, thumb.w as f32 / 2.0, theme.muted);
                c.pop_clip();
            }
        }
        _ => {
            for child in &node.children {
                walk(c, child, layout, theme, state, damage, index);
            }
        }
    }
}
