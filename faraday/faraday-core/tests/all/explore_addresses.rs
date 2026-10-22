//! Explore's address list and its search (F76): the path's account's
//! receive and change addresses, checked against BIP-84, BIP-86, BIP-49
//! and BIP-44's addresses for "abandon … about" and against Explore's
//! own reading of each whole path; a whole address found at its index on
//! either chain, a part listing every address that holds it, a miss
//! saying how far it looked, a deep search spread over ticks, and the
//! way back from both pages.

use faraday_bip::keys::Network;
use faraday_core::explore::addresses::DEPTHS;
use faraday_core::explore::{Page, XAction};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Explore on "abandon … about" at preset `preset` on `network`, with
/// `page` open.
fn on_page(network: Option<Network>, preset: u8, page: Page) -> Faraday {
    let mut app = faraday_core::testkit::started();
    if let Some(n) = network {
        app.press(Action::Nav(Screen::Settings));
        app.press(Action::Network(n));
    }
    app.press(Action::Entry(None));
    for c in ABANDON.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app.press(Action::XPreset(preset));
    app.press(Action::X(XAction::Page(page)));
    app
}

/// The list's page on `change`'s chain from `from`.
fn listed(app: &mut Faraday, change: bool, from: u32) -> Vec<(u32, String)> {
    let c = faraday_core::testkit::compute();
    app.press(Action::X(XAction::Chain(change)));
    if let Some(e) = app.explore.as_mut() {
        e.from = from;
    }
    app.explore_list(&c)
}

/// The address at `change`/`index` as Explore's own reading of the whole
/// path works it out: the key derived at the path itself, not through
/// the account's chains.
fn read_at(app: &mut Faraday, change: bool, index: u32) -> String {
    let c = faraday_core::testkit::compute();
    let e = app.explore.as_mut().unwrap();
    let account: Vec<&str> = e.path.split('/').take(3).collect();
    let whole = format!("{}/{}/{index}", account.join("/"), u8::from(change));
    let saved = std::mem::replace(&mut e.path, whole);
    let a = app.explore_reading(&c).unwrap().address.clone();
    app.explore.as_mut().unwrap().path = saved;
    a
}

/// "abandon … about"'s addresses: BIP-84's, BIP-86's and BIP-49's own
/// test vectors, BIP-49's mainnet first address and BIP-44's as BIP-44
/// wallets derive them: (network, preset, change, index, address).
const VECTORS: &[(Option<Network>, u8, bool, u32, &str)] = &[
    (
        None,
        0,
        false,
        0,
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
    ),
    (
        None,
        0,
        false,
        1,
        "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g",
    ),
    (
        None,
        0,
        true,
        0,
        "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el",
    ),
    (
        None,
        1,
        false,
        0,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
    ),
    (
        None,
        1,
        false,
        1,
        "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh",
    ),
    (
        None,
        1,
        true,
        0,
        "bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7",
    ),
    (None, 2, false, 0, "37VucYSaXLCAsxYyAPfbSi9eh4iEcbShgf"),
    (
        Some(Network::Testnet),
        2,
        false,
        0,
        "2Mww8dCYPUpKHofjgcXcBCEGmniw9CoaiD2",
    ),
    (None, 3, false, 0, "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"),
    (None, 3, false, 1, "1Ak8PffB2meyfYnbXZR9EGfLfFZVpzJvQP"),
    (None, 3, true, 0, "1J3J6EvPrv8q6AC3VCjWV45Uf3nssNMRtH"),
];

#[test]
fn the_list_gives_the_published_addresses_of_each_account() {
    for &(network, preset, change, index, want) in VECTORS {
        let mut app = on_page(network, preset, Page::Addresses);
        let page = listed(&mut app, change, 0);
        assert_eq!(
            page.iter()
                .find(|(i, _)| *i == index)
                .map(|(_, a)| a.as_str()),
            Some(want),
            "preset {preset}, change {change}, index {index}"
        );
    }
}

