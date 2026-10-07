//! MuSig2 key aggregation and `musig()` descriptors against the
//! published vectors.
//!
//! - `tools/vectors/bip327/key_agg_vectors.json` and
//!   `key_sort_vectors.json`: BIP-327's own `KeyAgg` and `KeySort`
//!   vectors, valid cases and error cases.
//! - `tools/vectors/bip328/vectors.txt`: BIP-328's aggregate keys and the
//!   synthetic extended public keys that carry them.
//! - `tools/vectors/bip390/descriptors.txt`: BIP-390's descriptors and
//!   the scripts they pay to, extracted from the BIP.

mod common;

use common::{hex, unhex};
use osk_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use osk_bip::keys::Network;
use osk_bip::musig::{
    self, AggNonce, MusigExpr, PartialSig, PubNonce, SecNonce, Tweak, aggregate_xpub, key_agg,
    parse_musig_expr, parse_tr_musig, script_pubkey_at, sort_keys,
};
use serde_json::Value;

const KEY_AGG: &str = include_str!("../../../tools/vectors/bip327/key_agg_vectors.json");
const KEY_SORT: &str = include_str!("../../../tools/vectors/bip327/key_sort_vectors.json");
const BIP328: &str = include_str!("../../../tools/vectors/bip328/vectors.txt");
const BIP390: &str = include_str!("../../../tools/vectors/bip390/descriptors.txt");

/// The BIP-390 descriptors this crate does not build a script for: a
/// participant written as a WIF private key, a taproot script tree, and
/// a `musig()` inside a tap leaf. Each must be refused rather than read
/// as something else.
const REFUSED: &[&str] = &[
    "rawtr(musig(KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU74sHUHy8S,03dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659,023590a94e768f8e1815c2f24b4d80a8e3149316c3518ce7b7ad338368d038ca66))",
    "tr(musig(xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL,xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y)/0/*,pk(f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9))",
    "tr(f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9,pk(musig(xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL,xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y)/0/*))",
];

fn keys_of(json: &Value, field: &str) -> Vec<String> {
    json[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().to_lowercase())
        .collect()
}

fn parse_key(text: &str) -> Option<PublicKey> {
    PublicKey::from_slice(&unhex(text)).ok()
}

