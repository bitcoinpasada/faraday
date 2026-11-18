//! `Screen::Catalog`: Tools, every flow on one page; Find also reaches a tool by its standards.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::Shape;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_S, TITLE_S};
use faraday_draw::tokens::{
    CATALOG_COLUMNS_MIN, CATALOG_ROW_GAP, CATALOG_TILE_H as TILE_H,
    CATALOG_TILE_H_PLAIN as TILE_H_PLAIN, CATALOG_TILE_MIN, PAGE_INSET,
};
use faraday_ui::parts::section_label;

pub(crate) use super::settings::rows::{ROWS, Row};

/// Tools: every flow by group, as many tiles to a row as fit and at
/// least three, in a page at most `TRANSFER_W` wide; Find a tool narrows them as it is typed.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::catalog::{GROUPS, TILES, matches};
    // A small panel has the name in its bar, Find across the panel, and a
    // tile a row.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + PAGE_INSET,
            (cw - 2.0 * PAGE_INSET).min(faraday_draw::tokens::TRANSFER_W),
        )
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if compact {
        14.0
    } else {
        super::bar_head(ui, None, EN.catalog_tools)
    };
    let mut y = top;
    // Find a tool: typing on this page goes here.
    let fw = if compact {
        width
    } else {
        320.0f32.min(width * 0.4)
    };
    let fx = x + width - fw;
    let here = Action::TypeHere(crate::edit::Here::Find);
    search_field(
        ui,
        (fx, y - 2.0, fw),
        (app.tools.find(), EN.catalog_find_a_tool),
        here,
    );
    y += if compact { 54.0 } else { 64.0 };
    let g = grid(compact, width);
    let tw3 = g.tile_w;
    let mut any = false;
    for group in GROUPS {
        let tiles: Vec<(usize, &crate::catalog::Tile)> = TILES
            .iter()
            .enumerate()
            .filter(|(_, t)| t.group == group && matches(t, app.tools.find()))
            .collect();
        if tiles.is_empty() {
            continue;
        }
        any = true;
        let faces: Vec<Face> = tiles
            .iter()
            .map(|(i, tile)| {
                let need = app.tile_need(tile.go);
                Face {
                    name: tile.name,
                    line: need.unwrap_or(tile.line),
                    warn: need.is_some(),
                    tags: &[],
                    opens: app.tile_opens(tile.go),
                    action: Action::Catalog(*i as u8),
                }
            })
            .collect();
        y += section(ui, (x, y), &g, group, &faces);
    }
    // The rows of Settings the same words find, under Settings' name.
    let found = crate::tools::search::hits(app.tools.find());
    let settings: Vec<u8> = found
        .iter()
        .filter_map(|h| match h {
            crate::tools::search::Hit::Setting(i) => Some(*i),
            _ => None,
        })
        .collect();
    if !settings.is_empty() {
        any = true;
        section_label(ui, x, y, EN.settings_settings);
        y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
        for i in settings {
            y += setting_hit(ui, (x, y, tw3), i);
        }
    }
    // The guide's words the same words find, each opening its definition.
    y += word_hits(ui, (x, y, tw3), &found);
    any |= found
        .iter()
        .any(|h| matches!(h, crate::tools::search::Hit::Word(_)));
    if !any {
        ui.text_as(
            x,
            y,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.catalog_no_tool_matches,
        );
        y += ui.advance_as(BODY_M, 30.0);
    }
    app.scroll.content_h().set(y - top + 40.0);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// A setting found by [`crate::tools::search::hits`], drawn from (x, y),
/// `w` wide, as Material's list item: its label and the section it is in,
/// a press of which opens Settings at it. Returns its height. Tools' Find
/// and Home's search both draw it.
pub(crate) fn setting_hit(ui: &mut Ui, at: (f32, f32, f32), i: u8) -> f32 {
    let Some(row) = crate::tools::search::row(i) else {
        return 0.0;
    };
    let item = crate::panes::screen::Item {
        headline: row.label,
        supporting: row.section(),
        trailing: None,
        action: Action::SettingAt(i),
    };
    crate::panes::screen::list_item(ui, at, &item, false)
}

