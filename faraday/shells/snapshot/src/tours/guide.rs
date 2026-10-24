//! The guide tour: the pictures in `docs/GUIDE.md` §2 to §7.

use crate::tour::{Tour, display, scroll_to, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::codex32::{C32, OskPress, SPLITS, Split, WAYS, Way as C32Way};
use faraday_core::create::NewKind;
use faraday_core::devices::DeviceCheck;
use faraday_core::keygen::{Start, Way, kstep};
use faraday_core::{Action, Faraday, Medium, Screen, Sheet, StickInfo, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key, TouchPhase};

/// EntropyLab's BitBox transcript for twelve words (`tools/reference/dice/
/// README.md`), the rolls the guide types: the last two are not taken.
const ROLLS: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";

/// Costs what the parts it runs cost. A fresh stick session as a person
/// meets it: the stick it started from still in, Secure Boot off at
/// start, a laptop named for no maker; then Home, the power sheet,
/// Wallets, and New key made from the dice transcript, with this
/// device's randomness shown on the way, the words and the quiz; then a
/// single-key wallet from that key, the Lock sheet while it waits for
/// its backup, and the wallet backed up (§4); a 2-of-3 over it and two
/// test keys (§5); an address checked, and that wallet's spend by QR,
/// with two warnings (§6); then a lock for a stick, the wallet restored
/// from the description on it (§7), and the PSBT on it signed after the
/// second key is scanned in (§6); a single key backed up in codex32
/// shares (§4.11) and Add a key's codex32 form (§5.1), each in a session of
/// its own; last, Faraday Desktop's Home and Demo's codex32 sample (§1.11).
pub(crate) fn guide_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Example", "Example 14", "10");
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.event(display(size, t.dpi));
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    // An IP-KVM attached at start: named before anything else, then
    // unplugged and checked again.
    t.app.usb_devices(&[super::devices::kvm()]);
    t.pump();
    t.shot("guide-devices-kvm")?;
    t.app.usb_devices(&[]);
    t.press(Action::DevicesCheck);
    for _ in 0..3 {
        t.tick();
    }
    t.shot("guide-home")?;
    t.press(Action::PowerAsk);
    t.shot("guide-power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("guide-wallets")?;
    t.go_on = false;
    t.press(Action::KeyGen(None));
    t.shot("guide-pull-sheet")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet == Some(Sheet::Devices) {
        t.shot("guide-devices-sheet")?;
        t.press(Action::DevicesUnderstood);
    }
    t.go_on = true;
    // Start opens closed on Make a new key with Length open (F103):
    // Change opens it, and Make a new key, pressed again, moves on.
    t.press(Action::KStep(kstep::START));
    t.shot("guide-newkey-start")?;
    t.press(Action::KStart(Start::Make.index()));
    t.shot("guide-newkey-length")?;
    // A length moves on to Randomness by itself.
    t.press(Action::KWords(12));
    t.shot("guide-newkey-randomness")?;
    new_key(t)?;
    single_key(t)?;
    board(t)?;
    checklist(c, t)?;
    multisig(t)?;
    multisig_board(t)?;
    scan_key(c, t)?;
    // §6 and §7: each part is tried, and every one that fails is named.
    let parts: [(&str, Part); 8] = [
        ("receive", receive),
        ("spend by QR", spend_by_qr),
        ("warnings", spend_warnings),
        ("stick and restore", stick_restore),
        ("spend from the stick", spend_from_stick),
        ("backup in shares", shares_backup),
        ("codex32 form", codex32_form),
        ("corrected key", corrected_key),
    ];
    let failed: Vec<String> = parts
        .into_iter()
        .filter_map(|(name, part)| part(c, t).err().map(|e| format!("{name}: {e}")))
        .collect();
    if !failed.is_empty() {
        return Err(failed.join("; "));
    }
    desktop_home(t)
}

/// §1.11: Faraday Desktop's Home as it opens, in Demo on testnet, with
/// no stick in; then Make codex32 shares in Demo, its entry with Use a
/// sample, and the sample in place of the strings once it is pressed.
fn desktop_home(t: &mut Tour) -> Result<(), String> {
    let size = t.size();
    t.app = Faraday::new();
    t.app.event(display(size, t.dpi));
    t.app.online = true;
    t.app.seed_line = true;
    t.app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    t.app.session.set_demo(&faraday_samples::SAMPLES);
    t.app.samples = Some(&faraday_samples::SAMPLES);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
    t.shot("guide-desktop-home")?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .ok_or("no 2 of 3")?;
    c32(t, C32::Split(two as u8));
    c32(t, C32::Next);
    c32_type(t, "name");
    c32(t, C32::Next);
    let dice = WAYS
        .iter()
        .position(|w| *w == C32Way::Dice)
        .ok_or("no dice")?;
    c32(t, C32::Way(dice as u8));
    c32(t, C32::Next);
    t.shot("guide-demo-codex32-enter")?;
    c32(t, C32::Sample);
    c32(t, C32::Next);
    t.shot("guide-demo-codex32-sample")
}

/// BIP 93's vector 2 (`NAME`, 2 of 3): shares A and C, and D made from
/// them (`tools/vectors/bip93/vectors.json`).
const NAME: [&str; 3] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
];

