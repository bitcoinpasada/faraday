//! Codex32 strings kept beyond paper (`docs/CODEX32.md` §9.5), as a
//! person backing up and getting a key back would notice it, against
//! BIP 93's vector 2 (`tools/vectors/bip93/vectors.json`): a string the
//! plan keeps in a vault is saved there as a share record once this
//! session holds it; a string kept on a stick is written there as a text
//! file or a Codex32QR picture, and the two files rebuild the key; a
//! string the session does not hold stops the save and the write, named;
//! shares in two vaults, unlocked one after the other, load the key; a
//! place's line counts its shares.

use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress, Thing, Zone};
use faraday_core::codex32_backup::Press;
use faraday_core::create::NewKind;
use faraday_core::plan::{self, At, Spot, What};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, bstep, cstep, testkit};
use faraday_keys::texts::Spec;
use faraday_shell_api::{App, Event, Key};
use faraday_ui::keyboard::OskPress;
use faraday_vault::codex32::{SetFacts, State};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

/// Vector 2: shares A, C and D; the fingerprint of its seed's key.
pub(crate) const A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
pub(crate) const C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
pub(crate) const D: &str = "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg";
pub(crate) const FP: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];

pub(crate) const PHRASE: &str = "home phrase";

pub(crate) fn ix(c: char) -> usize {
    plan::codex32::index_of(c as u8).unwrap()
}