#[test]
fn every_listed_address_is_the_one_at_its_whole_path() {
    for preset in 0..4 {
        let mut app = on_page(None, preset, Page::Addresses);
        for (change, from) in [(false, 0), (true, 0), (false, 2_147_483_000), (true, 95)] {
            let page = listed(&mut app, change, from);
            assert_eq!(page.len(), 10, "a page is ten addresses");
            for (i, a) in page {
                assert_eq!(
                    a,
                    read_at(&mut app, change, i),
                    "preset {preset}, {change}/{i}"
                );
            }
        }
    }
}

#[test]
fn the_list_follows_the_account_explored() {
    // The addresses kept are the account's: another script is another
    // list, never the one kept before.
    let mut app = on_page(None, 0, Page::Addresses);
    let segwit = listed(&mut app, false, 0);
    assert_eq!(segwit[0].1, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    app.press(Action::X(XAction::Page(Page::Path)));
    app.press(Action::XPreset(1));
    app.press(Action::X(XAction::Page(Page::Addresses)));
    let taproot = listed(&mut app, false, 0);
    assert_eq!(
        taproot[0].1,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
    );
    assert!(
        app.explore_kept() <= 10,
        "the SegWit addresses were dropped"
    );
    // Another index in the same account lists the same account.
    app.press(Action::X(XAction::Page(Page::Path)));
    app.press(Action::XIndex(5));
    app.press(Action::X(XAction::Page(Page::Addresses)));
    assert_eq!(listed(&mut app, false, 0), taproot);
    // A multisig account pays to no single key: no list.
    app.press(Action::X(XAction::Page(Page::Path)));
    app.press(Action::XPreset(4));
    assert!(app.explore_account().is_none());
}

#[test]
fn the_first_index_is_typed_and_paged() {
    let mut app = on_page(None, 0, Page::Addresses);
    for c in "1234".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(app.explore.as_ref().unwrap().from, 1234);
    app.press(Action::X(XAction::Step(1)));
    assert_eq!(app.explore.as_ref().unwrap().from, 1244);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.explore.as_ref().unwrap().from, 124);
    for _ in 0..20 {
        app.press(Action::X(XAction::Step(-1)));
    }
    assert_eq!(app.explore.as_ref().unwrap().from, 0);
}

/// Types `query`, picks the depth at `depth`, searches with Enter and
/// ticks until the search ends. Returns the ticks it took.
fn search(app: &mut Faraday, query: &str, depth: u8) -> u32 {
    for _ in 0..100 {
        app.event(Event::Key(Key::Backspace));
    }
    for c in query.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::X(XAction::Depth(depth)));
    app.event(Event::Key(Key::Enter));
    let mut ticks = 0;
    while app
        .explore
        .as_ref()
        .and_then(|e| e.search.as_ref())
        .is_some_and(|s| s.running())
    {
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        ticks += 1;
        assert!(ticks < 1_000_000, "the search ends");
    }
    ticks
}

/// What the search found: chain, index and address of each match, and
/// how many matched.
fn found(app: &Faraday) -> (Vec<(bool, u32, String)>, u32) {
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    let all = s
        .found
        .iter()
        .map(|f| (f.change, f.index, f.address.clone()))
        .collect();
    (all, s.matched)
}

/// The ranges the search recorded for what it found.
fn marks(app: &Faraday) -> Vec<std::ops::Range<usize>> {
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    s.found.iter().map(|f| f.mark.clone()).collect()
}

#[test]
fn a_whole_address_is_found_at_its_index_on_each_chain() {
    let mut list = on_page(None, 0, Page::Addresses);
    let receive_37 = listed(&mut list, false, 30)[7].1.clone();
    let change_640 = listed(&mut list, true, 640)[0].1.clone();
    let mut app = on_page(None, 0, Page::Find);
    let change_0 = "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el";
    search(&mut app, change_0, 0);
    assert_eq!(found(&app), (vec![(true, 0, change_0.into())], 1));
    search(&mut app, &receive_37, 0);
    assert_eq!(found(&app), (vec![(false, 37, receive_37.clone())], 1));
    // In capitals, as a bech32 address may be written.
    search(&mut app, &receive_37.to_ascii_uppercase(), 0);
    assert_eq!(found(&app), (vec![(false, 37, receive_37)], 1));
    // Past the first hundred, only a deeper search finds it.
    search(&mut app, &change_640, 0);
    assert_eq!(found(&app), (vec![], 0));
    search(&mut app, &change_640, 1);
    assert_eq!(found(&app), (vec![(true, 640, change_640)], 1));
}

