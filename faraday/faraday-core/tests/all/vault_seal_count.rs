//! A vault counts how many times its slot was sealed, inside the
//! encryption (`docs/VAULT.md` §11). The count is shown at
//! unlock and on the open vault's head; an unlock of a copy with a lower
//! count than one seen since power-on stops at a dialog that says it is
//! older, and opens only on **Open**; and a backup plan in one vault
//! records the counts of the others, so an older copy is named even after
//! the computer was off.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

type Files = Vec<(String, Vec<u8>)>;

/// A device with these files restored, the clock known.
fn process(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: faraday_core::files_from(outbox),
        kept: faraday_core::kept_from(kept),
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// Unlocks the vault file at place `at` of the Vaults list.
fn unlock(app: &mut Faraday, at: usize, phrase: &str) {
    let before = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| a.vaults().open.len() > before);
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "the vault did not open"
    );
}

/// The unlock just made stopped at the older-copy dialog: it says what
/// the count found, and **Open** goes on with the unlock.
fn open_anyway(app: &mut Faraday, found: &str) {
    assert_eq!(app.sheet, Some(Sheet::OlderCopy), "no older-copy dialog");
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(said(&texts, "Open this copy?"), "{texts:?}");
    assert!(said(&texts, found), "{texts:?}");
    app.press(Action::Vault(V::OlderOpen));
    assert_ne!(app.sheet, Some(Sheet::OlderCopy));
}

/// What the open vault's page says.
fn head(app: &mut Faraday) -> Vec<String> {
    // The Load sheet a vault with a key opens under is put away.
    if app.sheet.is_some() {
        app.press(Action::Cancel);
    }
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    app.drawn_texts()
}

fn said(texts: &[String], part: &str) -> bool {
    texts.iter().any(|t| t.contains(part))
}

/// What a lock saved: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = (
                faraday_core::testkit::plain_files(inbox),
                faraday_core::testkit::plain_files(outbox),
                faraday_core::testkit::plain_kept(kept),
            );
        }
    }
    last
}

/// A vault file named `label` with one passphrase, seal count 1.
fn made(label: &str, phrase: &str, seed: u8) -> Vec<u8> {
    crate::common::vault(label, phrase, seed)
}

/// `file` sealed once more under `nonce`, as another place would: the
/// next count.
fn resealed(file: &[u8], phrase: &str, nonce: u8) -> Vec<u8> {
    testkit::kdbx_reseal(file, phrase, nonce).unwrap()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The identifier of the vault `made` gives `id`, as a plan names it.
fn id_hex(id: u8) -> String {
    hex(&[id; 16])
}

/// The plan line that records `count` and the file's digest for vault `id`.
fn seals_line(id: u8, count: u64, file: &[u8]) -> String {
    format!(
        "seals {} {count} {}\n",
        id_hex(id),
        hex(&faraday_crypto::sha256(file))
    )
}

/// A vault `phrase` opens, labelled `label`, with a plan for the Savings
/// wallet that holds `lines`.
fn with_lines(label: &str, phrase: &str, id: u8, lines: &str) -> Vec<u8> {
    let plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, testkit::savings().as_bytes())
        .with(field::PLAN_ANSWERS, format!("seeds 1\n{lines}").as_bytes());
    crate::common::vault_of(label, phrase, id, vec![plan])
}

const A: &str = "a phrase";
const B: &str = "b phrase";

#[test]
fn a_new_vault_reads_sealed_once_and_each_seal_adds_one() {
    let original = made("Main", B, 1);
    let mut app = process(vec![("main.kdbx".into(), original)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(said(&texts, "Sealed once"), "{texts:?}");

    // Changed and locked: the next unlock reads two seals, with the time
    // by the clock that sealed it.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = process(inbox, outbox, kept);
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(said(&texts, "Sealed 2 times · last "), "{texts:?}");
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

/// A seal waits for five answers of the system's: a lock with none
/// seals nothing and the count stays.
#[test]
fn a_lock_with_too_few_answers_seals_nothing() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("main.kdbx"),
            made("Main", B, 1),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([0x40; 32])));
    unlock(&mut app, 0, B);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, outbox, _) = saved(&mut app);
    assert!(outbox.is_empty(), "nothing was sealed");
    assert_eq!(app.vaults().open.len(), 1, "the vault stays open");
    let texts = head(&mut app);
    assert!(said(&texts, "Sealed once"), "{texts:?}");
}

