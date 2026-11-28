use faraday_guide::{GuidePage, glossary, guide_groups};
use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::guide::{
    self, Find, GuideBlockModel, GuideBody, GuideGroup, GuideNone, GuideSection, GuideSheetModel,
    SheetBody, Tab, TabKind, TabsModel,
};
use faraday_organisms::how::{
    self, Construction, Field, FieldBody, HowBlock, HowModel, Kind, Part,
};
use faraday_organisms::{ButtonModel, FlowText, Offered, RowModel, Shown, Style};
use faraday_present::{Present, offer};
use faraday_strings::Strings;
use faraday_text::Text;

use crate::{GuideSheet, Model, Msg, PaneModel, Press, Refusal, SCROLL, State, TARGET};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        match *self {}
    }
}

impl Refused for Refusal {}

/// Letters, digits and spaces, lower-cased: "bip85" and "bip-85" search alike.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn words_in(text: &str, find: &str) -> bool {
    let hay = fold(text);
    find.split_whitespace()
        .all(|word| hay.contains(&fold(word)))
}

/// Whether the page's title or one of its headings holds every word of `find`.
fn found(page: &GuidePage, find: &str) -> bool {
    let heads = page.sections.iter().skip(1).map(|s| s.heading.as_str());
    std::iter::once(page.title.as_str())
        .chain(heads)
        .any(|text| words_in(text, find))
}

fn gate(state: &State, view: &View<'_>, press: Press) -> Offered<Press> {
    offer::<Press, GuideSheet>(state, &Msg::Press(press.clone()), press, view)
}

fn row<'s>(
    state: &State,
    view: &View<'_>,
    headline: &'static str,
    press: Press,
) -> RowModel<'s, Press> {
    RowModel {
        headline: Shown::plain(headline),
        supporting: Shown::plain(""),
        trailing: None,
        offered: gate(state, view, press),
        chosen: false,
        whole: false,
        chevron: false,
    }
}

fn button<'s>(
    state: &State,
    view: &View<'_>,
    label: &'static str,
    style: Style,
    press: Press,
) -> ButtonModel<'s, Press> {
    ButtonModel {
        label: Shown::plain(label),
        style,
        offered: gate(state, view, press),
        chosen: false,
        icon: None,
    }
}

/// A page's text: the lead, the sections, and the words the page's screen uses.
fn page<'s>(
    state: &State,
    view: &View<'_>,
    s: &'static Strings,
    page: &'static GuidePage,
    look: guide::GuideLook,
) -> GuideBlockModel<'s, Press> {
    let to = state.to();
    let sections = page
        .sections
        .iter()
        .map(|section| GuideSection {
            heading: if section.key == page.place {
                Shown::plain("")
            } else {
                Shown::plain(section.heading.as_str())
            },
            paragraphs: section
                .paragraphs
                .iter()
                .map(|para| FlowText {
                    runs: para
                        .iter()
                        .map(|span| (Shown::plain(span.text.as_str()), span.bold))
                        .collect(),
                    role: look.body,
                })
                .collect(),
            target: to == Some(section.key.as_str()),
        })
        .collect();
    let words = (!page.word_ix.is_empty()).then(|| {
        let all = glossary();
        let rows = page
            .word_ix
            .iter()
            .filter_map(|&i| {
                let w = all.get(usize::from(i))?;
                Some(row(state, view, w.word, Press::Word(u32::from(i))))
            })
            .collect();
        (Shown::plain(s.guide_words_here), rows)
    });
    GuideBlockModel {
        look,
        body: GuideBody::Page(guide::GuidePage { sections, words }),
    }
}

