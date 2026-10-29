//! A vault counts how many times its slot was sealed, inside the
//! encryption (`docs/VAULT-HARDENING.md` §5). The count is shown at
//! unlock and on the open vault's head; a copy with a lower count than
//! one seen since power-on says it is older; a stick visit never writes
//! an older copy over a newer one; and a backup plan in one vault records
//! the counts of the others, so an older copy is named even after the
//! computer was off.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Record};

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
        inbox,
        outbox,
        kept,
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
            last = (inbox, outbox, kept);
        }
    }
    last
}

/// A vault file named `label` with one passphrase, seal count 1.
fn made(label: &str, phrase: &str, seed: u8) -> Vec<u8> {
    crate::common::vault(label, &[phrase], seed)
}

/// `file` sealed once more under `nonce`, as another place would: the
/// next count.
fn resealed(file: &[u8], phrase: &str, nonce: u8) -> Vec<u8> {
    let c = testkit::compute();
    let (mut opened, contents) = faraday_vault::open(&c, file, phrase).unwrap();
    opened
        .seal(
            file,
            &contents,
            &[nonce; faraday_vault::NONCE_LEN],
            Some(1_790_000_000),
        )
        .unwrap()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The SHA-256 of the slot `phrase` opens in `file`.
fn slot_digest(file: &[u8], phrase: &str) -> [u8; 32] {
    let c = testkit::compute();
    let (opened, _) = faraday_vault::open(&c, file, phrase).unwrap();
    let h = faraday_vault::read_header(file).unwrap();
    let at = faraday_vault::slot_range(h.slot_len, opened.position());
    faraday_crypto::sha256(&file[at])
}

/// The position the plan's line must name: the slot `phrase` opens.
fn position(file: &[u8], phrase: &str) -> usize {
    let c = testkit::compute();
    faraday_vault::open(&c, file, phrase).unwrap().0.position()
}

/// `with_plan_for`, naming the slot of `other` that `other_phrase` opens.
fn plan_naming(other: &[u8], other_phrase: &str, count: u64, digest: [u8; 32]) -> Vec<u8> {
    let c = testkit::compute();
    let h = faraday_vault::read_header(other).unwrap();
    let line = format!(
        "seals {} {} {} {} {}\n",
        hex(&h.salt),
        h.slot_len,
        position(other, other_phrase),
        count,
        hex(&digest)
    );
    let desc = testkit::savings();
    let contents = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Plans"),
            Record::new(kind::PLAN)
                .with(field::PLAN_WALLET, desc.as_bytes())
                .with(field::PLAN_ANSWERS, format!("seeds 1\n{line}").as_bytes()),
        ],
    };
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        faraday_vault::Cost {
            memory_kib: faraday_vault::MIN_MEMORY_KIB,
            passes: 1,
            lanes: 1,
        },
        &["a phrase"],
        &[contents],
        &[0x33; 32],
    )
    .unwrap()
}

const A: &str = "a phrase";
const B: &str = "b phrase";

