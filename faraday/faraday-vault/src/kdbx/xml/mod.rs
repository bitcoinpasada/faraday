//! The XML document of a KDBX 4 file (`docs/VAULT.md` §7, §7.1): read to a
//! model, with what the model does not read kept as the bytes it came in,
//! and written again.
//!
//! What the model reads: `Meta/DatabaseName` and the database's custom data,
//! each group's UUID, name, custom data, groups and entries, each entry's
//! UUID, fields, custom data and last-modification time. Everything else
//! (attachment references, icons, other times, tags, AutoType, colours,
//! group settings, deleted-object records, the rest of `Meta`, custom data
//! whose key does not begin `faraday.`) is carried through: kept as the bytes
//! read, in its place, and written back there. An element the model reads and
//! the caller did not change is written back as it was read; one changed is
//! written in its plain form.
//!
//! **The inner stream**, as KeePassXC's `KdbxXmlReader` (`readString`) has
//! it. An element is protected when its `Protected` attribute, after
//! character references are resolved, is `True` in any case or `1`, and it
//! holds text only (a protected attribute on an element with child elements
//! is not one). Every such element is decrypted in document order on
//! reading, whatever its name (`Name`, `Key`, `UUID`, times and the rest,
//! not only a field's `Value`), the ones in carried bytes, History and
//! recycle bin included, so the stream stays in step. A
//! protected plaintext that is not UTF-8 is damage ([`Refused::NotText`]).
//! On writing, the elements the model reads are written unprotected, except
//! a field's `Value` flagged protected and a field whose `Key` was
//! protected and which was not changed; a carried protected element is
//! written protected again. Every one written is encrypted in document order
//! from its plaintext under the key given, so the stream is right whatever
//! was removed or changed; nothing of the ciphertext read is reused.
//! Everything read from the document that a person wrote or that is
//! protected is held in wiping buffers: the protected plaintexts, the text
//! of every leaf (names, keys, values, notes), the bytes carried as read
//! (History and the recycle bin included), and the buffers they grow in, so
//! a buffer that grows moves to a new wiping one and leaves no copy. What
//! is not wiped is the document's structure (tags, attributes, times,
//! UUIDs), which is not secret, and the XML text it is read from, which its
//! caller holds in a wiping buffer.
//!
//! **Kept, out of the model:** entry `History`, and the recycle bin unless
//! `Meta/RecycleBinEnabled` is present and not `True`, as KeePassXC has it
//! (the group `Meta`'s `RecycleBinUUID` names, with its entries; otherwise
//! that group is an ordinary one; a `Meta` with two `RecycleBinUUID`
//! elements keeps both groups out). They are read through, so the stream
//! stays in step and the protected values in them are held as secrets, and
//! carried as the bytes read, in their place: not a group or an entry of the
//! model, so nothing lists, searches, counts or loads them, an entry
//! removed takes its history with it, an edit adds none, and the writer
//! leaves `RecycleBinUUID` and `RecycleBinEnabled` as they were read. They
//! are counted ([`Document::history_entries`],
//! [`Document::recycle_bin_entries`]).
//!
//! **Marked, not refused.** What the model cannot place is read, kept and
//! written as it was, and marked: [`Entry::ambiguous`], [`Group::ambiguous`],
//! [`Document::ambiguous`] and [`CustomData::ambiguous`] say why (two UUIDs,
//! two `Times`, two lists of custom data, a UUID that is not 16 bytes or
//! none, a field with two keys, two values, none or text between them, two
//! fields of one key ([`Entry::duplicate_keys`]), two custom data items with
//! one key, a `Meta` with two `DatabaseName`, `CustomData` or
//! `RecycleBinUUID`). Every edit of an ambiguous entry, group, `Meta` or list
//! refuses, with no change; the layer above shows them as unreadable. A
//! group or an entry whose UUID another group or entry has
//! ([`Group::duplicate_uuid`], [`Entry::duplicate_uuid`]) is marked too.
//!
//! **Written.** Characters XML 1.0 forbids (U+0000 to U+0008, U+000B, U+000C,
//! U+000E to U+001F, U+FFFE, U+FFFF) are left out of every name, label,
//! value and custom data item written in plain form, as KeePass does; a
//! protected value is encrypted as it is.
//!
//! A comment inside a text element is skipped, as KeePassXC's
//! `readElementText` does, and kept in the carried bytes.
//!
//! A comment or a processing instruction before the root element is kept in
//! the prolog as read, and after the closing root tag in the epilog (the
//! whitespace there is dropped); a processing instruction elsewhere is
//! carried as read, and in a text element the model reads it is skipped like
//! a comment. In text,
//! CR LF and a lone CR are read as one LF (XML 1.0 §2.11); a character
//! reference to CR stays CR.
//!
//! **Refused**, naming what was found ([`Refused`]): a document not well
//! formed (a forbidden character raw, a character reference that is not
//! `&#` and decimal digits or `&#x` and hex digits for an allowed
//! character), a DOCTYPE, an entity other than XML's five and character
//! references, an XML declaration that is not at the start, anything but
//! XML whitespace, comments and processing instructions after the closing
//! root tag, nesting past
//! [`MAX_DEPTH`], a protected value that is not canonical Base64 of text, a
//! `Meta` or `Root` that is missing, repeated or out of place, and markup
//! inside a text element the model reads.
//!
//! The work is linear in the document: each byte is read once by the
//! tokenizer, nesting is capped, and an element takes at least four bytes.

use alloc::boxed::Box;
use alloc::vec::Vec;

use base64ct::{Base64, Encoding};
use zeroize::Zeroizing;

use super::InnerStream;

mod read;
mod refs;

pub use refs::referenced;

/// The deepest nesting of elements a document may have.
pub const MAX_DEPTH: usize = 64;