/// A press on Make codex32 shares.
fn c32(t: &mut Tour, p: C32) {
    t.press(Action::Codex32(p));
}

/// `text` typed on Make codex32 shares' keyboard.
fn c32_type(t: &mut Tour, text: &str) {
    for ch in text.chars() {
        c32(t, C32::Key(OskPress::Char(ch)));
    }
}

/// A fresh session with no stick in, on Wallets.
fn fresh_session(t: &mut Tour) {
    t.restart();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Nav(Screen::WalletsStart));
}

/// Costs what making a codex32 set and its wallet costs. §4.11: Make a
/// new wallet, Single key, backup in shares at 2 of 3:
/// Quorum, Keys, its slot opening Make codex32 shares on the identifier,
/// vector 2's characters typed and its three strings typed back, Strings
/// made, Lock in; Check; the board's standard way; the checklist's
/// codex32 item.
fn shares_backup(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::{bstep, plan};
    fresh_session(t);
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::Shares)?));
    t.shot("guide-shares-quorum")?;
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    t.shot("guide-shares-keys")?;
    t.press(Action::KeyGen(Some(0)));
    t.shot("guide-shares-identifier")?;
    c32_type(t, "name");
    c32(t, C32::Next);
    let typed = WAYS
        .iter()
        .position(|w| *w == C32Way::Typed)
        .ok_or("no typed way")?;
    c32(t, C32::Way(typed as u8));
    c32(t, C32::Next);
    for s in &NAME[..2] {
        c32_type(t, &s[9..35]);
    }
    // The worksheets, then each string typed back.
    c32(t, C32::Next);
    c32(t, C32::Next);
    for s in &NAME {
        c32_type(t, &s[3..]);
        c32(t, C32::Next);
    }
    t.shot("guide-shares-made")?;
    c32(t, C32::Lock);
    c32(t, C32::Next);
    if t.app.create().and_then(|c| c.open) != Some(cstep::CHECK) {
        t.press(Action::CStep(cstep::CHECK));
    }
    t.shot("guide-shares-check")?;
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.shot("guide-shares-board")?;
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    let item = bstep::of(plan::Item::Codex32(0));
    if open_item(t) != Some(item) {
        t.press(Action::BStep(item));
    }
    t.shot("guide-shares-item")?;
    shares_kept(t)
}

