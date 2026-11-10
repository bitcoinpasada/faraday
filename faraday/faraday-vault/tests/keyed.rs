//! A file vault that needs a key file or a YubiKey's secret
//! (`docs/VAULT.md` §3.2, F208): the key file forms KeePass reads, the
//! secret as typed, a vault made and sealed again with them, and pykeepass
//! (`tools/vectors/kdbx/keyed.py`) opening a file Faraday sealed and the
//! other way. The pykeepass checks are skipped, and said, where it is not
//! installed.

use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_crypto::work::{Compute, Work};
use faraday_vault::Cost;
use faraday_vault::file::{self, Error, Opened};
use faraday_vault::kdbx::keyfile::{self, KeyFileError, SecretError};
use faraday_vault::kdbx::{self, Components, RANDOM_LEN};

const PASSPHRASE: &str = "keyed vector passphrase";
const CHEAP: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

fn random(seed: u8) -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
}

fn key() -> [u8; 32] {
    std::array::from_fn(|i| i as u8)
}

const SECRET_HEX: &str = "000102030405060708090a0b0c0d0e0f10111213";

fn with_key() -> Components {
    Components::with_key_file(key().into())
}

fn with_secret() -> Components {
    Components::with_yubikey(keyfile::secret(SECRET_HEX).expect("40 digits"))
}

fn with_both() -> Components {
    Components::new(
        Some(key().into()),
        Some(keyfile::secret(SECRET_HEX).expect("40 digits")),
    )
}

fn make(components: &Components, random: &[u8; RANDOM_LEN]) -> Vec<u8> {
    file::create_holding(
        &compute(),
        [3; 16],
        "keyed",
        CHEAP,
        PASSPHRASE,
        components,
        random,
        Some(1_700_000_000),
        &[],
    )
    .expect("made")
}

fn open(bytes: &[u8], components: &Components) -> Result<Opened, Error> {
    file::open_with(
        &compute(),
        bytes,
        PASSPHRASE,
        components,
        &mut |_| Ok(()),
        &mut |_, _| Ok(None),
        None,
    )
}

fn is_wrong_credentials<T>(r: Result<T, Error>) -> bool {
    matches!(r, Err(Error::Kdbx(kdbx::Error::Passphrase)))
}

fn sha256(b: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    <[u8; 32]>::from(Sha256::digest(b))
}

fn xml(version: &str, data: &str, hash: Option<&str>) -> Vec<u8> {
    let hash = hash.map_or(String::new(), |h| format!(" Hash=\"{h}\""));
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>{version}</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data{hash}>{data}</Data>\n\t</Key>\n</KeyFile>\n"
    )
    .into_bytes()
}

const KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
/// The first four bytes of the SHA-256 of `key()`.
const KEY_HASH: &str = "630DCD29";

#[test]
fn every_form_of_key_file_gives_the_key_keepass_gives() {
    let read = |b: &[u8]| keyfile::read(b).map(|k| *k);
    assert_eq!(read(&xml("1.0", KEY_B64, None)), Ok(key()));
    assert_eq!(read(&xml("2.0", KEY_HEX, Some(KEY_HASH))), Ok(key()));
    // Hexadecimal with the white space KeePass writes, either case.
    let spaced = format!(
        "{} {}\n  {}",
        &KEY_HEX[..16],
        &KEY_HEX[16..32],
        &KEY_HEX[32..]
    )
    .to_uppercase();
    assert_eq!(
        read(&xml("2.0", &spaced, Some(&KEY_HASH.to_lowercase()))),
        Ok(key())
    );
    assert_eq!(read(&key()), Ok(key()));
    assert_eq!(read(KEY_HEX.as_bytes()), Ok(key()));
    assert_eq!(read(KEY_HEX.to_uppercase().as_bytes()), Ok(key()));
    // Any other file is hashed: SHA-256("this is a key file").
    let hashed = read(b"this is a key file").unwrap();
    assert_eq!(
        hashed
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "ea207ef24765a03f4601dfbc355f1675408129908ae220f826dd02d4de3323f5"
    );
}

#[test]
fn a_document_that_is_not_a_key_file_document_is_hashed_like_any_file() {
    let sha = sha256;
    for text in [
        "<KeyFile><Meta><Version>2.0</Version></Meta>",
        "<Other><Key/></Other>",
        "<!DOCTYPE Other><Other/>",
        "<KeyFile><Meta><Version>2.0</Version></Meta><Key><Data Hash=\"630DCD29\">00</Data></Key><x a=b/></KeyFile>",
        "<KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=</Data></Key><x>&foo;</x></KeyFile>",
        "plain text with <angle> brackets, not a key",
    ] {
        assert_eq!(
            *keyfile::read(text.as_bytes()).expect("hashed"),
            sha(text.as_bytes()),
            "{text}"
        );
    }
}

