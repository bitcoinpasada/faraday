//! The item and its layouts (`docs/AQRV.md` §3): entropy and a raw key
//! as the bytes, a text as `len` then the bytes, a codex32 string as
//! `codex32.rs` packs it.

use zeroize::Zeroizing;

use crate::{Kind, Layout, Refused, Size, codex32};

/// One item (§3). It has no `Debug`: it is the secret.
pub enum Item {
    /// BIP-39 entropy of 16, 20, 24, 28 or 32 bytes: the length gives the
    /// kind.
    Entropy(Zeroizing<Vec<u8>>),
    /// A codex32 string, exactly as written, in its case.
    Codex32(Zeroizing<String>),
    /// A BIP-39 passphrase, as given.
    Passphrase(Zeroizing<String>),
    /// A note, as given.
    Note(Zeroizing<String>),
    /// A raw 32-byte key.
    RawKey(Zeroizing<[u8; 32]>),
}

impl Item {
    /// The item's kind. None for entropy of a length no kind has.
    pub fn kind(&self) -> Option<Kind> {
        match self {
            Item::Entropy(bytes) => Kind::of_entropy_len(bytes.len()),
            Item::Codex32(_) => Some(Kind::Codex32),
            Item::Passphrase(_) => Some(Kind::Passphrase),
            Item::Note(_) => Some(Kind::Note),
            Item::RawKey(_) => Some(Kind::RawKey),
        }
    }
}

impl Kind {
    /// The kind of BIP-39 entropy of `n` bytes (§3).
    fn of_entropy_len(n: usize) -> Option<Kind> {
        Some(match n {
            16 => Kind::Words12,
            20 => Kind::Words15,
            24 => Kind::Words18,
            28 => Kind::Words21,
            32 => Kind::Words24,
            _ => return None,
        })
    }
}

/// An item as §3 lays it out, before the writer's draws fill the rest of
/// `P`.
pub(crate) struct Laid {
    pub kind: Kind,
    /// The item's bytes, its unused low bits zero.
    pub bytes: Zeroizing<Vec<u8>>,
    /// For codex32 only: the item's unused low bits in its last byte,
    /// which the writer fills from a random byte (§6 Writing step 1).
    pub unused_mask: Option<u8>,
}

/// §3: lays `item` out for `size`, or refuses it (§6 Writing step 1).
pub(crate) fn lay_out(item: &Item, size: Size) -> Result<Laid, Refused> {
    // A codex32 string is refused by its kind before it is packed.
    if matches!(item, Item::Codex32(_)) && !size.holds(Kind::Codex32) {
        return Err(Refused::KindNotHeld);
    }
    let (kind, bytes, unused_mask) = match item {
        Item::Entropy(bytes) => {
            let kind = Kind::of_entropy_len(bytes.len()).ok_or(Refused::EntropyLength)?;
            (kind, bytes.clone(), None)
        }
        Item::RawKey(key) => (Kind::RawKey, Zeroizing::new(key.to_vec()), None),
        Item::Passphrase(text) => (Kind::Passphrase, text_bytes(text, size)?, None),
        Item::Note(text) => (Kind::Note, text_bytes(text, size)?, None),
        Item::Codex32(string) => {
            let (bytes, mask) = codex32::pack(string, size)?;
            (Kind::Codex32, bytes, Some(mask))
        }
    };
    if !size.holds(kind) {
        return Err(Refused::KindNotHeld);
    }
    Ok(Laid {
        kind,
        bytes,
        unused_mask,
    })
}

/// §3: `len`, then the text's UTF-8 bytes, at most the room less one.
fn text_bytes(text: &str, size: Size) -> Result<Zeroizing<Vec<u8>>, Refused> {
    if text.is_empty() {
        return Err(Refused::EmptyText);
    }
    if text.len() > size.room() - 1 {
        return Err(Refused::TooLong);
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(1 + text.len()));
    bytes.push(text.len() as u8);
    bytes.extend_from_slice(text.as_bytes());
    Ok(bytes)
}

/// §6 Reading step 4: the item of `kind` in `p`, the room's bytes, or the
/// rule it fails. What follows the item in `p` is not read.
pub(crate) fn read(kind: Kind, p: &[u8]) -> Result<Item, Layout> {
    match kind {
        Kind::Words12 | Kind::Words15 | Kind::Words18 | Kind::Words21 | Kind::Words24 => {
            // For these five kinds the byte is also the entropy's length.
            let len = kind as usize;
            Ok(Item::Entropy(Zeroizing::new(p[..len].to_vec())))
        }
        Kind::RawKey => {
            let mut key = Zeroizing::new([0u8; 32]);
            key.copy_from_slice(&p[..32]);
            Ok(Item::RawKey(key))
        }
        Kind::Passphrase => text(p).map(Item::Passphrase),
        Kind::Note => text(p).map(Item::Note),
        Kind::Codex32 => codex32::unpack(p).map(Item::Codex32),
    }
}

/// §3: `len` in 1 to the room less one, then that many bytes of UTF-8.
fn text(p: &[u8]) -> Result<Zeroizing<String>, Layout> {
    let len = usize::from(p[0]);
    if len == 0 {
        return Err(Layout::LenZero);
    }
    if len > p.len() - 1 {
        return Err(Layout::LenPastRoom);
    }
    let text = core::str::from_utf8(&p[1..=len]).map_err(|_| Layout::NotUtf8)?;
    Ok(Zeroizing::new(text.to_owned()))
}

/// The fuzz target `aqrv_item`'s entry: [`read`] on a room of bytes, then
/// [`lay_out`] on what it read. Off by default; only the fuzz crate turns
/// the feature on, no app does.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_read(
    kind: Kind,
    size: Size,
    p: &[u8],
) -> Result<(Item, Result<(Kind, Vec<u8>, Option<u8>), Refused>), Layout> {
    let item = read(kind, p)?;
    let laid = lay_out(&item, size).map(|l| (l.kind, l.bytes.to_vec(), l.unused_mask));
    Ok((item, laid))
}
