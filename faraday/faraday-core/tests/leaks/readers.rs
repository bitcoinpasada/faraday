//! The readers of what a vault or a file holds: a vault's entries
//! listed, opened and copied, a Krux encrypted code opened, a GPG key
//! restored from its paperkey, and a Bitcoin Core wallet read. Each plants
//! the secret the reader takes in and ends by Lock where the flow has one.

use crate::common::{device, type_text};
use crate::forms::{
    ENTRY_NOTES, ENTRY_PASSWORD, PHRASE, TOTP_URI, VAULT_PASSPHRASE, entry_forms, seed_forms,
    vault_passphrase_forms,
};
use crate::{arm, finish};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

/// A vault of the test seed as a key and one entry holding a password,
/// notes and a TOTP secret.
fn vault(label: &str, id: u8) -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let key = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_LABEL, b"Leak key");
    let entry = Record::new(kind::ENTRY)
        .with(field::TITLE, b"Mail")
        .with(field::PASSWORD, ENTRY_PASSWORD.as_bytes())
        .with(field::NOTES, ENTRY_NOTES.as_bytes())
        .with(field::TOTP, TOTP_URI.as_bytes());
    crate::common::vault_of(label, VAULT_PASSPHRASE, id, vec![key, entry])
}

/// Every form the vault holds.
fn vault_forms() -> Vec<Form> {
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    forms.extend(entry_forms());
    forms
}

/// Unlocks place `at` of the Vaults list, the Load sheet answered Not now.
fn unlock(app: &mut Faraday, at: usize) {
    let before = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "the vault did not open"
    );
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
}

/// The page of the vault's first item of `category`, drawn.
fn open_item(app: &mut Faraday, category: usize) {
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
}

#[test]
fn a_vaults_entries_listed_and_opened_leave_no_copy_after_lock() {
    let file = vault("Cellar", 0x5e);
    let forms = vault_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    for category in [0, 2] {
        app.press(Action::Vault(V::Category(category)));
        let _ = app.frame();
    }
    open_item(&mut app, 0);
    open_item(&mut app, 2);
    crate::seed_load::lock(&mut app);
    finish("vault entries opened", armed, true);
}

#[test]
fn a_vaults_entry_copied_to_another_vault_leaves_no_copy_after_lock() {
    let (a, b) = (
        vault("Alpha", 0x61),
        crate::common::vault_of("Beta", VAULT_PASSPHRASE, 0x62, Vec::new()),
    );
    let forms = vault_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)]);
    unlock(&mut app, 0);
    unlock(&mut app, 1);
    testkit::vault_current(&mut app, 0);
    open_item(&mut app, 2);
    app.press(Action::Vault(V::CopyTo));
    assert_eq!(app.sheet, Some(Sheet::CopyTo));
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.sheet, None, "the copy was not made");
    crate::seed_load::lock(&mut app);
    finish("vault entry copied", armed, true);
}

/// The long passphrase a Krux code is sealed under.
const KEF_PASSPHRASE: &str = "Cobalt-Meridian-Thistle-6290-Juniper-Lantern-Opal-185";

/// A code Faraday seals around the test seed's words.
fn kef_code() -> Vec<u8> {
    kef_code_of(20, words())
}

