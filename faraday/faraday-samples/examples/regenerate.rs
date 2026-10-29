//! Writes the samples crate's data: `src/data.rs` and the files in `data/`.
//!
//! Run from the repository root, then format:
//!
//! ```text
//! cargo run -p faraday-samples --example regenerate
//! cargo fmt -p faraday-samples
//! ```
//!
//! It derives every fingerprint, wallet, spend, file and signed message
//! from the vectors in `tools/vectors` and from the testkit, once. The
//! crate's tests then check what it wrote against those same sources
//! (and against other implementations), so a change to the vectors or
//! the testkit shows up as a failing test until this is run again.

use std::fmt::Write as _;
use std::path::PathBuf;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Network;
use faraday_core::testkit::{self, compute};
use faraday_crypto::work::Compute;
use faraday_intake::{Part, part_of};
use faraday_keys::identity::{Material, fingerprint, fingerprint_of_words};

/// The one passphrase of every sample that takes one.
const PASSPHRASE: &str = faraday_core::samples::PASSPHRASE;

struct Key {
    name: &'static str,
    word: &'static str,
    count: u8,
    form: &'static str,
    size: String,
    listed: bool,
    parts: Vec<String>,
    combined: String,
    plain: Option<[u8; 4]>,
    with_passphrase: Option<[u8; 4]>,
    node: Option<[u8; 33]>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vector(path: &str) -> String {
    std::fs::read_to_string(root().join("../../tools/vectors").join(path)).expect("a vector file")
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&vector(path)).expect("json")
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn words_key(c: &Compute, name: &'static str, word: &'static str, count: u8) -> Key {
    let phrase = faraday_core::samples::repeated(word, count);
    Key {
        name,
        word,
        count,
        form: "Words",
        size: format!("Words({count})"),
        listed: true,
        plain: Some(fingerprint_of_words(c, &phrase, "").expect("words").0),
        with_passphrase: Some(
            fingerprint_of_words(c, &phrase, PASSPHRASE)
                .expect("words")
                .0,
        ),
        parts: vec![phrase],
        combined: String::new(),
        node: None,
    }
}

fn plain_words(
    c: &Compute,
    name: &'static str,
    phrase: &str,
    form: &'static str,
    size: String,
) -> Key {
    Key {
        name,
        word: "",
        count: 0,
        form,
        size,
        listed: true,
        plain: Some(
            fingerprint_of_words(c, phrase, "")
                .unwrap_or_else(|e| panic!("{name}: {e}"))
                .0,
        ),
        with_passphrase: Some(
            fingerprint_of_words(c, phrase, PASSPHRASE)
                .unwrap_or_else(|e| panic!("{name}: {e}"))
                .0,
        ),
        parts: vec![phrase.to_string()],
        combined: String::new(),
        node: None,
    }
}

fn seed_fp(c: &Compute, seed: &[u8]) -> [u8; 4] {
    fingerprint(c, Material::Seed(seed), "", Network::Mainnet)
        .expect("a master seed")
        .0
}

fn slip39_key(c: &Compute, name: &'static str, shares: &[String], size: String) -> Key {
    let parsed: Vec<_> = shares
        .iter()
        .map(|s| match part_of(s) {
            Some(Part::Slip39(s)) => s,
            _ => panic!("a SLIP-39 share"),
        })
        .collect();
    let fp = |pass: &str| {
        fingerprint(c, Material::Slip39(&parsed), pass, Network::Mainnet)
            .expect("shares recover")
            .0
    };
    Key {
        name,
        word: "",
        count: 0,
        form: "Slip39",
        size,
        listed: true,
        parts: shares.to_vec(),
        combined: String::new(),
        plain: Some(fp("")),
        with_passphrase: Some(fp(PASSPHRASE)),
        node: None,
    }
}

fn codex32_key(c: &Compute, name: &'static str, strings: &[String], size: String) -> Key {
    let parsed: Vec<_> = strings
        .iter()
        .map(|s| match part_of(s) {
            Some(Part::Codex32(s)) => s,
            _ => panic!("a codex32 string"),
        })
        .collect();
    let seed = faraday_keys::codex32_seed(&parsed).expect("a codex32 seed");
    Key {
        name,
        word: "",
        count: 0,
        form: "Codex32",
        size,
        listed: true,
        parts: strings.to_vec(),
        combined: String::new(),
        plain: Some(seed_fp(c, &seed)),
        with_passphrase: None,
        node: None,
    }
}

/// The key the first test vector of BIP-39 gives, in words and under
/// both passphrases.
fn bip39_vectors(c: &Compute) -> (Key, Key) {
    let v = json("bip39/vectors.json");
    let abandon = v["english"][0][1].as_str().expect("words");
    let jp = json("bip39/test_JP_BIP39.json");
    // Faraday reads a list's published form or its NFKD form, word by word;
    // the vector's own composition is neither, so the sample is its NFKD.
    let japanese: String = {
        use unicode_normalization::UnicodeNormalization;
        jp[0]["mnemonic"].as_str().expect("words").nfkd().collect()
    };
    (
        plain_words(
            c,
            "sample_abandon",
            abandon,
            "Words",
            "Words(12)".to_string(),
        ),
        plain_words(
            c,
            "sample_japanese",
            &japanese,
            "Words",
            "JapaneseWords(12)".to_string(),
        ),
    )
}

fn xor_key(c: &Compute) -> Key {
    let text = vector("xor/12-words-2-parts.txt");
    let line = |tag: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(tag))
            .expect("a line")
            .trim()
            .to_string()
    };
    let mut k = plain_words(
        c,
        "sample_xor",
        &line("seed words: "),
        "Xor",
        "Parts(2)".to_string(),
    );
    k.parts = vec![line("part 1 words: "), line("part 2 words: ")];
    k.combined = line("seed words: ");
    k
}

