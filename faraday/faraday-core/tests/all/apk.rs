//! An APK signing key made in a vault and used (`docs/APK-SIGNING.md` §8):
//! Make an APK key, the wait for two written copies, Sign an APK with every
//! refusal of §4.4, the typed second build, and a private key that never
//! reaches a file For the stick.

use faraday_apk::zip::{Apk, Refusal, Slice};
use faraday_apk::{key, pem};
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_core::backup_flow::Kind;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
use faraday_vault::records::{field, kind};

use crate::common::{Disk, Shell, type_text};

type Files = Vec<(String, Vec<u8>)>;

const FIXTURE: &str = "app.faradayoffline.fixture";
const OTHER: &str = "app.faradayoffline.other";

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/apk")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn entropy(app: &mut Faraday, n: u8) {
    for i in 0..n {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
}

fn display(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The vault the tests keep keys in: a label and nothing that loads, so
/// the process stays clean and takes a stick.
const PASSPHRASE: &str = "apk test";

fn vault_file() -> (String, Vec<u8>) {
    let c = testkit::compute();
    let contents = faraday_vault::Contents {
        records: vec![faraday_vault::Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Keys")],
    };
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let bytes = faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PASSPHRASE.as_bytes()],
        &[contents],
        &[0x7a; 32],
    )
    .unwrap();
    ("apk.ofv".into(), bytes)
}

/// A process restored with these files in Files and the clock known.
fn process(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    app.apk_signing = true;
    display(&mut app);
    entropy(&mut app, 8);
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    app.vaults.ms_per_unit = Some(180);
    app
}

/// The vault unlocked on its APK signing keys, the clock checked.
fn unlock(mut app: Faraday) -> Faraday {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(6)));
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    app
}

fn unlocked(apks: &[(&str, Vec<u8>)]) -> Faraday {
    let mut inbox = vec![vault_file()];
    inbox.extend(apks.iter().map(|(n, b)| (n.to_string(), b.clone())));
    unlock(process(inbox, Vec::new(), Vec::new()))
}

/// A key in the vault as one unlocked from a stick holds it: no backup
/// flow made it, so it signs at once. Its page is selected.
fn make(app: &mut Faraday, name: &str, package: &str) {
    use std::sync::atomic::{AtomicU8, Ordering};
    static SEED: AtomicU8 = AtomicU8::new(0x61);
    let seed = [SEED.fetch_add(1, Ordering::Relaxed); 32];
    let name = name.trim();
    let made = faraday_keys::apk::create(&seed, name, package, 1_791_482_652).unwrap();
    let cur = app.vaults.current;
    {
        let id = app.vaults.open[cur].id;
        app.vaults.edit(id, |s| s.insert(id, &made.key));
    }
    app.vaults.open[cur].changes += 1;
    let n = app
        .vaults
        .store()
        .contents_for_tests(app.vaults.open[cur].id)
        .of(kind::APK_KEY)
        .count();
    app.press(Action::Vault(V::Category(6)));
    app.press(Action::Vault(V::Item(n - 1)));
}