/// Vector 2's key typed in from strings A and C, its set known as one of
/// 2 of 3 with A, C and D.
pub(crate) fn key_from_strings(app: &mut Faraday) {
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    for s in [A, C] {
        type_text(app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    let k = app
        .session
        .keys
        .position_of(FP)
        .unwrap_or_else(|| panic!("{:?}", testkit::entry(app).error));
    app.session.keys.set_codex32(
        k,
        Some(SetFacts {
            threshold: 2,
            identifier: *b"name",
            length: 48,
            unknown: 0,
            strings: vec![
                (b'a', State::Made),
                (b'c', State::Made),
                (b'd', State::Made),
            ],
        }),
    );
}

/// A single-key wallet over the key typed in, loaded, and Back up open on
/// it: its set's strings one per place on paper.
pub(crate) fn wallet_and_backup(app: &mut Faraday) {
    let c = testkit::compute();
    let k = app.session.keys.position_of(FP).unwrap();
    let text = app
        .session
        .keys
        .key_text(
            &c,
            k,
            &Spec::Single(faraday_bip::keys::ScriptType::NativeSegwit, 0),
        )
        .unwrap()
        .unwrap();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![(
            "shares-wallet.txt".into(),
            format!("wpkh({text}/<0;1>/*)").into_bytes(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let w = app.session.wallets.len();
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
}

/// The checklist made and its codex32 item open.
pub(crate) fn codex32_item(app: &mut Faraday) {
    if app.backup().unwrap().stage != BStage::Checklist {
        app.press(Action::BChecklist);
    }
    let c = testkit::compute();
    assert_eq!(
        app.backup().unwrap().stage,
        BStage::Checklist,
        "{:?}",
        app.backup_kept_nowhere(&c)
    );
    let n = bstep::of(plan::Item::Codex32(0));
    assert!(app.backup_items_now(&c).contains(&n), "the codex32 item");
    // The blank templates, printed from Faraday Starter, come first.
    if app.backup().unwrap().open == Some(bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    open_item(app, n);
    assert_eq!(app.backup().unwrap().open, Some(n));
    assert!(app.backup().unwrap().c32.is_some());
}

/// Each string on paper typed back from its paper in the codex32 item,
/// open again where the checklist moved on.
pub(crate) fn type_each_back(app: &mut Faraday) {
    open_item(app, bstep::of(plan::Item::Codex32(0)));
    for s in [A, C, D] {
        let j = plan::codex32::index_of(s.as_bytes()[8]).unwrap();
        app.press(Action::BC32(Press::Open(j as u8)));
        for ch in s[3..].chars() {
            app.press(Action::BC32(Press::Key(OskPress::Char(ch))));
        }
        let st = app.backup().unwrap().c32.as_ref().unwrap();
        assert!(
            matches!(
                st.found,
                Some(faraday_core::codex32_backup::Found::Holds(_))
            ),
            "{:?}",
            st.found
        );
        app.press(Action::BC32(Press::Next));
    }
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, plan::Item::Codex32(0)));
}

/// Each string on paper skipped past its caution.
fn skip_each(app: &mut Faraday) {
    for j in [ix('a'), ix('c'), ix('d')] {
        app.press(Action::BC32(Press::Open(j as u8)));
        app.press(Action::BC32(Press::Skip));
        app.press(Action::BC32(Press::Skip));
    }
}

/// A change to Back up's plan made straight to its map.
fn board_edit(app: &mut Faraday, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
    let c = testkit::compute();
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    let a = testkit::backup_answers_mut(app);
    let m = plan::map(&c, &shape, a);
    a.set_map(f(m));
}

/// The share records open vault `o` holds: each string and fingerprint.
pub(crate) fn share_records(app: &Faraday, o: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    contents(app, o)
        .of(kind::CODEX32_SHARE)
        .map(|(_, r)| {
            (
                r.field(field::CODEX32_STRING).unwrap_or_default().to_vec(),
                r.field(field::CODEX32_KEY).unwrap_or_default().to_vec(),
            )
        })
        .collect()
}

/// Unlocks vault file `name` from the Vaults tab with `phrase`, loading
/// nothing on its Load sheet.
pub(crate) fn unlock_on_vaults(app: &mut Faraday, name: &str, phrase: &str) {
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault is listed");
    let before = app.vaults().open.len();
    app.press(Action::Vault(V::Open(n)));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "{:?}",
        app.vaults().unlock_error
    );
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
}

/// A vault file whose one slot, opened by `phrase`, holds `records`.
pub(crate) fn vault_of(phrase: &str, records: Vec<Record>, seed: u8) -> Vec<u8> {
    let c = testkit::compute();
    let mut all = vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Vault")];
    all.extend(records);
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        Cost {
            memory_kib: faraday_vault::MIN_MEMORY_KIB,
            passes: 1,
            lanes: 1,
        },
        &[phrase],
        &[Contents { records: all }],
        &[seed; 32],
    )
    .unwrap()
}

/// Share `s` of vector 2's key, as a vault's share record.
fn share_record(s: &str) -> Record {
    Record::new(kind::CODEX32_SHARE)
        .with(field::CODEX32_STRING, s.as_bytes())
        .with(field::CODEX32_KEY, &FP)
}

#[test]
fn a_string_the_plan_keeps_in_a_vault_is_saved_as_a_share_record() {
    let mut app = device(vec![("home.fvlt".into(), vault_of(PHRASE, Vec::new(), 7))]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    // D, the one string not typed in, kept in the vault as well as on
    // paper.
    vault_at(&mut app, 0, 0, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('d')));
    assert!(backup_held(&app, At::Vault(0)).contains(&What::Codex32Share(0, ix('d'))));
    codex32_item(&mut app);
    // Not typed in yet: the save names the string and writes no share.
    skip_each(&mut app);
    app.press(Action::BVaultSave(0));
    assert_eq!(
        app.toast_text(),
        Some("Share D of name is not here: type it in its Codex32 strings item first")
    );
    assert!(share_records(&app, 0).is_empty());
    let c = testkit::compute();
    assert!(!app.backup_item_done_now(&c, plan::Item::Vault(0)));
    // Typed back from the paper: the save keeps D, and only D.
    type_each_back(&mut app);
    app.press(Action::BVaultSave(0));
    assert_eq!(
        share_records(&app, 0),
        vec![(D.as_bytes().to_vec(), FP.to_vec())]
    );
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(0)));
    // Saved again, it is not kept twice.
    app.press(Action::BVaultSave(0));
    assert_eq!(share_records(&app, 0).len(), 1);
}

