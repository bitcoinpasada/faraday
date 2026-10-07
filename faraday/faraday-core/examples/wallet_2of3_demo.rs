//! Prints the demo wallet the Wallets design mockups show
//! (`design/prototype/project/WalletsWallet.dc.html` and
//! `WalletsSpend.dc.html`), derived with `osk-bip`.
//!
//! ```text
//! cargo run -p faraday-core --example wallet_2of3_demo
//! ```
//!
//! The 2-of-3 is the dummy seeds bacon, zebra and summer, each the one
//! word 24 times, at BIP-48 P2WSH `m/48'/0'/0'/2'` on mainnet: the three
//! keys, the `wsh(sortedmulti(2,…))` descriptor, receive and change
//! addresses 0 to 2, a fake funding transaction of 10 000 000 sat to
//! receive address 0, and the unsigned spend of it — 1 500 000 sat to the
//! "abandon … about" seed's BIP-84 first address, 8 497 860 back to change
//! address 0 — with its weight and fee rate once two signatures are in.
//! Last, the first address of the 3-of-5 the same three seeds are part of,
//! with trade and slice.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::absolute::LockTime;
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_bip::bitcoin::transaction::Version;
use osk_bip::bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use osk_bip::keys::{MasterKey, MultisigScriptType, Network, ScriptType};
use osk_bip::policy::WalletPolicy;

const NET: Network = Network::Mainnet;
const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The master key of `words`, with no passphrase.
fn master(words: &str) -> MasterKey {
    let m = Mnemonic::parse(Language::English, words).expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), NET)
}

/// The master key of one word repeated 24 times.
fn repeated(word: &str) -> MasterKey {
    master(&vec![word; 24].join(" "))
}

/// `wsh(sortedmulti(threshold, …))` over the BIP-48 P2WSH account of
/// every master.
fn multisig(masters: &[MasterKey], threshold: usize) -> WalletPolicy {
    let keys: Vec<String> = masters
        .iter()
        .map(|m| {
            let a = m
                .multisig_account_xpub(MultisigScriptType::NativeSegwit, 0)
                .expect("account");
            // The origin is written with `h` for hardened, as Sparrow and
            // Bitcoin Core write it; the checksum is over that text.
            let path = a.path().to_string().replace('\'', "h");
            format!("[{}/{path}]{}", a.master_fingerprint(), a.xpub())
        })
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let slots: Vec<String> = (0..masters.len()).map(|i| format!("@{i}/**")).collect();
    let template = format!("wsh(sortedmulti({threshold},{}))", slots.join(","));
    WalletPolicy::from_parts(&template, &keys).expect("policy built from fixed keys")
}

/// A version 2 transaction with one input and no witness.
fn tx(previous_output: OutPoint, output: Vec<TxOut>) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output,
    }
}

fn main() {
    let words = ["bacon", "zebra", "summer"];
    let masters: Vec<MasterKey> = words.iter().map(|w| repeated(w)).collect();
    for (word, m) in words.iter().zip(&masters) {
        let a = m
            .multisig_account_xpub(MultisigScriptType::NativeSegwit, 0)
            .expect("account");
        println!("{word} {} {}", m.fingerprint(), a.xpub());
    }
    let policy = multisig(&masters, 2);
    println!("{}", policy.to_descriptor_checksummed());
    for change in [false, true] {
        for i in 0..3 {
            let address = policy.address_at(NET, change, i).expect("address");
            println!("{}/{i} {address}", u8::from(change));
        }
    }

    // The funding transaction spends an outpoint nobody has: the SHA-256
    // of a fixed phrase, byte-reversed as a txid is shown.
    let mut fake = sha256::Hash::hash(b"openfaraday prototype funding").to_byte_array();
    fake.reverse();
    let funding = tx(
        OutPoint::new(Txid::from_byte_array(fake), 0),
        vec![TxOut {
            value: Amount::from_sat(10_000_000),
            script_pubkey: policy.script_at(false, 0).expect("receive script"),
        }],
    );
    let funding_id = funding.compute_txid();
    println!("funding txid {funding_id}");

    let abandon = master(ABANDON);
    let recipient = abandon
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("account")
        .address(false, 0)
        .expect("address");
    println!("abandon fp {} 84 0/0 {recipient}", abandon.fingerprint());

    let input = 10_000_000;
    let mut spend = tx(
        OutPoint::new(funding_id, 0),
        vec![
            TxOut {
                value: Amount::from_sat(1_500_000),
                script_pubkey: recipient.script_pubkey(),
            },
            TxOut {
                value: Amount::from_sat(8_497_860),
                script_pubkey: policy.script_at(true, 0).expect("change script"),
            },
        ],
    );
    let outputs: u64 = spend.output.iter().map(|o| o.value.to_sat()).sum();
    let fee = input - outputs;
    println!(
        "spend txid {} nonwitness bytes {}",
        spend.compute_txid(),
        spend.base_size()
    );

    // Signed, the input's witness is the empty item `CHECKMULTISIG`
    // drops, two signatures of the largest DER size with their sighash
    // byte, and the witness script of receive address 0.
    let witness_script = witness_script(&policy, false, 0);
    spend.input[0].witness = Witness::from_slice(&[
        Vec::new(),
        vec![0; 72],
        vec![0; 72],
        witness_script.into_bytes(),
    ]);
    let weight = spend.weight().to_wu();
    let vsize = spend.vsize();
    println!(
        "weight {weight} vsize {vsize} fee/vB {}",
        fee as f64 / vsize as f64
    );

    let family: Vec<MasterKey> = ["bacon", "zebra", "summer", "trade", "slice"]
        .iter()
        .map(|w| repeated(w))
        .collect();
    let family = multisig(&family, 3);
    println!(
        "3of5 0/0 {}",
        family.address_at(NET, false, 0).expect("address")
    );
}

/// The witness script of a `wsh` wallet at `change`/`index`, from the
/// wallet's own descriptor.
fn witness_script(policy: &WalletPolicy, change: bool, index: u32) -> ScriptBuf {
    use osk_bip::miniscript::descriptor::{Descriptor, DescriptorPublicKey};
    let descriptor: Descriptor<DescriptorPublicKey> =
        policy.to_descriptor().parse().expect("a descriptor");
    descriptor.into_single_descriptors().expect("two chains")[usize::from(change)]
        .clone()
        .at_derivation_index(index)
        .expect("index")
        .explicit_script()
        .expect("a wsh script")
}
