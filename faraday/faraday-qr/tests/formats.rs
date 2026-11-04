//! What Faraday writes reads back as what it was, and what does not hang
//! together is refused, saying why.

use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::keys::{MasterKey, Network};
use faraday_crypto::work::Compute;
use faraday_qr::{Arrived, Assembler, Step, bbqr, registry, specter};

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-qr's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

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
            what: faraday_qr::Transfer::Numbered,
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

/// A part number is digits only: a frame written `p+1of2` is not one of
/// SeedSigner's numbered parts and is not read as one.
#[test]
fn a_numbered_part_with_a_sign_is_not_read_as_one() {
    assert_eq!(specter::parse("p1of2 abc"), Some((1, 2, "abc")));
    for frame in ["p+1of2 abc", "p1of+2 abc", "pof2 abc", "p1of abc"] {
        assert_eq!(specter::parse(frame), None, "{frame}");
    }
}

/// Test key 1 (bacon ×24) at BIP-48's native multisig account on testnet.
fn bacon() -> (MasterKey, DerivationPath) {
    let c = compute();
    let words = vec!["bacon"; 24].join(" ");
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let master = MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), Network::Testnet);
    (master, "m/48'/1'/0'/2'".parse().unwrap())
}

#[test]
fn a_key_written_as_crypto_account_reads_back_as_its_key_line() {
    let c = compute();
    let (master, path) = bacon();
    let xpub = master.derive(&c, &path).to_xpub(&c);
    let comps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let cbor = registry::account_cbor(master.fingerprint().0, &comps, &xpub, false);
    let Some(registry::Read::Keys(text)) = registry::read("crypto-account", &cbor).unwrap() else {
        panic!("not keys");
    };
    let want = format!("[9a6a2580/48h/1h/0h/2h]{xpub}");
    assert!(text.lines().any(|l| l == want), "{text}");
    assert!(want.contains("]tpub"));
    // As a UR, Faraday's reader takes it, type and bytes intact.
    let text = faraday_codec::ur::encode_single("crypto-account", &cbor);
    let mut d = faraday_codec::ur::Decoder::new();
    assert_eq!(d.receive(&text), Ok(true));
    let Some(faraday_codec::ur::Message::Other { ur_type, cbor: got }) = d.message() else {
        panic!("not read");
    };
    assert_eq!(ur_type, "crypto-account");
    assert_eq!(got, &cbor);
}

#[test]
fn a_wallet_as_crypto_output_reads_as_its_descriptor() {
    use faraday_qr::cbor::{bytes, head};
    let c = compute();
    let (master, path) = bacon();
    let xpub = master.derive(&c, &path).to_xpub(&c);
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
    assert!(faraday_bip::policy::WalletPolicy::parse_any(&c, d.trim()).is_ok());
}

#[test]
fn a_seed_or_a_private_key_by_ur_is_refused_saying_what_it_is() {
    let seed = registry::read("crypto-seed", &[0xa0]).unwrap_err();
    assert_eq!(seed, registry::Error::Seed);
    // An hdkey that says it is private.
    let mut cbor = faraday_qr::cbor::head(5, 1);
    cbor.extend(faraday_qr::cbor::head(0, 2));
    cbor.push(0xf5);
    let private = registry::read("crypto-hdkey", &cbor).unwrap_err();
    assert_eq!(private, registry::Error::Private);
    assert_eq!(registry::read("crypto-unknown", &[0xa0]), Ok(None));
}

