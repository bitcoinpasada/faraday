//! A backup restores to the same wallet: for random seeds, passphrases,
//! share counts and thresholds, each form the keys crate writes is read back
//! (BIP-39's words by the `bip39` crate, which Faraday did not write, the
//! rest by the decoders that open published sets), and the wallet it gives
//! is the original's: the master fingerprint and the first receive and
//! change addresses of the native segwit account. A split restores from
//! every sufficient subset and refuses every insufficient one; it never
//! gives another key. Seed XOR has no threshold, so a part short is another
//! wallet, and the tests say that.
//!
//! The vault file and the QR vault are sealed under a random passphrase
//! (non-ASCII and long ones among them), opened and read back as the same
//! key; a wrong passphrase opens nothing.
//!
//! Each property runs a fixed number of cases from a fixed seed, sized by the
//! cost of the form (SLIP-39's key stretching is the dearest), so a run is
//! the same everywhere and leaves no regression files.
#![allow(clippy::disallowed_methods)]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::slip39::{self, GroupSpec, Share};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_keys::operations::{
    WordsForm, codex32_random_len, codex32_seed, seed_xor_random_len, slip39_random_len,
    slip39_seed,
};
use faraday_keys::vault::{self, Names, OpenVaults, SEAL_RANDOM_LEN};
use faraday_keys::{Keys, QR_RANDOM_LEN, QrHolds, QrOpenJob, Reveal};
use faraday_vault::records::{field, kind, words_payload};
use faraday_vault::{Cost, Record};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// A fixed number of cases, a fixed seed, nothing written on a failure and a
/// short shrink: the same run everywhere, and quick.
fn config(cases: u32) -> Config {
    Config {
        cases,
        failure_persistence: None,
        max_shrink_iters: 8,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x0fa7_ada1),
        ..Config::default()
    }
}

/// What a wallet is to the person who restores it.
#[derive(Debug, PartialEq, Eq)]
struct Wallet {
    fingerprint: [u8; 4],
    receive: String,
    change: String,
}

fn wallet(c: &Compute, master: &MasterKey) -> Wallet {
    let account = master
        .account_xpub(c, ScriptType::NativeSegwit, 0)
        .expect("an account");
    Wallet {
        fingerprint: master.fingerprint().0,
        receive: account.address(false, 0).expect("an address").to_string(),
        change: account.address(true, 0).expect("an address").to_string(),
    }
}

/// The wallet of BIP-39 `words` and `passphrase`, by Faraday's BIP-39.
fn words_wallet(c: &Compute, words: &str, passphrase: &str) -> Wallet {
    let m = Mnemonic::parse(c, Language::English, words).expect("the words");
    let seed = m.to_seed(c, passphrase.as_bytes()).expect("a seed");
    wallet(c, &MasterKey::from_seed(c, &seed, Network::Mainnet))
}

/// The wallet of a mnemonic the `bip39` crate holds, seeded by it.
fn outside_wallet(c: &Compute, m: &bip39::Mnemonic, passphrase: &str) -> Wallet {
    let seed = m.to_seed_normalized(passphrase);
    wallet(
        c,
        &MasterKey::from_seed(c, &Secret::new(seed), Network::Mainnet),
    )
}

/// The wallet of a master seed of 16 to 64 bytes (SLIP-39, codex32).
fn seed_wallet(c: &Compute, seed: &[u8]) -> Wallet {
    let bytes = SeedBytes::new(seed).expect("a seed length");
    wallet(
        c,
        &MasterKey::from_seed_bytes(c, &Secret::new(bytes), Network::Mainnet),
    )
}

