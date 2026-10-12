//! Builds the payload in (`docs/HUB.md` §1.3): the folder
//! `FARADAY_HUB_PAYLOAD` names holds `MANIFEST` and the files it lists,
//! and each is embedded as it is. The manifest is parsed here and every
//! file it names must be in the folder, or the build stops and says
//! which. Without the variable the hub app carries no payload and offers
//! only `--image`.
//!
//! The files are checked against their sums when they are used, not
//! here and not at start: the release's `SHA256SUMS` and its signature
//! cover the hub app file itself.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=FARADAY_HUB_PAYLOAD");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let mut code = String::from("// Written by build.rs.\n");
    match std::env::var_os("FARADAY_HUB_PAYLOAD").filter(|v| !v.is_empty()) {
        None => {
            code.push_str(
                "/// No payload was built in.\npub const MANIFEST: Option<&[u8]> = None;\n",
            );
            code.push_str(
                "/// The payload's files, by name.\npub const FILES: &[(&str, &[u8])] = &[];\n",
            );
        }
        Some(dir) => {
            let dir = PathBuf::from(dir);
            let dir = dir
                .canonicalize()
                .unwrap_or_else(|e| panic!("FARADAY_HUB_PAYLOAD {}: {e}", dir.display()));
            let manifest = dir.join("MANIFEST");
            println!("cargo:rerun-if-changed={}", manifest.display());
            let bytes =
                std::fs::read(&manifest).unwrap_or_else(|e| panic!("{}: {e}", manifest.display()));
            let parsed = faraday_payload::Manifest::parse(&bytes)
                .unwrap_or_else(|e| panic!("{}: {e:?}", manifest.display()));
            let missing: Vec<&str> = parsed
                .files()
                .iter()
                .filter(|f| !dir.join(&f.name).is_file())
                .map(|f| f.name.as_str())
                .collect();
            assert!(
                missing.is_empty(),
                "{}: the manifest names files the folder does not hold: {}",
                dir.display(),
                missing.join(", ")
            );
            // `include_bytes!` reads the file at compile time; its path
            // goes into cargo's dependency file, not into the binary.
            let _ = writeln!(
                code,
                "/// The payload's MANIFEST.\npub const MANIFEST: Option<&[u8]> = Some(include_bytes!({:?}));",
                manifest.display().to_string()
            );
            code.push_str(
                "/// The payload's files, by name.\npub const FILES: &[(&str, &[u8])] = &[\n",
            );
            for f in parsed.files() {
                let path = dir.join(&f.name);
                println!("cargo:rerun-if-changed={}", path.display());
                let _ = writeln!(
                    code,
                    "    ({:?}, include_bytes!({:?})),",
                    f.name,
                    path.display().to_string()
                );
            }
            code.push_str("];\n");
        }
    }
    std::fs::write(out.join("payload.rs"), code).expect("OUT_DIR is writable");
}