#[test]
fn an_hdkey_that_gives_its_key_twice_is_refused_as_two_readers_could_show_different_keys() {
    use faraday_qr::cbor::{bytes, head};
    // The curve's generator and its double, compressed.
    let hex = |h: &str| -> Vec<u8> {
        (0..h.len() / 2)
            .map(|i| u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).unwrap())
            .collect()
    };
    let one = hex("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798");
    let two = hex("02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5");
    let map = |entries: &[(&[u8], &[u8])]| {
        let mut cbor = head(5, entries.len() as u64);
        for (k, v) in entries {
            cbor.extend_from_slice(k);
            cbor.extend(bytes(v));
        }
        cbor
    };
    let chain = [7u8; 32];
    let (key, long_key, chain_key) = (head(0, 3), [0x18, 3], head(0, 4));
    // Tag 2, a bignum: the number 3 to a reader that knows the tag.
    let bignum_key = [0xc2, 0x41, 3];
    let once = map(&[(&key, &one), (&chain_key, &chain)]);
    assert!(registry::read("crypto-hdkey", &once).is_ok());
    let shape = registry::Error::Shape("crypto-hdkey".to_string());
    for twice in [
        map(&[(&key, &one), (&key, &two), (&chain_key, &chain)]),
        // The same key number written the long way is the same key.
        map(&[(&key, &one), (&long_key, &two), (&chain_key, &chain)]),
        // The same key number, to a second reader.
        map(&[(&key, &one), (&bignum_key, &two), (&chain_key, &chain)]),
    ] {
        assert_eq!(registry::read("crypto-hdkey", &twice), Err(shape.clone()));
    }
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

/// Hand-built CBOR for an hdkey around the generator point, so each test
/// changes one value and nothing else.
mod hdkey_cbor {
    use faraday_qr::cbor::{bytes, head};

    pub fn uint(n: u64) -> Vec<u8> {
        head(0, n)
    }

    /// A map of already-encoded `(key, value)` entries.
    pub fn map(entries: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let mut out = head(5, entries.len() as u64);
        for (k, v) in entries {
            out.extend(uint(*k));
            out.extend_from_slice(v);
        }
        out
    }

    pub fn tag(n: u64, inner: Vec<u8>) -> Vec<u8> {
        let mut out = head(6, n);
        out.extend(inner);
        out
    }

    /// A tagged key path: `components` as given, an optional source
    /// fingerprint and depth.
    pub fn keypath(
        components: Option<Vec<u8>>,
        source: Option<u64>,
        depth: Option<u64>,
    ) -> Vec<u8> {
        keypath_as(304, components, source, depth)
    }

    /// `keypath` under tag `n`.
    pub fn keypath_as(
        n: u64,
        components: Option<Vec<u8>>,
        source: Option<u64>,
        depth: Option<u64>,
    ) -> Vec<u8> {
        let mut e = Vec::new();
        if let Some(c) = components {
            e.push((1, c));
        }
        if let Some(s) = source {
            e.push((2, uint(s)));
        }
        if let Some(d) = depth {
            e.push((3, uint(d)));
        }
        tag(n, map(&e))
    }

    /// `[44, true, 0, hardened]`-style components.
    pub fn components(steps: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let mut out = head(4, steps.len() as u64 * 2);
        for (i, h) in steps {
            out.extend(uint(*i));
            out.extend_from_slice(h);
        }
        out
    }

    pub const TRUE: [u8; 1] = [0xf5];
    pub const FALSE: [u8; 1] = [0xf4];

    /// The hdkey's map with `extra` entries added, tagged 303.
    pub fn hdkey(extra: Vec<(u64, Vec<u8>)>) -> Vec<u8> {
        hdkey_as(303, extra)
    }

    /// `hdkey` under tag `n`.
    pub fn hdkey_as(n: u64, extra: Vec<(u64, Vec<u8>)>) -> Vec<u8> {
        let g = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
            0x5b, 0x16, 0xf8, 0x17, 0x98,
        ];
        let mut e = vec![(3, bytes(&g)), (4, bytes(&[7u8; 32]))];
        e.extend(extra);
        e.sort_by_key(|(k, _)| *k);
        tag(n, map(&e))
    }

    pub fn account(master: u64, outputs: Vec<Vec<u8>>) -> Vec<u8> {
        let mut arr = head(4, outputs.len() as u64);
        for o in outputs {
            arr.extend(o);
        }
        map(&[(1, uint(master)), (2, arr)])
    }
}

