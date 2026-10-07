//! Writing SLIP-39 shares (`docs/PLANNING.md` §16.107 rules 4 and 5):
//! Add a key › Create SLIP-39 shares, and Backup › SLIP-39 shares on a
//! key this device already holds.
//!
//! Every share here is read off the screen as a person reads it, word by
//! numbered word from the panel, and then typed back in through Load a
//! key › SLIP-39 shares in a fresh session. Nothing trusts one
//! computation: the shares this device writes are also handed to the
//! codec's own `Share::parse` and `recover`.

mod common;

use common::{Harness, PANEL, PHONE};
use opensigner_core::create::Step;
use opensigner_core::quiz::QuizState;
use opensigner_core::shares::Step as ShareStep;
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, ids};
use osk_bip::slip39::{self, Share};
use osk_entropy::{DiceRolls, Strength};
use osk_shell_api::Key;

/// Rolls that show no sanity flag, repeated as far as a strength needs.
const ROLLS: &str = "32461151351521144121541512665155412152342515356215";

/// Which source row "This device" is, in `SOURCE_ROWS` order.
const DEVICE: usize = 6;
/// Which source row dice is.
const DICE: usize = 0;

/// Add a key → "Create SLIP-39 shares" → `source` → the word count at
/// `count_row` (0 is 20 words, 1 is 33) → the entropy → the passphrase
/// offer.
fn start(h: &mut Harness, source: usize, count_row: usize) {
    h.open_add(ids::ADD_CREATE_SLIP39);
    assert_eq!(h.app.screen(), ScreenKind::Create);
    assert_eq!(h.app.create_step(), Some(Step::Source));
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, source),
        ids::CREATE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(Step::Count));
    // A share is 20 or 33 words, and those are the rows on offer.
    let texts = h.app.texts();
    for n in ["20", "33"] {
        assert!(
            texts.iter().any(|t| t == n),
            "{n} is not offered: {texts:?}"
        );
    }
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, count_row),
        ids::CREATE_COUNT_CONTINUE,
    );
    // There is no language to choose: a share is spelled from one list.
    gather(h, source, count_row);
}

/// One run of the chosen source's own screens, at the strength the word
/// count asks for.
fn gather(h: &mut Harness, source: usize, count_row: usize) {
    if source == DEVICE {
        assert_eq!(h.app.create_step(), Some(Step::Device));
        h.tap(ids::CREATE_CONTINUE);
        return;
    }
    // The dice source asks which published procedure reads the rolls;
    // direct selection is dimmed here, because a SLIP-39 master secret
    // is not spelled in BIP-39 words.
    if h.app.create_step() == Some(Step::Procedure) {
        h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    }
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    let strength = if count_row == 0 {
        Strength::Bits128
    } else {
        Strength::Bits256
    };
    let need = DiceRolls::needed(strength);
    let rolls: String = ROLLS.chars().cycle().take(need).collect();
    h.type_text(&rolls);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);
}

/// The rows the revealed panel is showing: "` 1. academic`" numbered
/// from one.
fn panel_rows(h: &Harness) -> Vec<(usize, String)> {
    h.app
        .texts()
        .into_iter()
        .filter_map(|t| {
            let (number, rest) = t.split_once(". ")?;
            let n: usize = number.trim().parse().ok()?;
            Some((n, rest.trim().to_string()))
        })
        .collect()
}

/// The `count` words of the share now on screen, read page by page with
/// the panel held open.
fn read_share(h: &mut Harness, count: usize) -> Vec<String> {
    let mut words = vec![String::new(); count];
    for _ in 0..6 {
        let point = h.press(ids::CREATE_REVEAL);
        for (n, word) in panel_rows(h) {
            if (1..=count).contains(&n) {
                words[n - 1] = word;
            }
        }
        h.release(point);
        if words.iter().all(|w| !w.is_empty()) {
            break;
        }
        assert!(h.app.rect_of(ids::WORDS_NEXT).is_some(), "the panel pages");
        h.tap(ids::WORDS_NEXT);
    }
    assert!(
        words.iter().all(|w| !w.is_empty()),
        "the share's words were not all on the screen"
    );
    words
}

