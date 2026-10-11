//! The text side of the blob's wallets record (`docs/PLANNING.md` §6,
//! §16.72).
//!
//! A device that keeps keys keeps the wallets in use with them, as
//! public data under the same protection, so that Wallets is not empty
//! after a restart on a device where Keys is not. What is kept is each
//! wallet's BIP-388 two-part text, or its descriptor where its key
//! arrived with no origin and so has no BIP-388 form
//! ([`WalletPolicy::to_text`] writes the one the wallet has).
//!
//! This module exists because the crate root may hold no heap text
//! (`tools/lint-secrets.sh` rule 3, which is about the words and the
//! PIN). The split is: the wallets are policies here and a fixed byte
//! array there. The crate root seals and opens [`BODY_LEN`] bytes and never
//! looks inside them; this module turns those bytes into wallets and
//! back, and never touches a key, a PIN or the cipher.
//!
//! The body is a count and [`MAX_WALLETS`] slots of [`SLOT_LEN`] bytes,
//! each holding one wallet's UTF-8 text zero-padded to the slot, so the
//! stored bytes say neither how many wallets there are nor how large
//! each one is.
//!
//! A named wallet's slot carries `name=<utf-8>` as its first line,
//! before the policy text; a slot with no such line is a wallet with no
//! name, so a blob written before names existed still opens.
//!
//! The name of a wallet over one key belongs with that key, and the
//! keys record has no room for it: its slots are a fixed
//! `MNEMONIC_LEN + 1` bytes, every one of them spoken for, and widening
//! them would change the record's length and so the blob's. The names
//! of those wallets therefore live here too, keyed by fingerprint, in
//! slots of the form `key=<8 hex>` and `name=<utf-8>`. They take slots
//! the wallets did not, so a device with sixteen wallets keeps no
//! single-sig names.
//!
//! [`SLOT_LEN`] is 2,304 bytes because the largest wallet this build
//! accepts is a 15-of-15 multisig: its template is 115 characters and
//! each of its fifteen keys is at most about 135 (a 111-character
//! extended key, an eight-hex master fingerprint and a derivation path),
//! which is about 2,141 bytes, and the rest is room for longer paths.
//! [`MAX_WALLETS`] is 16 because a wallet is registered by hand, one at
//! a time, from a code or a file.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::Fingerprint;
use osk_bip::policy::WalletPolicy;

use crate::names::WalletNames;

/// Wallets the blob has room for.
pub const MAX_WALLETS: usize = 16;
/// Bytes one wallet's text is padded to.
pub const SLOT_LEN: usize = 2304;
/// The record's body: the count, then every slot.
pub const BODY_LEN: usize = 1 + MAX_WALLETS * SLOT_LEN;

/// The line a named slot begins with.
const NAME_LINE: &str = "name=";
/// The line a single-sig name's slot begins with.
const KEY_LINE: &str = "key=";

/// The body for the wallets in use and the names they have been given.
/// Wallets past the sixteenth, and any whose text does not fit a slot,
/// are not kept: a device that keeps what it can is better than one
/// that keeps nothing, and the wallet is still the session's.
pub fn body(wallets: &[WalletPolicy], names: &WalletNames) -> Vec<u8> {
    let mut out = vec![0u8; BODY_LEN];
    let mut count = 0usize;
    let mut put = |text: &str, count: &mut usize| {
        let bytes = text.as_bytes();
        if *count == MAX_WALLETS || bytes.len() > SLOT_LEN {
            return;
        }
        let at = 1 + *count * SLOT_LEN;
        out[at..at + bytes.len()].copy_from_slice(bytes);
        *count += 1;
    };
    for wallet in wallets {
        let text = wallet.to_text();
        match names.policy_name(wallet) {
            Some(name) => put(&alloc::format!("{NAME_LINE}{name}\n{text}"), &mut count),
            None => put(&text, &mut count),
        }
    }
    for (fp, name) in names.key_names() {
        let hex = String::from(core::str::from_utf8(&fp.to_hex()).unwrap_or("00000000"));
        put(
            &alloc::format!("{KEY_LINE}{hex}\n{NAME_LINE}{name}"),
            &mut count,
        );
    }
    out[0] = count as u8;
    out
}

/// The wallets a body holds, in the order they were kept, and the names
/// it carries. A slot that is not a wallet this build reads is dropped
/// rather than failing the unlock: the keys matter more than the list
/// of wallets.
pub fn wallets(body: &[u8]) -> (Vec<WalletPolicy>, WalletNames) {
    let mut out = Vec::new();
    let mut names = WalletNames::new();
    if body.len() != BODY_LEN {
        return (out, names);
    }
    let count = usize::from(body[0]).min(MAX_WALLETS);
    for i in 0..count {
        let at = 1 + i * SLOT_LEN;
        let slot = &body[at..at + SLOT_LEN];
        let text = match core::str::from_utf8(slot) {
            Ok(text) => text.trim_end_matches('\0'),
            Err(_) => continue,
        };
        if let Some(rest) = text.strip_prefix(KEY_LINE) {
            if let Some((hex, name)) = rest.split_once('\n')
                && let Some(fp) = fingerprint(hex)
                && let Some(name) = name.strip_prefix(NAME_LINE)
            {
                names.set_key_name(fp, name);
            }
            continue;
        }
        let (name, text) = match text.strip_prefix(NAME_LINE) {
            Some(rest) => match rest.split_once('\n') {
                Some((name, text)) => (Some(name), text),
                None => (None, text),
            },
            None => (None, text),
        };
        if let Ok(policy) = WalletPolicy::parse_any(text)
            && !out.contains(&policy)
        {
            if let Some(name) = name {
                names.set_policy_name(&policy, name);
            }
            out.push(policy);
        }
    }
    (out, names)
}

/// The fingerprint eight hex characters spell.
fn fingerprint(hex: &str) -> Option<Fingerprint> {
    if hex.len() != 8 {
        return None;
    }
    let mut bytes = [0u8; 4];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(Fingerprint(bytes))
}