fn slip_shares(case: usize) -> Vec<String> {
    json("slip39/vectors.json")[case][1]
        .as_array()
        .expect("mnemonics")
        .iter()
        .map(|s| s.as_str().expect("text").to_string())
        .collect()
}

fn aezeed_key() -> Key {
    let words = "above judge emerge veteran reform crunch system all snap please shoulder \
                 vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    let node = unhex("0226594d21c0862a11168ab07cdbc15e7c7af5ee561b741259e311f1614f4df3b7");
    Key {
        name: "sample_aezeed",
        word: "",
        count: 0,
        form: "Aezeed",
        size: "Words(24)".to_string(),
        listed: true,
        parts: vec![words.to_string()],
        combined: String::new(),
        plain: None,
        with_passphrase: None,
        node: Some(node.try_into().expect("33 bytes")),
    }
}

/// The key the user guide makes (`docs/guide/walk/new-key.md`): the
/// words EntropyLab's published BitBox dice transcript names, as
/// `entropy/dice.json`'s first words vector gives their entropy.
fn guide_key(c: &Compute) -> Key {
    let v = json("entropy/dice.json");
    let entropy = v["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .find(|x| x["procedure"] == "words")
        .and_then(|x| x["entropy"].as_str())
        .expect("a words vector");
    let m = Mnemonic::from_entropy(Language::English, &unhex(entropy)).expect("words");
    let phrase: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    plain_words(
        c,
        "sample_guide",
        &phrase.join(" "),
        "Words",
        "Words(12)".to_string(),
    )
}

fn keys(c: &Compute) -> Vec<Key> {
    // The guide's key first: the sheet lists it at the top.
    let mut all = vec![guide_key(c)];
    all.extend(
        testkit::SAMPLE_WORDS
            .iter()
            .map(|&(w, n)| words_key(c, "sample_words", w, n)),
    );
    let (abandon, japanese) = bip39_vectors(c);
    all.push(abandon);
    all.push(japanese);
    all.push(slip39_key(
        c,
        "sample_slip39_one",
        &slip_shares(0),
        "Shares(1)".to_string(),
    ));
    all.push(slip39_key(
        c,
        "sample_slip39_two_of_three",
        &slip_shares(3),
        "Threshold { need: 2, total: 3 }".to_string(),
    ));
    all.push(slip39_key(
        c,
        "sample_slip39_groups",
        &slip_shares(36),
        format!("GroupShares({})", slip_shares(36).len()),
    ));
    let codex = json("bip93/vectors.json");
    let secret = codex["secrets"][0]["string"].as_str().expect("string");
    all.push(codex32_key(
        c,
        "sample_codex32_secret",
        &[secret.to_string()],
        "Secret".to_string(),
    ));
    let shares: Vec<String> = codex["sets"][0]["shares"]
        .as_array()
        .expect("shares")
        .iter()
        .map(|s| s.as_str().expect("text").to_string())
        .collect();
    let count = shares.len();
    all.push(codex32_key(
        c,
        "sample_codex32_shares",
        &shares,
        format!("Shares({count})"),
    ));
    all.push(xor_key(c));
    all.push(aezeed_key());
    for b in 1u8..=5 {
        all.push(Key {
            name: "sample_three_of_five",
            word: "",
            count: 0,
            form: "Seed",
            size: "Bytes(64)".to_string(),
            listed: false,
            parts: vec![hex(&[b; 64])],
            combined: String::new(),
            plain: Some(seed_fp(c, &[b; 64])),
            with_passphrase: None,
            node: None,
        });
    }
    all
}

struct Wallet {
    name: &'static str,
    kind: String,
    script: &'static str,
    text: String,
    spend: Option<String>,
    keys: Vec<[u8; 4]>,
}

/// What a wallet is and the script it pays to, as the policy reading it
/// says: `(kind, script)` as the text of `SampleKind` and `SampleScript`.
fn describe(c: &Compute, text: &str) -> (String, &'static str) {
    use faraday_bip::keys::ScriptType;
    use faraday_bip::policy::Template;
    let p = faraday_core::wallet::read_wallet(c, text).expect("a wallet");
    let tree = matches!(p.template(), Template::Tree) && p.tapscript_quorum().is_none();
    let script = match (tree, p.script_type()) {
        (true, _) => "TaprootTree",
        (_, ScriptType::Legacy) => "Legacy",
        (_, ScriptType::NestedSegwit) => "NestedSegwit",
        (_, ScriptType::NativeSegwit) => "NativeSegwit",
        (_, ScriptType::Taproot) => "Taproot",
    };
    let multi =
        |(need, total): (usize, usize)| format!("Multisig {{ need: {need}, total: {total} }}");
    let kind = if let Some(q) = p.tapscript_quorum().or_else(|| p.quorum()) {
        multi(q)
    } else {
        match p.template() {
            Template::MuSig => format!("MuSig({})", p.keys().len()),
            Template::Miniscript { .. } => "Timelock".to_string(),
            _ => "Single".to_string(),
        }
    };
    (kind, script)
}

fn wallets(c: &Compute, keys: &[Key]) -> Vec<Wallet> {
    let used = |text: &str| -> Vec<[u8; 4]> {
        keys.iter()
            .filter_map(|k| k.plain)
            .filter(|fp| text.contains(&hex(fp)))
            .collect()
    };
    let mut out = Vec::new();
    let ids = [
        ("savings", "sample_savings"),
        ("spending", "sample_spending"),
        ("taproot", "sample_taproot"),
        ("nested", "sample_nested"),
        ("legacy", "sample_legacy"),
        ("vault-nested", "sample_nested_multisig"),
        ("vault-legacy", "sample_legacy_multisig"),
        ("taproot-multisig", "sample_taproot_multisig"),
        ("inheritance", "sample_inheritance"),
        ("tree", "sample_tree"),
    ];
    for kit in testkit::kits() {
        let (_, name) = ids.iter().find(|(id, _)| *id == kit.id).expect("a kit");
        let spend = testkit::unsigned(&kit).expect("a spend");
        std::fs::write(
            root().join(format!("data/{}.psbt", kit.id)),
            spend.to_bytes(),
        )
        .expect("write");
        let (kind, script) = describe(c, &kit.descriptor);
        out.push(Wallet {
            name,
            kind,
            script,
            keys: used(&kit.descriptor),
            text: kit.descriptor.clone(),
            spend: Some(format!("data/{}.psbt", kit.id)),
        });
    }
    let musig = testkit::musig_policy().expect("musig");
    std::fs::write(
        root().join("data/musig.psbt"),
        testkit::musig_unsigned().expect("a spend").to_bytes(),
    )
    .expect("write");
    let musig_text = musig.to_descriptor_checksummed();
    let (kind, script) = describe(c, &musig_text);
    out.push(Wallet {
        name: "sample_musig",
        kind,
        script,
        keys: used(&musig_text),
        text: musig_text,
        spend: Some("data/musig.psbt".to_string()),
    });
    let five = testkit::three_of_five();
    let (kind, script) = describe(c, &five);
    out.push(Wallet {
        name: "sample_three_of_five",
        kind,
        script,
        keys: used(&five),
        text: five,
        spend: None,
    });
    out
}

/// The vault: sample key 1 and the Savings wallet, under `faraday`.
fn vault_file(c: &Compute) -> Vec<u8> {
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let words = faraday_core::samples::repeated("bacon", 24);
    let m = Mnemonic::parse(c, Language::English, &words).expect("words");
    let contents = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Sample vault"),
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, b"Sample key 1"),
            Record::new(kind::WALLET)
                .with(field::WALLET, testkit::savings().as_bytes())
                .with(field::WALLET_NAME, b"Savings"),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PASSPHRASE.as_bytes()],
        &[contents],
        &[0x7a; 32],
    )
    .expect("a vault")
}