fn sentence(entropy: &[u8]) -> String {
    let m = Mnemonic::from_entropy(Language::English, entropy).expect("entropy");
    m.indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A key of `words` with `passphrase` in a session.
fn keys_of_words(c: &Compute, words: &str, passphrase: &str) -> Keys {
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(c, words, passphrase, "", None)
        .expect("added");
    keys
}

fn keys_of_seed(c: &Compute, seed: &[u8]) -> Keys {
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(c, seed, "").expect("added");
    keys
}

/// The entropy of 12, 15, 18, 21 or 24 words.
fn entropy() -> impl Strategy<Value = Vec<u8>> {
    (0usize..5, prop::collection::vec(any::<u8>(), 32)).prop_map(|(n, mut bytes)| {
        bytes.truncate(16 + 4 * n);
        bytes
    })
}

/// A BIP-39 passphrase of printable ASCII.
fn passphrase() -> impl Strategy<Value = String> {
    "[ -~]{0,16}"
}

/// Bytes a split draws from, fixed by the case.
fn pool(len: usize) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), len)
}

fn hex_decode(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

/// A paper form of BIP-39 words read back by the `bip39` crate.
fn read_paper(form: WordsForm, text: &str) -> bip39::Mnemonic {
    let list = bip39::Language::English.word_list();
    match form {
        WordsForm::Words => bip39::Mnemonic::parse_in_normalized(bip39::Language::English, text),
        WordsForm::HexEntropy => {
            bip39::Mnemonic::from_entropy_in(bip39::Language::English, &hex_decode(text))
        }
        WordsForm::Numbers => {
            // A line a word: its number from 1, then its index in brackets.
            let words: Vec<&str> = text
                .lines()
                .map(|l| {
                    let n: usize = l.split_whitespace().next().unwrap().parse().unwrap();
                    list[n - 1]
                })
                .collect();
            bip39::Mnemonic::parse_in_normalized(bip39::Language::English, &words.join(" "))
        }
        WordsForm::Starts => {
            // Each word's first four letters: a word of fewer is whole.
            let words: Vec<&str> = text
                .split_whitespace()
                .map(|t| {
                    if let Some(whole) = list.iter().find(|w| **w == t) {
                        whole
                    } else {
                        let found = bip39::Language::English.words_by_prefix(t);
                        assert_eq!(found.len(), 1, "{t}");
                        found[0]
                    }
                })
                .collect();
            bip39::Mnemonic::parse_in_normalized(bip39::Language::English, &words.join(" "))
        }
    }
    .expect("the paper form reads as words")
}

proptest! {
    #![proptest_config(config(40))]

    /// BIP-39 words, in every way the keys crate writes them, are the same
    /// wallet read by the `bip39` crate, with a passphrase.
    #[test]
    fn the_words_and_their_paper_forms_restore_the_wallet(
        entropy in entropy(),
        pass in passphrase(),
    ) {
        let c = compute();
        let words = sentence(&entropy);
        let original = words_wallet(&c, &words, &pass);
        // The words are the entropy to an implementation Faraday did not write.
        let outside = bip39::Mnemonic::from_entropy_in(bip39::Language::English, &entropy).unwrap();
        prop_assert_eq!(outside.to_string(), words.clone());
        prop_assert_eq!(&outside_wallet(&c, &outside, &pass), &original);
        let keys = keys_of_words(&c, &words, &pass);
        for form in [
            WordsForm::Words,
            WordsForm::HexEntropy,
            WordsForm::Numbers,
            WordsForm::Starts,
        ] {
            let written = keys
                .words_written(&c, 0, form, Reveal::PaperForm)
                .expect("written")
                .release();
            let read = read_paper(form, written.as_str());
            prop_assert_eq!(&outside_wallet(&c, &read, &pass), &original, "{:?}", form);
        }
    }

    /// A SeedQR and a CompactSeedQR of 12 or 24 words read back as the same
    /// words, and the standard one's digits are the BIP's: four a word.
    #[test]
    fn both_seedqr_forms_restore_the_wallet(
        long in any::<bool>(),
        bytes in pool(32),
        pass in passphrase(),
    ) {
        let c = compute();
        let entropy = &bytes[..if long { 32 } else { 16 }];
        let words = sentence(entropy);
        let original = words_wallet(&c, &words, &pass);
        let m = Mnemonic::parse(&c, Language::English, &words).unwrap();
        let outside = bip39::Mnemonic::from_entropy_in(bip39::Language::English, entropy).unwrap();
        let published: String = outside
            .words()
            .map(|w| {
                let i = bip39::Language::English.word_list().iter().position(|x| *x == w).unwrap();
                format!("{i:04}")
            })
            .collect();
        let digits = faraday_codec::seedqr::to_digits(&m);
        prop_assert_eq!(digits.expose().as_bytes(), published.as_bytes());
        let from_digits = faraday_keys::words::from_seedqr_digits(published.as_bytes(), Language::English)
            .expect("the digits read");
        let from_bytes = faraday_keys::words::from_compact_seedqr(entropy, Language::English)
            .expect("the bytes read");
        for read in [&from_digits, &from_bytes] {
            let seed = read.to_seed(&c, pass.as_bytes()).unwrap();
            prop_assert_eq!(&wallet(&c, &MasterKey::from_seed(&c, &seed, Network::Mainnet)), &original);
        }
        // The codes the device draws are given for the same words.
        let keys = keys_of_words(&c, &words, &pass);
        for compact in [false, true] {
            let (_, count) = keys
                .seedqr_written(&c, 0, compact, Reveal::PaperForm)
                .expect("a code");
            prop_assert_eq!(count, m.word_count());
        }
    }

    /// Seed XOR in 2 to 4 parts: the parts, read by the `bip39` crate and
    /// XORed, are the wallet; with a part short they are another wallet.
    #[test]
    fn seed_xor_parts_restore_the_wallet_and_a_part_short_does_not(
        entropy in entropy(),
        pass in passphrase(),
        parts in 2u8..=4,
        random in pool(96),
    ) {
        let c = compute();
        let words = sentence(&entropy);
        let original = words_wallet(&c, &words, &pass);
        let keys = keys_of_words(&c, &words, &pass);
        let random = &random[..seed_xor_random_len(parts)];
        let lines = keys
            .seed_xor_converted(&c, 0, parts, random, Reveal::PaperForm)
            .expect("parts")
            .release();
        prop_assert_eq!(lines.len(), usize::from(parts));
        let read: Vec<bip39::Mnemonic> = lines
            .iter()
            .map(|l| bip39::Mnemonic::parse_in_normalized(bip39::Language::English, l.as_str()).unwrap())
            .collect();
        let xor = |chosen: &[usize]| {
            let mut acc = vec![0u8; entropy.len()];
            for &i in chosen {
                let (array, len) = read[i].to_entropy_array();
                for (a, b) in acc.iter_mut().zip(&array[..len]) {
                    *a ^= b;
                }
            }
            let m = bip39::Mnemonic::from_entropy_in(bip39::Language::English, &acc).unwrap();
            outside_wallet(&c, &m, &pass)
        };
        let all: Vec<usize> = (0..lines.len()).collect();
        prop_assert_eq!(&xor(&all), &original);
        for mask in 1u32..(1 << lines.len()) - 1 {
            let chosen: Vec<usize> = all.iter().copied().filter(|i| mask >> i & 1 == 1).collect();
            prop_assert_ne!(&xor(&chosen), &original, "{:?}", chosen);
        }
    }
}

/// The shapes of a SLIP-39 set: one group of `t` of `n`, or `g` groups of
/// which `gt` are needed.
fn shapes() -> impl Strategy<Value = (u8, Vec<GroupSpec>)> {
    let member = prop_oneof![Just((1u8, 1u8)), Just((2, 2)), Just((2, 3)), Just((3, 3)),];
    let one = (1u8..=6)
        .prop_flat_map(|n| (1u8..=n, Just(n)))
        .prop_filter("a threshold of 1 is a set of 1", |&(t, n)| t > 1 || n == 1);
    let single = one.prop_map(|(t, n)| {
        (
            1u8,
            vec![GroupSpec {
                threshold: t,
                count: n,
            }],
        )
    });
    let groups = (2usize..=3)
        .prop_flat_map(move |g| (2u8..=g as u8, prop::collection::vec(member.clone(), g)))
        .prop_map(|(gt, specs)| {
            (
                gt,
                specs
                    .into_iter()
                    .map(|(threshold, count)| GroupSpec { threshold, count })
                    .collect(),
            )
        });
    prop_oneof![single, groups]
}

/// Whether the members of `chosen` (in group, then member order) meet the
/// quorum of at least `group_threshold` groups.
fn meets(chosen: &[usize], group_threshold: u8, groups: &[GroupSpec]) -> bool {
    let mut start = 0;
    let mut met = 0;
    for g in groups {
        let end = start + usize::from(g.count);
        if chosen
            .iter()
            .filter(|&&i| (start..end).contains(&i))
            .count()
            >= usize::from(g.threshold)
        {
            met += 1;
        }
        start = end;
    }
    met >= group_threshold
}

/// Whether `chosen` is exactly a quorum: `group_threshold` groups, each with
/// exactly its threshold of members, and no share of any other.
fn exactly(chosen: &[usize], group_threshold: u8, groups: &[GroupSpec]) -> bool {
    let mut start = 0;
    let mut used = 0;
    for g in groups {
        let end = start + usize::from(g.count);
        match chosen
            .iter()
            .filter(|&&i| (start..end).contains(&i))
            .count()
        {
            0 => {}
            n if n == usize::from(g.threshold) => used += 1,
            _ => return false,
        }
        start = end;
    }
    used == group_threshold
}

fn subset(shares: &[Share], mask: u32) -> (Vec<usize>, Vec<Share>) {
    let chosen: Vec<usize> = (0..shares.len()).filter(|i| mask >> i & 1 == 1).collect();
    let own = chosen.iter().map(|&i| shares[i].duplicate()).collect();
    (chosen, own)
}

proptest! {
    #![proptest_config(config(24))]

    /// A SLIP-39 set, one group or several, with a passphrase: every
    /// quorum (a sample of them, the sets being dear to open) gives the
    /// seed's wallet and every set short of one is refused.
    #[test]
    fn slip39_quorums_restore_the_wallet_and_the_rest_are_refused(
        long in any::<bool>(),
        seed in pool(32),
        pass in "[ -~]{0,8}",
        identifier in 0u16..1 << 15,
        (group_threshold, groups) in shapes(),
        random in pool(320),
    ) {
        let c = compute();
        let seed = &seed[..if long { 32 } else { 16 }];
        let total: usize = groups.iter().map(|g| usize::from(g.count)).sum();
        let mut out: Vec<Share> = (0..total).map(|_| Share::default()).collect();
        let mut at = 0;
        let mut draw = |buf: &mut [u8]| {
            for b in buf {
                *b = random[at % random.len()];
                at += 1;
            }
        };
        let n = slip39::split(
            &c, seed, pass.as_bytes(), identifier, true, 1, group_threshold, &groups, &mut draw, &mut out,
        )
        .expect("a split");
        prop_assert_eq!(n, total);
        // The text a person writes reads back as the same shares.
        let shares: Vec<Share> = out
            .iter()
            .map(|s| {
                let text: Vec<&str> = s.indices().iter().map(|&i| slip39::word(i)).collect();
                Share::parse(&text.join(" ")).expect("a written share parses")
            })
            .collect();
        let mut quorums = 0usize;
        let mut sampled = 0usize;
        for mask in 1u32..1 << total {
            let (chosen, own) = subset(&shares, mask);
            if exactly(&chosen, group_threshold, &groups) {
                quorums += 1;
                // Every third quorum, at most five: the key stretching is dear.
                if quorums % 3 == 1 && sampled < 5 {
                    sampled += 1;
                    let secret = slip39_seed(&c, &own, pass.as_bytes()).expect("a quorum restores");
                    prop_assert_eq!(seed_wallet(&c, &secret), seed_wallet(&c, seed), "{:?}", chosen);
                }
            } else if !meets(&chosen, group_threshold, &groups) {
                prop_assert!(
                    slip39::recover(&c, &own, pass.as_bytes()).is_err(),
                    "{:?} of {} of {:?} gave a key", chosen, group_threshold, groups
                );
            }
        }
        prop_assert!(quorums > 0);
    }

    /// The sets Convert a seed writes, with no passphrase, restore the
    /// key's wallet from the first and from the last quorum.
    #[test]
    fn the_slip39_sets_convert_a_seed_writes_restore_the_wallet(
        long in any::<bool>(),
        seed in pool(32),
        (group_threshold, groups) in shapes(),
        random in pool(320),
    ) {
        let c = compute();
        let seed = &seed[..if long { 32 } else { 16 }];
        let keys = keys_of_seed(&c, seed);
        let need = slip39_random_len(seed.len(), group_threshold, &groups);
        let texts = keys
            .slip39_converted(&c, 0, group_threshold, &groups, &random[..need], Reveal::PaperForm)
            .expect("a set")
            .release();
        let shares: Vec<Share> = texts.iter().map(|t| Share::parse(t.as_str()).unwrap()).collect();
        let total = shares.len();
        let quorum = (1u32..1 << total)
            .map(|m| subset(&shares, m))
            .find(|(chosen, _)| exactly(chosen, group_threshold, &groups))
            .expect("a quorum");
        let secret = slip39_seed(&c, &quorum.1, b"").expect("restored");
        prop_assert_eq!(seed_wallet(&c, &secret), seed_wallet(&c, seed));
        for mask in 1u32..1 << total {
            let (chosen, own) = subset(&shares, mask);
            if !meets(&chosen, group_threshold, &groups) {
                prop_assert!(slip39_seed(&c, &own, b"").is_err(), "{:?}", chosen);
            }
        }
    }
}

proptest! {
    #![proptest_config(config(40))]

    /// A codex32 secret, whole (a seed not split) or as the shares of a `k`
    /// of `n` set, restores the seed's wallet from every `k` of the shares and
    /// refuses every fewer.
    #[test]
    fn codex32_secrets_and_shares_restore_the_wallet(
        length in prop::sample::select(vec![16usize, 24, 32, 64]),
        seed in pool(64),
        shared in any::<bool>(),
        (k, extra) in (2u8..=5, 0u8..=3),
        random in pool(512),
    ) {
        let c = compute();
        let seed = &seed[..length];
        let keys = keys_of_seed(&c, seed);
        let (k, n) = if shared { (k, k + extra) } else { (0, 1) };
        let need = codex32_random_len(k);
        let texts = keys
            .codex32_converted(&c, 0, k, n, (&random[..need], &[]), Reveal::PaperForm)
            .expect("a codex32 set")
            .release();
        prop_assert_eq!(texts.len(), usize::from(n));
        let parts: Vec<Codex32> = texts.iter().map(|t| Codex32::parse(t.as_str()).unwrap()).collect();
        let want = seed_wallet(&c, seed);
        let take = usize::from(k.max(1));
        for mask in 1u32..1 << parts.len() {
            let chosen: Vec<Codex32> = (0..parts.len())
                .filter(|i| mask >> i & 1 == 1)
                .map(|i| Codex32::parse(texts[i].as_str()).unwrap())
                .collect();
            if chosen.len() == take {
                let restored = codex32_seed(&chosen).expect("a quorum restores");
                prop_assert_eq!(&seed_wallet(&c, &restored), &want);
            } else if chosen.len() < take {
                prop_assert!(codex32_seed(&chosen).is_err(), "{} of {}", chosen.len(), k);
            }
        }
    }
}

/// The names the app writes, as the strings hold them.
fn names() -> Names {
    let s = |t: &'static str| t;
    Names {
        group_keys: s("Seeds"),
        group_wallets: s("Wallets"),
        group_notes: s("Notes"),
        group_sheets: s("Sheets"),
        group_gpg: s("GPG keys"),
        group_secure_boot: s("Secure Boot"),
        group_wallet_shares: s("Wallet shares"),
        group_passphrases: s("Passphrases"),
        group_shares: s("Codex32 shares"),
        title_key: s("Key {fingerprint}"),
        title_note: s("Note"),
        title_passphrase: s("Passphrase for {for}"),
        title_share: s("Codex32 share for {for}"),
        title_secure_boot: s("Secure Boot keys"),
    }
}

/// The cheapest cost a vault is made at: 8 MiB, one pass, one lane.
const COST: Cost = Cost {
    memory_kib: 8192,
    passes: 1,
    lanes: 1,
};

/// A vault passphrase: any printable text, non-ASCII and long ones among
/// them.
fn vault_passphrase() -> impl Strategy<Value = String> {
    prop_oneof!["[ -~]{1,24}", "\\PC{1,24}", "\\PC{150,200}"]
}

/// The same text with one more character: another passphrase.
fn other(pass: &str) -> String {
    format!("{pass}x")
}

proptest! {
    #![proptest_config(config(6))]

    /// A key sealed in a vault file under a passphrase opens, with that
    /// passphrase and no other, as the same wallet.
    #[test]
    fn a_key_in_a_vault_file_opens_as_the_same_wallet(
        entropy in entropy(),
        pass in vault_passphrase(),
        salt in any::<[u8; 16]>(),
    ) {
        let c = compute();
        let words = sentence(&entropy);
        let original = words_wallet(&c, &words, "");
        let m = Mnemonic::parse(&c, Language::English, &words).unwrap();
        let file = vault::create(&c, salt, "Test", COST, &pass, &[7u8; SEAL_RANDOM_LEN], None)
            .expect("a vault");
        let mut store = OpenVaults::default();
        store.set_names(names());
        let (id, _) = store.unlock(&c, file, &pass).expect("opens");
        store.insert(
            id,
            Record::new(kind::KEY)
                .with(field::KEY, &words_payload(&m))
                .with(field::KEY_LABEL, b"Test"),
        );
        let n = store.uuid_bytes(id).expect("held");
        let uuids: Vec<u8> = (0..n).map(|i| 0x10 + (i / 16) as u8).collect();
        let sealed = store
            .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, Some(1_780_000_000))
            .expect("a job")
            .run(&c)
            .into_file()
            .expect("sealed");
        store.seal_ended(id, true);
        prop_assert!(store.unlock(&c, sealed.clone(), &other(&pass)).is_err());
        let (again, _) = store.unlock(&c, sealed, &pass).expect("the sealed vault opens");
        let mut keys = Keys::on(Network::Mainnet);
        let all = store.records(again).of(kind::KEY).map(|(i, _)| i).collect();
        let loaded = store.load_from_vault(&c, &mut keys, again, &all, &Default::default());
        prop_assert_eq!(loaded.len(), 1);
        prop_assert_eq!(loaded[0].0, original.fingerprint);
        let written = keys
            .words_written(&c, 0, WordsForm::Words, Reveal::PaperForm)
            .expect("written")
            .release();
        prop_assert_eq!(words_wallet(&c, written.as_str(), ""), original);
    }
}

