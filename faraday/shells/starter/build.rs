//! The Windows resources of the Starter's `.exe`, and the one thing the
//! Starter carries (`docs/STARTER.md` §1.3): the release's image list.
//! `FARADAY_STARTER_IMAGES` names the release's `IMAGES` file; it is
//! parsed here (a list that does not parse stops the build and names the
//! file) and embedded as it is. Nothing else is embedded: the images
//! themselves are found at run time and each is checked against its line
//! in this list. Without the variable the Starter finds nothing and
//! offers only `--image`.

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
    println!("cargo:rerun-if-env-changed=FARADAY_STARTER_IMAGES");
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
    match std::env::var_os("FARADAY_STARTER_IMAGES").filter(|v| !v.is_empty()) {
        None => code
            .push_str("/// No image list was built in.\npub const IMAGES: Option<&[u8]> = None;\n"),
        Some(file) => {
            let file = PathBuf::from(file);
            let file = file
                .canonicalize()
                .unwrap_or_else(|e| panic!("FARADAY_STARTER_IMAGES {}: {e}", file.display()));
            println!("cargo:rerun-if-changed={}", file.display());
            let bytes = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            faraday_payload::ImageList::parse(&bytes)
                .unwrap_or_else(|e| panic!("{}: {e:?}", file.display()));
            // `include_bytes!` reads the file at compile time; its path
            // goes into cargo's dependency file, not into the binary.
            let _ = writeln!(
                code,
                "/// The release's image list.\npub const IMAGES: Option<&[u8]> = Some(include_bytes!({:?}));",
                file.display().to_string()
            );
        }
    }
    std::fs::write(out.join("images.rs"), code).expect("OUT_DIR is writable");
}