/// The checklist's vault item names the codex32 strings the plan keeps in
/// that vault, as the board's place lines count them, and none where the
/// plan keeps none there.
#[test]
fn the_vault_item_lists_the_strings_it_will_hold() {
    let line = "1 of 2 shares of fab6868a".to_string();
    let n = bstep::of(plan::Item::Vault(0));
    for with_string in [false, true] {
        let mut app = device(vec![("home.fvlt".into(), vault_of(PHRASE, Vec::new(), 7))]);
        unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
        key_from_strings(&mut app);
        wallet_and_backup(&mut app);
        // The vault keeps the seed, and string D when asked.
        vault_at(&mut app, 0, 0, &[0]);
        if with_string {
            into_vault(&mut app, 0, Thing::String(0, ix('d')));
        }
        codex32_item(&mut app);
        type_each_back(&mut app);
        open_item(&mut app, n);
        assert_eq!(app.backup().unwrap().open, Some(n));
        let shown = texts(&mut app);
        assert_eq!(shown.contains(&line), with_string, "{shown:?}");
    }
}

/// The plan asks how many strings of a key's set there are (§9.4): its
/// stepper raises the count, and the plan and the key's facts both keep
/// the strings past those known.
#[test]
fn the_plan_asks_how_many_strings_there_are() {
    let mut app = device(Vec::new());
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    codex32_item(&mut app);
    assert!(has(&mut app, "How many strings of name are there?"));
    app.press(Action::BC32(Press::Exist(1)));
    app.press(Action::BC32(Press::Exist(1)));
    assert!(has(&mut app, "And 2 more, their index not known"));
    let k = app.session.keys.position_of(FP).unwrap();
    let facts = app.session.keys[k].codex32.clone().expect("its set");
    assert_eq!(facts.unknown, 2);
    assert_eq!(facts.indices(), vec![b'a', b'c', b'd']);
    // Fewer than those known is not offered.
    for _ in 0..5 {
        app.press(Action::BC32(Press::Exist(-1)));
    }
    let facts = app.session.keys[k].codex32.clone().expect("its set");
    assert_eq!(facts.unknown, 0);
}

/// The strings a key was typed in from are kept with it: they go to a
/// vault without being typed again (§9.5).
#[test]
fn strings_typed_in_are_kept_with_their_key() {
    let mut app = device(vec![("home.fvlt".into(), vault_of(PHRASE, Vec::new(), 8))]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    key_from_strings(&mut app);
    let k = app.session.keys.position_of(FP).unwrap();
    assert!(app.session.keys.holds_codex32(k, b'a'));
    assert!(app.session.keys.holds_codex32(k, b'c'));
    assert!(!app.session.keys.holds_codex32(k, b'd'));
    wallet_and_backup(&mut app);
    vault_at(&mut app, 0, 0, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('a')));
    app.press(Action::BVaultSave(0));
    assert_eq!(
        share_records(&app, 0),
        vec![(A.as_bytes().to_vec(), FP.to_vec())]
    );
}