/// The most memory, in bytes, the model's structures may take for each
/// byte of the document (a format limit like `MAX_BINARIES`, named in
/// `docs/VAULT.md` §4): the reader keeps a running charge and refuses with
/// [`Refused::TooLarge`] once it passes this times the document's length
/// plus [`MODEL_BASE`].
///
/// What is charged is what the reader holds for each element it keeps: its
/// slot in the parent's list (a list grows by doubling, so twice), the box
/// the element is in, the list of its own parts at its capacity, its tags,
/// and for each stretch of carried bytes twice a piece and a small
/// allocation. The text itself is not charged: it is copied at most a few
/// times (a pair keeps its key and value once read and once as written), a
/// multiple of the length the 4 x file of F206 point 4 allows for.
///
/// The charge is held to the memory really taken by
/// `tests/reader_memory.rs`, which counts the allocations: never less, for
/// empty groups and entries, fields, items and times, and for the shapes of
/// real files. Measured (64-bit): a bare `<Entry/>` of 8 bytes holds 92
/// bytes a byte and is charged 120, a bare `<Group/>` holds 66 and is
/// charged 88: what this limit refuses past its base. A group with an
/// attribute (`<Group a="xxxxxxxxxx"/>`) holds 24 a byte and is charged 32.
/// A real entry of a KeePass app (a UUID, times, five strings, one
/// protected, each element on its own line, about 1,000 bytes) holds 5.6 a
/// byte and is charged 8.5; a flat list of KeePassXC's empty groups holds
/// 3.2 and is charged 5.3. The limit is 40, more than four times that, so a
/// real file is not refused. A crafted file is held to 40 bytes of
/// structures for each byte (the charge is never less than what is held).
pub const MODEL_PER_BYTE: usize = 40;

/// What a small document may take besides [`MODEL_PER_BYTE`] of its
/// length: 1 MiB.
pub const MODEL_BASE: usize = 1 << 20;

/// Seconds from 0001-01-01 to the Unix epoch: KDBX 4 times count from the
/// former.
const EPOCH_GAP: i64 = 62_135_596_800;
// Element tags, as the opening tags are written. Names are compared by
// `is`.
const T_FILE: &[u8] = b"<KeePassFile>";
const T_META: &[u8] = b"<Meta>";
const T_ROOT: &[u8] = b"<Root>";
const T_GROUP: &[u8] = b"<Group>";
const T_ENTRY: &[u8] = b"<Entry>";
const T_UUID: &[u8] = b"<UUID>";
const T_NAME: &[u8] = b"<Name>";
const T_DBNAME: &[u8] = b"<DatabaseName>";
const T_BIN: &[u8] = b"<RecycleBinUUID>";
const T_BINEN: &[u8] = b"<RecycleBinEnabled>";
const T_CUSTOM: &[u8] = b"<CustomData>";
const T_ITEM: &[u8] = b"<Item>";
const T_FIELD: &[u8] = b"<String>";
const T_KEY: &[u8] = b"<Key>";
const T_VALUE: &[u8] = b"<Value>";
const T_TIMES: &[u8] = b"<Times>";
const T_MODIFIED: &[u8] = b"<LastModificationTime>";
const T_HISTORY: &[u8] = b"<History>";

/// Whether `name` (between `&` and `;`) is one of XML's five references or
/// a character reference to a character XML allows.
pub(crate) fn valid_ref(name: &[u8]) -> bool {
    read::decode_ref(name).is_ok()
}

