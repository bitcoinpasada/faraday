//! Faraday's PSBT handling against rust-bitcoin's, on the same bytes.
//!
//! `Psbt::parse_bytes` calls `bitcoin::Psbt::deserialize` itself and then
//! refuses more, so that the two accept and refuse alike is not a
//! comparison of two readers. What is Faraday's own is compared here
//! against values this target works out from `bitcoin::Psbt` and from
//! the raw bytes, never through faraday-psbt's helpers:
//!
//! 1. the raw map reader (`maps_of`): a second reader in this target reads
//!    the maps, its pair counts agree with rust-bitcoin's parsed fields,
//!    and a PSBT read is written back byte for byte;
//! 2. acceptance: Faraday accepts only what rust-bitcoin accepts, and
//!    where rust-bitcoin accepts, Faraday's answer is the one this
//!    target works out from its own reading of the maps (bytes after the
//!    last map, a field of version 2, a taproot signature of sighash 0, a
//!    key origin of over 16 steps, a version other than 0), with the
//!    error of that kind;
//! 3. `inspect`: each input's amount, each output's amount, script and
//!    address, the totals and the fee, against the same worked out from
//!    the transaction and each input's `witness_utxo` and
//!    `non_witness_utxo`, and rust-bitcoin's `Psbt::fee()`; the fee is
//!    unknown (no rate) exactly where an input's amount is unknown, a
//!    total is over the 21 million, or the outputs exceed the inputs;
//! 4. writing: what Faraday writes is read by rust-bitcoin as the same
//!    PSBT, and what rust-bitcoin writes is read by Faraday;
//! 5. version 2 (rust-bitcoin reads none): a PSBT Faraday's reader accepts,
//!    made version 0, is read by rust-bitcoin, and the transaction (its
//!    version, outpoints, sequences, outputs, locktime) and the amounts
//!    and fee agree with what this target works out from the version 2
//!    fields.
//!
//! The sighashes are not compared: every one Faraday works out is
//! `bitcoin::sighash::SighashCache`'s (message.rs, finalize.rs, musig.rs,
//! sweep.rs), so there is no second implementation of Faraday's to hold
//! against it.

#![no_main]

use bitcoin::consensus::{deserialize, serialize};
use bitcoin::{Address, Amount, OutPoint, Transaction, TxOut};
use faraday_bip::keys::{MAX_PATH_STEPS, Network};
use faraday_crypto::work::Compute;
use faraday_psbt::v2::{self, PsbtV2};
use faraday_psbt::{Context, Error, Psbt, inspect};
use libfuzzer_sys::fuzz_target;

type Pairs = Vec<(Vec<u8>, Vec<u8>)>;

/// A compact size, as the PSBT format writes lengths.
fn compact(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
        let s = bytes.get(*at..at.checked_add(n)?)?;
        *at += n;
        Some(s)
    };
    let first = take(at, 1)?[0];
    Some(match first {
        0xfd => u64::from(u16::from_le_bytes(take(at, 2)?.try_into().ok()?)),
        0xfe => u64::from(u32::from_le_bytes(take(at, 4)?.try_into().ok()?)),
        0xff => u64::from_le_bytes(take(at, 8)?.try_into().ok()?),
        n => u64::from(n),
    })
}

/// The `want` maps after the magic, and whether the bytes end with the
/// last one; `None` where a map is cut short or repeats a key.
fn read_maps(bytes: &[u8], want: usize) -> Option<(Vec<Pairs>, bool)> {
    let mut at = 5;
    let mut maps = Vec::new();
    for _ in 0..want {
        let mut pairs: Pairs = Vec::new();
        loop {
            let key_len = usize::try_from(compact(bytes, &mut at)?).ok()?;
            if key_len == 0 {
                break;
            }
            let key = bytes.get(at..at.checked_add(key_len)?)?.to_vec();
            at += key_len;
            let value_len = usize::try_from(compact(bytes, &mut at)?).ok()?;
            let value = bytes.get(at..at.checked_add(value_len)?)?.to_vec();
            at += value_len;
            if pairs.iter().any(|(k, _)| *k == key) {
                return None;
            }
            pairs.push((key, value));
        }
        maps.push(pairs);
    }
    Some((maps, at == bytes.len()))
}

fn of_type(map: &Pairs, ty: u8) -> usize {
    map.iter().filter(|(k, _)| k[0] == ty).count()
}

fn has_key(map: &Pairs, key: &[u8]) -> bool {
    map.iter().any(|(k, _)| k == key)
}

