//! A vault's keys show the same fingerprints whichever thread works them
//! out: an unlock works every key record's fingerprint out on the worker,
//! so the main thread stretches no key when the answer is applied, and a
//! second copy of an open vault leaves nothing behind.

use faraday_shell_api::{App, Event};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

use super::type_text;
use crate::vault::VaultAction as V;
use crate::work::{Work, since, snapshot};
use crate::{Action, Faraday, Screen, StorageEvent, testkit};

fn seed(word: &str, passphrase: Option<&str>) -> Record {
    let c = testkit::compute();
    let m = faraday_keys::words::parse(
        &c,
        faraday_bip::bip39::Language::English,
        &testkit::test_words(word),
    )
    .expect("words");
    let r = Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m));
    match passphrase {
        Some(p) => r.with(field::KEY_PASSPHRASE, p.as_bytes()),
        None => r,
    }
}

/// A vault of a seed alone, the same seed with a passphrase, another
/// seed and the first seed kept again, with `extra`.
fn vault(extra: Option<Record>) -> Vec<u8> {
    let mut rs = vec![
        seed("bacon", None),
        seed("bacon", Some("a passphrase")),
        seed("zebra", None),
        seed("bacon", None).with(field::KEY_LABEL, b"Again"),
    ];
    rs.extend(extra);
    testkit::kdbx_vault("Keys", testkit::VAULT_PASSPHRASE, 0x41, rs).expect("a vault")
}

fn app_with(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.vaults.set_speed(Some(180));
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    app.press(Action::Nav(Screen::Vaults));
    app
}

/// Unlocks the file `name` and waits for its answer, the job on the
/// worker's thread.
fn unlock(app: &mut Faraday, name: &str) {
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("listed");
    app.press(Action::Vault(V::Open(n)));
    type_text(app, testkit::VAULT_PASSPHRASE);
    let opened = app.vaults.open.len();
    app.press_as(Action::Vault(V::Unlock), false);
    for t in 1..2000u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        if app.vaults.unlock_error.is_some() || app.vaults.open.len() > opened {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("no answer came");
}

#[test]
fn an_unlock_shows_each_keys_fingerprint_and_stretches_no_key_to_show_them() {
    let c = testkit::compute();
    let mut app = app_with(vec![("vault.kdbx", vault(None))]);
    unlock(&mut app, "vault.kdbx");
    assert_eq!(app.vaults.open.len(), 1, "the vault opened");
    // What the job counted is brought home with its answer, so the counts
    // cannot say which thread did it: what is shown after is found kept.
    let before = snapshot();
    let _ = app.frame();
    let o = app.vaults.open[0].id;
    for (_, r) in app.vaults.records(o).of(kind::KEY) {
        assert!(crate::vault::screen::key_identity(&c, &app, r).is_some());
    }
    let after = since(&before);
    let kept = app.vaults.fingerprints.borrow().clone();
    assert_eq!(
        kept.len(),
        3,
        "a seed alone, with a passphrase, and another; its second record is the first's"
    );
    let network = app.session.network();
    for (_, r) in app.vaults.records(o).of(kind::KEY) {
        let id = faraday_keys::vault::key_record_id(r).expect("an id");
        let (fp, mid) = faraday_keys::vault::key_identity(&c, network, r).expect("a key");
        assert!(
            kept.iter().any(|(k, f, m)| *k == id
                && f.as_str() == &*crate::wallet::fp_text(fp)
                && *m == mid),
            "the fingerprint shown is the one worked out fresh"
        );
    }
    assert_eq!(after.of(Work::Kdf), 0, "each key is found worked out");
    assert_eq!(after.of(Work::MasterKey), 0);
}

#[test]
fn a_second_copy_of_an_open_vault_is_refused_and_keeps_nothing_of_its_own() {
    let files = vec![
        ("vault.kdbx", vault(None)),
        ("copy.kdbx", vault(Some(seed("summer", None)))),
    ];
    let mut app = app_with(files);
    unlock(&mut app, "vault.kdbx");
    let held = app.vaults.fingerprints.borrow().clone();
    assert_eq!(app.vaults.open.len(), 1);
    app.sheet = None;
    app.press(Action::Nav(Screen::Vaults));
    unlock(&mut app, "copy.kdbx");
    assert_eq!(app.vaults.open.len(), 1, "the second copy was refused");
    assert!(app.vaults.unlock_error.is_some(), "and says so");
    assert_eq!(
        *app.vaults.fingerprints.borrow(),
        held,
        "no key of the refused copy is kept"
    );
}
