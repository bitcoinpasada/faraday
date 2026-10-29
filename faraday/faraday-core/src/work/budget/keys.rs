//! A character typed into a text field or a sheet does no costly work
//! beyond its named budget, the key and the frames after it together
//! (`docs/SPEED.md` §9 item 2), as a press does (`presses.rs`). A key stretch
//! (PBKDF2, scrypt, Argon2id) is never done per character: a field that
//! shows the key its passphrase makes works it out once typing pauses
//! ([`crate::motion::TYPED_PAUSE_MS`]), and that pause is checked here
//! too.

use faraday_shell_api::{App, Event, Key};

use super::walk;
use crate::vault::VaultAction as V;
use crate::work::{Counts, KINDS, PIXELS, Work, since, snapshot};
use crate::{Action, Faraday, Screen, testkit};

/// What one character may work out, the key and the frames after it, in
/// the heavy session, with the reason: (field, kind, budget, why). Every
/// kind not listed has a budget of none; a key stretch is never listed.
const KEY_BUDGETS: &[(&str, Work, u64, &str)] = &[
    (
        "Explore, its path",
        Work::KeyDerive,
        2,
        "Explore's reading is the account key and the address at the path as typed: derived for each path, then kept on Explore's state by it (Faraday::explore_reading)",
    ),
    (
        "Tools, a descriptor's checksum",
        Work::DescriptorParse,
        1,
        "the checksum calculator reads what is typed as a descriptor each time it changes, then keeps the reading by the text (Faraday::tools_reading)",
    ),
    (
        "Tools, Miniscript",
        Work::DescriptorParse,
        1,
        "the Miniscript calculator compiles the policy typed and reads the descriptor it makes each time it changes, then keeps the reading by the text (Faraday::tools_reading)",
    ),
    (
        "Tools, Miniscript",
        Work::KeyDerive,
        3,
        "the first character asks the loaded keys' multisig account keys a policy may name (Faraday::tools_keys): each derived once and kept on its key; three keys are loaded",
    ),
];

