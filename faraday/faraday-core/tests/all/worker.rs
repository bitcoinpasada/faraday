//! The worker that runs the costly part of a press (`docs/WORKERS.md`):
//! Load ticked on Practice's samples, whose answer is applied on a later
//! pass.

use faraday_core::samples::Pick;
use faraday_core::testkit;
use faraday_core::{Action, DESKTOP_MODES, Faraday, strings};
use faraday_shell_api::{App, Event};

/// Practice with the first sample key ticked on the Load samples sheet.
fn ticked() -> Faraday {
    let mut app = testkit::started();
    app.set_modes(DESKTOP_MODES);
    testkit::into_demo(&mut app);
    app.samples = Some(testkit::demo_samples());
    app.press(Action::Samples);
    app.press(Action::SampleTick(Pick::Key(0)));
    app
}

fn tick(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
}

#[test]
fn load_ticked_gives_the_same_key_on_the_worker_as_run_inline() {
    let mut inline = ticked();
    inline.press(Action::SamplesLoad);

    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    assert!(app.busy().is_some(), "the screen is busy while it runs");
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys.len(), inline.session.keys.len());
    assert_eq!(app.toast_text(), inline.toast_text());
    assert_eq!(app.sheet, None, "the sheet closes with the answer");
}

#[test]
fn a_press_while_it_runs_does_nothing() {
    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    assert!(app.busy().is_some());
    app.press(Action::SamplesLoad);
    app.press(Action::SampleTick(Pick::Key(1)));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1, "loaded once, the one key");
}

#[test]
fn back_while_it_runs_leaves_nothing_loaded() {
    let mut app = ticked();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    tick(&mut app, 1000);
    assert_eq!(app.session.keys.len(), 0, "the answer was dropped");
}

#[test]
fn a_lock_while_it_runs_leaves_nothing_loaded() {
    let mut app = ticked();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    app.press(Action::LockUnsealed);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    tick(&mut app, 1000);
    assert_eq!(app.session.keys.len(), 0, "no key is held after the lock");
}

mod vault_open {
    use super::*;
    use crate::common::{type_text, vault};
    use faraday_core::vault::VaultAction as V;
    use faraday_core::{Sheet, StickInfo, StorageEvent, testkit};
    use faraday_shell_api::{Key, TouchPhase};

    const PASS: &str = "correct horse";

    fn device() -> Faraday {
        let mut app = testkit::started();
        app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
        testkit::vault_speed(&mut app, Some(100));
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                faraday_text::Text::of("vault.kdbx"),
                vault("Test", PASS, 7),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app
    }

    fn asked(app: &mut Faraday, phrase: &str) {
        app.press(Action::Vault(V::Open(0)));
        type_text(app, phrase);
    }

    fn records(app: &Faraday) -> usize {
        let v = app.vaults();
        v.records(v.open[0].id).len()
    }

    #[test]
    fn a_vault_opened_on_the_worker_holds_what_it_holds_inline() {
        let mut inline = device();
        asked(&mut inline, PASS);
        inline.press(Action::Vault(V::Unlock));
        assert_eq!(inline.vaults().open.len(), 1);

        let mut app = device();
        asked(&mut app, PASS);
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        assert!(app.busy().is_some(), "the screen is busy while it opens");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        assert_eq!(app.vaults().open.len(), 1);
        assert_eq!(records(&app), records(&inline));
    }

    #[test]
    fn a_wrong_passphrase_on_the_worker_says_what_it_says_inline() {
        let mut inline = device();
        asked(&mut inline, "not it");
        inline.press(Action::Vault(V::Unlock));

        let mut app = device();
        asked(&mut app, "not it");
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        testkit::settle(&mut app);
        assert!(app.vaults().open.is_empty());
        assert!(app.vaults().unlock_error.is_some());
        assert_eq!(app.vaults().unlock_error, inline.vaults().unlock_error);
    }

    #[test]
    fn back_during_an_open_leaves_the_vault_locked() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        app.press(Action::Back);
        assert!(app.busy().is_none());
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        tick(&mut app, 1000);
        assert!(app.vaults().open.is_empty());
        assert!(app.vaults().passphrase.text.is_empty());
    }

    #[test]
    fn a_lock_during_an_open_leaves_nothing_unlocked() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        app.press(Action::LockUnsealed);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        tick(&mut app, 1000);
        assert!(app.vaults().open.is_empty());
        assert_eq!(app.session.keys.len(), 0, "no key is held after the lock");
    }

    #[test]
    fn back_by_mouse_right_after_unlock_leaves_the_vault_locked() {
        let mut app = device();
        asked(&mut app, PASS);
        let _ = app.frame();
        let (x, y) = app
            .where_offered(Action::Vault(V::Unlock))
            .expect("Unlock is on screen");
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch { x, y, phase });
        }
        app.event(Event::Back);
        let _ = app.frame();
        tick(&mut app, 1000);
        testkit::settle(&mut app);
        tick(&mut app, 2000);
        assert!(app.vaults().open.is_empty());
        assert!(app.vaults().passphrase.text.is_empty());
    }

    fn a_stick() -> StorageEvent {
        StorageEvent::Sticks(vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("stick"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }])
    }

    #[test]
    fn a_stick_put_in_during_an_open_leaves_the_vault_locked() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        app.storage(a_stick());
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(app.vaults().open.is_empty());
    }

    #[test]
    fn cancel_on_the_lock_sheet_leaves_a_running_open_running() {
        let mut app = testkit::started();
        app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
        testkit::vault_speed(&mut app, Some(100));
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![
                (faraday_text::Text::of("vault.kdbx"), vault("Test", PASS, 7)),
                (
                    faraday_text::Text::of("second.kdbx"),
                    vault("Second", PASS, 8),
                ),
            ]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        asked(&mut app, PASS);
        app.press(Action::Vault(V::Unlock));
        let id = app.vaults().open[0].id;
        testkit::vault_changed(&mut app, id);
        app.press(Action::Vault(V::Open(1)));
        app.press(Action::Vault(V::FocusPassphrase));
        type_text(&mut app, PASS);
        assert_eq!(app.vaults().passphrase.text, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        assert!(app.busy().is_some(), "the second open runs");
        app.press(Action::LockAsk);
        assert_eq!(app.sheet, Some(Sheet::LockAsk), "the sheet asks first");
        app.press(Action::Cancel);
        assert_eq!(app.sheet, None);
        assert!(app.busy().is_some(), "the open goes on");
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(app.vaults().open.len(), 2);
    }

    #[test]
    fn enter_during_an_open_starts_no_second_one() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        assert!(app.busy().is_some());
        app.event(Event::Key(Key::Enter));
        testkit::release_worker();
        testkit::settle(&mut app);
        let _ = app.frame();
        tick(&mut app, 1000);
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        assert_eq!(app.vaults().open.len(), 1, "opened once");
        assert_eq!(app.vaults().unlock_error, None);
    }

    fn a_stick_with_files() -> StorageEvent {
        StorageEvent::Sticks(vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("stick"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: vec![(faraday_text::Text::of("note.txt"), 5)],
            vaults: Vec::new(),
        }])
    }

    #[test]
    fn a_stick_that_moves_the_screen_during_an_open_drops_its_answer() {
        let mut app = device();
        asked(&mut app, PASS);
        let started_on = (app.screen, app.sheet);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        assert!(app.busy().is_some());
        app.storage(a_stick_with_files());
        assert_ne!(
            (app.screen, app.sheet),
            started_on,
            "the stick moved the screen"
        );
        assert!(app.busy().is_none(), "the job is not waited for");
        // Taken out again before the answer comes: nothing in the way of
        // the open but the answer's being stale.
        app.storage(StorageEvent::Sticks(Vec::new()));
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert!(
            app.vaults().open.is_empty(),
            "no vault opens from a stale answer"
        );
    }

    #[test]
    fn a_storage_event_that_moves_no_screen_leaves_the_open_running() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        let at = (app.screen, app.sheet);
        app.storage(StorageEvent::Memory {
            available_mib: 14_000,
        });
        app.storage(StorageEvent::Sticks(Vec::new()));
        assert_eq!((app.screen, app.sheet), at);
        assert!(app.busy().is_some(), "the open goes on");
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(app.vaults().open.len(), 1, "its answer applies");
    }

    #[test]
    fn a_row_pressed_while_an_open_runs_starts_nothing() {
        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        let (at, jobs) = ((app.screen, app.sheet), testkit::jobs_asked(&app));
        let _ = app.frame();
        app.press(Action::Vault(V::Open(0)));
        app.press(Action::Nav(faraday_core::Screen::Home));
        assert!(app.busy().is_some(), "the busy screen stays");
        assert_eq!((app.screen, app.sheet), at);
        assert_eq!(testkit::jobs_asked(&app), jobs);
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(app.vaults().open.len(), 1);
    }
}

