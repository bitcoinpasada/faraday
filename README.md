# Faraday

Faraday is an offline key appliance: a fork of OpenSignerKit / OpenSigner
that boots from a USB stick on an x86-64 UEFI PC, or from its card on a
Raspberry Pi, and runs entirely from RAM. It keeps OpenSigner's trust
model — one Rust binary on a kernel that has what the device needs and
nothing else, no network stack, no shell, no login, bit-identical
builds — and adds vaults, a Wallets tab, GPG, Secure Boot key
management and QR transfer on top of it.

**There is no release yet.** Build from source. Use test seeds only.

## What it does

| Area | Jobs |
|---|---|
| Vaults | Create a vault with one to four passphrases, each opening its own slot; unlock after the boot stick is removed; hold Bitcoin keys, wallets, notes and recovery sheets, entries (passwords and TOTP secrets), GPG keys and Secure Boot keys; seal and write back. |
| Wallets | Wallet creation, restore, backup and spending flows, built on OpenSigner's library crates for every piece of cryptography. |
| GPG | Create an Ed25519 certification key with an Ed25519 signing subkey, export the public certificate, make detached signatures, make a revocation certificate, renew expiry, write a paperkey backup. |
| Secure Boot | Generate PK, KEK and db keys and certificates, write enrolment files, sign and check `BOOTX64.EFI`. |
| QR | One scanner and one sender for the Wallets tab and the QR transfer tab: fountain-coded `ur:bytes`, BBQr, and the `faraday-file-v1` envelope. |
| Settings | Keyboard layout, display scale, idle lock, power off. |

See `PLAN.md` for the full spec, `docs/FLOWS.md` for every flow, and
`docs/VAULT.md` / `docs/QR.md` for the vault and QR formats.

## Relationship to OpenSigner

This repository is a full clone of OpenSignerKit/OpenSigner with its
history, kept as the git remote `upstream`. `core/` and `opensigner/`
are upstream, unedited — Faraday uses the `osk-*` crates as a library.
`faraday/` is Faraday itself: `faraday-core`, the desktop and stick
shells, the vault, GPG, Secure Boot and QR code. `docs/PLANNING.md`,
`docs/DESIGN.md` and `docs/UX.md` are upstream's own reference docs for
whatever `PLAN.md` doesn't change. See `PLAN.md` §2 for the full rule set
on what may and may not be edited upstream.

## Build

Prerequisites: stable Rust (see `rust-toolchain.toml`), a C compiler
(`secp256k1-sys` builds libsecp256k1 from source), and on Linux the X11
or Wayland client libraries at runtime, including `libxkbcommon-x11`.

```
# Quick dev build, for iterating: target/debug/faraday
cargo build -p faraday-desktop

# Release build: target/release/faraday
cargo build -p faraday-desktop --release

# Reproducible build in a pinned container: out/linux/faraday
export DOCKER=podman    # or docker
just faraday-linux-bin
```

## Run the desktop shell

```
./target/release/faraday [--sticks DIR] [--size WxH] [--full-kit]
```

Sticks are folders under `--sticks` (default `~/faraday-sticks`), each
one a USB stick the app can see plugged in or pulled out:

- **F2** plugs in the test stick (a 2-of-3 Taproot multisig's public
  backup files, an unsigned spend, and `vault.ofv`, passphrase `a`,
  holding its three seeds — all testnet), or pulls it out
- **F3** plugs in a blank stick
- **F4** pulls every stick
- **F5** plugs in every folder under `--sticks`

`--full-kit` carries the full test kit on the test stick instead of just
the backup test stick. Every session starts on testnet.

Build the test kit on its own with:

```
cargo run -q -p faraday-testkit -- out/testkit
```

## USB stick image

```
just faraday-stick-bin                    # the stick binary, static musl
just faraday-stick-image                  # out/stick/faraday-x86_64-uefi.img
just dev=1 faraday-stick-image            # + a serial console and login
```

Flash the image to a stick and boot a PC from it; the stick image starts
on mainnet. `faraday/image/run-build.sh` and
`opensigner/shells/pi/image/README.md` describe the image layout.

## Reproducible builds

Every artifact `just faraday-linux-bin` and `just faraday-stick-image`
produce is built in a container with the base image and every compiler
pinned by version, the commit date as `SOURCE_DATE_EPOCH`, and host
paths remapped out of the output, so a second builder gets the same
bytes.

## Repository rules

- No GitHub Actions or hosted CI. Everything builds and tests locally.
- No upstream file under `core/` or `opensigner/` is edited; upstream
  changes are merged via the `upstream` remote, not copied by hand.
- Nothing under `local/` is committed; use it for private notes.

## License

MIT. See `LICENSE`.
