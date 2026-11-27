//! The addresses in a text file: one a line, or a list after other
//! text. [`find`] reads a text token by token, a `bitcoin:` link by
//! BIP 21 ([`crate::bip21`]) and anything else as a Bitcoin address of
//! any network, and keeps each address once, in the order it first
//! stands. It reads nothing but addresses: a descriptor, a key or a
//! word list in the text is the caller's to name or refuse.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str::FromStr;

use bitcoin::address::{Address, NetworkUnchecked};

/// The most addresses [`find`] keeps.
pub const MAX_ADDRESSES: usize = 100;

/// The shortest text an address is: BIP 350's shortest, a version 16
/// witness program of two bytes (14 characters). A P2PKH address is
/// about 26.
const SHORTEST: usize = 14;

/// The addresses of a text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Found {
    /// The first [`MAX_ADDRESSES`] addresses, each once, as written
    /// (a link's, its address).
    pub addresses: Vec<String>,
    /// How many different addresses the text holds, past the cap too.
    pub total: usize,
}

/// A character that ends a plain token as whitespace does.
fn is_break(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ',' | ';' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '|'
        )
}

/// Whether `text` starts a `bitcoin:` link, in any case.
fn starts_link(text: &str) -> bool {
    text.as_bytes()
        .get(..8)
        .is_some_and(|h| h.eq_ignore_ascii_case(b"bitcoin:"))
}

/// The link at the start of `text`, as its length and whether it is
/// read: it ends at whitespace, at a quote, an angle bracket or a closing
/// square bracket, or at a closing parenthesis that no opening one in the
/// link matches (the characters that cut it). A comma, a semicolon or an apostrophe is the link's own
/// (RFC 3986), so the link is read whole and its parser judges it. A link
/// cut at one of those characters with more than closing punctuation
/// after it before the next whitespace is not a link set apart by its
/// text: it runs to the whitespace and is not read.
fn link_len(text: &str) -> (usize, bool) {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return cut_at(text, i),
            ')' => depth -= 1,
            '"' | '<' | '>' | ']' => return cut_at(text, i),
            c if c.is_whitespace() => return (i, true),
            _ => {}
        }
    }
    (text.len(), true)
}