/// Answers the quiz over `words` correctly.
fn pass_quiz(h: &mut Harness, words: &[String]) {
    for _ in 0..words.len() {
        let v = h.app.quiz_view().expect("a quiz on screen");
        assert_eq!(v.state, QuizState::Asking);
        let want = &words[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .unwrap_or_else(|| panic!("word {} not offered in {:?}", v.word_number, v.choices));
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
}

/// Reads the share on screen and goes on: the quiz where `quiz`, the
/// caution and the skip where not.
fn take_share(h: &mut Harness, count: usize, quiz: bool) -> String {
    assert_eq!(h.app.split_step(), Some(ShareStep::Words));
    let words = read_share(h, count);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.split_step(), Some(ShareStep::QuizStart));
    if quiz {
        h.tap(ids::QUIZ_START);
        pass_quiz(h, &words);
    } else {
        h.tap(ids::QUIZ_SKIP);
        assert_eq!(h.app.split_step(), Some(ShareStep::QuizSkip));
        h.tap(ids::QUIZ_SKIP_CONFIRM);
    }
    words.join(" ")
}

/// The plan Choices for one group of `count` shares, `threshold` of
/// which must be present.
fn plan_one_group(h: &mut Harness, count: usize, threshold: usize) {
    assert_eq!(h.app.split_step(), Some(ShareStep::Groups));
    h.choose(
        ids::at(ids::SHARE_GROUPS_BASE, 0),
        ids::SHARE_GROUPS_CONTINUE,
    );
    assert_eq!(h.app.split_step(), Some(ShareStep::Count));
    h.choose(
        ids::at(ids::SHARE_COUNT_BASE, count - 1),
        ids::SHARE_COUNT_CONTINUE,
    );
    assert_eq!(h.app.split_step(), Some(ShareStep::Threshold));
    h.choose(
        ids::at(ids::SHARE_THRESHOLD_BASE, threshold - 1),
        ids::SHARE_THRESHOLD_CONTINUE,
    );
}

/// The fingerprint of the one key loaded, as hex.
fn only_fingerprint(h: &Harness) -> String {
    let keys = h.app.fingerprints();
    assert_eq!(keys.len(), 1, "one key");
    keys[0].to_hex().iter().map(|b| *b as char).collect()
}

/// Loads `shares` through Load a key › SLIP-39 shares in a fresh
/// session and returns the fingerprint of the key they give, or `None`
/// where the set is still short.
fn load_shares(count_row: usize, shares: &[&str], passphrase: Option<&str>) -> Option<String> {
    let mut h = Harness::new(PANEL);
    h.open_slip39(count_row);
    for share in shares {
        h.type_share(share);
        assert!(
            h.app
                .texts()
                .iter()
                .any(|t| t == EN.load_share_accepted_title),
            "a share this device wrote was refused: {:?}",
            h.app.texts()
        );
        h.tap(ids::LOAD_CONTINUE);
    }
    if h.app.load_step() != Some(opensigner_core::load::Step::PassphraseOffer) {
        return None;
    }
    h.finish_passphrase(passphrase);
    Some(only_fingerprint(&h))
}

/// Create SLIP-39 shares from dice, one group of three, two of which
/// must be present: the three shares are shown, two of them open the
/// same key in a fresh session, and one alone keeps asking.
#[test]
fn three_shares_two_of_which_open_the_key_this_device_made() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DICE, 0);
    assert_eq!(h.app.create_step(), Some(Step::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    plan_one_group(&mut h, 3, 2);
    // One random value is needed for a threshold of two, and Create
    // gathers it from the source the key was made from.
    assert_eq!(h.app.split_step(), Some(ShareStep::Gather));
    gather(&mut h, DICE, 0);
    let mut shares = Vec::new();
    for i in 0..3 {
        let title = format!("Share {} of 3", i + 1);
        assert!(
            h.app.texts().contains(&title),
            "share {} is not titled {title:?}: {:?}",
            i + 1,
            h.app.texts()
        );
        shares.push(take_share(&mut h, 20, true));
    }
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);
    assert_eq!(h.app.backup_verified(0), Some(true), "every quiz passed");

    assert_eq!(
        load_shares(0, &[&shares[0], &shares[2]], None),
        Some(made),
        "shares 1 and 3 do not open the key this device made"
    );
    assert_eq!(
        load_shares(0, &[&shares[0]], None),
        None,
        "one share of a 2-of-3 backup opened the key"
    );
}

/// The same with a passphrase: the shares carry the secret under it, so
/// reading them under it gives the key that was added and reading them
/// under none gives another. The comparison the wizard showed names
/// both.
#[test]
fn a_passphrase_names_both_keys_and_writes_the_shares_under_it() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DICE, 0);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Passphrase));
    h.type_text("TREZOR");
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::PassphraseConfirm));
    let compared = h.app.texts();
    h.tap(ids::LOAD_WHICH_CONTINUE);
    plan_one_group(&mut h, 2, 2);
    gather(&mut h, DICE, 0);
    let shares: Vec<String> = (0..2).map(|_| take_share(&mut h, 20, false)).collect();
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);
    assert_eq!(h.app.backup_verified(0), Some(false), "a quiz was skipped");

    let both: Vec<&str> = shares.iter().map(String::as_str).collect();
    let with = load_shares(0, &both, Some("TREZOR")).expect("the shares open a key");
    let without = load_shares(0, &both, None).expect("the shares open a key");
    assert_eq!(with, made, "the passphrase key is the key that was added");
    assert_ne!(without, made, "no passphrase is another key");
    assert!(
        compared.contains(&with) && compared.contains(&without),
        "the comparison named {compared:?}, not {with} and {without}"
    );
}

