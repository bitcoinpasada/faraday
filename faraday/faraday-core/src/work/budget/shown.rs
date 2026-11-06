//! A secret the person chose to show is worked out once, when the press
//! that shows it is made, and held until it is hidden (F195): the
//! backup's words with their SeedQR numbers, the copy page's SeedQR, the
//! BIP-85 child, a vault key record's words, a GPG key's paperkey and an
//! entry's TOTP code (once for each period). The press costs what the
//! work is, once; the frames after it work out nothing; hiding it, leaving
//! the page and a lock drop what was held.

use super::{ticks, walk};
use crate::convert::{ConvertAction, ROWS};
use crate::plan::{self, At, Tag, What};
use crate::vault::VaultAction as V;
use crate::work::{Counts, KINDS, PIXELS, Work, since, snapshot};
use crate::{Action, Faraday, Screen, bstep, testkit};
use faraday_shell_api::App;

/// The heavy session on the backup's first copy page, the seed hidden.
fn copy_page() -> Faraday {
    let c = testkit::compute();
    let (mut app, w) = walk(&c, &mut |_, _| {});
    app.press(Action::Backup(w));
    // The first seed's copy takes the SeedQR as well as the words.
    let shape = app.plan_shape_now(w);
    let answers = testkit::backup_answers_mut(&mut app);
    let mut map = plan::stored_map(&shape, answers);
    for spot in &mut map {
        if spot.at == At::Place(0) && spot.holds.iter().any(|(h, _)| *h == What::Words(0)) {
            spot.holds.push((What::SeedQr(0), Tag::Secret));
        }
    }
    answers.set_map(map);
    app.press(Action::BStep(bstep::COPY));
    assert_eq!(app.backup().and_then(|b| b.open), Some(bstep::COPY));
    app.settle();
    let _ = app.frame();
    app
}

/// A press, then a frame: what it works out.
fn press_work(app: &mut Faraday, a: Action) -> Counts {
    let before = snapshot();
    app.press(a);
    let _ = app.frame();
    since(&before)
}

/// Two frames drawn again, nothing changed: no counted work at all.
fn frames_work_out_nothing(app: &mut Faraday) {
    app.settle();
    let _ = app.frame();
    let before = snapshot();
    let _ = app.frame();
    let _ = app.frame();
    let again = since(&before);
    for k in KINDS {
        assert_eq!(again.of(k), 0, "the frames did {k:?} {}", again.of(k));
    }
}

/// `work` is the press's: only the kinds named, as often as named (the
/// fade and the frosted page aside).
fn costs(work: &Counts, named: &[(Work, u64)]) {
    for k in KINDS {
        if PIXELS.contains(&k) {
            continue;
        }
        let want = named.iter().find(|(n, _)| *n == k).map_or(0, |(_, n)| *n);
        assert_eq!(work.of(k), want, "the press did {k:?} {}", work.of(k));
    }
}

#[test]
fn the_backups_words_and_their_seedqr_are_worked_out_once_when_shown() {
    let mut app = copy_page();
    let work = press_work(&mut app, Action::BReveal);
    assert!(app.backup().is_some_and(|b| b.reveal));
    assert!(app.backup.shown_number(0).is_some(), "the numbers are held");
    assert!(app.backup.shown_qr(false).is_some(), "the SeedQR is held");
    costs(&work, &[(Work::WordsParse, 1), (Work::QrEncode, 1)]);
    frames_work_out_nothing(&mut app);
    // Compact is another grid: made once by its press.
    let work = press_work(&mut app, Action::BCompact(true));
    assert!(app.backup.shown_qr(true).is_some());
    costs(&work, &[(Work::WordsParse, 1), (Work::QrEncode, 1)]);
    frames_work_out_nothing(&mut app);
    // Hidden, it is gone.
    let _ = press_work(&mut app, Action::BReveal);
    assert!(!app.backup().is_some_and(|b| b.reveal));
    assert!(app.backup.shown_number(0).is_none());
    assert!(app.backup.shown_qr(true).is_none());
    assert!(!app.backup.shown_held());
}

