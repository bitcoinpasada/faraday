//! Writing codex32 strings (`docs/PLANNING.md` §16.109 rules 4 and 5):
//! Add a key › Create Codex32 shares, and Backup › Codex32 on a key
//! this device already holds.
//!
//! Every string here is read off the Secret screen as a person reads
//! it, typed back into the entry that follows it, and then typed again
//! through Load a key › Codex32 in a fresh session. Nothing trusts one
//! computation: the strings this device writes are also handed to the
//! codec's own `Codex32::parse` and `recover`.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE};
use opensigner_core::codex32::Step as PlanStep;
use opensigner_core::create::Step;
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, ids};
use osk_bip::codex32::{Codex32, identifier_for};
use osk_bip::keys::{MasterKey, Network};
use osk_entropy::{DiceRolls, Strength};
use osk_shell_api::Key;
use osk_ui::widgets::keyboard::KeyInput;

/// Rolls that show no sanity flag, repeated as far as a strength needs.
const ROLLS: &str = "32461151351521144121541512665155412152342515356215";

/// Which source row "This device" is, in `SOURCE_ROWS` order.
const DEVICE: usize = 6;
/// Which source row dice is.
const DICE: usize = 0;

/// The lengths BIP 93 allows a string to be.
const LENGTHS: [usize; 6] = [48, 54, 61, 67, 74, 127];

/// Add a key → "Create Codex32 shares" → `source` → the seed length at
/// `row` → the entropy → the plan.
fn start(h: &mut Harness, source: usize, row: usize) {
    h.open_add(ids::ADD_CREATE_CODEX32);
    assert_eq!(h.app.screen(), ScreenKind::Create);
    assert_eq!(h.app.create_step(), Some(Step::Source));
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, source),
        ids::CREATE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(Step::Count));
    // The five lengths an entropy source makes; 512 bits is not offered.
    let texts = h.app.texts();
    for bits in ["128 bits", "160 bits", "192 bits", "224 bits", "256 bits"] {
        assert!(
            texts.iter().any(|t| t == bits),
            "{bits} is not offered: {texts:?}"
        );
    }
    assert!(
        !texts.iter().any(|t| t == "512 bits"),
        "512 bits is offered: {texts:?}"
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, row),
        ids::CREATE_COUNT_CONTINUE,
    );
    gather(h, source, Strength::ALL[row]);
}

/// One run of the chosen source's own screens, at `strength`.
fn gather(h: &mut Harness, source: usize, strength: Strength) {
    if source == DEVICE {
        assert_eq!(h.app.create_step(), Some(Step::Device));
        h.tap(ids::CREATE_CONTINUE);
        return;
    }
    // The dice source asks which published procedure reads the rolls;
    // direct selection is dimmed here, because a codex32 key is a master
    // seed and is never spelled in BIP-39 words.
    if h.app.create_step() == Some(Step::Procedure) {
        h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    }
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    let rolls: String = ROLLS
        .chars()
        .cycle()
        .take(DiceRolls::needed(strength))
        .collect();
    h.type_text(&rolls);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);
}

/// The string on the Secret screen, read with the panel held open.
fn read_string(h: &mut Harness) -> String {
    let point = h.press(ids::CREATE_REVEAL);
    let found = h
        .app
        .texts()
        .into_iter()
        .find(|t| t.starts_with("ms1") && LENGTHS.contains(&t.len()));
    h.release(point);
    found.unwrap_or_else(|| panic!("no codex32 string on the Secret screen"))
}

/// Reads the string on screen and goes on: typing it back where
/// `type_back`, the caution and the skip where not.
fn take_string(h: &mut Harness, type_back: bool) -> String {
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Shown));
    let string = read_string(h);
    if type_back {
        h.tap(ids::CREATE_CONTINUE);
        assert_eq!(h.app.codex32_step(), Some(PlanStep::TypeBack));
        h.type_codex32_back(&string);
    } else {
        h.tap(ids::QUIZ_SKIP);
        assert_eq!(h.app.codex32_step(), Some(PlanStep::TypeSkip));
        h.tap(ids::QUIZ_SKIP_CONFIRM);
    }
    string
}

