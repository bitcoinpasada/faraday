//! Every form the backup board keeps comes back (`docs/BACKUP-FLOW.md`
//! §17.12 items 4, 5 and 10): a wallet backed up with one form at one
//! place is restored from what that place's stick holds alone, with the
//! passphrase and the other shares from where the plan put them; a file
//! written to a stick is read back at once and written again once when it
//! does not read back; a passphrase kept alone in a vault is a record of
//! its own, and a key loaded from another open vault takes it.

use crate::common::*;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_core::backup_flow::{Problem, Step};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::plan::{self, At, Shape, Spot, What, form};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, StorageEvent, bstep, cstep, testkit};
use faraday_shell_api::{App, Event, Key};
use faraday_vault::records::{field, kind};

const PASS: &str = "TREZOR";

// ---- helpers ----

/// `n` empty sticks of different serials, S1 to Sn.
fn sticks(shell: &mut Shell, n: usize) {
    for k in 1..=n {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k as u32);
        shell.add(d);
    }
}

/// Test seed `n` typed in as a key, with `pass` as its passphrase when
/// there is one.
fn key_in(app: &mut Faraday, n: usize, pass: &str) -> [u8; 4] {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    if !pass.is_empty() {
        app.press(Action::EntryPassphrase);
        type_text(app, pass);
    }
    app.press(Action::EntryAdd);
    app.session.keys.last().unwrap().fingerprint().0
}

fn kind_index(kind: NewKind) -> u8 {
    NewKind::ALL.iter().position(|k| *k == kind).unwrap() as u8
}

fn backup_shape(app: &Faraday) -> Shape {
    app.plan_shape_now(app.backup.as_ref().unwrap().wallet)
}

/// A change to Back up's plan made straight to its map, as a drop on the
/// board makes it (the board itself is `backup_board.rs`'s).
fn board_edit(app: &mut Faraday, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
    let c = faraday_core::testkit::compute();
    let shape = backup_shape(app);
    let a = &mut app.backup.as_mut().unwrap().answers;
    let m = plan::map(&c, &shape, a);
    a.set_map(f(m));
}

/// The words and the paper forms of the description taken off every
/// place's paper on the board: nothing kept but what the test puts on
/// the map.
fn nothing_on_paper(app: &mut Faraday) {
    no_words(app);
    no_paper_description(app);
}

/// A wallet made in Create over test seeds `seeds` typed in here, the
/// first with `pass`: one key Native SegWit, or a 2-of-3. Its backup is
/// open with nothing on paper; the address it receives at first.
fn made(seeds: &[usize], pass: &str) -> (Faraday, String, Vec<[u8; 4]>) {
    let c = faraday_core::testkit::compute();
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let fps: Vec<[u8; 4]> = seeds
        .iter()
        .enumerate()
        .map(|(k, &n)| key_in(&mut app, n, if k == 0 { pass } else { "" }))
        .collect();
    app.press(Action::CreateWallet);
    let kind = if seeds.len() == 1 {
        NewKind::NativeSegwit
    } else {
        NewKind::Multi
    };
    app.press(Action::CKind(kind_index(kind)));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.screen, Screen::Backup);
    let address = app.session.address(&c, &app.session.wallets[0], false, 0);
    nothing_on_paper(&mut app);
    (app, address, fps)
}

/// One stick written as the flow asks: confirmed, held where it must be.
fn write_stick(shell: &mut Shell, app: &mut Faraday, d: &str) {
    shell.plug(app, d);
    for _ in 0..3 {
        let r = app.flow().and_then(|f| f.live.reading.as_ref());
        let Some(r) = r else { break };
        if r.problem.is_some() || r.bound {
            break;
        }
        if r.hold {
            shell.flow(app, F::Hold);
        } else {
            shell.flow(app, F::Tap);
        }
    }
    shell.pull(app, d);
}

