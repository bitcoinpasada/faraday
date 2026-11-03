//! The worker that runs the costly part of a press (`docs/WORKERS.md`):
//! Load ticked on Practice's samples, whose answer is applied on a later
//! pass.

use faraday_core::samples::Pick;
use faraday_core::testkit;
use faraday_core::{Action, DESKTOP_MODES, Faraday};
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
            inbox: vec![("vault.fvlt".into(), vault("Test", &[PASS], 7))],
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
            id: "stick".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }])
    }

    #[test]
    fn a_stick_put_in_during_an_open_leaves_the_vault_locked() {
        // What the press says with the stick already in.
        let mut before = device();
        before.storage(a_stick());
        asked(&mut before, PASS);
        before.press(Action::Vault(V::Unlock));
        assert!(before.vaults().unlock_error.is_some());

        let mut app = device();
        asked(&mut app, PASS);
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
        app.storage(a_stick());
        testkit::release_worker();
        testkit::settle(&mut app);
        assert!(app.vaults().open.is_empty());
        assert_eq!(app.vaults().unlock_error, before.vaults().unlock_error);
    }

    #[test]
    fn cancel_on_the_lock_sheet_leaves_a_running_open_running() {
        let mut app = testkit::started();
        app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
        testkit::vault_speed(&mut app, Some(100));
        app.storage(StorageEvent::Restored {
            inbox: vec![
                ("vault.fvlt".into(), vault("Test", &[PASS], 7)),
                ("second.fvlt".into(), vault("Second", &[PASS], 8)),
            ],
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
            inbox: files(),
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
        inline.press(Action::SignHere);

        let mut app = unsigned_spend();
        testkit::press_on_worker(&mut app, Action::SignHere);
        assert!(app.busy().is_some(), "the screen is busy while it signs");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        let (a, b) = (app.spend().unwrap(), inline.spend().unwrap());
        assert!(a.spend.finished.is_some());
        assert_eq!(a.spend.psbt.to_bytes(), b.spend.psbt.to_bytes());
        assert_eq!(a.spend.signed_here, b.spend.signed_here);
        assert!(!app.outbox.is_empty());
    }

    #[test]
    fn a_transaction_no_key_here_signs_says_what_it_says_inline() {
        let mut inline = inbox_of();
        open(&mut inline, false);
        inline.press(Action::SignHere);
        let said = inline.spend().unwrap().error.clone();
        assert!(said.is_some());

        let mut app = inbox_of();
        open(&mut app, false);
        let asked = testkit::jobs_asked(&app);
        testkit::press_on_worker(&mut app, Action::SignHere);
        assert!(app.busy().is_none(), "a refusal starts no job");
        assert_eq!(testkit::jobs_asked(&app), asked);
        assert_eq!(app.spend().unwrap().error, said);
    }

    #[test]
    fn back_during_signing_leaves_it_unsigned() {
        let mut app = unsigned_spend();
        testkit::hold_worker();
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
        testkit::press_on_worker(&mut app, Action::SignHere);
        app.press(Action::LockUnsealed);
        assert!(app.busy().is_none());
        assert_eq!(app.session.keys.len(), 0, "the lock forgot the key");
        // Unlocked and back at the same transaction before the answer.
        app.storage(StorageEvent::Restored {
            inbox: files(),
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
            testkit::press_on_worker(&mut app, Action::SignHere);
            testkit::settle(&mut app);
            let s = app.spend().unwrap();
            assert_eq!(s.error, None);
            assert!(!s.spend.signed_here.is_empty(), "round 2 signed");
            assert!(s.musig.is_none(), "nothing left to sign with");
        }

        #[test]
        fn back_during_a_musig_round_shares_no_nonce_and_keeps_no_round() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::hold_worker();
            testkit::press_on_worker(&mut app, Action::SignHere);
            app.press(Action::Back);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            assert!(our_nonce(&app).is_none(), "a nonce was shared");
            assert!(app.spend().unwrap().musig.is_none());
            // Pressed again, a new round opens and shares a nonce.
            app.press(Action::SignHere);
            assert!(our_nonce(&app).is_some());
        }

        #[test]
        fn back_during_round_two_loses_the_round_and_the_next_press_shares_a_new_nonce() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            app.press(Action::SignHere);
            let first = our_nonce(&app).expect("round 1 shares a nonce");
            let copies = nonce_copies(&app);
            for i in copies {
                app.press(Action::Collect(i));
            }
            testkit::hold_worker();
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
            app.press(Action::SignHere);
            let second = our_nonce(&app).expect("a new round shares a nonce");
            assert_ne!(first, second, "the nonce is a new one");
            assert!(app.spend().is_some(), "the spend stays open");
        }

        #[test]
        fn back_during_the_aggregation_leaves_the_spend_as_collect_made_it() {
            let mut app = device(true);
            app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
            app.press(Action::SignHere);
            let copies = nonce_copies(&app);
            app.press(Action::Collect(copies[0]));
            app.press(Action::Collect(copies[1]));
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
            testkit::hold_worker();
            testkit::press_on_worker(&mut app, Action::Collect(partials[1]));
            app.press(Action::Back);
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            let s = app.spend().expect("the spend stays open");
            assert!(!s.complete);
            assert_eq!(s.error, None);
            assert!(
                app.outbox.iter().any(|o| o.name == "musig-part.psbt"),
                "the merged partial signatures are in the Outbox"
            );
            // Pressed again, the aggregation completes.
            app.press(Action::Collect(partials[1]));
            assert!(app.spend().unwrap().complete, "the aggregation completed");
        }
    }
}
