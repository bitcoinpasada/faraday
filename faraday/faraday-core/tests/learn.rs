//! The ? on a screen opens OpenSigner's Learn pages for what is on it: a
//! multisig being created opens Multisig, a new key opens Randomness, and
//! another tab shows the next page.

use faraday_core::{Action, Faraday, Sheet};

fn shown(app: &Faraday) -> &'static str {
    app.learn.pages[app.learn.page].title
}

#[test]
fn the_question_mark_opens_the_pages_for_the_screen() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Multisig");
    app.press(Action::LearnPage(1));
    assert_eq!(shown(&app), "Xpubs and privacy");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);

    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::Learn);
    assert_eq!(shown(&app), "Randomness");
}

// The guides to starting a computer from the stick (docs/HUB.md §7.2),
// which the hub app's screens open by id: each id finds its page, and a
// guide puts what has to be in hand before any firmware step first.

fn guide(id: &str) -> &'static osk_learn::Page {
    faraday_core::learn::page(id).unwrap_or_else(|| panic!("no page {id}"))
}

#[test]
fn every_guide_the_hub_links_to_is_there() {
    let titles: Vec<&str> = faraday_core::learn::BOOT
        .iter()
        .map(|id| guide(id).title)
        .collect();
    assert_eq!(
        titles,
        [
            "Starting a Windows computer from the stick",
            "Starting a Linux computer from the stick",
            "BitLocker and starting from a stick",
            "What Secure Boot and BitLocker are",
            "Screens and devices that can record",
            "When the stick does not start",
            "Replacing Faraday on a stick",
            "Starting an Intel Mac from the stick",
        ]
    );
    assert!(faraday_core::learn::page("no-such-page").is_none());
}

#[test]
fn the_boot_entry_page_is_there() {
    assert_eq!(guide("boot-entry").title, "The Faraday boot entry on Linux");
}

#[test]
fn the_windows_guide_does_the_bitlocker_step_before_any_firmware_step() {
    let first = &guide("windows").sections[0];
    assert_eq!(first.heading, "BitLocker first");
    assert!(first.paragraphs[0].starts_with(
        "Do the BitLocker step in Faraday Starter on this computer before changing any firmware setting."
    ));
}

#[test]
fn the_linux_guide_asks_for_the_tpm_unlocked_disk_s_passphrase_first() {
    let first = &guide("linux").sections[0];
    assert_eq!(first.heading, "A disk unlocked by the TPM");
    assert!(first.paragraphs[0].contains("passphrase or recovery key"));
}

// The maker pages' firmware sketches (docs/HUB.md §7.8, F18) are drawn
// only by Faraday Starter and Faraday Companion: the desktop app, the
// stick and the Pi show the pages without them, and never print a
// marker.

#[test]
fn the_desktop_and_the_stick_never_print_a_sketch_marker() {
    let mut app = faraday_core::testkit::started();
    let mut markers = 0;
    for id in faraday_learn::ids() {
        let page = guide(id);
        markers += page
            .sections
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .filter(|p| faraday_learn::is_marker(p))
            .count();
        app.learn_with(vec![page]);
        app.learn.scroll = 0.0;
        loop {
            let texts = app.drawn_texts();
            assert!(
                !texts.iter().any(|t| t.trim_start().starts_with("::sketch")),
                "{id}"
            );
            assert!(!texts.iter().any(|t| t.contains("→ Disabled")), "{id}");
            if app.learn.scroll >= app.learn.max {
                break;
            }
            app.learn.scroll = (app.learn.scroll + 200.0).min(app.learn.max);
        }
        app.press(Action::Cancel);
    }
    assert!(markers > 0);
}
