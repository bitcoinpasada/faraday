//! Writes the QR fixtures `tools/scripts/scan-routing.txt` scans, one PNG
//! per routing case (`docs/UX.md` §4) plus a three-part animated BC-UR of
//! the demo PSBT, from the codec's own encoder.
//!
//! ```text
//! cargo run -p osk-qrgen -- [DIR]      # default tools/fixtures/qr
//! ```
//!
//! Every fixture is public test data: the BIP-39 test mnemonic, the
//! committed regtest demo PSBT, and keys derived from that mnemonic.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_codec::qr::{self, Ecc, Payload, QUIET_ZONE, QrMatrix};
use osk_codec::{seedqr, ur};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const DEMO_PSBT: &[u8] = include_bytes!("../../vectors/psbt/demo-regtest.psbt");
/// Pixels per module in the fixtures: large enough for the decoder at
/// any size, small enough to keep the files a few kilobytes.
const SCALE: usize = 4;
/// Fragment size of the animated fixture: three parts for the demo PSBT.
const UR_FRAGMENT: usize = 100;

fn main() -> ExitCode {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("tools/fixtures/qr"), PathBuf::from);
    match run(&dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let m = Mnemonic::parse(Language::English, ABANDON).map_err(|e| e.to_string())?;
    let master = MasterKey::from_seed(
        &m.to_seed(b"").map_err(|e| e.to_string())?,
        Network::Mainnet,
    );
    let account = master
        .account_xpub(ScriptType::NativeSegwit, 0)
        .map_err(|e| e.to_string())?;
    let address = account
        .address(false, 0)
        .map_err(|e| e.to_string())?
        .to_string();
    let bytes = |b: &[u8]| qr::encode(Payload::Bytes(b), Ecc::Low).map_err(|e| e.to_string());
    let mut fixtures: Vec<(String, QrMatrix)> = vec![
        (
            "seedqr-abandon".into(),
            seedqr::encode_seedqr(&m).map_err(|e| e.to_string())?,
        ),
        (
            "compactseedqr-abandon".into(),
            seedqr::encode_compact(&m).map_err(|e| e.to_string())?,
        ),
        ("psbt-demo".into(), bytes(DEMO_PSBT.trim_ascii())?),
        ("address".into(), bytes(address.as_bytes())?),
        ("xpub".into(), bytes(account.xpub_string().as_bytes())?),
        ("descriptor".into(), bytes(account.descriptor().as_bytes())?),
        ("words".into(), bytes(ABANDON.as_bytes())?),
        ("unknown".into(), bytes(b"hello, world")?),
    ];
    // The demo PSBT as pure UR parts, each an alphanumeric-mode code of
    // the upper-cased part.
    let text = std::str::from_utf8(DEMO_PSBT).map_err(|e| e.to_string())?;
    let psbt = osk_psbt::base64::decode(text.trim()).map_err(|e| format!("{e:?}"))?;
    let mut encoder = ur::Encoder::psbt(&psbt, UR_FRAGMENT).map_err(|e| e.to_string())?;
    let parts = encoder.fragment_count();
    for i in 1..=parts {
        let part = encoder.next_part().to_ascii_uppercase();
        let matrix = qr::encode(Payload::Alphanumeric(part.as_bytes()), Ecc::Low)
            .map_err(|e| e.to_string())?;
        fixtures.push((format!("psbt-ur-{i}-of-{parts}"), matrix));
    }
    for (name, matrix) in &fixtures {
        let (w, h, luma) = matrix.to_luma(SCALE, QUIET_ZONE);
        let path = dir.join(format!("{name}.png"));
        write_grey_png(&path, w, h, &luma)?;
        println!(
            "{} ({}x{} modules)",
            path.display(),
            matrix.size(),
            matrix.size()
        );
    }
    Ok(())
}

fn write_grey_png(path: &Path, width: usize, height: usize, luma: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(luma).map_err(|e| e.to_string())
}
