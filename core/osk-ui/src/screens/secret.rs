//! §5 Secret: "the whole body as one panel, eye". Used by the seed hex,
//! a master private key, a SeedQR and a CompactSeedQR.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use osk_codec::qr::QrMatrix;

use crate::components::{self, secrets::WordWidth};
use crate::layout::{Align, Id, Justify, Node};
use crate::organisms;
use crate::tokens;
use crate::widgets::{Icon, Tone, Widget};

use super::{Action, Chrome, Frame};

/// What the panel holds.
pub enum Value<'a> {
    /// A long string: the seed hex, an extended private key.
    Text(&'a str),
    /// A QR: a SeedQR or a CompactSeedQR.
    Code(Rc<QrMatrix>),
    /// A QR drawn as a grid to copy by hand, and the region of it this
    /// page shows (`docs/PLANNING.md` §8.2 item 5a).
    Grid {
        /// The modules.
        matrix: Rc<QrMatrix>,
        /// The first module of the region, `(x, y)`.
        origin: (u16, u16),
        /// The region's side in modules.
        span: u16,
    },
    /// A secret made of words, on the words panel: a rolled passphrase.
    Words {
        /// The words, in order.
        words: &'a [String],
        /// How wide a word of this list is.
        width: WordWidth,
    },
}

/// What a [`secret`] screen shows.
pub struct Secret<'a> {
    /// The app bar's title, which names the secret.
    pub title: &'a str,
    /// The secret.
    pub value: Value<'a>,
    /// Whether it is shown.
    pub revealed: bool,
    /// The panel's application id, which is also its touch surface.
    pub panel: Id,
    /// The app bar's eye, which shows the secret for 30 s (§4.10).
    pub eye: Option<Id>,
    /// The share of the eye's reveal still to run, drawn as the ring
    /// that empties clockwise. `None` while the eye is not running.
    pub remaining: Option<f32>,
    /// The facts under the panel, where the secret has any a person
    /// needs beside it: "Entropy · 77 bits" (§4.11).
    pub rows: Vec<components::Record>,
    /// The one way on, where the screen ends a flow rather than being a
    /// row a chevron came from.
    pub action: Option<Action>,
    /// The way past, beside that one, where the flow the screen stands
    /// in can be gone on with without doing what it asks: "Skip"
    /// beside "Continue" on a string that is about to be typed back.
    pub secondary: Option<Action>,
    /// The pager, where the panel is one page of a run: the quadrants of
    /// a transcription grid. It takes the bottom slot, since a screen
    /// that pages a secret has no action of its own.
    pub pager: Option<super::Pager>,
}

