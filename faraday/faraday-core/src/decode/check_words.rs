//! Check a signer's words: every finding, stated line and field of
//! `faraday_psbt::signer` as text from `en.rs`, in the wording of
//! `docs/SIGNER-CHECK.md` §6. Each match is exhaustive, with no `_` arm,
//! so a kind added to the crate fails to compile here until it has words.

use crate::review::sighash_text;
use crate::strings::{EN, fill};
use faraday_bip::bitcoin::{Address, OutPoint, Script};
use faraday_bip::keys::Network;
use faraday_psbt::signer::{FieldName, Finding, Info, Kind, NotChecked, Stated};
use faraday_psbt::verify::{Field, Where};
use faraday_psbt::{Level, SighashKind};

/// A finding's level word: the one that leads its line.
pub(crate) fn level_word(level: Level) -> &'static str {
    match level {
        Level::Blocked => EN.signer_level_blocked,
        Level::Danger => EN.signer_level_danger,
        Level::Caution => EN.signer_level_caution,
        Level::Info => EN.signer_level_info,
    }
}

/// Where a line is: "Transaction", "Input {i}" or "Output {i}", from 0.
fn place(at: Where) -> String {
    match at {
        Where::Transaction => EN.signer_at_transaction.to_string(),
        Where::Input(i) => fill(EN.signer_at_input, &[&i.to_string()]),
        Where::Output(i) => fill(EN.signer_at_output, &[&i.to_string()]),
    }
}

/// An amount in BTC.
fn amount(sats: u64) -> String {
    String::from(&*crate::decode::btc(sats))
}

/// A script as its address in short form, or its length where it has none.
fn script(s: &Script, network: Network) -> String {
    match Address::from_script(s, faraday_bip::bitcoin::Network::from(network)) {
        Ok(a) => String::from(&*faraday_ui::short(&a.to_string())),
        Err(_) => fill(EN.signer_script_bytes, &[&s.len().to_string()]),
    }
}

/// An outpoint: its transaction's short form and the output's number.
pub(crate) fn outpoint(o: &OutPoint) -> String {
    format!("{}:{}", &*faraday_ui::short(&o.txid.to_string()), o.vout)
}

/// A pair's type byte.
fn type_byte(t: &u8) -> String {
    format!("0x{t:02x}")
}

fn field(f: Field) -> &'static str {
    match f {
        Field::Version => EN.signer_field_version,
        Field::Locktime => EN.signer_field_locktime,
        Field::InputCount => EN.signer_field_input_count,
        Field::OutputCount => EN.signer_field_output_count,
        Field::Spends => EN.signer_field_spends,
        Field::Sequence => EN.signer_field_sequence,
        Field::Amount => EN.signer_field_amount,
        Field::Script => EN.signer_field_script,
        Field::PreviousTransaction => EN.signer_field_previous_transaction,
        Field::SpentOutput => EN.signer_field_spent_output,
        Field::RedeemScript => EN.signer_field_redeem_script,
        Field::WitnessScript => EN.signer_field_witness_script,
        Field::Sighash => EN.signer_field_sighash,
        Field::KeyOrigins => EN.signer_field_key_origins,
        Field::TaprootKeyOrigins => EN.signer_field_taproot_key_origins,
        Field::InternalKey => EN.signer_field_internal_key,
        Field::MerkleRoot => EN.signer_field_merkle_root,
        Field::LeafScripts => EN.signer_field_leaf_scripts,
        Field::Unknown => EN.signer_field_unknown,
        Field::Proprietary => EN.signer_field_proprietary,
        Field::Xpubs => EN.signer_field_xpubs,
        Field::Preimages => EN.signer_field_preimages,
        Field::TapTree => EN.signer_field_tap_tree,
        Field::PsbtVersion => EN.signer_field_psbt_version,
        Field::V2Fields => EN.signer_field_v2_fields,
        Field::ModifiableFlags => EN.signer_field_modifiable,
    }
}

