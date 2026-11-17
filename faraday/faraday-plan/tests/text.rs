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