#[test]
fn the_backups_seed_is_dropped_when_the_page_is_left() {
    let mut app = copy_page();
    let _ = press_work(&mut app, Action::BReveal);
    assert!(app.backup.shown_held());
    app.press(Action::Nav(crate::Screen::Home));
    assert!(!app.backup.shown_held());
}

#[test]
fn a_bip85_child_is_derived_once_when_shown() {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    app.press(Action::Bip85);
    app.press(Action::PKey(app.session.keys[0].fingerprint.0));
    app.press(Action::PStep(crate::bip85::pstep::RESULT));
    app.settle();
    let _ = app.frame();
    assert!(app.bip85.held().is_none());
    let work = press_work(&mut app, Action::PShow);
    assert!(app.bip85().is_some_and(|b| b.shown));
    assert!(app.bip85.held().is_some(), "the child is held");
    costs(&work, &[(Work::KeyDerive, 1)]);
    frames_work_out_nothing(&mut app);
    let _ = press_work(&mut app, Action::PShow);
    assert!(app.bip85.held().is_none());
}

/// Convert a seed's value, the eye closed and the flow open on its Result.
fn converted(row: &str) -> (Faraday, Counts) {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::Convert(ConvertAction::OpenOn(fp)));
    let at = ROWS.iter().position(|r| r.name == row).unwrap() as u8;
    let before = snapshot();
    app.press(Action::Convert(ConvertAction::To(at)));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(faraday_shell_api::Event::Tick { now_ms: now });
    app.settle();
    let _ = app.frame();
    let work = since(&before);
    (app, work)
}

/// What a person sees of the value: shown, hidden, then left.
fn convert_shown_once(row: &str, named: &[(Work, u64)]) {
    let (mut app, work) = converted(row);
    costs(&work, named);
    assert!(
        app.tools.convert().holds_value(),
        "the value is held, masked"
    );
    assert!(crate::convert::shown(&app).is_none());
    assert!(crate::convert::shown_code(&app).is_none());
    // The eye is only a view of what is held: nothing is worked out.
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(
        crate::convert::shown(&app).is_some() || crate::convert::shown_code(&app).is_some(),
        "the value shows"
    );
    frames_work_out_nothing(&mut app);
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(crate::convert::shown(&app).is_none());
    assert!(crate::convert::shown_code(&app).is_none());
    assert!(app.tools.convert().holds_value(), "hidden, still held");
    app.press(Action::Convert(ConvertAction::Show));
    app.press(Action::Convert(ConvertAction::Done));
    assert!(!app.tools.convert().holds_value(), "leaving drops it");
}

#[test]
fn convert_a_seeds_text_is_held_once_and_the_frames_work_out_nothing() {
    convert_shown_once("The words", &[(Work::WordsParse, 2)]);
}

#[test]
fn convert_a_seeds_code_is_held_once_and_the_frames_work_out_nothing() {
    convert_shown_once(
        "SeedQR",
        &[
            (Work::WordsParse, 2),
            (Work::QrEncode, 1),
            (Work::QrDecode, 1),
        ],
    );
}

#[test]
fn convert_a_set_is_joined_once_when_shown_and_the_frames_work_out_nothing() {
    use faraday_shell_api::{EntropyBytes, Event};
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::Convert(ConvertAction::OpenOn(fp)));
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    let at = ROWS
        .iter()
        .position(|r| r.name == "Seed XOR parts")
        .unwrap() as u8;
    app.press(Action::Convert(ConvertAction::To(at)));
    app.press(Action::Convert(ConvertAction::Make));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    app.settle();
    let _ = app.frame();
    assert!(app.tools.convert().holds_value(), "the set is held, masked");
    assert!(app.tools.convert().shown_text().is_none());
    // The press that opens the eye joins the set once; the frames after
    // it draw what is held.
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(app.tools.convert().shown_text().is_some(), "joined, held");
    frames_work_out_nothing(&mut app);
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(app.tools.convert().shown_text().is_none(), "hidden, gone");
    assert!(app.tools.convert().holds_value());
    app.press(Action::Convert(ConvertAction::Show));
    assert!(app.tools.convert().shown_text().is_some());
    app.press(Action::Convert(ConvertAction::Done));
    assert!(
        app.tools.convert().shown_text().is_none(),
        "leaving drops it"
    );
}