/// A link cut at `i` (not at whitespace): read if only closing
/// punctuation follows up to the next whitespace, or if closing
/// punctuation, a comma or semicolon and an opening quote or bracket
/// lead on to the next `bitcoin:` link or a whole address (a list
/// written without spaces); else it runs on to that whitespace and is
/// not read.
fn cut_at(text: &str, i: usize) -> (usize, bool) {
    let rest = &text[i..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let only_closing = rest[..end]
        .chars()
        .all(|c| matches!(c, ')' | ']' | '"' | '>' | '.' | ',' | ';' | ':'));
    if only_closing || starts_next_item(&rest[..end]) {
        (i, true)
    } else {
        (i + end, false)
    }
}

/// Whether `cut` (the text after a link's cut, up to whitespace) is
/// closing punctuation, a comma or semicolon, an opening quote or
/// bracket, then a `bitcoin:` link or a token that is a whole address.
fn starts_next_item(cut: &str) -> bool {
    let after_closing = cut.trim_start_matches([')', ']', '"', '>', '.']);
    let Some(after_sep) = after_closing.strip_prefix([',', ';']) else {
        return false;
    };
    let Some(item) = after_sep.strip_prefix(['"', '(', '[', '<']) else {
        return false;
    };
    starts_link(item) || address_of(item.split(is_break).next().unwrap_or("")).is_some()
}

/// The next token of `text` at or after `from`, as its start, its end and
/// whether it is read.
fn next_token(text: &str, from: usize) -> Option<(usize, usize, bool)> {
    let rest = &text[from..];
    let start = from + rest.find(|c: char| !is_break(c))?;
    let tail = &text[start..];
    let (len, read) = if starts_link(tail) {
        link_len(tail)
    } else {
        (tail.find(is_break).unwrap_or(tail.len()), true)
    };
    Some((start, start + len, read))
}

/// The address a token is, if it is one.
fn address_of(token: &str) -> Option<(&str, Address<NetworkUnchecked>)> {
    let link = starts_link(token);
    let token = if link {
        token
    } else {
        token.trim_end_matches(['.', ':'])
    };
    if token.len() < SHORTEST {
        return None;
    }
    let candidate = if link {
        match crate::bip21::parse(token) {
            Ok(uri) => uri.address,
            Err(_) => return None,
        }
    } else {
        token
    };
    let parsed = Address::<NetworkUnchecked>::from_str(candidate).ok()?;
    Some((candidate, parsed))
}

/// Costs: one BIP 21 parse or address decode per token of 14 bytes or
/// more (a bech32 checksum, or a base58 double SHA-256) and one set
/// insert per address found, so a 64 KiB text is a few thousand decodes
/// at most.
///
/// Reads the addresses of `text`. Pure; allocates the addresses kept.
///
/// An address counts once by what it parses to, so the upper-case and
/// lower-case spellings of one bech32 address are one; the first
/// spelling stands.
pub fn find(text: &str) -> Found {
    let mut found = Found::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut at = 0;
    while let Some((start, end, read)) = next_token(text, at) {
        at = end;
        if !read {
            continue;
        }
        let Some((a, parsed)) = address_of(&text[start..end]) else {
            continue;
        };
        if !seen.insert(parsed.assume_checked().to_string()) {
            continue;
        }
        found.total += 1;
        if found.addresses.len() < MAX_ADDRESSES {
            found.addresses.push(a.to_string());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    const A: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
    const B: &str = "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2";
    const C: &str = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";

    #[test]
    fn a_list_after_other_text_gives_each_address_once() {
        let text = format!("My payees:\n- {A}\n- {B}, {C}.\n{A}\nthanks");
        let f = find(&text);
        assert_eq!(f.addresses, [A, B, C]);
        assert_eq!(f.total, 3);
    }

    #[test]
    fn a_link_gives_its_address_and_a_bad_link_none() {
        let f = find(&format!("bitcoin:{A}?amount=0.1&label=x"));
        assert_eq!(f.addresses, [A]);
        assert!(find(&format!("bitcoin:{A}?req-x=1")).addresses.is_empty());
    }

    #[test]
    fn words_and_numbers_are_no_address() {
        assert!(
            find("pay the rent 12345678901234567890 soon")
                .addresses
                .is_empty()
        );
    }

    /// BIP 350's valid addresses and BIP 173's.
    const VALID: [&str; 6] = [
        "BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4",
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
        "bc1pw508d6qejxtdg4y5r3zarvary0c5xw7kw508d6qejxtdg4y5r3zarvary0c5xw7kt5nd6y",
        "BC1SW50QGDZ25J",
        "bc1zw508d6qejxtdg4y5r3zarvaryvaxxpcs",
        "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0",
    ];

    /// BIP 173 and BIP 350's invalid ones: a wrong prefix, a wrong
    /// checksum, a checksum of the wrong kind for the version, mixed case.
    const INVALID: [&str; 6] = [
        "tc1qw508d6qejxtdg4y5r3zarvary0c5xw7kg3g4ty",
        "BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T5",
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kemeawh",
        "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqh2y7hd",
        "tb1z0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqglt7rf",
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sL5k7",
    ];

    #[test]
    fn published_valid_addresses_are_found_whole_and_invalid_ones_not() {
        for v in VALID {
            assert_eq!(find(v).addresses, [v], "{v}");
        }
        for v in INVALID {
            assert!(find(v).addresses.is_empty(), "{v}");
            assert!(find(&format!("pay {v}\n")).addresses.is_empty(), "{v}");
        }
    }

    #[test]
    fn a_link_is_read_whole() {
        // A comma or an apostrophe is inside the link: it is Required, or
        // malformed, and gives nothing.
        for tail in [
            "?label=Smith,John&req-pop=1",
            "?label=O'Brien&req-x=1",
            "?amount=1,5",
            "?amount=1..",
            "?req-x=1",
            "?label=a)b&req-pop=1",
            "?amount=1)5",
            "?label=[x]&req-x=1",
            "?label=say\"hi\"&req-x=1",
            "?label=a<b>c&req-x=1",
        ] {
            assert!(find(&format!("bitcoin:{A}{tail}")).addresses.is_empty());
            assert!(find(&format!("BITCOIN:{A}{tail} {B}")).addresses == [B]);
        }
        // An address inside a label is text, not a listed address.
        let f = find(&format!("bitcoin:{A}?label=x,{B}"));
        assert_eq!(f.addresses, [A]);
        let f = find(&format!(
            "see (bitcoin:{A}?label=a(b)&amount=1) and \"{C}\""
        ));
        assert_eq!(f.addresses, [A, C]);
        // A list written without spaces gives each of its addresses.
        for list in [
            format!("[\"bitcoin:{A}\",\"bitcoin:{B}\"]"),
            format!("\"bitcoin:{A}\",\"{B}\""),
            format!("(bitcoin:{A}),({B})"),
        ] {
            assert_eq!(find(&list).addresses, [A, B], "{list}");
        }
        // Closing punctuation alone after a link is not more text.
        for one in [
            format!("[bitcoin:{A}]."),
            format!("\"bitcoin:{A}\","),
            format!("(bitcoin:{A})"),
        ] {
            assert_eq!(find(&one).addresses, [A], "{one}");
        }
        // Anything else after the cut still hides the run.
        assert!(
            find(&format!("bitcoin:{A}?label=\",&req-pop=1\""))
                .addresses
                .is_empty()
        );
    }

    #[test]
    fn bip21_example_link_with_a_bad_checksum_gives_nothing() {
        let link = "bitcoin:175tWpb8K1S7NmH4Zx6rewF9WQrcZv245W?amount=20.3&label=Luke-Jr";
        assert!(find(link).addresses.is_empty());
        assert!(
            find(&format!(
                "bitcoin:{B}?amount=20.3&label=Luke-Jr&message=Donation%20for%20project"
            ))
            .addresses
                == [B]
        );
    }

    #[test]
    fn one_bech32_address_in_two_cases_is_one() {
        let f = find(&format!("{A} {}", A.to_uppercase()));
        assert_eq!(f.addresses, [A]);
        assert_eq!(f.total, 1);
        let f = find(&format!("{} {A}", A.to_uppercase()));
        assert_eq!(f.addresses, [A.to_uppercase()]);
    }

    #[test]
    fn the_hundred_cap_keeps_the_first_and_counts_the_rest() {
        use bitcoin::hashes::Hash;
        use bitcoin::{Address, Network, PubkeyHash, ScriptBuf};
        let mut text = String::new();
        for i in 0..=150u8 {
            let s = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([i; 20]));
            text.push_str(
                &Address::from_script(&s, Network::Bitcoin)
                    .unwrap()
                    .to_string(),
            );
            text.push('\n');
        }
        let f = find(&text);
        assert_eq!(f.addresses.len(), MAX_ADDRESSES);
        assert_eq!(f.total, 151);
        assert_eq!(f.addresses[0], text.lines().next().unwrap());
        let exactly = text.lines().take(100).collect::<Vec<_>>().join("\n");
        assert_eq!(find(&exactly).total, 100);
    }
}
