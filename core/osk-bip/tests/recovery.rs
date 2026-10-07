//! Recovery wallets: the wallet built from primary keys, recovery keys
//! and a delay, and the same wallet read back out of a descriptor.
//!
//! The shapes are checked against Liana's: the committed fixtures in
//! `tools/vectors/psbt/` are descriptors Liana wrote, and the wallet
//! this crate builds from the same keys and the same delay is compared
//! against them character for character.

use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_bip::recovery::{
    BLOCKS_A_DAY, Error, Form, MAX_DELAY, Path, Recovery, RecoveryPolicy, blocks_for_days,
    days_of_blocks,
};
use osk_bip::spend::Lock;

/// The wallets Liana wrote, as `tools/vectors/psbt/` holds them.
const LIANA: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.policy");
const TREE: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.policy");

/// Five keys from the fixtures, used here as the parts a person picks.
const A: &str = "[73c5da0a/48'/1'/0'/2']tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
const B: &str = "[3f635a63/48'/1'/0'/2']tpubDFPtPArj4GzBEFHohegg1Xatrc1Fi9oSox5LzuSRX91miwQxuUrEpBxpvDRsmZYJKYFhgdK3UStsjC8JKXfUbMinjFqiEM4uNwzVaCaHpys";
const C: &str = "[73c5da0a/86'/1'/0']tpubDDfvzhdVV4unsoKt5aE6dcsNsfeWbTgmLZPi8LQDYU2xixrYemMfWJ3BaVneH3u7DBQePdTwhpybaKRU95pi6PMUtLPBJLVQRpzEnjfjZzX";
const D: &str = "[3f635a63/86'/1'/0']tpubDD4uFqwcQxcgHEhBFmoFbcLtEfSXw6bmLSeKeJHaqDtiXsvGRD6zCJ26HUxWp6ca6GAtNj6C3jyCGBAw2M9sW1bjiK1gFw6dckb6bKn8B3m";
const E: &str = "[b8688df1/86'/1'/0']tpubDDHnNFFb1gf8qGYVejVx4GwzQPwcphirPssHFMWFcL9iajxm1wWd5ye22T7UQrVPjwwifJaCJGBAphuu8oePzYTCN7eZgm1KnyfNDmcfnsu";

fn recovery(delay: u32, path: Path) -> Recovery {
    Recovery { delay, path }
}

/// Each way the wallet can be spent, as the keys it names and the waits
/// it imposes, so that a spend path is read without knowing what order
/// the compiler wrote the keys in.
fn ways(policy: &WalletPolicy) -> Vec<(Vec<String>, Vec<Lock>)> {
    let keys: Vec<String> = policy.keys().iter().map(|k| k.key_text()).collect();
    let mut ways: Vec<(Vec<String>, Vec<Lock>)> = policy
        .spend_paths()
        .into_iter()
        .map(|path| {
            (
                path.keys.iter().map(|i| keys[*i].clone()).collect(),
                path.locks,
            )
        })
        .collect();
    ways.sort();
    ways
}

fn way(keys: &[&str], locks: &[Lock]) -> (Vec<String>, Vec<Lock>) {
    (
        keys.iter().map(|k| String::from(*k)).collect(),
        locks.to_vec(),
    )
}

fn sorted(mut ways: Vec<(Vec<String>, Vec<Lock>)>) -> Vec<(Vec<String>, Vec<Lock>)> {
    ways.sort();
    ways
}

/// The wallet Liana wrote for one key now and one key after a year is
/// the wallet this crate builds from the same two keys and the same
/// delay, character for character.
#[test]
fn a_segwit_recovery_wallet_is_the_descriptor_liana_wrote() {
    let built = RecoveryPolicy {
        primary: Path::single(A),
        recovery: vec![recovery(52_560, Path::single(B))],
    }
    .to_wallet_policy(Form::SegWit)
    .expect("a recovery wallet");
    let fixture = WalletPolicy::parse_any(LIANA).expect("the fixture wallet");
    assert_eq!(built.to_descriptor(), fixture.to_descriptor());
    assert_eq!(format!("{}\n", built.to_text()), *LIANA);
}

