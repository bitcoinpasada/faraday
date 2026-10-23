//! A screen's frame does the same work whatever the Inbox holds and
//! however many wallets are loaded (`docs/SPEED.md` §8): what a frame
//! reads of each file and each wallet is kept, so drawing does not grow
//! with them. One copy of the test stick's files against five, one
//! wallet against every test wallet, with a spend open and without. A
//! frame also draws as many lines of text with five copies of the files
//! as with one: a list draws the rows that can be seen, not the rest
//! (`docs/SPEED.md` §9), and works out nothing of what the Inbox gives
//! (`found_in`, kept with the files) unless the files or the session
//! changed.

use super::{frame_work, hover_work};
use crate::work::{Counts, KINDS};
use crate::{Action, Faraday, Screen, StorageEvent, testkit};

/// A session with `copies` copies of the test stick's files in the Inbox
/// and the first `wallets` test wallets loaded (Savings first), with the
/// first two test seeds; Savings' spend open when `spend` is.
fn session(copies: usize, wallets: usize, spend: bool) -> Faraday {
    let c = crate::testkit::compute();
    let base = testkit::files().expect("the test stick's files");
    let mut inbox = Vec::new();
    for c in 0..copies {
        for (n, b) in &base {
            let name = if c == 0 {
                n.clone()
            } else {
                format!("{c}-{n}")
            };
            inbox.push((name, b.clone()));
        }
    }
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    for (word, l) in &testkit::TEST_SEEDS[..2] {
        let _ = app
            .session
            .keys
            .add_words(&c, &testkit::test_words(word), l, None);
    }
    let mut kits = testkit::kits();
    kits.sort_by_key(|k| k.id != "savings");
    for kit in kits.iter().take(wallets) {
        app.session
            .add_wallet(
                &c,
                kit.name,
                &kit.descriptor,
                "test",
                crate::wallet::Origin::Named,
            )
            .expect("a test wallet");
    }
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    if spend {
        let psbt = app
            .inbox
            .iter()
            .position(|i| i.name == "savings-unsigned.psbt")
            .expect("the spend is in the Inbox");
        app.press(Action::StartSpend(psbt));
        assert_eq!(app.screen, Screen::Spend, "the spend opened");
    }
    app
}

/// The screens compared, by name, each with how it is reached.
const SCREENS: [(&str, Action); 6] = [
    ("Home", Action::Nav(Screen::Home)),
    ("Wallets", Action::Nav(Screen::Wallets)),
    ("Savings' card", Action::OpenWallet(0)),
    ("Files", Action::Nav(Screen::Files)),
    ("Spend tab", Action::Nav(Screen::Family)),
    ("Spend", Action::Nav(Screen::Spend)),
];

/// Each screen's frame drawn again, and a pointer moved over it; and
/// the lines of text a frame of it draws.
fn works(app: &mut Faraday) -> Vec<(&'static str, Counts, Counts, u64, u64)> {
    SCREENS
        .iter()
        .map(|&(name, a)| {
            app.press(a);
            let frame = frame_work(app);
            let lines = lines_drawn(app);
            let before = app.inbox.rebuilds();
            app.out.dirty = true;
            let _ = faraday_shell_api::App::frame(app);
            let rebuilt = app.inbox.rebuilds() - before;
            (name, frame, hover_work(app), lines, rebuilt)
        })
        .collect()
}

/// The lines of text one frame of the screen at rest draws.
fn lines_drawn(app: &mut Faraday) -> u64 {
    use faraday_shell_api::App;
    let count = |app: &Faraday| app.canvas.as_ref().map_or(0, |c| c.lines_drawn());
    app.settle();
    let _ = app.frame();
    let before = count(app);
    app.out.dirty = true;
    let _ = app.frame();
    count(app) - before
}

fn text(c: &Counts) -> String {
    KINDS
        .iter()
        .filter(|k| c.of(**k) > 0)
        .map(|k| format!("{k:?} {}", c.of(*k)))
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn a_frame_does_the_same_work_with_five_times_the_files_and_every_wallet() {
    let all = testkit::kits().len();
    assert!(all >= 10, "the test wallets: {all}");
    for spend in [false, true] {
        let mut base = session(1, 1, spend);
        let base_work = works(&mut base);
        for (copies, wallets) in [(5, 1), (1, all), (5, all)] {
            let mut app = session(copies, wallets, spend);
            assert_eq!(app.inbox.len(), copies * base.inbox.len());
            let work = works(&mut app);
            assert_eq!(work.len(), base_work.len());
            for ((name, frame, hover, lines, rebuilt), (_, f1, _, l1, _)) in
                work.iter().zip(&base_work)
            {
                assert_eq!(
                    text(frame),
                    text(f1),
                    "{name}, spend open {spend}: a frame with {copies}x the files and {wallets} wallets did other work than with 1x and one"
                );
                assert_eq!(text(hover), "", "{name}: a pointer move did work");
                assert_eq!(
                    *rebuilt, 0,
                    "{name}, spend open {spend}: a frame with {copies}x the files worked out what the Inbox gives again"
                );
                if wallets == 1 {
                    assert_eq!(
                        lines, l1,
                        "{name}, spend open {spend}: a frame with {copies}x the files drew other lines of text than with 1x"
                    );
                }
            }
        }
    }
}

/// The boot stick's import sheet, the stick holding `copies` copies of
/// the test stick's files and the test vault, every read answered.
fn boot_import(copies: usize) -> Faraday {
    use crate::{StorageCommand, StorageEvent};
    use faraday_shell_api::App;
    let base = testkit::files().expect("the test stick's files");
    let mut files = Vec::new();
    for c in 0..copies {
        for (n, b) in &base {
            let name = if c == 0 {
                n.clone()
            } else {
                format!("{c}-{n}")
            };
            files.push((name, b.clone()));
        }
    }
    files.push((
        "vault.ofv".into(),
        testkit::test_vault().expect("the test vault"),
    ));
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app.storage(StorageEvent::Sticks(vec![crate::StickInfo {
        serial: None,
        id: "boot".into(),
        label: "boot".into(),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files
                    .iter()
                    .find(|(f, _)| *f == name)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                app.storage(StorageEvent::Read { stick, name, bytes });
            }
            StorageCommand::ReadQr { name, .. } => app.storage(StorageEvent::QrRead {
                name,
                bytes: Vec::new(),
                payloads: Vec::new(),
            }),
            _ => {}
        }
    }
    let _ = app.frame();
    assert_eq!(
        app.sheet,
        Some(crate::Sheet::Import),
        "the import sheet is up"
    );
    app
}

#[test]
fn the_boot_import_sheets_frame_does_the_same_work_with_five_times_the_files() {
    let mut one = boot_import(1);
    let mut five = boot_import(5);
    let (f1, f5) = (frame_work(&mut one), frame_work(&mut five));
    assert_eq!(
        text(&f1),
        text(&f5),
        "the import sheet's frame grew with the files"
    );
    assert_eq!(
        text(&f1).replace("FullCanvas 1", ""),
        "",
        "the import sheet's frame works out nothing but its kept backdrop"
    );
    assert_eq!(text(&hover_work(&mut five)), "", "a pointer move did work");
}
