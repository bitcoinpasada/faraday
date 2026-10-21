//! The name in a header: FARADAY alone, or with the word of the app it
//! heads after it (`docs/DECISIONS.md` F16).

use faraday_core::ui::pal::{DIM, TEXT};
use faraday_core::ui::{NameSuffix, Ui};
use faraday_draw::canvas::Canvas;
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

/// The width of the name drawn with `suffix`, and how many pixels of the
/// canvas to its right changed from the background.
fn drawn(suffix: NameSuffix) -> (f32, usize) {
    let mut canvas = Canvas::new(&DisplayInfo {
        width: 900,
        height: 80,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    let mut hits = Vec::new();
    let width = {
        let mut ui = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(faraday_draw::Color::rgb(255, 255, 255));
        ui.wordmark_named(10.0, 20.0, 40.0, TEXT, suffix)
    };
    let frame = canvas.frame();
    let alone = {
        let mut c2 = Canvas::new(&DisplayInfo {
            width: 900,
            height: 80,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        });
        let mut h2 = Vec::new();
        let mut ui = Ui::new(&mut c2, 1.0, &mut h2, None);
        ui.clear(faraday_draw::Color::rgb(255, 255, 255));
        let w = ui.wordmark_named(10.0, 20.0, 40.0, TEXT, NameSuffix::None);
        (w, 10 + w as usize)
    };
    let from = alone.1 + 2;
    let mut inked = 0;
    for y in 0..frame.height as usize {
        for x in from..frame.width as usize {
            let i = (y * frame.width as usize + x) * 4;
            if frame.rgba[i] < 250 {
                inked += 1;
            }
        }
    }
    let _ = DIM;
    (width, inked)
}

#[test]
fn every_suffix_adds_its_word_after_the_name_and_none_adds_nothing() {
    let (none_w, none_ink) = drawn(NameSuffix::None);
    assert_eq!(none_ink, 0);
    for suffix in [NameSuffix::Starter, NameSuffix::Desktop] {
        let (w, ink) = drawn(suffix);
        assert!(w > none_w * 1.5, "{suffix:?}: {w} against {none_w}");
        assert!(ink > 500, "{suffix:?} drew {ink} pixels");
    }
}
