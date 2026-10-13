//! What a phone holding the Companion lets in (`docs/COMPANION.md` §5):
//! every kind of secret is refused, from published vectors where there
//! are some and from Faraday's own writers, and nothing a wallet or a
//! coordinator hands over is.

use faraday_intake::{Secret, secret};
use osk_bip::bitcoin::base58;

/// A file under the repository's root.
fn repo(path: &str) -> Vec<u8> {
    let at = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&at).unwrap_or_else(|e| panic!("{at}: {e}"))
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_slice(&repo(path)).expect(path)
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn refused(bytes: &[u8], kind: Secret) {
    assert_eq!(
        secret(bytes),
        Some(kind),
        "{}",
        String::from_utf8_lossy(&bytes[..bytes.len().min(120)])
    );
}

fn let_in(bytes: &[u8]) {
    assert_eq!(
        secret(bytes),
        None,
        "{}",
        String::from_utf8_lossy(&bytes[..bytes.len().min(120)])
    );
}

/// BIP-39's published vectors in each of its ten lists. The vectors file
/// also carries Russian and Turkish, which are not BIP-39 lists.
fn bip39_mnemonics() -> Vec<String> {
    let mut out = Vec::new();
    let v = json("tools/vectors/bip39/vectors.json");
    for (language, list) in v.as_object().expect("languages") {
        if matches!(language.as_str(), "russian" | "turkish") {
            continue;
        }
        for entry in list.as_array().expect("vectors") {
            out.push(entry[1].as_str().expect("mnemonic").to_string());
        }
    }
    for entry in json("tools/vectors/bip39/test_JP_BIP39.json")
        .as_array()
        .expect("vectors")
    {
        out.push(entry["mnemonic"].as_str().expect("mnemonic").to_string());
    }
    out
}

#[test]
fn seed_words_in_every_language_are_refused() {
    let all = bip39_mnemonics();
    assert!(all.len() > 200);
    for m in &all {
        refused(m.as_bytes(), Secret::Bip39Words);
    }
}

#[test]
fn a_file_of_seed_words_is_refused() {
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let mut file = String::from("# My seed, written down\n");
    for (i, w) in words.split(' ').enumerate() {
        file.push_str(&format!("{}. {w}\n", i + 1));
    }
    refused(file.as_bytes(), Secret::Bip39Words);
    refused(
        format!("{}\n", words.to_uppercase()).as_bytes(),
        Secret::Bip39Words,
    );
}

/// SeedSigner's SeedQR specification ("Test SeedQRs"): a 24-word and a
/// 12-word seed as digits and as CompactSeedQR bytes.
const SEEDQR: [(&str, &[u8]); 2] = [
    (
        "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643",
        b"\x0et\xb6A\x07\xf9L\xc0\xcc\xfa\xe6\xa1=\xcb\xec6b\x15O\xecg\xe0\xe0\t\x99\xc0x\x92Y}\x19\n",
    ),
    (
        "073318950739065415961602009907670428187212261116",
        b"[\xbd\x9dq\xa8\xecy\x90\x83\x1a\xff5\x9dBeE",
    ),
];

#[test]
fn seedqr_and_compactseedqr_are_refused() {
    for (digits, compact) in SEEDQR {
        refused(digits.as_bytes(), Secret::SeedQr);
        refused(compact, Secret::CompactSeedQr);
    }
}

#[test]
fn codex32_strings_are_refused() {
    let v = json("tools/vectors/bip93/vectors.json");
    let mut n = 0;
    for s in v["secrets"].as_array().expect("secrets") {
        refused(
            s["string"].as_str().expect("string").as_bytes(),
            Secret::Codex32,
        );
        n += 1;
    }
    for set in v["sets"].as_array().expect("sets") {
        for share in set["shares"].as_array().expect("shares") {
            refused(share.as_str().expect("share").as_bytes(), Secret::Codex32);
            n += 1;
        }
    }
    assert!(n > 4);
}

#[test]
fn slip39_shares_are_refused() {
    let v = json("tools/vectors/slip39/vectors.json");
    let mut n = 0;
    for case in v.as_array().expect("vectors") {
        if case[2].as_str() == Some("") {
            continue;
        }
        let shares = case[1].as_array().expect("shares");
        for share in shares {
            refused(
                share.as_str().expect("share").as_bytes(),
                Secret::Slip39Share,
            );
            n += 1;
        }
        // A file holding all of them, one to a line.
        let file: Vec<&str> = shares.iter().filter_map(|s| s.as_str()).collect();
        refused(file.join("\n").as_bytes(), Secret::Slip39Share);
    }
    assert!(n > 10);
}

/// The master key of BIP-39's first English vector, as it publishes it.
fn master_xprv() -> String {
    json("tools/vectors/bip39/vectors.json")["english"][0][3]
        .as_str()
        .expect("xprv")
        .to_string()
}