/// A code of `version` Faraday seals around `contents`.
fn kef_code_of(version: u8, contents: zeroize::Zeroizing<Vec<u8>>) -> Vec<u8> {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.kef_seal(
        zeroize::Zeroizing::new(KEF_PASSPHRASE.as_bytes().to_vec()),
        contents,
        version,
        faraday_core::worker::kef::KefId::Random(8),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(&mut app);
    let mut sealed = app.kef_take_sealed().unwrap().unwrap();
    std::mem::take(&mut *sealed)
}

/// The seed words in a wiping buffer.
fn words() -> zeroize::Zeroizing<Vec<u8>> {
    let mut text = zeroize::Zeroizing::new(Vec::with_capacity(PHRASE.len()));
    text.extend_from_slice(PHRASE.as_bytes());
    text
}

/// A code of the compressed version 21 around the seed words and 775
/// spaces, so it inflates to over twice its size. Faraday writes no
/// compressed version, so the code is made as Krux makes it: the raw
/// deflate stream sealed as version 20's plaintext, read as version 21
/// (same key, same layout), the stream written here by hand.
fn compressed_kef_code() -> Vec<u8> {
    let mut stream = DeflateBits::default();
    // One final block with the fixed codes.
    stream.put(1, 1, false);
    stream.put(1, 2, false);
    for b in PHRASE.bytes().chain(*b" ") {
        stream.literal(b);
    }
    // Three matches of 258 bytes, one byte back.
    for _ in 0..3 {
        stream.put(0xC5, 8, true);
        stream.put(0, 5, true);
    }
    stream.put(0, 7, true);
    let mut code = kef_code_of(20, zeroize::Zeroizing::new(stream.bytes));
    // The version byte follows the id (8 bytes, after its length).
    assert_eq!(code[9], 20);
    code[9] = 21;
    code
}

/// Bits of a deflate stream, filled from each byte's low end.
#[derive(Default)]
struct DeflateBits {
    bytes: Vec<u8>,
    used: usize,
}

impl DeflateBits {
    /// `n` bits of `value`: a code most significant bit first, else the
    /// value's low bit first.
    fn put(&mut self, value: u32, n: usize, code: bool) {
        for i in 0..n {
            let bit = if code {
                (value >> (n - 1 - i)) & 1
            } else {
                (value >> i) & 1
            };
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            *self.bytes.last_mut().expect("a byte was pushed") |= (bit as u8) << (self.used % 8);
            self.used += 1;
        }
    }

    /// A literal below 144, which the fixed codes write as 8 bits.
    fn literal(&mut self, byte: u8) {
        assert!(byte < 144);
        self.put(0x30 + u32::from(byte), 8, true);
    }
}

#[test]
fn a_krux_code_sealed_with_padding_leaves_no_copy() {
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "code passphrase",
        KEF_PASSPHRASE.as_bytes().to_vec(),
    ));
    let armed = arm(&forms);
    let code = kef_code_of(5, words());
    assert!(!code.is_empty(), "the code was made");
    finish("krux code sealed with padding", armed, false);
}

#[test]
fn a_krux_code_sealed_leaves_no_copy() {
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "code passphrase",
        KEF_PASSPHRASE.as_bytes().to_vec(),
    ));
    let armed = arm(&forms);
    let code = kef_code();
    assert!(!code.is_empty(), "the code was made");
    finish("krux code sealed", armed, false);
}

#[test]
fn a_krux_code_opened_leaves_no_copy_after_lock() {
    opened_leaves_no_copy(kef_code(), "krux code opened", |app| {
        app.sheet == Some(Sheet::SeedRead)
    });
}

#[test]
fn a_compressed_krux_code_opened_leaves_no_copy_after_lock() {
    // Inflates to over twice its compressed size.
    opened_leaves_no_copy(
        compressed_kef_code(),
        "compressed krux code opened",
        |app| app.sheet == Some(Sheet::SeedRead) || testkit::kef_shows_contents(app),
    );
}

fn opened_leaves_no_copy(code: Vec<u8>, name: &str, opened: fn(&Faraday) -> bool) {
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "code passphrase",
        KEF_PASSPHRASE.as_bytes().to_vec(),
    ));
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.event(Event::Scanned { bytes: code });
    let _ = app.frame();
    let open = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .and_then(|c| c.ways.iter().position(|w| w.label() == "Open"))
        .expect("the card offers Open");
    app.press(Action::ReadGo(open as u8));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, KEF_PASSPHRASE);
    app.event(Event::Key(Key::Enter));
    for _ in 0..6 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    assert!(opened(&app), "the code did not open");
    app.press(Action::Cancel);
    crate::seed_load::lock(&mut app);
    finish(name, armed, true);
}