#[test]
fn a_key_file_the_three_readers_read_differently_is_refused() {
    let bad_hash = "00000000";
    let short = KEY_HEX[..62].to_string();
    for (name, file) in [
        ("hash fails", xml("2.0", KEY_HEX, Some(bad_hash))),
        ("no hash", xml("2.0", KEY_HEX, None)),
        ("not 32 bytes (v2)", xml("2.0", &short, Some(KEY_HASH))),
        ("not 32 bytes (v1)", xml("1.0", "AAECAwQFBgc=", None)),
        ("other version", xml("3.0", KEY_HEX, Some(KEY_HASH))),
        ("no key", b"<KeyFile><Meta><Version>1.0</Version></Meta></KeyFile>".to_vec()),
        ("bad base 64", xml("1.0", "!!!!", None)),
        (
            "two keys",
            format!(
                "<KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>{KEY_B64}</Data><Data>{KEY_B64}</Data></Key></KeyFile>"
            )
            .into_bytes(),
        ),
    ] {
        assert_eq!(keyfile::read(&file), Err(KeyFileError::Damaged), "{name}");
    }
    assert_eq!(keyfile::read(b""), Err(KeyFileError::Empty));
    assert_eq!(
        keyfile::read(&make(&Components::none(), &random(1))),
        Err(KeyFileError::Database)
    );
}

/// A key file document around a valid version 1 key, with `extra` before
/// `</KeyFile>`.
fn v1_with(extra: &str) -> String {
    format!(
        "<KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>{KEY_B64}</Data></Key>{extra}</KeyFile>"
    )
}

/// A key file document around a valid version 1 key, with `head` before
/// the root, the root element `root` and `tail` after it.
fn key_file(head: &str, root: &str, tail: &str) -> String {
    format!(
        "{head}<{root}><Meta><Version>1.0</Version></Meta><Key><Data>{KEY_B64}</Data></Key></KeyFile>{tail}"
    )
}

/// A key file document around a valid version 1 key as bytes, with `head`
/// before the root, `inner` before `</KeyFile>` and `tail` after it.
fn key_bytes(head: &[u8], inner: &[u8], tail: &[u8]) -> Vec<u8> {
    [
        head,
        b"<KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>",
        KEY_B64.as_bytes(),
        b"</Data></Key>",
        inner,
        b"</KeyFile>",
        tail,
    ]
    .concat()
}

/// `piece` before the root, inside it and after it, named for `what`.
fn everywhere(what: &str, piece: &[u8]) -> [(String, Vec<u8>); 3] {
    [
        (format!("{what} before"), key_bytes(piece, b"", b"")),
        (format!("{what} inside"), key_bytes(b"", piece, b"")),
        (format!("{what} after"), key_bytes(b"", b"", piece)),
    ]
}

// The tests below were checked against keepassxc-cli 2.7.10 and
// pykeepass 4.1.1 (the single quotes, the white space around `=`, the comment
// with a dash and a quoted root attribute, CDATA inside the root, the UTF-8
// byte order mark, the bytes XML does not allow, invalid UTF-8, the targets
// of processing instructions and the ISO-8859-15, latin1, windows-1252 and
// Shift_JIS names on ASCII files included; a `KeyFile` document with a byte
// above 127 under an encoding but UTF-8 is refused, whether the readers hash
// it or take it): a vault keyed by a passphrase and a version 2 key file,
// opened with a file holding the same key and the defect, then with the
// file's SHA-256 as the vault's key. Both opening with the key file: the XML
// key (the third test). Both only with the hash: hashed (the first). One
// each: refused (the second).

