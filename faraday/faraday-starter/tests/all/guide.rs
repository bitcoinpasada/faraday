//! The Starter's pages of the guide (`docs/GUIDE-IN-APP.md` §6): every
//! page opens a guide page that is written, and the Guide button on a task
//! page opens its sheet.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::{GUIDE_PLACES, Page, Starter, StarterAction, guide_key};

const PAGES: [Page; 23] = [
    Page::Home,
    Page::Make,
    Page::Tools,
    Page::Guide,
    Page::Which,
    Page::Disks,
    Page::Mode,
    Page::Confirm,
    Page::Writing,
    Page::Done,
    Page::Start,
    Page::Install,
    Page::Installed,
    Page::About,
    Page::Appearance,
    Page::Templates,
    Page::TemplateWords,
    Page::TemplateCopy,
    Page::TemplatePlaces,
    Page::TemplateForm,
    Page::TemplateLength,
    Page::TemplateWorksheets,
    Page::TemplateSize,
];

fn starter(width: u16, height: u16) -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width,
        height,
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

#[test]
fn every_page_opens_a_guide_page_that_is_written() {
    for page in PAGES {
        let key = guide_key(page);
        let place = key.split('.').next().unwrap();
        assert!(
            faraday_learn::guide_page(place).is_some(),
            "{page:?}: {key}"
        );
        if key.contains('.') {
            assert!(
                faraday_learn::guide_section(key).is_some(),
                "{page:?}: {key}"
            );
        }
    }
    for place in GUIDE_PLACES {
        assert!(faraday_learn::guide_page(place).is_some(), "{place}");
    }
}

#[test]
fn the_guide_destination_lists_the_starters_pages_and_a_press_opens_one() {
    let mut s = starter(1280, 800);
    s.press(StarterAction::Learn);
    let texts = s.drawn_texts();
    for place in GUIDE_PLACES {
        let title = &faraday_learn::guide_page(place).unwrap().title;
        assert!(texts.iter().any(|t| t == title), "{title}");
    }
    s.press(StarterAction::GuidePage(0));
    let title = &faraday_learn::guide_page(GUIDE_PLACES[0]).unwrap().title;
    assert!(s.drawn_texts().iter().filter(|t| *t == title).count() >= 1);
    s.press(StarterAction::Cancel);
    assert_eq!(s.page, Page::Guide);
}