/// The checklist made and done, Lock and write past each seed's warning,
/// and each stick of the write step written on S1, S2 … in turn.
fn lock_and_write(shell: &mut Shell, app: &mut Faraday) -> Faraday {
    let c = testkit::compute();
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup.as_ref().unwrap().stage,
        BStage::Checklist,
        "{:?}",
        app.backup_kept_nowhere(&c)
    );
    testkit::type_passphrases(app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(app, F::WalletLock);
    let_seed_files_out(app);
    shell.pump(app);
    assert!(app.restart_requested(), "locked");
    let mut app = shell.restart();
    let n = app.flow().unwrap().slots.len();
    for k in 0..n {
        write_stick(shell, &mut app, &format!("S{}", k + 1));
    }
    assert_eq!(
        app.flow().unwrap().step,
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    app
}

/// What the sticks `disks` hold, as one list of files.
fn on(shell: &Shell, disks: &[&str]) -> Vec<(String, Vec<u8>)> {
    disks
        .iter()
        .flat_map(|d| shell.disk(d).files.clone())
        .collect()
}

/// A Faraday with nothing in it but the files given, copied in from a
/// stick as a visit copies them: a text file as it is, a picture as the
/// disk process reads its codes.
fn copied_in(files: &[(String, Vec<u8>)]) -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = testkit::started();
    let text: Vec<(String, Vec<u8>)> = files
        .iter()
        .filter(|(n, _)| !n.ends_with(".png"))
        .cloned()
        .collect();
    app.storage(StorageEvent::Restored {
        inbox: text,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let mut pictures: Vec<&(String, Vec<u8>)> =
        files.iter().filter(|(n, _)| n.ends_with(".png")).collect();
    pictures.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, bytes) in pictures {
        let payloads = faraday_files::qr_in_png(&c, bytes).unwrap_or_default();
        assert!(!payloads.is_empty(), "{name} holds a code");
        app.storage(StorageEvent::QrRead {
            name: name.clone(),
            bytes: bytes.clone(),
            payloads,
        });
    }
    app
}

/// The seed the Inbox finds whose key is `fp`, made a wallet of the first
/// kind (Native SegWit) on its sheet: its first address.
fn seed_made_wallet(app: &mut Faraday, fp: [u8; 4]) -> String {
    let c = testkit::compute();
    let found = app.inbox_found_now(&c);
    assert!(
        found.seeds.iter().any(|s| s.fingerprint.0 == fp),
        "the seed {:02x?} is found: {:?}",
        fp,
        found.seeds
    );
    app.press(Action::PotentialOpen(fp));
    app.press(Action::PotentialMake);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint().0, fp);
    assert_eq!(app.session.wallets.len(), 1);
    app.session.address(&c, &app.session.wallets[0], false, 0)
}

/// Every wallet the Inbox describes, loaded: the first one's first
/// address.
fn described_wallet(app: &mut Faraday) -> String {
    let c = faraday_core::testkit::compute();
    app.press(Action::InboxLoad);
    assert_eq!(app.session.wallets.len(), 1, "one wallet");
    app.session.address(&c, &app.session.wallets[0], false, 0)
}

/// The fingerprint of test seed `n` with `pass`.
fn fp_of(n: usize, pass: &str) -> [u8; 4] {
    let c = testkit::compute();
    let m = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[n].0),
    )
    .unwrap();
    let seed = m.to_seed(&c, pass.as_bytes()).unwrap();
    MasterKey::from_seed(&c, &seed, testkit::NET)
        .fingerprint()
        .0
}

// ---- restored from one place: the seed's forms ----

/// A single key's words as a text file on Place 1's stick: the stick
/// alone gives the wallet back.
#[test]
fn a_words_file_restores_the_wallet() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let (mut app, address, fps) = made(&[2], "");
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedFile(0))
    });
    lock_and_write(&mut shell, &mut app);
    let mut back = copied_in(&on(&shell, &["S1"]));
    assert_eq!(seed_made_wallet(&mut back, fps[0]), address);
}