/// What a reader refuses, naming what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
pub enum Refused {
    /// Not well formed XML.
    NotWellFormed,
    /// Not UTF-8.
    NotUtf8,
    /// A declared encoding other than UTF-8.
    Encoding(Vec<u8>),
    /// A DOCTYPE.
    DocType,
    /// An entity other than XML's five and character references: its name.
    Entity(Vec<u8>),
    /// An XML declaration anywhere but the start (as a processing
    /// instruction, its target `xml`).
    ProcessingInstruction(Vec<u8>),
    /// Something but XML whitespace, comments and processing instructions
    /// after the closing root tag.
    AfterRoot,
    /// Elements nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// A protected value that is not canonical Base64 (where).
    Base64(&'static str),
    /// A protected value whose plaintext is not UTF-8: damage.
    NotText,
    /// A UUID that is not 16 bytes.
    Uuid,
    /// A structure the model cannot place (what).
    Structure(&'static str),
    /// A document whose elements would take more memory to hold than
    /// [`MODEL_PER_BYTE`] allows for its length.
    TooLarge,
    /// The reader's charge passed the memory the caller allows it: the
    /// bytes charged when it stopped.
    Memory(usize),
}

type R<T> = Result<T, Refused>;

fn is(name: &[u8], tag: &[u8]) -> bool {
    tag.len() >= 2 && name == &tag[1..tag.len() - 1]
}

/// A character XML 1.0 does not allow in a document.
fn forbidden(c: char) -> bool {
    matches!(c, '\0'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

fn is_ws(b: &u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

// ---------------------------------------------------------------- bytes

/// Makes room for `extra` more bytes in a wiping buffer: when it has none,
/// the bytes move to a larger wiping buffer and the old one is wiped as it
/// drops, so growing leaves no copy behind.
fn room(buf: &mut Zeroizing<Vec<u8>>, extra: usize) {
    if buf.capacity() - buf.len() < extra {
        let want = (buf.len() + extra).max(buf.capacity().saturating_mul(2));
        let mut grown = Zeroizing::new(Vec::with_capacity(want));
        grown.extend_from_slice(buf);
        *buf = grown;
    }
}

/// A copy of a wiping buffer, in a wiping buffer.
fn dup(b: &Zeroizing<Vec<u8>>) -> Zeroizing<Vec<u8>> {
    crate::wiped_copy(b)
}

/// A piece of carried bytes.
enum Piece {
    Bytes(Zeroizing<Vec<u8>>),
    /// A protected value's plaintext, encrypted when written.
    Secret(Zeroizing<Vec<u8>>),
}

/// Bytes kept as read, with the protected values in them.
#[derive(Default)]
struct Carried(Vec<Piece>);

impl Piece {
    fn duplicate(&self) -> Self {
        match self {
            Self::Bytes(b) => Self::Bytes(dup(b)),
            Self::Secret(b) => Self::Secret(dup(b)),
        }
    }
}

impl Carried {
    fn duplicate(&self) -> Self {
        Self(self.0.iter().map(Piece::duplicate).collect())
    }

    fn bytes(&mut self, b: &[u8]) {
        if b.is_empty() {
            return;
        }
        if let Some(Piece::Bytes(last)) = self.0.last_mut() {
            room(last, b.len());
            last.extend_from_slice(b);
        } else {
            self.0.push(Piece::Bytes(crate::wiped_copy(b)));
        }
    }

    fn secret(&mut self, plain: Zeroizing<Vec<u8>>) {
        self.0.push(Piece::Secret(plain));
    }

    /// Whether anything but XML whitespace was kept.
    fn has_content(&self) -> bool {
        self.0.iter().any(|p| match p {
            Piece::Bytes(b) => b.iter().any(|c| !c.is_ascii_whitespace()),
            Piece::Secret(_) => true,
        })
    }

    fn append(&mut self, other: Carried) {
        for piece in other.0 {
            match piece {
                Piece::Bytes(b) => self.bytes(&b),
                secret => self.0.push(secret),
            }
        }
    }

    fn write(&self, w: &mut W<'_>) {
        for piece in &self.0 {
            match piece {
                Piece::Bytes(b) => w.out.extend_from_slice(b),
                Piece::Secret(plain) => w.secret(plain),
            }
        }
    }
}

/// An element with text the model reads: its bytes as read, and the text.
struct Leaf {
    orig: Carried,
    text: Zeroizing<Vec<u8>>,
    /// Read from a protected element: written in plain form.
    protected: bool,
}

impl Leaf {
    fn duplicate(&self) -> Self {
        Self {
            orig: self.orig.duplicate(),
            text: dup(&self.text),
            protected: self.protected,
        }
    }
}

/// A copy of an optional leaf.
fn dup_leaf(l: &Option<Leaf>) -> Option<Leaf> {
    l.as_ref().map(Leaf::duplicate)
}

/// Where a write goes: a buffer, or a count of what a buffer would hold
/// (the same code path, so the two cannot differ).
enum Out {
    Buffer(Zeroizing<Vec<u8>>),
    Count(usize),
}

impl Out {
    fn extend_from_slice(&mut self, b: &[u8]) {
        match self {
            Self::Buffer(v) => v.extend_from_slice(b),
            Self::Count(n) => *n += b.len(),
        }
    }

    fn push(&mut self, b: u8) {
        self.extend_from_slice(&[b]);
    }

    fn len(&self) -> usize {
        match self {
            Self::Buffer(v) => v.len(),
            Self::Count(n) => *n,
        }
    }

    fn truncate(&mut self, len: usize) {
        match self {
            Self::Buffer(v) => v.truncate(len),
            Self::Count(n) => *n = (*n).min(len),
        }
    }
}

/// The writer's output and the inner stream it encrypts under.
struct W<'s> {
    out: Out,
    stream: &'s mut InnerStream,
}

impl W<'_> {
    fn raw(&mut self, b: &[u8]) {
        self.out.extend_from_slice(b);
    }

    fn close(&mut self, tag: &[u8]) {
        self.out.extend_from_slice(b"</");
        self.out.extend_from_slice(&tag[1..]);
    }

    /// Text in plain form: markup escaped, the characters XML 1.0 forbids
    /// left out.
    fn escaped(&mut self, text: &[u8]) {
        // The text is UTF-8 whatever its source (the document, a `&str`).
        let text = core::str::from_utf8(text).unwrap_or("");
        let mut utf8 = [0u8; 4];
        for ch in text.chars() {
            match ch {
                '&' => self.raw(b"&amp;"),
                '<' => self.raw(b"&lt;"),
                '>' => self.raw(b"&gt;"),
                '\r' => self.raw(b"&#13;"),
                c if forbidden(c) => {}
                c => self.raw(c.encode_utf8(&mut utf8).as_bytes()),
            }
        }
    }

    fn secret(&mut self, plain: &[u8]) {
        if let Out::Count(n) = &mut self.out {
            // The Base64 of the encrypted value is as long as that of the
            // plain one; the stream is not needed for a length.
            *n += Base64::encoded_len(plain);
            return;
        }
        let mut ct = crate::wiped_copy(plain);
        self.stream.apply(&mut ct);
        let text = Base64::encode_string(&ct);
        self.raw(text.as_bytes());
    }

    /// A leaf: as read when its value is as read, else in plain form; none
    /// when there was none and the value is empty.
    fn leaf(&mut self, tag: &[u8], leaf: &Option<Leaf>, value: &[u8]) {
        match leaf {
            Some(l) if !l.protected && l.text[..] == *value => l.orig.write(self),
            None if value.is_empty() => {}
            _ => {
                self.raw(tag);
                self.escaped(value);
                self.close(tag);
            }
        }
    }
}

/// An element with parts: its opening and closing tags as read.
struct Frame<P> {
    tag: &'static [u8],
    open: Zeroizing<Vec<u8>>,
    /// None: written self-closed while it has no content.
    close: Option<Zeroizing<Vec<u8>>>,
    /// Not in the document: written only with content.
    absent: bool,
    parts: Vec<P>,
}

trait Part: Sized {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    fn duplicate(&self) -> Self;
    fn carried(c: Carried) -> Self;
    fn tail(&mut self) -> Option<&mut Carried>;
}

impl<P: Part> Frame<P> {
    fn duplicate(&self) -> Self {
        Self {
            tag: self.tag,
            open: dup(&self.open),
            close: self.close.as_ref().map(dup),
            absent: self.absent,
            parts: self.parts.iter().map(P::duplicate).collect(),
        }
    }

    fn read(tag: &'static [u8], open: &[u8], selfclosed: bool) -> Self {
        Self {
            tag,
            open: crate::wiped_copy(open),
            close: if selfclosed {
                None
            } else {
                Some(Zeroizing::new(Vec::new()))
            },
            absent: false,
            parts: Vec::new(),
        }
    }

    /// A frame the document did not have.
    #[expect(clippy::disallowed_methods, reason = "a constant tag name")]
    fn absent(tag: &'static [u8]) -> Self {
        let mut open = tag[..tag.len() - 1].to_vec();
        open.extend_from_slice(b"/>");
        Self {
            tag,
            open: Zeroizing::new(open),
            close: None,
            absent: true,
            parts: Vec::new(),
        }
    }

    /// A new frame, with content.
    #[expect(clippy::disallowed_methods, reason = "a constant tag name")]
    fn fresh(tag: &'static [u8]) -> Self {
        let mut close = b"</".to_vec();
        close.extend_from_slice(&tag[1..]);
        Self {
            tag,
            open: crate::wiped_copy(tag),
            close: Some(Zeroizing::new(close)),
            absent: false,
            parts: Vec::new(),
        }
    }

    fn bytes(&mut self, b: &[u8]) {
        let mut c = Carried::default();
        c.bytes(b);
        self.carry(c);
    }

    fn carry(&mut self, c: Carried) {
        if c.0.is_empty() {
            return;
        }
        if let Some(last) = self.parts.last_mut().and_then(Part::tail) {
            last.append(c);
        } else {
            self.parts.push(P::carried(c));
        }
    }

    fn write(&self, w: &mut W<'_>, mut each: impl FnMut(&P, &mut W<'_>)) {
        let mark = w.out.len();
        match &self.close {
            Some(close) => {
                w.raw(&self.open);
                for p in &self.parts {
                    each(p, w);
                }
                w.raw(close);
            }
            None => {
                // Self-closed as read: opened only if something is written.
                let open = self.open.strip_suffix(b"/>").unwrap_or(&self.open[..]);
                let open = open.trim_ascii_end();
                w.raw(open);
                w.out.push(b'>');
                let after = w.out.len();
                for p in &self.parts {
                    each(p, w);
                }
                if w.out.len() == after {
                    w.out.truncate(mark);
                    if !self.absent {
                        w.raw(&self.open);
                    }
                } else {
                    w.close(self.tag);
                }
            }
        }
    }
}

// ----------------------------------------------------------- the model

/// A key and a value: an entry's field, or an item of custom data.
pub struct Pair {
    key: Zeroizing<Vec<u8>>,
    value: Zeroizing<Vec<u8>>,
    protected: bool,
    /// Why the field cannot be told apart from what it was read as.
    ambiguous: Option<&'static str>,
    orig: Option<Orig>,
}

/// What a pair was when read.
struct Orig {
    carried: Carried,
    key: Zeroizing<Vec<u8>>,
    value: Zeroizing<Vec<u8>>,
    protected: bool,
}

impl Orig {
    fn duplicate(&self) -> Self {
        Self {
            carried: self.carried.duplicate(),
            key: dup(&self.key),
            value: dup(&self.value),
            protected: self.protected,
        }
    }
}

impl Pair {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        Self {
            key: dup(&self.key),
            value: dup(&self.value),
            protected: self.protected,
            ambiguous: self.ambiguous,
            orig: self.orig.as_ref().map(Orig::duplicate),
        }
    }

    /// A new pair.
    pub fn new(key: &str, value: &str, protected: bool) -> Self {
        Self {
            key: crate::wiped_copy(key.as_bytes()),
            value: crate::wiped_copy(value.as_bytes()),
            protected,
            ambiguous: None,
            orig: None,
        }
    }

    /// The key.
    pub fn key(&self) -> &str {
        core::str::from_utf8(&self.key).unwrap_or("")
    }

    /// The value.
    pub fn value(&self) -> &str {
        core::str::from_utf8(&self.value).unwrap_or("")
    }

    /// Whether the value is protected.
    pub fn protected(&self) -> bool {
        self.protected
    }

    /// Replaces the value and its protection.
    pub fn set(&mut self, value: &str, protected: bool) {
        self.value = crate::wiped_copy(value.as_bytes());
        self.protected = protected;
    }

    fn write(&self, w: &mut W<'_>, tag: &[u8]) {
        if let Some(o) = &self.orig
            && o.key == self.key
            && o.protected == self.protected
            && o.value[..] == self.value[..]
        {
            o.carried.write(w);
            return;
        }
        w.raw(tag);
        w.raw(T_KEY);
        w.escaped(&self.key);
        w.close(T_KEY);
        if self.protected {
            w.raw(b"<Value Protected=\"True\">");
            w.secret(&self.value);
        } else {
            w.raw(T_VALUE);
            w.escaped(&self.value);
        }
        w.close(T_VALUE);
        w.close(tag);
    }
}

enum CdPart {
    Carried(Carried),
    Item(Pair),
}

impl Part for CdPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Item(p) => Self::Item(p.duplicate()),
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            Self::Item(_) => None,
        }
    }
}