#[test]
fn the_frames_of_a_shown_share_work_out_nothing() {
    use crate::convert::set::SetAct;
    use faraday_shell_api::{Event, Key};
    let mut app = testkit::started();
    app.press(Action::Convert(ConvertAction::Open));
    app.press(Action::Convert(ConvertAction::SetSource));
    for share in [
        "shadow pistol academic acid actress prayer class unknown daughter sweater \
         depict flip twice unkind craft early superior advocate guest smoking",
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder \
         mustang wrist rescue view short owner flip making coding armed",
    ] {
        for c in share.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.event(Event::Key(Key::Enter));
    }
    app.press(Action::Convert(ConvertAction::SetAct(SetAct::Add)));
    app.press(Action::Convert(ConvertAction::Make));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    app.settle();
    let _ = app.frame();
    assert!(
        app.tools.convert().holds_value(),
        "the share is held, masked"
    );
    assert!(app.tools.convert().shown_text().is_none());
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(app.tools.convert().shown_text().is_some());
    frames_work_out_nothing(&mut app);
    let _ = press_work(&mut app, Action::Convert(ConvertAction::Show));
    assert!(app.tools.convert().shown_text().is_none(), "hidden, gone");
    app.press(Action::Convert(ConvertAction::Done));
    assert!(!app.tools.convert().holds_value(), "leaving drops it");
}

/// The heavy session on the open vault's item `0` of `category`, its page
/// shown and settled: Bitcoin keys 0, Entries 2, GPG keys 4.
fn vault_page(category: usize) -> Faraday {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(0)));
    app.settle();
    let _ = app.frame();
    app
}

