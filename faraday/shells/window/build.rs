//! Delay-loads uxtheme.dll and dwmapi.dll on Windows (F297).
//!
//! winit calls `SetWindowTheme`, `DwmSetWindowAttribute` and
//! `DwmEnableBlurBehindWindow`. As plain imports their libraries are mapped
//! before `main`, and Windows looks for a library that is not a KnownDLL in
//! the program's own folder first (Downloads, say), ahead of the
//! `SetDefaultDllDirectories` call in `main` and out of reach of the load
//! configuration's `DependentLoadFlags`. Delay-loaded, they are loaded at
//! first use, after that call, from System32 only.
//!
//! Short (Microsoft-style) import libraries for the two, made by
//! `llvm-dlltool` from a .def, are linked ahead of windows-targets' own
//! import library (this crate sits between the programs and winit, so
//! rustc puts its `-l` first), so winit's `__imp_` symbols resolve from
//! them; `build.sh` passes `--delayload` for each to LLD.
use std::path::PathBuf;
use std::process::Command;

/// The libraries and the functions winit imports from each.
const DELAYED: [(&str, &[&str]); 2] = [
    ("uxtheme", &["SetWindowTheme"]),
    (
        "dwmapi",
        &["DwmEnableBlurBehindWindow", "DwmSetWindowAttribute"],
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "gnu" {
        return;
    }
    println!("cargo:rerun-if-env-changed=LLVM_DLLTOOL");
    let tool = std::env::var("LLVM_DLLTOOL").unwrap_or_else(|_| String::from("llvm-dlltool"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    for (dll, functions) in DELAYED {
        let mut def = format!("LIBRARY {dll}.dll\nEXPORTS\n");
        for f in functions {
            def.push_str(f);
            def.push('\n');
        }
        std::fs::write(out.join(format!("{dll}.def")), def).expect("OUT_DIR is writable");
        let status = Command::new(&tool)
            .current_dir(&out)
            .args(["-m", "i386:x86-64", "-d"])
            .arg(format!("{dll}.def"))
            .arg("-l")
            .arg(format!("libfaraday_delay_{dll}.a"))
            .status()
            .unwrap_or_else(|e| {
                panic!("{tool}: {e} (llvm builds the delay-load import libraries)")
            });
        assert!(status.success(), "{tool} failed for {dll}: {status}");
        println!("cargo:rustc-link-lib=dylib=faraday_delay_{dll}");
    }
    println!("cargo:rustc-link-search=native={}", out.display());
}
