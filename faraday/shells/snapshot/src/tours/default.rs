//! The default tour.

use super::boot::boot_tour;
use super::devices::devices_tour;
use super::seeds::seeds_tour;
use super::spend::spend_tour;
use crate::tour::{
    Tour, copy_digits, flow_at_make, flow_made, scroll_to, shares_on_paper, type_text, type_words,
};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Medium, Screen, StorageCommand, StorageEvent, bstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key};

/// The New key quiz, every word answered rightly.
pub(crate) fn pass_quiz(t: &mut Tour) {
    for _ in 0..30 {
        let Some(q) = t.app.keygen().and_then(|k| k.quiz.as_ref()) else {
            return;
        };
        if q.state() == faraday_core::keygen::QuizState::Passed {
            return;
        }
        let slot = q.correct_slot() as u8;
        t.press(Action::KQuiz(slot));
    }
}

/// A new key for a Create slot from 99 dice rolls, with a picture of each
/// card on the way.
pub(crate) fn dice_key(t: &mut Tour, slot: u8) -> Result<(), String> {
    t.press(Action::KeyGen(Some(slot)));
    // Start's Continue: the Length card open.
    t.press(Action::KNext);
    t.shot("keygen-length")?;
    t.press(Action::KWords(24));
    t.shot("keygen-source")?;
    t.press(Action::KGroup(1));
    t.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    t.shot("keygen-source-computed")?;
    t.press(Action::KNext);
    // Rolls a die would give: a fixed run from a small generator.
    let mut x: u32 = 0x2545_f491;
    for i in 0..99 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        t.press(Action::KRoll((x % 6) as u8 + 1));
        if i == 40 {
            t.shot("keygen-rolls")?;
        }
    }
    // The randomness check, shown on the entry card once the last roll
    // is in (`docs/history/NEW-WALLET.md` §3.2), before Continue moves on.
    t.shot("keygen-rolls-done")?;
    t.press(Action::KNext);
    t.shot("keygen-slot-key")?;
    t.press(Action::Learn);
    t.shot("learn-keygen")?;
    // The next page, scrolled to the dice table's em dashes.
    t.press(Action::LearnPage(1));
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 640,
        y: 400,
        dy: 260,
    });
    t.shot("learn-randomness-sources")?;
    t.press(Action::Cancel);
    t.press(Action::KLock);
    t.shot("keygen-slot-key-locked")?;
    Ok(())
}

/// New key from coin flips read off a die: the words as the flips come
/// in, a word in its list, the rest typed as a string, the checksum's
/// last word once the words are made; and the Word lists tile.
pub(crate) fn word_lists(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::{Way, kstep};
    use faraday_core::wordlist::WordListAction as WL;
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::Coins.index()));
    t.press(Action::KNext);
    t.press(Action::KByDie(true));
    t.press(Action::KTyping(false));
    for i in 0..60u32 {
        t.press(Action::KDie(((i * 5 + i / 3) % 6) as u8 + 1));
    }
    t.shot("keygen-coins-by-die")?;
    let first = t
        .app
        .keygen()
        .and_then(|k| k.live_words().first().copied())
        .ok_or("sixty flips named no word")?;
    t.press(Action::WordList(WL::Open(0, Some(first))));
    t.shot("wordlist-bip39-marked")?;
    t.press(Action::WordList(WL::Close));
    t.press(Action::KTyping(true));
    let rest: String = (0..68u32)
        .map(|i| char::from(b'1' + ((i * 7 + i / 4) % 6) as u8))
        .collect();
    type_text(t, &rest);
    t.shot("keygen-coins-typed")?;
    t.press(Action::KNext);
    t.press(Action::KStep(kstep::ENTER));
    t.shot("keygen-coins-made")?;
    for _ in 0..5 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("keygen-coins-checksum")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(
        faraday_core::catalog::TILES
            .iter()
            .position(|x| x.name == "EFF large list")
            .unwrap_or(0) as u8,
    ));
    type_text(t, "zeb");
    t.shot("wordlist-eff-large-find")?;
    t.press(Action::WordList(WL::Close));
    Ok(())
}

/// New key from rolls that name words (BitBox): the words as the rolls
/// come in, each roll over its bits, then the last word's rolled bits and
/// the checksum once the words are made.
pub(crate) fn rolled_words(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::{Way, kstep};
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::DiceWords.index()));
    t.press(Action::KNext);
    let rolls: String = (0..27u32)
        .map(|i| {
            let face = if i % 6 == 5 {
                i % 6 + 1
            } else {
                (i * 3 + i / 6) % 4 + 1
            };
            char::from(b'0' + face as u8)
        })
        .collect();
    type_text(t, &rolls);
    t.shot("keygen-bitbox-rolls")?;
    let rest: String = (27..72u32)
        .map(|i| {
            let face = if i % 6 == 5 {
                i % 5 + 2
            } else {
                (i * 7 + i / 5) % 4 + 1
            };
            char::from(b'0' + face as u8)
        })
        .collect();
    type_text(t, &rest);
    t.press(Action::KNext);
    t.press(Action::KStep(kstep::ENTER));
    // A frame first, so the keys have a scrolled region to move.
    let _ = t.app.frame();
    for _ in 0..14 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("keygen-bitbox-made")?;
    // On its own, New key goes from Key to Words and the Quiz, the key
    // added at Lock in and the Quiz's Done leaving the flow
    // (`docs/history/NEW-WALLET.md` §3.4). The words made opened the Key
    // card already.
    t.shot("keygen-key-own")?;
    t.press(Action::KLock);
    // Lock in went on to Words by itself.
    t.shot("keygen-key-own-locked")?;
    t.shot("keygen-words")?;
    t.press(Action::KShow);
    t.shot("keygen-words-shown")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("keygen-quiz")?;
    pass_quiz(t);
    t.shot("keygen-quiz-passed")?;
    t.press(Action::KAdd);
    t.press(Action::Nav(Screen::Catalog));
    Ok(())
}