/// Everything the current frame draws, as one text.
fn page_text(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

/// A lock, to its end: the vaults are closed.
fn lock(app: &mut Faraday) {
    app.press(Action::LockUnsealed);
    ticks(app, 400, 4);
    assert!(app.vaults.open.is_empty(), "the lock closed the vaults");
}

/// What is held of the vault page's secrets.
fn held(app: &Faraday) -> (bool, bool, bool) {
    let on = app.vault_on_page();
    (
        app.vaults.words_shown(on).is_some(),
        app.vaults.paperkey_shown(on).is_some(),
        app.vaults.code_held(on).is_some(),
    )
}

#[test]
fn a_key_records_words_are_parsed_once_when_shown_and_nothing_with_the_eye_shut() {
    let mut app = vault_page(0);
    // The eye shut: no parse on any frame, nothing held, no words drawn.
    frames_work_out_nothing(&mut app);
    assert_eq!(held(&app), (false, false, false));
    let words = testkit::test_words("bacon");
    assert!(!page_text(&mut app).contains(&words[..17]));
    let work = press_work(&mut app, Action::Vault(V::Reveal(0)));
    costs(&work, &[(Work::WordsParse, 1)]);
    assert!(held(&app).0, "the words are held");
    frames_work_out_nothing(&mut app);
    assert!(
        page_text(&mut app).contains(&words[..17]),
        "the words are drawn"
    );
    // Hidden, they are gone and drawn no more.
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert!(!held(&app).0);
    assert!(!page_text(&mut app).contains(&words[..17]));
    frames_work_out_nothing(&mut app);
}

#[test]
fn a_key_records_words_are_dropped_when_the_page_is_left_and_at_a_lock() {
    let mut app = vault_page(0);
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert!(held(&app).0);
    app.press(Action::Vault(V::ItemBack));
    assert!(!held(&app).0, "left");
    app.press(Action::Vault(V::Item(0)));
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert!(held(&app).0);
    lock(&mut app);
    assert!(app.vaults.words.is_none(), "locked");
}

/// The vault item's kept pixels, its secret shown, with the page the
/// region a scroll step would move.
fn pixels_with_the_words_shown(app: &mut Faraday) -> Vec<u8> {
    let _ = press_work(app, Action::Vault(V::Reveal(0)));
    assert!(held(app).0);
    app.scroll.went_to(crate::ui::Slot::VaultItem);
    app.draw_whole_next();
    let _ = app.frame();
    let kept = kept_pixels(app);
    assert!(!kept.is_empty(), "the region keeps its pixels");
    kept
}

fn kept_pixels(app: &Faraday) -> Vec<u8> {
    app.canvas
        .as_ref()
        .map_or(Vec::new(), |c| c.layer().to_vec())
}

#[test]
fn a_shown_secrets_pixels_are_not_kept_once_it_is_hidden_left_or_locked() {
    let mut app = vault_page(0);
    let shown = pixels_with_the_words_shown(&mut app);
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert_ne!(kept_pixels(&app), shown, "hidden");

    let shown = pixels_with_the_words_shown(&mut app);
    app.press(Action::Vault(V::ItemBack));
    let _ = app.frame();
    assert_ne!(kept_pixels(&app), shown, "left");

    app.press(Action::Vault(V::Item(0)));
    let shown = pixels_with_the_words_shown(&mut app);
    lock(&mut app);
    let _ = app.frame();
    assert_ne!(kept_pixels(&app), shown, "locked");
}

#[test]
fn a_gpg_paperkey_is_made_once_when_shown() {
    let mut app = vault_page(4);
    frames_work_out_nothing(&mut app);
    assert_eq!(held(&app), (false, false, false));
    let work = press_work(&mut app, Action::Vault(V::Reveal(0)));
    costs(&work, &[]);
    assert!(held(&app).1, "the paperkey is held");
    frames_work_out_nothing(&mut app);
    let key = app
        .vaults
        .paperkey_shown(app.vault_on_page())
        .expect("held")
        .to_string();
    let shown: String = page_text(&mut app).split_whitespace().collect();
    let first: String = key.split_whitespace().take(3).collect();
    assert!(shown.contains(&first), "the paperkey is drawn");
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert!(!held(&app).1);
    // Shown again, then the page left, then a lock.
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    app.press(Action::Vault(V::ItemBack));
    assert!(!held(&app).1, "left");
    app.press(Action::Vault(V::Item(0)));
    let _ = press_work(&mut app, Action::Vault(V::Reveal(0)));
    assert!(held(&app).1);
    lock(&mut app);
    assert!(app.vaults.paperkey.is_none(), "locked");
}

#[test]
fn a_gpg_paperkeys_code_is_made_once_at_the_press_and_dropped_with_the_sheet() {
    let mut app = vault_page(4);
    frames_work_out_nothing(&mut app);
    let work = press_work(&mut app, Action::Vault(V::PaperkeyQr));
    costs(&work, &[(Work::QrEncode, 1)]);
    assert!(app.qr().is_some(), "the press opens the sheet");
    // Drawing the sheet works nothing out.
    frames_work_out_nothing(&mut app);
    app.press(Action::Cancel);
    assert!(app.qr().is_none(), "Done drops the code");
    let _ = press_work(&mut app, Action::Vault(V::PaperkeyQr));
    app.press(Action::Vault(V::ItemBack));
    app.settle();
    lock(&mut app);
    assert!(app.qr().is_none(), "a lock drops the code");
}

/// The code the entry gives now, as the page draws it.
fn code_now(app: &Faraday) -> String {
    let now = app.clock_now().expect("a checked clock");
    faraday_keys::totp::code_at("JBSWY3DPEHPK3PXP", now)
        .expect("a TOTP key")
        .grouped
        .to_string()
}

#[test]
fn an_entrys_code_is_worked_out_once_for_each_period() {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    app.press(Action::Nav(Screen::VaultContents));
    let work = press_work(&mut app, Action::Vault(V::Category(2)));
    assert_eq!(work.of(Work::TotpCode), 1, "the page's code is made once");
    let work = press_work(&mut app, Action::Vault(V::Item(0)));
    assert_eq!(work.of(Work::TotpCode), 0, "opening its page makes no more");
    assert!(held(&app).2, "the code is held");
    app.settle();
    let _ = app.frame();
    assert!(page_text(&mut app).contains(&code_now(&app)));
    frames_work_out_nothing(&mut app);
    // The clock runs on inside the period: frames and ticks, no work.
    let period = app.vaults.code_held(true).expect("held").period;
    let left = period - app.clock_now().expect("clock") % period;
    let before = snapshot();
    let at = app.now_ms();
    for k in 1..left {
        app.event(faraday_shell_api::Event::Tick {
            now_ms: at + k * 1000,
        });
        let _ = app.frame();
    }
    assert_eq!(since(&before).of(Work::TotpCode), 0);
    // The period rolls over with no input: a new code, once.
    let old = code_now(&app);
    let before = snapshot();
    app.event(faraday_shell_api::Event::Tick {
        now_ms: at + left * 1000,
    });
    let _ = app.frame();
    let _ = app.frame();
    assert_eq!(since(&before).of(Work::TotpCode), 1);
    assert_ne!(code_now(&app), old);
    assert!(page_text(&mut app).contains(&code_now(&app)));
}

#[test]
fn an_entrys_code_is_dropped_when_the_page_is_left_and_at_a_lock() {
    let mut app = vault_page(2);
    assert!(held(&app).2);
    // On a wide panel the entry shows beside the list, so closing its
    // page keeps its code; leaving Vault contents drops it.
    app.press(Action::Vault(V::ItemBack));
    let _ = app.frame();
    assert!(held(&app).2, "beside the list");
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.vaults.code.is_none(), "left");
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    assert!(held(&app).2);
    // The lock itself drops it: no frame is drawn after this press.
    app.press(Action::LockUnsealed);
    assert!(app.vaults.open.is_empty(), "the lock closed the vaults");
    assert!(app.vaults.code.is_none(), "locked");
}

