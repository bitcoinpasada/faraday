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

/// The finder pattern of ISO/IEC 18004 §6.3.3 with its separator: 8 by 8,
/// `#` dark, as the standard's figure draws it at a code's top left.
const FINDER: [&str; 8] = [
    "#######.", "#.....#.", "#.###.#.", "#.###.#.", "#.###.#.", "#.....#.", "#######.", "........",
];

/// The alignment pattern of ISO/IEC 18004 §6.3.5, 5 by 5.
const ALIGNMENT: [&str; 5] = ["#####", "#...#", "#.#.#", "#...#", "#####"];

/// The centre of the one alignment pattern of versions 2 to 5 (ISO/IEC
/// 18004 Annex E), by side.
fn alignment_centre(side: usize) -> usize {
    match side {
        25 => 18,
        29 => 22,
        33 => 26,
        _ => 30,
    }
}

/// The squares of a code of `side` that are dark and fixed, as the
/// standard's figures put them (not the format areas, which depend on the
/// mask).
fn iso_dark(side: usize) -> std::collections::BTreeSet<(usize, usize)> {
    let mut dark = std::collections::BTreeSet::new();
    // Timing: row 6 and column 6, dark on even squares.
    for i in (8..side - 8).step_by(2) {
        dark.insert((i, 6));
        dark.insert((6, i));
    }
    for (i, row) in FINDER.iter().enumerate() {
        for (j, c) in row.bytes().enumerate() {
            if c == b'#' {
                dark.insert((j, i));
                dark.insert((side - 1 - j, i));
                dark.insert((j, side - 1 - i));
            }
        }
    }
    let c = alignment_centre(side);
    for (i, row) in ALIGNMENT.iter().enumerate() {
        for (j, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                dark.insert((c - 2 + j, c - 2 + i));
            }
        }
    }
    dark
}

/// The squares the sheet's PDF fills black, as grid columns and rows.
fn blackened(pdf: &str) -> std::collections::BTreeSet<(usize, usize)> {
    let mut fills = Vec::new();
    for line in pdf.lines() {
        let Some(rest) = line.strip_prefix("0.00 0.00 0.00 rg ") else {
            continue;
        };
        let n: Vec<f32> = rest
            .trim_end_matches(" re f")
            .split(' ')
            .filter_map(|v| v.parse().ok())
            .collect();
        fills.push((n[0], n[1], n[2]));
    }
    // The top left square is dark, and the top row has the largest y.
    let cell = fills[0].2;
    let x0 = fills.iter().map(|f| f.0).fold(f32::MAX, f32::min);
    let y0 = fills.iter().map(|f| f.1).fold(f32::MIN, f32::max);
    fills
        .iter()
        .map(|f| {
            (
                ((f.0 - x0) / cell).round() as usize,
                ((y0 - f.1) / cell).round() as usize,
            )
        })
        .collect()
}

#[test]
fn each_size_prints_the_dark_squares_of_a_real_code_and_no_others() {
    for n in [25usize, 29, 33, 37] {
        let mut s = starter();
        choose(&mut s, Some(n));
        let (_, pdf) = saved(&mut s);
        let printed = blackened(&pdf);
        let expected = iso_dark(n);
        let extra: Vec<_> = printed.difference(&expected).collect();
        let missing: Vec<_> = expected.difference(&printed).collect();
        assert!(extra.is_empty(), "{n}: blackened but light: {extra:?}");
        assert!(
            missing.is_empty(),
            "{n}: dark but not blackened: {missing:?}"
        );
    }
}