/// A list of custom data: Faraday's items, with the others kept, not read.
pub struct CustomData {
    frame: Frame<CdPart>,
    /// Why it is not edited: it, or what holds it, is ambiguous.
    ambiguous: Option<&'static str>,
}

impl CustomData {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        Self {
            frame: self.frame.duplicate(),
            ambiguous: self.ambiguous,
        }
    }

    fn none() -> Self {
        Self {
            frame: Frame::absent(T_CUSTOM),
            ambiguous: None,
        }
    }

    fn faraday_items(&self) -> impl Iterator<Item = &Pair> {
        self.frame.parts.iter().filter_map(|p| match p {
            CdPart::Item(pair) if pair.key.starts_with(b"faraday.") => Some(pair),
            _ => None,
        })
    }

    /// Faraday's items, in order.
    pub fn items(&self) -> impl Iterator<Item = (&str, &str)> {
        self.faraday_items().map(|p| (p.key(), p.value()))
    }

    /// The value of a Faraday key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.faraday_items()
            .find(|p| p.key[..] == *key.as_bytes())
            .map(Pair::value)
    }

    /// Sets a Faraday key; any other key is refused.
    pub fn set(&mut self, key: &str, value: &str) -> R<()> {
        self.editable()?;
        if !key.starts_with("faraday.") {
            return Err(Refused::Structure(
                "a custom data key not beginning faraday.",
            ));
        }
        for p in &mut self.frame.parts {
            if let CdPart::Item(pair) = p
                && pair.key[..] == *key.as_bytes()
            {
                pair.set(value, false);
                return Ok(());
            }
        }
        self.frame
            .parts
            .push(CdPart::Item(Pair::new(key, value, false)));
        Ok(())
    }

    fn editable(&self) -> R<()> {
        self.ambiguous
            .map_or(Ok(()), |why| Err(Refused::Structure(why)))
    }

    /// Why this list of custom data is ambiguous (two items with one key, a
    /// field of an item that cannot be told apart, or what holds it
    /// ambiguous), if it is; `set` and `remove` refuse on it.
    pub fn ambiguous(&self) -> Option<&'static str> {
        self.ambiguous
    }

    /// Removes a Faraday key; whether it was there. An ambiguous list is
    /// refused, unchanged.
    pub fn remove(&mut self, key: &str) -> R<bool> {
        self.editable()?;
        let at = self.frame.parts.iter().position(|p| {
            matches!(p, CdPart::Item(pair) if pair.key.starts_with(b"faraday.") && pair.key[..] == *key.as_bytes())
        });
        Ok(at.map(|i| self.frame.parts.remove(i)).is_some())
    }

    fn write(&self, w: &mut W<'_>) {
        self.frame.write(w, |p, w| match p {
            CdPart::Carried(c) => c.write(w),
            CdPart::Item(pair) => pair.write(w, T_ITEM),
        });
    }
}

enum TimesPart {
    Carried(Carried),
    Modified,
}

impl Part for TimesPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Modified => Self::Modified,
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            Self::Modified => None,
        }
    }
}

enum EntryPart {
    Carried(Carried),
    Uuid,
    Field(Box<Pair>),
    Times(Box<Frame<TimesPart>>),
    Custom,
}

impl Part for EntryPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Uuid => Self::Uuid,
            Self::Field(p) => Self::Field(Box::new(p.duplicate())),
            Self::Times(f) => Self::Times(Box::new(f.duplicate())),
            Self::Custom => Self::Custom,
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            _ => None,
        }
    }
}