/// A note a Krux code holds that a person may use as a passphrase.
const KEF_NOTE: &str = "Spare-passphrase Quartz-Harbor-Velvet-4417-Ember-Sable-902";

#[test]
fn a_krux_note_used_as_a_passphrase_leaves_no_copy_after_lock() {
    let mut note = zeroize::Zeroizing::new(Vec::with_capacity(KEF_NOTE.len()));
    note.extend_from_slice(KEF_NOTE.as_bytes());
    let code = kef_code_of(20, note);
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "code passphrase",
        KEF_PASSPHRASE.as_bytes().to_vec(),
    ));
    forms.push(Form::new("note", KEF_NOTE.as_bytes().to_vec()));
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    app.event(Event::Scanned { bytes: code });
    let _ = app.frame();
    let open = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .and_then(|c| c.ways.iter().position(|w| w.label() == "Open"))
        .expect("the card offers Open");
    app.press(Action::ReadGo(open as u8));
    type_text(&mut app, KEF_PASSPHRASE);
    app.event(Event::Key(Key::Enter));
    for _ in 0..6 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    assert!(testkit::kef_shows_contents(&app), "the note did not open");
    app.press(Action::KefRead(
        faraday_core::seeds::kef::KefAct::UseAsPassphrase,
    ));
    let _ = app.frame();
    assert_ne!(app.sheet, Some(Sheet::KefRead), "the note was not carried");
    app.press(Action::Cancel);
    crate::seed_load::lock(&mut app);
    finish("krux note used as passphrase", armed, true);
}

/// The key of `tools/vectors/paperkey/faraday-key.txt`: the seeds it was
/// made from.
fn paper_seeds() -> ([u8; 32], [u8; 32]) {
    let (mut primary, mut subkey) = ([0x11; 32], [0x22; 32]);
    primary[0] = 0;
    subkey[..2].fill(0);
    (primary, subkey)
}

#[test]
fn a_gpg_key_restored_from_its_paperkey_leaves_no_copy() {
    use faraday_pgp::{Armor, armor};
    const PRINTED: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");
    let (primary, subkey) = paper_seeds();
    let made = faraday_keys::gpg::create(
        &zeroize::Zeroizing::new(primary),
        &zeroize::Zeroizing::new(subkey),
        1_780_000_000,
        faraday_text::Text::of("Test Person <test@example.com>"),
        2 * 31_536_000,
    );
    let text = armor(Armor::PublicKey, &made.certificate);
    let certificate = faraday_pgp::Certificate::read(text.as_bytes()).expect("its certificate");
    let lines: Vec<&str> = PRINTED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let forms = vec![
        Form::new("GPG primary seed", primary.to_vec()),
        Form::new("GPG subkey seed", subkey.to_vec()),
    ];
    let c = testkit::compute();
    let armed = arm(&forms);
    let mut restore = faraday_keys::gpg::Restore::new(certificate);
    for (i, l) in lines.iter().enumerate() {
        restore
            .line(i as u32 + 1, l)
            .unwrap_or_else(|e| panic!("line {}: {e:?}", i + 1));
    }
    assert!(restore.complete(), "the paperkey did not complete");
    let restored = restore.finish(&c).expect("the key restores");
    drop(restored);
    drop(restore);
    finish("gpg paperkey restored", armed, false);
}