/// A single key's SeedQR as a picture on Place 1's stick: its code read
/// back gives the wallet.
#[test]
fn a_seedqr_picture_restores_the_wallet() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let (mut app, address, fps) = made(&[2], "");
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedQrFile(0))
    });
    lock_and_write(&mut shell, &mut app);
    let files = on(&shell, &["S1"]);
    assert!(files.iter().any(|(n, _)| n.starts_with("seedqr-")));
    let mut back = copied_in(&files);
    assert_eq!(seed_made_wallet(&mut back, fps[0]), address);
}

/// A key with a passphrase: its words on Place 1's stick, the passphrase
/// as a file on Place 2's. The two sticks give the key with its
/// passphrase, and the wallet.
#[test]
fn a_passphrase_file_beside_the_words_restores_the_key_with_its_passphrase() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let (mut app, address, fps) = made(&[2], PASS);
    assert_eq!(fps[0], fp_of(2, PASS));
    board_edit(&mut app, |m| {
        let m = plan::with_added(&m, At::Stick(0, 0), What::SeedFile(0));
        plan::with_added(&m, At::Stick(1, 0), What::PassphraseFile(0))
    });
    lock_and_write(&mut shell, &mut app);
    let pass_file = on(&shell, &["S2"]);
    assert!(
        pass_file
            .iter()
            .any(|(n, b)| n.starts_with("passphrase-") && b == b"TREZOR\n"),
        "{:?}",
        pass_file.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    // The words alone are another key.
    let alone = copied_in(&on(&shell, &["S1"]));
    assert!(
        alone
            .inbox_found_now(&c)
            .seeds
            .iter()
            .all(|s| s.fingerprint.0 != fps[0])
    );
    let mut back = copied_in(&on(&shell, &["S1", "S2"]));
    assert_eq!(seed_made_wallet(&mut back, fps[0]), address);
}

// ---- restored from one place: the description's forms ----

/// A 2-of-3 whose seeds are kept as files on Place 2's stick, with what
/// `at_place_1` keeps on Place 1's: written, Place 1's stick alone.
fn two_of_three_with(
    shell: &mut Shell,
    at_place_1: &dyn Fn(Vec<Spot>) -> Vec<Spot>,
) -> (Vec<(String, Vec<u8>)>, String) {
    sticks(shell, 2);
    let (mut app, address, _) = made(&[0, 1, 2], "");
    board_edit(&mut app, |m| {
        let mut m = m;
        for i in 0..3 {
            m = plan::with_added(&m, At::Stick(1, 0), What::SeedFile(i));
        }
        at_place_1(m)
    });
    lock_and_write(shell, &mut app);
    let place_1 = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .position(|s| s.target.files_place() == Some(0))
        .expect("Place 1's stick");
    (on(shell, &[&format!("S{}", place_1 + 1)]), address)
}

/// The description's text file at Place 1 gives the wallet back.
#[test]
fn the_descriptions_text_restores_the_wallet() {
    let mut shell = Shell::new();
    let (files, address) = two_of_three_with(&mut shell, &|m| {
        plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::TEXT))
    });
    assert!(files.iter().all(|(n, _)| !n.ends_with(".png")));
    assert_eq!(described_wallet(&mut copied_in(&files)), address);
}

/// The description's QR picture (in parts where it is long) at Place 1
/// gives the wallet back.
#[test]
fn the_descriptions_qr_picture_restores_the_wallet() {
    let mut shell = Shell::new();
    let (files, address) = two_of_three_with(&mut shell, &|m| {
        plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::QR))
    });
    assert!(
        files.iter().all(|(n, _)| n.ends_with(".png")),
        "pictures only"
    );
    assert_eq!(described_wallet(&mut copied_in(&files)), address);
}

/// The software's own file at Place 1 gives the wallet back.
#[test]
fn the_softwares_file_restores_the_wallet() {
    let mut shell = Shell::new();
    let (files, address) = two_of_three_with(&mut shell, &|m| {
        plan::with_added(&m, At::Stick(0, 0), What::SoftwareFile)
    });
    assert!(!files.is_empty());
    assert_eq!(described_wallet(&mut copied_in(&files)), address);
}

