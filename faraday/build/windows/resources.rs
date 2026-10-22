//! The Windows `.exe` resources both Faraday programs for Windows carry
//! (`docs/STARTER.md` §2): included by `shells/starter/build.rs` and
//! `shells/desktop/build.rs` with `#[path]`, so it is plain `std` and
//! nothing else, and it is not part of any crate.

use std::path::{Path, PathBuf};

/// What differs between the Windows programs' resources.
pub struct Program<'a> {
    /// The menu icon's PNG.
    pub icon_png: &'a Path,
    /// `faraday-strings/src/en.rs`, where the program's name is.
    pub strings: &'a Path,
    /// The field of `en.rs` that holds the name people see.
    pub name_field: &'a str,
    /// The crate's own name, for the version block's InternalName.
    pub internal_name: &'a str,
    /// The executable's file name, for OriginalFilename.
    pub exe: &'a str,
    /// The manifest's assembly identity.
    pub manifest_id: &'a str,
}

/// The `.exe`'s icon, version and manifest (`docs/STARTER.md` §2), compiled
/// by mingw-w64's `windres` (or `$WINDRES`) and linked into the program.
/// The icon is the menu icon's PNG inside an `.ico`, which Windows reads
/// as it is. The manifest asks for no elevation (`asInvoker`): the
/// Starter's writer is elevated per write. Shared by the build scripts
/// of `faraday-starter` and `faraday-desktop`, which `#[path]` it in.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn windows_resources(program: &Program) {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let png_path = program.icon_png;
    let strings_path = program.strings;
    println!("cargo:rerun-if-changed={}", png_path.display());
    println!("cargo:rerun-if-changed={}", strings_path.display());
    println!("cargo:rerun-if-env-changed=FARADAY_BUILD");
    println!("cargo:rerun-if-env-changed=WINDRES");

    let png = std::fs::read(png_path).expect("the menu icon");
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
    let strings = std::fs::read_to_string(strings_path).expect("faraday-strings/src/en.rs");
    let field = |key: &str| -> String {
        strings
            .split(&format!("{key}: \""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_else(|| panic!("{key} in faraday-strings/src/en.rs"))
            .to_string()
    };
    let name = field(program.name_field);
    let publisher = field("starter_publisher");
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
        .replace("@MENU_ID@", program.manifest_id),
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
      VALUE "InternalName", "{internal}"
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
        internal = clean(program.internal_name),
        exe = clean(program.exe),
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
    let object = out.join("app-res.o");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
