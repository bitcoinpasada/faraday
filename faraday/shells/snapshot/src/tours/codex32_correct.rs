//! The codex32correct tour: codex32 error correction's screens
//! (`docs/CODEX32.md` §8.4), drawn from the correction a codex32 form
//! holds, alone on the page, at the tour's size: the tick off and on, a
//! proposal's pairs and its pages, the re-entry from the paper and a
//! re-entry that differs, the repairs, the caution for unreadable boxes
//! and for a long run, a refusal, and the line beside a fingerprint.

use crate::tour::Tour;
use faraday_core::codex32_correct::{Correction, Mark, Press, screen};
use faraday_crypto::work::Compute;
use faraday_shell_api::Key;

/// BIP 93's vector 1, the secret alone.
const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";

/// `s` with each box of `at` (from 1) put to `to`.
fn with(s: &str, at: &[usize], to: u8) -> String {
    let mut b = s.as_bytes().to_vec();
    for &n in at {
        b[n - 1] = to;
    }
    String::from_utf8(b).unwrap_or_default()
}

/// Costs a decode of `text`. A ticked correction offered `text`.
fn offered(c: &Compute, text: &str) -> Correction {
    let mut k = Correction::new();
    k.press(Press::Tick);
    k.offer(c, text, None);
    k
}

fn shot(t: &mut Tour, name: &str, k: &Correction, mark: Option<Mark>) -> Result<(), String> {
    let (w, h) = t.size();
    let rgba = screen::preview(k, mark, w, h, t.dpi);
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

/// Costs a decode of each damaged string.
pub(crate) fn codex32_correct_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let mut k = Correction::new();
    shot(t, "tick-off", &k, None)?;
    k.press(Press::Tick);
    shot(t, "tick-on", &k, None)?;

    let one = offered(c, &with(SECRET, &[30], b'q'));
    shot(t, "pairs", &one, None)?;

    let mut six = offered(
        c,
        &with(&with(SECRET, &[15, 33], b'p'), &[20, 21, 25, 44], b'?'),
    );
    shot(t, "pairs-page-1", &six, None)?;
    six.press(Press::Next);
    shot(t, "pairs-page-2", &six, None)?;
    six.press(Press::Mistyped);
    six.key(Key::Char(char::from(SECRET.as_bytes()[14])));
    shot(t, "retype", &six, None)?;
    for _ in 0..5 {
        six.key(Key::Char('q'));
    }
    shot(t, "retype-differs", &six, None)?;

    let mut damaged = offered(c, &with(SECRET, &[30], b'q'));
    damaged.press(Press::Damaged);
    shot(t, "repairs", &damaged, None)?;

    let mut holes = offered(c, &with(SECRET, &[11, 27, 46], b'?'));
    shot(t, "pairs-unreadable", &holes, None)?;
    holes.press(Press::Unreadable);
    shot(t, "from-checksum", &holes, None)?;

    let run: Vec<usize> = (20..30).collect();
    let mut long = offered(c, &with(SECRET, &run, b'?'));
    shot(t, "caution", &long, None)?;
    long.press(Press::Continue);
    shot(t, "caution-pairs", &long, None)?;

    let far = offered(c, &with(SECRET, &[12, 18, 24, 30, 36, 42], b'q'));
    shot(t, "refused", &far, None)?;
    let short = offered(c, "ms1short");
    shot(t, "refused-malformed", &short, None)?;

    shot(
        t,
        "fingerprint-corrected",
        &Correction::new(),
        Some(Mark::Corrected),
    )?;
    shot(
        t,
        "fingerprint-repaired",
        &Correction::new(),
        Some(Mark::Repaired),
    )
}