/// §4.11 on: the plan changed so string A is also in a vault on a stick
/// at Place 1 and string C a file and a Codex32QR picture on a stick at
/// Place 2; each string typed back; the vault made and A saved into it as
/// a share record, the vault item naming the set before it is made, the
/// vault's Bitcoin keys listing it; the sticks
/// written.
fn shares_kept(t: &mut Tour) -> Result<(), String> {
    use faraday_core::codex32_backup::Press;
    use faraday_core::vault::VaultAction as V;
    use faraday_core::{bstep, plan};
    let ix = |c: u8| plan::codex32::index_of(c).ok_or("no index");
    let (a, c) = (ix(b'a')?, ix(b'c')?);
    t.press(Action::BPlan);
    t.press(Action::Board(BP::Customize));
    t.press(Action::Board(BP::AddVault));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.press(Action::Board(BP::Zone(Zone::Card(0))));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::String(0, a)))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::String(0, c)))));
    t.press(Action::Board(BP::Zone(Zone::NewStick(1))));
    let file = Thing::String(0, c);
    t.press(Action::Board(BP::Chip(Chip::Placed(
        Zone::Files(1, 0),
        file,
    ))));
    t.press(Action::Board(BP::Form(Zone::Files(1, 0), file, 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot("guide-shares-board-kept")?;
    t.press(Action::BChecklist);
    if open_item(t) == Some(bstep::BLANK) {
        t.press(Action::BNext(bstep::BLANK));
    }
    let item = bstep::of(plan::Item::Codex32(0));
    if open_item(t) != Some(item) {
        t.press(Action::BStep(item));
    }
    for s in &NAME {
        let j = plan::codex32::index_of(s.as_bytes()[8]).ok_or("no index")?;
        t.press(Action::BC32(Press::Open(j as u8)));
        for ch in s[3..].chars() {
            t.press(Action::BC32(Press::Key(OskPress::Char(ch))));
        }
        t.press(Action::BC32(Press::Next));
    }
    let vault = bstep::of(plan::Item::Vault(0));
    if open_item(t) != Some(vault) {
        t.press(Action::BStep(vault));
    }
    // The vault item names the set whose string it keeps.
    t.shot("guide-shares-vault-item")?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    vault_made(t);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.shot("guide-shares-vault-contents")?;
    t.press(Action::Nav(Screen::Backup));
    let envelopes = bstep::of(plan::Item::Envelopes);
    t.press(Action::BStep(envelopes));
    t.press(Action::BNext(envelopes));
    let sticks = bstep::of(plan::Item::Sticks);
    if open_item(t) != Some(sticks) {
        t.press(Action::BStep(sticks));
    }
    t.shot("guide-shares-sticks")?;
    write_sticks(t, "guide-shares-sticks")
}

/// Costs what loading a key from codex32 strings costs. §5.1: New key's
/// Type codex32 strings, the strings typed with a correction: its Key
/// card, then locked in with the key's correction line beside its
/// fingerprint; then the same on Add a key, and Wallets with the line.
fn corrected_key(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    fresh_session(t);
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::Codex32.index()));
    corrected_entry(t);
    t.shot("guide-newkey-codex32-typed")?;
    t.press(Action::KLock);
    t.shot("guide-newkey-codex32-key")?;
    fresh_session(t);
    let at = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .ok_or("no codex32 form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
    corrected_entry(t);
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    t.press(Action::Nav(Screen::Wallets));
    t.shot("guide-codex32-wallets-mark")
}

/// Suggest corrections ticked, vector 2's A typed with box 20 wrong, I
/// mistyped and the box typed again from the paper, then C typed.
fn corrected_entry(t: &mut Tour) {
    use faraday_core::codex32_correct::Press;
    t.press(Action::Codex32Correct(Press::Tick));
    let good = NAME[0].as_bytes()[19];
    let mut bad = NAME[0].as_bytes().to_vec();
    bad[19] = if good == b'q' { b'p' } else { b'q' };
    string_in(t, &String::from_utf8_lossy(&bad));
    t.press(Action::Codex32Correct(Press::Mistyped));
    t.app.event(Event::Key(Key::Char(char::from(good))));
    t.pump();
    string_in(t, NAME[1]);
}

/// One string typed into Add a key's codex32 form, then Enter, the
/// keyboard put away.
fn string_in(t: &mut Tour, s: &str) {
    type_text(t, s);
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

/// Costs what checking codex32 strings costs. §5.1: Add a key's codex32
/// form: Check a string on vector 2's A with a
/// box changed, its final row and its worksheet; then A added, with Make
/// the rest and Add a share; Make the rest from A and C.
fn codex32_form(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::forms::{C32Press, Form};
    fresh_session(t);
    let at = Form::ALL
        .iter()
        .position(|f| *f == Form::Codex32)
        .ok_or("no codex32 form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
    t.press(Action::Codex32Form(C32Press::Check));
    let mut bad = NAME[0].as_bytes().to_vec();
    bad[19] = if bad[19] == b'q' { b'p' } else { b'q' };
    string_in(t, &String::from_utf8_lossy(&bad));
    t.shot("guide-codex32-check")?;
    t.press(Action::Codex32Form(C32Press::Sheet));
    t.shot("guide-codex32-worksheet")?;
    t.press(Action::Codex32Form(C32Press::Check));
    string_in(t, NAME[0]);
    t.shot("guide-codex32-form")?;
    t.press(Action::Codex32Form(C32Press::Rest));
    string_in(t, NAME[1]);
    t.shot("guide-codex32-rest")
}

/// One part of the guide's §6 and §7.
type Part = fn(&Compute, &mut Tour) -> Result<(), String>;

/// New key from Randomness on: this device's generator chosen and its
/// entry card and its Add randomness page, then the dice transcript,
/// the words and the quiz.
fn new_key(t: &mut Tour) -> Result<(), String> {
    // A way moves on to its entry card by itself (F103); Randomness,
    // opened again, shows the way chosen (F104).
    t.press(Action::KGroup(2));
    t.press(Action::KWay(Way::Device.index()));
    t.shot("guide-newkey-device-entry")?;
    // Its Add randomness: the page on Dice with a few rolls in, then
    // Back to the Generator card.
    {
        use faraday_core::fresh::OwnSource;
        use faraday_core::fresh::page::Act;
        t.press(Action::Own(Act::Open));
        t.press(Action::Own(Act::Source(OwnSource::Dice)));
        for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
            t.press(Action::Own(Act::Roll(f)));
        }
        t.shot("guide-newkey-add-randomness")?;
        t.press(Action::Back);
    }
    t.press(Action::KStep(kstep::SOURCE));
    scroll_to(t, Action::KWay(Way::Device.index()));
    t.shot("guide-newkey-randomness-device")?;
    t.press(Action::KWay(Way::DiceWords.index()));
    t.press(Action::KStep(kstep::SOURCE));
    t.shot("guide-newkey-randomness-dice")?;
    t.press(Action::KWay(Way::DiceWords.index()));
    t.shot("guide-newkey-rolls-empty")?;
    // A 5 in a word's first five places is not taken.
    t.press(Action::KRoll(5));
    t.shot("guide-newkey-rolls-five")?;
    for (i, c) in ROLLS.bytes().enumerate() {
        t.press(Action::KRoll(c - b'0'));
        if i == 23 {
            t.shot("guide-newkey-rolls-some")?;
        }
    }
    scroll_to(t, Action::KNext);
    t.shot("guide-newkey-rolls-done")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-key")?;
    t.press(Action::KLock);
    t.shot("guide-newkey-locked")?;
    t.shot("guide-newkey-words")?;
    t.press(Action::KShow);
    t.shot("guide-newkey-words-shown")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("guide-newkey-quiz")?;
    let wrong = t
        .app
        .keygen()
        .and_then(|k| k.quiz.as_ref())
        .map(|q| (q.correct_slot() as u8 + 1) % 4)
        .ok_or("no quiz")?;
    t.press(Action::KQuiz(wrong));
    t.shot("guide-newkey-quiz-wrong")?;
    t.press(Action::KQuizRetry);
    super::default::pass_quiz(t);
    t.shot("guide-newkey-quiz-passed")?;
    t.press(Action::KAdd);
    t.shot("guide-newkey-loaded")
}

/// The dice key's words, as §3 makes them.
const DICE_WORDS: &str =
    "brand hobby ranch shoulder brass hockey ranch short brand hockey random torch";

/// §4.1: Make a new wallet, a single key from the dice key: Kind, the
/// vault question, Keys, Check, and its Back up step.
fn single_key(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    // Create opens on Kind (`docs/CODEX32.md` §6.9); a kind pressed
    // moves on (F103).
    t.shot("guide-single-kind")?;
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.shot("guide-single-next")?;
    // No moves on to Keys by itself.
    t.press(Action::CVaults(false));
    t.shot("guide-single-keys")?;
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.shot("guide-single-keys-filled")?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("guide-single-check")?;
    // Lock while the wallet waits for its backup: the sheet asks.
    t.press(Action::LockAsk);
    t.shot("guide-lock-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::CHECK));
    t.shot("guide-single-backup-step")?;
    t.press(Action::CBackup);
    t.shot("guide-board-standard")
}

/// `kind`'s number, as `Action::CKind` carries it.
fn kind_at(kind: NewKind) -> Result<u8, String> {
    NewKind::ALL
        .iter()
        .position(|k| *k == kind)
        .map(|i| i as u8)
        .ok_or_else(|| format!("{kind:?} is not offered"))
}

/// A finger on the panel at `at`, as `phase`.
fn touch(t: &mut Tour, (x, y): (u16, u16), phase: TouchPhase) {
    t.app.event(Event::Touch { x, y, phase });
    t.pump();
}

/// `chip` dragged by a finger onto `zone`: a shot on the way, over the
/// zone, and once dropped.
fn drag(t: &mut Tour, chip: Chip, zone: Zone, name: &str) -> Result<(), String> {
    let _ = t.app.frame();
    let from = t
        .app
        .where_offered(Action::Board(BP::Chip(chip)))
        .ok_or("the chip is not on screen")?;
    let to = t
        .app
        .where_offered(Action::Board(BP::Zone(zone)))
        .ok_or("the zone is not on screen")?;
    touch(t, from, TouchPhase::Down);
    let step =
        |a: u16, b: u16, k: i32| (i32::from(a) + (i32::from(b) - i32::from(a)) * k / 4) as u16;
    for k in 1..=4 {
        touch(
            t,
            (step(from.0, to.0, k), step(from.1, to.1, k)),
            TouchPhase::Move,
        );
    }
    t.shot(&format!("{name}-over"))?;
    touch(t, to, TouchPhase::Up);
    t.shot(&format!("{name}-dropped"))
}

/// §4.2 to §4.5: the board after Customize backup; a new vault dragged
/// onto Place 1; the seed's ⋮ menu putting it in the vault;
/// the description put there too; the SeedQR at Place 1 turned on.
fn board(t: &mut Tour) -> Result<(), String> {
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.shot("guide-board-customize")?;
    t.press(Action::Board(BP::AddVault));
    t.shot("guide-board-vault-added")?;
    drag(
        t,
        Chip::List(Thing::Vault(0)),
        Zone::Card(0),
        "guide-board-drag",
    )?;
    t.press(Action::Board(BP::Menu(Chip::List(Thing::Seed(0)))));
    t.shot("guide-board-menu")?;
    t.press(Action::Board(BP::MenuTo(Zone::Vault(0, 0, 0))));
    t.shot("guide-board-seed-in-vault")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Whole))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    let seed = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    t.press(Action::Board(BP::Chip(seed)));
    t.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot("guide-board-done")?;
    scroll_to(t, Action::BChecklist);
    t.shot("guide-board-plan")
}

/// Costs a mnemonic's checksum. §4.6 to §4.9: the checklist; the blank template; the copy by hand
/// and Check my copy; the vault made and saved into; the sticks written.
fn checklist(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::bstep;
    use faraday_core::vault::VaultAction as V;
    t.press(Action::BChecklist);
    t.shot("guide-checklist")?;
    t.press(Action::BNext(bstep::BLANK));
    t.shot("guide-checklist-copy")?;
    t.press(Action::BReveal);
    t.shot("guide-checklist-copy-shown")?;
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    t.shot("guide-check-copy")?;
    crate::tour::type_words(t, Some(5))?;
    t.shot("guide-check-copy-some")?;
    for w in DICE_WORDS.split_whitespace().skip(5) {
        type_text(t, &format!("{w} "));
    }
    // The words checked are the item done here, with no camera yet, and
    // the checklist moves on (C8); the item, opened again, takes the scan.
    if open_item(t) != Some(bstep::COPY) {
        t.press(Action::BStep(bstep::COPY));
    }
    t.shot("guide-check-copy-words")?;
    // The SeedQR copy, scanned.
    t.press(Action::BScan);
    super::site_tour::camera(t);
    t.shot("guide-check-copy-scan")?;
    t.app.event(Event::Scanned {
        bytes: crate::tour::copy_digits(c, t, None)?,
    });
    t.pump();
    t.shot("guide-check-copy-done")?;
    // An item done moves on by itself (C8): to the vault's, here.
    if open_item(t) != Some(bstep::VAULT) {
        t.press(Action::BStep(bstep::VAULT));
    }
    t.shot("guide-checklist-vault")?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    t.shot("guide-vault-create")?;
    vault_made(t);
    t.shot("guide-checklist-vault-made")?;
    t.press(Action::BVaultSave(0));
    t.shot("guide-checklist-vault-saved")?;
    if open_item(t) == Some(bstep::VAULT) {
        t.press(Action::BNext(bstep::VAULT));
    }
    t.shot("guide-checklist-next")?;
    if open_item(t) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.shot("guide-checklist-envelope")?;
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot("guide-checklist-sticks")?;
    write_sticks(t, "guide-sticks")
}

/// Create a vault's passphrase typed twice, the vault made, and unlocked
/// with it, back on the checklist.
fn vault_made(t: &mut Tour) {
    use faraday_core::vault::VaultAction as V;
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "guide vault phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "guide vault phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
}

/// The checklist's item open now.
fn open_item(t: &Tour) -> Option<u8> {
    t.app.backup().and_then(|b| b.open)
}

/// The one lock, then each stick the plan names, as the flow asks; the
/// shots named from `name`.
fn write_sticks(t: &mut Tour, name: &str) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.pump();
    t.shot(&format!("{name}-after-lock"))?;
    for n in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        t.shot(&format!("{name}-insert-{n}"))?;
        let stick = StickInfo {
            serial: Some(0x2000 + slot as u32),
            id: format!("s{slot}"),
            label: "KEYSTICK".into(),
            boot: false,
            files: Vec::new(),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        t.shot(&format!("{name}-read-{n}"))?;
        for _ in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            t.app
                .press(Action::Flow(if hold { F::Hold } else { F::Tap }));
            t.pump();
        }
        t.shot(&format!("{name}-written-{n}"))?;
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    for _ in 0..5 {
        t.tick();
    }
    t.shot(&format!("{name}-done"))
}

