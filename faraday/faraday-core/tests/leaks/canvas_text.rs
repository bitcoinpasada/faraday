//! A passphrase drawn behind its eye and cut off at the side of its clip,
//! with the canvas recording what it draws, leaves no copy of it once the
//! canvas is cleared (what Lock does to a frame).

use crate::arm;
use crate::forms::{PASSPHRASE, seed_forms};
use faraday_draw::{Canvas, Color, Font, Rect};
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

#[test]
fn a_passphrase_drawn_cut_off_leaves_no_copy_after_lock() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let mut canvas = Canvas::new(&DisplayInfo {
        width: 480,
        height: 200,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    canvas.record_ink(true);
    canvas.clear(Color::WHITE);
    canvas.push_clip(Rect::new(0, 0, 120, 100));
    canvas.text(4, 20, PASSPHRASE, Font::regular(16.0), Color::BLACK);
    canvas.pop_clip();
    assert!(
        canvas.cut().iter().any(|t| t.as_str() == PASSPHRASE),
        "the passphrase was not drawn cut off"
    );
    canvas.record_ink(false);
    canvas.clear(Color::WHITE);
    assert!(canvas.cut().is_empty() && canvas.ink().is_empty());
    let live = armed.scan_live();
    drop(canvas);
    let report = armed.finish();
    assert!(live.is_empty(), "kept after the canvas was cleared");
    assert!(report.freed.is_empty(), "{:?}", report.freed);
}