#[test]
fn a_document_not_well_formed_is_hashed_as_pykeepass_and_keepassxc_hash_it() {
    let decl = |d: &str| key_file(&format!("<?{d}?>"), "KeyFile", "");
    assert!(keyfile::read(v1_with("").as_bytes()).is_ok());
    for (name, file) in [
        (
            "repeated attribute on the root",
            key_file("", "KeyFile x=\"1\" x=\"2\"", ""),
        ),
        (
            "unquoted attribute on the root",
            key_file("", "KeyFile x=1", ""),
        ),
        ("empty root, unquoted attribute", "<KeyFile x=1/>".into()),
        ("unquoted attribute inside the root", v1_with("<a x=1/>")),
        ("declaration without quotes", decl("xml version=1.0")),
        (
            "declaration with a stray word",
            decl("xml version=\"1.0\" foo"),
        ),
        (
            "declaration after a comment",
            key_file("<!-- a --><?xml version=\"1.0\"?>", "KeyFile", ""),
        ),
        (
            "declaration inside the root",
            v1_with("<?xml version=\"1.0\"?>"),
        ),
        (
            "declaration twice",
            key_file(
                "<?xml version=\"1.0\"?><?xml version=\"1.0\"?>",
                "KeyFile",
                "",
            ),
        ),
        (
            "declaration after white space",
            format!(" {}", decl("xml version=\"1.0\"")),
        ),
        ("version 2.0", decl("xml version=\"2.0\"")),
        ("version 0.9", decl("xml version=\"0.9\"")),
        ("version 1.0 with a space", decl("xml version=\"1.0 \"")),
        ("version 1.x", decl("xml version=\"1.x\"")),
        ("version 1.0.0", decl("xml version=\"1.0.0\"")),
        (
            "standalone maybe",
            decl("xml version=\"1.0\" standalone=\"maybe\""),
        ),
        (
            "standalone YES",
            decl("xml version=\"1.0\" standalone=\"YES\""),
        ),
        (
            "standalone before encoding",
            decl("xml version=\"1.0\" standalone=\"yes\" encoding=\"UTF-8\""),
        ),
        (
            "encoding UTF-16",
            decl("xml version=\"1.0\" encoding=\"UTF-16\""),
        ),
        (
            "encoding UTF-32",
            decl("xml version=\"1.0\" encoding=\"UTF-32\""),
        ),
        (
            "encoding no reader knows",
            decl("xml version=\"1.0\" encoding=\"foo\""),
        ),
        ("target XML", decl("XML version=\"1.0\"")),
        ("target Xml", decl("Xml version=\"1.0\"")),
        ("empty target", decl(" x")),
        ("empty target inside the root", v1_with("<? x?>")),
        ("target XML inside the root", v1_with("<?XML x?>")),
        (
            "-- inside a comment",
            key_file("<!-- a -- b -->", "KeyFile", ""),
        ),
        (
            "comment ending in a dash",
            key_file("<!-- a --->", "KeyFile", ""),
        ),
        (
            "-- inside a comment in the root",
            v1_with("<!-- a -- b -->"),
        ),
        (
            "comment in the root ending in a dash",
            v1_with("<!-- a --->"),
        ),
    ] {
        assert_eq!(
            *keyfile::read(file.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e:?}")),
            sha256(file.as_bytes()),
            "{name}"
        );
    }
}

#[test]
fn markup_after_the_root_or_a_declaration_one_reader_reads_differently_is_refused() {
    let decl = |d: &str| key_file(&format!("<?{d}?>"), "KeyFile", "");
    for (name, file) in [
        (
            "comment with --",
            key_file("", "KeyFile", "<!-- a -- b -->"),
        ),
        (
            "comment ending in a dash",
            key_file("", "KeyFile", "<!-- a --->"),
        ),
        ("comment not closed", key_file("", "KeyFile", "<!-- a")),
        (
            "bad declaration",
            key_file("", "KeyFile", "<?xml version=1.0?>"),
        ),
        (
            "late declaration",
            key_file("", "KeyFile", "<?xml version=\"1.0\"?>"),
        ),
        (
            "processing instruction, empty target",
            key_file("", "KeyFile", "<? x?>"),
        ),
        (
            "processing instruction, target XML",
            key_file("", "KeyFile", "<?XML x?>"),
        ),
        (
            "processing instruction not closed",
            key_file("", "KeyFile", "<?pi x"),
        ),
        ("element", key_file("", "KeyFile", "<a/>")),
        (
            "second root",
            key_file("", "KeyFile", &key_file("", "KeyFile", "")),
        ),
        ("end tag", key_file("", "KeyFile", "</a>")),
        ("text", key_file("", "KeyFile", "junk")),
        ("reference", key_file("", "KeyFile", "&amp;")),
        ("less-than sign", key_file("", "KeyFile", "<")),
        ("doctype", key_file("", "KeyFile", "<!DOCTYPE a>")),
        ("version 1.1", decl("xml version=\"1.1\"")),
        ("version 1.9", decl("xml version=\"1.9\"")),
        ("version 1.", decl("xml version=\"1.\"")),
    ] {
        assert_eq!(
            keyfile::read(file.as_bytes()),
            Err(KeyFileError::Damaged),
            "{name}"
        );
    }
}