/// The heavy session on the Codes screen, settled; the entries it lists.
fn codes_screen() -> (Faraday, usize) {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    app.press(Action::Nav(Screen::Home));
    let before = snapshot();
    app.press(Action::Nav(Screen::Codes));
    let _ = app.frame();
    let n = app.vaults.codes_held().map_or(0, |h| h.0.len());
    assert!(n > 0, "the vault has entries with a code");
    assert_eq!(since(&before).of(Work::TotpCode) as usize, n);
    app.settle();
    let _ = app.frame();
    (app, n)
}

#[test]
fn the_codes_screen_works_out_each_entrys_code_once_for_each_period() {
    let (mut app, n) = codes_screen();
    assert_eq!(app.screen, Screen::Codes);
    frames_work_out_nothing(&mut app);
    let now = app.clock_now().expect("a clock");
    let texts = page_text(&mut app);
    let held = app.vaults.codes_held().expect("held");
    let first = held.0.iter().find_map(|l| match &l.result {
        Some(Ok(Some(s))) => Some(s.grouped.to_string()),
        _ => None,
    });
    assert!(texts.contains(&first.expect("a code")), "the code is drawn");
    // Inside the period the clock runs on: ticks and frames, no work.
    let period = held.0.first().map_or(0, |l| l.period);
    assert!(period > 0, "the first entry listed gives a code");
    let left = period - now % period;
    let at = app.now_ms();
    let before = snapshot();
    for k in 1..left {
        app.event(faraday_shell_api::Event::Tick {
            now_ms: at + k * 1000,
        });
        let _ = app.frame();
    }
    assert_eq!(since(&before).of(Work::TotpCode), 0);
    // The period rolls over: each entry of that period gets a new code,
    // the others none.
    let moved = app
        .vaults
        .codes_held()
        .expect("held")
        .0
        .iter()
        .filter(|l| l.period == period)
        .count() as u64;
    let before = snapshot();
    app.event(faraday_shell_api::Event::Tick {
        now_ms: at + left * 1000,
    });
    let _ = app.frame();
    let _ = app.frame();
    let made = since(&before).of(Work::TotpCode);
    assert_eq!(made, moved);
    assert!(made as usize <= n);
}

#[test]
fn the_codes_hold_is_dropped_on_leaving_the_screen_and_at_a_lock() {
    let (mut app, _) = codes_screen();
    assert!(app.vaults.codes_held().is_some());
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.vaults.codes_held().is_none(), "left");
    app.press(Action::Nav(Screen::Codes));
    let _ = app.frame();
    assert!(app.vaults.codes_held().is_some());
    // The lock itself drops them: no frame is drawn after this press.
    app.press(Action::LockUnsealed);
    assert!(app.vaults.open.is_empty(), "the lock closed the vaults");
    assert!(app.vaults.codes_held().is_none(), "locked");
}

