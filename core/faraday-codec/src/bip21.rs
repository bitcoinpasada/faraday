//! BIP-21: a `bitcoin:` payment URI, read.
//!
//! The scheme (any case), an address, then `?` and `&`-separated
//! `key=value` parameters, each key and value percent-decoded. `amount`
//! (decimal bitcoin, at most eight decimals), `label` and `message` are
//! understood; each may stand once. A parameter whose key begins `req-` is
//! one the reader must understand, and none is understood, so the URI is
//! refused (also when its key is percent-encoded); an understood key
//! written percent-encoded is refused. Any other parameter is ignored, Coldcard's `wallet=` included.
//! A bad percent escape, a value that is not UTF-8, a repeated understood
//! parameter, a bad amount or a parameter without `=` is a refusal, never
//! a guess. The address is returned as written; whether it is one is
//! `bitcoin::Address`'s to say.

use alloc::string::String;

/// The most bitcoin there can be, in satoshis.
const MAX_SATS: u64 = 21_000_000 * 100_000_000;

/// A `bitcoin:` URI read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri<'a> {
    /// What stands between the scheme and `?`; empty when there is none.
    pub address: &'a str,
    /// `amount`, in satoshis.
    pub amount: Option<u64>,
    /// `label`, percent-decoded.
    pub label: Option<String>,
    /// `message`, percent-decoded.
    pub message: Option<String>,
}

/// Why a text is not a `bitcoin:` URI Faraday reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The text does not begin `bitcoin:`.
    NotUri,
    /// A `req-` parameter: one the sender needs understood.
    Required,
    /// A bad percent escape, a value that is not UTF-8, a parameter
    /// without `=`, an empty parameter, a repeated `amount`, `label` or
    /// `message`, a percent-encoded `amount`, `label` or `message` key, or
    /// a bad `amount`.
    Malformed,
}

/// Reads `text` as a BIP-21 URI. Pure; allocates only the decoded `label`
/// and `message`.
pub fn parse(text: &str) -> Result<Uri<'_>, Error> {
    const SCHEME: &str = "bitcoin:";
    let head = text.as_bytes().get(..SCHEME.len()).ok_or(Error::NotUri)?;
    if !head.eq_ignore_ascii_case(SCHEME.as_bytes()) {
        return Err(Error::NotUri);
    }
    // The scheme is ASCII, so this is a character boundary.
    let rest = &text[SCHEME.len()..];
    let (address, query) = match rest.split_once('?') {
        Some((a, q)) => (a, Some(q)),
        None => (rest, None),
    };
    let mut uri = Uri {
        address,
        amount: None,
        label: None,
        message: None,
    };
    let mut required = false;
    if let Some(query) = query {
        for param in query.split('&') {
            let (key, value) = param.split_once('=').ok_or(Error::Malformed)?;
            let raw = key;
            let key = decode(key)?;
            let value = decode(value)?;
            // A key Faraday understands must be written plainly: encoded, it
            // is read by no other reader, so it is refused, not taken.
            if key != raw && matches!(key.as_str(), "amount" | "label" | "message") {
                return Err(Error::Malformed);
            }
            match key.as_str() {
                "amount" => {
                    if uri.amount.replace(amount(&value)?).is_some() {
                        return Err(Error::Malformed);
                    }
                }
                "label" => {
                    if uri.label.replace(value).is_some() {
                        return Err(Error::Malformed);
                    }
                }
                "message" => {
                    if uri.message.replace(value).is_some() {
                        return Err(Error::Malformed);
                    }
                }
                k if k.starts_with("req-") => required = true,
                _ => {}
            }
        }
    }
    // Malformed outranks required: the whole URI was read first.
    if required {
        return Err(Error::Required);
    }
    Ok(uri)
}

