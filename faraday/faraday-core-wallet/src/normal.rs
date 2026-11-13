//! A descriptor wallet's descriptors in the form Bitcoin Core lists them
//! (`listdescriptors`), made from the stored text and the cache of the
//! last hardened step (`docs/decisions/F229.md`).
//!
//! Core stores a descriptor of its own master key as the master key with
//! hardened steps and no origin, `pkh(tpub…/44h/1h/0h/0/*)`, and lists it
//! as `pkh([fingerprint/44h/1h/0h]tpub…/0/*)`: the key at the last
//! hardened step is the one its `walletdescriptorlhcache` record holds
//! (`BIP32PubkeyProvider::ToNormalizedString` in `descriptor.cpp`), and
//! the fingerprint is the master key's. A key with an origin keeps it,
//! with the hardened steps after the key added to its path. A key whose
//! steps are all unhardened, or hardened to the end (`/*h`), is as stored;
//! every hardened mark is written `h`, as Core writes it.

use faraday_bip::bitcoin::base58;
use faraday_bip::bitcoin::hashes::{Hash, hash160, sha256d as hash256};
use faraday_bip::descriptor::descriptor_checksum;
use faraday_text::{Text, text};
use zeroize::Zeroizing;

/// The functions whose arguments are not keys.
const NOT_KEYS: [&str; 8] = [
    "older",
    "after",
    "sha256",
    "hash256",
    "ripemd160",
    "hash160",
    "raw",
    "addr",
];

/// A key Core caches the account key of: what the check of the cached key
/// needs.
pub(super) struct Need {
    /// The key's place in the descriptor, counted over every key.
    pub index: u32,
    /// The master key's compressed public key.
    pub root: [u8; 33],
    /// The master key's chain code, wiped when dropped.
    pub chain_code: Zeroizing<[u8; 32]>,
    /// Its depth, parent fingerprint and child number.
    pub meta: [u8; 9],
    /// The steps from it to the cached key: the hardened ones and the
    /// unhardened ones between them, bit 31 set on the hardened.
    pub path: Vec<u32>,
    /// The cached key's 74 bytes, as the file holds them.
    pub cached: [u8; 74],
}

/// Why a descriptor has no normalized form.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failed {
    /// The key at this place needs a cached key the file lacks.
    Missing(u32),
    /// A key expression this reader does not state.
    Unreadable,
    /// A cached key whose depth, child number or parent fingerprint
    /// disagrees with the path printed beside it.
    Key,
}

/// `body` (a descriptor without its checksum) as Core lists it, with its
/// checksum, and the keys whose cached account key it used. `cache` gives
/// the cached key of a key's place.
pub(super) fn normalize(
    body: &str,
    cache: &dyn Fn(u32) -> Option<[u8; 74]>,
) -> Result<(Text, Vec<Need>), Failed> {
    let mut out = Text::with_room(body.len() + 64);
    // Sized up front, so that no growth leaves a copy of a chain code
    // behind: at most one key to a separator, and one after the last.
    let mut needs = Vec::with_capacity(
        1 + body
            .bytes()
            .filter(|b| matches!(b, b')' | b',' | b'{' | b'}'))
            .count(),
    );
    let mut names: Vec<Text> = Vec::new();
    let mut token = Text::new();
    let mut index = 0u32;
    let mut argument = |token: &str,
                        names: &[Text],
                        out: &mut Text,
                        needs: &mut Vec<Need>|
     -> Result<(), Failed> {
        let is_key = !token.is_empty()
            && !token.bytes().all(|b| b.is_ascii_digit())
            && names.last().is_none_or(|n| {
                // A wrapper (`v:`, `a:`, `s:`...) leaves the function as it is.
                let function = n.rsplit_once(':').map_or(n.as_str(), |(_, f)| f);
                !NOT_KEYS.contains(&function)
            });
        if !is_key {
            out.push_str(token);
            return Ok(());
        }
        let at = index;
        index += 1;
        key(token, at, cache, out, needs)
    };
    for ch in body.chars() {
        match ch {
            '(' => {
                out.push_str(&token);
                names.push(std::mem::take(&mut token));
                out.push('(');
            }
            ')' | ',' | '{' | '}' => {
                argument(&token, &names, &mut out, &mut needs)?;
                token.clear();
                if ch == ')' {
                    names.pop();
                }
                out.push(ch);
            }
            other => token.push(other),
        }
    }
    argument(&token, &names, &mut out, &mut needs)?;
    let sum = descriptor_checksum(&out).ok_or(Failed::Unreadable)?;
    out.push('#');
    out.push_str(core::str::from_utf8(&sum).map_err(|_| Failed::Unreadable)?);
    Ok((out, needs))
}