/// An entry.
pub struct Entry {
    frame: Frame<EntryPart>,
    uuid: [u8; 16],
    uuid_leaf: Option<Leaf>,
    custom: CustomData,
    modified: Option<i64>,
    mod_leaf: Option<Leaf>,
    dup_keys: Vec<Zeroizing<Vec<u8>>>,
    dup_uuid: bool,
    ambiguous: Option<&'static str>,
}

impl Entry {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        Self {
            frame: self.frame.duplicate(),
            uuid: self.uuid,
            uuid_leaf: dup_leaf(&self.uuid_leaf),
            custom: self.custom.duplicate(),
            modified: self.modified,
            mod_leaf: dup_leaf(&self.mod_leaf),
            dup_keys: self.dup_keys.iter().map(|k| crate::wiped_copy(k)).collect(),
            dup_uuid: self.dup_uuid,
            ambiguous: self.ambiguous,
        }
    }

    /// A new entry with this UUID and no fields.
    pub fn new(uuid: [u8; 16]) -> Self {
        let mut e = Self {
            frame: Frame::fresh(T_ENTRY),
            uuid,
            uuid_leaf: None,
            custom: CustomData::none(),
            modified: None,
            mod_leaf: None,
            dup_keys: Vec::new(),
            dup_uuid: false,
            ambiguous: None,
        };
        e.frame.parts.push(EntryPart::Uuid);
        e.finish();
        e
    }

    /// The markers every entry has, for what the document lacked.
    fn finish(&mut self) {
        let parts = &mut self.frame.parts;
        if !parts.iter().any(|p| matches!(p, EntryPart::Times(_))) {
            let mut times = Frame::absent(T_TIMES);
            times.parts.push(TimesPart::Modified);
            let at = parts
                .iter()
                .position(|p| matches!(p, EntryPart::Uuid))
                .map_or(0, |i| i + 1);
            parts.insert(at, EntryPart::Times(Box::new(times)));
        }
        if !parts.iter().any(|p| matches!(p, EntryPart::Custom)) {
            parts.push(EntryPart::Custom);
        }
    }

    /// The UUID.
    pub fn uuid(&self) -> [u8; 16] {
        self.uuid
    }

    /// The keys that two or more fields of this entry have, each once, in
    /// byte order. An entry with any is ambiguous: it is read
    /// and written as it was, and `set_field` and `remove_field` refuse on
    /// it.
    pub fn duplicate_keys(&self) -> impl Iterator<Item = &str> {
        self.dup_keys
            .iter()
            .map(|k| core::str::from_utf8(k).unwrap_or(""))
    }

    /// Whether another entry of the document has this entry's UUID.
    pub fn duplicate_uuid(&self) -> bool {
        self.dup_uuid
    }

    /// The fields, in order.
    pub fn fields(&self) -> impl Iterator<Item = &Pair> {
        self.frame.parts.iter().filter_map(|p| match p {
            EntryPart::Field(f) => Some(&**f),
            _ => None,
        })
    }

    /// The first field with this key.
    pub fn field(&self, key: &str) -> Option<&Pair> {
        self.fields().find(|f| f.key[..] == *key.as_bytes())
    }

    /// Sets the field with this key (the first one), or adds it after the
    /// last field. An entry with duplicate keys is refused, unchanged.
    pub fn set_field(&mut self, key: &str, value: &str, protected: bool) -> R<()> {
        self.editable()?;
        for p in &mut self.frame.parts {
            if let EntryPart::Field(f) = p
                && f.key[..] == *key.as_bytes()
            {
                f.set(value, protected);
                return Ok(());
            }
        }
        let at = self
            .frame
            .parts
            .iter()
            .rposition(|p| matches!(p, EntryPart::Field(_) | EntryPart::Times(_)))
            .map_or(0, |i| i + 1);
        self.frame.parts.insert(
            at,
            EntryPart::Field(Box::new(Pair::new(key, value, protected))),
        );
        Ok(())
    }

    /// Removes every field with this key; whether any was there. An entry
    /// with duplicate keys is refused, unchanged.
    pub fn remove_field(&mut self, key: &str) -> R<bool> {
        self.editable()?;
        let before = self.frame.parts.len();
        self.frame
            .parts
            .retain(|p| !matches!(p, EntryPart::Field(f) if f.key[..] == *key.as_bytes()));
        Ok(self.frame.parts.len() != before)
    }

    /// Why the entry cannot be told apart from what it was read as, if it
    /// cannot: two UUIDs or two `Times` or two lists of custom data, a
    /// UUID that is not 16 bytes or none, a field with two keys or two
    /// values, none or text between them, two fields of one key
    /// ([`Entry::duplicate_keys`]), two custom data items with one key. It
    /// is read and written as it was, and every edit refuses on it.
    pub fn ambiguous(&self) -> Option<&'static str> {
        self.ambiguous
    }

    fn editable(&self) -> R<()> {
        self.ambiguous
            .map_or(Ok(()), |why| Err(Refused::Structure(why)))
    }

    /// Whether the entry holds elements Faraday does not read and keeps as
    /// they were: attachments, tags, an icon, colours, AutoType and the
    /// like, and custom data whose key does not begin `faraday.`. Its
    /// times and its fields are not counted.
    pub fn carries_unread(&self) -> bool {
        self.frame
            .parts
            .iter()
            .any(|p| matches!(p, EntryPart::Carried(c) if c.has_content()))
            || self.custom.frame.parts.iter().any(|p| match p {
                CdPart::Carried(c) => c.has_content(),
                CdPart::Item(pair) => !pair.key.starts_with(b"faraday."),
            })
    }

    /// The entry's custom data.
    pub fn custom(&self) -> &CustomData {
        &self.custom
    }

    /// The entry's custom data, to change.
    pub fn custom_mut(&mut self) -> &mut CustomData {
        &mut self.custom
    }

    /// The last modification, in Unix seconds, when the document gives it in
    /// KDBX 4's form.
    pub fn modified(&self) -> Option<i64> {
        self.modified
    }

    /// Sets the last modification, written when the clock was set. An
    /// ambiguous entry is refused, unchanged.
    pub fn set_modified(&mut self, unix: i64) -> R<()> {
        self.editable()?;
        self.modified = Some(unix);
        Ok(())
    }

    fn write(&self, w: &mut W<'_>) {
        self.frame.write(w, |p, w| match p {
            EntryPart::Carried(c) => c.write(w),
            EntryPart::Uuid => w.leaf(
                T_UUID,
                &self.uuid_leaf,
                Base64::encode_string(&self.uuid).as_bytes(),
            ),
            EntryPart::Field(f) => f.write(w, T_FIELD),
            EntryPart::Times(t) => t.write(w, |tp, w| match tp {
                TimesPart::Carried(c) => c.write(w),
                TimesPart::Modified => {
                    let carried = match &self.mod_leaf {
                        Some(l) => {
                            #[expect(
                                clippy::disallowed_methods,
                                reason = "the text of a modification time, which is not secret"
                            )]
                            let text = l.text.to_vec();
                            Some(text)
                        }
                        None => None,
                    };
                    let value = match (self.modified, carried) {
                        (Some(t), _) => encode_time(t),
                        (None, Some(text)) => text,
                        (None, None) => Vec::new(),
                    };
                    w.leaf(T_MODIFIED, &self.mod_leaf, &value);
                }
            }),
            EntryPart::Custom => self.custom.write(w),
        });
    }
}

