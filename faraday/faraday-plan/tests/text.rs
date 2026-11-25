//! The plan's saved text read back: what is accepted and what is refused.

use faraday_plan::{Answers, Seed, Shape};
use faraday_text::Text;

fn shape() -> Shape {
    Shape {
        m: 1,
        paths: Vec::new(),
        keys: 1,
        seeds: vec![Seed {
            name: Text::of("a"),
            here: true,
            shares: false,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
        seed_alone_fails: false,
        text_files: false,
        silent: false,
        said: false,
    }
}

fn saved() -> String {
    String::from(&*Answers::defaults(&shape()).to_text())
}

fn read(text: &str) -> Option<Answers> {
    Answers::from_text(&shape(), text)
}

fn with(extra: &str) -> String {
    format!("{}{extra}\n", saved())
}

#[test]
fn a_saved_plan_reads_back() {
    let a = read(&saved()).expect("the plan reads");
    assert_eq!(a.to_text().as_str(), saved());
}

#[test]
fn lines_read_past_are_accepted() {
    for extra in [
        "held 0 x",
        "seals 1 2",
        "layers 3",
        "route 1",
        "preset 0",
        "preset 2",
        "custom 1",
        "custom 0",
        "part 0 0",
    ] {
        assert!(read(&with(extra)).is_some(), "{extra}");
    }
    assert!(read(&with("preset 2")).unwrap().custom);
    assert!(!read(&with("preset 0")).unwrap().custom);
}

#[test]
fn a_repeated_line_reads_as_the_last() {
    let a = read(&with("places 1")).expect("repeated places");
    assert_eq!(a.places, 1);
    assert!(read(&with("split 1")).unwrap().split);
    assert!(read(&with("part 0 0\npart 0 0")).is_some());
}

#[test]
fn an_unknown_or_malformed_line_is_refused() {
    for extra in [
        "unknown 1",
        "bogus",
        "",
        "places 1 ",
        "places 1025",
        "places x",
        "seeds 100",
        "seeds 10x0",
        "bare 1,x",
        "omit x",
        "pass 5 1",
        "pass x 1",
        "sticks 10",
        "sticks x 1",
        "vault 0 x",
        "spot",
        "spot zz",
        "rotated 0 1 abcd",
        "rotated 5 2 abcd",
        "rotated 0 2",
        "part 0 5",
        "part x 0",
        "codex32 5 x",
    ] {
        assert!(read(&with(extra)).is_none(), "{extra:?}");
    }
}

#[test]
fn trailing_text_is_refused() {
    assert!(read(&format!("{}trailing", saved())).is_none());
    assert!(read(&format!("{}\n", saved())).is_none());
}

fn without(text: &str, names: &[&str]) -> String {
    text.lines()
        .filter(|l| !names.contains(&l.split(' ').next().unwrap_or("")))
        .map(|l| format!("{l}\n"))
        .collect()
}

#[test]
fn a_plan_needs_eight_answer_lines() {
    let text = saved();
    for name in [
        "seeds", "places", "split", "wallet", "software", "form", "omit", "sticks", "vault",
    ] {
        assert!(read(&without(&text, &[name])).is_some(), "without {name}");
    }
    for pair in [["seeds", "wallet"], ["places", "omit"], ["vault", "sticks"]] {
        assert!(read(&without(&text, &pair)).is_none(), "without {pair:?}");
    }
    assert!(read("").is_none());
}

#[test]
fn a_spot_keeping_a_string_the_set_does_not_have_is_refused() {
    // Seed 0's set needs two strings and has indices 29 and 24.
    let set = "codex32 0 2 48 ac qprz 0";
    assert!(read(&with(&format!("{set}\nspot place0 x0.29 x0.24"))).is_some());
    assert!(read(&with(&format!("{set}\nspot place0 x0.29 x0.10"))).is_none());
}

#[test]
fn the_vaults_left_out_of_keeping_the_plan_read_back_and_none_adds_no_line() {
    assert!(
        !saved().contains("unkept"),
        "a plan leaving none out is as before"
    );
    let mut a = Answers::defaults(&shape());
    a.set_keeps_plan(2, false);
    a.set_keeps_plan(1, false);
    let text = String::from(&*a.to_text());
    assert!(text.contains("unkept 1,2\n"));
    let back = read(&text).expect("the plan reads");
    assert!(back.keeps_plan(0) && !back.keeps_plan(1) && !back.keeps_plan(2));
    assert_eq!(back.to_text().as_str(), text);
    assert!(read(&with("unkept 1,x")).is_none());
}

fn many(n: usize) -> Shape {
    let mut s = shape();
    s.seeds = (0..n)
        .map(|i| Seed {
            name: Text::of(&format!("s{i}")),
            here: true,
            shares: false,
            passphrase: false,
        })
        .collect();
    s.m = n;
    s.keys = n;
    s
}

#[test]
fn a_plan_of_65_seeds_is_refused_when_read() {
    let s = many(65);
    let text = String::from(&*Answers::defaults(&s).to_text());
    assert!(Answers::from_text(&s, &text).is_none());
}

/// A plan read from a vault file is written in a form that reads again.
#[test]
fn a_plan_read_is_saved_in_a_form_that_opens_again() {
    let shape = Shape {
        seeds: vec![Seed {
            name: Text::of("a"),
            here: true,
            shares: false,
            passphrase: true,
        }],
        ..shape()
    };
    let mut plan = Answers::defaults(&shape);
    plan.set_places(&shape, 1);
    let saved = String::from(&*plan.to_text());
    assert!(Answers::from_text(&shape, &saved).is_some());
    let pass_line = saved
        .lines()
        .find(|l| l.starts_with("pass "))
        .expect("a passphrase seed here has its places");
    // Its places row emptied is not one this wallet's plan writes: refused,
    // not read and then saved as a plan that no longer opens.
    let emptied = saved.replace(pass_line, "pass 0 ");
    assert!(Answers::from_text(&shape, &emptied).is_none());
}

// ---------------------------------------------------------------------
// The text file written in the picture's place: the item's own mark
// ---------------------------------------------------------------------

mod written {
    use super::*;
    use faraday_crypto::work::Compute;
    use faraday_plan::{
        At, Edit, Spot, Tag, What, edited, form, map, stored_map, wallet, with_added,
    };

    /// A silent payments wallet's shape: the one whose plan carries marks.
    fn silent_shape() -> Shape {
        Shape {
            silent: true,
            ..shape()
        }
    }

    fn read(text: &str) -> Option<Answers> {
        Answers::from_text(&silent_shape(), text)
    }

    fn text_shape() -> Shape {
        Shape {
            text_files: true,
            ..silent_shape()
        }
    }

    fn said_shape() -> Shape {
        Shape {
            said: true,
            ..silent_shape()
        }
    }

    #[expect(clippy::disallowed_methods, reason = "a mint: the plan text's tests")]
    fn c() -> Compute {
        Compute::new(|_| {})
    }

    fn picture() -> What {
        What::WalletFile(form::QR)
    }

    fn spot(at: At, holds: &[What]) -> Spot {
        Spot {
            at,
            holds: holds.iter().map(|&w| (w, Tag::Public)).collect(),
        }
    }

    /// A plan of `places` places, each with a stick of the picture, and
    /// the picture's form on.
    fn plan(places: usize) -> Answers {
        let mut a = Answers::defaults(&shape());
        let mut m = Vec::new();
        for p in 0..places {
            m.push(spot(At::Place(p), &[]));
            m.push(spot(At::Stick(p, 0), &[picture()]));
        }
        a.set_map(m);
        a
    }

    fn marked(a: &Answers, at: At) -> bool {
        a.written_on(at)
    }

    fn saved_map(a: &Answers) -> Vec<Spot> {
        stored_map(&shape(), a)
    }

    fn after(a: &mut Answers, edit: Edit) {
        let m = edited(&saved_map(a), edit, &[]);
        a.set_map(m);
    }

    fn holds(a: &Answers, shape: &Shape, at: At) -> Vec<What> {
        map(&c(), shape, a)
            .into_iter()
            .filter(|s| s.at == at)
            .flat_map(|s| s.holds.into_iter().map(|h| h.0))
            .collect()
    }

    #[test]
    fn a_marked_plan_round_trips_and_an_unmarked_plans_text_is_as_before() {
        let mut a = plan(2);
        let before = String::from(&*a.to_text());
        assert!(!before.contains("+txt"));
        a.wrote(&text_shape());
        let text = String::from(&*a.to_text());
        // Both sticks carry the mark; nothing else of the text changes.
        assert_eq!(text.matches("walletqr+txt").count(), 2);
        assert_eq!(text.replace("+txt", ""), before);
        let back = read(&text).expect("the marked plan reads");
        assert!(marked(&back, At::Stick(0, 0)) && marked(&back, At::Stick(1, 0)));
        assert_eq!(String::from(&*back.to_text()), text);
    }

    #[test]
    fn a_mark_anywhere_but_a_stick_picture_or_the_softwares_description_is_refused() {
        let mut a = plan(1);
        a.wrote(&text_shape());
        let good = String::from(&*a.to_text());
        assert!(read(&good).is_some());
        // The same mark on the picture's place, on a text file, on a bare
        // suffix and on a name the plan does not have.
        for (from, to) in [
            ("walletqr+txt", "wallettxt+txt"),
            ("walletqr+txt", "sheet+txt"),
            ("walletqr+txt", "walletfile+txt"),
            ("walletqr+txt", "+txt"),
            ("walletqr+txt", "walletqr+txt+txt"),
            ("walletqr+txt", "wallet+txt"),
        ] {
            assert!(read(&good.replace(from, to)).is_none(), "{to}");
        }
        let on_place = good.replace("spot stick0.0 walletqr+txt", "spot place0 walletqr+txt");
        assert_ne!(on_place, good);
        assert!(read(&on_place).is_none());
        // The software's description and a stick of files take it; a
        // sheet on the files stick does not.
        let soft = with("spot software wallet+txt");
        assert!(read(&soft).is_some());
        assert!(read(&with("spot software sheet+txt")).is_none());
        assert!(read(&with("spot files wallet+txt")).is_some());
        assert!(read(&with("spot files walletqr+txt")).is_none());
    }

    #[test]
    fn a_mark_in_the_plan_of_a_wallet_that_is_not_silent_is_refused() {
        let mut a = plan(1);
        a.wrote(&text_shape());
        let marked_text = String::from(&*a.to_text());
        assert!(read(&marked_text).is_some());
        assert!(Answers::from_text(&shape(), &marked_text).is_none());
        let soft = with("spot software wallet+txt");
        assert!(read(&soft).is_some());
        assert!(Answers::from_text(&shape(), &soft).is_none());
    }

    #[test]
    fn an_older_record_reads_the_picture() {
        let a = plan(1);
        let old = String::from(&*a.to_text());
        let back = read(&old).expect("reads");
        assert!(!back.written_text());
        assert_eq!(holds(&back, &said_shape(), At::Stick(0, 0)), [picture()]);
    }

    #[test]
    fn said_reads_the_text_file_on_the_marked_stick_and_made_reads_the_picture() {
        let mut a = plan(2);
        a.wrote(&text_shape());
        // The second stick's text file is cleared by a later lock, the
        // first stays: only the first is marked.
        let mut m = saved_map(&a);
        m[3].holds[0].1 = Tag::Public;
        a.set_map(m);
        let text = What::WalletFile(form::TEXT);
        assert_eq!(holds(&a, &said_shape(), At::Stick(0, 0)), [text]);
        assert_eq!(holds(&a, &said_shape(), At::Stick(1, 0)), [picture()]);
        // Made: the chosen form alone decides, marks or not.
        assert_eq!(holds(&a, &shape(), At::Stick(0, 0)), [picture()]);
        assert_eq!(holds(&a, &shape(), At::Stick(1, 0)), [picture()]);
        // The plan's map never shows the mark.
        assert!(
            map(&c(), &said_shape(), &a)
                .iter()
                .all(|s| s.holds.iter().all(|h| h.1 != Tag::PublicText))
        );
    }

    #[test]
    fn the_mark_follows_its_item_through_the_edits_that_move_it() {
        // An earlier place removed: the later stick is stick 0 of place 0.
        let mut a = plan(2);
        a.wrote(&text_shape());
        let mut m = saved_map(&a);
        m[1].holds[0].1 = Tag::Public;
        a.set_map(m);
        assert!(marked(&a, At::Stick(1, 0)) && !marked(&a, At::Stick(0, 0)));
        let mut removed = a.clone();
        after(&mut removed, Edit::RemovePlace(0));
        assert!(marked(&removed, At::Stick(0, 0)));
        // An earlier stick cleared.
        let mut cleared = plan(1);
        let mut m = saved_map(&cleared);
        m.insert(2, spot(At::Stick(0, 1), &[picture()]));
        cleared.set_map(m);
        cleared.wrote(&text_shape());
        after(&mut cleared, Edit::Clear(At::Stick(0, 0)));
        assert!(marked(&cleared, At::Stick(0, 0)));
        assert!(!marked(&cleared, At::Stick(0, 1)));
        // A vault's stick put before the marked stick, and taken off.
        let mut vault = plan(1);
        let mut m = saved_map(&vault);
        m.insert(1, spot(At::Stick(0, 0), &[What::VaultStick(0)]));
        m[2].at = At::Stick(0, 1);
        vault.set_map(m);
        vault.wrote(&text_shape());
        assert!(marked(&vault, At::Stick(0, 1)));
        after(&mut vault, Edit::Drop(At::Stick(0, 0), What::VaultStick(0)));
        assert!(marked(&vault, At::Stick(0, 0)));
        // A file added on an earlier stick.
        let mut file = plan(1);
        let mut m = saved_map(&file);
        m.insert(1, spot(At::Stick(0, 0), &[What::seed_file(0)]));
        m[2].at = At::Stick(0, 1);
        file.set_map(m);
        file.wrote(&text_shape());
        after(&mut file, Edit::Add(At::Stick(0, 0), What::Passphrase(0)));
        assert!(file.written_text());
        assert!(
            saved_map(&file)
                .iter()
                .any(|s| s.holds.contains(&(picture(), Tag::PublicText)))
        );
    }

    #[test]
    fn the_mark_goes_with_the_item_dropped_cleared_or_moved() {
        let marked_plan = || {
            let mut a = plan(2);
            a.wrote(&text_shape());
            a
        };
        let mut dropped = marked_plan();
        after(&mut dropped, Edit::Drop(At::Stick(0, 0), picture()));
        assert!(!marked(&dropped, At::Stick(0, 0)));
        assert!(marked(&dropped, At::Stick(1, 0)));
        let mut cleared = marked_plan();
        after(&mut cleared, Edit::Clear(At::Stick(0, 0)));
        assert!(!marked(&cleared, At::Stick(0, 0)) && marked(&cleared, At::Stick(1, 0)));
        // Moved to another stick: nothing is written there yet.
        let mut moved = plan(2);
        moved.wrote(&text_shape());
        after(
            &mut moved,
            Edit::Move(At::Stick(0, 0), picture(), At::Stick(1, 1)),
        );
        assert!(
            saved_map(&moved)
                .iter()
                .any(|s| s.at == At::Stick(1, 1) && s.holds == [(picture(), Tag::Public)])
        );
        assert!(marked(&moved, At::Stick(1, 0)));
        // Put back after it was dropped: a new item, the picture.
        let mut again = marked_plan();
        after(&mut again, Edit::Drop(At::Stick(1, 0), picture()));
        let m = with_added(&saved_map(&again), At::Stick(1, 0), picture());
        again.set_map(m);
        assert!(!marked(&again, At::Stick(1, 0)));
    }

    #[test]
    fn a_lock_that_makes_the_picture_clears_the_mark_and_an_ordinary_plan_is_unchanged() {
        let mut a = plan(2);
        a.wrote(&text_shape());
        assert!(a.written_text());
        a.wrote(&shape());
        assert!(!a.written_text());
        // An ordinary wallet's plan, with a map and without: the same bytes.
        let mut with_map = plan(2);
        let was = String::from(&*with_map.to_text());
        with_map.wrote(&shape());
        assert_eq!(String::from(&*with_map.to_text()), was);
        let mut none = Answers::defaults(&shape());
        let was = String::from(&*none.to_text());
        none.wrote(&shape());
        assert_eq!(String::from(&*none.to_text()), was);
    }

    #[test]
    fn an_old_software_plan_without_a_map_is_marked_and_its_plan_is_the_same() {
        let mut a = Answers::defaults(&shape());
        a.wallet[wallet::SOFTWARE] = true;
        a.wallet[wallet::FILES] = false;
        a.form = [false, false];
        assert!(a.map.is_none());
        let strip = |m: Vec<Spot>| -> Vec<Spot> {
            m.into_iter()
                .map(|mut s| {
                    for h in &mut s.holds {
                        h.1 = if h.1 == Tag::PublicText {
                            Tag::Public
                        } else {
                            h.1
                        };
                    }
                    s
                })
                .collect()
        };
        let before = saved_map(&a);
        let made_before = map(&c(), &shape(), &a);
        a.wrote(&text_shape());
        assert!(a.map.is_some());
        assert!(marked(&a, At::Software));
        assert_eq!(strip(saved_map(&a)), before);
        assert_eq!(map(&c(), &shape(), &a), made_before);
        // Said, the software reads the text file; the record reads back.
        assert!(a.form_in(&said_shape())[form::TEXT]);
        assert!(!a.form_in(&shape())[form::TEXT]);
        let back = read(&String::from(&*a.to_text())).expect("reads");
        assert!(marked(&back, At::Software));
    }
}