fn field_name(f: FieldName) -> &'static str {
    match f {
        FieldName::Compared(f) => field(f),
        FieldName::MusigParticipants => EN.signer_field_musig_participants,
        FieldName::MusigNonce => EN.signer_field_musig_nonce,
        FieldName::MusigPartialSig => EN.signer_field_musig_partial_sig,
    }
}

fn stated_field(s: Stated) -> &'static str {
    match s {
        Stated::WitnessScript => EN.signer_field_witness_script,
        Stated::RedeemScript => EN.signer_field_redeem_script,
        Stated::SpentOutput => EN.signer_field_spent_output,
        Stated::PreviousTransaction => EN.signer_field_previous_transaction,
        Stated::TaprootKey => EN.signer_field_internal_key,
        Stated::Sighash => EN.signer_field_sighash,
    }
}

fn not_checked(n: NotChecked) -> &'static str {
    match n {
        NotChecked::Signature(u) => crate::review::unchecked_text(u),
        NotChecked::MusigWallet => EN.signer_unchecked_musig_wallet,
        NotChecked::MusigNonce => EN.signer_unchecked_musig_nonce,
    }
}

/// A line about a signature's sighash and the one asked for.
fn sighash(
    text: &str,
    at: &str,
    key: &str,
    (signed, asked): (&SighashKind, &SighashKind),
) -> String {
    let (signed, asked) = (sighash_text(*signed), sighash_text(*asked));
    fill(text, &[at, key, &signed, &asked])
}

/// An unknown or proprietary pair: its type or subtype and its sizes,
/// never its content (SIGNER-CHECK §6.6). `None` for any other kind.
fn pair(at: &str, kind: &Kind) -> Option<String> {
    let pn = |v: &usize| v.to_string();
    match kind {
        Kind::UnknownAdded {
            type_value,
            key_len,
            value_len,
        } => Some(fill(
            EN.signer_p_unknown_added,
            &[at, &type_byte(type_value), &pn(key_len), &pn(value_len)],
        )),
        Kind::UnknownChanged {
            type_value,
            from,
            to,
        } => Some(fill(
            EN.signer_p_unknown_changed,
            &[at, &type_byte(type_value), &pn(from), &pn(to)],
        )),
        Kind::ProprietaryAdded {
            prefix_len,
            subtype,
            value_len,
        } => Some(fill(
            EN.signer_p_proprietary_added,
            &[at, &pn(prefix_len), &subtype.to_string(), &pn(value_len)],
        )),
        Kind::ProprietaryChanged {
            prefix_len,
            subtype,
            from,
            to,
        } => Some(fill(
            EN.signer_p_proprietary_changed,
            &[
                at,
                &pn(prefix_len),
                &subtype.to_string(),
                &pn(from),
                &pn(to),
            ],
        )),
        Kind::UnknownRemoved { type_value } => Some(fill(
            EN.signer_p_unknown_removed,
            &[at, &type_byte(type_value)],
        )),
        Kind::ProprietaryRemoved {
            prefix_len,
            subtype,
        } => Some(fill(
            EN.signer_p_proprietary_removed,
            &[at, &pn(prefix_len), &subtype.to_string()],
        )),
        _ => None,
    }
}