/// A message signed by sample Spending's first receive address.
fn signed_message(c: &Compute) -> (String, String) {
    use faraday_core::wallet::{Origin, Session};
    let mut s = Session::on(Network::Testnet);
    let words = faraday_core::samples::repeated("bacon", 24);
    s.keys.add_words(c, &words, "", None).expect("key");
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .expect("spending");
    let w = s
        .add_wallet(c, kit.name, &kit.descriptor, "Sample", Origin::Statement)
        .expect("wallet");
    let address = s.address(c, &s.wallets[w], false, 0);
    let message = "Faraday sample message";
    let format =
        faraday_psbt::message::Format::for_script(faraday_bip::keys::ScriptType::NativeSegwit)[0];
    let signed = s.sign_message(c, w, 0, message, format).expect("signed");
    assert_eq!(signed.address, address);
    (
        address.clone(),
        faraday_psbt::message::signed_text(&address, &signed.signature, message),
    )
}

fn arr(b: &[u8]) -> String {
    let mut s = String::from("[");
    for x in b {
        write!(s, "0x{x:02x}, ").expect("string");
    }
    s.push(']');
    s
}

fn opt(fp: Option<&[u8]>) -> String {
    fp.map_or("None".to_string(), |b| format!("Some({})", arr(b)))
}