/// Share 1 at Place 1 and Share 2 at Place 2, as text files or as QR
/// pictures: the two places' sticks together give the wallet back.
fn shares_restore(f: usize) {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let (mut app, address, _) = made(&[0, 1, 2], "");
    board_edit(&mut app, |m| {
        let mut m = plan::with_added(&m, At::Stick(0, 0), What::ShareFile(0, f));
        m = plan::with_added(&m, At::Stick(1, 0), What::ShareFile(1, f));
        for i in 0..3 {
            m = plan::with_added(&m, At::Stick(1, 0), What::SeedFile(i));
        }
        m
    });
    lock_and_write(&mut shell, &mut app);
    let files = on(&shell, &["S1", "S2"]);
    let shares: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .filter(|(n, _)| n.contains("-share-"))
        .collect();
    assert!(!shares.is_empty());
    let mut back = copied_in(&shares);
    back.press(Action::Nav(Screen::Files));
    let _ = back.frame();
    back.press(Action::RestoreShares);
    assert_eq!(back.session.wallets.len(), 1, "{:?}", back.toast_text());
    assert_eq!(
        back.session.address(&c, &back.session.wallets[0], false, 0),
        address
    );
}

#[test]
fn the_shares_text_files_restore_the_wallet() {
    shares_restore(form::TEXT);
}

#[test]
fn the_shares_qr_pictures_restore_the_wallet() {
    shares_restore(form::QR);
}

// ---- read back after the write ----

/// A single key's SeedQR picture on Place 1's stick, the plan ready to
/// lock; the key's fingerprint.
fn seedqr_on_stick_1(shell: &mut Shell) -> (Faraday, [u8; 4]) {
    sticks(shell, 2);
    let (mut app, _, fps) = made(&[2], "");
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedQrFile(0))
    });
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(let_seed_files_out(&mut app), 1);
    shell.pump(&mut app);
    (shell.restart(), fps[0])
}

/// A picture whose code does not read back the first time is written
/// again, and the stick counts once it reads.
#[test]
fn a_picture_that_does_not_read_back_is_written_again_once() {
    let mut shell = Shell::new();
    let (mut app, _) = seedqr_on_stick_1(&mut shell);
    shell.blank_pictures.push(("png".into(), 1));
    write_stick(&mut shell, &mut app, "S1");
    let f = app.flow().unwrap();
    assert_eq!(f.step, Step::Done, "{:?}", f.live.problem);
    let pictures = shell
        .writes
        .iter()
        .filter(|(_, n)| n.ends_with(".png"))
        .count();
    assert_eq!(pictures, 2, "written, then written again");
}

/// Twice not read back: the stick's write does not count, and the card
/// says which file did not read back from which stick.
#[test]
fn a_picture_that_does_not_read_back_twice_leaves_the_stick_not_done() {
    let mut shell = Shell::new();
    let (mut app, fp) = seedqr_on_stick_1(&mut shell);
    shell.blank_pictures.push(("png".into(), 2));
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    let name = format!(
        "seedqr-{}.png",
        faraday_core::wallet::fp_text(Fingerprint(fp)).to_lowercase()
    );
    let f = app.flow().unwrap();
    assert!(f.slots[0].written.is_none(), "not done");
    assert_eq!(
        f.live.reading.as_ref().and_then(|r| r.problem.clone()),
        Some(Problem::NoReadBack(name.clone()))
    );
    expect(
        &mut app,
        &format!("{name} did not read back from KEYSTICK; try another stick"),
    );
    shell.pull(&mut app, "S1");
    // Another stick, by Use another stick: written and read back, it
    // counts.
    shell.plug(&mut app, "S2");
    assert_eq!(
        app.flow()
            .unwrap()
            .live
            .reading
            .as_ref()
            .and_then(|r| r.problem.clone()),
        Some(Problem::NotRead(app.flow().unwrap().slots[0].target))
    );
    shell.flow(&mut app, F::UseAnother);
    shell.pull(&mut app, "S2");
    write_stick(&mut shell, &mut app, "S2");
    assert_eq!(app.flow().unwrap().step, Step::Done);
}