/// A field: its name, what opens it with typing going to it, and what is
/// typed.
type Field = (&'static str, fn(&mut Faraday), String);

/// The budget of `kind` for a character typed into `field`.
fn budget(field: &str, kind: Work) -> u64 {
    KEY_BUDGETS
        .iter()
        .find(|(f, k, ..)| *f == field && *k == kind)
        .map_or(0, |b| b.2)
}

/// The key, then a frame, a tick and a frame: the work it asks for now
/// and on the tick after.
fn key_work(app: &mut Faraday, ch: char) -> Counts {
    let before = snapshot();
    app.event(Event::Key(Key::Char(ch)));
    let _ = app.frame();
    let now = app.now_ms;
    app.event(Event::Tick { now_ms: now + 16 });
    let _ = app.frame();
    since(&before)
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// `screen` with nothing open over it.
fn on(app: &mut Faraday, screen: Screen) {
    app.inbox.clear_potential();
    app.seed_read.clear();
    app.sheet = None;
    app.press(Action::Nav(screen));
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn fields() -> Vec<Field> {
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let other = testkit::test_words(testkit::TEST_SEEDS[2].0);
    let kit = testkit::kits()[0].descriptor.clone();
    let mut out: Vec<Field> = vec![
        (
            "Load a key, its words",
            |app| {
                on(app, Screen::Home);
                app.press(Action::Entry(None));
            },
            words,
        ),
        (
            "Load a key, its passphrase",
            |app| {
                on(app, Screen::Home);
                app.press(Action::Entry(None));
                type_text(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
                app.press(Action::EntryPassphrase);
            },
            "a passphrase".into(),
        ),
        (
            "A seed in Files made a wallet, its passphrase",
            |app| {
                on(app, Screen::Files);
                let c = app.compute();
                let fp = app
                    .inbox_found_now(&c)
                    .seeds
                    .first()
                    .map(|s| s.fingerprint)
                    .expect("the test stick holds a seed");
                app.potential_open(fp);
                app.press(Action::PotentialType);
            },
            "a passphrase".into(),
        ),
        (
            "A SeedQR read, its passphrase",
            |app| {
                on(app, Screen::Home);
                app.seed_read_open(&testkit::test_words(testkit::TEST_SEEDS[2].0));
                app.press(Action::SeedRead(crate::seeds::read::Choice::AddPass));
            },
            "a passphrase".into(),
        ),
        (
            "A SeedQR read, its passphrase again",
            |app| {
                on(app, Screen::Home);
                app.seed_read_open(&testkit::test_words(testkit::TEST_SEEDS[2].0));
                app.press(Action::SeedRead(crate::seeds::read::Choice::AddPass));
                type_text(app, "a passphrase");
                app.press(Action::SeedRead(crate::seeds::read::Choice::Field(1)));
            },
            "a passphrase".into(),
        ),
        (
            "Sign a message",
            |app| {
                on(app, Screen::Home);
                app.press(Action::SignMessage);
                app.press(Action::MType);
            },
            "Faraday signs this".into(),
        ),
        (
            "Explore, its path",
            |app| {
                on(app, Screen::Home);
                app.press(Action::Explore);
            },
            "/0/1/2".into(),
        ),
        (
            "Lightning, an aezeed's words",
            |app| {
                on(app, Screen::Home);
                app.press(Action::Lightning);
            },
            other,
        ),
        (
            "Rename a wallet",
            |app| {
                on(app, Screen::Wallets);
                app.press(Action::Rename);
            },
            " savings".into(),
        ),
        (
            "Restore a GPG key, a paperkey's line",
            |app| {
                on(app, Screen::Home);
                restore_at_lines(app);
            },
            "  1: 00 04 84 6A F8 B7 D8".into(),
        ),
    ];
    for (i, (name, text)) in [
        ("Tools, hashes", "abc".to_string()),
        ("Tools, encodings", "deadbeef".to_string()),
        ("Tools, a descriptor's checksum", kit),
        ("Tools, convert a key", "xpub6".to_string()),
        ("Tools, units", "21000".to_string()),
        ("Tools, Miniscript", "and(pk(A),older(144))".to_string()),
    ]
    .into_iter()
    .enumerate()
    {
        let open: fn(&mut Faraday) = match i {
            0 => |app| tool(app, 0),
            1 => |app| tool(app, 1),
            2 => |app| tool(app, 2),
            3 => |app| tool(app, 3),
            4 => |app| tool(app, 4),
            _ => |app| tool(app, 5),
        };
        out.push((name, open, text));
    }
    // Last: the test vault is locked for it.
    out.push((
        "Unlock a vault, its passphrase",
        |app| {
            while !app.vaults.open.is_empty() {
                let _ = app.vault_lock_one(0);
            }
            on(app, Screen::Vaults);
            let n = app
                .vault_files()
                .iter()
                .position(|f| f.name == "vault.fvlt")
                .expect("the test vault is listed");
            app.press(Action::Vault(V::Open(n)));
        },
        "a passphrase".into(),
    ));
    out
}

/// Step 4 of a restore of a GPG key, its lines taking typing, on a public
/// key GnuPG made (`tools/vectors/paperkey/gpg-unprotected.asc`, of
/// Faraday's shape).
fn restore_at_lines(app: &mut Faraday) {
    use crate::backup_flow::steps::FlowAction as F;
    use crate::backup_flow::{BackupFlow, RestoreFrom, Step};
    let cert = include_bytes!("../../../../../tools/vectors/paperkey/gpg-unprotected.asc");
    let c = testkit::compute();
    let from = RestoreFrom::read(cert).expect("a public key of Faraday's shape");
    let mut f = BackupFlow::restoring(&c, from);
    f.step = Step::Make;
    let i = testkit::flow_push(app, f);
    app.press(Action::Flow(F::Show(i)));
    assert!(app.gpg_restore_typing(), "step 4's line takes typing");
}

/// Tools, calculator `n` open.
fn tool(app: &mut Faraday, n: u8) {
    on(app, Screen::Home);
    app.press(Action::Tools);
    app.press(Action::TTool(n));
}

#[test]
fn a_character_typed_into_any_field_does_no_costly_work_beyond_its_budget() {
    assert!(
        KEY_BUDGETS.iter().all(|(_, k, ..)| *k != Work::Kdf),
        "a key stretch is never budgeted per character"
    );
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for (name, open, text) in fields() {
        open(&mut app);
        app.settle();
        let _ = app.frame();
        let mut most = [0u64; KINDS.len()];
        for ch in text.chars() {
            let work = key_work(&mut app, ch);
            for (i, k) in KINDS.iter().enumerate() {
                most[i] = most[i].max(work.of(*k));
            }
        }
        let mut line = format!("{name} ({:?}, {:?}):", app.screen, app.sheet);
        for (i, k) in KINDS.iter().enumerate() {
            if most[i] == 0 || PIXELS.contains(k) {
                continue;
            }
            line.push_str(&format!(" {k:?} {}", most[i]));
            let b = budget(name, *k);
            if most[i] > b {
                failures.push(format!(
                    "{name}: a character typed did {k:?} {} times (budget {b})",
                    most[i]
                ));
            }
        }
        lines.push(line);
    }
    assert!(
        failures.is_empty(),
        "{}\n\nEach field, the most one character did:\n{}",
        failures.join("\n"),
        lines.join("\n")
    );
}

#[test]
fn a_passphrase_typed_shows_the_key_it_makes_once_typing_pauses() {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    on(&mut app, Screen::Home);
    app.seed_read_open(&testkit::test_words(testkit::TEST_SEEDS[2].0));
    app.press(Action::SeedRead(crate::seeds::read::Choice::AddPass));
    type_text(&mut app, "a passphrase");
    app.press(Action::SeedRead(crate::seeds::read::Choice::Field(1)));
    let start = app.now_ms;
    type_text(&mut app, "a passphrase");
    let before = snapshot();
    app.event(Event::Tick {
        now_ms: start + crate::motion::TYPED_PAUSE_MS / 2,
    });
    let _ = app.frame();
    assert_eq!(
        since(&before).of(Work::Kdf),
        0,
        "nothing is worked out while typing goes on"
    );
    app.event(Event::Tick {
        now_ms: start + crate::motion::TYPED_PAUSE_MS,
    });
    let _ = app.frame();
    assert_eq!(
        since(&before).of(Work::Kdf),
        1,
        "the key is worked out once typing pauses"
    );
    let shown = app.seed_read.state().and_then(|s| s.with_key());
    let want = crate::scan_key::key_of(
        &c,
        &testkit::test_words(testkit::TEST_SEEDS[2].0),
        "a passphrase",
    );
    assert_eq!(shown, want, "the sheet shows the key the passphrase makes");
}