/// The contents: the place's page first under Here, then every page under its group's heading,
/// each a one-line row, over the pages Find leaves.
fn contents<'s>(
    state: &'s State,
    view: &View<'_>,
    s: &'static Strings,
    look: guide::GuideLook,
) -> SheetBody<'s, Press> {
    let find = state.find();
    let mut groups = Vec::new();
    if let Some(here) = state.here().filter(|p| found(p, find)) {
        groups.push(GuideGroup {
            heading: Shown::plain(s.guide_here),
            rows: vec![row(state, view, here.title.as_str(), Press::Here)],
        });
    }
    let mut n = 0u32;
    for group in guide_groups() {
        let first = n;
        n += u32::try_from(group.pages.len()).unwrap_or(u32::MAX);
        let rows: Vec<_> = (first..)
            .zip(group.pages.iter())
            .filter(|(_, p)| found(p, find))
            .map(|(i, p)| row(state, view, p.title.as_str(), Press::Page(i)))
            .collect();
        if !rows.is_empty() {
            groups.push(GuideGroup {
                heading: Shown::plain(group.title),
                rows,
            });
        }
    }
    let none = groups.is_empty().then(|| GuideNone {
        line: Shown::plain(s.guide_no_page_matches),
        clear: Shown::plain(s.guide_clear_find),
        offered: gate(state, view, Press::ClearFind),
    });
    SheetBody::Contents {
        find: Find {
            typed: Shown::plain(state.find_text()),
            hint: Shown::plain(s.guide_find),
            offered: gate(state, view, Press::Focus),
        },
        block: GuideBlockModel {
            look,
            body: GuideBody::Contents { groups, none },
        },
    }
}

fn line(l: &faraday_guide::block::Line) -> how::Line<'static> {
    how::Line {
        text: Shown::plain(l.text),
        kind: match l.kind {
            faraday_guide::block::Kind::Plain => Kind::Plain,
            faraday_guide::block::Kind::Mono => Kind::Mono,
            faraday_guide::block::Kind::Warn => Kind::Warn,
        },
    }
}

