use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::nav::{Raise, SearchBarModel, WaysModel};
use faraday_organisms::session_pane::SessionPaneModel;
use faraday_organisms::{ButtonModel, Icon, Offered, Shown, Style};
use faraday_present::{Present, offer};
use faraday_templates::menu_page::{Body, Face, Mark, MenuPageModel, Section};
use faraday_templates::panes::{Left, Narrow, Panes, Tile};
use faraday_templates::screen::BarSearch;

use faraday_strings::Strings;
use faraday_text::Text;

use crate::lead::{Lead, jobs, make, nothing_loaded, pick, small_panel, wallets};
use crate::{Home, Model, Msg, Place, Press, Refusal, SCROLL, State};

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
        let small = small_panel(&view.facts.display);
        // The Pi's Home has no search field: its head, lead, places and Scan are all there is
        // (`main`'s Pi menu model); the wide Home's search is the bar's.
        let row = (!small).then(|| BarSearch {
            field: SearchBarModel {
                typed: Shown::plain(state.find()),
                hint: Shown::plain(s.home_search_hint),
                focused: state.focused(),
                here: Press::Focus,
                clear: (Press::Clear, Shown::plain(s.vault_search_clear)),
                raise: Some(Raise::Level),
            },
            ways: Some(ways(state, view, s)),
        });
        let mut waiting = jobs(view, s);
        let typed = !state.typed().trim().is_empty();
        let (above, sections) = if typed {
            (None, found(state, view, show))
        } else if small {
            // The Pi's lead falls back to Make a new wallet, the last job.
            waiting.push(make(s));
            let lead = lead_card(state, view, waiting);
            (lead, places(state, view, show))
        } else {
            let lead = lead_card(state, view, waiting);
            (None, wide(state, view, show, lead))
        };
        Model {
            page: MenuPageModel {
                scroll: SCROLL,
                row,
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
                above,
                results: None,
                details: None,
                on_placed: None,
                glide: None,
                tile_scroll: None,
                own: None,
            },
        }
    }
}

/// Home's first actions beside the search: New wallet, Restore and Spend from backup, tonal
/// with their icons, and the one Start button that stands for them where they do not fit.
fn ways<'s>(state: &State, view: &View<'s>, s: &'s Strings) -> WaysModel<'s, Press> {
    let button = |icon: Icon, label: &'static str, press: Press, style: Style| ButtonModel {
        label: Shown::plain(label),
        style,
        offered: gate(state, view, press),
        chosen: false,
        icon: Some(icon),
    };
    WaysModel {
        pills: vec![
            button(Icon::Flag, s.home_ways_new_short, Press::Make, Style::Tonal),
            button(
                Icon::Wallet,
                s.home_ways_restore_short,
                Press::Restore,
                Style::Tonal,
            ),
            button(
                Icon::Sign,
                s.home_ways_spend_short,
                Press::SpendBackup,
                Style::Tonal,
            ),
        ],
        start: ButtonModel {
            label: Shown::plain(s.home_ways_start),
            style: Style::Filled,
            offered: gate(state, view, Press::Make),
            chosen: false,
            icon: None,
        },
        pills_ok: true,
        raise: Some(Raise::Level),
    }
}

/// The lead as a block across the page: its icon's row in the selected tone, the card as the
/// press.
fn lead_card<'s>(state: &State, view: &View<'s>, jobs: Vec<Lead>) -> Option<Content<'s, Press>> {
    let lead = pick(jobs, state.shown())?;
    Some(Content(vec![Block::Rows(vec![
        faraday_organisms::RowModel {
            headline: Shown::made(lead.line),
            supporting: Shown::made(lead.under),
            trailing: None,
            offered: gate(state, view, lead.press),
            chosen: true,
            whole: false,
            chevron: true,
        },
    ])]))
}

/// The wide Home's sections: the lead, the wallets as cards and, while nothing is loaded, the
/// three first actions.
fn wide<'s>(
    state: &State,
    view: &View<'s>,
    show: &Show<'s>,
    lead: Option<Content<'s, Press>>,
) -> Vec<Section<'s, Press>> {
    let mut sections = Vec::new();
    if let Some(lead) = lead {
        sections.push(Section {
            label: None,
            body: Body::Block(lead),
        });
    }
    let cards = wallet_cards(state, view, show);
    if !cards.is_empty() {
        sections.push(Section {
            label: None,
            body: Body::Tiles(cards),
        });
    }
    sections.extend(first_run(state, view, show));
    sections
}

/// The session's wallets as tiles: the name and the one line that matters now, a warning where
/// the line is something to do.
fn wallet_cards<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Vec<Face<'s, Press>> {
    wallets(view, show.strings())
        .into_iter()
        .map(|card| Face {
            mark: Mark::Beside(Icon::Wallet.into()),
            name: Shown::made(card.name),
            line: Shown::made(card.line),
            warn: card.needs,
            tags: Vec::new(),
            offered: gate(state, view, Press::Place(Place::Wallets)),
        })
        .collect()
}

/// The places the Pi's Home lists, in the panel's order: Wallets, Check, Files, Vaults, Tools,
/// Guide and Settings, each with what it holds.
fn places<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Vec<Section<'s, Press>> {
    let s = show.strings();
    let session = view.session;
    let count = |n: usize| match n {
        0 => Shown::plain(""),
        n => Shown::made(Text::of(&Text::num(n as u64))),
    };
    let tile = |icon: Icon, name: &'static str, line: Shown<'s>, press: Press| Face {
        mark: Mark::Beside(icon.into()),
        name: Shown::plain(name),
        line,
        warn: false,
        tags: Vec::new(),
        offered: gate(state, view, press),
    };
    let files = Text::of(&format!(
        "{} \u{b7} {}",
        session.inbox().len(),
        session.outbox().len()
    ));
    vec![Section {
        label: None,
        body: Body::Tiles(vec![
            tile(
                Icon::Wallet,
                s.compact_wallets,
                count(session.wallets().len()),
                Press::Place(Place::Wallets),
            ),
            tile(
                Icon::Verified,
                s.compact_check,
                Shown::plain(""),
                Press::Place(Place::Check),
            ),
            tile(
                Icon::File,
                s.compact_files,
                Shown::made(files),
                Press::Place(Place::Files),
            ),
            tile(
                Icon::Lock,
                s.compact_vaults,
                if view.vaults.current().is_some() {
                    Shown::made(Text::fill(s.compact_open, &["1"]))
                } else {
                    Shown::plain("")
                },
                Press::Place(Place::Vaults),
            ),
            tile(
                Icon::Tools,
                s.compact_tools,
                Shown::plain(""),
                Press::Place(Place::Tools),
            ),
            tile(Icon::Help, s.compact_guide, Shown::plain(""), Press::Guide),
            tile(
                Icon::Settings,
                s.compact_settings,
                Shown::plain(""),
                Press::Place(Place::Settings),
            ),
        ]),
    }]
}

/// Nothing loaded: the three first actions, the first of them Make a new wallet.
fn first_run<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Vec<Section<'s, Press>> {
    if !nothing_loaded(view) {
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
    vec![Section {
        label: None,
        body: Body::Tiles(vec![
            face(Icon::Flag, s.home_ways_make_a_new, "", Press::Make),
            face(Icon::Wallet, s.home_ways_restore_wallet, "", Press::Restore),
            face(Icon::Sign, s.home_ways_spend_backup, "", Press::SpendBackup),
        ]),
    }]
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