fn keys(app: &Faraday) -> Vec<faraday_vault::Record> {
    app.vaults
        .store()
        .contents_for_tests(app.vaults.open[app.vaults.current].id)
        .of(kind::APK_KEY)
        .map(|(_, r)| r.clone())
        .collect()
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

fn has(app: &mut Faraday, want: &str) -> bool {
    texts(app).iter().any(|t| t == want)
}

fn sha_hex(bytes: &[u8]) -> String {
    sha256::Hash::hash(bytes)
        .to_byte_array()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Sign an APK open on the selected key, the APK at Files place `name`
/// looked at.
fn review(app: &mut Faraday, name: &str) {
    app.press(Action::Vault(V::ApkSignOpen));
    let k = app.inbox.iter().position(|i| i.name == name).unwrap();
    app.press(Action::Vault(V::ApkPick(k)));
}

fn type_second(app: &mut Faraday, text: &str) {
    app.press(Action::Vault(V::ApkSecond));
    for _ in 0..80 {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(app, text);
}

/// The Outbox file with this name.
fn out(app: &Faraday, name: &str) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .find(|i| i.name == name)
        .map(|i| i.bytes.clone())
}

/// What the last save kept: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes {
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

/// The flow of the APK key as far as Make: two empty sticks of one label
/// and serial, one new vault.
fn flow_to_make(shell: &mut Shell, keep_time: bool) -> Faraday {
    use faraday_core::backup_flow::steps::FlowAction as F;
    shell.add(Disk::empty("S1", "KEYSTICK"));
    shell.add(Disk::empty("S2", "KEYSTICK"));
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    app.apk_signing = true;
    if keep_time {
        crate::common::keep_time(&mut app);
    }
    crate::common::start(&mut app, Kind::Apk);
    shell.flow(&mut app, F::Continue);
    for (disk, hold) in [("S1", false), ("S2", true)] {
        shell.plug(&mut app, disk);
        shell.flow(&mut app, if hold { F::Hold } else { F::Tap });
        shell.pull(&mut app, disk);
    }
    crate::common::create_vault(shell, &mut app, 0, PASSPHRASE);
    app
}

#[test]
fn make_an_apk_key_in_the_flow_puts_a_record_in_every_vault_and_the_certificate_for_the_stick() {
    let mut shell = Shell::new();
    let mut app = flow_to_make(&mut shell, true);
    crate::common::make_key(&mut shell, &mut app, &["Faraday Test", FIXTURE]);
    let made = keys(&app);
    assert_eq!(made.len(), 1);
    let r = &made[0];
    assert_eq!(r.text(field::APK_PACKAGE), Some(FIXTURE));
    let cert = r.field(field::APK_CERT).unwrap().to_vec();
    let info = key::describe(&cert).unwrap();
    assert_eq!(info.name, "Faraday Test");
    // Valid from midnight UTC of the day it was made.
    assert_eq!(info.made % 86_400, 0);
    assert_eq!(info.made, 1_791_482_652 - 1_791_482_652 % 86_400);
    // The certificate waits For the stick, as PEM.
    let stem = format!("faraday-test-{}", &faraday_apk::hex(&info.fingerprint)[..8]);
    let pem_file = out(&app, &format!("{stem}-certificate.pem")).expect("the certificate");
    assert_eq!(pem::read(&pem_file).as_deref(), Some(&cert[..]));
    // The result names what was made, a row per vault it is in.
    let t = texts(&mut app);
    for want in [
        "APK signing key made",
        "Faraday Test",
        FIXTURE,
        "RSA 3072",
        "In Vault 1",
        "Show as QR",
        "PNG",
    ] {
        assert!(t.iter().any(|x| x == want), "{want}: {t:?}");
    }
    // The fingerprint, in groups of four, wraps over lines.
    let grouped = key::grouped(&info.fingerprint);
    assert!(t.iter().any(|x| x.starts_with(&grouped[..24])), "{t:?}");
}

#[test]
fn the_vault_sealed_and_opened_again_shows_the_same_name_fingerprint_and_package() {
    let c = testkit::compute();
    use faraday_core::backup_flow::steps::FlowAction as F;
    let mut shell = Shell::new();
    let mut app = flow_to_make(&mut shell, true);
    crate::common::make_key(&mut shell, &mut app, &["Faraday Test", FIXTURE]);
    let cert = keys(&app)[0].field(field::APK_CERT).unwrap().to_vec();
    let secrets = secrets_of(keys(&app)[0].field(field::APK_KEY).unwrap());
    shell.flow(&mut app, F::MadeContinue);
    let app = shell.lock(&mut app);
    // What the lock kept for the flow holds none of the private key.
    let (inbox, outbox, kept) = shell.boxes.clone();
    assert!(kept.iter().any(|(n, _)| n == "backup-flow"));
    for (name, bytes) in kept.iter().chain(&inbox).chain(&outbox) {
        if name.ends_with(".ofv") {
            continue;
        }
        for s in &secrets {
            assert!(!contains(bytes, s), "{name} holds a secret part");
        }
    }
    let (name, bytes) = {
        let i = app
            .outbox
            .iter()
            .find(|i| i.name.ends_with(".ofv"))
            .unwrap();
        (i.name.clone(), i.bytes.clone())
    };
    assert!(!name.is_empty());
    let (_, contents) = faraday_vault::open(&c, &bytes, PASSPHRASE.as_bytes()).unwrap();
    let r = contents.of(kind::APK_KEY).next().unwrap().1;
    assert_eq!(r.field(field::APK_CERT), Some(&cert[..]));
    assert_eq!(r.text(field::APK_PACKAGE), Some(FIXTURE));
    assert_eq!(
        key::describe(r.field(field::APK_CERT).unwrap())
            .unwrap()
            .name,
        "Faraday Test"
    );
}

#[test]
fn a_certificate_name_left_empty_and_a_bad_package_are_refused() {
    // Refused on the spot, with the form still open to be put right.
    let try_make = |name: &str, package: &str| {
        let mut shell = Shell::new();
        let mut app = flow_to_make(&mut shell, true);
        let toast = crate::common::make_key(&mut shell, &mut app, &[name, package]);
        assert!(app.vaults.form.is_some(), "{name:?} {package:?}");
        assert!(keys(&app).is_empty(), "{name:?} {package:?}");
        assert!(!app.flow().unwrap().made);
        toast
    };
    assert_eq!(
        try_make("   ", FIXTURE).as_deref(),
        Some("Name the certificate")
    );
    for bad in ["other", "1app.x", "app. x", "app..x", "app.x-y", ""] {
        assert_eq!(
            try_make("Faraday Test", bad).as_deref(),
            Some("Not a package name"),
            "{bad:?}"
        );
    }
    // The name is trimmed, and a package of Android's form is taken.
    let mut shell = Shell::new();
    let mut app = flow_to_make(&mut shell, true);
    crate::common::make_key(&mut shell, &mut app, &["  Faraday Test ", "app.a_b.c9"]);
    let cert = keys(&app)[0].field(field::APK_CERT).unwrap().to_vec();
    assert_eq!(key::describe(&cert).unwrap().name, "Faraday Test");
}

#[test]
fn on_the_stick_make_a_key_asks_for_the_time_first() {
    let mut shell = Shell::new();
    let mut app = flow_to_make(&mut shell, false);
    crate::common::make_key(&mut shell, &mut app, &["Faraday Test", FIXTURE]);
    assert_eq!(app.screen, Screen::SetTime);
    assert!(keys(&app).is_empty(), "nothing made");
}

/// An app with a key for `package`, as a vault unlocked from a stick
/// holds one.
fn ready(package: &str, apks: &[(&str, Vec<u8>)]) -> Faraday {
    let mut app = unlocked(apks);
    make(&mut app, "Faraday Test", package);
    app
}

fn listed(app: &Faraday, package: &str) -> Vec<(String, Option<String>)> {
    let c = faraday_core::testkit::compute();
    app.apk_listed(&c, package)
        .into_iter()
        .map(|l| (l.name, l.why.map(|w| w.text())))
        .collect()
}

#[test]
fn each_refusal_is_listed_with_its_words_and_a_good_apk_is_not_refused() {
    let apks = [
        ("good-unsigned.apk", fixture("small-unsigned.apk")),
        ("old.apk", fixture("old-sdk-unsigned.apk")),
        ("unaligned.apk", fixture("unaligned-unsigned.apk")),
        ("prefix.apk", fixture("prefix-unsigned.apk")),
        ("gap.apk", fixture("gap-unsigned.apk")),
        ("order.apk", fixture("order-unsigned.apk")),
        ("local-method.apk", fixture("local-method-unsigned.apk")),
        ("local-sizes.apk", fixture("local-sizes-unsigned.apk")),
        ("local-flags.apk", fixture("local-flags-unsigned.apk")),
        ("descriptor.apk", fixture("descriptor-unsigned.apk")),
        ("twice.apk", fixture("duplicate-manifest-unsigned.apk")),
        (
            "version-reference.apk",
            fixture("version-name-reference-unsigned.apk"),
        ),
        ("codename.apk", fixture("codename-sdk-unsigned.apk")),
        ("fake-id.apk", fixture("fake-id-unsigned.apk")),
        (
            "psbt.psbt",
            testkit::files()
                .unwrap()
                .into_iter()
                .find(|(n, _)| n.ends_with(".psbt"))
                .unwrap()
                .1,
        ),
        ("picture.png", b"\x89PNG\r\n\x1a\n not a picture".to_vec()),
        (
            "random.bin",
            (0..500u32).map(|i| (i * 31 % 251) as u8).collect(),
        ),
    ];
    let app = ready(FIXTURE, &apks);
    let got = listed(&app, FIXTURE);
    let why = |name: &str| got.iter().find(|(n, _)| n == name).map(|(_, w)| w.clone());
    assert_eq!(why("good-unsigned.apk"), Some(None));
    assert_eq!(
        why("old.apk"),
        Some(Some("Supports Android before 7.0".into()))
    );
    assert_eq!(why("unaligned.apk"), Some(Some("Not aligned".into())));
    for name in [
        "prefix.apk",
        "gap.apk",
        "order.apk",
        "local-method.apk",
        "local-sizes.apk",
        "local-flags.apk",
        "descriptor.apk",
        "twice.apk",
        "version-reference.apk",
        "codename.apk",
        // An attribute named like one of the four with another resource
        // ID is two readers' two readings: refused.
        "fake-id.apk",
    ] {
        assert_eq!(why(name), Some(Some("Not an APK".into())), "{name}");
    }
    // A PSBT, a picture and random bytes are not APKs at all: not listed.
    for name in ["psbt.psbt", "picture.png", "random.bin"] {
        assert_eq!(why(name), None, "{name}");
    }
}

#[test]
fn an_apk_that_is_already_signed_is_listed_already_signed() {
    let unsigned = fixture("small-unsigned.apk");
    let pair = key::make(&[0x42; 32], "Faraday Test", 1_791_000_000).unwrap();
    let mut src = Slice(&unsigned);
    let apk = Apk::read(&mut src).unwrap();
    let signed = faraday_apk::sign::sign(
        &mut faraday_sb::rng::Stream::new(&[1; 32], 0),
        &mut src,
        &apk,
        &pair.key,
        &pair.cert,
    )
    .and_then(|s| faraday_apk::sign::assemble(&mut src, &apk, &s))
    .unwrap();
    assert_eq!(
        Apk::read(&mut Slice(&signed)).unwrap_err(),
        Refusal::AlreadySigned
    );
    let app = ready(FIXTURE, &[("signed.apk", signed)]);
    assert_eq!(
        listed(&app, FIXTURE),
        vec![("signed.apk".to_string(), Some("Already signed".to_string()))]
    );
}

#[test]
fn the_review_states_the_package_version_android_and_the_name_it_will_write() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(
        FIXTURE,
        &[("example-0.1.3-android-unsigned.apk", apk.clone())],
    );
    review(&mut app, "example-0.1.3-android-unsigned.apk");
    let t = texts(&mut app);
    for want in [
        "Sign this APK?",
        "example-0.1.3-android-unsigned.apk",
        FIXTURE,
        "1.0 (1)",
        "8.0 and later",
        "example-0.1.3-android.apk",
        "Enter the second build's SHA-256",
    ] {
        assert!(t.iter().any(|x| x == want), "{want}: {t:?}");
    }
    // The SHA-256, in groups of four, wraps over lines.
    let sha = faraday_apk::key::grouped(&sha256::Hash::hash(&apk).to_byte_array());
    assert!(t.iter().any(|x| x.starts_with(&sha[..24])), "{t:?}");
}

#[test]
fn the_hold_is_inert_until_the_second_build_matches() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(FIXTURE, &[("x-unsigned.apk", apk.clone())]);
    review(&mut app, "x-unsigned.apk");
    let sha = sha_hex(&apk);
    let before = app.outbox.len();
    let try_hold = |app: &mut Faraday| {
        entropy(app, 1);
        app.press(Action::Vault(V::ApkHoldSign));
        app.outbox.len()
    };
    // Nothing typed.
    assert_eq!(try_hold(&mut app), before);
    // Fewer than 32 digits.
    type_second(&mut app, &sha[..31]);
    assert!(has(&mut app, "At least 32 hex digits"));
    assert_eq!(try_hold(&mut app), before);
    // The digits of another file.
    type_second(&mut app, &"0".repeat(40));
    assert!(has(&mut app, "Does not match this file"));
    assert_eq!(try_hold(&mut app), before);
    // 32 digits that are the file's, spaced and in capitals, sign.
    let spaced: String = sha[..32]
        .to_uppercase()
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    type_second(&mut app, &spaced);
    assert!(texts(&mut app).iter().any(|t| t.starts_with("Matches")));
    assert_eq!(try_hold(&mut app), before + 1);
    let signed = out(&app, "x.apk").expect("signed APK For the stick");
    assert!(signed.len() > apk.len());
    assert_eq!(
        Apk::read(&mut Slice(&signed)).unwrap_err(),
        Refusal::AlreadySigned
    );
    assert!(
        app.inbox.iter().any(|i| i.name == "x-unsigned.apk"),
        "the unsigned file stays"
    );
}