fn encode_time(unix: i64) -> Vec<u8> {
    let ticks = unix.saturating_add(EPOCH_GAP);
    Base64::encode_string(&ticks.to_le_bytes()).into_bytes()
}

fn decode_time(text: &[u8]) -> Option<i64> {
    let text = core::str::from_utf8(text).ok()?;
    let mut buf = [0u8; 8];
    let got = Base64::decode(text, &mut buf).ok()?;
    let ticks = i64::from_le_bytes(<[u8; 8]>::try_from(got).ok()?);
    ticks.checked_sub(EPOCH_GAP)
}

enum GroupPart {
    Carried(Carried),
    Uuid,
    Name,
    Custom,
    Group(Box<Group>),
    Entry(Box<Entry>),
}

impl Part for GroupPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Uuid => Self::Uuid,
            Self::Name => Self::Name,
            Self::Custom => Self::Custom,
            Self::Group(g) => Self::Group(Box::new(g.duplicate())),
            Self::Entry(e) => Self::Entry(Box::new(e.duplicate())),
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            _ => None,
        }
    }
}

/// A group.
pub struct Group {
    frame: Frame<GroupPart>,
    uuid: [u8; 16],
    uuid_leaf: Option<Leaf>,
    name: Zeroizing<Vec<u8>>,
    name_leaf: Option<Leaf>,
    custom: CustomData,
    dup_uuid: bool,
    ambiguous: Option<&'static str>,
}

impl Group {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        Self {
            frame: self.frame.duplicate(),
            uuid: self.uuid,
            uuid_leaf: dup_leaf(&self.uuid_leaf),
            name: dup(&self.name),
            name_leaf: dup_leaf(&self.name_leaf),
            custom: self.custom.duplicate(),
            dup_uuid: self.dup_uuid,
            ambiguous: self.ambiguous,
        }
    }

    /// A new group.
    pub fn new(uuid: [u8; 16], name: &str) -> Self {
        let mut g = Self {
            frame: Frame::fresh(T_GROUP),
            uuid,
            uuid_leaf: None,
            name: crate::wiped_copy(name.as_bytes()),
            name_leaf: None,
            custom: CustomData::none(),
            dup_uuid: false,
            ambiguous: None,
        };
        g.frame.parts.push(GroupPart::Uuid);
        g.finish();
        g
    }

    /// The markers every group has: a name, and custom data before the
    /// first child.
    fn finish(&mut self) {
        let parts = &mut self.frame.parts;
        if !parts.iter().any(|p| matches!(p, GroupPart::Name)) {
            let at = parts
                .iter()
                .position(|p| matches!(p, GroupPart::Uuid))
                .map_or(0, |i| i + 1);
            parts.insert(at, GroupPart::Name);
        }
        if !parts.iter().any(|p| matches!(p, GroupPart::Custom)) {
            let at = parts
                .iter()
                .position(|p| matches!(p, GroupPart::Group(_) | GroupPart::Entry(_)))
                .unwrap_or(parts.len());
            parts.insert(at, GroupPart::Custom);
        }
    }

    /// The UUID.
    pub fn uuid(&self) -> [u8; 16] {
        self.uuid
    }

    /// Whether another group of the document has this group's UUID.
    pub fn duplicate_uuid(&self) -> bool {
        self.dup_uuid
    }

    /// The name.
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name).unwrap_or("")
    }

    /// Why the group cannot be told apart from what it was read as, if it
    /// cannot: two UUIDs, two names or two lists of custom data, a UUID
    /// that is not 16 bytes or none, two custom data items with one key.
    /// It is read and written as it was, and every edit refuses on it.
    pub fn ambiguous(&self) -> Option<&'static str> {
        self.ambiguous
    }

    fn editable(&self) -> R<()> {
        self.ambiguous
            .map_or(Ok(()), |why| Err(Refused::Structure(why)))
    }

    /// Renames the group. An ambiguous group is refused, unchanged.
    pub fn set_name(&mut self, name: &str) -> R<()> {
        self.editable()?;
        self.name = crate::wiped_copy(name.as_bytes());
        Ok(())
    }

    /// The group's custom data.
    pub fn custom(&self) -> &CustomData {
        &self.custom
    }

    /// The group's custom data, to change.
    pub fn custom_mut(&mut self) -> &mut CustomData {
        &mut self.custom
    }

    /// The groups directly under this one.
    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.frame.parts.iter().filter_map(|p| match p {
            GroupPart::Group(g) => Some(&**g),
            _ => None,
        })
    }

    /// The groups directly under this one, to change.
    pub fn groups_mut(&mut self) -> impl Iterator<Item = &mut Group> {
        self.frame.parts.iter_mut().filter_map(|p| match p {
            GroupPart::Group(g) => Some(&mut **g),
            _ => None,
        })
    }

    /// The entries directly in this group.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.frame.parts.iter().filter_map(|p| match p {
            GroupPart::Entry(e) => Some(&**e),
            _ => None,
        })
    }

    /// The entries directly in this group, to change.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        self.frame.parts.iter_mut().filter_map(|p| match p {
            GroupPart::Entry(e) => Some(&mut **e),
            _ => None,
        })
    }

    /// Adds an entry at the end.
    pub fn push_entry(&mut self, entry: Entry) -> R<()> {
        self.editable()?;
        self.frame.parts.push(GroupPart::Entry(Box::new(entry)));
        Ok(())
    }

    /// Adds a group at the end.
    pub fn push_group(&mut self, group: Group) -> R<()> {
        self.editable()?;
        self.frame.parts.push(GroupPart::Group(Box::new(group)));
        Ok(())
    }

    /// Removes the entry with this UUID from this group; whether it was
    /// there.
    pub fn remove_entry(&mut self, uuid: &[u8; 16]) -> R<bool> {
        self.editable()?;
        let at = self
            .frame
            .parts
            .iter()
            .position(|p| matches!(p, GroupPart::Entry(e) if e.uuid == *uuid));
        Ok(at.map(|i| self.frame.parts.remove(i)).is_some())
    }

    /// How many entries are in this group and under it.
    pub fn entry_count(&self) -> usize {
        self.entries().count() + self.groups().map(Group::entry_count).sum::<usize>()
    }

    fn write(&self, w: &mut W<'_>) {
        self.frame.write(w, |p, w| match p {
            GroupPart::Carried(c) => c.write(w),
            GroupPart::Uuid => w.leaf(
                T_UUID,
                &self.uuid_leaf,
                Base64::encode_string(&self.uuid).as_bytes(),
            ),
            GroupPart::Name => w.leaf(T_NAME, &self.name_leaf, &self.name),
            GroupPart::Custom => self.custom.write(w),
            GroupPart::Group(g) => g.write(w),
            GroupPart::Entry(e) => e.write(w),
        });
    }
}

