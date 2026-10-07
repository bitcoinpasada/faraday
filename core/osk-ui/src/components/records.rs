//! §4.11 Records and results: the table, the result, the warning card
//! and the dimmed reason.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::{Edges, Metrics, SizeClass};
use crate::layout::{Align, Id, Node};
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::{Icon, PanelStyle, Tone, WarningLevel, Widget};

/// One row of a [`record`]: a label and a value beside it, or a
/// reference row that takes the table's whole width (§4.11).
pub enum Record {
    /// A label and a short value in the value column.
    Fact {
        /// What the value is.
        label: String,
        /// The value, already styled by the component that made it.
        value: Node,
    },
    /// §4.11: "A long string is never a block here: it is a reference
    /// row in the table's flow." The row is [`super::reference_row`],
    /// which opens the Compare screen.
    Reference {
        /// Application id: the whole row is the target.
        id: Id,
        /// What the string is.
        label: String,
        /// The whole string; the row shows its head and tail.
        value: String,
    },
    /// §4.2 Value row drawn flat inside the table: a mode the screen
    /// states and the person can change. The value is short and shown
    /// whole; the whole row opens the Choice screen for it.
    Value {
        /// Application id: the whole row is the target.
        id: Id,
        /// What the value is.
        label: String,
        /// The value, whole.
        value: String,
        /// Whether the value is read character by character and so is
        /// set in the mono face: a descriptor's summary.
        mono: bool,
    },
    /// §4.3: the same value row on one line — the label, then the
    /// value, both at the label token — for an entry screen whose space
    /// above the field holds one line and not two.
    OneLine {
        /// Application id where the row opens a Choice, as [`Record::Value`]
        /// does; `None` where the row is a fact and nothing more.
        id: Option<Id>,
        /// What the value is.
        label: String,
        /// The value, whole.
        value: String,
        /// The monospace face, for a fingerprint.
        mono: bool,
        /// The fingerprint glyph before the value, where the value is a
        /// fingerprint (`docs/PLANNING.md` §16.131).
        glyph: bool,
    },
    /// §4.1: a fact whose value goes under its label rather than beside
    /// it, drawn flat. For the row whose label is as long as its value —
    /// an input's number and amount over the script it spends — which
    /// the value column cannot hold side by side.
    Stacked {
        /// What the value is.
        label: String,
        /// The value.
        value: String,
    },
    /// §4.2 Toggle drawn flat inside the table: the one switch a review
    /// screen carries, which is the acknowledgement §4.11 asks a danger
    /// for before the hold.
    Toggle {
        /// Application id: the whole row is the target.
        id: Id,
        /// What is switched.
        label: String,
        /// The current state.
        on: bool,
    },
    /// §4.13 Row action inside a result's table: "A menu row. Ways out
    /// of a result." The way on from a checksum whose descriptor is a
    /// wallet this device can load.
    Action {
        /// Application id: the whole row is the target.
        id: Id,
        /// The row's glyph.
        icon: Icon,
        /// What the row does.
        label: String,
    },
    /// §4.11 dimmed control, as a row of the table: a way out the shell
    /// cannot serve, with the two or three words that say why under its
    /// label.
    Dimmed {
        /// The row's glyph.
        icon: Icon,
        /// What the row would do.
        label: String,
        /// Why it cannot: "no clipboard".
        reason: Option<String>,
    },
    /// A badge on its own line: §4.8's "MINE" on an output.
    Badge(Node),
    /// §4.4 glyph row: a leading key or eye that says whether this
    /// device holds the private key, then the label and the value.
    Glyph {
        /// The key glyph or the eye.
        icon: Icon,
        /// What the value is.
        label: String,
        /// The value, in the monospace face a fingerprint is read in.
        value: String,
        /// The value's tone: the caution of a key a warning card names,
        /// and [`Tone::Text`] everywhere else.
        tone: Tone,
    },
}

impl Record {
    /// A row whose value is one line of body text.
    pub fn text(label: impl Into<String>, value: impl Into<String>, tone: Tone) -> Self {
        Record::Fact {
            label: label.into(),
            value: super::text::value(value, false, tone),
        }
    }

    /// A row whose value is one line in the monospace face: a
    /// fingerprint, a path, an amount.
    pub fn mono(label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Fact {
            label: label.into(),
            value: super::text::value(value, true, Tone::Text),
        }
    }

    /// A row whose value is a fingerprint, or several: the fingerprint
    /// glyph before each, in the monospace face (`docs/PLANNING.md`
    /// §16.131). A placeholder in its place is drawn as [`Record::mono`]
    /// draws it, with no glyph.
    pub fn fingerprint(label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Fact {
            label: label.into(),
            value: super::text::fingerprint_value(value, Tone::Text),
        }
    }