#[test]
fn all_64_digits_sign_too() {
    let apk = fixture("sdk33-unsigned.apk");
    let mut app = ready(FIXTURE, &[("y.apk", apk.clone())]);
    review(&mut app, "y.apk");
    type_second(&mut app, &sha_hex(&apk));
    entropy(&mut app, 1);
    app.press(Action::Vault(V::ApkHoldSign));
    assert!(
        out(&app, "y-signed.apk").is_some(),
        "{:?}",
        app.toast_text()
    );
}

#[test]
fn a_key_for_another_package_lists_the_apk_as_another_app_from_the_page_and_from_files() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(OTHER, &[("x-unsigned.apk", apk)]);
    let want = format!("Another app: {FIXTURE}");
    assert_eq!(
        listed(&app, OTHER),
        vec![("x-unsigned.apk".to_string(), Some(want.clone()))]
    );
    app.press(Action::Vault(V::ApkSignOpen));
    assert!(has(&mut app, &want));
    // Files: the row says it and offers nothing to sign with.
    app.press(Action::Nav(Screen::Files));
    let t = texts(&mut app);
    assert!(t.contains(&want), "{t:?}");
    assert!(!t.iter().any(|x| x == "Sign with APK key"), "{t:?}");
    // The package is shown on the key's page.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::ApkBack));
    assert!(has(&mut app, OTHER));
}