enum MetaPart {
    Carried(Carried),
    Name,
    Custom,
}

impl Part for MetaPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Name => Self::Name,
            Self::Custom => Self::Custom,
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            _ => None,
        }
    }
}

struct Meta {
    frame: Frame<MetaPart>,
    label: Vec<u8>,
    label_leaf: Option<Leaf>,
    custom: CustomData,
    ambiguous: Option<&'static str>,
}

impl Meta {
    fn duplicate(&self) -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "the vault label, a name and not a secret"
        )]
        let label = self.label.to_vec();
        Self {
            frame: self.frame.duplicate(),
            label,
            label_leaf: dup_leaf(&self.label_leaf),
            custom: self.custom.duplicate(),
            ambiguous: self.ambiguous,
        }
    }

    fn write(&self, w: &mut W<'_>) {
        self.frame.write(w, |p, w| match p {
            MetaPart::Carried(c) => c.write(w),
            MetaPart::Name => w.leaf(T_DBNAME, &self.label_leaf, &self.label),
            MetaPart::Custom => self.custom.write(w),
        });
    }
}

enum RootPart {
    Carried(Carried),
    Group(Box<Group>),
}

impl Part for RootPart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Group(g) => Self::Group(Box::new(g.duplicate())),
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            Self::Group(_) => None,
        }
    }
}

enum FilePart {
    Carried(Carried),
    Meta(Box<Meta>),
    Root(Frame<RootPart>),
}

impl Part for FilePart {
    fn duplicate(&self) -> Self {
        match self {
            Self::Carried(c) => Self::Carried(c.duplicate()),
            Self::Meta(m) => Self::Meta(Box::new(m.duplicate())),
            Self::Root(f) => Self::Root(f.duplicate()),
        }
    }
    fn carried(c: Carried) -> Self {
        Self::Carried(c)
    }
    fn tail(&mut self) -> Option<&mut Carried> {
        match self {
            Self::Carried(c) => Some(c),
            _ => None,
        }
    }
}

/// A KDBX 4 XML document.
pub struct Document {
    prolog: Vec<u8>,
    epilog: Vec<u8>,
    file: Frame<FilePart>,
    history: usize,
    dropped: usize,
    /// What the reader charged for what it built ([`MODEL_PER_BYTE`]).
    charged: usize,
}

/// The reader's running charge for what it holds, and where it stops.
struct Meter {
    charged: usize,
    /// The most the format allows for the document's length.
    limit: usize,
    /// The most the caller allows, if it says.
    budget: Option<usize>,
}

impl Meter {
    /// Charges `n` bytes of what is about to be held, and refuses once the
    /// charge passes the budget ([`Refused::Memory`]) or the limit
    /// ([`Refused::TooLarge`]); no more is taken after.
    fn spend(&mut self, n: usize) -> R<()> {
        self.charged = self.charged.saturating_add(n);
        if let Some(budget) = self.budget
            && self.charged > budget
        {
            return Err(Refused::Memory(self.charged));
        }
        if self.charged > self.limit {
            return Err(Refused::TooLarge);
        }
        Ok(())
    }
}

/// The XML a new vault starts from.
const PROLOG: &[u8] = b"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n";

/// How many documents have been written in this process.
static WRITES: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// How many times a document has been written in this process, by
/// anything: a test that an edit writes none compares two readings. The
/// counter is process-wide, so such a test is exact only where it runs
/// alone in its process, as nextest runs each test.
pub fn writes() -> usize {
    WRITES.load(core::sync::atomic::Ordering::Relaxed)
}