/// A text file whose bytes do not read back the first time is written
/// again, and the stick counts.
#[test]
fn a_text_file_that_does_not_read_back_is_written_again_once() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let (mut app, _, _) = made(&[2], "");
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedFile(0))
    });
    shell.mismatch_names.push((".txt".into(), 1));
    let app = lock_and_write(&mut shell, &mut app);
    assert!(app.flow().unwrap().slots[0].written.is_some());
}

/// The SHA-256 of `bytes`, as lower-case hex.
fn sha_hex(bytes: &[u8]) -> String {
    use faraday_bip::bitcoin::hashes::{Hash, sha256};
    sha256::Hash::hash(bytes)
        .to_byte_array()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A seed's SeedQR, words file and passphrase file written to a stick and
/// read back: what is kept across a lock names them, and holds nothing
/// made from the seed or its passphrase: not the SeedQR's digits, the
/// words, the passphrase, nor the SHA-256 of any of them or of the files.
#[test]
fn after_a_seedqr_is_written_and_read_back_the_kept_state_holds_nothing_of_the_seed() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let (mut app, _, _) = made(&[2], PASS);
    board_edit(&mut app, |m| {
        let m = plan::with_added(&m, At::Stick(0, 0), What::SeedQrFile(0));
        let m = plan::with_added(&m, At::Stick(0, 0), What::SeedFile(0));
        plan::with_added(&m, At::Stick(0, 0), What::PassphraseFile(0))
    });
    let mut app = lock_and_write(&mut shell, &mut app);
    shell.pump(&mut app);
    let words = testkit::test_words(testkit::TEST_SEEDS[2].0);
    let m = Mnemonic::parse(&c, Language::English, &words).unwrap();
    let digits = faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec();
    let on_stick = shell.disk("S1").files.clone();
    assert!(
        on_stick.iter().any(|(n, _)| n.starts_with("seedqr-")),
        "the SeedQR was written"
    );
    let mut secret: Vec<String> = vec![
        String::from_utf8(digits.clone()).unwrap(),
        words.clone(),
        sha_hex(&digits),
        sha_hex(words.as_bytes()),
        sha_hex(format!("{words}\n").as_bytes()),
        sha_hex(PASS.as_bytes()),
        sha_hex(format!("{PASS}\n").as_bytes()),
    ];
    for (_, bytes) in &on_stick {
        secret.push(sha_hex(bytes));
    }
    let kept = shell.boxes.2.clone();
    let flow = kept
        .iter()
        .find(|(n, _)| n == "backup-flow")
        .map(|(_, b)| String::from_utf8_lossy(b).to_string())
        .expect("the flow is kept");
    assert!(flow.contains("seedqr-"), "the picture is named: {flow}");
    for (name, bytes) in &kept {
        let text = String::from_utf8_lossy(bytes).to_lowercase();
        for s in &secret {
            assert!(!text.contains(&s.to_lowercase()), "{name} holds {s}");
        }
        assert!(
            !text.contains(&PASS.to_lowercase()),
            "{name} holds the passphrase"
        );
    }
}

// ---- a passphrase alone in a vault ----