/// Groups: two of three groups, one share in the first, two of three in
/// the second and three of five in the third, over a 256-bit secret.
/// Every share is titled by its group, and which groups are present is
/// what decides whether the key comes back.
#[test]
fn a_backup_in_groups_is_written_group_by_group() {
    let mut h = Harness::new(PHONE);
    start(&mut h, DEVICE, 1);
    assert_eq!(h.app.create_step(), Some(Step::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.split_step(), Some(ShareStep::Groups));
    h.choose(
        ids::at(ids::SHARE_GROUPS_BASE, 2),
        ids::SHARE_GROUPS_CONTINUE,
    );
    assert_eq!(h.app.split_step(), Some(ShareStep::GroupThreshold));
    h.choose(
        ids::at(ids::SHARE_QUORUM_BASE, 1),
        ids::SHARE_QUORUM_CONTINUE,
    );
    for (count, threshold) in [(1usize, 1usize), (3, 2), (5, 3)] {
        assert_eq!(h.app.split_step(), Some(ShareStep::Count));
        h.choose(
            ids::at(ids::SHARE_COUNT_BASE, count - 1),
            ids::SHARE_COUNT_CONTINUE,
        );
        // A group of one share has no threshold to choose.
        if count == 1 {
            continue;
        }
        assert_eq!(h.app.split_step(), Some(ShareStep::Threshold));
        h.choose(
            ids::at(ids::SHARE_THRESHOLD_BASE, threshold - 1),
            ids::SHARE_THRESHOLD_CONTINUE,
        );
    }
    // One value at the group level, none in the first group, one in the
    // second and two in the third.
    for _ in 0..4 {
        assert_eq!(h.app.split_step(), Some(ShareStep::Gather));
        gather(&mut h, DEVICE, 1);
    }
    let mut shares: Vec<Vec<String>> = vec![Vec::new(), Vec::new(), Vec::new()];
    for (group, count) in [(1usize, 1usize), (2, 3), (3, 5)] {
        for i in 0..count {
            let title = format!("Group {group} \u{00b7} Share {} of {count}", i + 1);
            assert!(
                h.app.texts().contains(&title),
                "the share is not titled {title:?}: {:?}",
                h.app.texts()
            );
            shares[group - 1].push(take_share(&mut h, 33, false));
        }
    }
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);

    let enough = [
        shares[0][0].as_str(),
        shares[1][0].as_str(),
        shares[1][1].as_str(),
    ];
    assert_eq!(load_shares(1, &enough, None), Some(made.clone()));
    let short = [shares[0][0].as_str(), shares[1][0].as_str()];
    assert_eq!(
        load_shares(1, &short, None),
        None,
        "one group short opened the key"
    );
    let other = [
        shares[1][0].as_str(),
        shares[1][1].as_str(),
        shares[2][0].as_str(),
        shares[2][1].as_str(),
        shares[2][2].as_str(),
    ];
    assert_eq!(load_shares(1, &other, None), Some(made));

    // The codec reads back what this device wrote, and every share's
    // header states the plan it was made under.
    let all: Vec<Share> = shares
        .iter()
        .flatten()
        .map(|s| Share::parse(s).expect("a share this device wrote"))
        .collect();
    let identifier = all[0].identifier();
    for share in &all {
        assert_eq!(share.identifier(), identifier, "one backup, one identifier");
        assert!(share.is_extendable());
        assert_eq!(share.iteration_exponent(), 1);
        assert_eq!(share.group_threshold(), 2);
        assert_eq!(share.group_count(), 3);
        assert_eq!(share.secret_len(), 32);
    }
    let by_group = |g: u8| all.iter().filter(move |s| s.group_index() == g);
    assert_eq!(by_group(0).count(), 1);
    assert_eq!(by_group(1).count(), 3);
    assert_eq!(by_group(2).count(), 5);
    assert_eq!(by_group(1).next().unwrap().member_threshold(), 2);
    assert_eq!(by_group(2).next().unwrap().member_threshold(), 3);
    let set: Vec<Share> = by_group(0).chain(by_group(1).take(2)).cloned().collect();
    let secret = slip39::recover(&set, b"").expect("the codec reads its own shares");
    assert_eq!(secret.expose().as_bytes().len(), 32);
    // Every share of a split is a share of its own.
    for (i, a) in all.iter().enumerate() {
        for b in all.iter().skip(i + 1) {
            assert_ne!(a.indices(), b.indices(), "two shares are the same words");
        }
    }
}