impl Document {
    /// A second copy, its secret bytes in wiping buffers like the first's.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "the XML declaration before the root, which is not secret"
        )]
        let prolog = self.prolog.to_vec();
        #[expect(
            clippy::disallowed_methods,
            reason = "the text after the root, which is not secret"
        )]
        let epilog = self.epilog.to_vec();
        Self {
            prolog,
            epilog,
            file: self.file.duplicate(),
            history: self.history,
            dropped: self.dropped,
            charged: self.charged,
        }
    }

    /// The document Faraday makes for a new vault: the root group named as
    /// the vault with this UUID, and `Meta` with the generator and the
    /// label. No history, no recycle bin.
    #[expect(
        clippy::disallowed_methods,
        reason = "the constant prolog of the template"
    )]
    pub fn new(id: [u8; 16], name: &str) -> Self {
        let mut text = PROLOG.to_vec();
        text.extend_from_slice(
            b"<KeePassFile><Meta><Generator>Faraday</Generator><RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID></Meta><Root><Group><UUID>",
        );
        text.extend_from_slice(Base64::encode_string(&id).as_bytes());
        text.extend_from_slice(b"</UUID></Group></Root></KeePassFile>");
        let mut doc = Self::read(&text, &[0]).expect("the template is a document");
        doc.set_label(name).expect("the template is not ambiguous");
        doc.root_mut()
            .set_name(name)
            .expect("the template is not ambiguous");
        doc
    }

    /// Costs an encryption of the protected values and a copy of the
    /// document, linear in its size (at most the vault's largest, 128 MiB):
    /// writes it, the values encrypted in document order under
    /// `inner_key`. It has no padding; `seal` adds it.
    pub fn write(&self, inner_key: &[u8]) -> Zeroizing<Vec<u8>> {
        WRITES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        match self.written(Out::Buffer(Zeroizing::new(Vec::new())), inner_key) {
            Out::Buffer(v) => v,
            Out::Count(_) => unreachable!("a buffer stays a buffer"),
        }
    }

    /// Costs a walk of the document, linear in its size and keeping none
    /// of it: how many bytes [`Document::write`] gives, counted by the same
    /// code and not written.
    pub fn written_len(&self) -> usize {
        self.written(Out::Count(0), &[0]).len()
    }

    /// The document written into `out`.
    fn written(&self, out: Out, inner_key: &[u8]) -> Out {
        let mut stream = InnerStream::new(inner_key);
        let mut w = W {
            out,
            stream: &mut stream,
        };
        w.raw(&self.prolog);
        self.file.write(&mut w, |p, w| match p {
            FilePart::Carried(c) => c.write(w),
            FilePart::Meta(m) => m.write(w),
            FilePart::Root(r) => r.write(w, |rp, w| match rp {
                RootPart::Carried(c) => c.write(w),
                RootPart::Group(g) => g.write(w),
            }),
        });
        w.raw(&self.epilog);
        w.out
    }

    /// Marks the groups and the entries whose UUID another group or entry
    /// has.
    fn mark_duplicates(&mut self, meter: &mut Meter) -> R<()> {
        fn collect(
            g: &Group,
            groups: &mut Vec<[u8; 16]>,
            entries: &mut Vec<[u8; 16]>,
            meter: &mut Meter,
        ) -> R<()> {
            // Two lists that grow by doubling: 32 bytes for each UUID.
            meter.spend(32 * (1 + g.entries().count()))?;
            groups.push(g.uuid);
            entries.extend(g.entries().map(|e| e.uuid));
            for sub in g.groups() {
                collect(sub, groups, entries, meter)?;
            }
            Ok(())
        }
        fn repeated(mut all: Vec<[u8; 16]>) -> Vec<[u8; 16]> {
            all.sort_unstable();
            let mut out: Vec<[u8; 16]> = all
                .windows(2)
                .filter(|w| w[0] == w[1])
                .map(|w| w[0])
                .collect();
            out.dedup();
            out
        }
        fn mark(g: &mut Group, groups: &[[u8; 16]], entries: &[[u8; 16]]) {
            g.dup_uuid = groups.binary_search(&g.uuid).is_ok();
            for e in g.entries_mut() {
                e.dup_uuid = entries.binary_search(&e.uuid).is_ok();
            }
            for sub in g.groups_mut() {
                mark(sub, groups, entries);
            }
        }
        let (mut groups, mut entries) = (Vec::new(), Vec::new());
        collect(self.root(), &mut groups, &mut entries, meter)?;
        let (groups, entries) = (repeated(groups), repeated(entries));
        if !groups.is_empty() || !entries.is_empty() {
            mark(self.root_mut(), &groups, &entries);
        }
        Ok(())
    }

    /// How many entries have history, kept in the document.
    pub fn history_entries(&self) -> usize {
        self.history
    }

    /// How many entries were in the recycle bin, kept in the document.
    pub fn recycle_bin_entries(&self) -> usize {
        self.dropped
    }

    fn meta(&self) -> &Meta {
        self.file
            .parts
            .iter()
            .find_map(|p| match p {
                FilePart::Meta(m) => Some(&**m),
                _ => None,
            })
            .expect("a document has Meta")
    }

    fn meta_mut(&mut self) -> &mut Meta {
        self.file
            .parts
            .iter_mut()
            .find_map(|p| match p {
                FilePart::Meta(m) => Some(&mut **m),
                _ => None,
            })
            .expect("a document has Meta")
    }

    /// The vault's label: `Meta/DatabaseName`.
    pub fn label(&self) -> &str {
        core::str::from_utf8(&self.meta().label).unwrap_or("")
    }

    /// Sets the label. An ambiguous `Meta` is refused, unchanged.
    #[expect(
        clippy::disallowed_methods,
        reason = "the vault label, a name and not a secret"
    )]
    pub fn set_label(&mut self, label: &str) -> R<()> {
        let meta = self.meta_mut();
        if let Some(why) = meta.ambiguous {
            return Err(Refused::Structure(why));
        }
        meta.label = label.as_bytes().to_vec();
        Ok(())
    }

    /// Why the database's `Meta` cannot be told apart from what it was read
    /// as, if it cannot: two `DatabaseName`, `CustomData` or
    /// `RecycleBinUUID` elements, a `RecycleBinUUID` that is not 16 bytes,
    /// two custom data items with one key. Its custom data is not edited.
    pub fn ambiguous(&self) -> Option<&'static str> {
        self.meta().ambiguous
    }

    /// The database's custom data.
    pub fn custom(&self) -> &CustomData {
        &self.meta().custom
    }

    /// The database's custom data, to change.
    pub fn custom_mut(&mut self) -> &mut CustomData {
        &mut self.meta_mut().custom
    }

    /// The root group.
    pub fn root(&self) -> &Group {
        self.file
            .parts
            .iter()
            .find_map(|p| match p {
                FilePart::Root(r) => r.parts.iter().find_map(|rp| match rp {
                    RootPart::Group(g) => Some(&**g),
                    _ => None,
                }),
                _ => None,
            })
            .expect("a document has a root group")
    }

    /// The root group, to change.
    pub fn root_mut(&mut self) -> &mut Group {
        self.file
            .parts
            .iter_mut()
            .find_map(|p| match p {
                FilePart::Root(r) => r.parts.iter_mut().find_map(|rp| match rp {
                    RootPart::Group(g) => Some(&mut **g),
                    _ => None,
                }),
                _ => None,
            })
            .expect("a document has a root group")
    }

    /// The vault identifier: the root group's UUID as 32 lower-case hex
    /// digits.
    pub fn vault_id(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        faraday_crypto::hex::encode_lower(&self.root().uuid, &mut out);
        out
    }
}

/// Reads `document`, and when it reads, writes it and reads that again: the
/// two writes must agree, the first under a new inner key. For the fuzzer. Whether it was read.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_document(document: &[u8]) -> bool {
    const KEY: &[u8] = &[7; 64];
    // What is written is under another inner key, as a seal makes it: the
    // protected values then sit at other stream offsets than they were read.
    const NEW: &[u8] = &[9; 64];
    let Ok(doc) = Document::read(document, KEY) else {
        return false;
    };
    let first = doc.write(NEW);
    let again = Document::read(&first, NEW).expect("a written document reads");
    assert_eq!(&first[..], &again.write(NEW)[..]);
    // The attachment references of what is written are read.
    let _ = referenced(&first, 3);
    // The length counted without a write is the length written.
    assert_eq!(doc.written_len(), first.len());
    true
}
