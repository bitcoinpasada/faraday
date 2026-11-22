//! `Screen::Catalog`: Tools, every flow on one page; Find also reaches a tool by its standards.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Glyph;
use crate::ui::Shape;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{
    BODY_M, BODY_S, FACE_ABOVE_GAP, FACE_ABOVE_GLYPH, FACE_ABOVE_ICON, FACE_INSET, FACE_LINE_GAP,
    FACE_LINE_Y, FACE_TAGS_Y, FACE_TOP, HOME_TILE_GLYPH, HOME_TILE_ICON, HOME_TILE_INSET,
    HOME_TILE_TEXT_X, LABEL_S, TITLE_S,
};

use faraday_draw::tokens::{CATALOG_TILE_H as TILE_H, CATALOG_TILE_H_PLAIN as TILE_H_PLAIN};
use faraday_ui::parts::section_label;

pub(crate) use super::settings::{card, cards_matching};

/// Tools: every flow by group on the menu page container, with the
/// search bar standing over them; Find a tool narrows them as it is typed.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use super::menu_page::{self, Page, Section};
    let sections: Vec<Section> = tiles_by_group(app)
        .into_iter()
        .map(|(name, faces)| Section::tiles(Some(name), faces))
        .collect();
    let page = Page {
        name: EN.catalog_tools,
        back: false,
        search: Some(Bar::Tools),
        most_columns: usize::MAX,
        fixed_columns: None,
        above: None,
        bar_still: false,
        ways: false,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}

/// Every tile of Tools, by group in order.
fn tiles_by_group(app: &View) -> Vec<(&'static str, Vec<Face<'static>>)> {
    use crate::catalog::{GROUPS, TILES};
    GROUPS
        .iter()
        .map(|&group| {
            let faces = TILES
                .iter()
                .enumerate()
                .filter(|(_, t)| t.group == group)
                .map(|(i, tile)| {
                    let need = app.tile_need(tile.go);
                    Face {
                        mark: Mark::None,
                        name: tile.name,
                        line: need.unwrap_or(tile.line),
                        warn: need.is_some(),
                        tags: &[],
                        opens: app.tile_opens(tile.go),
                        action: Action::Catalog(i as u8),
                    }
                })
                .collect();
            (group, faces)
        })
        .collect()
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

/// Which page a search bar and its results belong to: each keeps its own
/// typed text.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bar {
    Home,
    Tools,
}

