//! A copy of a form's output (`docs/KEY-FORMS.md` §5): how the plan spells
//! it in a note and labels it on the map. The outputs a form has are the
//! registry's (`faraday_forms::Form::outputs`); this file adds only the
//! plan's own spelling and label for each, so a form added to the registry
//! adds a row here and no variant to [`What`].

use super::*;
use faraday_forms::{Medium::File, Medium::Paper, Writing as W};

/// The plan's spelling and label of one output of a form. A saved note
/// spells a copy `prefix`, the seed, then its piece (`w0`, `x0.10`,
/// `slip0.1.2`).
struct Row {
    writing: Writing,
    medium: Medium,
    /// What a note starts the copy with.
    prefix: &'static str,
    /// The map's label, with the seed's name and then the piece's numbers
    /// filled in.
    label: &'static str,
}

const fn row(writing: Writing, medium: Medium, prefix: &'static str, label: &'static str) -> Row {
    Row {
        writing,
        medium,
        prefix,
        label,
    }
}

const WORDS: [Row; 4] = [
    row(W::Words, Paper, "w", EN.plan_seed_words),
    row(W::SeedQr, Paper, "q", EN.plan_seed_seedqr),
    row(W::Words, File, "file", EN.plan_seed_file),
    row(W::SeedQr, File, "qrfile", EN.plan_seedqr_file),
];
const SHARES: [Row; 1] = [row(W::ShareWords, Paper, "slip", EN.plan_slip39_share)];
const CODEX32: [Row; 4] = [
    row(W::Codex32String, Paper, "x", EN.c32_plan_string),
    row(W::Codex32Qr, Paper, "xq", EN.c32_plan_qr),
    row(W::Codex32String, File, "xt", EN.c32_plan_file),
    row(W::Codex32Qr, File, "xp", EN.c32_plan_qr_file),
];
const PARTS: [Row; 1] = [row(W::PartWords, Paper, "xorp", EN.plan_xor_part)];
const KRUX: [Row; 2] = [
    row(W::KruxCode, Paper, "kefp", EN.plan_kef_code_paper),
    row(W::KruxCode, File, "kef", EN.plan_kef_code),
];

/// A form's rows: one for each of its registry outputs.
fn rows(form: KeyForm) -> &'static [Row] {
    match form {
        KeyForm::Words => &WORDS,
        KeyForm::Slip39 => &SHARES,
        KeyForm::Codex32 => &CODEX32,
        KeyForm::Xor => &PARTS,
        KeyForm::Krux => &KRUX,
        KeyForm::Xprv | KeyForm::HexSeed => &[],
    }
}

/// What a form's pieces are numbered by.
#[derive(Clone, Copy)]
enum Shape {
    Whole,
    Part,
    Member,
}

fn shape(form: KeyForm) -> Shape {
    match form {
        KeyForm::Words | KeyForm::Krux | KeyForm::Xprv | KeyForm::HexSeed => Shape::Whole,
        KeyForm::Codex32 | KeyForm::Xor => Shape::Part,
        KeyForm::Slip39 => Shape::Member,
    }
}

impl Written {
    fn row(self) -> Option<&'static Row> {
        rows(self.form)
            .iter()
            .find(|r| r.writing == self.writing && r.medium == self.medium)
    }

    /// The copy as a note spells it: `w0`, `xq0.10`, `slip0.1.2`.
    pub fn code(self) -> Text {
        let prefix = self.row().map_or("", |r| r.prefix);
        let i = self.seed;
        match self.piece {
            Piece::Whole => text!("{prefix}{i}"),
            Piece::Part(j) => text!("{prefix}{i}.{j}"),
            Piece::Member(g, m) => text!("{prefix}{i}.{g}.{m}"),
        }
    }

    /// A note's spelling read back: the form and output whose prefix and
    /// piece it is, if the registry lists the pair.
    pub fn read(s: &str) -> Option<Written> {
        KeyForm::ALL.into_iter().find_map(|form| {
            rows(form).iter().find_map(|r| {
                let rest = s.strip_prefix(r.prefix)?;
                let (seed, piece) = read_piece(form, rest)?;
                Written::new(form, seed, piece, r.writing, r.medium)
            })
        })
    }

    /// The map's label of the copy, with the seed's name.
    pub fn label(self, seed: &str) -> Text {
        let l = self.row().map_or("", |r| r.label);
        match self.piece {
            Piece::Whole => Text::fill(l, &[seed]),
            Piece::Part(j) => match self.form {
                KeyForm::Codex32 => Text::fill(
                    l,
                    &[
                        seed,
                        &text!("{}", char::to_ascii_uppercase(&codex32::index_char(j))),
                    ],
                ),
                KeyForm::Words
                | KeyForm::Slip39
                | KeyForm::Xor
                | KeyForm::Krux
                | KeyForm::Xprv
                | KeyForm::HexSeed => Text::fill(l, &[seed, &Text::num(j as u64 + 1)]),
            },
            Piece::Member(g, m) => Text::fill(
                l,
                &[seed, &Text::num(g as u64 + 1), &Text::num(m as u64 + 1)],
            ),
        }
    }
}

