//! Compiles `src/camera.c` when the target is Windows, and does nothing
//! anywhere else.
//!
//! The decision is made from `CARGO_CFG_TARGET_OS` at run time, never
//! from `#[cfg(windows)]`: this script runs on the build host, and the
//! Windows build cross-compiles from Linux (faraday/build/windows/).

fn main() {
    println!("cargo:rerun-if-changed=src/camera.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    cc::Build::new().file("src/camera.c").compile("faraday_mf");
    // Only libraries every Windows has. `mfuuid` is a static library of
    // GUIDs; `ole32` is COM. Media Foundation's own DLLs (`mfplat`, `mf`,
    // `mfreadwrite`) are loaded by `src/camera.c` when the camera is
    // used, not imported: Windows' N editions ship without them, and an
    // import would stop the whole program from starting there.
    for lib in ["mfuuid", "ole32"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