mod aezeed_read {
    use super::*;
    use faraday_shell_api::Key;

    const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    const NODE_KEY: &str = "024c7005923a074fd38b16ace7be4914ec9f929717692bfb585019be13290c9b1d";

    fn typed(words: &str) -> Faraday {
        let mut app = testkit::started();
        app.press(Action::Lightning);
        app.press(Action::LAezeed);
        for c in words
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
        {
            app.event(Event::Key(Key::Char(c)));
        }
        app
    }

    /// Enter, the frame that shows it, and the tick that starts the read.
    fn entered(app: &mut Faraday) {
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
    }

    #[test]
    fn an_aezeed_read_on_the_worker_gives_the_node_key() {
        let mut app = typed(CIPHER_SEED);
        entered(&mut app);
        assert!(app.busy().is_some(), "the screen is busy while it reads");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        let l = app.lightning().unwrap();
        let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
        let id: String = n.public().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(id, NODE_KEY);
    }

    #[test]
    fn a_wrong_aezeed_on_the_worker_says_what_it_says_inline() {
        let words = ["abandon"; 24].join(" ");
        let mut inline = typed(&words);
        inline.press(Action::LResolve);
        let said = inline.lightning().unwrap().error.clone();
        assert!(said.is_some());

        let mut app = typed(&words);
        entered(&mut app);
        testkit::settle(&mut app);
        let l = app.lightning().unwrap();
        assert!(l.node.is_none());
        assert_eq!(l.error, said);
    }

    /// Enter, held: the read is asked of the worker and waits there.
    fn entered_held(app: &mut Faraday) {
        testkit::hold_worker();
        entered(app);
        assert!(app.busy().is_some());
    }

    /// Back to the Lightning screen and the words typed again, so a
    /// stale answer applied would have a screen to join.
    fn typed_again(app: &mut Faraday) {
        app.press(Action::Lightning);
        app.press(Action::LAezeed);
        for c in CIPHER_SEED.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
    }

    #[test]
    fn back_during_an_aezeed_read_drops_it() {
        let mut app = typed(CIPHER_SEED);
        entered_held(&mut app);
        app.press(Action::Back);
        assert!(app.busy().is_none());
        typed_again(&mut app);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert!(app.lightning().is_none_or(|l| l.node.is_none()));
    }

    #[test]
    fn escape_during_an_aezeed_read_drops_it() {
        let mut app = typed(CIPHER_SEED);
        entered_held(&mut app);
        app.event(Event::Key(Key::Escape));
        assert!(app.busy().is_none());
        typed_again(&mut app);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert!(app.lightning().is_none_or(|l| l.node.is_none()));
    }

    #[test]
    fn a_lock_during_an_aezeed_read_leaves_no_node_key() {
        let mut app = typed(CIPHER_SEED);
        entered_held(&mut app);
        app.press(Action::LockUnsealed);
        typed_again(&mut app);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert!(app.lightning().is_none_or(|l| l.node.is_none()));
    }

    #[test]
    fn enter_during_an_aezeed_read_starts_no_second_one() {
        let mut app = typed(CIPHER_SEED);
        entered_held(&mut app);
        let asked = testkit::jobs_asked(&app);
        // A second Enter while the first read runs is refused; it must not
        // wait for the next tick to start another.
        app.event(Event::Key(Key::Enter));
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        assert!(app.busy().is_none());
        assert_eq!(testkit::jobs_asked(&app), asked, "a second read was asked");
    }

    #[test]
    fn two_enters_before_the_tick_start_one_read() {
        let mut app = typed(CIPHER_SEED);
        let asked = testkit::jobs_asked(&app);
        app.event(Event::Key(Key::Enter));
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        testkit::settle(&mut app);
        assert_eq!(testkit::jobs_asked(&app), asked + 1);
    }
}

mod signing {
    use super::*;
    use crate::common::add_key;
    use faraday_core::StorageEvent;
    use faraday_core::testkit::Kit;

    fn kit(id: &str) -> Kit {
        testkit::kits().into_iter().find(|k| k.id == id).unwrap()
    }

    fn at(app: &Faraday, name: &str) -> usize {
        app.inbox().iter().position(|i| i.name == name).unwrap()
    }

