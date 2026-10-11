//! §4.8 Badges: the three things that are a badge, and the rule that a
//! state is not one.
//!
//! A badge names what a thing *is* and never changes while the screen is
//! open: the network, the tier, the kind of an output. A state that the
//! person can act on — "not verified" — is a row value in the caution
//! tone ([`state_row`]), because a badge cannot carry a label and cannot
//! be tapped.

use alloc::string::String;

use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{Tone, Widget};

use super::choice::value_row;

/// The chain a key or a transaction belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// The real chain; no badge.
    Mainnet,
    /// The public test chain.
    Testnet,
    /// The signed test chain.
    Signet,
    /// A local chain.
    Regtest,
}

impl Network {
    /// The badge's label, or `None` on mainnet.
    pub fn label(self) -> Option<&'static str> {
        match self {
            Network::Mainnet => None,
            Network::Testnet => Some("TESTNET"),
            Network::Signet => Some("SIGNET"),
            Network::Regtest => Some("REGTEST"),
        }
    }
}

/// §4.8 Network: "A caution pill 'REGTEST' / 'TESTNET' / 'SIGNET';
/// absent on mainnet. On Home and on every Sign review screen."
pub fn network_badge(network: Network) -> Option<Node> {
    network
        .label()
        .map(|label| Node::widget(Widget::badge(label, Tone::Caution)))
}

/// §4.8 Tier: "'Tier C · desktop' on Home's status line; opens the tiers
/// screen."
///
/// The label is the caller's, because it names the tier and the host the
/// build runs on, which this crate does not know. `id` makes the pill the
/// target that opens the tiers screen; it is as wide as its label either
/// way, so the status line keeps its shape.
pub fn tier_badge(id: Option<Id>, label: impl Into<String>) -> Node {
    Node::widget(match id {
        Some(id) => Widget::badge_button(id, label, Tone::Info),
        None => Widget::badge(label, Tone::Info),
    })
}

/// What a badge on a transaction output says (§4.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputBadge {
    /// Pays a verified change address of a loaded key.
    Mine,
    /// Looks like change but derives from no loaded key.
    MineNotVerified,
    /// Too small to be worth spending.
    Dust,
}

impl OutputBadge {
    /// The badge's label.
    pub fn label(self) -> &'static str {
        match self {
            OutputBadge::Mine => "MINE",
            OutputBadge::MineNotVerified => "MINE · NOT VERIFIED",
            OutputBadge::Dust => "DUST",
        }
    }

    /// The tone §4.8 gives it: success, danger, caution.
    pub fn tone(self) -> Tone {
        match self {
            OutputBadge::Mine => Tone::Success,
            OutputBadge::MineNotVerified => Tone::Danger,
            OutputBadge::Dust => Tone::Caution,
        }
    }
}

/// §4.8 Mine: "On a transaction output that pays one of the loaded
/// keys: 'MINE' (success) when the change was verified, 'MINE · NOT
/// VERIFIED' (danger) when not. Outputs to others carry no badge; there
/// is no 'recipient' badge and no 'kind' row."
pub fn output_badge(badge: OutputBadge) -> Node {
    Node::widget(Widget::badge(badge.label(), badge.tone()))
}

/// §4.8 Caution pill: a badge that names what a thing is and that the
/// person should weigh before taking it — "trusts this device" on the
/// source that takes the device's own randomness. It never changes
/// while the screen is open, which is what makes it a badge and not a
/// state row.
pub fn caution_badge(label: impl Into<String>) -> Node {
    Node::widget(Widget::badge(label.into(), Tone::Caution))
}

/// §4.8 State value: "Backup 'not verified' in the caution tone as a row
/// value; never a badge."
///
/// The same row as §4.2's mode row, in the tone the state deserves.
pub fn state_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    state: impl Into<String>,
    tone: Tone,
) -> Node {
    value_row(class, id, label, state, tone)
}

/// A row of badges: the status line's tier and network, in that order,
/// with the network left out on mainnet.
pub fn badge_row(tier: Option<Node>, network: Option<Node>, session: Option<Node>) -> Node {
    let mut row = crate::layout::Node::row()
        .gap(tokens::GAP_SMALL)
        .align(crate::layout::Align::Center);
    for badge in [tier, network, session].into_iter().flatten() {
        row = row.child(badge);
    }
    row
}
