//! Check an address' scanner on the wide layout: the camera's frame in a
//! pane at the left of the card where the pair stands
//! (`docs/WORKSPACE.md` §14, `docs/FLOW-CONTAINER.md` §4), the card holding
//! the title and Cancel. The scan is the sheet's own state, one camera;
//! only where it is drawn differs. Nothing here holds a secret: the frame
//! is drawn from the scan state and leaves with it.

use crate::drawing::View;
use crate::screens::hub_layout::{self, Left, Narrow, Panes, Tile};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Ask, Faraday, Screen, Sheet};
use faraday_draw::tokens::{BODY_L, GAP};

/// Check an address' panes: the scanner's frame and the card.
const PANES: Panes = Panes {
    left: Left::Own,
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// Whether the scanner on screen stands in a pane, the page beside it
/// live: this page's own, or the session pane (`session_pane::scan`). A
/// lookup.
pub(crate) fn scanning(app: &Faraday) -> bool {
    own(app) || crate::screens::pane_scanning(app)
}

/// Whether the scanner opened for this page is on screen to be drawn in
/// its own pane: its picture is up, no read card has replaced it and there
/// is no camera to choose between (the sheet has the chooser). A lookup.
fn own(app: &Faraday) -> bool {
    app.screen == Screen::CheckAddress
        && app.sheet == Some(Sheet::Scan)
        && app.cameras().len() <= 1
        && app
            .scan()
            .is_some_and(|s| s.ask == Ask::Address && s.card.is_none())
}

/// Where the pair stands, when the scanner is in a pane: the wide layout,
/// the scanner up and the room holds the pair (`flow::pair`). A lookup.
pub(crate) fn place(app: &Faraday, compact: bool, room: f32) -> Option<hub_layout::Place> {
    if !stands(app, compact) {
        return None;
    }
    PANES.place(room, app.guide_beside_pair())
}

/// Whether the page is one that stands as the pair, given the room. A
/// lookup.
pub(crate) fn stands(app: &Faraday, compact: bool) -> bool {
    !compact && own(app)
}

/// Whether the pair stands in a `room`: the page's area is then the work
/// width. A lookup.
pub(crate) fn place_in(app: &Faraday, room: f32) -> bool {
    place(app, app.is_compact(), room).is_some()
}

/// The pane's frame and the card's title and Cancel, with the pair's
/// lines. Returns the page's end `y`.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, p: hub_layout::Place, y: f32) -> f32 {
    let Some(sc) = app.scan() else { return y };
    let (px, w) = (x0 + p.pane_x, p.pane_w);
    let h = w * 0.75;
    ui.fill(px, y, w, h, 8.0, Role::SurfaceContainerHighest);
    crate::screens::scan_frame::draw(ui, (px, y, w, h), sc);
    let (x, cw) = (x0 + p.card_x, p.card_w);
    let mut cy = y;
    cy += ui.wrap_as((x, cy, cw), BODY_L, Role::OnSurface, EN.scan_title_address) + GAP;
    // Paste and the samples stand as in the sheet (`docs/QR.md` §8 item 5).
    crate::screens::scan_demo_offers(app, ui);
    let paste = app.scan_pastes();
    let ways = [
        (EN.scan_paste, Style::Secondary, Action::ScanPaste),
        (EN.scan_cancel, Style::Secondary, Action::Cancel),
    ];
    cy += crate::screens::button_rows(ui, x, cy, cw, &ways[usize::from(!paste)..]);
    let end = cy.max(y + h);
    let at = x0 + p.pane_x + p.pane_w + p.gap / 2.0;
    let edges = (x0 + p.pane_x, x0 + p.card_x + p.card_w);
    hub_layout::lines(ui, (at, y, end), Some((edges, f32::MAX)));
    end
}

/// After an action pressed on `from` while the scanner stood in a pane:
/// the page is live beside it, so a press that leaves the page or puts
/// another sheet up ends the scan as the sheet's own Cancel does: the
/// camera goes off and the frame, what was seen and the card go with the
/// scan state. A read card under another sheet is left to it
/// (`Faraday::read_card_tidy`). A scan that ended itself (Cancel, a code
/// read) has nothing left.
pub(crate) fn left(app: &mut Faraday, from: Screen) {
    let Some(scan) = app.camera.scan() else {
        return;
    };
    let under = scan.card.is_some() && app.sheet.is_some();
    if app.screen != from || (app.sheet != Some(Sheet::Scan) && !under) {
        app.camera.close();
        if app.sheet == Some(Sheet::Scan) {
            app.sheet = None;
        }
        app.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
    }
}