#[test]
fn a_declaration_comment_or_instruction_both_readers_take_leaves_the_xml_key() {
    let decl = |d: &str| key_file(&format!("<?{d}?>"), "KeyFile", "");
    for (name, file) in [
        (
            "declaration with everything",
            decl("xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\""),
        ),
        (
            "standalone no",
            decl("xml version=\"1.0\" standalone=\"no\""),
        ),
        ("single quotes", decl("xml version='1.0'")),
        ("white space around =", decl("xml version = \"1.0\" ")),
        (
            "encoding ISO-8859-1",
            decl("xml version=\"1.0\" encoding=\"ISO-8859-1\""),
        ),
        (
            "encoding US-ASCII",
            decl("xml version=\"1.0\" encoding=\"US-ASCII\""),
        ),
        (
            "encoding utf8",
            decl("xml version=\"1.0\" encoding=\"utf8\""),
        ),
        (
            "comment with a dash",
            key_file("<!-- a - b -->", "KeyFile x=\"1\"", ""),
        ),
        (
            "target xml-stylesheet",
            key_file("<?xml-stylesheet a?>", "KeyFile", ""),
        ),
        ("instruction before", key_file("<?pi x?>", "KeyFile", "")),
        ("instruction inside the root", v1_with("<?pi x?>")),
        ("comment after", key_file("", "KeyFile", "<!-- ok -->")),
        ("instruction after", key_file("", "KeyFile", "<?pi x?>")),
        ("white space after", key_file("", "KeyFile", "\n \n")),
        (
            "cdata inside the root, outside the key",
            v1_with("<![CDATA[x]]>"),
        ),
        ("byte order mark", format!("\u{feff}{}", v1_with(""))),
    ] {
        assert_eq!(*keyfile::read(file.as_bytes()).unwrap(), key(), "{name}");
    }
}

#[test]
fn bytes_xml_does_not_allow_before_or_inside_the_root_are_hashed_after_it_refused() {
    let hashed = |what: &str, file: &[u8]| {
        assert_eq!(
            *keyfile::read(file).unwrap_or_else(|e| panic!("{what}: {e:?}")),
            sha256(file),
            "{what}"
        );
    };
    for (what, piece) in [
        ("control character in a comment", &b"<!-- \x01 -->"[..]),
        ("NUL in a comment", b"<!-- \x00 -->"),
        ("form feed in a comment", b"<!-- \x0c -->"),
        ("U+FFFE in a comment", "<!-- \u{fffe} -->".as_bytes()),
        ("U+FFFF in a comment", "<!-- \u{ffff} -->".as_bytes()),
        (
            "control character in a processing instruction",
            b"<?p x\x01y?>",
        ),
        ("processing instruction target 1x", b"<?1x?>"),
        ("processing instruction target a\"b\"", b"<?a\"b\"?>"),
        ("processing instruction target -a", b"<?-a?>"),
        ("processing instruction target a:b", b"<?a:b?>"),
        ("processing instruction target a'b'", b"<?a'b'?>"),
        ("processing instruction target a/", b"<?a/?>"),
    ] {
        let [before, inside, after] = everywhere(what, piece);
        hashed(&before.0, &before.1);
        hashed(&inside.0, &inside.1);
        assert_eq!(
            keyfile::read(&after.1),
            Err(KeyFileError::Damaged),
            "{}",
            after.0
        );
    }
    // Not valid UTF-8, declared or not: both readers hash it wherever it is.
    for decl in [&b""[..], b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"] {
        for (what, piece) in [
            ("0xff", &b"<!-- \xff -->"[..]),
            ("lone continuation byte", b"<!-- \x80 -->"),
            ("overlong", b"<!-- \xc0\xaf -->"),
            ("truncated", b"<!-- \xe4\xb8 -->"),
            ("surrogate", b"<!-- \xed\xa0\x80 -->"),
            ("beyond U+10FFFF", b"<!-- \xf4\x90\x80\x80 -->"),
        ] {
            for (name, file) in everywhere(what, piece) {
                let file = [decl, &file].concat();
                hashed(&name, &file);
            }
        }
    }
}

#[test]
fn a_character_xml_does_not_allow_where_one_reader_stops_reading_is_refused() {
    for (what, piece) in [
        (
            "processing instruction target with a control character",
            &b"<?p\x01?>"[..],
        ),
        ("processing instruction target with DEL", b"<?p\x7f?>"),
        (
            "processing instruction target of a middle dot",
            "<?\u{b7}a?>".as_bytes(),
        ),
        (
            "processing instruction target of a multiplication sign",
            "<?\u{d7}?>".as_bytes(),
        ),
    ] {
        for (name, file) in everywhere(what, piece) {
            assert_eq!(keyfile::read(&file), Err(KeyFileError::Damaged), "{name}");
        }
    }
    // The last character of the file cut short, after the root.
    assert_eq!(
        keyfile::read(&key_bytes(b"", b"", b"\xe9")),
        Err(KeyFileError::Damaged)
    );
}