/// §5.1 to §5.4: a fresh session; the dice key typed back from its
/// paper and test keys 2 and 3 typed; Make a new wallet, a 2-of-3:
/// Kind, Quorum, Keys, Check; the wallet card; its wallet QR.
fn multisig(t: &mut Tour) -> Result<(), String> {
    t.restart();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.pump();
    // Wallets' Add a key is New key on Start: Change, Type the words,
    // the words typed, Lock in and Done.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::Words.index()));
    t.shot("guide-type-words")?;
    type_text(t, DICE_WORDS);
    t.shot("guide-type-words-filled")?;
    t.press(Action::KNext);
    t.shot("guide-type-words-key")?;
    t.press(Action::KLock);
    if t.app.keygen().is_some() {
        t.press(Action::KNext);
    }
    t.shot("guide-type-words-added")?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    // Multisig moves on to Quorum; Kind, opened again, shows it ticked,
    // and Multisig, pressed again, moves on (F103, F104).
    let multi = kind_at(NewKind::Multi)?;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CStep(cstep::KIND));
    t.shot("guide-multi-kind")?;
    t.press(Action::CKind(multi));
    t.shot("guide-multi-quorum")?;
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    t.shot("guide-multi-keys")?;
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.shot("guide-multi-keys-filled")?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("guide-multi-check")?;
    scroll_to(t, Action::CNext(cstep::CHECK));
    t.shot("guide-multi-check-addresses")?;
    let w = t
        .app
        .create()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot("guide-multi-card")?;
    t.press(Action::QrWallet(w));
    t.shot("guide-multi-wallet-qr")?;
    t.press(Action::Cancel);
    Ok(())
}