#[test]
fn a_new_vault_reads_sealed_once_and_each_seal_adds_one() {
    let original = made("Main", B, 1);
    let mut app = process(vec![("main.fvlt".into(), original)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(said(&texts, "Sealed once"), "{texts:?}");

    // Changed and locked: the next unlock reads two seals, with the time
    // by the clock that sealed it.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = process(inbox, outbox, kept);
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(said(&texts, "Sealed 2 times · last "), "{texts:?}");
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

#[test]
fn an_older_copy_opened_after_a_newer_one_in_one_power_on_says_so() {
    let original = made("Main", B, 1);
    let mut app = process(
        vec![("main.fvlt".into(), original.clone())],
        Vec::new(),
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Lock);
    let (_, _, kept) = saved(&mut app);

    // The next process, with the first copy and what the last kept.
    let mut app = process(
        vec![("main.fvlt".into(), original)],
        Vec::new(),
        kept.clone(),
    );
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(said(&texts, "An older copy: sealed once"), "{texts:?}");
    assert!(
        said(&texts, "A copy sealed 2 times was open since power-on"),
        "{texts:?}"
    );
    // Warned, not refused: the vault is open.
    assert_eq!(app.vaults().open.len(), 1);

    // Power off: nothing is kept, and the older copy opens with no
    // warning, which is what a count can detect (VAULT-HARDENING §5.4).
    let original = made("Main", B, 1);
    let mut app = process(vec![("main.fvlt".into(), original)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

#[test]
fn two_copies_sealed_from_one_copy_say_so() {
    let original = made("Main", B, 1);
    let one = resealed(&original, B, 0x51);
    let other = resealed(&original, B, 0x52);
    assert_ne!(one, other);
    // The first is open and locked unchanged: it is what the session saw.
    let mut app = process(vec![("one.fvlt".into(), one)], Vec::new(), Vec::new());
    unlock(&mut app, 0, B);
    app.press(Action::Lock);
    let (_, _, kept) = saved(&mut app);
    let mut app = process(vec![("other.fvlt".into(), other)], Vec::new(), kept);
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(
        said(
            &texts,
            "Another copy was sealed 2 times too, with other changes"
        ),
        "{texts:?}"
    );
}

fn plans_vault_and_b(b_at_unlock: &[u8], a_line_count: u64, a_line_digest: [u8; 32]) -> Files {
    let a = plan_naming(b_at_unlock, B, a_line_count, a_line_digest);
    vec![("a.fvlt".into(), a)]
}

#[test]
fn a_plan_in_one_vault_names_an_older_copy_of_another_after_power_off() {
    let b_old = made("Savings", B, 2);
    let b_new = resealed(&b_old, B, 0x61);
    let digest = slot_digest(&b_new, B);

    // Power-on: A (holding the plan that records B at 2) and B's older
    // copy; nothing else is kept.
    let mut inbox = plans_vault_and_b(&b_old, 2, digest);
    inbox.push(("b.fvlt".into(), b_old.clone()));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
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
    let mut inbox = plans_vault_and_b(&b_old, 2, digest);
    inbox.push(("b.fvlt".into(), b_old));
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
    let digest = slot_digest(&b_new, B);

    // B current: nothing is said.
    let mut inbox = plans_vault_and_b(&b_old, 2, digest);
    inbox.push(("b.fvlt".into(), b_new.clone()));
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
    let mut inbox = plans_vault_and_b(&b_old, 2, digest);
    inbox.push(("b.fvlt".into(), b_alt));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    let texts = head(&mut app);
    assert!(said(&texts, "sealed 2 times too"), "{texts:?}");

    // An older copy of the plan's vault carries an older line: no warning
    // on B, the limit §5.6 states.
    let old_line = plans_vault_and_b(&b_old, 1, slot_digest(&b_old, B));
    let mut inbox = old_line;
    inbox.push(("b.fvlt".into(), b_new));
    let mut app = process(inbox, Vec::new(), Vec::new());
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");
}

#[test]
fn another_copy_in_files_is_read_with_the_key_just_opened() {
    let original = made("Main", B, 1);
    let newer = resealed(&original, B, 0x71);
    let other = resealed(&original, B, 0x72);

    // The older copy opens, the newer one is in Files beside it.
    let mut app = process(
        vec![("newer.fvlt".into(), newer.clone())],
        vec![("main.fvlt".into(), original.clone())],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(
        said(
            &texts,
            "An older copy: sealed once. A copy sealed 2 times is in Files."
        ),
        "{texts:?}"
    );
    assert_eq!(app.vaults().open.len(), 1, "warned, not refused");

    // The newer copy opens with the older beside it: nothing is said.
    let mut app = process(
        vec![("main.fvlt".into(), original)],
        vec![("newer.fvlt".into(), newer.clone())],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(!said(&texts, "older copy"), "{texts:?}");

    // Two copies sealed from one copy.
    let mut app = process(
        vec![("other.fvlt".into(), other)],
        vec![("newer.fvlt".into(), newer)],
        Vec::new(),
    );
    unlock(&mut app, 0, B);
    let texts = head(&mut app);
    assert!(
        said(
            &texts,
            "Another copy was sealed 2 times too, with other changes"
        ),
        "{texts:?}"
    );
}

/// A vault `phrase` opens that keeps a plan for the Savings wallet.
fn with_plan(label: &str, phrase: &str, seed: u8) -> Vec<u8> {
    let c = testkit::compute();
    let contents = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes()),
            Record::new(kind::PLAN)
                .with(field::PLAN_WALLET, testkit::savings().as_bytes())
                .with(field::PLAN_ANSWERS, b"seeds 1\n"),
        ],
    };
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        faraday_vault::Cost {
            memory_kib: faraday_vault::MIN_MEMORY_KIB,
            passes: 1,
            lanes: 1,
        },
        &[phrase],
        &[contents],
        &[seed; 32],
    )
    .unwrap()
}

/// The `seals` lines of the plan in the vault `phrase` opens in `file`.
fn plan_lines(file: &[u8], phrase: &str) -> Vec<String> {
    let c = testkit::compute();
    let (_, contents) = faraday_vault::open(&c, file, phrase).unwrap();
    contents
        .records
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
    let (ha, hb) = (
        faraday_vault::read_header(&a).unwrap(),
        faraday_vault::read_header(&b).unwrap(),
    );
    let mut app = process(
        vec![("a.fvlt".into(), a), ("b.fvlt".into(), b)],
        Vec::new(),
        Vec::new(),
    );
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    app.press(Action::Lock);
    let (_, outbox, _) = saved(&mut app);
    let find = |salt: [u8; 32]| {
        outbox
            .iter()
            .find(|(_, f)| faraday_vault::read_header(f).is_ok_and(|h| h.salt == salt))
            .map(|(_, f)| f.clone())
            .expect("the vault was sealed")
    };
    let (a2, b2) = (find(ha.salt), find(hb.salt));
    let in_a = plan_lines(&a2, A);
    let in_b = plan_lines(&b2, B);
    assert_eq!(in_a.len(), 1, "{in_a:?}");
    assert_eq!(in_b.len(), 1, "{in_b:?}");
    assert!(
        in_a[0].starts_with(&format!("seals {} ", hex(&hb.salt))),
        "{in_a:?}"
    );
    assert!(
        in_b[0].starts_with(&format!("seals {} ", hex(&ha.salt))),
        "{in_b:?}"
    );
    // The first vault sealed took the other's count before it was sealed
    // (1); the second took the first's after (2).
    assert_eq!(in_a[0].split(' ').nth(4), Some("1"));
    assert_eq!(in_b[0].split(' ').nth(4), Some("2"));
}