/// The guide's words among `found`, from (x, y), `w` wide, under their
/// heading, each a Material list item: the word, a press of which opens
/// the guide at its definition. Returns the height used, none when no
/// word was found. Tools' Find and Home's search both draw it.
pub(crate) fn word_hits(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    found: &[crate::tools::search::Hit],
) -> f32 {
    let glossary = faraday_guide::glossary();
    let mut cy = y;
    for hit in found {
        let crate::tools::search::Hit::Word(i) = *hit else {
            continue;
        };
        let Some(word) = glossary.get(usize::from(i)) else {
            continue;
        };
        if cy == y {
            section_label(ui, x, cy, EN.guide_words_found);
            cy += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
        }
        let item = crate::panes::screen::Item {
            headline: word.word,
            supporting: "",
            trailing: None,
            action: Action::GuideWord(i),
        };
        cy += crate::panes::screen::list_item(ui, (x, cy, w), &item, false);
    }
    cy - y
}

/// Home's search bar at (`x`, `y`), `w` wide: Material's search bar, as
/// the vault's is drawn (a full-round `SurfaceContainerHigh` field, the
/// magnifying glass leading, the hint until something is typed, a
/// trailing × that clears while it holds text). Typing lands in it with
/// no press first, so at rest it draws no caret. Returns the height it
/// takes, the gap under it counted.
pub(crate) fn home_search_bar(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    use faraday_draw::tokens::{
        BODY_L, GAP, GAP_SMALL, SEARCH_BAR_H, SEARCH_GLYPH, SEARCH_ICON, SEARCH_INSET, TITLE_M,
        TOUCH,
    };
    let typed = app.tools.home_find();
    let here = Action::TypeHere(crate::edit::Here::HomeFind);
    let h = SEARCH_BAR_H;
    ui.fill(x, y, w, h, h / 2.0, Role::SurfaceContainerHigh);
    ui.icon(
        x + SEARCH_INSET,
        y + (h - SEARCH_ICON) / 2.0,
        SEARCH_ICON,
        faraday_draw::widgets::Icon::SEARCH,
        SEARCH_GLYPH,
        Role::OnSurfaceVariant,
    );
    ui.press_box(x, y, w, h, here);
    let tx = x + 2.0 * SEARCH_INSET + SEARCH_ICON;
    let clear = if typed.is_empty() { 0.0 } else { TOUCH };
    let room = (x + w - tx - SEARCH_INSET - clear).max(0.0);
    if typed.is_empty() {
        let shown = ui.fit_as(BODY_L, EN.home_search_hint, room);
        ui.text_mid_as((tx, y, h), BODY_L, Role::OnSurfaceVariant, &shown);
    } else {
        ui.typed(
            tx,
            y,
            h,
            BODY_L.size,
            crate::ui::W::from(BODY_L.face),
            Role::OnSurface,
            typed,
            room,
            true,
            here,
        );
        let cx = x + w - GAP_SMALL - TOUCH;
        let cy = y + (h - TOUCH) / 2.0;
        ui.text_mid_as(
            (cx + (TOUCH - ui.measure_as(TITLE_M, "×")) / 2.0, cy, TOUCH),
            TITLE_M,
            Role::OnSurfaceVariant,
            "×",
        );
        ui.hit_tip(
            (cx, cy, TOUCH, TOUCH),
            Action::HomeSearchClear,
            EN.vault_search_clear,
        );
    }
    h + GAP
}