    /// A row whose value the caller built and which is one line.
    pub fn line(label: impl Into<String>, value: Node) -> Self {
        Record::Fact {
            label: label.into(),
            value,
        }
    }

    /// A long string: a reference row in the table's flow (§4.5).
    pub fn reference(id: Id, label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Reference {
            id,
            label: label.into(),
            value: value.into(),
        }
    }

    /// §4.2: a mode the record states and a tap changes — the wordlist
    /// a scanned key's words came from. Short, shown whole, and the row
    /// opens the Choice screen for it.
    pub fn value(id: Id, label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Value {
            id,
            label: label.into(),
            value: value.into(),
            mono: false,
        }
    }

    /// The same row over a short value read character by character — a
    /// descriptor's summary, "wpkh · 73c5da0a · #qf45pmyh" — shown whole
    /// in the mono face, where a reference row would elide it.
    pub fn mono_value(id: Id, label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Value {
            id,
            label: label.into(),
            value: value.into(),
            mono: true,
        }
    }

    /// §4.3: a value row on one line, for the space above an entry
    /// field that holds one line and not two.
    pub fn one_line(
        id: Option<Id>,
        label: impl Into<String>,
        value: impl Into<String>,
        mono: bool,
    ) -> Self {
        Record::OneLine {
            id,
            label: label.into(),
            value: value.into(),
            mono,
            glyph: false,
        }
    }

    /// §4.1: a fact the value column is too narrow for, on two lines.
    pub fn stacked(label: impl Into<String>, value: impl Into<String>) -> Self {
        Record::Stacked {
            label: label.into(),
            value: value.into(),
        }
    }

    /// §4.2: the one switch a table carries — "Acknowledged" over a
    /// danger warning. The row is the target, not the switch.
    pub fn toggle(id: Id, label: impl Into<String>, on: bool) -> Self {
        Record::Toggle {
            id,
            label: label.into(),
            on,
        }
    }

    /// §4.13: the way out of a result, as a row of its table.
    pub fn action(id: Id, icon: Icon, label: impl Into<String>) -> Self {
        Record::Action {
            id,
            icon,
            label: label.into(),
        }
    }

    /// §4.11's dimmed control, as a row of the same table: a way out
    /// the shell cannot serve, with the reason under its label.
    pub fn dimmed(icon: Icon, label: impl Into<String>, reason: Option<String>) -> Self {
        Record::Dimmed {
            icon,
            label: label.into(),
            reason,
        }
    }

    /// §4.8's output badge, on a line of its own.
    pub fn badge(node: Node) -> Self {
        Record::Badge(node)
    }

    /// §4.4: a row a leading glyph marks — the key this device holds,
    /// or the public key it only watches.
    pub fn glyph(
        icon: Icon,
        label: impl Into<String>,
        value: impl Into<String>,
        tone: Tone,
    ) -> Self {
        Record::Glyph {
            icon,
            label: label.into(),
            value: value.into(),
            tone,
        }
    }
}

/// Width in dp of the label column: a share of the body, never narrower
/// than two or three words and never more than a third of a wide pane.
pub fn label_width(m: &Metrics) -> f32 {
    let body = (m.width_dp - 2.0 * tokens::PAD).max(0.0);
    (body * tokens::RECORD_LABEL_SHARE).clamp(tokens::RECORD_LABEL_MIN, tokens::RECORD_LABEL_MAX)
}

/// Width in dp of the value column, which is what the label column
/// leaves.
pub fn value_width(m: &Metrics) -> f32 {
    (m.width_dp - 2.0 * tokens::PAD - label_width(m) - tokens::GAP).max(0.0)
}

