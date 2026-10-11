//! §4.13 Actions: the four ways a screen offers to do something.

use alloc::string::String;

use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{ButtonStyle, Icon, Widget};

/// §4.13 Primary: "One accent button at the bottom. Confirms typed or
/// rolled input; moves on."
///
/// `enabled` is false while the input is not acceptable yet; the button
/// keeps its place and its size, so nothing moves when it becomes live.
pub fn primary(id: Id, label: impl Into<String>, enabled: bool) -> Node {
    Node::widget(Widget::Button {
        id,
        label: label.into(),
        style: ButtonStyle::Primary,
        enabled,
    })
    .height(tokens::CTA)
}

/// §4.13 Secondary: "A plain button beside it, natural width. The safe
/// alternative."
pub fn secondary(id: Id, label: impl Into<String>) -> Node {
    Node::widget(Widget::button(id, label, ButtonStyle::Secondary)).height(tokens::CTA)
}

/// §4.13 Hold: "An outlined button that fills while held. Sign, forget a
/// key, wipe. The label is the verb: 'Hold to sign'. No sentence above
/// it."
///
/// Nothing above the button belongs to this component: what will happen
/// is the record on the screen, not a line attached to the control.
/// `danger` colours the fill for a step that removes something.
/// `enabled` is false while the step is not available yet; the button
/// keeps its place and its size, as a primary does.
pub fn hold(id: Id, label: impl Into<String>, danger: bool, enabled: bool) -> Node {
    let style = if danger {
        ButtonStyle::Danger
    } else {
        ButtonStyle::Primary
    };
    Node::widget(Widget::hold_button_enabled(id, label, style, enabled)).height(tokens::CTA)
}

/// §4.13 Row action: "A menu row. Ways out of a result: 'Show as QR',
/// 'Save to file'."
pub fn row_action(
    class: crate::geom::SizeClass,
    id: Id,
    icon: Icon,
    label: impl Into<String>,
) -> Node {
    Node::widget(Widget::menu_row(id, icon, label, None).with_height(tokens::menu_row(class)))
}