/// §5.5: the 2-of-3's board, the standard way; a share at Place 3 that
/// lets it see the balance; a second seed at Place 1 that lets it spend.
fn multisig_board(t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    t.shot("guide-multi-board")?;
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Whole)));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Share(0)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(2))));
    t.pump();
    t.shot("guide-multi-sees-sheet")?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Seed(1)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    t.pump();
    t.shot("guide-multi-spend-sheet")?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.pump();
    Ok(())
}

/// Costs a mnemonic's checksum. The SeedQR digits of `words`, as a
/// camera reads them off the code.
fn seedqr(c: &Compute, words: &str) -> Result<Vec<u8>, String> {
    let m = faraday_bip::bip39::Mnemonic::parse(c, faraday_bip::bip39::Language::English, words)
        .map_err(|e| format!("{e:?}"))?;
    Ok(faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec())
}

/// Costs two mnemonics' checksums. §5.6: Remove every key; the 2-of-3's
/// card short of keys; Scan a key; test key 2's SeedQR, then the dice
/// key's.
fn scan_key(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("guide-settings-session")?;
    t.press(Action::RemoveKeys);
    t.shot("guide-keys-removed")?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot("guide-multi-card-short")?;
    t.press(Action::ScanKey(w));
    super::site_tour::camera(t);
    t.shot("guide-scan-key-camera")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("guide-scan-key-one", 0)?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, DICE_WORDS)?,
    });
    t.pump();
    t.shot_after("guide-scan-key-two", 0)?;
    t.shot("guide-multi-card-can-spend")
}