/// A finding as one line (SIGNER-CHECK §6), amounts in BTC and scripts as
/// addresses for `network`.
pub(crate) fn finding(f: &Finding, network: Network) -> String {
    let at = place(f.at);
    let n = |v: usize| v.to_string();
    match &f.kind {
        Kind::Version { sent, returned } => fill(
            EN.signer_t_version,
            &[&at, &sent.to_string(), &returned.to_string()],
        ),
        Kind::Locktime { sent, returned } => fill(
            EN.signer_t_locktime,
            &[&at, &sent.to_string(), &returned.to_string()],
        ),
        Kind::InputCount { sent, returned } => {
            fill(EN.signer_t_inputs, &[&at, &n(*sent), &n(*returned)])
        }
        Kind::OutputCount { sent, returned } => {
            fill(EN.signer_t_outputs, &[&at, &n(*sent), &n(*returned)])
        }
        Kind::Spends { sent, returned } => fill(
            EN.signer_t_spends,
            &[&at, &outpoint(returned), &outpoint(sent)],
        ),
        Kind::Sequence { sent, returned } => fill(
            EN.signer_t_sequence,
            &[&at, &sent.to_string(), &returned.to_string()],
        ),
        Kind::OutputAmount { sent, returned } => fill(
            EN.signer_t_amount,
            &[&at, &amount(*returned), &amount(*sent)],
        ),
        Kind::OutputScript { sent, returned } => fill(
            EN.signer_t_pays,
            &[&at, &script(returned, network), &script(sent, network)],
        ),
        Kind::SignatureInvalid { key } => fill(EN.signer_s_invalid, &[&at, key]),
        Kind::SignatureUnchecked { key, reason } => {
            fill(EN.signer_s_unchecked, &[&at, key, not_checked(*reason)])
        }
        Kind::SignatureUnnamed { key } => fill(EN.signer_s_unnamed, &[&at, key]),
        Kind::SignatureLeafUnknown { key } => fill(EN.signer_s_leaf, &[&at, key]),
        Kind::SighashOther { key, signed, asked } => {
            sighash(EN.signer_s_sighash, &at, key, (signed, asked))
        }
        Kind::SighashNone { key, signed, asked } => {
            sighash(EN.signer_s_sighash_none, &at, key, (signed, asked))
        }
        Kind::SighashStricter { key, signed, asked } => {
            sighash(EN.signer_s_sighash_more, &at, key, (signed, asked))
        }
        Kind::RepeatedNonce { key, inputs } => {
            let list: Vec<String> = inputs.iter().map(|i| i.to_string()).collect();
            fill(EN.signer_s_nonce_reuse, &[key, &list.join(", ")])
        }
        Kind::NonceNotDeterministic { fingerprint } => {
            fill(EN.signer_s_not_reproduced, &[&at, &fingerprint.to_string()])
        }
        Kind::NonceNotRecomputed { fingerprint } => {
            fill(EN.signer_s_not_recomputed, &[&at, &fingerprint.to_string()])
        }
        Kind::SignatureRemoved { key } => fill(EN.signer_s_removed, &[&at, key]),
        Kind::SignatureChanged { key } => fill(EN.signer_s_changed, &[&at, key]),
        Kind::NoSignatureAdded => EN.signer_s_none_added.to_string(),
        Kind::FinalDiffers => fill(EN.signer_f_differs, &[&at]),
        Kind::FinalNotCompared => fill(EN.signer_f_not_compared, &[&at]),
        Kind::FinalChanged => fill(EN.signer_f_changed, &[&at]),
        Kind::FinalRemoved => fill(EN.signer_f_removed, &[&at]),
        Kind::FieldAdded(x) => fill(EN.signer_m_added, &[&at, field_name(*x)]),
        Kind::FieldChanged(x) => fill(EN.signer_m_changed, &[&at, field_name(*x)]),
        Kind::FieldRemoved(x) => fill(EN.signer_m_removed, &[&at, field_name(*x)]),
        Kind::UnknownAdded { .. }
        | Kind::UnknownChanged { .. }
        | Kind::ProprietaryAdded { .. }
        | Kind::ProprietaryChanged { .. }
        | Kind::UnknownRemoved { .. }
        | Kind::ProprietaryRemoved { .. } => pair(&at, &f.kind).unwrap_or_default(),
        Kind::UnaccountedBytes { count } => fill(EN.signer_e_unaccounted, &[&n(*count)]),
        Kind::Unnamed(x) => fill(EN.signer_unnamed, &[&at, field(*x)]),
    }
}

/// A line the report states that is no finding (SIGNER-CHECK §6.8).
pub(crate) fn stated(i: &Info) -> String {
    match i {
        Info::NonceAdded { input, key } => fill(
            EN.signer_i_nonce_added,
            &[&place(Where::Input(*input)), key],
        ),
        Info::Finalized { input } => fill(EN.signer_i_finalized, &[&place(Where::Input(*input))]),
        Info::FieldStated { at, field } => {
            fill(EN.signer_m_added, &[&place(*at), stated_field(*field)])
        }
    }
}