#[test]
fn files_offers_sign_with_apk_key_for_an_apk_a_key_fits_and_it_opens_the_review() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(FIXTURE, &[("x-unsigned.apk", apk)]);
    app.press(Action::Nav(Screen::Files));
    assert!(has(&mut app, "Sign with APK key"));
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "x-unsigned.apk")
        .unwrap();
    app.press(Action::Vault(V::ApkFromFiles(k)));
    assert_eq!(app.screen, Screen::VaultContents);
    assert!(has(&mut app, "Sign this APK?"));
}

#[test]
fn with_two_keys_for_the_package_files_asks_which_key() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(FIXTURE, &[("x-unsigned.apk", apk)]);
    make(&mut app, "Second Test", FIXTURE);
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "x-unsigned.apk")
        .unwrap();
    app.press(Action::Vault(V::ApkFromFiles(k)));
    assert!(has(&mut app, "Which key?"));
    let first = app.apk_keys_for(FIXTURE);
    assert_eq!(first.len(), 2);
    app.press(Action::Vault(V::ApkKeyPick(first[1].0, first[1].1)));
    assert!(has(&mut app, "Sign this APK?"));
}

#[test]
fn the_output_is_named_without_unsigned_or_with_signed_added() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(
        FIXTURE,
        &[
            ("a-unsigned.apk", apk.clone()),
            ("b.apk", apk.clone()),
            ("c", apk.clone()),
            ("D-UNSIGNED.APK", apk.clone()),
            ("e-unsigned", apk.clone()),
        ],
    );
    for (input, output) in [
        ("a-unsigned.apk", "a.apk"),
        ("b.apk", "b-signed.apk"),
        ("c", "c-signed.apk"),
        ("D-UNSIGNED.APK", "D.APK"),
        ("e-unsigned", "e-unsigned-signed.apk"),
    ] {
        review(&mut app, input);
        type_second(&mut app, &sha_hex(&apk));
        entropy(&mut app, 1);
        app.press(Action::Vault(V::ApkHoldSign));
        assert!(out(&app, output).is_some(), "{input} -> {output}");
    }
}

