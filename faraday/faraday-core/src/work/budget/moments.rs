//! The moments a person waits on do a fixed amount of costly work
//! (`docs/SPEED.md` §8): the app started to Home, a vault unlocked to its
//! Load sheet and loaded, a PSBT opened to its review, and signed. Each
//! moment's work, frames included, is exactly its table: a step that
//! starts doing a costly thing twice, or something new, fails here.

use faraday_shell_api::{App, Event};

use super::type_text;
use crate::vault::VaultAction as V;
use crate::work::{Counts, KINDS, Work, since, snapshot};
use crate::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};

/// What a moment works out, exactly: (kind, count, why). Every kind not
/// listed is none.
type Table = &'static [(Work, u64, &'static str)];

/// The FROST wallet's record and its spend on the test stick, and its
/// record in the test vault: there only in a build with `unreleased`,
/// FROST being compiled out of the app (docs/decisions/F109.md).
const FROST: u64 = if cfg!(feature = "unreleased") { 1 } else { 0 };

/// Checks `work` against `table`.
fn check(moment: &str, work: &Counts, table: Table) {
    let mut wrong = Vec::new();
    for k in KINDS {
        let want = table.iter().find(|t| t.0 == k).map_or(0, |t| t.1);
        let n = work.of(k);
        if n != want {
            wrong.push(format!("{k:?} {n} (budget {want})"));
        }
    }
    assert!(wrong.is_empty(), "{moment}: {}", wrong.join(", "));
}

/// The press, then frames and ticks until it asks for nothing more.
fn press_and_draw(app: &mut Faraday, a: Action) {
    app.press(a);
    for t in 1..4u64 {
        let _ = app.frame();
        let now = app.now_ms;
        app.event(Event::Tick {
            now_ms: now + t * 16,
        });
    }
    let _ = app.frame();
}

const STARTUP: Table = &[
    (
        Work::DescriptorParse,
        56 + FROST,
        "the classifier tries each text file of the test stick that is no other kind as a wallet description, once as it comes in (40 of its 79 files; 41 of 82 with FROST's three): the 24 wallet files (25 with FROST's) read once each; the 16 that are none (three seeds' words, twelve xpubs, the vault's entries) refused whole and then read line by line as descriptors (faraday-intake's multipath reader), two parses each; what a wallet file says is kept on it (Item::wallet)",
    ),
    (
        Work::FileHash,
        2,
        "the test stick's vault and EFI image, each hashed once as it comes in (Item::sha)",
    ),
    (
        Work::PsbtParse,
        20 + FROST,
        "each of the test stick's 20 PSBTs (21 with FROST's) parsed once as it comes in, its summary kept (Item::psbt)",
    ),
];

#[test]
fn startup_to_home_does_its_fixed_work() {
    let c = crate::testkit::compute();
    // The self-test's own work (`faraday_selftest`), known vectors run at the
    // first Display: whatever it is, the start does it once.
    let before = snapshot();
    assert!(faraday_selftest::run(&c).is_ok());
    let selftest = since(&before);
    // The test stick's files, made (signed, some of them) before the
    // count starts.
    let files = testkit::files().expect("the test stick's files");
    let before = snapshot();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    let work = since(&before).since(&selftest);
    assert_eq!(app.screen, Screen::Home);
    let psbts = app.inbox.iter().filter(|i| i.psbt().is_some()).count() as u64;
    assert_eq!(
        work.of(Work::PsbtParse),
        psbts,
        "each PSBT file is parsed once as it comes in"
    );
    check("startup to Home", &work, STARTUP);
}

/// The test vault opened on the Vaults tab with its first passphrase
/// typed, the stick's files in the Inbox, ready to unlock.
fn at_unlock() -> Faraday {
    let files = testkit::files().expect("the test stick's files");
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.vaults.set_speed(Some(180));
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == "vault.ofv")
        .expect("the test vault is listed");
    app.press(Action::Vault(V::Open(n)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    let _ = app.frame();
    app
}

const UNLOCK: Table = &[
    (
        Work::Kdf,
        5,
        "Argon2id once over the passphrase; PBKDF2 for the vault's seed (its identity, kept) and for the three seed files Files lists under the Load sheet (their fingerprints, kept)",
    ),
    (
        Work::MasterKey,
        4,
        "the vault's seed and the three seed files in Files, each made a master key once for its fingerprint, then kept",
    ),
    (
        Work::WordsParse,
        5,
        "the three seed files in Files once each; the vault's key record twice, for its words' hash (VaultRecords) and its identity (key_identity), each kept",
    ),
    (
        Work::FileHash,
        2,
        "the vault seed's words' hash and its master key's id, each kept",
    ),
    (
        Work::KeyDerive,
        1,
        "the vault seed's master key's xpub, for its id (key_identity), kept",
    ),
    (
        Work::DescriptorParse,
        13 + FROST,
        "each of the vault's 11 wallet records (12 with FROST's) read once and kept (VaultRecords); Savings' share sheets in Files put together, the config they make read once for its name (config_name_kept) and once for its wallet (read_kept), each kept",
    ),
    (
        Work::Blur,
        1,
        "the Load sheet frosts the page under it once",
    ),
    (
        Work::FullCanvas,
        4,
        "the change to Files fades from the Vaults tab (one copy, blended on the frames after), and the sheet keeps its frosted page and puts it back; a step of the fade after the sheet opens blends the two kept frames and draws nothing again",
    ),
];
const LOAD: Table = &[
    (
        Work::WordsParse,
        2,
        "the vault's seed read from its record, then parsed again as it is loaded (Session::add_words_with)",
    ),
    (Work::Kdf, 1, "PBKDF2 for the seed loaded"),
    (
        Work::MasterKey,
        1,
        "the seed loaded made a master key, its fingerprint kept on the key",
    ),
    (
        Work::FileHash,
        1,
        "the loaded key's id (wallet::master_id), kept on the key",
    ),
    (
        Work::KeyDerive,
        1,
        "the loaded key's master xpub, for its id (wallet::master_id), kept on the key",
    ),
    (
        Work::Address,
        12 + FROST,
        "Wallets opens on a wallet's card, which shows its first receive address: derived once, then kept (1); each script a wallet derives for the PSBTs in Files (WalletMatch below) is a script derivation, counted at its source (11, 12 with FROST's wallet)",
    ),
    (
        Work::WalletMatch,
        11 + FROST,
        "each card's first button asks which wallet each PSBT in Files spends from: each wallet derives its script at a PSBT's places the first time it is asked, then keeps it (one more with FROST's wallet)",
    ),
    (
        Work::FullCanvas,
        5,
        "the change to Wallets fades from the frame before: one copy, blended on each frame after",
    ),
];

#[test]
fn unlock_to_the_load_sheet_and_load_do_their_fixed_work() {
    let mut app = at_unlock();
    let before = snapshot();
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        if app.sheet == Some(Sheet::UnlockLoad) {
            break;
        }
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad), "the Load sheet opened");
    let _ = app.frame();
    check("unlock to the Load sheet", &since(&before), UNLOCK);

    let before = snapshot();
    press_and_draw(&mut app, Action::Vault(V::LoadTicked));
    assert!(
        !app.session.wallets.is_empty(),
        "the vault's wallets loaded"
    );
    check("Load ticked", &since(&before), LOAD);
}

