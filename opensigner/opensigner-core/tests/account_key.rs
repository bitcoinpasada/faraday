//! A key's public accounts, for a coordinator (`docs/PLANNING.md`
//! §16.110 rule 1): the Account key row on a key's page, the Choice
//! that asks which account, and what each account is handed over as.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE};
use opensigner_core::{ExportFormat, ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, MultisigScriptType, Network, ScriptType};
use osk_shell_api::{Command, Event, FileKind};

/// The accounts the Choice offers, in its order.
const LEGACY: usize = 0;
const NESTED: usize = 1;
const SEGWIT: usize = 2;
const TAPROOT: usize = 3;
const NESTED_MULTISIG: usize = 4;
const SEGWIT_MULTISIG: usize = 5;

/// Network rows of the Settings Choice, in `Network::ALL` order.
const MAINNET: usize = 0;
const REGTEST: usize = 3;

fn master(network: Network) -> MasterKey {
    let m = Mnemonic::parse(Language::English, &ABANDON.join(" ")).expect("the test words");
    MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), network)
}

/// `[73c5da0a/84h/0h/0h]xpub…`: what a coordinator is handed.
fn origin(
    fingerprint: osk_bip::keys::Fingerprint,
    path: &osk_bip::keys::DerivationPath,
    xpub: &osk_bip::keys::Xpub,
) -> String {
    let mut text = format!("[{fingerprint}");
    for child in path {
        text.push_str(&format!("/{child:#}"));
    }
    text.push_str(&format!("]{xpub}"));
    text
}

/// The account key of row `row` of the Choice, derived here.
fn expected(network: Network, row: usize) -> String {
    let m = master(network);
    match row {
        NESTED_MULTISIG | SEGWIT_MULTISIG => {
            let script = match row {
                NESTED_MULTISIG => MultisigScriptType::NestedSegwit,
                _ => MultisigScriptType::NativeSegwit,
            };
            let a = m.multisig_account_xpub(script, 0).expect("the account");
            origin(a.master_fingerprint(), a.path(), a.xpub())
        }
        _ => {
            let script = ScriptType::ALL[row];
            let a = m.account_xpub(script, 0).expect("the account");
            origin(a.master_fingerprint(), a.path(), a.xpub())
        }
    }
}

/// A device with the test key loaded, on `network`.
fn device(network_row: usize) -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    if network_row != MAINNET {
        h.set_network(network_row);
    }
    h
}

/// The key's page › Account key › row `row` › Continue.
fn open_account(h: &mut Harness, row: usize) {
    h.open_key(0);
    h.tap(ids::DETAIL_ACCOUNT);
    h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Export);
}

/// Checks the format at `format` and returns what the screen now shows.
fn in_format(h: &mut Harness, format: ExportFormat) -> String {
    let row = ExportFormat::ALL
        .iter()
        .position(|f| *f == format)
        .expect("the format is in the list");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
    h.app.export_string().expect("the string")
}

/// The row is on every loaded key's page, and it opens the question
/// with a row per account and the path under each name.
#[test]
fn a_keys_page_offers_its_accounts_with_their_paths() {
    let s = &strings::EN;
    let mut h = device(MAINNET);
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert!(
        h.app.texts().iter().any(|t| t == s.key_account_row),
        "the Account key row: {:?}",
        h.app.texts()
    );

    h.tap(ids::DETAIL_ACCOUNT);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(s.account_which_title), "the question: {texts:?}");
    for name in [
        s.account_legacy,
        s.account_nested,
        s.account_segwit,
        s.account_taproot,
        s.account_multisig_nested,
        s.account_multisig_segwit,
    ] {
        assert!(says(name), "{name} is a row: {texts:?}");
    }
    for path in ["m/44h/0h/0h", "m/84h/0h/0h", "m/48h/0h/0h/2h"] {
        assert!(says(path), "{path} is under its name: {texts:?}");
    }
}

/// Every account is handed over as the key the coordinator asks for,
/// with the origin, on the network the device is set to.
#[test]
fn every_account_exports_the_key_at_its_own_path() {
    for (row, network) in [(MAINNET, Network::Mainnet), (REGTEST, Network::Regtest)] {
        for account in [
            LEGACY,
            NESTED,
            SEGWIT,
            TAPROOT,
            NESTED_MULTISIG,
            SEGWIT_MULTISIG,
        ] {
            let mut h = device(row);
            open_account(&mut h, account);
            assert_eq!(
                h.app.export_string().as_deref(),
                Some(expected(network, account).as_str()),
                "account {account} on {network}"
            );
        }
    }
}

/// SLIP-132 is a single-signature form and BIP 129's key record is a
/// multisig one; neither is offered where it says nothing.
#[test]
fn the_formats_offered_follow_the_account() {
    let s = &strings::EN;
    let mut h = device(MAINNET);
    open_account(&mut h, SEGWIT);
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.export_slip132),
        "SLIP-132 on a single-signature account: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == s.export_bsms_signer),
        "no key record there: {texts:?}"
    );
    h.tap(ids::BACK);
    // The SLIP-132 form of the SegWit account is the zpub every
    // coordinator that asks for one expects.
    assert!(
        in_format(&mut h, ExportFormat::Slip132).starts_with("zpub"),
        "the SLIP-132 form"
    );

    let mut h = device(MAINNET);
    open_account(&mut h, SEGWIT_MULTISIG);
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.export_bsms_signer),
        "the key record on a BIP-48 account: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == s.export_slip132),
        "no SLIP-132 there: {texts:?}"
    );

    // Taproot has no SLIP-132 prefix and never will.
    let mut h = device(MAINNET);
    open_account(&mut h, TAPROOT);
    h.tap(ids::EXPORT_FORMAT);
    assert!(
        !h.app.texts().iter().any(|t| t == s.export_slip132),
        "no SLIP-132 for Taproot: {:?}",
        h.app.texts()
    );
}