/// A process holding the first copy of a vault, unlocked, where a copy
/// sealed twice was open since power-on (what the last process kept).
fn older_unlocked() -> Faraday {
    let original = made("Main", B, 1);
    let mut app = process(
        vec![("main.kdbx".into(), original.clone())],
        Vec::new(),
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, _, kept) = saved(&mut app);

    // The next process, with the first copy and what the last kept.
    let mut app = process(vec![("main.kdbx".into(), original)], Vec::new(), kept);
    unlock(&mut app, 0, B);
    app
}

/// What the dialog says of it (wrapped at its width: its first part).
const OLDER: &str = "An older copy: sealed once.";

#[test]
fn a_seal_from_an_older_copy_goes_beside_the_newer_copy_waiting() {
    let add_key = |app: &mut Faraday, seed: usize| {
        app.press(Action::Entry(None));
        type_text(app, &testkit::test_words(testkit::TEST_SEEDS[seed].0));
        app.press(Action::EntryAdd);
        app.press(Action::Vault(V::SaveKey(0)));
        crate::common::top_up(app);
    };
    let original = made("Main", B, 1);
    let mut app = process(
        vec![("main.kdbx".into(), original.clone())],
        Vec::new(),
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    add_key(&mut app, 1);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, newer, _) = saved(&mut app);
    assert_eq!(newer.len(), 1);

    // A process that has seen only the newer copy, waiting in the Outbox.
    let mut app = process(Vec::new(), newer.clone(), Vec::new());
    unlock(&mut app, 0, B);
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, _, kept) = saved(&mut app);

    // The older copy read in: its own row, and it opens only on Open.
    let mut app = process(vec![("main.kdbx".into(), original)], newer.clone(), kept);
    unlock(&mut app, 1, B);
    open_anyway(&mut app, OLDER);
    add_key(&mut app, 2);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, outbox, _) = saved(&mut app);
    let mut names: Vec<&str> = outbox.iter().map(|f| f.0.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["main-2.kdbx", "main.kdbx"]);
    let waiting = outbox.iter().find(|f| f.0 == "main.kdbx").unwrap();
    assert_eq!(waiting.1, newer[0].1, "the newer copy was replaced");
}