/// That same fixture, read the other way: a person opening Liana's
/// wallet is told who spends now and who spends after the wait.
#[test]
fn liana_s_own_wallet_reads_back_as_its_keys_and_its_delay() {
    let policy = WalletPolicy::parse_any(LIANA).expect("the fixture wallet");
    assert_eq!(
        RecoveryPolicy::from_wallet_policy(&policy),
        Some(RecoveryPolicy {
            primary: Path::single(A),
            recovery: vec![recovery(52_560, Path::single(B))],
        })
    );
}

/// The taproot fixture keeps two keys on the always-available path —
/// one of them the key path — and one key behind the wait.
#[test]
fn a_taproot_wallet_with_a_leaf_that_waits_reads_back_as_two_paths() {
    let policy = WalletPolicy::parse_any(TREE).expect("the fixture wallet");
    let read = RecoveryPolicy::from_wallet_policy(&policy).expect("a recovery wallet");
    assert_eq!(read.primary.threshold, 1);
    assert_eq!(read.primary.keys, vec![String::from(C), String::from(E)]);
    assert_eq!(read.recovery, vec![recovery(4_320, Path::single(D))]);
}

/// A wallet with no timelocked path is not a recovery wallet, whatever
/// else it is.
#[test]
fn a_wallet_that_never_waits_is_not_a_recovery_wallet() {
    for text in [
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))",
        "tr(@0/**)",
        "wpkh(@0/**)",
    ] {
        let keys: Vec<&str> = [A, B, C][..text.matches('@').count()].to_vec();
        let policy = WalletPolicy::from_parts(text, &keys).expect(text);
        assert_eq!(RecoveryPolicy::from_wallet_policy(&policy), None, "{text}");
    }
}

/// Every wallet this crate builds is read back as the wallet it was
/// built from, in both forms and whatever the paths hold.
#[test]
fn every_built_wallet_reads_back_as_the_paths_it_was_built_from() {
    let cases = [
        RecoveryPolicy {
            primary: Path::single(A),
            recovery: vec![recovery(52_560, Path::single(B))],
        },
        RecoveryPolicy {
            primary: Path::single(A),
            recovery: vec![recovery(1_008, Path::multi(2, &[C, D, E]))],
        },
        RecoveryPolicy {
            primary: Path::multi(2, &[A, B, C]),
            recovery: vec![recovery(4_320, Path::single(D))],
        },
        RecoveryPolicy {
            primary: Path::multi(2, &[A, B]),
            recovery: vec![recovery(4_320, Path::single(D))],
        },
        RecoveryPolicy {
            primary: Path::single(A),
            recovery: vec![
                recovery(1_008, Path::single(D)),
                recovery(4_320, Path::single(E)),
            ],
        },
        RecoveryPolicy {
            primary: Path::multi(2, &[A, B]),
            recovery: vec![
                recovery(1_008, Path::multi(2, &[C, D])),
                recovery(26_280, Path::single(E)),
            ],
        },
    ];
    for wanted in cases {
        for form in [Form::SegWit, Form::Taproot] {
            let policy = wanted
                .to_wallet_policy(form)
                .unwrap_or_else(|e| panic!("{wanted:?} in {form:?}: {e}"));
            assert_eq!(
                RecoveryPolicy::from_wallet_policy(&policy).as_ref(),
                Some(&wanted),
                "{wanted:?} in {form:?} is {}",
                policy.to_descriptor()
            );
            // The wallet written down and read back from its text is
            // still the same wallet.
            let again = WalletPolicy::parse_any(&policy.to_text()).expect("the wallet's own text");
            assert_eq!(again, policy);
        }
    }
}