/// Backup › SLIP-39 shares on a SLIP-39 key: the flow makes a second
/// backup, with an identifier of its own, whose shares open the same
/// key; and a key made of words is not offered the row.
#[test]
fn a_second_backup_of_the_same_key_is_a_second_backup() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DEVICE, 0);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    plan_one_group(&mut h, 2, 2);
    gather(&mut h, DEVICE, 0);
    let first: Vec<String> = (0..2).map(|_| take_share(&mut h, 20, false)).collect();
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let made = only_fingerprint(&h);

    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    assert!(
        h.app.rect_of(ids::BACKUP_SLIP39).is_some(),
        "a SLIP-39 key's Backup menu has no live shares row"
    );
    h.tap(ids::BACKUP_SLIP39);
    // The shares are made under a passphrase of this backup's own.
    assert_eq!(h.app.split_step(), Some(ShareStep::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    plan_one_group(&mut h, 3, 2);
    // Backup asks where the randomness comes from; Create does not.
    assert_eq!(h.app.split_step(), Some(ShareStep::Source));
    h.choose(
        ids::at(ids::SHARE_SOURCE_BASE, DEVICE),
        ids::SHARE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.split_step(), Some(ShareStep::Gather));
    assert_eq!(h.app.create_step(), Some(Step::Device));
    h.tap(ids::CREATE_CONTINUE);
    let second: Vec<String> = (0..3).map(|_| take_share(&mut h, 20, true)).collect();
    assert_eq!(h.app.split_step(), Some(ShareStep::Result));
    let result = h.app.texts();
    for line in [EN.slip39_result_title, "1 of 1", "2 of 3 shares"] {
        assert!(
            result.iter().any(|t| t == line),
            "the result does not state {line:?}: {result:?}"
        );
    }
    h.tap(ids::QUIZ_DONE);
    assert_eq!(h.app.backup_verified(0), Some(true));

    let new = [second[0].as_str(), second[2].as_str()];
    assert_eq!(
        load_shares(0, &new, None),
        Some(made),
        "the new shares open another key"
    );
    let a = Share::parse(&first[0]).unwrap();
    let b = Share::parse(&second[0]).unwrap();
    assert_ne!(
        a.identifier(),
        b.identifier(),
        "two backups of one key share an identifier"
    );

    // A key made of words is never backed up as SLIP-39 shares.
    h.go_home();
    h.start_load(&common::ABANDON);
    h.finish_load(None);
    h.open_key(1);
    h.tap(ids::DETAIL_BACKUP);
    assert!(
        h.app.rect_of(ids::BACKUP_SLIP39).is_none(),
        "a BIP-39 key's Backup menu offers SLIP-39 shares"
    );
}

/// The chevron from the second share comes back to the first, with its
/// own words; a wrong quiz answer is refused.
#[test]
fn back_from_a_share_comes_to_the_one_before_it() {
    let mut h = Harness::new(PANEL);
    start(&mut h, DEVICE, 0);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    plan_one_group(&mut h, 2, 2);
    gather(&mut h, DEVICE, 0);
    let first = read_share(&mut h, 20);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    // A word that is not the one asked for is refused, and the screen
    // never names the right one.
    let v = h.app.quiz_view().expect("a quiz");
    let want = &first[v.word_number - 1];
    let wrong = v.choices.iter().position(|c| c != want).expect("a decoy");
    h.choose(ids::at(ids::QUIZ_CHOICE_BASE, wrong), ids::QUIZ_CONTINUE);
    assert_eq!(h.app.quiz_view().unwrap().state, QuizState::Wrong);
    for text in h.app.texts() {
        assert!(!text.contains(want.as_str()), "the screen names {want:?}");
    }
    h.tap(ids::QUIZ_RETRY);
    pass_quiz(&mut h, &first);

    assert_eq!(h.app.split_step(), Some(ShareStep::Words));
    let title = "Share 2 of 2";
    assert!(h.app.texts().iter().any(|t| t == title));
    h.tap(ids::BACK);
    assert_eq!(h.app.split_step(), Some(ShareStep::Words));
    assert_eq!(read_share(&mut h, 20), first, "the first share came back");
}