/// A spelling's seed and piece: digits, then the form's piece, each part
/// after a dot.
fn read_piece(form: KeyForm, rest: &str) -> Option<(usize, Piece)> {
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut f = rest.split('.').map(|n| n.parse::<usize>().ok());
    let seed = f.next()??;
    let piece = match shape(form) {
        Shape::Whole => Piece::Whole,
        Shape::Part => {
            let j = f.next()??;
            // A codex32 share index is one of 32 characters.
            if form == KeyForm::Codex32 && j >= 32 {
                return None;
            }
            Piece::Part(j)
        }
        Shape::Member => Piece::Member(f.next()??, f.next()??),
    };
    f.next().is_none().then_some((seed, piece))
}

/// A copy of `form` as `writing` on `medium`: shorthand for the pairs the
/// plan and its callers name most.
impl What {
    /// Seed `i`'s words, by hand.
    pub const fn words(i: usize) -> What {
        what(KeyForm::Words, i, Piece::Whole, W::Words, Paper)
    }
    /// Seed `i`'s SeedQR, by hand.
    pub const fn seed_qr(i: usize) -> What {
        what(KeyForm::Words, i, Piece::Whole, W::SeedQr, Paper)
    }
    /// Seed `i`'s words as a text file: on the stick of files, or on a
    /// stick at a place (`docs/BACKUP-FLOW.md` §17.3).
    pub const fn seed_file(i: usize) -> What {
        what(KeyForm::Words, i, Piece::Whole, W::Words, File)
    }
    /// Seed `i`'s SeedQR as a picture, on a stick at a place.
    pub const fn seed_qr_file(i: usize) -> What {
        what(KeyForm::Words, i, Piece::Whole, W::SeedQr, File)
    }
    /// Piece `j` (its share index's bech32 value) of seed `i`'s codex32
    /// set, written on paper, or in a vault its share record
    /// (`docs/CODEX32.md` §9.5).
    pub const fn codex32_share(i: usize, j: usize) -> What {
        what(KeyForm::Codex32, i, Piece::Part(j), W::Codex32String, Paper)
    }
    /// That string's Codex32QR, drawn by hand on paper.
    pub const fn codex32_qr_paper(i: usize, j: usize) -> What {
        what(KeyForm::Codex32, i, Piece::Part(j), W::Codex32Qr, Paper)
    }
    /// That string as a `.txt` file, on a stick at a place.
    pub const fn codex32_share_file(i: usize, j: usize) -> What {
        what(KeyForm::Codex32, i, Piece::Part(j), W::Codex32String, File)
    }
    /// That string's Codex32QR as a `.png` picture, on a stick at a place.
    pub const fn codex32_qr_file(i: usize, j: usize) -> What {
        what(KeyForm::Codex32, i, Piece::Part(j), W::Codex32Qr, File)
    }
    /// Seed `i`'s Krux encrypted code as a picture and a `.kef` file, on a
    /// stick at a place (`docs/KEF.md` §15.1): an extra copy that no
    /// check counts.
    pub const fn kef_code(i: usize) -> What {
        what(KeyForm::Krux, i, Piece::Whole, W::KruxCode, File)
    }
    /// That code drawn by hand on paper at a place.
    pub const fn kef_code_paper(i: usize) -> What {
        what(KeyForm::Krux, i, Piece::Whole, W::KruxCode, Paper)
    }
    /// Part `j` (from 0) of seed `i`'s Seed XOR set, written on paper at
    /// a place (`docs/decisions/F276.md`): every part is needed.
    pub const fn xor_part(i: usize, j: usize) -> What {
        what(KeyForm::Xor, i, Piece::Part(j), W::PartWords, Paper)
    }
    /// Member `m` of group `g` of seed `i`'s SLIP-39 set, written on
    /// paper at a place (`docs/decisions/F276.md`).
    pub const fn slip39_share(i: usize, g: usize, m: usize) -> What {
        what(
            KeyForm::Slip39,
            i,
            Piece::Member(g, m),
            W::ShareWords,
            Paper,
        )
    }

