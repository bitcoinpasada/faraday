//! Builds the payload in (`docs/STARTER.md` §1.3): the folder
//! `FARADAY_STARTER_PAYLOAD` names holds `MANIFEST` and the files it lists,
//! and each is embedded as it is. The manifest is parsed here and every
//! file it names must be in the folder, or the build stops and says
//! which. Without the variable the Starter carries no payload and offers
//! only `--image`.
//!
//! The files are checked against their sums when they are used, not
//! here and not at start: the release's `SHA256SUMS` and its signature
//! cover the Starter file itself.

use std::fmt::Write as _;
use std::path::PathBuf;

/// The install's file and path names (`faraday-core/src/names.rs`).
#[path = "../../faraday-core/src/names.rs"]
#[allow(dead_code)]
mod names;

/// The `.exe` resources, shared with the desktop app's build script.
#[path = "../../build/windows/resources.rs"]
mod resources;

fn main() {
    println!("cargo:rerun-if-env-changed=FARADAY_STARTER_PAYLOAD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
        resources::windows_resources(&resources::Program {
            icon_png: &manifest_dir.join("assets/icon.png"),
            strings: &manifest_dir.join("../../faraday-strings/src/en.rs"),
            name_field: "starter_menu_name",
            internal_name: "faraday-starter",
            exe: names::WIN_EXE,
            manifest_id: names::MENU_ID,
        });
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let mut code = String::from("// Written by build.rs.\n");
    match std::env::var_os("FARADAY_STARTER_PAYLOAD").filter(|v| !v.is_empty()) {
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
                .unwrap_or_else(|e| panic!("FARADAY_STARTER_PAYLOAD {}: {e}", dir.display()));
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