#[test]
fn a_name_that_would_pass_230_bytes_is_listed_too_long_and_one_that_shortens_signs() {
    let apk = fixture("small-unsigned.apk");
    let long = format!("{}.apk", "a".repeat(224));
    let ok = format!("{}-unsigned.apk", "b".repeat(230 - "-unsigned.apk".len()));
    assert_eq!(ok.len(), 230);
    let mut app = ready(FIXTURE, &[(&long, apk.clone()), (&ok, apk.clone())]);
    let got = listed(&app, FIXTURE);
    assert_eq!(
        got.iter().find(|(n, _)| *n == long).unwrap().1.as_deref(),
        Some("Name too long")
    );
    assert_eq!(got.iter().find(|(n, _)| *n == ok).unwrap().1, None);
    review(&mut app, &ok);
    type_second(&mut app, &sha_hex(&apk));
    entropy(&mut app, 1);
    app.press(Action::Vault(V::ApkHoldSign));
    let short = format!("{}.apk", "b".repeat(230 - "-unsigned.apk".len()));
    assert!(out(&app, &short).is_some());
}

#[test]
fn with_no_apk_copied_in_it_says_to_copy_one_in_first() {
    let mut app = ready(FIXTURE, &[]);
    app.press(Action::Vault(V::ApkSignOpen));
    assert!(has(&mut app, "Copy the APK in on a stick visit first."));
}