fn emit_key(out: &mut String, k: &Key) {
    let parts: Vec<String> = k.parts.iter().map(|p| format!("{p:?}")).collect();
    writeln!(
        out,
        "    SampleKey {{ name: {:?}, word: {:?}, count: {}, form: SampleForm::{}, \
         size: SampleSize::{}, listed: {}, parts: &[{}], combined: {:?}, plain: {}, with_passphrase: {}, node: {} }},",
        k.name,
        k.word,
        k.count,
        k.form,
        k.size,
        k.listed,
        parts.join(", "),
        k.combined,
        opt(k.plain.as_ref().map(|f| &f[..])),
        opt(k.with_passphrase.as_ref().map(|f| &f[..])),
        opt(k.node.as_ref().map(|f| &f[..])),
    )
    .expect("string");
}

fn emit_wallet(out: &mut String, w: &Wallet) {
    let spend = w
        .spend
        .as_ref()
        .map_or("&[]".to_string(), |f| format!("include_bytes!(\"../{f}\")"));
    let keys: Vec<String> = w.keys.iter().map(|k| arr(k)).collect();
    writeln!(
        out,
        "    SampleWallet {{ name: {:?}, kind: SampleKind::{}, script: SampleScript::{}, \
         text: {:?}, spend: {}, keys: &[{}] }},",
        w.name,
        w.kind,
        w.script,
        w.text,
        spend,
        keys.join(", "),
    )
    .expect("string");
}

fn main() {
    let c = compute();
    std::fs::create_dir_all(root().join("data")).expect("data dir");
    let keys = keys(&c);
    let wallets = wallets(&c, &keys);
    std::fs::write(root().join("data/sample.fvlt"), vault_file(&c)).expect("write");
    let (address, message) = signed_message(&c);
    let totp = testkit::ENTRIES_FILE.lines().next().expect("a line");

    let mut out = String::from(
        "//! Generated by `examples/regenerate.rs`. Do not edit by hand.\n\n\
         use faraday_core::samples::{SampleForm, SampleInputs, SampleKey, SampleKind, SampleScript, SampleSize, SampleWallet, Samples};\n\n\
         pub(crate) static KEYS: &[SampleKey] = &[\n",
    );
    for k in &keys {
        emit_key(&mut out, k);
    }
    out.push_str("];\n\npub(crate) static WALLETS: &[SampleWallet] = &[\n");
    for w in &wallets {
        emit_wallet(&mut out, w);
    }
    out.push_str("];\n\n");
    write!(
        out,
        "pub(crate) static INPUTS: SampleInputs = SampleInputs {{\n    vault: include_bytes!(\"../data/sample.fvlt\"),\n    signed_message: {message:?},\n    address: {address:?},\n    totp: {totp:?},\n}};\n\n\
         /// Every sample.\npub static SAMPLES: Samples = Samples {{ keys: KEYS, wallets: WALLETS, inputs: INPUTS }};\n",
    )
    .expect("string");
    std::fs::write(root().join("src/data.rs"), out).expect("write");
}