/// The wallet the guide made last, by its place in the session.
fn last_wallet(t: &Tour) -> Result<usize, String> {
    t.app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or_else(|| "no wallet".to_string())
}

/// The wallet at `w` as a test kit's wallet, its spend paying `fee`.
fn kit_of(t: &Tour, w: usize, fee: u64) -> faraday_core::testkit::Kit {
    faraday_core::testkit::Kit {
        id: "guide",
        name: "New wallet 1",
        descriptor: t.app.session.wallets[w]
            .policy()
            .to_descriptor_checksummed(),
        cosigner: Some(1),
        fee,
    }
}

/// `bytes`, a PSBT, held up to the camera now open as animated UR
/// parts until the camera closes; a shot named `mid` at the second part.
fn show_psbt(t: &mut Tour, bytes: &[u8], mid: Option<&str>) -> Result<(), String> {
    super::site_tour::camera(t);
    let mut enc = faraday_codec::ur::Encoder::psbt(bytes, 120).map_err(|e| format!("{e:?}"))?;
    let mut parts = 0;
    while t.app.scan().is_some() && parts < 80 {
        t.app.event(Event::Scanned {
            bytes: enc.next_part().into_bytes(),
        });
        parts += 1;
        if parts == 2
            && let Some(name) = mid
        {
            t.app.storage(StorageEvent::QrSeen {
                width: 640,
                height: 480,
                corners: [(220, 120), (430, 130), (420, 340), (210, 330)],
                module_tenths: 60,
                read: true,
            });
            t.shot(name)?;
        }
    }
    t.pump();
    if t.app.scan().is_some() {
        return Err(format!("the camera still waits after {parts} parts"));
    }
    Ok(())
}