#[test]
fn an_older_copy_opened_after_a_newer_one_in_one_power_on_says_so() {
    let mut app = older_unlocked();
    open_anyway(&mut app, OLDER);
    let texts = head(&mut app);
    assert!(said(&texts, "An older copy: sealed once"), "{texts:?}");
    assert!(
        said(&texts, "A copy sealed 2 times was open since power-on"),
        "{texts:?}"
    );
    // Warned and confirmed, not refused: the vault is open.
    assert_eq!(app.vaults().open.len(), 1);

    // Power off: nothing is kept, and the older copy opens with no
    // warning, which is what a count can detect (VAULT.md §11).
    let original = made("Main", B, 1);
    let mut app = process(vec![("main.kdbx".into(), original)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

#[test]
fn an_older_copy_stops_at_a_dialog_and_opens_only_on_open() {
    let mut app = older_unlocked();
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    let _ = app.frame();
    let texts = app.drawn_texts();
    for part in ["Open this copy?", "Main", OLDER, "Keep locked", "Open"] {
        assert!(said(&texts, part), "{part}: {texts:?}");
    }
    // Not open yet: the unlock has not landed and nothing says it did.
    assert_eq!(app.screen, Screen::Unlock);
    assert!(!app.toast_text().is_some_and(|t| t.contains("unlocked")));
    // A tick does not open it either.
    settle(&mut app, |_| false);
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    assert_eq!(app.screen, Screen::Unlock);

    app.press(Action::Vault(V::OlderOpen));
    assert_eq!(app.sheet, None);
    assert_ne!(app.screen, Screen::Unlock, "the unlock landed");
    assert!(
        app.toast_text().is_some_and(|t| t.contains("unlocked")),
        "{:?}",
        app.toast_text()
    );
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn keep_locked_or_escape_leaves_the_older_copy_locked() {
    // Escape on a sheet is its Cancel.
    for way in [Action::Vault(V::OlderBack), Action::Cancel] {
        let mut app = older_unlocked();
        app.press(way);
        assert!(app.vaults().open.is_empty(), "{way:?}: still open");
        assert_eq!(app.sheet, None, "{way:?}");
        assert_eq!(app.screen, Screen::Unlock, "{way:?}");
        let _ = app.frame();
        let texts = app.drawn_texts();
        assert!(!said(&texts, "Open this copy?"), "{texts:?}");
        // Unlocked again, it stops at the dialog again.
        unlock(&mut app, 0, B);
        assert_eq!(app.sheet, Some(Sheet::OlderCopy), "{way:?}");
    }
}

#[test]
fn a_lock_while_the_dialog_shows_closes_the_older_copy_and_ends_the_dialog() {
    let mut app = older_unlocked();
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty(), "the lock closed it");
    assert_ne!(app.sheet, Some(Sheet::OlderCopy));
    settle(&mut app, |_| false);
    assert_ne!(app.sheet, Some(Sheet::OlderCopy));
    // Unlocked again, it stops at the dialog again, and Open opens it.
    unlock(&mut app, 0, B);
    open_anyway(&mut app, OLDER);
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn two_copies_sealed_from_one_copy_say_so() {
    let original = made("Main", B, 1);
    let one = resealed(&original, B, 0x51);
    let other = resealed(&original, B, 0x52);
    assert_ne!(one, other);
    // The first is open and locked unchanged: it is what the session saw.
    let mut app = process(vec![("one.kdbx".into(), one)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, _, kept) = saved(&mut app);
    let mut app = process(vec![("other.kdbx".into(), other)], Vec::new(), kept);
    unlock(&mut app, 0, B);
    open_anyway(&mut app, "Another copy was sealed 2 times too");
    let texts = head(&mut app);
    assert!(
        said(
            &texts,
            "Another copy was sealed 2 times too, with other changes"
        ),
        "{texts:?}"
    );
}

/// Vault A, holding a plan that records `file` as vault B's copy at
/// `count`.
fn plans_vault_and_b(count: u64, file: &[u8]) -> Files {
    let a = with_lines("Plans", A, 3, &seals_line(2, count, file));
    vec![("a.kdbx".into(), a)]
}

#[test]
fn a_plan_in_one_vault_names_an_older_copy_of_another_after_power_off() {
    let b_old = made("Savings", B, 2);
    let b_new = resealed(&b_old, B, 0x61);

    // Power-on: A (holding the plan that records B at 2) and B's older
    // copy; nothing else is kept.
    let mut inbox = plans_vault_and_b(2, &b_new);
    inbox.push(("b.kdbx".into(), b_old.clone()));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    open_anyway(&mut app, "Plans's backup plan records");
    let texts = head(&mut app);
    assert!(
        said(
            &texts,
            "An older copy: sealed once. Plans's backup plan records a copy sealed 2 times"
        ),
        "{texts:?}"
    );
    assert_eq!(app.vaults().open.len(), 2, "warned, not refused");

    // The older copy first, then the plan's vault: the warning appears
    // when the plan opens.
    let mut inbox = plans_vault_and_b(2, &b_new);
    inbox.push(("b.kdbx".into(), b_old));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 1, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");
    unlock(&mut app, 0, A);
    app.press(Action::Vault(V::Show(0)));
    let texts = head(&mut app);
    assert!(said(&texts, "An older copy: sealed once"), "{texts:?}");
}

#[test]
fn a_plan_that_records_the_copy_opened_says_nothing_and_one_with_other_bytes_says_so() {
    let b_old = made("Savings", B, 2);
    let b_new = resealed(&b_old, B, 0x61);

    // B current: nothing is said.
    let mut inbox = plans_vault_and_b(2, &b_new);
    inbox.push(("b.kdbx".into(), b_new.clone()));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    let texts = head(&mut app);
    assert!(
        !said(&texts, "older copy") && !said(&texts, "too, with other"),
        "{texts:?}"
    );

    // B at the plan's count with other bytes.
    let b_alt = resealed(&b_old, B, 0x62);
    let mut inbox = plans_vault_and_b(2, &b_new);
    inbox.push(("b.kdbx".into(), b_alt));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    open_anyway(&mut app, "sealed 2 times too");
    let texts = head(&mut app);
    assert!(said(&texts, "sealed 2 times too"), "{texts:?}");

    // An older copy of the plan's vault carries an older line: no warning
    // on B, the limit `docs/VAULT.md` §11 states.
    let old_line = plans_vault_and_b(1, &b_old);
    let mut inbox = old_line;
    inbox.push(("b.kdbx".into(), b_new));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

#[test]
fn a_copy_in_files_that_was_never_opened_is_not_compared() {
    let original = made("Main", B, 1);
    let newer = resealed(&original, B, 0x71);
    let other = resealed(&original, B, 0x72);

    // The older copy opens with a newer one unopened in Files beside it:
    // no dialog and nothing said (`docs/VAULT.md` §11).
    let mut app = process(
        vec![("newer.kdbx".into(), newer.clone())],
        vec![("main.kdbx".into(), original.clone())],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    assert_eq!(app.sheet, None);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");

    // The newer copy opens with the older beside it: nothing is said.
    let mut app = process(
        vec![("main.kdbx".into(), original)],
        vec![("newer.kdbx".into(), newer.clone())],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");

    // Another copy of the same count, unopened in Files: nothing is said.
    let mut app = process(
        vec![("other.kdbx".into(), other)],
        vec![("newer.kdbx".into(), newer)],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    assert_eq!(app.sheet, None);
    let texts = head(&mut app);
    assert!(!said(&texts, "Another copy"), "{texts:?}");
}

/// A vault `phrase` opens that keeps a plan for the Savings wallet.
fn with_plan(label: &str, phrase: &str, id: u8) -> Vec<u8> {
    with_lines(label, phrase, id, "")
}

/// The `seals` lines of the plan in the vault `phrase` opens in `file`.
fn plan_lines(file: &[u8], phrase: &str) -> Vec<String> {
    let c = testkit::compute();
    let mut store = faraday_keys::vault::OpenVaults::default();
    let (vid, _) = store.unlock(&c, file.to_vec(), phrase).unwrap();
    store
        .contents_for_tests(vid)
        .iter()
        .filter(|r| r.kind == kind::PLAN)
        .flat_map(|r| {
            let text = r
                .fields
                .iter()
                .find(|f| f.number == field::PLAN_ANSWERS)
                .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                .unwrap_or_default();
            text.lines()
                .filter(|l| l.starts_with("seals "))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn a_plan_placing_two_vaults_records_each_for_the_other_at_the_lock() {
    let a = with_plan("Plans A", A, 5);
    let b = with_plan("Plans B", B, 6);
    let mut app = process(
        vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)],
        Vec::new(),
        Vec::new(),
    );
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let (_, outbox, _) = saved(&mut app);
    assert_eq!(outbox.len(), 2, "both were sealed");
    let find = |phrase: &str| {
        outbox
            .iter()
            .find(|(_, f)| {
                faraday_keys::vault::OpenVaults::default()
                    .unlock(&testkit::compute(), f.clone(), phrase)
                    .is_ok()
            })
            .map(|(_, f)| f.clone())
            .expect("the vault was sealed")
    };
    let (a2, b2) = (find(A), find(B));
    let in_a = plan_lines(&a2, A);
    let in_b = plan_lines(&b2, B);
    assert_eq!(in_a.len(), 1, "{in_a:?}");
    assert_eq!(in_b.len(), 1, "{in_b:?}");
    assert!(
        in_a[0].starts_with(&format!("seals {} ", id_hex(6))),
        "{in_a:?}"
    );
    assert!(
        in_b[0].starts_with(&format!("seals {} ", id_hex(5))),
        "{in_b:?}"
    );
    // The vault sealed first took the other's count as it was opened (c);
    // the one sealed second took the first's after its seal (c + 1).
    let at = |file: &[u8]| outbox.iter().position(|(_, f)| f == file).unwrap();
    let (first, second) = if at(&a2) < at(&b2) {
        (&in_a, &in_b)
    } else {
        (&in_b, &in_a)
    };
    let count = |lines: &[String]| lines[0].split(' ').nth(2).unwrap().parse::<u64>().unwrap();
    assert_eq!(count(second), count(first) + 1, "{in_a:?} {in_b:?}");
}
