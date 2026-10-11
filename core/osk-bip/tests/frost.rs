//! BIP 445 FROST signing against the published vectors, and the trusted
//! dealer of `docs/PLANNING.md` §16.103 against those vectors' key
//! material and against RFC 9591's Appendix F.5 numbers.
//!
//! The six files under `tools/vectors/bip445/` are the reference
//! repository's own, with the layout its `test_vectors_summary.md`
//! describes: four test groups per file, one per `(t, n)`, each with the
//! key setup and the shared inputs its cases index into.

mod common;

use common::{hex, unhex};
use osk_bip::bitcoin::secp256k1::{Message, PublicKey, Secp256k1, XOnlyPublicKey, schnorr};
use osk_bip::frost::{
    self, Contrib, Error, PartialSig, PubNonce, SecNonce, SecShare, SessionContext, ThresholdInfo,
    Tweak,
};
use serde_json::Value;

/// The verification-only context every test but the BIP-340 check uses.
type Verify = osk_bip::bitcoin::secp256k1::VerifyOnly;

const NONCE_GEN: &str = include_str!("../../../tools/vectors/bip445/nonce_gen_vectors.json");
const NONCE_AGG: &str = include_str!("../../../tools/vectors/bip445/nonce_agg_vectors.json");
const SIGN_VERIFY: &str = include_str!("../../../tools/vectors/bip445/sign_verify_vectors.json");
const TWEAK: &str = include_str!("../../../tools/vectors/bip445/tweak_vectors.json");
const DET_SIGN: &str = include_str!("../../../tools/vectors/bip445/det_sign_vectors.json");
const SIG_AGG: &str = include_str!("../../../tools/vectors/bip445/sig_agg_vectors.json");

/// The appended public share that is no point on the curve, which the
/// error cases of every file use.
const BOGUS_PUBSHARE: &str = "020000000000000000000000000000000000000000000000000000000000000007";

// ---------------------------------------------------------------------
// Reading the files
// ---------------------------------------------------------------------

