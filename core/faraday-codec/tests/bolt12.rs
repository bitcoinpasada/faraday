//! BOLT #12 offers: every vector of the specification, valid and invalid
//! (`tools/vectors/lightning-bolt12`), read as a person pasting or scanning
//! an offer would see it, and the refusals Faraday adds.

use faraday_codec::bolt12::{Amount, Chain, Error, FirstNode, Offer, decode};

const OFFERS: &str = include_str!("../../../tools/vectors/lightning-bolt12/offers-test.json");
const FORMATS: &str =
    include_str!("../../../tools/vectors/lightning-bolt12/format-string-test.json");
const NODE: &str = "02eec7245d6b7d2ccb30380bfbe2a3648cd7a942653f5aa340edcea1f283686619";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn offers() -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(OFFERS)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn offer(containing: &str) -> Offer {
    let e = offers()
        .into_iter()
        .find(|e| e["description"].as_str().unwrap().contains(containing))
        .unwrap();
    decode(e["bolt12"].as_str().unwrap()).unwrap()
}

fn tu(v: &[u8]) -> u64 {
    v.iter().fold(0, |a, &x| a << 8 | u64::from(x))
}

/// The offer's text for these fields, as a writer would make it.
fn text(fields: &[(u8, &[u8])]) -> String {
    let mut bytes = Vec::new();
    for (kind, v) in fields {
        bytes.push(*kind);
        bytes.push(v.len() as u8);
        bytes.extend_from_slice(v);
    }
    let charset = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let (mut out, mut acc, mut bits) = (String::from("lno1"), 0u32, 0);
    for b in bytes {
        acc = acc << 8 | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(charset[(acc >> bits & 31) as usize] as char);
        }
        acc &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(charset[(acc << (5 - bits) & 31) as usize] as char);
    }
    out
}

fn issuer() -> Vec<u8> {
    unhex(NODE)
}

#[test]
fn every_offer_of_the_specification_is_read_or_refused() {
    for e in offers() {
        let what = e["description"].as_str().unwrap();
        let result = decode(e["bolt12"].as_str().unwrap());
        if !e["valid"].as_bool().unwrap() {
            assert!(result.is_err(), "{what} was read");
            continue;
        }
        let o = result.unwrap_or_else(|err| panic!("{what}: {err:?}"));
        for f in e["fields"].as_array().unwrap() {
            let (kind, v) = (
                f["type"].as_u64().unwrap(),
                unhex(f["hex"].as_str().unwrap()),
            );
            assert_eq!(f["length"].as_u64().unwrap() as usize, v.len(), "{what}");
            match kind {
                2 => assert_eq!(
                    hex(&o.chains.iter().flat_map(|c| c.hash()).collect::<Vec<_>>()),
                    hex(&v),
                    "{what}"
                ),
                4 => assert_eq!(o.metadata.as_deref(), Some(&v[..]), "{what}"),
                6 => assert!(
                    matches!(&o.amount, Some(Amount::Currency { code, .. }) if code.as_bytes() == v),
                    "{what}"
                ),
                8 => match o.amount.as_ref().unwrap() {
                    Amount::Bitcoin { msat } => assert_eq!(*msat, tu(&v), "{what}"),
                    Amount::Currency { units, .. } => assert_eq!(*units, tu(&v), "{what}"),
                },
                10 => assert_eq!(o.description.as_deref().unwrap().as_bytes(), v, "{what}"),
                12 => {
                    let top = v.len() as u32 * 8;
                    let set: Vec<u32> = (0..top)
                        .filter(|b| v[v.len() - 1 - (b / 8) as usize] >> (b % 8) & 1 == 1)
                        .collect();
                    assert_eq!(o.features, set, "{what}");
                }
                14 => assert_eq!(o.expiry, Some(tu(&v)), "{what}"),
                16 => {
                    let mut want: Vec<(Vec<u8>, u8)> = Vec::new();
                    let mut at = 0;
                    while at < v.len() {
                        let n = if v[at] < 2 { 9 } else { 33 };
                        let first = v[at..at + n].to_vec();
                        at += n + 33;
                        let hops = v[at];
                        at += 1;
                        for _ in 0..hops {
                            at += 33;
                            at += 2 + (usize::from(v[at]) << 8 | usize::from(v[at + 1]));
                        }
                        want.push((first, hops));
                    }
                    let got: Vec<(Vec<u8>, u8)> = o
                        .paths
                        .iter()
                        .map(|p| {
                            let first = match p.first {
                                FirstNode::Node(n) => n.to_vec(),
                                FirstNode::Channel {
                                    short_channel_id,
                                    direction,
                                } => [&[direction][..], &short_channel_id.to_be_bytes()].concat(),
                            };
                            (first, p.hops)
                        })
                        .collect();
                    assert_eq!(got, want, "{what}");
                }
                18 => assert_eq!(o.issuer.as_deref().unwrap().as_bytes(), v, "{what}"),
                20 => assert_eq!(o.quantity_max, Some(tu(&v)), "{what}"),
                22 => assert_eq!(hex(&o.issuer_id.unwrap()), hex(&v), "{what}"),
                _ => {
                    let u = o.unknown.iter().find(|u| u.kind == kind).expect(what);
                    assert_eq!(u.data, v, "{what}");
                }
            }
        }
    }
}