/// What BIP-370 and BIP-341 refuse in a version 0 PSBT that rust-bitcoin
/// reads, from the maps and rust-bitcoin's fields.
fn refused_in_v0(maps: &[Pairs], inner: &bitcoin::Psbt) -> bool {
    let n_in = inner.inputs.len();
    for (i, map) in maps.iter().enumerate() {
        for (k, v) in map {
            let ty = k[0];
            let v2_only = if i == 0 {
                (2..=6).contains(&ty)
            } else if i <= n_in {
                (0x0e..=0x12).contains(&ty)
            } else {
                ty == 3 || ty == 4
            };
            if v2_only {
                return true;
            }
            let zero_sighash = v.len() == 65 && v[64] == 0;
            if i >= 1 && i <= n_in && zero_sighash && (k[..] == [0x13] || ty == 0x14) {
                return true;
            }
        }
    }
    let long = |p: &bitcoin::bip32::DerivationPath| p.len() > MAX_PATH_STEPS;
    inner.xpub.values().any(|(_, p)| long(p))
        || inner.inputs.iter().any(|i| {
            i.bip32_derivation.values().any(|(_, p)| long(p))
                || i.tap_key_origins.values().any(|(_, (_, p))| long(p))
        })
        || inner.outputs.iter().any(|o| {
            o.bip32_derivation.values().any(|(_, p)| long(p))
                || o.tap_key_origins.values().any(|(_, (_, p))| long(p))
        })
}

/// The pair counts of this target's maps against rust-bitcoin's fields.
fn maps_agree(maps: &[Pairs], inner: &bitcoin::Psbt) {
    let n_in = inner.inputs.len();
    let global = &maps[0];
    let raw_tx = &global
        .iter()
        .find(|(k, _)| k[..] == [0x00])
        .expect("rust-bitcoin needs it")
        .1;
    // Written without a witness marker, as a transaction of no inputs
    // is (a `deserialize` of a `Transaction` would not read that).
    let tx = &inner.unsigned_tx;
    let mut want = serialize(&tx.version);
    want.extend(serialize(&tx.input));
    want.extend(serialize(&tx.output));
    want.extend(serialize(&tx.lock_time));
    assert_eq!(*raw_tx, want);
    assert_eq!(of_type(global, 0x01), inner.xpub.len());
    for (i, input) in inner.inputs.iter().enumerate() {
        let map = &maps[1 + i];
        assert_eq!(of_type(map, 0x02), input.partial_sigs.len());
        assert_eq!(of_type(map, 0x06), input.bip32_derivation.len());
        assert_eq!(of_type(map, 0x14), input.tap_script_sigs.len());
        assert_eq!(of_type(map, 0x15), input.tap_scripts.len());
        assert_eq!(of_type(map, 0x16), input.tap_key_origins.len());
        assert_eq!(has_key(map, &[0x00]), input.non_witness_utxo.is_some());
        assert_eq!(has_key(map, &[0x01]), input.witness_utxo.is_some());
        assert_eq!(has_key(map, &[0x13]), input.tap_key_sig.is_some());
    }
    for (i, output) in inner.outputs.iter().enumerate() {
        let map = &maps[1 + n_in + i];
        assert_eq!(of_type(map, 0x02), output.bip32_derivation.len());
        assert_eq!(of_type(map, 0x07), output.tap_key_origins.len());
        assert_eq!(has_key(map, &[0x00]), output.redeem_script.is_some());
        assert_eq!(has_key(map, &[0x01]), output.witness_script.is_some());
    }
}

/// The output input `prev` spends, as a node reads it: the output of the
/// previous transaction where there is one and it is the one spent,
/// else the `witness_utxo`.
fn spent(
    prev: OutPoint,
    non_witness: Option<&Transaction>,
    witness: Option<&TxOut>,
) -> Option<Amount> {
    match non_witness {
        Some(t) if t.compute_txid() == prev.txid => {
            t.output.get(prev.vout as usize).map(|o| o.value)
        }
        Some(_) => None,
        None => witness.map(|o| o.value),
    }
}

/// rust-bitcoin's own fee, where it reads the same outputs Faraday does.
/// `Psbt::fee()` takes an input's `witness_utxo` before its previous
/// transaction and does not check the previous transaction's txid; Faraday
/// takes the previous transaction (a node reads that one) and states the
/// output unknown where it is not the one spent. They differ by design
/// where an input carries both and they disagree, or the previous
/// transaction is not the one spent, so the fee is compared elsewhere.
fn their_fee(psbt: &bitcoin::Psbt) -> Option<Amount> {
    let same = psbt.unsigned_tx.input.iter().zip(&psbt.inputs).all(|(txin, input)| {
        let prev = txin.previous_output;
        match (&input.witness_utxo, &input.non_witness_utxo) {
            (_, Some(t)) if t.compute_txid() != prev.txid => false,
            (Some(w), Some(t)) => t.output.get(prev.vout as usize) == Some(w),
            _ => true,
        }
    });
    if same { psbt.fee().ok() } else { None }
}