fn array_of(json: &Value, field: &str) -> Vec<String> {
    json[field]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

fn text(value: &Value) -> String {
    value.as_str().unwrap().to_lowercase()
}

fn bytes32(value: &Value) -> [u8; 32] {
    unhex(&text(value)).try_into().unwrap()
}

fn maybe_hex(value: &Value) -> Option<Vec<u8>> {
    value.as_str().map(|s| unhex(&s.to_lowercase()))
}

fn indices(case: &Value, field: &str) -> Vec<usize> {
    case[field]
        .as_array()
        .map(|a| a.iter().map(|i| i.as_u64().unwrap() as usize).collect())
        .unwrap_or_default()
}

fn ids_of(case: &Value) -> Vec<u32> {
    case["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap() as u32)
        .collect()
}

fn key(text: &str) -> PublicKey {
    PublicKey::from_slice(&unhex(text)).unwrap()
}

/// One `(t, n)` test group with the inputs its cases index into.
struct Group {
    id: String,
    t: usize,
    n: usize,
    thresh_pk: PublicKey,
    pubshares: Vec<String>,
    pubnonces: Vec<String>,
    secshares: Vec<String>,
    secnonces: Vec<String>,
    tweaks: Vec<String>,
    cases: Value,
}

impl Group {
    /// The public shares a case names, each blamed on its position when
    /// it is no point.
    fn pubshares_at(&self, case: &Value) -> Option<Result<Vec<PublicKey>, Error>> {
        if case["pubshare_indices"].is_null() {
            return None;
        }
        Some(
            indices(case, "pubshare_indices")
                .iter()
                .enumerate()
                .map(|(position, i)| frost::parse_pubshare(&unhex(&self.pubshares[*i]), position))
                .collect(),
        )
    }

    fn pubnonces_at(&self, case: &Value) -> Result<Vec<PubNonce>, Error> {
        indices(case, "pubnonce_indices")
            .iter()
            .enumerate()
            .map(|(position, i)| frost::parse_pubnonce(&unhex(&self.pubnonces[*i]), position))
            .collect()
    }

    fn secshare_at(&self, case: &Value) -> Result<SecShare, Error> {
        let index = case["secshare_index"].as_u64().unwrap() as usize;
        SecShare::from_bytes(&unhex(&self.secshares[index]).try_into().unwrap())
    }

    fn secnonce_at(&self, case: &Value) -> Result<SecNonce, Error> {
        let index = case["secnonce_index"].as_u64().unwrap() as usize;
        SecNonce::from_bytes(&unhex(&self.secnonces[index]))
    }

    /// The tweaks a case names by index, with their modes.
    fn tweaks_at(&self, case: &Value) -> Vec<Tweak> {
        indices(case, "tweak_indices")
            .iter()
            .zip(case["is_xonly"].as_array().unwrap())
            .map(|(i, mode)| Tweak {
                bytes: unhex(&self.tweaks[*i]).try_into().unwrap(),
                x_only: mode.as_bool().unwrap(),
            })
            .collect()
    }

    fn real_secshares(&self) -> Vec<SecShare> {
        self.secshares[..self.n]
            .iter()
            .map(|s| SecShare::from_bytes(&unhex(s).try_into().unwrap()).unwrap())
            .collect()
    }

    fn real_pubshares(&self) -> Vec<PublicKey> {
        self.pubshares[..self.n].iter().map(|p| key(p)).collect()
    }
}

fn groups(file: &str) -> Vec<Group> {
    let json: Value = serde_json::from_str(file).unwrap();
    json["test_groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| Group {
            id: g["tg_id"].as_str().unwrap().into(),
            t: g["t"].as_u64().unwrap() as usize,
            n: g["n"].as_u64().unwrap() as usize,
            thresh_pk: key(&text(&g["thresh_pk"])),
            pubshares: array_of(g, "pubshares"),
            pubnonces: array_of(g, "pubnonces"),
            secshares: array_of(g, "secshares"),
            secnonces: array_of(g, "secnonces"),
            tweaks: array_of(g, "tweaks"),
            cases: g.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------
// The errors the reference raises
// ---------------------------------------------------------------------

/// The variant each message the reference raises maps to. Two of its
/// refusals have no variant here, because the Rust types refuse the
/// input before a function sees it: a tweak that is not 32 bytes, and a
/// tweak list and mode list of different lengths, which are one
/// [`Tweak`] each here.
fn value_error(message: &str) -> Option<Error> {
    Some(match message {
        "The threshold must be 1 <= t <= n." => Error::ThresholdRange,
        "The number of participants must be n <= 128." => Error::TooManyParticipants,
        "The number of signers must be between t and n." => Error::SignerCount,
        "The pubshares and ids lists must have the same length." => Error::PubShareCount,
        "Invalid id at index 0" => Error::IdOutOfRange { index: 0 },
        "The ids list contains duplicate elements." => Error::DuplicateIds,
        "Invalid pubshare at index 0." => Error::InvalidPubShare { index: 0 },
        "Invalid pubshare at index 1." => Error::InvalidPubShare { index: 1 },
        "The threshold public key must not be the point at infinity." => Error::ThreshPkInfinity,
        "The provided key material is incorrect: the public shares do not match the threshold public key." => {
            Error::ThreshPkMismatch
        }
        "first secnonce value is out of range." => Error::FirstSecNonceOutOfRange,
        "second secnonce value is out of range." => Error::SecondSecNonceOutOfRange,
        "The signer's secret share value is out of range." => Error::SecShareOutOfRange,
        "The signer's id is missing from the ids list." => Error::NotASigner,
        "The signer's pubshare is missing from the pubshares list." => Error::PubShareMismatch,
        "The psigs and ids lists must have the same length." => Error::PartialSigCount,
        "The tweak value is out of range." => Error::TweakOutOfRange,
        "The result of tweaking cannot be infinity." => Error::TweakInfinity,
        _ => return None,
    })
}

fn contrib_of(name: &str) -> Contrib {
    match name {
        "pubnonce" => Contrib::PubNonce,
        "aggnonce" => Contrib::AggNonce,
        "aggothernonce" => Contrib::AggOtherNonce,
        "psig" => Contrib::PartialSig,
        other => panic!("unknown contribution {other}"),
    }
}

fn expected_error(case: &Value) -> Option<Error> {
    let error = &case["error"];
    match error["type"].as_str().unwrap() {
        "InvalidContributionError" => Some(Error::InvalidContribution {
            signer: error["signer_index"].as_u64().map(|i| i as usize),
            contrib: contrib_of(error["contrib"].as_str().unwrap()),
        }),
        "ValueError" => value_error(error["message"].as_str().unwrap()),
        other => panic!("unknown error type {other}"),
    }
}

/// A tweak list the Rust types cannot express: a tweak that is not 32
/// bytes, or a mode list of another length than the tweak list.
fn tweaks_are_unexpressible(group: &Group, case: &Value) -> bool {
    let chosen = indices(case, "tweak_indices");
    let modes = case["is_xonly"].as_array().unwrap().len();
    chosen.len() != modes || chosen.iter().any(|i| unhex(&group.tweaks[*i]).len() != 32)
}

// ---------------------------------------------------------------------
// NonceGen and NonceAgg
// ---------------------------------------------------------------------

/// Every `NonceGen` case draws the secret and public nonce the file
/// states, with each optional input present and absent.
#[test]
fn frost_nonce_gen_vectors() {
    let json: Value = serde_json::from_str(NONCE_GEN).unwrap();
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for case in json["valid_tests"].as_array().unwrap() {
        let rand = bytes32(&case["rand"]);
        let secshare = case["secshare"]
            .as_str()
            .map(|s| SecShare::from_bytes(&unhex(&s.to_lowercase()).try_into().unwrap()).unwrap());
        let pubshare = case["pubshare"].as_str().map(|p| key(&p.to_lowercase()));
        let thresh_pk_xonly = case["thresh_pk_xonly"]
            .as_str()
            .map(|k| -> [u8; 32] { unhex(&k.to_lowercase()).try_into().unwrap() });
        let msg = maybe_hex(&case["msg"]);
        let extra_in = maybe_hex(&case["extra_in"]);
        let (secnonce, pubnonce) = frost::nonce_gen(
            &secp,
            &rand,
            secshare.as_ref(),
            pubshare.as_ref(),
            thresh_pk_xonly.as_ref(),
            msg.as_deref(),
            extra_in.as_deref(),
        )
        .unwrap();
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(hex(&secnonce.serialize()), text(&expected[0]), "{case}");
        assert_eq!(hex(&pubnonce.serialize()), text(&expected[1]), "{case}");
        checked += 1;
    }
    assert_eq!(checked, 5);
}

/// `NonceAgg` sums both halves, writes infinity as 33 zero bytes, and
/// names the signer whose nonce it cannot read.
#[test]
fn frost_nonce_agg_vectors() {
    let json: Value = serde_json::from_str(NONCE_AGG).unwrap();
    let pubnonces = array_of(&json, "pubnonces");
    let mut checked = 0;
    for case in json["valid_tests"].as_array().unwrap() {
        let nonces: Vec<PubNonce> = indices(case, "pubnonce_indices")
            .iter()
            .enumerate()
            .map(|(position, i)| frost::parse_pubnonce(&unhex(&pubnonces[*i]), position).unwrap())
            .collect();
        assert_eq!(
            hex(&frost::nonce_agg(&nonces).unwrap().serialize()),
            text(&case["expected"]),
            "{case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 2);
    let mut errors = 0;
    for case in json["error_tests"].as_array().unwrap() {
        let refused = indices(case, "pubnonce_indices")
            .iter()
            .enumerate()
            .find_map(|(position, i)| {
                frost::parse_pubnonce(&unhex(&pubnonces[*i]), position).err()
            });
        assert_eq!(refused, expected_error(case), "{case}");
        errors += 1;
    }
    assert_eq!(errors, 3);
}

// ---------------------------------------------------------------------
// Sign and PartialSigVerify
// ---------------------------------------------------------------------

/// Runs one sign case the way a signer would: read the public shares,
/// the share and the nonce, then sign.
fn run_sign(
    secp: &Secp256k1<Verify>,
    group: &Group,
    case: &Value,
    tweaks: Vec<Tweak>,
) -> Result<PartialSig, Error> {
    let pubshares = group.pubshares_at(case).transpose()?;
    let secshare = group.secshare_at(case)?;
    let secnonce = group.secnonce_at(case)?;
    let ctx = SessionContext {
        n: group.n,
        t: group.t,
        ids: ids_of(case),
        pubshares,
        thresh_pk: group.thresh_pk,
        aggnonce: frost::parse_aggnonce(&unhex(&text(&case["aggnonce"])), Contrib::AggNonce)?,
        tweaks,
        msg: unhex(&text(&case["msg"])),
    };
    frost::sign(
        secp,
        secnonce,
        &secshare,
        case["my_id"].as_u64().unwrap() as u32,
        &ctx,
    )
}

/// Every `Sign` case gives the partial signature the file states, and
/// every such signature verifies against its own nonce and public share.
#[test]
fn frost_sign_vectors() {
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for group in groups(SIGN_VERIFY) {
        for case in group.cases["valid_tests"].as_array().unwrap() {
            let ids = ids_of(case);
            let nonces = group.pubnonces_at(case).unwrap();
            let aggnonce =
                frost::parse_aggnonce(&unhex(&text(&case["aggnonce"])), Contrib::AggNonce).unwrap();
            assert_eq!(frost::nonce_agg(&nonces).unwrap(), aggnonce, "{case}");
            let psig = run_sign(&secp, &group, case, Vec::new()).unwrap();
            assert_eq!(
                hex(&psig.serialize()),
                text(&case["expected"]),
                "{} {case}",
                group.id
            );
            if let Some(pubshares) = group.pubshares_at(case) {
                let pubshares = pubshares.unwrap();
                let my_id = case["my_id"].as_u64().unwrap() as u32;
                let position = ids.iter().position(|id| *id == my_id).unwrap();
                frost::partial_sig_verify(
                    &secp,
                    &psig,
                    &nonces,
                    group.n,
                    group.t,
                    &ids,
                    &pubshares,
                    &group.thresh_pk,
                    &[],
                    &unhex(&text(&case["msg"])),
                    position,
                )
                .unwrap();
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 29);
}

/// Every `Sign` error case is refused for the reason the file names.
#[test]
fn frost_sign_error_vectors() {
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for group in groups(SIGN_VERIFY) {
        for case in group.cases["sign_error_tests"].as_array().unwrap() {
            let refused = run_sign(&secp, &group, case, Vec::new()).unwrap_err();
            assert_eq!(Some(refused), expected_error(case), "{} {case}", group.id);
            checked += 1;
        }
    }
    assert_eq!(checked, 52);
}

/// Every `PartialSigVerify` failure is reported as a failure and nothing
/// else, and every malformed contribution is named instead.
#[test]
fn frost_partial_sig_verify_vectors() {
    let secp = Secp256k1::verification_only();
    let (mut failures, mut errors) = (0, 0);
    for group in groups(SIGN_VERIFY) {
        for case in group.cases["verify_fail_tests"].as_array().unwrap() {
            let ids = ids_of(case);
            let pubshares = group.pubshares_at(case).unwrap().unwrap();
            let nonces = group.pubnonces_at(case).unwrap();
            let position = case["signer_index"].as_u64().unwrap() as usize;
            let refused = match PartialSig::from_bytes(&unhex(&text(&case["psig"]))) {
                Err(_) => Error::VerifyFailed,
                Ok(psig) => frost::partial_sig_verify(
                    &secp,
                    &psig,
                    &nonces,
                    group.n,
                    group.t,
                    &ids,
                    &pubshares,
                    &group.thresh_pk,
                    &[],
                    &unhex(&text(&case["msg"])),
                    position,
                )
                .unwrap_err(),
            };
            assert_eq!(refused, Error::VerifyFailed, "{} {case}", group.id);
            failures += 1;
        }
        for case in group.cases["verify_error_tests"].as_array().unwrap() {
            let ids = ids_of(case);
            let position = case["signer_index"].as_u64().unwrap() as usize;
            let refused = group
                .pubshares_at(case)
                .unwrap()
                .and_then(|pubshares| {
                    let nonces = group.pubnonces_at(case)?;
                    let psig = frost::parse_psig(&unhex(&text(&case["psig"])), position)?;
                    frost::partial_sig_verify(
                        &secp,
                        &psig,
                        &nonces,
                        group.n,
                        group.t,
                        &ids,
                        &pubshares,
                        &group.thresh_pk,
                        &[],
                        &unhex(&text(&case["msg"])),
                        position,
                    )
                })
                .unwrap_err();
            assert_ne!(refused, Error::VerifyFailed, "{} {case}", group.id);
            assert_eq!(Some(refused), expected_error(case), "{} {case}", group.id);
            errors += 1;
        }
    }
    assert_eq!((failures, errors), (12, 8));
}

/// The tweak vectors: a signer's partial signature under every order of
/// plain and x-only tweaks the file gives, and every refusal.
#[test]
fn frost_tweak_vectors() {
    let secp = Secp256k1::verification_only();
    let (mut checked, mut errors, mut unexpressible) = (0, 0, 0);
    for group in groups(TWEAK) {
        for case in group.cases["valid_tests"].as_array().unwrap() {
            let tweaks = group.tweaks_at(case);
            let psig = run_sign(&secp, &group, case, tweaks.clone()).unwrap();
            assert_eq!(
                hex(&psig.serialize()),
                text(&case["expected"]),
                "{} {case}",
                group.id
            );
            let ids = ids_of(case);
            let my_id = case["my_id"].as_u64().unwrap() as u32;
            let position = ids.iter().position(|id| *id == my_id).unwrap();
            frost::partial_sig_verify(
                &secp,
                &psig,
                &group.pubnonces_at(case).unwrap(),
                group.n,
                group.t,
                &ids,
                &group.pubshares_at(case).unwrap().unwrap(),
                &group.thresh_pk,
                &tweaks,
                &unhex(&text(&case["msg"])),
                position,
            )
            .unwrap();
            checked += 1;
        }
        for case in group.cases["error_tests"].as_array().unwrap() {
            match expected_error(case) {
                Some(expected) => {
                    let refused = run_sign(&secp, &group, case, group.tweaks_at(case)).unwrap_err();
                    assert_eq!(refused, expected, "{} {case}", group.id);
                    errors += 1;
                }
                None => {
                    assert!(tweaks_are_unexpressible(&group, case), "{case}");
                    unexpressible += 1;
                }
            }
        }
    }
    assert_eq!((checked, errors, unexpressible), (28, 8, 8));
}

// ---------------------------------------------------------------------
// DeterministicSign
// ---------------------------------------------------------------------

/// The tweaks a `det_sign` case writes inline.
fn inline_tweaks(case: &Value) -> Vec<Tweak> {
    case["tweaks"]
        .as_array()
        .unwrap()
        .iter()
        .zip(case["is_xonly"].as_array().unwrap())
        .map(|(t, mode)| Tweak {
            bytes: bytes32(t),
            x_only: mode.as_bool().unwrap(),
        })
        .collect()
}

fn run_deterministic_sign(
    secp: &Secp256k1<Verify>,
    group: &Group,
    case: &Value,
) -> Result<(PubNonce, PartialSig), Error> {
    let pubshares = group.pubshares_at(case).transpose()?;
    let secshare = group.secshare_at(case)?;
    let aggothernonce = case["aggothernonce"]
        .as_str()
        .map(|a| frost::parse_aggnonce(&unhex(&a.to_lowercase()), Contrib::AggOtherNonce))
        .transpose()?;
    let aux_rand = case["aux_rand"]
        .as_str()
        .map(|r| -> [u8; 32] { unhex(&r.to_lowercase()).try_into().unwrap() });
    frost::deterministic_sign(
        secp,
        &secshare,
        case["my_id"].as_u64().unwrap() as u32,
        aggothernonce.as_ref(),
        group.n,
        group.t,
        &ids_of(case),
        pubshares.as_deref(),
        &group.thresh_pk,
        &inline_tweaks(case),
        &unhex(&text(&case["msg"])),
        aux_rand.as_ref(),
    )
}

/// `DeterministicSign` gives the nonce and the partial signature the
/// file states, including for a sole signer with no other nonce, and
/// refuses every error case for the reason the file names.
#[test]
fn frost_det_sign_vectors() {
    let secp = Secp256k1::verification_only();
    let (mut checked, mut errors) = (0, 0);
    for group in groups(DET_SIGN) {
        for case in group.cases["valid_tests"].as_array().unwrap() {
            let (pubnonce, psig) = run_deterministic_sign(&secp, &group, case).unwrap();
            let expected = case["expected"].as_array().unwrap();
            assert_eq!(
                hex(&pubnonce.serialize()),
                text(&expected[0]),
                "{} {case}",
                group.id
            );
            assert_eq!(
                hex(&psig.serialize()),
                text(&expected[1]),
                "{} {case}",
                group.id
            );
            checked += 1;
        }
        for case in group.cases["error_tests"].as_array().unwrap() {
            let refused = run_deterministic_sign(&secp, &group, case).unwrap_err();
            assert_eq!(Some(refused), expected_error(case), "{} {case}", group.id);
            errors += 1;
        }
    }
    assert_eq!((checked, errors), (37, 48));
}

// ---------------------------------------------------------------------
// PartialSigAgg
// ---------------------------------------------------------------------

/// `PartialSigAgg` gives the 64-byte signature the file states, and that
/// signature is a BIP-340 signature of the message under the tweaked
/// group key.
#[test]
fn frost_sig_agg_vectors() {
    let secp = Secp256k1::new();
    let (mut checked, mut errors) = (0, 0);
    for group in groups(SIG_AGG) {
        for case in group.cases["valid_tests"].as_array().unwrap() {
            let msg = unhex(&text(&case["msg"]));
            let ctx = SessionContext {
                n: group.n,
                t: group.t,
                ids: ids_of(case),
                pubshares: group.pubshares_at(case).map(Result::unwrap),
                thresh_pk: group.thresh_pk,
                aggnonce: frost::parse_aggnonce(
                    &unhex(&text(&case["aggnonce"])),
                    Contrib::AggNonce,
                )
                .unwrap(),
                tweaks: group.tweaks_at(case),
                msg: msg.clone(),
            };
            let session = frost::session_values(&secp, &ctx).unwrap();
            let psigs: Vec<PartialSig> = case["psigs"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(position, p)| frost::parse_psig(&unhex(&text(p)), position).unwrap())
                .collect();
            let sig = frost::partial_sig_agg(&psigs, &session).unwrap();
            assert_eq!(hex(&sig), text(&case["expected"]), "{} {case}", group.id);
            secp.verify_schnorr(
                &schnorr::Signature::from_slice(&sig).unwrap(),
                &Message::from_digest_slice(&msg).unwrap(),
                &XOnlyPublicKey::from_slice(&session.aggregate.serialize_x_only()).unwrap(),
            )
            .unwrap();
            checked += 1;
        }
        for case in group.cases["error_tests"].as_array().unwrap() {
            let ctx = SessionContext {
                n: group.n,
                t: group.t,
                ids: ids_of(case),
                pubshares: group.pubshares_at(case).map(Result::unwrap),
                thresh_pk: group.thresh_pk,
                aggnonce: frost::parse_aggnonce(
                    &unhex(&text(&case["aggnonce"])),
                    Contrib::AggNonce,
                )
                .unwrap(),
                tweaks: group.tweaks_at(case),
                msg: unhex(&text(&case["msg"])),
            };
            let refused = case["psigs"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(position, p)| frost::parse_psig(&unhex(&text(p)), position))
                .collect::<Result<Vec<PartialSig>, Error>>()
                .and_then(|psigs| {
                    let session = frost::session_values(&secp, &ctx)?;
                    frost::partial_sig_agg(&psigs, &session)
                })
                .unwrap_err();
            assert_eq!(Some(refused), expected_error(case), "{} {case}", group.id);
            errors += 1;
        }
    }
    assert_eq!((checked, errors), (18, 8));
}

// ---------------------------------------------------------------------
// The trusted dealer
// ---------------------------------------------------------------------

/// Every `t`-sized subset of `0..n`.
fn subsets(n: usize, t: usize) -> Vec<Vec<u32>> {
    (0u32..1 << n)
        .filter(|mask| mask.count_ones() as usize == t)
        .map(|mask| (0..n as u32).filter(|i| mask >> i & 1 == 1).collect())
        .collect()
}

/// From any `t` of a group's secret shares, the other shares and the
/// whole public record come back: the same secret shares, the same
/// public shares, and the same group key.
#[test]
fn the_dealer_recovers_a_group_from_any_threshold_subset() {
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for group in groups(SIGN_VERIFY) {
        let secshares = group.real_secshares();
        let pubshares = group.real_pubshares();
        for subset in subsets(group.n, group.t) {
            let held: Vec<(u32, &SecShare)> = subset
                .iter()
                .map(|id| (*id, &secshares[*id as usize]))
                .collect();
            for target in 0..group.n as u32 {
                let recovered = frost::recover_share(group.t, &held, target).unwrap();
                assert_eq!(
                    hex(&recovered.secret_bytes()),
                    group.secshares[target as usize],
                    "{} share {target} from {subset:?}",
                    group.id
                );
            }
            let info = frost::recover_info(&secp, group.n, group.t, &held).unwrap();
            assert_eq!(info.thresh_pk, group.thresh_pk, "{}", group.id);
            assert_eq!(
                info.pubshares,
                pubshares.iter().copied().map(Some).collect::<Vec<_>>(),
                "{}",
                group.id
            );
            info.validate(&secp).unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 3 + 3 + 1 + 10);
}

/// A group's record is accepted; one whose public share is another
/// group's point, or one carrying fewer shares than the threshold, is
/// not, and a share that is no point is refused as it is read.
#[test]
fn a_record_is_refused_when_its_shares_are_not_the_groups() {
    let secp = Secp256k1::verification_only();
    let mut checked = 0;
    for group in groups(SIGN_VERIFY) {
        let pubshares: Vec<Option<PublicKey>> =
            group.real_pubshares().into_iter().map(Some).collect();
        let info = ThresholdInfo {
            t: group.t,
            thresh_pk: group.thresh_pk,
            pubshares: pubshares.clone(),
        };
        info.validate(&secp).unwrap();

        // The appended public share of every file is no point at all.
        assert_eq!(
            frost::parse_pubshare(&unhex(BOGUS_PUBSHARE), 0),
            Err(Error::InvalidPubShare { index: 0 })
        );

        // A point that is not this group's share, in a real share's
        // place: it is off the polynomial, gives another group key, or
        // leaves the shares interpolating to infinity.
        let mut swapped = pubshares.clone();
        swapped[group.n - 1] = Some(key(&group.pubshares[group.n + 1]));
        let refused = ThresholdInfo {
            t: group.t,
            thresh_pk: group.thresh_pk,
            pubshares: swapped,
        }
        .validate(&secp)
        .unwrap_err();
        assert!(
            matches!(
                refused,
                Error::PubSharesNotOnOnePolynomial
                    | Error::ThreshPkMismatch
                    | Error::ThreshPkInfinity
            ),
            "{} {refused}",
            group.id
        );

        // Fewer shares present than the threshold.
        let mut thinned = pubshares.clone();
        for share in thinned.iter_mut().take(group.n - group.t + 1) {
            *share = None;
        }
        assert_eq!(
            ThresholdInfo {
                t: group.t,
                thresh_pk: group.thresh_pk,
                pubshares: thinned,
            }
            .validate(&secp),
            Err(Error::TooFewPubShares),
            "{}",
            group.id
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// RFC 9591 Appendix F.5, which the reference's own dealer tests carry:
/// the 2-of-3 shares of a known group secret. Dealing from the first two
/// gives the third and the group key; any two rebuild the one left out.
#[test]
fn the_dealer_matches_the_rfc_9591_shares() {
    const SHARES: [&str; 3] = [
        "08f89ffe80ac94dcb920c26f3f46140bfc7f95b493f8310f5fc1ea2b01f4254c",
        "04f0feac2edcedc6ce1253b7fab8c86b856a797f44d83d82a385554e6e401984",
        "00e95d59dd0d46b0e303e500b62b7ccb0e555d49f5b849f5e748c071da8c0dbc",
    ];
    const SECRET: &str = "0d004150d27c3bf2a42f312683d35fac7394b1e9e318249c1bfe7f0795a83114";
    let secp = Secp256k1::verification_only();
    let share = |i: usize| SecShare::from_bytes(&unhex(SHARES[i]).try_into().unwrap()).unwrap();
    let group_key = SecShare::from_bytes(&unhex(SECRET).try_into().unwrap())
        .unwrap()
        .public_share(&secp);

    let dealt = frost::deal(&secp, 3, 2, &[(0, share(0)), (1, share(1))]).unwrap();
    assert_eq!(dealt.info.thresh_pk, group_key);
    assert_eq!(dealt.info.t, 2);
    for (i, dealt_share) in dealt.shares.iter().enumerate() {
        assert_eq!(hex(&dealt_share.secret_bytes()), SHARES[i]);
        assert_eq!(
            dealt.info.pubshares[i],
            Some(dealt_share.public_share(&secp))
        );
    }
    dealt.info.validate(&secp).unwrap();

    for (a, b, missing) in [(0, 1, 2), (1, 2, 0), (0, 2, 1)] {
        let (first, second) = (share(a), share(b));
        let held = [(a as u32, &first), (b as u32, &second)];
        assert_eq!(
            hex(&frost::recover_share(2, &held, missing as u32)
                .unwrap()
                .secret_bytes()),
            SHARES[missing]
        );
        assert_eq!(
            frost::recover_info(&secp, 3, 2, &held).unwrap().thresh_pk,
            group_key
        );
    }
}

/// A threshold of one gives every participant the same share, so any one
/// of them signs alone.
#[test]
fn a_threshold_of_one_gives_every_participant_the_same_share() {
    let secp = Secp256k1::verification_only();
    let only = SecShare::from_bytes(&[7u8; 32]).unwrap();
    let dealt = frost::deal(&secp, 4, 1, &[(2, only)]).unwrap();
    for share in &dealt.shares {
        assert_eq!(share.secret_bytes(), [7u8; 32]);
    }
    assert_eq!(dealt.info.thresh_pk, dealt.shares[0].public_share(&secp));
    dealt.info.validate(&secp).unwrap();
}

/// The dealer refuses a set of chosen shares that does not fix the
/// polynomial, and a group larger than BIP 445 allows.
#[test]
fn the_dealer_refuses_a_set_that_fixes_no_polynomial() {
    let secp = Secp256k1::verification_only();
    let share = |b: u8| SecShare::from_bytes(&[b; 32]).unwrap();
    assert_eq!(
        frost::deal(&secp, 3, 2, &[(0, share(1))]).unwrap_err(),
        Error::ChosenCount
    );
    assert_eq!(
        frost::deal(&secp, 3, 2, &[(0, share(1)), (1, share(2)), (2, share(3))]).unwrap_err(),
        Error::ChosenCount
    );
    assert_eq!(
        frost::deal(&secp, 3, 2, &[(1, share(1)), (1, share(2))]).unwrap_err(),
        Error::ChosenDuplicate
    );
    assert_eq!(
        frost::deal(&secp, 3, 2, &[(0, share(1)), (3, share(2))]).unwrap_err(),
        Error::ChosenIdOutOfRange
    );
    assert_eq!(
        frost::deal(&secp, 3, 0, &[]).unwrap_err(),
        Error::ThresholdRange
    );
    assert_eq!(
        frost::deal(&secp, 129, 1, &[(0, share(1))]).unwrap_err(),
        Error::TooManyParticipants
    );
}

/// A share is a scalar in `1..n`: zero and the curve order are no
/// shares, and the largest scalar below the order is one.
#[test]
fn a_share_is_a_scalar_below_the_curve_order() {
    let order = unhex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    let last = unhex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140");
    assert_eq!(
        SecShare::from_bytes(&[0u8; 32]).unwrap_err(),
        Error::SecShareOutOfRange
    );
    assert_eq!(
        SecShare::from_bytes(&order.try_into().unwrap()).unwrap_err(),
        Error::SecShareOutOfRange
    );
    assert!(SecShare::from_bytes(&last.try_into().unwrap()).is_ok());
    assert_eq!(
        SecNonce::from_bytes(&[0u8; 64]).unwrap_err(),
        Error::FirstSecNonceOutOfRange
    );
    let mut half_zero = [1u8; 64];
    half_zero[32..].copy_from_slice(&[0u8; 32]);
    assert_eq!(
        SecNonce::from_bytes(&half_zero).unwrap_err(),
        Error::SecondSecNonceOutOfRange
    );
}