/// Savings and its unsigned spend in the Inbox, with the two test seeds
/// that sign it.
fn with_spend() -> Faraday {
    let c = crate::testkit::compute();
    let mut app = testkit::started();
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    for (word, l) in &testkit::TEST_SEEDS[..2] {
        let _ = app
            .session
            .keys
            .add_words(&c, &testkit::test_words(word), l, None);
    }
    let kits = testkit::kits();
    let kit = kits.iter().find(|k| k.id == "savings").expect("Savings");
    app.session
        .add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            crate::wallet::Origin::Named,
        )
        .expect("Savings loads");
    let psbt = testkit::unsigned(kit).expect("Savings' spend");
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings-unsigned.psbt".into(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app
}

const OPEN: Table = &[
    (
        Work::PsbtParse,
        1,
        "the file's bytes parsed once to open the spend (Files keeps only its summary)",
    ),
    (
        Work::PsbtInspect,
        1,
        "inspected once, kept on the spend until the PSBT or the session changes (SpendState::inspected)",
    ),
    (
        Work::KeyDerive,
        12,
        "the inspection's context: each of the two seeds' six account xpubs (KeyRef::from_master)",
    ),
    (
        Work::WalletMatch,
        1,
        "Savings derives its script at the input's place once, then keeps it (Wallet::script_at)",
    ),
    (
        Work::Address,
        4,
        "Savings' script at the input's place for WalletMatch (1); the inspection derives it at the places the PSBT claims are Savings', to verify them: the input's wallet and its spend route, and the change output (3)",
    ),
    (
        Work::FullCanvas,
        5,
        "the change to the spend's screen fades from the frame before: one copy, blended on each frame after",
    ),
];
const SIGN: Table = &[
    (Work::Sign, 1, "one signing pass with both seeds"),
    (
        Work::PsbtInspect,
        2,
        "the signing's own inspection before it signs (faraday_psbt::sign), and the signed PSBT's, kept on the spend; the keys are chosen from the spend's kept inspection",
    ),
    (
        Work::PsbtParse,
        1,
        "the signed PSBT put For the stick, parsed once as it comes into the Outbox",
    ),
    (
        Work::KeyDerive,
        44,
        "three contexts of the two seeds' six account xpubs each (36): the signing's, the signed PSBT's inspection and the nonce check's (kept on the spend); each seed's key derived and checked against the input's key as it signs (4); each signature's key derived again to recompute its nonce (4)",
    ),
    (
        Work::DescriptorParse,
        2,
        "the finished transaction put For the stick, tried as a wallet description as it is classified: refused whole, then line by line (faraday-intake's multipath reader)",
    ),
    (
        Work::Address,
        6,
        "each of the two inspections (the signing's own and the signed PSBT's) derives Savings' script at the input's and the change output's places to verify the claims (3 each)",
    ),
    (
        Work::FullCanvas,
        4,
        "the spend moves to its next page and fades from the frame before: one copy, blended on each frame after",
    ),
];

#[test]
fn a_psbt_opened_to_its_review_and_signed_do_their_fixed_work() {
    let mut app = with_spend();
    let before = snapshot();
    press_and_draw(&mut app, Action::StartSpend(0));
    assert_eq!(app.screen, Screen::Spend);
    check("a PSBT opened to its review", &since(&before), OPEN);

    let before = snapshot();
    press_and_draw(&mut app, Action::SignHere);
    let s = app.wallets.spend().expect("the spend");
    assert_eq!(s.spend.signed_here.len(), 2, "both seeds signed");
    check("Sign here", &since(&before), SIGN);
}