/// Add a key from the lists typed on keyboards of their own: Japanese
/// with a word half typed, Korean, Simplified Chinese with a reading
/// awaiting its tone and with its characters offered, and Traditional.
pub(crate) fn keyed_entry(t: &mut Tour) -> Result<(), String> {
    use faraday_bip::bip39::Language;
    use faraday_core::forms::LANGUAGES;
    let at = |l: Language| LANGUAGES.iter().position(|x| *x == l).unwrap_or(0) as u8;
    t.press(Action::Entry(None));
    t.press(Action::EntryLanguages);
    t.press(Action::EntryLanguage(at(Language::Japanese)));
    t.press(Action::EntryKey('あ'));
    t.shot("entry-japanese")?;
    for _ in 0..13 {
        t.press(Action::EntryKey('あ'));
        t.press(Action::EntryCandidate(0));
    }
    t.press(Action::EntryKey('か'));
    t.shot("entry-japanese-thirteen")?;
    t.press(Action::EntryLanguage(at(Language::Korean)));
    t.shot("entry-korean")?;
    t.press(Action::EntryLanguage(at(Language::ChineseSimplified)));
    type_text(t, "shi");
    t.shot("entry-chinese-pinyin")?;
    type_text(t, "4");
    t.shot("entry-chinese-pinyin-tone")?;
    t.press(Action::EntryLanguage(at(Language::ChineseTraditional)));
    t.shot("entry-chinese-zhuyin")?;
    t.press(Action::Nav(Screen::Wallets));
    Ok(())
}

/// A key's silent payments wallet among the wallets: its card, and its
/// page with Check a payment open.
pub(crate) fn silent_wallet(t: &mut Tour) -> Result<(), String> {
    use faraday_core::silent::sstep;
    t.press(Action::Silent);
    if let Some(fp) = t.app.session.keys.first().map(|k| k.fingerprint.0) {
        t.press(Action::SKey(fp));
    }
    t.press(Action::SAddWallet);
    let i = t.app.session.wallets.len().saturating_sub(1);
    t.press(Action::PickWallet(i));
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallet-silent")?;
    t.press(Action::SWallet(i));
    t.press(Action::SStep(sstep::CHECK));
    t.shot("silent-check")?;
    t.press(Action::Nav(Screen::Wallets));
    Ok(())
}

pub(crate) fn fp_hex(fp: [u8; 4]) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