#[test]
fn a_processing_instruction_target_with_a_high_byte_under_an_encoding_is_refused() {
    let decl = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>";
    for (name, file) in everywhere("a Latin-1 target", b"<?\xe9 x?>") {
        let file = [&decl[..], &file].concat();
        assert_eq!(keyfile::read(&file), Err(KeyFileError::Damaged), "{name}");
    }
}

#[test]
fn a_high_byte_is_hashed_with_another_root_under_latin_1_and_in_any_document_declared_utf_16() {
    // Run with keepassxc-cli 2.7.10 and pykeepass 4.1.1: both open a vault
    // keyed by the file's SHA-256.
    for file in [
        &b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><config>caf\xe9</config>"[..],
        &b"<?xml version=\"1.0\" encoding=\"UTF-16\"?><KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>AAAA</Data></Key></KeyFile><!-- \xe9 -->"[..],
    ] {
        assert_eq!(keyfile::read(file).map(|k| *k), Ok(sha256(file)));
    }
}

#[test]
fn a_shift_jis_document_is_refused_once_its_root_is_read_whatever_follows() {
    // A Shift_JIS reader reads `\x81]` as one character, so the CDATA
    // section ends later than quick-xml ends it.
    let decl = b"<?xml version=\"1.0\" encoding=\"Shift_JIS\"?>";
    for inner in [
        &b"<![CDATA[\x81]]><!-- -- -->]]>"[..],
        b"<![CDATA[\x81]]><?xml x?>]]>",
        b"<![CDATA[\x81]]></a>]]>",
    ] {
        let file = key_bytes(decl, inner, b"");
        assert_eq!(keyfile::read(&file), Err(KeyFileError::Damaged));
    }
}

#[test]
fn a_target_both_readers_refuse_is_hashed_before_a_root_whatever_its_high_byte() {
    // Run with keepassxc-cli 2.7.10 and pykeepass 4.1.1: both open a vault
    // keyed by the first file's SHA-256. The second's target starts with a
    // digit, which both readers refuse as they do an ASCII one.
    let decl = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>";
    for pi in [&b"<?a!\xe9 x?>"[..], b"<?1\xe9 x?>"] {
        let file = key_bytes(&[decl, pi].concat(), b"", b"");
        assert_eq!(keyfile::read(&file).map(|k| *k), Ok(sha256(&file)));
    }
}

#[test]
fn a_second_byte_order_mark_hashes_the_file_and_one_leaves_the_rest_checked() {
    // Run with keepassxc-cli 2.7.10 and pykeepass 4.1.1: both open a vault
    // keyed by the file's SHA-256 and neither one keyed by the XML key.
    let bom = b"\xef\xbb\xbf";
    let one = [&bom[..], &key_bytes(b"", b"", b"")].concat();
    assert_eq!(keyfile::read(&one).map(|k| *k), Ok(key()));
    let two = [&bom[..], &one].concat();
    assert_eq!(keyfile::read(&two).map(|k| *k), Ok(sha256(&two)));
    // A character XML does not allow in the last bytes after the root.
    let tail = [&bom[..], &key_bytes(b"", b"", b"<!--\x01-->")].concat();
    assert_eq!(keyfile::read(&tail), Err(KeyFileError::Damaged));
}

#[test]
fn a_byte_above_127_under_an_encoding_but_utf_8_is_refused() {
    for enc in [
        "ISO-8859-1",
        "ISO-8859-15",
        "latin1",
        "windows-1252",
        "Shift_JIS",
        "US-ASCII",
        "ascii",
    ] {
        let decl = format!("<?xml version=\"1.0\" encoding=\"{enc}\"?>");
        for (what, piece) in [
            ("a Latin-1 letter", &b"<!-- \xe9 -->"[..]),
            ("a UTF-8 letter", "<!-- \u{e9} -->".as_bytes()),
        ] {
            for (name, file) in everywhere(what, piece) {
                let file = [decl.as_bytes(), &file].concat();
                assert_eq!(
                    keyfile::read(&file),
                    Err(KeyFileError::Damaged),
                    "{enc}: {name}"
                );
            }
        }
        let cdata = key_bytes(decl.as_bytes(), b"<![CDATA[\x81]]>", b"");
        assert_eq!(
            keyfile::read(&cdata),
            Err(KeyFileError::Damaged),
            "{enc}: a Shift_JIS lead byte in CDATA"
        );
    }
}