/// That master key, read.
fn master() -> osk_bip::bitcoin::bip32::Xpriv {
    use std::str::FromStr;
    osk_bip::bitcoin::bip32::Xpriv::from_str(&master_xprv()).expect("BIP-39's vector")
}

/// The private versions SLIP-132 registers: xprv, yprv, zprv, Yprv,
/// Zprv and their testnet forms.
const PRIVATE_VERSIONS: [[u8; 4]; 10] = [
    [0x04, 0x88, 0xAD, 0xE4],
    [0x04, 0x9D, 0x78, 0x78],
    [0x04, 0xB2, 0x43, 0x0C],
    [0x02, 0x95, 0xB0, 0x05],
    [0x02, 0xAA, 0x7A, 0x99],
    [0x04, 0x35, 0x83, 0x94],
    [0x04, 0x4A, 0x4E, 0x28],
    [0x04, 0x5F, 0x18, 0xBC],
    [0x02, 0x42, 0x85, 0xB5],
    [0x02, 0x57, 0x50, 0x48],
];

#[test]
fn extended_private_keys_are_refused_in_every_form() {
    master();
    let data = base58::decode_check(&master_xprv()).expect("base58check");
    for version in PRIVATE_VERSIONS {
        let mut d = data.clone();
        d[..4].copy_from_slice(&version);
        let key = base58::encode_check(&d);
        refused(key.as_bytes(), Secret::ExtendedPrivateKey);
        refused(
            format!("[d34db33f/84h/0h/0h]{key}").as_bytes(),
            Secret::ExtendedPrivateKey,
        );
        refused(
            format!("wpkh([d34db33f/84'/0'/0']{key}/0/*)\n").as_bytes(),
            Secret::ExtendedPrivateKey,
        );
    }
    // Every xprv the published seed vectors derive.
    for s in json("tools/vectors/bip93/vectors.json")["secrets"]
        .as_array()
        .expect("secrets")
    {
        if let Some(x) = s["xprv"].as_str() {
            refused(x.as_bytes(), Secret::ExtendedPrivateKey);
        }
    }
    for case in json("tools/vectors/slip39/vectors.json")
        .as_array()
        .expect("vectors")
    {
        if let Some(x) = case[3].as_str().filter(|x| !x.is_empty()) {
            refused(x.as_bytes(), Secret::ExtendedPrivateKey);
        }
    }
}

#[test]
fn wif_private_keys_from_a_published_key_are_refused() {
    use osk_bip::bitcoin::{NetworkKind, PrivateKey};
    let key = master().private_key.secret_bytes();
    for network in [NetworkKind::Main, NetworkKind::Test] {
        let mut k = PrivateKey::from_slice(&key, network).expect("the vector's key");
        refused(k.to_wif().as_bytes(), Secret::Wif);
        k.compressed = false;
        refused(k.to_wif().as_bytes(), Secret::Wif);
    }
    let wif = PrivateKey::from_slice(&key, NetworkKind::Main)
        .expect("the vector's key")
        .to_wif();
    refused(format!("wpkh({wif})").as_bytes(), Secret::Wif);
}