/// A string typed into the codex32 form counts with the shares an open
/// vault keeps of its set (§9.5): one typed and one in the vault load the
/// key; Check a string says the vault has more of its set.
#[test]
fn a_string_typed_counts_with_a_vaults_share() {
    let mut app = device(vec![(
        "home.fvlt".into(),
        vault_of(PHRASE, vec![share_record(C)], 9),
    )]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    assert!(
        app.session.keys.is_empty(),
        "one share of 2 of 3 is nothing"
    );
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    app.press(Action::Codex32Form(faraday_core::forms::C32Press::Check));
    type_text(&mut app, D);
    app.event(Event::Key(Key::Enter));
    assert_eq!(testkit::entry(&app).codex32.check_in_vaults, 1);
    assert!(has(&mut app, "1 more of this set in open vaults"));
    assert!(app.session.keys.is_empty(), "checking loads nothing");
    app.press(Action::Codex32Form(faraday_core::forms::C32Press::Check));
    type_text(&mut app, A);
    app.event(Event::Key(Key::Enter));
    let k = app
        .session
        .keys
        .position_of(FP)
        .unwrap_or_else(|| panic!("{:?}", testkit::entry(&app).error));
    let facts = app.session.keys[k].codex32.clone().expect("its set");
    assert_eq!(facts.indices(), vec![b'a', b'c']);
    assert!(app.session.keys.holds_codex32(k, b'a'));
    assert!(app.session.keys.holds_codex32(k, b'c'));
}

#[test]
fn shares_in_two_vaults_load_the_key_once_both_are_unlocked() {
    let mut app = device(vec![
        (
            "home.fvlt".into(),
            vault_of(PHRASE, vec![share_record(A)], 3),
        ),
        (
            "bank.fvlt".into(),
            vault_of("bank phrase", vec![share_record(C)], 4),
        ),
    ]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    assert!(
        app.session.keys.is_empty(),
        "one share of 2 of 3 is nothing"
    );
    // The vault lists its share by index and key.
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Share A of fab6868a"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "codex32 name · threshold 2"),
        "{texts:?}"
    );
    unlock_on_vaults(&mut app, "bank.fvlt", "bank phrase");
    let k = app
        .session
        .keys
        .position_of(FP)
        .expect("A and C rebuild the key");
    let facts = app.session.keys[k].codex32.clone().expect("its set");
    assert_eq!((facts.threshold, &facts.identifier), (2, b"name"));
    assert_eq!(facts.indices(), vec![b'a', b'c']);
}

#[test]
fn shares_of_another_key_named_in_a_record_load_nothing() {
    // A record that names another fingerprint: the strings rebuild a seed
    // whose key is not the one named, so nothing loads.
    let wrong = |s: &str| {
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, s.as_bytes())
            .with(field::CODEX32_KEY, &[1, 2, 3, 4])
    };
    let mut app = device(vec![(
        "home.fvlt".into(),
        vault_of(PHRASE, vec![wrong(A), wrong(D)], 5),
    )]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    assert!(app.session.keys.is_empty());
}

/// Share C of another seed's 2-of-3 set under vector 2's identifier
/// (`name`) and length, as a vault's share record naming that seed's own
/// key; with that key's fingerprint.
fn other_seeds_c() -> (Record, [u8; 4]) {
    use faraday_bip::codex32::Codex32;
    let c = testkit::compute();
    let seed = [0x5au8; 16];
    let secret = Codex32::from_seed(2, *b"name", &seed).unwrap();
    let set = secret.split(2, 3, &[9u8; 64]).unwrap();
    let share = match set.iter().find(|s| s.share_index() == b'c') {
        Some(s) => Codex32::parse(s.encode().as_str()).unwrap(),
        None => Codex32::derive_share(&set[..2], b'c').unwrap(),
    };
    let fp = faraday_keys::identity::fingerprint_of_seed(&c, &seed)
        .unwrap()
        .0;
    assert_ne!(fp, FP);
    let text = share.encode().as_str().to_string();
    assert_eq!(text.len(), A.len());
    (
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, text.as_bytes())
            .with(field::CODEX32_KEY, &fp),
        fp,
    )
}

/// Types string `s` into the codex32 form and presses Enter.
fn type_string(app: &mut Faraday, s: &str) {
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    type_text(app, s);
    app.event(Event::Key(Key::Enter));
}

/// Strings of two seeds under one identifier, threshold and length in an
/// open vault: a string of vector 2's set typed in loads vector 2's key,
/// with its own seed's share, and never a key the two seeds' strings
/// would make together.
#[test]
fn a_string_typed_loads_its_own_seed_beside_another_seeds_share() {
    let (other, other_fp) = other_seeds_c();
    let mut app = device(vec![(
        "home.fvlt".into(),
        vault_of(PHRASE, vec![other, share_record(D)], 11),
    )]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    assert!(app.session.keys.is_empty());
    type_string(&mut app, A);
    let fps: Vec<[u8; 4]> = (0..app.session.keys.len())
        .map(|k| app.session.keys[k].fingerprint.0)
        .collect();
    assert!(fps.iter().all(|f| *f == FP), "{fps:?}");
    assert!(!fps.contains(&other_fp));
    let k = app
        .session
        .keys
        .position_of(FP)
        .unwrap_or_else(|| panic!("{:?}", testkit::entry(&app).error));
    let facts = app.session.keys[k].codex32.clone().expect("its set");
    assert_eq!(facts.indices(), vec![b'a', b'd']);
}

/// A string typed in beside only another seed's share of the same
/// identifier, threshold and length loads no key.
#[test]
fn a_string_typed_with_only_another_seeds_share_loads_nothing() {
    let (other, _) = other_seeds_c();
    let mut app = device(vec![(
        "home.fvlt".into(),
        vault_of(PHRASE, vec![other], 12),
    )]);
    unlock_on_vaults(&mut app, "home.fvlt", PHRASE);
    type_string(&mut app, A);
    assert!(app.session.keys.is_empty(), "a key loaded");
}

/// Vector 2's key's single-key wallet made in Create, Back up open on it
/// with nothing on paper but its strings, one per place.
fn made_with_set() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    key_from_strings(&mut app);
    app.press(Action::CreateWallet);
    let kind = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .unwrap() as u8;
    app.press(Action::CKind(kind));
    app.press(Action::CSlotHere(0, FP));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    assert_eq!(app.screen, Screen::Backup);
    no_vaults(&mut app);
    no_description_files(&mut app);
    app
}