#[test]
fn a_bitcoin_core_wallet_read_leaves_no_copy() {
    use faraday_bip::bitcoin::bip32::Xpriv;
    use faraday_bip::keys::Network;
    use faraday_core::core_wallet::read;
    use faraday_intake::secret_buf::SecretText;
    use std::str::FromStr;
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/core-wallet/encrypted.json"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("dump")).expect("json");
    let hex = |v: &serde_json::Value| {
        let s = v.as_str().expect("hex");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
            .collect::<Vec<u8>>()
    };
    let records: Vec<(Vec<u8>, Vec<u8>)> = json["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|r| (hex(&r[0]), hex(&r[1])))
        .collect();
    let id = u32::try_from(json["application_id"].as_u64().expect("id")).expect("id");
    let desc = json["listdescriptors"]["descriptors"][0]["desc"]
        .as_str()
        .expect("a private descriptor");
    let start = desc.find("tprv").expect("its master key");
    let tprv = &desc[start..start + desc[start..].find('/').expect("its path")];
    let key = Xpriv::from_str(tprv).expect("a tprv");
    let forms = vec![
        Form::new("core master tprv text", tprv.as_bytes().to_vec()),
        Form::new("core master tprv", key.encode().to_vec()),
        Form::new(
            "core master private key",
            key.private_key.secret_bytes().to_vec(),
        ),
        Form::new("core master chain code", key.chain_code.as_bytes().to_vec()),
    ];
    let c = testkit::compute();
    let pass = SecretText::of("faraday");
    let armed = arm(&forms);
    let wallet = read(&c, id, &records, Network::Regtest, Some(&pass)).expect("the wallet reads");
    assert!(wallet.is_whole(), "{:?}", wallet.failures);
    drop(wallet);
    finish("core wallet read", armed, false);
}

#[test]
fn a_words_file_read_leaves_no_freed_copy_of_its_words_or_their_numbers() {
    let mut forms = seed_forms("");
    // The characters of the first words, as a `Vec<char>` holds them.
    for word in PHRASE.split(' ').take(3) {
        forms.push(Form::new(
            "a word's characters (u32, little endian)",
            word.chars()
                .flat_map(|c| (c as u32).to_le_bytes())
                .collect(),
        ));
    }
    // The inputs are built before the arming and left unfreed: what is
    // looked at is what the reading itself frees.
    let words = std::mem::ManuallyDrop::new(faraday_core::forms::file_words(PHRASE));
    let wrong = std::mem::ManuallyDrop::new(format!(
        "{} zzzz",
        PHRASE.rsplit_once(' ').map_or("", |p| p.0)
    ));
    let armed = arm(&forms);
    assert!(
        faraday_core::forms::typed_mnemonic(Language::English, &words).is_ok(),
        "the words are a mnemonic"
    );
    assert!(faraday_core::forms::typed_mnemonic(Language::English, &wrong).is_err());
    finish("words file read", armed, false);
}

/// BIP-32 test vector 1's master key, the private key a file or a
/// descriptor should never hold.
const PLANTED_XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

/// Every form of the planted extended private key.
fn planted_xprv_forms() -> Vec<Form> {
    use faraday_bip::bitcoin::bip32::Xpriv;
    use std::str::FromStr;
    let key = Xpriv::from_str(PLANTED_XPRV).expect("the vector's xprv");
    vec![
        Form::new("planted xprv text", PLANTED_XPRV.as_bytes().to_vec()),
        Form::new("planted xprv", key.encode().to_vec()),
        Form::new(
            "planted private key",
            key.private_key.secret_bytes().to_vec(),
        ),
        Form::new("planted chain code", key.chain_code.as_bytes().to_vec()),
    ]
}

#[test]
fn an_xprv_in_a_caravan_files_xpub_field_refused_leaves_no_copy() {
    let file = format!(
        r#"{{"name":"Planted","addressType":"P2WSH","network":"mainnet","client":{{"type":"public"}},"quorum":{{"requiredSigners":1,"totalSigners":1}},"extendedPublicKeys":[{{"name":"A","bip32Path":"m/48'/0'/0'/2'","xpub":"{PLANTED_XPRV}","xfp":"00000000"}}],"startingAddressIndex":0}}"#
    );
    let c = testkit::compute();
    let forms = planted_xprv_forms();
    let armed = arm(&forms);
    let read = faraday_core::wallet::read_wallet(&c, &file);
    assert!(read.is_err(), "a private key is refused");
    drop(read);
    finish("xprv in a caravan file refused", armed, false);
}

#[test]
fn an_xprv_in_a_wallet_policy_refused_leaves_no_copy() {
    let policy = format!("wsh(sortedmulti(1,[00000000/48h/0h/0h/2h]{PLANTED_XPRV}/<0;1>/*))");
    let c = testkit::compute();
    let forms = planted_xprv_forms();
    let armed = arm(&forms);
    let read = faraday_core::wallet::read_wallet(&c, &policy);
    assert!(read.is_err(), "a private key is refused");
    drop(read);
    finish("xprv in a wallet policy refused", armed, false);
}

#[test]
fn a_seed_in_a_multi_part_ur_leaves_no_copy() {
    use faraday_codec::ur::{Decoder, Encoder, Message};
    // 64 bytes in parts of at most 16: the encoder wraps them in CBOR (a
    // two-byte header, 66 bytes in all) and cuts that into five fragments
    // of 14. The decoder resolves some fragments from mixed parts, joins
    // them and checks the whole. Each fragment and the whole are searched
    // for.
    let seed: Vec<u8> = (0..64u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let parts: Vec<faraday_text::Text> = {
        let mut encoder = Encoder::bytes(&seed, 16).expect("an encoder");
        (0..9).map(|_| encoder.next_part()).collect()
    };
    let mut forms = vec![Form::new("UR seed", seed.clone())];
    let wrapped: Vec<u8> = [0x58, 0x40].iter().chain(&seed).copied().collect();
    for (i, fragment) in wrapped.chunks(14).enumerate() {
        forms.push(Form::new(
            format!("UR seed fragment {i}"),
            fragment.to_vec(),
        ));
    }
    let armed = arm(&forms);
    let mut decoder = Decoder::new();
    // The first part is skipped and the others come out of order, so
    // mixed parts are held and reduced.
    for text in parts.iter().skip(1).rev() {
        if decoder.receive(text).expect("a part") {
            break;
        }
    }
    assert!(
        decoder.is_complete(),
        "the parts did not complete the message"
    );
    match decoder.message() {
        Some(Message::Bytes(bytes)) => assert!(bytes[..] == seed[..], "another message"),
        _ => panic!("not a byte message"),
    }
    drop(decoder);
    finish("multi-part ur seed", armed, false);
}

#[test]
fn an_xprv_in_a_listdescriptors_true_file_refused_leaves_no_copy() {
    let file = format!(
        r#"{{"wallet_name":"planted","descriptors":[{{"desc":"wpkh({PLANTED_XPRV}/84h/0h/0h/<0;1>/*)#xrfujdhd","timestamp":0,"active":true,"internal":false,"range":[0,999],"next":0}}]}}"#
    );
    let c = testkit::compute();
    let forms = planted_xprv_forms();
    let armed = arm(&forms);
    let read = faraday_core::wallet::read_wallet(&c, &file);
    // Its checksum holds and its derivation is a multipath one, so it is
    // refused for the key: a private key is not a public one.
    let refusal = read
        .as_ref()
        .expect_err("a private key is refused as not an xpub");
    let shown = refusal.text();
    let shown = shown.as_str();
    assert!(
        shown.contains(faraday_strings::EN.bip_key_not_xpub),
        "not refused as a key that is not an xpub: {shown}"
    );
    drop(read);
    finish("xprv in a listdescriptors true file refused", armed, false);
}

/// Raw DEFLATE (one fixed-Huffman block) of `text` followed by `copies`
/// copies of its own 258 bytes, so a few bytes inflate to many and a
/// growing output buffer has to be grown.
fn deflate_repeating(text: &[u8], copies: usize) -> Vec<u8> {
    const DIST_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u32; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    let mut bits: Vec<bool> = Vec::new();
    // Written low bit first (header, extra bits) or high bit first (codes).
    let low = |bits: &mut Vec<bool>, v: u32, n: u32| bits.extend((0..n).map(|i| v >> i & 1 == 1));
    let high =
        |bits: &mut Vec<bool>, v: u32, n: u32| bits.extend((0..n).rev().map(|i| v >> i & 1 == 1));
    low(&mut bits, 1, 1); // the last block
    low(&mut bits, 1, 2); // fixed Huffman codes
    for &b in text {
        if b < 144 {
            high(&mut bits, 0x30 + u32::from(b), 8);
        } else {
            high(&mut bits, 0x190 + u32::from(b) - 144, 9);
        }
    }
    let sym = DIST_BASE.iter().rposition(|&b| b <= text.len()).unwrap();
    for _ in 0..copies {
        high(&mut bits, 0xC5, 8); // length 258
        high(&mut bits, sym as u32, 5);
        low(
            &mut bits,
            (text.len() - DIST_BASE[sym]) as u32,
            DIST_EXTRA[sym],
        );
    }
    high(&mut bits, 0, 7); // the end of the block
    bits.chunks(8)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |a, (i, &b)| a | u8::from(b) << i)
        })
        .collect()
}