/// Types a string into the type-back entry without pressing ✓.
fn type_only(h: &mut Harness, string: &str) {
    let lower = string.to_ascii_lowercase();
    for c in lower.strip_prefix("ms1").expect("a codex32 string").chars() {
        h.pad(ids::CODEX32_KEYBOARD, KeyInput::Char(c));
    }
}

/// The plan Choices: one string, or `shares` shares of which
/// `threshold` must be present.
fn plan(h: &mut Harness, shares: usize, threshold: usize) {
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Split));
    if shares == 0 {
        h.choose(ids::CODEX32_SPLIT_NO, ids::CODEX32_SPLIT_CONTINUE);
        return;
    }
    h.choose(ids::CODEX32_SPLIT_YES, ids::CODEX32_SPLIT_CONTINUE);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Count));
    h.choose(
        ids::at(ids::CODEX32_COUNT_BASE, shares - 1),
        ids::CODEX32_COUNT_CONTINUE,
    );
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Threshold));
    h.choose(
        ids::at(ids::CODEX32_THRESHOLD_BASE, threshold - 1),
        ids::CODEX32_THRESHOLD_CONTINUE,
    );
}

/// The fingerprint of the one key loaded, as hex.
fn only_fingerprint(h: &Harness) -> String {
    let keys = h.app.fingerprints();
    assert_eq!(keys.len(), 1, "one key");
    keys[0].to_hex().iter().map(|b| *b as char).collect()
}

fn hex(fp: osk_bip::keys::Fingerprint) -> String {
    fp.to_hex().iter().map(|b| *b as char).collect()
}

/// Types `strings` through Load a key › Codex32 in a fresh session and
/// returns the fingerprint of the key they give, or `None` where the
/// set is still short.
fn load_strings(strings: &[&str]) -> Option<String> {
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    for (i, string) in strings.iter().enumerate() {
        h.type_codex32(string);
        if h.app.load_step() == Some(opensigner_core::load::Step::Confirm) {
            assert_eq!(i + 1, strings.len(), "a string past the secret");
            break;
        }
        assert!(
            h.app
                .texts()
                .iter()
                .any(|t| t == EN.load_share_accepted_title),
            "a string this device wrote was refused: {:?}",
            h.app.texts()
        );
        h.tap(ids::LOAD_CONTINUE);
    }
    if h.app.load_step() != Some(opensigner_core::load::Step::Confirm) {
        return None;
    }
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    Some(only_fingerprint(&h))
}

/// The master fingerprint of the seed a set of strings recovers.
fn fingerprint_of(secret: &Codex32) -> String {
    let seed = secret.to_seed().expect("the secret carries a seed");
    let bytes = osk_crypto::SeedBytes::new(seed.expose().bytes()).expect("a seed");
    hex(MasterKey::from_seed_bytes(&osk_crypto::Secret::new(bytes), Network::Mainnet).fingerprint())
}