/// The pieces of a PKCS #8 RSA key that must never reach a file: the
/// whole encoding, the private exponent and each prime.
fn secrets_of(pkcs8: &[u8]) -> Vec<Vec<u8>> {
    use faraday_sb::der::{children, read};
    let mut out = vec![pkcs8.to_vec()];
    // PrivateKeyInfo { version, algorithm, OCTET STRING { RSAPrivateKey } }.
    let (_, info, _) = read(pkcs8, 0).unwrap();
    let parts = children(&pkcs8[info.clone()]).unwrap();
    let octets = &pkcs8[info][parts[2].1.clone()];
    let (_, rsa, _) = read(octets, 0).unwrap();
    let ints = children(&octets[rsa.clone()]).unwrap();
    // version, n, e, d, p, q, ...
    for i in [3usize, 4, 5] {
        let v = &octets[rsa.clone()][ints[i].1.clone()];
        let v = if v[0] == 0 { &v[1..] } else { v };
        out.push(v.to_vec());
    }
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn nothing_for_the_stick_ever_holds_the_private_key() {
    let apk = fixture("small-unsigned.apk");
    let mut app = unlocked(&[("x-unsigned.apk", apk.clone())]);
    let check = |app: &Faraday, what: &str| {
        let Some((_, r)) = app.vaults.open.first().and_then(|v| {
            app.vaults
                .store()
                .contents_for_tests(v.id)
                .of(kind::APK_KEY)
                .next()
        }) else {
            return;
        };
        let secrets = secrets_of(r.field(field::APK_KEY).unwrap());
        // Files, both halves, holds none of it.
        for item in app.outbox.iter().chain(&app.inbox) {
            if item.kind == faraday_core::wallet::FileKind::Vault {
                continue;
            }
            for s in &secrets {
                assert!(
                    !contains(&item.bytes, s),
                    "{what}: {} holds a secret part",
                    item.name
                );
            }
        }
    };
    make(&mut app, "Faraday Test", FIXTURE);
    check(&app, "after Make");
    app.press(Action::Vault(V::ApkExport));
    check(&app, "after Certificate");
    app.press(Action::ShowCode(faraday_core::Code::ApkCertificate));
    check(&app, "after Show as QR");
    app.press(Action::CodePng(faraday_core::Code::ApkCertificate));
    check(&app, "after PNG");
    review(&mut app, "x-unsigned.apk");
    type_second(&mut app, &sha_hex(&apk));
    entropy(&mut app, 1);
    app.press(Action::Vault(V::ApkHoldSign));
    check(&app, "after Sign");
    assert!(out(&app, "x.apk").is_some());
    // The state a lock keeps holds none of it either.
    let secrets = secrets_of(
        app.vaults
            .store()
            .contents_for_tests(app.vaults.open[0].id)
            .of(kind::APK_KEY)
            .next()
            .unwrap()
            .1
            .field(field::APK_KEY)
            .unwrap(),
    );
    entropy(&mut app, 2);
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    for (name, bytes) in kept.iter().chain(&inbox).chain(&outbox) {
        if name.ends_with(".ofv") {
            continue;
        }
        for s in &secrets {
            assert!(
                !contains(bytes, s),
                "{name} holds a secret part after a lock"
            );
        }
    }
}

#[test]
fn the_file_changing_between_the_review_and_the_hold_signs_nothing() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(FIXTURE, &[("x-unsigned.apk", apk.clone())]);
    review(&mut app, "x-unsigned.apk");
    type_second(&mut app, &sha_hex(&apk));
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "x-unsigned.apk")
        .unwrap();
    let last = app.inbox[k].bytes.len() - 1;
    app.inbox[k].bytes[last] ^= 1;
    let before = app.outbox.len();
    entropy(&mut app, 1);
    app.press(Action::Vault(V::ApkHoldSign));
    assert_eq!(app.outbox.len(), before);
    assert_eq!(
        app.toast_text(),
        Some("The file changed since it was reviewed")
    );
}

