//! The Windows `.exe`'s resources: the icon, the version and the
//! `asInvoker` manifest, as the desktop test app's. Only when the target
//! is Windows, read at run time because this script runs on the build
//! computer.

use std::path::PathBuf;

/// The `.exe` resources, shared with the other desktop programs' build
/// scripts.
#[path = "../../faraday/build/windows/resources.rs"]
mod resources;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
        resources::windows_resources(&resources::Program {
            icon_png: &manifest_dir.join("../../faraday/shells/starter/assets/icon.png"),
            strings: &manifest_dir.join("../../faraday/faraday-strings/src/en.rs"),
            name_field: "aqrv_window_title",
            internal_name: "aqrv",
            exe: "aqrv.exe",
            manifest_id: "app.faradayoffline.ArgonQrVault.Cli",
        });
    }
}
