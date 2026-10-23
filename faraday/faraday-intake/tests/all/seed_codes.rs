//! The seed a scanned code holds: SeedQR and CompactSeedQR over the worked
//! examples of SeedSigner's specification (`docs/seed_qr/README.md`, "Test
//! SeedQRs"), and a seed's words as plain text.

use crate::common::unhex;
use faraday_intake::{seed_in_code, seedqr_words};

/// Words, SeedQR digits, CompactSeedQR bytes as hex.
const SPEC: &[(&str, &str, &str)] = &[
    (
        "attack pizza motion avocado network gather crop fresh patrol unusual wild holiday candy pony ranch winter theme error hybrid van cereal salon goddess expire",
        "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643",
        "0e74b64107f94cc0ccfae6a13dcbec3662154fec67e0e00999c07892597d190a",
    ),
    (
        "atom solve joy ugly ankle message setup typical bean era cactus various odor refuse element afraid meadow quick medal plate wisdom swap noble shallow",
        "011416550964188800731119157218870156061002561932122514430573003611011405110613292018175411971576",
        "0e59dde276009317f1275f1389888078c99368d1e82489b5f629531fc5b6a56e",
    ),
    (
        "sound federal bonus bleak light raise false engage round stock update render quote truck quality fringe palace foot recipe labor glow tortoise potato still",
        "166206750203018810361417065805941507171219081456140818651401074412730727143709940798183613501710",
        "cfca8c658bc81962549252bc7ac3ba5b0b01d26bcae89f2b5ecebe263dcb2a36",
    ),
    (
        "forum undo fragile fade shy sign arrest garment culture tube off merit",
        "073318950739065415961602009907670428187212261116",
        "5bbd9d71a8ec7990831aff359d426545",
    ),
    (
        "good battle boil exact add seed angle hurry success glad carbon whisper",
        "080301540200062600251559007008931730078802752004",
        "6462686427203385c2337dd84c5089fd",
    ),
    (
        "approve fruit lens brass ring actual stool coin doll boss strong rate",
        "008607501025021714880023171503630517020917211425",
        "0acbba008d9ba005f5996b40a3475cd9",
    ),
    (
        "dignity utility vacant shiver thought canoe feel multiply item youth actor coyote",
        "049619221923158517990268067811630950204300210397",
        "3e1e0bc1e31e0e4315348b76dfec0a98",
    ),
    (
        "corn voice scrap arrow original diamond trial property benefit choose junk lock",
        "038719631547010112530489185713790169032209701051",
        "307eaf058659ca7a7a0d63152509e541",
    ),
    (
        "vocal tray giggle tool duck letter category pattern train magnet excite swamp",
        "196218530783182905421028028912901848107106301753",
        "f55cf587f2543d01090d0ae710bd3b6d",
    ),
];

#[test]
fn a_seedqr_reads_as_the_words_the_specification_gives() {
    for (words, digits, _) in SPEC {
        let read = seedqr_words(digits.as_bytes()).unwrap_or_else(|| panic!("{words}"));
        assert_eq!(read.as_str(), *words);
    }
}

#[test]
fn a_compact_seedqr_reads_as_the_words_the_specification_gives() {
    for (words, _, entropy) in SPEC {
        let read = seedqr_words(&unhex(entropy)).unwrap_or_else(|| panic!("{words}"));
        assert_eq!(read.as_str(), *words);
    }
}

#[test]
fn a_scanned_seedqr_of_either_kind_is_a_seed() {
    let c = crate::common::compute();
    for (words, digits, entropy) in SPEC {
        for payload in [digits.as_bytes().to_vec(), unhex(entropy)] {
            let read = seed_in_code(&c, &payload).unwrap_or_else(|| panic!("{words}"));
            assert_eq!(read.as_str(), *words);
        }
    }
}

#[test]
fn a_code_with_the_wrong_number_of_digits_or_a_word_past_2047_is_not_a_seedqr() {
    let (_, digits, _) = SPEC[3];
    // 11 words of a 12-word SeedQR.
    assert!(seedqr_words(&digits.as_bytes()[..44]).is_none());
    // One digit too many.
    assert!(seedqr_words(format!("{digits}0").as_bytes()).is_none());
    // The first word's index is 2048.
    let mut bad = digits.to_string();
    bad.replace_range(..4, "2048");
    assert!(seedqr_words(bad.as_bytes()).is_none());
}

#[test]
fn a_code_that_is_neither_digits_nor_entropy_is_not_a_seedqr() {
    assert!(seedqr_words(b"").is_none());
    assert!(seedqr_words(b"bitcoin:bc1q5y4r767v5fzx74ez4nw36hjqrhr4ayeyut5px6").is_none());
    // 16 bytes that are text are words or an address, not entropy.
    assert!(seedqr_words(b"abandon abandon!").is_none());
}

#[test]
fn a_code_of_seed_words_reads_as_the_seed_however_it_is_spaced() {
    let c = crate::common::compute();
    for [_, words, _, _] in crate::common::bip39_english() {
        let n = words.split(' ').count();
        let spaced = words.replace(' ', "  \n");
        for text in [words.clone(), spaced] {
            let read = seed_in_code(&c, text.as_bytes()).unwrap_or_else(|| panic!("{n} words"));
            assert_eq!(read.as_str(), words);
        }
    }
}

#[test]
fn words_with_a_wrong_checksum_or_a_wrong_count_are_not_a_seed() {
    let c = crate::common::compute();
    let twelve = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    assert!(seed_in_code(&c, twelve.as_bytes()).is_some());
    let bad_sum = twelve.replace("about", "abandon");
    assert!(seed_in_code(&c, bad_sum.as_bytes()).is_none());
    let eleven = twelve.rsplit_once(' ').unwrap().0;
    assert!(seed_in_code(&c, eleven.as_bytes()).is_none());
    assert!(seed_in_code(&c, b"").is_none());
    assert!(seed_in_code(&c, &[0xff, 0xfe, 0x00]).is_none());
}