/// Costs what `visit_tour`, `copy_tour`, `public_tour` and the rest it
/// calls cost.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn default_tour(c: &Compute, t: &mut Tour, medium: Medium) -> Result<(), String> {
    if medium == Medium::Stick {
        devices_tour(t)?;
    } else {
        t.shot("home-empty")?;
    }
    // Settings' Time section, and Set the time with the clock not known
    // yet (`docs/TOTP.md` §9).
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings-time")?;
    t.press(Action::OpenSetTime);
    t.shot("set-time")?;
    t.press(Action::SetTimeBack);
    t.press(Action::Nav(Screen::Home));
    // A stick arrives while nothing secret is held: the visit.
    t.sticks(true);
    // Every file Faraday reads comes in ticked but a seed's.
    t.shot("visit")?;
    // A file on the stick: its ⋮, Rename, Delete from the stick and
    // Details (`docs/FLOWS.md` decision 61 item 4).
    let k = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n.ends_with(".psbt"))
        .ok_or("the kit has no PSBT")?;
    t.press(Action::File(FilePress::Menu(Row::Stick(k))));
    t.shot("visit-file-menu")?;
    t.press(Action::Cancel);
    // Delete files: one sheet, chosen files, the free space or the whole
    // partition.
    t.press(Action::DeleteFiles);
    t.press(Action::DeletePick(1));
    t.shot("visit-delete-files")?;
    t.press(Action::DeleteWhat(1));
    t.shot("visit-delete-free")?;
    t.press(Action::DeleteWhat(2));
    t.press(Action::DeletePasses(2));
    t.shot("visit-delete-whole")?;
    // Zeroing is a hold, at the sheet's foot (F131).
    scroll_to(t, Action::Vault(V::HoldWipe));
    t.shot("visit-delete-whole-hold")?;
    t.press(Action::Cancel);
    t.press(Action::VisitInAll);
    for name in [
        "savings-unsigned.psbt",
        "taproot-multisig-unsigned.psbt",
        "taproot-multisig-cosigner-5d388376.psbt",
        "spending-unsigned.psbt",
        "savings-cosigner-5d388376.psbt",
        "inheritance-unsigned.psbt",
    ] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.shot("visit-chosen")?;
    t.press(Action::VisitCopy);
    t.shot("visit-copied")?;
    // A picture of a wallet's QR code is read off the stick into Files.
    let before = t.app.inbox().len();
    let png = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "nested-wallet-qr.png")
        .ok_or("the kit has no nested-wallet-qr.png")?;
    // Ticked like any file: its codes are read on Import.
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(png));
    t.press(Action::VisitCopy);
    // What its code holds, and the picture itself, kept to copy on.
    if t.app.inbox().len() != before + 2 {
        return Err("the QR code in nested-wallet-qr.png did not reach Files".into());
    }
    t.shot("visit-png-read")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-stick-attached")?;
    // Add a key with the stick in: the sheet asks for it to be pulled,
    // and Add a key opens when it is.
    t.press(Action::Entry(None));
    t.shot("home-pull")?;
    t.sticks(false);
    t.shot("entry-after-pull")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-files")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("files")?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("start-before")?;
    t.load_kit()?;
    t.shot("start")?;
    t.press(Action::OpenWallet(0));
    t.shot("wallet-card")?;
    t.press(Action::Entry(Some([0x5d, 0x38, 0x83, 0x76])));
    for c in "zebra zebra zeb".chars() {
        t.app.event(Event::Key(if c == ' ' {
            Key::Char(' ')
        } else {
            Key::Char(c)
        }));
    }
    t.shot("entry-typing")?;
    // The other forms: one SLIP-39 share in, the next being typed.
    t.press(Action::EntryForm(1));
    type_text(
        t,
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    );
    t.app.event(Event::Key(Key::Enter));
    type_text(t, "shadow pistol academic acid");
    t.shot("entry-slip39")?;
    t.press(Action::EntryForm(0));
    // Spanish words, typed without their accents.
    t.press(Action::EntryLanguage(1));
    type_text(
        t,
        "abaco abdomen abeja abierto abogado abono aborto abrazo abrir abuelo abuso acabar",
    );
    t.shot("entry-spanish")?;
    t.press(Action::EntryClear);
    t.press(Action::EntryLanguage(0));
    t.press(Action::Nav(Screen::Wallets));
    let psbt = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .ok_or("no psbt")?;
    t.press(Action::StartSpend(psbt));
    t.shot("spend-transaction")?;
    t.press(Action::ToggleTable);
    t.shot("spend-table")?;
    t.press(Action::StepNext(2));
    t.shot("spend-check")?;
    t.press(Action::StepNext(1));
    t.shot("spend-signers")?;
    t.press(Action::StepNext(4));
    t.shot("spend-sign")?;
    t.press(Action::SignHere);
    t.shot("spend-signed")?;
    let copy = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-cosigner-5d388376.psbt")
        .ok_or("no copy")?;
    t.press(Action::Collect(copy));
    t.shot("spend-finished")?;
    // The end of signing: the nonce check, the raw hex, and the
    // transaction taken apart.
    t.press(Action::SpendHex);
    t.app.list_offset = 0.0;
    // Drawn before the scroll, as a shell draws after each press: how far
    // the page now goes is what the scroll stops at.
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 700,
        y: 400,
        dy: 600,
    });
    t.shot("spend-nonce-check")?;
    t.press(Action::SpendHex);
    t.press(Action::DecodeFinished);
    t.shot("decode")?;
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 700,
        y: 400,
        dy: 500,
    });
    t.shot("decode-outputs")?;
    // The Spend tab with wallets and seeds loaded: what is loaded and the
    // next step for each.
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-loaded")?;
    t.press(Action::Nav(Screen::Spend));
    t.press(Action::QrSigned);
    t.shot("qr-signed-psbt")?;
    t.press(Action::Cancel);
    // Finish put both files For the stick; the primary opens Files.
    t.shot("spend-in-outbox")?;
    t.press(Action::Primary);
    t.shot("files-outbox")?;
    // A file For the stick: its ⋮ and the Rename popup (`docs/FLOWS.md`
    // decision 61).
    t.press(Action::File(FilePress::Menu(Row::Out(0))));
    t.shot("files-menu")?;
    t.press(Action::File(FilePress::Rename(Row::Out(0))));
    t.shot("files-rename")?;
    t.press(Action::Cancel);
    // A single-key spend: no signers or signatures steps.
    let single = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "spending-unsigned.psbt")
        .ok_or("no single")?;
    t.press(Action::StartSpend(single));
    t.shot("single-check")?;
    for n in [1u8, 2, 3] {
        t.press(Action::StepNext(n));
    }
    t.shot("single-sign")?;
    t.press(Action::SignHere);
    t.shot("single-finished")?;
    // A Taproot multisig spend, signed here and collected.
    let tap = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "taproot-multisig-unsigned.psbt")
        .ok_or("no tap")?;
    t.press(Action::StartSpend(tap));
    for n in [1u8, 2, 3, 4] {
        t.press(Action::StepNext(n));
    }
    t.press(Action::SignHere);
    t.shot("tapmulti-signed")?;
    let tcopy = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "taproot-multisig-cosigner-5d388376.psbt")
        .ok_or("no tap copy")?;
    t.press(Action::Collect(tcopy));
    t.shot("tapmulti-finished")?;
    // MuSig2, signing last: both nonces in, sign, both partials in.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    let names: Vec<String> = t.stick().files.iter().map(|(n, _)| n.clone()).collect();
    t.press(Action::VisitInAll);
    for (i, n) in names.iter().enumerate() {
        if n.starts_with("musig-") && n.ends_with(".psbt") || n == "inheritance-unsigned.psbt" {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    t.sticks(false);
    t.load_kit()?;
    let mu = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "musig-unsigned.psbt")
        .ok_or("no musig psbt")?;
    t.press(Action::StartSpend(mu));
    for n in [1u8, 2, 3, 4] {
        t.press(Action::StepNext(n));
    }
    for name in ["musig-nonce-5d388376.psbt", "musig-nonce-cf2e083d.psbt"] {
        let i = t
            .app
            .inbox()
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::Collect(i));
    }
    t.shot("musig-nonces")?;
    t.press(Action::StepNext(9));
    t.press(Action::SignHere);
    for name in ["musig-partial-5d388376.psbt", "musig-partial-cf2e083d.psbt"] {
        let i = t
            .app
            .inbox()
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::Collect(i));
    }
    t.shot("musig-finished")?;
    if t.app.spend().is_none_or(|s| s.spend.finished.is_none()) {
        return Err(format!(
            "the MuSig2 spend did not finish: {:?}",
            t.app.spend().and_then(|s| s.error.clone())
        ));
    }
    println!("MuSig2 spend finished in the app");
    // Restore Savings from two of its three shares, then type test key 1.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    t.press(Action::VisitInAll);
    for name in ["savings-share-1-of-3.txt", "savings-share-2-of-3.txt"] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    // The descriptor files too, for the wallets loaded after the restore.
    t.copy_wallets();
    t.sticks(false);
    t.press(Action::RestoreWallet);
    for name in ["savings-share-1-of-3.txt", "savings-share-2-of-3.txt"] {
        let i = t
            .app
            .inbox()
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::RShare(i));
    }
    t.press(Action::RStep(faraday_core::rstep::DESCRIPTION));
    t.shot("restore-shares")?;
    t.press(Action::RRebuild);
    // The question of vaults, in its turn before the seeds.
    t.press(Action::RVaults(false));
    t.press(Action::RNext(faraday_core::rstep::WHERE));
    let fp = t
        .app
        .session
        .wallets
        .first()
        .and_then(|w| w.policy().keys()[0].fingerprint())
        .ok_or("no wallet")?;
    t.press(Action::Entry(Some(fp.0)));
    t.type_key(0);
    t.shot("restore-seeds")?;
    if t.app.screen != Screen::Restore {
        return Err("adding the key did not return to the restore".into());
    }
    // A SeedQR held up to the camera from Add a key: Test key 2's words.
    let keys_before = t.app.session.keys.len();
    t.press(Action::Entry(None));
    t.press(Action::ScanSeed);
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
    t.shot("scan-seedqr")?;
    let zebra = faraday_bip::bip39::Mnemonic::parse(
        c,
        faraday_bip::bip39::Language::English,
        &testkit::test_words("zebra"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let digits = faraday_codec::seedqr::to_digits(&zebra);
    t.app.event(Event::Scanned {
        bytes: digits.expose().as_bytes().to_vec(),
    });
    t.pump();
    // The seed read asks how it loads: without a passphrase, or with one
    // typed twice, each with its key.
    if t.app.sheet != Some(faraday_core::Sheet::SeedRead) {
        return Err("the scanned SeedQR did not ask how it loads".into());
    }
    t.shot("scan-seedqr-read")?;
    t.press(Action::SeedRead(faraday_core::seeds::read::Choice::AddPass));
    type_text(t, "a passphrase");
    t.press(Action::SeedRead(faraday_core::seeds::read::Choice::Field(
        1,
    )));
    type_text(t, "a passphrase");
    t.shot("scan-seedqr-passphrase")?;
    t.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    let zebra_fp = [0x5d, 0x38, 0x83, 0x76];
    if t.app.session.keys.len() != keys_before + 1
        || !t
            .app
            .session
            .keys
            .all()
            .any(|k| k.fingerprint.0 == zebra_fp)
    {
        return Err("the scanned SeedQR did not add Test key 2".into());
    }
    t.shot("scan-seedqr-added")?;
    println!("SeedQR scanned into Add a key");
    t.load_kit()?;
    // The camera: a frame, then the Savings PSBT as animated UR parts.
    let savings = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    t.press(Action::Scan);
    let frame: Vec<u8> = (0..640u32 * 480)
        .map(|i| ((i % 640) / 3 + (i / 640) / 4) as u8)
        .collect();
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: frame,
        chroma: None,
    });
    let mut enc = faraday_codec::ur::Encoder::psbt(&savings, 120).map_err(|e| format!("{e:?}"))?;
    let mut parts = 0;
    while t.app.scan().is_some() && parts < 40 {
        let part = enc.next_part();
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
        parts += 1;
        if parts == 2 {
            // Two cameras, and a code found where the scanner saw it.
            t.app.storage(StorageEvent::Cameras(vec![
                ("/dev/video0".into(), "Integrated Camera".into()),
                ("/dev/video2".into(), "USB Camera".into()),
            ]));
            t.app.storage(StorageEvent::QrSeen {
                width: 640,
                height: 480,
                corners: [(220, 120), (430, 130), (420, 340), (210, 330)],
                module_tenths: 60,
                read: true,
            });
            t.shot("scan-progress")?;
        }
    }
    t.pump();
    let got = t
        .app
        .inbox()
        .iter()
        .find(|i| i.name.starts_with("scanned-"))
        .ok_or("nothing scanned")?;
    if got.bytes != savings {
        return Err("the scanned PSBT differs from the one shown".into());
    }
    println!("scanned the Savings PSBT in {parts} UR parts");
    // The Inheritance wallet: the Path step.
    let inh = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "inheritance-unsigned.psbt");
    if let Some(i) = inh {
        t.press(Action::StartSpend(i));
        t.press(Action::StepNext(1));
        t.shot("inheritance-path")?;
    }
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-list")?;
    // The backup of Savings: the presets, then paper only, each place
    // with its own share.
    t.press(Action::Backup(0));
    t.shot("backup-plan")?;
    t.backup_as(faraday_core::plan::Preset::Paper);
    shares_on_paper(c, t);
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::BLANK));
    t.shot("backup-blank")?;
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.shot("backup-seed")?;
    // The copy scanned with its third word drawn wrong.
    t.press(Action::BScan);
    let wrong = copy_digits(c, t, Some(2))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("backup-scan-wrong")?;
    t.press(Action::Cancel);
    // Another paper form: test key 1 split into two Seed XOR parts.
    t.press(Action::BXor(2));
    t.shot("backup-xor")?;
    t.press(Action::BPaperHide);
    // The copy typed back.
    t.press(Action::BCheck);
    type_words(t, None)?;
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("backup-split")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("backup-envelope")?;
    // Every item done: what waits for the stick, and the way to one.
    t.press(Action::BNext(bstep::ENVELOPE));
    // The last item: one lock, then each stick by place.
    t.shot("backup-write-to-sticks")?;
    // The sheets went For the stick as PDFs with the checklist; written
    // beside the shots for a look. The blank template is printed from
    // Faraday Starter.
    t.press(Action::Backup(0));
    t.press(Action::BOut(3));
    let name = "savings-backup.pdf";
    let item = t
        .app
        .outbox
        .iter()
        .find(|it| it.name == name)
        .ok_or(format!("no {name}"))?;
    if !item.bytes.starts_with(b"%PDF-") {
        return Err(format!("{name} is not a PDF"));
    }
    std::fs::write(t.out.join(name), &item.bytes).map_err(|e| e.to_string())?;
    // A message, signed with the Spending wallet and checked.
    t.press(Action::SignMessage);
    let spending = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .ok_or("no Spending")?;
    t.press(Action::MWallet(spending));
    t.press(Action::MNext(0));
    for c in "I control this address.".chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.shot("message-text")?;
    t.press(Action::MNext(1));
    t.press(Action::MNext(2));
    t.press(Action::MSign);
    t.shot("message-signed")?;
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "spending-message.txt")
    {
        t.press(Action::VisitInAll);
        t.press(Action::VisitIn(i));
        t.press(Action::VisitCopy);
    }
    t.sticks(false);
    t.load_kit()?;
    let m = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "spending-message.txt")
        .ok_or("no message")?;
    t.press(Action::CheckMessage(m));
    t.shot("message-check")?;
    // Create a 2-of-3: test key 1 here, a new key, and a cosigner's key
    // from a file.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "xpub-3-cf2e083d-multisig.txt")
    {
        t.press(Action::VisitInAll);
        t.press(Action::VisitIn(i));
        t.press(Action::VisitCopy);
    }
    t.sticks(false);
    t.load_kit()?;
    t.press(Action::CreateWallet);
    // Create opens on Kind (`docs/CODEX32.md` §6.9), the backup in shares
    // the default; Multisig instead.
    t.press(Action::CKind(4));
    t.shot("create-kind")?;
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    let bacon = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, bacon));
    dice_key(t, 1)?;
    // The third cosigner's key is not here yet: the slot waits for it.
    t.press(Action::CSlotLater(2));
    t.press(Action::CKeyOut(0));
    let out = format!("xpub-{}.txt", fp_hex(bacon));
    if !t.app.outbox.iter().any(|i| i.name == out) {
        return Err(format!(
            "{out} did not reach the Outbox: {:?} {:?} keys {}",
            t.app.toast_text(),
            t.app.screen,
            t.app.session.keys.len()
        ));
    }
    t.shot("create-keys-later")?;
    t.press(Action::CNext(2));
    t.shot("create-waiting")?;
    if t.app.create_keys(c).is_ok() {
        return Err("a wallet with a key to come could be made".into());
    }
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("start-create-waiting")?;
    t.press(Action::CreateWallet);
    if t.app.create_waiting() != 1 {
        return Err("Create did not come back to the wallet being made".into());
    }
    // The key file arrives; it fills the waiting slot.
    let kf = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "xpub-3-cf2e083d-multisig.txt")
        .ok_or("no key file")?;
    t.press(Action::CSlotFile(2, kf));
    t.shot("create-keys")?;
    // Keys' Continue makes the wallet and opens Check.
    t.press(Action::CNext(2));
    t.shot("create-made")?;
    t.press(Action::OpenWallet(0));
    t.press(Action::QrWallet(0));
    t.shot("qr-wallet")?;
    // The code on screen is the descriptor, as a camera reads it.
    let read = t.decode(c);
    let want = t.app.session.wallets[0]
        .policy()
        .to_descriptor_checksummed();
    if read.first() != Some(&want) {
        return Err(format!("the wallet QR reads {read:?}, not the descriptor"));
    }
    println!("wallet QR decodes to the descriptor");
    t.press(Action::Cancel);
    // A stick arrives while a key is held.
    t.sticks(true);
    t.shot("lock-sheet")?;
    t.press(Action::NotNow);
    t.shot("not-now")?;
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    t.shot("after-lock-visit")?;
    // Vaults: the test stick's vault copied in, unlocked, an entry added,
    // and sealed into the Outbox when the session locks.
    let vi = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "vault.ofv")
        .ok_or("the kit has no vault.ofv")?;
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(vi));
    // The boot program comes in with its release's signed sums (F134).
    for name in [
        "vault-entries.txt",
        "BOOTX64.EFI",
        "SHA256SUMS",
        "SHA256SUMS.asc",
    ] {
        let k = t
            .stick()
            .files
            .iter()
            .position(|(n, _)| n == name)
            .ok_or(format!("the kit has no {name}"))?;
        t.press(Action::VisitIn(k));
    }
    t.press(Action::VisitCopy);
    let original = t
        .app
        .inbox()
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or("vault.ofv was not copied in")?;
    // With the stick still in, the passphrase waits for it to be pulled.
    t.press(Action::Vault(V::Open(0)));
    t.shot("unlock-stick")?;
    t.sticks(false);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults")?;
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("unlock")?;
    t.press(Action::Vault(V::Unlock));
    t.tick();
    // Unlocked from the Vaults list (`docs/history/NEW-WALLET.md` §11.1a): its
    // Load sheet (F62), every card ticked; Load ticked lands on Wallets.
    if t.app.sheet != Some(faraday_core::Sheet::UnlockLoad) || !t.app.session.keys.is_empty() {
        return Err("the unlock did not ask what to load".into());
    }
    t.shot("vault-load-sheet")?;
    t.press(Action::Vault(V::LoadTickAll));
    t.shot("vault-load-sheet-none")?;
    t.press(Action::Vault(V::LoadTickAll));
    t.press(Action::Vault(V::LoadTicked));
    if t.app.screen != Screen::Wallets || t.app.session.keys.is_empty() {
        return Err("the unlock did not load the vault's wallets".into());
    }
    t.shot("wallets-vault-open")?;
    t.press(Action::Nav(Screen::VaultContents));
    if t.app.vaults().open.len() != 1
        || t.app.session.keys.len() != 1
        || !t.app.session.wallets.iter().any(|w| w.name == "Spending")
    {
        return Err(format!(
            "the test vault did not unlock with its key and wallet: {:?}",
            t.app.vaults().unlock_error
        ));
    }
    t.shot("vault-keys")?;
    t.press(Action::Vault(V::Category(2)));
    t.shot("vault-entries")?;
    t.press(Action::Vault(V::Add));
    type_text(t, "Bank");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "me@example.com");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "hunter2");
    t.shot("vault-entry-form")?;
    t.press(Action::Vault(V::FormSave));
    t.shot("vault-entry-added")?;
    t.press(Action::Vault(V::Category(3)));
    t.shot("vault-notes")?;
    // Entries from a text file and from a TOTP setup code.
    t.press(Action::Vault(V::Category(2)));
    t.press(Action::Vault(V::ImportAllEntries));
    t.press(Action::Vault(V::ScanEntry));
    t.app.event(Event::Scanned {
        bytes: b"otpauth://totp/Mail:me%40example.com?secret=ABCDEFGH&issuer=Mail".to_vec(),
    });
    t.pump();
    t.shot("vault-entry-scanned")?;
    t.press(Action::Vault(V::FormSave));
    let entries = |t: &Tour| {
        t.app
            .vaults()
            .records(t.app.vaults().open[0].id)
            .of(6)
            .count()
    };
    if entries(t) != 5 {
        return Err(format!(
            "expected 5 entries after the imports, found {}",
            entries(t)
        ));
    }
    t.shot("vault-entries-imported")?;
    t.press(Action::Vault(V::Category(0)));
    // Test key 3 with a BIP-39 passphrase, saved with it.
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[2].0));
    t.press(Action::EntryPassphrase);
    type_text(t, "TREZOR");
    t.shot("entry-passphrase")?;
    t.press(Action::EntryAdd);
    let with = t
        .app
        .session
        .keys
        .all()
        .position(|k| k.has_passphrase)
        .ok_or("the key with a passphrase was not added")?;
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Add));
    t.shot("vault-save-key")?;
    t.press(Action::Vault(V::SaveKeyWithPassphrase(with)));
    let id = t.app.vaults().open[0].id;
    if !t.app.vaults().records(id).of(2).any(|(i, _)| {
        &*t.app.vaults().store().reveal(
            c,
            id,
            i,
            faraday_vault::records::field::KEY_PASSPHRASE,
            faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Passphrase),
        ) == "TREZOR"
    }) {
        return Err("the key was not saved with its passphrase".into());
    }
    t.shot("vault-keys-two")?;
    println!("entries imported and scanned, a key saved with its BIP-39 passphrase");
    // The vault's Spending wallet backed up with Paper and vault, its
    // copy checked once for each place: Backups, and the card's line.
    let spending = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .ok_or("Spending is not loaded")?;
    t.press(Action::Backup(spending));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    if t.app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        t.press(Action::BStep(bstep::COPY));
    }
    t.press(Action::BReveal);
    for _ in 0..2 {
        t.press(Action::BCheck);
        type_words(t, None)?;
    }
    t.press(Action::BackupsOf(spending));
    t.shot("backups")?;
    t.press(Action::OpenWallet(spending));
    t.shot("wallet-card-backup")?;
    t.press(Action::Nav(Screen::VaultContents));
    // A GPG key made in the vault, its certificate and revocation in the
    // Outbox, and a file signed with it.
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::Vault(V::Category(4)));
    flow_at_make(c, t, faraday_core::backup_flow::Kind::Gpg);
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    t.shot("vault-gpg-make")?;
    // On the stick, with the clock not checked, Make asks for the time
    // first (`docs/TOTP.md` §2.2); Keep this time, then Make again.
    t.press(Action::Flow(
        faraday_core::backup_flow::steps::FlowAction::Make,
    ));
    if t.app.screen != Screen::SetTime {
        return Err("making a GPG key did not ask for the time".into());
    }
    flow_made(t, faraday_core::backup_flow::Kind::Gpg)?;
    let gpg_files: Vec<String> = t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".asc"))
        .map(|i| i.name.clone())
        .collect();
    if gpg_files.len() != 1 {
        return Err(format!("making a GPG key left {gpg_files:?} in the Outbox"));
    }
    t.shot("vault-gpg-key")?;
    t.press(Action::Vault(V::GpgSignPick));
    // Two files ticked: the checkbox list, before Sign 2 files.
    let signable: Vec<String> = t
        .app
        .inbox()
        .iter()
        .filter(|i| !i.name.ends_with(".asc"))
        .map(|i| i.name.clone())
        .collect();
    for name in ["vault-entries.txt", "BOOTX64.EFI"] {
        let k = signable
            .iter()
            .position(|n| n == name)
            .ok_or(format!("no {name} to sign"))?;
        t.press(Action::Vault(V::GpgTick(k)));
    }
    t.shot("vault-gpg-sign")?;
    // Sign opens Sign with {key}: the files, the time, the hold (F63).
    t.press(Action::Vault(V::GpgSign));
    if t.app.sheet != Some(faraday_core::Sheet::GpgSign) {
        return Err("Sign did not open its sheet".into());
    }
    t.shot("vault-gpg-sign-sheet")?;
    t.press(Action::Vault(V::GpgHoldSign));
    for name in ["vault-entries.txt", "BOOTX64.EFI"] {
        if !t.app.outbox.iter().any(|i| i.name == format!("{name}.asc")) {
            return Err(format!(
                "signing left no signature for {name} in the Outbox"
            ));
        }
    }
    println!("a GPG key made in the vault signed two files at once");
    // TOTP codes for the entries the imported file left in the vault
    // (`docs/TOTP.md` §9): the clock is the GPG step's fixed one, so the
    // codes are the same in every render.
    t.press(Action::OpenCodes);
    if t.app.screen == Screen::SetTime {
        t.press(Action::SetTimeKeep);
    }
    if t.app.screen != Screen::Codes {
        return Err("TOTP codes did not open".into());
    }
    t.shot("totp-codes")?;
    let totp_item = t
        .app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .of(6)
        .position(|(_, r)| r.public_text(1) == Some("Example Git"))
        .ok_or("the imported TOTP entry is not in the vault")?;
    t.press(Action::CodesOpen(0, totp_item));
    t.shot("vault-entry-code")?;
    // Secure Boot keys made in the vault, the enrolment files, and the
    // test image signed after its hash is shown.
    t.press(Action::Vault(V::Category(5)));
    flow_at_make(c, t, faraday_core::backup_flow::Kind::SecureBoot);
    type_text(t, "Test");
    flow_made(t, faraday_core::backup_flow::Kind::SecureBoot)?;
    if t.app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .of(8)
        .next()
        .is_none()
    {
        return Err("no Secure Boot keys were made".into());
    }
    t.shot("vault-secure-boot")?;
    t.press(Action::Vault(V::SbEnrol));
    for name in ["PK.auth", "KEK.auth", "db.auth", "db.esl", "db.cer"] {
        if !t.app.outbox.iter().any(|i| i.name == name) {
            return Err(format!("{name} is not in the Outbox"));
        }
    }
    t.press(Action::Vault(V::SbImages));
    let efi = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "BOOTX64.EFI")
        .ok_or("the test image was not copied in")?;
    t.press(Action::Vault(V::SbPick(efi)));
    t.shot("vault-secure-boot-sign")?;
    t.press(Action::Vault(V::SbHoldSign));
    let signed = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "BOOTX64.EFI")
        .map(|i| i.bytes.clone())
        .ok_or("the signed image is not in the Outbox")?;
    let db_cert = t
        .app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .of(8)
        .find_map(|(_, r)| r.public(faraday_keys::vault::sb_field::DB_CERT))
        .ok_or("the keys do not read back")?;
    if faraday_sb::pe::check(&signed, db_cert) != Ok(faraday_sb::Check::Valid) {
        return Err("the signed image does not check against the db key".into());
    }
    println!("Secure Boot keys made, enrolment files written, an image signed");
    t.press(Action::LockAsk);
    t.shot("vault-lock-ask")?;
    // Lock seals the changed slot; the decoy slot is copied byte for byte.
    t.app.press(Action::Lock);
    let sealed = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or("locking put no sealed vault in the Outbox")?;
    // What the lock saved for the next process: the boxes and the kept
    // state.
    let mut saved = None;
    while let Some(c) = t.app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            saved = Some((inbox, outbox, kept));
        }
    }
    t.pump();
    let (_, main) = faraday_vault::open(c, &sealed, testkit::VAULT_PASSPHRASES[0].as_bytes())
        .map_err(|e| format!("{e:?}"))?;
    let (_, decoy) = faraday_vault::open(c, &sealed, testkit::VAULT_PASSPHRASES[1].as_bytes())
        .map_err(|e| format!("{e:?}"))?;
    let slot = 65_536 + 40;
    let changed = (0..4)
        .filter(|i| {
            let at = faraday_vault::HEADER_LEN + i * slot;
            original[at..at + slot] != sealed[at..at + slot]
        })
        .count();
    if !main.records.iter().any(|r| r.text(1) == Some("Bank"))
        || decoy.records.iter().filter(|r| r.kind == 6).count() != 1
        || changed != 1
    {
        return Err("the sealed vault lost its entry, or changed more than the open slot".into());
    }
    println!("vault unlocked, an entry added, sealed on lock with the other slots unchanged");
    // The next process, given what the lock saved: the locked vault's row
    // and Home's lead say what it held when last seen open. Then a fresh
    // process again, as the lock left it, for the rest of the tour.
    let (inbox, outbox, kept) = saved.ok_or("the lock saved nothing")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults-remembered")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-vault-remembered")?;
    // A stick for the vault sealed at lock: the visit goes to Unlock.
    t.sticks(true);
    t.shot("visit-then-unlock")?;
    t.press(Action::VisitWrite);
    t.sticks(false);
    if t.app.screen != Screen::Unlock {
        return Err("pulling the stick after writing the vault did not go to Unlock".into());
    }
    // After the visit: Spending is listed from what the vault was seen
    // to hold, its vault line on the stick written.
    t.press(Action::Nav(Screen::Backups));
    let at = t
        .app
        .backups_now(c)
        .iter()
        .position(|e| e.name == "Spending")
        .ok_or("Spending is not listed after the lock")?;
    t.press(Action::BackupsPage(at));
    t.shot("backups-after-visit")?;
    t.restart();
    // A new vault: two passphrases at the Light cost.
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    // It opens on Name and passphrases, the size closed on its default.
    t.shot("create-vault")?;
    t.press(Action::Vault(V::CStep(faraday_core::vault::vstep::PRESET)));
    t.shot("create-vault-size")?;
    // Customise: the three cards in the size's place.
    t.press(Action::Vault(V::CCustomise));
    t.shot("create-vault-customise")?;
    t.press(Action::Vault(V::CNext(faraday_core::vault::vstep::WHERE)));
    t.shot("create-vault-cost")?;
    // A cost pressed moves on to the space per passphrase (F103).
    t.press(Action::Vault(V::CPreset(0)));
    t.shot("create-vault-space")?;
    // A fresh Create, on the default size, makes the tour's vault.
    t.press(Action::Vault(V::Create));
    // The first passphrase from dice: six words, thirty rolls, run
    // together with no spaces.
    let rolls = "123456123456123456123456123456";
    t.press(Action::Vault(V::Dice(0)));
    type_text(t, rolls);
    t.shot("create-vault-dice")?;
    // The first word in its list, and back to the rolls as they were.
    let first =
        faraday_core::vault::dice_word_indices(rolls, faraday_bip::diceware::List::Large)[0];
    t.press(Action::WordList(
        faraday_core::wordlist::WordListAction::Open(1, Some(first)),
    ));
    t.shot("create-vault-dice-word")?;
    t.press(Action::WordList(
        faraday_core::wordlist::WordListAction::Close,
    ));
    t.press(Action::Vault(V::DiceUse));
    let dice_phrase = t
        .app
        .vaults()
        .create
        .as_ref()
        .map(|c| c.phrases[0].0.text.to_string())
        .unwrap_or_default();
    let expect =
        faraday_core::vault::dice_words(rolls, faraday_bip::diceware::List::Large).join("");
    if dice_phrase != expect {
        return Err(format!("dice gave {dice_phrase:?}, expected {expect:?}"));
    }
    t.press(Action::Vault(V::CAddPhrase));
    type_text(t, "second passphrase");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "second passphrase");
    t.shot("create-vault-phrases")?;
    t.press(Action::Vault(V::CGo));
    t.tick();
    let made = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or_else(|| {
            format!(
                "no vault was made: {:?}",
                t.app.vaults().create.as_ref().and_then(|c| c.error.clone())
            )
        })?;
    for p in [dice_phrase.as_str(), "second passphrase"] {
        faraday_vault::open(c, &made, p.as_bytes()).map_err(|e| format!("{p}: {e:?}"))?;
    }
    // Made: Unlock, the new vault picked, its passphrase asked once more.
    t.shot("create-vault-unlock")?;
    println!("a vault made here opens with each of its two passphrases");
    let before = t.app.vaults().open.len();
    let row = t
        .app
        .vault_files()
        .iter()
        .position(|f| Some(&f.name) == t.app.vaults().just_made.as_ref())
        .ok_or("the vault just made is not listed")?;
    t.press(Action::Vault(V::Open(row)));
    // Opening it now is the same Unlock screen as any other vault: its
    // passphrase typed again, not carried over from making it.
    type_text(t, &dice_phrase);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    if t.app.vaults().open.len() != before + 1 {
        return Err(format!(
            "the vault just made did not open on its own passphrase: {:?}",
            t.app.vaults().unlock_error
        ));
    }
    t.shot("vault-kept-open")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("files-outbox-vault-open")?;
    println!("the vault just made opens on its first passphrase typed again");
    seeds_tour(t)?;
    boot_tour(c, t)?;
    spend_tour(t)?;
    // Tools: every flow, and Find a tool.
    t.press(Action::Nav(Screen::Catalog));
    t.shot("tools")?;
    type_text(t, "bip-8");
    t.shot("tools-find")?;
    t.app.event(Event::Key(Key::Escape));
    // New key as SLIP-39 shares: the plan and what shares reveal.
    t.press(Action::KeyGenSlip39);
    t.shot("keygen-slip39")?;
    t.press(Action::Nav(Screen::Catalog));
    word_lists(t)?;
    rolled_words(t)?;
    keyed_entry(t)?;
    silent_wallet(t)?;
    t.press(Action::Nav(Screen::Catalog));
    // Create › MuSig2: every key signs.
    t.press(Action::Catalog(
        faraday_core::catalog::TILES
            .iter()
            .position(|x| x.name == "MuSig2")
            .unwrap_or(0) as u8,
    ));
    t.press(Action::CNext(faraday_core::cstep::KIND));
    t.press(Action::CNext(faraday_core::cstep::QUORUM));
    t.shot("create-musig")?;
    // Left alone with a key loaded until the warning before the idle
    // lock (`IDLE_WARN_MS` before `IDLE_LOCK_MIN`): the warning and its
    // countdown. Not over Make a new wallet, whose keys wait for their
    // backup and keep the auto-lock off: a fresh session, test key 1
    // typed.
    t.restart();
    t.app.event(Event::Tick { now_ms: t.now });
    t.press(Action::Entry(None));
    t.type_key(0);
    t.now += u64::from(faraday_core::IDLE_LOCK_MIN) * 60_000 - faraday_core::IDLE_WARN_MS + 20_000;
    t.app.event(Event::Tick { now_ms: t.now });
    t.shot("idle-warning")?;
    t.app.event(Event::Key(Key::Escape));
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings")?;
    // The bottom of Settings, where About says what the self-test found.
    for _ in 0..30 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("settings-about")?;
    t.press(Action::PowerAsk);
    t.shot("power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    // A stick that started with Secure Boot off reminds at power-off.
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.maker = "LENOVO".into();
    t.app.maker_family = "ThinkPad X1 Carbon".into();
    t.app.maker_model = "Example 14".into();
    t.press(Action::PowerAsk);
    t.shot("power-sheet-secure-boot-off")?;
    t.app.event(Event::Key(Key::Escape));
    t.app.secure_boot = faraday_core::SecureBootAtStart::Unknown;
    // New key's hint for cards, with its en dash.
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(faraday_core::keygen::Way::Cards.index()));
    t.press(Action::KNext);
    t.shot("keygen-cards")?;
    // Last, since nothing after it does anything: a self-test that failed.
    t.app.run_selftest_with(&[faraday_selftest::Check {
        name: "BIP-39 vector (entropy → words → seed)",
        run: |_| false,
    }]);
    t.shot("selftest-failed")?;
    Ok(())
}