#[test]
fn the_text_form_follows_the_format_vectors() {
    let list: Vec<serde_json::Value> = serde_json::from_str(FORMATS).unwrap();
    let first = decode(list[0]["string"].as_str().unwrap()).unwrap();
    for e in list {
        let what = e["comment"].as_str().unwrap();
        let result = decode(e["string"].as_str().unwrap());
        if e["valid"].as_bool().unwrap() {
            assert_eq!(result.as_ref(), Ok(&first), "{what}");
        } else {
            assert!(result.is_err(), "{what}");
        }
    }
}

#[test]
fn the_fields_read_as_the_vectors_describe_them() {
    let o = offer("Minimal");
    assert_eq!(hex(&o.issuer_id.unwrap()), NODE);
    assert!(o.chains.is_empty() && o.amount.is_none() && o.description.is_none());
    assert_eq!(offer("for testnet").chains, [Chain::Testnet3]);
    assert_eq!(
        offer("bitcoin or liquidv1").chains[1],
        Chain::Bitcoin,
        "the other chain is kept"
    );
    assert!(matches!(
        offer("bitcoin or liquidv1").chains[0],
        Chain::Other(_)
    ));
    assert_eq!(
        offer("with amount").amount,
        Some(Amount::Bitcoin { msat: 10000 })
    );
    assert_eq!(
        offer("with currency").amount,
        Some(Amount::Currency {
            code: "USD".into(),
            exponent: 2,
            units: 10000
        })
    );
    assert_eq!(offer("with expiry").expiry, Some(2_051_184_600));
    assert_eq!(
        offer("with issuer").issuer.as_deref(),
        Some("https://bolt12.org BOLT12 industries")
    );
    assert_eq!(offer("with quantity").quantity_max, Some(5));
    assert_eq!(offer("unlimited").quantity_max, Some(0));
    assert_eq!(offer("with feature").features, [99]);
    assert_eq!(
        offer("with description").description.as_deref(),
        Some("Test vectors")
    );
}

#[test]
fn paths_show_their_first_node_and_hops() {
    let o = offer("with blinded path via Bob");
    assert_eq!(o.paths.len(), 1);
    assert_eq!(o.paths[0].hops, 2);
    assert!(matches!(o.paths[0].first, FirstNode::Node(n) if matches!(n[0], 2 | 3)));
    assert!(o.issuer_id.is_some());
    let o = offer("sciddir");
    assert_eq!(
        o.paths[0].first,
        FirstNode::Channel {
            short_channel_id: 42,
            direction: 0
        }
    );
    let o = offer("second blinded path");
    assert_eq!(o.paths.len(), 2);
    assert_eq!(
        o.paths[1].first,
        FirstNode::Channel {
            short_channel_id: 1 << 40 | 2 << 16 | 3,
            direction: 1
        }
    );
    assert_eq!(offer("no issuer_id").issuer_id, None);
}

#[test]
fn unknown_odd_fields_are_listed() {
    let o = offer("unknown odd field");
    assert_eq!(
        (o.unknown[0].kind, o.unknown[0].data.as_slice()),
        (33, &b"helloworld"[..])
    );
    assert_eq!(offer("experimental").unknown[0].kind, 1_000_000_033);
}