fn read_key(cbor: &[u8]) -> Result<String, registry::Error> {
    match registry::read("crypto-hdkey", cbor) {
        Ok(Some(registry::Read::Keys(t))) => Ok(t),
        Ok(other) => panic!("{other:?}"),
        Err(e) => Err(e),
    }
}

fn read_account(cbor: &[u8]) -> Result<String, registry::Error> {
    match registry::read("crypto-account", cbor) {
        Ok(Some(registry::Read::Keys(t))) => Ok(t),
        Ok(other) => panic!("{other:?}"),
        Err(e) => Err(e),
    }
}

#[test]
fn an_hdkey_with_a_path_the_registry_does_not_allow_is_refused_not_read_as_another_path() {
    use hdkey_cbor::*;
    let ok_path = components(&[(48, TRUE.to_vec())]);
    let origin = |c: Option<Vec<u8>>| vec![(6, keypath(c, Some(0x9a6a2580), None))];
    // A well-formed origin reads.
    let t = read_key(&hdkey(origin(Some(ok_path.clone())))).unwrap();
    assert!(t.contains("[9a6a2580/48h]"), "{t}");
    // The hardened flag is a boolean.
    let flag = components(&[(48, uint(1))]);
    assert_eq!(
        read_key(&hdkey(origin(Some(flag)))),
        Err(registry::Error::Path)
    );
    // The components are required, and are an array.
    assert_eq!(read_key(&hdkey(origin(None))), Err(registry::Error::Path));
    assert_eq!(
        read_key(&hdkey(origin(Some(uint(48))))),
        Err(registry::Error::Path)
    );
    // Children are a key path too.
    let children = vec![(7, keypath(None, None, None))];
    assert_eq!(read_key(&hdkey(children)), Err(registry::Error::Path));
    // A source fingerprint that is not a 32-bit number.
    let big = vec![(6, keypath(Some(ok_path.clone()), Some(1 << 32), None))];
    assert_eq!(read_key(&hdkey(big)), Err(registry::Error::Path));
    // Another tag around the path is not a key path.
    let retagged = tag(305, map(&[(1, ok_path.clone()), (2, uint(0x9a6a2580))]));
    assert_eq!(
        read_key(&hdkey(vec![(6, retagged)])),
        Err(registry::Error::Path)
    );
}

#[test]
fn an_hdkey_stating_a_depth_the_path_contradicts_or_a_wrong_network_is_refused() {
    use hdkey_cbor::*;
    let path = components(&[(48, TRUE.to_vec()), (1, TRUE.to_vec())]);
    let at = |depth: Option<u64>| vec![(6, keypath(Some(path.clone()), Some(1), depth))];
    assert!(read_key(&hdkey(at(Some(2)))).is_ok());
    // A depth other than the steps given is refused, deeper too: a partial
    // path would read as a false full origin.
    assert_eq!(read_key(&hdkey(at(Some(4)))), Err(registry::Error::Path));
    assert_eq!(read_key(&hdkey(at(Some(1)))), Err(registry::Error::Path));
    // A depth equal to the steps shows the origin as `[fp/path]`.
    let t = read_key(&hdkey(at(Some(2)))).unwrap();
    assert!(t.contains("[00000001/48h/1h]"), "{t}");
    // Network: 0 and 1 are Bitcoin's; another number is no network we know.
    let net = |n: Option<u64>| {
        let e = n.map(|n| vec![(2, uint(n))]).unwrap_or_default();
        vec![(5, tag(305, map(&e)))]
    };
    assert!(read_key(&hdkey(net(Some(0)))).is_ok());
    assert!(read_key(&hdkey(net(None))).is_ok());
    let test = read_key(&hdkey(net(Some(1)))).unwrap();
    assert!(test.contains("tpub"), "{test}");
    assert_eq!(read_key(&hdkey(net(Some(2)))), Err(registry::Error::Key));
    assert_eq!(
        read_key(&hdkey(vec![(5, uint(1))])),
        Err(registry::Error::Key)
    );
    // The parent fingerprint is a 32-bit number, and the privacy flag a bool.
    assert_eq!(
        read_key(&hdkey(vec![(8, uint(1 << 32))])),
        Err(registry::Error::Key)
    );
    assert_eq!(
        read_key(&hdkey(vec![(8, vec![0x41, 1])])),
        Err(registry::Error::Key)
    );
    assert_eq!(
        read_key(&hdkey(vec![(2, uint(0))])),
        Err(registry::Error::Key)
    );
}