/// The ways the built wallet can be spent are the paths it was built
/// from: the primary's keys with no wait, each recovery path's keys
/// after its wait.
#[test]
fn the_ways_a_recovery_wallet_spends_are_the_paths_it_was_built_from() {
    let two_of_three = RecoveryPolicy {
        primary: Path::multi(2, &[A, B, C]),
        recovery: vec![recovery(4_320, Path::single(D))],
    };
    for form in [Form::SegWit, Form::Taproot] {
        let policy = two_of_three.to_wallet_policy(form).expect("a wallet");
        assert_eq!(
            ways(&policy),
            sorted(vec![
                way(&[A, B], &[]),
                way(&[A, C], &[]),
                way(&[B, C], &[]),
                way(&[D], &[Lock::Blocks(4_320)]),
            ]),
            "{form:?}"
        );
    }

    let two_delays = RecoveryPolicy {
        primary: Path::single(A),
        recovery: vec![
            recovery(1_008, Path::single(D)),
            recovery(4_320, Path::single(E)),
        ],
    };
    for form in [Form::SegWit, Form::Taproot] {
        let policy = two_delays.to_wallet_policy(form).expect("a wallet");
        assert_eq!(
            ways(&policy),
            sorted(vec![
                way(&[A], &[]),
                way(&[D], &[Lock::Blocks(1_008)]),
                way(&[E], &[Lock::Blocks(4_320)]),
            ]),
            "{form:?}"
        );
    }
}

/// A taproot recovery wallet whose primary is one key spends that key
/// on the key path; one whose primary needs more than one key cannot,
/// so its internal key is the unspendable one every holder of the
/// descriptor can recompute.
#[test]
fn a_taproot_wallet_puts_one_primary_key_on_the_key_path() {
    let one = RecoveryPolicy {
        primary: Path::single(A),
        recovery: vec![recovery(52_560, Path::single(B))],
    }
    .to_wallet_policy(Form::Taproot)
    .expect("a wallet");
    assert!(
        one.to_descriptor().starts_with(&format!("tr({A}/<0;1>/*,")),
        "{}",
        one.to_descriptor()
    );

    let two = RecoveryPolicy {
        primary: Path::multi(2, &[A, B]),
        recovery: vec![recovery(52_560, Path::single(D))],
    }
    .to_wallet_policy(Form::Taproot)
    .expect("a wallet");
    assert!(
        two.to_descriptor().starts_with("tr(tpub"),
        "{}",
        two.to_descriptor()
    );
    // Nothing of the primary is on the key path, so both keys are still
    // wanted for a spend that imposes no wait.
    assert_eq!(
        ways(&two),
        sorted(vec![way(&[A, B], &[]), way(&[D], &[Lock::Blocks(52_560)])])
    );
}

/// The refusals, each by the thing a person got wrong.
#[test]
fn a_wallet_no_one_could_recover_is_refused() {
    let primary = || Path::single(A);
    let cases = [
        (
            RecoveryPolicy {
                primary: primary(),
                recovery: Vec::new(),
            },
            Error::NoRecovery,
        ),
        (
            RecoveryPolicy {
                primary: primary(),
                recovery: vec![recovery(0, Path::single(B))],
            },
            Error::Delay,
        ),
        (
            RecoveryPolicy {
                primary: primary(),
                recovery: vec![recovery(MAX_DELAY + 1, Path::single(B))],
            },
            Error::Delay,
        ),
        (
            RecoveryPolicy {
                primary: primary(),
                recovery: vec![
                    recovery(1_008, Path::single(B)),
                    recovery(1_008, Path::single(C)),
                ],
            },
            Error::Delay,
        ),
        (
            RecoveryPolicy {
                primary: primary(),
                recovery: vec![recovery(1_008, Path::single(A))],
            },
            Error::Key,
        ),
        (
            RecoveryPolicy {
                primary: Path::multi(3, &[A, B]),
                recovery: vec![recovery(1_008, Path::single(C))],
            },
            Error::Path,
        ),
        (
            RecoveryPolicy {
                primary: Path {
                    threshold: 1,
                    keys: vec![String::from("not a key")],
                },
                recovery: vec![recovery(1_008, Path::single(C))],
            },
            Error::Key,
        ),
    ];
    for (policy, error) in cases {
        for form in [Form::SegWit, Form::Taproot] {
            assert_eq!(policy.to_wallet_policy(form), Err(error), "{policy:?}");
        }
    }
}