/// §4.11 Record: "A table: label column left, value column right, on
/// every class. A short value sits beside its label and may wrap to a
/// second line. A long string is never a block here: it is a reference
/// row in the table's flow. One fact per row."
///
/// Two shapes, one table. A short value starts at the value column's x
/// on every row, so the eye runs down the column. A long string is a
/// reference row drawn flat — label above, elided value below, chevron,
/// no surface — which takes the table's whole width and opens the
/// Compare screen, because
/// a third of the width would break it into more lines than it has
/// characters to spare and reading it whole is a screen of its own
/// (§4.5).
pub fn record(m: &Metrics, rows: Vec<Record>) -> Node {
    let width = label_width(m);
    let value_column = value_width(m);
    let gap = if m.class == SizeClass::Small {
        tokens::GAP_SMALL
    } else {
        tokens::GAP
    };
    Node::column().gap(gap).children(rows.into_iter().map(|r| {
        match r {
            Record::Fact { label, value: v } => Node::row()
                .gap(tokens::GAP)
                .align(Align::Start)
                .child(super::text::label(label).width(width))
                .child(v.weight(1.0).max_width(value_column)),
            Record::Reference { id, label, value } => {
                super::strings::flat_reference_row(m.class, id, label, &value)
            }
            // §4.11: "every row looks like a table row". A row that
            // opens a screen and states no value is one line, so its
            // label sits on the chevron's own line rather than at the top
            // of a two-line band.
            Record::Value {
                id,
                label,
                value,
                mono,
            } => Node::widget(
                Widget::list_row(Some(id), label, (!value.is_empty()).then_some(value), None)
                    .with_height(tokens::stacked_row(m.class))
                    .with_value_below(mono, Tone::Text)
                    .flat(),
            ),
            Record::OneLine {
                id,
                label,
                value,
                mono,
                glyph,
            } => {
                let g = glyph
                    .then(|| super::identity::fingerprint_glyph(&value))
                    .flatten();
                let row = Widget::list_row(id, label, Some(value), None)
                    .with_height(tokens::one_line_row(m.class))
                    .with_value_beside(mono, Tone::Text)
                    .tight()
                    .flat();
                Node::widget(match g {
                    Some(g) => row.with_value_glyph(g),
                    None => row,
                })
            }
            Record::Stacked { label, value } => {
                super::choice::flat_fact_row(m.class, label, value, false)
            }
            Record::Toggle { id, label, on } => {
                super::choice::toggle_row(m.class, id, label, on, None)
            }
            Record::Action { id, icon, label } => super::row_action(m.class, id, icon, label),
            Record::Dimmed {
                icon,
                label,
                reason,
            } => super::dimmed_row(m.class, Some(icon), label, reason),
            Record::Badge(node) => Node::row().align(Align::Center).child(node),
            // §4.11: "every row looks like a table row". The glyph sits
            // where a row's leading mark sits, and the value under the
            // label, so the mark is read before the key it marks.
            Record::Glyph {
                icon,
                label,
                value,
                tone,
            } => Node::widget(
                Widget::list_row(None, label, Some(value), None)
                    .with_icon(icon)
                    .with_height(tokens::stacked_row(m.class))
                    .with_value_below(true, tone)
                    .without_chevron()
                    .flat(),
            ),
        }
    }))
}

/// Height in dp of a table of `rows` at this class: what a screen
/// measures its table against before it places it (`docs/DESIGN.md`
/// §2.6: "top-anchored ... when it takes more than half the zone,
/// vertically centred in it when it takes less").
///
/// A short value is one line of body text, a reference row is the
/// two-line row every value row is, and a badge is its own pill. The
/// count is what the table asks for, not what a wrapped value might take
/// on a narrow pane: it decides where the table sits, not how tall it is
/// drawn.
pub fn record_height(class: SizeClass, rows: &[Record]) -> f32 {
    if rows.is_empty() {
        return 0.0;
    }
    let gap = if class == SizeClass::Small {
        tokens::GAP_SMALL
    } else {
        tokens::GAP
    };
    let mut height = gap * (rows.len() as f32 - 1.0);
    for row in rows {
        height += match row {
            Record::Fact { .. } => tokens::scaled_line(tokens::BODY, class),
            Record::OneLine { .. } => tokens::one_line_row(class),
            Record::Reference { .. }
            | Record::Value { .. }
            | Record::Stacked { .. }
            | Record::Glyph { .. } => tokens::stacked_row(class),
            Record::Toggle { .. } => tokens::choice_row(class),
            Record::Action { .. } => tokens::menu_row(class),
            Record::Dimmed { reason, .. } => {
                if reason.is_some() {
                    tokens::stacked_row(class)
                } else {
                    tokens::menu_row(class)
                }
            }
            Record::Badge(_) => badge_line(class),
        };
    }
    height
}

/// Height in dp of one badge on a line of its own: its caption and the
/// padding above and below it (§4.8).
fn badge_line(class: SizeClass) -> f32 {
    tokens::scaled_line(tokens::CAPTION, class) + 2.0 * tokens::BADGE_PAD_Y
}

/// Width in dp of the text column inside a [`warning_card`]: what the
/// card's own padding and the level icon leave of the body's width.
///
/// The label and the value share it, one above the other, so both have
/// the card's whole width rather than a share of it.
fn warning_text_width(m: &Metrics) -> f32 {
    (m.width_dp - 2.0 * tokens::PAD - 2.0 * tokens::GAP - tokens::ICON_SMALL - tokens::GAP).max(0.0)
}