/// Create Codex32 shares from dice at 128 bits, not split: one string
/// is shown, typed back, and it opens the same key in a fresh session.
#[test]
fn one_string_is_the_key_and_opens_it_again() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DICE, 0);
    plan(&mut h, 0, 0);
    // A seed that is not split is fed no randomness, so nothing is
    // gathered and the string is there at once.
    assert!(h.app.texts().iter().any(|t| t == EN.codex32_string_title));
    let string = take_string(&mut h, true);
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);
    assert_eq!(
        h.app.backup_verified(0),
        Some(true),
        "every string typed back"
    );

    assert_eq!(
        load_strings(&[&string]),
        Some(made.clone()),
        "the string does not open the key this device made"
    );

    let parsed = Codex32::parse(&string).expect("a string this device wrote");
    assert_eq!(parsed.threshold(), 0, "a string that was never split");
    assert_eq!(parsed.share_index(), b's', "the secret's own index");
    let print: Vec<u8> = (0..4)
        .map(|i| u8::from_str_radix(&made[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    assert_eq!(
        parsed.identifier(),
        identifier_for([print[0], print[1], print[2], print[3]]),
        "the identifier is not the one the fingerprint gives"
    );
}

/// 256 bits split three of five: five shares, any three of which open
/// the key, and two of which do not.
#[test]
fn three_of_five_shares_open_the_key_and_two_keep_asking() {
    let mut h = Harness::new(PHONE);
    start(&mut h, DEVICE, 4);
    plan(&mut h, 5, 3);
    // Two random shares are needed for a threshold of three, and Create
    // gathers them from the source the key was made from.
    for _ in 0..2 {
        assert_eq!(h.app.codex32_step(), Some(PlanStep::Gather));
        gather(&mut h, DEVICE, Strength::Bits256);
    }
    let mut shares = Vec::new();
    for i in 0..5 {
        let title = format!("Share {} of 5", i + 1);
        assert!(
            h.app.texts().contains(&title),
            "share {} is not titled {title:?}: {:?}",
            i + 1,
            h.app.texts()
        );
        shares.push(take_string(&mut h, true));
    }
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);

    let three = [shares[0].as_str(), shares[2].as_str(), shares[4].as_str()];
    assert_eq!(load_strings(&three), Some(made.clone()));
    let two = [shares[0].as_str(), shares[2].as_str()];
    assert_eq!(
        load_strings(&two),
        None,
        "two shares of a 3-of-5 set opened the key"
    );

    // The codec reads back what this device wrote, and every share's
    // header states the plan it was made under.
    let parsed: Vec<Codex32> = shares
        .iter()
        .map(|s| Codex32::parse(s).expect("a string this device wrote"))
        .collect();
    let identifier = parsed[0].identifier();
    for share in &parsed {
        assert_eq!(share.threshold(), 3, "every share states the threshold");
        assert_eq!(share.identifier(), identifier, "one set, one identifier");
        assert_ne!(share.share_index(), b's', "a share is not the secret");
    }
    let set: Vec<Codex32> = shares[..3]
        .iter()
        .map(|s| Codex32::parse(s).unwrap())
        .collect();
    let secret = Codex32::recover(&set).expect("the codec reads its own shares");
    assert_eq!(fingerprint_of(&secret), made, "another key came back");
    let seed = secret.to_seed().unwrap();
    let again = Codex32::from_seed(3, identifier, seed.expose().bytes()).unwrap();
    assert_eq!(
        secret.encode().as_str(),
        again.encode().as_str(),
        "the recovered secret is not the one the seed encodes"
    );

    // A second backup of the same key is a second set of shares under
    // the same identifier, since the identifier is the seed's.
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_CODEX32);
    plan(&mut h, 3, 2);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Source));
    h.choose(
        ids::at(ids::CODEX32_SOURCE_BASE, DEVICE),
        ids::CODEX32_SOURCE_CONTINUE,
    );
    gather(&mut h, DEVICE, Strength::Bits256);
    let second: Vec<String> = (0..3).map(|_| take_string(&mut h, true)).collect();
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Result));
    h.tap(ids::QUIZ_DONE);
    let other = Codex32::parse(&second[0]).unwrap();
    assert_eq!(other.identifier(), identifier, "one seed, two identifiers");
    assert!(
        second.iter().all(|s| !shares.contains(s)),
        "two splits of one seed wrote the same share"
    );
    let pair = [second[0].as_str(), second[2].as_str()];
    assert_eq!(load_strings(&pair), Some(made));
}

/// "Type it back": a character wrong leaves ✓ dead with the codec's own
/// reason, another string of the same set is refused as not the one
/// shown, and skipping a string leaves the key unverified.
#[test]
fn a_string_typed_back_wrong_is_refused_and_a_skip_leaves_the_key_unverified() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DICE, 0);
    plan(&mut h, 2, 2);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Gather));
    gather(&mut h, DICE, Strength::Bits128);

    let first = read_string(&mut h);
    h.tap(ids::CREATE_CONTINUE);
    // One character wrong is a string whose checksum fails, so ✓ never
    // answers and the line under the field says why.
    let mut wrong = first.clone();
    let last = wrong.pop().expect("a string");
    wrong.push(if last == 'q' { 'p' } else { 'q' });
    type_only(&mut h, &wrong);
    assert!(
        h.app.texts().iter().any(|t| t == "checksum does not match"),
        "{:?}",
        h.app.texts()
    );
    assert!(
        h.app
            .key_rect(ids::CODEX32_KEYBOARD, KeyInput::Done)
            .is_none(),
        "a string with a character wrong can be taken"
    );
    h.pad(ids::CODEX32_KEYBOARD, KeyInput::Backspace);
    h.pad(ids::CODEX32_KEYBOARD, KeyInput::Char(last));
    h.pad(ids::CODEX32_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Shown));

    // The second string: the first one is a string this set has, and it
    // is still not the one on screen.
    let second = read_string(&mut h);
    assert_ne!(second, first);
    h.tap(ids::CREATE_CONTINUE);
    h.type_codex32_back(&first);
    assert_eq!(
        h.app.codex32_step(),
        Some(PlanStep::TypeBack),
        "another string of the set went on"
    );
    assert!(
        h.app.texts().iter().any(|t| t == EN.codex32_not_same),
        "the line under the field does not say so: {:?}",
        h.app.texts()
    );
    // The field keeps what was typed.
    assert!(
        h.app.texts().contains(&first),
        "the field was emptied: {:?}",
        h.app.texts()
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Shown));

    // Skipping that string leaves the key unverified.
    take_string(&mut h, false);
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    assert_eq!(
        h.app.backup_verified(0),
        Some(false),
        "a string was skipped"
    );
}

