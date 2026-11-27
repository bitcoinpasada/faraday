use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::nav::{Raise, SearchBarModel};
use faraday_organisms::session_pane::SessionPaneModel;
use faraday_organisms::{Icon, Offered, Shown};
use faraday_present::{Present, offer};
use faraday_templates::menu_page::{Body, Face, Mark, MenuPageModel, Section};
use faraday_templates::panes::{Left, Narrow, Panes, Tile};
use faraday_templates::screen::BarSearch;

use crate::{Home, Model, Msg, Press, Refusal, SCROLL, State};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        match *self {}
    }
}

impl Refused for Refusal {}

/// The offered press: the one gate decides (04 V11).
fn gate(state: &State, view: &View<'_>, press: Press) -> Offered<Press> {
    offer::<Press, Home>(state, &Msg::Press(press.clone()), press, view)
}

impl Present for Home {
    type Model<'s> = Model<'s>;

    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        let s = show.strings();
        let row = BarSearch {
            field: SearchBarModel {
                typed: Shown::plain(state.find()),
                hint: Shown::plain(s.home_search_hint),
                focused: state.focused(),
                here: Press::Focus,
                clear: (Press::Clear, Shown::plain(s.vault_search_clear)),
                raise: Some(Raise::Level),
            },
            ways: None,
        };
        let sections = if state.typed().trim().is_empty() {
            first_run(state, view, show)
        } else {
            found(state, view, show)
        };
        Model {
            page: MenuPageModel {
                scroll: SCROLL,
                row: Some(row),
                most_columns: 2,
                fixed_columns: None,
                bar_still: false,
                panes: Some(Panes {
                    left: Left::Session(SessionPaneModel::empty(
                        Shown::plain(s.home_box_heading),
                        Shown::plain(s.session_nothing_loaded),
                        false,
                    )),
                    tile: Tile::Fill,
                    // The stacked arm is not built.
                    stacks: false,
                    narrow: Narrow::OneColumn,
                }),
                sections,
            },
        }
    }
}

/// Whether the session holds a key or a wallet.
fn something_loaded(view: &View<'_>) -> bool {
    view.session.keys().all().next().is_some() || !view.session.wallets().is_empty()
}

/// Nothing loaded: the lead (Make a new wallet, the first job when none other waits) and the
/// three first actions. Wallet cards and the other jobs are not built here.
fn first_run<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Vec<Section<'s, Press>> {
    if something_loaded(view) {
        return Vec::new();
    }
    let s = show.strings();
    let face = |icon: Icon, name: &'static str, line: &'static str, press: Press| Face {
        mark: Mark::Beside(icon.into()),
        name: Shown::plain(name),
        line: Shown::plain(line),
        warn: false,
        tags: Vec::new(),
        offered: gate(state, view, press),
    };
    vec![
        Section {
            label: None,
            body: Body::Tiles(vec![face(
                Icon::Wallet,
                s.home_lead_make_a_new,
                s.home_lead_single_key,
                Press::Make,
            )]),
        },
        Section {
            label: Some(Shown::plain(s.home_ways_start)),
            body: Body::Tiles(vec![
                face(Icon::Flag, s.home_ways_make_a_new, "", Press::Make),
                face(Icon::Wallet, s.home_ways_restore_wallet, "", Press::Restore),
                face(Icon::Sign, s.home_ways_spend_backup, "", Press::SpendBackup),
            ]),
        },
    ]
}

/// The guide words the typed words found, each a row whose press opens the guide at it, or the
/// line saying nothing matched.
fn found<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Vec<Section<'s, Press>> {
    let s = show.strings();
    let glossary = faraday_guide::glossary();
    let rows: Vec<_> = state
        .found()
        .iter()
        .filter_map(|&at| {
            let word = glossary.get(usize::try_from(at).ok()?)?;
            Some(faraday_organisms::RowModel {
                headline: Shown::plain(word.word),
                supporting: Shown::plain(""),
                trailing: None,
                offered: gate(state, view, Press::Word(at)),
                chosen: false,
                whole: false,
                chevron: true,
            })
        })
        .collect();
    let blocks = if rows.is_empty() {
        vec![Block::Heading(Shown::plain(s.catalog_no_tool_matches))]
    } else {
        vec![
            Block::Heading(Shown::plain(s.guide_words_found)),
            Block::Rows(rows),
        ]
    };
    vec![Section {
        label: None,
        body: Body::Block(Content(blocks)),
    }]
}