/// `s` in lower case, as a [`Text`].
fn lowered(s: &str) -> Text {
    s.chars().map(|c| c.to_ascii_lowercase()).collect()
}

/// `h` for every hardened mark of a path's steps.
fn marked(step: &str) -> Option<(u32, bool)> {
    let (digits, hardened) = match step.strip_suffix(['\'', 'h', 'H']) {
        Some(digits) => (digits, true),
        None => (step, false),
    };
    let n: u32 = digits.parse().ok()?;
    (n < 1 << 31).then_some((n, hardened))
}

fn path_text(steps: &[u32]) -> Text {
    let mut out = Text::new();
    for &s in steps {
        out.push('/');
        out.push_str(&Text::num(u64::from(s & 0x7FFF_FFFF)));
        if s >> 31 == 1 {
            out.push('h');
        }
    }
    out
}

const BASE58: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// The 78 bytes of an extended key written in base58check, or `None` where
/// the text is not base58, its checksum fails or it does not decode to 82
/// bytes. The same as `base58::decode_check` with a 78-byte result, but
/// the work is in wiped memory: a decoded key holds a chain code, and the
/// library's decoding frees its buffer unwiped.
pub(crate) fn extended_key_bytes(text: &str) -> Option<Zeroizing<[u8; 78]>> {
    const TOTAL: usize = 82;
    // The number so far, least significant byte first.
    let mut number = Zeroizing::new([0u8; TOTAL]);
    let mut used = 0usize;
    for ch in text.bytes() {
        let mut carry = BASE58.iter().position(|&b| b == ch)?;
        for byte in &mut number[..used] {
            carry += usize::from(*byte) * 58;
            *byte = (carry & 0xFF) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            *number.get_mut(used)? = (carry & 0xFF) as u8;
            used += 1;
            carry >>= 8;
        }
    }
    let zeros = text.bytes().take_while(|&b| b == b'1').count();
    if zeros + used != TOTAL {
        return None;
    }
    let mut bytes = Zeroizing::new([0u8; TOTAL]);
    for (to, from) in bytes[zeros..].iter_mut().zip(number[..used].iter().rev()) {
        *to = *from;
    }
    let (payload, check) = bytes.split_at(78);
    if hash256::Hash::hash(payload).to_byte_array()[..4] != *check {
        return None;
    }
    let mut key = Zeroizing::new([0u8; 78]);
    key.copy_from_slice(payload);
    Some(key)
}

