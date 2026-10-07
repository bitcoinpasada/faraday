//! Miniscript and taproot-tree wallets: the addresses they pay to, the
//! ways they can be spent, and the forms they are written in.
//!
//! Addresses are checked against `miniscript` used directly, which is
//! the second implementation the review of an address needs, and against
//! the addresses recorded in `tools/vectors/psbt/README.md`, which
//! Bitcoin Core's `deriveaddresses` produced.

use std::str::FromStr;

use osk_bip::keys::{Network, ScriptType};
use osk_bip::miniscript::Descriptor;
use osk_bip::miniscript::descriptor::DescriptorPublicKey;
use osk_bip::policy::{Error, MiniscriptKind, Template, WalletPolicy};
use osk_bip::spend::{Lock, SpendPath};

/// The two fixture wallets, as `tools/vectors/psbt/` holds them.
const LIANA: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.policy");
const TREE: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.policy");

fn path(keys: &[usize], locks: &[Lock]) -> SpendPath {
    SpendPath {
        keys: keys.to_vec(),
        locks: locks.to_vec(),
    }
}

/// The address a wallet pays to is the address `miniscript` derives from
/// the same descriptor, on both chains and at more than one index.
fn matches_miniscript(policy: &WalletPolicy, network: Network) {
    let descriptor = Descriptor::<DescriptorPublicKey>::from_str(&policy.to_descriptor())
        .expect("the wallet's own descriptor");
    let chains = descriptor
        .into_single_descriptors()
        .expect("a receive chain and a change chain");
    let secp = bitcoin::secp256k1::Secp256k1::verification_only();
    for (chain, descriptor) in chains.iter().enumerate() {
        for index in [0u32, 1, 7] {
            let theirs = descriptor
                .clone()
                .at_derivation_index(index)
                .expect("index")
                .derived_descriptor(&secp)
                .expect("derivation")
                .address(bitcoin::Network::from(network))
                .expect("an address");
            let ours = policy
                .address_at(network, chain == 1, index)
                .expect("an address");
            assert_eq!(ours, theirs, "chain {chain} index {index}");
        }
    }
}

/// The Liana-shaped wallet: one key spends now, the other spends alone
/// after a year of blocks.
#[test]
fn a_wsh_miniscript_wallet_names_its_two_spend_paths() {
    let policy = WalletPolicy::parse_any(LIANA).expect("the fixture wallet");
    assert_eq!(
        policy.template(),
        Template::Miniscript {
            kind: MiniscriptKind::Wsh
        }
    );
    assert_eq!(policy.script_type(), ScriptType::NativeSegwit);
    assert_eq!(policy.quorum(), None);
    assert_eq!(policy.keys().len(), 2);
    assert_eq!(
        policy.spend_paths(),
        vec![path(&[0], &[]), path(&[1], &[Lock::Blocks(52_560)]),]
    );
    matches_miniscript(&policy, Network::Regtest);
}

/// The taproot tree: the key path first, then the leaf that waits and
/// the leaf that does not.
#[test]
fn a_taproot_tree_wallet_lists_the_key_path_before_its_leaves() {
    let policy = WalletPolicy::parse_any(TREE).expect("the fixture wallet");
    assert_eq!(policy.template(), Template::Tree);
    assert_eq!(policy.script_type(), ScriptType::Taproot);
    assert_eq!(policy.keys().len(), 3);
    assert_eq!(
        policy.spend_paths(),
        vec![
            path(&[0], &[]),
            path(&[1], &[Lock::Blocks(4_320)]),
            path(&[2], &[]),
        ]
    );
    matches_miniscript(&policy, Network::Regtest);
}