proptest! {
    #![proptest_config(config(8))]

    /// A key of 12 or 24 words sealed as a QR vault opens, with its
    /// passphrase and no other, as the same wallet.
    #[test]
    fn a_key_in_a_qr_vault_opens_as_the_same_wallet(
        long in any::<bool>(),
        bytes in pool(32),
        pass in vault_passphrase(),
        random in pool(QR_RANDOM_LEN),
    ) {
        let c = compute();
        let entropy = &bytes[..if long { 32 } else { 16 }];
        let words = sentence(entropy);
        let original = words_wallet(&c, &words, "");
        let keys = keys_of_words(&c, &words, "");
        let random: [u8; QR_RANDOM_LEN] = random.try_into().unwrap();
        let code = keys
            .qr_seal_key_job(&c, 0, &pass, 16, zeroize::Zeroizing::new(random))
            .and_then(|j| j.run(&c))
            .expect("sealed");
        prop_assert!(QrOpenJob::new(&code, &other(&pass)).run(&c).is_err());
        let mut reader = Keys::on(Network::Mainnet);
        let opened = QrOpenJob::new(&code, &pass).run(&c).expect("opens");
        let holds = reader.qr_hold(opened).expect("held");
        prop_assert_eq!(holds, QrHolds::Seed { words: if long { 24 } else { 12 } });
        let read = reader.qr_words(Reveal::QrVault).to_string();
        prop_assert_eq!(words_wallet(&c, &read, ""), original);
    }
}
