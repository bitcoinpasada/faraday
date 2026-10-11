//! The group record of a threshold wallet (`docs/PLANNING.md` §16.103
//! item 2): what a FROST group publishes about itself, as one text form.
//!
//! ```text
//! osk-threshold 1
//! threshold 2
//! group 02d772a0…
//! share 0 039ee333…
//! share 1 0284dc4a…
//! share 2 036441ec…
//! tr(tpubD6NzVbkrYhZ4X…/<0;1>/*)#checksum
//! ```
//!
//! The record is public data: the threshold, the group key, every
//! participant's public share, and the descriptor the wallet's addresses
//! come from. No secret share appears in it, and nothing in this module
//! touches one.
//!
//! The form is this project's own, kept until a BIP defines a `frost()`
//! key expression or a record of its own; when one does, the device
//! reads both.

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::NetworkKind;
use bitcoin::bip32::Xpub;
use bitcoin::hashes::{Hash, hash160};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};

use crate::descriptor::descriptor_checksum;
use crate::frost::ThresholdInfo;
use crate::keys::{Fingerprint, ScriptType};
use crate::musig;
use crate::policy::{DESCRIPTOR_SUFFIX, Error, Template, WalletPolicy};

/// The first line's name.
pub const MAGIC: &str = "osk-threshold";
/// The version this build writes and the only one it reads.
pub const VERSION: &str = "1";
/// The most participants a record carries: what one QR and one slot of
/// the blob hold. BIP 445 allows 128, and key generation offers five.
pub const MAX_PARTICIPANTS: usize = 15;

const THRESHOLD_LINE: &str = "threshold ";
const GROUP_LINE: &str = "group ";
const SHARE_LINE: &str = "share ";

/// A group's public record: BIP 445's [`ThresholdInfo`] and the
/// synthetic extended public key of the group key, which is what the
/// wallet's descriptor derives from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdRecord {
    /// The threshold, the group key and one public share per
    /// participant.
    pub info: ThresholdInfo,
    /// BIP-328's extended public key over the group key, on the
    /// network its version bytes state.
    pub xpub: Xpub,
}

impl ThresholdRecord {
    /// Whether `text` is offered as a record: its first line that is
    /// neither blank nor a comment names the format.
    pub fn looks_like_record(text: &str) -> bool {
        text.lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .is_some_and(|l| l.starts_with(MAGIC))
    }

    /// The record the group key, the shares and the descriptor make, for
    /// a group this device dealt or recovered.
    pub fn new(info: ThresholdInfo, network: NetworkKind) -> ThresholdRecord {
        let xpub = musig::synthetic_xpub(info.thresh_pk, network);
        ThresholdRecord { info, xpub }
    }

    /// Reads a record, checking everything it states.
    ///
    /// The threshold is between one and the participant count, the
    /// participants are numbered from zero with none missing, every key
    /// is a point, the shares lie on one polynomial that gives this
    /// group key, the descriptor is the group key's own synthetic xpub
    /// with both chains, and its checksum holds.
    pub fn parse(text: &str) -> Result<ThresholdRecord, Error> {
        let secp = Secp256k1::verification_only();
        ThresholdRecord::parse_with(&secp, text)
    }

    /// The same, with a caller's secp context.
    pub fn parse_with<C: Verification>(
        secp: &Secp256k1<C>,
        text: &str,
    ) -> Result<ThresholdRecord, Error> {
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        let head = lines.next().ok_or(Error::Template)?;
        match head.split_once(' ') {
            Some((MAGIC, VERSION)) => {}
            _ => return Err(Error::Template),
        }
        let t: usize = lines
            .next()
            .and_then(|l| l.strip_prefix(THRESHOLD_LINE))
            .ok_or(Error::Template)?
            .parse()
            .map_err(|_| Error::Threshold)?;
        let group = point(
            lines
                .next()
                .and_then(|l| l.strip_prefix(GROUP_LINE))
                .ok_or(Error::Template)?,
        )?;
        let mut pubshares: Vec<Option<PublicKey>> = Vec::new();
        let mut descriptor: Option<&str> = None;
        for line in lines {
            match line.strip_prefix(SHARE_LINE) {
                // Every share line comes before the descriptor, which is
                // the record's last line.
                Some(_) if descriptor.is_some() => return Err(Error::Template),
                Some(share) => {
                    let (id, key) = share.split_once(' ').ok_or(Error::Share)?;
                    // The identifiers are 0, 1, … in order with none
                    // missing: BIP 445 allows a record to leave a share
                    // out, and this version of the file does not.
                    if id.parse() != Ok(pubshares.len()) {
                        return Err(Error::Share);
                    }
                    pubshares.push(Some(point(key)?));
                }
                None if descriptor.is_some() => return Err(Error::Template),
                None => descriptor = Some(line),
            }
        }
        let n = pubshares.len();
        if n == 0 || n > MAX_PARTICIPANTS {
            return Err(Error::Threshold);
        }
        if t < 1 || t > n {
            return Err(Error::Threshold);
        }
        let info = ThresholdInfo {
            t,
            thresh_pk: group,
            pubshares,
        };
        info.validate(secp).map_err(|_| Error::Group)?;
        let xpub = descriptor_xpub(descriptor.ok_or(Error::Template)?)?;
        if xpub != musig::synthetic_xpub(group, xpub.network) {
            return Err(Error::Xpub);
        }
        Ok(ThresholdRecord { info, xpub })
    }

