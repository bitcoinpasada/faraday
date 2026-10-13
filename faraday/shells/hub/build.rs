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
use std::path::{Path, PathBuf};

/// The install's file and path names (`faraday-core/src/names.rs`).
#[path = "../../faraday-core/src/names.rs"]
#[allow(dead_code)]
mod names;

fn main() {
    println!("cargo:rerun-if-env-changed=FARADAY_HUB_PAYLOAD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resources();
    }
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

/// The `.exe`'s icon, version and manifest (`docs/HUB.md` §2), compiled
/// by mingw-w64's `windres` (or `$WINDRES`) and linked into the hub app.
/// The icon is the menu icon's PNG inside an `.ico`, which Windows reads
/// as it is. The manifest asks for no elevation (`asInvoker`): the
/// writer is elevated per write.
fn windows_resources() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let manifest_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let png_path = manifest_dir.join("assets/icon.png");
    let strings_path = manifest_dir.join("../../faraday-strings/src/en.rs");
    println!("cargo:rerun-if-changed={}", png_path.display());
    println!("cargo:rerun-if-changed={}", strings_path.display());
    println!("cargo:rerun-if-env-changed=FARADAY_BUILD");
    println!("cargo:rerun-if-env-changed=WINDRES");

    let png = std::fs::read(&png_path).expect("the menu icon");
    let mut ico = Vec::new();
    ico.extend([0, 0, 1, 0, 1, 0]); // reserved, type 1 (icon), one image
    ico.extend([0, 0, 0, 0]); // 256 x 256 (0 is 256), no palette, reserved
    ico.extend(1u16.to_le_bytes()); // planes
    ico.extend(32u16.to_le_bytes()); // bits per pixel
    ico.extend((png.len() as u32).to_le_bytes());
    ico.extend(22u32.to_le_bytes()); // where the image starts
    ico.extend(&png);
    std::fs::write(out.join("app.ico"), ico).expect("OUT_DIR is writable");

    // The name people see, as the Start menu shortcut's: faraday-strings's en.rs.
    let strings = std::fs::read_to_string(&strings_path).expect("faraday-strings/src/en.rs");
    let field = |key: &str| -> String {
        strings
            .split(&format!("{key}: \""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_else(|| panic!("{key} in faraday-strings/src/en.rs"))
            .to_string()
    };
    let name = field("hub_menu_name");
    let publisher = field("hub_publisher");
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets it");
    let build = std::env::var("FARADAY_BUILD")
        .ok()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| version.clone());
    let numbers: Vec<u16> = version
        .split(['.', '-', '+'])
        .take(3)
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let quad = format!(
        "{},{},{},0",
        numbers.first().unwrap_or(&0),
        numbers.get(1).unwrap_or(&0),
        numbers.get(2).unwrap_or(&0)
    );
    let clean = |s: &str| s.replace(['"', '\\'], "");

    std::fs::write(
        out.join("app.manifest"),
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
  <assemblyIdentity type="win32" name="@MENU_ID@" version="1.0.0.0" processorArchitecture="amd64"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <asmv3:application>
    <asmv3:windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </asmv3:windowsSettings>
  </asmv3:application>
</assembly>
"#
        .replace("@MENU_ID@", names::MENU_ID),
    )
    .expect("OUT_DIR is writable");

    // Paths in the script are relative: windres runs in OUT_DIR, so no
    // folder of the build computer is named in what it reads.
    let rc = format!(
        r#"1 ICON "app.ico"
1 24 "app.manifest"
1 VERSIONINFO
FILEVERSION {quad}
PRODUCTVERSION {quad}
FILEFLAGSMASK 0x3f
FILEFLAGS 0
FILEOS 0x40004
FILETYPE 1
FILESUBTYPE 0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "{publisher}"
      VALUE "FileDescription", "{name}"
      VALUE "FileVersion", "{build}"
      VALUE "InternalName", "faraday-hub"
      VALUE "OriginalFilename", "{exe}"
      VALUE "ProductName", "{name}"
      VALUE "ProductVersion", "{build}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        name = clean(&name),
        exe = clean(names::WIN_EXE),
        publisher = clean(&publisher),
        build = clean(&build),
    );
    std::fs::write(out.join("app.rc"), rc).expect("OUT_DIR is writable");
    let windres =
        std::env::var("WINDRES").unwrap_or_else(|_| String::from("x86_64-w64-mingw32-windres"));
    let status = std::process::Command::new(&windres)
        .current_dir(&out)
        .args([
            "--input",
            "app.rc",
            "--output",
            "app-res.o",
            "--output-format",
            "coff",
        ])
        .status()
        .unwrap_or_else(|e| {
            panic!("{windres}: {e} (mingw-w64's binutils builds the .exe's resources)")
        });
    assert!(status.success(), "{windres} failed: {status}");
    let object: &Path = &out.join("app-res.o");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