/// The board's drop of a passphrase into a vault: with its seed there, in
/// the key record; without, a passphrase record of its own, which counts
/// as the passphrase kept; a seed added beside it takes it in.
#[test]
fn a_passphrase_dropped_in_a_vault_without_its_seed_is_a_record_of_its_own() {
    let c = faraday_core::testkit::compute();
    let shape = Shape {
        m: 1,
        keys: 1,
        seeds: vec![plan::Seed {
            name: "aaaa0001".into(),
            here: true,
            passphrase: true,
        }],
        splits: false,
        vaults: 0,
    };
    let mut a = plan::Answers::preset(&shape, plan::Preset::Paper);
    let mut m = plan::map(&c, &shape, &a);
    for p in 0..plan::places_of(&m) {
        m = plan::without(&m, At::Place(p), What::Passphrase(0));
    }
    let m = plan::with_vault_stick(&m, 0, 0);
    let m = plan::with_vault_stick(&m, 1.min(plan::places_of(&m) - 1), 1);
    let m = plan::with_added(&m, At::Vault(1), What::Passphrase(0));
    let held = |m: &[Spot], at: At| plan::holds_at(m, at);
    assert_eq!(held(&m, At::Vault(1)), vec![What::PassphraseSealed(0)]);
    a.set_map(m.clone());
    assert_eq!(plan::kept_nowhere(&c, &shape, &a, &[0]), None, "kept");
    let gone = plan::without(&m, At::Vault(1), What::PassphraseSealed(0));
    a.set_map(gone);
    assert!(plan::kept_nowhere(&c, &shape, &a, &[0]).is_some());
    // Its seed added beside it: one key record with its passphrase.
    let m = plan::with_added(&m, At::Vault(1), What::Seed(0));
    assert_eq!(held(&m, At::Vault(1)), vec![What::SeedPassphrase(0)]);
    // Dropped where its seed is: into the key record.
    let m = plan::with_added(&m, At::Vault(0), What::Seed(0));
    let m = plan::with_added(&m, At::Vault(0), What::Passphrase(0));
    assert_eq!(held(&m, At::Vault(0)), vec![What::SeedPassphrase(0)]);
    // A plan text keeps it.
    a.set_map(plan::with_added(
        &plan::without(&m, At::Vault(1), What::SeedPassphrase(0)),
        At::Vault(1),
        What::Passphrase(0),
    ));
    let text = a.to_text();
    assert!(text.contains("vpass0"), "{text}");
    assert_eq!(plan::Answers::from_text(&shape, &text), Some(a));
}

/// A vault file holding the records given, under `phrase`.
fn vault_with(records: Vec<faraday_vault::Record>, phrase: &str, salt: u8) -> Vec<u8> {
    let c = testkit::compute();
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let mut records = records;
    records.insert(
        0,
        faraday_vault::Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Main"),
    );
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[phrase.as_bytes()],
        &[faraday_vault::Contents { records }],
        &[salt; 32],
    )
    .unwrap()
}

/// Unlocks the vault file `name` with `phrase`.
fn unlock_file(app: &mut Faraday, name: &str, phrase: &str) {
    let k = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault file");
    app.press(Action::Vault(V::Open(k)));
    for c in phrase.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A key record of a seed's words alone in one vault, its passphrase
/// record in another: loaded with both open, the key is the one with the
/// passphrase; with only the words' vault open, the words alone.
#[test]
fn a_key_loads_with_its_passphrase_kept_in_another_open_vault() {
    let c = testkit::compute();
    let words = Mnemonic::parse(&c, Language::English, &testkit::test_words("summer")).unwrap();
    let key = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words));
    let with = fp_of(2, PASS);
    let pass = faraday_vault::Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &with)
        .with(field::PASSPHRASE, PASS.as_bytes());
    let files = vec![
        (
            "words.ofv".to_string(),
            vault_with(vec![key], "words phrase", 1),
        ),
        (
            "apart.ofv".to_string(),
            vault_with(vec![pass], "apart phrase", 2),
        ),
    ];
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files.clone(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    unlock_file(&mut app, "apart.ofv", "apart phrase");
    unlock_file(&mut app, "words.ofv", "words phrase");
    assert_eq!(app.vaults.open.len(), 2, "{:?}", app.vaults.unlock_error);
    let v = app
        .vaults
        .open
        .iter()
        .position(|o| o.contents.of(kind::KEY).count() == 1)
        .unwrap();
    app.press(Action::Vault(V::LoadAll(v)));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint().0, with);
    // The words' vault alone: the words alone.
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    unlock_file(&mut app, "words.ofv", "words phrase");
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint().0, fp_of(2, ""));
}