#[test]
fn another_apps_apk_reached_through_the_list_is_never_signed() {
    let apk = fixture("small-unsigned.apk");
    let mut app = ready(OTHER, &[("x-unsigned.apk", apk.clone())]);
    // The review opens by Inbox place, whatever the list says.
    review(&mut app, "x-unsigned.apk");
    type_second(&mut app, &sha_hex(&apk));
    let before = app.outbox.len();
    entropy(&mut app, 1);
    app.press(Action::Vault(V::ApkHoldSign));
    assert_eq!(app.outbox.len(), before);
    assert!(out(&app, "x.apk").is_none());
}

#[test]
fn files_reads_a_file_that_is_one_pem_certificate_as_public() {
    let pair = key::make(&[0x42; 32], "Faraday Test", 1_791_000_000).unwrap();
    let pem_text = pem::encode(&pair.cert);
    let mut app = process(
        vec![
            ("c.pem".into(), pem_text.clone().into_bytes()),
            (
                "twice.pem".into(),
                format!("{pem_text}{pem_text}").into_bytes(),
            ),
            ("text.pem".into(), format!("hello\n{pem_text}").into_bytes()),
        ],
        Vec::new(),
        Vec::new(),
    );
    let kinds: Vec<(String, faraday_core::wallet::FileKind)> =
        app.inbox.iter().map(|i| (i.name.clone(), i.kind)).collect();
    let kind_of = |n: &str| kinds.iter().find(|(k, _)| k == n).unwrap().1;
    assert_eq!(
        kind_of("c.pem"),
        faraday_core::wallet::FileKind::Certificate
    );
    assert!(!kind_of("c.pem").may_be_secret());
    assert_ne!(
        kind_of("twice.pem"),
        faraday_core::wallet::FileKind::Certificate
    );
    assert_ne!(
        kind_of("text.pem"),
        faraday_core::wallet::FileKind::Certificate
    );
    let _ = &mut app;
}