/// CBOR's head for `major` and `n`, and a text string.
fn cbor_text(s: &str) -> Vec<u8> {
    let mut out = faraday_qr::cbor::head(3, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
    out
}

/// A map from small unsigned keys to already-encoded values.
fn cbor_map(pairs: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut out = faraday_qr::cbor::head(5, pairs.len() as u64);
    for (k, v) in pairs {
        out.extend(faraday_qr::cbor::head(0, u64::from(*k)));
        out.extend_from_slice(v);
    }
    out
}

const TRUE: u8 = 0xF5;

#[test]
fn ur_seeds_and_private_hdkeys_are_refused() {
    // BCR-2020-006's crypto-seed: a payload and a creation date.
    let mut date = faraday_qr::cbor::head(6, 100);
    date.extend(faraday_qr::cbor::head(0, 18394));
    let seed = cbor_map(&[
        (
            1,
            faraday_qr::cbor::bytes(&unhex("c7098580125e2ab0981253468b2dbc52")),
        ),
        (2, date),
    ]);
    for name in ["crypto-seed", "seed"] {
        let ur = faraday_qr::ur::single(name, &seed);
        refused(ur.as_bytes(), Secret::UrSeed);
        refused(ur.to_uppercase().as_bytes(), Secret::UrSeed);
    }
    // BCR-2020-006's crypto-bip39: the words and their language.
    let words = "shield group erode awake lock sausage cash glare wave crew flame glove";
    let mut list = faraday_qr::cbor::head(4, 12);
    for w in words.split(' ') {
        list.extend(cbor_text(w));
    }
    let bip39 = cbor_map(&[(1, list), (2, cbor_text("en"))]);
    for name in ["crypto-bip39", "bip39"] {
        refused(
            faraday_qr::ur::single(name, &bip39).as_bytes(),
            Secret::UrBip39,
        );
    }
    // A master key as BCR-2020-007's first example states one (is-master,
    // key data, chain code): BIP-39's first vector's.
    let m = master();
    let mut key = vec![0u8];
    key.extend(m.private_key.secret_bytes());
    let chain = faraday_qr::cbor::bytes(m.chain_code.as_bytes());
    let master = cbor_map(&[
        (1, vec![TRUE]),
        (3, faraday_qr::cbor::bytes(&key)),
        (4, chain.clone()),
    ]);
    let private = cbor_map(&[
        (2, vec![TRUE]),
        (3, faraday_qr::cbor::bytes(&key)),
        (4, chain.clone()),
    ]);
    for name in ["crypto-hdkey", "hdkey"] {
        for cbor in [&master, &private] {
            refused(
                faraday_qr::ur::single(name, cbor).as_bytes(),
                Secret::UrPrivateKey,
            );
        }
    }
    // The same key's public half is an account key, and comes in.
    let public = cbor_map(&[
        (
            3,
            faraday_qr::cbor::bytes(
                &m.private_key
                    .public_key(&osk_bip::bitcoin::secp256k1::Secp256k1::new())
                    .serialize(),
            ),
        ),
        (4, chain),
    ]);
    let_in(faraday_qr::ur::single("crypto-hdkey", &public).as_bytes());
}

#[test]
fn faradays_own_secret_files_are_refused() {
    refused(
        &repo("tools/vectors/backup/backup-v3-words-12-english.oskb"),
        Secret::EncryptedBackup,
    );
    refused(
        &repo("tools/vectors/backup/backup-kdbx-words-12-english.kdbx"),
        Secret::KeePass,
    );
    refused(&repo("tools/vectors/vault/vault-one.ofv"), Secret::Vault);
    refused(
        &repo("tools/vectors/psbt/wallet-threshold-carry.osk"),
        Secret::ThresholdCarry,
    );
}

#[test]
fn authenticator_secrets_and_vault_entries_are_refused() {
    refused(
        b"otpauth://totp/Example%20Git:octo?secret=JBSWY3DPEHPK3PXP&issuer=Example%20Git\n",
        Secret::Totp,
    );
    refused(
        b"otpauth-migration://offline?data=CjEKCkhlbGxvId6tvu8SGEV4YW1wbGU6YWxpY2VAZ29vZ2xlLmNvbRoHRXhhbXBsZTAC",
        Secret::TotpMigration,
    );
    refused(
        b"title: Bank\nusername: alice\npassword: correct horse battery staple\n",
        Secret::VaultEntries,
    );
}

#[test]
fn a_secret_inside_a_file_envelope_is_refused() {
    let words = b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n";
    let env = faraday_qr::envelope::pack("seed.txt", words, "file").expect("envelope");
    refused(&env, Secret::Bip39Words);
}

/// BIP-174's valid PSBTs, binary and base64, and the finished
/// transaction its first case holds.
fn bip174() -> (Vec<Vec<u8>>, String) {
    let text = String::from_utf8(repo("tools/vectors/psbt/bip174.txt")).expect("text");
    let mut psbts = Vec::new();
    let mut transaction = String::new();
    for record in text.split("\n\n") {
        let field = |k: &str| {
            record
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{k}: ")))
                .map(str::to_string)
        };
        if field("case").is_some_and(|c| c.starts_with("Network transaction")) {
            transaction = field("hex").expect("hex");
        }
        if field("expect").as_deref() == Some("valid") {
            psbts.push(unhex(&field("hex").expect("hex")));
            psbts.push(field("base64").expect("base64").into_bytes());
        }
    }
    assert!(psbts.len() > 4 && !transaction.is_empty());
    (psbts, transaction)
}

#[test]
fn nothing_a_wallet_hands_over_is_refused() {
    let (psbts, transaction) = bip174();
    for p in &psbts {
        let_in(p);
    }
    let_in(transaction.as_bytes());
    let_in(&repo("tools/vectors/wallet/account-zpub.txt"));
    let_in(&repo(
        "faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-receive.txt",
    ));
    let_in(&repo(
        "faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-1.txt",
    ));
    // BIP-84's first address, and a legacy one.
    let_in(b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8zmfp6l0");
    let_in(b"1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2");
    let env = faraday_qr::envelope::pack("spend.psbt", &psbts[0], "file").expect("envelope");
    let_in(&env);
    let_in(b"Meet at the bridge at noon, and bring the printed sheet.");
}