#[test]
fn a_part_lists_every_address_that_holds_it() {
    let mut app = on_page(None, 0, Page::Addresses);
    let mut every = Vec::new();
    for from in (0..100).step_by(10) {
        for change in [false, true] {
            for (i, a) in listed(&mut app, change, from) {
                every.push((i, change, a));
            }
        }
    }
    // Index by index, receive before change, as the search goes.
    every.sort();
    let part = "x9";
    let want: Vec<(bool, u32, String)> = every
        .into_iter()
        .filter(|(_, _, a)| a.contains(part))
        .map(|(i, change, a)| (change, i, a))
        .collect();
    assert!(want.len() >= 2, "the part is in several addresses");
    app.press(Action::X(XAction::Page(Page::Find)));
    search(&mut app, part, 0);
    assert_eq!(found(&app), (want.clone(), want.len() as u32));
    let _ = app.frame();
}

#[test]
fn a_miss_says_how_far_it_looked() {
    let mut app = on_page(None, 0, Page::Find);
    // BIP-86's first address is none of BIP-84's account's.
    search(
        &mut app,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
        0,
    );
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    assert_eq!((s.matched, s.next, s.depth), (0, 100, 100));
    let _ = app.frame();
}

#[test]
fn the_deepest_search_runs_a_tick_at_a_time_and_can_stop() {
    let mut app = on_page(None, 0, Page::Find);
    for c in "zzzzzzzzzz".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::X(XAction::Depth(2)));
    app.press(Action::X(XAction::Search));
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    assert!(
        s.running() && s.next <= 4,
        "the first tick takes a few indices"
    );
    // Stop: the search ends where it is.
    app.press(Action::X(XAction::Search));
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    assert!(!s.running() && s.depth == s.next);
    // The whole of it, timed.
    let started = std::time::Instant::now();
    let ticks = search(&mut app, "zzzzzzzzzz", 2);
    eprintln!(
        "the first {} of each chain searched in {:?} over {ticks} ticks",
        DEPTHS[2],
        started.elapsed()
    );
    let s = app.explore.as_ref().unwrap().search.as_ref().unwrap();
    assert_eq!((s.matched, s.next), (0, DEPTHS[2]));
    assert!(ticks > 1, "the search was spread over ticks");
}

#[test]
fn a_paste_fills_the_field_and_a_secret_is_refused() {
    use faraday_shell_api::FileKind;
    let mut app = on_page(None, 0, Page::Find);
    app.clipboard = true;
    let change_0 = "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el";
    app.press(Action::X(XAction::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: format!("bitcoin:{change_0}?amount=1"),
    });
    let e = app.explore.as_ref().unwrap();
    assert_eq!((e.query.as_str(), e.note), (change_0, None));
    app.press(Action::X(XAction::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: ABANDON.into(),
    });
    let e = app.explore.as_ref().unwrap();
    assert_eq!(e.query, change_0);
    assert!(e.note.is_some(), "the words are refused and named");
}

