//! What a person sealing and opening codes would notice, apart from the
//! vectors.

use aqrv::{Header, Item, NotOpened, Refused, Size, Step, open, recognise, seal};
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

fn compute() -> Compute {
    Compute::unchecked()
}

/// A deterministic source of bytes, not random: enough for these tests.
fn counter() -> impl FnMut(&mut [u8]) {
    let mut n = 0u8;
    move |buf: &mut [u8]| {
        for b in buf {
            n = n.wrapping_add(37);
            *b = n;
        }
    }
}

fn entropy(len: usize) -> Item {
    Item::Entropy(Zeroizing::new(vec![0x5a; len]))
}

fn text(s: &str) -> Zeroizing<String> {
    Zeroizing::new(s.to_owned())
}

/// A codex32 string of a 28-byte seed (67 characters), BIP 93's
/// vector 8, taken from the vectors.
fn codex32_33_bytes() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/aqrv/aqrv.json"
    );
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["item_length"] == 33)
        .unwrap();
    case["expect"]["item"]["string"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn shown(item: &Item) -> String {
    match item {
        Item::Entropy(b) => format!("{b:?}"),
        Item::Passphrase(s) | Item::Note(s) | Item::Codex32(s) => s.to_string(),
        Item::RawKey(k) => format!("{:?}", &k[..]),
    }
}

#[test]
fn sealing_the_same_item_twice_gives_two_envelopes_that_open_alike() {
    let c = compute();
    let item = entropy(16);
    let mut random = counter();
    let a = seal(&c, &item, "pass", Step::Mib64, Size::Small, &mut random).unwrap();
    let b = seal(&c, &item, "pass", Step::Mib64, Size::Small, &mut random).unwrap();
    assert_ne!(a, b);
    for envelope in [a, b] {
        let opened = open(&c, &envelope, "pass").unwrap();
        assert_eq!(shown(&opened.item), shown(&item));
    }
}

#[test]
fn an_empty_passphrase_and_empty_text_are_refused() {
    let c = compute();
    let mut random = counter();
    let got = seal(&c, &entropy(16), "", Step::Mib64, Size::Small, &mut random);
    assert!(matches!(got, Err(Refused::EmptyPassphrase)));
    for item in [Item::Passphrase(text("")), Item::Note(text(""))] {
        let got = seal(&c, &item, "pass", Step::Mib64, Size::Large, &mut random);
        assert!(matches!(got, Err(Refused::EmptyText)));
    }
}

#[test]
fn a_33_byte_codex32_string_is_refused_at_53_bytes_and_written_at_62() {
    let c = compute();
    let mut random = counter();
    let string = codex32_33_bytes();
    let item = Item::Codex32(Zeroizing::new(string.clone()));
    let small = seal(&c, &item, "pass", Step::Mib64, Size::Small, &mut random);
    assert!(matches!(small, Err(Refused::Codex32TooLongForSmall)));
    let large = seal(&c, &item, "pass", Step::Mib64, Size::Large, &mut random).unwrap();
    assert_eq!(large.len(), 62);
    let opened = open(&c, &large, "pass").unwrap();
    assert_eq!(shown(&opened.item), string);
}

#[test]
fn a_passphrase_typed_in_nfd_opens_what_was_sealed_in_nfc() {
    let c = compute();
    let mut random = counter();
    let composed = "caf\u{e9}";
    let decomposed = "cafe\u{301}";
    let item = Item::Note(text("a note"));
    let envelope = seal(&c, &item, composed, Step::Mib64, Size::Small, &mut random).unwrap();
    let opened = open(&c, &envelope, decomposed).unwrap();
    assert_eq!(shown(&opened.item), "a note");
}

#[test]
fn a_wrong_passphrase_does_not_open() {
    let c = compute();
    let mut random = counter();
    let envelope = seal(
        &c,
        &entropy(32),
        "right",
        Step::Mib64,
        Size::Large,
        &mut random,
    )
    .unwrap();
    assert!(matches!(
        open(&c, &envelope, "wrong"),
        Err(NotOpened::NoTag)
    ));
}

#[test]
fn the_step_and_size_are_read_without_a_passphrase() {
    let mut bytes = vec![0u8; 62];
    for step in Step::ALL {
        bytes[0] = step.header();
        let got = recognise(&bytes);
        assert_eq!(got.map(|h| h.step.memory_kib()), Some(step.memory_kib()));
        assert_eq!(got.map(|h| h.size), Some(Size::Large));
    }
    bytes[0] = 0xF8;
    bytes.truncate(53);
    assert_eq!(
        recognise(&bytes),
        Some(Header {
            size: Size::Small,
            step: Step::Mib64
        })
    );
    // A reserved step, another marker and another length.
    bytes[0] = 0xFD;
    assert_eq!(recognise(&bytes), None);
    bytes[0] = 0xF0;
    assert_eq!(recognise(&bytes), None);
    bytes[0] = 0xF8;
    bytes.truncate(52);
    assert_eq!(recognise(&bytes), None);
}

#[test]
fn the_writer_draws_the_salt_then_the_fill_byte_then_the_padding() {
    let c = compute();
    let draws = |item: &Item, size: Size| {
        let mut lengths = Vec::new();
        let mut random = |buf: &mut [u8]| {
            lengths.push(buf.len());
            buf.fill(0x11);
        };
        seal(&c, item, "pass", Step::Mib64, size, &mut random).unwrap();
        lengths
    };
    // 24 bytes of entropy in a room of 32: the salt, then 8 of padding.
    assert_eq!(draws(&entropy(24), Size::Small), [16, 8]);
    // An item that fills the room needs no padding.
    assert_eq!(draws(&entropy(32), Size::Small), [16]);
    // A codex32 string of 21 bytes: the salt, the fill byte, 11 of padding.
    let string = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    let item = Item::Codex32(text(string));
    assert_eq!(draws(&item, Size::Small), [16, 1, 11]);
}
