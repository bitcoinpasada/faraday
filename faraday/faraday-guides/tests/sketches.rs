//! The maker pages' firmware sketches (`docs/HUB.md` §7.8, F18): every
//! marker on a page draws a sketch, each sketch shows the person which
//! row to look for, and none names a maker or a model.

use faraday_guides::{SKETCHES, ids, page, sketch_of};
use faraday_learn::sketch::{Mark, Row, Sketch};
use faraday_learn::{MAKERS, is_marker, step};

/// Every marker paragraph, with the page it is on.
fn markers() -> Vec<(&'static str, &'static str)> {
    ids()
        .flat_map(|id| {
            let p = page(id).expect("a page for every id");
            p.sections
                .iter()
                .flat_map(|s| s.paragraphs.iter())
                .filter(|para| is_marker(para))
                .map(move |para| (id, *para))
        })
        .collect()
}

fn rows(s: &Sketch) -> impl Iterator<Item = &Row> {
    s.rows
        .iter()
        .chain(s.dialog.iter().flat_map(|d| d.rows.iter()))
}

#[test]
fn every_marker_on_a_page_names_a_sketch() {
    let found = markers();
    assert!(!found.is_empty());
    for (id, para) in found {
        assert!(sketch_of(para).is_some(), "{id}: {para}");
    }
}

#[test]
fn every_sketch_is_on_a_page() {
    let used: Vec<&str> = markers()
        .into_iter()
        .filter_map(|(_, para)| sketch_of(para).map(|s| s.id))
        .collect();
    for s in SKETCHES {
        assert!(used.contains(&s.id), "{} is on no page", s.id);
    }
}

#[test]
fn every_sketch_marks_the_row_to_look_for() {
    for s in SKETCHES {
        assert!(
            rows(s).any(|r| matches!(r.mark, Mark::Target | Mark::Change(_))),
            "{}",
            s.id
        );
    }
}

#[test]
fn every_maker_page_shows_where_secure_boot_is_turned_off() {
    // The step that turns Secure Boot off, the first that names it, has a
    // sketch among what is under it, up to the next step.
    for m in MAKERS {
        let p = page(m.id).expect("a maker page");
        let paras: Vec<&str> = p
            .sections
            .iter()
            .flat_map(|s| s.paragraphs.iter().copied())
            .collect();
        let at = paras
            .iter()
            .position(|para| step(para).is_some_and(|(_, t)| t.contains("Secure Boot")))
            .unwrap_or_else(|| panic!("{}: no step turns Secure Boot off", m.id));
        let under = paras[at + 1..]
            .iter()
            .take_while(|para| step(para).is_none());
        assert!(
            under.clone().any(|para| sketch_of(para).is_some()),
            "{}",
            m.id
        );
    }
}

#[test]
fn every_sketch_is_under_a_step_or_what_is_different() {
    // A marker follows a step, or a paragraph under one, never a page's
    // opening or its sources.
    for id in ids() {
        let p = page(id).expect("a page for every id");
        for s in p.sections {
            let has = s.paragraphs.iter().any(|para| is_marker(para));
            if has {
                assert!(
                    matches!(
                        s.heading,
                        "Steps" | "Afterwards" | "If something is different"
                    ),
                    "{id}: a sketch under {}",
                    s.heading
                );
                assert!(!is_marker(s.paragraphs[0]), "{id}: {}", s.heading);
            }
        }
    }
}

#[test]
fn no_sketch_names_a_maker_a_model_or_a_firmware_maker() {
    const NAMES: [&str; 16] = [
        "Dell",
        "HP",
        "Lenovo",
        "ThinkPad",
        "IdeaPad",
        "Yoga",
        "Legion",
        "ASUS",
        "ROG",
        "Acer",
        "MSI",
        "Surface",
        "Framework",
        "Insyde",
        "Aptio",
        "Kingston",
    ];
    for s in SKETCHES {
        let mut words: Vec<&str> = vec![s.title, s.heading, s.text];
        words.extend(s.items.iter().copied());
        words.extend(s.keys.iter().copied());
        for r in rows(s) {
            words.extend([r.label, r.value]);
            if let Mark::Change(to) = r.mark {
                words.push(to);
            }
        }
        if let Some(d) = s.dialog {
            words.extend([d.title, d.text]);
            words.extend(d.buttons.iter().copied());
        }
        for text in words {
            for word in text.split(|c: char| !c.is_ascii_alphanumeric()) {
                assert!(!NAMES.contains(&word), "{}: {text}", s.id);
            }
        }
    }
}

#[test]
fn a_paragraph_that_is_not_a_marker_is_text() {
    assert!(sketch_of("Press F10 to save and exit.").is_none());
    assert!(!is_marker("Press F10 to save and exit."));
    assert!(is_marker("::sketch no-such-sketch"));
    assert!(sketch_of("::sketch no-such-sketch").is_none());
}