/// Costs an address derivation. §6.1: Scan; the wallet's receive
/// address 0 as Sparrow shows it, found; then an address of no loaded
/// wallet (BIP 84's first test address).
fn receive(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let net = t.app.session.network();
    let ours = t.app.session.wallets[w].address_on(c, net, false, 0);
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Scan);
    super::site_tour::camera(t);
    t.shot("guide-receive-scan")?;
    t.app.event(Event::Scanned {
        bytes: ours.into_bytes(),
    });
    t.pump();
    t.shot_after("guide-receive-found", 0)?;
    t.app.event(Event::Scanned {
        bytes: b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".to_vec(),
    });
    t.pump();
    t.shot_after("guide-receive-not-ours", 0)?;
    t.press(Action::Cancel);
    Ok(())
}

/// Costs what spending costs. §6.2 to §6.7: the card's Spend from this
/// wallet; Load the wallet in
/// Sparrow; Write the payment in Sparrow; the PSBT scanned as animated
/// codes; the review and its table; Who signs with both keys here; Sign
/// it; Send it, with the signed PSBT and the finished transaction as
/// codes.
fn spend_by_qr(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    use faraday_core::wallet::step;
    let w = last_wallet(t)?;
    let (psbt, _) = faraday_core::testkit::spend_of(&kit_of(t, w, 2_140))?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot("guide-spend-card")?;
    t.press(Action::Family(F::SpendFrom(w)));
    t.shot("guide-spend-sparrow")?;
    t.press(Action::Family(F::Next(page::CHECK)));
    t.shot("guide-spend-write")?;
    t.press(Action::Scan);
    show_psbt(t, &psbt.to_bytes(), Some("guide-spend-scanning"))?;
    t.shot("guide-spend-transaction")?;
    t.press(Action::ToggleTable);
    t.shot("guide-spend-table")?;
    t.press(Action::ToggleTable);
    t.press(Action::StepNext(step::TRANSACTION));
    if t.app.spend().and_then(|s| s.open) == Some(step::CHECK) {
        t.shot("guide-spend-check")?;
        t.press(Action::StepNext(step::CHECK));
    }
    t.shot("guide-spend-signers")?;
    t.press(Action::StepNext(step::SIGNERS));
    t.shot("guide-spend-sign")?;
    t.press(Action::SignHere);
    t.shot("guide-spend-signed")?;
    if t.app.spend().and_then(|s| s.open) == Some(step::SIGN) {
        t.press(Action::StepNext(step::SIGN));
    }
    if t.app.spend().and_then(|s| s.open) == Some(step::COLLECT) {
        t.shot("guide-spend-collect")?;
        t.press(Action::StepNext(step::COLLECT));
    }
    t.shot("guide-spend-send")?;
    t.press(Action::QrSigned);
    t.shot("guide-spend-qr-signed")?;
    t.press(Action::Cancel);
    t.press(Action::Family(F::QrTx));
    t.shot("guide-spend-qr-tx")?;
    t.press(Action::Cancel);
    t.press(Action::StepNext(step::FINISH));
    t.shot("guide-spend-away")
}

/// Costs what reading a PSBT costs. §6.8: a PSBT of the guide's wallet
/// whose fee is 0.012 BTC, and the
/// test kit's Savings PSBT, a wallet not loaded here: each scanned from
/// Home, which files it while a spend is under way, then started from
/// Files.
fn spend_warnings(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let fee = faraday_core::testkit::unsigned(&kit_of(t, w, 1_200_000))?.to_bytes();
    let other = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    for (bytes, name) in [(fee, "guide-warn-fee"), (other, "guide-warn-other-wallet")] {
        t.press(Action::Nav(Screen::Home));
        t.press(Action::Scan);
        show_psbt(t, &bytes, None)?;
        let at = t
            .app
            .inbox()
            .iter()
            .rposition(|i| i.bytes == bytes)
            .ok_or("the scanned PSBT is not in Files")?;
        t.press(Action::StartSpend(at));
        t.shot(name)?;
    }
    Ok(())
}

/// Where the guide's stick keeps its files.
fn stick_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("faraday-guide-stick")
}

/// The stick Sparrow wrote the PSBT to, beside the wallet's public
/// description from its backup.
fn guide_stick(files: &[(String, Vec<u8>)]) -> Result<StickInfo, String> {
    let dir = stick_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(StickInfo {
        id: dir.display().to_string(),
        label: "TRANSFER".into(),
        boot: false,
        serial: Some(0x3000),
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    })
}