/// Shares A and D kept on two sticks: A as its text, D as its Codex32QR.
/// A was typed in with the key, so this session holds it; D was not.
fn files_on_two_sticks(app: &mut Faraday) {
    board_edit(app, |m| {
        let m = plan::with_added(&m, At::Stick(0, 0), What::Codex32ShareFile(0, ix('a')));
        plan::with_added(&m, At::Stick(1, 0), What::Codex32QrFile(0, ix('d')))
    });
}

#[test]
fn a_string_not_here_stops_lock_and_write() {
    let mut shell = Shell::new();
    let mut app = made_with_set();
    files_on_two_sticks(&mut app);
    codex32_item(&mut app);
    skip_each(&mut app);
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        app.toast_text(),
        Some("Share D of name is not here: type it in its Codex32 strings item first")
    );
    assert!(!app.restart_requested(), "not locked");
}

/// A codex32 string's file let out for a stick is a secret let out:
/// after the lock, with the wallet loaded again, Backups says the stick
/// that keeps it is unprotected, as it says of a seed's words.
#[test]
fn backups_says_a_stick_with_a_string_let_out_is_unprotected() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let mut app = made_with_set();
    let wallet = &app.session.wallets[0];
    let (name, descriptor) = (
        wallet.name.clone(),
        wallet.policy().to_descriptor_checksummed(),
    );
    files_on_two_sticks(&mut app);
    codex32_item(&mut app);
    type_each_back(&mut app);
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let_seed_files_out(&mut app);
    shell.pump(&mut app);
    let mut app = shell.restart();
    assert!(
        app.outbox
            .iter()
            .any(|i| i.name == "codex32-fab6868a-a.txt" && i.secret),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    app.session
        .add_wallet(
            &c,
            &name,
            &descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let all = app.backups_now(&c);
    let lines = all.iter().find_map(|e| e.lines.clone()).unwrap_or_else(|| {
        panic!(
            "no plan: {:?}",
            all.iter().map(|e| &e.name).collect::<Vec<_>>()
        )
    });
    // Place 1's stick keeps share A's file, Place 2's D's picture, Place
    // 3's none.
    let state = |p: usize| {
        lines
            .iter()
            .find(|l| l.at == Some(At::Place(p)))
            .map(|l| l.state.clone())
            .unwrap_or_else(|| panic!("{lines:?}"))
    };
    assert_eq!(state(0), "For the stick, unprotected", "{lines:?}");
    assert_eq!(state(1), "For the stick, unprotected", "{lines:?}");
    assert_eq!(state(2), "For the stick", "{lines:?}");
}

#[test]
fn strings_on_two_sticks_are_written_and_rebuild_the_key() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    for k in 1..=4u32 {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k);
        shell.add(d);
    }
    let mut app = made_with_set();
    files_on_two_sticks(&mut app);
    codex32_item(&mut app);
    type_each_back(&mut app);
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let_seed_files_out(&mut app);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "locked: {:?}", app.toast_text());
    let mut app = shell.restart();
    let n = app.flow().unwrap().slots.len();
    for k in 0..n {
        let d = format!("S{}", k + 1);
        shell.plug(&mut app, &d);
        for _ in 0..3 {
            let r = app.flow().and_then(|f| f.live.reading.as_ref());
            let Some(r) = r else { break };
            if r.problem.is_some() || r.bound {
                break;
            }
            let a = if r.hold { F::Hold } else { F::Tap };
            shell.flow(&mut app, a);
        }
        shell.pull(&mut app, &d);
    }
    assert_eq!(
        app.flow().unwrap().step,
        Step::Done,
        "{n} sticks: {:?} {:?}",
        app.flow().unwrap().live.problem,
        app.flow()
            .unwrap()
            .slots
            .iter()
            .map(|s| s.written.is_some())
            .collect::<Vec<_>>()
    );
    let files: Vec<(String, Vec<u8>)> = (1..=n)
        .flat_map(|k| shell.disk(&format!("S{k}")).files.clone())
        .collect();
    let text = files
        .iter()
        .find(|(n, _)| n == "codex32-fab6868a-a.txt")
        .unwrap_or_else(|| panic!("{:?}", files.iter().map(|f| &f.0).collect::<Vec<_>>()));
    assert_eq!(text.1, format!("{A}\n").into_bytes());
    let picture = files
        .iter()
        .find(|(n, _)| n == "codex32qr-fab6868a-d.png")
        .expect("D's Codex32QR");
    let read = faraday_files::qr_in_png(&c, &picture.1).unwrap().to_vec();
    assert_eq!(read, vec![D.to_ascii_uppercase().into_bytes()]);
    // The two files, copied into a Faraday with nothing in it, are the
    // key's seed.
    let mut back = testkit::started();
    back.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![text.clone()],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    back.storage(faraday_core::StorageEvent::QrRead {
        name: picture.0.clone(),
        bytes: picture.1.clone(),
        payloads: read,
    });
    assert!(
        back.inbox_found_now(&c)
            .seeds
            .iter()
            .any(|s| s.fingerprint.0 == FP),
        "A and D rebuild the key"
    );
}

