//! Reading the XML document (`super`): the tokenizer's events checked and
//! made into the model, every byte of what the model does not read kept.

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;

use base64ct::{Base64, Encoding};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use zeroize::Zeroizing;

use super::*;

/// What a stretch of carried bytes costs besides its text: a piece in a
/// list that grows by doubling, and the allocation's own overhead.
const CARRIED: usize = 2 * size_of::<Piece>() + 16;

/// An allocation's own overhead.
const ALLOC: usize = 16;

/// What an entry holds besides its text: the box, its parts and, when the
/// document gave it no Times (`times` false), the frame `finish` adds.
fn entry_cost(en: &Entry, times: bool) -> usize {
    let mut held = size_of::<Entry>() + ALLOC + frame_cost(&en.frame);
    if !times {
        held += size_of::<Frame<TimesPart>>() + ALLOC + 4 * size_of::<TimesPart>() + 2 * ALLOC;
    }
    held
}

/// What a frame holds besides its parts' contents: the parts' list at its
/// capacity and the bytes of its tags.
fn frame_cost<P>(fr: &Frame<P>) -> usize {
    fr.parts.capacity() * size_of::<P>()
        + fr.open.capacity()
        + fr.close.as_ref().map_or(0, Vec::capacity)
        + 2 * ALLOC
}

impl Document {
    /// How much memory the reader charged for the structures it built,
    /// against the limit [`MODEL_PER_BYTE`] sets.
    pub fn charged(&self) -> usize {
        self.charged
    }

    /// Costs a parse of the document and a decryption of its protected
    /// values, linear in its size (at most the vault's largest, 128 MiB):
    /// reads it, decrypting them under the inner stream's key. XML whitespace after the root (the line feeds a vault
    /// is padded with among it) is accepted and not kept; a comment or a
    /// processing instruction before or after the root is kept.
    pub fn read(src: &[u8], inner_key: &[u8]) -> R<Self> {
        Self::read_within(src, inner_key, None)
    }

    /// Costs what [`Document::read`] costs, for a caller that says what
    /// memory the structures may take: the running charge stops at `budget`
    /// bytes with [`Refused::Memory`], taking no more, so a crafted file
    /// cannot pass what the system has free however its factor compares to
    /// [`MODEL_PER_BYTE`]. None: that limit alone. The text kept twice (a
    /// pair's key and value as read and as written), the duplicate keys
    /// and the lists that find duplicate UUIDs are charged as they grow.
    pub fn read_within(src: &[u8], inner_key: &[u8], budget: Option<usize>) -> R<Self> {
        let text = core::str::from_utf8(src).map_err(|_| Refused::NotUtf8)?;
        if text.chars().any(forbidden) {
            return Err(Refused::NotWellFormed);
        }
        // The reader does not count a byte order mark in its positions, so
        // the document is read without it and the prolog gets it back.
        const BOM: &[u8] = b"\xEF\xBB\xBF";
        let bom = if src.starts_with(BOM) { BOM } else { b"" };
        let src = &src[bom.len()..];
        // A second mark is counted by no one: refused, as before.
        if src.starts_with(BOM) {
            return Err(Refused::Structure("content before the root element"));
        }
        let mut rd = Rd {
            src,
            r: Reader::from_reader(src),
            pos: 0,
            depth: 0,
            stream: InnerStream::new(inner_key),
            bins: BTreeSet::new(),
            bin_enabled: true,
            skipped_entries: 0,
            history: 0,
            dropped: 0,
            meter: Meter {
                charged: 0,
                limit: src
                    .len()
                    .saturating_mul(MODEL_PER_BYTE)
                    .saturating_add(MODEL_BASE),
                budget,
            },
            meta_seen: false,
            root_seen: false,
        };
        let (a, b) = loop {
            let (ev, a, b) = rd.next()?;
            match &ev {
                Event::Decl(d) => {
                    if let Some(enc) = d.encoding() {
                        let enc = enc.map_err(|_| Refused::NotWellFormed)?;
                        if !enc.eq_ignore_ascii_case(b"utf-8") {
                            return Err(Refused::Encoding(enc.to_vec()));
                        }
                    }
                }
                Event::Text(t) if t.iter().all(is_ws) => {}
                // A comment or a processing instruction before the root is
                // kept in the prolog as read.
                Event::Comment(_) | Event::PI(_) => {}
                Event::Start(e) if is(e.name().as_ref(), T_FILE) => break (a, b),
                Event::Start(_) | Event::Empty(_) => {
                    return Err(Refused::Structure("the root element is not KeePassFile"));
                }
                _ => return Err(Refused::Structure("content before the root element")),
            }
        };
        let mut file = Frame::<FilePart>::read(T_FILE, &src[a..b], false);
        rd.children(&mut file, &mut |s, fr, e, a, b, empty| {
            let name = e.name();
            let name = name.as_ref();
            if is(name, T_META) {
                if s.meta_seen || s.root_seen {
                    return Err(Refused::Structure("Meta twice, or after Root"));
                }
                s.meta_seen = true;
                let m = s.meta(a, b, empty)?;
                fr.parts.push(FilePart::Meta(Box::new(m)));
                Ok(true)
            } else if is(name, T_ROOT) {
                if s.root_seen {
                    return Err(Refused::Structure("Root twice"));
                }
                s.root_seen = true;
                let r = s.root(a, b, empty)?;
                fr.parts.push(FilePart::Root(r));
                Ok(true)
            } else {
                Ok(false)
            }
        })?;
        if !rd.meta_seen || !rd.root_seen {
            return Err(Refused::Structure("a document without Meta or Root"));
        }
        let epilog = rd.epilog()?;
        let mut doc = Self {
            prolog: [bom, &src[..a]].concat(),
            epilog,
            file,
            history: rd.history,
            dropped: rd.dropped,
            charged: rd.meter.charged,
        };
        doc.mark_duplicates(&mut rd.meter)?;
        doc.charged = rd.meter.charged;
        Ok(doc)
    }
}