/// Height in dp of a [`warning_card`] whose value takes `lines` lines:
/// the height of the two-line row it is — the label over the value —
/// and, where the value wraps, what the extra lines add to it.
///
/// `lines` is 1 for the card §4.11 describes: the label above and the
/// value on one line under it, which is a two-line row and as tall as
/// one. A value that does not fit the card's whole width wraps under
/// its label and the card grows by a line; a caller that measures a
/// page before it is laid out passes what it wants reserved.
pub fn warning_card_height(class: SizeClass, lines: usize) -> f32 {
    let text = tokens::scaled_line(tokens::LABEL, class)
        + tokens::STACKED_LINE_GAP
        + tokens::scaled_line(tokens::BODY, class) * lines.max(1) as f32;
    (text.max(tokens::ICON_SMALL) + 2.0 * tokens::GAP_SMALL).max(tokens::stacked_row(class))
}

/// §4.11 Result: "Icon and a coloured title at the top of the body, then
/// a record under it. The colour is on the title."
pub fn result(icon: Icon, tone: Tone, title: impl Into<String>) -> Node {
    Node::row()
        .gap(tokens::GAP)
        .align(Align::Center)
        .child(Node::widget(Widget::icon(icon, tokens::ICON_LARGE, tone)))
        .child(
            Node::widget(
                Widget::paragraph(title, Font::semibold(tokens::TITLE), tone)
                    .align(TextAlign::Start),
            )
            .weight(1.0),
        )
}

/// §4.11 Warning: "A card: level icon, label, value. 'Change · not
/// verified', 'Fee · 12 % of the amount'. Ranked danger, caution, info."
///
/// A label and a value, never a sentence: what the warning is about and
/// what is wrong with it.
///
/// §4.11: "Two lines, as a row with a value is: the label above, the
/// value below it across the card's whole width, the icon beside them.
/// Nothing is cut and nothing is squeezed into half the panel." The two
/// lines take the same fonts, gap and height a two-line row does, so a
/// card and a row read as one pattern; the icon is centred on the pair.
/// A value the card's width cannot hold wraps under its label and the
/// card grows by a line, which is rare.
pub fn warning_card(
    m: &Metrics,
    level: WarningLevel,
    label: impl Into<String>,
    value: impl Into<String>,
) -> Node {
    let text = warning_text_width(m);
    Node::stack()
        .child(Node::widget(Widget::Panel {
            style: PanelStyle::Warning(level),
        }))
        .child(
            Node::row()
                .padding(Edges::symmetric(tokens::GAP, tokens::GAP_SMALL))
                .min_height(tokens::stacked_row(m.class))
                .gap(tokens::GAP)
                .align(Align::Center)
                .child(Node::widget(Widget::icon(
                    level.icon(),
                    tokens::ICON_SMALL,
                    level.tone(),
                )))
                .child(
                    Node::column()
                        .gap(tokens::STACKED_LINE_GAP)
                        .align(Align::Start)
                        .weight(1.0)
                        .max_width(text)
                        .child(super::text::label(label))
                        .child(Node::widget(Widget::paragraph(
                            value,
                            Font::regular(tokens::BODY),
                            level.tone(),
                        ))),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale};
    use crate::layout::{LayoutCtx, solve};

    /// The four sizes `just snapshots` renders.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    const ADDRESS: &str = "bc1pxwww0ct9ue7e8tdnlmug5m2tamfn7q06sahstg39ys4c9f3340qqxrdu9k";

    /// §4.11: "A short value sits beside its label ... a long string is
    /// never a block here: it is a reference row in the table's flow."
    /// So the short values share one x, and the reference row starts at
    /// the label column and takes the table's whole width.
    #[test]
    fn no_value_in_a_record_runs_past_the_margin() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let factor = ctx.scale.factor();
            let m = Metrics::new(area.w as f32 / factor, area.h as f32 / factor, class);
            let rows = alloc::vec![
                Record::line(
                    "Amount",
                    super::super::text::value("60 000 sats", true, Tone::Text).id(Id(1))
                ),
                Record::line(
                    "Fee",
                    super::super::text::value("1 000 sats", true, Tone::Text).id(Id(2))
                ),
                Record::reference(Id(3), "To", ADDRESS),
                Record::line(
                    "Path",
                    super::super::text::value("m/84h/0h/0h", true, Tone::Text).id(Id(4))
                ),
            ];
            let l = solve(
                &Node::column()
                    .padding(crate::geom::Edges::symmetric(tokens::PAD, 0.0))
                    .child(record(&m, rows)),
                area,
                &ctx,
            );
            for id in [Id(1), Id(2), Id(3), Id(4)] {
                let r = l.rect(id).expect("a value");
                assert!(
                    r.right() <= area.right() - ctx.px(tokens::PAD),
                    "value {id:?} runs past the margin at {}x{}",
                    area.w,
                    area.h
                );
            }
        }
    }
}