#[test]
fn an_invoice_request_and_other_prefixes_are_refused_with_their_reason() {
    let o = &offers()[0];
    let t = o["bolt12"].as_str().unwrap();
    assert_eq!(
        decode(&t.replacen("lno", "lnr", 1)),
        Err(Error::InvoiceRequest)
    );
    assert_eq!(decode(&t.replacen("lno", "lni", 1)), Err(Error::Prefix));
    assert_eq!(decode(&t.replacen("lno", "lnz", 1)), Err(Error::Prefix));
    assert_eq!(decode("lno"), Err(Error::Bech32));
    assert!(decode(&format!("lightning:{t}")).is_ok());
    assert!(decode(&format!("  LIGHTNING:{}\n", t.to_uppercase())).is_ok());
}

#[test]
fn what_bolt12_leaves_open_is_refused() {
    let id = issuer();
    let ok = |extra: &[(u8, &[u8])]| {
        let mut f = extra.to_vec();
        f.push((22, &id));
        decode(&text(&f))
    };
    assert!(ok(&[]).is_ok());
    let main = faraday_codec::bolt12::Chain::Bitcoin.hash();
    let t3 = Chain::Testnet3.hash();
    assert_eq!(
        ok(&[(2, &[main, main].concat())]),
        Err(Error::ChainRepeated)
    );
    assert_eq!(ok(&[(2, &[7u8; 32])]), Err(Error::ChainUnknown));
    assert!(ok(&[(2, &[[7u8; 32], t3].concat())]).is_ok());
    assert_eq!(ok(&[(2, &[7u8; 31])]), Err(Error::Field(2)));
    assert_eq!(ok(&[(2, &[])]), Err(Error::ChainsEmpty));
    assert_eq!(
        ok(&[(6, b"XYZ"), (8, &[5]), (10, b"x")]),
        Err(Error::Currency)
    );
    assert_eq!(
        ok(&[(6, b"usd"), (8, &[5]), (10, b"x")]),
        Err(Error::Currency)
    );
    assert!(ok(&[(6, b"JPY"), (8, &[5]), (10, b"x")]).is_ok());
    assert_eq!(
        ok(&[(6, b"USD"), (10, b"x")]),
        Err(Error::CurrencyWithoutAmount)
    );
    assert_eq!(ok(&[(8, &[5])]), Err(Error::AmountWithoutDescription));
    assert_eq!(ok(&[(8, &[]), (10, b"x")]), Err(Error::AmountZero));
    assert_eq!(decode(&text(&[(16, &[]), (22, &id)])), Err(Error::Path));
    assert_eq!(ok(&[(8, &[0, 5]), (10, b"x")]), Err(Error::NotMinimal));
    assert_eq!(
        ok(&[(8, &[1, 0, 0, 0, 0, 0, 0, 0, 0]), (10, b"x")]),
        Err(Error::Field(8))
    );
    let big = 2_100_000_000_000_000_001u64.to_be_bytes();
    assert_eq!(ok(&[(8, &big), (10, b"x")]), Err(Error::AmountTooLarge));
    let max = 2_100_000_000_000_000_000u64.to_be_bytes();
    assert!(ok(&[(8, &max), (10, b"x")]).is_ok());
    let late = 253_402_300_800u64.to_be_bytes();
    assert_eq!(ok(&[(14, &late[3..])]), Err(Error::ExpiryTooLate));
    let last = 253_402_300_799u64.to_be_bytes();
    assert!(ok(&[(14, &last[3..])]).is_ok());
    assert_eq!(ok(&[(12, &[0, 1])]), Err(Error::FeaturesNotMinimal));
    assert_eq!(ok(&[(12, &[2])]).unwrap().features, [1]);
    assert_eq!(ok(&[(12, &[4])]), Err(Error::UnknownRequiredFeature(2)));
    assert_eq!(ok(&[(12, &[1, 0])]), Err(Error::UnknownRequiredFeature(8)));
    let after = |f: &[(u8, &[u8])]| decode(&text(&[&[(22, &id[..])], f].concat()));
    assert_eq!(after(&[(33, b"x"), (35, b"")]).unwrap().unknown.len(), 2);
    assert_eq!(after(&[(34, b"x")]), Err(Error::UnknownEven(34)));
    assert_eq!(after(&[(80, b"x")]), Err(Error::OutOfRange(80)));
}