// ------------------------------------------------------------- reading

struct Rd<'a> {
    src: &'a [u8],
    r: Reader<&'a [u8]>,
    pos: usize,
    depth: usize,
    stream: InnerStream,
    /// The groups `Meta` names as the recycle bin (two `RecycleBinUUID`
    /// elements name two, and both are kept out of the model).
    bins: BTreeSet<[u8; 16]>,
    bin_enabled: bool,
    skipped_entries: usize,
    history: usize,
    dropped: usize,
    /// What the model built so far is charged at, and the most it may be.
    meter: Meter,
    meta_seen: bool,
    root_seen: bool,
}

struct Text {
    text: Vec<u8>,
    /// Where the end tag begins.
    end_at: usize,
    /// The comments in it, as read.
    comments: Vec<u8>,
}

type Handler<'a, 'h, P> =
    dyn FnMut(&mut Rd<'a>, &mut Frame<P>, &BytesStart<'a>, usize, usize, bool) -> R<bool> + 'h;

/// The character a reference names, if XML allows it.
fn decode_ref(name: &[u8]) -> R<char> {
    let bad = || Refused::Entity(name.to_vec());
    match name {
        b"amp" => Ok('&'),
        b"lt" => Ok('<'),
        b"gt" => Ok('>'),
        b"quot" => Ok('"'),
        b"apos" => Ok('\''),
        [b'#', rest @ ..] => {
            let (digits, radix) = match rest {
                [b'x', hex @ ..] => (hex, 16),
                dec => (dec, 10),
            };
            let digit = |c: &u8| {
                if radix == 16 {
                    c.is_ascii_hexdigit()
                } else {
                    c.is_ascii_digit()
                }
            };
            if digits.is_empty() || digits.len() > 8 || !digits.iter().all(digit) {
                return Err(bad());
            }
            let text = core::str::from_utf8(digits).map_err(|_| bad())?;
            let n = u32::from_str_radix(text, radix).map_err(|_| bad())?;
            let ok =
                matches!(n, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF);
            char::from_u32(n).filter(|_| ok).ok_or_else(bad)
        }
        _ => Err(bad()),
    }
}

/// Every reference in a tag's bytes is XML's five or a character.
fn check_refs(raw: &[u8]) -> R<()> {
    let mut at = 0;
    while let Some(p) = raw[at..].iter().position(|&c| c == b'&') {
        let from = at + p + 1;
        let len = raw[from..]
            .iter()
            .position(|&c| c == b';')
            .ok_or(Refused::NotWellFormed)?;
        decode_ref(&raw[from..from + len])?;
        at = from + len + 1;
    }
    Ok(())
}

