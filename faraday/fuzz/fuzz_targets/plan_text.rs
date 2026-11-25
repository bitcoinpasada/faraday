//! A backup plan's text as a vault file keeps it (`docs/VAULT.md` §7,
//! type 11): `faraday_plan::Answers::from_text` over any text, for each of
//! several wallet shapes, and `Note::from_text`, the plan's notes. The vault
//! file is online input. Whatever the text, a read gives answers or `None`,
//! and answers read are written and read back, and what is read from that
//! writing is written and read as itself (a free text, a vault's name, is
//! cleaned when written, so the first read back may differ from the read).
//! A note read is written and reads back as the same note.

#![no_main]

use faraday_core::plan::{Answers, Note, Seed, Shape};
use faraday_text::Text;
use libfuzzer_sys::fuzz_target;

fn shape(m: usize, seeds: &[(bool, bool, bool)], vaults: usize, paths: Vec<u64>) -> Shape {
    Shape {
        m,
        paths,
        keys: seeds.len(),
        seeds: seeds
            .iter()
            .enumerate()
            .map(|(i, &(here, shares, passphrase))| Seed {
                name: Text::of(&format!("{i:08x}")),
                here,
                shares,
                passphrase,
            })
            .collect(),
        splits: seeds.len() > 1,
        vaults,
        seed_alone_fails: seeds.len() > 1,
        text_files: false,
        silent: false,
        said: false,
    }
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut shapes = [
        shape(1, &[(true, false, false)], 0, vec![]),
        shape(1, &[(false, false, false)], 0, vec![]),
        shape(2, &[(true, false, true), (true, false, false), (false, false, false)], 2, vec![]),
        shape(2, &[(true, false, false), (false, true, false), (true, false, false)], 3, vec![0b011, 0b101]),
    ];
    // A silent wallet's plan is the one that takes the text file's mark.
    shapes[0].silent = true;
    for shape in &shapes {
        if let Some(answers) = Answers::from_text(shape, &text) {
            let back = Answers::from_text(shape, &answers.to_text())
                .expect("answers written read back");
            let again = Answers::from_text(shape, &back.to_text())
                .expect("answers written read back");
            assert_eq!(again, back);
        }
    }
    // A note read is written back as it was read.
    for seeds in [0, 1, 4, usize::MAX] {
        if let Some(note) = Note::from_text(&text, seeds) {
            let back = Note::from_text(&note.to_text(), seeds).expect("note written read back");
            assert_eq!(back, note);
        }
    }
});