/// What Home shows in place of itself while something is typed in its
/// search bar: the matching tiles as buttons, then the rows of Settings, from (`x`, `y`), `width`
/// wide, each opening as the Tools tile does, or a line saying none
/// matches. Returns the height it takes.
pub(crate) fn home_results(
    app: &View,
    ui: &mut Ui,
    (x, y): (f32, f32),
    width: f32,
    compact: bool,
) -> f32 {
    use crate::catalog::TILES;
    use crate::tools::search::{Hit, hits};
    let found = hits(app.tools.home_find());
    if found.is_empty() {
        ui.text_as(
            x,
            y,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.catalog_no_tool_matches,
        );
        return ui.advance_as(BODY_M, faraday_draw::tokens::HOME_RESULTS_LINE);
    }
    let g = grid(compact, width);
    let faces: Vec<Face> = found
        .iter()
        .filter_map(|hit| match *hit {
            Hit::Tile(i) => Some((i, &TILES[i])),
            Hit::Setting(_) => None,
        })
        .map(|(i, tile)| {
            let need = app.tile_need(tile.go);
            Face {
                name: tile.name,
                line: need.unwrap_or(tile.line),
                warn: need.is_some(),
                tags: tile.tags,
                opens: app.tile_opens(tile.go),
                action: Action::Catalog(i as u8),
            }
        })
        .collect();
    let mut used = g.draw_rows(ui, (x, y), &faces);
    for hit in &found {
        if let Hit::Setting(i) = *hit {
            used += setting_hit(ui, (x, y + used, width), i);
        }
    }
    used + word_hits(ui, (x, y + used, width), &found)
}

/// A search field at (x, y), `w` wide: Material's search field, in
/// Tools' look, with `hint` until something is typed. Typing on its page
/// always lands here, with no click to focus it first, so the caret is
/// drawn whether or not anything is typed yet: the one sign that this
/// box is already live. A press places it (`docs/TEXT-CURSOR.md` §3).
/// Tools' Find a tool and the guide's Find in the guide both draw it.
pub(crate) fn search_field(ui: &mut Ui, at: (f32, f32, f32), text: (&str, &str), here: Action) {
    search_field_caret(ui, at, text, here, true);
}

/// [`search_field`], with the caret at rest drawn only where `rest` is
/// true: Home's bar draws none until something is typed, so Home at rest
/// asks for no frame (the caret's blink would).
fn search_field_caret(
    ui: &mut Ui,
    (fx, fy, fw): (f32, f32, f32),
    (typed, hint): (&str, &str),
    here: Action,
    rest: bool,
) {
    ui.fill(fx, fy, fw, 40.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(fx, fy, fw, 40.0, 8.0, Role::Outline);
    let (tx, fh, room) = (
        fx + faraday_draw::tokens::TYPED_INSET,
        faraday_draw::tokens::TEXT_FIELD,
        fw - 28.0,
    );
    ui.press_box(fx, fy, fw, fh, here);
    if typed.is_empty() {
        let shown = ui.fit_as(BODY_M, hint, room);
        ui.text_mid_as((tx, fy, fh), BODY_M, Role::OnSurfaceVariant, &shown);
        if rest {
            ui.caret(
                tx,
                fy + (fh - faraday_draw::tokens::CARET_MIN) / 2.0,
                faraday_draw::tokens::CARET_MIN,
            );
        }
    } else {
        ui.typed(
            tx,
            fy,
            fh,
            BODY_M.size,
            crate::ui::W::R,
            Role::OnSurface,
            typed,
            room,
            true,
            here,
        );
    }
}

/// The grid tiles stand in: as many columns as `CATALOG_TILE_MIN` leaves
/// room for, never fewer than three beside the navigation (`docs/LAYOUT.md`
/// §17), one on a small panel. Tools and Check both use it.
pub(crate) struct Grid {
    /// Tiles to a row.
    pub per: usize,
    /// A tile's width.
    pub tile_w: f32,
}

const TILE_GAP: f32 = 14.0;

/// The grid for a page `width` wide.
pub(crate) fn grid(compact: bool, width: f32) -> Grid {
    let per = if compact {
        1
    } else {
        (((width + TILE_GAP) / (CATALOG_TILE_MIN + TILE_GAP)) as usize).max(CATALOG_COLUMNS_MIN)
    };
    Grid {
        per,
        tile_w: (width - (per as f32 - 1.0) * TILE_GAP) / per as f32,
    }
}

impl Grid {
    /// The space between tiles.
    pub fn gap(&self) -> f32 {
        TILE_GAP
    }

    /// Draws `faces` in rows from `at`, a row as tall as the tallest tile
    /// in it and never under `CATALOG_TILE_H`, and returns their height.
    pub fn draw_rows(&self, ui: &mut Ui, (x, y): (f32, f32), faces: &[Face]) -> f32 {
        let mut top = y;
        for row in faces.chunks(self.per) {
            let rh = row
                .iter()
                .map(|f| f.height(ui, self.tile_w))
                .fold(TILE_H, f32::max);
            for (c, face) in row.iter().enumerate() {
                face.draw(
                    ui,
                    x + c as f32 * (self.tile_w + TILE_GAP),
                    top,
                    self.tile_w,
                );
            }
            top += rh + CATALOG_ROW_GAP;
        }
        top - y
    }
}

/// A group of tiles under its name from `at`, in `g`: the layout Tools and
/// Check share. Returns the height up to where the next group starts.
pub(crate) fn section(
    ui: &mut Ui,
    (x, y): (f32, f32),
    g: &Grid,
    name: &str,
    faces: &[Face],
) -> f32 {
    section_label(ui, x, y, name);
    let head = ui.line_as(TITLE_S) + faraday_draw::tokens::GRID;
    head + g.draw_rows(ui, (x, y + head), faces) + faraday_draw::tokens::GRID * 2.0
}

/// What a tile shows.
pub(crate) struct Face<'a> {
    /// Its name.
    pub name: &'a str,
    /// The line under it.
    pub line: &'a str,
    /// The line is a warning (what the tile needs first).
    pub warn: bool,
    /// Small tags at its foot.
    pub tags: &'a [&'a str],
    /// Whether a press acts.
    pub opens: bool,
    /// What a press does.
    pub action: Action,
}