/// Costs what restoring costs. §6.3 on a stick and §7.1: the stick
/// plugged in with keys loaded, and
/// Lock and use stick; the visit; then Restore an existing wallet from
/// the description that came in, key 1 typed, a seed of no key of the
/// wallet refused, keys 2 and 3 Not here, Check and Done.
fn stick_restore(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let kit = kit_of(t, w, 1_850);
    let psbt = faraday_core::testkit::unsigned(&kit)?;
    let mut files: Vec<(String, Vec<u8>)> =
        faraday_core::testkit::public_files(kit.name, &kit.descriptor)?
            .into_iter()
            .filter(|(n, _)| n.ends_with("-descriptor.txt"))
            .collect();
    files.push(("payment.psbt".into(), psbt.to_bytes()));
    let stick = guide_stick(&files)?;
    // The PSBTs scanned so far, deleted from Files: the stick's is the
    // one the card offers.
    faraday_core::testkit::inbox_retain(&mut t.app, |i| !i.name.starts_with("scanned-"));
    t.app.storage(StorageEvent::Sticks(vec![stick.clone()]));
    t.pump();
    t.shot("guide-stick-attached")?;
    t.press(Action::Lock);
    if let Some((inbox, outbox, kept)) = t.boxes.clone() {
        t.app.storage(StorageEvent::Restored {
            inbox,
            outbox,
            kept,
        });
    }
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.pump();
    t.shot("guide-stick-visit")?;
    t.press(Action::VisitCopy);
    t.shot("guide-stick-copied")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    restore_wallet(t)
}

/// §7.1: Restore an existing wallet, from Files.
fn restore_wallet(t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::RestoreWallet);
    t.shot("guide-restore-kind")?;
    t.press(Action::RFromFiles);
    t.shot("guide-restore-described")?;
    // No moves on to Keys by itself (F103).
    t.press(Action::RVaults(false));
    t.shot("guide-restore-vaults")?;
    if t.app.restore().and_then(|r| r.open) != Some(rstep::SEEDS) {
        return Err("No did not move on to Keys".into());
    }
    t.shot("guide-restore-keys")?;
    t.press(Action::RSlotWords);
    t.shot("guide-restore-type")?;
    type_text(t, DICE_WORDS);
    t.press(Action::EntryAdd);
    t.shot("guide-restore-key-1")?;
    t.press(Action::RSlotWords);
    type_text(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.press(Action::EntryAdd);
    t.shot("guide-restore-refused")?;
    let w = t
        .app
        .restore()
        .and_then(|r| r.wallet)
        .ok_or("the description made no wallet")?;
    let empty: Vec<u8> = t
        .app
        .session
        .slots(&t.app.session.wallets[w])
        .iter()
        .enumerate()
        .filter(|(_, s)| s.held_by.is_none())
        .map(|(k, _)| k as u8)
        .collect();
    for k in empty {
        t.press(Action::RAbsent(k));
    }
    t.shot("guide-restore-not-here")?;
    t.press(Action::Seeds(S::Make));
    t.shot("guide-restore-check")?;
    scroll_to(t, Action::RNext(rstep::CHECK));
    t.shot("guide-restore-check-addresses")?;
    t.press(Action::RNext(rstep::CHECK));
    t.shot("guide-restore-done")
}

/// Costs a mnemonic's checksum. §6.3 to §6.7 from the stick, with one
/// key here: the card's Sign the PSBT from the stick; the review; Who
/// signs, short a key; its Scan a key, test key 2's SeedQR; Sign it;
/// Send it, and writing to a stick.
fn spend_from_stick(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::FamilyAction as F;
    use faraday_core::wallet::step;
    let w = last_wallet(t)?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot("guide-stick-card")?;
    t.press(Action::Family(F::SpendFrom(w)));
    t.shot("guide-stick-transaction")?;
    t.press(Action::StepNext(step::TRANSACTION));
    if t.app.spend().and_then(|s| s.open) == Some(step::CHECK) {
        t.shot("guide-stick-check")?;
        t.press(Action::StepNext(step::CHECK));
    }
    t.shot("guide-stick-signers")?;
    t.press(Action::ScanKey(w));
    super::site_tour::camera(t);
    t.shot("guide-stick-scan-key")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("guide-stick-signers-two", 0)?;
    t.press(Action::StepNext(step::SIGNERS));
    t.press(Action::SignHere);
    t.shot("guide-stick-signed")?;
    for k in [step::SIGN, step::COLLECT] {
        if t.app.spend().and_then(|s| s.open) == Some(k) {
            t.press(Action::StepNext(k));
        }
    }
    t.shot("guide-stick-send")?;
    t.press(Action::Primary);
    t.shot("guide-stick-write-out")?;
    // The stick's files, gone with the tour.
    let _ = std::fs::remove_dir_all(stick_dir());
    Ok(())
}