impl<'a> Rd<'a> {
    /// What follows the root element: XML whitespace, dropped, and the
    /// comments and processing instructions, which are returned as read.
    fn epilog(&mut self) -> R<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let ev = self.r.read_event().map_err(|_| Refused::NotWellFormed)?;
            let a = self.pos;
            self.pos = self.r.buffer_position() as usize;
            match ev {
                Event::Eof => return Ok(out),
                Event::Text(t) if t.iter().all(is_ws) => {}
                Event::Comment(_) | Event::PI(_) => out.extend_from_slice(&self.src[a..self.pos]),
                _ => return Err(Refused::AfterRoot),
            }
        }
    }

    /// Charges `n` bytes of what the model is about to hold, and refuses
    /// the document once the charge passes its limit.
    fn spend(&mut self, n: usize) -> R<()> {
        self.meter.spend(n)
    }

    /// The next event and the bytes it spans.
    fn next(&mut self) -> R<(Event<'a>, usize, usize)> {
        let ev = self.r.read_event().map_err(|_| Refused::NotWellFormed)?;
        let a = self.pos;
        let b = self.r.buffer_position() as usize;
        self.pos = b;
        match &ev {
            Event::Start(_) => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(Refused::TooDeep);
                }
                check_refs(&self.src[a..b])?;
            }
            Event::Empty(_) => check_refs(&self.src[a..b])?,
            Event::End(_) => self.depth = self.depth.saturating_sub(1),
            Event::GeneralRef(r) => {
                decode_ref(r)?;
            }
            Event::DocType(_) => return Err(Refused::DocType),
            Event::Decl(_) if a != 0 => {
                return Err(Refused::ProcessingInstruction(b"xml".to_vec()));
            }
            Event::Eof => return Err(Refused::NotWellFormed),
            _ => {}
        }
        Ok((ev, a, b))
    }

    /// The text of an element whose start tag was just read, up to its end
    /// tag.
    fn read_text(&mut self) -> R<Text> {
        self.text_or_markup()?
            .ok_or(Refused::Structure("markup inside a text element"))
    }

    /// As `read_text`; none when the element holds markup.
    fn text_or_markup(&mut self) -> R<Option<Text>> {
        let mut text = Vec::new();
        let mut comments = Vec::new();
        loop {
            let (ev, a, _) = self.next()?;
            match ev {
                Event::Text(t) => push_normalised(&mut text, &t),
                Event::CData(c) => push_normalised(&mut text, &c),
                Event::GeneralRef(r) => {
                    let ch = decode_ref(&r)?;
                    let mut buf = [0u8; 4];
                    text.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                // As KeePassXC's readElementText: a comment is skipped, and
                // so is a processing instruction.
                Event::Comment(_) | Event::PI(_) => {
                    comments.extend_from_slice(&self.src[a..self.pos])
                }
                Event::End(_) => {
                    return Ok(Some(Text {
                        text,
                        end_at: a,
                        comments,
                    }));
                }
                _ => return Ok(None),
            }
        }
    }

    /// A protected value's plaintext from its Base64 text.
    fn open_secret(&mut self, text: &[u8]) -> R<Zeroizing<Vec<u8>>> {
        let s = core::str::from_utf8(text).map_err(|_| Refused::Base64("a protected value"))?;
        let ct = Base64::decode_vec(s).map_err(|_| Refused::Base64("a protected value"))?;
        let mut plain = Zeroizing::new(ct);
        self.stream.apply(&mut plain);
        // Not text: the stream is out of step, the file damaged.
        core::str::from_utf8(&plain).map_err(|_| Refused::NotText)?;
        Ok(plain)
    }

    /// A protected element from its start tag (`a..b`) to its end tag, into
    /// `c`; the plaintext.
    fn protected_value(&mut self, a: usize, b: usize, c: &mut Carried) -> R<Zeroizing<Vec<u8>>> {
        // The start tag is a piece of its own, with its bytes.
        self.spend(CARRIED + (b - a))?;
        c.bytes(&self.src[a..b]);
        let t = self.read_text()?;
        self.opened(t, c)
    }

    /// The plaintext of a protected element's text, kept in `c`.
    fn opened(&mut self, t: Text, c: &mut Carried) -> R<Zeroizing<Vec<u8>>> {
        // The secret is a piece, and the bytes after it another, each in a
        // list that grows by doubling; the bytes between are kept too.
        self.spend(2 * CARRIED + t.comments.len() + (self.pos - t.end_at))?;
        let plain = self.open_secret(&t.text)?;
        c.bytes(&t.comments);
        // The plaintext is kept twice: carried, and as the value.
        self.spend(plain.len())?;
        c.secret(plain.clone());
        c.bytes(&self.src[t.end_at..self.pos]);
        Ok(plain)
    }

    /// A protected element in carried bytes, from its start tag (`a..b`):
    /// decrypted and kept in `c` when it holds text only. When it holds
    /// markup the attribute means nothing, nothing is consumed, and false.
    fn carried_secret(&mut self, a: usize, b: usize, c: &mut Carried) -> R<bool> {
        let saved = (self.r.clone(), self.pos, self.depth);
        match self.text_or_markup()? {
            Some(t) => {
                self.spend(CARRIED + (b - a))?;
                c.bytes(&self.src[a..b]);
                self.opened(t, c)?;
                Ok(true)
            }
            None => {
                (self.r, self.pos, self.depth) = saved;
                Ok(false)
            }
        }
    }

    /// An element from its start tag to its end, kept in `c`; the protected
    /// values in it decrypted in order.
    fn skip_element(
        &mut self,
        e: &BytesStart<'a>,
        a: usize,
        b: usize,
        empty: bool,
        c: &mut Carried,
    ) -> R<()> {
        if empty {
            c.bytes(&self.src[a..b]);
            return Ok(());
        }
        if is_protected(e)? && self.carried_secret(a, b, c)? {
            return Ok(());
        }
        c.bytes(&self.src[a..b]);
        let mut level = 1usize;
        while level > 0 {
            let (ev, a, b) = self.next()?;
            match &ev {
                Event::Start(e2) => {
                    if is(e2.name().as_ref(), T_ENTRY) {
                        self.skipped_entries += 1;
                    }
                    if !(is_protected(e2)? && self.carried_secret(a, b, c)?) {
                        c.bytes(&self.src[a..b]);
                        level += 1;
                    }
                }
                Event::End(_) => {
                    c.bytes(&self.src[a..b]);
                    level -= 1;
                }
                _ => c.bytes(&self.src[a..b]),
            }
        }
        Ok(())
    }

    /// The children of an element, each offered to `f` and carried when it
    /// does not take it.
    fn children<P: Part>(&mut self, fr: &mut Frame<P>, f: &mut Handler<'a, '_, P>) -> R<()> {
        if fr.close.is_none() {
            return Ok(());
        }
        loop {
            let (ev, a, b) = self.next()?;
            match &ev {
                Event::Start(e) | Event::Empty(e) => {
                    let empty = matches!(ev, Event::Empty(_));
                    self.spend(2 * size_of::<P>() + CARRIED)?;
                    if !f(self, fr, e, a, b, empty)? {
                        let mut c = Carried::default();
                        self.skip_element(e, a, b, empty, &mut c)?;
                        fr.carry(c);
                    }
                }
                Event::End(_) => {
                    fr.close = Some(self.src[a..b].to_vec());
                    return Ok(());
                }
                _ => {
                    self.spend(2 * size_of::<P>() + CARRIED)?;
                    fr.bytes(&self.src[a..b]);
                }
            }
        }
    }

    /// An element with text the model reads, from its start tag `e`
    /// (`a..b`); a protected one is decrypted and kept protected.
    fn leaf(&mut self, e: &BytesStart<'a>, a: usize, b: usize, empty: bool) -> R<Leaf> {
        self.spend(CARRIED)?;
        let mut orig = Carried::default();
        if empty {
            orig.bytes(&self.src[a..b]);
            return Ok(Leaf {
                orig,
                text: Zeroizing::new(Vec::new()),
                protected: false,
            });
        }
        if is_protected(e)? {
            let text = self.protected_value(a, b, &mut orig)?;
            return Ok(Leaf {
                orig,
                text,
                protected: true,
            });
        }
        let t = self.read_text()?;
        orig.bytes(&self.src[a..self.pos]);
        Ok(Leaf {
            orig,
            text: Zeroizing::new(t.text),
            protected: false,
        })
    }

    /// A key and a value: a field of an entry or an `Item` of custom
    /// data.
    fn pair(&mut self, a: usize, b: usize, empty: bool) -> R<Pair> {
        self.spend(CARRIED)?;
        let mut c = Carried::default();
        c.bytes(&self.src[a..b]);
        let mut key: Option<Vec<u8>> = None;
        let mut value: Option<(Zeroizing<Vec<u8>>, bool)> = None;
        let mut ambiguous = None;
        if !empty {
            loop {
                let (ev, a, b) = self.next()?;
                match &ev {
                    Event::Start(e) | Event::Empty(e) => {
                        let empty = matches!(ev, Event::Empty(_));
                        let name = e.name();
                        let name = name.as_ref();
                        if is(name, T_KEY) {
                            if key.is_some() {
                                ambiguous = Some("a field with two keys");
                                self.skip_element(e, a, b, empty, &mut c)?;
                                continue;
                            }
                            let leaf = self.leaf(e, a, b, empty)?;
                            key = Some(leaf.text.to_vec());
                            c.append(leaf.orig);
                        } else if is(name, T_VALUE) {
                            if value.is_some() {
                                ambiguous = Some("a field with two values");
                                self.skip_element(e, a, b, empty, &mut c)?;
                                continue;
                            }
                            let protected = is_protected(e)?;
                            if empty {
                                c.bytes(&self.src[a..b]);
                                value = Some((Zeroizing::new(Vec::new()), protected));
                            } else if protected {
                                let plain = self.protected_value(a, b, &mut c)?;
                                value = Some((plain, true));
                            } else {
                                let t = self.read_text()?;
                                c.bytes(&self.src[a..self.pos]);
                                value = Some((Zeroizing::new(t.text), false));
                            }
                        } else {
                            self.skip_element(e, a, b, empty, &mut c)?;
                        }
                    }
                    Event::End(_) => {
                        c.bytes(&self.src[a..b]);
                        break;
                    }
                    Event::Text(t) if t.iter().all(is_ws) => c.bytes(&self.src[a..b]),
                    Event::Comment(_) | Event::PI(_) => c.bytes(&self.src[a..b]),
                    _ => {
                        ambiguous = Some("text inside a field");
                        c.bytes(&self.src[a..b]);
                    }
                }
            }
        }
        if key.is_none() {
            ambiguous = Some("a field without a key");
        }
        let key = key.unwrap_or_default();
        let (value, protected) = value.unwrap_or_else(|| (Zeroizing::new(Vec::new()), false));
        // The key and the value are kept as read and as written.
        self.spend(key.len() + value.len() + size_of::<Pair>() + ALLOC)?;
        Ok(Pair {
            key: key.clone(),
            value: value.clone(),
            protected,
            ambiguous,
            orig: Some(Orig {
                carried: c,
                key,
                value,
                protected,
            }),
        })
    }
}

