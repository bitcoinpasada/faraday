//! The account keys a file offers (`main`'s `create::read_keys`): a key file is any file that offers one.

use faraday_bip::policy::PolicyKey;
use faraday_bip::slip132;
use faraday_text::Text;

/// The account key in a file, if the file is one: the first of
/// [`read_keys`].
pub fn read_key(text: &str) -> Option<Text> {
    read_keys(text).into_iter().next()
}

/// Every account key a file offers, as `[fingerprint/path]xpub`: a BIP 129
/// key record whose signature is the key's; Coldcard's multisig key file
/// (`ccxp-….json`, which Passport and Unchained also write: flat `p2wsh`,
/// `p2sh_p2wsh` and `p2sh` fields, each with its `_deriv`); Coldcard's
/// generic export, one object per account; a single-key file (Cobo's,
/// Keystone's, Coldcard's for Wasabi); or `[fingerprint/path]xpub` on
/// lines of their own.
pub fn read_keys(text: &str) -> Vec<Text> {
    if faraday_bip::bsms::is_signer_record(text) {
        return faraday_bip::bsms::SignerRecord::parse(text)
            .ok()
            .filter(|r| r.verify().is_ok())
            .map(|r| Text::of(&r.key.key_text()))
            .into_iter()
            .collect();
    }
    if text.trim_start().starts_with('{') {
        // A single-key file (Cobo, Keystone, Coldcard's for Wasabi): its
        // one key.
        if let Ok(v) = faraday_bip::json::parse(text)
            && let Some(read) = faraday_intake::single_key::read(&v)
        {
            return read
                .ok()
                .and_then(|k| PolicyKey::parse(&k.key).ok())
                .map(|k| Text::of(&k.key_text()))
                .into_iter()
                .collect();
        }
        // Coldcard's multisig key file or generic export.
        return faraday_bip::coldcard::cosigner_keys(text)
            .iter()
            .map(|k| Text::of(k))
            .collect();
    }
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| match slip132::key_line(l) {
            Some(read) => PolicyKey::parse(&read.ok()?.text).ok(),
            None => PolicyKey::parse(l).ok(),
        })
        .map(|k| Text::of(&k.key_text()))
        .collect()
}