/// Backup › Codex32 on a key made of words: the string is the 512-bit
/// seed those words come to, 127 characters of it, and the Result says
/// what the backup holds.
#[test]
fn a_key_made_of_words_is_backed_up_as_the_seed_it_comes_to() {
    for passphrase in [None, Some("TREZOR")] {
        let mut h = Harness::new(PANEL);
        h.start_load(&ABANDON);
        h.finish_load(passphrase);
        let made = only_fingerprint(&h);

        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
        assert!(
            h.app.rect_of(ids::BACKUP_CODEX32).is_some(),
            "a key with words has no live Codex32 row"
        );
        h.tap(ids::BACKUP_CODEX32);
        plan(&mut h, 0, 0);
        let string = take_string(&mut h, true);
        assert_eq!(string.len(), 127, "a 512-bit seed is 127 characters");
        assert_eq!(h.app.codex32_step(), Some(PlanStep::Result));
        let result = h.app.texts();
        for line in [
            EN.codex32_result_title,
            EN.codex32_split_none,
            EN.codex32_holds_seed,
        ] {
            assert!(
                result.iter().any(|t| t == line),
                "the result does not state {line:?}: {result:?}"
            );
        }
        h.tap(ids::QUIZ_DONE);
        assert_eq!(h.app.screen(), ScreenKind::BackupMenu);

        assert_eq!(
            load_strings(&[&string]),
            Some(made),
            "the string does not open the key it was written from"
        );
    }
}

/// Backup › Codex32 on the two keys that have no words: a codex32 key
/// and a SLIP-39 key. Neither Result says anything about words.
#[test]
fn a_key_with_no_words_is_backed_up_as_its_own_seed() {
    const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(SECRET);
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);

    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_CODEX32);
    plan(&mut h, 3, 2);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Source));
    h.choose(
        ids::at(ids::CODEX32_SOURCE_BASE, DICE),
        ids::CODEX32_SOURCE_CONTINUE,
    );
    gather(&mut h, DICE, Strength::Bits128);
    let shares: Vec<String> = (0..3).map(|_| take_string(&mut h, true)).collect();
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Result));
    let result = h.app.texts();
    assert!(
        !result.iter().any(|t| t == EN.codex32_holds_seed),
        "a key with no words is told its backup is not its words: {result:?}"
    );
    assert!(result.iter().any(|t| t == "2 of 3"), "{result:?}");
    h.tap(ids::QUIZ_DONE);

    let pair = [shares[0].as_str(), shares[1].as_str()];
    assert_eq!(load_strings(&pair), Some(made));

    // A SLIP-39 key's seed is its master secret, and it is written the
    // same way.
    let mut h = Harness::new(PANEL);
    start(&mut h, DICE, 0);
    plan(&mut h, 0, 0);
    let first = take_string(&mut h, true);
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);
    assert_eq!(load_strings(&[&first]), Some(made));
}

/// The chevron from the second share comes back to the first, with its
/// own string.
#[test]
fn back_from_a_string_comes_to_the_one_before_it() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DEVICE, 0);
    plan(&mut h, 2, 2);
    gather(&mut h, DEVICE, Strength::Bits128);
    let first = take_string(&mut h, true);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Shown));
    assert!(h.app.texts().iter().any(|t| t == "Share 2 of 2"));
    h.tap(ids::BACK);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Shown));
    assert_eq!(read_string(&mut h), first, "the first string came back");
}
