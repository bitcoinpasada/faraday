//! Wallets of the shapes Liana writes (a primary path now, recovery paths
//! behind relative delays), read from fixed descriptors: the ways each
//! spends, and the addresses Bitcoin Core's `deriveaddresses` gives for
//! the taproot ones, recorded in `tools/vectors/psbt/README.md`.

use faraday_bip::keys::Network;
use faraday_bip::policy::WalletPolicy;
use faraday_bip::spend::Lock;
use faraday_crypto::work::Compute;

/// Five keys from the fixtures.
const A: &str = "[73c5da0a/48h/1h/0h/2h]tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
const B: &str = "[3f635a63/48h/1h/0h/2h]tpubDFPtPArj4GzBEFHohegg1Xatrc1Fi9oSox5LzuSRX91miwQxuUrEpBxpvDRsmZYJKYFhgdK3UStsjC8JKXfUbMinjFqiEM4uNwzVaCaHpys";
const C: &str = "[73c5da0a/86h/1h/0h]tpubDDfvzhdVV4unsoKt5aE6dcsNsfeWbTgmLZPi8LQDYU2xixrYemMfWJ3BaVneH3u7DBQePdTwhpybaKRU95pi6PMUtLPBJLVQRpzEnjfjZzX";
const D: &str = "[3f635a63/86h/1h/0h]tpubDD4uFqwcQxcgHEhBFmoFbcLtEfSXw6bmLSeKeJHaqDtiXsvGRD6zCJ26HUxWp6ca6GAtNj6C3jyCGBAw2M9sW1bjiK1gFw6dckb6bKn8B3m";
const E: &str = "[b8688df1/86h/1h/0h]tpubDDHnNFFb1gf8qGYVejVx4GwzQPwcphirPssHFMWFcL9iajxm1wWd5ye22T7UQrVPjwwifJaCJGBAphuu8oePzYTCN7eZgm1KnyfNDmcfnsu";

/// Each way the wallet can be spent, as the keys it names and the waits
/// it imposes, so that a spend path is read without knowing what order
/// the compiler wrote the keys in.
fn ways(policy: &WalletPolicy) -> Vec<(Vec<String>, Vec<Lock>)> {
    let keys: Vec<String> = policy
        .keys()
        .iter()
        .map(|k| String::from(&*k.key_text()))
        .collect();
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

fn wallet(text: &str) -> WalletPolicy {
    WalletPolicy::from_descriptor(&Compute::unchecked(), text).expect("a wallet")
}

/// The unspendable internal keys of the two taproot wallets that have none
/// to spend: BIP-341's NUMS point over a chain code made from the leaves' keys.
const NUMS_TWO: &str = "tpubD6NzVbkrYhZ4Xc7sHx4R7YdxNd5kU29D9btbVAQwegYJ1xvbyxWZ2qQHxheE51nqZEtHMUsKk6E4XZhSbVrLCX4vJgBfcowgHeRSNweREV7";
const NUMS_THREE: &str = "tpubD6NzVbkrYhZ4YEMt6RPfFSdAoBk9tE5P1bwqnXvjGRetZw1XpDJC9juvdmMeAv962qpHkj1eXxupAhKY7rt4BTFgGP3MD5ab8HzZAa5aP26";
const CH: &str = "/<0;1>/*";

/// Two of three keys now, or one key after 4320 blocks, in both forms.
fn two_of_three() -> [WalletPolicy; 2] {
    [
        wallet(&format!(
            "wsh(or_d(multi(2,{A}{CH},{B}{CH},{C}{CH}),and_v(v:pkh({D}{CH}),older(4320))))"
        )),
        wallet(&format!(
            "tr({NUMS_THREE}{CH},{{and_v(v:pk({D}{CH}),older(4320)),multi_a(2,{A}{CH},{B}{CH},{C}{CH})}})"
        )),
    ]
}

/// One key now, one after 1008 blocks, one after 4320, in both forms.
fn two_delays() -> [WalletPolicy; 2] {
    [
        wallet(&format!(
            "wsh(or_i(and_v(v:pkh({E}{CH}),older(4320)),or_d(pk({A}{CH}),and_v(v:pkh({D}{CH}),older(1008)))))"
        )),
        wallet(&format!(
            "tr({A}{CH},{{and_v(v:pk({D}{CH}),older(1008)),and_v(v:pk({E}{CH}),older(4320))}})"
        )),
    ]
}

#[test]
fn the_ways_a_recovery_wallet_spends_are_its_primary_and_recovery_paths() {
    for (i, policy) in two_of_three().iter().enumerate() {
        assert_eq!(
            ways(policy),
            sorted(vec![
                way(&[A, B], &[]),
                way(&[A, C], &[]),
                way(&[B, C], &[]),
                way(&[D], &[Lock::Blocks(4_320)]),
            ]),
            "form {i}"
        );
    }
    for (i, policy) in two_delays().iter().enumerate() {
        assert_eq!(
            ways(policy),
            sorted(vec![
                way(&[A], &[]),
                way(&[D], &[Lock::Blocks(1_008)]),
                way(&[E], &[Lock::Blocks(4_320)]),
            ]),
            "form {i}"
        );
    }
    // The longest wait that can be written is still a wallet.
    let longest = wallet(&format!(
        "wsh(or_d(pk({A}{CH}),and_v(v:pkh({B}{CH}),older(65535))))"
    ));
    assert_eq!(
        ways(&longest),
        sorted(vec![way(&[A], &[]), way(&[B], &[Lock::Blocks(65_535)])])
    );
}

/// A taproot wallet whose primary is one key has that key on the key path;
/// one whose primary needs two cannot, so its internal key is the
/// unspendable one and both keys are still wanted for a spend with no wait.
#[test]
fn an_unspendable_internal_key_is_not_a_way_to_spend() {
    let one = wallet(&format!("tr({A}{CH},and_v(v:pk({B}{CH}),older(52560)))"));
    assert_eq!(
        ways(&one),
        sorted(vec![way(&[A], &[]), way(&[B], &[Lock::Blocks(52_560)])])
    );
    let two = wallet(&format!(
        "tr({NUMS_TWO}{CH},{{and_v(v:pk({D}{CH}),older(52560)),and_v(v:pk({A}{CH}),pk({B}{CH}))}})"
    ));
    assert_eq!(
        ways(&two),
        sorted(vec![way(&[A, B], &[]), way(&[D], &[Lock::Blocks(52_560)])])
    );
}

/// The addresses Bitcoin Core's `deriveaddresses` gives for the two
/// taproot Liana-shaped wallets: the one whose primary key takes the key
/// path, and the one whose internal key is unspendable.
#[test]
fn the_taproot_recovery_wallets_pay_the_addresses_bitcoin_core_derives() {
    let c = Compute::unchecked();
    let cases = [
        (
            format!("tr({A}{CH},and_v(v:pk({B}{CH}),older(52560)))"),
            [
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
            format!(
                "tr({NUMS_TWO}{CH},{{and_v(v:pk({D}{CH}),older(52560)),and_v(v:pk({A}{CH}),pk({B}{CH}))}})"
            ),
            [
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
    for (text, addresses) in &cases {
        let policy = wallet(text);
        for (index, (receive, change)) in addresses.iter().enumerate() {
            let index = index as u32;
            let at = |change| {
                policy
                    .address_at(&c, Network::Regtest, change, index)
                    .expect("an address")
                    .to_string()
            };
            assert_eq!(at(false), *receive);
            assert_eq!(at(true), *change);
        }
    }
}