/// Percent-decodes `s` (`+` stays a plus) to UTF-8.
fn decode(s: &str) -> Result<String, Error> {
    let b = s.as_bytes();
    let mut out = alloc::vec::Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hi = b.get(i + 1).and_then(|&c| hex(c)).ok_or(Error::Malformed)?;
            let lo = b.get(i + 2).and_then(|&c| hex(c)).ok_or(Error::Malformed)?;
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| Error::Malformed)
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Decimal bitcoin to satoshis: digits, optionally a point and up to
/// eight more, at least one digit in all, no sign or exponent, no more
/// than 21 million bitcoin.
fn amount(s: &str) -> Result<u64, Error> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.len() + frac.len() == 0
        || frac.len() > 8
        || !whole
            .bytes()
            .chain(frac.bytes())
            .all(|c| c.is_ascii_digit())
    {
        return Err(Error::Malformed);
    }
    let mut sats: u64 = 0;
    for c in whole.bytes() {
        sats = sats
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(c - b'0')))
            .ok_or(Error::Malformed)?;
    }
    sats = sats.checked_mul(100_000_000).ok_or(Error::Malformed)?;
    let mut f: u64 = 0;
    for i in 0..8 {
        f = f * 10 + frac.as_bytes().get(i).map_or(0, |c| u64::from(c - b'0'));
    }
    let sats = sats.checked_add(f).ok_or(Error::Malformed)?;
    if sats > MAX_SATS {
        return Err(Error::Malformed);
    }
    Ok(sats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    /// A valid mainnet address (BIP-21's own example address has a bad
    /// checksum, which `bip21` refuses; the parser here does not read it).
    const A: &str = "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2";

    /// BIP-21's "Examples" section (bips/bip-0021.mediawiki), word for word.
    #[test]
    fn the_bips_own_examples() {
        let a = "175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W";
        let u = parse("bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W").unwrap();
        assert_eq!((u.address, u.amount, u.label), (a, None, None));
        let u = parse("bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?label=Luke-Jr").unwrap();
        assert_eq!(u.label.as_deref(), Some("Luke-Jr"));
        let u =
            parse("bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?amount=20.3&label=Luke-Jr").unwrap();
        assert_eq!(
            (u.amount, u.label.as_deref()),
            (Some(2_030_000_000), Some("Luke-Jr"))
        );
        let u = parse(
            "bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?amount=50&label=Luke-Jr&message=Donation%20for%20project%20xyz",
        )
        .unwrap();
        assert_eq!(u.amount, Some(5_000_000_000));
        assert_eq!(u.message.as_deref(), Some("Donation for project xyz"));
        // "Requires parameters not understood" and "Some other parameters".
        assert_eq!(
            parse(
                "bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?req-somethingyoudontunderstand=50&req-somethingelseyoudontget=999"
            ),
            Err(Error::Required)
        );
        let u = parse(
            "bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?somethingyoudontunderstand=50&somethingelseyoudontget=999",
        )
        .unwrap();
        assert_eq!(
            (u.address, u.amount, u.label, u.message),
            (a, None, None, None)
        );
    }

    #[test]
    fn what_is_refused_says_why() {
        let cases = [
            ("amount=1&amount=2", Error::Malformed),
            ("label=a&label=b", Error::Malformed),
            ("message=a&message=b", Error::Malformed),
            ("req-x=1&amount=x", Error::Malformed),
            ("amount=1&req-x=1", Error::Required),
            ("%72eq-x=1", Error::Required),
            ("req-=1", Error::Required),
            ("%61mount=7", Error::Malformed),
            ("%6cabel=a", Error::Malformed),
            ("%6dessage=a", Error::Malformed),
        ];
        for (tail, want) in cases {
            let uri = format!("bitcoin:{A}?{tail}");
            assert_eq!(parse(&uri).err(), Some(want), "{uri}");
        }
        assert_eq!(parse(A), Err(Error::NotUri));
        assert_eq!(parse("bitcoin"), Err(Error::NotUri));
        assert_eq!(parse("bitcoín:x"), Err(Error::NotUri));
        // Coldcard's own parameter is not taken.
        let text = format!("bitcoin:{A}?wallet=abc");
        let u = parse(&text).unwrap();
        assert_eq!((u.amount, u.label, u.message), (None, None, None));
    }

    type Out<'a> = Option<(Option<u64>, Option<&'a str>, Option<&'a str>)>;

    /// Each input read by the `bip21` crate 0.5.0 (rust-bitcoin's, not
    /// Faraday's: `Uri::<NetworkUnchecked, NoExtras>::from_str`, in a
    /// scratch crate outside this repository, `<A>` the address above)
    /// and by this parser. The last field is empty where they agree and
    /// says why where this parser refuses what `bip21` takes: BIP-21 does
    /// not say, and a URI that is not read as written is refused here.
    #[rustfmt::skip]
    const VECTORS: &[(&str, Out<'static>, Out<'static>, &str)] = &[
        ("bitcoin:<A>", Some((None, None, None)), Some((None, None, None)), ""),
        ("bitcoin:<A>?label=Luke-Jr", Some((None, Some("Luke-Jr"), None)), Some((None, Some("Luke-Jr"), None)), ""),
        ("bitcoin:<A>?amount=20.3&label=Luke-Jr", Some((Some(2_030_000_000), Some("Luke-Jr"), None)), Some((Some(2_030_000_000), Some("Luke-Jr"), None)), ""),
        ("bitcoin:<A>?amount=50&label=Luke-Jr&message=Donation%20for%20project%20xyz", Some((Some(5_000_000_000), Some("Luke-Jr"), Some("Donation for project xyz"))), Some((Some(5_000_000_000), Some("Luke-Jr"), Some("Donation for project xyz"))), ""),
        ("bitcoin:<A>?req-somethingyoudontunderstand=50&req-somethingelseyoudontget=999", None, None, ""),
        ("bitcoin:<A>?somethingyoudontunderstand=50&somethingelseyoudontget=999", Some((None, None, None)), Some((None, None, None)), ""),
        ("BITCOIN:<A>?amount=1", Some((Some(100_000_000), None, None)), Some((Some(100_000_000), None, None)), ""),
        ("BitCoin:<A>?amount=0.00000001", Some((Some(1), None, None)), Some((Some(1), None, None)), ""),
        ("bitcoin:<A>?amount=1&amount=2", Some((Some(200_000_000), None, None)), None, "a second amount"),
        ("bitcoin:<A>?amount=1&label=a&label=b", Some((Some(100_000_000), Some("b"), None)), None, "a second label"),
        ("bitcoin:<A>?label=a%2", None, None, ""),
        ("bitcoin:<A>?label=a%zz", None, None, ""),
        ("bitcoin:<A>?label=a%", None, None, ""),
        ("bitcoin:<A>?label=%ff", Some((None, None, None)), None, "a label that is not UTF-8 (bip21 reads it but cannot give it)"),
        ("bitcoin:<A>?label=a+b", Some((None, Some("a+b"), None)), Some((None, Some("a+b"), None)), ""),
        ("bitcoin:<A>?label=a%20b%26c", Some((None, Some("a b&c"), None)), Some((None, Some("a b&c"), None)), ""),
        ("bitcoin:<A>?amount=1e3", None, None, ""),
        ("bitcoin:<A>?amount=-1", None, None, ""),
        ("bitcoin:<A>?amount=+1", None, None, ""),
        ("bitcoin:<A>?amount=.5", Some((Some(50_000_000), None, None)), Some((Some(50_000_000), None, None)), ""),
        ("bitcoin:<A>?amount=1.", Some((Some(100_000_000), None, None)), Some((Some(100_000_000), None, None)), ""),
        ("bitcoin:<A>?amount=", None, None, ""),
        ("bitcoin:<A>?amount=.", Some((Some(0), None, None)), None, "an amount with no digit"),
        ("bitcoin:<A>?amount=0.000000001", None, None, ""),
        ("bitcoin:<A>?amount=21000000", Some((Some(2_100_000_000_000_000), None, None)), Some((Some(2_100_000_000_000_000), None, None)), ""),
        ("bitcoin:<A>?amount=21000000.00000001", Some((Some(2_100_000_000_000_001), None, None)), None, "an amount over 21 million bitcoin"),
        ("bitcoin:<A>?amount=99999999999999999999", None, None, ""),
        ("bitcoin:<A>?amount=1,5", None, None, ""),
        ("bitcoin:<A>?amount=1.2.3", None, None, ""),
        ("bitcoin:<A>?", None, None, ""),
        ("bitcoin:<A>?&", None, None, ""),
        ("bitcoin:<A>?label", None, None, ""),
        ("bitcoin:<A>?label=a&", None, None, ""),
        ("bitcoin:<A>?wallet=abc", Some((None, None, None)), Some((None, None, None)), ""),
        ("bitcoin:<A>?%72eq-x=1", Some((None, None, None)), None, "a percent-encoded req- key, which is req- once decoded"),
        ("bitcoin:<A>?req-x=1&amount=1&amount=2", None, None, ""),
        ("bitcoin:<A>?req-=1", None, None, ""),
        ("bitcoin:<A>?Req-x=1", Some((None, None, None)), Some((None, None, None)), ""),
        ("bitcoin:<A>?Amount=1", Some((None, None, None)), Some((None, None, None)), ""),
        ("bitcoin:<A>?%61mount=7", Some((None, None, None)), None, "a percent-encoded amount, label or message key (bip21 does not decode a key, so reads none; refused here)"),
        ("bitcoin:<A>?label=a=b", Some((None, Some("a=b"), None)), Some((None, Some("a=b"), None)), ""),
    ];

    #[test]
    fn the_same_inputs_as_the_bip21_crate() {
        for (input, outside, ours, why) in VECTORS {
            let input = input.replace("<A>", A);
            let read = parse(&input).ok();
            let got: Out<'_> = read
                .as_ref()
                .map(|u| (u.amount, u.label.as_deref(), u.message.as_deref()));
            assert_eq!(got, *ours, "{input}");
            // Where the two differ, the row says why.
            assert_eq!(outside == ours, why.is_empty(), "{input}: {why}");
        }
    }

    #[test]
    fn a_text_that_is_no_uri() {
        assert_eq!(parse(""), Err(Error::NotUri));
        assert_eq!(parse("bitcoin:").map(|u| u.address), Ok(""));
    }
}
