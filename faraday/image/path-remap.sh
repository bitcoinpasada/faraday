# Sourced (`. faraday/image/path-remap.sh`) by every build of a binary
# that is published: faraday.just's faraday-stick-bin and faraday-pi-bin,
# and the desktop release build (README, "Build"). The compilers embed
# absolute source paths in a binary for its panic and assert messages;
# this rewrites them as they are compiled in, so a published binary
# carries no home directory, user name or other local path, and two
# builders in different directories get the same bytes.
#
# Rust through RUSTFLAGS; C (libsecp256k1, built by the cc crate) through
# CFLAGS, or through CFLAGS_<target> when the build has set that, since
# cc then reads only the target's. Appends to what the build set already.
# Where two prefixes match, the later, more specific one wins.

_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
_cargo="${CARGO_HOME:-$HOME/.cargo}"
_rust="--remap-path-prefix=$HOME=/build --remap-path-prefix=$_cargo/registry/src=/cargo --remap-path-prefix=$_cargo/git/checkouts=/cargo-git --remap-path-prefix=$_root=/faraday"
_c="-ffile-prefix-map=$HOME=/build -ffile-prefix-map=$_cargo/registry/src=/cargo -ffile-prefix-map=$_cargo/git/checkouts=/cargo-git -ffile-prefix-map=$_root=/faraday"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$_rust"
export CFLAGS="${CFLAGS:+$CFLAGS }$_c"
if [ -n "${RUST_TARGET:-}" ]; then
    _var="CFLAGS_${RUST_TARGET//-/_}"
    if [ -n "${!_var:-}" ]; then
        export "$_var=${!_var} $_c"
    fi
fi
unset _root _cargo _rust _c _var
