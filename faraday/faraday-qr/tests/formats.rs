//! What Faraday writes reads back as what it was, and what does not hang
//! together is refused, saying why.

use faraday_qr::{Arrived, Assembler, Step, bbqr, registry, specter};
use osk_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use osk_bip::keys::{MasterKey, Network};

fn feed_all(parts: &[String]) -> Step {
    let mut a = Assembler::default();
    let mut last = Step::NotMine;
    for p in parts {
        last = a.feed(p);
    }
    last
}

#[test]
fn a_psbt_written_as_bbqr_reads_back() {
    let psbt: Vec<u8> = (0..3_000u32).map(|i| (i * 7) as u8).collect();
    let parts = bbqr::encode('P', &psbt, 200).unwrap();
    assert_eq!(parts.len(), 15);
    assert!(parts.iter().all(|p| p.starts_with("B$2P0F")));
    assert_eq!(feed_all(&parts), Step::Done(Arrived::Psbt(psbt)));
}

#[test]
fn bbqr_parts_that_do_not_hang_together_are_refused() {
    let one = bbqr::encode('P', &[1u8; 500], 100).unwrap();
    let other = bbqr::encode('T', &[2u8; 500], 100).unwrap();
    let mut a = Assembler::default();
    a.feed(&one[0]);
    // A part of another transfer.
    assert!(matches!(a.feed(&other[1]), Step::Refused(_)));
    // The same part with other contents.
    let mut a = Assembler::default();
    a.feed(&one[0]);
    let swap = if one[0].as_bytes()[8] == b'Z' {
        'Y'
    } else {
        'Z'
    };
    let changed = format!("{}{swap}{}", &one[0][..8], &one[0][9..]);
    assert!(matches!(a.feed(&changed), Step::Refused(_)));
    // A type Faraday does not read, and a header that is not BBQr.
    assert!(matches!(
        feed_all(&["B$2C0100AAAAAAAA".into()]),
        Step::Refused(_)
    ));
    assert!(matches!(
        feed_all(&["B$9P0100AAAAAAAA".into()]),
        Step::Refused(_)
    ));
    // A frame over the limit.
    assert!(matches!(feed_all(&["B".repeat(5_000)]), Step::Refused(_)));
}

#[test]
fn numbered_parts_read_back_and_say_what_is_missing() {
    let text = "wsh(sortedmulti(2,[aaaaaaaa/48h/0h/0h/2h]xpub/<0;1>/*))".repeat(4);
    let parts = specter::encode(&text, 60);
    assert_eq!(parts.len(), 4);
    let mut a = Assembler::default();
    assert_eq!(
        a.feed(&parts[2]),
        Step::Part {
            what: "numbered parts",
            have: 1,
            total: 4,
            missing: vec![1, 2, 4],
        }
    );
    for p in [&parts[0], &parts[1]] {
        a.feed(p);
    }
    assert_eq!(a.feed(&parts[3]), Step::Done(Arrived::Text(text)));
}

/// Test key 1 (bacon ×24) at BIP-48's native multisig account on testnet.
fn bacon() -> (MasterKey, DerivationPath) {
    let words = vec!["bacon"; 24].join(" ");
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words).unwrap();
    let master = MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Testnet);
    (master, "m/48'/1'/0'/2'".parse().unwrap())
}

#[test]
fn a_key_written_as_crypto_account_reads_back_as_its_key_line() {
    let (master, path) = bacon();
    let xpub = master.derive(&path).to_xpub();
    let comps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let cbor = registry::account_cbor(master.fingerprint().0, &comps, &xpub, false);
    let Some(registry::Read::Keys(text)) = registry::read("crypto-account", &cbor).unwrap() else {
        panic!("not keys");
    };
    let want = format!("[9a6a2580/48h/1h/0h/2h]{xpub}");
    assert!(text.lines().any(|l| l == want), "{text}");
    assert!(want.contains("]tpub"));
}

