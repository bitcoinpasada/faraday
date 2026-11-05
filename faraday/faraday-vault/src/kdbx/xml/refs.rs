//! Which attachments a document refers to (`super`, `docs/VAULT.md` §7.1).

use alloc::vec;
use alloc::vec::Vec;

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

/// Costs a read of `xml` with the tokenizer, linear in its size (at most
/// the vault's largest, 4 MiB): for each of `count` attachments, whether
/// the document refers to it. Every element anywhere is looked at: an
/// entry's, a history entry's, a recycle bin entry's. A reference to a
/// position that does not exist marks nothing. None when something cannot
/// be understood (the text does not tokenize, a `Ref` that is not a plain
/// number, a `Binary` element that is not a `Key` and a `Value` with a
/// `Ref`), and then every attachment is to be kept.
pub fn referenced(xml: &[u8], count: usize) -> Option<Vec<bool>> {
    let mut used = vec![false; count];
    let mut r = Reader::from_reader(xml);
    // Inside a `Binary`: the `Value` elements with a `Ref` seen so far.
    let mut binary: Option<usize> = None;
    // Depth below the `Binary`, whose children are looked at.
    let mut depth = 0usize;
    loop {
        let ev = r.read_event().ok()?;
        match &ev {
            Event::Eof => return Some(used),
            Event::Start(e) | Event::Empty(e) => {
                let empty = matches!(ev, Event::Empty(_));
                let name = e.local_name();
                let name = name.as_ref();
                let found = reference(e, &mut used)?;
                if name == b"Binary" {
                    if binary.is_some() || empty {
                        return None;
                    }
                    binary = Some(0);
                    depth = 0;
                    continue;
                }
                if let Some(seen) = binary.as_mut() {
                    if depth == 0 {
                        match name {
                            b"Key" => {}
                            b"Value" if found => *seen += 1,
                            _ => return None,
                        }
                    }
                    if !empty {
                        depth += 1;
                    }
                }
            }
            Event::End(_) if binary.is_some() => {
                if depth == 0 {
                    // The end of the `Binary`.
                    if binary != Some(1) {
                        return None;
                    }
                    binary = None;
                } else {
                    depth -= 1;
                }
            }
            _ => {}
        }
    }
}

/// Marks the position a `Ref` attribute of `e` states. None when it is not
/// a plain number or the attributes cannot be read; Some whether there was
/// one.
fn reference(e: &BytesStart<'_>, used: &mut [bool]) -> Option<bool> {
    let mut found = false;
    for attr in e.attributes() {
        let attr = attr.ok()?;
        if attr.key.local_name().as_ref() != b"Ref" {
            continue;
        }
        let value = attr
            .normalized_value(quick_xml::XmlVersion::Explicit1_0)
            .ok()?;
        let digits = value.as_bytes();
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return None;
        }
        found = true;
        // Too large for a position: no attachment has it.
        if let Ok(at) = value.parse::<usize>()
            && let Some(slot) = used.get_mut(at)
        {
            *slot = true;
        }
    }
    Some(found)
}
