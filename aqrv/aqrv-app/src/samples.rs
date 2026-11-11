//! The samples of Make's first step: the public inputs of the test
//! vectors (`tools/vectors/aqrv/aqrv.json`, BIP 93), so a code made from
//! one is a code the vectors' reader opens. They hold no real funds.

use aqrv::Item;
use faraday_strings::Strings;
use zeroize::Zeroizing;

/// One sample: its name and its type as the list shows them, and its item.
pub struct Sample {
    /// The name, from the strings.
    pub name: fn(&Strings) -> &'static str,
    /// The type under the name, from the strings.
    pub kind: fn(&Strings) -> &'static str,
    item: fn() -> Item,
}

impl Sample {
    /// The item this sample seals.
    pub fn item(&self) -> Item {
        (self.item)()
    }
}

fn entropy(hex: &str) -> Item {
    Item::Entropy(Zeroizing::new(unhex(hex)))
}

fn unhex(hex: &str) -> Vec<u8> {
    faraday_codec::encodings::from_hex(hex).expect("a sample's hex is well formed")
}

/// Every sample, in the order the list shows them.
pub static SAMPLES: [Sample; 7] = [
    Sample {
        name: |s| s.aqrv_sample_words12,
        kind: |s| s.aqrv_type_words12,
        item: || entropy("4a1961ee49593ede1a397643bbed0c39"),
    },
    Sample {
        name: |s| s.aqrv_sample_words24,
        kind: |s| s.aqrv_type_words24,
        item: || entropy("e352ee2cc2f809ad3da89a0a6f804a02027793af5cd60992bdd2cc45d6fd71cd"),
    },
    Sample {
        name: |s| s.aqrv_sample_codex_secret,
        kind: |s| s.aqrv_type_codex32,
        item: || {
            Item::Codex32(Zeroizing::new(
                "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw".to_owned(),
            ))
        },
    },
    Sample {
        name: |s| s.aqrv_sample_codex_share,
        kind: |s| s.aqrv_type_codex32,
        item: || {
            Item::Codex32(Zeroizing::new(
                "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm".to_owned(),
            ))
        },
    },
    Sample {
        name: |s| s.aqrv_sample_note,
        kind: |s| s.aqrv_type_note,
        item: || Item::Note(Zeroizing::new("Zoë café 日本語 ✓".to_owned())),
    },
    Sample {
        name: |s| s.aqrv_sample_passphrase,
        kind: |s| s.aqrv_type_passphrase,
        item: || Item::Passphrase(Zeroizing::new("0123456789abcdefghijklmnopqrstu".to_owned())),
    },
    Sample {
        name: |s| s.aqrv_sample_raw_key,
        kind: |s| s.aqrv_type_raw_key,
        item: || {
            let mut key = Zeroizing::new([0u8; 32]);
            key.copy_from_slice(&unhex(
                "cfef577881aa9695f48c0b9ba9d83f67789f55989073accaef48bf40a2222fde",
            ));
            Item::RawKey(key)
        },
    },
];
