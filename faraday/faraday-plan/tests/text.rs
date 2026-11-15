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
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
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

#[test]
fn a_plan_missing_a_line_is_refused() {
    let text = saved();
    for name in ["seeds", "wallet", "software", "form", "omit", "split", "sticks", "vault"] {
        let cut: String = text
            .lines()
            .filter(|l| l.split(' ').next() != Some(name))
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(read(&cut).is_none(), "without {name}");
    }
    assert!(read("").is_none());
}