#[test]
fn a_place_counts_its_shares() {
    let mut app = device(Vec::new());
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    board(&mut app, BoardPress::Customize);
    let line = |app: &mut Faraday, _p: usize| -> Vec<String> { texts(app) };
    let one = line(&mut app, 0);
    assert!(
        one.contains(&"1 of 2 shares of fab6868a · alone: nothing".to_string()),
        "{one:?}"
    );
    move_to(
        &mut app,
        Zone::Paper(1),
        Thing::String(0, ix('c')),
        Zone::Paper(0),
    );
    let two = line(&mut app, 0);
    assert!(
        two.contains(&"2 of 2 shares of fab6868a: the seed".to_string()),
        "{two:?}"
    );
}

/// A codex32 string's file on a stick at a place that the plan came with,
/// not dropped there, asks **Save without encryption?**'s hold at Lock and
/// write, once, and nothing of it is made or locked before (F129); Cancel
/// stops the lock and leaves the plan as it is.
#[test]
fn a_string_file_the_plan_came_with_asks_the_hold_once_at_lock_and_write() {
    let mut shell = Shell::new();
    let mut app = made_with_set();
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::Codex32ShareFile(0, ix('a')))
    });
    assert_eq!(testkit::plain_asked(&app), None, "nothing asked before");
    codex32_item(&mut app);
    type_each_back(&mut app);
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let (name, on) = testkit::plain_asked(&app).expect("the hold, at Lock and write");
    assert_eq!(on, "stick at Place 1");
    let made = |app: &Faraday| app.outbox.iter().any(|i| i.name == name);
    assert!(!made(&app), "nothing made before the hold");
    assert!(!app.restart_requested(), "not locked");
    // Cancel: no lock, and the plan keeps the file.
    app.press(Action::Cancel);
    assert_eq!(testkit::plain_asked(&app), None);
    shell.pump(&mut app);
    assert!(!app.restart_requested(), "not locked");
    assert!(!made(&app));
    let kept = plan::holds_at(&backup_map(&app), At::Stick(0, 0));
    assert!(
        kept.contains(&What::Codex32ShareFile(0, ix('a'))),
        "{kept:?}"
    );
    // Again, held: the one file, then the lock.
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(let_seed_files_out(&mut app), 1, "asked once");
    assert!(app.outbox.iter().any(|i| i.name == name && i.secret));
    shell.pump(&mut app);
    assert!(app.restart_requested(), "locked: {:?}", app.toast_text());
}