#[test]
fn an_edited_entrys_row_shows_its_new_code_and_typing_in_the_search_works_out_none() {
    let (mut app, _) = codes_screen();
    let c = testkit::compute();
    let id = app.vaults.open[0].id;
    let at = app.vaults.shown_codes(id)[0];
    let uri = b"otpauth://totp/Edited?secret=GEZDGNBVGY3TQOJQ";
    app.vaults
        .edit(id, |s| {
            s.set_fields(
                &c,
                id,
                Some(at),
                faraday_vault::records::kind::ENTRY,
                &[(6, uri)],
            )
        })
        .expect("edited");
    let now = app.clock_now().expect("a clock");
    let new = faraday_keys::totp::code_at("GEZDGNBVGY3TQOJQ", now)
        .expect("a key")
        .grouped
        .to_string();
    app.settle();
    let _ = app.frame();
    assert!(page_text(&mut app).contains(&new), "the new code is drawn");
    // Typing in the search bar works out no code.
    app.press(Action::Vault(V::Search(crate::vault::SearchAct::Focus(
        crate::vault::SearchIn::Codes,
    ))));
    let before = snapshot();
    for ch in "ma".chars() {
        app.event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Char(
            ch,
        )));
        let _ = app.frame();
    }
    assert_eq!(since(&before).of(Work::TotpCode), 0);
}

/// A key as a person presses it: the event, then the frame it asks for.
fn key(app: &mut Faraday, k: faraday_shell_api::Key) {
    app.event(faraday_shell_api::Event::Key(k));
    let _ = app.frame();
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        key(app, faraday_shell_api::Key::Char(c));
    }
}

/// The code the entry page holds, as drawn grouped.
fn page_code(app: &Faraday) -> String {
    match app.vaults.code_held(true).map(|h| &h.result) {
        Some(Ok(Some(s))) => s.grouped.to_string(),
        _ => panic!("the page holds no code"),
    }
}

#[test]
fn typing_in_the_items_search_shows_the_new_entrys_code_after_the_frame() {
    let mut app = vault_page(2);
    let id = app.vaults.open[0].id;
    // A second entry with a TOTP secret, so the search moves the page.
    let c = testkit::compute();
    let title = "Zeta".to_string();
    app.vaults
        .edit(id, |s| {
            s.set_fields(
                &c,
                id,
                None,
                faraday_vault::records::kind::ENTRY,
                &[
                    (faraday_vault::records::field::TITLE, title.as_bytes()),
                    (
                        faraday_vault::records::field::TOTP,
                        b"otpauth://totp/Z?secret=GEZDGNBVGY3TQOJQ",
                    ),
                ],
            )
        })
        .expect("added");
    app.settle();
    let _ = app.frame();
    let at = app
        .vaults
        .records(id)
        .iter()
        .find(|(_, r)| r.public_text(faraday_vault::records::field::TITLE) == Some("Zeta"))
        .map(|(i, _)| i)
        .expect("the new entry");
    assert_ne!(app.vaults.selected_place(), Some(at), "it is not the first");
    app.press(Action::Vault(V::Search(crate::vault::SearchAct::Focus(
        crate::vault::SearchIn::Items,
    ))));
    // The first character already moves the page to the new entry.
    key(&mut app, faraday_shell_api::Key::Char('Z'));
    assert_eq!(app.vaults.code_held(true).map(|h| h.at.record), Some(at));
    assert!(!page_text(&mut app).contains(crate::EN.vault_unreadable));
    type_in(&mut app, &title[1..]);
    assert_eq!(app.vaults.selected_place(), Some(at));
    assert_eq!(app.vaults.code_held(true).map(|h| h.at.record), Some(at));
    let text = page_text(&mut app);
    assert!(text.contains(&page_code(&app)), "the code is drawn");
    assert!(!text.contains(crate::EN.vault_unreadable), "{text}");
}