/// An account key's code saved as a PNG is a picture a QR reader reads
/// back as the account key (`docs/PLANNING.md` §16.134 rules 4 and 5),
/// and the screen says where it went.
#[test]
fn an_account_keys_code_saves_as_a_png_that_reads_back() {
    let s = &strings::EN;
    // On the phone: the panel's code screen has no room for the row
    // beside the Animated toggle (§16.134).
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    open_account(&mut h, SEGWIT);
    let shown = h.app.export_string().expect("the string");
    h.tap(ids::EXPORT_SHOW);
    assert!(
        h.app.texts().iter().any(|t| t == s.action_save_png),
        "the row: {:?}",
        h.app.texts()
    );
    h.seen.clear();
    h.tap(ids::SAVE_PNG);
    let (kind, name, png) = h
        .seen
        .iter()
        .find_map(|c| match c {
            Command::WriteFile {
                kind,
                name_hint,
                bytes,
            } => Some((*kind, name_hint.clone(), bytes.clone())),
            _ => None,
        })
        .expect("the picture was handed to the shell");
    assert_eq!(kind, FileKind::Png);
    assert!(name.ends_with(".png"), "{name}");

    let (width, height, luma) = read_png(&png);
    assert_eq!(width, height, "a square");
    let decoded = osk_codec::decode_luma(width, height, &luma);
    assert_eq!(decoded.len(), 1, "one code in the picture");
    assert_eq!(decoded[0].bytes, shown.as_bytes());
    cross_check_png(&png);

    h.send(Event::FileWritten { kind });
    assert!(
        h.app.texts().iter().any(|t| t.contains(&name)),
        "the screen says the name: {:?}",
        h.app.texts()
    );
}

/// A one-bit greyscale PNG of stored deflate blocks, as a width, a
/// height and 8-bit luma. Every chunk's CRC and the stream's Adler-32
/// are checked on the way.
fn read_png(png: &[u8]) -> (usize, usize, Vec<u8>) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "the signature");
    let mut at = 8;
    let (mut width, mut height) = (0usize, 0usize);
    let mut idat = Vec::new();
    loop {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().expect("4")) as usize;
        let kind = &png[at + 4..at + 8];
        let data = &png[at + 8..at + 8 + len];
        let crc = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().expect("4"));
        assert_eq!(crc, crc32(&png[at + 4..at + 8 + len]), "the chunk's CRC");
        match kind {
            b"IHDR" => {
                width = u32::from_be_bytes(data[0..4].try_into().expect("4")) as usize;
                height = u32::from_be_bytes(data[4..8].try_into().expect("4")) as usize;
                assert_eq!(&data[8..13], &[1, 0, 0, 0, 0], "one-bit grey");
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    // zlib: the header, stored blocks, the Adler-32.
    assert_eq!((u16::from(idat[0]) << 8 | u16::from(idat[1])) % 31, 0);
    let mut raw = Vec::new();
    let mut i = 2;
    loop {
        let header = idat[i];
        assert_eq!(header >> 1, 0, "a stored block");
        let len = u16::from_le_bytes([idat[i + 1], idat[i + 2]]) as usize;
        let nlen = u16::from_le_bytes([idat[i + 3], idat[i + 4]]) as usize;
        assert_eq!(len ^ 0xFFFF, nlen);
        raw.extend_from_slice(&idat[i + 5..i + 5 + len]);
        i += 5 + len;
        if header & 1 == 1 {
            break;
        }
    }
    let adler = u32::from_be_bytes(idat[i..i + 4].try_into().expect("4"));
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    assert_eq!(adler, (b << 16) | a, "the Adler-32");

    let row = width.div_ceil(8) + 1;
    assert_eq!(raw.len(), row * height);
    let mut luma = vec![0u8; width * height];
    for y in 0..height {
        assert_eq!(raw[y * row], 0, "no filter");
        for x in 0..width {
            let bit = raw[y * row + 1 + x / 8] & (0x80 >> (x % 8));
            luma[y * width + x] = if bit == 0 { 0 } else { 255 };
        }
    }
    (width, height, luma)
}

/// CRC-32 as PNG and zlib define it.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The same file through Python's zlib, where there is a Python to run
/// it: every chunk's CRC and the inflated rows agree with another
/// implementation.
fn cross_check_png(png: &[u8]) {
    use std::io::Write as _;
    use std::process::{Command as Process, Stdio};
    let script = "import sys, struct, zlib\n\
        d = sys.stdin.buffer.read()\n\
        assert d[:8] == b'\\x89PNG\\r\\n\\x1a\\n'\n\
        i, idat = 8, b''\n\
        while True:\n\
        \x20   n = struct.unpack('>I', d[i:i+4])[0]\n\
        \x20   t = d[i+4:i+8]\n\
        \x20   assert zlib.crc32(d[i+4:i+8+n]) == struct.unpack('>I', d[i+8+n:i+12+n])[0]\n\
        \x20   if t == b'IDAT': idat += d[i+8:i+8+n]\n\
        \x20   if t == b'IEND': break\n\
        \x20   i += 12 + n\n\
        print(len(zlib.decompress(idat)))\n";
    let Ok(mut child) = Process::new("python3")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return;
    };
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(png)
        .expect("write the picture");
    let out = child.wait_with_output().expect("run python3");
    assert!(out.status.success(), "zlib reads the picture");
    let (width, height, _) = read_png(png);
    let inflated: usize = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("a length");
    assert_eq!(inflated, (width.div_ceil(8) + 1) * height);
}