#[test]
fn names_and_characters_both_readers_take_leave_the_xml_key() {
    for enc in ["ISO-8859-15", "latin1", "windows-1252", "Shift_JIS"] {
        let file = key_file(
            &format!("<?xml version=\"1.0\" encoding=\"{enc}\"?>"),
            "KeyFile",
            "",
        );
        assert_eq!(*keyfile::read(file.as_bytes()).unwrap(), key(), "{enc}");
    }
    for (what, piece) in [
        (
            "UTF-8 letters in a comment",
            "<!-- \u{e9}\u{4e2d} -->".as_bytes(),
        ),
        ("tab, line feed and carriage return", b"<!--\t\r\n-->"),
        ("U+FFFD", "<!--\u{fffd}-->".as_bytes()),
        ("the last character", "<!--\u{10ffff}-->".as_bytes()),
        ("target a", b"<?a?>"),
        ("target of a letter with an accent", "<?\u{e9}?>".as_bytes()),
        ("target a-b.c1", b"<?a-b.c1?>"),
        ("target _a", b"<?_a?>"),
        ("target of a Chinese character", "<?\u{4e2d}?>".as_bytes()),
        ("target ending in a middle dot", "<?a\u{b7}?>".as_bytes()),
        ("target with data", b"<?a b?>"),
    ] {
        for (name, file) in everywhere(what, piece) {
            assert_eq!(*keyfile::read(&file).unwrap(), key(), "{name}");
        }
    }
}

#[test]
fn a_key_file_nested_deeper_than_a_vault_is_refused() {
    let deep = format!("{}{}", "<a>".repeat(100), "</a>".repeat(100));
    assert_eq!(
        keyfile::read(v1_with(&deep).as_bytes()),
        Err(KeyFileError::Damaged)
    );
    let unclosed = format!("<KeyFile>{}", "<a>".repeat(100_000));
    assert_eq!(
        keyfile::read(unclosed.as_bytes()),
        Err(KeyFileError::Damaged)
    );
}

#[test]
fn markup_inside_the_key_or_the_version_is_refused() {
    let data = format!(
        "<KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>{}<b>{}</b></Data></Key></KeyFile>",
        &KEY_B64[..20],
        &KEY_B64[20..]
    );
    assert_eq!(keyfile::read(data.as_bytes()), Err(KeyFileError::Damaged));
    let version = format!(
        "<KeyFile><Meta><Version>1.<b>0</b></Version></Meta><Key><Data>{KEY_B64}</Data></Key></KeyFile>"
    );
    assert_eq!(
        keyfile::read(version.as_bytes()),
        Err(KeyFileError::Damaged)
    );
}

#[test]
fn a_document_a_strict_reader_rejects_is_refused_where_another_takes_it() {
    // Refused whole: KeePassXC reads these as XML, a strict parser does not.
    for (name, file) in [
        (
            "cdata after the root",
            format!("{}<![CDATA[x]]>", v1_with("")),
        ),
        ("doctype", format!("<!DOCTYPE KeyFile>{}", v1_with(""))),
        (
            "doctype with entities",
            format!("<!DOCTYPE KeyFile [<!ENTITY a \"b\">]>{}", v1_with("")),
        ),
    ] {
        assert_eq!(
            keyfile::read(file.as_bytes()),
            Err(KeyFileError::Damaged),
            "{name}"
        );
    }
}

