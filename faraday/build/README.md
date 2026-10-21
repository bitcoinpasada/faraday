# faraday/build

Containers that build the apps for release.

- `linux/`: Debian 11 (glibc 2.31), so one binary runs on Ubuntu 20.04 and
  later, Debian 11 and later, Fedora 32 and later and Arch. `build.sh`
  builds one package: by default the desktop app (`faraday-desktop`,
  `faraday`); the Starter is built with the arguments
  `faraday-starter-shell faraday-starter`. Run by
  `just faraday-linux-bin`.
- `windows/`: a Debian container that cross-compiles for
  `x86_64-pc-windows-gnu` with mingw-w64. `build.sh` builds the Starter by
  default (`faraday-starter-shell`, `faraday-starter`), or another package,
  and has a `clippy` mode for the Windows target. Run by
  `just faraday-windows-bin`. `resources.rs` is the code the Windows programs' `build.rs` files include to set the `.exe` resources.

Both pin the base image by digest and refuse a Rust toolchain other than
the one in `rust-toolchain.toml`. Output is `out/linux/` and `out/windows/`.

`faraday/image/` is different: it builds the Buildroot images for the stick
and the Pi (`just faraday-stick-image`, `just faraday-pi-image`), whole
operating system images with the app inside. See `faraday/image/README.md`.