#[test]
fn deleting_the_codes_query_brings_every_row_back_and_works_out_no_code() {
    let (mut app, n) = codes_screen();
    app.press(Action::Vault(V::Search(crate::vault::SearchAct::Focus(
        crate::vault::SearchIn::Codes,
    ))));
    let before = snapshot();
    type_in(&mut app, "zzzz");
    assert!(!page_text(&mut app).contains(crate::EN.codes_unreadable));
    for _ in 0..4 {
        key(&mut app, faraday_shell_api::Key::Backspace);
    }
    assert_eq!(since(&before).of(Work::TotpCode), 0);
    let text = page_text(&mut app);
    let held = app.vaults.codes_held().expect("held");
    assert_eq!(held.0.len(), n);
    for l in &held.0 {
        if let Some(Ok(Some(s))) = &l.result {
            assert!(text.contains(&*s.grouped), "every row is back: {text}");
        }
    }
}

#[test]
fn leaving_an_entrys_edit_form_by_escape_or_enter_shows_its_code_after_the_frame() {
    use faraday_shell_api::Key;
    let mut app = vault_page(2);
    app.press(Action::Vault(V::Edit));
    assert!(app.vaults.form.is_some());
    let _ = app.frame();
    key(&mut app, Key::Escape);
    assert!(app.vaults.form.is_none());
    let text = page_text(&mut app);
    assert!(text.contains(&page_code(&app)), "Escape: {text}");
    assert!(!text.contains(crate::EN.vault_unreadable), "Escape: {text}");
    // Enter on the last field saves; a longer URI gives other digits.
    let old = page_code(&app);
    app.press(Action::Vault(V::Edit));
    app.press(Action::Vault(V::FocusField(5)));
    type_in(&mut app, "&digits=8");
    key(&mut app, Key::Enter);
    assert!(app.vaults.form.is_none(), "saved");
    let text = page_text(&mut app);
    let new = page_code(&app);
    assert_ne!(new, old, "the new secret's code");
    assert!(text.contains(&new), "Enter: {text}");
}

#[test]
fn a_sheet_over_an_entrys_page_draws_no_unreadable_badge() {
    let mut app = vault_page(2);
    app.press(Action::Vault(V::TotpQr));
    let _ = app.frame();
    assert!(app.sheet.is_some(), "a sheet is over the page");
    let text = page_text(&mut app);
    assert!(!text.contains(crate::EN.vault_unreadable), "{text}");
}

#[test]
fn the_codes_screen_draws_hotp_unreadable_and_two_periods_each_rolling_over_alone() {
    use faraday_vault::records::{field, kind};
    let (mut app, _) = codes_screen();
    let c = testkit::compute();
    let id = app.vaults.open[0].id;
    for (title, uri) in [
        (
            "H",
            &b"otpauth://hotp/H?secret=GEZDGNBVGY3TQOJQ&counter=1"[..],
        ),
        ("U", b"not a key at all"),
        ("P", b"otpauth://totp/P?secret=GEZDGNBVGY3TQOJQ&period=60"),
        ("Q", b"otpauth://totp/Q?secret=GEZDGNBVGY3TQOJQ"),
    ] {
        app.vaults
            .edit(id, |s| {
                s.set_fields(
                    &c,
                    id,
                    None,
                    kind::ENTRY,
                    &[(field::TITLE, title.as_bytes()), (field::TOTP, uri)],
                )
            })
            .expect("added");
    }
    let _ = app.frame();
    let text = page_text(&mut app);
    assert!(text.contains(crate::EN.codes_hotp), "{text}");
    assert!(text.contains(crate::EN.codes_unreadable), "{text}");
    assert_eq!(app.vaults.codes_held().map(|h| h.0.len()), Some(5));
    let mut moved_some = false;
    for round in 1..=2 {
        let now = app.clock_now().expect("a clock");
        let next = now + (30 - now % 30);
        let expected = app
            .vaults
            .codes_held()
            .expect("held")
            .0
            .iter()
            .filter(|l| l.period > 0 && next / l.period != now / l.period)
            .count() as u64;
        let all = app
            .vaults
            .codes_held()
            .expect("held")
            .0
            .iter()
            .filter(|l| l.period > 0)
            .count() as u64;
        moved_some |= expected < all;
        let before = snapshot();
        app.event(faraday_shell_api::Event::Tick {
            now_ms: app.now_ms() + (next - now) * 1000,
        });
        let _ = app.frame();
        let _ = app.frame();
        assert_eq!(since(&before).of(Work::TotpCode), expected, "round {round}");
    }
    assert!(moved_some, "one rollover left the 60 s entry alone");
}