impl Face<'_> {
    /// What the name and the line under it take over one line each at
    /// `tw3` wide: they wrap rather than end in an ellipsis.
    fn grown(&self, ui: &Ui, tw3: f32) -> (f32, f32) {
        let room = tw3 - 32.0;
        let over = |role, s| (ui.wrap_height_as(role, s, room) - ui.line_as(role)).max(0.0);
        (over(TITLE_S, self.name), over(BODY_S, self.line))
    }

    /// The tile's height at `tw3` wide: the tags' row only where it has
    /// tags, and a line more for each that wraps.
    pub fn height(&self, ui: &Ui, tw3: f32) -> f32 {
        let (n, l) = self.grown(ui, tw3);
        n + l + self.base_height()
    }

    fn base_height(&self) -> f32 {
        if self.tags.is_empty() {
            TILE_H_PLAIN
        } else {
            TILE_H
        }
    }

    /// Draws the tile at (`tx`, `ty`), `tw3` wide.
    pub fn draw(&self, ui: &mut Ui, tx: f32, ty: f32, tw3: f32) {
        let (grow_n, grow_l) = self.grown(ui, tw3);
        let th = self.height(ui, tw3);
        let r = faraday_draw::tokens::SHAPE_M;
        ui.fill(tx, ty, tw3, th, r, Role::SurfaceContainerLow);
        ui.stroke(tx, ty, tw3, th, r, Role::OutlineVariant);
        if self.opens {
            ui.press(
                Shape::new(tx, ty, tw3, th, r),
                Role::OnSurface.paint(),
                self.action,
            );
        }
        let ink = if self.opens {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        ui.wrap_as((tx + 16.0, ty + 12.0, tw3 - 32.0), TITLE_S, ink, self.name);
        let warn = if self.warn {
            Role::Warning
        } else {
            Role::OnSurfaceVariant
        };
        let ly = ty + 34.0 + grow_n;
        ui.wrap_as((tx + 16.0, ly, tw3 - 32.0), BODY_S, warn, self.line);
        let tag_y = ty + 56.0 + grow_n + grow_l;
        // The standards, as small tags.
        let mut gx = tx + 16.0;
        for tag in self.tags {
            let gw = ui.measure_as(LABEL_S.mono(), tag) + 14.0;
            if gx + gw > tx + tw3 - 12.0 {
                break;
            }
            ui.fill(
                gx,
                tag_y,
                gw,
                18.0,
                faraday_draw::tokens::shape_full(18.0),
                Role::SurfaceContainerHighest,
            );
            ui.text_mid_as(
                (gx + 7.0, tag_y, 18.0),
                LABEL_S.mono(),
                Role::OnSurfaceVariant,
                tag,
            );
            gx += gw + 6.0;
        }
    }
}