/// Whether an element is protected, as KeePassXC reads it: its `Protected`
/// attribute, with references resolved, is `True` in any case or `1`.
fn is_protected(e: &BytesStart<'_>) -> R<bool> {
    for attr in e.attributes() {
        let attr = attr.map_err(|_| Refused::NotWellFormed)?;
        if attr.key.as_ref() == b"Protected" {
            let mut value = Vec::new();
            let raw = &attr.value[..];
            let mut at = 0;
            while at < raw.len() {
                if raw[at] == b'&' {
                    let len = raw[at + 1..]
                        .iter()
                        .position(|&c| c == b';')
                        .ok_or(Refused::NotWellFormed)?;
                    let ch = decode_ref(&raw[at + 1..at + 1 + len])?;
                    let mut buf = [0u8; 4];
                    value.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    at += len + 2;
                } else {
                    value.push(raw[at]);
                    at += 1;
                }
            }
            return Ok(value.eq_ignore_ascii_case(b"true") || value == b"1");
        }
    }
    Ok(false)
}

fn uuid_of(text: &[u8]) -> R<[u8; 16]> {
    let s = core::str::from_utf8(text).map_err(|_| Refused::Uuid)?;
    let mut buf = [0u8; 16];
    let got = Base64::decode(s, &mut buf).map_err(|_| Refused::Uuid)?;
    if got.len() != 16 {
        return Err(Refused::Uuid);
    }
    Ok(buf)
}