#[test]
fn text_fields_hold_no_control_or_unassigned_characters() {
    let id = issuer();
    let with = |d: &str| decode(&text(&[(10, d.as_bytes()), (22, &id)]));
    assert_eq!(
        with("Coffee, 2 cups").unwrap().description.unwrap(),
        "Coffee, 2 cups"
    );
    assert!(with("caf\u{e9} \u{65e5}\u{672c}").is_ok());
    for bad in [
        "a\nb",
        "a\u{7f}b",
        "a\u{200b}b",
        "a\u{2028}b",
        "a\u{2029}b",
        "a\u{e000}b",
        "a\u{378}b",
        "a\u{ffff}b",
    ] {
        assert_eq!(with(bad), Err(Error::Text(10)), "{bad:?}");
    }
    assert_eq!(
        decode(&text(&[(10, &[0xff, 0xfe]), (22, &id)])),
        Err(Error::Text(10))
    );
    assert_eq!(
        decode(&text(&[(18, b"a\x01"), (22, &id)])),
        Err(Error::Text(18))
    );
}

#[test]
fn the_text_is_bounded_and_its_padding_is_zero() {
    let t = offers()[0]["bolt12"].as_str().unwrap().to_string();
    assert!(
        decode(&format!("{t}{}", " ".repeat(10))).is_ok(),
        "ends are trimmed"
    );
    assert_eq!(
        decode(&format!("{t}{}", "q".repeat(4300))),
        Err(Error::TooLong)
    );
    let mut mixed = t.clone();
    mixed.replace_range(5..6, &mixed[5..6].to_uppercase());
    assert_eq!(decode(&mixed), Err(Error::MixedCase));
    // A text whose last character carries padding bits: set one of them.
    let id = issuer();
    for n in 0..8 {
        let t = text(&[(10, &vec![b'x'; n]), (22, &id)]);
        if ((t.len() - 4) * 5).is_multiple_of(8) {
            continue;
        }
        assert!(decode(&t).is_ok());
        let charset = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
        let last = t.chars().last().unwrap();
        let v = charset.find(last).unwrap();
        let set = charset.chars().nth(v | 1).unwrap();
        let spoiled = format!("{}{set}", &t[..t.len() - 1]);
        assert_eq!(decode(&spoiled), Err(Error::Padding));
        return;
    }
    panic!("no padding");
}

#[test]
fn a_plus_joins_only_between_bech32_characters() {
    let t = text(&[(22, &issuer())]);
    assert!(decode(&format!("{}+{}", &t[..6], &t[6..])).is_ok());
    assert_eq!(decode(&format!("ln+{}", &t[2..])), Err(Error::Join));
    assert_eq!(decode(&format!("lno+{}", &t[3..])), Err(Error::Join));
}

#[test]
fn a_chain_hash_is_the_genesis_block_of_its_network() {
    use bitcoin::blockdata::constants::genesis_block;
    use bitcoin::hashes::Hash;
    for (chain, net) in [
        (Chain::Bitcoin, bitcoin::Network::Bitcoin),
        (Chain::Testnet3, bitcoin::Network::Testnet),
        (Chain::Testnet4, bitcoin::Network::Testnet4),
        (Chain::Signet, bitcoin::Network::Signet),
        (Chain::Regtest, bitcoin::Network::Regtest),
    ] {
        assert_eq!(
            chain.hash(),
            genesis_block(net).block_hash().to_byte_array()
        );
    }
}

#[test]
fn the_currency_table_is_the_published_iso_4217_list() {
    const LIST: &str = include_str!("../../../tools/vectors/lightning-bolt12/iso4217-list-one.xml");
    let tag = |entry: &str, name: &str| -> Option<String> {
        let open = format!("<{name}>");
        let from = entry.find(&open)? + open.len();
        let to = entry[from..].find(&format!("</{name}>"))?;
        Some(entry[from..from + to].to_string())
    };
    let mut want = std::collections::BTreeMap::new();
    for entry in LIST.split("<CcyNtry>").skip(1) {
        let (Some(code), Some(units)) = (tag(entry, "Ccy"), tag(entry, "CcyMnrUnts")) else {
            continue;
        };
        if let Ok(units) = units.parse::<u8>() {
            want.insert(code, units);
        }
    }
    let have: Vec<(String, u8)> = faraday_codec::bolt12::CURRENCIES
        .iter()
        .map(|(c, u)| (c.to_string(), *u))
        .collect();
    assert_eq!(have, want.into_iter().collect::<Vec<_>>());
}