/// One key expression, Core's way.
fn key(
    token: &str,
    index: u32,
    cache: &dyn Fn(u32) -> Option<[u8; 74]>,
    out: &mut Text,
    needs: &mut Vec<Need>,
) -> Result<(), Failed> {
    if token.contains(['<', ';', '>']) {
        return Err(Failed::Unreadable);
    }
    // The origin: its fingerprint and path.
    let (origin, rest) = match token.strip_prefix('[') {
        Some(t) => {
            let (inside, rest) = t.split_once(']').ok_or(Failed::Unreadable)?;
            let mut parts = inside.split('/');
            let fp = parts.next().ok_or(Failed::Unreadable)?;
            if fp.len() != 8 || !fp.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Failed::Unreadable);
            }
            let steps = parts
                .map(|s| marked(s).map(|(n, h)| n | (u32::from(h) << 31)))
                .collect::<Option<Vec<u32>>>()
                .ok_or(Failed::Unreadable)?;
            (Some((lowered(fp), steps)), rest)
        }
        None => (None, token),
    };
    let mut parts = rest.split('/');
    let head = parts.next().ok_or(Failed::Unreadable)?;
    let raw = parts.collect::<Vec<&str>>();
    let decoded = extended_key_bytes(head);
    let Some(x) = decoded else {
        // A single key: written as hex, which is how Core lists it.
        if !raw.is_empty() {
            return Err(Failed::Unreadable);
        }
        if let Some((fp, steps)) = &origin {
            out.push('[');
            out.push_str(fp);
            out.push_str(&path_text(steps));
            out.push(']');
        }
        out.push_str(&lowered(head));
        return Ok(());
    };
    // An extended public key (a private one is refused).
    if x[..4] != [0x04, 0x88, 0xB2, 0x1E] && x[..4] != [0x04, 0x35, 0x87, 0xCF] {
        return Err(Failed::Unreadable);
    }
    let ranged = raw
        .last()
        // not wording: a descriptor's range step
        .is_some_and(|l| matches!(*l, "*" | "*'" | "*h" | "*H"));
    let hardened_range = ranged && raw.last().is_some_and(|l| *l != "*");
    let steps = &raw[..raw.len() - usize::from(ranged)];
    let steps = steps
        .iter()
        .map(|s| marked(s).map(|(n, h)| n | (u32::from(h) << 31)))
        .collect::<Option<Vec<u32>>>()
        .ok_or(Failed::Unreadable)?;
    let last_hardened = steps.iter().rposition(|s| s >> 31 == 1);
    let origin_text = |fp: &str, path: &[u32]| text!("[{fp}{}]", &*path_text(path));
    // No hardened step to normalize, or hardened to the end: as stored,
    // with the origin's marks and the path's written `h`.
    let Some(last) = last_hardened.filter(|_| !hardened_range) else {
        if let Some((fp, path)) = &origin {
            out.push_str(&origin_text(fp, path));
        }
        out.push_str(head);
        out.push_str(&path_text(&steps));
        if ranged {
            out.push_str(if hardened_range { "/*h" } else { "/*" });
        }
        return Ok(());
    };
    let cached = cache(index).ok_or(Failed::Missing(index))?;
    let mut root = [0u8; 33];
    root.copy_from_slice(&x[45..78]);
    let mut meta = [0u8; 9];
    meta.copy_from_slice(&x[4..13]);
    // The cached key's depth, child number and (one step on) parent
    // fingerprint are what the path says, as a descriptor file's key
    // is read against its origin.
    let walked = steps[..=last].len();
    let child = steps[last].to_be_bytes();
    let parent_ok = walked != 1 || cached[1..5] == hash160::Hash::hash(&root).to_byte_array()[..4];
    if usize::from(cached[0]) != usize::from(meta[0]) + walked
        || cached[5..9] != child
        || !parent_ok
    {
        return Err(Failed::Key);
    }
    let mut version = [0u8; 4];
    version.copy_from_slice(&x[..4]);
    let mut bytes = Zeroizing::new(Vec::with_capacity(78));
    bytes.extend_from_slice(&version);
    bytes.extend_from_slice(&cached);
    let mut chain_code = Zeroizing::new([0u8; 32]);
    chain_code.copy_from_slice(&x[13..45]);
    let (fp, path) = match &origin {
        Some((fp, path)) => (fp.clone(), [path.as_slice(), &steps[..=last]].concat()),
        None => (
            faraday_crypto::hex::encode_string(&hash160::Hash::hash(&root).to_byte_array()[..4]),
            steps[..=last].to_vec(),
        ),
    };
    out.push_str(&origin_text(&fp, &path));
    out.push_str(&base58::encode_check(&bytes));
    out.push_str(&path_text(&steps[last + 1..]));
    if ranged {
        out.push_str("/*");
    }
    needs.push(Need {
        index,
        root,
        chain_code,
        meta,
        path: steps[..=last].to_vec(),
        cached,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator point's key as a master key at depth 0, with a
    /// chain code of 7s.
    fn master() -> Text {
        let mut b = vec![0x04, 0x35, 0x87, 0xCF, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        b.extend([7u8; 32]);
        b.extend(faraday_crypto::hex::decode_vec(G).expect("hex"));
        Text::of(&base58::encode_check(&b))
    }

    const G: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    /// A cached key `depth` deep, whose child number is `child`.
    fn cached(depth: u8, child: u32) -> [u8; 74] {
        let mut c = [7u8; 74];
        c[0] = depth;
        c[1..5].copy_from_slice(&[1, 2, 3, 4]);
        c[5..9].copy_from_slice(&child.to_be_bytes());
        c[41..].copy_from_slice(&faraday_crypto::hex::decode_vec(G).expect("hex"));
        c
    }

    fn listed(body: &str) -> Result<(Text, Vec<Need>), Failed> {
        let by = cached(4, 2 | 1 << 31);
        normalize(body, &move |i| (i == 0).then_some(by))
            .map(|(text, needs)| (Text::of(text.split('#').next().expect("text")), needs))
    }

    #[test]
    fn an_extended_key_decodes_as_the_library_decodes_it() {
        let good = master();
        let mut flipped = good.as_bytes().to_vec();
        flipped[20] = if flipped[20] == b'2' { b'3' } else { b'2' };
        let flipped = Text::from_utf8_lossy(&flipped);
        let ones = text!("1{}", &*good);
        let twice = text!("{}{}", &*good, &*good);
        let short = base58::encode_check(&[5u8; 77]);
        let long = base58::encode_check(&[5u8; 79]);
        let mut zeros = [5u8; 78];
        zeros[..2].fill(0);
        let zeros = base58::encode_check(&zeros);
        for text in [
            good.as_str(),
            flipped.as_str(),
            ones.as_str(),
            twice.as_str(),
            short.as_str(),
            long.as_str(),
            zeros.as_str(),
            "",
            "1111",
            "0OIl",
            "tpub",
        ] {
            let library = base58::decode_check(text).ok().filter(|b| b.len() == 78);
            let ours = extended_key_bytes(text);
            assert_eq!(
                ours.as_deref().map(|b| b.as_slice()),
                library.as_deref(),
                "{text}"
            );
        }
        assert!(extended_key_bytes(&zeros).is_some());
    }

    #[test]
    fn a_hash_in_a_wrapper_has_no_key_place() {
        let x = master();
        let x = &*x;
        let hash = "ab".repeat(32);
        for wrapper in ["v:sha256", "a:hash160", "sv:ripemd160", "n:hash256"] {
            let body = text!("wsh(and_v({wrapper}({hash}),pk({x}/48h/1h/0h/2h/0/*)))");
            let (_, needs) = listed(&body).expect(wrapper);
            assert_eq!(needs[0].index, 0, "{wrapper}");
        }
    }

    #[test]
    fn a_hardened_range_and_an_apostrophe_are_written_as_core_writes_them() {
        let x = master();
        let x = &*x;
        let (text, needs) = listed(&text!("pkh({x}/44'/1'/0'/0/*h)")).expect("hardened range");
        assert_eq!(text, text!("pkh({x}/44h/1h/0h/0/*h)"));
        assert!(needs.is_empty());
        let (text, _) = listed(&text!("pkh([d34db33f/44'/1']{x}/0/*)")).expect("origin");
        assert_eq!(text, text!("pkh([d34db33f/44h/1h]{x}/0/*)"));
    }

    #[test]
    fn a_cached_key_of_another_depth_is_not_taken() {
        let x = master();
        let x = &*x;
        let body = text!("wsh(pk({x}/48h/1h/0h/2h/0/*))");
        let wrong = cached(5, 2 | 1 << 31);
        assert_eq!(normalize(&body, &|_| Some(wrong)).err(), Some(Failed::Key));
        let child = cached(4, 3 | 1 << 31);
        assert_eq!(normalize(&body, &|_| Some(child)).err(), Some(Failed::Key));
    }
}