/// A BBQr of `file_type` carrying `data`, in encoding `2`, or in `Z` when
/// `data` is already raw DEFLATE.
fn bbqr_parts(file_type: char, data: &[u8], z: bool) -> Vec<std::mem::ManuallyDrop<String>> {
    faraday_qr::bbqr::encode(file_type, data, 100)
        .expect("parts")
        .iter()
        .map(|p| {
            let text = if z {
                format!("B$Z{}", &p.as_str()[3..])
            } else {
                String::from(p.as_str())
            };
            std::mem::ManuallyDrop::new(text)
        })
        .collect()
}

/// Feeds every part to an assembler and keeps what arrived unfreed: what
/// is looked at is what the joining and inflating free.
fn scan_bbqr(parts: &[std::mem::ManuallyDrop<String>]) {
    let mut assembler = faraday_qr::Assembler::default();
    let mut last = None;
    for p in parts {
        last = Some(assembler.feed(p));
    }
    assert!(
        matches!(last, Some(faraday_qr::Step::Done(_))),
        "the parts did not complete"
    );
    std::mem::forget(last);
}

#[test]
fn a_seed_in_a_compressed_bbqr_leaves_no_freed_copy() {
    let text = PHRASE.as_bytes();
    let deflated = deflate_repeating(text, 40);
    let parts = bbqr_parts('U', &deflated, true);
    let forms = seed_forms("");
    let armed = arm(&forms);
    scan_bbqr(&parts);
    finish("compressed bbqr seed", armed, false);
}

#[test]
fn a_seed_in_a_bbqr_file_envelope_leaves_no_freed_copy() {
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "phrase as base64",
        faraday_bip::base64::encode(PHRASE.as_bytes()).into_bytes(),
    ));
    let envelope = faraday_qr::envelope::pack("seed-words.txt", PHRASE.as_bytes(), "file")
        .expect("an envelope");
    let parts = bbqr_parts('J', &envelope, false);
    let armed = arm(&forms);
    scan_bbqr(&parts);
    finish("bbqr envelope seed", armed, false);
}

#[test]
fn a_seed_over_comment_lines_beside_a_descriptor_classified_leaves_no_copy() {
    let descriptor = include_str!("../vectors/interop/bluewallet/coldcardmk4-descriptor.txt");
    let comments: Vec<String> = PHRASE.split(' ').map(|w| format!("# {w}")).collect();
    let file = format!("{}\n{descriptor}", comments.join("\n"));
    let c = testkit::compute();
    let forms = seed_forms("");
    let armed = arm(&forms);
    let kind = faraday_core::wallet::classify(&c, "wallet.txt", file.as_bytes());
    assert_eq!(kind, faraday_core::wallet::FileKind::SecretWallet);
    finish("seed in comments classified", armed, false);
}