#[test]
fn an_account_that_leaves_something_out_or_wears_another_tag_is_refused() {
    use hdkey_cbor::*;
    let origin = |fp: u64| {
        vec![(
            6,
            keypath(
                Some(components(&[(48, TRUE.to_vec()), (0, TRUE.to_vec())])),
                Some(fp),
                None,
            ),
        )]
    };
    let wsh = |fp: u64| tag(308, tag(401, hdkey(origin(fp))));
    assert!(read_account(&account(0x9a6a2580, vec![wsh(0x9a6a2580)])).is_ok());
    // An entry that is not a script expression is not dropped quietly.
    let with_junk = account(0x9a6a2580, vec![wsh(0x9a6a2580), uint(5)]);
    assert_eq!(read_account(&with_junk), Err(registry::Error::Wallet));
    // Another tag around the account, or around the key, is not that type.
    let tagged_account = tag(305, account(0x9a6a2580, vec![wsh(0x9a6a2580)]));
    assert_eq!(read_account(&tagged_account), Err(registry::Error::Wallet));
    let wrong_key = {
        let k = hdkey(origin(0x9a6a2580));
        // Re-wrap the map under tag 304.
        let inner = k[3..].to_vec();
        tag(308, tag(401, tag(304, inner)))
    };
    assert_eq!(
        read_account(&account(0x9a6a2580, vec![wrong_key])),
        Err(registry::Error::Wallet)
    );
    // The master fingerprint is a 32-bit number.
    assert_eq!(
        read_account(&account(1 << 32, vec![wsh(0x9a6a2580)])),
        Err(registry::Error::NoFingerprint)
    );
}

#[test]
fn a_key_whose_origin_names_another_master_than_its_account_still_reads() {
    use hdkey_cbor::*;
    // The specification makes a key's own source fingerprint the truth for
    // that key, so a different one in the account is allowed.
    let origin = vec![(
        6,
        keypath(
            Some(components(&[(48, TRUE.to_vec())])),
            Some(0x11111111),
            None,
        ),
    )];
    let out = tag(308, tag(401, hdkey(origin)));
    let t = read_account(&account(0x9a6a2580, vec![out])).unwrap();
    assert!(t.contains("[11111111/48h]"), "{t}");
}

#[test]
fn hdkey() {
    use hdkey_cbor::*;
    // BCR-2020-007's second vector, under the tags `ur:hdkey` gives its
    // types.
    let steps = components(&[
        (44, TRUE.to_vec()),
        (1, TRUE.to_vec()),
        (1, TRUE.to_vec()),
        (0, FALSE.to_vec()),
        (1, FALSE.to_vec()),
    ]);
    for source in [None, Some(0xe9181cf3)] {
        let e = vec![
            (5, tag(40305, map(&[(2, uint(1))]))),
            (6, keypath_as(40304, Some(steps.clone()), source, None)),
            (8, uint(3910671603)),
        ];
        let cbor = hdkey_as(40303, e);
        let Ok(Some(registry::Read::Keys(t))) = registry::read("hdkey", &cbor) else {
            panic!("refused");
        };
        assert!(t.contains("tpub"), "{t}");
        assert_eq!(
            t.contains("[e9181cf3/44h/1h/1h/0/1]"),
            source.is_some(),
            "{t}"
        );
    }
}