    fn files() -> Vec<(String, Vec<u8>)> {
        let k = kit("spending");
        let psbt = testkit::unsigned(&k).unwrap();
        vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ]
    }

    fn inbox_of() -> Faraday {
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(files()),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app
    }

    /// The wallet and test key 1 loaded, the spend opened, nothing signed.
    fn open(app: &mut Faraday, key: bool) {
        app.press(Action::LoadWallet(at(app, "spending-wallet.txt")));
        if key {
            add_key(app, "bacon");
        }
        app.press(Action::StartSpend(at(app, "spend.psbt")));
    }

    fn unsigned_spend() -> Faraday {
        let mut app = inbox_of();
        open(&mut app, true);
        app
    }

    fn nothing_signed(app: &Faraday) {
        let s = app.spend().expect("the spend is open");
        assert!(s.spend.signed_here.is_empty(), "something signed");
        assert!(s.spend.finished.is_none());
        assert!(app.outbox.is_empty(), "the Outbox holds a file");
    }

    #[test]
    fn a_transaction_signed_on_the_worker_has_the_same_signatures() {
        let mut inline = unsigned_spend();
        testkit::steps_before_sign_done(&mut inline);
        inline.press(Action::SignHere);

        let mut app = unsigned_spend();
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        assert!(app.busy().is_some(), "the screen is busy while it signs");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        let (a, b) = (app.spend().unwrap(), inline.spend().unwrap());
        assert!(a.spend.finished.is_some());
        assert_eq!(a.spend.psbt.to_bytes(), b.spend.psbt.to_bytes());
        assert_eq!(a.spend.signed_here, b.spend.signed_here);
        assert_eq!(a.spend.wrote, b.spend.wrote);
        assert_eq!((&a.signers, &a.to_sign), (&b.signers, &b.to_sign));
        assert_eq!(
            (a.complete, a.open, &a.error),
            (b.complete, b.open, &b.error)
        );
        assert!(!app.outbox.is_empty());
        let names = |x: &Faraday| x.outbox.iter().map(|o| o.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&app), names(&inline));
    }

    #[test]
    fn a_session_copy_for_the_worker_reads_a_spend_as_the_session_does() {
        let mut app = unsigned_spend();
        testkit::steps_before_sign_done(&mut app);
        app.press(Action::SignHere);
        let psbt = &app.spend().unwrap().spend.psbt;
        let c = testkit::compute();
        let copy = app.session.job_copy();
        assert_eq!(copy.inspect_inputs(), app.session.inspect_inputs());
        assert_eq!(copy.inspect(&c, psbt), app.session.inspect(&c, psbt));
        assert_eq!(copy.wallet_for(&c, psbt), app.session.wallet_for(&c, psbt));
        let (a, b) = (
            copy.nonce_check(&c, psbt),
            app.session.nonce_check(&c, psbt),
        );
        assert!(!a.rows.is_empty());
        assert_eq!(a.rows.len(), b.rows.len());
        assert_eq!(a.reused.len(), b.reused.len());
        assert_eq!(a.unread, b.unread);
        // The words stay on the main thread.
        let reveal = faraday_keys::Reveal::Words;
        assert!(copy.keys.reveal(0, reveal).is_empty());
        assert!(!app.session.keys.reveal(0, reveal).is_empty());
    }

    #[test]
    fn a_wallet_takes_home_the_scripts_its_copy_derived() {
        use faraday_core::work::{Work, since, snapshot};
        let app = unsigned_spend();
        let c = testkit::compute();
        let wallet = &app.session.wallets[0];
        let copy = wallet.job_copy();
        assert!(copy.script_at(&c, false, 9).is_some());
        wallet.take_kept(copy);
        let before = snapshot();
        assert!(wallet.script_at(&c, false, 9).is_some());
        assert_eq!(since(&before).of(Work::WalletMatch), 0, "derived again");
        assert!(wallet.script_at(&c, false, 10).is_some());
        assert_eq!(since(&before).of(Work::WalletMatch), 1);
    }

    #[test]
    fn a_transaction_no_key_here_signs_says_what_it_says_inline() {
        let mut inline = inbox_of();
        open(&mut inline, false);
        testkit::steps_before_sign_done(&mut inline);
        inline.press(Action::SignHere);
        let said = inline.spend().unwrap().error.clone();
        assert!(said.is_some());

        let mut app = inbox_of();
        open(&mut app, false);
        let asked = testkit::jobs_asked(&app);
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        assert!(app.busy().is_none(), "a refusal starts no job");
        assert_eq!(testkit::jobs_asked(&app), asked);
        assert_eq!(app.spend().unwrap().error, said);
    }

    #[test]
    fn back_during_signing_leaves_it_unsigned() {
        let mut app = unsigned_spend();
        testkit::hold_worker();
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        assert!(app.busy().is_some());
        app.press(Action::Back);
        assert!(app.busy().is_none());
        // The same transaction opened again, so that a stale answer
        // applied would have a spend to sign.
        app.press(Action::StartSpend(at(&app, "spend.psbt")));
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        nothing_signed(&app);
    }

    #[test]
    fn a_lock_during_signing_leaves_it_unsigned_and_the_key_gone() {
        let mut app = unsigned_spend();
        testkit::hold_worker();
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        app.press(Action::LockUnsealed);
        assert!(app.busy().is_none());
        assert_eq!(app.session.keys.len(), 0, "the lock forgot the key");
        // Unlocked and back at the same transaction before the answer.
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(files()),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        open(&mut app, true);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        nothing_signed(&app);
    }

    #[test]
    fn a_lock_during_signing_leaves_the_store_empty_and_the_outbox_too() {
        let mut app = unsigned_spend();
        testkit::hold_worker();
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        app.press(Action::LockUnsealed);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert_eq!(app.session.keys.len(), 0, "the key is dropped");
        assert!(app.busy().is_none(), "the job ended");
        assert!(app.spend().is_none());
        assert!(app.outbox.is_empty());
    }

    #[test]
    fn a_key_randomness_reached_while_it_signs_is_blinded_when_the_job_ends() {
        let mut app = inbox_of();
        app.event(faraday_shell_api::Event::Entropy(
            faraday_shell_api::EntropyBytes::new([0x11; 32]),
        ));
        open(&mut app, true);
        let before = app.session.keys.master_for_tests(0).blindings();
        testkit::hold_worker();
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        // The person adds randomness while the job holds the key.
        let mut rolls = faraday_entropy::DiceRolls::new();
        for r in 1..=6 {
            assert!(rolls.push(r));
        }
        assert!(app.own_add(faraday_core::fresh::OwnSource::Dice, &rolls.commitment(), 6));
        assert_eq!(
            app.session.keys.master_for_tests(0).blindings(),
            before,
            "the key the job holds is not changed under it"
        );
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(
            !app.spend().unwrap().spend.signed_here.is_empty(),
            "it signed"
        );
        assert_eq!(
            app.session.keys.master_for_tests(0).blindings(),
            before + 1,
            "blinded as the answer was applied"
        );
        assert!(!app.session.keys.owes_reblind());
    }

    mod reading {
        use super::*;
        use faraday_core::{Screen, StickInfo};
        use faraday_shell_api::Event as Ev;

        fn loaded() -> Faraday {
            let mut app = inbox_of();
            app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
            add_key(&mut app, "bacon");
            app
        }

        fn go(app: &mut Faraday) {
            let i = at(app, "spend.psbt");
            testkit::press_on_worker(app, Action::StartSpend(i));
        }

        /// The same spend open in both: what a person reads on the review.
        fn same_review(a: &Faraday, b: &Faraday) {
            assert_eq!(a.screen, b.screen);
            let (a, b) = (a.spend().expect("opened"), b.spend().expect("opened"));
            assert_eq!(a.spend.psbt.to_bytes(), b.spend.psbt.to_bytes());
            assert_eq!(a.wallet, b.wallet);
            assert_eq!(a.signers, b.signers);
            assert_eq!((&a.to_sign, &a.in_file), (&b.to_sign, &b.in_file));
            assert_eq!(
                (a.complete, &a.refused, &a.steps),
                (b.complete, &b.refused, &b.steps)
            );
            assert_eq!(a.needed, b.needed);
            assert_eq!(a.open, b.open);
            assert_eq!(a.done, b.done);
            assert_eq!(format!("{:?}", a.inspection), format!("{:?}", b.inspection));
        }

        #[test]
        fn a_psbt_opened_from_a_file_on_the_worker_opens_the_same_review() {
            let mut inline = loaded();
            inline.press(Action::StartSpend(at(&inline, "spend.psbt")));

            let mut app = loaded();
            go(&mut app);
            assert!(app.busy().is_some(), "the screen is busy while it reads");
            assert!(app.spend().is_none(), "nothing opens before the answer");
            testkit::settle(&mut app);
            assert!(app.busy().is_none());
            assert_eq!(app.screen, Screen::Spend);
            same_review(&app, &inline);
        }

        #[test]
        fn a_psbt_completed_by_its_last_scanned_part_opens_the_same_review() {
            let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
            let mut enc = faraday_codec::ur::Encoder::psbt(&psbt, 60).unwrap();
            let parts: Vec<String> = (0..enc.fragment_count())
                .map(|_| String::from(&*enc.next_part()))
                .collect();
            let scanned = || {
                let mut app = Faraday::new();
                app.press(Action::Scan);
                for p in &parts {
                    app.event(Ev::Scanned {
                        bytes: p.as_bytes().to_vec(),
                    });
                }
                app
            };
            let mut inline = scanned();
            inline.press(Action::ReadGo(0));

            let mut app = scanned();
            assert_eq!(app.inbox().last().map(|i| i.bytes.to_vec()), Some(psbt));
            testkit::press_on_worker(&mut app, Action::ReadGo(0));
            assert!(app.busy().is_some());
            assert!(app.spend().is_none());
            testkit::settle(&mut app);
            same_review(&app, &inline);
        }

        #[test]
        fn a_file_that_is_no_psbt_says_what_it_says_inline() {
            let named = |app: &mut Faraday| {
                app.storage(StorageEvent::Restored {
                    inbox: faraday_core::files_from(vec![(
                        faraday_text::Text::of("notes.psbt"),
                        b"not a psbt".to_vec(),
                    )]),
                    outbox: Vec::new(),
                    kept: Vec::new(),
                });
            };
            let mut inline = testkit::started();
            named(&mut inline);
            inline.press(Action::StartSpend(0));
            let said = inline.toast_text().map(str::to_string);
            assert!(said.is_some());

            let mut app = testkit::started();
            named(&mut app);
            testkit::press_on_worker(&mut app, Action::StartSpend(0));
            testkit::settle(&mut app);
            assert_eq!(app.toast_text().map(str::to_string), said);
            assert!(app.spend().is_none());
        }

        #[test]
        fn a_psbt_of_another_network_than_the_wallets_says_what_it_says_inline() {
            let mut inner = testkit::unsigned(&kit("spending")).unwrap().into_inner();
            for (_, path) in inner.inputs[0].bip32_derivation.values_mut() {
                let mut steps: Vec<_> = path.into_iter().copied().collect();
                steps[1] = faraday_bip::bitcoin::bip32::ChildNumber::from_hardened_idx(0).unwrap();
                *path = steps.into();
            }
            let mainnet = faraday_psbt::Psbt::from(inner).to_bytes();
            let make = || {
                let mut app = testkit::started();
                app.storage(StorageEvent::Restored {
                    inbox: faraday_core::files_from(vec![
                        (faraday_text::Text::of("main.psbt"), mainnet.clone()),
                        (
                            faraday_text::Text::of("spending-wallet.txt"),
                            kit("spending").descriptor.into_bytes(),
                        ),
                    ]),
                    outbox: Vec::new(),
                    kept: Vec::new(),
                });
                app.press(Action::LoadWallet(1));
                app
            };
            let mut inline = make();
            inline.press(Action::StartSpend(0));
            let said = inline.toast_text().map(str::to_string);
            assert!(said.is_some());

            let mut app = make();
            testkit::press_on_worker(&mut app, Action::StartSpend(0));
            testkit::settle(&mut app);
            assert_eq!(app.toast_text().map(str::to_string), said);
            assert!(app.spend().is_none());
        }

        #[test]
        fn back_during_the_read_opens_nothing() {
            let mut app = loaded();
            testkit::hold_worker();
            go(&mut app);
            assert!(app.busy().is_some());
            app.press(Action::Back);
            assert!(app.busy().is_none());
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            assert!(app.spend().is_none(), "the stale answer opened a spend");
            assert_ne!(app.screen, Screen::Spend);
        }

        #[test]
        fn a_lock_during_the_read_then_the_same_file_opened_again_keeps_the_new_spend() {
            let mut app = loaded();
            testkit::hold_worker();
            go(&mut app);
            app.press(Action::LockUnsealed);
            assert!(app.busy().is_none());
            // Unlocked, the wallet and key back, the file opened and
            // signed before the first answer arrives.
            app.storage(StorageEvent::Restored {
                inbox: faraday_core::files_from(files()),
                outbox: Vec::new(),
                kept: Vec::new(),
            });
            open(&mut app, true);
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            assert!(!app.spend().unwrap().spend.signed_here.is_empty());
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            let s = app.spend().expect("the spend stays open");
            assert!(
                !s.spend.signed_here.is_empty(),
                "the stale answer replaced the spend"
            );
        }

        fn a_stick() -> StorageEvent {
            StorageEvent::Sticks(vec![StickInfo {
                make: Default::default(),
                serial: None,
                id: faraday_text::Text::of("stick"),
                label: faraday_text::Text::of("STICK"),
                boot: false,
                files: Vec::new(),
                vaults: Vec::new(),
            }])
        }

        #[test]
        fn a_stick_put_in_during_the_read_keeps_the_visit_and_opens_nothing() {
            let mut app = inbox_of();
            app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
            testkit::hold_worker();
            go(&mut app);
            app.storage(a_stick());
            assert_eq!(app.screen, Screen::Files);
            testkit::release_worker();
            testkit::settle(&mut app);
            assert_eq!(app.screen, Screen::Files, "the answer left the visit");
            assert!(app.spend().is_none(), "a spend opened behind the person");
        }

        #[test]
        fn a_stick_put_in_with_a_key_held_during_the_read_keeps_the_lock_and_opens_nothing() {
            let mut app = loaded();
            testkit::hold_worker();
            go(&mut app);
            app.storage(a_stick());
            assert_eq!(app.sheet, Some(faraday_core::Sheet::Lock));
            let screen = app.screen;
            testkit::release_worker();
            testkit::settle(&mut app);
            assert_eq!(app.sheet, Some(faraday_core::Sheet::Lock));
            assert_eq!(app.screen, screen);
            assert!(app.spend().is_none(), "a spend opened behind the lock");
        }

        #[test]
        fn a_psbt_of_the_other_network_kind_moves_an_empty_session_and_opens_the_same_review() {
            let make = || {
                let mut app = testkit::started();
                app.storage(StorageEvent::Restored {
                    inbox: faraday_core::files_from(files()),
                    outbox: Vec::new(),
                    kept: Vec::new(),
                });
                app
            };
            let mut inline = make();
            inline.press(Action::StartSpend(at(&inline, "spend.psbt")));
            let mut app = make();
            go(&mut app);
            testkit::settle(&mut app);
            assert_eq!(app.session.network(), inline.session.network());
            assert_ne!(app.session.network(), faraday_bip::keys::Network::Mainnet);
            same_review(&app, &inline);
        }

        #[test]
        fn a_second_press_while_it_reads_starts_no_second_read() {
            let mut app = loaded();
            testkit::hold_worker();
            go(&mut app);
            let asked = testkit::jobs_asked(&app);
            go(&mut app);
            assert_eq!(testkit::jobs_asked(&app), asked, "a second read was asked");
            testkit::release_worker();
            testkit::settle(&mut app);
            assert!(app.spend().is_some());
            assert_eq!(testkit::jobs_asked(&app), asked);
        }
    }

    mod musig {
        use super::*;
        use crate::rounds::{at, device, our_nonce};

        fn nonce_copies(app: &Faraday) -> Vec<usize> {
            app.inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-nonce-"))
                .map(|(i, _)| i)
                .collect()
        }

        #[test]
        fn a_musig_round_on_the_worker_completes() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            testkit::press_on_worker(&mut app, Action::SignHere);
            testkit::settle(&mut app);
            assert!(our_nonce(&app).is_some(), "round 1 shares a nonce");
            assert!(app.spend().unwrap().musig.is_some(), "the round is kept");
            let copies: Vec<usize> = app
                .inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-nonce-"))
                .map(|(i, _)| i)
                .collect();
            for i in copies {
                app.press(Action::Collect(i));
            }
            testkit::steps_before_sign_done(&mut app);
            testkit::press_on_worker(&mut app, Action::SignHere);
            testkit::settle(&mut app);
            let s = app.spend().unwrap();
            assert_eq!(s.error, None);
            assert!(!s.spend.signed_here.is_empty(), "round 2 signed");
            assert!(s.musig.is_none(), "nothing left to sign with");
        }

        /// What a person reads of the spend after a pass, apart from the
        /// bytes of nonces drawn at random.
        fn read_of(app: &Faraday) -> impl PartialEq + std::fmt::Debug {
            let s = app.spend().unwrap();
            (
                s.error.clone(),
                s.signers.clone(),
                s.to_sign.clone(),
                s.complete,
                s.open,
                s.spend.signed_here.clone(),
                s.spend.wrote.len(),
                s.musig.as_ref().map(|m| m.len()),
                app.outbox
                    .iter()
                    .map(|o| o.name.clone())
                    .collect::<Vec<_>>(),
            )
        }

        fn round(app: &mut Faraday, worker: bool) {
            if worker {
                testkit::steps_before_sign_done(app);
                testkit::press_on_worker(app, Action::SignHere);
                testkit::settle(app);
            } else {
                testkit::steps_before_sign_done(app);
                app.press(Action::SignHere);
            }
        }

        #[test]
        fn a_musig_round_one_and_two_signed_on_the_worker_read_the_same_as_inline() {
            let mut inline = device(true);
            let mut app = device(true);
            for a in [&mut inline, &mut app] {
                a.press(Action::StartSpend(at(a, "musig-unsigned.psbt")));
            }
            round(&mut inline, false);
            round(&mut app, true);
            assert_eq!(read_of(&app), read_of(&inline), "round one");
            assert!(our_nonce(&app).is_some());
            for a in [&mut inline, &mut app] {
                for i in nonce_copies(a) {
                    a.press(Action::Collect(i));
                }
            }
            round(&mut inline, false);
            round(&mut app, true);
            assert_eq!(read_of(&app), read_of(&inline), "round two");
            assert!(!app.spend().unwrap().spend.wrote.is_empty());
        }

        #[test]
        fn back_during_a_musig_round_shares_no_nonce_and_keeps_no_round() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::hold_worker();
            testkit::steps_before_sign_done(&mut app);
            testkit::press_on_worker(&mut app, Action::SignHere);
            app.press(Action::Back);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            assert!(our_nonce(&app).is_none(), "a nonce was shared");
            assert!(app.spend().unwrap().musig.is_none());
            // Pressed again, a new round opens and shares a nonce.
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            assert!(our_nonce(&app).is_some());
        }

        #[test]
        fn back_during_round_two_loses_the_round_and_the_next_press_shares_a_new_nonce() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let first = our_nonce(&app).expect("round 1 shares a nonce");
            let copies = nonce_copies(&app);
            for i in copies {
                app.press(Action::Collect(i));
            }
            testkit::hold_worker();
            testkit::steps_before_sign_done(&mut app);
            testkit::press_on_worker(&mut app, Action::SignHere);
            app.press(Action::Back);
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            assert!(app.outbox.iter().all(|o| !o.name.contains("signed")));
            if let Some(s) = app.spend() {
                assert!(s.musig.is_none(), "the round is lost");
                assert!(s.spend.signed_here.is_empty(), "nothing signed");
            }
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let second = our_nonce(&app).expect("a new round shares a nonce");
            assert_ne!(first, second, "the nonce is a new one");
            assert!(app.spend().is_some(), "the spend stays open");
        }

        /// The spend opened again from a cosigner's partial-signature file,
        /// which also carries this device's own part.
        fn reopened() -> (Faraday, Vec<usize>) {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let copies = nonce_copies(&app);
            app.press(Action::Collect(copies[0]));
            app.press(Action::Collect(copies[1]));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let partials: Vec<usize> = app
                .inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-partial-"))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(partials.len(), 2);
            (app, partials)
        }

        #[test]
        fn this_devices_own_part_in_a_reopened_file_is_unconfirmed_and_not_signed_again() {
            let (mut app, partials) = reopened();
            app.press(Action::StartSpend(partials[0]));
            let s = app.spend().expect("the spend is open");
            let mine = testkit::seed_fingerprint(0).0;
            assert!(s.signers.is_empty(), "nothing is confirmed");
            assert!(s.in_file.contains(&mine), "this device's own part");
            assert!(!s.to_sign.contains(&mine), "it is not offered to sign");
            assert_eq!((s.in_file.len(), s.to_sign.len()), (2, 1));
            assert!(!s.complete);
        }

        /// One file holding every part, made by merging this device's own
        /// file with the two cosigners'.
        fn every_part_in_one_file(app: &Faraday, partials: &[usize]) -> Vec<u8> {
            let mut all = app.spend().unwrap().spend.psbt.to_bytes();
            for i in partials {
                all = merged(&all, &app.inbox()[*i].bytes);
            }
            all
        }

        #[test]
        fn a_file_with_every_part_in_it_opens_aggregated() {
            let (app, partials) = reopened();
            let whole = every_part_in_one_file(&app, &partials);
            // Opened by a device holding a key, and by one holding none.
            for keyed in [true, false] {
                let mut other = if keyed {
                    device(true)
                } else {
                    let mut a = testkit::started();
                    a.storage(faraday_core::StorageEvent::Restored {
                        inbox: faraday_core::files_from(
                            testkit::files()
                                .unwrap()
                                .into_iter()
                                .filter(|(n, _)| n.starts_with("musig-"))
                                .collect(),
                        ),
                        outbox: Vec::new(),
                        kept: Vec::new(),
                    });
                    a.press(Action::LoadWallet(at(&a, "musig-wallet.txt")));
                    a
                };
                let c = testkit::compute();
                let kind = faraday_core::wallet::classify(&c, "whole.psbt", &whole);
                testkit::inbox_push(
                    &mut other,
                    faraday_core::Item {
                        name: "whole.psbt".to_string(),
                        bytes: (whole.clone()).into(),
                        kind,
                        secret: false,
                        picture: None,
                        sha: None,
                        facts: None,
                        wallet: None,
                        psbt: None,
                        outpoints: Vec::new(),
                        unsigned_txid: None,
                    },
                );
                let at_whole = other.inbox().len() - 1;
                other.press(Action::StartSpend(at_whole));
                let s = other.spend().expect("the spend is open");
                assert!(s.complete, "keyed {keyed}: opens complete");
                assert_eq!(
                    (s.signers.len(), s.in_file.len(), s.to_sign.len()),
                    (3, 0, 0),
                    "keyed {keyed}"
                );
            }
        }

        #[test]
        fn collecting_the_last_partial_aggregates_at_once_and_takes_back_the_part_file() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let copies = nonce_copies(&app);
            app.press(Action::Collect(copies[0]));
            app.press(Action::Collect(copies[1]));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let partials: Vec<usize> = app
                .inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-partial-"))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(partials.len(), 2);
            app.press(Action::Collect(partials[0]));
            assert!(!app.spend().unwrap().complete);
            assert!(
                app.outbox.iter().any(|o| o.name == "musig-part.psbt"),
                "the partial signatures so far are in the Outbox"
            );
            app.press(Action::Collect(partials[1]));
            let s = app.spend().expect("the spend stays open");
            assert!(s.complete, "the aggregation completed");
            assert_eq!(s.error, None);
            assert!(
                app.outbox.iter().all(|o| o.name != "musig-part.psbt"),
                "a finished spend leaves no part file for more signers"
            );
        }

        #[test]
        fn collecting_the_cosigners_nonces_signs_nothing_until_sign_is_pressed() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let shared = our_nonce(&app);
            for i in nonce_copies(&app) {
                app.press(Action::Collect(i));
            }
            let s = app.spend().unwrap();
            assert!(s.spend.signed_here.is_empty(), "Collect signed");
            assert!(s.musig.is_some(), "the nonce is still held for Sign");
            assert!(!s.to_sign.is_empty(), "Sign is offered");
            assert_eq!(our_nonce(&app), shared, "no new nonce was drawn");
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let s = app.spend().unwrap();
            assert_eq!(s.spend.signed_here.len(), 1, "the press signed");
            assert_eq!(s.error, None);
        }

        #[test]
        fn collecting_a_copy_with_our_own_part_changed_offers_sign_and_signs_nothing() {
            let (mut app, _) = reopened();
            let mine = testkit::seed_fingerprint(0).0;
            let own = app.spend().unwrap().spend.psbt.to_bytes();
            let shared = our_nonce(&app);
            let c = testkit::compute();
            let mut copy = faraday_psbt::Psbt::parse_bytes(&c, &own).unwrap();
            for (k, v) in copy.inner_mut().inputs[0].unknown.iter_mut() {
                if k.type_value == faraday_psbt::musig::IN_PARTIAL_SIG
                    && k.key[..33] == shared_key(&app)
                {
                    v[31] ^= 1;
                }
            }
            let bytes = copy.to_bytes();
            let kind = faraday_core::wallet::classify(&c, "copy.psbt", &bytes);
            testkit::inbox_push(
                &mut app,
                faraday_core::Item {
                    name: "copy.psbt".to_string(),
                    bytes: bytes.into(),
                    kind,
                    secret: false,
                    picture: None,
                    sha: None,
                    facts: None,
                    wallet: None,
                    psbt: None,
                    outpoints: Vec::new(),
                    unsigned_txid: None,
                },
            );
            let at_copy = app.inbox().len() - 1;
            let outbox = app.outbox.len();
            for _ in 0..2 {
                app.press(Action::Collect(at_copy));
                let s = app.spend().unwrap();
                assert!(s.failed.contains(&mine), "reads Does not verify");
                assert!(s.to_sign.contains(&mine), "Sign is offered");
                assert!(s.musig.is_none(), "Collect drew a nonce");
                assert_eq!(s.spend.signed_here.len(), 1, "Collect signed");
                assert_eq!(our_nonce(&app), shared, "no new nonce");
                assert!(app.outbox.len() <= outbox + 1);
            }
        }

        /// This device's account key, as the first 33 bytes of an entry.
        fn shared_key(app: &Faraday) -> [u8; 33] {
            let c = testkit::compute();
            app.session
                .keys
                .master_for_tests(0)
                .derive(&c, &"m/86'/1'/0'".parse().unwrap())
                .to_xpub(&c)
                .public_key
                .serialize()
        }

        /// Three devices (test keys 0, 1, 2) on the MuSig2 spend.
        fn trio(n: usize, entropy: u8) -> Faraday {
            let mut app = testkit::started();
            app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
                [entropy; 32],
            )));
            app.storage(faraday_core::StorageEvent::Restored {
                inbox: faraday_core::files_from(
                    testkit::files()
                        .unwrap()
                        .into_iter()
                        .filter(|(n, _)| n.starts_with("musig-"))
                        .collect(),
                ),
                outbox: Vec::new(),
                kept: Vec::new(),
            });
            app.press(Action::LoadWallet(at(&app, "musig-wallet.txt")));
            app.press(Action::Entry(None));
            for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
                app.event(Event::Key(faraday_shell_api::Key::Char(c)));
            }
            app.press(Action::EntryAdd);
            app
        }

        fn file_of(app: &Faraday) -> Vec<u8> {
            app.spend().unwrap().spend.psbt.to_bytes()
        }

        fn merged(a: &[u8], b: &[u8]) -> Vec<u8> {
            let c = testkit::compute();
            let mut x = faraday_psbt::Psbt::parse_bytes(&c, a).unwrap();
            let y = faraday_psbt::Psbt::parse_bytes(&c, b).unwrap();
            x.inner_mut().combine(y.inner().clone()).unwrap();
            x.to_bytes()
        }

        fn give(app: &mut Faraday, bytes: Vec<u8>) {
            let c = testkit::compute();
            let kind = faraday_core::wallet::classify(&c, "next.psbt", &bytes);
            testkit::inbox_push(
                app,
                faraday_core::Item {
                    name: "next.psbt".to_string(),
                    bytes: bytes.into(),
                    kind,
                    secret: false,
                    picture: None,
                    sha: None,
                    facts: None,
                    wallet: None,
                    psbt: None,
                    outpoints: Vec::new(),
                    unsigned_txid: None,
                },
            );
            let last = app.inbox().len() - 1;
            app.press(Action::Collect(last));
        }

        /// Nonces exchanged among three devices; nothing signed yet.
        fn nonces_exchanged() -> Vec<Faraday> {
            let mut all: Vec<Faraday> = (0..3).map(|n| trio(n, 5 + n as u8)).collect();
            let mut shared = Vec::new();
            for a in all.iter_mut() {
                a.press(Action::StartSpend(at(a, "musig-unsigned.psbt")));
                testkit::steps_before_sign_done(a);
                a.press(Action::SignHere);
                shared.push(file_of(a));
            }
            let every = merged(&merged(&shared[0], &shared[1]), &shared[2]);
            for a in all.iter_mut() {
                give(a, every.clone());
            }
            all
        }

        /// Device 1 signs; device 2 locks, comes back and shares a new nonce.
        /// The coordinator merges the two files in `new_first` order.
        fn restarted_cosigner(new_first: bool) -> (Vec<Faraday>, Vec<u8>) {
            let mut all = nonces_exchanged();
            let every = file_of(&all[2]);
            all[1].press(Action::SignHere);
            let old = file_of(&all[1]);
            let mut again = trio(2, 77);
            give(&mut again, every);
            let i = again.inbox().len() - 1;
            again.press(Action::StartSpend(i));
            testkit::steps_before_sign_done(&mut again);
            again.press(Action::SignHere);
            let new = file_of(&again);
            all[2] = again;
            let m = if new_first {
                merged(&new, &old)
            } else {
                merged(&old, &new)
            };
            (all, m)
        }

        #[test]
        fn a_cosigner_who_shared_a_new_nonce_is_not_refused_when_the_older_nonce_wins_the_merge() {
            let (mut all, m) = restarted_cosigner(true);
            let k1 = testkit::seed_fingerprint(1).0;
            give(&mut all[2], m);
            all[2].press(Action::SignHere);
            let s = all[2].spend().unwrap();
            assert_eq!(s.error, None, "Sign is not refused");
            assert!(!s.in_file.contains(&k1), "the cosigner's old part is gone");
            assert!(s.to_sign.contains(&k1), "the cosigner is asked again");
            assert!(s.spend.signed_here.is_empty(), "nothing signed under it");
            assert!(s.musig.is_some(), "the nonce stays held");
        }

        fn nonce_count(app: &Faraday) -> usize {
            faraday_psbt::musig::pub_nonces(&app.spend().unwrap().spend.psbt.inner().inputs[0])
                .unwrap()
                .len()
        }

        #[test]
        fn a_mend_sends_the_part_file_on_and_drops_the_nonce_of_the_partial_it_removed() {
            let (mut all, m) = restarted_cosigner(true);
            give(&mut all[2], m);
            assert_eq!(nonce_count(&all[2]), 3);
            let parts = all[2].outbox.len();
            all[2].press(Action::SignHere);
            let s = all[2].spend().unwrap();
            assert_eq!(s.error, None);
            assert_eq!(s.open, Some(9), "the screen moves on to the nonces");
            assert_eq!(
                nonce_count(&all[2]),
                2,
                "the spent nonce went with the partial"
            );
            assert!(s.spend.signed_here.is_empty());
            let part = all[2]
                .outbox
                .iter()
                .rev()
                .find(|o| o.name.ends_with("part.psbt"))
                .expect("the part file is in the Outbox");
            assert_eq!(
                *part.bytes,
                file_of(&all[2]),
                "the file that goes out is the mended one"
            );
            assert!(all[2].outbox.len() >= parts);
            let mended = file_of(&all[2]);
            all[2].press(Action::SignHere);
            assert_eq!(file_of(&all[2]), mended, "a second press signs nothing");
            assert!(all[2].spend().unwrap().spend.signed_here.is_empty());
        }

        #[test]
        fn a_round_mended_and_passed_on_completes() {
            let (mut all, m) = restarted_cosigner(true);
            give(&mut all[2], m);
            all[2].press(Action::SignHere);
            let mended = file_of(&all[2]);
            for (at, a) in all.iter_mut().enumerate() {
                if at != 2 {
                    give(a, mended.clone());
                }
            }
            for _ in 0..8 {
                for at in 0..3 {
                    all[at].press(Action::SignHere);
                    let f = file_of(&all[at]);
                    for (o, a) in all.iter_mut().enumerate() {
                        if o != at {
                            give(a, f.clone());
                        }
                    }
                }
                if all.iter().any(|a| a.spend().unwrap().complete) {
                    return;
                }
            }
            panic!("the round did not complete after the mend");
        }

        #[test]
        fn collecting_a_new_nonce_on_a_device_that_has_signed_neither_signs_nor_draws_one() {
            let (mut all, m) = restarted_cosigner(false);
            let before = file_of(&all[1]);
            assert_eq!(all[1].spend().unwrap().spend.signed_here.len(), 1);
            give(&mut all[1], m);
            let s = all[1].spend().unwrap();
            assert_eq!(s.spend.signed_here.len(), 1, "Collect signed again");
            assert_eq!(s.error, None);
            assert!(s.musig.is_none(), "Collect drew a nonce");
            let own = |b: &[u8]| {
                let p = faraday_psbt::Psbt::parse_bytes(&testkit::compute(), b).unwrap();
                faraday_psbt::musig::partial_sigs(&p.inner().inputs[0])
                    .unwrap()
                    .into_iter()
                    .map(|e| (e.participant.serialize(), e.value.serialize()))
                    .collect::<Vec<_>>()
            };
            let nonces = |b: &[u8]| {
                let p = faraday_psbt::Psbt::parse_bytes(&testkit::compute(), b).unwrap();
                faraday_psbt::musig::pub_nonces(&p.inner().inputs[0])
                    .unwrap()
                    .into_iter()
                    .map(|e| (e.participant.serialize(), e.value.serialize()))
                    .collect::<Vec<_>>()
            };
            let k1_key = faraday_psbt::musig::partial_sigs(
                &faraday_psbt::Psbt::parse_bytes(&testkit::compute(), &before)
                    .unwrap()
                    .inner()
                    .inputs[0],
            )
            .unwrap()[0]
                .participant
                .serialize();
            assert_eq!(
                nonces(&before).into_iter().find(|n| n.0 == k1_key),
                nonces(&file_of(&all[1]))
                    .into_iter()
                    .find(|n| n.0 == k1_key),
                "the device's own nonce is kept"
            );
            let was = own(&before);
            let now = own(&file_of(&all[1]));
            assert!(
                was.iter().all(|w| now.contains(w)),
                "the device's own partial signature is not replaced"
            );
        }

        fn sign_card_text(app: &mut Faraday) -> Vec<String> {
            let steps = app.spend().unwrap().steps.clone();
            for n in steps {
                if n == 5 {
                    break;
                }
                app.press(Action::StepNext(n));
            }
            app.press(Action::Step(5));
            app.drawn_texts()
        }

        #[test]
        fn the_sign_card_names_no_key_as_signed_while_it_is_still_to_sign() {
            let (mut all, m) = restarted_cosigner(false);
            let signed = |t: &[String]| t.iter().any(|l| l.starts_with("Signed with"));
            assert!(
                signed(&sign_card_text(&mut all[1])),
                "a key that signed is named"
            );
            give(&mut all[1], m);
            let k1 = testkit::seed_fingerprint(1).0;
            assert!(all[1].spend().unwrap().to_sign.contains(&k1));
            let t = sign_card_text(&mut all[1]);
            assert!(!signed(&t), "a key still to sign is named as signed: {t:?}");
            assert!(
                t.iter().any(|l| l.starts_with("Sign with")),
                "the card offers Sign: {t:?}"
            );
        }

        #[test]
        fn a_round_where_the_new_nonce_wins_the_merge_completes_after_the_presses() {
            let (mut all, m) = restarted_cosigner(false);
            for a in all.iter_mut() {
                give(a, m.clone());
            }
            for _ in 0..8 {
                let mut files = Vec::new();
                for a in all.iter_mut() {
                    testkit::steps_before_sign_done(a);
                    a.press(Action::SignHere);
                    files.push(file_of(a));
                }
                for (at, a) in all.iter_mut().enumerate() {
                    for (from, f) in files.iter().enumerate() {
                        if from != at {
                            give(a, f.clone());
                        }
                    }
                }
                if all.iter().any(|a| a.spend().unwrap().complete) {
                    return;
                }
            }
            panic!("the round did not complete");
        }

        /// A completed aggregation takes off the stick its own part-signed
        /// file and no other transaction's of the same name.
        #[test]
        fn a_completed_aggregation_removes_only_its_own_part_file() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            for i in nonce_copies(&app) {
                app.press(Action::Collect(i));
            }
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let partials: Vec<usize> = app
                .inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-partial-"))
                .map(|(i, _)| i)
                .collect();
            // The file of that name now belongs to another transaction.
            for o in app
                .outbox
                .iter_mut()
                .filter(|o| o.name == "musig-part.psbt")
            {
                o.bytes = (b"psbt of another transaction".to_vec()).into();
            }
            app.press(Action::Collect(partials[0]));
            let names = |app: &Faraday| -> Vec<String> {
                app.outbox
                    .iter()
                    .filter(|o| o.name.starts_with("musig-part"))
                    .map(|o| o.name.clone())
                    .collect()
            };
            assert_eq!(names(&app), ["musig-part.psbt", "musig-part-2.psbt"]);
            app.press(Action::Collect(partials[1]));
            assert!(app.spend().unwrap().complete, "the aggregation completed");
            assert_eq!(names(&app), ["musig-part.psbt"]);
        }

        /// Signed here, the same file opened again, then a file holding every
        /// part collected; `other` has another transaction at the part file's
        /// name by then. The part files left in the Outbox.
        fn part_files_after_a_reopened_spend_completes(other: bool) -> Vec<Vec<u8>> {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            for i in nonce_copies(&app) {
                app.press(Action::Collect(i));
            }
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
            let partials: Vec<usize> = app
                .inbox()
                .iter()
                .enumerate()
                .filter(|(_, i)| i.name.starts_with("musig-partial-"))
                .map(|(i, _)| i)
                .collect();
            let whole = every_part_in_one_file(&app, &partials);
            let another = testkit::musig_unsigned_at(1).unwrap().to_bytes();
            let waiting = app
                .outbox
                .iter_mut()
                .find(|o| o.name == "musig-part.psbt")
                .expect("the part file is in the Outbox");
            if other {
                waiting.bytes = (another).into();
            }
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            give(&mut app, whole);
            assert!(app.spend().unwrap().complete, "the spend completed");
            app.outbox
                .iter()
                .filter(|o| o.name.starts_with("musig-part"))
                .map(|o| o.bytes.to_vec())
                .collect()
        }

        #[test]
        fn a_reopened_spend_that_completes_takes_back_its_part_file() {
            assert!(part_files_after_a_reopened_spend_completes(false).is_empty());
        }

        #[test]
        fn a_reopened_spend_that_completes_leaves_another_transactions_file() {
            assert_eq!(
                part_files_after_a_reopened_spend_completes(true),
                [testkit::musig_unsigned_at(1).unwrap().to_bytes()]
            );
        }
    }
}