/// The search bar of Home and of Tools at (`x`, `y`), `w` wide: Material's search bar, as
/// the vault's is drawn (a full-round `SurfaceContainerHigh` field, the
/// magnifying glass leading, the hint until something is typed, a
/// trailing × that clears while it holds text). Typing lands in it with
/// no press first, so at rest it draws no caret; a press shows one,
/// blinking, until something is typed. `page_top` is the top of the page
/// the bar moves with, if it does, as on a Home taller than its window (`home_pinned`): its
/// place is then kept, so the page follows the results no further than
/// the bar at its top; a bar that stays has none. Returns the height it
/// takes, the gap under it counted.
pub(crate) fn home_search_bar(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    page_top: Option<f32>,
    on: Bar,
) -> f32 {
    use faraday_draw::tokens::{
        BODY_L, GAP, GAP_SMALL, SEARCH_BAR_H, SEARCH_GLYPH, SEARCH_ICON, SEARCH_INSET, TITLE_M,
        TOUCH,
    };
    let (typed, here, focused) = match on {
        Bar::Home => (
            app.tools.home_find(),
            Action::TypeHere(crate::edit::Here::HomeFind),
            app.tools.home_focused(),
        ),
        Bar::Tools => (
            app.tools.find(),
            Action::TypeHere(crate::edit::Here::Find),
            app.tools.catalog_focused(),
        ),
    };
    let h = SEARCH_BAR_H;
    if on == Bar::Home {
        app.tools.note_home_bar(page_top.map_or(0.0, |top| y - top));
    }
    ui.fill(x, y, w, h, h / 2.0, Role::SurfaceContainerHigh);
    // The state layer takes the pill's own shape, not the default corner.
    ui.press(
        crate::ui::Shape::new(x, y, w, h, h / 2.0),
        Role::OnSurface.paint(),
        here,
    );
    ui.icon(
        x + SEARCH_INSET,
        y + (h - SEARCH_ICON) / 2.0,
        SEARCH_ICON,
        faraday_draw::widgets::Icon::SEARCH,
        SEARCH_GLYPH,
        Role::OnSurfaceVariant,
    );
    let tx = x + 2.0 * SEARCH_INSET + SEARCH_ICON;
    let clear = if typed.is_empty() { 0.0 } else { TOUCH };
    let room = (x + w - tx - SEARCH_INSET - clear).max(0.0);
    if typed.is_empty() {
        let shown = ui.fit_as(BODY_L, EN.home_search_hint, room);
        ui.text_mid_as((tx, y, h), BODY_L, Role::OnSurfaceVariant, &shown);
        if focused {
            let ch = faraday_draw::tokens::CARET_MIN;
            ui.caret(tx, y + (h - ch) / 2.0, ch);
        }
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

/// What Home, or Tools, shows in place of itself while something is typed in its
/// search bar: the matching tiles as buttons, then the cards of Settings, drawn as on Settings, from (`x`, `y`), `width`
/// wide, each opening as the Tools tile does, or a line saying none
/// matches. Returns the height it takes.
pub(crate) fn home_results(
    app: &View,
    ui: &mut Ui,
    (x, y): (f32, f32),
    width: f32,
    on: Bar,
) -> f32 {
    use crate::catalog::TILES;
    use crate::tools::search::{Hit, hits};
    let found = hits(match on {
        Bar::Home => app.tools.home_find(),
        Bar::Tools => app.tools.find(),
    });
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
    let g = super::menu_page::grid(width, usize::MAX);
    let faces: Vec<Face> = found
        .iter()
        .filter_map(|hit| match *hit {
            Hit::Tile(i) => Some((i, &TILES[i])),
            Hit::Card(_) | Hit::Word(_) => None,
        })
        .map(|(i, tile)| {
            let need = app.tile_need(tile.go);
            Face {
                mark: Mark::None,
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
    if !faces.is_empty() {
        used += faraday_draw::tokens::MENU_GAP;
    }
    for hit in &found {
        if let Hit::Card(i) = *hit {
            used += card(app, ui, i, (x, y + used, width));
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

/// The icon a tile has, if it has one.
#[derive(Clone, Copy)]
pub(crate) enum Mark {
    /// None: the text starts at the tile's edge (Tools, Check).
    None,
    /// An icon in a container before the text, centred on the tile's
    /// height (Home's wallets, keys and places).
    Beside(Glyph),
    /// An icon in a container over the text, for a tile too narrow to
    /// have the text beside it (the Pi's panel Home).
    Above(Glyph),
}

/// What a tile shows.
pub(crate) struct Face<'a> {
    /// Its icon.
    pub mark: Mark,
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
    /// Where the text starts from the tile's left edge.
    fn text_x(&self) -> f32 {
        match self.mark {
            Mark::Beside(_) => HOME_TILE_TEXT_X,
            Mark::None | Mark::Above(_) => FACE_INSET,
        }
    }

    /// Where the name starts from the tile's top: under the icon where it
    /// stands over the text.
    fn name_y(&self) -> f32 {
        match self.mark {
            Mark::Above(_) => FACE_TOP + FACE_ABOVE_ICON + FACE_ABOVE_GAP,
            Mark::None | Mark::Beside(_) => FACE_TOP,
        }
    }

    /// What the name and the line under it take over one line each at
    /// `tw3` wide: they wrap rather than end in an ellipsis.
    fn grown(&self, ui: &Ui, tw3: f32) -> (f32, f32) {
        let room = tw3 - self.text_x() - FACE_INSET;
        let over = |role, s| (ui.wrap_height_as(role, s, room) - ui.line_as(role)).max(0.0);
        (over(TITLE_S, self.name), over(BODY_S, self.line))
    }

    /// The tile's height at `tw3` wide: the tags' row only where it has
    /// tags, and a line more for each that wraps.
    pub fn height(&self, ui: &Ui, tw3: f32) -> f32 {
        let (n, l) = self.grown(ui, tw3);
        if let Mark::Above(_) = self.mark {
            let line = if self.line.is_empty() {
                0.0
            } else {
                FACE_LINE_GAP + ui.line_as(BODY_S) + l
            };
            return self.name_y() + ui.line_as(TITLE_S) + n + line + FACE_TOP;
        }
        n + l + self.base_height()
    }

    fn base_height(&self) -> f32 {
        if self.tags.is_empty() {
            TILE_H_PLAIN
        } else {
            TILE_H
        }
    }

    /// The icon in its container at `(x, y)`, `size` wide.
    fn draw_mark(&self, ui: &mut Ui, (x, y): (f32, f32), (size, glyph_size): (f32, f32)) {
        let (Mark::Beside(glyph) | Mark::Above(glyph)) = self.mark else {
            return;
        };
        let (bg, fg) = if self.opens {
            (
                Role::SecondaryContainer.paint(),
                Role::OnSecondaryContainer.paint(),
            )
        } else {
            (
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTAINER),
                crate::ui::off(),
            )
        };
        ui.fill(x, y, size, size, faraday_draw::tokens::SHAPE_M, bg);
        ui.icon(x, y, size, glyph, glyph_size, fg);
    }

    /// Draws the tile at (`tx`, `ty`), `tw3` wide and `th` tall.
    pub fn draw(&self, ui: &mut Ui, tx: f32, ty: f32, (tw3, th): (f32, f32)) {
        let (grow_n, grow_l) = self.grown(ui, tw3);
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
        match self.mark {
            Mark::Beside(_) => {
                let ib = HOME_TILE_ICON;
                self.draw_mark(
                    ui,
                    (tx + HOME_TILE_INSET, ty + (th - ib) / 2.0),
                    (ib, HOME_TILE_GLYPH),
                );
            }
            Mark::Above(_) => self.draw_mark(
                ui,
                (tx + FACE_TOP, ty + FACE_TOP),
                (FACE_ABOVE_ICON, FACE_ABOVE_GLYPH),
            ),
            Mark::None => {}
        }
        let ink = if self.opens {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        let (x, room) = (tx + self.text_x(), tw3 - self.text_x() - FACE_INSET);
        // Beside an icon a tile with no line has its name centred.
        let beside_alone = matches!(self.mark, Mark::Beside(_)) && self.line.is_empty();
        let top = if beside_alone {
            let block = ui.wrap_height_as(TITLE_S, self.name, room);
            ty + (th - block) / 2.0
        } else {
            ty + self.name_y()
        };
        ui.wrap_as((x, top, room), TITLE_S, ink, self.name);
        let warn = if self.warn {
            Role::Warning
        } else {
            Role::OnSurfaceVariant
        };
        let ly = top + FACE_LINE_Y + grow_n;
        ui.wrap_as((x, ly, room), BODY_S, warn, self.line);
        let tag_y = top + FACE_TAGS_Y + grow_n + grow_l;
        // The standards, as small tags.
        let mut gx = tx + FACE_INSET;
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
