# Fuzzing: Faraday's own targets

Twenty-one `cargo fuzz` targets for the parsers that live in Faraday's own
crates, each reading something the device did not write: a stick's
filesystem and files, a scanned code, a camera frame, a screen's EDID, a
USB device's descriptor, another wallet's file. The parsers in the shared
crates (camera frames, PSBTs, descriptors, keys, key shares, JSON) are
fuzzed from `fuzz/` (`fuzz/README.md`).

This runs on a developer's machine and nowhere else. There is no hosted
CI. The run before a release is `just fuzz-release`, which runs these
targets and those of `fuzz/` (`fuzz/README.md`, "Before a release").

## Setup

`cargo-fuzz` and the dated nightly that `faraday/fuzz/rust-toolchain.toml`
pins (the same as `fuzz/rust-toolchain.toml`), both at user level; the
workspace itself stays on its pinned stable toolchain.

```
cargo install cargo-fuzz --locked
rustup toolchain install nightly-2026-10-13 --profile minimal
```

(The date is the one in the file; the recipes read it from there.)

## Running

From the repository root:

```
just fuzz-list                          the targets of both directories
just fuzz fat                           one target, 60 seconds
just fuzz classify_file 300             one target, 300 seconds
just fuzz-release                       every target of both, 10 minutes each
```

`just fuzz` finds the directory a target is in and runs
`cargo +NIGHTLY fuzz run --fuzz-dir faraday/fuzz TARGET -- -max_total_time=N`
from the repository root, NIGHTLY being the pinned one. Pass `--fuzz-dir
faraday/fuzz` to `cargo +NIGHTLY fuzz` directly for anything libFuzzer can
do that the recipe does not pass through. Run from inside this directory, `cargo fuzz` finds `fuzz/` instead;
that is why the recipe passes the directory.

This crate is its own workspace, so a sanitizer build shares neither the
lock file nor the target directory with the rest of the repository.
Everything it builds lands in `faraday/fuzz/target`. Its `Cargo.lock` is
committed and was made from the workspace's, so the targets fuzz the
versions that ship (a fresh resolution picks a newer release candidate of a
pinned crate that does not build). `cargo-fuzz` has no `--locked`, so the
recipes run `cargo metadata --locked` on it first. To regenerate it, copy
the root `Cargo.lock` over it and run `cargo +NIGHTLY metadata
--manifest-path faraday/fuzz/Cargo.toml`. Its dependencies are vetted with
the workspace's (`docs/deps/libfuzzer-sys.md`).

The run before a release, the corpora kept and published, and how to
reproduce a release's run from its commit are in `fuzz/README.md`,
"Before a release".

## Layout

- `fuzz_targets/` — one file per target.
- `corpus/<target>/` — the seed inputs, committed, all with an extension.
  A run adds its own entries beside them, named after their SHA-1 and
  ignored by git. `just fuzz-release` grows its corpus elsewhere
  (`fuzz/README.md`, "Before a release").
- `artifacts/<target>/` — an input that crashed. Committed ones have an
  extension and a name: the minimized reproducer of a crash, kept after
  it is fixed. What libFuzzer writes for itself (`crash-<sha1>`, `oom-…`,
  `slow-unit-…`) is ignored.

Inputs that once crashed a target belong in its corpus, so they keep being
run.

## The targets

Targets that need Faraday's app build `faraday-core` with its `testkit`
feature (the scan sheet opened for a purpose, `testkit::open_scan`); no
shipped binary does.