fn construction(c: &faraday_guide::construction::Construction) -> Construction<'static> {
    use faraday_guide::construction::Item as I;
    Construction {
        title: Shown::plain(c.title),
        sections: c
            .sections
            .iter()
            .map(|p| how::Section {
                heading: Shown::plain(p.heading),
                items: p
                    .items
                    .iter()
                    .map(|it| match it {
                        I::Para(t) => how::Item::Para(Shown::plain(*t)),
                        I::Table(t) => how::Item::Table(how::Table {
                            head: t.head.iter().map(|h| Shown::plain(*h)).collect(),
                            rows: t
                                .rows
                                .iter()
                                .map(|r| r.iter().map(|c| Shown::plain(*c)).collect())
                                .collect(),
                        }),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The last passing run of the block's cross-check area, as the tab states it.
fn run(b: &faraday_guide::Block, s: &Strings) -> Text {
    match b.cross_check {
        faraday_guide::block::CrossCheck::Area(a) => {
            Text::fill(s.how_cc_seed_cases, &[a.seed, &a.cases.to_string()])
        }
        faraday_guide::block::CrossCheck::Not(_) => Text::default(),
    }
}

fn lines(all: &[faraday_guide::block::Line]) -> FieldBody<'static> {
    FieldBody::Lines(all.iter().map(line).collect())
}

fn field<'s>(label: &'static str, body: FieldBody<'s>) -> Field<'s> {
    Field {
        label: Shown::plain(label),
        body,
    }
}

/// The How tab's blocks: the nine fields of each, in order, the Code entries and the last run
/// as `State` put them together when the page opened.
fn how_body(state: &State, s: &'static Strings, pane: bool) -> HowModel<'static> {
    let mut parts = Vec::new();
    let blocks = state.how_blocks();
    for (at, b) in blocks.iter().enumerate() {
        let release = state.release();
        let one = [faraday_guide::block::Line {
            text: b.action,
            kind: faraday_guide::block::Kind::Plain,
        }];
        let cross = match b.cross_check {
            faraday_guide::block::CrossCheck::Not(why) => how::Cross::Not(Shown::plain(why)),
            faraday_guide::block::CrossCheck::Area(a) => {
                let part = |label: &'static str, all: &[faraday_guide::block::Line]| {
                    (Shown::plain(label), all.iter().map(line).collect())
                };
                how::Cross::Area {
                    heading: Shown::plain(s.how_cc_area),
                    area: Shown::plain(a.area),
                    parts: vec![
                        part(s.how_cc_code, a.code),
                        part(s.how_cc_second, a.second),
                        part(s.how_cc_vectors, a.vectors),
                        part(s.how_cc_random, a.random),
                        part(s.how_cc_differences, a.differences),
                        part(s.how_cc_when, a.when),
                        (
                            Shown::plain(s.how_cc_last_run),
                            vec![how::Line {
                                text: Shown::made(run(b, s)),
                                kind: Kind::Mono,
                            }],
                        ),
                    ],
                }
            }
        };
        let mut code: Vec<how::Line<'static>> = b
            .code
            .iter()
            .map(|c| how::Line {
                text: Shown::made(Text::fill(s.how_code_entry, &[release, c.file, c.function])),
                kind: Kind::Mono,
            })
            .collect();
        code.push(how::Line {
            text: Shown::plain(s.how_repository),
            kind: Kind::Mono,
        });
        parts.push(Part::Block(HowBlock {
            name: Shown::plain(b.name),
            fields: vec![
                field(s.how_field_action, lines(&one)),
                field(s.how_field_standards, lines(b.standards)),
                field(s.how_field_implementation, lines(b.implementation)),
                field(s.how_field_checked, lines(b.checked)),
                field(
                    s.how_field_crosscheck,
                    FieldBody::Cross {
                        independence: b.independence.iter().map(line).collect(),
                        outcome: cross,
                    },
                ),
                Field {
                    label: Shown::plain(s.how_field_code),
                    body: FieldBody::Lines(code),
                },
                field(s.how_field_secret, lines(b.secret)),
                field(s.how_field_limits, lines(b.limits)),
                field(s.how_field_own_calls, lines(b.own_calls)),
            ],
        }));
        for c in faraday_guide::how::constructions_after_block(&blocks, at) {
            parts.push(Part::Construction(construction(c)));
        }
    }
    HowModel {
        parts,
        look: guide::how_look(pane),
    }
}

impl Present for GuideSheet {
    type Model<'s> = Model<'s>;

    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        if !state.open() {
            return Model {
                guide: None,
                pane: None,
            };
        }
        let s = show.strings();
        let pane = state.pane_shown() && !state.sheet();
        let room = state.room();
        let (look, head) = if pane {
            guide::pane_look()
        } else {
            guide::sheet_look()
        };
        let how_on = state.how_on();
        let tabs = state.how_offered().then(|| TabsModel {
            kind: TabKind::Primary,
            items: vec![
                Tab {
                    label: Shown::plain(s.guide_tab_guide),
                    on: !how_on,
                    done: false,
                    offered: gate(state, view, Press::TabGuide),
                },
                Tab {
                    label: Shown::plain(s.guide_tab_how),
                    on: how_on,
                    done: false,
                    offered: gate(state, view, Press::TabHow),
                },
            ],
        });
        let shown_page = state.shown_page();
        let title = shown_page
            .filter(|_| !how_on)
            .map(|p| Shown::plain(p.title.as_str()));
        let body = if how_on {
            SheetBody::How(how_body(state, s, pane))
        } else if state.contents() {
            contents(state, view, s, look)
        } else if let Some(p) = shown_page {
            SheetBody::Page(page(state, view, s, p, look))
        } else {
            SheetBody::Page(GuideBlockModel {
                look,
                body: GuideBody::Contents {
                    groups: Vec::new(),
                    none: None,
                },
            })
        };
        let contents_button = (!state.contents() && !how_on)
            .then(|| button(state, view, s.guide_contents, Style::Text, Press::Contents));
        let done = (!room.held && !pane)
            .then(|| button(state, view, s.guide_done, Style::Tonal, Press::Close));
        Model {
            guide: Some(GuideSheetModel {
                held: room.held,
                searching: room.compact && !room.held && state.contents() && state.find_focus(),
                tabs,
                title,
                head,
                body,
                contents: contents_button,
                done,
                beside: (!room.held && !pane).then_some(Press::Close),
                scroll: SCROLL,
                target: Some(TARGET),
            }),
            pane: pane.then(|| PaneModel {
                close: Shown::plain(s.guide_close),
            }),
        }
    }
}
