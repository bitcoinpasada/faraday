//! The dice5 tour: the codex32 five-dice roller of Faraday Demo
//! (`docs/CODEX32.md` §5.5) drawn from its state, alone on the page:
//! at rest, mid-tumble, landed with each die's two rolls side by side
//! and an equal pair struck, and twenty-faced dice in other colours. Its
//! controls are the step-through's, in the codex32 entry's Demo only card
//! (the roller tour).

use crate::tour::Tour;
use faraday_core::codex32::{DiceChoice, DieColour, DieRow};
use faraday_core::testkit::five::{Act, FiveRoller, Reach, Where};
use faraday_core::testkit::five_screen::{Pairs, preview};

/// Fresh answers that give `faces` in order.
fn answer(faces: &[u8]) -> impl FnMut() -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    for (slot, f) in out.iter_mut().rev().zip(faces) {
        *slot = f - 1;
    }
    move || Some(out)
}

fn shot(t: &mut Tour, name: &str, r: &FiveRoller, p: &Pairs) -> Result<(), String> {
    let (w, h) = t.size();
    let (w, h) = (w.min(520), h.min(560));
    let rgba = preview(r, p, w, h, t.dpi);
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

/// No roll yet, of `choice`'s dice.
fn empty(choice: DiceChoice) -> Pairs<'static> {
    Pairs {
        first: [None; 5],
        second: [None; 5],
        struck: &[],
        colours: choice.colours,
        faces: choice.faces,
        masked: false,
    }
}

pub(crate) fn dice_five_tour(t: &mut Tour) -> Result<(), String> {
    let at = Where {
        seq: &[],
        done: 0,
        share_left: 26,
        all_left: 52,
    };
    let six = DiceChoice::default();
    let mut r = FiveRoller::default();
    shot(t, "rest", &r, &empty(six))?;
    r.act(Act::Reach(Reach::Character), &at, 0);
    // Part-way through the first tumble, then once its pass has landed.
    let mut draw = answer(&[1, 2, 3, 4, 5, 2, 3, 4, 5, 6]);
    r.act(Act::Roll, &at, 0);
    let _ = r.tick(0, false, &at, &mut draw);
    let _ = r.tick(120, false, &at, &mut draw);
    let _ = r.tick(300, false, &at, &mut draw);
    shot(t, "tumbling", &r, &empty(six))?;
    let _ = r.tick(700, false, &at, &mut draw);
    r.stop();
    let first = Pairs {
        first: [Some(1), Some(2), Some(3), Some(4), Some(5)],
        ..empty(six)
    };
    shot(t, "landed", &r, &first)?;
    // Both rolls in: die 3's pair came out equal, struck, and is being
    // rolled again.
    let struck = [DieRow {
        die: 2,
        first: 3,
        second: 3,
    }];
    let both = Pairs {
        first: [Some(1), Some(2), None, Some(4), Some(5)],
        second: [Some(2), Some(1), None, Some(6), Some(3)],
        struck: &struck,
        ..empty(six)
    };
    shot(t, "pairs", &r, &both)?;
    // Twenty-faced dice, chosen in other colours.
    let d20 = DiceChoice {
        faces: 20,
        colours: [
            DieColour::Black,
            DieColour::White,
            DieColour::Orange,
            DieColour::Blue,
            DieColour::Green,
        ],
    };
    let mut r20 = FiveRoller::default();
    r20.set_faces(20);
    let twenty = Pairs {
        first: [Some(17), Some(3), Some(20), Some(9), Some(12)],
        second: [Some(4), Some(15), Some(1), Some(11), Some(19)],
        ..empty(d20)
    };
    shot(t, "d20", &r20, &twenty)
}