/// Collect on the open spend, as a job on the worker (`docs/WORKERS.md`
/// §2.1): what it gives is what the press gives inline.
mod collect {
    use super::*;
    use crate::common::add_key;
    use faraday_core::StorageEvent;
    use faraday_core::testkit::Kit;

    fn kit(id: &str) -> Kit {
        testkit::kits().into_iter().find(|k| k.id == id).unwrap()
    }

    fn at(app: &Faraday, name: &str) -> usize {
        app.inbox().iter().position(|i| i.name == name).unwrap()
    }

    /// A device holding test key 1 with the kit's wallet loaded and its
    /// transaction open, and the other signer's copy in the Inbox as
    /// `from-a.psbt`, an unrelated transaction as `other.psbt` and
    /// the open one again as `same.psbt`. `signed` has this device sign first.
    fn device(id: &str, signed: bool) -> Faraday {
        device_with(id, signed, |bytes| bytes)
    }

    /// `device`, the open transaction's file `spend.psbt` written as `layout`
    /// writes the bytes of the plain file.
    fn device_with(id: &str, signed: bool, layout: fn(Vec<u8>) -> Vec<u8>) -> Faraday {
        let k = kit(id);
        let (unsigned, cosigned) = testkit::spend_of(&k).unwrap();
        let (_, theirs) = cosigned.expect("a kit with a cosigner");
        let other = testkit::unsigned(&kit("spending")).unwrap();
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![
                (
                    faraday_text::Text::of("spend.psbt"),
                    layout(unsigned.to_bytes()),
                ),
                (faraday_text::Text::of("same.psbt"), unsigned.to_bytes()),
                (faraday_text::Text::of("other.psbt"), other.to_bytes()),
                (faraday_text::Text::of("from-a.psbt"), theirs.to_bytes()),
                (
                    faraday_text::Text::of("wallet.txt"),
                    k.descriptor.into_bytes(),
                ),
            ]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::LoadWallet(at(&app, "wallet.txt")));
        add_key(&mut app, "bacon");
        app.press(Action::StartSpend(at(&app, "spend.psbt")));
        if signed {
            testkit::steps_before_sign_done(&mut app);
            app.press(Action::SignHere);
        }
        app
    }

    /// What a person reads of the spend and the Outbox.
    fn seen(app: &Faraday) -> String {
        let s = app.spend().expect("the spend is open");
        let outbox: Vec<_> = app
            .outbox
            .iter()
            .map(|i| format!("{} {:?}", i.name, i.bytes))
            .collect();
        format!(
            "{:?}\n{:?} {:?} {:?} {} {:?} {:?} {:?} {:?}\n{:?}\n{:?}",
            s.spend.psbt.to_bytes(),
            s.signers,
            s.to_sign,
            s.in_file,
            s.complete,
            s.open,
            s.error,
            s.spend.collected,
            s.spend.signed_here,
            outbox,
            app.toast_text(),
        )
    }

    fn on_worker(app: &mut Faraday, name: &str) {
        let i = at(app, name);
        testkit::press_on_worker(app, Action::Collect(i));
    }

    fn same_as_inline(id: &str, signed: bool, file: &str) {
        let mut inline = device(id, signed);
        let i = at(&inline, file);
        inline.press(Action::Collect(i));

        let mut app = device(id, signed);
        on_worker(&mut app, file);
        assert!(app.busy().is_some(), "the screen is busy while it collects");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        assert_eq!(seen(&app), seen(&inline));
    }

    #[test]
    fn a_two_of_three_collected_on_the_worker_reads_as_it_does_inline() {
        same_as_inline("savings", false, "from-a.psbt");
        same_as_inline("savings", true, "from-a.psbt");
    }

    #[test]
    fn a_taproot_multisig_collected_on_the_worker_reads_as_it_does_inline() {
        same_as_inline("taproot-multisig", false, "from-a.psbt");
        same_as_inline("taproot-multisig", true, "from-a.psbt");
    }

    #[test]
    fn a_collect_that_completes_the_spend_goes_to_finish_as_it_does_inline() {
        let mut app = device("savings", true);
        on_worker(&mut app, "from-a.psbt");
        testkit::settle(&mut app);
        let s = app.spend().unwrap();
        assert!(s.complete);
        // Review comes first, also for a PSBT the collect made complete.
        assert_eq!(s.open, Some(faraday_core::wallet::step::TRANSACTION));
    }

    #[test]
    fn a_refused_file_says_on_the_worker_what_it_says_inline() {
        same_as_inline("savings", false, "other.psbt");
        let mut app = device("savings", false);
        on_worker(&mut app, "other.psbt");
        testkit::settle(&mut app);
        assert!(app.spend().unwrap().error.is_some());
    }

    #[test]
    fn a_file_with_nothing_new_says_so_as_it_does_inline() {
        same_as_inline("savings", false, "same.psbt");
        let mut app = device("savings", false);
        on_worker(&mut app, "same.psbt");
        testkit::settle(&mut app);
        let said = app.spend().unwrap().error.clone().unwrap();
        assert!(said.contains("same.psbt"), "{said}");
    }

    /// The file with an explicit `PSBT_GLOBAL_VERSION` of 0 after its
    /// transaction, which the reader keeps as written.
    fn with_explicit_version(mut bytes: Vec<u8>) -> Vec<u8> {
        // After the magic: key length 1, key type 0, a compact-size length
        // and the transaction.
        let (len, at) = match bytes[7] {
            0xfd => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
            n => (n as usize, 8),
        };
        let end = at + len;
        bytes.splice(end..end, [0x01, 0xfb, 0x04, 0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn a_file_with_nothing_new_to_a_spend_read_in_another_layout_says_so_and_writes_no_part() {
        let plain = device("savings", false)
            .spend()
            .unwrap()
            .spend
            .psbt
            .to_bytes();
        let mut inline = device_with("savings", false, with_explicit_version);
        assert_ne!(
            inline.spend().unwrap().spend.psbt.to_bytes(),
            plain,
            "the spend was read in another layout"
        );
        let i = at(&inline, "same.psbt");
        inline.press(Action::Collect(i));
        let said = inline.spend().unwrap().error.clone().unwrap();
        let nothing = strings::fill(strings::EN.collect_has_nothing, &["same.psbt"]);
        assert_eq!(said, nothing);
        assert!(inline.outbox.is_empty());

        let mut app = device_with("savings", false, with_explicit_version);
        on_worker(&mut app, "same.psbt");
        testkit::settle(&mut app);
        let said = app.spend().unwrap().error.clone().unwrap();
        assert_eq!(said, nothing);
        assert!(app.outbox.is_empty());
        assert_eq!(seen(&app), seen(&inline));
    }

    #[test]
    fn back_during_a_collect_leaves_the_psbt_and_the_outbox_as_they_were() {
        let mut app = device("savings", false);
        let before = seen(&app);
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        app.press(Action::Back);
        assert!(app.busy().is_none());
        // The same transaction opened again, so that a stale answer
        // applied would have a spend to merge into.
        app.press(Action::StartSpend(at(&app, "spend.psbt")));
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert_eq!(seen(&app), before);
        assert!(app.outbox.is_empty());
    }

    #[test]
    fn a_lock_during_a_collect_leaves_nothing_open_and_nothing_in_the_outbox() {
        let mut app = device("savings", false);
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        app.press(Action::LockUnsealed);
        assert!(app.busy().is_none());
        assert_eq!(app.session.keys.len(), 0, "the lock forgot the key");
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert_eq!(app.session.keys.len(), 0);
        assert!(app.spend().is_none());
        assert!(app.outbox.is_empty());
    }

    #[test]
    fn a_second_collect_while_one_runs_asks_no_job() {
        let mut app = device("savings", false);
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        let asked = testkit::jobs_asked(&app);
        on_worker(&mut app, "same.psbt");
        assert_eq!(testkit::jobs_asked(&app), asked);
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(app.spend().unwrap().error.is_none());
    }

    #[test]
    fn the_presses_a_collect_leaves_alone_do_nothing_while_it_runs() {
        let mut app = device("savings", false);
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        let asked = testkit::jobs_asked(&app);
        for a in [
            Action::SignHere,
            Action::Primary,
            Action::ToggleTable,
            Action::ReviewHold,
            Action::SignedToOutbox,
            Action::TxToOutbox,
            Action::Step(faraday_core::wallet::step::SIGN),
        ] {
            testkit::press_on_worker(&mut app, a);
        }
        assert_eq!(testkit::jobs_asked(&app), asked);
        assert!(app.spend().unwrap().spend.signed_here.is_empty());
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(app.spend().unwrap().spend.signed_here.is_empty());
        assert!(app.spend().unwrap().error.is_none());
    }

    #[test]
    fn a_file_arriving_while_it_runs_does_not_stop_it_applying() {
        let mut app = device("savings", false);
        let mut inline = device("savings", false);
        let i = at(&inline, "from-a.psbt");
        inline.press(Action::Collect(i));
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        let mut inbox: Vec<_> = app
            .inbox()
            .iter()
            .map(|i| (i.name.clone(), i.bytes.to_vec()))
            .collect();
        inbox.push(("late.psbt".to_string(), vec![1, 2, 3]));
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(inbox),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(
            app.spend().unwrap().spend.psbt.to_bytes(),
            inline.spend().unwrap().spend.psbt.to_bytes()
        );
    }

    #[test]
    fn a_key_randomness_reached_while_it_collects_is_blinded_when_the_job_ends() {
        let mut app = device("savings", false);
        app.event(faraday_shell_api::Event::Entropy(
            faraday_shell_api::EntropyBytes::new([0x11; 32]),
        ));
        let before = app.session.keys.master_for_tests(0).blindings();
        testkit::hold_worker();
        on_worker(&mut app, "from-a.psbt");
        let mut rolls = faraday_entropy::DiceRolls::new();
        for r in 1..=6 {
            assert!(rolls.push(r));
        }
        assert!(app.own_add(faraday_core::fresh::OwnSource::Dice, &rolls.commitment(), 6));
        assert_eq!(
            app.session.keys.master_for_tests(0).blindings(),
            before,
            "the key the job holds is not changed under it"
        );
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(
            app.session.keys.master_for_tests(0).blindings(),
            before + 1,
            "blinded as the answer was applied"
        );
        assert!(!app.session.keys.owes_reblind());
    }

    #[test]
    fn a_musig_round_held_here_is_intact_after_a_collect_on_the_worker_and_signs() {
        use crate::rounds::{at, device, our_nonce};
        let mut app = device(true);
        app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
        testkit::steps_before_sign_done(&mut app);
        app.press(Action::SignHere);
        let nonce = our_nonce(&app).expect("round 1 shares a nonce");
        let states = app.spend().unwrap().signers.clone();
        let copies: Vec<usize> = app
            .inbox()
            .iter()
            .enumerate()
            .filter(|(_, i)| i.name.starts_with("musig-nonce-"))
            .map(|(i, _)| i)
            .collect();
        assert!(!copies.is_empty());
        for i in copies {
            testkit::press_on_worker(&mut app, Action::Collect(i));
            testkit::settle(&mut app);
        }
        let s = app.spend().unwrap();
        assert!(s.musig.is_some(), "the round is kept");
        assert_eq!(s.signers, states);
        assert_eq!(our_nonce(&app), Some(nonce));
        testkit::steps_before_sign_done(&mut app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        testkit::settle(&mut app);
        let s = app.spend().unwrap();
        assert_eq!(s.error, None);
        assert!(!s.spend.signed_here.is_empty(), "round 2 signed");
    }
}