// ---------------------------------------------------------------------
// APK signing switched off, as it is by default (F59).
// ---------------------------------------------------------------------

/// A process with the vault and one APK in Files, the switch off.
fn switched_off(apks: &[(&str, Vec<u8>)]) -> Faraday {
    let mut inbox = vec![vault_file()];
    inbox.extend(apks.iter().map(|(n, b)| (n.to_string(), b.clone())));
    let mut app = process(inbox, Vec::new(), Vec::new());
    app.apk_signing = false;
    app
}

fn open_vault(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, 1);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::VaultContents));
}

#[test]
fn with_apk_signing_off_tools_has_no_apk_tile_and_typing_apk_finds_none() {
    let mut app = switched_off(&[]);
    app.press(Action::Nav(Screen::Catalog));
    type_text(&mut app, "apk");
    assert!(!texts(&mut app).iter().any(|t| t.contains("APK")));
    app.apk_signing = true;
    assert!(has(&mut app, "APK signing key"));
}

#[test]
fn with_apk_signing_off_a_vault_without_an_apk_key_lists_no_such_category() {
    let mut app = switched_off(&[]);
    open_vault(&mut app);
    assert!(!has(&mut app, "APK signing keys"));
    assert!(!has(&mut app, "Make an APK key"));
    app.press(Action::Vault(V::Category(6)));
    app.press(Action::Vault(V::AddKind(6)));
    assert!(!has(&mut app, "APK signing keys"));
    app.press(Action::Vault(V::ApkSignOpen));
    assert!(!app.vaults.apk.signing);
}

#[test]
fn with_apk_signing_off_an_apk_in_files_has_no_sign_row_but_is_named_an_apk() {
    let apk = fixture("small-unsigned.apk");
    let mut app = switched_off(&[("x-unsigned.apk", apk)]);
    open_vault(&mut app);
    app.press(Action::Nav(Screen::Files));
    assert!(texts(&mut app).iter().any(|t| t.starts_with("APK ·")));
    for row in ["Sign with APK key", "Back up first", "Sign an APK"] {
        assert!(!has(&mut app, row), "{row}");
    }
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "x-unsigned.apk")
        .unwrap();
    app.press(Action::Vault(V::ApkFromFiles(k)));
    assert!(!app.vaults.apk.signing);
    assert_ne!(app.screen, Screen::VaultContents);
}

#[test]
fn with_apk_signing_off_a_vault_that_holds_an_apk_key_still_lists_it() {
    let mut app = ready(FIXTURE, &[]);
    app.apk_signing = false;
    app.press(Action::Nav(Screen::VaultContents));
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t.starts_with("APK signing keys"))
    );
    app.press(Action::Vault(V::Category(6)));
    assert_eq!(app.vaults.category, 6);
}
