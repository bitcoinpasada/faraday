//! §5 Hub: the status line and the grid of area tiles. Used by Home,
//! which is the launcher (`docs/PLANNING.md` §16.104 rule 4).

use alloc::string::String;
use alloc::vec::Vec;

use crate::components::{self, Network, Tile};
use crate::layout::{Id, Node};
use crate::organisms;
use crate::tokens;

use super::{Chrome, SCROLL};

/// Home's status line (§4.1, §4.8): the tier badge, the network badge,
/// the session badge while one is open, and at the right the lock.
pub struct Status {
    /// The tier and the host the build runs on: "Tier C · desktop".
    pub tier: String,
    /// The badge opens the tiers screen; `None` leaves it a label.
    pub tier_id: Option<Id>,
    /// The chain. Mainnet carries no badge.
    pub network: Network,
    /// The lock icon button, which locks the session now (§4.1).
    pub lock: Option<Id>,
    /// A badge beside the network's while this device holds a MuSig2
    /// secret nonce (§4.1). It is a label and opens nothing.
    pub session: Option<String>,
    /// A line that stands in for the badges while it is set: what a
    /// second Back does after a first one on Home (§4.1). The same
    /// caption line every field reserves (§4.3), so nothing moves.
    pub notice: Option<String>,
}

/// The status line: the badges at the left and the lock at the
/// right, on an app bar's height so the content under it starts where
/// every other screen's does (§4.1). Nothing else, and no countdown:
/// §2.10 says status shows state, not urgency. While a notice is set it
/// stands where the badges do, on the caption line every field
/// reserves, so the list under it does not move.
fn status_line(c: &Chrome, status: &Status) -> Node {
    let left = match &status.notice {
        Some(notice) => components::inline_error(c.class(), Some(notice.clone())),
        None => {
            let tier = components::tier_badge(status.tier_id, status.tier.clone());
            let session = status
                .session
                .clone()
                .map(|label| components::tier_badge(None, label));
            components::badges::badge_row(
                Some(tier),
                components::network_badge(status.network),
                session,
            )
        }
    };
    components::status_line(left, status.lock)
}

/// §5 Hub as the launcher: the status line, then the tiles of the six
/// areas in [`organisms::tile_grid`]'s columns, and nothing else.
///
/// The grid takes the whole body, so it never scrolls: three rows of two
/// on a portrait panel, and on `wide` the same grid in the pane beside
/// the sidebar that lists the same six.
///
/// Made of [`components::tier_badge`] and [`components::network_badge`]
/// (§4.8), [`components::status_line`] (§4.1) and
/// [`organisms::tile_grid`] (§4.1).
pub fn launcher(c: &Chrome, status: &Status, tiles: Vec<Tile>) -> Node {
    let body = organisms::tile_grid(
        c.m,
        tiles
            .into_iter()
            .map(|t| organisms::TileSpec {
                id: t.id,
                icon: t.icon,
                label: t.label,
                enabled: t.enabled,
                badge: t.badge,
            })
            .collect(),
    )
    .padding(crate::geom::Edges {
        left: 0.0,
        right: 0.0,
        top: tokens::GAP,
        bottom: tokens::GAP,
    });
    let cap = c.cap().map(|cap| cap + 2.0 * tokens::PAD);
    c.framed(
        false,
        organisms::screen_at(
            SCROLL,
            status_line(c, status),
            body,
            0.0,
            None,
            tokens::action_bottom(c.inset_bottom()),
            cap,
        ),
    )
}