/// A wait stated in days is that many days of blocks, and the longest
/// wait a relative timelock can hold is about fifteen months.
#[test]
fn a_delay_in_days_is_that_many_days_of_blocks() {
    assert_eq!(blocks_for_days(1), Some(BLOCKS_A_DAY));
    assert_eq!(blocks_for_days(365), Some(52_560));
    assert_eq!(days_of_blocks(52_560), 365);
    assert_eq!(days_of_blocks(52_559), 364);
    assert_eq!(blocks_for_days(0), None);
    assert_eq!(blocks_for_days(455), Some(65_520));
    assert_eq!(blocks_for_days(456), None);
    assert_eq!(blocks_for_days(u32::MAX), None);

    // The longest wait that can be written is still a wallet.
    let policy = RecoveryPolicy {
        primary: Path::single(A),
        recovery: vec![recovery(MAX_DELAY, Path::single(B))],
    }
    .to_wallet_policy(Form::SegWit)
    .expect("a wallet");
    assert_eq!(
        ways(&policy),
        sorted(vec![way(&[A], &[]), way(&[B], &[Lock::Blocks(MAX_DELAY)])])
    );
}

/// The addresses Bitcoin Core's `deriveaddresses` gives for the two
/// taproot recovery wallets, recorded in `tools/vectors/psbt/README.md`:
/// the one whose primary key takes the key path, and the one whose
/// internal key is unspendable.
#[test]
fn the_taproot_recovery_wallets_pay_the_addresses_bitcoin_core_derives() {
    /// The primary keys, the threshold over them, and the wallet's
    /// first two receive and change addresses.
    type Case = (
        &'static [&'static str],
        usize,
        &'static [(&'static str, &'static str)],
    );
    const CASES: &[Case] = &[
        (
            &[A],
            1,
            &[
                (
                    "bcrt1pw3s5nepc02jlyvk8hqqjs3ujwatfagv46t782nk9r9eh97grnwnsmgpq7z",
                    "bcrt1pzsezgsyppy7md9v3m788trauf8ext7lthk7yezhprrd42u63c92sjtnn7m",
                ),
                (
                    "bcrt1p4autzdywsrar9ea5k28acfetg0m6lm8ghwphdgklr2fphuefpxlspu9j37",
                    "bcrt1puh67whutvxxcpumrunw6l7h7q8avlprcx9xvjs3qnyqcqcfjgndszym3gv",
                ),
            ],
        ),
        (
            &[A, B],
            2,
            &[
                (
                    "bcrt1pswh84u67t65juskhtdgg6racc76txy69pl75t2axf400j2x0x3kq0u5qxn",
                    "bcrt1pqqzwemhxyn255eg6nle9nydaudllrq8u3uwja57nxrc27p55m5xqe9fcc6",
                ),
                (
                    "bcrt1paglrj4pjtjwm2fu4c8pkzz5fp7z0a4aed3u7ua7ahzzuwj4s0xhqsqvp2l",
                    "bcrt1p8uunumevnv4y7dqw6wk08cdppxh0r8paksx4vx8g42twlyychnxsma6g5w",
                ),
            ],
        ),
    ];
    for (keys, threshold, addresses) in CASES {
        let recovery_key = if keys.len() == 1 { B } else { D };
        let policy = RecoveryPolicy {
            primary: Path::multi(*threshold, keys),
            recovery: vec![recovery(52_560, Path::single(recovery_key))],
        }
        .to_wallet_policy(Form::Taproot)
        .expect("a wallet");
        for (index, (receive, change)) in addresses.iter().enumerate() {
            let index = index as u32;
            assert_eq!(
                policy
                    .address_at(Network::Regtest, false, index)
                    .expect("an address")
                    .to_string(),
                *receive
            );
            assert_eq!(
                policy
                    .address_at(Network::Regtest, true, index)
                    .expect("an address")
                    .to_string(),
                *change
            );
        }
    }
}
