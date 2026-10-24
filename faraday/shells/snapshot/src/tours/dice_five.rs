//! The dice5 tour: the codex32 five-dice roller of Faraday Desktop
//! (`docs/CODEX32.md` §5.5) drawn from its state, alone on the page:
//! at rest, each choice, mid-tumble, landed, and the roll disabled where
//! nothing is left to enter.

use crate::tour::Tour;
use faraday_core::testkit::five::{Act, FiveRoller, Reach, Where};
use faraday_core::testkit::five_screen::preview;

/// Fresh answers that give `faces` in order.
fn answer(faces: &[u8]) -> impl FnMut() -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    for (slot, f) in out.iter_mut().rev().zip(faces) {
        *slot = f - 1;
    }
    move || Some(out)
}

fn shot(t: &mut Tour, name: &str, r: &FiveRoller, can_roll: bool) -> Result<(), String> {
    let (w, h) = t.size();
    let (w, h) = (w.min(520), h.min(420));
    let rgba = preview(r, can_roll, w, h, t.dpi);
    t.n += 1;
    let path = t.out.join(format!("{:02}-{name}.png", t.n));
    let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), u32::from(w), u32::from(h));
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut out = enc.write_header().map_err(|e| e.to_string())?;
    out.write_image_data(&rgba).map_err(|e| e.to_string())?;
    println!("{}", path.display());
    Ok(())
}

pub(crate) fn dice_five_tour(t: &mut Tour) -> Result<(), String> {
    let at = Where {
        seq: &[],
        done: 0,
        share_left: 26,
        all_left: 52,
    };
    let mut r = FiveRoller::default();
    shot(t, "rest", &r, true)?;
    for reach in [Reach::Pass, Reach::Share, Reach::End] {
        r.act(Act::Reach(reach), &at, 0);
        shot(t, "choice", &r, true)?;
    }
    r.act(Act::Reach(Reach::Character), &at, 0);
    // Part-way through the first tumble, then once its pass has landed.
    let mut draw = answer(&[1, 2, 3, 4, 5, 2, 3, 4, 5, 6]);
    r.act(Act::Roll, &at, 0);
    let _ = r.tick(0, false, &at, &mut draw);
    let _ = r.tick(120, false, &at, &mut draw);
    let _ = r.tick(300, false, &at, &mut draw);
    shot(t, "tumbling", &r, true)?;
    let _ = r.tick(700, false, &at, &mut draw);
    r.stop();
    shot(t, "landed", &r, true)?;
    shot(t, "nothing-left", &FiveRoller::default(), false)
}
