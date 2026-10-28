//! Blank templates (`docs/STARTER.md` §3.8, batch T): what a person sees and
//! gets from the Blank templates page.

use faraday_core::pdf;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{Reason, StarterCommand, StarterEvent};
use faraday_starter::template::Copy;
use faraday_starter::{Page, Starter, StarterAction, TemplateChoice};

/// A token for the PDFs these tests make, counting nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's tests, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

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

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// Chooses on the Blank templates page, as a person taps through it.
fn choose(s: &mut Starter, words: usize, copy: Copy, places: usize) {
    s.press(StarterAction::Templates);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Words));
    s.press(StarterAction::TemplateWords(words));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Copy));
    s.press(StarterAction::TemplateCopy(copy));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Places));
    while s.template.places < places {
        s.press(StarterAction::PlacesMore);
    }
    s.press(StarterAction::Back);
    assert_eq!(s.page, Page::Templates);
}

fn texts(s: &mut Starter) -> String {
    s.drawn_texts().join("\n").to_lowercase()
}

#[test]
fn each_choice_of_words_and_copy_saves_its_pdf_under_its_name() {
    let cases = [
        (12, Copy::Words, "words", "12-words-words"),
        (12, Copy::SeedQr, "seedqr", "12-words-seedqr"),
        (12, Copy::Both, "words seedqr", "12-words-words-seedqr"),
        (24, Copy::Words, "words", "24-words-words"),
        (24, Copy::SeedQr, "seedqr", "24-words-seedqr"),
        (24, Copy::Both, "words seedqr", "24-words-words-seedqr"),
    ];
    for (words, copy, line, name) in cases {
        let mut s = starter();
        choose(&mut s, words, copy, 3);
        s.press(StarterAction::SaveTemplate);
        let c = commands(&mut s);
        let [
            StarterCommand::SaveTemplate {
                name: got,
                pdf: bytes,
            },
        ] = c.as_slice()
        else {
            panic!("one save expected");
        };
        assert_eq!(got, &format!("faraday-blank-template-{name}.pdf"));
        let sheet = s.template.sheet();
        assert!(sheet.contains(&format!("\nwords: {words}\n")), "{sheet}");
        assert!(sheet.contains(&format!("\ncopy: {line}\n")), "{sheet}");
        assert!(sheet.contains("\nplaces: 3\n"), "{sheet}");
        assert!(sheet.contains("\nnetwork: mainnet\n"), "{sheet}");
        assert!(
            !sheet.contains("name:") && !sheet.contains("shape:"),
            "{sheet}"
        );
        assert_eq!(bytes, &pdf::sheet(&compute(), &sheet).expect("a PDF"));
        assert!(bytes.starts_with(b"%PDF"));
    }
}

#[test]
fn the_defaults_are_24_words_copied_as_words_in_2_places() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    let page = texts(&mut s);
    assert!(page.contains("24") && page.contains("words") && page.contains("2"));
    s.press(StarterAction::SaveTemplate);
    let c = commands(&mut s);
    let [StarterCommand::SaveTemplate { name, .. }] = c.as_slice() else {
        panic!("one save expected");
    };
    assert_eq!(name, "faraday-blank-template-24-words-words.pdf");
    assert!(s.template.sheet().contains("\nplaces: 2\n"));
}

#[test]
fn places_are_one_up_to_what_the_page_holds_and_each_is_printed() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Places));
    for _ in 0..5 {
        s.press(StarterAction::PlacesFewer);
    }
    assert_eq!(s.template.places, 1);
    for _ in 0..40 {
        s.press(StarterAction::PlacesMore);
    }
    let most = s.template.places;
    assert!(most > 1 && most < 41, "{most}");
    let _ = s.drawn_texts();
    assert!(!s.offers_action(StarterAction::PlacesMore), "More is off");
    // Every place counted has its line on the PDF.
    let pdf = pdf::sheet(&compute(), &s.template.sheet()).expect("a PDF");
    let lines = pdf.windows(9).filter(|w| *w == b"(Place) T").count();
    assert_eq!(lines, most);
}

#[test]
fn print_makes_the_same_pdf_as_save_and_asks_the_shell_to_open_it() {
    let mut s = starter();
    choose(&mut s, 12, Copy::Both, 2);
    s.press(StarterAction::SaveTemplate);
    s.press(StarterAction::PrintTemplate);
    let c = commands(&mut s);
    let [
        StarterCommand::SaveTemplate {
            name: a,
            pdf: saved,
        },
        StarterCommand::PrintTemplate {
            name: b,
            pdf: printed,
        },
    ] = c.as_slice()
    else {
        panic!("{c:?}");
    };
    assert_eq!(a, b);
    assert_eq!(saved, printed);
}

#[test]
fn after_a_save_the_path_is_said_and_open_it_asks_the_shell_to_open_that_file() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    s.press(StarterAction::SaveTemplate);
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::TemplateSaved(Ok("/dl/blank (2).pdf".into())));
    assert!(texts(&mut s).contains("saved /dl/blank (2).pdf"));
    s.press(StarterAction::OpenTemplate);
    assert_eq!(
        commands(&mut s),
        vec![StarterCommand::OpenTemplate {
            path: "/dl/blank (2).pdf".into()
        }]
    );
    s.starter_event(StarterEvent::TemplateOpened(Err(Reason::Missing(
        "xdg-open".into(),
    ))));
    assert!(texts(&mut s).contains("xdg-open"));
}

#[test]
fn a_failed_save_says_why_and_offers_no_open() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    s.press(StarterAction::SaveTemplate);
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::TemplateSaved(Err(Reason::File {
        path: "/dl".into(),
        detail: "disk full".into(),
    })));
    assert!(texts(&mut s).contains("disk full"));
    s.press(StarterAction::OpenTemplate);
    assert!(commands(&mut s).is_empty());
}

#[test]
fn nothing_asks_for_or_shows_a_wallet_key_or_seed() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    let mut all = texts(&mut s);
    for choice in [
        TemplateChoice::Words,
        TemplateChoice::Copy,
        TemplateChoice::Places,
    ] {
        s.press(StarterAction::TemplateChoice(choice));
        all.push_str(&texts(&mut s));
        s.press(StarterAction::Back);
    }
    for word in ["wallet", "key", "passphrase", "recovery", "enter", "type "] {
        assert!(!all.contains(word), "{word} in {all}");
    }
    // No entropy, no typing field: only the commands for the file.
    s.press(StarterAction::SaveTemplate);
    s.press(StarterAction::PrintTemplate);
    for c in commands(&mut s) {
        assert!(matches!(
            c,
            StarterCommand::SaveTemplate { .. } | StarterCommand::PrintTemplate { .. }
        ));
    }
    let sheet = s.template.sheet();
    for field in ["descriptor", "key:", "address:", "name:", "shape:"] {
        assert!(!sheet.contains(field), "{sheet}");
    }
}