#[test]
fn an_hdkey_origin_that_could_pass_as_unknown_or_contradicts_itself_is_refused() {
    use hdkey_cbor::*;
    let one = components(&[(48, TRUE.to_vec())]);
    let with = |source: Option<u64>, parent: Option<u64>| {
        let mut e = vec![(6, keypath(Some(one.clone()), source, None))];
        if let Some(p) = parent {
            e.push((8, uint(p)));
        }
        read_key(&hdkey(e))
    };
    // Source fingerprint 0 stands for an unknown master.
    assert_eq!(with(Some(0), None), Err(registry::Error::Path));
    // With one step the parent is the master: identical where both given.
    assert!(with(Some(5), Some(5)).is_ok());
    assert!(with(Some(5), None).is_ok());
    assert_eq!(with(Some(5), Some(6)), Err(registry::Error::Path));
    // Parent fingerprint 0 is read (a depth-0 key has one).
    assert!(read_key(&hdkey(vec![(8, uint(0))])).is_ok());
}

#[test]
fn an_hdkey_with_an_odd_coin_type_or_master_flag_is_refused() {
    use hdkey_cbor::*;
    let coin = |c: Vec<u8>| read_key(&hdkey(vec![(5, tag(305, map(&[(1, c)])))]));
    assert!(coin(uint(0)).is_ok());
    assert!(coin(uint(1)).is_ok());
    assert_eq!(coin(uint(2)), Err(registry::Error::Key));
    assert_eq!(coin(vec![0xf5]), Err(registry::Error::Key));
    // `is-master` is a bool, and a master key is private.
    assert!(read_key(&hdkey(vec![(1, FALSE.to_vec())])).is_ok());
    assert_eq!(
        read_key(&hdkey(vec![(1, TRUE.to_vec())])),
        Err(registry::Error::Key)
    );
    assert_eq!(
        read_key(&hdkey(vec![(1, uint(1))])),
        Err(registry::Error::Key)
    );
}

#[test]
fn an_hdkey_with_private_key_data_says_it_is_private_whatever_its_flags_say() {
    use faraday_qr::cbor::bytes;
    use hdkey_cbor::*;
    let mut private = [7u8; 33];
    private[0] = 0;
    let chain = (4, bytes(&[7u8; 32]));
    let key = (3, bytes(&private));
    let read = |e: Vec<(u64, Vec<u8>)>| read_key(&tag(303, map(&e)));
    let master = (1, TRUE.to_vec());
    assert_eq!(
        read(vec![master, key.clone(), chain.clone()]),
        Err(registry::Error::Private)
    );
    assert_eq!(
        read(vec![key.clone(), chain.clone()]),
        Err(registry::Error::Private)
    );
    assert_eq!(
        read(vec![(2, FALSE.to_vec()), key, chain]),
        Err(registry::Error::Private)
    );
}

#[test]
fn an_hdkey_with_long_key_data_a_parent_at_depth_0_or_a_wildcard_origin_is_refused() {
    use faraday_qr::cbor::{bytes, head};
    use hdkey_cbor::*;
    // Key data of 65 bytes, a real point written uncompressed.
    let mut long = vec![0x04];
    long.extend([
        0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b,
        0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8,
        0x17, 0x98, 0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65, 0x5d, 0xa4, 0xfb, 0xfc, 0x0e,
        0x11, 0x08, 0xa8, 0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19, 0x9c, 0x47, 0xd0, 0x8f,
        0xfb, 0x10, 0xd4, 0xb8,
    ]);
    let e = [(3, bytes(&long)), (4, bytes(&[7u8; 32]))];
    assert_eq!(read_key(&tag(303, map(&e))), Err(registry::Error::Key));
    // At depth 0 BIP-32 allows no parent but 0.
    assert_eq!(
        read_key(&hdkey(vec![(8, uint(5))])),
        Err(registry::Error::Key)
    );
    assert!(read_key(&hdkey(vec![(8, uint(0))])).is_ok());
    // A wildcard is a step of children, never of an origin.
    let mut star = head(4, 4);
    star.extend(uint(48));
    star.extend(TRUE);
    star.extend(head(4, 0));
    star.extend(FALSE);
    let e = (6, keypath(Some(star), Some(1), None));
    assert_eq!(read_key(&hdkey(vec![e])), Err(registry::Error::Path));
}
