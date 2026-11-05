//! The Starter's blank templates for a Krux code (`docs/KEF.md` §15.5,
//! `docs/STARTER.md` §3.8): what a person offered the Form row gets.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::StarterCommand;
use faraday_starter::template::Form;
use faraday_starter::{Starter, StarterAction, TemplateChoice};

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s
}

/// Chooses Krux code on the Blank templates page and, if given, a size.
fn choose(s: &mut Starter, size: Option<usize>) {
    s.press(StarterAction::Templates);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Form));
    s.press(StarterAction::TemplateForm(Form::Krux));
    s.press(StarterAction::Back);
    if let Some(n) = size {
        s.press(StarterAction::TemplateChoice(TemplateChoice::Size));
        s.press(StarterAction::TemplateSize(n));
        s.press(StarterAction::Back);
    }
}

/// The name and PDF the Save button makes.
fn saved(s: &mut Starter) -> (String, String) {
    s.press(StarterAction::SaveTemplate);
    let c: Vec<_> = std::iter::from_fn(|| s.next_command()).collect();
    let [StarterCommand::SaveTemplate { name, pdf }] = c.as_slice() else {
        panic!("one save expected: {c:?}");
    };
    (name.clone(), pdf.iter().map(|&b| char::from(b)).collect())
}

#[test]
fn each_size_gives_its_file_and_a_grid_of_that_size() {
    for n in [25usize, 29, 33, 37] {
        let mut s = starter();
        choose(&mut s, Some(n));
        let (name, pdf) = saved(&mut s);
        assert_eq!(
            name,
            format!("faraday-blank-template-krux-code-{n}x{n}.pdf")
        );
        assert!(pdf.contains(&format!("{n} \u{d7} {n} squares")), "{n}");
        // The grid's blocks are named 1 to k across and A to the k-th
        // letter down; the next letter is not on the sheet.
        let k = n.div_ceil(5);
        let letter = |i: usize| char::from(b'A' + u8::try_from(i).unwrap());
        assert!(pdf.contains(&format!("({}) Tj", letter(k - 1))), "{n}");
        assert!(!pdf.contains(&format!("({}) Tj", letter(k))), "{n}");
        assert!(pdf.contains("never written on this sheet"));
        assert!(!pdf.contains("Never photographed"));
    }
}

#[test]
fn the_size_is_33_until_another_is_chosen() {
    let mut s = starter();
    choose(&mut s, None);
    assert_eq!(
        saved(&mut s).0,
        "faraday-blank-template-krux-code-33x33.pdf"
    );
}

#[test]
fn the_other_forms_keep_their_files() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    assert_eq!(saved(&mut s).0, "faraday-blank-template-24-words-words.pdf");
}
