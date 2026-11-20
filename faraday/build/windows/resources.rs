//! The Windows `.exe` resources both Faraday programs for Windows carry
//! (`docs/STARTER.md` §2): included by `shells/starter/build.rs` and
//! `shells/demo/build.rs` with `#[path]`, so it is plain `std` and
//! nothing else, and it is not part of any crate.

use std::path::{Path, PathBuf};

/// What differs between the Windows programs' resources.
pub struct Program<'a> {
    /// The menu icon's PNG.
    pub icon_png: &'a Path,
    /// `faraday-strings/src/en/`, the folder where the program's name is.
    pub strings: &'a Path,
    /// The field of `en/` that holds the name people see.
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
/// of `faraday-starter` and `faraday-demo`, which `#[path]` it in.
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

    // The name people see, as the Start menu shortcut's: faraday-strings's en/.
    let mut strings = String::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(strings_path)
        .expect("faraday-strings/src/en/")
        .map(|e| e.expect("faraday-strings/src/en/").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    for file in files {
        strings
            .push_str(&std::fs::read_to_string(&file).expect("a file of faraday-strings/src/en/"));
    }
    let field = |key: &str| -> String {
        strings
            .split(&format!("{key}: \""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_else(|| panic!("{key} in faraday-strings/src/en/"))
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
    let load_config = out.join("load-config.o");
    std::fs::write(&load_config, load_config_object()).expect("OUT_DIR is writable");
    println!("cargo:rustc-link-arg-bins={}", load_config.display());
    // The section is not referenced by any code: the linker keeps it
    // because its symbol is asked for.
    println!("cargo:rustc-link-arg-bins=-Wl,-u,{LOAD_CONFIG_SYMBOL}");
}

/// The symbol of the load configuration's section: the name LLD looks for
/// to point the image's load configuration directory at it.
const LOAD_CONFIG_SYMBOL: &str = "_load_config_used";

/// A COFF object (x86-64) holding a section `.lcfg` with the `.exe`'s load
/// configuration, `IMAGE_LOAD_CONFIG_DIRECTORY64` up to and including
/// `DependentLoadFlags` (offset 0x4E), its `Size` the 0x50 bytes written.
/// The flags are `LOAD_LIBRARY_SEARCH_SYSTEM32`, for what the libraries
/// loaded later load in turn. They do not apply to the `.exe`'s own imports
/// (those are mapped first): those are KnownDLLs and API sets only, with
/// `uxtheme.dll` and `dwmapi.dll` delay-loaded and `propsys.dll` loaded by
/// name, and `main` calls `SetDefaultDllDirectories` first
/// (`faraday-window`'s `system32`). LLD
/// points the image's load configuration directory at the symbol
/// `_load_config_used`; `build.sh` fails the build when the flag is not in
/// the finished `.exe`. The object is written here as bytes, so no
/// compiler or `unsafe` is needed and the same bytes come out every time.
fn load_config_object() -> Vec<u8> {
    const SIZE: usize = 0x50;
    let name = format!("{LOAD_CONFIG_SYMBOL}\0");
    let name = name.as_bytes();
    let mut data = vec![0u8; SIZE];
    data[0..4].copy_from_slice(&(SIZE as u32).to_le_bytes());
    data[0x4E..0x50].copy_from_slice(&0x0800u16.to_le_bytes());

    let symbols_at = 20 + 40 + SIZE;
    let mut o = Vec::new();
    // File header: AMD64, one section, the symbol table after the data.
    o.extend(0x8664u16.to_le_bytes());
    o.extend(1u16.to_le_bytes());
    o.extend(0u32.to_le_bytes());
    o.extend((symbols_at as u32).to_le_bytes());
    o.extend(1u32.to_le_bytes());
    o.extend(0u16.to_le_bytes());
    o.extend(0u16.to_le_bytes());
    // `.lcfg`: initialised data, 8-byte aligned, readable.
    o.extend(b".lcfg\0\0\0");
    o.extend(0u32.to_le_bytes());
    o.extend(0u32.to_le_bytes());
    o.extend((SIZE as u32).to_le_bytes());
    o.extend(60u32.to_le_bytes());
    o.extend([0u8; 12]);
    o.extend(0x4040_0040u32.to_le_bytes());
    o.extend(&data);
    // The one symbol, external, in section 1, its name in the string table.
    o.extend(0u32.to_le_bytes());
    o.extend(4u32.to_le_bytes());
    o.extend(0u32.to_le_bytes());
    o.extend(1u16.to_le_bytes());
    o.extend(0u16.to_le_bytes());
    o.extend([2u8, 0u8]);
    o.extend(((4 + name.len()) as u32).to_le_bytes());
    o.extend(name);
    o
}