/// §5 Secret: "the whole body as one panel, the facts it needs beside
/// it, eye."
///
/// §2.8: "Long secrets and long strings live on their own screen,
/// reached by a row." §4.10: "one panel as tall as the string at the
/// largest mono size that fits the width, in the content zone. Masked,
/// the panel holds one centred eye-slash icon and nothing else." The
/// panel gives the string no caption in either state: an orange-outlined
/// box is a secret and needs no line under it saying so (§2.1).
///
/// The panel is sized to the string, not to the screen: the chunked
/// string takes the largest size from the mono ramp — up to
/// [`tokens::comparison_max`], which is what §5 Transcribe means by "the
/// largest type that fits" — at which its groups of four fit the width in
/// lines the space holds, and the panel is those lines and its padding. The masked panel keeps that rectangle and
/// draws one eye-slash in the middle of it, so nothing moves when a
/// finger lands on it. §2.6 then places it: one block, centred in the
/// space under the app bar on every class, inside the column cap on
/// `wide`.
///
/// Made of [`organisms::secret_panel`] (§4.10), with
/// [`components::comparison_string`] (§4.5) or
/// [`components::qr_block`] (§4.9) inside it and
/// [`components::bar_eye`] (§4.1) for the eye.
pub fn secret(c: &Chrome, s: Secret<'_>) -> Node {
    let class = c.class();
    // The panel is sized to the secret in both states: the same string
    // or the same square, drawn or not, so nothing moves when a finger
    // lands on it. Masked, one eye-slash is the only mark on it.
    if let Value::Words { words, width } = &s.value {
        // A secret made of words is the words panel: rows of words, not
        // one string chunked in fours (§4.10).
        let panel = components::secret_panel(
            class,
            s.panel,
            components::Secret::Words {
                words,
                first: 1,
                indices: None,
                width: *width,
                steel: false,
            },
            s.revealed,
            components::secrets::panel_room(c.column().width_dp),
        );
        let mut block = Node::column().gap(c.body_gap()).child(panel);
        if !s.rows.is_empty() {
            block = block.child(components::record(&c.column(), s.rows));
        }
        let footer = footer_of(c, s.pager, s.action, s.secondary);
        let trailing = s.eye.map(|id| components::bar_eye(id, s.remaining));
        return c.fixed(
            Frame::new(s.title, c.place(block, None))
                .trailing(trailing)
                .footer(footer)
                .dimmed(),
        );
    }
    let content = match &s.value {
        Value::Text(text) => {
            if s.revealed {
                components::comparison_string(class, s.panel, *text, true)
            } else {
                // The same string, the same ramp, drawing nothing: what
                // keeps the panel's rectangle identical in both states
                // (§4.10).
                masked(components::masked_comparison_string(
                    class, s.panel, *text, true,
                ))
            }
        }
        Value::Words { .. } => unreachable!("drawn above"),
        Value::Code(matrix) => {
            let code = components::qr_block(
                class,
                matrix.clone(),
                (!s.revealed).then_some((s.panel, false)),
            );
            if s.revealed { code } else { masked(code) }
        }
        Value::Grid {
            matrix,
            origin,
            span,
        } => {
            let grid = components::qr_grid(matrix.clone(), *origin, *span, (s.panel, s.revealed));
            if s.revealed { grid } else { masked(grid) }
        }
    };
    // §4.10 masks the panel with "one centred eye-slash icon and nothing
    // else", so a panel narrower or shorter than that icon would be a
    // mark spilling out of its own box. The floor is the icon and the
    // panel's padding, and it applies in both states, which is what
    // keeps the rectangle identical masked and revealed: four checksum
    // bits and a 111-character key are the same box either way.
    let panel = organisms::secret_panel(s.panel, content, None)
        .min_height(tokens::ICON_LARGE + 2.0 * tokens::GAP);
    let trailing = s.eye.map(|id| components::bar_eye(id, s.remaining));
    // The facts belong to the panel, so they are inside the one block
    // §2.6 centres and move with it.
    let mut block = Node::column().gap(c.body_gap()).child(panel);
    if !s.rows.is_empty() {
        block = block.child(components::record(&c.column(), s.rows));
    }
    let footer = footer_of(c, s.pager, s.action, s.secondary);
    // The panel keeps its own height inside the block §2.6 centres, so
    // the spare height falls above and below it and never stretches the
    // box round the string.
    c.fixed(
        Frame::new(s.title, c.place(block, None))
            .trailing(trailing)
            .footer(footer)
            .dimmed(),
    )
}

/// The bottom slot: the pager of a paged secret, or the action where
/// the screen ends a flow, with the way past beside it where the flow
/// offers one. A secret that pages has no action of its own — the
/// chevron is the way out — so the pager and the actions never meet.
fn footer_of(
    c: &Chrome,
    pager: Option<super::Pager>,
    action: Option<Action>,
    secondary: Option<Action>,
) -> Vec<Node> {
    if let Some(p) = pager {
        return c.buttons(alloc::vec![p.node(c.class(), false)]);
    }
    let Some(action) = action else {
        return Vec::new();
    };
    match secondary {
        Some(s) => c.actions(alloc::vec![s, action]),
        None => c.actions(alloc::vec![action]),
    }
}

/// §4.10: "Masked, the panel holds one centred eye-slash icon and
/// nothing else." `content` is the secret's own rectangle drawing
/// nothing, and the icon is laid over it without adding to it: the
/// overlay measures nothing and is stretched to the stack, so the
/// masked panel is exactly the revealed one whatever the icon's size
/// against the string's or the square's. §2.8: "Nobody is shown a
/// hundred bullets."
fn masked(content: Node) -> Node {
    Node::stack().align(Align::Center).child(content).child(
        Node::column()
            .justify(Justify::Center)
            .align(Align::Center)
            .child(Node::widget(Widget::icon(
                Icon::EyeOff,
                tokens::ICON_LARGE,
                Tone::Muted,
            )))
            .align_self(Align::Stretch)
            .max_height(0.0)
            .max_width(0.0),
    )
}