fn sum(amounts: &[Amount]) -> u128 {
    amounts.iter().map(|a| u128::from(a.to_sat())).sum()
}

/// Whether amounts are each and together within the 21 million.
fn in_range(amounts: &[Amount]) -> bool {
    let max = u128::from(Amount::MAX_MONEY.to_sat());
    amounts.iter().all(|a| u128::from(a.to_sat()) <= max) && sum(amounts) <= max
}

/// What the Sign screens show of a transaction, against what is worked
/// out here: `inputs` is each input's spent amount as the fields give it.
fn check_inspection(psbt: &Psbt, inputs: &[Option<Amount>], fee_by_bitcoin: Option<Amount>) {
    let c = Compute::unchecked();
    let ctx = Context {
        network: Network::Mainnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let tx = psbt.unsigned_tx();
    let seen = inspect(&c, psbt, &ctx);

    assert_eq!(seen.inputs.len(), tx.input.len());
    for ((info, txin), amount) in seen.inputs.iter().zip(&tx.input).zip(inputs) {
        assert_eq!(info.txid, txin.previous_output.txid);
        assert_eq!(info.vout, txin.previous_output.vout);
        assert_eq!(info.sequence, txin.sequence);
        assert_eq!(info.amount, *amount);
    }
    assert_eq!(seen.outputs.len(), tx.output.len());
    for (info, txout) in seen.outputs.iter().zip(&tx.output) {
        assert_eq!(info.amount, txout.value);
        assert_eq!(info.script_pubkey, txout.script_pubkey);
        // An address is shown where the script has one, else the field is empty.
        let address = Address::from_script(&txout.script_pubkey, bitcoin::Network::Bitcoin)
            .map(|a| a.to_string())
            .unwrap_or_default();
        assert_eq!(info.address, address);
    }
    assert_eq!(seen.version, tx.version.0);

    let known: Vec<Amount> = inputs.iter().flatten().copied().collect();
    let outs: Vec<Amount> = tx.output.iter().map(|o| o.value).collect();
    let (in_ok, out_ok) = (in_range(&known), in_range(&outs));
    if in_ok {
        assert_eq!(u128::from(seen.total_in.to_sat()), sum(&known));
    }
    if out_ok {
        assert_eq!(u128::from(seen.total_out.to_sat()), sum(&outs));
    }
    // The rule: the fee is known when every input's amount is, both
    // totals are within the 21 million, and the outputs do not exceed
    // the inputs; the fee rate is given exactly then.
    let fee_known = known.len() == inputs.len() && in_ok && out_ok && sum(&known) >= sum(&outs);
    assert_eq!(seen.fee_rate_sat_vb.is_some(), fee_known);
    if fee_known {
        let fee = sum(&known) - sum(&outs);
        assert_eq!(u128::from(seen.fee.to_sat()), fee);
        if let Some(theirs) = fee_by_bitcoin {
            assert_eq!(u128::from(theirs.to_sat()), fee);
        }
    }
}

/// A PSBT rust-bitcoin read: everything compared.
fn rust_bitcoin_read(c: &Compute, data: &[u8], inner: &bitcoin::Psbt) {
    let n_in = inner.inputs.len();
    let maps = read_maps(data, 1 + n_in + inner.outputs.len());
    let structure = maps.as_ref().filter(|(_, whole)| *whole).map(|(m, _)| m);
    let expected_ok =
        inner.version == 0 && structure.is_some_and(|maps| !refused_in_v0(maps, inner));

    // 2: acceptance.
    let ours = Psbt::parse_bytes(c, data);
    match (&ours, expected_ok) {
        (Ok(_), true) => {}
        (Err(Error::UnsupportedVersion(v)), false) if inner.version != 0 => {
            assert_eq!(*v, inner.version);
        }
        (Err(Error::Parse(_)), false) if inner.version == 0 => {}
        _ => panic!("Faraday answered {:?}, expected ok = {expected_ok}", ours.map(|_| ())),
    }
    let Ok(psbt) = ours else { return };
    let maps = structure.expect("accepted");

    // 1: the maps.
    maps_agree(maps, inner);
    assert_eq!(psbt.inner(), inner);

    // 4: writing.
    assert_eq!(psbt.to_bytes(), data, "a PSBT read is written back as it was");
    assert_eq!(&bitcoin::Psbt::deserialize(&psbt.to_bytes()).expect("read again"), inner);
    let theirs = Psbt::parse_bytes(c, &inner.serialize())
        .expect("what rust-bitcoin writes is read by Faraday");
    assert_eq!(theirs.inner(), inner);

    // 3: inspect.
    let amounts: Vec<Option<Amount>> = inner
        .unsigned_tx
        .input
        .iter()
        .zip(&inner.inputs)
        .map(|(t, i)| {
            spent(t.previous_output, i.non_witness_utxo.as_ref(), i.witness_utxo.as_ref())
        })
        .collect();
    check_inspection(&psbt, &amounts, their_fee(inner));
}

/// A version 2 PSBT Faraday's reader accepted: made version 0 it is read
/// by rust-bitcoin as the transaction its fields give.
fn version_2(c: &Compute, psbt: &PsbtV2) {
    let Ok(v0) = psbt.to_v0(c) else { return };
    let bytes = v0.to_bytes();
    let theirs = bitcoin::Psbt::deserialize(&bytes).expect("rust-bitcoin reads the version 0 PSBT");
    assert_eq!(&theirs, v0.inner());
    let again = Psbt::parse_bytes(c, &theirs.serialize()).expect("read");
    assert_eq!(again.inner(), &theirs);

    // The transaction, from the version 2 fields.
    let tx = &theirs.unsigned_tx;
    let u32_of = |v: &[u8]| u32::from_le_bytes(v.try_into().expect("four bytes"));
    let version = psbt.global.get_type(v2::GLOBAL_TX_VERSION).expect("checked on reading");
    assert_eq!(tx.version.0 as u32, u32_of(version));
    assert_eq!(tx.input.len(), psbt.inputs.len());
    let mut heights = Vec::new();
    let mut times = Vec::new();
    for (txin, map) in tx.input.iter().zip(&psbt.inputs) {
        let mut want = map.get_type(v2::IN_PREVIOUS_TXID).expect("checked").to_vec();
        want.extend_from_slice(map.get_type(v2::IN_OUTPUT_INDEX).expect("checked"));
        assert_eq!(bitcoin::consensus::serialize(&txin.previous_output), want, "outpoint");
        assert_eq!(txin.sequence.0, map.get_type(v2::IN_SEQUENCE).map_or(u32::MAX, u32_of));
        heights.extend(map.get_type(v2::IN_REQUIRED_HEIGHT_LOCKTIME).map(u32_of));
        times.extend(map.get_type(v2::IN_REQUIRED_TIME_LOCKTIME).map(u32_of));
    }
    assert_eq!(tx.output.len(), psbt.outputs.len());
    for (txout, map) in tx.output.iter().zip(&psbt.outputs) {
        let amount = u64::from_le_bytes(
            map.get_type(v2::OUT_AMOUNT).expect("checked").try_into().expect("eight bytes"),
        );
        assert_eq!(txout.value.to_sat(), amount);
        assert_eq!(txout.script_pubkey.as_bytes(), map.get_type(v2::OUT_SCRIPT).expect("given"));
    }
    let lock = tx.lock_time.to_consensus_u32();
    if heights.is_empty() && times.is_empty() {
        let fallback = psbt.global.get_type(v2::GLOBAL_FALLBACK_LOCKTIME).map_or(0, u32_of);
        assert_eq!(lock, fallback);
    } else {
        // BIP-370: the largest required height, else the largest required time.
        assert!(heights.contains(&lock) || times.contains(&lock));
    }

    // The amounts, from the version 2 fields.
    let amounts: Vec<Option<Amount>> = tx
        .input
        .iter()
        .zip(&psbt.inputs)
        .map(|(txin, map)| {
            let non_witness: Option<Transaction> =
                map.get_type(v2::IN_NON_WITNESS_UTXO).and_then(|v| deserialize(v).ok());
            let witness: Option<TxOut> =
                map.get_type(v2::IN_WITNESS_UTXO).and_then(|v| deserialize(v).ok());
            spent(txin.previous_output, non_witness.as_ref(), witness.as_ref())
        })
        .collect();
    check_inspection(&v0, &amounts, their_fee(&theirs));
}

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    if let Ok(inner) = bitcoin::Psbt::deserialize(data) {
        rust_bitcoin_read(&c, data, &inner);
    } else if let Ok(psbt) = Psbt::parse_bytes(&c, data) {
        panic!("Faraday accepted what rust-bitcoin refuses: {}", psbt.to_base64());
    }
    if let Ok(psbt) = PsbtV2::parse_bytes(&c, data) {
        version_2(&c, &psbt);
    }
});