#[test]
fn back_from_the_list_and_the_search_returns_to_the_path_as_it_was() {
    let mut app = on_page(None, 1, Page::Path);
    app.press(Action::XIndex(3));
    let path = app.explore.as_ref().unwrap().path.clone();
    app.press(Action::X(XAction::Page(Page::Addresses)));
    app.event(Event::Key(Key::Char('7')));
    let _ = app.frame();
    // The way back the page draws.
    assert!(app.offers(Action::X(XAction::Page(Page::Path))));
    app.press(Action::X(XAction::Page(Page::Path)));
    let e = app.explore.as_ref().unwrap();
    assert_eq!(
        (app.screen, e.page, &e.path),
        (Screen::Explore, Page::Path, &path)
    );
    // The list again where it was left.
    app.press(Action::X(XAction::Page(Page::Addresses)));
    assert_eq!(app.explore.as_ref().unwrap().from, 7);
    // Escape from the search is the same way back; a second leaves
    // Explore.
    app.press(Action::X(XAction::Page(Page::Find)));
    app.event(Event::Key(Key::Escape));
    let e = app.explore.as_ref().unwrap();
    assert_eq!(
        (app.screen, e.page, &e.path),
        (Screen::Explore, Page::Path, &path)
    );
    app.event(Event::Key(Key::Escape));
    assert_ne!(app.screen, Screen::Explore);
}

#[test]
fn on_the_small_panel_the_bar_goes_back_to_the_path() {
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let mut app = on_page(None, 0, Page::Find);
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    assert!(app.offers(Action::X(XAction::Page(Page::Path))));
}

const CHANGE_0: &str = "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el";

#[test]
fn a_found_address_records_where_the_match_lies() {
    let mut app = on_page(None, 0, Page::Find);
    // A part from the middle of an address that no other holds.
    let part = &CHANGE_0[20..27];
    search(&mut app, part, 0);
    assert_eq!(found(&app).1, 1);
    assert_eq!(marks(&app), vec![20..27]);
    assert_eq!(&CHANGE_0[marks(&app)[0].clone()], part);
    // Typed in capitals, a bech32 address still matches where it lies.
    search(&mut app, &part.to_ascii_uppercase(), 0);
    assert_eq!(marks(&app), vec![20..27]);
    // The whole address marks the whole address, in either case.
    search(&mut app, CHANGE_0, 0);
    assert_eq!(marks(&app), vec![0..CHANGE_0.len()]);
    search(&mut app, &CHANGE_0.to_ascii_uppercase(), 0);
    assert_eq!(marks(&app), vec![0..CHANGE_0.len()]);
}

/// The columns, from the left, of the pixels in `frame` drawn in the
/// accent (blue, whichever theme: the text and the rest of the row are
/// grey) that `baseline` does not have, as (first, last), or none.
fn accent_columns(frame: &[u8], baseline: &[u8], width: usize) -> Option<(usize, usize)> {
    let near = |p: &[u8]| p[2] > p[0].saturating_add(60);
    let mut cols = frame
        .chunks_exact(4)
        .zip(baseline.chunks_exact(4))
        .enumerate()
        .filter(|(_, (f, b))| near(f) && !near(b))
        .map(|(i, _)| i % width);
    let first = cols.next()?;
    Some((first, cols.fold(first, usize::max)))
}

#[test]
fn the_found_row_draws_the_matched_part_in_the_accent() {
    let mut app = on_page(None, 0, Page::Find);
    let drawn = |app: &mut Faraday| {
        let f = app.frame();
        (f.rgba.to_vec(), usize::from(f.width))
    };
    for c in CHANGE_0[20..27].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let (before, width) = drawn(&mut app);
    search(&mut app, &CHANGE_0[20..27], 0);
    let (part, _) = drawn(&mut app);
    search(&mut app, CHANGE_0, 0);
    let (whole, _) = drawn(&mut app);
    let (w0, w1) = accent_columns(&whole, &before, width).expect("the whole address is accent");
    let (p0, p1) = accent_columns(&part, &before, width).expect("the matched part is accent");
    // The part lies inside the address, and is a fraction of its width:
    // 7 of 42 characters, with the start and the end left in the text colour.
    assert!(p0 > w0 + (w1 - w0) / 4, "the start is not accent");
    assert!(p1 < w1 - (w1 - w0) / 4, "the end is not accent");
    assert!((p1 - p0) * 3 < w1 - w0, "only the matched characters");
}