#[test]
fn a_wallet_as_crypto_output_reads_as_its_descriptor() {
    use faraday_qr::cbor::{bytes, head};
    let (master, path) = bacon();
    let xpub = master.derive(&path).to_xpub();
    // wsh(sortedmulti(1, key/0/*)) as Sparrow writes it: tags 308, 401,
    // 407, and the key a tagged hdkey with origin and children.
    let uint = |n: u64| head(0, n);
    let mut keypath = head(5, 3);
    keypath.extend(uint(1));
    keypath.extend(head(4, 8));
    for i in [48u64, 1, 0, 2] {
        keypath.extend(uint(i));
        keypath.push(0xf5);
    }
    keypath.extend(uint(2));
    keypath.extend(uint(u64::from(u32::from_be_bytes(master.fingerprint().0))));
    keypath.extend(uint(3));
    keypath.extend(uint(4));
    let mut children = head(5, 1);
    children.extend(uint(1));
    children.extend(head(4, 4));
    children.extend(uint(0));
    children.push(0xf4);
    children.extend(head(4, 0));
    children.push(0xf4);
    let mut hd = head(6, 303);
    hd.extend(head(5, 6));
    hd.extend(uint(3));
    hd.extend(bytes(&xpub.public_key.serialize()));
    hd.extend(uint(4));
    hd.extend(bytes(xpub.chain_code.as_bytes()));
    hd.extend(uint(5));
    hd.extend(head(6, 305));
    hd.extend(head(5, 1));
    hd.extend(uint(2));
    hd.extend(uint(1));
    hd.extend(uint(6));
    hd.extend(head(6, 304));
    hd.extend(keypath);
    hd.extend(uint(7));
    hd.extend(head(6, 304));
    hd.extend(children);
    hd.extend(uint(8));
    hd.extend(uint(u64::from(u32::from_be_bytes(
        *xpub.parent_fingerprint.as_bytes(),
    ))));
    let mut cbor = head(6, 308);
    cbor.extend(head(6, 401));
    cbor.extend(head(6, 407));
    cbor.extend(head(5, 2));
    cbor.extend(uint(1));
    cbor.extend(uint(1));
    cbor.extend(uint(2));
    cbor.extend(head(4, 1));
    cbor.extend(hd);
    let Some(registry::Read::Descriptor(d)) = registry::read("crypto-output", &cbor).unwrap()
    else {
        panic!("not a descriptor");
    };
    assert_eq!(
        d.trim(),
        format!("wsh(sortedmulti(1,[9a6a2580/48h/1h/0h/2h]{xpub}/<0;1>/*))")
    );
    // And it is a wallet Faraday reads.
    assert!(osk_bip::policy::WalletPolicy::parse_any(d.trim()).is_ok());
}

#[test]
fn a_seed_or_a_private_key_by_ur_is_refused_saying_what_it_is() {
    let seed = registry::read("crypto-seed", &[0xa0]).unwrap_err();
    assert!(seed.contains("seed"), "{seed}");
    // An hdkey that says it is private.
    let mut cbor = faraday_qr::cbor::head(5, 1);
    cbor.extend(faraday_qr::cbor::head(0, 2));
    cbor.push(0xf5);
    let private = registry::read("crypto-hdkey", &cbor).unwrap_err();
    assert!(private.contains("private key"), "{private}");
    assert_eq!(registry::read("crypto-unknown", &[0xa0]), Ok(None));
}

#[test]
fn a_finished_transaction_in_hex_is_known_as_one() {
    let tx = "0200000001000000000000000000000000000000000000000000000000000000000000000000000000\
              00ffffffff01a086010000000000160014f6672856b1b206a8f45aa636ab12d6b25be6b02600000000";
    assert!(matches!(
        faraday_qr::classify_text(tx),
        Arrived::Transaction(_)
    ));
    assert!(matches!(
        faraday_qr::classify_text("deadbeef"),
        Arrived::Text(_)
    ));
}

#[test]
fn a_key_as_ur_crypto_account_is_read_by_osk_codec_and_back_here() {
    let (master, path) = bacon();
    let xpub = master.derive(&path).to_xpub();
    let comps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let cbor = registry::account_cbor(master.fingerprint().0, &comps, &xpub, false);
    let text = faraday_qr::ur::single("crypto-account", &cbor);
    // OpenSigner's own UR reader takes it, type and bytes intact.
    let mut d = osk_codec::ur::Decoder::new();
    assert_eq!(d.receive(&text), Ok(true));
    let Some(osk_codec::ur::Message::Other { ur_type, cbor: got }) = d.message() else {
        panic!("not read");
    };
    assert_eq!(ur_type, "crypto-account");
    assert_eq!(got, &cbor);
}