impl<'a> Rd<'a> {
    fn custom(&mut self, a: usize, b: usize, empty: bool) -> R<CustomData> {
        let mut fr = Frame::<CdPart>::read(T_CUSTOM, &self.src[a..b], empty);
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            if !is(e.name().as_ref(), T_ITEM) {
                return Ok(false);
            }
            let pair = s.pair(a, b, empty)?;
            fr.parts.push(CdPart::Item(pair));
            Ok(true)
        })?;
        let mut keys: Vec<&[u8]> = fr
            .parts
            .iter()
            .filter_map(|p| match p {
                CdPart::Item(pair) if pair.key.starts_with(b"faraday.") => Some(&pair.key[..]),
                _ => None,
            })
            .collect();
        keys.sort_unstable();
        let mut ambiguous = fr.parts.iter().find_map(|p| match p {
            CdPart::Item(pair) => pair.ambiguous,
            _ => None,
        });
        if keys.windows(2).any(|w| w[0] == w[1]) {
            ambiguous = Some("two custom data items with one key");
        }
        Ok(CustomData {
            frame: fr,
            ambiguous,
        })
    }

    fn meta(&mut self, a: usize, b: usize, empty: bool) -> R<Meta> {
        let mut fr = Frame::<MetaPart>::read(T_META, &self.src[a..b], empty);
        let mut label = Vec::new();
        let mut label_leaf = None;
        let mut bin_seen = false;
        let mut custom = None;
        let mut ambiguous = None;
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            let name = e.name();
            let name = name.as_ref();
            if is(name, T_DBNAME) {
                if label_leaf.is_some() {
                    ambiguous = Some("DatabaseName twice");
                    return Ok(false);
                }
                let l = s.leaf(e, a, b, empty)?;
                label = l.text.to_vec();
                label_leaf = Some(l);
                fr.parts.push(MetaPart::Name);
            } else if is(name, T_CUSTOM) {
                if custom.is_some() {
                    ambiguous = Some("CustomData twice");
                    return Ok(false);
                }
                custom = Some(s.custom(a, b, empty)?);
                fr.parts.push(MetaPart::Custom);
            } else if is(name, T_BIN) {
                // Every one is read and every id kept, so that a Meta with
                // two names both as the recycle bin hides both groups; the
                // Meta is marked ambiguous. A set: a repeat adds nothing.
                if bin_seen {
                    ambiguous = Some("RecycleBinUUID twice");
                }
                bin_seen = true;
                let l = s.leaf(e, a, b, empty)?;
                match uuid_of(&l.text) {
                    Ok(uuid) => {
                        if uuid != [0; 16] {
                            s.bins.insert(uuid);
                        }
                    }
                    Err(_) => {
                        ambiguous = ambiguous.or(Some("a RecycleBinUUID that is not 16 bytes"));
                    }
                }
                // Kept as read, whatever it names.
                fr.carry(l.orig);
            } else if is(name, T_BINEN) {
                // Read, and kept as it is.
                let l = s.leaf(e, a, b, empty)?;
                // As KeePassXC: absent is enabled, empty and "false" are not.
                s.bin_enabled = l.text.eq_ignore_ascii_case(b"true");
                fr.carry(l.orig);
            } else {
                return Ok(false);
            }
            Ok(true)
        })?;
        if label_leaf.is_none() {
            fr.parts.push(MetaPart::Name);
        }
        if custom.is_none() {
            fr.parts.push(MetaPart::Custom);
        }
        let mut custom = custom.unwrap_or_else(CustomData::none);
        let ambiguous = ambiguous.or(custom.ambiguous);
        custom.ambiguous = ambiguous;
        Ok(Meta {
            frame: fr,
            label,
            label_leaf,
            custom,
            ambiguous,
        })
    }

    fn root(&mut self, a: usize, b: usize, empty: bool) -> R<Frame<RootPart>> {
        let mut fr = Frame::<RootPart>::read(T_ROOT, &self.src[a..b], empty);
        let mut seen = false;
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            if !is(e.name().as_ref(), T_GROUP) {
                return Ok(false);
            }
            if seen {
                return Err(Refused::Structure("two root groups"));
            }
            seen = true;
            let g = s.group(a, b, empty)?;
            fr.parts.push(RootPart::Group(Box::new(g)));
            Ok(true)
        })?;
        if !seen {
            return Err(Refused::Structure("no root group"));
        }
        Ok(fr)
    }

    /// Costs one walk of the group's XML, without decrypting anything.
    /// Whether the group whose start tag was just read is the recycle bin,
    /// and how many entries are in it and under it. Nothing is decrypted
    /// and the reader is left where it was: only the group's own `UUID`s
    /// and the `Entry` elements under groups are looked at, so what one
    /// reading accepts, a later one under another inner key accepts too.
    /// Any of the group's `UUID`s naming the bin makes it the bin, as
    /// `group` would take a later one after an invalid first; one marked
    /// Protected cannot be read here without moving the stream and is
    /// refused. The bin is then read once, by `skip_element`.
    fn bin_probe(&mut self) -> R<Option<usize>> {
        let saved = (self.r.clone(), self.pos, self.depth);
        let found = self.probe_group();
        (self.r, self.pos, self.depth) = saved;
        let (bin, count) = found?;
        Ok(bin.then_some(count))
    }

    fn probe_group(&mut self) -> R<(bool, usize)> {
        let mut bin = false;
        let mut count = 0;
        // Whether each open element is a group under groups only.
        let mut open = vec![true];
        while !open.is_empty() {
            let (ev, _, _) = self.next()?;
            match &ev {
                Event::Start(e) | Event::Empty(e) => {
                    let empty = matches!(ev, Event::Empty(_));
                    let n = e.name();
                    let under_groups = open.last() == Some(&true);
                    if open.len() == 1 && is(n.as_ref(), T_UUID) {
                        if !empty {
                            if is_protected(e)? {
                                return Err(Refused::Structure(
                                    "a group's UUID marked Protected beside a recycle bin",
                                ));
                            }
                            let at = (self.r.clone(), self.pos, self.depth);
                            if let Some(t) = self.text_or_markup()? {
                                bin |= uuid_of(&t.text).is_ok_and(|u| self.bins.contains(&u));
                            } else {
                                // Markup in the UUID: it names nothing, and
                                // is walked as any element, so its end does
                                // not close the group.
                                (self.r, self.pos, self.depth) = at;
                                open.push(false);
                            }
                        }
                        continue;
                    }
                    if under_groups && is(n.as_ref(), T_ENTRY) {
                        count += 1;
                    }
                    if !empty {
                        open.push(under_groups && is(n.as_ref(), T_GROUP));
                    }
                }
                Event::End(_) => {
                    open.pop();
                }
                _ => {}
            }
        }
        Ok((bin, count))
    }

    fn group(&mut self, a: usize, b: usize, empty: bool) -> R<Group> {
        let mut fr = Frame::<GroupPart>::read(T_GROUP, &self.src[a..b], empty);
        let mut uuid = None;
        let mut name = None;
        let mut custom = None;
        let mut ambiguous = None;
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            let n = e.name();
            let n = n.as_ref();
            if is(n, T_UUID) {
                if uuid.is_some() {
                    ambiguous = Some("a group with two UUIDs");
                    return Ok(false);
                }
                let l = s.leaf(e, a, b, empty)?;
                if let Ok(u) = uuid_of(&l.text) {
                    uuid = Some((u, l));
                    fr.parts.push(GroupPart::Uuid);
                } else {
                    ambiguous = Some("a UUID that is not 16 bytes");
                    fr.carry(l.orig);
                }
            } else if is(n, T_NAME) {
                if name.is_some() {
                    ambiguous = Some("a group with two names");
                    return Ok(false);
                }
                name = Some(s.leaf(e, a, b, empty)?);
                fr.parts.push(GroupPart::Name);
            } else if is(n, T_CUSTOM) {
                if custom.is_some() {
                    ambiguous = Some("a group with two lists of custom data");
                    return Ok(false);
                }
                custom = Some(s.custom(a, b, empty)?);
                fr.parts.push(GroupPart::Custom);
            } else if is(n, T_GROUP) {
                let bin = if s.bin_enabled && !s.bins.is_empty() && !empty {
                    s.bin_probe()?
                } else {
                    None
                };
                match bin {
                    Some(count) => {
                        // The recycle bin is kept as the bytes it came in,
                        // out of the model: nothing in it is listed,
                        // searched or loaded, and it is written back. It is
                        // read once, here; the probe decrypted nothing.
                        s.dropped += count;
                        let mut c = Carried::default();
                        s.skip_element(e, a, b, empty, &mut c)?;
                        fr.carry(c);
                    }
                    None => {
                        let g = s.group(a, b, empty)?;
                        fr.parts.push(GroupPart::Group(Box::new(g)));
                    }
                }
            } else if is(n, T_ENTRY) {
                let en = s.entry(a, b, empty)?;
                fr.parts.push(GroupPart::Entry(Box::new(en)));
            } else {
                return Ok(false);
            }
            Ok(true)
        })?;
        if uuid.is_none() {
            ambiguous = ambiguous.or(Some("a group without a UUID"));
        }
        let (uuid, uuid_leaf) = match uuid {
            Some((u, l)) => (u, Some(l)),
            None => ([0; 16], None),
        };
        let name_leaf = name;
        let mut custom = custom.unwrap_or_else(CustomData::none);
        let ambiguous = ambiguous.or(custom.ambiguous);
        custom.ambiguous = ambiguous;
        let mut g = Group {
            frame: fr,
            uuid,
            uuid_leaf,
            name: name_leaf
                .as_ref()
                .map(|l| l.text.to_vec())
                .unwrap_or_default(),
            name_leaf,
            custom,
            dup_uuid: false,
            ambiguous,
        };
        g.finish();
        self.spend(size_of::<Group>() + ALLOC + frame_cost(&g.frame))?;
        Ok(g)
    }

    fn times(
        &mut self,
        a: usize,
        b: usize,
        empty: bool,
    ) -> R<(Frame<TimesPart>, Option<Leaf>, Option<&'static str>)> {
        let mut fr = Frame::<TimesPart>::read(T_TIMES, &self.src[a..b], empty);
        let mut leaf = None;
        let mut ambiguous = None;
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            if !is(e.name().as_ref(), T_MODIFIED) {
                return Ok(false);
            }
            if leaf.is_some() {
                ambiguous = Some("LastModificationTime twice");
                return Ok(false);
            }
            leaf = Some(s.leaf(e, a, b, empty)?);
            fr.parts.push(TimesPart::Modified);
            Ok(true)
        })?;
        if leaf.is_none() {
            fr.parts.push(TimesPart::Modified);
        }
        Ok((fr, leaf, ambiguous))
    }

    fn entry(&mut self, a: usize, b: usize, empty: bool) -> R<Entry> {
        let mut fr = Frame::<EntryPart>::read(T_ENTRY, &self.src[a..b], empty);
        let mut uuid = None;
        let mut custom = None;
        let mut mod_leaf = None;
        let mut times = false;
        let mut ambiguous = None;
        self.children(&mut fr, &mut |s, fr, e, a, b, empty| {
            let n = e.name();
            let n = n.as_ref();
            if is(n, T_UUID) {
                if uuid.is_some() {
                    ambiguous = Some("an entry with two UUIDs");
                    return Ok(false);
                }
                let l = s.leaf(e, a, b, empty)?;
                if let Ok(u) = uuid_of(&l.text) {
                    uuid = Some((u, l));
                    fr.parts.push(EntryPart::Uuid);
                } else {
                    ambiguous = Some("a UUID that is not 16 bytes");
                    fr.carry(l.orig);
                }
            } else if is(n, T_FIELD) {
                let pair = s.pair(a, b, empty)?;
                fr.parts.push(EntryPart::Field(Box::new(pair)));
            } else if is(n, T_TIMES) {
                if times {
                    ambiguous = Some("an entry with two Times");
                    return Ok(false);
                }
                times = true;
                let (t, leaf, amb) = s.times(a, b, empty)?;
                s.spend(size_of::<Frame<TimesPart>>() + ALLOC + frame_cost(&t))?;
                ambiguous = ambiguous.or(amb);
                mod_leaf = leaf;
                fr.parts.push(EntryPart::Times(Box::new(t)));
            } else if is(n, T_CUSTOM) {
                if custom.is_some() {
                    ambiguous = Some("an entry with two lists of custom data");
                    return Ok(false);
                }
                custom = Some(s.custom(a, b, empty)?);
                fr.parts.push(EntryPart::Custom);
            } else if is(n, T_HISTORY) {
                // Kept as the bytes it came in, out of the model (its
                // protected values in wiping memory), and counted.
                let before = s.skipped_entries;
                let mut c = Carried::default();
                s.skip_element(e, a, b, empty, &mut c)?;
                fr.carry(c);
                if s.skipped_entries > before {
                    s.history += 1;
                }
            } else {
                return Ok(false);
            }
            Ok(true)
        })?;
        if uuid.is_none() {
            ambiguous = ambiguous.or(Some("an entry without a UUID"));
        }
        let (uuid, uuid_leaf) = match uuid {
            Some((u, l)) => (u, Some(l)),
            None => ([0; 16], None),
        };
        let modified = mod_leaf.as_ref().and_then(|l| decode_time(&l.text));
        let mut en = Entry {
            frame: fr,
            uuid,
            uuid_leaf,
            custom: custom.unwrap_or_else(CustomData::none),
            modified,
            mod_leaf,
            dup_keys: Vec::new(),
            dup_uuid: false,
            ambiguous: None,
        };
        en.finish();
        self.spend(entry_cost(&en, times))?;
        // The list of keys, a pointer and a length each, doubling.
        self.spend(2 * size_of::<&[u8]>() * en.fields().count())?;
        let mut keys: Vec<&[u8]> = en.fields().map(|f| &f.key[..]).collect();
        keys.sort_unstable();
        let mut repeated: Vec<&[u8]> = keys
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        repeated.dedup();
        let mut copies = 0;
        for k in &repeated {
            copies += k.len() + size_of::<Vec<u8>>() + 16;
        }
        self.spend(copies)?;
        let dups: Vec<Vec<u8>> = repeated.iter().map(|k| k.to_vec()).collect();
        if !dups.is_empty() {
            ambiguous = ambiguous.or(Some("an entry with two fields of one key"));
        }
        en.dup_keys = dups;
        let pairs = en.fields().find_map(|f| f.ambiguous);
        en.ambiguous = ambiguous.or(pairs).or(en.custom.ambiguous);
        en.custom.ambiguous = en.ambiguous;
        Ok(en)
    }
}

/// Appends `bytes` with each CR LF and each lone CR as one LF, as XML 1.0
/// §2.11 has a parser pass line ends on (a character reference to CR is a
/// separate event and stays).
fn push_normalised(out: &mut Vec<u8>, bytes: &[u8]) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push(b'\n');
            if bytes.get(i + 1) == Some(&b'\n') {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
}