| Target | What it feeds |
|---|---|
| `fat` | `faraday_fat::Volume::open` with the input as a partition image: the boot sector, the FATs and the root directory. Every file listed is read, then a file is created, renamed and deleted, so the write paths walk the same structures. `boot_sector_label` reads the first sector alone. |
| `read_wallet` | `faraday_intake::wallet::read_wallet` with the input as text: a single-key file, a Caravan config or an Electrum wallet file. The JSON reader under it is fuzzed on its own in `fuzz/` (`json`). |
| `classify_file` | `faraday_core::wallet::classify_read`, which Files runs on every file that comes in (a visit's, and the boot stick's with no press): the vault header, EFI image, certificate, PSBT, GPG, signed message, multisig share, wallet, key, words, entries, seed part and raw transaction readers in order. The first byte picks the name's extension. |
| `inbox_found` | Several files at once: the input split at each `0x1e` byte, brought into the Inbox as a stick's files are, put together by `inbox_found_now` (multisig shares merged, SLIP-39 and codex32 groups recovered and given a fingerprint, cosigner keys read), then loaded. SLIP-39 shares that ask for more than the base number of PBKDF2 rounds are left out. |
| `qr_assembler` | Scanned parts: the lines fed to one `faraday_qr::Assembler` (BBQr in base32, hex and inflated; numbered parts), a finished transfer kept as the Inbox keeps it; the whole text to `classify_text` and the transaction-in-hex reader; the bytes to the Faraday file envelope. |
| `ur_registry` | `faraday_qr::registry::read` for each UR type the registry reads (the first byte picks it) and `cbor::Value::decode`. The fountain decoder under it is `ur_receive` in `fuzz/`. |
| `png_qr` | `faraday_files::qr_in_png`: the `png` decoder, the size limit and the QR decoder over what it makes. |
| `app_scanned` | A `Faraday` with the scan sheet open for each scan purpose in turn (the first byte picks it), every line sent as `Event::Scanned`, then the text pasted: the classifier, the seed readers, the address note, a vault entry's TOTP reader and Transfer's receiver. |
| `edid` | The kernel's boot parameters (the EDID inside them, the Secure Boot byte) and an EDID as a monitor, a KVM or the Starter's display check hands it over. |
| `hid_ranges` | `hid::ranges` over a USB touchscreen's HID report descriptor; the stick shell's `hid.rs` is included by path. |
| `bip39_words` | `Mnemonic::parse`, `forms::typed_mnemonic` and `faraday_intake::file_words` over text, in the language the first byte picks. |
| `vault_header` | `faraday_vault::read_header` over any file. |
| `vault_records` | `records::Contents::decode` over a slot's plaintext, then the records written out again. |
| `vault_entries` | `entries::parse` and `totp::parse`: a vault entries file and a TOTP setup code. |
| `files_proto` | The disk process pipe: `Response::decode` and `Request::decode`, each decoded value encoded and decoded again to the same value, and the framing read as a stream. |
| `pe_image` | `pe::image_hash`, `check` against a fixed key's certificate and against a certificate from the input, `sign`, `x509::Cert::read` and `der::children`. |
| `gpt` | The Starter's `gpt::read` and `faraday_boot` over the input as a disk image, with the header's and the entries' checksums made right first; the Starter shell's `gpt.rs` is included by path. And `faraday_boot::find_release`, with which the Starter reads a stick's version, over the input as a boot partition's bytes. |
| `v4l2_luma` | The V4L2 shell's frame converters (`luma`, `chroma`, the two downscales) over a format, a size, a stride and a buffer taken from the input; `abi.rs` and `convert.rs` are included by path. |
| `manifest` | `faraday_payload::Manifest::parse`. |
| `plist` | The Starter's macOS property list reader; `plist.rs` is included by path. |

`classify_file`, `inbox_found`, `read_wallet` and `png_qr` take inputs up to
a megabyte under `just fuzz-release` (`tools/fuzz-release.sh` sets the
limits), since the 4 KiB libFuzzer default cannot show a size that
multiplies. For the same reason the others of the stick's file readers run
with `-max_len=262144`.

Not fuzzed here: the other macOS and Windows camera converters, the stick
shell's evdev and keyboard readers (the kernel shapes what they read), and
the Windows writer pipe and BitLocker helper (a line protocol with fixed
line limits).
