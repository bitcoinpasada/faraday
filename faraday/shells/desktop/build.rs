//! The Windows desktop app's `.exe` resources: the icon, the version and
//! the `asInvoker` manifest, as the Starter's (`docs/STARTER.md` §2).
//! Only when the target is Windows, read at run time because this
//! script runs on the build computer.

use std::path::PathBuf;

/// The install's file and path names (`faraday-core/src/names.rs`).
#[path = "../../faraday-core/src/names.rs"]
#[allow(dead_code)]
mod names;

/// The `.exe` resources, shared with the Starter's build script.
#[path = "../../build/windows/resources.rs"]
mod resources;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
        resources::windows_resources(&resources::Program {
            icon_png: &manifest_dir.join("../starter/assets/icon.png"),
            strings: &manifest_dir.join("../../faraday-strings/src/en.rs"),
            name_field: "name_faraday_desktop",
            internal_name: "faraday-demo",
            exe: names::WIN_DESKTOP_EXE,
            manifest_id: names::MENU_DESKTOP_ID,
        });
    }
}