/// The addresses Bitcoin Core's `deriveaddresses` gives for the same two
/// descriptors, recorded in `tools/vectors/psbt/README.md`.
#[test]
fn the_fixture_wallets_pay_the_addresses_bitcoin_core_derives() {
    const CASES: &[(&str, &[(&str, &str)])] = &[
        (
            LIANA,
            &[
                (
                    "bcrt1qcn77knrrdsk3wpjmndhe4xuntfqq3ft7xtr0yu23grfeqgsxkn2qjk0fz5",
                    "bcrt1q5kpvx7hd28xgmxq2rv5cvvwu2dtm5qfhxzsdg7300jjy3r694r2qn045yn",
                ),
                (
                    "bcrt1qk93r8dd9jsf9ev4lzdzknj49se3fkmq9hju0tx8mwuhchfahn28q0l7hyz",
                    "bcrt1qemhk2n6cfm5w9xqhuzp94wgwprclmwvkk2ex9fksr55chlmjnugq083cg5",
                ),
            ],
        ),
        (
            TREE,
            &[
                (
                    "bcrt1p0sf2utyzdhg8gcek3zjrh638jyns5dnfk0g6l25a9aem2v9lwjhqm20kl5",
                    "bcrt1pw60dya6xsx3y3xh35xu5r3xzdyh2j942kaz9gg8qtfhg7dz7t0usv0ge4q",
                ),
                (
                    "bcrt1pn4y4wpq7zdvjq8kyy5r6mfkckxkdvtwqcunp2mh80mqf4x47qu2q7gaumk",
                    "bcrt1pnxumakwwk2yr6ejukntjxkf00ap8e06t6axel3jlsqjng9l9yuqqncymvc",
                ),
            ],
        ),
    ];
    for (text, addresses) in CASES {
        let policy = WalletPolicy::parse_any(text).expect("the fixture wallet");
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

/// Both forms of one miniscript wallet are one wallet: the two-part
/// policy the fixture holds, and the descriptor with its checksum.
#[test]
fn a_miniscript_wallet_reads_the_same_from_both_forms() {
    for text in [LIANA, TREE] {
        let policy = WalletPolicy::parse_any(text).expect("the fixture wallet");
        assert_eq!(format!("{}\n", policy.to_text()), *text);
        let descriptor = policy.to_descriptor_checksummed();
        assert_eq!(WalletPolicy::parse_any(&descriptor).unwrap(), policy);
        // The checksum this crate computes is the one `miniscript`
        // writes for the same descriptor.
        let theirs = Descriptor::<DescriptorPublicKey>::from_str(&policy.to_descriptor())
            .expect("a descriptor")
            .to_string();
        assert_eq!(theirs, descriptor);
    }
}

/// A key with a receive chain and nothing else is not a wallet: its
/// change could never be verified.
#[test]
fn a_miniscript_key_without_both_chains_is_refused() {
    let policy = WalletPolicy::parse_any(LIANA).expect("the fixture wallet");
    let one_chain = policy.to_descriptor().replace("/<0;1>/*", "/0/*");
    assert_eq!(
        WalletPolicy::from_descriptor(&one_chain),
        Err(Error::Derivation)
    );
}

/// A threshold with a recovery leaf: every combination of keys that
/// satisfies the threshold is one line, and the recovery path is another.
#[test]
fn a_threshold_lists_every_combination_that_satisfies_it() {
    let keys: Vec<String> = WalletPolicy::parse_any(TREE)
        .expect("the fixture wallet")
        .keys()
        .iter()
        .map(|k| k.key_text())
        .collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let policy = WalletPolicy::from_parts(
        "wsh(or_d(multi(2,@0/**,@1/**,@2/**),and_v(v:pkh(@2/**),older(1008))))",
        &refs,
    )
    .expect("a 2-of-3 with a recovery leaf");
    assert_eq!(
        policy.spend_paths(),
        vec![
            path(&[0, 1], &[]),
            path(&[0, 2], &[]),
            path(&[1, 2], &[]),
            path(&[2], &[Lock::Blocks(1_008)]),
        ]
    );
}

/// `after()` above 500 000 000 is a time, below it a height.
#[test]
fn an_absolute_locktime_is_a_height_or_a_time() {
    let policy = WalletPolicy::parse_any(LIANA).expect("the fixture wallet");
    let keys: Vec<String> = policy.keys().iter().map(|k| k.key_text()).collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    for (template, lock) in [
        (
            "wsh(or_d(pk(@0/**),and_v(v:pkh(@1/**),after(800000))))",
            Lock::Height(800_000),
        ),
        (
            "wsh(or_d(pk(@0/**),and_v(v:pkh(@1/**),after(1789000000))))",
            Lock::Time(1_789_000_000),
        ),
    ] {
        let policy = WalletPolicy::from_parts(template, &refs).expect(template);
        assert_eq!(
            policy.spend_paths(),
            vec![path(&[0], &[]), path(&[1], &[lock])]
        );
    }
}
