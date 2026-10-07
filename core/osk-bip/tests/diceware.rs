//! The baked EFF lists against the files they were baked from
//! (`tools/vectors/eff/`, whose README records their digests).

use osk_bip::diceware::List;
use osk_crypto::sha256;

/// The three committed lists, in the order [`List::ALL`] names them.
const FILES: [&str; 3] = [
    include_str!("../../../tools/vectors/eff/eff_large_wordlist.txt"),
    include_str!("../../../tools/vectors/eff/eff_short_wordlist_1.txt"),
    include_str!("../../../tools/vectors/eff/eff_short_wordlist_2_0.txt"),
];

/// The roll at `index`: the index in base six with one added to each
/// digit, most significant die first.
fn rolls_of(index: usize, dice: usize) -> Vec<u8> {
    let mut out = vec![0u8; dice];
    let mut i = index;
    for slot in out.iter_mut().rev() {
        *slot = (i % 6) as u8 + 1;
        i /= 6;
    }
    out
}

#[test]
fn every_roll_names_the_word_the_file_names() {
    for (list, file) in List::ALL.into_iter().zip(FILES) {
        let lines: Vec<&str> = file.lines().collect();
        assert_eq!(lines.len(), list.len());
        assert_eq!(list.len(), 6usize.pow(list.dice_per_word() as u32));
        for (i, line) in lines.iter().enumerate() {
            let (rolls, word) = line.split_once('\t').expect("rolls, a tab, a word");
            let thrown = rolls_of(i, list.dice_per_word());
            let spelled: String = thrown.iter().map(|d| char::from(b'0' + d)).collect();
            assert_eq!(rolls, spelled, "{list:?} line {}", i + 1);
            assert_eq!(list.word(&thrown), Some(word), "{list:?} roll {rolls}");
        }
    }
}

#[test]
fn the_digests_match_the_committed_files() {
    for (list, file) in List::ALL.into_iter().zip(FILES) {
        let digest: String = sha256(file.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, list.sha256(), "{list:?}");
    }
}

#[test]
fn a_roll_that_is_not_a_throw_of_this_list_names_no_word() {
    assert_eq!(List::Large.word(&[1, 1, 1, 1]), None, "four of five dice");
    assert_eq!(List::Short1.word(&[1, 1, 1, 1, 1]), None, "five of four");
    assert_eq!(List::Large.word(&[1, 1, 1, 1, 7]), None, "no such face");
    assert_eq!(List::Large.word(&[1, 1, 1, 1, 0]), None, "no such face");
}
