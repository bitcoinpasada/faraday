//! The blank sheets the Starter prints: the template a seed is copied onto by hand, and the
//! squares of a Codex32QR the sheet prints already. They hold no secret and nothing of a plan,
//! and may go to any printer. Moved here from `faraday-core`'s `wallet_backup` and
//! `codex32_backup` modules (`docs/spec/09-manifest.md` §2.1); [`crate::pdf::sheet`] reads what
//! [`sheet_blank`] writes.

use core::fmt::Write as _;

/// What the blank template has to copy a seed onto: the word lines, the
/// SeedQR grid, or both (`docs/WALLETS.md` §13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankCopy {
    /// The word lines.
    Words,
    /// The SeedQR grid.
    SeedQr,
    /// Both on one sheet.
    Both,
}

impl BlankCopy {
    /// Every choice, in the order a Choice lists them.
    pub const ALL: [BlankCopy; 3] = [BlankCopy::Words, BlankCopy::SeedQr, BlankCopy::Both];

    /// The sheet file's `copy:` value.
    pub fn line(self) -> String {
        match self {
            BlankCopy::Words => String::from("words"),
            BlankCopy::SeedQr => String::from("seedqr"),
            BlankCopy::Both => format!("{} {}", BlankCopy::Words.line(), BlankCopy::SeedQr.line()),
        }
    }
}

/// What a wallet adds to the blank template, as the caller worked it out: the name as the sheet
/// prints it, its shape in a few words, and the path each of its keys is taken at (without the
/// leading `m/`, a path in the order of the keys, repeats allowed).
#[derive(Debug, Clone, Copy)]
pub struct BlankWallet<'a> {
    /// The wallet's name, as one line.
    pub name: &'a str,
    /// The wallet's shape, in a few words.
    pub shape: &'a str,
    /// The path of each key, which the seed alone does not say.
    pub paths: &'a [&'a str],
}

/// The blank template: numbered word lines, the SeedQR grid's fixed
/// squares, or both as `copy` says, for a seed of `words` words, and a
/// page of `places` empty lines, "Place ____ holds ____", to fill in by
/// hand. It holds no secret and nothing of the plan, and may go to any
/// printer. `wallet` adds the path, script and network to fill in;
/// `network` is its name (`mainnet`, `testnet`, `signet` or `regtest`),
/// written in lower case. Each value is written on its own line: a line
/// break or other control character in it is written as a space, so no
/// value adds a line of its own to the sheet.
pub fn sheet_blank(
    words: usize,
    copy: BlankCopy,
    wallet: Option<BlankWallet<'_>>,
    network: &str,
    places: usize,
) -> String {
    let mut out = format!(
        "faraday-sheet 1\nkind: blank\nwords: {words}\ncopy: {}\nnetwork: {}\nplaces: {places}\n",
        copy.line(),
        one_line(network).to_ascii_lowercase(),
    );
    if let Some(w) = wallet {
        let _ = writeln!(
            out,
            "name: {}\nshape: {}",
            one_line(w.name),
            one_line(w.shape)
        );
        // The path its keys are taken at, which the seed alone does not say.
        let mut paths = String::new();
        for p in w.paths {
            let p = format!("m/{}", one_line(p));
            if !paths.split(", ").any(|q| q == p) {
                if !paths.is_empty() {
                    paths.push_str(", ");
                }
                paths.push_str(&p);
            }
        }
        if !paths.is_empty() {
            let _ = writeln!(out, "path: {paths}");
        }
    }
    out
}

/// `value` as one line of the sheet file: each control character, a line break among them, as a
/// space.
fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// The side of a Codex32QR in squares: version 3 (`docs/CODEX32.md`
/// §10.1).
pub const QR_SIDE: usize = 29;

/// Whether square (x, y) of a Codex32QR is printed dark on the Starter's
/// Codex32QR sheet (`docs/CODEX32.md` §11), the patterns every version 3
/// code shares, so the person draws only the rest: the three finders, the
/// timing lines between them, the alignment square and the dark module.
/// Not the separators, which are light, nor the format areas, which
/// depend on the code's mask. The device's copy grid draws these grey,
/// as printed already.
pub fn codex32_printed(x: usize, y: usize) -> bool {
    let n = QR_SIDE;
    if x >= n || y >= n {
        return false;
    }
    if x == 8 && y == n - 8 {
        return true;
    }
    // A finder at (ox, oy): a dark ring, a light ring, a dark 3 x 3.
    for (ox, oy) in [(0, 0), (n - 7, 0), (0, n - 7)] {
        if (ox..ox + 7).contains(&x) && (oy..oy + 7).contains(&y) {
            let (dx, dy) = (x - ox, y - oy);
            let ring = dx == 0 || dx == 6 || dy == 0 || dy == 6;
            let core = (2..=4).contains(&dx) && (2..=4).contains(&dy);
            return ring || core;
        }
    }
    // The timing lines run between the finders, dark on even squares.
    if y == 6 && (8..n - 8).contains(&x) {
        return x.is_multiple_of(2);
    }
    if x == 6 && (8..n - 8).contains(&y) {
        return y.is_multiple_of(2);
    }
    // The alignment square, centred on (n - 7, n - 7).
    let (dx, dy) = (x.abs_diff(n - 7), y.abs_diff(n - 7));
    dx <= 2 && dy <= 2 && (dx == 2 || dy == 2 || (dx == 0 && dy == 0))
}