    /// How many shares sign.
    pub fn t(&self) -> usize {
        self.info.t
    }

    /// How many participants there are.
    pub fn n(&self) -> usize {
        self.info.n()
    }

    /// The wallet's descriptor, without its checksum.
    pub fn descriptor(&self) -> String {
        alloc::format!("tr({}{DESCRIPTOR_SUFFIX})", self.xpub)
    }

    /// The same with its BIP-380 checksum, which is the record's last
    /// line.
    pub fn descriptor_checksummed(&self) -> String {
        let descriptor = self.descriptor();
        let sum =
            descriptor_checksum(&descriptor).expect("descriptor uses only charset characters");
        alloc::format!("{descriptor}#{}", String::from_utf8_lossy(&sum))
    }

    /// The whole record as the file and the QR carry it.
    pub fn to_text(&self) -> String {
        let mut out = alloc::format!(
            "{MAGIC} {VERSION}\n{THRESHOLD_LINE}{}\n{GROUP_LINE}{}",
            self.info.t,
            hex(&self.info.thresh_pk.serialize())
        );
        for (i, share) in self.info.pubshares.iter().enumerate() {
            let Some(share) = share else { continue };
            out.push('\n');
            out.push_str(SHARE_LINE);
            out.push_str(&alloc::format!("{i} {}", hex(&share.serialize())));
        }
        out.push('\n');
        out.push_str(&self.descriptor_checksummed());
        out
    }

    /// The wallet's fingerprint: the synthetic xpub's own, which is what
    /// a coordinator writes beside the wallet's keys.
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint(self.xpub.fingerprint().to_bytes())
    }

    /// Participant `i`'s fingerprint: hash160 of its public share, first
    /// four bytes, which is the form a master fingerprint has and is
    /// printed beside the share's words.
    pub fn share_fingerprint(&self, i: usize) -> Option<Fingerprint> {
        Some(share_fingerprint(&(*self.info.pubshares.get(i)?)?))
    }
}

/// A public share's fingerprint: hash160 of the compressed point, first
/// four bytes. This is the form a master fingerprint has, and it is what
/// the device prints beside a share's words.
pub fn share_fingerprint(share: &PublicKey) -> Fingerprint {
    let hash = hash160::Hash::hash(&share.serialize());
    let bytes = hash.to_byte_array();
    Fingerprint([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// One compressed public key in lower-case hex.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

/// A key the record states: 33 bytes of lower-case hex that are a point
/// on the curve.
fn point(text: &str) -> Result<PublicKey, Error> {
    let text = text.trim();
    if text.len() != 66 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Key);
    }
    let mut bytes = [0u8; 33];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| Error::Key)?;
    }
    PublicKey::from_slice(&bytes).map_err(|_| Error::Key)
}

/// The extended public key of the record's descriptor line, which is
/// `tr(XPUB/<0;1>/*)` with a checksum and no origin: the group key has
/// no master to name.
fn descriptor_xpub(text: &str) -> Result<Xpub, Error> {
    if !text.contains('#') {
        return Err(Error::Checksum);
    }
    let policy = WalletPolicy::from_descriptor(text)?;
    if policy.template()
        != (Template::Single {
            script: ScriptType::Taproot,
        })
    {
        return Err(Error::Template);
    }
    let key = policy.keys().first().ok_or(Error::Key)?;
    if key.fingerprint().is_some() {
        return Err(Error::Key);
    }
    Ok(*key.xpub())
}
