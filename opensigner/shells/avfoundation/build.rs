//! Compiles `src/camera.m` on macOS, and does nothing anywhere else.

#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rerun-if-changed=src/camera.m");
    // The host is a Mac, but the target need not be (an iOS build from
    // the same checkout): the Objective-C is macOS only.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new()
        .file("src/camera.m")
        // ARC, as the file is written for.
        .flag("-fobjc-arc")
        .compile("osk_avf");
    for framework in ["AVFoundation", "CoreMedia", "CoreVideo", "Foundation"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("cargo:rerun-if-changed=src/camera.m");
}