    /// Seed `i`'s copy by hand as `writing`, when the words form lists
    /// that output on paper.
    pub fn words_copy(i: usize, writing: Writing) -> Option<What> {
        Written::new(KeyForm::Words, i, Piece::Whole, writing, Paper).map(What::Written)
    }

    /// The seed it holds a copy of in the words form, on any medium.
    pub fn words_form_seed(self) -> Option<usize> {
        match self {
            What::Written(c) if c.form == KeyForm::Words => Some(c.seed),
            _ => None,
        }
    }

    /// The seed whose words form it holds a copy of on `medium`: its
    /// words or its SeedQR.
    pub fn words_seed_on(self, medium: Medium) -> Option<usize> {
        match self {
            What::Written(c) if c.form == KeyForm::Words && c.medium == medium => Some(c.seed),
            _ => None,
        }
    }

    /// Whether it is a copy of `form`, however written.
    pub fn is_form(self, form: KeyForm) -> bool {
        matches!(self, What::Written(c) if c.form == form)
    }

    /// Whether it is a seed's words written by hand.
    pub fn is_words(self) -> bool {
        matches!(self, What::Written(c) if c.is(KeyForm::Words, W::Words, Paper))
    }

    /// Whether it is a seed's SeedQR picture on a stick.
    pub fn is_seed_qr_file(self) -> bool {
        matches!(self, What::Written(c) if c.is(KeyForm::Words, W::SeedQr, File))
    }

    /// The seed whose words are the text file it is.
    pub fn words_file_seed(self) -> Option<usize> {
        match self {
            What::Written(c) if c.is(KeyForm::Words, W::Words, File) => Some(c.seed),
            _ => None,
        }
    }

    /// The seed a file kept on a stick is of: its words file, its SeedQR
    /// picture or its passphrase file.
    pub fn seed_stick_file_seed(self) -> Option<usize> {
        match self {
            What::Written(c) if c.form == KeyForm::Words && c.medium == File => Some(c.seed),
            What::PassphraseFile(i) => Some(i),
            _ => None,
        }
    }

    /// The form's copy it is, if it is one.
    pub const fn written(self) -> Option<Written> {
        match self {
            What::Written(w) => Some(w),
            _ => None,
        }
    }
}

const fn what(form: KeyForm, seed: usize, piece: Piece, writing: Writing, medium: Medium) -> What {
    What::Written(Written {
        form,
        seed,
        piece,
        writing,
        medium,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spelling a note wrote before copies were read from the
    /// registry, with the copy it names.
    fn saved() -> Vec<(&'static str, What)> {
        vec![
            ("w0", What::words(0)),
            ("q1", What::seed_qr(1)),
            ("file2", What::seed_file(2)),
            ("qrfile3", What::seed_qr_file(3)),
            ("x0.10", What::codex32_share(0, 10)),
            ("xq1.0", What::codex32_qr_paper(1, 0)),
            ("xt2.31", What::codex32_share_file(2, 31)),
            ("xp0.5", What::codex32_qr_file(0, 5)),
            ("kef4", What::kef_code(4)),
            ("kefp4", What::kef_code_paper(4)),
            ("xorp0.1", What::xor_part(0, 1)),
            ("slip0.1.2", What::slip39_share(0, 1, 2)),
        ]
    }

    #[test]
    fn a_saved_spelling_reads_back_as_the_same_copy() {
        for (code, what) in saved() {
            assert_eq!(what_read(code), Some(what), "{code}");
            assert_eq!(&*what_code(what), code);
        }
    }

    #[test]
    fn a_share_index_past_the_codex32_alphabet_is_refused() {
        assert_eq!(what_read("x0.32"), None);
        assert_eq!(what_read("xq0.40"), None);
    }

    #[test]
    fn every_output_of_every_form_has_a_spelling_that_reads_back() {
        for form in KeyForm::ALL {
            assert_eq!(rows(form).len(), form.outputs().len());
            for o in form.outputs() {
                let piece = match shape(form) {
                    Shape::Whole => Piece::Whole,
                    Shape::Part => Piece::Part(3),
                    Shape::Member => Piece::Member(1, 2),
                };
                let w = Written::new(form, 7, piece, o.writing, o.medium).expect("listed");
                assert_eq!(Written::read(&w.code()), Some(w));
            }
        }
    }

    #[test]
    fn a_copy_the_registry_does_not_list_is_refused() {
        assert!(
            Written::new(
                KeyForm::Krux,
                0,
                Piece::Whole,
                Writing::SeedQr,
                Medium::Paper
            )
            .is_none()
        );
    }
}