/// Every valid `KeyAgg` case aggregates to the key the BIP states, in
/// the order the case gives the keys in.
#[test]
fn bip327_key_agg_vectors() {
    let json: Value = serde_json::from_str(KEY_AGG).unwrap();
    let pubkeys = keys_of(&json, "pubkeys");
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = case["key_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| parse_key(&pubkeys[i.as_u64().unwrap() as usize]).unwrap())
            .collect();
        let aggregate = key_agg(&keys).unwrap();
        assert_eq!(
            hex(&aggregate.serialize_x_only()),
            case["expected"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// Every error case is refused: a participant key that is no point on
/// the curve never reaches aggregation, and a tweak out of range or one
/// that lands on infinity fails the tweak.
#[test]
fn bip327_key_agg_error_vectors() {
    let json: Value = serde_json::from_str(KEY_AGG).unwrap();
    let pubkeys = keys_of(&json, "pubkeys");
    let tweaks = keys_of(&json, "tweaks");
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for case in json["error_test_cases"].as_array().unwrap() {
        let parsed: Vec<Option<PublicKey>> = case["key_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| parse_key(&pubkeys[i.as_u64().unwrap() as usize]))
            .collect();
        if parsed.iter().any(Option::is_none) {
            checked += 1;
            continue;
        }
        let keys: Vec<PublicKey> = parsed.into_iter().map(Option::unwrap).collect();
        let mut state = key_agg(&keys);
        for (t, x_only) in case["tweak_indices"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["is_xonly"].as_array().unwrap())
        {
            let tweak: [u8; 32] = unhex(&tweaks[t.as_u64().unwrap() as usize])
                .try_into()
                .unwrap();
            state = state.and_then(|key| key.apply_tweak(&secp, &tweak, x_only.as_bool().unwrap()));
        }
        assert!(state.is_err(), "{case} was accepted");
        checked += 1;
    }
    assert_eq!(checked, 5);
}

/// `KeySort` puts the keys in the order the BIP's vector states, with
/// repeated keys kept.
#[test]
fn bip327_key_sort_vectors() {
    let json: Value = serde_json::from_str(KEY_SORT).unwrap();
    let keys: Vec<PublicKey> = keys_of(&json, "pubkeys")
        .iter()
        .map(|k| parse_key(k).unwrap())
        .collect();
    let expected = keys_of(&json, "sorted_pubkeys");
    let sorted: Vec<String> = sort_keys(&keys)
        .iter()
        .map(|k| hex(&k.serialize()))
        .collect();
    assert_eq!(sorted, expected);
}

/// Every BIP-328 case: the aggregate of the participants is the key the
/// BIP states, and the synthetic extended public key is the one it
/// publishes.
#[test]
fn bip328_synthetic_xpub_vectors() {
    let mut checked = 0;
    for line in BIP328.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let keys: Vec<PublicKey> = fields
            .next()
            .unwrap()
            .split(',')
            .map(|k| parse_key(&k.to_lowercase()).unwrap())
            .collect();
        let aggregate = fields.next().unwrap();
        let xpub = fields.next().unwrap();
        let synthetic = aggregate_xpub(&keys, Network::Mainnet).unwrap();
        assert_eq!(hex(&synthetic.public_key.serialize()), aggregate);
        assert_eq!(synthetic.to_string(), xpub);
        checked += 1;
    }
    assert_eq!(checked, 3);
}

struct Case {
    descriptor: String,
    scripts: Vec<String>,
    valid: bool,
}

fn bip390_cases() -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();
    for line in BIP390.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(descriptor) = line.strip_prefix("valid ") {
            cases.push(Case {
                descriptor: descriptor.into(),
                scripts: Vec::new(),
                valid: true,
            });
        } else if let Some(descriptor) = line.strip_prefix("invalid ") {
            cases.push(Case {
                descriptor: descriptor.into(),
                scripts: Vec::new(),
                valid: false,
            });
        } else {
            cases.last_mut().unwrap().scripts.push(line.into());
        }
    }
    cases
}

/// The x-only key a `rawtr()` pays to directly, as a script.
fn raw_script(expr: &MusigExpr, index: u32) -> String {
    let secp = Secp256k1::verification_only();
    let key = expr.key_at(&secp, false, index).unwrap();
    format!("5120{}", hex(&key.x_only_public_key().0.serialize()))
}

/// Every BIP-390 descriptor this crate builds pays to the script the BIP
/// lists, at each index the BIP lists one for.
#[test]
fn bip390_descriptors_produce_their_scripts() {
    let mut checked = 0;
    for case in bip390_cases().iter().filter(|c| c.valid) {
        if REFUSED.contains(&case.descriptor.as_str()) {
            continue;
        }
        assert!(
            !case.scripts.is_empty(),
            "{} has no script",
            case.descriptor
        );
        if let Some(body) = case
            .descriptor
            .strip_prefix("rawtr(")
            .and_then(|d| d.strip_suffix(')'))
        {
            let expr = parse_musig_expr(body).expect(&case.descriptor);
            for (index, script) in case.scripts.iter().enumerate() {
                assert_eq!(
                    &raw_script(&expr, index as u32),
                    script,
                    "{}",
                    case.descriptor
                );
                checked += 1;
            }
        } else {
            let expr = parse_tr_musig(&case.descriptor).expect(&case.descriptor);
            for (index, script) in case.scripts.iter().enumerate() {
                let built = script_pubkey_at(&expr, false, index as u32).unwrap();
                assert_eq!(&hex(built.as_bytes()), script, "{}", case.descriptor);
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 5);
}

/// Every descriptor BIP-390 calls invalid is refused, and so is every
/// valid one whose shape this crate does not build, which a wallet must
/// not silently read as a different wallet.
#[test]
fn bip390_refuses_what_it_cannot_build() {
    for case in bip390_cases() {
        if !case.valid || REFUSED.contains(&case.descriptor.as_str()) {
            let inner = case
                .descriptor
                .strip_prefix("rawtr(")
                .and_then(|d| d.strip_suffix(')'));
            let read = match inner {
                Some(body) => parse_musig_expr(body).map(|_| ()),
                None => parse_tr_musig(&case.descriptor).map(|_| ()),
            };
            assert!(read.is_err(), "{} was accepted", case.descriptor);
        }
    }
}

/// The change chain of a multipath `musig()` is the second path of the
/// pair, and `/**` is `/<0;1>/*`.
#[test]
fn a_multipath_musig_derives_two_chains() {
    const A: &str = "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
    const B: &str = "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y";
    let starred = parse_tr_musig(&format!("tr(musig({A},{B})/**)")).unwrap();
    let spelled = parse_tr_musig(&format!("tr(musig({A},{B})/<0;1>/*)")).unwrap();
    let receive = parse_tr_musig(&format!("tr(musig({A},{B})/0/*)")).unwrap();
    let change = parse_tr_musig(&format!("tr(musig({A},{B})/1/*)")).unwrap();
    for index in 0..3 {
        assert_eq!(
            script_pubkey_at(&starred, false, index).unwrap(),
            script_pubkey_at(&spelled, false, index).unwrap()
        );
        assert_eq!(
            script_pubkey_at(&starred, false, index).unwrap(),
            script_pubkey_at(&receive, false, index).unwrap()
        );
        assert_eq!(
            script_pubkey_at(&starred, true, index).unwrap(),
            script_pubkey_at(&change, false, index).unwrap()
        );
        assert_ne!(
            script_pubkey_at(&starred, false, index).unwrap(),
            script_pubkey_at(&starred, true, index).unwrap()
        );
    }
}

/// Every participant may carry a range of its own when the `musig()`
/// has none (BIP-390: "KEY can contain child derivation specified by
/// `/*`"), and each is derived at the same chain and index. The result
/// is another wallet than derivation from the aggregate gives.
#[test]
fn every_participant_may_be_ranged() {
    const A: &str = "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
    const B: &str = "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y";
    let per_key = parse_tr_musig(&format!("tr(musig({A}/<0;1>/*,{B}/<0;1>/*))")).unwrap();
    let fixed = parse_tr_musig(&format!("tr(musig({A}/0/2,{B}/0/2))")).unwrap();
    let aggregate = parse_tr_musig(&format!("tr(musig({A},{B})/<0;1>/*)")).unwrap();
    assert!(per_key.is_ranged());
    assert_eq!(
        script_pubkey_at(&per_key, false, 2).unwrap(),
        script_pubkey_at(&fixed, false, 0).unwrap()
    );
    assert_ne!(
        script_pubkey_at(&per_key, false, 2).unwrap(),
        script_pubkey_at(&per_key, true, 2).unwrap()
    );
    assert_ne!(
        script_pubkey_at(&per_key, false, 2).unwrap(),
        script_pubkey_at(&aggregate, false, 2).unwrap()
    );
}

/// The order the participants are written in does not change the wallet,
/// because BIP-390 sorts them before aggregating.
#[test]
fn participant_order_does_not_change_the_addresses() {
    const A: &str = "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
    const B: &str = "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y";
    let one = parse_tr_musig(&format!("tr(musig({A},{B})/0/*)")).unwrap();
    let other = parse_tr_musig(&format!("tr(musig({B},{A})/0/*)")).unwrap();
    for index in 0..3 {
        assert_eq!(
            script_pubkey_at(&one, false, index).unwrap(),
            script_pubkey_at(&other, false, index).unwrap()
        );
    }
}

/// A key expression's origin is read and kept, so a review can show
/// which device each participant is.
#[test]
fn a_participant_carries_its_origin() {
    const KEY: &str = "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw";
    const OTHER: &str = "[b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7";
    let expr = parse_tr_musig(&format!("tr(musig({KEY},{OTHER})/**)")).unwrap();
    assert_eq!(
        expr.participants()[0].origin(),
        Some("6738736c/48'/0'/0'/2'")
    );
    assert_eq!(
        expr.participants()[1].origin(),
        Some("b2b1f0cf/48'/0'/0'/2'")
    );
    assert!(expr.participants().iter().all(|p| p.xpub().is_some()));
    assert!(script_pubkey_at(&expr, false, 0).is_ok());
}

/// A `musig()` inside another `musig()`, and one with a single
/// participant, are not wallets this reads.
#[test]
fn a_nested_or_lone_musig_is_refused() {
    const A: &str = "02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9";
    const B: &str = "03dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659";
    assert!(parse_musig_expr(&format!("musig(musig({A},{B}),{A})")).is_err());
    assert!(parse_musig_expr(&format!("musig({A})")).is_err());
    assert!(parse_musig_expr(&format!("{A},{B}")).is_err());
}

/// A tweak at or above the curve order is no tweak.
#[test]
fn a_tweak_out_of_range_is_refused() {
    const A: &str = "02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9";
    const B: &str = "03dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659";
    let secp = Secp256k1::verification_only();
    let aggregate = key_agg(&[parse_key(A).unwrap(), parse_key(B).unwrap()]).unwrap();
    let order: [u8; 32] = unhex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
        .try_into()
        .unwrap();
    assert_eq!(
        aggregate.apply_tweak(&secp, &order, false),
        Err(musig::Error::TweakOutOfRange)
    );
    let last = unhex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140");
    assert!(
        aggregate
            .apply_tweak(&secp, &last.try_into().unwrap(), false)
            .is_ok()
    );
}

/// A BIP-390 MuSig2 wallet policy pays, on its receive chain, to the
/// aggregate keys the BIP publishes for `rawtr(musig(A,B)/0/*)`, each
/// under the taproot tweak `tr()` applies. The policy's own descriptor
/// reads back as the same wallet.
#[test]
fn a_musig_wallet_policy_pays_to_the_published_aggregate_keys() {
    use osk_bip::policy::WalletPolicy;

    const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-musig.policy");
    let policy = WalletPolicy::parse(POLICY).unwrap();
    let published = bip390_cases()
        .into_iter()
        .find(|c| c.valid && c.descriptor.starts_with("rawtr(musig(xpub"))
        .expect("the ranged rawtr vector");
    let expr = parse_musig_expr(
        published
            .descriptor
            .strip_prefix("rawtr(")
            .and_then(|d| d.strip_suffix(')'))
            .unwrap(),
    )
    .unwrap();
    for (index, internal) in published.scripts.iter().enumerate() {
        let index = index as u32;
        // The internal key is the one the BIP publishes.
        assert_eq!(&raw_script(&expr, index), internal);
        // The wallet pays to that key tweaked, which is what the same
        // aggregate written as `tr(musig(A,B)/0/*)` pays to.
        let tweaked = script_pubkey_at(&expr, false, index).unwrap();
        assert_eq!(policy.script_at(false, index).unwrap(), tweaked);
    }
    assert_eq!(
        WalletPolicy::from_descriptor(&policy.to_descriptor_checksummed()).unwrap(),
        policy
    );
}

// ---------------------------------------------------------------------
// BIP-327's signing vectors
// ---------------------------------------------------------------------

const NONCE_GEN: &str = include_str!("../../../tools/vectors/bip327/nonce_gen_vectors.json");
const NONCE_AGG: &str = include_str!("../../../tools/vectors/bip327/nonce_agg_vectors.json");
const SIGN_VERIFY: &str = include_str!("../../../tools/vectors/bip327/sign_verify_vectors.json");
const SIG_AGG: &str = include_str!("../../../tools/vectors/bip327/sig_agg_vectors.json");
const TWEAK: &str = include_str!("../../../tools/vectors/bip327/tweak_vectors.json");
const DET_SIGN: &str = include_str!("../../../tools/vectors/bip327/det_sign_vectors.json");

fn secret(text: &str) -> SecretKey {
    SecretKey::from_slice(&unhex(text)).unwrap()
}

fn maybe_hex(value: &Value) -> Option<Vec<u8>> {
    value.as_str().map(unhex)
}

fn array_of(json: &Value, field: &str) -> Vec<String> {
    json[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().to_lowercase())
        .collect()
}

fn indices(case: &Value, field: &str) -> Vec<usize> {
    case[field]
        .as_array()
        .map(|a| a.iter().map(|i| i.as_u64().unwrap() as usize).collect())
        .unwrap_or_default()
}

/// The tweak list a case names, in the order it names them.
fn tweaks_of(case: &Value, tweaks: &[String], field: &str) -> Vec<Tweak> {
    let is_xonly = case["is_xonly"].as_array().cloned().unwrap_or_default();
    indices(case, field)
        .iter()
        .zip(is_xonly)
        .map(|(i, x)| Tweak {
            bytes: unhex(&tweaks[*i]).try_into().unwrap(),
            x_only: x.as_bool().unwrap(),
        })
        .collect()
}

/// The tweaks a case writes inline rather than by index.
fn inline_tweaks(case: &Value) -> Vec<Tweak> {
    case["tweaks"]
        .as_array()
        .unwrap()
        .iter()
        .zip(case["is_xonly"].as_array().unwrap())
        .map(|(t, x)| Tweak {
            bytes: unhex(&t.as_str().unwrap().to_lowercase())
                .try_into()
                .unwrap(),
            x_only: x.as_bool().unwrap(),
        })
        .collect()
}

/// Every `NonceGen` case draws the secret and public nonce the file
/// states from the `rand'` it fixes.
#[test]
fn bip327_nonce_gen_vectors() {
    let json: Value = serde_json::from_str(NONCE_GEN).unwrap();
    let secp = Secp256k1::new();
    let mut checked = 0;
    for case in json["test_cases"].as_array().unwrap() {
        let rand: [u8; 32] = unhex(&case["rand_"].as_str().unwrap().to_lowercase())
            .try_into()
            .unwrap();
        let sk = case["sk"].as_str().map(|s| secret(&s.to_lowercase()));
        let pk = parse_key(&case["pk"].as_str().unwrap().to_lowercase()).unwrap();
        let aggpk: Option<[u8; 32]> = case["aggpk"]
            .as_str()
            .map(|a| unhex(&a.to_lowercase()).try_into().unwrap());
        let msg = maybe_hex(&case["msg"]);
        let extra_in = maybe_hex(&case["extra_in"]);
        let (secnonce, pubnonce) = musig::nonce_gen(
            &secp,
            &rand,
            sk.as_ref(),
            &pk,
            aggpk.as_ref(),
            msg.as_deref(),
            extra_in.as_deref(),
        )
        .unwrap();
        assert_eq!(
            hex(&secnonce.serialize()),
            case["expected_secnonce"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        assert_eq!(
            hex(&pubnonce.serialize()),
            case["expected_pubnonce"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// `NonceAgg` sums both halves, writes infinity as 33 zero bytes, and
/// refuses a nonce half that is no point.
#[test]
fn bip327_nonce_agg_vectors() {
    let json: Value = serde_json::from_str(NONCE_AGG).unwrap();
    let pnonces = array_of(&json, "pnonces");
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let nonces: Vec<PubNonce> = indices(case, "pnonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])).unwrap())
            .collect();
        assert_eq!(
            hex(&musig::nonce_agg(&nonces).unwrap().serialize()),
            case["expected"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 2);
    let mut errors = 0;
    for case in json["error_test_cases"].as_array().unwrap() {
        let parsed: Vec<Result<PubNonce, _>> = indices(case, "pnonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])))
            .collect();
        assert!(parsed.iter().any(Result::is_err), "{case} was accepted");
        errors += 1;
    }
    assert_eq!(errors, 3);
}

/// Every `Sign` case gives the partial signature the file states, and
/// every such signature verifies against its own nonce and key.
#[test]
fn bip327_sign_vectors() {
    let json: Value = serde_json::from_str(SIGN_VERIFY).unwrap();
    let secp = Secp256k1::new();
    let sk = secret(&json["sk"].as_str().unwrap().to_lowercase());
    let pubkeys = array_of(&json, "pubkeys");
    let pnonces = array_of(&json, "pnonces");
    let aggnonces = array_of(&json, "aggnonces");
    let msgs = array_of(&json, "msgs");
    let secnonces = array_of(&json, "secnonces");
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let nonces: Vec<PubNonce> = indices(case, "nonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])).unwrap())
            .collect();
        let aggnonce = AggNonce::from_bytes(&unhex(
            &aggnonces[case["aggnonce_index"].as_u64().unwrap() as usize],
        ))
        .unwrap();
        let msg = unhex(&msgs[case["msg_index"].as_u64().unwrap() as usize]);
        let session = musig::session_values(&secp, &keys, &[], &aggnonce, &msg).unwrap();
        let secnonce = SecNonce::from_bytes(&unhex(&secnonces[0])).unwrap();
        let psig = musig::sign(&secp, secnonce, &sk, &session).unwrap();
        assert_eq!(
            hex(&psig.serialize()),
            case["expected"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        let signer = case["signer_index"].as_u64().unwrap() as usize;
        musig::partial_sig_verify(&secp, &psig, &nonces, &keys, &[], &msg, signer).unwrap();
        checked += 1;
    }
    assert_eq!(checked, 6);
}

/// Every `Sign` error case is refused: a key that is no point, an
/// aggregate nonce that is no pair of points, a secret nonce out of
/// range, and a signer whose key is not in the list.
#[test]
fn bip327_sign_error_vectors() {
    let json: Value = serde_json::from_str(SIGN_VERIFY).unwrap();
    let secp = Secp256k1::new();
    let sk = secret(&json["sk"].as_str().unwrap().to_lowercase());
    let pubkeys = array_of(&json, "pubkeys");
    let aggnonces = array_of(&json, "aggnonces");
    let msgs = array_of(&json, "msgs");
    let secnonces = array_of(&json, "secnonces");
    let mut checked = 0;
    for case in json["sign_error_test_cases"].as_array().unwrap() {
        let parsed: Vec<Option<PublicKey>> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]))
            .collect();
        let msg = unhex(&msgs[case["msg_index"].as_u64().unwrap() as usize]);
        let aggnonce = AggNonce::from_bytes(&unhex(
            &aggnonces[case["aggnonce_index"].as_u64().unwrap() as usize],
        ));
        let secnonce = SecNonce::from_bytes(&unhex(
            &secnonces[case["secnonce_index"].as_u64().unwrap() as usize],
        ))
        .unwrap();
        let failed = match (parsed.iter().all(Option::is_some), aggnonce) {
            (false, _) => true,
            (_, Err(_)) => true,
            (true, Ok(aggnonce)) => {
                let keys: Vec<PublicKey> = parsed.into_iter().map(Option::unwrap).collect();
                match musig::session_values(&secp, &keys, &[], &aggnonce, &msg) {
                    Err(_) => true,
                    Ok(session) => musig::sign(&secp, secnonce, &sk, &session).is_err(),
                }
            }
        };
        assert!(failed, "{case} was accepted");
        checked += 1;
    }
    assert_eq!(checked, 6);
}

/// Every `PartialSigVerify` failure is reported as one, and every
/// malformed contribution is refused before verification.
#[test]
fn bip327_partial_sig_verify_vectors() {
    let json: Value = serde_json::from_str(SIGN_VERIFY).unwrap();
    let secp = Secp256k1::new();
    let pubkeys = array_of(&json, "pubkeys");
    let pnonces = array_of(&json, "pnonces");
    let msgs = array_of(&json, "msgs");
    let mut failures = 0;
    for case in json["verify_fail_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let nonces: Vec<PubNonce> = indices(case, "nonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])).unwrap())
            .collect();
        let msg = unhex(&msgs[case["msg_index"].as_u64().unwrap() as usize]);
        let signer = case["signer_index"].as_u64().unwrap() as usize;
        let sig = case["sig"].as_str().unwrap().to_lowercase();
        let refused = match PartialSig::from_bytes(&unhex(&sig)) {
            Err(_) => true,
            Ok(psig) => {
                musig::partial_sig_verify(&secp, &psig, &nonces, &keys, &[], &msg, signer).is_err()
            }
        };
        assert!(refused, "{case} verified");
        failures += 1;
    }
    assert_eq!(failures, 3);
    let mut errors = 0;
    for case in json["verify_error_test_cases"].as_array().unwrap() {
        let keys: Vec<Option<PublicKey>> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]))
            .collect();
        let nonces: Vec<Result<PubNonce, _>> = indices(case, "nonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])))
            .collect();
        let refused = keys.iter().any(Option::is_none) || nonces.iter().any(Result::is_err);
        assert!(refused, "{case} was accepted");
        errors += 1;
    }
    assert_eq!(errors, 2);
}

/// The tweak vectors: a signer's partial signature under plain and
/// x-only tweaks, in the orders the file gives, and the refusal of a
/// tweak at the curve order.
#[test]
fn bip327_tweak_vectors() {
    let json: Value = serde_json::from_str(TWEAK).unwrap();
    let secp = Secp256k1::new();
    let sk = secret(&json["sk"].as_str().unwrap().to_lowercase());
    let pubkeys = array_of(&json, "pubkeys");
    let pnonces = array_of(&json, "pnonces");
    let tweaks = array_of(&json, "tweaks");
    let secnonce_bytes = unhex(&json["secnonce"].as_str().unwrap().to_lowercase());
    let aggnonce =
        AggNonce::from_bytes(&unhex(&json["aggnonce"].as_str().unwrap().to_lowercase())).unwrap();
    let msg = unhex(&json["msg"].as_str().unwrap().to_lowercase());
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let nonces: Vec<PubNonce> = indices(case, "nonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])).unwrap())
            .collect();
        let list = tweaks_of(case, &tweaks, "tweak_indices");
        let session = musig::session_values(&secp, &keys, &list, &aggnonce, &msg).unwrap();
        let secnonce = SecNonce::from_bytes(&secnonce_bytes).unwrap();
        let psig = musig::sign(&secp, secnonce, &sk, &session).unwrap();
        assert_eq!(
            hex(&psig.serialize()),
            case["expected"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        let signer = case["signer_index"].as_u64().unwrap() as usize;
        musig::partial_sig_verify(&secp, &psig, &nonces, &keys, &list, &msg, signer).unwrap();
        checked += 1;
    }
    assert_eq!(checked, 5);
    let mut errors = 0;
    for case in json["error_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let list = tweaks_of(case, &tweaks, "tweak_indices");
        assert!(
            musig::session_values(&secp, &keys, &list, &aggnonce, &msg).is_err(),
            "{case} was accepted"
        );
        errors += 1;
    }
    assert_eq!(errors, 1);
}

/// `PartialSigAgg` produces the 64-byte signature the file states, and
/// that signature is a BIP-340 signature of the message under the
/// tweaked aggregate key.
#[test]
fn bip327_sig_agg_vectors() {
    let json: Value = serde_json::from_str(SIG_AGG).unwrap();
    let secp = Secp256k1::new();
    let pubkeys = array_of(&json, "pubkeys");
    let pnonces = array_of(&json, "pnonces");
    let tweaks = array_of(&json, "tweaks");
    let psigs = array_of(&json, "psigs");
    let msg = unhex(&json["msg"].as_str().unwrap().to_lowercase());
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let nonces: Vec<PubNonce> = indices(case, "nonce_indices")
            .iter()
            .map(|i| PubNonce::from_bytes(&unhex(&pnonces[*i])).unwrap())
            .collect();
        let list = tweaks_of(case, &tweaks, "tweak_indices");
        let aggnonce =
            AggNonce::from_bytes(&unhex(&case["aggnonce"].as_str().unwrap().to_lowercase()))
                .unwrap();
        assert_eq!(musig::nonce_agg(&nonces).unwrap(), aggnonce, "{case}");
        let session = musig::session_values(&secp, &keys, &list, &aggnonce, &msg).unwrap();
        let signatures: Vec<PartialSig> = indices(case, "psig_indices")
            .iter()
            .map(|i| PartialSig::from_bytes(&unhex(&psigs[*i])).unwrap())
            .collect();
        let sig = musig::partial_sig_agg(&signatures, &session).unwrap();
        assert_eq!(
            hex(&sig),
            case["expected"].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        let schnorr = osk_bip::bitcoin::secp256k1::schnorr::Signature::from_slice(&sig).unwrap();
        let key = osk_bip::bitcoin::secp256k1::XOnlyPublicKey::from_slice(
            &session.aggregate_key().serialize_x_only(),
        )
        .unwrap();
        secp.verify_schnorr(
            &schnorr,
            &osk_bip::bitcoin::secp256k1::Message::from_digest_slice(&msg).unwrap(),
            &key,
        )
        .unwrap();
        checked += 1;
    }
    assert_eq!(checked, 4);
    let mut errors = 0;
    for case in json["error_test_cases"].as_array().unwrap() {
        let refused = indices(case, "psig_indices")
            .iter()
            .any(|i| PartialSig::from_bytes(&unhex(&psigs[*i])).is_err());
        assert!(refused, "{case} was accepted");
        errors += 1;
    }
    assert_eq!(errors, 1);
}

/// `DeterministicSign`: the nonce and the partial signature the file
/// states, with and without the optional `rand`, and every error case
/// refused.
#[test]
fn bip327_det_sign_vectors() {
    let json: Value = serde_json::from_str(DET_SIGN).unwrap();
    let secp = Secp256k1::new();
    let sk = secret(&json["sk"].as_str().unwrap().to_lowercase());
    let pubkeys = array_of(&json, "pubkeys");
    let msgs = array_of(&json, "msgs");
    let mut checked = 0;
    for case in json["valid_test_cases"].as_array().unwrap() {
        let keys: Vec<PublicKey> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]).unwrap())
            .collect();
        let aggothernonce = AggNonce::from_bytes(&unhex(
            &case["aggothernonce"].as_str().unwrap().to_lowercase(),
        ))
        .unwrap();
        let list = inline_tweaks(case);
        let msg = unhex(&msgs[case["msg_index"].as_u64().unwrap() as usize]);
        let rand: Option<[u8; 32]> = case["rand"]
            .as_str()
            .map(|r| unhex(&r.to_lowercase()).try_into().unwrap());
        let (pubnonce, psig) = musig::deterministic_sign(
            &secp,
            &sk,
            &aggothernonce,
            &keys,
            &list,
            &msg,
            rand.as_ref(),
        )
        .unwrap();
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(
            hex(&pubnonce.serialize()),
            expected[0].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        assert_eq!(
            hex(&psig.serialize()),
            expected[1].as_str().unwrap().to_lowercase(),
            "{case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
    let mut errors = 0;
    for case in json["error_test_cases"].as_array().unwrap() {
        let parsed: Vec<Option<PublicKey>> = indices(case, "key_indices")
            .iter()
            .map(|i| parse_key(&pubkeys[*i]))
            .collect();
        let aggothernonce = AggNonce::from_bytes(&unhex(
            &case["aggothernonce"].as_str().unwrap().to_lowercase(),
        ));
        let msg = unhex(&msgs[case["msg_index"].as_u64().unwrap() as usize]);
        let rand: Option<[u8; 32]> = case["rand"]
            .as_str()
            .map(|r| unhex(&r.to_lowercase()).try_into().unwrap());
        let refused = match (parsed.iter().all(Option::is_some), aggothernonce) {
            (false, _) | (_, Err(_)) => true,
            (true, Ok(aggothernonce)) => {
                let keys: Vec<PublicKey> = parsed.into_iter().map(Option::unwrap).collect();
                let list = inline_tweaks(case);
                musig::deterministic_sign(
                    &secp,
                    &sk,
                    &aggothernonce,
                    &keys,
                    &list,
                    &msg,
                    rand.as_ref(),
                )
                .is_err()
            }
        };
        assert!(refused, "{case} was accepted");
        errors += 1;
    }
    assert_eq!(errors, 5);
}