#[test]
fn a_utf_16_key_file_document_is_refused() {
    let text = String::from_utf8(xml("2.0", KEY_HEX, Some(KEY_HASH))).unwrap();
    for (name, le) in [("little endian", true), ("big endian", false)] {
        let mut file = if le {
            vec![0xFF, 0xFE]
        } else {
            vec![0xFE, 0xFF]
        };
        for u in text.encode_utf16() {
            file.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        assert_eq!(keyfile::read(&file), Err(KeyFileError::Damaged), "{name}");
    }
    // 32 bytes that happen to begin with a byte order mark are the key.
    let mut raw = [7u8; 32];
    raw[..2].copy_from_slice(&[0xFF, 0xFE]);
    assert_eq!(*keyfile::read(&raw).unwrap(), raw);
}

#[test]
fn only_the_versions_keepassxc_reads_are_taken() {
    for version in [" 2.0 ", "2.00", "2.0\n", "1.0 ", " 1.00", "2.1"] {
        let file = if version.trim().starts_with('2') {
            xml(version, KEY_HEX, Some(KEY_HASH))
        } else {
            xml(version, KEY_B64, None)
        };
        assert_eq!(
            keyfile::read(&file),
            Err(KeyFileError::Damaged),
            "{version:?}"
        );
    }
    assert_eq!(
        keyfile::read(&xml("1.00", KEY_B64, None)).map(|k| *k),
        Ok(key())
    );
}

#[test]
fn a_secret_is_forty_digits_however_a_tool_prints_them() {
    let want = *keyfile::secret(SECRET_HEX).unwrap();
    assert_eq!(want[0], 0x00);
    assert_eq!(want[19], 0x13);
    for form in [
        SECRET_HEX.to_uppercase(),
        "00010203 04050607 08090a0b 0c0d0e0f 10111213".to_string(),
        "00:01:02:03:04:05:06:07:08:09:0a:0b:0c:0d:0e:0f:10:11:12:13".to_string(),
    ] {
        assert_eq!(*keyfile::secret(&form).unwrap(), want, "{form}");
    }
    assert_eq!(
        keyfile::secret(&SECRET_HEX[..38]).err(),
        Some(SecretError {
            digits: 38,
            stray: false
        })
    );
    assert_eq!(
        keyfile::secret(&format!("{SECRET_HEX}00")).err(),
        Some(SecretError {
            digits: 42,
            stray: false
        })
    );
    assert!(
        keyfile::secret(&format!("{}g", &SECRET_HEX[..39]))
            .unwrap_err()
            .stray
    );
}

#[test]
fn a_vault_with_a_key_file_or_a_secret_opens_only_with_what_it_was_made_with() {
    for made_with in [with_key(), with_secret(), with_both()] {
        let bytes = make(&made_with, &random(2));
        assert!(open(&bytes, &made_with).is_ok());
        assert!(is_wrong_credentials(open(&bytes, &Components::none())));
        for other in [with_key(), with_secret(), with_both()] {
            let same = other.has_key_file() == made_with.has_key_file()
                && other.has_yubikey() == made_with.has_yubikey();
            assert_eq!(open(&bytes, &other).is_ok(), same);
        }
    }
    // The same passphrase alone is a different vault.
    assert!(is_wrong_credentials(open(
        &make(&Components::none(), &random(2)),
        &with_key()
    )));
}

#[test]
fn a_seal_keeps_what_the_vault_was_opened_with() {
    for components in [with_key(), with_secret(), with_both()] {
        let first = make(&components, &random(3));
        let opened = open(&first, &components).unwrap();
        let second = file::seal(&compute(), &opened, &[], &random(4), Some(1_700_000_100)).unwrap();
        assert!(open(&second, &components).is_ok());
        assert!(is_wrong_credentials(open(&second, &Components::none())));
    }
}

#[test]
fn a_salt_ending_0x20_is_changed_to_0x21_only_when_a_secret_is_held() {
    let mut r = random(5);
    r[44 + 31] = 0x20;
    let salt = |bytes: &[u8]| {
        let want = &r[44..44 + 31];
        let at = bytes
            .windows(31)
            .position(|w| w == want)
            .expect("the salt is in the header");
        bytes[at + 31]
    };
    assert_eq!(salt(&make(&Components::none(), &r)), 0x20);
    assert_eq!(salt(&make(&with_key(), &r)), 0x20);
    let kept = make(&with_secret(), &r);
    assert_eq!(salt(&kept), 0x21);
    assert!(open(&kept, &with_secret()).is_ok());
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// `keyed.py` with `args`: its output, or none where pykeepass is not
/// installed (said, not silent).
fn keyed_py(args: &[&str]) -> Option<std::process::Output> {
    let out = Command::new("python3")
        .args(["-I", "-B"])
        .arg(repo("tools/vectors/kdbx/keyed.py"))
        .args(args)
        .output()
        .ok()?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return None;
    }
    Some(out)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-keyed-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn key_file_v2(dir: &Path) -> PathBuf {
    let path = dir.join("vault.keyx");
    std::fs::write(&path, xml("2.0", KEY_HEX, Some(KEY_HASH))).unwrap();
    path
}

#[test]
fn pykeepass_opens_a_vault_faraday_sealed_with_a_key_file_and_the_other_way() {
    let dir = scratch("keyfile");
    let key_path = key_file_v2(&dir);
    let key_arg = key_path.to_str().unwrap();
    let sealed = dir.join("faraday.kdbx");
    std::fs::write(&sealed, make(&with_key(), &random(6))).unwrap();
    let Some(out) = keyed_py(&[
        "read",
        sealed.to_str().unwrap(),
        PASSPHRASE,
        "--keyfile",
        key_arg,
    ]) else {
        return;
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let alone = keyed_py(&["read", sealed.to_str().unwrap(), PASSPHRASE]).unwrap();
    assert!(!alone.status.success());

    // A file pykeepass made under the same key file, through the key file as
    // Faraday reads it.
    let theirs = dir.join("pykeepass.kdbx");
    let made = keyed_py(&[
        "make",
        theirs.to_str().unwrap(),
        PASSPHRASE,
        "--keyfile",
        key_arg,
    ])
    .unwrap();
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let key = keyfile::read(&std::fs::read(&key_path).unwrap()).unwrap();
    let components = Components::with_key_file(key);
    let bytes = std::fs::read(&theirs).unwrap();
    assert!(open(&bytes, &components).is_ok());
    assert!(is_wrong_credentials(open(&bytes, &Components::none())));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pykeepass_opens_a_vault_faraday_sealed_under_a_secret_and_the_other_way() {
    let dir = scratch("secret");
    let sealed = dir.join("faraday.kdbx");
    // The salts end in other bytes than the padding byte's 0x20, which no
    // outside value covers: the padding strip is checked against keyed.py,
    // Faraday's own script, only in a_salt_ending_0x20_is_changed_to_0x21.
    for (n, both) in [(7u8, false), (8, true)] {
        let components = if both { with_both() } else { with_secret() };
        std::fs::write(&sealed, make(&components, &random(n))).unwrap();
        let mut args = vec![
            "read",
            sealed.to_str().unwrap(),
            PASSPHRASE,
            "--secret",
            SECRET_HEX,
        ];
        if both {
            args.extend(["--key", KEY_HEX]);
        }
        let Some(out) = keyed_py(&args) else {
            return;
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    for (both, components) in [(false, with_secret()), (true, with_both())] {
        let theirs = dir.join("pykeepass.kdbx");
        let mut args = vec![
            "make",
            theirs.to_str().unwrap(),
            PASSPHRASE,
            "--secret",
            SECRET_HEX,
        ];
        if both {
            args.extend(["--key", KEY_HEX]);
        }
        let made = keyed_py(&args).unwrap();
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        let bytes = std::fs::read(&theirs).unwrap();
        assert_eq!(open(&bytes, &components).err(), None, "both {both}");
        assert!(is_wrong_credentials(open(&bytes, &Components::none())));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/kdbx")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The passphrase of the keepassxc-cli vectors (`tools/vectors/kdbx/README.md`).
const KEEPASSXC_PASSPHRASE: &str = "kdbx vector passphrase";

fn opens_to_example_and_other(bytes: &[u8], passphrase: &str, components: &Components) {
    let o = open_as(bytes, passphrase, components).expect("opens");
    let mut titles: Vec<&str> = o
        .document()
        .root()
        .entries()
        .filter_map(|e| e.field("Title").map(|f| f.value()))
        .collect();
    titles.sort_unstable();
    assert_eq!(titles, ["Example", "Other"]);
}

fn keepassxc_key() -> Components {
    let key = keyfile::read(&vector("keepassxc.keyx")).expect("a key file");
    Components::with_key_file(key)
}

#[test]
fn a_vault_keepassxc_locked_with_a_passphrase_and_a_key_file_opens_with_both() {
    let bytes = vector("keepassxc-keyfile-password.kdbx");
    opens_to_example_and_other(&bytes, KEEPASSXC_PASSPHRASE, &keepassxc_key());
    assert!(is_wrong_credentials(open_as(
        &bytes,
        KEEPASSXC_PASSPHRASE,
        &Components::none()
    )));
    assert!(is_wrong_credentials(open_as(&bytes, "b", &keepassxc_key())));
    assert!(is_wrong_credentials(open_as(
        &bytes,
        KEEPASSXC_PASSPHRASE,
        &with_key()
    )));
}

/// A vault has a passphrase (`docs/VAULT.md` section 3.2): one keepassxc-cli
/// locked with its key file alone is refused for having none, and a
/// passphrase typed with that key file does not open it.
#[test]
fn a_vault_keepassxc_locked_with_a_key_file_alone_is_refused_for_having_no_passphrase() {
    let bytes = vector("keepassxc-keyfile-only.kdbx");
    let refused = open_as(&bytes, "", &keepassxc_key());
    assert!(matches!(
        refused,
        Err(Error::Kdbx(kdbx::Error::EmptyPassphrase))
    ));
    assert!(is_wrong_credentials(open_as(
        &bytes,
        PASSPHRASE,
        &keepassxc_key()
    )));
}

/// kdbxweb (KeeWeb's library) made it with a function standing in for the
/// YubiKey: HMAC-SHA1 under `SECRET_HEX`.
#[test]
fn a_vault_kdbxweb_locked_with_a_passphrase_and_a_yubikey_opens_with_its_secret() {
    let bytes = vector("kdbxweb-yubikey-password.kdbx");
    opens_to_example_and_other(&bytes, PASSPHRASE, &with_secret());
    assert!(is_wrong_credentials(open_as(
        &bytes,
        PASSPHRASE,
        &Components::none()
    )));
    assert!(is_wrong_credentials(open_as(
        &bytes,
        PASSPHRASE,
        &with_key()
    )));
}

fn open_as(bytes: &[u8], passphrase: &str, components: &Components) -> Result<Opened, Error> {
    file::open_with(
        &compute(),
        bytes,
        passphrase,
        components,
        &mut |_| Ok(()),
        &mut |_, _| Ok(None),
        None,
    )
}
