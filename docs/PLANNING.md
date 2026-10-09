# OpenSignerKit (OSK) & OpenSigner — Project Planning Document

**Status:** Draft v0.6 — the decisions in §16 are being implemented; everything else is still for iteration
**Date:** 2026-09-07

> **How to read this document.** This file and UX.md were drafted by Claude in conversation with the project owner, and Claude will also do the implementation. Nothing here is a hard-and-fast rule: it records our current best understanding, and when implementation or use reveals something better, the document and the code change together. "Always" and "never" mean "deliberate default with a reason — change it knowingly, not accidentally." The security policies in §5 are the closest thing to firm commitments; even those may be refined.
>
> **Naming:** *OpenSignerKit (OSK)* is the Rust library — the `osk-*` core crates and the core↔shell contract. It is a building block that other projects may use. *OpenSigner* is the end-user application built on it, kept in the same repository as the reference implementation of OSK. Nothing product-specific goes in the OSK crates; anything OpenSigner needs that isn't generic lives in `opensigner/`.
> Sections marked ❓ are open questions that need a decision before build.

---

## 0. One-paragraph summary

OpenSignerKit is a single Rust core that handles Bitcoin seeds, keys, and signing entirely offline, wrapped in the thinnest possible platform shells for: DIY devices (Raspberry Pi + small touchscreen, SeedSigner/Krux-style), Android (especially GrapheneOS), iOS, and offline desktop. Identical behavior everywhere; the only difference is how pixels reach the screen and how input arrives. The same breadth of wallets, keys and formats everywhere too, so a person can check one implementation's outcome against another's, or use OpenSigner as one device among several. The OS is treated as untrusted and is told as little as possible.

---

## 1. Principles (in priority order)

1. **Correctness of Bitcoin operations.** A wrong signature or wrong address is worse than any UX flaw.
2. **Minimize the trusted computing base.** Every dependency, every syscall, every OS feature used must justify itself.
3. **Secrets exist in plaintext for the shortest possible time, in the smallest possible region.**
4. **Airgapped by construction.** No networking code exists in the binary. Not disabled — absent.
5. **Verifiability.** Reproducible builds are a hard release requirement: no artifact ships unless it has been independently reproduced (Section 11.1).
6. **One core, many shells.** Platform code is glue only. If logic is in a shell, that's a bug.
7. **Honesty about limits.** Each platform gets an explicit assurance tier. We never imply a phone is as safe as a dedicated device.
8. **Tools, not just a wallet.** First-class support for the "explorer/verifier" workflows power users currently do by hand.
9. **Breadth, explained, decided by the user.** OpenSigner supports every well-specified standard, de facto standard and widely used protocol it can, explains the purpose and the trade-offs in Learn, and leaves the choice to the person. It never narrows the options to what is simplest to build. Two gaps it exists to close: coordinators like Sparrow and Liana run online, on a desktop, and do not port to a phone or an offline device; signers like SeedSigner run offline anywhere but offer a fraction of what those coordinators do. OpenSigner is the offline implementation with the coordinator's breadth and the same UX on every device a person already has, so that it can verify another implementation's outcome, or stand as one device among several, without networking and without storing anything outside a secure element where one exists. A kind that is read but not buildable, or built but not readable, is a gap to close, and every kind has a Learn page before or with the pass that builds it (owner directive, 2026-09-18; §16.104, §16.105).

---

## 2. Threat model

### 2.1 Assets
- Seed entropy / BIP-39 mnemonic
- BIP-39 passphrase
- Derived private keys (xprv, per-address keys)
- Signing nonces (a leaked or biased nonce = leaked key)
- Secondary: xpubs/descriptors (privacy), PSBT contents (privacy), PIN/unlock credentials

### 2.2 Adversaries we defend against
| Adversary | Capability | Mitigation posture |
|---|---|---|
| Malware on a *paired* online computer/phone | Feeds malicious PSBTs, lies about addresses, fee, change | Full PSBT inspection, change detection, address verification, no trust in coordinator |
| Malicious/compromised supply chain of *our* software | Modified binary | Reproducible builds, signed releases, small dependency tree, vendored deps, source-build path |
| Evil-maid / physical access to a powered-off device | Reads storage | Stateless mode by default; SE-wrapped or strongly encrypted storage otherwise |
| Physical access to a *running* unlocked device | Reads screen/memory | Auto-lock, scoped decryption, memory zeroing, no secrets on screen without explicit reveal |
| Side channels via OS (logs, crash reports, backups, screenshots, clipboard, swap) | Passive leakage | Never log, disable backups/screenshots, mlock where available, panic=abort with silent handler. The clipboard carries public strings only, in both directions, and only for a tap the person made: a payload the mask policy calls a secret is refused on the way in and carries no Copy row on the way out (§16.88) |
| Kleptographic signing (malicious firmware/lib leaking key via nonce) | Nonce control | Deterministic nonces (RFC 6979) + user-verifiable nonce validation + anti-exfil protocol (Section 8.4) |
| Weak entropy (bad RNG) | Predictable seeds | User-supplied entropy (dice/coins/cards) with statistical checks; never trust a single RNG source alone; mix sources |

### 2.3 Adversaries we do NOT defend against (explicit non-goals)
- Compromised OS kernel / hypervisor with full memory read on the host platform. On phones and desktop this is out of scope; we reduce exposure window but cannot prevent it.
- Hardware implants in the Pi/phone.
- Rubber-hose. (Plausible deniability via passphrase is supported but not guaranteed.)
- Bugs in vetted upstream crypto libraries (we mitigate via test vectors and cross-checking, not via reimplementation).

### 2.4 Trust boundaries per platform
```
DIY Pi device    : [our binary] | [minimal Linux we build] | [Pi firmware/SoC]
Android          : [our .so]    | [thin Kotlin shell] | [Android/Graphene] | [SE: StrongBox]
iOS              : [our .a]     | [thin Swift shell]  | [iOS]              | [SE: Secure Enclave]
Desktop native   : [our binary] | [OS]
Desktop HTML     : [our WASM]   | [browser JS engine] | [OS]     ← weakest
```

---

## 3. Platforms and assurance tiers

| Tier | Platform | Notes |
|---|---|---|
| **A** | Pi + touchscreen, custom minimal Linux, camera, no storage by default | Closest to SeedSigner/Krux. Only tier where we control the whole OS image. The Pi 3 has no secure boot, so the card the person flashed is the whole of the software's provenance; the device says so at start. |
| **B** | Android (especially GrapheneOS) with StrongBox; iOS with Secure Enclave | Convenience tier with real hardware key-wrapping. Persistent-seed mode allowed. What the element promises holds while the operating system is intact: the app refuses to run on a device whose boot the platform says is not verified. |
| **C** | Stock Android, desktop native (Linux/macOS/Windows) | Stateless-only recommended. |
| **D** | Single-file HTML/WASM in a browser | Convenience/verification tool. Stateless only. UI warns clearly. |

Tier is displayed in-app (About screen) and affects defaults (e.g. Tier D refuses to persist anything).

### 3.1 Hardware reference targets
- **Primary dev device:** Raspberry Pi 3B+, Waveshare 2.8" DPI capacitive touch panel (480×640 native portrait; Goodix GT911 touch controller on a bit-banged I²C bus; 18-bit RGB666 over parallel DPI; the Pi firmware exposes it as a 16-bit RGB565 `/dev/fb0`), Pi Camera OV5647 (v1; the IMX219 v2 also works; §16.33). The SeedSigner project's published image for the same panel is a reference for boot config, display init, touch handling and the camera stack; nothing is shared with it and this repository builds on its own. Known quirks: `dpi_output_format` must be `0x7F206` (the vendor value swaps red and blue), and the touch controller reports a 640×480 grid while the panel is 480×640, so read the real ranges from the device.
- **Display philosophy for DIY:** support whatever panel the builder buys. Any resolution ≥ the minimum (Section 4.3), any orientation. Touch is required (UX.md §2); physical buttons are optional accelerators.
- Future: Pi Zero 2 W (Wi-Fi/BT hardware present → must be disabled at kernel level / physically removed), other SPI panels.
- ❓ Bare-metal Pi (no Linux) as a long-term Tier A+ target.

---

## 4. Architecture

### 4.1 Crate layout
```
opensignerkit/         # the library (OSK)
  core/                # no_std + alloc. ALL logic. Zero I/O. Zero platform code.
    osk-crypto/       # thin wrappers over vetted crates; secret types; zeroization
    osk-bip/          # BIP-32/39/85/… , descriptors, address derivation
    osk-psbt/         # parsing, inspection, policy checks, signing, finalization
    osk-entropy/      # dice/coin/card/camera entropy, statistical checks
    osk-codec/        # QR payloads: SeedQR, BC-UR, base64/hex/base58/bech32
    osk-backup/       # encrypted files: osk-backup (.oskb) and KDBX 4
    osk-keep/         # the kept-key blob: layout, PIN + Argon2id + secure-element keys
    osk-ui/           # layout engine + widgets → pixel buffer (tiny-skia); no flows
    osk-shell-api/    # Event/Command types (Section 4.2); the only thing a shell depends on

opensigner/            # the reference app (same repo)
  opensigner-core/     # state machine: screens, flows, wording. Events in → Commands out
  opensigner-ffi/      # the C ABI + JNI over opensigner-core; the only crate with unsafe
  shells/
    pi/                # DPI framebuffer, I²C touch, V4L2 camera (§16.33)
    android/           # Kotlin: one View over the FFI; Camera2 frames (§16.33), later StrongBox
    ios/               # Swift: CALayer + AVCapture frames + Secure Enclave wrapping
    desktop/           # winit + softbuffer, optional webcam
    web/               # WASM + <canvas>, getUserMedia
    tui/               # terminal harness for testing (dev only)

tools/                 # shared
    vectors/           # test vectors, cross-check scripts
    reproduce/         # reproducible build scripts, container definitions
```

### 4.2 The core ↔ shell contract
The shell is deliberately dumb. It can do exactly four things:

```rust
// Shell → Core
enum Event {
    Touch { x: u16, y: u16, phase: TouchPhase },
    Button(ButtonId),
    Key(char),                 // desktop/TUI
    CameraFrame { width, height, luma }, // 8-bit luma only, any size; ~640 px wide is enough
    CameraUnavailable,         // reply to CameraOn: no camera, or access refused
    Tick(u64),                 // monotonic ms, for timeouts
    Storage(StorageResult),    // reply to a storage request
    File { kind, bytes },      // reply to RequestFile: raw file content
    FileUnavailable { kind },  // reply to RequestFile: no file channel here, or the read failed
    FileCancelled { kind },    // reply to RequestFile: the person closed the picker (§16.48)
    Entropy { bytes: [u8; 32] }, // reply to RequestEntropy: the shell's RNG (§16.21)
    Display { w: u16, h: u16, dpi: u16, buttons: u8 }, // sent once at init; touch is always present
}

// Core → Shell
enum Command {
    Draw(&FrameBuffer),        // core owns the buffer; shell blits it
    CameraOn, CameraOff,
    RequestFile { kind },      // §7 file/SD channel: File, FileCancelled or FileUnavailable
    WriteFile { kind, name_hint, bytes }, // only for a user action; shell picks the place
    StorageRead(key), StorageWrite(key, bytes), StorageDelete(key),
    SecureWrap(bytes), SecureUnwrap(bytes),   // only on Tier B
    RequestEntropy,            // §16.21: the core has no RNG; the shell answers with Entropy
    Vibrate(ms), Exit,
}
```
Consequences:
- The shell never sees a seed, key, or mnemonic word. It sees pixels, opaque blobs, and the bytes of files the user chose to hand over (a PSBT is not a secret).
- Every UI decision (layout, fonts, wording, flows) lives in core and is identical on every platform.
- Testing the entire product = driving `Event`s into core and asserting on `Command`s. No hardware needed.

### 4.3 Rendering (responsive, not resizable)
- `tiny-skia` (pure Rust, `no_std`-capable, anti-aliased paths/strokes/curves; vendored and reviewed) draws into a core-owned framebuffer (RGBA8888, tiny-skia's native premultiplied format; the shell converts to whatever its display wants — the Pi shell packs to RGB565 for `/dev/fb0`, phones and desktop blit 32-bit directly; §16.3) sized to whatever the shell reports in `Event::Display`. Size is fixed for the session — no live resizing — but *any* size is supported so DIY builders can use any panel.
- **Minimum supported resolution:** 240 across the short side and 320 along the long one, in either orientation (UX.md §2; §16.4). Below that, refuse with a clear message. Panels under ~2.4" and d-pad-only devices are not targets; SeedSigner and Krux already serve them.
- **Layout model:** flexbox-lite. Screens are declared as a tree of rows/columns with weights, min sizes, and padding; the layout engine solves it for the actual pixel size. Same screen definition serves 240×320, 480×640, a 1080×2340 phone, and a desktop window.
- **Size classes** (derived from physical size, using reported DPI), as defined in UX.md §2: `small` (~2.4"–3.5"), `mobile` (~3.5"–7"), `wide` (tablet/desktop). Size class may swap a component variant (row height, key height, hub columns) but **never reorders steps or hides information**. Two or three variants per component, not per screen. The `wide` templates are in §16.22.
- **Reference sizes**, which every screen is reviewed at (§13.1): `small` 480×640 @ 286 dpi (the 2.8" panel) and 240×320 @ 143 dpi (the minimum), `mobile` 1080×2340 @ 420 dpi, `wide` 960×640 @ 160 dpi. **Small panels are portrait**; the layout engine still solves landscape, but nothing is designed for it.
- **The desktop window has one size**: 960×640 at 160 dpi, the `wide` reference. A desktop build is not resizable and does not adapt to the monitor; `--size` and `--dpi` are review flags for the other reference screens (§16.27).
- **Density-independent units:** all dimensions in dp; text sizes scale with DPI so a 2.8" 480×640 panel and a 2.8" 240×320 panel show the same physical layout.
- Orientation: portrait is the design target on small panels; landscape still solves through the same layout engine, and DIY builders set orientation in config.
- **Fonts and icons: outlines extracted at build time, never parsed at runtime.** A build tool takes vendored fonts (Noto Sans Regular and SemiBold, a monospace, an icon face and the four Noto Sans CJK region subsets), extracts exactly the code points required — the union of all ten BIP-39 wordlists plus every UI translation string — and writes each glyph once as an outline in font units, in a format of ours (§16.39). No TTF parser ships on the device: `osk-ui` reads that format and fills a glyph with `tiny-skia` at the exact pixel size a display asks for, caching the coverage bitmap. Sizes are not rounded to a baked size. Every build includes every language. Rendering is pixel-deterministic for a given (w, h, dpi).
- Touch targets ≥ 9 mm physical (≈ 44 dp); this is the hard constraint that drives the `small` variants.
- QR display: always rendered at the largest integer module scale that fits, with quiet zone; animated BC-UR frame rate adjusts to size.

### 4.4 Input
- Touch (Pi capacitive, phones, desktop mouse) — required; every flow is designed for touch
- Optional physical buttons (Pi GPIO) — accelerators only (scroll, back, confirm), never the sole input
- Keyboard on desktop/TUI only, and it drives every screen: Tab and the arrows move a focus, Enter and Space act on it, Escape is Back (DESIGN §4.15)
- Camera frames → the preview in the core; the QR decode (`rqrr`, the core's own `scan::decode_frame`) on a worker thread in the shell, which hands the payload back as `Event::Scanned` and never interprets it (§16.36)

### 4.5 Text input
- **One keyboard, ours.** All text entry goes through an on-screen keyboard rendered by `osk-ui` into the pixel buffer. The system IME (iOS/Android keyboards, including third-party) is never invoked; there are no native text fields anywhere. Consequence: no autofill, no accessibility-service access to typed text, no keyboard telemetry. Known trade-off: screen readers cannot read the UI (❓ document as unsupported, or add an opt-in audio-free "large text" mode).
- **Keyboard variants** per size class: BIP-39 (predictive), hex, decimal, numeric PIN, full passphrase (printable ASCII only; §16.2). All fully usable with touch or mouse.
- **BIP-39 keyboard:** candidates appear from the first letter and narrow as you type; keys that cannot lead to a valid word are disabled. Candidate count scales with size class — reference: the existing SeedSigner touch fork shows up to 6 on a 2.8" panel.
- **Physical keyboard (desktop/TUI only):** accepted via `Event::Key` and, where the shell can see a key come up, `Event::KeyUp`. Mobile shells never emit either. It drives every screen, not only the ones that take text: Tab and Shift+Tab move a focus over the layout's hit targets, the arrows move it within a row and walk a list, Enter and Space act on what is focused and hold a hold while the key is down, and a screen says nothing about any of it (DESIGN §4.15, §16.102). A PIN pad takes the keyboard's digits, Backspace and Enter as well as touch (§16.99).
- **Anti-observation defaults:** no key pop-up previews, no press highlight that persists after release, no keypress sounds or haptics for secrets, optional scrambled layout for PIN pads (mitigates click-position logging on desktop).
- **Desktop reality:** infostealers typically combine keylogging, clipboard scraping, and screen capture; the on-screen keyboard removes one vector, masking removes most of a second, and a compromised desktop remains out of scope (Section 2.3). Tier C/D docs state this.

### 4.6 Secret display and reveal policy
- Typed characters show for ~500 ms then mask to •; "paranoid" setting masks immediately.
- BIP-39 entry shows word candidates from the first letter (needed for usability). **The screen shows the word being typed and nothing else**: no chips of the words already committed, no indices; the title carries the progress ("Word 7 of 12"), and a backspace on an empty field steps back to the previous word, which returns to the field for editing. Word numbers appear only on the backup words screen, beside the words, inside its secret panel and under its mask.
- Full reveal of any secret (words, passphrase, xprv, WIF) requires an explicit press-and-hold, auto-re-masks on release, and is logged nowhere. The gesture is the **secret panel** (`docs/DESIGN.md` §4.10): a finger anywhere on the panel reveals what it holds, and the eye in the app bar shows it for 30 s on the screens a person transcribes from. Tier D never shows a full word list.
- **The eye, on the five transcription screens only** (Backup › Show words, the Create wizard's words step, SeedQR, CompactSeedQR, Explore › Words): an app-bar action that reveals the panel for 30 s without a finger on it, because a person copying a key onto steel needs both hands. The panel's caption counts the seconds down and the panel masks itself when they run out; the reveal also ends when the step, the Explore screen or the screen changes, and it is never restored on re-entry. Hold-to-reveal still works there. Every other secret panel is hold-only.
- **One mask policy**: whatever the entropy turns into is the key too. The words, their places in the wordlist, the checksum bits, a word's 11-bit group and its index are all masked with bullets of the width the value takes, and they all open together.
- Preferred verification paths never reveal the secret: fingerprint, checksum validity, address match, backup quiz (random single word).
- Screen-capture protection (FLAG_SECURE / capture detection) is always on, not just during reveal.

### 4.7 FFI to mobile shells
- **Hand-written C ABI, not UniFFI.** The contract is a handful of functions and no rich types; a binding generator would add proc-macros, generated marshalling code, and a larger audit surface for no benefit. Built as `opensigner/opensigner-ffi`, which produces `libopensigner.so`; the design as built is §16.28 and its README.
- **Surface as built** (17 functions, all integers plus one array or string):
  - `new(width, height, dpi, tier, version) -> handle` (sends `Event::Display`), `free(handle)`
  - `touch(handle, x, y, phase)`, `scroll(handle, x, y, dy)`, `tick(handle, now_ms)`, `key(handle, code, ch)`
  - `file(handle, kind, bytes)`, `fileUnavailable(handle, kind)`, `entropy(handle, bytes[32])`, `cameraFrame(handle, w, h, luma)`, `cameraUnavailable(handle)`
  - `pollCommand(handle) -> code`, then `commandKind`, `commandBytes`, `commandName`, `commandMs` for that command's payload
  - `frame(handle) -> direct ByteBuffer` over the core-owned framebuffer
  - Storage and secure-element calls are absent until the core has the matching commands (§4.2).
- **Handles, not pointers.** A shell holds a 64-bit handle into a table: slot index plus a generation counter, so a handle used after free resolves to nothing. Every entry point tolerates a bad handle by doing nothing.
- Framebuffer lives in Rust memory; the shell blits from the pointer each frame. Outbound data across the boundary is only pixels and opaque (sealed) storage blobs — no secret ever crosses.
- **No panic crosses the boundary**: every entry point runs inside `catch_unwind`. The release profile's `panic = "abort"` (§5.4) makes the point moot in a shipped build; the guard keeps debug builds and host tests honest.
- Header via `cbindgen` at dev time (tooling, not a runtime dependency) or written by hand.
- iOS: Swift calls the C ABI directly through a module map.
- Android: thin hand-written JNI over the same C ABI, using `jni-sys` for the `jni.h` type definitions only (`docs/deps/jni-sys.md`); Kotlin shell for the camera (Camera2, §16.33) and StrongBox, which are Java APIs regardless. ❓ Evaluate `android-activity` (Rust-native activity) later if the Kotlin shell grows.
- Desktop/Pi/TUI/WASM shells are Rust and call the core directly — no FFI.

---

## 5. Secret handling policy

### 5.1 Types
- `Secret<T>`: wrapper that zeroizes on drop, no `Debug`/`Display`/`Clone` by default, constant-time `Eq`.
- `Sealed<T>`: encrypted-in-memory form. Ciphertext + nonce; key held in a `SessionKey`.
- Accessing plaintext requires a closure: `sealed.with(|plain| { … })`. Plaintext is a stack/arena buffer zeroed when the closure returns. No plaintext escapes.

### 5.2 Session key
- The core has no RNG. The key is built from 32 bytes the shell supplies on `Command::RequestEntropy` (OS RNG on desktop and phones, hardware RNG on a device), asked for at start and again on every lock and wipe; the answer rotates the key and every sealed value is re-sealed under the new one (§16.21). Until the first answer the key is *weak* (mixed from what the app has: display parameters, the clock) and nothing is sealed under it; the app runs, and seals the moment entropy arrives.
- Tier B: session key is itself wrapped by the SE; unwrap requires biometric/PIN per policy. Not implemented yet.
- Held in a single `mlock`ed page where available (not yet). Rotated on lock.

### 5.3 What is never allowed
- `Debug` impls on secret types
- `log`, `tracing`, `println!`, `eprintln!` in core (enforced by lint/CI grep)
- Backtraces (`RUST_BACKTRACE` ignored; panic handler prints nothing and aborts)
- Clipboard for any secret: the core reads it only for a tap on "Paste" and writes it only for a tap on "Copy", and refuses words, a seed code, an extended private key or a private key either way (§16.88)
- Secrets in `String` (use fixed-size byte arrays; words are indices `u16`, not strings, until render time)
- Heap allocation of plaintext secrets (custom zeroing allocator as belt-and-braces)
- Screenshots/screen-recording (FLAG_SECURE on Android, `isCaptured` handling on iOS)
- OS backup of app data (opt out on both mobile platforms)
- Any file write not explicitly triggered by the user

### 5.4 Build hardening
- `panic = "abort"`, `opt-level = "z"` or `"s"`, `lto = "fat"`, `codegen-units = 1`, `strip = true`, `debug = 0`
- `#![forbid(unsafe_code)]` in all core crates except `osk-crypto`, which holds five audited `unsafe` blocks, all of them in `pinned.rs`: the page-aligned allocation that gives a long-lived secret pages of its own, its release, the two dereferences of the pointer to it, and the `mlock`/`munlock` pair over it, the last behind the `pin-pages` feature and `cfg(unix)`. This is the crate's only allocation and the workspace core's only `unsafe` (§16.49)
- `cargo deny` for license/advisory/duplicate checks; `cargo vet` for audit records
- Vendored dependencies committed to the repo (`cargo vendor`) so builds never touch the network

### 5.5 Per-platform memory reality (to be printed in docs, not hidden)
| Platform | mlock | Swap risk | Notes |
|---|---|---|---|
| Pi custom Linux | yes | none (no swap configured) | best case |
| Android | yes (limited) | zRAM compression possible | FLAG_SECURE, no backup, SE wrap |
| iOS | no reliable mlock | app memory can be paged/compressed | Secure Enclave wrap; short exposure windows are the main tool |
| Desktop | yes (may need privilege) | user's swap config | document how to disable swap |
| Browser | no | yes | Tier D — stateless, warn |

---

## 6. Storage modes

| Mode | Where | Default on | Notes |
|---|---|---|---|
| **Stateless** | nowhere; seed re-entered/scanned each session | Tier A, C, D | SeedSigner model. Nothing to steal from a powered-off device. |
| **SE-wrapped** | the words encrypted under a key derived from Argon2id of the app PIN and an HMAC the secure element computes under a per-use-authenticated key; the ciphertext wrapped once more by a hardware AES key and kept in app storage | Tier B (opt-in, off by default) | No offline attack: every guess needs the chip and a fresh authentication; eight wrong PINs delete the chip's keys, which makes any copy of the blob useless. A duress PIN deletes everything and shows an empty app. The passphrase is never stored. Device-bound: lose the device = restore from words (§16.36). |

The blob (`core/osk-keep/src/lib.rs`, format version 5,
77,039 bytes) holds four records under one 77-byte header of Argon2
parameters, salts and the attempt counter:

| Record | Plaintext | Size | Holds |
|---|---|---|---|
| keys | 5,102 | nonce 24 + 5,102 + tag 16 | a kind byte, a count, and 100 fixed slots of 51 bytes: one mnemonic and its backup flag |
| duress | 52 | nonce 24 + 52 + tag 16 | a kind byte and one such slot, or random bytes under a random key when no duress PIN is set |
| wallets | 36,866 | nonce 24 + 36,866 + tag 16 | a kind byte, a count, and 16 fixed slots of 2,304 bytes: one wallet's BIP-388 text (or its descriptor, for a wallet whose key has no origin), zero-padded |
| notes | 34,818 | nonce 24 + 34,818 + tag 16 | a kind byte, a count, and 8 fixed slots of 4,352 bytes: one note or one recovery sheet as the `osk-backup` container's kind-3 or kind-4 payload — the kind, a `u16` length and the bytes — zero-padded |

Every record is the same size whatever it holds, so the stored bytes say
neither how many keys, wallets or notes there are nor whether a duress
PIN exists. The keys, wallets and notes records open under the storage
PIN and are rewritten together whenever a key, a wallet or a kept item
changes, under the key that opened them and with no word to the element.
The wallets are public data kept under the keys' protection; forgetting
the last key forgets the blob and the rest with it.

A version-4 blob — the same header and first three records, with no
notes record — is read rather than refused, and the first write after
that makes it version 5 (§16.112 pass E3). Versions 1, 2 and 3 are not
read by this build. The version byte is outside the associated data,
because an upgrade rewrites three records and cannot rewrite the duress
record, whose key is the duress PIN's own; what the byte says is the
layout, and `header` checks it against the blob's length.
| **Encrypted file** | user-supplied strong passphrase (Argon2id) → file on SD/microSD | Tier A opt-in (Krux model) | ❓ Whether to support at all; adds attack surface. |
| **Temporary** | in RAM only, survives app backgrounding until timeout | all | Configurable timeout, default short. |

Non-secret settings (theme, tier, preferences, saved *xpubs/descriptors*) may be stored in plaintext; saved descriptors are a privacy choice the user makes explicitly.

---

## 7. I/O channels (airgap policy)

| Channel | In | Out | Status |
|---|---|---|---|
| QR (static + BC-UR animated) | ✔ | ✔ | Primary. Required on all tiers. |
| microSD / SD card | ✔ | ✔ | Tier A/C. Files: PSBT, descriptors, wordlists. Never seeds unless encrypted-file mode. |
| NFC | ✔ | ✔ | ❓ Phones only. Convenient for PSBT with Coldcard-style flows. Adds radio surface. Default off, compile-time feature. |
| USB | ✖ | ✖ | Never. No HWI-style USB signing. |
| Network | ✖ | ✖ | Never. No code exists. |
| Clipboard | ✖ for secrets | ✖ for secrets | Built (§16.88). Public strings only, read on "Paste" and written on "Copy" and at no other time: addresses, xpubs, descriptors, policies, txids, signatures, signed transactions, encrypted backups. Android uses `ClipboardManager`; the desktop shell runs the platform's own tool (`docs/deps/clipboard.md`); the Pi has none and says so. |

---

## 8. Feature catalogue

Legend: **M** = MVP (needed to be a usable signer), **1** = v1.0, **2** = later, **?** = undecided.

### 8.1 Seed creation & entropy
| # | Feature | Pri |
|---|---|---|
| 1 | Dice rolls (d6) → 128/256-bit entropy, with roll-count and bias/repeat sanity checks | M |
| 2 | Coin flips → entropy | M |
| 3 | Playing cards (52-card deck, with/without jokers, shuffle-order entropy) | 1 |
| 4 | Camera image entropy (SeedSigner style) with hash-mixing and quality check | 1 |
| 5 | Mix multiple sources (dice ⊕ camera ⊕ OS RNG) with user-visible commitment of each | 1 |
| 6 | Raw entropy entry (hex/binary) | M |
| 7 | Pure OS/hardware RNG generation (with "you are trusting the device" warning) | 1 |
| 8 | Word-by-word manual mnemonic creation with final-word checksum calculator | M |
| 9 | 12 / 15 / 18 / 21 / 24 words | M (12/24) — done; 1 (rest) — done, §16.108 |
| 10 | Entropy visualiser: show bits, hex, and how they map to words | 1 |
| 11 | EFF diceware passphrase generation (long & short wordlists) via dice | 1 — done, §16.74 |
| 12 | Card-shuffle → diceware passphrase | 2 |
| 13 | BIP-85 child seed / xprv / WIF / hex derivation, and the password applications (base64, base85) | 1 — done, §16.67 (application 39') and §16.114 pass M1 (2', 32', 128169', 707764', 707785'), all six checked against the BIP's own vectors |
| 14 | Seed XOR (Coldcard-compatible split/combine) | 1 |
| 15 | SLIP-39 Shamir backup | ? |
| 16 | Seed combination/split explainer (why XOR ≠ Shamir) | 2 |
| 17 | Alternative dice procedures, each byte-identical to the published one so a seed made elsewhere by that procedure is reproduced here | 1 — landed, §16.115 pass P1. Three procedures, taken from the sources quoted in `tools/reference/dice/README.md`: hashed (Coldcard, SeedSigner, EntropyLab's "Base 10"), which is what this device already did; six written as zero then hashed (Keystone, iancoleman's "Dice", EntropyLab's "Dice [1-6]"); and words chosen by the dice (EntropyLab's "BitBox diceware"), five rolls of 1–4 and a coin roll a word, the last word rolled too and its low bits the checksum (§16.139). The brief's "dice as coin flips" and "base-10 digits hashed" are not separate procedures; EntropyLab's D++ needs a D8 and two D16 and is not built — §16.115 has both findings |
| 18 | LND aezeed cipher seed decoding (scrypt, AEZ v5, CRC-32C) to the entropy and the LND node key path, and the LDK node key from a BIP-39 phrase; decoding only | done, §16.116 pass P2. `osk_bip::aezeed` over `osk_bip::aez` (AEZ v5 written here) and `osk_crypto`'s own scrypt and BLAKE2b, on every AEZ vector and on LND's own — which are made at `scryptN = 16` and are not decodable by a released LND, so the same seeds re-enciphered at the production cost are checked too; Tools › Lightning node key |
| 19 | Vanity address grinder over a key's passphrase counter or account index, stop on first find, the found value written back to the key | done, §16.117 pass P3. `osk_bip::vanity` over EntropyLab's counter and order, checked against two vectors its own WebAssembly produced here; the key page's Vanity address row, a budget of candidates a frame rather than a thread, the rate measured on screen; a key that already carries a passphrase has no base to extend, so that dial is dimmed there |

### 8.2 Seed import, export, and backup formats
| # | Feature | Pri |
|---|---|---|
| 1 | Type words (predictive keyboard, 4-letter uniqueness) | M |
| 2 | SeedQR & CompactSeedQR scan + display | M |
| 3 | BIP-39 word numbers (1–2048), binary, hex, decimal entry | M |
| 4 | BIP-39 passphrase (with confirm + fingerprint display) | M |
| 5 | Export to: SeedQR, CompactSeedQR (on screen), word list screen, word-number list | M |
| 5a | **SeedQR paper transcription**: display the QR as a hand-drawable grid — 21×21 (12-word Compact), 25×25 (12-word / 24-word Compact), 29×29 (24-word) — with thick borders every 3–5 modules, row/column labels, and page-by-quadrant zoom on small screens, plus a printable blank template on desktop. Matches the community SeedQR transcription templates | 1 |
| 6 | Steel/paper backup helper: grid layout with word numbers, printable template (desktop) | 1 |
| 7 | Backup verification quiz: **all** words, presented in random order, each chosen from candidates; available at any time for any loaded key (not just at creation) | 1 |
| 8 | Fingerprint display (master + per-passphrase) so users can distinguish seeds | M |
| 9 | All official BIP-39 wordlists (English, Japanese, Korean, Spanish, Chinese Simplified/Traditional, French, Italian, Czech, Portuguese) as first-class: language is asked right after word count ("What language are the words?", English default at top). Language-specific keyboards: Latin with accents (accent-insensitive matching, as BIP-39 prescribes normalization), kana keyboard for Japanese, Hangul jamo composition for Korean (arithmetic, no tables), Chinese via static lookup (pinyin/stroke/radical tables ❓ which). Wordlists embedded in binary; language stored alongside the seed for round-trip export | 1 |
| 10 | Encrypted export: `osk-backup` under Argon2id and XChaCha20-Poly1305 for anything the device encrypts — words, a master seed, a note, a wallet's recovery sheet — with `docs/BACKUP.md`, `tools/backup/decrypt.py` and published vectors, and KDBX 4 as a write-only second form so that an heir opens an export in a KeePass app. age and PGP are not proposed (§16.112) | Done (§16.78, §16.96, §16.112 passes E1 and E2) |
| SLIP-39 Shamir backup: load a key from shares, split a key into shares (Trezor's; the master secret is the seed, not BIP-39 words) | 1 — landed. `osk_bip::slip39` (§16.106) on all 45 published vectors; the screens in §16.107 — Load a key › SLIP-39 shares (pass S1), Add a key › Create SLIP-39 shares and Backup › SLIP-39 shares, in groups and thresholds (pass S2) |
| Codex32 (BIP 93): load a key from shares or one string, write one; error detection now, correction with the screen that offers it | 1 — landed but for error correction. `osk_bip::codex32` (§16.106) on every inline vector; the screens in §16.109 — Load a key › Codex32, the key kind, Scan's route and Tier B's slot (pass C1), Add a key › Create Codex32 shares, Backup › Codex32 for every key and "Type it back" (pass C2). Error correction is still open, with the screen that would offer it |

### 8.3 BIP-39 / key explorer tools
| # | Feature | Pri |
|---|---|---|
| 1 | Word ↔ index ↔ 11-bit binary ↔ hex explorer | M — done, §16.74 |
| 2 | Full mnemonic → entropy bits → checksum bits breakdown, checksum validity | M |
| 3 | Mnemonic → seed (PBKDF2) → xprv → xpub step display | 1 |
| 4 | Derivation-path explorer: type any path, see xpub/xprv/address at each level | 1 |
| 5 | Address explorer: legacy/nested/native segwit/taproot; receive & change; N addresses; QR | M |
| 6 | Address verification: scan an address QR, tell user whether it belongs to this seed/descriptor (and which index) | M |
| 7 | xpub ↔ ypub/zpub/Ypub/Zpub SLIP-132 conversion (with "this is just an encoding" explainer) | 1 |
| 8 | WIF / hex private key display for a single address (guarded, for sweeps) | 1 |
| 9 | Descriptor builder: single-sig, multisig, taproot; checksum calculator/validator | 1 |
| 10 | Miniscript policy → descriptor compile & analyse (timelocks, spend paths) | 2 |
| 11 | Base58/Bech32/Bech32m/hex/binary encoder-decoder scratchpad | 1 |
| 12 | Hash tools: SHA-256, double-SHA, HASH160, HMAC — for verifying things by hand | 1 |
| 13 | Unit converter (sat/BTC/mBTC/bits) | 2 |

### 8.4 Signing & PSBT
| # | Feature | Pri |
|---|---|---|
| 1 | Scan/load PSBT (QR, BC-UR, SD, file) | M |
| 2 | Full PSBT inspector: inputs, outputs, fee, fee rate, sighash types, locktime, RBF flag, version | M |
| 3 | Change detection via descriptor/derivation, with loud warning if change can't be verified | M |
| 4 | Self-transfer / consolidation detection | 1 |
| 5 | Multisig: show quorum, which cosigners already signed, whether this key is a participant | M |
| 6 | Taproot key-path and script-path signing | M (key-path), 1 (script-path) |
| 7 | Deterministic nonces by default: RFC 6979 for ECDSA, BIP-340 with fixed aux-rand for Schnorr | M |
| 8 | **Nonce validation** (deterministic mode): recompute the expected nonce independently and confirm the produced signature matches; display per-input "nonce OK". Plus **cross-implementation check**: export signature bytes so the user can compare against the same PSBT signed by another implementation — identical bytes prove no hidden nonce channel | M |
| 9 | **Anti-exfil / anti-klepto** (Blockstream protocol): deterministic base nonce + host randomness, host verifies the commitment. Layered on 6979, not a replacement; per-signature choice, requires coordinator support (Jade/Green today). In this mode validation is host-side, not on-device | 1 |
| 10 | Sign-then-verify: every signature is verified against the pubkey before output | M |
| 11 | Warnings: unusual sighash, high fee, dust outputs, non-standard scripts, mismatched network, unknown derivation | M |
| 12 | Output PSBT (QR/BC-UR/SD/file); output finalized tx hex when all sigs present | M |
| 13 | Message signing: BIP-137 (legacy), BIP-322 | 1 |
| 14 | Signature verification (given address + message + sig) | 1 |
| 15 | Raw tx decoder (paste hex → structured view) | 1 |
| 16 | Wallet registration / descriptor export QR for coordinators (Sparrow, BlueWallet, Nunchuk, Specter…) | M — done (§16.68, §16.71, §16.72): loaded from Add › Load a wallet or Scan (§16.75) as a policy, a descriptor (single-key or multisig), a bare extended public key, a coordinator's text config or Coldcard's JSON account export; exported as a checksummed descriptor or a BIP-388 policy, as text and as a code; kept across restarts on Tier B. The UR key forms (`crypto-hdkey`, `crypto-account`) remain |
| 17 | Multisig config verification: re-derive from xpubs, show descriptor checksum, detect swapped xpubs | 1 — done for re-derivation and the checksum (§16.68, §16.72): the review states the quorum, the checksum, the script type and every key with its glyph, or "origin unknown" where a key arrived without one, and the wallet's own address list is derived from the policy; swapped-xpub detection remains |
| 18 | Testnet / signet / regtest | M |
| 19 | MuSig2 (BIP-327/373/390) — key aggregation, `musig()` descriptors, address verification, PSBT field inspection | 1 — BIP-327 `KeyAgg`/`KeySort`/`ApplyTweak`, BIP-328 aggregate xpubs and BIP-390 `musig()` in `tr()` in `osk-bip::musig` (§16.69); `Template::MuSig` loads, reviews, exports and derives addresses for a `tr(musig(…)/**)` wallet (§16.70), and Verify answers for its addresses; BIP-373's PSBT fields, and with them change detection for such a wallet, remain |
| 20 | MuSig2 signing: two-round flow over QR; secret nonces sealed in memory, session-bound, single-use, destroyed on sign/lock/timeout; explicit "session open — do not power off" state; never persisted, on any tier, including Tier B where the secure element could wrap them (§16.47) | 2 — designed (§16.100): the PSBT fields, signing last with no session against Bitcoin Core 31.1, then the session |
| 21 | Threshold wallet, dealer FROST on one device: BIP 445 signing, trusted-dealer key generation, shares as 24-word phrases, a public group record, `osk` PSBT records until a BIP assigns bytes, one bound nonce file in transit and nothing kept on the device | done (§16.103), in four passes. F1: BIP 445's algorithms and the trusted dealer in `osk-bip::frost`, against all six vector files. F2a: the group record, `Template::Threshold`, the synthetic xpub and the addresses, and the app reading, reviewing, using, exporting, keeping and forgetting a threshold wallet it holds no share of. F2b: `LoadedShare`, Add > Load a share and Add > Create a threshold wallet, the share label, the share's own page and backup, and the key glyph on a wallet this device holds a share of. F3: the `osk` PSBT records, the carry file, the two locations, the Sign flow, and Bitcoin Core and BIP 445's reference implementation both judging the result |
| 22 | Taproot multisig by script path, `tr(NUMS, sortedmulti_a(k, …))` (BIP 387): load, review, addresses, change, sign, build | 0 — done (§16.106 passes R5 and D3): the Add a wallet kind, the account at BIP 48's `2'`, the review's "Key path · unspendable" row and the Wallets row, and the Bitcoin Core 31.1 round trip with its three fixtures — Core imports `sortedmulti_a`, funds it, and accepts the transaction two of the three keys signed here |
| 23 | MuSig2 key path with a multisig script fallback, `tr(musig(…), sortedmulti_a(…))` (BIP 390's example, what Core builds) | 2 |
| 24 | Recovery and inheritance wallets built on the device from keys and delays, Liana's shapes in SegWit and taproot, decaying multisig | 1 — the kind and its screens landed (§16.106 pass R5), and up to three recovery paths with a wait typed in days in pass D2: "Another path later?" after every wait, each wait longer than the last, "Type a number of days" on a pad bounded at 455, both forms, and a scanned Liana descriptor reviewed as Recovery with a row per path; decaying multisig remains |
| 25 | BSMS (BIP 129) signer and descriptor records, read and written | 0 — done but for encryption (§16.110 pass D1): the descriptor record's review and its Export landed in §16.106 pass R5, and the key record is written from a key's Account key row with the session token and the description typed, signed by the account node's own key through a door on the master. Encrypted records remain, and need AES-256-CTR and HMAC-SHA256 in the tree |
| 26 | Verify the signatures already on a PSBT: every ECDSA and Schnorr signature, in partial-signature fields and inside finalized witnesses, checked against its key and the input's sighash, reported per signature | Done (§16.111 pass V1) |
| 27 | Repeated-nonce detection: the same `r` under one public key within a transaction is a danger the review refuses; a cross-session nonce history as an optional encrypted file, later | Done (§16.111 pass V1); the history waits on §16.112 |
| 28 | Deterministic-nonce check of another device's signature: a signature by a key this device holds recomputed under RFC 6979 (low R and first) or BIP-340 with zero aux, and the review says whether it matches; the cross-implementation check of row 8 done on the device | Done (§16.111 pass V1) |
| 29 | Compare two PSBTs: the transaction, the signing state and the metadata diffed on decoded contents | Done (§16.111 pass V1) |
| 30 | Inscription envelopes in tap leaves and witnesses reported on the review | Done (§16.111 pass V1) |
| 31 | Silent Payments (BIP-352): the `sp1q…` address and labelled addresses from a key, BIP-392 `spscan`/`spspend` descriptors, the BIP-321 URI and BIP-353 DNS record, and verification of a received output; sending waits for BIP-375's PSBT fields | Receiving done, §16.113 pass SP1, every published vector passing; `spspend` not written and sending not built |
| 32 | Bitcoin Core exports for a wallet: `importdescriptors` JSON | 1 — done, §16.114 pass M1, accepted by Bitcoin Core 31.1 on regtest |
| 33 | Anti-exfil transcript verification (Jade's sign-to-contract), verifying another device's commitment without a key | 2 — with row 9 |

### 8.5 Device & session
| # | Feature | Pri |
|---|---|---|
| 1 | Lock screen / PIN (with tier-appropriate backing) | 1 |
| 2 | Auto-lock & wipe timers | M |
| 3 | Multiple seeds loaded at once. Every screen that touches a key shows its master fingerprint (and passphrase-fingerprint if applicable) in a fixed position; signing shows which loaded fingerprints participate in the PSBT; seed switcher lists by fingerprint with optional user label. Fingerprint is the *only* identity — no auto-generated names | M |
| 4 | Assurance-tier display, build hash, version | M |
| 5 | Self-test on start: run BIP-39/32/PSBT test vectors, refuse to run on failure | M |
| 6 | Memory wipe on exit (explicit) | M |
| 7 | Decoy/duress PIN (opens empty session) | ? |
| 8 | Screen brightness/theme (dark default) | 2 |
| 9 | Localisation | 2 |
| 10 | The app's shape: Home is a launcher of six tiles, Keys is a flat list of keys, a wallet is a policy over them and is added explicitly, and a wallet's page carries the keys it is made of | done (§16.104), in three passes. R1: the launcher, Keys, Wallets, Add a key's four ways, explicit single-sig wallets, the key page and the wallet page with its filtered Keys review. R2: Add a wallet as one wizard for all four kinds, FROST built from loaded keys, the word "share" off every working screen. R3: the checked passphrase entry from a wallet's not-loaded row, Tier B's keep choice for a wallet over a passphrase key and the row that changes it, and the blob rules as tests. E1 (§16.112) added a wallet's Recovery sheet row and Tools › Notes to the same shape |

### 8.6 Education / explainers (short in-app cards)
Why passphrases, why verify change, what a fingerprint is, nonce attacks, why dice, single-sig vs multisig, xpub privacy. Priority 1–2.

### 8.7 Explicitly out of scope
- Any network access, fee estimation from network, balance display, UTXO fetching
- Lightning
- Altcoins (❓ Liquid — probably no)
- Coin-join coordination
- Being a "wallet" in the coordinator sense — we are a signer + toolbox

---

## 9. Standards to implement / support
BIP-32, 39, 43, 44, 48, 49, 84, 85, 86, 87 (multisig descriptors), 137, 174 (PSBT v0), 370 (PSBT v2 ❓), 322, 340/341/342 (Schnorr/Taproot), 350 (bech32m), 380–386 (descriptors), 388 ❓ (wallet policies), 327/373/390 (MuSig2, PSBT fields, descriptors), SLIP-132 (encodings only), SLIP-39 ❓, BC-UR (crypto-psbt, crypto-account, crypto-hdkey, bytes), SeedQR / CompactSeedQR, Coldcard Seed XOR, Blockstream anti-exfil, RFC 6979, EFF wordlists.

---

## 10. Dependencies

### 10.1 Policy
- Crypto: only widely-used, audited, actively-maintained crates. Pinned versions, vendored, `cargo vet` audit record for each.
- Everything else: prefer zero dependencies; if needed, prefer small `no_std` crates with no transitive deps.
- No crate count target. Every crate in a device's release dependency graph is there because something in the architecture needs it and nothing in the tree can do the work; any addition needs a written justification in `docs/deps/`. The core graph stands at 51 crates, the workspace's own eight among them, and §16.62 says why that is the floor (§16.60, §16.61).
- No build scripts (`build.rs`) that download anything.
- No proc-macro crates in core if avoidable (they execute at build time).

### 10.2 Candidate crates (to be verified at time of build — versions omitted deliberately)
| Purpose | Crate | Notes |
|---|---|---|
| Bitcoin primitives, PSBT, Taproot | `bitcoin` (rust-bitcoin) | core. In use since milestone 2: the 0.32 line with `default-features = false` (`no_std` + `alloc`; no `std`, `rand`, `serde` or `secp-recovery`), which is why `osk-bip` allocates. Its `Xpriv::decode` skips two BIP-32 vector-5 checks (the zero pad byte before the key, and zero parent fingerprint/child number at depth 0), so an extended key that arrives as text is decoded by `osk-bip`'s `xkey` instead: base58check, length, version, depth, pad byte and key material are checked before `bitcoin` sees the bytes (§16.47) |
| secp256k1 | `secp256k1` (bindings to libsecp256k1) | the one C dep we accept: it is a required dependency of `bitcoin` and `miniscript` regardless, and the only option with a MuSig2 module. Kept behind `osk-crypto`'s signing interface so pure-Rust `k256` can be added later as a cross-check or replacement (§16.1) |
| Descriptors/miniscript | `miniscript` | core |
| BIP-39 | in-house, in `osk-bip` | ~300 lines plus wordlists; the `bip39` crate is a dev-dependency used to cross-check every vector (§16.2) |
| Hashes | `bitcoin_hashes`, `sha2`, `hmac`, `pbkdf2` | |
| KDF for encrypted storage | `argon2` | only if encrypted-file mode kept |
| AEAD for in-memory sealing | `chacha20poly1305` | |
| Stream cipher for the KDBX 4 export | `chacha20` | the crate `chacha20poly1305` is built on, already in the graph and not re-exported by it; KDBX encrypts with the bare cipher and authenticates with its own HMACs (§16.112 pass E2, `docs/deps/chacha20.md`). No new crate |
| Zeroization | `zeroize` | |
| QR decode | `rqrr` | pure Rust; needs `std`, so it sits behind `osk-codec`'s `decode` feature (§16.20, `docs/deps/rqrr.md`) |
| QR encode | in-house, in `osk-codec` | no published encoder is `no_std`; a port of Nayuki's `qrcodegen` with zeroizing buffers (§16.20) |
| BC-UR | in-house, in `osk-codec` | BCR-2020-005 and -012 in about a thousand lines, `no_std` + `alloc`, no dependency; the `ur` crate it replaced cost nine (§16.61, `docs/deps/ur.md`) |
| Rendering | `tiny-skia` | pure Rust, AA, vendored + reviewed; small dep tree (`tiny-skia-path`, `arrayvec`, `bytemuck`…) |
| Glyph outlines (build-time only) | `ttf-parser` | runs on the build machine, not on device; reads TrueType and CFF outlines (§16.39) |
| Desktop window | `winit` + `softbuffer` | shells only |
| Camera on Linux | `libc` | shells only; the raw V4L2 `ioctl`, `mmap` and `poll` declarations for `opensigner-v4l2`, one of the four places allowed `unsafe` (§16.33, `docs/deps/libc.md`) |
| Pinning secret pages | `libc` | `osk-crypto`, behind its `pin-pages` feature and `cfg(unix)`, which only the desktop and Pi shells turn on: `mlock` and `munlock` over the page-aligned allocations that hold the session key and the loaded master keys. §5.4's exemption, and the crate's only `unsafe` (§16.49) |
| Camera on macOS | `cc` | shells only, build time only, macOS only; compiles the one Objective-C file of `opensigner-avfoundation`, another of them. Already in the graph: `secp256k1-sys` builds libsecp256k1 with it (§16.33, `docs/deps/cc.md`) |
| Pi SPI/I²C | `rppal` or raw `spidev` ioctl | shells only |

Four crates lift `#![forbid(unsafe_code)]`, and no others: `osk-crypto` for the pinned-page allocation and the `mlock` pair (§5.4, §16.49), `opensigner-v4l2` for the V4L2 ioctls, `opensigner-avfoundation` for its Objective-C bridge, and `opensigner-ffi` for the JNI entry points (§5.4's tree line, §16.33).

### 10.3 Things we will write ourselves
SeedQR codec, BC-UR (bytewords, the fountain codec and the part CBOR; §16.61), Seed XOR, EFF diceware, entropy statistics, the whole UI/state machine, anti-exfil client side, nonce validation, storage sealing, MuSig2 session state machine (the BIP-327 math itself comes from `secp256k1`'s musig module).

---

## 11. Build, reproducibility, distribution

### 11.1 Reproducibility — hard requirement
A release is **blocked** unless at least two independent builders (different machines, different people) reproduce every artifact from the tagged source. Per-platform definition of "reproduced":

| Artifact | Standard |
|---|---|
| Pi image | Bit-identical image (Buildroot with pinned toolchain, `SOURCE_DATE_EPOCH`, no timestamps) |
| Desktop binaries | Bit-identical |
| HTML/WASM single file | Bit-identical; hash printed in the app's About screen |
| Android APK | Identical after stripping the signing block (F-Droid method); the Rust `.so` inside bit-identical |
| iOS | Rust static library bit-identical; full app bundle verifiable only for sideloaded/AltStore builds — App Store re-signing/processing is documented as unverifiable |

Reproduction results (builder, hash, date) are published with each release. The in-app About screen shows the core-library hash so users can compare against published values on every platform, even iOS.

### 11.2 Build details
- Pinned Rust toolchain via `rust-toolchain.toml`; builds run in a pinned container.
- Pi and stick images: both inputs the build pulls from a network are pinned by hash, not by name. Buildroot is cloned at tag `2024.02.13` and the build fails unless `HEAD` is commit `e687e3815f76c1c5ea9fb52b6558bedfe53ab117` (`justfile`'s `buildroot_commit`, checked in `run-build.sh`); the build container is `debian:bookworm-slim@sha256:88200866…` in `opensigner/shells/pi/image/Dockerfile`. The kernel tarball is pinned by the board's `# KERNEL_SHA256` line (§16.95). A tag and an image name are both things somebody else can move.
- Reproducible-build CI that builds twice from clean and diffs (a pre-check; does not replace independent reproduction).
- Release artifacts signed (minisign/OpenPGP ❓); hashes published; in-app "About" shows build hash for comparison.
- Pi image: Buildroot-based, read-only rootfs, no networking stack compiled into kernel, no swap, no shell/SSH in release image, our binary as init or sole process. True since §16.93, and checked on every build by `opensigner/shells/pi/image/check-kernel-config.sh`.
- Android: F-Droid-style reproducible APK; no Play Services; `targetSdk` current; all permissions except camera absent from manifest.
- iOS: App Store constraints acknowledged; ❓ also support sideload/AltStore builds for reproducibility.
- Desktop: static binaries; single HTML build is a separate artifact with a prominent Tier-D banner.

---

## 12. Testing strategy
- Every BIP test vector in-repo; self-test on device start.
- Cross-check signing against at least two independent implementations (e.g. Bitcoin Core `signrawtransactionwithkey`, and a Python reference) in CI.
- Property tests: sign → verify round trips; PSBT parse → serialise idempotence; SeedQR round trips.
- Tests are Rust unit and integration tests (`cargo test`), nothing else, until there is a significant UI. The layout engine and state machine are tested on their outputs (solved rectangles, screen transitions, emitted `Command`s), not on pixels. Pixel-level or golden-image tests are deliberately deferred (§16.7).
- Fuzzing: PSBT parser, QR payload parsers, descriptor parser, BC-UR decoder.
- A "hostile coordinator" test suite: PSBTs designed to hide change, alter fees, swap outputs, use odd sighashes — the app must warn on each.
- Memory tests: run under a debugger/Valgrind-style tool after a signing flow and scan process memory for the seed bytes → must find zero occurrences.

---

## 13. UX principles
- Every screen: what am I looking at, what is at stake, what action am I confirming.
- Irreversible actions — sign, forget a key, wipe every key, wipe and exit — take a tap to a screen that states what will happen and a hold on that screen to do it. Reversible ones, adding a key included, are a tap (§16.27).
- Secrets are hidden by default; reveal is explicit press-and-hold, and re-hides on release/timeout (Section 4.6).
- Fingerprints everywhere a seed is referenced.
- Warnings are ranked (info / caution / danger / blocked). Danger warnings cannot be dismissed with the same tap that confirms, and a blocked warning cannot be dismissed at all: the signer refuses the transaction.
- Same navigation model on a 240×320 panel and a 6" phone; size classes only re-flow, never re-order.
- No jargon without a one-tap explainer.
- **Long strings for human comparison** (addresses, xpubs, hashes, descriptors, txids): chunked in groups of 4 with alternating emphasis, monospace, and shown *entirely* on one screen at the largest size that fits. **Only strings longer than sixteen characters are chunked**: a fingerprint is eight hex characters and reads as one word, so it is never split (§16.23). Never marquee/scroll, never paged inside a panel. Inside another screen a long string is a reference row (head and tail, one tap away from the whole string on its own Compare screen); the whole string is drawn only there and on the Address screen (`docs/DESIGN.md` §4.5). A verification screen never asks the user to accept a string from its head or tail alone.
- The screen rules themselves — chrome, placement, the content inventory, the sixteen reusable screens — are `docs/DESIGN.md` (§16.31). This section keeps only the principles above.

### 13.1 Screen review instead of wireframes (see UX.md §8)
- No HTML wireframes. Because the core renders pixels, the cheapest wireframe is the real renderer. A headless **snapshot** shell takes `--size WxH --dpi N` plus a scripted event sequence and writes PNGs; the desktop shell takes the same flags for interactive use. Every screen is reviewed at the four reference sizes of §4.3: 240×320 @ 143 dpi and 480×640 @ 286 dpi (`small`, portrait), 1080×2340 @ 420 dpi (`mobile`) and 960×640 @ 160 dpi (`wide`, which is also the desktop window's own size).
- The snapshot shell is a review tool only. It is not a test harness (§16.7).
- Order: Home + key strip + scanner → Load → Sign → Verify address → Create (dice) → Key detail + backup → Wallet export/config → Explore → Learn → Settings (UX.md §8).

---

## 14. Roadmap (rough)
| Phase | Deliverable | Status (2026-09-07) |
|---|---|---|
| 0 | Documents iterated; threat model reviewed; crate shortlist vetted. Ongoing — §16 records decisions as they are made | done for the decisions in §16; the rest iterates |
| 1 | Workspace skeleton; `osk-ui` layout engine; snapshot and desktop shells; screens reviewed as PNGs at three sizes (replaces wireframes) | done: workspace, `osk-ui`, snapshot + desktop shells, four review sizes |
| 2 | `core` crates: bip, psbt, entropy, codec — with full vector tests, TUI harness. Runs in parallel with Phase 1 | done for bip, psbt, entropy, codec; TUI harness dropped in favour of the snapshot shell |
| 3 | `opensigner-core` state machine on top of `osk-ui`; flow tests driving `Event`s and asserting on `Command`s | done for Load, Create, Sign, Verify (address), Explore, Settings, session lock/wipe, Learn (nine pages, §16.36); every screen rebuilt on the design system (§16.31–§16.32) and re-reviewed (§16.33); remembering a wallet config is §15 item 36 |
| 4 | Pi shell (display, touch, camera) in this repository + a Buildroot image built in this repository → first end-to-end signing | done for the first device: `just pi-image` builds the card (§16.35), the 3B+ with the 2.8" panel boots into the app, touch, camera and QR decode work on the panel; the file channel and settings persistence landed (§16.36); a second board is next |
| 5 | Android shell + StrongBox wrapping | shell built (§16.28) with Camera2 capture (§16.35), debug APK at `out/android/`; StrongBox not started |
| 6 | iOS shell + Secure Enclave wrapping | not started |
| 7 | Toolbox features (8.3/8.4 priority-1 items), anti-exfil | partly: nonce validation, SeedQR, SLIP-132, path explorer, message signing, multisig registration (§16.68) done; anti-exfil pending |
| 8 | External review / audit; reproducible-build verification by third parties | not started |
| 9 | HTML/WASM Tier-D build | not started |

---

## 15. Open questions (❓ collected)
1. ~~Pure-Rust `k256` vs libsecp256k1 bindings for the curve.~~ Decided: §16.1.
2. ~~Implement BIP-39 in-house or use the `bip39` crate.~~ Decided: §16.2.
3. ~~RGB565 vs RGB888 framebuffer; minimum supported resolution; `tiny` size class.~~ Decided: §16.3, §16.4.
16. Component set: derived from the screen reviews (§13.1), not defined up front. Layout primitives (row/column/weight/min-max/padding/scroll/safe-area) are fixed now.
4. Keep encrypted-file (SD) storage mode at all?
5. NFC on phones: worth the radio surface?
6. ~~SLIP-39 support.~~ Decided: yes, in full, as a key kind of its own (§16.107).
7. PSBT v2 (BIP-370) at launch.
8. ~~BIP-388 wallet policies vs raw descriptors for multisig registration.~~ Decided: BIP-388 wallet policies (2026-09-09).
9. ~~Duress PIN.~~ Decided: yes, for the key kept on a Tier B device; entering it deletes the stored key and shows an empty app, and nothing in the stored bytes tells it from the real PIN (§16.36).
10. ~~Which Pi camera module to standardise on; whether to support USB webcams on Pi.~~ Decided: OV5647 (v1 / Zero camera) on the legacy stack, IMX219 also works, v3 is out; a USB webcam is the same V4L2 module and may work but is not a target (§16.33).
11. Bare-metal Pi as an eventual Tier A+.
12. ~~Release signing scheme and who holds keys.~~ Decided in part: no app stores, no Apple Developer Program, self-generated keys for Android and Linux, ad-hoc signing on macOS (§16.33). Who holds the Android and Linux keys, and how they are rotated, is still open. Decided further (2026-09-09): release builds are published on GitHub releases with a GPG-signed manifest of SHA-256 sums and verification instructions for users, the way Sparrow does; the signing key lives in the owner's isolated vault and signing runs through the build box's wrapper, never in a session (2026-09-09).
13. ~~MuSig2 nonce state in Tier B persistent mode: allow SE-wrapped persistence of an open signing session, or force in-session only everywhere?~~ Decided: in-session only, everywhere (§16.47).
14. Crate prefix `osk-`: check crates.io availability before first publish.

Raised during implementation (2026-09-07); each is answerable by the owner, none blocks the next milestone:

17. Keys loaded by typing are marked "backup unverified" until the quiz passes. Keep, or trust a typed key?
18. Passphrases are printable ASCII only (§16.2). Confirm, or plan NFKD support.
19. BC-UR: keep the `ur` crate (nine crates, a second major of `bitcoin_hashes`) or write the fountain codec in-house once the crate budget bites. *Settled 2026-09-11: in-house, §16.61.*
20. ~~Weak-session window: when the shell has not yet answered `RequestEntropy`, "Hold to add" proceeds with the seed in an unsealed `Secret`. Block instead?~~ Decided: blocked. "Add key" is dead until the answer arrives (§16.48).
21. Signing: an unsupported input that names a selected key is an error today. Skip it and sign the rest?
22. `MissingUtxo` is a danger for every input, including inputs that are not ours; coinjoin-style PSBTs would need the force path.
23. ~~`KeyRef` carries only single-sig account xpubs; BIP-48 xpubs would let multisig change be verified before descriptor registration exists.~~ Done: `KeyRef` carries BIP-48's two account xpubs as well (§16.47).
24. ~~rust-bitcoin 0.32 accepts seven of the sixteen BIP-32 vector-5 invalid keys (five distinct reasons; the item said five); xprv import needs an in-house strict decoder (§10.2).~~ Done: `osk-bip::xkey` decodes every extended key that arrives as text (§16.47).
25. ~~`STATIC_QR_MAX_BYTES` (2953) yields a version-40 code at 2 px per module on the 2.8" panel; lower it there, or always prefer UR on `small`?~~ Decided: the 0.5 mm pitch floor at the class's QR side decides static against animated and the part count (§16.33, DESIGN §4.9).
26. Explore in typed-input mode keeps a `MasterKey` in RAM for the duration of the visit rather than re-deriving per frame.
27. ~~On `wide` the sidebar is hidden during Sign, the scanner and the lock screen so secrets never share a screen with navigation. Show it greyed instead?~~ Decided: §16.29 — greyed, on every screen.
28. ~~The desktop window is fixed at 960×640; a builder on a large monitor may want an integer zoom, which `--scale` already gives the shell.~~ Decided: fixed; `--scale` is the zoom (§16.27).
29. ~~`mlock` on Tier A/C is not implemented.~~ Done (2026-09-10): the session key and every loaded master key sit on pages of their own that `osk-crypto` pins, behind the `pin-pages` feature the desktop and Pi shells turn on; a refusal is carried on from and reported once on stderr (§16.49). What remains of this item is session PIN backing on Tier B: the secure element holds no session PIN yet (§16.48).
30. ~~`Event::FileUnavailable` cannot say why: on Android a cancelled document picker reaches the user as "This device has no file channel", which is wrong. Split the event, or reword it to cover both?~~ Decided: split, into `FileUnavailable` and `FileCancelled` (§16.48).
31. ~~The Android shell ticks every 50 ms whether or not it is in the foreground, so auto-lock and auto-wipe run while the app is backgrounded. That is the safe default and it costs battery; a stopped activity could instead lock at once.~~ Decided: `onStop` locks at once and stops the ticker; the clock is `elapsedRealtime`, so the wipe timer is honoured across the gap (§16.48).
32. ~~Android Tier B needs `SecureWrap`/`SecureUnwrap` in the contract and StrongBox behind them, plus a decision on whether persistent-seed mode is offered at all (§6).~~ Decided: persistent-seed mode is offered on Android, opt-in and off by default, behind hardware-backed key wrapping and the app's own PIN; the layering is settled in §16.36 once the owner confirms it.
33. The Android APK is debug-signed and not yet reproducible (§11.1): F-Droid-style verification needs a release signing story and a pinned container that builds both the `.so` and the APK.
34. ~~Safe-area insets: the core assumes a 24 dp gesture area at the bottom of every `mobile` display.~~ Done: `DisplayInfo` carries the shell's insets and the token is gone (§16.36).

Raised by the design system and the re-review (2026-09-08 and 09); answerable by the owner, none blocks the next milestone:

35. ~~SLIP-132 for Taproot: there is no SLIP-132 prefix for it, so the format Choice dims the row ("same as account key"). Hide the row for Taproot instead?~~ Decided: the row is absent for Taproot, not dimmed (§16.46).
36. Descriptor review (job E2): a scanned descriptor gets a review screen (checksum, script type, key count, the keys and their glyphs) but nothing remembers a wallet across sessions on a stateless tier. Is "remember this wallet" in scope, and for which tiers? **Decided (2026-09-12, §16.71 decision 5) and built (§16.72):** on Tier B the policies in use follow the kept keys into the blob, public data under the same protection; nothing is kept on a stateless tier, and nothing on Tier A.
37. ~~How many BIP-39 languages ship.~~ Decided: all ten, none dimmed. Each script has a keyboard of its own — QWERTY, the 五十音, the two-set layout, pinyin and 注音 — and every list is typed, read and quizzed end to end (§16.40–§16.42).
38. ~~Explore reachable with no key from Home.~~ Decided: yes; Explore's empty state offers "Type words" and "Load a key" rows.
39. ~~Sign review: the change row shows the amount only; "verified" and the MINE badge are on the output screen, because the word does not fit the panel's table. Show the state on the review too, at the cost of a scroll on `small`?~~ Decided: the review states it too, and the panel's table scrolls (§16.46).
40. Verify's address entry uses the passphrase keyboard; a bech32/base58 layout (§4.3 names it) is not built. The typed address is shown whole above the keyboard meanwhile.
41. Helper mode on the quiz is a " · helper" title suffix. Enough, or a badge component?
42. ~~No PSBT fixture triggers a warning card, so job F3 (the coordinator lying or careless) has never been reviewed on a render.~~ Done: a fixture per warning and the Warnings step in `just snapshots` (§16.36).
43. ~~PIN pad shuffled by default.~~ Decided: off by default; the setting stays (DESIGN §4.3).
44. ~~Explainers behind ⓘ against Learn pages: the PIN shuffle, the passphrase consequence and the quiz rule have no explainer today; which of them is worth a Learn page (DESIGN §7).~~ Decided: no ⓘ anywhere and no prose on a working screen; every explanation is a Learn page (§16.37).
45. ~~Settings do not persist (no storage yet), so the camera-rotation setting (§16.35) is set again after every boot.~~ Decided: the six Settings rows persist in plaintext, nothing else does, and Tier D keeps nothing (§16.36).
46. Pi Zero W and the original Zero are ARMv6: a second Rust target (`arm-unknown-linux-gnueabihf`), C flags for VFPv2, and a board directory. The Zero 2 W is the 3B+'s SoC family and has a board directory already, untested. Whether the ARMv6 boards are worth a toolchain in the image build depends on the decode cost after §16.35's decode work.
47. ~~If a rate-limited, reduced-frame decode still leaves the Pi's scan screen rough, the remaining shape is SeedSigner's: a decode thread.~~ Done: the shell decodes on a worker with the core's own policy; the core keeps the preview and the routing (§16.36).
48. The mark. The pixel key (§16.35) is the boot logo and the Android launcher icon; the desktop bundle, the About screen and the README have no mark yet.
49. ~~The Pi's file channel offers the newest file on the card that could be what was asked for (§16.36), because the core has no list screen and the shell draws nothing. A person with several PSBTs on one card has to know the rule. A file-list screen would be a seventeenth screen in DESIGN §5 and a contract change (a list event, a choice command); whether it is worth that, or the rule stays, is the owner's call.~~ Decided: a file list, as a Menu of the files the shell listed, on the Pi and the desktop; Android keeps the system picker (§16.36).
50. ~~`Command::WriteFile` has no answering event, so a save that fails leaves the core's "saved as" row saying a file was written.~~ Done: `FileWritten` / `FileNotWritten` (§16.36).
51. Anti-exfil (§8.4 item 9) needs a nonce the signer tweaks by the host's randomness, which the safe `secp256k1` crate cannot produce: only `sign_ecdsa_with_noncedata` exists, and the tweak needs libsecp256k1's nonce-function hook through the `-sys` crate, which is `unsafe` in a third crate. More to the point, no coordinator drives the protocol over QR or PSBT today: Jade speaks it to Green over its own RPC, and no PSBT field carries the commitment exchange. Building the signer half now is code nothing can call. Proposed: defer until a coordinator adopts a QR or PSBT form, and record here which one when it appears (2026-09-09).
52. Secure Boot signing with a key the owner holds. The stick's
`BOOTX64.EFI` is unsigned, so a machine that enforces Secure Boot takes
it only if the firmware is put into setup mode and the image's hash
enrolled — per release, per machine. Signing the EFI application with a
key of the owner's, and publishing the certificate for one enrolment
that outlives every release, is the alternative; it needs a key, a place
to keep it, a signing step in the release flow that never runs in a
session, and a decision about whether an unsigned build stays available
for people who would rather enrol a hash. Raised 2026-09-16 (§16.95).
53. A tested-laptops list. `boards/x86_64-uefi` is marked untested: the
stick has booted in QEMU with OVMF and on no physical machine. What a
list would record per machine — firmware vendor and version, whether
Secure Boot had to be turned off, whether the framebuffer came up,
whether the built-in keyboard and touchpad were found and which bus they
were on, whether the webcam was USB or CSI-2 — is the shape of the
question; where it lives (this file, the image README, a page in
`docs/`) is not settled. Raised 2026-09-16 (§16.95).
54. A screen density that can change without a rebuild. The stick's
density is fixed at 160 by `opensigner.dpi=160`, compiled into the kernel
command line, so a 4K laptop panel draws the whole product about a
quarter of the size it should be and the only fix is to edit
`boards/x86_64-uefi/cmdline.txt` and build an image. Two shapes: a
density row in Settings, which every device would get and which the core
already has a place for; or a `dpi=` line in the settings file on the
exchange partition, which the shell already reads and which needs no new
screen. Neither is built. Raised 2026-09-16 (§16.97).
55. ~~A way through the first screens with the keyboard alone. Everything
is reached by touching or pointing at it: there is no focus, no Tab and
no Enter. A person whose touchpad this kernel does not drive can press
Esc off the first screen and then go nowhere, which is the state the Dell
XPS 9360 in §16.97 was in before the i2c controller went into the kernel
— and the next machine with an undriven pad is in it again. What it
needs is a focus a key can move and a key that activates what is focused,
across the sixteen screens, which is a design-system question (DESIGN §4)
and not a shell one. Raised 2026-09-16 (§16.97). §16.99 is its first
slice: the PIN pad, which is the first screen a locked device shows,
takes the keyboard.~~ **Decided: DESIGN §4.15** (§16.102).

---

## 16. Decisions log

One entry per decision: what, why, and what would reopen it. Newest last.

### 16.1 Curve library: libsecp256k1 via the `secp256k1` crate (2026-09-07)
- `bitcoin` and `miniscript` require the `secp256k1` crate; there is no k256 backend. Choosing k256 for signing would either put two curve implementations in the binary or mean rewriting PSBT, descriptor and taproot handling ourselves — the largest and most error-prone code in the product.
- libsecp256k1 is Bitcoin Core's library: continuously reviewed, constant-time tested under valgrind, fuzzed, and it carries the MuSig2 module (8.4 #19–20) and the nonce-function hooks anti-exfil needs (8.4 #9).
- k256 is a credible alternative: pure Rust, complete projective formulas, an NCC Group review in 2023 whose findings were fixed, BIP-340 support. What it lacks for us is MuSig2 and adoption by any Bitcoin PSBT/descriptor library.
- Cost accepted: a C compiler in the reproducible-build container (`secp256k1-sys` compiles the C source in its build script) and a small shim for the wasm target.
- Mitigation: `osk-crypto` exposes a narrow signing and derivation interface. k256 can be added behind it later as a cross-check (sign with both, compare bytes — the 8.4 #8 check) or as a replacement.
- Reopen if rust-bitcoin gains a pure-Rust backend, or reproducible C builds prove troublesome.

### 16.2 BIP-39: in-house, word indices not strings (2026-09-07)
- Why indices: a committed word is an integer 0–2047. A `u16` gives fixed-size, stack-allocated, zeroizable state with no heap copies and language-independent equality. A `String` allocates, is copied on reallocation where zeroize cannot reach, and needs per-language comparison rules. The `bip39` crate stores indices internally too, so this was never a reason to avoid it.
- Why in-house anyway: the algorithm is ~300 lines (entropy → SHA-256 checksum → 11-bit indices; PBKDF2-HMAC-SHA512 seed), and the explorer tools (8.3 #1–2, the final-word calculator, checksum-bit display) need the intermediate values a library hides. The crate's remaining contribution is NFKD normalization through `unicode-normalization`, which brings large Unicode tables. We avoid needing it: wordlists are embedded already NFKD-normalized at build time, and passphrases are restricted to printable ASCII, which is NFKD-invariant. Coldcard, Trezor and most hardware signers impose the same restriction. A non-ASCII passphrase created elsewhere cannot be entered; documented limitation.
- The `bip39` crate is a dev-dependency: every wordlist and every vector is cross-checked against it in tests.
- Reopen if users need non-ASCII passphrases.

### 16.3 Framebuffer: RGBA8888 in core, shells convert (2026-09-07)
- RGB565 packs a pixel into 16 bits (5 red, 6 green, 5 blue); RGB888/RGBA8888 uses 24/32 bits. 565 halves memory and bus bandwidth and shows slight banding in gradients, which flat UI never has.
- tiny-skia renders premultiplied RGBA8888 natively. The reference panel takes 18-bit RGB666 over DPI, and the Pi firmware exposes it as a 16-bit RGB565 framebuffer by default, so the Pi shell packs 8888 → 565 on blit, one pass per frame. Phones and desktop blit 32-bit directly. A 480×640 RGBA8888 buffer is 1.2 MB, trivial on every target.

### 16.4 Screens and input: touch required, 240×320 minimum, three size classes (2026-09-07)
- UX.md §2 supersedes the earlier text here: classes are `small` / `mobile` / `wide`; panels smaller than 240×320 and d-pad-only devices are not targets; physical buttons are accelerators. This removes the T9 keyboard and the `tiny` class.
- The `wide` reference moved to 960×640 at 160 dpi, which is also the desktop window's own fixed size (§16.27).

### 16.5 Wireframes replaced by the renderer (2026-09-07)
- See §13.1. Screens are reviewed as PNGs from the snapshot shell, and interactively in the desktop shell with the same size and DPI flags.

### 16.7 Testing: Rust unit and integration tests only, no golden images (2026-09-07)
- Golden-image tests are premature: they freeze pixels before the UI has settled, and every font, spacing or wording change invalidates them. Until there is a significant UI, all tests are ordinary `cargo test` unit and integration tests over crate APIs: vectors for `osk-bip`/`osk-psbt`/`osk-codec`, layout-engine tests on solved geometry, state-machine tests on `Event` → `Command` sequences.
- The snapshot shell exists for humans to look at screens, not for tests. Reopen when the UI is stable enough that pixel regressions are worth catching.

### 16.6 Repository conventions (2026-09-07)
- Planning documents live in `docs/` and are committed; they are living documents, and git history is the decision log.
- No GitHub Actions and no hosted CI of any kind. Actions are disabled at the repository level. Builds, tests and reproducibility checks run locally.
- The desktop shell targets Linux, macOS and Windows. Contributors build natively on their own machine; the snapshot shell is the review tool for anyone without target hardware.

### 16.8 Dependency vendoring deferred (2026-09-07)
- `Cargo.lock` is committed from the first commit, so every build resolves the same versions. `cargo vendor` (§5.4) happens before the first release build, once the dependency set has settled; vendoring now would mean re-vendoring on every crate added during Phases 1–2.
- Reopen if a dependency disappears from crates.io or if a build must run without network before the first release.

### 16.9 Glyph atlases: 8-bit uncompressed, `fontdue` baker, synthesized check marks (2026-09-07)
- §4.3 planned 4-bit alpha with RLE. Milestone 3a baked 8-bit uncompressed coverage; the pass-A redesign moved to 4-bit (§16.24), which the format's flag bit 0 already described. RLE over the nibbles is still open and would help the icon faces most. Reopen when the BIP-39 wordlist union and CJK faces are added and binary size matters.
- The baker uses `fontdue` (parse + rasterise in one call, two transitive crates) rather than `ab_glyph`; either is fine for a build tool, `fontdue` is less code to own, and it reads the CFF outlines of the two icon OTFs as well as the TrueType outlines of the Noto faces. It runs on the build box only.
- Noto Sans lacks the arrows, triangles, check and cross the UI needs. Arrows and triangles are borrowed from Noto Sans Mono at bake time; `✓` and `✗` are synthesized from polygons in the baker. Icons do not come from a text face at all (§16.24).
- Kerning is not baked. UI strings are short and the monospace face has none.

### 16.10 Layout engine: no shrinking of unweighted children (2026-09-07)
- The flexbox-lite solver grows weighted children into leftover space and shrinks weighted children (down to their minimum) on overflow. Unweighted children keep their intrinsic size. A screen whose content does not fit therefore scrolls (`Scroll` node, used by the wizard body, document frame and every gallery page) or clips; it never reflows in a way the author did not ask for. This keeps the "size classes only re-flow, never re-order" rule (§13) enforceable: every variant is explicit.
- Keyboard keys are 44 dp tall on `mobile`/`wide` and 40 dp on `small`, shrinking to 36 dp when the offered height forces it (UX.md §2; the 44 dp touch target rule bends for keyboards on the 2.8" panel, as it does on every phone keyboard). PIN pads keep 44 dp on `small`.

### 16.11 Chunked strings: shrink, then page (2026-09-07)
- §13 says whole string on screen, font reduced as needed, explicit pager when it truly cannot fit. Implemented as: try monospace sizes from 16 dp down to 10 dp in 2 dp steps; the first size at which all groups of four fit in the offered width and height wins. Inside a scroll region the height is unbounded, so a chunked string there always fits at some size. Only a height-constrained placement (a fixed card on `small`) pages, with "chars a–b of n · page i of j" and tap to advance.

### 16.12 BIP-39 keyboard: accent folding, and valid final words as a preference (2026-09-07)
- The on-screen BIP-39 keyboard is letters only (§4.5), but the Spanish, French, Italian, Czech and Portuguese lists carry accents. Because the lists are embedded in NFKD form, every accent is a separate combining mark (U+0300–U+036F); dropping those marks turns each word into lower-case ASCII (`osk_bip::bip39::fold`), and a test asserts this holds for all five lists. Prefix matching and the enabled-key set use the folded form; the candidate strip shows the published form, composed back to the Latin-1 glyphs the atlases have (Czech carons are dropped until the wordlist union is baked, §16.9). No Unicode tables on the device, as §16.2 requires. Japanese, Korean and Chinese wait for their keyboards.
- For the final word, candidates are filtered to the words whose checksum bits fit the words typed so far, but only while at least one matches the prefix: with a strict filter a wrong earlier word would leave the user stuck at word 12 with their word greyed out and no diagnosis. When no valid final word matches, every matching word is offered, the checksum step fails, and the wrong-word heuristic runs.
- Reopen if a user needs the strict "only valid last words" behaviour (SeedSigner's) as an option.

### 16.13 Checksum failure: which word is wrong (2026-09-07)
- Trying every single-word substitution flags every position: a random replacement passes a 4-bit checksum one time in sixteen and there are 2048 candidates, so 12-word mnemonics always admit a fix at every position. The heuristic therefore only tries *similar* substitutes: words sharing the first three letters (a neighbouring candidate on the strip) or one edit away (a misread backup). With about three such neighbours per word, a wrong position is flagged with high probability and a right one about one time in five, so the screen says "probably" and lists every flagged position with a "Fix word N" row that re-types just that word and keeps the rest.
- Reopen when real usage shows a different error distribution, or if a stronger signal (e.g. the user's own uncertainty markers) is added.


### 16.14 Schnorr nonces: BIP-340 with all-zero aux-rand (2026-09-07)
- ECDSA uses libsecp256k1's RFC 6979 nonce function (8.4 #7). For Schnorr, BIP-340 takes 32 bytes of auxiliary randomness; `osk-psbt` passes all zeros (`sign_schnorr_no_aux_rand`). BIP-340 states the scheme stays secure with fixed aux-rand: the nonce is still `H(t ‖ P ‖ m)` with `t = d ⊕ H(aux)`, keyed by the secret key and the message, so it is deterministic and unique per message. What fixed aux-rand gives up is side-channel and fault hardening, which is what the anti-exfil protocol (8.4 #9) will address later with host-supplied randomness the host can verify.
- The gain is the nonce-validation property of 8.4 #8: the same PSBT and key always produce the same bytes, so a user can compare against a second implementation and a hidden nonce channel shows up as a mismatch. `sign` also signs every input twice and refuses if the bytes differ ("deterministic mode" check); the cross-implementation half is exercised in tests against Bitcoin Core's BIP-174 signatures and RFC 6979 vectors from python-ecdsa and Core's `key_tests`.
- Reopen when anti-exfil lands (per-signature choice between deterministic and anti-exfil modes).

### 16.15 PSBT warning thresholds and refusal rules (2026-09-07)
- Thresholds are constants in `osk-psbt` (`core/osk-psbt/README.md` has the table): high fee is a caution at 5 % of the amount sent to others and a danger at 20 % or above 0.01 BTC absolute; fee rate ≥ 1000 sat/vB is a danger; dust follows `bitcoin`'s `minimal_non_dust`; `SIGHASH_NONE` and non-standard sighash values are dangers, `SINGLE`/`ANYONECANPAY` cautions. The share is measured against what leaves the wallet, so a fee larger than the payment is flagged even when change is large.
- Refusals: `sign` returns an error, never a silent skip, for any danger warning without `force`, for a key origin that does not re-derive to the key the PSBT names, for a key not present in the spent script, for signature verification or determinism failures, and for inputs it cannot sign (taproot script path, unknown scripts, BIP-174 signer-check failures such as a `non_witness_utxo` that is not the spent transaction). Change is verified only by re-deriving from the account xpub and comparing the whole scriptPubKey; an origin that names our fingerprint but does not reproduce the script is a `ChangeSpoof` danger.
- Ownership of multisig inputs is recognised by fingerprint at inspection time and verified against the derived key at signing time; descriptor registration for multisig accounts (8.4 #16–17), taproot script paths (8.4 #6 script-path) and anything beyond `multi`/`sortedmulti` need `miniscript` and are deferred with it. `bitcoin` has no PSBT finalizer, so single-sig and plain-multisig finalization is in-house, checked against the BIP-174 finalizer and extractor vectors.
- `bitcoin`'s optional `base64` feature pulls `base64` with `std`; the crate ships a 60-line codec instead.

### 16.16 File channel: request and reply, no picker in the core (2026-09-07)
- The SD card / file channel of §7 is `Command::RequestFile { kind }` answered by exactly one `Event::File { kind, bytes }`, `Event::FileCancelled { kind }` or `Event::FileUnavailable { kind }`, and `Command::WriteFile { kind, name_hint, bytes }` for output. The core asks; the shell never pushes a file on its own. Pushing would let the shell decide when a PSBT appears in the flow, which is the shell interpreting content (§4.2 forbids it) and a way for a compromised shell to steer the user; a request/reply keeps every file read tied to a tap the user made and keeps §5.3's "any file write not explicitly triggered by the user" enforceable at the contract level.
- Bytes cross raw. The core parses (binary or base64 PSBT, later descriptors and wordlists), so the shell stays free of format knowledge and one code path handles a file from SD, a desktop path, or a share sheet.
- No picker yet: the desktop shell takes `--psbt` and `--out` on the command line and answers everything else `FileUnavailable`; the snapshot shell queues bytes from a `file` script line. A file dialog means a GUI toolkit dependency (`rfd` or per-platform code) that the reference shells do not need for review and testing, and the Pi shell will list a mounted SD card itself. The contract is the same either way; a picker is a shell feature, not a core one.
- Reopen when the Android and iOS shells land (share sheet and document picker are the natural sources), or if a shell needs to stream a file too large to hold in memory.

### 16.17 Entropy conventions: dice are hashed as ASCII, coins are packed (2026-09-07)
- **Dice (8.1 #1).** The entropy is SHA-256 of the rolls written as the ASCII digits `1`–`6`, truncated to 16 or 32 bytes. Coldcard and SeedSigner hash the same string, so a user can reproduce a key elsewhere or check it by hand: `echo -n 3246115135… | sha256sum`. Fifty rolls are required for 128 bits and 99 for 256 (log2 6 ≈ 2.585 bits per roll; 99 rolls carry 255.9 bits, the count the other tools use). Extra rolls up to 200 are accepted and all go into the hash.
- **Coins (8.1 #2).** One bit per flip, heads = 1, packed MSB-first (the first flip is the top bit of the first byte), no hashing. A fair coin already yields uniform bits, packing is what every other tool does, and it keeps the hex checkable by hand. 128 or 256 flips.
- **Hex (8.1 #6).** 32 or 64 digits, taken as they are.
- Reopen if a source needs mixing (8.1 #5), which will hash regardless.

### 16.18 Entropy sanity thresholds are cautions, not blocks (2026-09-07)
- Dice: `long_run` at 8 identical rolls in a row; `sequential` at 12 rolls counting up or down through the faces (wrapping); `skewed` when the chi-square statistic over the six faces (5 d.o.f.) exceeds 20.5, the 0.999 quantile (20.515), checked only from 50 rolls. Coins: `long_run` at 12; `skewed` when the heads count is more than 3.5 σ (σ = √n / 2) from n / 2, computed without a square root as `(2h − n)² · 4 > 49 n`. Constants live in `osk-entropy`.
- Every flag is shown as a caution card with the numbers behind it and a "Roll again" / "Continue anyway" choice; only "too few" stops the user. A fair die produces a flagged sequence about one time in a thousand, and a user who really rolled it must not be blocked; a user who tapped `1 2 3 4 5 6` to get through is told why that is guessable. The stats table (count per face, longest run, chi-square) is always shown, flagged or not (UX.md B1).
- Reopen when the card and camera sources land, which need their own checks.

### 16.19 Loaded keys keep their mnemonic in temporary storage mode (2026-09-07)
- `LoadedKey` holds the [`Mnemonic`] (word indices, language) next to the master key, in the default temporary (RAM-only) storage mode, the only mode so far. The backup word screens (D1, D2), the quiz for any loaded key (D5, D6, K1) and the coming SeedQR export (8.2 #5) all need the words; without them a user would have to re-type a key to verify its backup, which defeats D5.
- This widens the plaintext-in-RAM window relative to §5.1's plan: `Sealed<T>` is not implemented yet, so the words sit in plaintext for the session, as the master key already does. The marginal exposure is the words themselves, which an attacker with the master key already effectively has; the memory-wipe and stateless rules (§5, §6) apply unchanged, and the words are inline `u16`s in a zeroize-on-drop struct, never heap text.
- `Sealed<T>` will wrap both the master key and the mnemonic in a later milestone; a storage mode that seals the key on disk will keep `mnemonic` at `None` and hide the word screens.
- Reopen when `Sealed<T>` lands, or if a platform's memory reality (§5.5) argues for dropping the words after the quiz. *Update:* §16.21 seals the seed and the words; the master key stays live while unlocked, for the reason given there.

### 16.20 QR in and out: in-house encoder, `rqrr` behind a `std` feature, `ur` for BC-UR (2026-09-07)
- **Encoder in-house.** None of `qrcodegen`, `qrcode` or `fast_qr` builds without `std`. `osk-codec::qr` is a port of Project Nayuki's `qrcodegen` (MIT) restructured for `no_std` and for secrets: the module grid is a fixed 3917-byte bitset that zeroizes on drop, the bit stream and codeword buffers are `Zeroizing` vectors, and the error-correction level is never boosted, so a SeedQR comes out at level L as the SeedQR specification states. The caller picks the mode: SeedQR digits go in numeric mode as the specification requires (48 digits → 25×25, 96 → 29×29; CompactSeedQR 16 bytes → 21×21, 32 → 25×25, all checked against the nine worked examples in SeedSigner's `docs/seed_qr/README.md`); PSBT base64 and descriptors go in byte mode because the alphanumeric alphabet has no lowercase; UR bytewords are upper-cased into alphanumeric mode.
- **Decoder: `rqrr`, `std`, behind a feature.** `rqrr` 0.11 needs `std::io::Write` and `std::error::Error`; so do `quircs` (a C binding) and `rxing`. `osk-codec` enables `rqrr` behind its default `decode` feature and is, in that configuration, the one core crate that is not `no_std`. The Pi, desktop and mobile shells all run with `std`, so nothing ships differently; the bare-metal target, if it ever comes, would move decoding behind the contract or need a port. Deviation from §4.1, to revisit.
- **BC-UR: in-house (§16.61).** A coordinator such as Sparrow emits an endless stream in which only the first *n* parts are the plain fragments; every later part is a random XOR mix, so a receiver that joins late finishes only with a real fountain decoder. That decoder, the bytewords alphabet and the part CBOR are `osk_codec::ur`, `no_std` + `alloc` and with no dependency of their own; the `ur` crate they replaced cost nine crates and is gone (§16.61, `docs/deps/ur.md`). `crypto-account`, `crypto-hdkey` and BIP-388 wallet policies are later: the wallet export is the descriptor or the account key as a plain text QR, which Sparrow, Nunchuk and BlueWallet read.
- **Secret QRs.** A SeedQR is the seed. The digits live in a fixed buffer inside `Secret`, the matrix zeroizes on drop, the widget tree that carries it (through an `Rc<QrMatrix>`) is dropped after every frame, and the secret frame keeps the code blank, with a "hold to show" placeholder, until the hold button is under a finger, on every size class rather than only on `small` (UX.md §5 names `small`; the code is the whole seed everywhere). A scanned frame that may picture a SeedQR is a `Zeroizing` vector on the core side; the decoder's own scratch space is `rqrr`'s and is not wiped, an accepted gap while `rqrr` is a dependency.
- **Static or animated.** The Sign result shows one static byte-mode code when the payload fits version 40 at level L (`STATIC_QR_MAX_BYTES` = 2953 bytes: the final transaction as hex, or the signed PSBT as base64 text, since text is what every phone scanner and coordinator reads), otherwise an animated `ur:crypto-psbt` with 200-byte fragments (`UR_FRAGMENT_LEN`, a version-10 code) advancing every 250 ms (`UR_FRAME_MS`) on ticks, with "part *i* of *n*". Both are always offered through two chips; the static chip is dead when the payload is too long. The 2953-byte cap is generous: a version-40 code on the 2.8" panel is 2 px per module and hard to scan, so a user with a big PSBT will prefer the UR chip. Reopen with real scanning experience across coordinators.
- **Routing.** One scanner (UX.md §4) reached from Home, from Load ("Scan a SeedQR"), from Sign ("Scan QR") and from Verify ("Scan address"); the expectation only changes the hint. Classification is by content (`osk-codec::classify`): SeedQR digits, `psbt\xff` or its base64 prefix `cHNidP8`, `ur:`, an address of any network, a known descriptor function with parentheses, base58check `xpub`/`tpub`/SLIP-132, whitespace-separated words all in one wordlist (precomposed accents folded, no Unicode tables), then 16/32 raw bytes as a CompactSeedQR, else unknown with a chunked-hex "treat as PSBT / text" chooser. A shell with no camera answers `CameraUnavailable` once and the scanner offers "Load from file instead", which routes the file's content the same way, so every route is exercised by the snapshot shell and the integration tests. The scanner is never left on the navigation stack: a route goes Home first, then to the target screen.

### 16.21 Session security: `Sealed<T>`, the session PIN, and the entropy protocol (2026-09-07)
- **What is sealed.** A loaded key is its 64-byte BIP-39 seed and its words (`MnemonicBytes`: 24 `u16` indices, count, language tag), both `Sealed` under the session key with ChaCha20-Poly1305 (`docs/deps/chacha20poly1305.md`). The passphrase is never kept: the seed already includes it. The `MasterKey` is *not* sealed: rust-bitcoin's `Xpriv` is `Copy` and derivation copies it freely, so sealing it would mean unsealing on every address and every signature for no gain in exposure; instead it exists only while the session is unlocked and is rebuilt from the sealed seed on unlock. Locking drops every master key and the derived xpub and address caches. `Sealed::with` decrypts into a stack buffer that is zeroized when the closure returns, and the `SealedBytes` trait fixes the buffer size at compile time so nothing is allocated for plaintext.
- **Nonces without an RNG.** The 96-bit nonce is a 32-bit seal counter followed by a 64-bit prefix derived once per key as `HMAC-SHA512(key, "osk-sealed-nonce")[..8]`. The counter never repeats within a key (sealing refuses at 2³² − 1), a rotated key starts a new counter under a new prefix, and everything sealed under the old key is re-sealed under the new one before the old one is zeroized, so no `(key, nonce)` pair repeats. Uniqueness is all the AEAD requires; the prefix only keeps nonces from being predictable constants.
- **Entropy protocol.** The core asks with `Command::RequestEntropy` and the shell answers with one `Event::Entropy { bytes: [u8; 32] }`. It asks at start, on every lock and on every wipe; the answer rotates the session key. Until the first answer the key is *weak*, mixed from the display parameters and the clock, marked as such, and refuses to seal: the app keeps the seed and words in plaintext `Secret`s (as it did before this milestone) and seals them when entropy arrives. The weak mix is not a substitute for the shell's entropy and is never treated as one; Settings states when a session is weak. The desktop shell answers from the OS RNG (`getrandom`, shell-only, `docs/deps/getrandom.md`); the snapshot shell answers from a fixed seed so that renders are reproducible. A shell with no RNG leaves the request unanswered and the app works, unsealed. The answer also redraws the frame: the PIN pad scramble is derived from the key, and the drawn pad and the hit targets must agree.
- **Session PIN.** The first key of a session appends a "Set a session PIN" step to the Load and Create wizards: 4–8 digits, typed twice, on the on-screen pad only (§4.5). It is kept in RAM as HMAC-SHA512 of the digits under a 32-byte salt derived from the session key at the time it is set (the salt is snapshotted, so a later key rotation does not invalidate the hash); the digits are zeroized once the hash exists. The lock screen shows the loaded fingerprints, the entry dots, the attempts left and the pad, nothing else, with no navigation; five wrong PINs wipe the session (keys, caches, PIN, session key rotated) and return to the empty Home. Without keys there is no PIN and nothing to lock. Tier B secure-element backing of the PIN (§8.5 #1) is later; on Tier D a lock is a wipe, since a browser keeps nothing past it.
- **Timers.** Auto-lock after 120 s of no input (30 s / 2 / 5 / 15 min), auto-wipe after 10 min (5 / 10 / 30 / 60 min / never, with a caution on never), wipe never shorter than lock (choosing one moves the other). Any touch, key, button or scroll restarts both; ticks drive them, so a shell must keep ticking while idle. The last minute before a lock is shown as "lock in N s" on Home and over the wizards and the Sign flow. A lock cancels any wizard, PSBT or scanner in progress, since those hold plaintext. Wipe-and-exit (hold, then confirm) and every other `Command::Exit` wipe first; dropping the `OpenSigner` zeroizes the session key, the PIN hash, every sealed value and every live master key. What cannot be guaranteed is inside rust-bitcoin: copies of `Xpriv` that derivation leaves on the stack, and libsecp256k1's own scratch space.
- **PIN pad scramble.** On by default on Tiers C and D (§4.5: click-position logging), off on A and B, a setting either way. The seed is derived from the session key, so the pad is the same for one session key and different after every rotation, which means the unlock pad never matches the set-PIN pad's layout.
- Reopen when Tier B lands (SE-wrapped session key and PIN), when `mlock` is added, or if the weak-key window (start to first entropy) proves long on some shell, which would argue for blocking the finish hold until entropy arrives instead of holding plaintext.

### 16.22 Explore secrets, and the `wide` navigation template (2026-09-07)
- **What Explore treats as secret.** The words, and with them every form of them the workspace shows: the 0-based index, the 11-bit group, the entropy hex, the checksum bits and the first byte of SHA-256(entropy) (its top bits are the checksum, which is part of the last word's index); the 64-byte seed; and every extended private key, at the master and at each level of the path. Each is masked as bullets until a finger is on the secret panel that holds it, and re-masks on release (§4.6, §16.27). The typed passphrase masks after 500 ms like the Load wizard's. Public: extended public keys, fingerprints, addresses, the path text and the SLIP-132 forms, which are cached per (key, network, path) because the tree is rebuilt every frame.
- **Typed words.** With no key loaded (or by choice), Explore reuses the Load wizard's count, language, word and checksum steps as a sub-flow and takes the indices back without adding a key; a bad checksum gets the wizard's own diagnosis. The words and passphrase sit inline in `explore.rs` (a secret-lint file) with the master key built from them, and the whole struct is zeroized when another screen is entered. The seed is never kept: the seed row runs PBKDF2 while it is held. Holding the typed master key is the one cache of secret material, accepted because it is the working key of the typed words and rebuilding it is a PBKDF2 run per frame.
- **`wide` template.** UX.md §2 and §4 take the form: a 240 dp sidebar (Home, then the hub tiles in order, with the current area on a surface panel) beside the area's document as a column capped at 800 dp and centred; Home's pane is the status line and the same tile grid the other classes show (§16.27). Wizards, secret frames, the Sign wizard, the scanner and the lock screen take the whole window as a centred column capped at 640 × 800 dp, so no button or keyboard spans a desktop window and the footer stays near the content; a keyboard inside a document is capped at 720 dp. Explore's key screen puts the path and the keys in one column and the encodings and the rows out in another, and an open explainer becomes a side panel beside them. Reopen when a tablet shell lands and the sidebar needs to collapse to icons.

### 16.23 Portrait first, and one chrome for every screen (2026-09-07)
- The first UI was reviewed and rejected. Small panels are **portrait**: the 2.8" reference is 480×640 at 286 dpi and the minimum is 240×320 at 143 dpi (§4.3, UX.md §2). Landscape still solves through the same layout engine; nothing is designed for it. The desktop shell defaults to `--size 480x640 --dpi 286`, and `wide` moved from 110 to 160 dpi, where its text is readable.
- Step indicators are gone. Every non-Home screen has the same chrome: a 56 dp **app bar** (back chevron in a 48 dp target, centred title, optional trailing action), a body, and — where the screen has something to confirm — one bottom-anchored 52 dp accent button. A screen that only asks the user to choose has none: the tap that chooses also moves on (§16.27). Where progress matters it is in the title ("Word 7 of 12", "Quiz 3/12").
- **Home does not scroll.** A status line, then a grid of equal tiles (2 × 3 on `small` and `mobile`, 3 × 2 in the `wide` content pane, with the sidebar beside it; §16.27). A screen that must scroll draws a **scrollbar** — a thin thumb at the right edge, proportional to the visible fraction — and fixed-size runs use a **pager** instead.
- **Chunking rule:** groups of four are for strings longer than sixteen characters. A fingerprint is eight hex characters, reads as one word, and is never split (§13). This fixed fingerprints appearing as `73c5 da0a` in chips, key rows and the key strip.
- **Metrics** follow the SeedSigner touch UI, which was the quality bar: an 8 dp grid, 16 dp screen padding, 48 dp targets, 12 dp radius, 56 dp app bar and list rows, 72 dp menu rows. Type ramp in dp: display 26, title 20, body 16, label 14, caption 12, mono 16. Theme: black ground, `#1C1C1E` surface, `#FF9F0A` accent.

### 16.24 Icons come from an icon face, not from the text font (2026-09-07)
- The first UI drew "icons" as characters pulled from Noto Sans (`▸`, `₿`, `§`, `…`). At icon sizes they are the wrong shape, the wrong weight and the wrong size. Replaced by a closed `Icon` enum in `osk-ui`, each variant mapping to one code point in a baked **icon face**.
- Sources, both vendored in `tools/fonts/`: `seedsigner-icons.otf` from the MIT-licensed SeedSigner project (`U+E900`–`U+E923`), and `Font_Awesome_6_Free-Solid-900.otf` for the icons SeedSigner's set lacks (lock, camera, dice, coins, file, eye, trash, compass, book, magnifier, shield, wallet, list). Font Awesome Free: the font file is under OFL 1.1, the icon designs under CC BY 4.0; both licences and the attribution are in `tools/fonts/README.md`.
- `fontdue` reads the CFF outlines of both OTFs correctly, so the baker did not have to move to `ab_glyph`.
- Icon sizes are baked to cover 20, 24, 28, 32, 40 and 48 dp at 143, 160, 286 and 420 dpi, and snap at runtime like text. A crate test asserts every `Icon` resolves at every baked size, so the baker's list and the enum cannot drift.
- Noto Sans **SemiBold** replaces Bold for titles, row labels and buttons; the Bold face was dropped rather than baked alongside it. The mono face gained sizes up to 84 px so a fingerprint can be drawn large.
- Atlas coverage is now **4 bits per sample** (`flags` bit 0), which the format already reserved. That halves every atlas and pays for the icon face: `core/osk-ui/assets` is 3.3 MB for four faces where it was 1.4 MB for three. Run-length encoding over the nibbles is the next size cut, and would help the icon faces most.

### 16.25 All user-facing wording in one file (2026-09-07)
- Every string a user can read lives in `opensigner/opensigner-core/src/strings/en.rs`: a `pub struct Strings` of `&'static str`, one `pub static EN`, and `OpenSigner::strings()`. A second language is one more static of the same shape.
- Views hold no user-facing literal. `tools/lint-strings.sh` (wired into `just lint`) greps `views/` for a double quote followed by a letter and fails, ignoring comments, panic and assertion messages, and the two notation prefixes `m/` and `BIP-`. Format templates carry `{}` placeholders and are filled by `strings::fill`, so a translation can reorder them.
- The extraction was also a rewrite: labels of one to three words, sentences of at most twelve words, no metaphors, no "just", no reassurance, no hint under a field that explains itself. Warnings state the fact and the consequence in two short sentences. Tests assert that every field is non-empty, free of double and trailing spaces, and made of short sentences.

### 16.26 Every screen is one screenful: pages, not scrolling (2026-09-07)
- Pass B rebuilt the screens pass A had not reached: Sign, the scanner, Verify, Explore, Create's entry and words, Backup, the wallet export and the status screens. The rule that decided most of them: **a screen shows one screenful.** A run of fixed-size things (outputs, words, path levels, export formats) is *paged*, with the page in the title ("Output 1 of 2", "Words 1 of 2", "Level 3 of 6") and one call to action that walks the run and then leaves it. Only a genuine list (the randomness sources, many inputs) scrolls, and it draws a scrollbar.
- **Sign** is now the reference's shape: an **overview** of centred label/value pairs (send, to, fee, change, inputs) over "Review details"; one **output** per screen with the address chunked and centred and the pager around the button; the **inputs** on one screen as short rows; the **warnings** on their own screen, which exists only when there are warnings and whose Continue is dead until a danger is acknowledged; a **confirm** screen of one mark, one sentence, the signing keys and the hold; and a **result** of one mark over three rows (code, file, signature bytes). The transaction QR is full screen with the part in the title and one app-bar action switching between the single code and the animated parts.
- **Explore** became a menu of five sections, each its own screen, instead of one long column of collapsible sections. The words table pages eight rows at a time; the path explorer separates the editor (field, presets, keyboard) from the results (one level per page: path, fingerprint, extended keys, and the addresses at the leaf). Pass 4 replaced the menu and the level pager with two screens (§16.30).
- **Secrets** all reveal the same way. Pass B put a hold button at the bottom of the screen for that; §16.27 replaced it with the secret panel, which is the surface the secret is drawn on. The words screens page by what the panel holds — six on `small`, twelve where the screen is taller (§16.29) — so that a page always fits one panel.
- **Metrics**: menu rows are 52 dp on `small` (72 dp elsewhere) with a 4 dp gap, so five rows and the app bar fit the 2.8" panel; the key detail menu carries its state as row trailing text ("not verified", "no") rather than badges, which is what made it fit. `ButtonStyle::Text` was added for a secondary action that must not compete (Skip on the quiz). Buttons and row labels drop a text size before they clip, and the hero title wraps.
- **Chunked strings** gained a centred variant and an explicit size floor (8 dp), because an extended key at 12 dp takes six lines and a screen has room for four. `OpenSigner::overflow()` reports anything the last frame placed outside its clip; the layout tests assert it is empty on every screen that is meant to fit.

### 16.27 One desktop window, one hub, one secret element, one hold (2026-09-07)
The second review of the redesign. What changed, and why:
- **The desktop window has one size.** A desktop build does not support arbitrary monitors, so it opens at 960×640 at 160 dpi — the `wide` class, landscape, small enough for any laptop — and `--size`/`--dpi` are review flags for the other reference screens. The `wide` reference size moved from 1440×900 to the same 960×640, so what the shell shows and what `just snapshots` renders are the same screen. The reference sizes are now 240×320 @ 143, 480×640 @ 286, 1080×2340 @ 420 and 960×640 @ 160.
- **Home is the status line and the hub grid.** The key strip and the two empty-state buttons are gone: Home was carrying three ways to reach the same two jobs. Keys is a tile with the loaded-key count in its corner, and Load and Create live behind it. (The §16.31 rebuild put key rows back above the grid on `mobile`; the owner asked again on 2026-09-12 and they are gone again: on a phone the only sign of a loaded key on Home is the Keys badge.) Settings left the grid for the status line, and Learn took its place, dimmed. The grid is the same shape on every class — equal tiles, one 8 dp gap, the 16 dp screen padding at the edges — which a layout test asserts at all four sizes.
- **The status line earns its height.** 56 dp: the tier badge, which opens a screen saying what tiers A–D mean and which this device is; the network badge off mainnet; the lock, with a countdown inside the last minute; the settings gear. The version moved to Settings › About. The lock icon is on the line whenever there is something to lock, not only during the countdown: the touch that would reach it is also the touch that restarts the timer, so a control that appears only in the last minute can never be used.
- **The back chevron is the cancel.** Every Cancel button on a screen that has a back chevron is gone, the scanner's included. A test walks the screens and fails on a screen that offers both.
- **One secret element.** The hold button beside a masked value is replaced by the **secret panel**: a tinted surface with an accent border which, masked, shows the masked content and one caption, "Touch and hold to show", and which reveals what it holds while a finger is anywhere on it. It carries the words and their wordlist numbers, the seed and every extended private key, the two seed codes, the math screen, the dice and coin entries and Explore's tables. `HitTarget::Reveal` is the new target: a press with no duration and no completion, so nothing irreversible can hang off a reveal. Whatever a panel holds fits it on one page, because the panel is the only touch target inside it.
- **No "current" or "later" labels.** An unavailable choice is a dimmed, untappable row; the current choice carries a check mark. A screen whose whole content is one choice advances on the tap that chooses — source, word count, wordlist, how a transaction arrives — and has no call to action. The rule is now: tap to choose, a call to action to confirm entered data, hold for the irreversible step.
- **The keyboard reaches the edges.** Hit rectangles tile the whole keyboard region out to the screen edges — the bottom row to the bottom of the screen, the outer columns to the left and right — while the visible keys stand 16 dp clear of the bottom. A crate test asserts the rectangles are pairwise adjacent and their union is the region. The field above a keyboard is one 52 dp row, and the hints under fields are gone.
- **Word entry shows one word.** No chips of the words already committed, no indices; the title is the progress, and a backspace on an empty field steps back to the previous word, which returns to the field for editing (§4.6).
- **A hold is only the last step of something irreversible.** Sign, forget a key, wipe every key, wipe and exit — each a tap to a screen that states the consequence, then the hold. Adding a key is a plain tap, because a key can be forgotten. A test checks every hold button's id against that allowlist across the screens.
- **Long values are never crowded.** Explore's encodings page split into the account key and its SLIP-132 form, one per page; the path explorer's leaf addresses moved to a page after the last level. Pass 4 put both encodings back on one screen, which is the comparison they exist for, and sent the leaf addresses to the Addresses screen (§16.30).
- The format switch has a glyph of its own (Font Awesome's `right-left`, `U+F362`) instead of the restart arrow, and Create's seven randomness sources are a two-column tile grid, which fits the 2.8" panel where seven rows did not.

### 16.28 The FFI and the Android shell as built (2026-09-07)
The first shell that is not written in Rust, so the boundary had to be
decided rather than described. What was built, and why:

- **Handles, not pointers.** `create` boxes an `OpenSigner` and returns a
  64-bit handle: a slot index in a table, plus a generation counter in the
  high half. Freeing bumps the generation, so a handle reused after free
  resolves to nothing rather than to freed memory, and every entry point
  answers an unknown handle by doing nothing. A shell can be wrong about
  lifetime without being unsound about it. The table is thread-local, not
  global behind a mutex: `OpenSigner` holds an `Rc` and is not `Send`, and
  a shell drives it from one thread. The alternative was an
  `unsafe impl Send`, which would have asserted something untrue.
- **Command codes plus payload accessors.** `pollCommand` returns a small
  integer and parks that command's payload on the instance;
  `commandKind`, `commandBytes`, `commandName` and `commandMs` read it
  back. So no struct crosses the boundary, no allocation protocol has to
  be agreed on, and every signature is integers plus at most one array or
  string — which is what keeps the JNI layer short enough to read in one
  sitting. `commandBytes` takes the bytes rather than copying them, so a
  file the core handed over exists in one place afterwards.
- **The frame is a direct `ByteBuffer`.** `NewDirectByteBuffer` over the
  core's framebuffer, so Rust copies nothing and Kotlin's
  `Bitmap.copyPixelsFromBuffer` is the only copy between `tiny-skia` and
  the screen. The core's premultiplied RGBA8888 is byte-for-byte what an
  `ARGB_8888` bitmap wants. The buffer is valid until the next call on
  that handle; the core in fact allocates its canvas once and never moves
  it, but the rule is stated tightly so a later core can reallocate.
- **`catch_unwind` at every entry point.** Unwinding into a Java frame is
  undefined behaviour. Release builds abort on panic (§5.4) and so never
  unwind at all; the guard is what makes debug builds and host tests
  behave the same way.
- **`unsafe` is confined to one file.** `opensigner-ffi` is the only crate
  that does not inherit the workspace's `unsafe_code = "forbid"` — a
  `forbid` cannot be lifted by the crate that needs it. All of it lives in
  `src/jni.rs`, under `deny(unsafe_op_in_unsafe_fn)`, doing four
  conversions and nothing else: read a `byte[]`, read a `String`, build a
  `byte[]` or `String`, wrap the framebuffer. No pointer arithmetic, no
  transmute, no `Send`/`Sync` assertion. The rest of the crate — handle
  table, mappings, command encoding — is safe Rust with host tests.
- **The Kotlin shell has no dependencies** beyond the Kotlin standard
  library. No AndroidX, no Material, no Play services, no analytics:
  framework APIs only, so the deprecated `startActivityForResult` is used
  in place of pulling in `androidx.activity`. Every library in a signing
  shell is a library inside the trust boundary (§2.4), and the shell is
  small enough not to need one — one `Activity`, one `View`, one `object`
  of `external fun`s.
- **Tier C, not B.** Stock Android without StrongBox wrapping is Tier C
  (§3). Tier B needs the secure-element commands, which the core does not
  have yet.
- **What the shell does with each command**: `Draw` invalidates the view;
  `RequestFile` opens `ACTION_OPEN_DOCUMENT` and answers with the bytes,
  with `FileCancelled` when the person closed it, or with
  `FileUnavailable` when there is no picker or the read failed (§16.48); `WriteFile` opens
  `ACTION_CREATE_DOCUMENT` with the core's name hint; `RequestEntropy` is
  answered at once from `SecureRandom`, and the shell's copy is zeroized;
  `CameraOn` is answered `CameraUnavailable` until the Camera2 capture arrives (§16.33);
  `Vibrate` is a no-op, because the manifest declares no permissions at
  all (§11.2); `Exit` is `finishAndRemoveTask()`. The system back gesture
  is `Key::Escape`, the only key the shell ever sends — text goes through
  the core's own keyboard (§4.5).
- **`FLAG_SECURE` costs the screenshots.** It is set before
  `setContentView` and is never conditional, so `adb exec-out screencap`
  returns black — which is the flag working. Review screenshots come from
  the emulator's own console instead, which reads the host framebuffer
  below the layer that enforces the rule
  (`tools/android-screenshot.py`). No debug switch was added to turn the
  flag off; it would be the one line of this shell most likely to be
  copied into a release.
- **The activity is never recreated.** Portrait-locked, every
  configuration change declared, `allowBackup="false"`, nothing saved in
  `onSaveInstanceState`. A loaded key lives in the core's memory and in
  nothing else, so a recreated activity would silently be a wipe.

### 16.29 The use of space per size class, and the idioms behind it (2026-09-07)
The third UX pass answered one question — what each size class does with
the room it has — and the answer was mostly policy, not engine. What was
decided:

- **Composites branch on dp, not only on the class.** `geom::Metrics`
  carries `width_dp`, `height_dp` and the class, and `OpenSigner::metrics`
  reports the frame the current screen is drawn in, which on `wide` is
  the content pane. The class thresholds are unchanged; what changed is
  that a screen can now ask how much room it actually has. Without this a
  411 × 891 dp phone and a 411 × 600 dp phone laid out identically.
- **Spare height falls below the body.** `organisms::Body` names the
  three placements a screen can want: `Top` (the default for a step),
  `Center` (verdict screens only) and `Fill` (a code, a viewfinder, a
  grid). Before this every `action_screen` centred its body in the whole
  height, which put a 450 dp block in the middle of a 891 dp phone with
  black above and below it and nothing in reach of a thumb.
- **The `wide` sidebar is on every screen, dimmed where it must not be
  used.** It was on 77 screens and gone on 84, and the content origin
  moved 80 px between two consecutive screens of one flow. Hiding it was
  protecting two different things — the secret and the destructive step —
  with one blunt rule; dimming it protects both (no hit targets) and
  costs nothing. Home's `wide` pane now shows the key list, because the
  sidebar already lists the six areas and no screen should list them
  twice. One consequence: an area is one id wherever its control is
  drawn, so `SIDEBAR_BASE` is gone and the tile ids serve both.
- **The type ramp scales with the class; lengths do not.** 1.0, 1.1, 1.3
  through `tokens::type_scale_pct`, applied by `Scale`, so no screen asks
  for a size of its own. A phone showing 12 dp captions as its dominant
  text was under-using an 891 dp screen, and 16 dp in a desktop window at
  twice the viewing distance was a third too small. Chips and candidate
  cells gained a floor of 44 dp below `wide` and 32 dp on it: the
  candidate strip is the Load flow's primary control and was a 36 dp
  target on every size, including the ones with a finger on them.
- **One rendering per idiom.** A choice is a list of rows with a check
  mark, unless every option is at most six characters, in which case it
  is a segmented row that wraps; the word-count button grid and the
  Create source tile grid are gone. A dimmed row now says why it is
  dimmed. A label/value overview is `organisms::record`, two columns
  above `small` and one line per pair on `small`. A run of pages is
  `organisms::run`, which drops the pager and shows the whole run when it
  fits the width, as the export code and its string do in a `wide` pane.
- **What the tests hold.** No screen overflows at any of the four
  reference sizes; the sidebar's rectangle is identical on every `wide`
  screen; Home's grid reaches the bottom padding on a phone and no tile
  is more than a quarter taller than it is wide; chips and candidates
  meet their class's floor; a record's values share one column.

Reopened by: a size class whose room is not covered by these rules — a
tablet in landscape, or a panel between 358 and 600 dp tall, where the
grid's aspect cap and the `Top` placement would both want re-measuring.

### 16.30 What a screen is called, and which key it acts on (2026-09-07)
The fourth UX pass answered the owner's question about Explore and the
navigation complaints behind it. What was decided:

- **Explore is two screens and the path editor.** The five-section menu,
  the per-section Next/Done pager and the "Level N of 7" pager are gone.
  **Keys** carries the applied path as a breadcrumb whose crumbs select a
  level, the purpose presets, the keys at the level that is boxed, both
  encodings of the account together, and the rows out; **Words and bits**
  carries the words table, the checksum arithmetic with the verdict first,
  and the seed. The area was hard to navigate because it had three
  navigation systems stacked on a body that was mostly one value and a
  pager; two screens need none of them.
- **A screen says which key it is acting on.** UX.md §4 had specified a
  "Using" chip since the first draft and nothing drew it, so the typed-words
  and loaded-key screens of Explore were pixel-identical. The chip is now on
  both Explore screens, on the Sign entry and every Sign review step (the
  keys the transaction is about), and on Verify › address. It is the compact
  chip on `small`, where a 44 dp band above a review screen is not free, and
  it is absent from the Sign result, whose three ways out need the height.
- **A mode is a labelled control in the body.** Receive/Change, the export
  format and the two shapes of the transaction code are segmented rows;
  "Show the math" is a row. Six unlabelled app-bar glyphs meant seven
  different things, two of them collided with the session lock, and one
  silently changed meaning between two Explore screens. What is left in an
  app bar is the back chevron, the eye of a transcription screen, and an ⓘ
  that means only "explain this screen".
- **One trailing mark, one meaning.** A navigable row carries a chevron with
  its value in front of it; a check mark marks the current item of a choice
  list, and a choice list has no chevrons. The navigation rail marks the
  current area with the accent instead.
- **A code decides its own language step.** A decoded SeedQR has a valid
  checksum by construction, and plain-text words name their wordlist, so the
  scan path lands on the confirm screen with a "Word language — English ›"
  row on it. Typing still asks, as two rows: English, checked, and "Another
  language ›". The language is part of the key, so it stays one tap from the
  fingerprint; it is no longer a wall in front of it.
- **Three screens became documents.** The Explore key screen, the wallet
  export and the address screen each gained a labelled control that a
  268 x 286 dp panel has no room for. Rather than squeeze the code or the
  keys, they scroll on `small` with a scrollbar and keep their content at
  the size it is worth reading; the Sign inputs step and the Create sanity
  check do the same when a caution card is on them. On a phone and in a
  window they still fit.
- **What the tests hold.** Every Explore screen and every Sign review step
  draws the "Using" chip; the language step is skipped after a seed-code
  scan and the confirm carries the language row; a mismatched PIN repeat
  shows the inline error and restarts the entry; every dimmed row anywhere
  in the app carries a reason of one to three words; and every screen at
  each of the four reference sizes either fits or is a document that
  scrolls, with nothing stranded outside a scroll region.

Reopened by: a `small` panel taller than 358 dp, where the three documents
would fit again and the rule "this screen scrolls on `small`" would be
measuring the wrong thing.

### 16.31 The design system (2026-09-08)
Six fix passes from the first UX review and one batch of composed screens
had improved screens one at a time without making the product uniform:
the same content was drawn several ways, and every brief that listed
items produced an assembled screen. The owner asked for "a complete
design overhaul, starting from first principles", and the answer is
`docs/DESIGN.md`. What was decided:

- **Decide once per kind of content, then build every screen from the
  decisions.** DESIGN §4 is the inventory: one rendering and one
  behaviour per kind (rows with values, long strings, amounts, badges,
  codes, secrets, records, actions). §5 names seven screen intents and
  the sixteen reusable screens every flow is built from; a screen spec
  after this is three lines: which screen, which content, what is left
  off.
- **Every dimension is a token.** `core/osk-ui/src/tokens.rs` holds every
  dp, ratio and timing the document names, per class as `const fn`, and
  `tools/lint-tokens.sh` fails the build on a layout literal in the
  components, screens, organisms or gallery. A review is settled by
  editing one file.
- **One placement law.** Actions and the controls the thumb works are
  bottom-anchored; lists and documents start at the top and scroll;
  everything else is one block centred in the space between (§2.6). It
  replaced a zone rule that produced voids and split layouts in gallery
  pass 4.
- **The owner's rules, verbatim where possible.** One choice per row on
  touch screens, checked then confirmed with Continue; rows with values
  are label above, value below; long strings are a reference row that
  opens a Compare screen, whole only there and on the Address screen;
  amounts in one unit by setting, "sats" lowercase; no prose on a
  working screen; secrets are a place (a panel of fixed geometry, or a
  Secret screen) with the eye's ring as the only countdown; the hold
  confirms sign, forget and wipe, and the word "irreversible" appears
  nowhere; a 24 dp safe-area inset under everything on `mobile`.
- **Built through a gallery first.** Five gallery passes (tokens and
  components, the reusable screens, the owner's review, ergonomics,
  intents) took the document to v0.5 before an application screen was
  touched; the owner reviewed contact sheets of the gallery at each
  pass. `--app gallery` renders 36 pages, one per §4 group and one per
  §5 screen, and the snapshot recipe includes them.
- The composition specs of 2026-09-08 (`docs/COMPOSITION.md`, twelve
  per-screen dp tables) were the step between item-list briefs and the
  design system; the document is deleted, its rules live in DESIGN §2
  and its budgets in the tokens.

Reopened by: a content kind with no rule in §4, or a screen that is not
one of the sixteen. Both are additions to the document first.

### 16.32 The application rebuilt from the sixteen screens (2026-09-08)
Five passes rebuilt every application screen from `osk_ui::screens`:
Home, Keys, Load and the lock screen; Create and Backup; Sign, Verify,
Addresses, Wallet export, Passphrase and Forget; Scan, Inspect, Settings,
About, Tiers, Wipe and the terminal screens; Explore. Each pass was
briefed per screen as "reusable screen · content · left off" against the
document, never as a feature list, and each was reviewed against the
renders before it was committed. What the tests hold: every `ScreenKind`
is built by the screens module; no view uses a composite the design
system replaced (the old organisms are deleted); every string in `en.rs`
is reachable through the app. About two hundred prose strings left the
working screens; the explainers survive as Document screens behind ⓘ.

### 16.33 Cameras: no camera library on any platform (2026-09-09)
The core already owns the camera contract (`CameraOn`, `CameraOff`,
8-bit luma `CameraFrame`s, `CameraUnavailable`) and decodes frames
itself with `rqrr` (§16.20); every shell today answers "unavailable".
The owner's rule for what comes next: a camera library is acceptable
only if it is minimal, vendored and small enough to review in full. The
decision is that no platform needs one; each shell captures through the
operating system's own interface with code we write and can read.

- **Linux desktop and the Pi share one V4L2 module.** Raw `ioctl`s
  through `libc` (already in the dependency graph): query capabilities,
  set a 640 × 480 format, four memory-mapped buffers, stream, `poll`,
  dequeue, hand the luma to the core, requeue. Formats asked for in
  order: `GREY`, then `YUYV` (every second byte), then `NV12`/`YU12`
  (the first width × height bytes). The V4L2 structs and request numbers
  are declared in the module, about two hundred lines; no `v4l` crate.
  The first `/dev/video*` that streams a video format is the camera;
  no device chooser yet.
- **The Pi uses the legacy camera stack.** `start_x=1` and
  `gpu_mem=128` in `config.txt`, the firmware's `start_x.elf`, and the
  kernel's `bcm2835` camera driver built in, which exposes the module as
  a plain V4L2 device the shared module reads. This is the stack the
  SeedSigner image runs on the same panel. It supports the OV5647
  (Camera Module v1 and the Zero camera, which is the owner's and the
  one SeedSigner standardises on) and the IMX219 (v2); the v3 module
  needs libcamera, which is far too large to vendor and review, so it
  is not supported. The image build includes the camera bits from the
  first image so the camera does not need a second image.
- **Android uses the platform Camera2 API**, not CameraX: the shell has
  no library dependency today and stays that way. An `ImageReader` at
  640 × 480 `YUV_420_888` gives the Y plane as the luma the contract
  wants; the CAMERA permission is requested at the first `CameraOn`, and
  a refusal is `cameraUnavailable`. About 250 lines of Kotlin.
- **macOS uses AVFoundation from one small Objective-C file** (about a
  hundred lines: a capture session, the default video device, a
  bi-planar 4:2:0 output, a delegate that hands the Y plane to a C
  callback), compiled by the `cc` crate the workspace already builds
  libsecp256k1 with, behind a three-function C ABI. No Objective-C
  binding crates. The camera permission needs an `.app` bundle with an
  `NSCameraUsageDescription`, ad-hoc signed; a `just mac-app` recipe
  makes it. Built and tested on the owner's Mac only.
- **Windows is deferred** (Media Foundation, the same one-small-file
  pattern) until someone needs it; "Read a file" works there today.
- **Distribution, settled with it (§15 item 12).** No Play Store and no
  App Store. Android and Linux binaries are signed with keys we generate;
  the macOS bundle is ad-hoc signed and unsigned for Gatekeeper, opened
  with the documented two-click bypass or built from source; iOS is a
  build-it-yourself platform. No Apple Developer Program: it verifies
  and publishes a legal identity, and the project is anonymous.
- **Order:** the V4L2 module goes into the Pi image work, since the
  image needs the camera configuration from the start; Android next,
  because it is the smallest change and the owner can test it at once;
  macOS last, with the bundle recipe.

Reopened by: a platform whose camera interface cannot be reached
without a library (a Wayland-only desktop with no V4L2 device, or an
Android version that removes Camera2), or a Pi camera module that only
libcamera drives becoming the one people buy.

### 16.34 The re-review, the fix passes, and why a pass takes an hour (2026-09-09)
A fresh agent reviewed every rebuilt screen at the four renders against
DESIGN.md v0.5 and the 2026-09-07 review. Eight of that review's ten
blockers were closed; three of them (secret-panel geometry, the keyboard
ramp, the permanent `wide` sidebar) measured correct to the pixel at
every size. Ten findings remained, and most were a rule applied
faithfully producing a bad screen, so DESIGN.md went to v0.6 before
anything was fixed. What was decided:

- **The QR side is the token and the part count follows from the pitch
  floor.** The four different sides on one class came from §4.9's own
  "the square takes what is left"; now the square is the class's side
  and the toggle and progress rows give way. The 0.5 mm floor is hard:
  the BC-UR fragment size is computed from the largest QR version that
  keeps the pitch at the class's side (`opensigner-core/src/codes.rs`),
  so the transaction is three parts on the panel and every part scans.
  The wallet export has an animated form for when it needs one; the
  demo descriptor does not.
- **The `wide` column is centred in the pane** with its scrollbar
  against it; the Sign review on `small` keeps its one-screenful table
  and is the one written exception to "classes never hide content".
- **Verify shows the typed address whole above the keyboard**; the
  words-so-far panel appears with the first accepted word and carries
  the eye; the path editor is two labelled groups in the script-type
  vocabulary; Compare, Secret and Address step the mono size up to fill
  their block; dimmed reasons sit under the label on every class, and a
  row dead only because the feature is unbuilt carries no reason at all
  (the owner: "dim already means unavailable"). The PIN pad is not
  shuffled by default.
- **Leftover:** `OutputBadge::Dust` is still drawn on a dust output
  while §4.11 already has a dust warning card, two renderings of one
  fact; no fixture triggers either. Decide with the warning fixtures
  (§15 item 42).

**Why a pass takes an hour.** The two fix passes were measured from
their transcripts (every model turn and tool call timestamped):

| | pass 6a | pass 6b |
|---|---|---|
| wall clock | 66 min | 59 min |
| model turns | 45 min | 41 min |
| tool execution | 21 min | 18 min |
| requests | 250 | 328 |
| tool calls per request | 1.0 | 1.0 |
| context at the end | 530k tokens | 465k tokens |

The passes were sequential (one tool call per round trip, every grep
its own request) and each round trip carried a context that grew from
27k to over 400k tokens; the model turn averaged 4 s under 100k tokens
of context and 10 to 13 s above 250k, with 100 to 400 output tokens
either way. Builds were 13 to 21 min because `just` ran five times.
The Python-script edits the harness's auto mode encourages were a
minor cost (5 to 8 min a pass, most under 3 s each).

The changes: an agent definition (`osk-implementer`, outside the
repository) that runs implementation passes at medium reasoning effort
and carries the working method as standing rules (two to four tool
calls per turn, a context budget with ranges instead of whole files,
narrow test binaries while iterating and one `just` at the end), and
passes of four or five items so the context never reaches 400k. The
transcript analysis runs after every pass so the effect is measured,
not guessed.

Reopened by: a pass that is still over 30 minutes with those rules in
force, which would mean the per-request cost, not the request count, is
the limit.

### 16.35 Cameras on every platform, the Pi card, and the first hardware test (2026-09-09)
One session, all on the medium-effort agent definition (§16.34), with
three passes run in parallel by the owner's one-time exception to the
one-agent rule and no clobbering, because their files were disjoint
and each brief said what it must not touch.

- **Android (Camera2)**, the platform API and nothing else: an
  `ImageReader` at 640 × 480, the Y plane at up to ten frames a second,
  the CAMERA permission asked for at the first scan and the only one in
  the manifest, a refusal or a device error one `cameraUnavailable`.
  Frames are turned upright by `SENSOR_ORIENTATION` as they are copied.
- **macOS (AVFoundation)**, one Objective-C file in a new crate
  `opensigner-avfoundation`, the second and last crate allowed
  `unsafe`, behind a three-function C ABI; `cc` named as a workspace
  dependency (`docs/deps/cc.md`); `just mac-app` builds the ad-hoc
  signed bundle with the camera-usage string. Written blind on the
  Linux box and compiled first time on the owner's Mac.
- **The Pi card**, `just pi-image`: Buildroot 2024.02 in a container
  from the repository's own Dockerfile, the Raspberry Pi 6.1 kernel with
  the DPI framebuffer, the Goodix touch controller, the DMA the panel
  needs and the legacy bcm2835 camera all built in and no loadable
  modules — though "no modules" turned out to mean the defconfig's 1203
  modules were built *into* the kernel, which §16.93 fixed — the
  PI_X firmware, Waveshare's overlays fetched from Waveshare at a pinned
  hash, BusyBox init with no login and no network applet, the whole
  root filesystem an initramfs inside `zImage`, the static shell binary
  as the one program and poweroff when it exits. The 3B+ booted into
  the app; touch, camera and decode worked on the panel.
- **The preview.** The viewfinder shows the camera's latest frame,
  scaled to cover the square, brackets and state on a translucent band
  over it (DESIGN v0.7, §4.9), in colour: a frame may carry its NV12
  chroma beside the luma, every shell that has colour sends it, the
  core decodes from the luma alone and converts only the pixels it
  draws. The preview buffer is shared with the drawn tree and wiped by
  whichever lets go last, since a frame may picture a SeedQR.
- **Camera rotation**, a Settings row (0°, 90°, 180°, 270°) applied in
  the core before the preview and the decode, for cameras mounted
  sideways in cases people build. Session-only until settings persist
  (§15 item 45).
- **Two bugs the panel found.** A flick kept scrolling for seconds after
  the finger lifted: the Pi loop took one touch per 50 ms pass and
  painted the panel for each, while the controller reports a moving
  finger a hundred times a second; now every waiting touch goes in per
  pass, only the newest of a run of moves is kept, and the panel is
  painted once at the end. And the scan screen froze with the camera
  on: the loop drained the frame channel until it was empty, and on the
  3B+ a decode takes longer than the gap between frames, so it never
  got back to the ticks and the input; now one frame per pass, the
  newest, in the Pi and desktop shells alike. A third bug was in the
  build: Buildroot installed the prebuilt shell binary once and never
  again, so a rebuilt image carried the first build's binary; every
  build now reinstalls it and fails if the binary in the image differs
  from the one on the host.
- **The image tree is laid out for variants**: `boards/<board>`,
  `panels/<panel>`, `common/`, `variants/dev/`, assembled at build time;
  `just pi-image board=pi3 panel=waveshare-28dpi` with those defaults,
  `dev=1` for a dev card (console on the panel, a getty on serial, the
  shell verbose, never for a real card). Each board directory carries
  its Rust target and C flags, so an ARMv6 Zero is a directory and a
  toolchain (§15 item 46). The release card keeps no console on the
  panel and SysRq compiled out; the kernel paints the mark as its boot
  logo the moment it owns the display, so a black panel means the
  firmware or the display path, not a working device that is quiet.
- **Tests cover user-facing behaviour** (CLAUDE.md). An audit of every
  test in the repository classified 508 of them; the ones that restated
  a design rule, a token's value or a library's behaviour, three
  quarters of them in `osk-ui`'s own unit tests, went. The dev profile
  now builds our own library crates optimised: a full-screen render is
  19 ms unoptimised and 4 ms optimised, the tests render thousands of
  screens, and the clean build is 15 s slower for it.
- **The decode does not own the loop.** Every frame redraws the
  preview; the decode runs at most four times a second of the core's
  clock (`scan::DECODE_INTERVAL_MS`), reads the preview's box-averaged
  copy first and the full frame only when that finds nothing (1.5 to
  1.9× cheaper on this box, and the dense UR part still resolves from
  the full frame). Measured for the release profile: the rasteriser
  `tiny-skia` at opt-level 3 makes a scroll pass 2.6× faster and the Pi
  binary 128 KB smaller, so it is the one package built for speed; the
  QR decoder at 3 was 1.4× at best and `osk-ui` at 3 changed nothing,
  so both stay at the size setting. The framebuffer write is the part
  no bench here can time; the dev card's `--verbose` shell is where it
  gets measured if the panel is still not smooth.
- **Root is locked outright** on the release card. The first image set
  the root password to `*`, which Buildroot hashes like any password;
  with root login off it writes `*` into the field itself, which no
  password matches.
- **The mark**: a pixel key on a 16 × 24 grid, orange with a lit edge,
  drawn as exact squares so it is the boot logo on the panel and a
  crisp vector launcher icon at any density; one text grid is the
  source of both.

Measured pass cost on the medium-effort definition, against §16.34's
59–66 min: Android 19 min, macOS 15 min, the Pi card 38 min including a
25-minute Buildroot build, the preview 24 min, colour 18 min, the test
audit 19 + 28 min, the decode work 21 min. The context stayed under
200k tokens in every pass but one: the Pi variants pass took 63 min and
840 requests at 266k, because three additions were sent to it while it
ran and it waited through two full image builds. Quality needed a flag
removed, a doc comment moved and one wipe made deterministic, nothing
structural; a brief that grows while its agent runs is the one thing
that measurably cost.

Reopened by: the decode work (rate-limited, faster-built, on the
reduced frame first) not making the Pi's scan screen smooth, which
points at §15 item 47; a Zero owner; a second panel.

### 16.36 The Pi's file channel, and the settings that survive a restart (2026-09-09)
Three passes on the medium-effort agent definition, one at a time.

- **The file channel on the Pi** (§7, §16.16). Init mounts the card's
  second partition, `OSKDATA`, at `/mnt/microsd` with
  `flush,noexec,nosuid,nodev`; a card with no second partition still
  boots into the app. The shell has no UI of its own, so it cannot show
  a list: a `RequestFile` is answered with the newest file on the
  partition that could be what was asked for, a name ending in `.psbt`
  for a PSBT and anything but the partition's own note for the scanner's
  *Read a file*, up to 4 MiB. `WriteFile` saves under the core's name
  hint reduced to what FAT holds and never over a file already there
  (`signed.psbt`, `signed-2.psbt`, …), flushed before the core's screen
  says saved. On the device the directory must be a mount point before
  it is read or written, so nothing is ever written into the initramfs.
  The rule is a stopgap for a list screen (§15 item 49), and a failed
  write still cannot reach the screen (§15 item 50).
- **Settings persist** (§6, §5.3, §3). `Command::StoreSettings { bytes }`
  and `Event::Settings { bytes }`: the core asks whenever the user
  changes one of the six Settings rows, network, unit, auto-lock,
  auto-wipe, the PIN-pad shuffle and the camera rotation, and the shell
  hands the bytes back once, after `Display` and before any input.
  Nothing about a key is ever in them, nothing is stored at start or on
  a tick, and a Tier D device never asks. The format is the core's own
  few lines of text, `opensigner-settings 1` then `key=value`, with no
  dependency, so a person can read the file on the card; reading
  forgives in one direction only, an unknown key ignored, a bad value
  left at its default, a bad header leaving them all. The threat-model
  cost is nil: the file holds no secret, and a tampered one can at most
  set a preference the Settings screen shows. The desktop keeps the file
  in the platform's config directory (`--settings`, `--no-settings`),
  the Pi keeps `opensigner-settings.txt` on the `OSKDATA` partition and
  never offers it as a file, Android keeps it in the app's private
  directory with OS backup already off, and the snapshot shell keeps
  nothing but takes a `settings` script line.
- **Every warning has a file** (§15 item 42, closed). Sixteen PSBTs
  under `tools/vectors/psbt`, one per `WarningKind`, written by an
  `osk-psbt` example from the abandon key on regtest with no randomness,
  and a test that the committed bytes are what the example builds; a
  Sign test for each behaviour a person sees (a caution passes, a danger
  waits for the acknowledgement, information is shown, the danger is
  listed first); `tools/scripts/sign-warnings.txt` in `just snapshots`.
  The first render of the Warnings step found the card cutting its
  label off on the small panels when the value was long. A first fix
  wrapped the value beside the label in the record's value column, three
  lines in half a 268 dp panel; the owner rejected it, and the card is
  now what every other row with a value is, the label above and the
  value below across the card's width (DESIGN §4.11). The core can list
  every text a frame drew in a box too small for it, which is how the
  test says the words are whole.
- **The camera-rotation row only where a camera can be mounted
  sideways.** `DisplayInfo::camera_fixed`: true where the platform turns
  the frames upright (Android, from the sensor orientation) or the
  camera is part of the machine (a webcam, the Mac), false for a module
  in a case someone built (the Pi). The core offers the row and applies
  a rotation only where it is false, so a settings file carried over
  from a Pi turns nothing on a phone. The owner asked for it after the
  Android build showed a setting that could only make the preview wrong.
- **Learn, the first nine pages** (UX L1–L7, L10, L12, L14). The Home
  tile is live and opens a Menu of pages, each a Document: Words;
  Signer, wallet, node; Transactions; Randomness; Backups; Passphrases;
  Verifying; The air gap; Mistakes and scams. Every page is under 220
  words of plain language in `strings/en.rs`, scrolls on the 240×320
  panel with nothing cut, and touches no key, session or setting. The
  PIN-shuffle explainer stays behind ⓘ (DESIGN §7 item 3). The glossary
  (L17), the goal-to-job map (L18), the practice run on signet (L16),
  multisig (L9), xpubs and privacy (L11), the secure element (L13) and
  inheritance (L15) are not written.
- **The shell reports its insets** (§15 item 34, closed).
  `DisplayInfo::inset_bottom` and `inset_top`, in pixels: the strips at
  the frame's edges the person cannot use. The 24 dp the core guessed
  for every `mobile` display is gone; the Android shell pads its view by
  the system bars and the cutout and reports zero, so the bottom action
  and the keyboard faces no longer stand a second 24 dp clear of a bar
  the frame already excludes. The phone renders in `just snapshots`
  report 63 px, 24 dp at 420 dpi, and are unchanged. A test compares a
  phone that reports the strip with one that does not.
- **A save is called saved only when the shell says so** (§15 item 50,
  closed). `Event::FileWritten` and `FileNotWritten` answer every
  `WriteFile` exactly once, as `File` and `FileUnavailable` answer a
  request. The Sign result's file row waits, then names the file or says
  "not saved" with Save still offered; a second tap sends the bytes
  again. On Android a cancelled picker is a save that did not happen.
- **The first numbers from the board** (the dev card's timings, a Pi 3B+
  with the 2.8" panel): an idle pass costs nothing at 20 a second;
  convert and write are 3.7 and 2.0 ms; a scroll move is 13 to 52 ms,
  81 worst, one layout and one full paint each; a camera frame is 235 ms,
  445 worst, because the decode takes longer than its own interval and
  so runs on every frame, the reduced and then the full frame when the
  scene holds no code; opening the camera is 535 ms inside a tap. The
  plan that follows from them: decode and open the camera on a worker
  thread in the shell, the full-frame retry rarer; then reuse the layout
  and shift the pixels in place on a scroll move, painting only the
  strip that came into view, with a 16 ms tick while a finger is down.
  Repainting only a changed rectangle for taps was considered and
  dropped: on the panel a tap changes most of the screen.
- **The decode leaves the loop** (§15 item 47, closed). The core is
  `no_std` and single-threaded by design, so the decode moves to the
  shells: `Event::CameraFrame` is preview only, and `Event::Scanned`
  carries the payload of a code a shell decoded, which the core routes as
  it routed its own finds. One crate, `opensigner-scanner`, holds the
  worker every shell uses: one thread, the newest frame replacing any
  not yet taken so a slow board decodes fewer frames instead of falling
  behind, the reduced copy every time and the full frame only on every
  fourth miss, frames in `Zeroizing`. The policy itself,
  `scan::decode_frame`, stays in the core so the snapshot shell and the
  tests decode the same way without a thread. The Pi opens its camera on
  the capture thread too, so the tap that opens the scanner returns at
  once. On the FFI the instance owns the worker and Kotlin is unchanged.
- **A scroll move shifts the pixels.** On `Action::Scrolled` from a
  drag or a wheel, when nothing else on the screen is changing (no hold,
  reveal, mask, animation or press cancelled by the same move, and not
  on Sign, Scan or an export QR), the core moves the region's items in
  the layout it already has, moves the region's pixels in place, and
  paints only the strip that came into view and the scrollbar's column;
  everything else still goes through a full render. Widgets cut by the
  panel's edge are repainted rather than moved, because the rasteriser's
  anti-aliasing at the edge is not translation-invariant. Tests compare
  the shifted frame with a fresh draw pixel for pixel on the Settings
  list and a Learn page at three sizes and for a wheel on the desktop;
  no review render changed. The Pi loop waits 16 ms instead of 50
  between passes while a finger is down. The second card's timings
  showed a move still at 58 ms, and a bench on this box showed why: the
  fast path cost as much as a full draw. Two causes, both fixed. The
  canvas rebuilt a panel-sized clip mask on every widget's clip push and
  pop inside a scroll region, some thirty a frame; it now keeps the
  masks of the last few clips, which also cut every full render by a
  fifth. And the fast path repainted the scrollbar's whole column,
  which crosses every row, and marked the region's content column, a
  container that always crosses the panel's edge, as needing repaint;
  now only the thumb's old and new places are repainted and only
  widgets count for the edge. A move is 0.2 to 0.5 ms on this box
  against 2.3 for a full draw.
- **The Pi loop wakes on a frame.** With the decode gone the scan screen
  still made nine passes a second, because the loop waited its 50 ms
  tick on the touch channel before it looked for a frame. The loop now
  waits on one channel that carries what woke it, a touch or a note
  that a frame is ready, and a wake ends the wait early; the camera's
  thread drops the note when the channel is full so it never blocks.
  The third card showed the wake changed nothing: the V4L2 module caps
  capture at ten frames a second, and a frame still cost the loop 46 ms
  for the preview alone. The Pi's capture thread now reduces each frame
  once and hands the core the 320×240 copy, so the core reduces nothing
  and moves a quarter of the bytes, while the scanner still gets the
  full frame; the Pi asks the camera for fifteen a second, the desktop
  keeps ten.

- **Files, a list to pick from** (§15 item 49, decided). A shell that
  can list its files answers `RequestFile` with `Event::FileList`, one
  entry per file with size and date, newest first, and the core shows it
  as a Menu over the screen that asked: the date as the caption and the
  name as the bold line, a tap sending `Command::ReadFile` for that name
  and the bytes landing where they always did. Back is no file chosen.
  The Pi lists the card, the desktop a directory (`--files`, Downloads by
  default), each refusing a name it did not list; Android keeps the
  system picker and answers with the file, as does the snapshot shell's
  `file` line. The date is formatted in the core without a dependency,
  in UTC; a board with no clock shows the size instead.
- **Decisions of 2026-09-09** (§15): persistent-seed mode on Android is
  offered, opt-in and off by default, layered behind hardware-backed
  wrapping and the app's PIN (item 32); a wallet config, if ever kept,
  is kept only under the same protection and never on Tier A (item 36);
  multisig registration uses BIP-388 wallet policies (item 8); releases
  are published on GitHub releases with a GPG-signed manifest of SHA-256
  sums and a verification page, the key in the owner's isolated vault
  and the signing through the build box's wrapper, never in a session
  (item 12).

- **A release is artifacts, a signed manifest, and a page** (§15 item
  12). `just release VERSION` refuses a dirty tree and a version the
  crates do not carry, builds the card, the APK and the Linux binary,
  and writes `manifest.txt`, one SHA-256 per file in name order under a
  header naming the commit; a macOS bundle built on a Mac joins the
  directory by hand and the manifest covers it. `just release-sign`
  runs `opensigner-release-sign`, a command on the build machine and not
  in this repository, which signs the manifest with the release key it
  alone can reach, and verifies the signature against
  `tools/release/pubkey.asc`. `docs/VERIFY.md` tells a user how to
  check a download: import the key, compare its fingerprint, verify the
  manifest, check the file. No key material is in the repository, and
  the Android artifact stays a debug build until §15 item 33 is done.

- **A key kept on the device, bound to its secure element** (§6, §15
  items 32 and 9). The contract gains `DisplayInfo::secure`
  (none, TEE, StrongBox), `SecretKept` at start, and four exchanges:
  `SecureMac` (an HMAC-SHA256 the element computes under a key that
  never leaves it and needs the person's authentication for every use),
  `StoreSecret`, `LoadSecret` and `ForgetSecret`, each answered exactly
  once. The core's `keep` module holds the format: a 262-byte blob, a
  header of version, Argon2 parameters, two salts and an attempt count,
  then two records of equal length under XChaCha20-Poly1305 with the
  header as associated data, the key record holding the words and the
  backup flag, never a passphrase, and the duress record holding a
  marker under the duress PIN or random bytes under a random key when
  none is set. The key that opens a record is
  HMAC-SHA256(Argon2id(pin, salt), mac), so no copy of the blob can be
  attacked without the chip. The app counts the wrong PINs and asks the
  shell to forget the blob after eight, or at once under the duress PIN,
  which then shows an empty app; the chip counts nothing about the app's
  PIN and deletes nothing of its own. The session PIN is the storage PIN. Argon2id costs 64 MiB and
  three passes, eighty milliseconds on this box. The attempt count is
  the app's, unauthenticated by design so a wrong PIN can bump it
  without re-encrypting; on a rooted device it could be reset, and the
  guess rate is then bounded by coerced authentications rather than by
  eight. The `argon2` crate joins (`docs/deps/argon2.md`), with the older
  RustCrypto trait generation it pulls, eight crates. The desktop, Pi
  and snapshot shells report no secure hardware; the Android shell is
  pass B.

- **The Android shell keeps the key** (pass B). Two Keystore keys made
  StrongBox-first with a TEE fallback: an HMAC-SHA256 key that needs a
  biometric or the device credential for every use, is invalidated by a
  biometric enrolment and usable only while the device is unlocked, and
  an AES-GCM key that wraps the blob once more into the app's private
  files. The platform `BiometricPrompt` carries the MAC as its crypto
  object, so the element only answers after the person did; no AndroidX
  dependency, because the feature is gated on Android 11, where the
  platform prompt offers credential fallback itself. A symmetric key
  attests nothing, so the hardware level and the verified-boot state are
  read from a throwaway P-256 key's attestation, deleted at once, with
  `KeyInfo` as the fallback; a device that reports software, or has no
  secure lock screen, stays Tier C and offers no way to keep a key. The
  emulator reports software and an unverified boot, so the round trip
  (keep, relaunch, unlock by PIN and credential, forget, keys gone) was
  proven there under a temporary patch and the StrongBox branch is
  untested until a real phone runs it; the verified-boot flag has no
  place on a screen yet.

- **A multisig wallet, registered for the session as a BIP-388 policy**
  (§15 item 8). `osk-bip::policy` parses a policy, a template with
  `@i/**` placeholders and a key per line, or a plain `multi` and
  `sortedmulti` descriptor, enforces the BIP's rules, and turns either
  into the other with the checksum; the BIP's own example table and its
  invalid list are the tests. A scanned or read policy opens the
  descriptor review as a Wallet: quorum, checksum, script type, each
  cosigner with MINE on a loaded key, and Use this wallet, which holds
  it for the session, lists it on Keys under the keys, and makes the
  Sign flow verify that wallet's change and count a loaded cosigner as
  yours, through `Context::wallets`; a change claim naming a wallet key
  that does not reproduce its script is a spoof. Verify answers for a
  wallet's addresses too. Nothing is remembered across restarts (item
  36 open). A 2-of-3 fixture from the abandon key and two fixed
  cosigners, with its PSBT, and `tools/scripts/wallet.txt` in
  `just snapshots`. The export gains no policy form: a multisig policy
  needs the cosigners' keys, which the device never holds.

- **The string tidiness checks live in the strings lint.** The test
  audit (§16.35) removed a unit test that read every string in `en.rs`
  for emptiness, double or edge spaces, "just" and "don't worry"; the
  owner wanted the checks back, and `tools/lint-strings.sh` now makes
  them on every `just`, where a rule about wording belongs.

- **Sign a message, and check a signed one** (§8.4 item 13; UX F7, G4).
  `osk-psbt::message` signs and verifies BIP-137 (the compact signature
  with the header ranges Electrum and Trezor use for nested and native
  segwit) and BIP-322 "simple" for p2wpkh and p2tr, built in-house on
  rust-bitcoin's transaction and sighash types with the `secp-recovery`
  feature; BIP-322's published vectors are reproduced or verified, and
  Trezor's BIP-137 vectors reproduced byte for byte (Electrum's verify
  but grind a low R). A message arrives by QR or file, is read whole
  with the script type, address and format as rows above it, signed
  after a hold, and leaves as a text QR, a compare string and a file in
  the three-line form: address, signature, then the message. Verify's
  "Signed message" reads that form back and says valid or invalid.
  Legacy signs BIP-137, taproot BIP-322, native segwit either, nested
  BIP-137 only since "simple" has no scriptSig.

- **The verified-boot state on About.** `DisplayInfo::boot`, unknown,
  verified or unverified, from the Android attestation's root of trust
  and unknown everywhere else. About states it, and a Tier B device with
  an unverified boot carries a caution card on the Keep hold, since an
  unlocked bootloader weakens what the chip promises; the person may go
  on. A hold can now carry warning cards under its table.

- **Learn, five more pages** (UX L9, L11, L13, L15, L17): Multisig;
  Xpubs and privacy; The secure element; Inheritance; and a Glossary
  of the terms the screens use, one line each in alphabetical order.
  Fourteen pages now. The practice run on signet (L16) and the
  goal-to-job map (L18) remain unwritten.

- **The three explainers DESIGN §7 named, and a page on nonces.** ⓘ on
  Settings (the pad shuffle), the passphrase offer and the key's
  Passphrase screen (the consequence), and the Backup menu (the quiz
  rule), each a short Document; the Choice screen gained an info slot
  for it. Learn's fifteenth page, Nonces, says how the device picks a
  nonce, that it signs each input twice, how to compare bytes against a
  second signer, and that signers which retry the nonce for a shorter
  signature, Bitcoin Core among them, differ without a leak; the
  verification page carries the same steps. Whether the device should
  grind for a low R as Core does went to the owner, and is settled in
  §16.38.

- **Every release artifact from a pinned container, built twice and
  compared** (§11.1, §11.2, §15 item 33 in part). `tools/build/linux`
  and `tools/build/android` are Dockerfiles at a base image digest with
  the workspace's Rust by version, the musl target, and for Android a
  pinned JDK, command-line tools, platform, build-tools, NDK and
  `cargo-ndk`; `just linux-bin` and `just android-release-apk` build in
  them with `SOURCE_DATE_EPOCH` from the commit and paths remapped, and
  `just release` uses them. `just reproduce` builds each twice from
  clean and diffs: the Linux binary, the unsigned release APK and both
  `.so` files came out bit-identical on this box, once AGP's
  version-control and dependency metadata were left out of the APK.
  `pi=1` adds the image, untested for want of hours. The release APK is
  unsigned until the owner's key exists; the debug APK stays the one to
  try.

### 16.37 No ⓘ, no prose on a working screen (2026-09-09)

The owner's rule was always "no prose on a working screen". Over §16.36
I kept it by the letter and broke it in effect: an ⓘ in the app bar of
nine screens, each opening a Document of paragraphs. The owner called
it out and the rule is now stated the way they meant it, in CLAUDE.md
and DESIGN §2.1, §4.1, §4.12: no sentence, no caption that explains,
and no control that opens one. Explanations live in Learn.

Removed: `Overlay::Document`, `Explainer` and `open_explainer` from the
core; `Reading::info`, `Hold::info`, `choice_with_info` and the app bar's
info slot from `osk-ui` (the trailing slot holds the eye and nothing
else); nine `*_INFO` ids; fifty-five strings; nine snapshot frames. The
Sign screens' "Yours", "Replaceable" and "Locktime" moved to Learn ›
Transactions; how the words are made to Words; the PBKDF2 seed and the
BIP-32 split to Randomness; what the quiz asks and what skipping leaves
to Backups; that a kept key stores the words and never the passphrase
to Passphrases; SLIP-132 prefixes to Xpubs and privacy; `m`, `h` and
the level order to the glossary. A sixteenth page, Signing a message,
says what a signature proves, what it does not, and the three-line
form. The PIN-shuffle text was dropped: the toggle's label stands
alone, and the one page about PINs, The secure element, has no room
under 220 words.

The tests that opened an ⓘ went with it. A test that asserted "no
on-screen text ends in a full stop" was written and then removed: it
states the design rule, which CLAUDE.md forbids a test to do.

### 16.38 The ECDSA nonce: low R or first (2026-09-09)

The owner's rule: the point of a deterministic nonce is that a person
can sign the same thing in a second implementation and compare bytes.
Two ways of picking the RFC 6979 nonce are both standard and both in
wide use, so the device offers both and the person picks the one their
other software uses.

**Low R** is Bitcoin Core's grinding. The first attempt is plain RFC
6979 with no additional data; while the signature's `r` is 2^255 or
more (compact first byte `>= 0x80`, DER `r` of 33 bytes), the counter
goes up by one and is written little-endian into the first four bytes
of 32 bytes of RFC 6979 "additional data". Bitcoin Core (`CKey::Sign`),
Sparrow and Electrum all do this for transactions; Sparrow and Electrum
also do it for BIP-137 message signatures. Sparrow's scheme was read
from drongo's source (`ECKey.signEcdsa` with `HMacDSANonceKCalculator`:
a 32-byte little-endian counter after the private key and the hash,
null on the first try) and is identical to Core's and to rust-secp's
`sign_grind_with_check`, which backs `Secp256k1::sign_ecdsa_low_r`.

**First** is plain RFC 6979, first attempt only: what the RFC's own
vectors, BIP-174's published role vectors, Trezor and Bitcoin Core's
message signing (`SignCompact`) produce. That BIP-174's vectors were
made without grinding is visible in them: the combiner vector's
signer-2 output carries a signature whose `r` is 33 DER bytes.

Low R is the default, because it is what the wallets people pair this
device with produce, and because the shorter signature is the one that
goes on the chain. The setting is the seventh persisted one
(`nonce=low_r` / `nonce=first`); a settings file without the key reads
as Low R.

rust-secp has no low-R recoverable signer, so BIP-137's compact
signature grinds in `osk-psbt::message::sign_recoverable_low_r`, the
same loop written out over `sign_ecdsa_recoverable_with_noncedata`.
Schnorr is untouched: BIP-340 has no `r` to grind.

The tests say it in bytes. Every Electrum BIP-137 vector now reproduces
byte for byte under Low R — the `continue` that skipped them is gone —
and Trezor's still reproduce under First. The BIP-174 role chain
reproduces the combiner, finalizer and extractor vectors under First;
under Low R the three signatures whose vector `r` is already low come
out unchanged, the fourth is ground to a different, shorter signature
that still verifies, and the PSBT still finalizes and extracts.

Learn › Nonces now names both choices, what each matches, and that a
message from Core is compared against First; `docs/VERIFY.md` carries
the same table. The page lost a few words elsewhere to stay under 220.

### 16.39 Glyph outlines through tiny-skia (2026-09-10)

Text was drawn from bitmaps baked at sixteen pixel sizes per face, and a
size in dp snapped to the nearest baked size. The CJK wordlists would
have added 3,650 glyphs, 8 MB of atlases at nine sizes. `osk-ui` already
depends on `tiny-skia`, which fills anti-aliased paths for every other
shape on a screen; glyphs were the one thing going around it.

The owner's decision: store each glyph once as an outline, fill it with
tiny-skia at whatever pixel size a display asks for, and cache the
result. Existing faces first; CJK later. No font parser on the device
stays true — the outline format is ours (`tools/fontbake/README.md`) and
its input is embedded.

`tools/fontbake` keeps parsing the vendored fonts on the build box, now
with `ttf-parser` (already in the lock as `fontdue`'s dependency, so the
lock gained no crate and lost five) and emits one
`core/osk-ui/assets/<face>.outl` per face. The glyph sets, the
per-code-point source rules and the `✓`/`✗` fallback are unchanged; the
two synthesized symbols became line contours from the same polygons.

On the device, `fonts::Outlines` parses a face file, `SizedFace` is a
family at a whole pixel size with that size's ascent, descent, line gap
and per-glyph advance, and a `GlyphCache` owned by the `Canvas` holds the
filled coverage bitmaps, keyed by (family, pixel size, code point) and
bounded by a byte budget it clears whole. Measurement never rasterises.
tiny-skia is built `default-features = false` with `no-std-float`, so the
fill is scalar and a glyph is the same bytes on every architecture.

The numbers, measured on the build box:

| | Before | After |
|---|---|---|
| `core/osk-ui/assets/` | 4.1 MB | 140 KB |
| `target/release/opensigner-desktop` | 10,190,720 B | 6,190,656 B |

The first draw of a screen now fills the glyphs it uses. On the 480×640
panel, Home takes 4.3 ms cold and 2.6 ms with the cache warm; Learn ›
Words takes 3.4 ms and 2.7 ms. The panel's own refresh is the slower
half of a frame, and nothing on a screen changes size between the two
draws.

Sizes are now exact: a 16 dp body on a 286 dpi panel is 29 px because
that is what 16 dp is, not because 29 was on a list. Every layout test
passed unchanged, so no screen's fit moved.

### 16.40 CJK glyphs and readings (2026-09-10)

The owner's decision: the Japanese, Korean and two Chinese lists become
first-class, with a kana keyboard, a jamo keyboard, pinyin for Simplified
and 注音 for Traditional. This is the first of three passes and changes
nothing a user sees; it puts in place everything those keyboards draw and
everything they look up. The four languages stay dead rows on the
language Choice, so §15 item 37 is still open — pass 3 settles it.

**A fifth face.** `core/osk-ui/assets/cjk.outl`, `Family::Cjk`, is never
chosen by a `Font`: it is the fallback every text face reads when its own
file lacks a code point, so a glyph lookup is face → `cjk.outl` → `?`. A
borrowed glyph carries the units per em of the file it came from, and the
requesting `SizedFace` scales it by its own pixel size over that, so its
advance and bounding box are right at that size. The requesting face's
ascent, descent and line height are untouched: a CJK glyph may reach a
little past the line box, which is accepted. The cache key is still
(requesting family, pixel size, code point).

Its glyph set is computed rather than listed, so it cannot drift from the
lists: every character of every word of the ten lists in both the
published and the composed form, every keycap of the three non-Latin
keyboards, and the ideographic space, less what the text faces already
carry. That is 3,723 code points. The baker therefore depends on
`osk-bip` by path, the only tool in `tools/` that does.

The four Noto Sans CJK region subsets are 23 MB and are not committed:
`tools/fontbake` downloads each into the git-ignored `tools/fonts/cjk/`
when it is missing and checks it against a digest pinned in `main.rs` and
in `tools/fonts/README.md`. Only the `.outl` files are committed, so
`cargo build` still needs neither the fonts nor the network.

**Composed forms.** The published Japanese list is base kana plus the
combining marks U+3099 and U+309A, and the published Korean list is
conjoining jamo. That is right for derivation and for typing and wrong
for reading. Each wordlist module gained `DISPLAY`, the NFC form, beside
`WORDS` and `NFKD`; `Language::word_display` reads it. Four lists have a
table of their own — Spanish and French are published decomposed too —
and in the other six `DISPLAY` is a reference to `WORDS`. `WORDS` and
`NFKD` are unchanged, so no seed and no wordlist digest moved.

**Readings.** `core/osk-bip/src/wordlists/readings.rs` holds every
Mandarin reading of each of the 2,821 characters in the union of the two
Chinese lists, from the pinned Unihan `kMandarin` and `kHanyuPinyin`
(narrowed to the modern dictionaries in §16.44): 389 syllables in both
spellings, and 1,033 of the characters had more than one reading. `gen.py` converts pinyin to 注音 with a table of
initials, medials and finals and refuses to emit a syllable that does not
round-trip back to the pinyin it came from. `Language::readings`,
`candidates_pinyin` and `candidates_zhuyin` are what pass 3 will drive
the candidate strip with; each Chinese word is one character, so a
candidate is a word index.

The numbers, measured on the build box:

| | Before | After |
|---|---|---|
| `core/osk-ui/assets/` | 140 KB | 1.5 MB (`cjk.outl` is 1,396,057 B) |
| `target/release/opensigner-desktop` | 6,190,656 B | 7,587,232 B |

The binary grew by 1,396,576 B, which is the new face and almost nothing
else. On the 480×640 panel at 16 dp, a line of twelve Korean words takes
0.58 ms to draw cold and 0.03 ms with the cache warm, and a line of
twelve Traditional characters 0.57 ms and 0.04 ms; twelve English words
are 0.25 ms and 0.03 ms. A hanzi outline has more contours than a letter,
so filling one costs more, and it is still a fraction of the panel's own
refresh.

### 16.41 The kana and jamo keyboards (2026-09-10)

Pass 2 of three. Japanese and Korean become rows a person can choose:
both lists are typed, read and quizzed end to end. Chinese waits for
pass 3, and §15 item 37 stays open until then.

**One typed form.** A word's typed form used to be its ASCII fold, and
the prefix a byte array. Both are now `bip39::Typed`, a fixed array of at
most sixteen `char` keys, and `Language::typed(idx)` says what those keys
are: the fold for the Latin lists, unchanged; the NFKD word itself for
Japanese, which is base kana, the combining voicing marks and the small
kana; and the two-set key sequence of the conjoining jamo for Korean.
`candidates_typed` replaces `candidates_folded`, and `Folded` is gone —
`osk-codec`'s precomposed word matching uses `Typed` too.

Because the Korean sequence is what the fingers do rather than what the
syllables are, prefix matching needs no disambiguation: 가나 types
ㄱㅏㄴㅏ and 각 types ㄱㅏㄱ, so after ㄱㅏㄱ both 가격 and 각자 are still
candidates and both a vowel and a consonant are live.

**One mask.** The enabled set was a 26-bit mask over letters. It is now
`KeyMask = u64`, and `keyboard::key_bit(kind, c)` numbers a key within
its own kind: `c - 'a'` for BIP-39, the key's place in the layout for
kana and jamo. Bit 63 is still `DONE_DISABLED`. `LoadWizard::enabled_keys`
maps each candidate's next character through that: a small kana lights
its full-size key, because 小 is what makes it small, and the two
combining marks light ゛ and ゜.

**The kana grid.** Ten columns of the consonant groups, five rows of the
vowels. The brief put ゛, ゜, 小, backspace and ✓ in the five cells the
五十音 leaves empty — や at い and え, わ at い, う and え — and that is
where they are; ん takes the わ column's last cell, where the archaic を
would sit, because the Japanese list uses ん in 1,229 of its words and
uses を, ゐ and ゑ in none. That makes the grid fifty keys, not the
fifty-five the brief counted: no arrangement of ten columns by five rows
has fifty-five cells, and adding a column to reach it would have cost
every key a fifth of its width on the panel that has the least.

Five rows do not fit a 240 × 320 dp panel at the 40 dp key a three-row
keyboard has, so the grid takes `KEY_HEIGHT_GRID` (28 dp) there and may
squeeze to `KEY_MIN_HEIGHT_GRID` (24 dp). Above 240 dp of width nothing
changes: the phone and the window draw it at the class's own key.

**The words panel per language.** The panel padded every word to eight
characters and assumed the monospace advance. A kana or a Hangul syllable
is about an em wide, so the panel now takes a `WordWidth` — the
characters of the list's longest word (`Language::max_display_chars`,
generated with the lists) and one character of its script — measures the
widest row it can hold in the panel's own face, and falls from two
columns to one where two do not fit. A page follows the columns, so the
panel keeps the height its class gives it: Japanese on a phone is one
column of twelve rows and two pages, Korean and every Latin list are two
columns of twelve and one page, and the 240 dp panels were one column
already.

The entry screen's panel of the words accepted so far is not paged — it
keeps a row for every word of the secret — so a list that has to change
the panel's shape does not get one there, and neither the panel nor the
eye is drawn, as on the 240 dp panels. That is Japanese on a phone; on
the desktop window, where two columns of seven kana fit, it is drawn.
Worth the owner's eye: the alternative is a smaller face for the wide
scripts, which would keep the panel on every class at the cost of a type
size the design system does not have.

**One way to show a word.** `text::shown_word(lang, idx)` is the composed
form for every list, and every screen that shows a whole word goes
through it: the candidate strip, the words panel, the Words screen, the
quiz and Explore. `compose_latin1`, which dropped the marks Latin-1 has
no glyph for, is gone; pass 1's `DISPLAY` tables carry the composed form
for all four decomposed lists, so `ěšč` now reads as itself as well.
`text::shown_prefix` composes what has been typed: the kana pair table
for Japanese, the two-set automaton for Korean, arithmetic over U+AC00
rather than a table of syllables.

**Physical keyboards.** The kana keyboard accepts hiragana U+3041–U+3096
and the voicing marks in either form; the jamo keyboard accepts the
compatibility jamo U+3131–U+3163. No romanisation: a kana keyboard types
kana. A small kana typed directly has no key of its own, so the enabled
set does not gate it and the wizard takes it when a candidate wants it.

The font baker and `tests/glyphs.rs` now read the two key lists from
`osk-ui` instead of repeating their ranges, so a keycap cannot be drawn
that was not baked; `cjk.outl` already carried every one of them, so no
face changed. `ZHUYIN_KEYS` stays a constant until pass 3 defines that
keyboard.

### 16.42 Pinyin and 注音; ten languages (2026-09-10)

Pass 3 of three. The two Chinese lists become rows a person can choose,
and §15 item 37 is settled: all ten wordlists ship, none dimmed.

**A word that is read, not spelled.** Each word of the two Chinese lists
is one character, and a character has no spelling — it has a Mandarin
reading. `Language::typed(idx)` is therefore `None` for both lists and
there is no one key sequence to match a prefix against: a polyphone has
several readings, and two characters often share one. The wizard splits
what has been typed into a syllable and a tone instead — the tone key is
always the last one, a digit on the pinyin keyboard and one of the four
marks on the 注音 one — and matches through pass 1's
`candidates_pinyin` and `candidates_zhuyin`, which search every reading
of every character of the list. 中 is found under `zhong` in the first
tone and in the fourth, and 行 under both `xing` and `hang`.

`Language::word_readings(idx)` is the primitive the keyboard needed that
a prefix search cannot give: the readings of one word in that list's own
script. From it come the live keys — a letter or a bopomofo is live while
some candidate's syllable continues with it, a tone key is live in the
tones that syllable actually has — and whether what has been typed is
already a whole syllable.

**Homophony, stated.** Mandarin is homophonic, and the counts are worth
having in the open. Of the Simplified list, 710 syllable-and-tone
readings were shared by two or more characters, and 1,733 of the 2,048
characters shared every reading they had; on the Traditional list it was
767 and 1,760 (before §16.44 narrowed the readings). So auto-commit on a unique match, which is how the other
eight lists accept a word, almost never fires here: the candidate strip
is how a character is taken, and that is by design rather than by
omission.

**The strip pages.** A syllable without a tone leaves dozens of
characters — `yi` leaves sixty-seven on the Simplified list — and eight
cells are not enough. The strip's last cell carries the chevron
instead of a word
and advances by a page; the wizard keeps an offset that every key resets.
Only the two read lists page: a spelled word is narrowed by typing
another key, so nothing changed for the other eight.

**The first tone has no mark.** 注音 marks the second, third, fourth and
neutral tones and leaves the first unmarked, so a 注音 syllable can be
finished without a tone key at all. The wizard treats a spelling as
finished only where no key is still offered — no bopomofo would lengthen
the syllable and no mark would give it another tone — because while
either is live the person has not chosen yet. The brief asked for the
unmarked first tone to narrow the strip as soon as the spelling was
whole; that was tried and dropped. ㄉㄜ is a whole syllable and no
character in the list is read ㄉㄜ in the first tone, so the strip went
empty and the field said "No word starts like that" while ˊ and ˙ were
both live. Showing every tone of the syllable until a mark is pressed is
what a bopomofo keyboard does, and it is what the strip does now; ✓ or a
tap takes a first-tone character. (§16.43 supersedes this paragraph: the
first tone became a key of its own, ˉ, and no candidate shows before a
tone.)

**The two layouts.** Pinyin is the BIP-39 letter rows unchanged plus a
fourth row of `1 2 3 4 5` with the backspace and ✓ that row 3 carries on
the letters-only keyboard; `v` types `ü`, as it does on every Chinese
IME. 注音 is the 大千 layout: eleven keys in the first row, ten in the
others, the marks ˇ ˋ ˊ ˙ where that layout puts them and ㄦ at the end
of the first row, with backspace and ✓ after the last. The rows have
different unit counts, so the key width follows the row — that is the
layout people already have on a Taiwanese keyboard, and matching it is
worth more than an even pitch. Physical keyboards accept ASCII letters
and `1`–`5` for pinyin, bopomofo U+3105–U+3129 and the four marks for
注音, and no romanisation.

`keyboard::keycaps(KeyboardKind::Zhuyin)` replaced the baker's
`ZHUYIN_KEYS` constant, so all four non-Latin keyboards now hand the font
baker and `tests/glyphs.rs` their own key lists and no keycap can be
drawn that was not baked. `cjk.outl` already carried every one of them,
so no face changed.

**What the snapshots show.** `tools/scripts/load-key-zh-hans.txt` and
`load-key-zh-hant.txt` walk a twelve-word key on each list at the four
review sizes: the dead keys after one letter, the tone keys waking when
the spelling is a syllable, `yi` filling the strip with a chevron in its
last cell and the page it turns to, and the checksum. The scripts needed
two verbs the snapshot shell did not have — `cand <strip> <cell>` and
`candmore` — because a homophone is taken from the strip rather than by
finishing a word.

### 16.43 The tone completes a reading (2026-09-10)

**The decision.** On both Chinese keyboards the candidate strip stays
empty until a tone has been typed, and the 注音 keyboard gains ˉ so that
the first tone is a key like the other four.

**Why the strip waits.** A syllable on its own is not a choice a person
can make from eight cells. On the Simplified list `yi` leaves
sixty-seven characters, and 1,500 of the 2,048 have some reading whose
syllable alone leaves more than the strip holds, so the common case was
a full strip and a chevron. With the tone the reading is a real
narrowing: taking each character's heading reading, 1,755 of the 2,048
landed in a group of eight or fewer, 240 were identified outright, and
the largest group was `yi4` with 27 (§16.44 has the figures after the
rare readings went). Before the tone the strip's search is
by prefix, so `yi` reaches `yin` and `ying` too; the tone ends the
syllable, and with one the spelling must match whole, so `yi4` offers
exactly those 27. This is also what a 注音 input method does: no
candidates before the tone key.

An incomplete reading is not an error, so the strip is empty and the
error line says nothing — "No word starts like that" would be wrong
about `zhong`, which is a perfectly good syllable. Letters and bopomofo
stay live by the readings; the tone keys wake once the spelling is a
whole syllable of some remaining character, in the tones that syllable
actually has. After the tone only backspace, ✓ and the strip respond.

**ˉ, the eleventh key of row 2.** 大千 leaves the first tone to the space
bar, which this keyboard has no room for. `ˉ` (U+02C9, Spacing Modifier
Letters) ends the second row, so the row is eleven keys like the first
and every reading ends with a tone key. `keycaps(Zhuyin)` carries it, so
the baker and `tests/glyphs.rs` picked it up; the glyph came from
`NotoSans-Regular.ttf`, the same Latin face the other four marks come
from. The pass-3 rule that an unmarked spelling meant the first tone is
gone, and ✓ no longer stands in for a tone: it accepts a single
remaining candidate and is dead otherwise, as everywhere.

**Auto-commit came back.** §16.42 recorded that a unique match almost
never fires on these lists. With the tone required, 240 characters of
the Simplified list and 270 of the Traditional one had a reading no
other character of that list shares: `shuo1` is 说 and nothing else, so
it commits itself the way an exactly typed English word does. Paging
stays for the groups that still exceed the strip.

### 16.44 Rare readings removed (2026-09-10)

The readings table took every reading the Hanyu Da Zidian records
(`kHanyuPinyin`), so 王 answered to yù, and 服 and 佛 to bì. Those are
real but historical, nobody types them, and they padded the largest
groups by a character or two. The owner's decision: remove them. The
head reading alone (`kMandarin`) goes too far the other way, dropping 行
háng, 長 cháng, 重 chóng, 中 zhòng and 乐 yuè. The table now takes
`kMandarin` plus the two modern dictionaries Unihan carries, `kXHC1983`
(现代汉语词典) and `kTGHZ2013` (通用规范汉字字典), which keep those
polyphones and lose the historical ones.

The figures after the change, counting a reading exactly (the strip's
basis once a tone is typed): 385 syllables; 495 characters with more
than one reading, down from 1,033. On the Simplified list 586 tone
readings are shared by two or more characters and 1,652 characters share
every reading they have (571 and 1,643 on the Traditional list); by each
character's heading reading, 1,831 of 2,048 land in a group of eight or
fewer, 339 commit themselves, and 217 still page. The largest group on
either list is `yi4` with 20, then `shi4` with 17; 22 groups need a
second page and 2 a third. One side effect: 一 now carries yì and yí
as well as yī, since the modern dictionaries record its tone sandhi, so
it heads the `yi4` strip, being first in list order.

### 16.45 Twenty candidates at once (2026-09-10)

**The decision.** A Chinese candidate is one glyph, so the strip holds
far more than the two or three cells a spelled word needs. On the two
Chinese lists it becomes ten cells a row and two rows on every size
class, at the keyboard's own key pitch. The largest tone group either
list has is `yi4` with 20 (§16.44), so a whole group is on the screen at
once and nobody pages.

**The cell is a key.** `candidate_count` sizes a cell by the widest word
in the strip, which is what fits "abandon" on a Latin strip; that stays
for the spelled lists. A one-character strip ignores it: the cells tile
the strip's width ten to a row with no gap, each painting a face inset
from its cell the way a key does, and the strip is full-bleed like the
keyboard under it. So a cell is exactly a key wide at every reference
size — 24 px (26.8 dp) on the 240 × 320 px panel, 48 px at 480 × 640,
108 px (41 dp) on the phone, 72 px in the desktop window — and the strip
and the keys read as one control. `CANDIDATE_CELLS_HANZI = 10` names the
per-row count, `CANDIDATE_ROWS_HANZI = 2` the rows.

**Vertical room on the short panel.** Two 44 dp rows do not share the
268 × 358 dp panel with the app bar, the field, the error line and a
four-row pinyin or 注音 keyboard: the keys are already near their 36 dp
floor there, and §4.3 gives them their height first. The one-character
cell is therefore 32 dp on `small` (`CANDIDATE_CELL_HANZI_SMALL`) and
the class's touch floor elsewhere — 44 dp on a phone, 32 dp in a
window. That leaves the panel with 56 app bar, 52 field, two 32 dp rows,
the reserved error line and a 37 dp key, inside the 358 dp. The keyboard
gave up nothing. The words-so-far panel is already absent on `small`.

**The pager stays as the fallback.** `MAX_CANDIDATES` is now a capacity
per script: 8 for the spelled lists, 20 for the Chinese ones. With the
readings as they stand no group exceeds 20, so the chevron never shows
and the strip never pages. The offset, the chevron and `page_forward`
are kept for a group of twenty-one or more — another reading table,
another list — and the snapshot shell keeps `candmore`. What the
snapshot scripts show instead is one frame of the full twenty under
`yi4` / `ㄧˋ`, where the page-2 frames were.

### 16.46 The tiers in the owner's words, and two rows that were hiding (2026-09-10)

**The prose rule says what it always meant.** "No prose on a working
screen, explanations live in Learn" was read as a ban on paragraphs
everywhere, which is not the rule. What is banned everywhere is mannered
prose — metaphor and flourish standing in for direct statement. A
working screen is still labels, values and actions and never an
explanation; Learn and About are not working screens and may hold
paragraphs; and text of any length anywhere in the product is plain
statement of fact in an encyclopedia voice. `CLAUDE.md`'s first bullet
and DESIGN §2 principle 1 now say that. (§16.141 records one such
statement the owner asked for on a working sheet: what Import does with
the files not chosen.)

**Tier A and Tier B are named for what they are.** "Dedicated device"
described the hardware, not the property that matters, and "phone with
secure element" was the implementation. They become "airgapped signer"
and "secure phone"; the badges follow ("Tier A · airgapped", "A ·
airgapped"). The short badge is what the 268 dp status line draws
(§4.8), and it is the same length as the words it replaces, so the line
keeps its shape at every reference size; nothing on Home is cut at
240 × 320, 480 × 640, 1080 × 2340 or 960 × 640.

**The four statements say what the platform can see.** Each names the
attacker's actual reach — other software on the machine, the page host,
the element that has to be present — rather than summarising the tier.
They are longer than the ones they replace, so the tiers Document
scrolls further on the panel; it was already a document, and scrolling
is what a document does.

**"Shuffle PIN pad".** "Shuffle the pad" leaves the reader to work out
which pad, in a settings list that names its subject in every other row.
The row fits at every reference size.

**§15 item 35: no SLIP-132 row on Taproot.** There is no SLIP-132 prefix
for P2TR and there will not be one — a Taproot wallet is handed a
descriptor — so the dimmed row was making a promise. §4.11 gives a
dimmed reason only for "a condition of this device or this state that
the user could change", and no choice of this person's brings the format
back. The row is left off the Choice for Taproot, and
`export_slip132_same` ("same as account key") goes with it.

Explore's SLIP-132 row keeps its dimmed reason ("only for Nested and
SegWit"). It is the other case: the row is dimmed because the path being
explored implies a script type that has no SLIP-132 form, and the path
is the one thing that screen exists to change. Typing a different path
brings the row back, which is exactly the condition §4.11 dims for.

**§15 item 39: the review states the change it verified.** The review
distinguished the bad case — "not verified" in the danger tone — and
said nothing in the good one, so the verified state and the MINE badge
lived only on the per-output screens, which a person who taps Sign and
then holds never opens. The change entry now carries §4.8's Mine badge
under the amount when every change output was derived from a loaded key.
No new component, badge kind or string: it is `output_badge` with
`OutputBadge::Mine`, the same badge the output screen draws.

The cost is the accepted one. On the 268 dp panel the review's table is
now a document: 222 px of content in a 203 px region at 240 × 320, and
434 px in 404 px at 480 × 640, so about 20 dp scrolls into view. Nothing
is stranded and no row was shrunk, dropped or re-measured to avoid it;
`the_sign_review_is_one_screenful_at_every_size` now expects a document
on `small` and one screenful on the phone and in the window.

Leaving the row off exposed a stale pick behind it. The export flow
keeps the chosen format as an index that survives a change of script
type, so SLIP-132 picked on a SegWit key stayed picked when the script
type became Taproot: the format row named a format the Choice no longer
offered, and the Choice opened with nothing chosen. Changing the script
type now drops a format that script type has no form of, falling back to
the account key — which on Taproot is the string SLIP-132 produced
anyway, since `slip132::encode_xpub` gives Taproot the BIP-32 `xpub`
version bytes.

### 16.47 Strict extended keys, BIP-48 accounts, and the MuSig2 nonce (2026-09-10)

Three items closed: §15 items 24, 23 and 13.

**A strict decoder for anything that arrives as text (§15 item 24).**
`bitcoin` 0.32 accepted seven of the sixteen serialisations BIP-32 test
vector 5 says a parser must reject, for five reasons:

| Vector 5 reason | Why it got through |
|---|---|
| prvkey version / pubkey mismatch | `Xpriv::decode` does not check the pad byte at offset 45 |
| invalid prvkey prefix 04 | same |
| invalid prvkey prefix 01 | same |
| zero depth with non-zero parent fingerprint (the `xprv` and the `xpub`) | neither half checks that a depth-0 key has no parent and no index |
| zero depth with non-zero index (the `xprv` and the `xpub`) | same |

Nothing in the tree was wrong because of it — no path imported an `xprv` from text yet — but the
consequence waiting to happen is exact: a person pastes an `xprv` every
other wallet rejects, this device accepts it, and the addresses it
derives are addresses nobody can reproduce.

`core/osk-bip/src/xkey.rs` now decodes the text: base58check, 78 bytes,
the version bytes asked for, a zero parent fingerprint and child number
at depth 0, a zero pad byte and a scalar in `1..n` for a private key, a
point with a 0x02 or 0x03 prefix for a public one. `bitcoin`'s
base58check and secp types do the primitives; the rules are ours, and all
of them are applied before `Xpriv::decode` or `Xpub::decode` sees the
bytes. `MasterKey::decode` is the import path for an `xprv`, and the
xpub side goes the same way: `slip132::decode_xpub` shares the payload
checks, and the classifier, the scanned-key reading in Explore's
inspector and the policy parser's `[origin]xpub` all decode through
`xkey`.

The wording a person sees did not get vaguer. The vocabulary is the one
SLIP-132 decoding already used — `Base58`, `Length`, `UnknownVersion`,
`Key` — moved to `xkey` and shared, plus two new variants for the two
rejections none of the old words could state honestly: `Pad`, the byte
before a private key, and `ZeroDepth`, a key claiming depth 0 while
carrying a parent. `MasterKey::decode` reports either that or the
existing `NotMaster`.

The vectors are the BIP's own, in `tools/vectors/bip32/invalid-keys.txt`
with the source URL, the date and the file's SHA-256 in a README beside
it, the way `tools/wordlists` records Unihan. The test reads the file:
every key is rejected, and the reason the decoder gives is the reason the
BIP states.

**BIP-48 account xpubs (§15 item 23).** `KeyRef` carried only the four
single-signature accounts, so a multisig change output could not be
checked at all until a wallet policy had been registered: it showed as
unverified change, the review showed the danger state, and its amount
counted as a payment to others. It now carries BIP-48's two account
xpubs as well — `m/48'/coin'/account'/1'` for P2SH-P2WSH and `…/2'` for
P2WSH, and only those two, since the `3'` some wallets use for Taproot
multisig is not in BIP-48.

They are a second field and a second type, `MultisigAccountXpub`, not
more `AccountXpub`s. `AccountXpub` exists to turn one key into addresses
and a single-signature descriptor, and a multisig account can do neither
alone; overloading it would make `address` and `descriptor` lie. The new
type offers what one key can honestly do by itself, which is derive its
own public key on a chain, and that is what change verification needs.

Verification keeps the rule it had: an output is ours only when the
device reproduces the script in front of it. For a multisig output the
script comes from the PSBT's own `PSBT_OUT_WITNESS_SCRIPT`, so the check
is that the scriptPubKey commits to that script (directly, or through the
P2SH redeem script), and that one of the script's keys is this key
derived at the stated BIP-48 path. The threshold and the cosigners remain
the coordinator's word — settling those is what registering a wallet
policy is for — but the money is proved to go to a script holding a key
of ours on a change chain rather than anywhere the coordinator likes. A
coordinator that states no witness script gets the old answer:
unverified.

One thing changed with it. An output states one origin per key of the
script, so a multisig change output names cosigners this device does not
hold, and each of those used to raise "not loaded; treat it as a payment
to others". Those warnings are now decided by the output as a whole: a
caution another origin raised is noise once an origin has verified the
output, and goes. A danger stays either way. An origin that claims one
of our keys and derives to some other script is worth saying even about
an output we went on to verify, and a rule that drops dangers on a
verified output is a rule that can be aimed at.

**The MuSig2 nonce is never persisted (§15 item 13).** A MuSig2 signing
session's secret nonce lives in memory for the session and is destroyed
with it, on every tier, including Tier B where the secure element could
wrap it. A nonce that survives a restart is the classic way to reuse one,
and reusing a MuSig2 nonce hands over the private key. An interrupted
session is cheap: start again. §8.4 item 20 carries the rule for whoever
builds the session. No code was added; no MuSig2 session exists in the
tree yet.

What would reopen it: nothing about the nonce. The strict decoder would
be revisited if `bitcoin` starts applying the vector-5 rules itself, in
which case `xkey` becomes a thin wrapper rather than a decoder. The
BIP-48 shape would be revisited if a standard adds a third script type,
or if wallet policies become persistent, which would make the
witness-script route the fallback rather than the first answer.

### 16.48 The weak-session window, a cancelled pick, and a quiet background (2026-09-10)

Three items closed — §15 items 20, 30 and 31 — and half of item 29.

**The window where a seed was not sealed (§15 item 20).** The core asks
the shell for 32 bytes at start and rotates its session key onto them;
until the answer comes the key is a weak one and a key added meanwhile
sits in the clear (§16.21). The window is about a millisecond on every
shell that exists — the desktop shell answers `RequestEntropy` from
`getrandom` inside the same `poll_command` loop, before the frame that
asked is drawn — but nothing enforced that, and a shell that never
answered would take a seed anyway.

"Add key" on the confirm step is now dead until the request has been
answered. §4.13 keeps a disabled primary in its place and at its size, so
the button is there, greyed, and the screen says nothing about why: the
person sees a dimmed button for less than a frame, which no wording could
help with, and DESIGN §2 principle 1 allows a line of reason on a dimmed
control but does not ask for one. There is no spinner and no progress
state. The core refuses the step as well as dimming it, so a shell that
somehow delivered the tap gets nothing.

This makes "a key held under a weak session key" unreachable rather than
rare: the seed is sealed before it is stored, always. The sealing path
for a weak key stays, because the contract still allows a shell to answer
late or not at all, and everything but adding a key keeps working while
it does — which the Settings screen states, and which
`late_entropy_seals_what_was_plaintext_and_lock_survives_no_answer` used
to cover. That test is now
`a_key_is_not_added_until_the_shell_has_answered_the_entropy_request`,
which is the rule it encodes today.

**A cancelled picker is not a missing channel (§15 item 30).** One event
carried two different facts: "this device has no file channel" and "the
person closed the picker". Android sent it for both, so backing out of
the document picker told the person their phone could not read files.

`Event::FileCancelled { kind }` is now the second answer to
`Command::RequestFile` and `Command::ReadFile`. A shell answers each
request with exactly one of `File`, `FileCancelled` or `FileUnavailable`.
`FileUnavailable` keeps its meaning and its screen: the Sign entry row
says "no file", the scanner goes back to what it was doing.
`FileCancelled` puts the person back on the screen they came from with
nothing said, because they closed the picker and know it. No string was
added: a cancelled action that says something has invented a problem.

Only Android can send the new event, and its picker result says which
happened: `RESULT_OK` with bytes is a file, `RESULT_OK` with a read that
failed or no picker at all is unavailable, and anything else — the back
gesture, the picker's own cancel — is cancelled. The desktop, Pi and
snapshot shells have no picker for a person to close; the card and the
listing are the picker, and their unavailable answers were always the
truthful one. The snapshot shell's documented behaviour is unchanged, so
no script changed.

**A backgrounded app should be locked, not busy (§15 item 31).** The
Android shell posted a tick every 50 ms for the life of the process. A
phone in a pocket ran the auto-lock and auto-wipe timers at 20 Hz for
nothing.

`onStop` now locks the session at once and stops the ticker; `onStart`
starts it again. Locking on the way out is both the cheap answer and the
safe one: the person who comes back to a backgrounded app finds the lock
screen, which is what the auto-lock timeout would have given them a
minute later anyway. The core is told through one new event, `Event::Lock`
— "lock now, ahead of the timeout" — which does nothing to a session with
no PIN or no keys. There is no second time source in the core.

The wipe timer is a promise about wall-clock time, so the clock had to
become one. The shell ticked on `SystemClock.uptimeMillis`, which stops
while the device sleeps: a phone left overnight would have come back with
an auto-wipe timer that had barely moved. It now ticks on
`SystemClock.elapsedRealtime`, which counts sleep, and `onStart` sends
one tick before anything is drawn. The elapsed hour is therefore applied
by the core's own timers, from its own clock, before the first frame: a
background longer than the wipe timeout comes back to an app with no
keys, and a short one to a locked app that still has them.
`Event::Tick`'s contract now states the requirement — `CLOCK_BOOTTIME` on
Linux, `elapsedRealtime` on Android — because a shell that ticks on an
uptime clock quietly extends every timeout the person set.

One exception to locking on `onStop`: `ACTION_OPEN_DOCUMENT` and
`ACTION_CREATE_DOCUMENT` stop the activity while the document UI is on
top of it. The person is in the middle of the flow that asked for the
file, so a lock there would throw away the PSBT they are choosing and
send them back to a PIN pad instead of a review screen. `onStop` skips
the lock while a picker this activity started is outstanding, and stops
the ticker either way.

**`mlock` (§15 item 29), half-decided.** Tier A and Tier C can pin the
pages that hold secrets so a seed cannot reach swap in plaintext. What is
settled: it belongs to the Pi and desktop shells, not to the `no_std`
core; a `RLIMIT_MEMLOCK` too small to satisfy is carried on from, not
refused, with the failure on stderr where a developer sees it and a user
does not; the browser cannot do it and Android's keys are
element-wrapped, so neither is in scope. What is not settled is where the
one `unsafe` call lives. §10.2 names `opensigner-v4l2` and
`opensigner-avfoundation` as the crates allowed `unsafe`, and §5.4 says
`osk-crypto` may hold audited `unsafe` for exactly this — two answers
that do not agree, and the second one contradicts "not in the core".
Nothing was written, so nothing has to be undone when the placement is
decided.

### 16.49 The pages that hold secrets are pinned (2026-09-10)

Swap is the one place a secret can outlive the power switch. The kernel
may write any page of the process to disk, in the clear, and what it
writes stays there: a seed that was in RAM for a minute can be read off
the swap partition weeks later. §15 item 29 has said so since the start;
this closes its Tier A and Tier C half. `mlock` tells the kernel to keep
named pages resident, and `munlock` releases them.

**The buffers, not the process.** A shell could call `mlockall` and be
done with it, but a shell has no pointer to the core's secrets, so that
call would pin every page the process has — megabytes of framebuffer,
wordlists and rendering scratch — which the usual 64 KiB `RLIMIT_MEMLOCK`
refuses outright. Pinning the individual buffers costs a page each and
stays far under any limit. That puts the call where the secrets are, in
`osk-crypto`, which is where §5.4 always said the audited `unsafe` would
go: the exemption is used, not widened. It is five small blocks — taking
the pages, releasing them, the two dereferences of the pointer to them,
and the `mlock`/`munlock` pair — and no other crate under `core/` has
any.

**Whole pages, because `mlock` does not nest.** Both calls take an
address and act on the pages containing it, and there is no count behind
them: one `munlock` releases a page however many times it was locked. Two
secrets sharing a page would therefore unpin each other on the first
drop, silently, and the survivor would be swappable while its owner
believed otherwise. So `Pinned<T>` puts its value in a page-aligned
allocation whose size is `size_of::<T>()` rounded up to a whole number of
pages — every value gets pages that nothing else is on. The page is 4096
bytes everywhere the feature can be turned on, and 16384 on 64-bit macOS.

**An allocation, not a field.** `osk-crypto` said it did not allocate,
and it now allocates for exactly one thing: giving a secret pages of its
own. The pages are an allocation the type makes and frees itself, and a
`Pinned<T>` is the raw pointer to it — not a `Box`, whose `from_raw`
promises a pointer made under `Layout::new::<T>()` when this one is
deliberately larger and more aligned than that. That is what makes the address stable,
so the pages are pinned once at construction and released once on drop,
with nothing to re-check and nothing to keep in step. The first version
put the pages inside the value with `repr(align)`, and every consequence
of that was bad: a lock taken in a constructor held the temporary the
value was built in rather than the home it was moved to (`VmLck` showed
one locked page for two live secrets), moving the value memcpy'd the
whole page and left the plaintext behind at the old address — the very
exposure `mlock` exists to prevent — and every type holding one inherited
a 4 KiB size and a 4 KiB alignment, so a `Vec` of loaded keys took a page
per key. Moving the pointer version copies eight bytes and leaves
nothing. Every consumer already has an allocator: `opensigner-core` is
`no_std` plus `alloc`, and so are the Android and browser builds.

**What is pinned.** The session key, and every loaded master key.

- The session key is held for the whole session, and the seed, the words
  and the PIN digits are all sealed under it (§16.21). A copy of this key
  in swap opens every one of them wherever their ciphertext ended up, so
  it is the page that matters most and the only one whose pinning
  protects three other secrets at once.
- A `MasterKey` holds the BIP-32 master `xpriv`, which is plaintext for
  as long as the session is unlocked and is the key every signature comes
  from. It now lives on a pinned page instead of merely being erased on
  drop.

**What is not, and why.** The sealed seed and words are ciphertext: swap
may have them, and without the pinned session key they are nothing. A
`Sealed::with` closure's plaintext, a derived child key, a passphrase
being typed and the PIN pad's entry all live for one call or one screen
and are gone before the kernel would consider paging them; a page each
would cost more than it buys, and §5.3's stack discipline already covers
them. The session PIN's salted HMAC (`session.rs`) is long-lived and is
not pinned: it is not key material, though it would let an attacker who
has both the swap file and a kept-key blob guess a PIN against an HMAC
rather than against the blob's Argon2id. That is worth revisiting; it is
not part of this pass. Android and the browser are out of scope as
before: the browser cannot pin at all, and on Android the element wraps
the blob at rest rather than the keys in use — the master key and the
session key are plaintext in RAM for as long as the app is unlocked. What
Android does not have is a swap file: its swap is zRAM, compressed pages
that never leave memory. So the exposure there is a memory image, not a
disk (security review 2026-09-11, L9).

**A refusal is not fatal.** `RLIMIT_MEMLOCK` is 64 KiB on some machines
and 8 MiB on others, and a low one means `mlock` fails. The buffer then
holds the secret exactly as it would otherwise, unpinned, and the
application carries on: refusing to run would turn a paging risk into a
device that does nothing. The first refusal is reported once — one line
on the desktop shell's stderr, and on the Pi only under `--verbose`,
where a developer bringing a board up sees it and the person holding the
finished device never does. `RLIMIT_MEMLOCK` is a machine setting only a
developer can change, so there is no screen, no string and no warning
card for it.

**Off by default.** `osk-crypto` is `no_std` and also builds for the
browser and for Android, neither of which may link `libc`, so the calls
are behind a `pin-pages` feature that is additionally `cfg(unix)`. The
desktop and Pi shells turn it on through their own dependency on
`osk-crypto`; the Android, browser and snapshot builds never name it.
With the feature off the type is the same type — same storage, same
alignment, same zeroization on drop — minus the two calls, so the core
has one code path and not two.

### 16.50 Two taps for a candidate on the small panel (2026-09-10)

**The decision.** On `small` the first tap on a candidate selects it and
the second accepts it. `medium` and `wide` keep the single tap.

**Why.** A candidate cell on the 240 × 320 panel is 24 px, 4.3 mm, and a
fingertip covers it completely. A press highlight under the finger tells
the typist nothing: they learn what they hit only once it is committed.
On a Chinese strip that is twenty homophones side by side (§16.45) and a
mis-tap yields a plausible wrong character — 中 for 忠 — which the
checksum catches only at the end of twelve words and cannot locate. Two
taps make the strip readable: the first says what is under the finger,
the second takes it. This is what Trezor's devices do. On `medium` and
`wide` the cell is 6.5 mm, the finger does not hide it, and one tap
stands.

**Every strip, not only the Chinese ones.** The Latin, kana and jamo
strips take two taps on the panel as well, so the rule a person learns is
one rule and not a property of the wordlist they chose.

**Auto-select replaced auto-commit.** A reading whose tone left exactly
one character used to commit it with no tap at all (§16.43), and so did
the checksum filter when it left one word for the last position. On the
panel that character is now selected and waits for the tap every other
character costs — one keystroke pattern, one tap pattern, whether the
group was one character or twenty. The tone-first rule is unchanged: a
reading is still typed whole, tone included, before candidates show. A
spelled word that is typed out in full still commits itself on every
class: the person spelled it letter by letter and nothing was hidden
under a finger, so there is nothing for a second tap to confirm.

**A key clears the selection.** The alternative was to keep it on its
word where the new list still contains it. Clearing is less surprising:
a keystroke means the typist is still narrowing, the cells move under a
strip that has been rebuilt, and a selection that survived would sit
somewhere else than where it was made — so the habit of tapping the same
place twice would select rather than accept. Backspace, a page turn and
an accepted word clear it the same way. Nothing is ever accepted that the
typist last saw somewhere else on the screen.

**The idiom is the chosen cell.** A selected cell is drawn the way §4's
chosen chip is: the accent fill and its own text colour. The outline that
marks the one candidate ✓ takes is 2 px of accent inside a 24 px cell and
does not carry there; the fill is visible in the ring of screen a
fingertip leaves and unmistakable once the finger lifts. The lone
candidate's chip, which is outlined where a tap accepts, is filled on the
panel, because there it is selected. What no idiom can do is show the
state *under* the finger: a 4.3 mm cell is wholly covered while the
finger is down, so the selection is read after it lifts. Making it
legible during the press would need something outside the cell — the
field echoing the selected word beside what was typed, `zhong1 中` — which
is the smallest addition available if the owner wants it; it is not built.

**The cost.** A twelve-word mnemonic taken entirely from the strip costs
twelve extra taps on the panel: one per word. Typing a spelled word out
in full costs nothing extra, and neither does finishing it with ✓, so the
cost falls on the two Chinese lists, where the strip is the only way a
character is taken. `medium` and `wide` are unchanged.

### 16.51 The PIN's verifier is sealed too (2026-09-10)

§16.49 pinned the pages that hold the session key and the master keys,
and left one long-lived secret unpinned and unsealed: the session PIN's
verifier. `Session` held a 32-byte salt and `HMAC-SHA512(salt, digits)`
in plain `Secret`s, on ordinary pages the kernel may write to swap in
the clear. The digits beside them were already sealed under the session
key; the verifier was not. That open point is now closed, and closed by
sealing rather than by pinning.

**A fast hash of a short secret is the secret.** A PIN is four to eight
digits, which is at most a hundred million values, and HMAC-SHA512 is one
hash of a few bytes. Somebody holding a memory image or a hibernation
file that contains the salt and the hash recovers the digits in under a
second on any machine.

**What that would walk around.** Those digits are also the storage PIN
of a key kept on the device, and the kept blob is built so that guessing
the PIN is not worth attempting: Argon2id at 64 MiB per guess, and a
secure element that authenticates every use and deletes its key after a
set number of failures (§6, `keep.rs`). Somebody who read the PIN off a
swapped page enters it correctly the first time, so the Argon2id cost is
paid once and the element's counter never moves. The blob's whole design
rests on the PIN being expensive, and an unsealed verifier made it free.
It takes a memory image rather than routine paging, so this is defence in
depth and not a live break; it is also cheap to close.

**Sealed, not pinned.** §16.49's rule is that ciphertext in swap is
harmless, and the key it is sealed under is the page that gets pinned.
The verifier now follows the seed, the words and the digits: one
`Sealed<PinBits>` holding the salt and the hash together, because they
are only ever used together, in the one place that checks a typed PIN.
`PinHash::matches` unseals, hashes and compares in constant time as
before; it runs once per PIN a person types, so its cost is not worth a
second design. Rotation is the same as the digits': `Session::reseal`
re-seals both when the shell's fresh entropy rotates the key, which is
what happens on every lock.

**No plaintext fallback, so no plaintext.** `HeldPin` used to keep the
digits in the clear while the session key was still weak, because a
shell may answer `RequestEntropy` late. Copying that to the verifier
would have kept exactly the bytes this pass is closing. It is not
needed: §16.48 made "Add key" dead until the entropy request is
answered, and the first key is what sets the session PIN. The one other
path that sets a PIN — opening the key kept on the device, whose
storage PIN becomes the session PIN — was not gated, so a shell that
answered the kept-secret channel but not the entropy request could still
have set one. Its pad now waits for the same answer. `Session::set_pin`
returns whether it took the PIN, and takes neither the verifier nor the
digits if the key cannot seal, so there is no state where a PIN is held
in the clear. `HeldPin`'s plaintext form is gone with the case that
needed it.

### 16.52 The Pi runs as nobody, and reaches nothing it does not use (2026-09-11)

The security review of 2026-09-11 read the built Pi kernel `.config` and
the inittab and found the app running as root with `/dev/vcio`,
`/dev/mem`, debugfs, the raw `/dev/mmcblk0*` nodes and three EEPROM
drivers all reachable (M8). None of it is used by the shell, and all of
it is what a compromised shell — a decoder exploit on a hostile camera
frame, M5 — would use to persist or to exfiltrate.

**The one store that cannot be wiped.** The SoC's OTP has eight customer
rows: 256 bits, exactly a 24-word seed's entropy, written through the
firmware mailbox with `vcgencmd otp_set` and readable afterwards with
`otp_dump` on any card. Nothing erases it. The answer is not wiping but
reachability: `CONFIG_BCM_VCIO` is off, so there is no `/dev/vcio` to
write it through, while `RASPBERRYPI_FIRMWARE` and `BCM2835_MBOX` stay
in the kernel because the framebuffer needs them. `DEVMEM`, `DEBUG_FS`,
`SWAP`, `MTD`, the three `EEPROM_*` drivers, `NVMEM_SYSFS`,
`BLK_DEV_RAM`, `PROC_KCORE` and `KEXEC` are off with it, each with the
reason written beside it in `linux.fragment`.

**Init is the only root.** The app runs as `opensigner`, uid and gid
200, declared in `common/users.table` with no password, no home and
`/bin/false` as its shell. The inittab line is `setpriv --nnp
/usr/bin/setuidgid opensigner …`: BusyBox's `setpriv` has no `--reuid`,
so `setuidgid` does the drop and `setpriv` contributes `--nnp`, which
gives up the right to gain privileges through an exec. `/etc/mdev.conf`
hands that account the three devices the app opens — `fb0`,
`input/event*`, `video0` — and nothing else; `mdev`'s own default of
root:root 0660 keeps `/dev/mmcblk0*`, `/dev/i2c-*` and `/dev/vchiq` out
of reach. The card's data partition is mounted `uid=200,gid=200,
umask=077`, because vfat has no owners of its own, and the boot
partition is never mounted at all.

**Powering off is init's line, not the app's.** An unprivileged process
cannot call `reboot(2)`, so the shell now draws its last frame, waits,
and exits; `::wait:/sbin/poweroff -f` is the next line init runs.
`--no-poweroff` is gone with the branch it selected, and the pi tests
follow.

**Entropy from `/dev/random`.** Since Linux 5.6 it blocks only until the
CSPRNG is initialised and never afterwards, and the board seeds that
from `HW_RANDOM_BCM2835` before the first frame, so the guarantee is
free (L2).

**No secure boot, said once.** The Pi 3 has no verified boot: the FAT
boot partition holds `config.txt`, `cmdline.txt` and the kernel
unsigned, and a person with the card for a minute replaces the software
(M6). A build hash on About would not help, because whoever replaces the
image replaces the hash. So a Tier A device opens on one screen —
`ScreenKind::NoSecureBoot`, `views/secure_boot.rs` — stating that the
device does not support secure boot and that only a card you flashed
yourself should be used, with one way on to Home and nothing behind it.
The Tier A statement says the same thing. What reopens this: a board
with signed boot, the Pi 4's EEPROM with a customer key or a Pi 5, which
is the Tier A+ target.

### 16.53 A guess at a time: the kept key's format is version 2 (2026-09-11)

The review's H2: `KEK = HMAC(Argon2id(pin, salt_pin), mac)` asked the
secure element to authenticate `salt_se`, a constant in the blob's
header. The element's answer was therefore the same value for every PIN
tried, so an attacker who read one tag out of the app's memory — root on
the phone, or an unlocked bootloader, during one legitimate unlock —
held it for every later guess, and the rest of the search ran offline at
one Argon2id per candidate: about 80 ms, so a 4-digit PIN in a quarter of
an hour.

**The element is asked about the guess.** `challenge =
HMAC-SHA256(Argon2id(pin, salt_pin), salt_se)`, the element authenticates
that, and `KEK = HMAC-SHA256(Argon2id(pin, salt_pin), mac)`. The cost to
the person is unchanged, one authentication per typed PIN, and a captured
tag is now worth exactly the one candidate it was made for — which is
the PIN that already opened the blob. `keep::challenge` returns a `Guess`
carrying the stretched PIN and its challenge, so the stretching happens
once per attempt and `write`, `open` and `set_duress` take the `Guess`
rather than the digits. `VERSION` is 2 and a version-1 blob is not read;
nothing is in the field. (The rule that an earlier version is refused
held until §16.112's pass E3, which reads a version-4 blob because
phones now hold kept keys and a bump that lost them would cost every
owner for a notes feature. Versions 1 to 3 stay refused.)

**What the chip does and does not count.** §16.36 said "the chip's key is
deleted after eight wrong PINs". It is not: the app counts, and after
eight it sends `ForgetSecret`, which is what deletes the blob and the
element's keys. The chip counts nothing about the app's PIN. The text
says that now, in `keep.rs` and in the Tier B statement, which states
what the element does promise — every attempt costs the device lock or a
biometric — and the condition it holds under: an operating system that is
intact.

**An automatic wipe is RAM only.** H1: `wipe_secrets` sent
`ForgetSecret` whenever a key was kept, and the auto-wipe timer, five
wrong session PINs and Android's `onStop` all reached it, so
backgrounding the app past ten minutes deleted the stored key. The two
are now separate: `wipe_secrets` clears memory and nothing else, and
`wipe_everything` is what the deliberate paths call — Wipe all keys,
Wipe and exit, Forget, the duress PIN, and the eighth wrong storage PIN.
The timer stays, RAM-only, at its default: it is worth something on Tier
C, where any process can read memory, and on a Tier A device left
powered on, and it costs nothing on Tier B once the stored key survives
it. The wipe screens say which keys go.

**A phone whose boot is not verified is not a signer.** On a device the
platform reports as `BootState::Unverified` — an unlocked bootloader, or
a system nobody signed — no tier statement this app makes is true: a root
process reads the master key, the session key and the element's tag out
of `/proc/<pid>/mem`, and `FLAG_SECURE` stops nothing. The app shows one
screen saying so and offers Exit (`ScreenKind::BootRefused`, derived from
the reported boot state rather than being a place the app navigates to).
GrapheneOS is the primary Tier B target and relocks under its own key, so
`SELF_SIGNED` with `deviceLocked` is verified, not refused; the Android
shell reads `deviceLocked` from the attestation's root of trust
alongside `verifiedBootState`. No root detection beyond attestation: it
is a game that cannot be won, and Learn › Devices states the assumption
instead. L1 is in the same pass — `KeptSecret.beginMac` narrowed its
`forget()` to `KeyPermanentlyInvalidatedException`, so a transient
Keystore error no longer destroys the kept key.

### 16.54 A transaction that lies about itself is refused (2026-09-11)

Two review findings, fixed together because they are one screen.

**The two-round fee attack (H3).** For p2wpkh, p2sh-p2wpkh, p2wsh and
p2sh-p2wsh, `classify` took the input's amount from `witness_utxo` alone
and `inspect` said nothing when the previous transaction was absent. That
is the attack Trezor, Ledger and Coldcard were hit with in 2020: a
coordinator lies about a different input's amount in each of two signing
rounds and combines the valid signatures into a transaction whose fee is
the difference. The fee warnings do not fire, because each round's lie
makes the fee look ordinary. Taproot is immune — `Prevouts::All` commits
to every input's amount — and legacy already required the previous
transaction. Now every SegWit v0 input the device signs must carry
`non_witness_utxo`, checked against the outpoint and the amount as
`classify` already checked them when both were present, and an input
without it raises `WarningKind::AmountUnverified`. The test builder adds
the previous transaction to SegWit v0 inputs, so every fixture with such
an input was rewritten.

**Dangers that were evidence, not risk (M2).** `can_sign` needed only
`acknowledged`, so `ChangeSpoof` — a change output claiming one of our
keys and paying somewhere else — and `UtxoMismatch` could be read and
signed through. Neither is a transaction a person should be allowed to
approve: both are a coordinator caught lying. `Level::Blocked` sits above
`Danger`; `ChangeSpoof`, `UtxoMismatch` and `AmountUnverified` are
blocked; `sign()` returns `Error::Blocked` with or without `force`; and
the Sign review ends at the warning with no acknowledgement offered and
the hold shut. DESIGN §4.11, UX §4 and §13 carry the fourth rank.

What reopens this: a wallet that cannot supply previous transactions for
its SegWit v0 inputs. Every coordinator worth pairing with can, and the
cost is a larger PSBT — which is why `MAX_FRAGMENTS` went up.

That reopening fired: §16.98 narrows the rule to transactions with more
than one input.

### 16.55 Blinded contexts, and pages filled in place (2026-09-11)

**The context is blinded per key (M1).** `Secp256k1::new()` was used
unblinded everywhere, so libsecp256k1's projective-coordinate blinding —
its defence against power and timing analysis of a scalar multiplication
— was never seeded. `seeded_randomize` needs no crate feature. A
`MasterKey` now owns its context, blinded when the key is made and again
whenever the session hands down fresh entropy (`reblind`, `blinded`),
and the signing paths take the context as their first argument
(`SessionKey::secp_blind`, `Session::secp_blind`, `message::sign`)
rather than each making one. The self-test blinds from a fixed seed,
having no session to derive one from.

**Nothing left in the frame (M3).** `Pinned::new` copied its argument to
the page and left the caller's copy where it stood; it now zeroizes the
source and forgets it. `Pinned::new_with` goes further and makes no copy
at all: the page starts zeroed and the caller fills it in place, which is
how `SessionKey::build` now assembles a key from entropy.

**Two more vectors on every start (L7).** The self-test had no Schnorr
vector and no non-Latin wordlist; it now runs BIP-340 and the Japanese
BIP-39 vector with its non-ASCII passphrase, six checks in all. The
Japanese vector is the only caller of `to_seed_unchecked` outside the
vector tests, and its doc says so. Its PBKDF2 costs nothing measurable:
the dev profile builds `osk-bip` and `osk-crypto` optimised, and the
flows suite runs in the same time with the check as without it.

**A test that reads its own memory (§12).** `opensigner-core/tests/
memory.rs` loads the test key, signs the demo transaction, wipes every
key, drops the application and then searches this process's writable
anonymous, heap and stack mappings through `/proc/self/mem` for sixteen
bytes from the middle of the seed and sixteen from the middle of the
master private key. The needles are stated as constants rather than
derived, because a test that computed them would find itself. It finds
neither after the wipe. It is `#[ignore]`d: it is Linux-only, and what a
moved-from copy inside `bitcoin` or `secp256k1` leaves behind is a
property of the build profile rather than of this code, so it belongs to
the profile you point it at and not to `just`.

### 16.56 Four small edges and a dependency policy (2026-09-11)

- **Settings are clamped** (M4). `settings::read` accepted any `u64` for
  the two timers, and the file is writable by anything that can reach the
  card or the desktop config directory, so a tampered line could set an
  auto-lock of a year. A value that is not in `LOCK_OPTIONS` or
  `WIPE_OPTIONS` now leaves that timer at its default. Nothing else in
  the file can do harm: it holds no secret, and every other key is a
  choice the Settings screen shows.
- **A UR part cannot ask for a gigabyte** (L6). The count comes out of
  the part's own text and the decoder reserves a slot per fragment before
  any part is checked. The cap was already there; it was too low.
  `MAX_FRAGMENTS` is 2048, because a hundred-input PSBT at the app's
  fragment sizes is several hundred parts and H3's previous transactions
  make it longer — 2048 parts of the largest fragment is still under a
  mebibyte.
- **`PinEntry::same_as` short-circuits on length** (L5), which leaks the
  length of a PIN the same screen displays as dots it draws one per
  digit. It is commented rather than changed.
- **jni-sys is pinned to 0.3.0** (M7). 0.3.1 re-exports 0.4, which builds
  its function table with a proc-macro and so puts `jni-sys-macros`,
  `syn`, `quote` and `proc-macro2` in the FFI crate's graph. 0.3.0 stands
  alone and declares the same types. The FFI graph now reaches `syn`
  only through `g2gen`, which comes with `rqrr`; that is the next crate
  on §10.1's list.
- **`deny.toml`** (M7, §5.4). Advisories, licences, duplicates and
  sources, run locally like everything else. Each allowed licence is
  named rather than inferred from a class, each duplicate that exists
  today is skipped with the reason it exists, and the one advisory
  ignored is named by its RUSTSEC id — `ttf-parser`, unmaintained,
  reached only by the font-baking build tool and by `winit` on the
  desktop, neither of which is in a device binary. The wildcard check is
  a warning rather than an error: the seven `core/osk-*` crates are
  publishable and depend on each other by path, which cargo-deny counts
  as unpinned and which becomes true the day they are published.
  `cargo deny check` is commented out in the justfile's `lint` recipe
  until `cargo install cargo-deny` is part of the documented setup, so
  that `just` still runs on a machine without it.
- **VERIFY.md and the README say there is no release** (L8). The
  fingerprint in VERIFY.md is a placeholder and the APK is unsigned;
  both now say so rather than reading as instructions for something that
  exists.

### 16.57 A Schnorr signature can be fresh instead (2026-09-11)

§16.14 chose BIP-340 with all-zero auxiliary randomness so that a taproot
signature is reproducible: another implementation holding the same words
over the same transaction produces the same 64 bytes, which is the check
docs/VERIFY.md asks a user to run. The cost is BIP-340's own
recommendation — 32 fresh bytes per signature blind the nonce derivation
against a fault or side-channel attack — which the zero mode gives up
(security review 2026-09-11, L3).

**Both, as a setting.** `osk_psbt::Schnorr` is the choice, `Deterministic`
or `Fresh`, kept as the eighth line of the settings file
(`schnorr=deterministic|fresh`) and offered as the Settings row beside
Nonce. Deterministic is the default, because comparing bytes against a
second implementation is the property this device is verified by, and a
person who wants the randomness instead can say so.

**The bytes are the caller's.** `sign()` and `message::sign()` take an
`osk_psbt::Aux`, which is `Deterministic` or `Fresh` with the 32 bytes
for this call, drawn by the core from the session key with a count of the
draws (`OpenSigner::schnorr_aux`), so no two signings of a session share
them and the crate still owns no random number generator. `sign()` hashes
the call's bytes with the input index, so two inputs of one transaction
do not share them either. The sign-twice check is unchanged: both
signings use the input's one value, so a second signature that differs is
the signing going wrong rather than the randomness working.

### 16.58 Entropy that cannot be printed (2026-09-11)

`Event::Entropy { bytes: [u8; 32] }` carried the seed of the session key
through an enum that derives `Debug`, so any shell that logged the event
it was about to deliver would write it out, and the array sat in whatever
copies the delivery made until they were overwritten (security review
2026-09-11, L4).

The variant is now `Event::Entropy(EntropyBytes)`. `EntropyBytes` prints
`EntropyBytes(..)`, hands the bytes over exactly once through
`into_bytes(self)`, and zeroizes what it still holds when it is dropped,
so an event that is built and never delivered leaves nothing behind. It
keeps `PartialEq`, because tests compare events, and `Clone`, because
`Event` does.

That makes `zeroize` the one dependency of `osk-shell-api`, which had
none. The alternative was a hand-written volatile wipe in `Drop`,
duplicating in the one crate that has no `unsafe` what the rest of the
workspace already gets from a crate it already builds
(`docs/deps/zeroize.md`).

### 16.59 Fuzzing, and the two crashes it found (2026-09-11)

§12 asked for fuzzing of every parser that reads something the device did
not write and nothing existed. `fuzz/` now holds ten `cargo fuzz`
targets, one per such parser, with a committed seed corpus
(`fuzz/README.md`).

**It is a local practice and it gates nothing.** The targets exist, the
corpus is committed, and the minimized input for any crash that is still
open is committed beside it. A developer runs `just fuzz TARGET SECONDS`
on their own machine when they change a parser or want to spend an
afternoon on one. No hosted runner, no GitHub Actions, no release step
that waits on a run (§16.56's dependency policy, and the same reason: a
metered service is not something this project's build depends on). What a
run found and when is written into `fuzz/README.md`, so the record of the
practice is in the repository rather than in a service's history.

Eight targets were clean on the first pass. The two that were not are the
two places where a hostile input chose how much work the device did:

**A camera frame could power the device off.** `rqrr` 0.11.0's
`Perspective::map` asserted that the mapped point fits in an `i32`, and a
frame with a near-degenerate capstone group breaks that during grid
detection, before any payload is read. The Pi shell is built with
`panic = "abort"` and is the only process on the board, so a printed
sheet held in front of the camera powers the device off in the middle of
a signing session. The crate is now vendored at `third_party/rqrr`
(`[patch.crates-io]` in the root manifest and in `fuzz/Cargo.toml`),
`map` returns `Option<Point>`, and every caller rejects the candidate
grid instead of asserting — a frame that cannot be sampled yields no
grid, which is what a frame with no QR code in it already yields.
Vendoring is the first step of `local/crate-reduction-plan.md`'s plan for
this crate: with `lru` and `g2p` trimmed out of the copy it takes
thirteen crates off the device graphs. `third_party/rqrr/OPENSIGNER.md`
records the change and the panic sites still in the copy.

**A file on storage could choose the Argon2 cost.** `keep::header`
checked the blob's length and version byte and took the Argon2id memory,
passes and lanes from the bytes; `stretch` then allocated what they asked
for. A blob claiming 16 GiB aborts on the allocation the moment a PIN is
typed, and one claiming 448 MiB over 257 passes hangs inside a single
attempt. The header is associated data, so a changed cost could never
have opened anything — the whole cost of the lie was the allocation and
the time. `header()` now returns `None` unless each of the three is
between the `argon2` crate's minimum and `DEVICE_PARAMS`, which is the
same shape as §16.56's clamp on `settings::read`: a file that asks for
something this build would not have written is not a file this build
reads. Tests still write a cheaper cost through `Header::cost`, since a
suite that spends 64 MiB per attempt is a suite nobody runs.

### 16.60 Argon2 on one generation, and rqrr with nothing behind it (2026-09-11)

`local/crate-reduction-plan.md` (security review item M7) listed three
dependency changes worth making. Two of them are done.

**`argon2` 0.5 → 0.6.** One line in the root manifest. Version 0.5 took
`blake2 0.10`, and with it a second copy of the RustCrypto hashing stack
that `sha2`, `hmac` and `pbkdf2` already bring on the 0.11 generation:
`digest 0.10`, `block-buffer 0.10`, `crypto-common 0.1`, `generic-array`
and `cpufeatures 0.2`. Five crates, and four stale skips in `deny.toml`.
`keep.rs` did not change: `Params::new`, `Block` and
`hash_password_into_with_memory` are the same, and the blob format was
never touched by which crate version wrote it.

**`rqrr` trimmed.** The vendored copy (§16.59) had two dependencies, and
twelve crates stood behind them. `lru` held one cache of 251 regions
keyed by a `u8`; it is an array of that length now, with a recency stamp
per entry. The eviction had to stay — a version 30 symbol at three pixels
a module has thousands of black regions, and a table that refused the
252nd stops finding capstones part-way down the frame — so the array
keeps what `LruCache` did: the oldest entry is repainted black, and its
index is reused. `g2p` generated two Galois fields with a proc-macro.
GF(2^8) is now one short file beside the decoder, the same field with the same
generator and the same tables, built in a `const` block. GF(2^4) is gone
entirely: it existed to BCH-decode the 15-bit format information, and the
nearest of the 32 valid format codewords is the same answer, since the
code's minimum distance is 7. That is what quirc, which `rqrr` is a port
of, does.

`g2gen` was the only proc-macro in a device dependency graph.

| Graph | Before | After `argon2` | After both |
|---|--:|--:|--:|
| `opensigner-core` | 77 | 72 | 60 |
| `opensigner-ffi` | 80 | 75 | 63 |
| `opensigner-pi` | 81 | 76 | 64 |
| `opensigner-desktop` | 150 | 145 | 137 |

**And the panic pass the vendoring was for.** Ten more sites that a
hostile frame can reach are rejections now: `rotate_capstone`'s
perspective, the two `panic!`s in the flood fill's colouring, a
`partial_cmp` that is `None` for a NaN score, and the bit reader's and
Reed-Solomon decoder's assertions. Each is "no capstone", "no grid" or a
decode error — outcomes a frame with no QR code in it already produces.
The copy carries `#![forbid(unsafe_code)]`, which is the thing a
dependency cannot be given.
`third_party/rqrr/OPENSIGNER.md` has the table.

The third change, BC-UR in-house, is open. It is worth ten more crates,
and §10.1's target of under 40 needs it and more besides.

### 16.61 BC-UR in-house, and the last of the crate-reduction plan (2026-09-11)

`local/crate-reduction-plan.md` §5.2 was the third and last of the three
dependency changes. The `ur` 0.5.2 crate is gone; `core/osk-codec/src/ur/`
is the whole of BCR-2020-005 and BCR-2020-012 in about a thousand lines,
`no_std` + `alloc`, `#![forbid(unsafe_code)]`, with SHA-256 from
`osk-crypto` and nothing else.

**What left.** Nine crates for 1,687 lines of the crate's own code:
`ur`, `bitcoin_hashes` 1.x with `bitcoin-internals` and
`bitcoin-consensus-encoding` behind it — a second rust-bitcoin generation
beside the 0.14 line `bitcoin` 0.32 uses, and there only to hash eight
bytes of fountain seed — `crc` and `crc-catalog` for one bitwise CRC-32,
`minicbor` for a five-element array, and `rand_xoshiro` with `rand_core`.
`minicbor`'s derive macro was also what kept `syn`, `quote`,
`proc-macro2` and `unicode-ident` in the device graphs after §16.60 took
`g2gen` out, so those four leave with it.

| Graph | Before | After |
|---|--:|--:|
| `opensigner-core` | 60 | 51 |
| `opensigner-ffi` | 63 | 54 |
| `opensigner-pi` | 64 | 55 |
| `opensigner-desktop` | 137 | 128 |

The plan predicted 50 for the core graph. 51 is the measured figure; the
headline in §1 of the plan was taken from the 77-crate baseline and did
not carry §16.60's correction, where `rqrr` removed twelve crates rather
than thirteen.

**Why it had to be byte for byte.** Sparrow, Nunchuk and BlueWallet must
read what the device shows, and the device must finish what they show.
Four details decide that and none is stated plainly in the specification:
the part is an *untagged* CBOR array of `seqNum`, `seqLen`, `messageLen`,
`checksum` and `data`; the fragment seed is the sequence number and the
checksum, each 32-bit big-endian, hashed with SHA-256 into four
xoshiro256\*\* state words read big-endian; `next_int` is a multiply by a
double and not a modulus; and the degree comes from Vose's alias sampler
over the weights `1/i`. Any of the four written "more correctly" yields a
stream that decodes nowhere.

**How that is held.** The specification's own vectors, the `ur` crate's
unit vectors copied into the modules, and — because a specification
vector cannot catch a payload no document covers — three streams over a
120-byte payload at fragment lengths 20, 60 and 400 recorded from the
crate itself and asserted as constants. Before the crate was removed it
was kept as a dev-dependency and run beside the new code over 200 random
payloads at random fragment lengths: every single-part string, every one
of up to 400 part strings per case, and each decoder finishing the
other's stream with half the parts dropped. All 200 matched. That test is
gone with the dependency; `docs/deps/ur.md` is the record.

**Two deliberate differences**, neither of them interoperable behaviour:
a part whose CBOR is followed by trailing bytes is refused, where the
crate ignored them; and only the minimal bytewords style is written,
since no UR uses the standard or URI styles.

`docs/PLANNING.md` §10.1's target of under 40 crates in the core graph is
not reached and, as the plan's §7 says, is not reachable while `bitcoin`,
`secp256k1`, `tiny-skia` and the RustCrypto 0.11 generation stay. 51 is
what the architecture costs with every avoidable dependency gone. Whether
§10.1 is amended to that figure or kept as an acknowledged breach is a
decision still open.

### 16.62 Two numbers that were rules, retired (2026-09-11)

Both were set before the thing they measured existed, and both were
answered the same way by the owner: the project does not need an
arbitrary target.

**Crate count.** §10.1 said under 40 crates in the core graph. The three
removals of §16.60 and §16.61 took it from 77 to 51, and
`local/crate-reduction-plan.md` §7 lists the only three ways to take
eleven more: an in-house Argon2id, an in-house rasteriser in place of
`tiny-skia`, or `bitcoin` itself. Each is exactly the code §5.3 argues
against writing, so 40 was not a target that the architecture could
meet, only a number to fall short of. §10.1 now has no count. The rule
that stays is the one that was doing the work: every crate in a device
graph is there because something needs it and nothing in the tree can do
the job, every addition carries a page in `docs/deps/`, and `deny.toml`
and `supply-chain/` hold the rest. The measured figure, 51 with the
workspace's own eight among them, is recorded so that the next person to
count knows what they are comparing against.

**Learn page length.** §16.36 set 220 words a page, and §16.37 and §16.38
trimmed text to hold it. No page holds it now: the shortest is
Transactions at 215 words and the longest Bitcoin software at 590,
and every one of the sixteen grew for a reason recorded in this log —
the two nonce settings, the Blocked rank, what a secure element does and
does not count, the boot statement. Cutting words to reach a number
would have meant dropping one of those. The rule is gone. What a page
must be is already in CLAUDE.md: plain statement of fact, explanation
that a working screen is not allowed to carry, and nothing else. The
page is as long as that takes. `tools/learn/sync.py import` still prints
the counts, because a page that doubles is worth a look; it is a signal
and not a limit.

### 16.63 One PIN pad (2026-09-11)

The first run of the 2026-09-11 build on the owner's GrapheneOS phone
found three things in the stored key's PIN handling, and one rule from
the owner covers all of them: a person sees one PIN pad and is never
made aware that two exist. The system's own authentication prompt is a
different matter and is fine, because it is visibly the phone's.

**What there was.** Two pads that looked alike and were not. The stored
key's pad, reached from Keys › Stored key while the app was unlocked,
went through the blob and the element: it honoured the duress PIN and
counted wrong PINs toward the key's removal. The lock screen, which is
where a key already in memory waits, checked the session PIN in memory
and nothing else. So the duress PIN typed on the lock screen was one of
five wrong PINs, and the lock screen is exactly where a person under
coercion is standing. And the count of wrong PINs was never set back:
`count_attempt` added one, `write` started at zero, and nothing in
between touched the byte, so wrong PINs accumulated over the life of a
blob across correct opens and restarts. Four typed today after four
typed last week removed the key.

**What there is.** On a device that keeps a key, the lock screen is the
stored key's pad as well. The session PIN opens the session as before,
with no element involved. Any other PIN is tried against the blob, which
asks the element and so shows the phone's prompt: the duress record
opening removes everything and shows the empty Home, saying nothing; the
key record opening — the stored key's PIN typed over a session whose
keys were loaded under another PIN — ends that session and opens the
stored key's, as a fresh start would; neither opening counts one against
the stored key, with the same caption the stored key's pad shows, and
the eighth removes the key from the device and the keys from memory with
it, since the PIN that would unseal them is the one nobody typed. The
session's own count of five, which wipes memory, does not run on such a
device: one pad has one rule, and with a stored key on the device that
rule is removal after eight. Dismissing the phone's prompt costs nothing
and changes nothing.

A right PIN ends the run of wrong ones, on either pad: the count is of
PINs typed since the key last opened, and the blob is stored again with
the byte at zero. The byte was always outside the associated data so
that a wrong PIN could bump it; a right one setting it back changes
nothing about what an attacker with the bytes can do, which is to reset
it themselves and pay the element for every attempt anyway.

The stored key's pad carries the lock screen's title, "Enter your PIN".
The row in Keys that opens it is what says "Stored key". DESIGN §4.3
and §5 say the two are one pad.

**Cost.** A mistyped PIN on the lock screen of a device that keeps a key
brings up the phone's prompt, because the element is being asked
whether the PIN is the duress PIN. When the session's key came from the
stored blob the two PINs are the same and a right PIN never reaches the
element. The alternative was to route every unlock through the element,
which is consistent with "the chip gates each attempt" but puts a prompt
on every unlock; the owner's rule does not require it.

### 16.64 The PIN is the front door (2026-09-12)

§16.63 made the two pads behave alike. The owner's next question was
why the stored key's pad was somewhere a person went — Home › Keys ›
Stored key — instead of what the app opened on: "the PIN is supposed to
be when the app opens, before I see anything."

**The pad is the first screen.** On a device that keeps a key, the
stored key's pad shows whenever no key is loaded: at start, after the
automatic wipe, after any path that leaves memory empty while the blob
is still on the device. It has no back and nothing behind it. The right
PIN opens the stored key into the session and the app is where it was
going; the duress PIN removes everything and shows the empty Home; the
eighth wrong PIN removes the key and shows the Result that says so.
Nothing on the device — Learn, Settings, Verify — is reachable ahead of
the PIN, which is the point: a phone with a key on it asks who is
holding it before it shows anything, the way the lock screen already did
for a key in memory. The two together are one pad from the person's
side: a key in memory or a key on the device, the same title, the same
caption, the same PINs doing the same things.

This is `front_door()`, a condition like `is_locked()` rather than a
screen that is navigated to, so no path through the app can land on Home
with a stored key and no way to open it. The "Stored key" row in Keys,
the count it carried and `Screen::StoredKey` are gone; `ScreenKind::
StoredKey` stays as what the pad reports itself as.

**A blob the shell cannot produce is not kept.** The first run of
§16.63 on the owner's phone typed wrong PINs into a pad that did
nothing: no prompt, no count, no removal. The core's path to the element
had not changed, so the failure was the shell's, and two shell states
produce exactly that: a prompt whose callback never came, which left the
one-prompt-at-a-time flag set and every later request answered "no
element"; and Keystore keys invalidated under the app, on which the
shell deletes the blob without telling the core, so the core keeps
asking for a blob that is gone. Both are closed. The Android shell keeps
the `CancellationSignal` of the prompt in front of the person and
cancels it when the core asks again, dropping the stale answer if it
ever comes; and when it finds no usable key and no blob, it tells the
core nothing is kept. On the core's side, a `LoadSecret` the shell
answers with nothing, or with bytes this build does not read, sets
`secret_kept` false: the pad goes and the empty app is what shows,
rather than a door with nothing behind it.

Not reproduced here. The phone's own state is what will say which of
the two it was, and whether it was one of them at all.

### 16.65 Keeping a key is part of adding it, and forgetting it is forgetting it (2026-09-12)

Three more from the owner's first day with a stored key on a phone,
under the same rule as §16.63 and §16.64: the stored key is the most
security-critical flow in the app, and every path through it has to do
what a person would expect it to do.

**A key that was never kept was never kept.** The owner added a key,
set a PIN, swiped the app away and found the key gone. That is the
model — a key is in memory until it is kept — but nothing in the flow
said so, and a person who has just typed twelve words and a PIN on a
phone assumes the phone has them. On a device that can keep a key and
keeps none, adding a key now ends on the Keep hold instead of Home: the
same screen the key menu offered, with the same rows, reached without
looking for it. Back is "not now"; the key is added either way, and the
key menu still offers the row later. A device that cannot keep
anything, or already keeps a key, goes Home as before.

The offer first carried the hold alone, and the back chevron was the
only way to decline it — which the owner read as no way to decline it.
It now carries "Not now" beside the hold, which does what the chevron
does: the key stays added and nothing is kept. It is the first Hold with
a secondary action, and DESIGN §4.13 and §5 say a Hold may carry one,
for the step that declines it. The same screen reached from the key
menu's row carries the hold alone, because a row that was tapped is
left by the chevron.

**Forgetting the kept key forgets it.** The key menu's Forget removed a
key from memory and nothing else. On a device that kept that key, the
result was a pad asking for the PIN — no key loaded, a blob on the
device — and the PIN brought the key straight back; the owner read that,
correctly, as forget not working. Forgetting the key the device keeps
now forgets the copy on the device with it, and the Forget hold's table
says so ("Stored key · removed from this device") before the hold.
Settings › Forget stored key is gone: it did what this does, from a
place nobody would look for it, and with the pad as the front door a
stored key is never unloaded while the app is open, so a forget that
left the key in memory had no case left. Wipe all keys still takes the
stored key too, and says so.

**One number on the pad.** With no key kept, the lock screen guards
memory alone and wiped it after five wrong PINs; with one kept, eight.
The owner saw "4 left before wipe" where they had just seen eight and
asked whether it was a different flow. It was, and the difference bought
nothing: three fewer guesses at a PIN that is typed by hand and protects
a key the person holds a backup of. `session::ATTEMPTS` is eight. The
captions still differ — "before wipe" and "before removal" — because
what happens differs, and that is the one thing the pad should say.

### 16.66 One PIN, every key (2026-09-12)

The owner asked whether keys had PINs of their own. They never did: the
first key sets the session PIN and every later key is added under it,
the lock screen takes it, and the stored key opened with the same
digits. What was true is that the blob on the device held exactly one
key. A second key added while one was kept lived in memory only, got no
offer to keep it, and was gone when the app closed — and nothing said
so. The owner's rule is one PIN for everything and keys that can be
removed one at a time.

**The blob holds every key.** Format version 3 (`keep.rs`): the keys
record is one sealed message with room for `MAX_KEPT` keys — a hundred,
51 bytes each, so about five kilobytes — and the same size with one key
in it as with a hundred, so a copy of the file says nothing about how
many keys are on the device. The duress record is unchanged in shape.
Version 2 is not read; nothing is in the field.

**The keys on the device follow the keys in memory.** "Keep on this
device" keeps every loaded key that has words, once each — two keys over
the same words, one behind a passphrase, are one set of words on the
device. From then on a key added, a key forgotten and a backup quiz
passed each rewrite the keys record (`sync_kept`), and forgetting the
last kept key forgets the blob and the element's keys with it. The
rewrite needs no word from the element: the key the record is sealed
under (`Kek`) is kept for the session, sealed under the session key like
every other secret and resealed when the session key rotates, from the
moment the blob is written or opened until the keys go. Keys are in
memory only between those two moments, so the key is always there when
a rewrite is due. Each rewrite draws a fresh nonce from a per-write
derivation of the session key, which is the one thing XChaCha20-Poly1305
requires of a rewrite under the same key.

**What a person sees.** The Keep hold lists every key it will keep. The
key menu says "Kept on this device" for every key with words once the
device keeps anything, and offers "Keep on this device" only while it
keeps nothing. Forget on any kept key says the stored copy goes too.
Opening the app with the PIN brings every key back and lands on Home.
The Learn page says a key added afterwards is kept the same way and a
key forgotten is removed from the phone.

### 16.67 Open a passphrase, open a BIP-85 child (2026-09-12)

The key menu's "Passphrase" screen said whether the key carried one and,
where it did not, offered "Load with a passphrase" — which restarted the
Load wizard from its first step. Getting to the passphrase key of words
already in memory meant choosing the source, the count and the language
again and retyping twelve or twenty-four words. The owner's rule is that
the person types only the passphrase.

**Two rows, one word.** The key menu now carries "Open passphrase", on a
key that holds its words, and "Open BIP-85 child seed" on every key.
"Open" is the owner's word for both: the parent is what is held, and
what comes out of it is opened rather than loaded. Each row is one
screen — an Entry for the passphrase, a Choice and an Entry for the
child's word count and index — and each lands on the opened key's own
menu, with the chevron going back to the key list.

**Opened keys are the session's.** A key opened from another is marked
derived: `kept_keys` skips it, so a device that keeps keys writes the
words and not what was opened over them, and neither "Keep on this
device" nor "Kept on this device" appears on its menu. Forgetting it
removes it from memory and nothing else. The reason is the one the
passphrase exists for: the device holding the words is what a thief
takes, and the passphrase is what stands between that thief and the
funds. Writing the passphrase key beside the words would put both on the
same device and leave the passphrase protecting nothing. A BIP-85 child
follows the same rule for the same reason — it is the parent's key by
another name, and the parent is already kept.

**Which key, and what happened.** The owner's finding on the first
build: nothing said what adding a passphrase or a child seed would
produce, or what it had produced. Both entry screens now carry a fact
row above the field — "Key" and the fingerprint of what the typed input
derives, recomputed on every keystroke — and an empty passphrase or an
out-of-range index states no key rather than the parent's, so nothing
reads as a key to add before there is one. ✓ no longer jumps to the new
key's menu: it lands on a Result stating the key that was added and its
word count, with "Open key" to that menu and the chevron to the key
list. The Load wizard's passphrase step already had its own "Which key?"
comparison and is unchanged.

**The derivation.** `osk-bip::bip85` implements application 39' only:
`m/83696968'/39'/{language}'/{words}'/{index}'`, the derived private key
through `HMAC-SHA512("bip-entropy-from-k", k)`, and the first
`words * 4 / 3` bytes of that as the child's entropy. The language code
is the wordlist's position in `Language::ALL`, which is the order BIP-85
gives (English 0 through Portuguese 9). The intermediate private key and
the hash are zeroized before the function returns. The word-count choice
offers 12, 18 and 24; BIP-39's other two counts are derivable and not on
the screen.

**The vectors.** The BIP's own master key,
`xprv9s21ZrQH143K2LBWUUQRFXhucrQqBpKdRRxNVq2zBqsx8HVqFk2uYo8kmbaLLHRdqtQpUm98uKfu3vca1LqdGhUtyoFnCNkfmXRyPXLjbKb`,
and its three published English children at index 0: 12 words from
entropy `6250b68daf746d12a24d58b4787a714b`, 18 words from
`938033ed8b12698449d4bbca3c853c66b293ea1b1ce9d9dc`, 24 words from
`ae131e2312cdc61331542efe0d1077bac5ea803adf24b313a4f0e48e9c51f37f`.
`core/osk-bip/tests/bip85.rs` checks all three, that index 1 differs
from index 0, and that a word count BIP-39 does not define is an error.

**What a person sees.** "Open passphrase" is the passphrase keyboard
with the masked field the Load wizard uses, the last character visible
for half a second. ✓ is dead until something is typed, because an empty
passphrase is the key that is already loaded. "Open BIP-85 child seed"
asks the word count as a Choice and the index on a digit pad, with the
caption stating the range when what is typed is past the last hardened
child. Opening a key whose fingerprint is already loaded goes to that
key rather than adding a second copy. The key menu is now six rows,
which the two panel sizes cannot show at once, so it scrolls like every
other list.

### 16.68 A wallet is loaded from Keys and has a menu (2026-09-12)

A wallet could only arrive through Home › Scan. Nothing on Keys said a
wallet could be loaded at all, and the owner's finding on the Android
build was the plain one: "I don't see any way to load a multisig." Once
a wallet was in use it became a row on Keys that reopened the review it
came from, and that was all it could do — no addresses, no export,
nothing a coordinator or a second signer asks for.

**Keys offers it.** Keys now ends with "Load a wallet", the §4.14 start
row, present whether or not a key is loaded. It opens the scanner with
"Scan a wallet" as its title, which reads a code or a file. §4.14 gains
the rule the row needs: a start row may also end a list that has a way
to grow, so the row sits under the keys and the wallets rather than
replacing them.

**A wallet has a menu.** A wallet in use is `Screen::Wallet`, a §5 Menu
titled with what the wallet is — "2 of 3 · SegWit" — carrying
**Addresses · Export · Keys · Forget**. Its row on Keys opens it, and so
does "Use this wallet" on a review, which now lands on the menu with
Keys behind it instead of leaving the person on the document they just
accepted. "Keys" opens that review again, without its Forget action:
Forget is a row of the menu. Forgetting a wallet is a tap. §2.7 asks for
a hold where an action cannot be undone; a wallet is public data that
can be read again from the same file, and what the row removes is a row.

**Addresses and export are one screen each, not two.** `Screen::Addresses`
and `Screen::Export` now carry an owner — a loaded key or a wallet in
use — rather than a key index, so one view draws each and there is one
definition of what an address list and an export are. A key's script
type is a choice and its path follows from it; a wallet's script type is
a fact of its template and its keys carry their own origins, so those
rows are §4.12 facts with no chevron. `screens::addresses` takes the
Choice's id as an `Option` for the same reason. The export offers a key
the three forms of its account key and a wallet its checksummed
descriptor or its BIP-388 policy; the animated BC-UR path is the one
that was already there, and a 2-of-3 descriptor is four parts on the
2.8" panel.

**The text file coordinators write.** Sparrow, Nunchuk, Specter and
Coldcard all exchange a multisig wallet as Coldcard's text config —
`Name:`, `Policy: 2 of 3`, `Derivation:`, `Format: P2WSH`, then one
`fingerprint: xpub` per line. `osk-bip::multisig_config` reads it into a
`WalletPolicy` through `from_parts` with a `sortedmulti` template, which
is what every one of those coordinators produces; a `Derivation:` line
applies to the keys after it, so cosigners at different paths are read
correctly. `P2WSH`, `P2SH-P2WSH` (also written `P2WSH-P2SH`) and `P2SH`
are the three forms. The keys may carry SLIP-132 version bytes — the
single-signature `zpub`/`vpub` or the multisignature `Zpub`/`Vpub` and
their P2SH-wrapped pairs, which Coldcard, Nunchuk and Specter write —
and each is read back to `xpub`/`tpub` before the policy is built, since
the version bytes say nothing the `Format:` line does not; one test
reads the same key in three spellings and gets one wallet with one
checksum. The classifier gains `PayloadKind::MultisigConfig`,
so a file and a QR take the same route to the same review.
`tools/vectors/psbt/wallet-2of3.txt` is the same wallet as
`wallet-2of3.policy`, and the tests check they reach one wallet with one
checksum.

**What stays open.** A wallet is still the session's: nothing is
remembered across restarts, which is §15 item 36. Which addresses a
wallet derives is not cached beyond the screen showing them — a
multisig address is three public derivations and a script, cheap enough
that the list is filled as far as it is shown and dropped when the
screen is left.

### 16.69 MuSig2 key aggregation and musig() descriptors (2026-09-12)

The workspace's `secp256k1` is 0.29, pinned by `bitcoin` 0.32, and has no
musig module. §8.4 item 19 therefore had nothing behind it: `policy.rs`
listed `tr(musig(@0,@1)/**)` among the policies it refuses, and a
`musig()` wallet could not be read, let alone reviewed. The aggregation
and address side is now `core/osk-bip/src/musig.rs`. Signing (item 20) is
not in it.

**BIP-327 in house.** `key_agg` is the BIP's `KeyAgg`: `HashKeys` over
the compressed keys, `GetSecondKey`, `KeyAggCoeff` with the tagged hashes
`KeyAgg list` and `KeyAgg coefficient`, and `Q` as the sum of the scaled
points. The points come from `secp256k1`'s `mul_tweak` and
`combine_keys`; the scalars are 32 bytes with addition, negation and
reduction mod `n` written here, because `Scalar` has no arithmetic and
`SecretKey` cannot hold zero, which `tacc` starts at. `key_agg` sorts
nothing — BIP-327's vectors depend on the order given — and `sort_keys`
is `KeySort` for the callers that need it. `AggregateKey` carries `gacc`
and `tacc`, so `apply_tweak` is the BIP's `ApplyTweak` for both the
x-only and the plain form and a signer can pick the state up later.

**BIP-328.** `aggregate_xpub` publishes the aggregate point as an
extended public key: depth 0, no parent, index 0, and the fixed chain
code `868087ca…67e38965`. Unhardened BIP-32 derivation from it gives the
per-address keys, which is what a `musig()` with a derivation of its own
derives from.

**BIP-390.** `parse_musig_expr` reads `musig(KEY,…)` with or without a
derivation, `parse_tr_musig` reads the `tr(musig(…))` around it, and
`script_pubkey_at` builds the output script: the participants derived,
sorted with `KeySort`, aggregated, then BIP-341-tweaked with no script
path. The BIP's rules are enforced as the parse: a derivation on the
`musig()` requires every participant to be an xpub with no range of its
own, every step is unhardened, the wildcard is last, at most one step is
multipath, and a `musig()` inside a `musig()` is not a key. Sorting
before aggregation is why the order the participants are written in does
not change the wallet. `parse_key` in `policy.rs` is private, so the key
expression is parsed here; the extended key still goes through
`xkey::decode_xpub`, so a `musig()` participant is held to the same
strictness as every other extended key the device reads (§16.47).

**Verified against.** BIP-327's own `key_agg_vectors.json` and
`key_sort_vectors.json`, stored verbatim under `tools/vectors/bip327/`:
four valid aggregations, five error cases, and the sort. BIP-328's three
published aggregate keys and synthetic xpubs,
`tools/vectors/bip328/vectors.txt`. BIP-390's descriptors,
`tools/vectors/bip390/descriptors.txt` extracted from the BIP: fifteen
invalid descriptors, all refused, and six valid ones. Three of those six
are built and match the scripts the BIP lists — `tr(musig())` over three
plain keys, `rawtr(musig()/0/*)` over two xpubs at indices 0, 1 and 2,
and `tr(musig(xpub/1,xpub/1)/2)`, which is one key twice. The other
three name shapes this module does not build: a participant written as a
WIF private key, a taproot script tree, and a `musig()` inside a tap
leaf. The test requires those to be refused, so that a wallet the module
cannot build is never read as a different one.

**What stays open.** `Template::MuSig` in `WalletPolicy`, so a `musig()`
wallet registers, exports and shows a review like any other; the Verify
screens' address check against it; BIP-373's PSBT participant, nonce and
partial-signature fields; and signing, which is §8.4 item 20 and carries
the nonce rule of §16.47 — in-session only, sealed in memory,
single-use, never written to any tier. The follow-up needs four things
from this module and nothing else: `parse_tr_musig` for the descriptor,
`MusigExpr::participants` for the review rows (each carries its origin
and its xpub), `script_pubkey_at` for the addresses, and
`MusigExpr::aggregate_at`, whose `AggregateKey` is where a signer's
`gacc` and `tacc` start. Derivation from the aggregate is a plain tweak
of that key, which a signer has to fold into the accumulators itself;
this module does not track it past `aggregate_at`.

### 16.70 A MuSig2 wallet loads like any other (2026-09-12)

§16.69 left the aggregation and the addresses in `osk-bip::musig` with
nothing in the product reaching them: `policy.rs` still listed
`tr(musig(@0,@1)/**)` among the policies it refuses, so the owner's ask
— "I should be able to load a multisig or musig2 and export the
descriptor, browse addresses" — was half answered. `WalletPolicy` now has
a third template.

**What parses.** `Template::MuSig` is `tr(musig(@0,@1,…)/**)` as a
BIP-388 policy and
`tr(musig([fp/path]xpub,…)/<0;1>/*)#checksum` as a plain descriptor;
both forms read to one policy, and each writes back the way it arrived.
The keys are the participants, and every one of them carries an origin,
as BIP-388 requires of a `multi` key and for the same reason: the
coordinator names which master each xpub came from. Two or more
participants, pairwise distinct.

**What is refused, and why.** The derivation belongs to the `musig()`,
not to a participant: BIP-390 derives the addresses from the aggregate
key, so `tr(musig(@0/**,@1/**))` — a participant range — is a different
wallet (one aggregate key per index) and is not one a wallet policy
writes. It is refused as `Error::Derivation`, as are a `musig()` with no
derivation at all and one with any derivation other than `/**` or
`/<0;1>/*`; a `musig()` of one key is `Error::Placeholder`, and a
participant with no origin is `Error::Key`. `policy.rs`'s
invalid-policies test lost its `tr(musig(@0,@1)/**)` line, which is now
valid, and gained those five.

**No quorum.** `quorum()` is `None` for a MuSig2 wallet. "2 of 2" states
a script fact that is not there: the output carries one key and one
signature, and the m-of-n a multisig script holds has no counterpart in
it. What the wallet has instead is a number of keys, so the review's
first row is **MuSig2 · 2 keys** and the menu is titled
**MuSig2 · 2 keys · Taproot**, which fits the 268 dp panel without
truncation. Everything else the screens do follows from the template:
`script_type` is Taproot, so Addresses states Taproot as a fact and
lists the wallet's own addresses, and Export offers the checksummed
descriptor (`tr · 6738736c · #sa59qtsr`) and the BIP-388 policy text.

**What derives.** `script_at` builds the script through
`osk_bip::musig`: `MusigExpr::from_xpubs` is the wallet's participants
with the two chains on the aggregate, and `script_pubkey_at` is the
BIP-341 tweak of the aggregate at that chain and index. Rendering the
descriptor and parsing it back would have worked and is one string
round-trip per address; the constructor is the same expression without
the text.

**Verified against.** `tools/vectors/psbt/wallet-musig.policy` is
`tr(musig(A,B)/**)` over BIP-390's two vector xpubs. The BIP publishes
the aggregate keys of `rawtr(musig(A,B)/0/*)` at indices 0, 1 and 2;
`rawtr` pays to that key directly and `tr` pays to it tweaked, so the
wallet's receive chain is those three keys under the BIP-341 tweak. The
`osk-bip` test asserts both halves — the internal keys are the BIP's, and
the wallet's scripts are those keys tweaked — and the flow test asserts
the three mainnet addresses as strings
(`bc1pn0wx4s…`, `bc1pvax58t…`, `bc1ptfpkgn…`), computed once from the
published keys with an implementation of the tweak written for the
purpose. The vector keys are mainnet, which is the device's default, so
the test sets no network.

**What stays open.** Signing, which is §8.4 item 20. And change
detection: a PSBT states a key origin per output, and for a MuSig2
wallet a participant's origin names its own account key and stops there
— the chain and the index are steps of the aggregate, not of the
participant. So `leaf_of` is `None` for every MuSig2 policy, no output
claims such a wallet, and change of one stays "not verified" rather than
being reported wrongly. What would name the address is BIP-373's
`PSBT_OUT_MUSIG2_PARTICIPANT_PUBKEYS`, which the PSBT reader does not
have, and that is §8.4 item 19's remainder. Verify's address check has
no such gap: it derives the wallet's own addresses and compares, so it
answers for a MuSig2 wallet the way it does for any other, which a test
states.

### 16.71 Six nouns, eight tiles (2026-09-12)

Wallets had no home. A key's menu carried **Addresses** and **Wallet
export**, which are facts about a wallet and not about a secret; the
Keys list mixed key rows with the policies in use and with "Load a
wallet"; Explore kept a second address list of its own; and Settings was
a gear in the status line, the one area that was not a tile. Six nouns
the product is made of — key, wallet, signing, verifying, tools,
learning — had five places between them.

**What the tree is now.** Home is eight tiles, 2 × 4 on a portrait
panel: **Scan · Keys · Wallets · Sign · Verify · Tools · Learn ·
Settings**. Every area is one tap from Home and one row of the `wide`
sidebar, in the same order; the status line keeps the tier badge, the
network badge and the lock, and nothing else. The grid fits the
268 × 358 dp panel without scrolling, and a panel wider than it is tall
draws the same eight in four columns rather than stranding a row below
the fold.

- **Keys is secrets only**: the loaded keys, with "Load a key" and
  "Create a key" as its actions. A key's menu is Backup · Open
  passphrase · Open BIP-85 child · Keep on this device (Tier B) ·
  Forget.
- **Wallets is public data with addresses**: one row per loaded key (its
  own single-sig wallet, script type chosen inside), one row per policy
  in use, then "Load a wallet". A wallet's menu is Addresses · Export ·
  Keys, and Forget for a policy — a single-sig wallet follows its key.
  `Screen::Wallet` takes a `WalletRef`, which is `SingleSig(key)` or
  `Policy(wallet)`, and Addresses and Export take the same reference, so
  one screen draws each kind.
- **Tools replaces Explore as a tile**: Key explorer opens today's
  Explore unchanged, and Word list, Dice passphrase, Hashes, Encodings,
  Descriptor checksum, Convert key and Units are dimmed rows with no
  reason, because nothing a person can do would make them work yet
  (§4.11).
- **Verify answers four questions**: Address · Signed message · Wallet ·
  Software. A wallet read here is reviewed and closed; using it is what
  Wallets is for.

**The glyph rule.** What a person needs to know about a key or a wallet
is whether this device can sign for it, so it is a mark at the start of
every row that names one: the **key glyph** where this device holds the
private key, the **eye** where it holds the public key alone. A 2-of-3
with one of this device's keys carries the key; one with none carries
the eye. The glyphs appear on the Wallets rows and on a wallet's Keys
review, and nowhere on the Keys tile's list, where every row is a secret
already. The MINE badge keeps its one meaning — an output or an address
that pays one of this device's wallets — and is no longer drawn on a key
row. The eye also means "reveal" on the app bar of a screen with a
secret panel; there it is a button, here a marker, and the two never
share a screen. Both meanings are "look" (DESIGN §4.4).

**Decided with the owner**, each as recommended:

1. **Eight tiles, not six.** Six would have meant Tools inside Verify or
   Wallets inside Keys, which is the muddle being fixed. Built in this
   pass.
2. **Single-sig wallets are listed automatically**, one per loaded key,
   with the script type chosen inside rather than at creation. Built in
   this pass.
3. **GPG-signed messages** stay open: no standard derives an OpenPGP key
   from a BIP-39 seed except BIP-85's RSA application, which is slow on a
   Pi 3 and RSA only, and OpenPGP is a large format. Listed under Sign ›
   Message, not built until a concrete use names it.
4. **Encrypted backup** is AES-GCM under Argon2id of a passphrase, as one
   file or an animated QR, under Backup: it needs no new dependency,
   since the kept-key blob already uses both, and a short script can
   reproduce it. Priority 1, a later pass.
5. **Wallets are remembered across restarts** on Tier B: the policies in
   use follow the kept keys into the blob, as public data under the same
   protection, and nothing is kept on a stateless tier. §15 item 36 is
   decided by this; a later pass builds it.

Read-only single-sig wallets from an xpub — the same object with no
private key, loaded from a descriptor or a bare xpub and carrying the eye
— are decided and are built in §16.72. The seven tools are still a later
pass.

### 16.72 A wallet without a key, and wallets that stay (2026-09-12)

Two things were refused that a person had every reason to expect. A
single-key descriptor — `wpkh([73c5da0a/84'/0'/0']xpub…/<0;1>/*)`, what
every coordinator hands out for a watch-only wallet — was shown as a
document to read and could not be used, although a multisig descriptor
one line longer became a wallet. And a bare extended public key was a
document too, because BIP-388 requires an origin on every key and the
device had nowhere to put a key without one. Meanwhile Wallets, now a
tile of its own, was empty at every start on a device whose Keys was not,
which reads as a bug rather than as a policy.

**What parses now.** `PolicyKey`'s origin is an `Option`. `fingerprint()`
answers `None` where there is no origin and never substitutes the xpub's
own BIP-32 identifier, which names the key as a parent and not as a
master; `identifier_fingerprint()` is that identifier, under its own
name. `from_descriptor` accepts an origin-less key only in a single-key
template — `multi`, `sortedmulti` and `musig` still refuse one, because
BIP-388 and BIP-390 write an origin on every key of a policy.
`to_descriptor` writes the key bare, which is a valid BIP-380
descriptor; `to_text` has no BIP-388 form to write for such a wallet and
returns that descriptor instead of refusing, so that the wallet always
has one text a person can write down or a device can store, and
`parse_any` reads it back as the same wallet. `key(fingerprint)` and
`leaf_of` match on the origin, so a wallet with no origin matches no
output and is never used for change or for signing.

**How one arrives.** Three ways, all landing on the same review with
"Use this wallet":

- a single-key descriptor at Wallets › Load a wallet, Verify › Wallet or
  Scan — any descriptor a policy can be built from is now a wallet being
  offered, and only one that is no wallet at all stays a document;
- a bare `xpub`/`tpub` or a SLIP-132 spelling, through a "Which script
  type?" Choice whose check starts on the type the SLIP-132 prefix names
  and on SegWit where the prefix names none (`xpub` and `tpub` are
  BIP-32's own and name no script). The chevron leaves the key as the
  document it was;
- Coldcard's JSON account export, which carries `xfp` and one object per
  single-sig standard. `osk_bip::coldcard` reads the first of `bip84`,
  `bip86`, `bip49`, `bip44` that the file offers. Its `p2wsh` fields are
  a cosigner's contribution to a policy, not a wallet, and belong to the
  wallet builder (§8.3 item 9).

**What it looks like.** On Wallets it is a policy row: the eye where none
of its keys is one of this device's, the key glyph where one is; the
master fingerprint as its label, or the script type's name where there is
no origin to name a master. Nothing says "read-only" or "watch-only" —
the glyph says it. On the review the key row's value is `origin unknown`
where a fingerprint would stand. Export offers the descriptor and, only
where every key has an origin, the BIP-388 policy.

**The blob's new record.** Format version 4 adds a wallets record beside
the keys and duress records: 36,866 bytes of plaintext, a kind byte and a
count over sixteen fixed slots of 2,304 bytes, each one wallet's text
zero-padded, so the stored bytes say neither how many wallets there are
nor how long each is. The bound: the largest wallet this build accepts is
a 15-of-15 multisig, whose template is 115 characters and whose fifteen
keys are at most about 135 each, so about 2,141 bytes, and sixteen
wallets is more than a device registered by hand from codes and files
will hold. The record is sealed under the same key as the keys record and
is rewritten with it whenever a key or a wallet changes — no word to the
element either time — and read back at unlock; forgetting the last key
forgets the blob and the wallets with it, and a wipe forgets both. A
stateless tier keeps nothing, as before: its wallets are the session's.

`keep.rs` may hold no heap text (`tools/lint-secrets.sh` rule 3), so the
wallets' text lives in `keep_wallets.rs`, which the lint does not cover:
that module turns policies into the record's body and back, and `keep.rs`
seals and opens a fixed-length byte array it never looks inside.

**A version-3 blob is not read by this build**, as versions 1 and 2 are
not. `keep::header` refuses it, which is the path bytes the app cannot
read already take: the pad that would ask for its PIN goes, nothing is
kept as far as the app is concerned, and the shell's copy is left where
it is rather than deleted behind the person's back.

**Still open.** The UR key forms (`crypto-hdkey`, `crypto-account`,
Keystone's and Jade's account exports) are out of scope here: they are a
CBOR format with a registry of its own, and reading one belongs with the
UR work rather than with this pass. Swapped-xpub detection against a
previously seen config (§8.4 item 17) is still not built.

### 16.73 Back where nothing is behind leaves the app (2026-09-12)

On the Android build the system Back did nothing on Home, on the lock
screen and on the stored key's pad. The shell forwards the gesture to the
core as its "go back" key, and the core's back at Home went to Home; on
the two pads the gesture was not read at all. The owner's finding: "the
Android app system back button does not work when on the home screen or
PIN pad. It should close the app."

**One rule, three screens.** Back where nothing is behind is the way out
of the app, pressed twice — the Android idiom. The first press shows
"Back again to exit and clear memory" on the screen's caption line for
`LEAVE_WINDOW_MS` (two seconds): on Home the line stands where the
badges do, on the caption line every field reserves, so the grid does not
move; on the two pads it is the line that carries "Wrong PIN · 4 left".
A second press inside the window leaves; the window runs out by itself,
and any screen change closes it. Nothing about the window touches the
lock and wipe timers, which every input restarts anyway.

**Leaving is not "Wipe and exit".** The one exit the app had went
through `exit()`, which forgets the key kept on the device as well as
memory, because it is the action a person holds to give the device's copy
up. A Back is not that decision. `leave()` clears memory — every key,
every cache, the session and its PIN — asks the shell to exit, and leaves
the blob where it is, so the next start opens on the pad with the same
PIN. The terminal screen says which happened: "Exit · Session ended ·
Keys cleared from memory" after a Back, "Wipe and exit · Session ended ·
Keys removed" after the hold.

**Desktop.** Escape is Back, so Escape twice on Home closes the desktop
app after clearing memory. Recorded as intended; it is the same rule.

Four tests state the rule from the person's side: one Back on Home asks
and a second leaves with memory cleared; the window runs out and a Back
on another screen is that screen's Back; Back twice on the lock screen
and on the stored key's pad leaves without a `ForgetSecret`, and the
next start opens on the pad with the PIN.

### 16.74 Two tools: the word list and the dice passphrase (2026-09-12)

Tools listed seven dimmed rows behind the key explorer (§16.71). The two
that were asked for first are built: **Word list** (§8.3 #1) and **Dice
passphrase** (§8.1 #11). Both are standalone — no key, no session state,
nothing written anywhere.

**Word list.** A language Choice, then an Entry whose field is read by
the notation a "Search by" mode row names: the word itself, its 1-based
number, its eleven bits, or its three hex digits. Searching by word is
the Load wizard's own word entry — one `LoadWizard` driven as a
one-word search, so the ten keyboards, the dimmed keys and the candidate
strip are the machinery that was already there rather than a second word
search beside it. A number, binary or hex value the list has no word for
says why on the caption line and leaves ✓ dead.

A word's screen is a Record titled with the word: **Number** from 1 and
**Index** from 0, both labelled, because the app numbers words from 1
everywhere (UX §7.5) while BIP-39 and every other tool counts from 0, and
a screen that showed one of them would be read as the other. Binary and
hex are mono. A §4.1 pager labelled with the word walks the list in
place, so Back from the fortieth word read is the search and not thirty-
nine screens of history. "Browse the list" is the fifth row of the mode
Choice: one fact row is all §4.3's space above a field holds on a 268 dp
panel, so a second row for it would have been drawn under the field.

**Dice passphrase.** The three EFF lists are baked into `osk-bip` the way
the BIP-39 lists are: `tools/diceware/gen.py` reads the files committed
under `tools/vectors/eff/`, checks that each is exactly its rolls in
order, and writes one Rust module per list; a test rebuilds every roll
from the index and compares the word and the digest against the file, so
the digests in that README stay meaningful. `List` answers
`dice_per_word`, `word(rolls)` and `bits_per_word`, and `bits(words)`
rounds down — six long-list words are 77 bits, eight short-list words 82.

The flow is a list Choice, a length Choice whose rows state what each
length is worth, the Create wizard's dice pad with its count and its
sanity checks, and a Secret screen. As each group of four or five rolls
completes, its word joins a masked panel above the pad, so the phrase is
seen to build; the app bar's eye shows it, and a completed word ends the
look the way an accepted word does in the Load wizard. The passphrase
screen states "Entropy · 77 bits" and offers Done. No `StoreSecret` is
ever issued, the rolls live in a zeroizing `DiceRolls`, and Back with any
roll entered asks first, with the Choice Explore uses for typed words.

**What the design system gained**, each because a screen these tools need
had nowhere to put something:

- §4.3's fact row above a field may be a §4.2 mode row that opens a
  Choice, which is what "Search by · Number" is. It is still one row: the
  panel leaves about a row's height above the field and the keyboard.
- §5 Pad may carry the words a run has completed, on the masked panel and
  with the app bar's eye, in the space §2.6 centres — the same panel and
  the same rule word entry already had, on the class that has room for it.
- §5 Secret may carry facts under its panel and one action, and its panel
  may be words rather than one string chunked in fours.
- §5 Record may carry a §4.1 pager in the bottom action slot.
- A `Binary` keyboard: 0, 1, backspace and ✓ in one row of tall keys, the
  coin pad's geometry with the coin pad's two keys renamed.

### 16.75 Home is the list of wallets; Keys, Wallets, Sign and Verify are not areas (2026-09-12)

The eight tiles of §16.71 lasted a day. The owner's verdict on the build
was that the difference between Keys, Wallets, Sign and Verify was not
clear, and the fault was the sorting rule: the grid filed the product by
data type — secrets under Keys, public data under Wallets, verbs under
Sign and Verify, an input method under Scan — and a person does not
think in those categories. They think "the key I loaded" or "my savings
multisig", and sign, check an address, export, back up and forget are all
things done *to* that one object. One seed showed up on three screens.

Every signer in use avoids this one of two ways. SeedSigner, Krux, Jade
and Coldcard load one seed and make its menu the app. Sparrow, Nunchuk,
Passport and Keystone make the wallet the unit, with its keys a setting
inside it. OpenSigner is multi-key, so it takes both: **Home is the list
of what is loaded, and each thing on it has one page with everything you
can do to it.**

**The tree.** Home: the status line; one row per wallet — a loaded key's
own single-sig wallet, a policy in use, a wallet over public keys alone,
each with the key or the eye — or the three start rows while nothing is
loaded; a band of three small tiles, Tools · Learn · Settings; then Add
and Scan, Scan primary. A wallet's page has the same shape for every
kind: the actions, then Key or Keys, then Forget. A loaded key's wallet:
Sign a transaction · Sign a message · Addresses · Check an address ·
Export · Key · Forget. A policy this device holds a key of: the same
without Sign a message. A wallet it holds no key of: no Sign rows. Add is
a menu: Load a key · Create a key · Load a wallet. On `wide` the sidebar
is Home · Tools · Learn · Settings.

**What went.** The Keys list, the Wallets list, the Sign entry menu, the
Verify entry menu, the 2 × 4 grid, the key count badge, and the "Key"
context row on the two entry menus. The key menu lost Forget, which is
the wallet page's row. Verify's four questions went where their objects
are: an address is checked from any wallet's page or by Scan; a signed
message is a tool; a scanned wallet's review ends in "Add this wallet"
or the chevron; the software is Settings › About.

**What stayed.** The Load and Create wizards, the key menu's screens,
the wallet review, the Sign review and result, Addresses, Export, Tools,
Learn, Settings, the lock screen, the two-press Back, Tier B
persistence. The glyph rule of §16.71 stands, on Home's rows and a
wallet's Keys review.

**Rejected.** SeedSigner's flat menu — Backup, Passphrase, BIP-85 and
Keep directly on the seed's page, ten rows, no nesting — because a
policy's page would then have a different shape from a seed's, and
because "Key" inside a wallet is the plainer statement of what a seed is
for: the thing that signs for this wallet.

### 16.76 The wallet builder (2026-09-12)

A multisig or MuSig2 wallet could only arrive from a coordinator. Now
Add › **New wallet** makes one on the device (§8.3 item 9): a script
type — SegWit as `wsh(sortedmulti)` with keys at BIP-48 `2'`, Nested as
`sh(wsh(sortedmulti))` at `1'`, MuSig2 as `tr(musig())` with keys at
BIP-86, the account the BIP-390 vectors use — then the keys, then the
threshold, then the same review a loaded wallet gets. Legacy `sh(multi)`
and Taproot `multi_a` are dimmed rows: the policy code has no account
path for the first and no template for the second.

A key comes from two places. One of this device's loaded keys
contributes its account key for the script type, written with BIP-388's
apostrophes so the checksum matches a coordinator's policy. A scanned
key is read from a key with its origin in descriptor notation, a
single-key descriptor, a Coldcard JSON export's `p2wsh` / `p2sh_p2wsh` /
`bip86` account (its SLIP-132 spellings converted through the strict
decoder), or a bare xpub, which arrives with no origin and so makes a
wallet with a descriptor but no BIP-388 form. A key on the other
network, a key already in, and a text that is a whole wallet are refused
on the scanner's reason line. The scanner opens over the wizard and
returns to the Keys step, so a scanned key does not end the build.

MuSig2 participants are written in BIP-327 `KeySort` order, so the same
keys gathered in any order make the same wallet; the checked-in
`wallet-musig.policy` fixture writes them in the BIPs' vector order and
is the same wallet under a different checksum. The result is an
ordinary `WalletPolicy`: on Home with the glyph, kept across restarts on
Tier B, exported as a descriptor and a policy, verifying change in a
transaction like a loaded one. A test builds the `wallet-2of3` fixture
from the test key and its two cosigners and compares checksums.

### 16.77 A swapped key is named on the review (2026-09-12)

Job E2's last clause. A review built by `wallet_doc` — from Scan, Add ›
Load a wallet, a coordinator's config, a Coldcard export or the builder
— compares the policy against every wallet in use with the same
template and key count. Exactly one differing key is a swap: the review
carries a danger card, "Differs from the wallet in use", naming the
wallet in use as Home names it and the change — "Key b8688df1, same
fingerprint, different key" when only the xpub changed, "Key X replaced
by Y" when the origin changed, "replaced by a key with no origin" when
it went — and the changed key's row is in the caution tone. Two or more
differing keys is a different wallet and gets no card. A single-key
review whose origin names a loaded key's master is checked against what
that master derives at the claimed path; a mismatch says "Claims to be
73c5da0a and is not".

With a danger card present, "Add this wallet" is a hold rather than a
tap (§2.7 of the design system), on the scanned review and on the
builder's. `Action` gained a `hold` form for it, and a glyph row a tone.

### 16.78 The encrypted backup (2026-09-13)

§16.71 decision 4, built. A key's words under a passphrase, as one file
or one QR, under Backup › Encrypted backup; read back by Load a key ›
Encrypted backup or by Scan, which asks the passphrase and lands the Load
wizard at its confirm step. The decision named AES-GCM; the code base
already carries XChaCha20-Poly1305 and Argon2id for the kept-key blob,
so the backup uses those two and no new crate.

The format, `osk-backup` version 1, is 73 bytes of header plus the
sealed plaintext: `OSKB`, a version byte, the three Argon2id costs, a
32-byte salt (bytes 0..49 are the AEAD's associated data), a 24-byte
nonce, the ciphertext and its tag. The plaintext is the word count, the
language's index in `Language::ALL`, and the entropy — 107 bytes for
twelve words, 123 for twenty-four. A passphrase key backs up as its
parent's words, and the Result says so; the wallet passphrase is never
in it. The Argon2id costs are the device's own (`keep::DEVICE_PARAMS`),
and the salt and nonce come from a per-session counter off the session
key, so two backups never share either.

The format is not this app's secret: `tools/backup/decrypt.py`, ninety
lines over the standard library and either `argon2-cffi` with `pynacl`
or `cryptography` 44+, reads one back, and `tools/vectors/backup/` holds
a fixed vector both implementations must open to the same words, which
the test runs when the libraries are present and skips otherwise.
Nothing about it is installed or fetched.

Two contract changes came with it: `Command::WriteFile`'s name hint is
an owned string, since the name carries the fingerprint; and the QR
screen's Animated row is optional, since a backup is one static code on
every class. A Learn page, "Encrypted backups", says what it is, that the
passphrase is the backup, what a good one is, and how it is protected.

### 16.79 The six tools (2026-09-13)

§8.3's calculators, built as Tools rows: Hashes, Encodings, Descriptor
checksum, Convert key, Units and Decode a transaction. Each is a
standalone calculator over `osk_crypto`, `bitcoin`'s hashes and base58,
the `bech32` crate `bitcoin` re-exports (`osk_codec::encodings` wraps
the decoders and encoders), `osk_bip::descriptor` and `slip132`, and
`osk_psbt`'s inspector; no crate was added. Decode runs the Sign review
in a reading mode: no Confirm step, Done on the last page, this
device's keys and wallets as the context when any are loaded so a
transaction paying one of them shows MINE and verified change, and with
none the change row says "no key loaded" in the muted tone while the
two warnings that only mean something with a key are not raised. Units
takes a whole number of the chosen unit on the digit pad; the other
three units follow live beside the field on `mobile` and `wide`, and ✓
opens them as a Record on every class, because the 268 dp panel has
one row above a field. Correctness is tested against the SHA-256 and
BIP-32 vectors, the BIP-173 and BIP-350 strings, the BIP-380 checksum
vectors and SLIP-132 conversions of the BIP-32 vector-1 key.

### 16.80 Miniscript and Taproot tree wallets, through rust-miniscript (2026-09-13)

A `wsh(<miniscript>)`, `sh(wsh(<miniscript>))` or `tr(<key>, <tree>)`
descriptor — what Liana, Sparrow and Nunchuk make for timelocked
recovery — is a wallet: loaded from Scan, Add › Load a wallet or a
file, reviewed, on Home with the glyph, with Addresses and Export, kept
on Tier B, and recognised in a transaction's inputs and change. Signing
its inputs is the next entry.

**The dependency.** This is the one place the project takes a large
crate rather than writing the format. Miniscript's type system, script
encoding and satisfaction rules are each a specification, and a
home-grown version would be wrong where it costs money: the address a
person is shown and the signature a spend needs. `miniscript` 12.3.7,
`no-std` and `compiler` features, brought exactly one new crate into
the graph (its two dependencies were already there); `docs/deps/
miniscript.md` records the reasoning, the features and the cost, and
`osk-bip` builds for `wasm32-unknown-unknown` with it, so the core stays
`no_std`.

**What was built.** `Template::Miniscript` and `Template::Tree` on
`WalletPolicy`, parsed when no existing template matches, with the same
methods the other kinds have and a BIP-388 form with placeholders; a
key with one chain only is refused. `osk_bip::spend` lifts a
miniscript to its semantic policy and lists the spend paths — keys plus
`older`, `after`, time and preimage locks — with block counts stated as
blocks and as an approximate duration, capped at 64 paths. The review's
first row is the kind, then a row per spend path, then the keys
lettered A, B, C. Tools gained **Miniscript**: a typed concrete policy
compiled to a `wsh` or `tr` descriptor, its paths listed, and "Load as
wallet" when every key is an origin-bearing xpub. A Learn page, "Spend
paths and timelocks", says what a path is, what `older` and `after`
mean, when the clock starts and who chooses the path.

**Checked against.** Two fixtures written by the PSBT example — a
Liana-style `wsh(or_d(pk(A), and_v(v:pkh(B), older(52560))))` and a
`tr(A, {and_v(v:pk(B), older(4320)), pk(C)})` on regtest — with every
receive and change address and both checksums matched against
rust-miniscript used directly and against Bitcoin Core 29.4's
`deriveaddresses` on a throwaway local regtest node, recorded in the
vectors README. One existing test moved: a `wsh(and_v(v:pk, older))`
that was asserted refused is now a wallet, and the test encodes the
rule that still holds.

### 16.81 Signing miniscript and Taproot script-path inputs (2026-09-13)

The other half of §16.80. A `wsh` or `sh(wsh)` miniscript input gets an
ECDSA partial signature per key origin naming one of this device's
masters, over the BIP-143 sighash with the witness script as scriptCode,
membership checked through the parsed miniscript's key set; a Taproot
input gets a BIP-341 script-path signature per leaf in `tap_scripts`
whose script names a key we derive from an origin claiming that leaf,
under `(xonly, leaf_hash)` in `tap_script_sigs`, and the key path as
well when the internal key is ours and the tweaked output matches.
Every signature is verified before it is written. Finalization of these
inputs goes to rust-miniscript's PSBT finalizer, which is available
under `no-std`; its interpreter check is what leaves a PSBT partial when
a cosigner is missing or a timelock is not met.

The review names the route: a new `SpendRoute` per input — the leaf the
PSBT carries, the key path when only the internal key is claimed, or for
`wsh` the wallet's spend path whose keys the PSBT states origins for,
choosing among several the ones the sequence and locktime satisfy and
then the longest wait — and a `TimelockNotMet` caution names the path
and the lock when the transaction's own fields do not meet it. This
device knows no chain height, so that is the only judgement it makes.
`UnsupportedInput` now fires only for an unknown script or a Taproot
tree the input gives no way into.

Checked against Bitcoin Core 29.4 on a throwaway local regtest node,
recorded in the vectors README: the Liana wallet spent through `pk(A)`
and through its recovery path after 52 560 confirmations, and the tree
wallet by its key path and through the `pk(C)` leaf, every finalized
transaction accepted by `testmempoolaccept`, and the recovery spend
refused as `non-BIP68-final` before the wait had passed, matching the
device's own caution. One rust-miniscript defect is worked around:
`substitute_raw_pkh` rebuilds only the root node, so the key-set walk is
our own.

### 16.82 Three backup helpers: the SeedQR grid, steel numbers, Seed XOR (2026-09-13)

§8.2 items 5a, 6 and 14, under a key's Backup menu. **Draw a SeedQR**
draws the SeedQR or CompactSeedQR as a grid a person copies by hand: a
new `qr_grid` component over the same matrix the QR screen encodes,
modules as squares, a thick guide every five modules counted from the
symbol's origin, row and column labels in a gutter; on `small` it pages
by quadrant with one shared guide band between neighbours, elsewhere
one page. **Numbers for steel** is the Words screen as `0001 · ABAN ·
abandon` rows, the two forms a numbered plate and a letter-punch plate
take, and on `wide` "Print template" writes a blank numbered table
through the file channel, never the words. **Split into parts** is Seed
XOR, Coldcard's scheme: 2, 3 or 4 parts of the key's entropy, each a
valid mnemonic and a real key with its own fingerprint, all but the last
drawn from the session key's entropy and the last the XOR of the rest;
the Result names each part's fingerprint so a part can be checked later
by loading it. Create a key gained "Seed XOR parts", which runs the word
entry once per part and hands the combined entropy to the passphrase
offer, skipping the quiz, since the key already existed.

Coldcard's published examples were not reachable offline, so
`tools/vectors/xor/` holds vectors constructed from the definition and
says so; the test packs the 11-bit indices, checks the checksum and folds
the XOR itself before comparing. A Learn page, "Seed XOR", says a part
is a real key that pays to real addresses and that XOR is not Shamir.

### 16.83 Four more entropy sources (2026-09-13)

§8.1 items 3, 4, 5 and 7, as rows of Create a key's source Choice.
**Playing cards**: a 52-card deck on a card pad of thirteen ranks and
four suits, a tracker that refuses a card already drawn, each draw
worth log2 of the cards remaining, so 128 bits is the first 25 draws and
256 bits needs a second shuffled deck, which the pad asks for; the
entropy is SHA-256 over the draws' indices, as dice are hashed.
**Camera noise**: the scanner turned into a viewfinder with a shutter,
three frames at 128 bits and six at 256, each frame's luma hashed and
the digests hashed together (osk-crypto has a one-shot SHA-256 and a
streaming state could not be zeroized like the rest of the crate); a
frame with fewer than 32 distinct luma values is refused as too little
variation, and the Result states the pixel count, the distinct values,
the mean and the variance as facts. **The device's randomness**: the
shell's answer to `RequestEntropy`, shown as a Result whose title is
"Not checkable", naming the source — the operating system, a trusted
execution environment, or a separate security chip — with a "trusts
this device" caution badge; on Tier D the row is dimmed as "browser".
**Mixing**: two to five of the sources as toggle rows, each source's own
steps in turn, then a Result listing every source's commitment — the
SHA-256 of its raw input, as a reference row — and the key is SHA-256
over the commitments in order, so a person who wrote the commitments
down first can check that no source changed after the words were seen.
A Learn page, "Where randomness comes from", states what each source
trusts. The ♠♥♦♣ glyphs were baked into the fallback face. The pad's
caption line gained a muted form for a state that is not a fault, which
is what "Deck 1" is.

### 16.84 Wallet names, and the address keyboard (2026-09-13)

**Names.** A wallet's page opens with a Name value row; the name is
typed on the passphrase keyboard, 24 characters at most, and an empty
✓ clears it. On Home a named row shows the name over its shape and the
fingerprint or checksum leaves the row, since the page states it; the
page's title stays the shape or the fingerprint, so the identity is
never only a typed name. A coordinator config's `Name:` line and a
Coldcard export's name become the name on load. On Tier B a policy's
name rides in its slot of the wallets record as a `name=` first line,
and a slot without one is an unnamed wallet, so an existing blob opens;
a single-sig wallet's name takes a spare slot of the same record keyed
by fingerprint, because the keys record has no byte to spare without
changing its size. A name never appears in an export.

**The address keyboard.** DESIGN §4.3's bech32/base58 layout, built:
the opening layer is the bech32 alphabet on QWERTY's places with `1`
and `b` for the human-readable part and the separator, shift is a
sticky layer key to the base58 alphabet with its own case key, and the
bottom row keeps shift, backspace and ✓ in the same four places on
every layer. A key is live only when what is typed plus that character
can still become an address on the set network: the network's own
prefix, then the alphabet within its length limit. ✓ stays dead until
the text parses. Verify's "Type an address" is the one place an
address is typed.

### 16.85 The first run, and "Try it" from Learn (2026-09-13)

Job A4. A ninth persisted setting, `first_run_done`; until it is set,
Home with nothing loaded is the **Start here** Document — the words,
the backup plan, what this device is, what happens next, four short
sections kept as a Learn page so the mirror and the sync check cover
it — with Continue opening Add and the chevron setting the flag. On
Tier D nothing persists, so it is every start. A key created during the
first run has no Skip on its quiz and lands on a "Key created" Result
whose second action, "Check an address", opens the key's Addresses,
and leaving that list sets the flag; a key loaded sets it by itself,
since a person loading a key has a backup already. Settings › Start
here reopens the page without touching the flag. The network is never
changed: practising on signet is a Settings choice the page names in one
sentence.

Every Learn page whose subject has a flow ends in one row that opens it
("Try it": Create a key, Roll dice for a key, Verify a backup, Make an
encrypted backup, Open a passphrase, Check an address, New wallet,
Miniscript tool, Export, Sign a message), going to Add when the flow
needs a key and none is loaded. The design system's Document may now
end in one Menu row and carry one bottom action; nothing else about it
changed. A settings file from an earlier build has no `first_run_done`
line and reads as unset, so such a device shows Start here once.

### 16.86 The block above an entry field, panel column fit, and the checksum result (2026-09-14)

Pass A of the 2026-09-13 whole-app review: two rendering bugs, two
sizing rules, the type ramp, one screen and a tail of wording.

**The block above the field comes before the keys.** §4.3 used to let
Verify's keyboard alone drop to the 36 dp key floor, because the typed
address needs the room. Generalised: an Entry whose reserved block — a
fact row, a mode row, the typed address — does not fit above the field
at the class's key height drops its keyboard to the floor, and where it
still does not fit the row is drawn on one line, label then value, both
at the label size. Nothing is drawn over the field on any class. The
Hashes and Policy entries on the 268 dp panels were drawing their mode
row under the field and losing half of it.

**A words panel measures its columns at the scale it draws at, and
against the column it is drawn in.** The widest row was measured
unscaled while the panel drew at the class's type scale, and on `wide`
against the 720 dp pane while the panel is drawn inside the 480 dp
column cap. The fallback when two columns do not fit is one column, not
no panel: a panel takes two columns where the width allows them at the
class's monospace size, one where it does not, and is left off only
where even one column is taller than the space it sits in — which is
what the 240 dp panels hit. The Words screen pages a one-column list as
it pages any other; the Entry and Pad panels keep one row per word of
the secret in that column. Each row is also capped at its column's
share, so a row still too wide drops a monospace step rather than
crossing the panel's edge.

Two panels that the old shape test drew over the field are now left off
by the height test: a 24-word entry panel in the desktop window (496 dp
of panel against 289 dp of space) and a 12-word one in a Japanese or
Italian list, which needs a column of twelve 40 dp rows that no class
has the height for above an entry group.

**A line steps down the ramp from its own size.** The one-line shrink
started at the body token, so a line already set below it — the steel
rows' monospace — never shrank and was cut instead. It now starts under
whatever size the line is set in.

**Keycap sizes are tokens** (`KEYCAP_PAD`, `KEYCAP_LETTER`,
`KEYCAP_DENSE`) and `tools/lint-tokens.sh` covers
`core/osk-ui/src/widgets/` as well as the components and screens, so the
widgets' key heights, pad columns, key units, switch, tile and chunk
numbers are names in `tokens.rs` like every other dimension.

**§3 lists what the code sizes.** `DISPLAY` (26) was named in §3 and
used nowhere: the token and the word are gone. The four monospace sizes
that are live — the words panel's 20, the numbered panel's 15, the steel
row's 14 and the dense chip's 13 — and the comparison ladder from 32
down to 8 in twos are named there in a sentence each.

**The descriptor checksum result states a verdict.** It showed the
descriptor row and "Wallet · no". It now opens with "Checksum · valid",
"Checksum · wrong" or "Checksum · missing", then the checksum this
device computed on its own row, then the descriptor reference row, and,
where the descriptor is one this device can derive from, the §4.13 row
action "Load as wallet", which opens the wallet review. The "Wallet"
fact row is gone; a row that offers to load it says the same thing and
is a way on.

**Wording.** One vocabulary for the key a passphrase or a BIP-85 index
opens: the Result is titled "Passphrase key" or "BIP-85 child", says
"Added", and its action "Open wallet" opens the new key's wallet page
rather than its key menu. Camera noise is titled "Camera noise" and its
button keeps the verb. Forget key reads "Its wallet · removed with it"
and, per policy, "2 of 3 · SegWit · stays, can no longer sign". The
miniscript review numbers its spend paths, "Path 1", "Path 2". The Units
result opens with the amount as it was typed. The device-randomness
result states "Trust · this device's software" as a caution-tone row
instead of a badge. Learn page titles are sentence case, and Learn ›
Verifying names the route that exists: open the wallet on Home, "Check
an address", then scan or type.

### 16.87 A ceiling on the comparison ladder, 24 dp on the phone (2026-09-14)

Pass B of the 2026-09-13 review: long strings on the phone.

The comparison ladder runs from 32 dp down to 8 in twos, and a string
takes the largest rung that fits the block it is given. On `small` the
block binds — an xprv lands at 18 to 24 dp, a seed at 26 to 28 — but on
`mobile` nothing binds, so every long string reached the top of the
ladder: 35 dp drawn, 92 px, twice body text and 1.75 times the words
panel beside it. An address, an xprv and a typed address were all
headlines.

The ladder now has a ceiling as well as a floor, per class, named as
`tokens::comparison_max`. On `small` and `wide` it is the top of the
ladder, as before. On `mobile` it is 24 dp: the phone is read at arm's
length beside body text at 16 and a words panel at 20, and a rung above
24 stops reading as a string to compare. The rule is one sentence in §5's
Transcribe row and §4.10's long secret row, and §3 says where the ramp
stops.

One ladder and one ceiling for every long string. A secret is not sized
differently from a public address: what sets the size is that a person is
reading the string character by character, which is the same work either
way.

The ladder serves the Secret screen, Compare, the Address screen and
Verify's typed address, so all four change together on the phone and
none of the 240, 480 or 960 dp renders moves.

### 16.88 The clipboard, and scan as the way in everywhere (2026-09-14)

Pass C of the 2026-09-13 review: input was keyboard-only where it should
not be. Six tools took a string and offered no way in but a keyboard —
64 hex digits, a 111-character extended key, a 450-character descriptor —
and no shell had a clipboard at all, because a rule written for secrets
had been applied to everything.

Scan is now the default screen for every input. The ways in are a band
under the square — §4.1's tile idiom, the one Home draws over its
actions — holding "Read a file", "Paste" and "Type". Paste takes the
clipboard through the same router a scan uses, so a pasted payload
reaches the screen a scanned one would. Type opens the keyboard the flow
behind the scanner has for the value, and is absent where it has none.
On a shell with no camera the same screen says "No camera" in the square
and the band stays live, so a device without a camera is not a device
without an input.

The band rather than three rows, because of what three rows cost. At 268
dp three 52 dp rows left the viewfinder about 105 dp square and the state
line crossed the brackets: the screen a person aims a camera by had
become a menu with a small picture at the top. One band is the height of
one tile whatever it holds, so the square keeps most of the pane's width
on every class, and a scanner with two ways in holds two equal tiles. A
way this shell cannot serve is a dimmed tile and nothing else: dimming
already says unavailable, and the state line inside the square still says
"Nothing to paste" after a tap on a live Paste that found none.

A refused secret is not an unusable payload, and the screen says so
rather than reporting an error: the title is "Refused", the coloured line
"Not pasted", and one row names where it came from and what it was —
"Clipboard · a secret", or "QR · a secret" for a scan refused under a
calculator.

The flows that changed: the five calculators that take a string (Hashes,
Encodings, Descriptor checksum, Convert key, Miniscript), Check an
address (whose two-row menu goes; the wallet page's row opens the
scanner), the wallet builder's "Scan a key" (whose Type row is a new
entry for an xpub or a `[fp/path]xpub` origin), Seed XOR parts (each part
arrives as a seed code or is typed), and Explore's "Words". Sign, Verify,
Decode and Load were already scanners and gain the rows.

`scan::Expect` gained one variant per calculator, and the router reaches
them: the payload's text becomes the field, and the answer opens where
the tool can work one out. Where it cannot, the Type entry opens with the
text in the field and the inline error under it, which is how a person
sees what was wrong. The Unknown QR result gained "Hash it" and "Show
encodings" beside the two ways it already had.

Secrets keep the rule the old comment over-applied. A payload that
classifies as words in the clear, a seed code, an extended private key or
a private key is refused on the way in from the clipboard, and refused
under every calculator whatever it arrived on; the scanner opened for
seed words has no Paste row at all. Nothing the mask policy calls a
secret carries a Copy row.

Copy is a row action for public strings: on the Compare screen, on Wallet
export, and on the Sign, Message and Encrypted backup results. The
Address screen has none, because §4.5 keeps the whole address on the
screen at the largest size that fits and a 268 dp panel has no room for
both; the address is copied from the Compare screen its reference row
opens, which is the same string.

The shell contract gained four events and two commands
(`RequestClipboard`/`Clipboard`/`ClipboardUnavailable`,
`WriteClipboard`/`ClipboardWritten`/`ClipboardNotWritten`) and one
`FileKind::Text`. Both are emitted only for a tap. A read that brings
nothing back is one event for all three of its causes — nothing on the
clipboard, no clipboard here, content that is not text — because the
screen says the same thing about all of them: "Nothing to paste", on the
state line, for two seconds. A write that does not happen is the one
answer that says the shell has no clipboard, and from then on the Paste
and Copy rows are dimmed with "no clipboard".

### 16.89 The desktop shell's clipboard is the platform's own tool (2026-09-14)

The desktop shell reads and writes the clipboard by running
`wl-paste`/`wl-copy`, `xclip` or `pbpaste`/`pbcopy`, in that order, and
takes no clipboard crate. `arboard` and `copypasta` were both considered
and declined: each links a window-system stack of its own beside the one
winit already owns, inside a process that holds seed words, and each
keeps a clipboard connection alive that outlives the tap. The tools are
already on the machines this shell runs on. A session with none of them
is a session with no clipboard as far as the core is concerned, which is
a state the screens already state. The decision is written down beside
the other dependency decisions, in `docs/deps/clipboard.md`.

### 16.90 One meaning per glyph (2026-09-14)

The icon set had 45 variants, twelve of them unused, and a dozen glyphs
carrying two or three meanings at once: the wrench marked the Tools tile
and all six calculator rows, the pencil marked four different rows, the
key glyph was both §4.4's "this device can sign" marker and a plain
bullet on four menu rows, and "Load a key" was a plus in Add and a key in
Explore. The rule adopted (DESIGN §3, §4.4): a glyph is either a fact
marker — the key and the eye — or the identity of one row, never both;
the fact markers appear only where §4.4 puts them; within one menu no two
rows share a glyph; a label that appears in two menus carries the same
glyph in both; and a severity mark on a result is not a row bullet.

The set is now 61 variants, every one of them drawn somewhere. What
changed: Home on the sidebar is a house; "Load a key" is a download
everywhere it appears; "Load a wallet" is the scan glyph, which nothing
else used; "New wallet" is layers; the two message rows are a closed and
an opened envelope; "Decode a transaction" is a receipt; "Export" is
file-export; a wallet's "Key"/"Keys" row is the fingerprint; "Open BIP-85
child" is the derivation glyph; "Keep on this device" is a drive;
"Duress PIN" is a masked figure; "Start here" is a flag; the Backup menu
is eight distinct glyphs; the six calculators carry what each works on;
Explore's "Words and bits" is a superscript; and Addresses' "More" is an
ellipsis. SLIP-132 is a value row, so it carries no glyph at all, which
made `Row::Dimmed`'s icon optional.

Font Awesome 6 Free Solid as vendored has no `file-shield`, so the
encrypted backup carries `vault` instead. The keyboard's ✓ key has its
own `Done` variant (Font Awesome's check) so that `Icon::Check` is only
the choice marker, which is what the comment over the trailing-mark code
has always claimed. Removed as unused: CheckboxOff, CheckboxOn,
ChevronDown, Plus, Restart, Change, MicroSd, Camera and Coin. The gallery
lists the whole set, one row per glyph with its name, at the end of the
navigation page.

### 16.91 Names and structure (2026-09-14)

The 2026-09-13 review's §5: rows that read by the wrong line, labels that
named the wrong thing, and a flow filed under the wrong verb.

**What a wallet is called.** A wallet row's bold line is now what a
person calls the wallet — its name where it has one, else the
fingerprint of a loaded key's single-sig wallet or the shape of a policy
("2 of 3 · SegWit", "Miniscript · SegWit", "MuSig2 · 2 keys · Taproot").
The muted label above it is the kind and the network: "single-sig ·
mainnet", "multisig · regtest", "miniscript · regtest", "MuSig2 ·
mainnet". The descriptor checksum is gone from Home; it stays on the
wallet's page and its export, which is where it is compared. The wallet
page's title is the row's bold line, so a loaded key's wallet is titled
by its fingerprint rather than "Wallet 73c5da0a", and the swapped-key
card and the Forget table name a wallet the same way.

**No "Name · none".** A named wallet keeps its Name value row first. An
unnamed one has no Name row at all; the way to give it one is a "Set a
name" row after Export, which becomes the value row at the top once a
name is typed. The row id is unchanged, so the field it opens is the
same.

**Seed XOR is called that, and combining is a load.** The Backup row is
"Split with Seed XOR". Combining parts moved from Create a key's sources
to Load a key's, after "Read an encrypted backup", because what it gives
back is a key that already existed. Create's source rows are seven, and
the combine runs the same word count, wordlist and part entries it
always did, ending on the same confirm step.

**The builder.** "Add a key" is the last row of the Keys list rather
than an action, so an empty Keys screen is one row (§4.14) and Continue
stands alone. "Which key?" is a Menu of the loaded keys not yet in with
"Scan a key" last, where a tap is the answer: no check, no Continue.
Explore's "Which key?" stays a Choice, having no action row. The review
of a wallet built here drops the checksum row and the count of keys:
nothing was given to this device to check, and the key rows follow. The
script list reads Legacy · Nested · SegWit · Taproot · MuSig2, so the
four script types are in the single-sig order and the two Taproot forms
sit together; Legacy and Taproot multisig are dimmed with no reason.

**Typed words are a key.** Explore's Addresses row is live over typed
words and derives from the words and passphrase the screen holds. The
list is one of Explore's own screens: leaving it for anywhere but
Explore zeroizes the words and drops the addresses they derived.

**Names.** Add is "Load a key · Create a key · Scan a wallet · Build a
multisig", and the Learn page that ends in the builder says the same.
Settings' "Session key" row is "Memory key", which is what it reports:
the health of the key that encrypts what is in memory.

### 16.92 A Seed XOR split's random parts come from a source the person chooses (2026-09-15)

**Why.** `make_split` used to derive every random part of a split by
HMAC over the session key, which is the shell's 32 bytes stretched. The
parts therefore rested on the platform's generator, and neither a screen
nor Learn said so. Someone who rolled dice for their key because they
do not trust that generator was made to trust it for the split without
being told — and if a random part could be predicted, the last part
alone gives up the key.

**The rule.** A split's random parts are made exactly as a key's entropy
is made in Create a key, from a source the person chooses, and the
split's result names that source. The device never picks the source and
never derives a part from the session key.

**The flow.** Backup › Split with Seed XOR: "How many parts?" as before;
then "Random parts from?", a §5 Choice with `SOURCE_ROWS`' rows in
Create's order, Create's row strings, Create's dimming and Create's
caution on "This device", under ids of its own; then, once per random
part in part order, the chosen source's own Create screens at the key's
strength and wordlist — no word-count step, no language step, no "show
the math", since a split is not making a key. Then the parts in turn as
Words screens and the Result, which gains `Random parts · <source>`
before the fingerprints and, for This device, the Trust row the Device
step shows, in the caution tone.

**The caption scheme.** One scheme for every source: the part goes in
front of the screen's own title, `Part 1 of 3 · Roll 12 of 50`, the way
a typed part's entry already reads `Part 1 · Word 3 of 12`. Putting it
in a caption would have worked on the three Result screens and failed on
the pad (whose caption line is the card deck's), the hex entry (whose
caption line is the reserved error line) and the viewfinder (whose state
line is the frame count), so no single caption slot exists across the
seven sources.

**Structure.** `BackupFlow` holds a `CreateWizard` in a gathering mode,
the way `combining()` already bends that wizard for Load. The gatherer
is built by `CreateWizard::gathering`, starts at the source's own first
step, knows which part it is making, and ends at `Step::Words`, where
the flow takes the mnemonic's entropy as that part and drops the wizard.
No view, id or string is duplicated: `build_screen` draws the gatherer
with `view_create`, so the entry, sanity, camera, device and mix screens
and their tests and snapshots cover both. `OpenSigner::gatherer` is
where the camera, the shutter and the entropy answer reach the wizard,
whichever flow holds it. Back inside the gatherer takes its own steps
first, then the source choice for the first part and the previous part's
entry — empty, since a taken part's entry is zeroized — for a later one.

**Deleted.** The session-key derivation in `make_split` and the
`splits_made` counter it was salted with. `make_split` still unseals the
key's words, so the XOR of the key with the random parts stays in
`lib.rs`; the random parts now come from the flow.

§16.82's description of the split stands as history.

### 16.93 The Pi kernel is what the device needs and nothing else (2026-09-16)

**What was wrong.** `common/linux.fragment` said `# CONFIG_MODULES is not
set` and a comment claiming that this dropped the several hundred drivers
the board defconfig builds as modules. It did the opposite. Kconfig turns
a `=m` into a `=y` when modules are off, so the `bcm2709` defconfig's 1203
`=m` lines became 1203 built-ins. The kernel that shipped carried the
whole network stack — IPv4, IPv6, netfilter, the wireless and Bluetooth
stacks, every Ethernet and USB network driver, and the board's own
Broadcom Wi-Fi driver — along with sound, DRM, USB gadget, DVB, infrared,
joysticks, IIO, and ext4, btrfs, xfs, f2fs, NFS and CIFS. §11.2 and the
security review's model both say this kernel has no network stack. Until
now that was false.

**The rule.** The Pi kernel contains the drivers the board, the panel, the
touch controller, the camera and the card need, the framebuffer, evdev,
V4L2, the initramfs, devtmpfs, FAT and the serial console, and nothing
else. No option is built in because a defconfig listed it as a module.

**How.** `external/external.mk` rewrites the board's defconfig in the
unpacked kernel tree at `post-patch`, before Buildroot reads it: every
`=m` line is deleted and `CONFIG_MODULES` is turned off there, so a
tristate nobody names resolves to `n` instead of `y`. What the device
needs is then named, with a reason per line, in the three
`linux.fragment` files, and what it does not have is named the same way.

**The check.** `image/check-kernel-config.sh` holds two lists — 52 options
that must not be built and 22 that must be — each with its reason, and
`build.sh` runs it after `make linux-configure` and before the kernel is
built. A forbidden option fails the build with the offending line. Fed
the old configuration it reports 45 violations and exits 1; fed the new
one it passes. The required list matters as much as the forbidden one: a
driver that silently loses a dependency when the subsystem above it goes
out fails here rather than on the owner's desk.

**Four options did not go quietly**, and each one is a rule about how
Kconfig works rather than about this board.

- `CONFIG_NET` was turned back on after the fragments, by Buildroot
  itself: `linux.mk`'s config fixups enable it whenever
  `BR2_ROOTFS_DEVICE_CREATION_DYNAMIC_MDEV` is set, because mdev in
  daemon mode reads uevents from a netlink socket. This image runs
  `mdev -s` once from `rcS` and has no hotplug helper, so the device
  creation mode is now `DYNAMIC_DEVTMPFS` and the nodes come from the
  kernel, as they already did.
- `CONFIG_HID` is selected by `USB_HID`, which defaults to `y` as soon as
  USB and INPUT are both on. USB is on for the webcam, so `USB_HID` is
  off explicitly.
- `CONFIG_CGROUPS` is selected by `SCHED_AUTOGROUP`, which groups a
  desktop's session processes. It is off.
- `CONFIG_DVB_CORE` has no prompt at all: it follows
  `MEDIA_DIGITAL_TV_SUPPORT`, which defaults to `y` whenever the media
  filter is off. The filter is now on, with cameras the only media type
  selected.

**USB, kept on purpose.** The board's host controller and
`USB_VIDEO_CLASS` are built, so a plain USB webcam appears as a
`/dev/video*` and the shell — which takes the first one — can scan with
it. That is the whole reason USB is on: gadget mode, HID, mass storage and
SCSI are out, and the USB network adapters went with `CONFIG_NET`, so the
port carries a camera or nothing.

**Three things stay on that read as if they should not**, each for one
driver. `CONFIG_PM`, because the Raspberry Pi firmware power-domain driver
depends on it and both the USB controller and the camera sit behind those
domains; `SUSPEND` and `HIBERNATION` are out. `CONFIG_WATCHDOG`, because
`BCM2835_WDT` owns `pm_power_off` on a Pi and `poweroff -f` — init's next
line after the app exits — does nothing without it. `CONFIG_STAGING`,
because the legacy camera lives there.

**BusyBox is static and the C library is musl.** BusyBox was the only
package Buildroot built for the target, so the 2.2 MB of glibc shared
objects in the rootfs existed for it alone. Buildroot's "static only" is
not offered with glibc (`BR2_STATIC_LIBS` depends on
`!BR2_TOOLCHAIN_USES_GLIBC`), so the toolchain is musl and
`BR2_STATIC_LIBS=y`; glibc-static could not be measured against it for
that reason. The image now has no `/lib/*.so` and no dynamic loader.
BusyBox itself grew from 767 KB to 857 KB; the rootfs cpio fell from
8.51 MB to 6.44 MB.

**Partitions.** The boot partition was 96 MB for about 32 MB of content
and is now 32 MB for about 16 MB. The exchange partition stays at 32 MB.
Both sizes now say in `genimage.cfg` why they are what they are.

**The numbers**, `pi3` + `waveshare-28dpi`, release variant:

| | before | after |
|---|---|---|
| built-in options (`=y` in the kernel `.config`) | 3156 | 743 |
| `Image`, the uncompressed kernel | 54.24 MB | 14.15 MB |
| `zImage`, compressed, with the initramfs inside | 25.90 MB | 7.07 MB |
| `rootfs.cpio.gz` | 5.03 MB | 4.04 MB |
| `opensigner-pi.img` | 132 MB | 68 MB |

The dev variant is the same to within its one extra option, `VT_CONSOLE`,
which puts the kernel console on the panel: 744 built-ins, `zImage`
7.08 MB, image 68 MB.

2404 built-in options went. By group: 799 network stack, netfilter and
network drivers; 290 USB gadget, HID, storage, SCSI, serial adapters and
USB miscellany; 262 infrared, DVB, tuners, radio and CEC; 176 optional
i2c/spi/one-wire chips, LEDs and thermal; 154 filesystems and the NLS
tables they wanted; 152 sound; 113 kernel crypto, AF_ALG, AppArmor and
the TPM; 104 IIO sensors, joysticks and the other input drivers; 88 DRM,
KMS and the tiny-panel drivers; 71 MD, DM, MTD, loop, NBD, zram and swap;
47 tracing, profiling, BPF, audit and task accounting; 31 Bluetooth; 27
namespaces, cgroups and autogrouping; and the rest singly, mostly camera
sensor drivers the media autoselect used to pull in and the Broadcom
Wi-Fi driver.

§11.2's claim that the Pi image has no networking stack compiled into the
kernel is true from this pass, and the build checks it every time.

**Verified on hardware (2026-09-21).** The owner booted the release
image built from `eb71a68` on the 3B+ with the 2.8" panel. The panel,
the touch controller and the camera all work, and the app keeps working
after the card is pulled, which is what the initramfs is for. Until that
boot the kernel configuration had only been checked against the
`compatible` strings in the built `bcm2710-rpi-3-b-plus.dtb` and in the
`waveshare-28dpi-3b-4b` and `waveshare-touch-28dpi` overlays, which is
where the keep-list comes from. The poweroff path after Wipe and exit
was not part of that session's report.

### 16.94 A keyboard, a mouse and a cursor in the Pi shell (2026-09-16)

**Why.** The same binary that drives the Pi's soldered-on panel is what a
laptop runs when it boots from the OpenSigner stick. A laptop has a
framebuffer of an arbitrary size, a keyboard, a touchpad, usually a
mouse, and sometimes a touchscreen. None of that changes what OpenSigner
is; it changes what the shell reads.

**The input model.** A mouse is a touch at a cursor. The shell owns the
cursor position in framebuffer pixels; `BTN_LEFT` down, the moves while
it is down, and `BTN_LEFT` up become `Event::Touch` Down, Move and Up
there, and the wheel becomes `Event::Scroll` there at 48 px a notch, the
distance the desktop shell scrolls a line. The core gains no event and no
screen is designed twice: every screen works with a mouse for the reason
it works with a finger. A keyboard is `Event::Key`, which the core has
had since the desktop shell — one US layout in one table, Shift for case
and the shifted punctuation, Caps Lock ignored, the kernel's auto-repeat
repeating Backspace and nothing else, as on the desktop. A keyboard is an
accelerator: every flow stays fully usable with touch alone.

A touchpad moves the cursor by its finger's travel while `BTN_TOUCH` is
set, and clicks with `BTN_LEFT`; a pad with no button clicks when a
finger lands. No acceleration — one device unit is one pixel, times
`--pointer-speed`, default 1 — because a signing device has no muscle
memory to honour and a person aiming at a 48 dp row wants the pointer
where they put it. No gestures: nothing in OpenSigner is one.

**The cursor is the shell's.** The core composes a frame that knows
nothing about pointers. The shell converts that frame into the panel's
format in a buffer of its own and draws the arrow into that copy, so the
core's frame is never modified and the next frame starts from a clean
conversion with nothing to restore. The arrow is 12 × 19 px at 160 dpi,
scaled by a whole number for a denser screen, white with a one-pixel
black edge. It is hidden until a pointer first moves, hidden again the
moment a finger touches a touchscreen, and never drawn on a machine with
no pointing device — so the Pi's panel gets the bytes it always got.

**The discovery rule.** Every `/dev/input/event*` is opened at start and
classified from its sysfs capability bitmaps
(`capabilities/{ev,key,rel,abs}` and `properties`), which the kernel
prints as text: a keyboard has `EV_KEY` with the letter keys; a mouse has
`EV_REL` with `REL_X`/`REL_Y`; a touchpad has `EV_ABS` with
`ABS_X`/`ABS_Y` and `BTN_TOOL_FINGER` or `INPUT_PROP_POINTER` and not
`INPUT_PROP_DIRECT`; a touchscreen has `ABS_MT_POSITION_X` or
`INPUT_PROP_DIRECT` and is not a pad, since a pad reports multitouch too
and its finger is a pointer. One device may be several, and a receiver
that is a keyboard and a mouse is read as both. Text, not an ioctl:
`EVIOCGBIT` would need `libc` and `unsafe`, which this crate forbids
(§16.33 draws the same line for the camera, and pays for it in one
audited crate). `--touch-name` still wins for touch, which is how the
Pi's Goodix panel is found and how a laptop whose touchscreen the rules
misread can be told.

**No hotplug.** Devices are found once, at start. The image runs `mdev`
with no hotplug helper and the shell has no netlink socket, so a keyboard
or mouse plugged in later is not seen. On a laptop booted from the stick
they are already in.

**Density.** `--dpi` first; failing that `opensigner.dpi=N` on the kernel
command line, which the stick sets; failing that 160, the density the
design system's dp are 1:1 at. A firmware framebuffer says how many
pixels it has and never how large they are, so a laptop has nothing to
ask: a person who finds the text small edits one boot entry. The Pi's
`panel.conf` still passes `--dpi 286`, so nothing about the panel
changes. `--size` stays optional and sysfs stays the answer when it is
absent.

**What the laptop sizes needed in the core: nothing.** Home, a Words
screen with its keyboard, the scanner, Sign review and Settings were
rendered at 1920 × 1080 @ 160, 1366 × 768 @ 120 and 2560 × 1600 @ 227.
All three are the `wide` class, all three lay out, nothing clips and no
panel fails to fit. The screens are spacious — a content column in the
middle of a large black field — which is what a screen designed for a
2.8" panel looks like on a laptop, and is not a defect to design away in
this pass.

### 16.95 OpenSigner on a USB stick: a laptop as the device (2026-09-16)

**Why.** A person who wants an airgapped signer has to buy one, or build
one out of a Pi, a panel and a camera. They already own a laptop. Booted
from a stick that carries its whole operating system, and left with no
network stack to reach anything with, a laptop is the same device the Pi
is for as long as it is powered from the stick: the same binary, the same
sixteen screens, the same file channel. Nothing is installed and nothing
on its disk is read or written.

**What it is.** `just stick-image` builds
`out/stick/opensigner-x86_64-uefi.img`: a GPT image whose first partition
is an EFI system partition holding one file, `EFI/BOOT/BOOTX64.EFI`, and
whose second is the same `OSKDATA` exchange partition the Pi's card has.
That one file is the kernel. `CONFIG_EFI_STUB` makes `bzImage` a PE/COFF
application with its own loader and the whole root filesystem is the
initramfs inside it, so there is no bootloader, no shim, no configuration
file and no firmware blob anywhere on the stick. The command line is
compiled in, with `CONFIG_CMDLINE_OVERRIDE`, because nothing is there to
pass one.

**The image tree did not fork.** `boards/x86_64-uefi` and
`panels/efi-framebuffer` are two more directories in the tree §16.93 left,
and what was Pi-shaped in `common/` moved to the Pi's board: the firmware
configuration, the console name, the card's `genimage.cfg`, the device
trees, `HID`, `SCSI` and `USB_STORAGE` being off. `board.conf` now says
how a machine boots — whether its firmware reads a configuration file
(`FIRMWARE`), whether it passes a command line or the kernel carries one
(`CMDLINE`), what the exchange partition is called (`EXCHANGE`), which tty
a dev login goes on (`CONSOLE_TTY`) — and `build.sh` reads that instead of
assuming. `panel.conf` may now say almost nothing: a firmware framebuffer
reports its own size in sysfs, no firmware reports a density, and there is
no touch controller with a fixed name, so the file is a name and a logo
size.

`check-kernel-config.sh` now reads `kernel.forbidden` and
`kernel.required` out of each part directory rather than holding two
arrays. That is what the two devices needed: `HID`, `SCSI` and
`USB_STORAGE` are forbidden on the Pi, whose touch controller is its whole
input device, and required on the stick, whose keyboard and touchpad are
HIDs and which boots from a USB disk. A list with an exception in it stops
being readable as the rule it states. A symbol in both lists at once now
fails the build, since two rules that cannot both hold would otherwise be
settled by whichever ran last.

**The kernel** is mainline 6.6.84 from kernel.org — a longterm series —
with `x86_64_defconfig`, its `=m` lines dropped by §16.93's hook, the
common fragment, and the board's. Each board's `buildroot.fragment` now
carries the tarball's sha256 and `build.sh` checks the download against
it; Buildroot checks a hash of its own only for the handful of versions
its `linux.hash` lists, and checked nothing at all for the Pi's tarball
from GitHub.

**The decisions inside that kernel**, each of which is a position and not
a default:

- **Secure Boot: unsigned, and the stance is enrolment.** The image is not
  signed by Microsoft's CA and carries no shim, because a shim is a second
  bootloader signed by somebody else and its whole purpose is to run what
  that somebody else's chain permits. A machine that enforces Secure Boot
  therefore takes this stick only if Secure Boot is turned off or the
  image's hash is enrolled in the firmware's own database. Signing with a
  key the owner holds, so that a person enrols one key rather than a hash
  per release, is §15 item 52.
- **Both IOMMUs on from boot, in strict mode.** `INTEL_IOMMU` and
  `AMD_IOMMU` with `INTEL_IOMMU_DEFAULT_ON` and
  `IOMMU_DEFAULT_DMA_STRICT`. A laptop's USB4 or Thunderbolt port is a
  PCIe slot on the outside of the case, and a PCIe device reads all of
  memory unless something stops it. `THUNDERBOLT` itself is not built, so
  the port carries USB and DisplayPort and nothing that speaks PCIe.
- **No hotplug.** Input devices are found once, at start (§16.94), and
  `rcS` waits for the stick's own second partition to appear before
  running `mdev -s`, because on a machine whose devices arrive over USB
  that wait is what the bus settling looks like. A keyboard plugged in
  after the app is up is not seen. **Superseded by §16.97:** the shell
  lists `/dev/input` again every two seconds, so it is. The rest of this
  bullet still holds.
- **Entropy from the CPU is mixed and not credited.** `RANDOM_TRUST_CPU`
  and `RANDOM_TRUST_BOOTLOADER` are both off. The kernel still stirs
  RDRAND and the firmware's seed into the pool; what these options change
  is whether it will hand out randomness on their word alone. A generator
  inside a CPU cannot be examined and a seed from the firmware was chosen
  by the firmware.
- **Microcode: no blob, and the loader could not be removed.**
  `CONFIG_MICROCODE` is `def_bool y` with no prompt on any kernel built
  for an Intel or an AMD CPU, so it is in this image whether or not it is
  wanted. What it does at boot is look inside the initramfs for
  `kernel/x86/microcode/*.bin`, and there is no such file: the image
  carries no firmware blob of any kind, so the CPU runs the microcode its
  own flash holds. `MICROCODE_LATE_LOADING`, which is a prompt, is off.
  **To revisit:** the other side of this is a CPU running with known
  errata unpatched, and shipping a blob would be the first blob in the
  image.
- **No suspend, no hibernate, and no runtime power management.**
  `CONFIG_PM` is off with `SUSPEND` and `HIBERNATION`. A device that holds
  a seed in DRAM has no state to write to a disk and no reason to be
  asleep on somebody's desk. ACPI still powers the machine off, which is
  init's `poweroff -f` after the app exits.
- **No efivarfs, no `/dev/cpu/*/msr`, no paravirtualised guest support.**
  The stick changes nothing about the machine it is plugged into, and a
  boot entry written from it would be exactly that.
- **`CONFIG_EXPERT` is on.** It builds nothing. What it does is give a
  prompt to symbols that otherwise have none, and a symbol with no prompt
  cannot be turned off: `NAMESPACES` and `VGA_CONSOLE` are both `default
  y` with the prompt hidden without it. Every ARM defconfig sets it, which
  is why the Pi never needed the line.

**The display, and the one flag that had to go.** The firmware leaves a
framebuffer running and hands over its address, size, depth and stride;
`SYSFB_SIMPLEFB` turns that into a platform device and `FB_SIMPLE` or
`FB_EFI` publishes it as `/dev/fb0` with the same numbers in sysfs, which
is what §16.94's shell reads when it is given no `--size`. The panel
directory must therefore *not* pass `--fb /dev/fb0`: `--fb` means "this is
a file off the device" and turns the sysfs probe off, which is right for
writing frames into a file on a desktop and exactly wrong here. The first
build that passed it drew a 480×640 16-bit frame into the corner of a
1280×800 32-bit framebuffer, which is what that mistake looks like.

**The boot logo is a 320×320 square in the corner.** The kernel draws its
logo at the top left and does not scale it, so §16.93's full-panel logo
needs a panel size, and this display's is not known until it is on. A
square that fits inside the smallest display anybody would boot this on is
what there is; `panel.conf` says so with `LOGO_WIDTH` and `LOGO_HEIGHT`.

**The ESP is 48 MB because FAT32 says so.** A FAT32 filesystem is only
valid with at least 65525 clusters, and a 32 MB partition has about 64500.
`mkfs.fat` makes one anyway with a warning, and OVMF refuses to mount it —
which reads as `failed to load ... Not Found` and no boot at all, with
nothing pointing at the filesystem. The exchange partition stays at 32 MB.

**The test.** `just stick-test` boots the image in QEMU with OVMF and KVM,
as a USB disk on an xHCI controller with a USB keyboard and a USB mouse,
and checks over QMP what a person would check: that the screen settles on
something that is not black and carries the action bar's orange; that the
action bar and then Esc reach Home; that a click on Home's first row opens
it; and that Esc goes back to a frame identical to the one it left. It
writes a screendump per step, as a PPM and a PNG, because a frame that
changed is not a frame that changed correctly. It passes on both variants.
The click targets are read off the frame — the centre of the accent
pixels is inside the primary button, and the topmost band of the surface
colour to the right of the sidebar is the first row — rather than computed
from the layout, since a test that recomputed the layout would pass on a
screen nobody could use.

**The numbers**, `x86_64-uefi` + `efi-framebuffer`:

| | release | dev |
|---|---|---|
| built-in options (`=y` in the kernel `.config`) | 1096 | 1097 |
| `bzImage`, which is `BOOTX64.EFI`, with the initramfs inside it | 9.60 MB | 9.60 MB |
| `rootfs.cpio.gz` | 4.40 MB | 4.40 MB |
| `opensigner-x86_64-uefi.img` | 81 MB | 81 MB |

The kernel configuration check holds 59 forbidden options and 39 required
ones for this board, against the Pi's 52 and 22. The Pi's own numbers are
unchanged: the same 743 built-ins, the same `.config`, the same images.

**Not verified on hardware.** Nobody booted a laptop. What has booted is
QEMU with OVMF, which is the same firmware interface and the same USB
path, and which found the framebuffer, the keyboard and the mouse and
mounted the exchange partition by label. What QEMU cannot show is a real
machine's i2c touchpad, a real webcam, or a firmware that will not run an
unsigned image. §15 item 53 is the list of machines that have been tried,
which is empty.

### 16.96 An encrypted backup is the same size and the same name whatever it holds (2026-09-16)

**The rule.** Every encrypted backup is the same length, and its name
says nothing about the key. What is inside a backup is learned only by
opening it.

Version 1 (§16.78, which stands as history) leaked that twice. Its
plaintext was the word count, the language and the entropy the words
encode, so a file was 107 bytes for a 12-word key and 123 for a 24-word
one: the length of the file, and the size of the QR, said which. And the
file the shell wrote was named with the key's fingerprint, so a card in a
drawer said which wallet its backup belonged to without being opened. The
kept-key blob already did this right — 42218 bytes with one key or a
hundred — and the backup now does too.

**Format version 2.** The plaintext is fixed at 34 bytes: the word count,
the language's index in `Language::ALL`, then 32 bytes of entropy,
zero-padded on the right where the words encode fewer. The word count
says how many of those bytes are the key's. A backup is therefore always
123 bytes, `MIN_LEN` and `MAX_LEN` are one `LEN`, and the QR is one size
on every display class. The pad is zero, is checked to be zero on open —
a non-zero pad is the same `Error::Format` a bad length is — and is
zeroized with the rest. Version 1 is not read: nothing has shipped, and
`open` reports it as the existing "unsupported backup". The kept-key
blob's own 3 → 4 is the precedent.

**The name.** `Command::WriteFile`'s name hint for a backup is
`opensigner-backup.oskb`, with no fingerprint and no date, and it is the
same string for every key; the shell's existing rule of never overwriting
gives `opensigner-backup-2.oskb` for a second one. The Result screen
already carries the key's fingerprint and the saved file's name, which is
where a person learns which file is which. `WriteFile`'s name hint stays
an owned string.

`tools/backup/decrypt.py` reads version 2 only: the fixed length, the pad
stripped by the word count, a non-zero pad refused. `tools/vectors/backup/`
now holds two vectors, a 12-word English one and a 24-word Japanese one,
both 123 bytes, from different seeds because they share a passphrase; the
test opens both with this code and with the script when its libraries are
installed. Learn › Encrypted backups says the file is the same 123 bytes
whatever it holds and that its name says nothing about the key.

---

### 16.97 The stick after its first outside review (2026-09-16)

**Source.** A reviewer read §16.95 and the image tree and booted the stick
on a Dell XPS 9360. Seven points came back: the kernel builds drivers for
the laptop's own disk; a framebuffer depth the shell cannot write draws
noise instead of stopping; input devices are found once; the Buildroot tag
and the container image are names and not hashes; the density is fixed at
a rebuild; the touchpad does nothing; and a laptop has no serial port to
find out why. Six of the seven changed something. The seventh, the
density, is §15 item 54: recorded, not built.

**The laptop's own disk is not visible to the kernel.** §16.95 said
nothing on the machine's disk is read or written, and that rested on
nothing mounting it. `x86_64_defconfig` builds `ATA`, `SATA_AHCI` and
`ATA_PIIX` in, and nothing forbade `BLK_DEV_NVME` or `MMC`, so the disk
was there as a block device and the claim rested on restraint. It now
rests on the kernel: `ATA`, `BLK_DEV_NVME`, `NVME_CORE`, `MMC` and
`SCSI_LOWLEVEL` are in `boards/x86_64-uefi/kernel.forbidden` and off in
the fragment, so a SATA, an NVMe, an eMMC or a RAID-attached disk never
becomes a node. `SCSI` and `BLK_DEV_SD` stay: a USB mass-storage device is
a SCSI disk, and that is the stick itself. This also closes the one path
by which a partition on the internal disk labelled `OSKDATA` could have
been mounted in place of the stick's, since `rcS` finds the exchange
partition by label.

**A depth the shell cannot write is an error.** `fb.rs` accepted 16 and 32
bits per pixel; anything else made `Geometry::from_sysfs` return nothing
and `main.rs` fell back to the 480×640 RGB565 panel this shell was written
for, which on a laptop is noise in the corner of the screen. Two changes.
24 bits per pixel is now written, as three bytes: `Depth::Bgr888` and
`Depth::Rgb888`. Which of the two is a machine's is read from the `red`,
`green` and `blue` bitfields in sysfs where a driver publishes them, and
is otherwise blue first — the kernel's `simplefb` format table has one
24-bit entry, `r8g8b8`, with red at bit offset 16 and blue at 0, which
little-endian puts in memory as blue, green, red. And on a device, a
framebuffer directory that exists and cannot be described is now a refusal
with the reason ("fb0 is 30 bits per pixel, which this shell cannot
write"), not a fallback, whatever `--size` and `--depth` say. The 480×640
default is the file mode's and nothing else's. The reason goes to stderr,
which is to say it appears under `--verbose`, which is the dev image and
the boot report; the release image has no console, so there was never
anywhere else for it to go.

**Devices plugged in after start are found.** `evdev::Watch` lists
`/dev/input` again every two seconds and classifies whatever is new
exactly as `find_devices` classifies what was there at boot. A reader
thread that ends names its device, and the entry is dropped once the node
itself is gone, so a device whose reader stopped for some other reason is
not opened again and again, and the same port plugged again is a new
device. A mouse plugged into a machine that had none brings the arrow with
it; the last pointer unplugged takes it away. No netlink, no hotplug
helper, no ioctl: a `read_dir` and the sysfs text §16.94 already reads.
This supersedes §16.95's "no hotplug" bullet.

**Both remote inputs are pinned by hash.** `run-build.sh` cloned Buildroot
at tag `2024.02.13`, and a tag is a name somebody can move. The justfile
now also carries `buildroot_commit`, and the build fails before compiling
anything unless `git rev-parse HEAD` in the checkout equals it, printing
both hashes. The tag stays, because that is what a person reads. The
Dockerfile's `FROM debian:bookworm-slim` is likewise now pinned to the
digest that tag named on 2026-09-16. §11.2 names both.

**A laptop's touchpad.** On the XPS 9360 the keyboard worked and the
touchpad did nothing. Its pad is I2C-HID, and the stick had `I2C_HID_ACPI`
and `I2C_DESIGNWARE_PLATFORM` — the protocol and the ACPI-enumerated bus.
On Skylake and later the i2c controllers are PCI devices behind Intel's
Low Power Subsystem driver, so without `MFD_INTEL_LPSS_PCI` and
`I2C_DESIGNWARE_PCI` no bus appeared at all; and the pad's interrupt is a
pin on the chipset's GPIO controller, so without `PINCTRL_SUNRISEPOINT`
the device would have been found and never interrupted. Both halves are
now built, along with every other Intel pin controller the 6.6 tree has,
`PINCTRL_AMD` and `I2C_AMD_MP2` for AMD machines, and the PS/2 vendor
protocols and RMI-over-SMBus for the pads that are not HIDs at all. Each
of these drivers is a pin table or a protocol and a few hundred lines; a
laptop is one of them and there is no way to know which from here. The
twelve that a pad cannot work without are in `kernel.required`. The cost
is 33 built-in options and 88 KB:

| | before | after |
|---|---|---|
| built-in options (`=y`) | 1096 | 1129 |
| `bzImage`, which is `BOOTX64.EFI` | 10 064 896 B | 10 155 008 B |

**A boot report, because a laptop has no serial port.** The dev variant's
console was `ttyS0`, which is a header under a laptop's bottom cover if it
is anywhere. It is now `tty0` as well, so the boot prints on the screen
until the app takes the framebuffer. And the dev variant writes
`opensigner-boot.txt` to the exchange partition on every boot: `rcS`
writes the kernel log there as root, because `/dev/kmsg` is not the
unprivileged app's to read, and the shell appends the panel it chose and
every input and capture device it found with what it made of each. The
person plugs the stick into any computer and reads it. Two files say it is
the dev variant and nothing else has to: `/etc/opensigner/boot-report`
holds the name, and `--boot-report` is in `/etc/opensigner/args`. Neither
is in a release image, which writes nothing to the medium at boot.

**What this did to the Pi.** Its kernel is untouched: the same `.config`,
the same 743 built-in options, the same 52 forbidden and 22 required. The
only shared file that changed is `rcS`, which gained the boot-report block
— inert without `/etc/opensigner/boot-report`, which no release image has
— so the Pi's rootfs and therefore its image are no longer byte-identical
to §16.95's, and nothing they do differs.

**What was recorded and not changed.** The density fixed at 160 is §15
item 54. A keyboard-only path through the first screens — what a person
with a pad this kernel does not drive has instead of nothing — is §15 item
55, and it is a design-system question before it is a shell one. Secure
Boot and MIPI cameras were already in the image README and stand.

### 16.98 A single-input SegWit transaction needs no previous transaction (2026-09-17)

§16.54 refused every SegWit v0 input without `non_witness_utxo`, and
ended with the condition that would reopen it: a wallet that cannot
supply previous transactions for such inputs. Sparrow is that wallet.
Over QR, `HeadersController.showPSBT` attaches previous transactions only
when the PSBT has more than one input and a keystore is registered as
device type Krux — `WalletModel.includeNonWitnessUtxoForQR()` is true for
Krux alone. A single-input spend from a segwit wallet therefore never
carries them, whatever the registration, and that spend is the everyday
one.

The rule is now the transaction's input count. A SegWit v0 input
(p2wpkh, p2sh-p2wpkh, p2wsh, p2sh-p2wsh and the miniscript wrappers
`classify` reads) without `non_witness_utxo` raises
`WarningKind::AmountUnverified` and is refused by `sign` only when the
transaction has more than one input. A single-input transaction with
`witness_utxo` alone is inspected and signed, as taproot already was.
Legacy and taproot are unchanged, and the warning's kind, level and
string are unchanged.

The attack §16.54 closed needs two signatures the chain accepts, on two
inputs of one transaction, each made in a round under a different lie
about the total spent. BIP-143 commits each signature to its own input's
stated amount, so with one input a lie yields a signature no node accepts
and nothing to combine it with. The count is the transaction's rather
than the number of inputs naming our keys: a coordinator can strip the
key origin from input B in round one and from input A in round two, so
the device signs one input per round and the two signatures still
combine.

`classify` carries the rule, because it already receives the whole
`bitcoin::Psbt` and both `inspect` and `sign` then read one flag. The
published vector `tools/vectors/psbt/warn-amount-unverified.psbt` is now
a two-input p2wpkh spend, since a one-input one is no longer blocked.
Registering a Sparrow keystore as device type Krux is what makes Sparrow
send previous transactions with a multi-input PSBT over QR, and
`README.md` says so.

### 16.99 The PIN pad takes the keyboard (2026-09-17)

The lock screen and every other PIN pad are unchanged on screen and now
also accept a keyboard's digits, Backspace and Enter, shuffled or not.
`accepts_char(Pin, c)` takes `'0'..='9'` and nothing else. Enter is the ✓
key: `UiState::key` refuses it on a `Pin` keyboard whose ✓ is dead, so an
incomplete PIN is never submitted and never costs an attempt, and the
set-PIN, repeat-PIN, stored-key and lock pads all get that from one
place. A digit past the longest PIN is dropped, as a tap on the pad is.
Nothing was added, moved or dimmed on any screen.

A laptop booted from the stick has a keyboard, and that is how its person
types. The pad's shuffle was the guard against click-position logging on
a desktop, and it still guards a touched entry; a desktop that can log
keys can also capture the screen, so the pad never protected against that
machine — §4.5's own "desktop reality" line says so. The anti-observation
line in §4.5 stands as it is.

What would reopen it: a platform where a key log is possible and a screen
capture is not.

### 16.100 MuSig2 signing: how the rounds run, and against what (2026-09-17)

§8.4 item 20 is now designed. §16.69 and §16.70 left a `musig()` wallet
that loads, reviews, exports and derives addresses, and a PSBT spending
from it that reaches the review as a taproot key-path spend of a key the
device does not hold. What follows is the rule for signing it, decided
against what exists outside this tree on this date.

**What exists outside.** Bitcoin Core signs `musig()` descriptor wallets
since the wallet change merged on 2025-10-14, first in 31.0; its nonces
live in memory only and a restart restarts the session. Ledger's Bitcoin
app has signed since 2.4.0 (April 2025): at the end of round 1 it writes
a 64-byte session (one secret seed per session, every per-input nonce
derived from it) to flash, and before round 2 it loads the session,
deletes it from flash, regenerates the nonces and signs. Coldcard's
experimental branch signs since 6.5.0X (March 2026). No QR coordinator
speaks the rounds: Sparrow has nothing as of 2.5.5, Nunchuk's MuSig2
wallets sign with hot keys only. The only counterparts a device can be
checked against today are Bitcoin Core over the file channel and
Ledger's own test script. `rust-secp256k1` carries libsecp256k1's MuSig2
module since 0.33.0 (August 2026); the `bitcoin` crate this tree pins
(0.32.x) depends on 0.29, and its 0.33 is a beta pinned to a yanked
secp256k1 beta. Neither crate reads BIP-373's fields.

**Decisions.** Build it now, against Bitcoin Core 31.1 on regtest, with
the PSBT carried by the file channel and by the same `ur:crypto-psbt`
animation every other PSBT uses; nothing new is defined for QR. The
rounds are written by hand over the curve primitives, the way
`osk-bip::musig` already writes `KeyAgg`, and tested against every
BIP-327 vector file; when a stable `bitcoin` release depends on a
`secp256k1` with the module, the module becomes a cross-check and not a
replacement. The nonce rule of §16.47 stands: a secret nonce lives in
memory for one session and is never written to any tier.

**What Bitcoin Core 31.1 writes, read on this machine.** A regtest wallet
`tr(musig([73c5da0a/86h/1h/0h]tpub…,[b503c405/86h/1h/0h]tpub…)/<0;1>/*)`,
Core holding the second key, funded and asked for a spend:

- `PSBT_IN_TAP_BIP32_DERIVATION` carries every participant's account
  key with its own origin (`73c5da0a` at `m/86h/1h/0h`), and the
  internal key with the aggregate's BIP-328 fingerprint (`2830b927`) and
  the sub-path (`m/0/0`). A device therefore finds itself by origin, as
  it does in every other script type, and reads the derivation from the
  aggregate off the internal key's entry.
- `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` is keyed by the root aggregate
  (`KeyAgg` of the sorted participants, before any derivation) and lists
  the participants in aggregation order.
- `PSBT_IN_MUSIG2_PUB_NONCE`, and so `PSBT_IN_MUSIG2_PARTIAL_SIG`, are
  keyed by the participant's key and **the taproot output key** in
  compressed form: the aggregate after the BIP-32 derivation tweaks and
  the BIP-341 tweak, which is the witness program with a parity byte.
  BIP-373's text reads as the root aggregate; Core's `SignMuSig2` uses
  the tweaked key, and Core is the reference and the only counterpart.
- The change output carries the same participants field and the same
  three derivation entries, its internal key at `m/1/0`.
- With only its own nonce present, `walletprocesspsbt` adds nothing and
  reports the PSBT incomplete; with every nonce present it writes its
  partial signature, and with every partial signature present it
  aggregates and finalizes.

**The fields.** `osk-psbt` reads and writes BIP-373's four fields
through `bitcoin::Psbt`'s `unknown` maps, which is where a field the
crate has no type for survives a round trip: input types `0x1a`, `0x1b`,
`0x1c` and output type `0x08`, with the key layouts BIP-373 gives.
A nonce or partial-signature key with the 32-byte tapleaf hash appended
is a script-path MuSig2 spend and is refused as unsupported: a `musig()`
inside a tap tree or a `multi_a` is not a wallet this tree loads
(§16.70). A nonce or partial-signature key naming another signer is read
whichever of the three aggregate forms it carries — root aggregate,
derived aggregate, output key — because a second signer may follow the
BIP's words rather than Core's code; a key this device writes carries
the output key.

**Which inputs it signs.** A `P2trKey` input whose internal key is the
BIP-328 derivation of an aggregate whose participants include a loaded
key. The wallet must be registered: the participant set is what the
aggregate is, and a set that differs from every registered `musig()`
wallet's is a different wallet, so the input is blocked with "MuSig2
wallet not registered" rather than signed for a set the coordinator
chose. The participants field, the sorted participants of the
registered wallet, and `KeyAgg` of them must agree with the root
aggregate the field is keyed by; the internal key must equal the
aggregate derived along the sub-path the derivation entry states; and
the output key must equal `script_pubkey_at`'s. Any mismatch is a block
that names the input. Two loaded keys that are both participants are two
participants, each signed for. A merkle root on the input is refused:
`tr(musig())` has no tree.

**The tweaks and the message.** From the root aggregate, one
`ApplyTweak` per unhardened step of the sub-path with the plain form,
the tweak being what `Xpub::ckd_pub_tweak` returns for that child of the
BIP-328 aggregate xpub; then one `ApplyTweak` with the x-only form and
BIP-341's tap tweak of the derived key with no merkle root. `gacc` and
`tacc` come out of that chain and into the signature, which is why
`AggregateKey` carries them (§16.69). The message is the BIP-341
key-path sighash under `SIGHASH_DEFAULT`, the sighash every taproot
input in this tree already computes.

**Two ways through the rounds, and which is used when.**

- *Signing last, with no session.* BIP-327's `DeterministicSign`
  derives a signer's nonce from the other signers' aggregate nonce, the
  keys, the tweaks and the message, so it needs nothing kept between
  rounds and cannot reuse a nonce: the same inputs give the same
  signature. When a PSBT arrives with every other participant's nonce
  present, the device produces its public nonce and its partial
  signature in one pass, and, when the other partial signatures are
  present too, aggregates and finalizes. With Bitcoin Core as the other
  signer this is the whole flow: Core's first `walletprocesspsbt` writes
  its nonce, the device signs last, Core's second call finishes. One
  participant per transaction can sign this way, because it needs the
  others' nonces first.
- *A session.* When another participant's nonce is missing — a second
  OpenSigner in the same wallet, or a coordinator that wants this
  device's nonce first — the device runs BIP-327's `NonceGen` with the
  shell's entropy (one 32-byte draw per session, one tagged hash of it
  per input and participant as the `rand'` each call needs, `sk`, `pk`,
  the output key as `aggpk`, the sighash as `m`), writes its public
  nonces, and keeps the secret nonces in memory as the session. The
  session is keyed by the unsigned transaction's txid and the set of
  inputs and participants it covers. When the same transaction returns
  with every nonce present, the session signs and is destroyed; it is
  also destroyed by lock, wipe, exit, a new session, and by signing.
  There is at most one.
- A PSBT that carries this device's public nonce for a session the
  device no longer holds is signed afresh: the stale nonce is replaced
  and every partial signature already on that input is dropped, since
  each was made against an aggregate nonce that included the stale one.
  The review says so as a caution, "Nonce replaced · earlier signatures
  dropped", and the coordinator's other signers sign again. The
  alternative, refusing, leaves a transaction nobody can finish.

**What the person sees.** No screen is added. The review is the Sign
review as it is, with the wallet's row reading **MuSig2 · 2 keys**. On
the Hold screen the "Sign with" chips are the participant keys this
device holds. After a pass that ended in a partial signature the Result
is "Partially signed" as for any multisig, with the rows it already has.
After a pass that ended in round 1 the Result is titled **Nonce shared**
with the rows *Session · open*, *Ends · power off, lock, or signing*,
*Signatures · 0 of 2*, and the same Save and QR actions, and Home's
status line carries *Session open* until the session ends; each of those
is a fact of the device's state. The person then hands the PSBT to the
coordinator, gets it back with the other nonces, and reads it through
Sign again, where the open session picks it up. Nothing on a working
screen explains the rounds; Learn's nonce page does.

**Change.** A change output whose participants field, derivation
entries and script match a registered `musig()` wallet at the chain and
index its internal-key entry states is verified change, through
`WalletPolicy::script_at`, which closes the gap §16.69 left; any other
output claiming the wallet's keys is the `ChangeSpoof` block it already
is for a multisig.

**Verified against.** All eight of BIP-327's vector files, now in
`tools/vectors/bip327`: `nonce_gen` with the fixed `rand'` the file
gives, `nonce_agg`, `sign_verify`, `sig_agg`, `tweak` and `det_sign`
join `key_agg` and `key_sort`. Then Bitcoin Core 31.1 on regtest, both
orders: Core's nonce first and the device signing last with no session,
and the device's nonce first and Core finishing, with
`testmempoolaccept` judging the final transaction as it did in
`tools/vectors/psbt/README.md`. The PSBTs of those runs are the
fixtures.

**In two passes.** The first is the crate and the stateless flow: the
fields, the rounds and their vectors, participation and the blocks,
change verification, signing last, aggregation and finalization, the
Core fixtures in both orders, and the review and Result rows for a
partial signature. The second is the session: `NonceGen` from the
shell's entropy, the session in the app and its ends, the *Nonce shared*
Result and the Home status line, the stale-nonce caution, and the
device-first Core fixture read through the app.

What would reopen it: a coordinator that speaks the rounds over QR,
which would set the transport a session waits on; a stable `bitcoin`
release on a `secp256k1` with the MuSig2 module, which would add the
cross-check; BIP-373 being amended to say which aggregate key the nonce
is keyed by, which would settle the form this device writes.

Pass M1 landed on 2026-09-17: the rounds in `osk-bip::musig` against all
eight BIP-327 vector files, BIP-373's four fields in
`osk-psbt::musig`, participation and the blocks in `inspect`, change
verification, signing last with `DeterministicSign`, partial-signature
verification, aggregation and finalization. Bitcoin Core 31.1 on regtest
accepted the finished transaction:
`f0d3252c8eaca5c515afc70479d8fea6e57c691b7cbb05076436aec523ede2a8`, 154
vB, 1 550 sat, `testmempoolaccept` allowed, after Core's second
`walletprocesspsbt` reported `complete: true` over the PSBT this device
wrote. Three things the design above got wrong or left unsaid. The
BIP-328 aggregate fingerprint is per wallet, so `2830b927` is that one
run's and not a constant; the fixture run, whose second key is
`9d6d0393`, has another, and what the device matches on is
`aggregate_xpub`'s own fingerprint, computed per input. `NonceAgg` is
defined over public nonces, so `DeterministicSign` refuses an
`aggothernonce` whose half is the extended encoding of infinity, even
though an aggregate nonce may carry one — BIP-327's own error vector says
so, and the aggregation this crate does for the session accepts what the
appendix refuses. And "a PSBT that needs this device's nonce first" is
blocked whenever *any* participant it holds lacks the others' nonces,
which includes the case of two loaded keys in one wallet: neither can
derive its nonce from the other's before the other exists, so the session
of pass M2 is what that case waits on, not just the coordinator-first
one.

Pass M2 landed on 2026-09-17: the session. `sign` now decides per input
and per participant — round 2 from a secret nonce the session holds,
signing last with `DeterministicSign`, or round 1 drawing a nonce, and a
participant whose partial signature is already on the input signs nothing
again. `osk_psbt::MusigSession` holds the txid, the seed and one secret
nonce per input and participant, hands each out by value, and is wiped on
drop; the app draws the seed from the session key the way `schnorr_aux`
draws its bytes and clears the session at lock, wipe, exit, a session for
another transaction and the signing it was opened for. The Result after
round 1 is "Nonce shared" with the Session and Ends rows, the hold reads
"Hold to share nonce", Home carries "Session open", and a nonce of ours
that no session holds is the caution "Nonce replaced · earlier signatures
dropped".

Bitcoin Core 31.1 on regtest ran both orders over one wallet and one
unsigned transaction, txid
`8f06551973e373166a8eb841a92fa0cb93bf02d2d65f8cd1c8b06b7346650b33`, 154
vB, 616 WU, 1 550 sat, `testmempoolaccept` allowed both times; the
witnesses differ, because the two runs aggregate different nonces.
Device-first took two `walletprocesspsbt` calls, and what the first does
is the thing worth recording: with every nonce already on the input — the
device's and, after that call, Core's — Core still writes only its nonce
and reports `complete: false`. The second call writes its partial
signature; a third changes nothing. A coordinator that calls once and
stops therefore hands back a PSBT with two nonces and no signature, which
this device reads as round 2 all the same, since what it needs is the
other nonce.

Four things the design above and the pass's brief got wrong or left
unsaid. A participant that has already signed is a fourth case, ahead of
the three: without it the aggregation-only pass replaces its own nonce
and drops the signatures it is there to aggregate. A session that holds a
nonce for a transaction that comes back without some other participant's
nonce waits — it neither signs nor draws again, because its public nonce
is already out. The device-first Core reply is a fixture of its own,
`wallet-musig-device-first-core.psbt`, because round 2 cannot be
reproduced without the bytes Core answered with. And the three badges of
Home's status line do not fit the 268 dp panel beside the lock, so §4.8
gains one form: the tier's letter alone while the session badge stands
there. The fixtures of both orders come from this one run, so the M1
transaction recorded above belongs to the wallet the earlier run built,
not to the files in the tree.

### 16.101 A USB stick plugged in after boot is a file channel (2026-09-17)

The stick's file channel was its own `OSKDATA` partition, mounted once by
init; a person could not pull the stick, put a PSBT on it from a computer
and plug it back in, and a second stick was never seen. The owner's
words after the first laptop boot: "file based PSBT is basically
unusable".

Every FAT partition on a USB disk, present at boot or plugged in later,
is now mounted by init and read by the app; a partition that goes away is
unmounted. `rcS` still mounts the boot medium's `OSKDATA` once, then
starts `/usr/sbin/opensigner-usb-watch`, a POSIX-sh loop that reads
`/sys/class/block` once a second, probes a new partition with BusyBox
`blkid`, and mounts it when its type is `vfat` and nothing else: the
boot medium's label goes back to `/mnt/microsd` whichever node it
returns as, so the settings file and the dev variant's boot report keep
one path, and every other partition goes to `/mnt/usb/<label>`, or the
node's name when it has no label, with `-2`, `-3` on a collision. The
label is reduced to letters, digits, `_` and `-` before it names a
directory, so no label can name a path outside `/mnt/usb`. Two
partitions are never mounted: the one labelled `OSKBOOT`, which holds
the kernel on this or any other OpenSigner medium, and anything else on
the disk the machine booted from. The watcher runs only where the
exchange line names a label, which is the USB-booted stick; a Pi's card
is fixed and its kernel has no USB storage. Mount options are the
exchange partition's: `flush,noexec,nosuid,nodev,uid=200,gid=200,umask=077`.
No uevent helper and no `mdev -d`: polling `/sys` adds no daemon that
takes kernel events, and a person plugging a stick in does not notice a
second.

The shell lists every mounted place on each request, `/mnt/microsd`
first, then `/mnt/usb/*` in name order, newest file first across all of
them. A file is named `PLACE/FILE` when more than one place is mounted
and by its bare name when one is; a read resolves the name against that
same fresh listing, so a name that was not listed reaches nothing. A
save goes to the boot medium's partition when it is mounted and to the
first other place otherwise; settings and timings stay on the boot
medium. `Event::FileList.place` joins the place names, which the empty
Files row shows.

Verified in QEMU: the dev image's `stick-test` hot-plugs a FAT disk
labelled `KINGSTON` over QMP after the app has drawn, reads
`opensigner: mounted /dev/sdb1 at /mnt/usb/KINGSTON` and the unmount
line off the console, and sees no line for the boot stick's `OSKBOOT`
partition; the release image boots and passes its test with the same
disk attached. The app's own Files screen listing the plugged-in file is
covered by the shell's tests over two directories, not by the QEMU run.

The first laptop run of it, 2026-09-18: the stick's light flashed without
stopping, and the boot stick pulled and put back was not read. Both were
the loop. It asked `blkid` about every unmounted partition every second,
and the boot stick's EFI partition is always unmounted, so the stick was
read once a second; and it took a mount as live while a node of that name
existed, so a stick that came back as `sda` again kept its stale mount
and was never remounted. A partition is now known by its identity — the
sysfs path of its disk's device and the number the USB bus gave that
device when it enumerated it — and is probed once, when that identity
first appears; a mount whose identity has changed under it is a mount of
a device that is gone and is taken down before anything new is looked
at. The path alone was the first version of this and was wrong in a way
QEMU hid: the kernel frees a SCSI host number when a disk goes and hands
it to the next, and a stick put back into the same port has the same
path, so on a laptop the identity would not have changed; QEMU had given
each re-added device a new port. The device number is what the bus hands
out afresh to every device it enumerates, and is not reused until the
count has gone round 127. The QEMU test now puts each stick back into
the port it came from, expects the second stick mounted again and
`/mnt/microsd` to go and to return, and counts block reads over eight
quiet seconds afterwards, which must be none. Its log shows both sticks
returning on the SCSI host and port they left, and nothing read.

What would reopen it: a kernel with a filesystem other than FAT, which
§16.95's deny list rules out; or a medium whose exchange partition is
found by node rather than label wanting hot-plug, which no board has.

### 16.102 The keyboard drives every screen (2026-09-17)

DESIGN §4.15. A shell with a keyboard drives all sixteen screens with
it. Tab and Shift+Tab move a focus over the layout's hit targets in the
order the solver placed them, wrapping at the ends; Up and Down do the
same while something has focus and scroll the first scrolling region
while nothing does; Left and Right move within a row and turn a Words
pager's page; Enter and Space act on what is focused as a tap does, hold
a hold while the key is down, and reach the on-screen keyboard's ✓ when
nothing is focused; Escape is still Back. The focused item is outlined
by a 2 dp accent ring inside its own rectangle at its own radius, and
that ring is the only thing any of this adds to a screen. Nothing on a
screen says the keys exist. A dimmed control takes focus and does
nothing, as it does nothing under a finger. A touch or a pointer press
takes the ring away, and nothing has focus when a screen opens.

Why: the first boot of the stick on a laptop (§16.97's reviewer, whose
notes the owner relayed). The keyboard on the PIN pad (§16.99) was the
thing that worked, and the rest of the app did not follow it — a machine whose touchpad the kernel does not drive
(§16.97) reached the lock screen and stopped there. The idiom is the
desktop's own, so it needs no explanation, and a working screen carries
no explanation anyway (DESIGN §2.1). Writing it once over the hit targets
rather than per screen is what keeps every screen the same and keeps the
sixteen screens' specs unchanged.

What it costs: a key coming up. A hold is held with Enter the way it is
held with a finger, so `Event::KeyUp` is in the shell contract, optional
— a shell that never sends it cannot hold with a key, and its hold
button does nothing on Enter. The desktop and Pi shells both send it,
and both let Tab and the arrows auto-repeat as Backspace already did, so
holding Down walks a list.

What would reopen it: a shell class where a key has a meaning of its
own, such as a physical d-pad, whose Up and Down are the device's only
input and not an accelerator over touch. Nothing in the tree is one.

### 16.103 A threshold wallet: dealer FROST, one device, nothing kept (2026-09-18)

**What.** A wallet kind "Threshold · m of n" whose one key is a FROST
group key: any m of n shares sign a BIP-340 key-path spend, and the
chain sees a single-sig taproot spend. Key generation is the trusted
dealer's, on the owner's own device. Signing is BIP 445's. The device
keeps nothing between the two rounds; one secret nonce travels on the
stick. This is §8.4 item 21, designed here; the roadmap row says so.

**The route it is designed for.** One owner. One OpenSigner, carried.
A share at home, a share at another location, a third share somewhere
else that this route never uses. A spend is: scan Sparrow's PSBT at the
first location and sign there; carry the stick; sign at the second
location; hand Sparrow the finished transaction. Either location can be
first. The device is powered off in between and holds nothing.

**Why FROST and not the MuSig2 tree.** The tree form (`musig()` leaves,
§16.100's follow-up) spends by script path whenever fewer than all n
sign, which on this route is always: one signature plus a 34-byte
script and a control block, private but not invisible. FROST signs the
key path with any m. BIP 445 reached "one tweak from publication" on
2026-09-16, with vectors, a reference and implementations under way, so
the signing standard is no longer the risk it was in §16.100's survey.

**Why the standard, and what is ours.** Everything the protocol defines
is taken from BIP 445 as it stands: `ThresholdInfo`, `nonce_gen`,
`nonce_agg`, the session context, `sign`, `partial_sig_verify`,
`partial_sig_agg`, `deterministic_sign`, the tweak context with `gacc`
and `tacc`, the tagged hashes `BIP0445/aux`, `BIP0445/nonce`,
`BIP0445/noncecoef`, `BIP0445/deterministic/nonce`, the identifier
range and the participant bound of 128. Ours, each with the rule for
replacing it:

1. The share on paper: 24 BIP-39 words with a valid checksum, whose 32
   bytes are the share scalar itself, never hashed into a seed. No
   standard exists; when one does, the device reads both. The words are
   indistinguishable from a seed phrase, and loaded as a seed they are a
   working single-sig wallet, which the owner may fund as a decoy; the
   device never guesses which role a phrase has and asks.
2. The group record: `ThresholdInfo` serialized, with `n`, the
   identifiers, the synthetic xpub and the descriptor, as one file and
   one QR. Public. When a BIP defines a `frost()` key expression or a
   descriptor form, the record gains it and the device reads both.
3. The PSBT session fields: BIP 174 proprietary records under the
   identifier `osk`, one type each for a public nonce and a partial
   signature, keyed by participant identifier and the taproot output
   key, values as BIP 445 serializes them. The secret nonce is never in
   the PSBT. When a BIP assigns bytes, the device writes those and reads
   both for one release, then drops `osk`, as btclib intends for its
   own. Sparrow never sees these fields: it reads the PSBT it built and
   the finished transaction, and the session fields are stripped before
   the transaction leaves the device.
4. The nonce file: the second share's secret nonce, bound to the
   transaction, beside the PSBT on the stick. Ours entirely; nothing
   will standardize it, since it exists only because one device plays
   two participants.

**Key generation.** On the owner's device, from Create › "Threshold
wallet": choose m and n (2 and 3 first; n at most 5 in this version),
then the polynomial of degree m-1 over the scalar field. Its first m
coefficients' worth of freedom is m shares chosen freely: from the
shell's entropy, from dice as any key is, or an existing 24-word
phrase's bytes taken as a scalar, which lets a location that already
keeps a phrase keep only that. The other n-m shares and the group key
are computed. The device shows each share as words with the quiz, and
the group record as a QR and a file; then it forgets the secret and
every share it did not choose to keep loaded. A share alone derives
nothing; with the record it derives addresses; m shares rebuild the
record and a replacement share for a lost one. The identifier of share
i is BIP 445's 0-based `id`; the scalar is the polynomial at `id + 1`,
which the pass confirms against the reference.

**The group key and addresses.** The group key is a plain key. It gets
a synthetic xpub the way BIP 328 gives one to a MuSig2 aggregate, with
the same fixed chain code, and the wallet is `tr(XPUB/<0;1>/*)`: to
Sparrow and Core a single-sig taproot wallet with an air-gapped signer,
which never learns there is a threshold. Derivation is unhardened only,
there being no private master. Each address is the group key tweaked
by the BIP-32 steps as plain tweaks and then by BIP 341's tap tweak as
the x-only one, through BIP 445's tweak context, exactly as §16.100's
MuSig2 does through `AggregateKey`. The wallet's fingerprint is the
synthetic xpub's; a share's fingerprint is hash160 of its public share.
Both are labels the device prints beside the words so a person copies a
label that was computed for them.

**Signing.** Per input the device holds one share, `s_a`, and knows
from the record every public share. At the first location it draws two
nonces with BIP 445's `nonce_gen`: its own, with `s_a`, and the second
share's, without a secret share, which the BIP allows; both from the
shell's entropy, with the sighash as `msg` and the output key as the
x-only key. It writes both public nonces and its partial signature into
the PSBT, and the second nonce's secret half into the nonce file,
bound to the sighash, the signer set and the first public nonce. At
the second location the device holds `s_b` and does, in this order, or
refuses at the first failure with a named reason: the stored secret
nonce's point equals the second public nonce in the PSBT; the signer
set and sighash in the file equal the PSBT's; the first partial
signature verifies with `partial_sig_verify` against the first public
share, both nonces, the tweaks and the message; then `sign`,
`partial_sig_agg`, BIP-340 verification of the result against the
output key, `tap_key_sig`, `finalize`, and every `osk` record removed.
The nonce file is deleted. Nothing but the final transaction leaves
the device.

**Why a stored nonce is safe here and nowhere else.** A reused nonce
leaks a share only when it signs under two different challenges, and
the challenge changes only when someone varies a nonce or the message.
Here both nonces were drawn by this device, the file binds them and the
message, and the second location accepts nothing it cannot verify by
arithmetic: a replayed file yields the identical signature; a
substituted nonce breaks the first partial signature's verification,
which cannot be recomputed without the first share; the second partial
signature never leaves the device, so a copy of the file taken in
transit is a random scalar with nothing to pair it with. §16.47's rule
stands for every multi-party session; this is not one. The design pass
that adds a second device to this route reopens the question, and the
answer there is §16.100's session or Tier B's counter, not this file.
Encrypting the file under a passphrase typed at both locations is
defence in depth and is offered, not required.

**What the person sees.** No new screen. Home lists "Threshold · 2 of 3
· Taproot" with the key glyph when a share is loaded and the eye when
only the record is. Load offers "a share of a threshold wallet" beside
"a key", and never infers the role from the words. The Words screen of
a share carries "Share 2 of 3 · a4dc80c1 · wallet 2830b927" as its
label. The Sign review is the review; the hold reads "Hold to sign" at
both locations; the Result is "Partly signed · 1 of 2" then "Signed".
The wallet's menu has the record as its export, alongside the
descriptor.

**In three passes.**

- *F1, the protocol.* `osk-bip::frost`: BIP 445's algorithms by hand
  over the curve primitives, as `osk-bip::musig` is, sharing its scalar
  and tagged-hash helpers; `SecNonce` not `Clone`, wiped on drop,
  consumed by `sign`. The seven vector files from the reference under
  `tools/vectors/bip445/` with digests, every valid and error case
  asserted. The dealer: shares from chosen scalars, the rest computed,
  recovery of the record and of a missing share from m shares, with
  tests against small hand-computed cases and against the reference's
  `secshares`/`pubshares`.
- *F2, the wallet.* `Template::Threshold` in `WalletPolicy` with the
  record's serialization; the synthetic xpub; `script_at` through the
  tweak context; Create › Threshold wallet with the share sources;
  Load › a share; the labels; the encrypted backup carrying a share as
  it carries words; Home, Addresses, Verify and Export for the kind;
  DESIGN's Hub label and share label lines. Fixtures: a regtest 2-of-3
  record and its three shares from a fixed seed.
- *F3, the spend.* `osk-psbt`: the `osk` records, participation from
  the record, the two-location rules, the nonce file's format and its
  checks, aggregation into `tap_key_sig`, stripping. The core's Sign
  flow: the same screens, Save writes the PSBT and the nonce file, Read
  takes both. Tests: a tampered file, a replayed file, a substituted
  nonce, a different signer set, each refused by name; the full route
  through the app with one harness playing both locations; and on
  regtest, Core holding the single-sig view of the wallet, building the
  PSBT, and judging the finished transaction with `testmempoolaccept`,
  with BIP 445's Python reference playing the second share once as an
  independent check of the partial signatures.

**Open in this entry.** Whether `nonce_gen` for the second share should
take the first share's secret as `extra_in` for misuse resistance, or
nothing; the reference is silent and the pass decides with a reason.
Whether the nonce file is one file per PSBT or a section of the PSBT's
own file with a device-only marker; one file per PSBT is the working
assumption. Whether the decoy use of a share's words as a seed is said
in Learn or left to the reader; Learn says it.

**What would reopen it.** BIP 445 changing before publication, which
the vectors would show; a BIP for FROST PSBT fields or a `frost()`
descriptor, which replaces items 2 and 3 above; a second signing device
on the route, which is a different design; a person wanting m of n
larger than the words and a QR can carry, which is n above five.

**Pass F1 landed on 2026-09-18:** the protocol and the dealer.
`osk-bip::frost` writes BIP 445's `NonceGen`, `NonceAgg`,
`GetSessionValues`, `Sign`, `PartialSigVerify`, `PartialSigAgg`,
`DeterministicSign` and `ValidateThresholdInfo` over the curve
primitives `osk-bip::musig` already had, which are now `pub(crate)` and
shared; the nonce, the aggregate nonce, the partial signature and the
tweak are BIP-327's types unchanged, and the tweak context is an
`AggregateKey` with a new `untweaked` constructor for a key no
aggregation stands behind. A `SecShare` and a `SecNonce` are not
`Clone`, are wiped on drop and carry no derived `Debug`; `sign` takes
the nonce by value and verifies its own partial signature before
returning it. The dealer takes `t` chosen shares, computes the rest,
and rebuilds a lost share or the whole public record from any `t`. All
six vector files pass, every valid and every error case, against the
four `(t, n)` groups: 29 signatures, 52 refusals to sign, 12
verification failures and 8 verification errors, 28 tweaked signatures,
37 deterministic signatures with 48 refusals, and 18 aggregate
signatures that also verify as BIP-340 signatures under the tweaked
group key.

Four things the design above got wrong or left unsaid. The F1 bullet
says seven vector files; six carry cases, the seventh is the
reference's description of their layout, and neither
`ValidateThresholdInfo` nor key generation has vectors at all, which is
why the dealer is checked against RFC 9591's Appendix F.5 shares in the
reference's `trusted_dealer.py` and against every group's own
`secshares` recovered from every `t`-sized subset of them. Lagrange
interpolation needs a division mod `n`, which neither BIP-327 nor this
tree had; the inverse is a variable-time binary extended Euclid, which
is sound here because every value inverted is a product of participant
identifiers that the signer set states in the clear, while the secret
share is only ever multiplied. Where the reference parses bytes inside
an algorithm, Rust parses at the door, so four refusals belong to the
reader and not the algorithm — `parse_pubshare`, `parse_pubnonce`,
`parse_psig` and `parse_aggnonce`, each taking the position it blames,
which is the position in the list the contribution arrived in and not
the participant's identifier. Two refusals have no Rust form at all, a
tweak that is not 32 bytes and a tweak list whose mode list is another
length, because a `Tweak` is 32 bytes and carries its own mode; the
tests assert the shape the type refuses instead of a variant. And
`DeriveThreshPubkey` can fail on its own: public shares that
interpolate to infinity are the refusal four signing error vectors
expect, which is a failure of the signer set rather than of any one
contribution.

Three things the design said hold. The share of identifier `id` is the
polynomial at `id + 1` and the group secret its value at 0; the code
works in identifier coordinates, where the same polynomial has the
share at `id` and the secret at `-1`, because that is what the BIP's
own `DerivePubshareAt` uses, and in those coordinates the dealer never
forms the secret at all: the group key comes from the public shares,
and `recover_info` rebuilds a record by public arithmetic alone. A
threshold of one gives every participant the same share, which the
reference's own dealer test states and the 1-of-3 vector group carries
in its three identical secret shares. And `DeterministicSign` refuses
an `aggothernonce` whose half is the extended encoding of infinity, as
BIP-327's does and for the same reason: `NonceAgg` is defined over
public nonces. The `extra_in` question stays open — the parameter is
there and this pass fixes no use for it.

**Pass F2a landed on 2026-09-18:** the record and the wallet kind, with
no share loaded anywhere. The record is written down here, because this
is the first place it exists:

```text
osk-threshold 1
threshold 2
group 0376c80f9704bbf33f96b9fcf464deec0548997110fc1e0fed81c9bea5e0ea8559
share 0 0374bdb32605b725d806c1e32a0bf21ffa4d77730de273c04b9a70ada79fb8e424
share 1 02dc9d772e60e1c333e9c2e91bd29a4bc044d7043169be842bd9bc7c7684ef956a
share 2 02b5cdec6ed9651a3a59d3f16b6972cdd8ddb2ae5cc4375b316cd810ad1f398e2b
tr(tpubD6NzVbkrYhZ4XgHkCEtfpuZPJDLaLPxu5ZBEtAbub9GcUX1mTS2t3eCnBZaMe1T8T6J9PyYd7XcxwrVTy67Je62zzB9ZEqZr9Ze5GEL5bps/<0;1>/*)#q5zp64n6
```

That is the committed regtest fixture,
`tools/vectors/psbt/wallet-threshold-regtest.record`. Line 1 names the
format and its version; blank lines and lines beginning with `#` are
skipped, as a wallet policy's are. Then `threshold t`, then `group` with
the group key compressed in lower-case hex, then one `share i` line per
participant, numbered from zero in order with none missing — BIP 445
lets a record leave a share out and this version of the file does not —
and last the descriptor, `tr(XPUB/<0;1>/*)` with its BIP-380 checksum,
where XPUB is BIP 328's synthetic extended public key over the group
key (depth 0, no parent, index 0, the fixed chain code) and its version
bytes state the network. The reader checks all of it: `1 <= t <= n`,
`n <= 15`, every key a point, `ValidateThresholdInfo`, the descriptor's
key equal to `musig::synthetic_xpub(group, network)`, and the checksum.
Each refusal is a `policy::Error` variant, three of them new — `Share`
for the numbering, `Group` for shares that are not one polynomial or do
not give this group key, `Xpub` for a descriptor over another key.
`ThresholdRecord` lives in `osk-bip::threshold`, holds a
`frost::ThresholdInfo` and an `Xpub`, and carries the wallet's
fingerprint (the xpub's) and each participant's (hash160 of its public
share, first four bytes).

`Template::Threshold { threshold, participants }` is the wallet kind.
`WalletPolicy` keeps the record beside the template, as it keeps a
parsed descriptor for the two miniscript kinds; `parse_any` routes text
whose first line names the format to it, and `parse` and
`from_descriptor` refuse it as they refuse any unknown text. The policy
has one key, the synthetic xpub with no origin, and `check` allows that
for this template as it does for a single-key one. `quorum()` is `None`
for the reason MuSig2's is: the output carries one key and one
signature, and the `t` of `n` is a fact about the shares. `script_at`
is the single-key taproot path over that xpub, shared with
`Single { Taproot }`, and the first receive and change addresses are
checked against `miniscript` deriving the record's own descriptor line.
`leaf_of` answers for the synthetic fingerprint at `0/i` and `1/i`,
which is the origin a coordinator holding `tr(XPUB/<0;1>/*)` writes, so
F3 has change verification. In the app the kind is one row,
**Threshold** · **2 of 3**, the review's participants are "Share 1 of 3"
with that share's fingerprint and the eye, Home's row reads
"threshold · regtest" beside "Threshold · 2 of 3 · Taproot", the menu's
Keys row is **Shares**, there is no Sign row, and Export offers the
checksummed descriptor and the record. A threshold wallet in use is kept
in the blob as its record text and comes back after a restart.

Three things the design left unsaid, and one it got wrong. The record's
network is the descriptor's version bytes and nothing else: the design
says "the synthetic xpub and the descriptor" without saying which states
the network, and a record carries no network line, so reading a `tpub`
record as mainnet is not a refusal but a different wallet, whose xpub
differs. The design says the policy text is exported "alongside the
descriptor", which meant the Export screen's policy row, and that row
was offered only to a wallet whose every key carries an origin; a
threshold wallet's key carries none, so the rule is now "a wallet with a
text of its own", which is its record. The design's `n` bound is five
for Create and 128 for BIP 445, and neither is the file's: 15 is what a
QR and a blob slot hold, and that is what the reader enforces. And the
design called F2 one pass; the share words, Load a share, Create and the
backup are a second half that touches secrets, and splitting them off
left this pass with nothing secret in it at all, which is why every
threshold row carries the eye and no `SecShare` reaches the app.

**Pass F2b landed on 2026-09-18:** the secret half of F2. A share in
memory is `LoadedShare` in `load.rs`, held exactly as a key's words are
— `Held<MnemonicBytes>` sealed under the session key, resealed on
every rotation, wiped on drop — with the language, the public share
computed once at load, and the share fingerprint beside it. It derives
nothing: no master, no accounts, no single-sig wallet, and `secret` is
the only door the scalar comes through. `OpenSigner` gained `shares`
beside `keys`, and everything that asked whether there is something to
lock, wipe, leave or time out now asks `has_secrets`, which counts both.
A share's wallet is found and never stored: the registered threshold
wallet whose record carries this public share, and the identifier is
that share's position, so reading the record later gives a share on its
own its wallet with no state to keep in step. The label is
`share_label`, "Share 2 of 3 · a4dc80c1 · wallet 2830b927" or
"Share · a4dc80c1", and it is the one string the Words screen, the
wallet page's Share row, the confirmation and Home all print.

Load a share is the Load wizard in a share mode: no count step (a share
is 24 words), no passphrase steps, and a confirmation that states the
share rather than a master fingerprint. Create a threshold wallet is
`ThresholdWizard` in `opensigner-core/src/threshold.rs`, a secret file
with no heap text: how many shares, how many must sign, then one source
Choice per chosen share, the dealer, every share's words and quiz, the
group record, which share stays loaded, and the confirmation. The
chosen shares take identifiers 0..t in the order they are chosen, which
is what makes a typed phrase land where the person put it.

Five things the design got wrong or left unsaid. The design says the
record is a QR screen with Save and Copy; §5's QR screen had no
action row at all, so it gained one (`qr_with_actions`), Copy is the app
bar's own row that every public string already has, and Continue is the
way on — a QR with no way forward is a dead end, which is what the
missing row would have been. The design says the wizard's confirmation
is "Hold to add"; §2.7 makes a hold the mark of something that
cannot be undone, and adding a wallet is undone by forgetting it, so it
is a tap, as the key wizards' confirmation already is. The refused
chosen share has no error line, because §5's Choice screen has
none: the reason stands at the trailing edge of the row it was taken
from, which is §4.8's place for what a person should weigh about an
option they can still take. The design says a 12-word phrase offered to
Load a share is refused on the error line "with the reason the wizard
has for a wrong count"; the wizard has no such reason and no way to
reach its own Words step with 12 words, so the refusal belongs to the
scanner the phrase arrived at, which already has an error line and now
has "not 24 words" to put on it. And the share's Backup is a menu, not
a flow: Words, Encrypted backup and Verify backup, because the seed
codes, the grid, the steel rows and Seed XOR are a key's backups and
carry a key's word counts.

Two things the design said hold. A share's words are a seed phrase and
the device never guesses: the same 24 words loaded through Load a key
are a single-sig wallet with a master fingerprint and no screen of it
says "share", and loaded through Load a share they are a share and
derive nothing. And the dealer never needs the group secret: `deal`
returns the shares and the public record, `ThresholdRecord::new` makes
the wallet from the record, and the wizard zeroizes the dealt set and
every share not kept when it drops.

**Open in this pass.** A share is not kept in the Tier B blob. The keys
record has no slot shape for a share, "Keep on this device" is not
offered on the share menu, and a restart of a device that keeps keys
comes back with its threshold wallet and no share of it, which is the
eye where the key glyph stood. Giving the blob a shares record is its
own pass; until then a share is session memory, as every wallet policy
was before §15 item 32.

**Pass F3 landed on 2026-09-18:** the spend. `osk_psbt::threshold`
writes the two `osk` proprietary records — a public nonce under subtype
`0x00` and a partial signature under `0x01`, each keyed by the
participant's public share and the taproot output key — with the readers
and writers `musig.rs` has and a `strip` that runs over every finished
input, so a coordinator sees the PSBT it built and the transaction and
never the session. `ThresholdInput` in `inspect` establishes an input in
the order §16.100 established a MuSig2 one: the registered wallet found
by the synthetic fingerprint and the two-step path Core writes, the
internal key, the tweaks, the output key, the script, the signer set the
records name, the shares this device holds, and what the pass will do
with each. `sign` takes the loaded shares as a door the scalar comes
through for the length of one call and the carry section the flow read,
and does per share exactly one of four things: nothing, because it has
signed; a later location, from the stored nonce, after four checks in
order; a first location, drawing every signer's nonce from the seed and
putting the others into the carry section; or a refusal by name. The app
gained the Sign row on a threshold wallet's page, the "Then with" chip
row where the `t - 1` other shares are chosen, the Result "Partly
signed · 1 of 2" whose Save writes `partly-signed.osk` and whose QR is
not offered, and the carry file routed into Sign the way a PSBT is.

Three decisions the design left open. `extra_in` is none for every
signer: this share's secret already enters its own nonce through
`secshare`, the other signers' nonces are bound by the carry section, and
mixing this share's secret into a nonce that is written to a file would
open a path from the share to the stick for no gain. The nonce file is
one file with the PSBT inside it, not a file beside it: the shell's file
channel answers one `RequestFile` with one file and a picker shell cannot
fetch a sibling by name, so a file "beside" the PSBT would work on the
stick and not on the desktop; the magic `OSKC` keeps anything that reads
PSBTs from taking it for one. And there is no encryption yet: §16.103
offers a passphrase as defence in depth, and it stays open.

Bitcoin Core 31.1 on regtest imported the fixture record's own descriptor
watch-only, funded it and built the spend; the device signed at two
locations and Core judged the result. `finalizepsbt` reported
`complete: true` and `testmempoolaccept` allowed txid
`0f140e0ee3b7339234c362e39488e93b726d1f3cc04f70fc2b9b313c02fbac68`, 154
vB, 616 WU, 1 550 sat. What Core wrote for the internal key is what the
design predicted: one `taproot_bip32_derivs` entry with the synthetic
xpub's own fingerprint and the relative path, `c027851f` at `m/0/0`, with
no origin above it, which `WalletPolicy::leaf_of` already read, so the
reader needed no change. BIP 445's reference implementation, vendored
under `tools/reference/bip445/` with its MIT licence and the digests of
every file, was then given share 1's secret share and the secret nonce
out of the carry file and asked to sign the same session: it produced
`45add421…`, the partial signature this tree wrote, verified both partial
signatures with `partial_sig_verify`, and aggregated to `6c52734b…`, the
signature in the accepted transaction. `just threshold-reference` runs
it; it is not part of `just`, because it is Python.

Four things the design got wrong or left unsaid. The design says the
"Sign with" and "Then with" chips carry a share's label, "Share 2 of 3 ·
a4dc80c1"; §4.4's chip is a fingerprint and the row of them does not
wrap, so a label that long puts the second chip off the side of a 480 px
panel where nothing can tap it. A share is its own fingerprint on a chip,
exactly as a key is its master's, and which share it is belongs to the
record and the wallet page. The design's Result string, "Partly signed",
is the string the Sign flow already had for a transaction some of whose
inputs are unsigned, so no `sign_result_partly` was added: what a
threshold pass changes is the count beside it, which is the partial
signatures over `t`. A threshold input names no loaded key in its
origins — the group's one key descends from no master — so
"none of your keys can sign this transaction" fired on every threshold
spend and the review's first page stayed dead; both now ask whether a
share participates, and the review's key context names the share. And
the design says the refusals are the second location's; three of them are
the reader's and show up in the review before any hold, because
`Context` carries the carry section and `inspect` can make every check
the signer makes.

**Still open after this pass.** Encrypting the carry file under a
passphrase typed at both locations. A share is still not kept in the
Tier B blob, which F2b left open. And the stored-nonce argument covers
one device carrying one file; a second signing device on the route is a
different design, and the file this pass writes is read only by the
device that wrote it.

### 16.104 Keys are keys, wallets are policies over them, Home is a launcher (2026-09-18)

**Why.** The threshold wallet (§16.103) added a third shape to Home —
a share alone is a row, a share inside a wallet is not — and Add grew to
six rows that mix secrets with public things. The owner's verdict: the
keys and the wallets are mixed together, and the way through the app
is not simple. §16.75 had merged the two into one list because §16.71's
split showed one seed on three screens. The cause of that was not the
split; it was the single-sig wallet the device made up for every key,
which put the key on Keys, on Wallets and inside every policy it was
in. This entry splits them again and removes the cause.

**The model, as rules.**

1. **A key is 24 (or 12 to 24) words and nothing more.** It has a
   fingerprint. It is not a wallet, not a share and not a signer until
   a wallet uses it. Keys is a flat list of fingerprints, one row per
   key, and the list never says how a key was made: a passphrase key
   and a BIP-85 child are peers of every other key. What a key is made
   of is a fact on its own page, where backup needs it — "24 words",
   "24 words and a passphrase", "BIP-85 child" — and nowhere else.
   Keys is the only place a secret is made, shown, backed up, derived
   and forgotten. Its Add a key menu: Load a key (words, a seed code,
   an encrypted backup, Seed XOR parts) · Create a key · Open with
   passphrase · Open BIP-85 child, the last two asking which loaded key
   to start from. A key's page: its facts, Backup, Keep on this device
   (Tier B), Forget.
2. **A wallet is a policy over keys, with the addresses that follow.**
   Wallets lists what was registered or built here, and nothing the
   device made up: no automatic single-sig wallet. Add a wallet asks
   the kind first — Single-sig · Multisig · MuSig2 · FROST — then which
   loaded keys, then what the kind needs: the script type for a
   single-sig, the threshold for a multisig, how many shares and how
   many must sign for FROST. A descriptor, a coordinator file or a
   FROST record read by Scan arrives at the same review and "Add this
   wallet". A single-sig wallet is explicit: a person wanting SegWit
   and Taproot addresses from one key adds two wallets, which is one
   more step than before and the step that makes the model honest. A
   wallet's page: Sign a transaction (when this device holds a key of
   it), Sign a message (single-sig), Addresses, Check an address,
   Export, Keys, Forget. Its Keys row is the Keys screen filtered to
   its members, each with the key glyph or the eye, and a member that
   is not loaded says so; the same rows, the same key page, Forget
   there.
3. **The word "share" leaves the product.** A FROST wallet chooses
   24-word keys as its shares and finds them again by computing each
   loaded key's public share and matching it against the record. The
   label printed beside a key's words is its fingerprint, the one Keys
   shows; the group record still carries hash160 of each public share
   for other software, and the device matches by public share, never
   by label. FROST creation picks t loaded keys as the chosen shares,
   computes the other n − t, shows each computed one as words with the
   quiz, and every one of them is a loaded key until forgotten; there
   is no "which share stays loaded" step, because forgetting keys is
   what Keys is for, and on every tier but B nothing is kept anyway.
   The kind is named FROST, beside MuSig2, because "threshold" was
   doing double duty with "how many must sign".
4. **Home is a launcher.** The status line, then six tiles — Wallets ·
   Keys · Scan · Tools · Learn · Settings — in two columns, and nothing
   else. Home never changes shape. Wallets and Keys carry their own
   empty-state rows. Scan is the one way in for anything read: a PSBT
   or a message goes to signing, a descriptor or a record to the wallet
   review, words to Load a key. On `wide` the sidebar lists the six.
5. **Signing needs keys, never a wallet.** A PSBT names its keys; a
   loaded key at a standard path signs a single-sig input with no
   wallet registered, as today. A wallet is what verifies a policy's
   change, lists addresses and exports.
6. **A passphrase is part of the key and the door to a wallet.** Both
   are true, and the flow serves both without storing anything about
   how a key was made. A passphrase key is a key like any other in
   Keys. A wallet records only the fingerprints of its keys. When
   signing or verifying a message names a key that is not loaded, the
   review says which fingerprint is missing, and that row opens Add a
   key: load the words, open with passphrase from a loaded key, or
   open a BIP-85 child — whichever the person knows it was. A key
   made from a wallet's not-loaded row is checked against the
   fingerprint the wallet named; a mismatch says which fingerprint the
   passphrase gave and does not keep the key, since it was made for
   this wallet and is not its key. Made from Keys directly, a key is
   a key and nothing checks it. The device never infers that a
   missing key is a passphrase key. Addresses, Check an address and
   Export never ask: the descriptor carries the public keys.
7. **Tier B.** The blob keeps words and wallets. A passphrase key is
   never in the blob: its seed could be kept sealed under the PIN
   without the passphrase text, and that would make the PIN alone
   open the hidden wallet, which is what a passphrase exists to
   prevent; Trezor and Ledger make the same choice. After a restart a
   passphrase wallet comes back with its addresses and without its
   key, which is rule 6's case. Whether a wallet whose key is a
   passphrase key is kept at all is the person's choice at its
   creation, off by default, because the kept descriptor is evidence
   the wallet exists; the choice is a row on the wallet's page.

**Rejected.** Keys as children of other keys (a passphrase key or a
BIP-85 child nested under its parent): people treat keys separately,
and the nesting states a fact about a secret's construction on a
public list. A wallet remembering the parent of a passphrase key so it
can ask for the passphrase itself: not typical of any product, and
though it gives an attacker who has the blob no new capability (the
words and the wallet's xpub already allow an offline search), it puts
a fact about a secret into public metadata for the sake of one prompt.
Keys or Wallets as sections of one Home list: the launcher makes the
split explicit and keeps Home from ever changing shape. "Policy" as
the noun on Home: it is the right word for the rules row inside a
wallet, and "wallet" is what Sparrow, Core and every coordinator call
the thing with addresses.

**What this changes in DESIGN.** §4.1 Hub becomes the launcher; §4.14
Empty state moves to Wallets and Keys; §4.4's glyphs appear on Wallets
rows and a wallet's Keys review and never on Keys; §5 Menu gains the
Keys screen, the Wallets screen, Add a key, Add a wallet, the kind's
key picker and the key page, and loses the automatic single-sig wallet;
"Threshold" is "FROST" wherever a kind is named. The crates underneath
do not change, except that `osk-psbt` finds shares among loaded keys
rather than in a list of their own.

**In three passes.** R1: the launcher, Keys, Wallets, Add a key with
its four ways, explicit single-sig wallets, the key page, the wallet
page with its filtered Keys row, the not-loaded row's return path,
every test and snapshot script that walks Home. R2: Add a wallet's
four kinds in one flow, FROST from loaded keys with the share noun
gone, `LoadedShare` folded into `LoadedKey`, the record matched by
public share. R3: Tier B — the keep choice for a passphrase wallet, the
checked passphrase entry from a wallet's row, and the blob rules
stated in tests.

**Pass R1 landed.** The launcher, Keys, Wallets, Add a key, Add a
wallet, explicit single-sig wallets, the key page, the wallet page's
filtered Keys review and the not-loaded row's return path are built, and
every test and snapshot script that walked Home walks the new tree.
`HUB_TILES` grew from three to six and `osk_ui::screens::launcher` draws
them in `organisms::tile_grid`, which already gave two columns on a
portrait panel; `screens::hub` and its band went with the tile band of
§16.75. The sidebar on `wide` is those six and no Home row, because Home
is not an area. A single-sig wallet is built from the chosen key's
account descriptor and registered through the same review every other
wallet ends at, so "Add this wallet", the swap card, the Tier B blob and
"the same descriptor twice is one row" all came for free.
`WalletRef::SingleSig` became `WalletRef::Key` and now reaches only the
address list, where Explore's loaded key and the first run's "Check an
address" need it; a key has no export of its own, and the two account
forms a coordinator asks for (`xpub`, SLIP-132) are served from a
single-sig wallet's own key instead. That last sentence is revised by
§16.110 rule 1: a key exports its public accounts from an "Account key"
row of its own page, and a single-sig wallet's Export keeps the two
forms for the one account it is built on.

What the design left unsaid. The Scan tile is an area of the sidebar on
`wide` but opens a scanner rather than a list, so it is the one entry
that is selected while a flow runs; that reads correctly and is what
§16.104 rule 4 asks for, but the entry is a verb among five nouns. A
wallet's "Keys" row is the filtered list for every kind but FROST, whose
members are public shares and not fingerprints; matching those against
loaded keys is R2's work, so a FROST wallet keeps §16.103's Shares
review and its Share row unchanged. "Load a share" could not leave Add
as rule 3 asks, because a share is still a `LoadedShare` and the Load
wizard cannot decide from 24 words which one a person means until R2
matches by public share; it stands as a fifth row of Add a key, which is
where a secret belongs, rather than on Add a wallet. The entry also did
not say what a wallet's Export offers for a wallet over one key, what
the key page's title is (its fingerprint, not "Key 73c5da0a"), or where
Forget moved to on a key — all three are decided here and stated in
DESIGN §5.

What R2 and R3 still have. R2: Add a wallet's four kinds in one flow
(the three built ones are reached from it and unchanged inside),
`LoadedShare` folded into `LoadedKey`, FROST creation from loaded keys,
the share noun gone, the record matched by public share, and the "which
share stays loaded" step removed. R3: Tier B's keep choice for a
passphrase wallet, the checked passphrase entry from a wallet's
not-loaded row — a key made there is a key in this pass, and nothing
compares it against the fingerprint the wallet named — and the blob
rules as tests. The Sign review's "not loaded" warning for a named key
keeps its wording and has no return path yet; it is R3's, with the
check.

**Pass R2 landed.** Add a wallet is one wizard for all four kinds, a
FROST group is built from loaded keys, and the word "share" is on no
working screen. `BuildWizard` became `WalletWizard` in `build.rs`: a
kind Choice (Single-sig · Multisig · MuSig2 · FROST), then — for FROST
only — "How many keys?" and "How many must sign?", then one Keys step
for every kind, then the kind's own steps (a script type for a
single-sig or a multisig wallet, a threshold for a multisig one,
nothing for MuSig2), then the review and "Add this wallet". The Keys
step is a Choice whose rows are the loaded keys in Keys' order, several
of which carry the check; a key the kind cannot use is dimmed with
§4.11's reason; the cosigners scanned in stand under them, each taken
back out by a second tap on its row; and "Scan a key" is the last row
for the two kinds that take a cosigner from outside. Add a wallet's
menu is now two rows, "New wallet" and "Scan a wallet". The dealer's
own file, `threshold.rs`, is a `Dealer` the wizard holds: it keeps the
dealt set, the record, the words of each computed key and its quiz, and
it holds no heap text, so `tools/lint-secrets.sh` still covers it.

`LoadedShare` is gone. A `LoadedKey` computes its public share once at
load — `SecShare::from_bytes(entropy).public_share()` for a 24-word
phrase whose bytes are a scalar, `None` for anything else — and carries
it beside the fingerprint, so matching a key against a record's members
costs nothing on any screen. `osk_psbt::Context::shares` is built from
the loaded keys that have one, and the `ShareKey` doors open
`LoadedKey::secret_share`, which is the only place the scalar exists.
`Screen::Share`, `ShareForget`, `ShareBackupMenu`, `ScreenKind::Share`,
`BackupFlow::for_share`, the share mode of `LoadWizard` and every
string, id and DESIGN line that named a share are gone with it; a
24-word key is loaded through Load a key and backed up, kept, locked
and forgotten as any key is.

**The two fingerprints of a FROST member, and where each shows.** A
24-word key that is a member of a group has a BIP-32 master
fingerprint, which is the key seen as a key, and the fingerprint of its
public share, which is the key seen as a member. They are the same
secret read two ways and neither can be computed from the other without
the words. Keys shows the master fingerprint, because Keys is a list of
keys and says nothing about what any of them is used for. The wallet's
Keys review shows the share fingerprint, because that is what the group
record carries for other software and what this device prints beside
the words it deals — so the paper a person copied is the paper the
review names. The Sign hold's "Sign with" and "Then with" chips carry
the share fingerprint, as §16.103 F3 settled, and the Sign review's key
context now names the member by the master fingerprint of the loaded
key that matches, which is the fingerprint every other key on that
screen carries. There is no way to show one fingerprint everywhere: the
record is public and cannot carry a master fingerprint without stating
a fact about a secret, and Keys cannot carry a share fingerprint
without saying which group a key belongs to. Two fingerprints, each in
the one place it belongs, is the answer this pass takes.

Six things the design got wrong or left unsaid. The Keys step is a
Choice, and a Choice has no glyph slot, so a scanned cosigner is a
checked row with its path as its subtitle and not "a row with the eye";
the glyph belongs to Wallets and to a wallet's Keys review, which is
where §4.4 already puts it. "Continue is dimmed with the reason" has no
form either — an Action carries no reason — so the count a FROST group
needs is a dimmed row at the end of the list, "Needs 2 keys", and
Continue is simply dead above it. The reason for a key whose entropy is
no scalar is "not usable" and not "not a share", because rule 3 takes
the noun off every working screen and a reason is a working screen's
text. The script type is chosen after the keys for a multisig wallet,
and the account a key contributes depends on it, so every gathered key
is read again when it changes; a scanned cosigner keeps the text it
arrived as for exactly that. A FROST group has no descriptor until it
is dealt, so its review is not "the wallet review every kind has
today": it states the kind, the network and the chosen keys, and "Add
this wallet" is what runs the dealer. And a wallet over one key exports
that key's account out of the account cache, which the single-sig flow
used to fill on its way through; the wizard fills it where the script
type settles, and writes the hardened levels with `h` rather than `'`
so that the descriptor is character for character the one R1 made.

Two the design said hold. Every key of a group this device dealt is
loaded until it is forgotten, and Keys is where a person forgets the
ones that belong elsewhere: there is no "which share stays loaded?"
step and nothing is lost by its going. And the chosen keys take
identifiers 0 to t − 1 in the order they are checked, which is what
makes a key a person picked land where they put it.

What R3 still has, unchanged: Tier B's keep choice for a passphrase
wallet, the checked passphrase entry from a wallet's not-loaded row,
and the blob rules as tests.

**Outside this pass.** The tree at `main` did not build: `osk-bip`'s
`codex32` (§16.106) writes `#[derive(Zeroize, ZeroizeOnDrop)]` and the
workspace's `zeroize` dependency had `default-features = false` with no
`derive`, so seventeen errors stood in `cargo build --workspace` before
this pass touched anything. The feature is on in `Cargo.toml` now;
`zeroize_derive` was already in `Cargo.lock` and in the local registry,
so nothing was fetched.

**Pass R3 landed.** Rules 6 and 7 in full. A key made through Open
with passphrase or Open BIP-85 child from a wallet's not-loaded member
row is compared with the member that row named: a match loads it and
the chevron from the Result is the review, with the glyph; a mismatch
states "This passphrase gives 1a2b3c4d, not b2b1f0cf" on the step's own
error line, keeps nothing, and leaves the field as it was so the person
can type again. `add_key_return` carries the member beside the wallet,
as `Member::Master(fingerprint)` for a policy's key and
`Member::Share(point)` for a FROST group's, which is the same pair of
fingerprints R2 settled; Load a key from that row is unchecked, because
a person may be loading a different key on purpose, and from Keys
nothing asks. Tier B: "Add this wallet" is followed by the Choice "Keep
this wallet on the device?" with "No" checked, for a wallet this device
built over a loaded passphrase key on a device that keeps keys; the
wallet's page carries "Kept on this device" as a §4.2 toggle for any
wallet on such a device; and the wallets record is the wallets that are
kept, written through `sync_kept` as before, with the rest simply not in
the body. A passphrase key was already out of the blob and is now stated
as a test under both answers.

What the design got wrong or left unsaid. "The same statement" for a
BIP-85 child would have been "This passphrase gives …" on a screen
where no passphrase was typed, so the child's is "This child is 1a2b3c4d,
not b2b1f0cf": the same two fingerprints, a true sentence on each screen.
"Returns to the review" is R1's return path, which lands on the Result
that names the key that was added and puts the review under the chevron;
the Result is kept, because it is the one screen that says which key
arrived, and the review is one tap behind it. The entry says the choice
is "at its creation" and the row is "on the wallet's page", which leaves
a scanned descriptor unanswered: a wallet that arrives as a descriptor
is kept as any policy is and its row is the only way to change that,
because this device does not know how another device's keys were made.
And a wallet not kept is tracked by its descriptor checksum rather than
by its index, since Forget renumbers the list; the checksum is what
`names` already keys a wallet's name by.

### 16.105 The info button: one bridge from a working screen to Learn (2026-09-18)

**Why.** Principle 9 says every kind is explained in Learn, and
DESIGN §4.12 says a working screen never explains. The two need a
bridge, and an earlier build had put prose on working screens, which
took time to remove. The owner's ask: one small help control, in one
place, leading to the Learn page for what the screen is for.

**The rule.** The app bar's trailing slot (§4.1) holds the eye on a
secret screen; otherwise it holds the **info button** on any screen
that has a Learn page; otherwise it is empty. One control per screen,
always in the same place, never on a row. The glyph is an "i" in a
circle at the eye's size, one glyph added through `fontbake`; a
question mark reads as something wrong. A Words screen keeps its eye
and no info button: the explanation of words belongs on the step
before it, the Choice that led there. The button opens a specific
Learn page or section, never the Learn menu; the mapping from screen
to page is one table in the core. Back returns to the screen the
button was on, because the page is pushed on the screen stack and the
wizard lives beside the stack, untouched. A Learn page opened this way
draws no "Try it" row: the person is already in the flow. On `wide`
the pane's app bar is the same. Nothing else changes: no prose on
working screens, no captions under rows, no links in tables.

**The mapping, first version.** Create's source Choice → randomness;
Load's source Choice → words (and seed XOR, encrypted backups, Shamir
and Codex32 as their sources are chosen); the passphrase steps →
passphrases; Add a wallet's kind Choice → kinds of wallets; the
multisig, MuSig2, FROST and recovery steps → their pages; the Sign
review → transactions; Addresses and Check an address → verifying;
Export → xpubs and coordinator files; Backup → backups; Keep on this
device → the secure element; the nonce Result → nonces; Tools' rows →
tools. The Learn pages that did not exist — kinds of wallets, FROST,
backups in other forms, coordinator files — are written first.

**In one pass, R4**, after R3: the glyph, the slot rule, the table,
the "Try it" rule, and a test that every screen in the table reaches
its page and returns.

**Pass R4 landed.** The info button, the map behind it, and the return
path. `osk_ui::screens::Chrome` carries an `info: Option<Id>`, and the
two frames every screen goes through — `scrolling` and `fixed`, plus the
Entry screen's own app bar — fill the trailing slot with the screen's own
control where it has one and the info button where it does not, so the
rule is written once and no screen can draw both. `OpenSigner::tap`
answers `ids::INFO` before it answers anything else but the chevron.

**The glyph was already there.** `Icon::Info` is U+E912 of the SeedSigner
icon face, and it is an "i" in a circle at the eye's size: an outlined
ring, the stem and the dot, the same square line box the eye sits in. No
glyph was added and `just fonts` did not run. The entry named Lucide and
Phosphor; this tree has neither, and its glyphs come from the SeedSigner
icon face (U+E9xx) and Font Awesome 6 Free Solid. The glyph already marks
an info-level warning card and an informational Result, which is the same
meaning in the same tone, so §16.90's one meaning per glyph holds rather
than breaks.

**The page could not be pushed the ordinary way.** `push` runs `entered`,
which clears what belongs to the screen being left: the Sign flow, the
message being signed, the address cache, the scanned wallet. A page
pushed from the Sign review would have thrown the transaction away. So
`open_learn_topic` puts the current screen on the stack, sets
`Screen::LearnTopic(page, section)` and resets the scroll, and nothing
else; `back` pops it before every other arm it has. That is what "the
wizard beside the stack is untouched" costs here, and it is why the
chevron from a Learn page is a return and not a cancel. `build_screen`
and `screen()` both answer `LearnTopic` before they look at the wizard,
because a wizard is drawn over the screen rather than beside it.

**Scrolled to its heading.** `osk_ui::screens::Section` gained an
optional id, the view puts `ids::LEARN_HEADING` on the section the map
named, and the frame that placed it scrolls that heading to the top of
the view. The mechanism is the one the path editor already had for its
checked preset row; it now carries an `Anchor`, because a preset row goes
just inside the clip and a heading goes to the top of it. The map names a
section by its place in the page rather than by its heading, since a
heading is a string of `en.rs` and a second copy of it in the map would
be a second place to change.

**The map as built** (`opensigner-core/src/learn_map.rs`, which also
holds the "Try it" association `views/learn.rs` used to match by pointer
identity in a long chain — one `Topic` per page, both directions):

| Screen or step | Page (section) |
|---|---|
| Keys · Add a key · Open BIP-85 child | Seed words |
| A key's page · Backup menu | Backups |
| Open with passphrase · the passphrase steps of Load and Create | Passphrases |
| Keep on this device | The secure element |
| Create › source Choice | Randomness |
| Create › dice, coins, cards, camera, device, mix steps | Where randomness comes from |
| Load › source Choice, "Type the words" or "Scan a SeedQR" checked | Seed words |
| Load › source Choice, "Encrypted backup" checked | Encrypted backups |
| Load › source Choice, "Seed XOR parts" checked | Seed XOR |
| Backup › the encrypted backup's steps | Encrypted backups |
| Backup › the Seed XOR split's counts, sources and result | Seed XOR |
| Wallets · Add a wallet · a wallet's Keys review | Kinds of wallets |
| Add a wallet › kind Choice | Kinds of wallets |
| Add a wallet › Keys, script type, threshold, review | Kinds of wallets (the kind's section) |
| Add a wallet › FROST counts, words, quiz, record | FROST |
| A wallet's page | Kinds of wallets (the kind's section) |
| Addresses, the Address screen, Check an address | Verifying |
| Export | Xpubs and privacy |
| A scanned wallet's review (Inspect) | Coordinator files |
| Sign review, outputs, inputs, warnings, hold | Transactions |
| Sign result for a FROST spend | FROST |
| Sign result "Nonce shared" | Nonces |
| Message sign and check | Signing a message |
| The scanner, wherever it is opened | The air gap |
| Tools | Tools |
| Settings, About, the tiers document | Bitcoin software |

Everything else has no button, and the eye keeps the slot on every
screen that carries one: the Words screen, the Secret screen and word
entry with its panel. The entropy pad draws no eye of its own today, so
the dice and coin steps carry the button and their masked entries are
still shown by holding them.

**What the design got wrong or left unsaid.** "Sign review, outputs,
inputs, stats" names a screen that does not exist: §5 lists "stats" among
the Record screen's users, but the Sign flow has no such step and nothing
was mapped for it. The sources the table wanted for Load — Shamir and
Codex32 — are not rows of the wizard yet (§16.106 built the codecs, not
the screens), so only the four that exist are mapped. Export is "Xpubs
and privacy" alone, because there is no BSMS export to send anyone to
"Coordinator files" from. "Words (the BIP-85 section, if the page has
one)" — it has none, so Open BIP-85 child opens the page. And "Check an
address" opens the scanner before the address field, so the two screens
behind that row are the air gap and then verifying, which reads right and
is not what the table implies.

Three additions the table did not name. The Add a wallet wizard's own
review is the kind's section, like the wallet's page it becomes. The
Backup flow's encrypted and Seed XOR steps are their pages, on the same
ground as Load's sources. And the scanner is the air gap wherever it is
opened, not only from Home, since Scan is the one way in for anything
read.

Deliberately left without a button: the Sign result of a plain signature,
"Which key?", the Load wizard's count, language and words steps, the
quiz, Explore, the word list, the calculators, Decode a transaction, and
every terminal Result. A page opened for one of those would be a page
about something the person has already done.

### 16.106 Seven streams in one hour: the breadth principle 9 asks for, below the screens (2026-09-18)

A one-time exception to one agent at a time: seven implementers ran in
parallel, each in its own worktree, each kept out of the files the R1
pass was rewriting, and the orchestrator merged the seven branches by
hand. What each landed, and what each found the design or the standard
had wrong or left unsaid. None of them reaches a screen yet; the
launcher passes R2 to R4 and the kinds' own passes wire them in.

**Learn.** Four pages — Kinds of wallets, FROST, Backups in other forms,
Coordinator files — and eleven glossary terms, written as
`docs/learn/*.md` and imported into `en.rs`; the threshold section of
Multisig is now the FROST page. The pages are written to §16.104's
model. Two things the design left unsaid: a Liana-shaped recovery
wallet and a watch-only wallet are wallets the device holds and neither
is one of Add a wallet's four kinds, so the page says they arrive as a
descriptor rather than inventing a row; and a page that describes a
flow that does not exist yet says so in one sentence in a section of
its own, which is what "What this device reads today" is. `learn_try`
matches pages by pointer identity in a long chain, which the info
button (§16.105) will want as a per-page association instead.

**SLIP-39.** `osk_bip::slip39` reads and writes the share, does Shamir
over GF(256) with the digest share, recovers two levels, and encrypts
and decrypts the master secret with the four-round Feistel network over
PBKDF2-HMAC-SHA256, which `osk-crypto` now offers beside its SHA-512
twin, tested against RFC 4231 and RFC 7914. The 1024 words are embedded
with their digest. All 45 published vectors pass and each recovered
master secret is the xprv the file states. What a person will get
wrong: SLIP-39 recovers a master secret that is the BIP-32 seed itself,
not entropy to spell as BIP-39 words, so "convert to 24 words" would
hand back a different wallet from Trezor's; and the passphrase is not
checked — every passphrase decrypts to some secret, and only the
checksum and the digest share catch a mistyped set. Open: whether the
device ever creates a SLIP-39 key rather than only reading one, and
that a BIP-39 key cannot be given a SLIP-39 backup of the same wallet,
since the master secret would have to be the 64-byte seed.

**Codex32.** `osk_bip::codex32` writes BIP 93 over BIP-39's
conventions: symbols are `u8` in `0..32`, the string, payload and seed
are fixed-size arrays wiped on drop, text only at the edges. Both
checksums are one BCH code each over a `u128` residue, and recovery and
share derivation are the same Lagrange interpolation over GF(32) applied
to every character position, checksum included. Every inline vector of
the BIP passes. Decided: the payload is held with its checksum; `recover`
refuses the `s` index among its shares while `derive_share` accepts it;
`split` takes the caller's randomness. Error correction is not
implemented, because a correction the person must confirm is a screen;
a failing string is refused and a test walks one substitution through
every position. Open: the screens, whether the identifier is chosen or
typed, and whether a Codex32 backup of a BIP-39 key stores the seed or
the entropy.

**BSMS.** `osk_bip::bsms` reads and writes BIP 129's two records. A key
record's signature is over its four lines joined by newlines with none
at the end — not in the BIP's prose, settled against its vectors — in
the legacy compact form every signer writes, and the check recovers the
key from the signature and compares it whole. A descriptor record is the
same wallet and checksum as its coordinator-config twin, and its first
address is checked as the policy's receive address 0. Pinned down: `/**`
on the keys and the paths line must agree; only `/0/*,/1/*` is
accepted; plain-key records are refused. Encryption is refused by name:
the token is PBKDF2-SHA512 over AES-256-CTR with HMAC-SHA256, and this
tree has neither AES nor HMAC-SHA256. `base64` moved down from
`osk-psbt` to `osk-bip` and is re-exported. Open: the Export that writes
a signer record and the review that shows a descriptor record's first
address, M, N, the paths and this device's position.

**Recovery.** `osk_bip::recovery` makes the wallets Liana writes from a
`RecoveryPolicy` of paths, thresholds and block delays, in `wsh` and in
`tr`. The design assumed the shapes could be written down; they cannot:
Liana states a miniscript policy and the compiler's answer changes shape
with the parts (`or_d` for a one-key primary, `or_i` with the recovery
branch first for a two-of-two primary, another `or_i` for a second
recovery path). So the module states Liana's policy with Liana's 99
against 1 weights and compiles it once when a wallet is made, and the
answer is Liana's descriptor character for character; the addresses of
both taproot shapes were checked against Bitcoin Core 31.1. Two bugs
fell out: a taproot wallet whose paths are all leaves carries an
internal key with no origin, BIP 341's NUMS point over the leaves' keys,
and `check` refused it, so no such wallet could be loaded; and
`spend_paths` offered that key as a way to spend. Both fixed. Open: the
Add a wallet kind, whether a wallet's Keys list shows an unspendable
internal key, and that a policy whose primary and recovery share a key
is refused where Liana refuses only a repeat within one path.

**Taproot multisig.** `tr(NUMS, sortedmulti_a(k, …))` loads, reviews,
derives addresses, verifies change and signs. §16.70 and §16.100 said a
`multi_a` in a tap tree was not a wallet this tree loads; that was half
true and for the wrong reason: `multi_a` was always read as a
`Template::Tree` and the leaf path signed it, and what refused the
wallet was the reader — rust-miniscript has no `sortedmulti_a`, the
form the field writes, and the reader took the internal key for one of
the wallet's keys. Both are read now: a plain internal key stays in the
template's text and is not a key of the wallet, and `sortedmulti_a` is
held as its `multi_a` twin with the leaf rebuilt at every index from
the derived keys in lexicographic order, which BIP 387 defines and an
account-level sort would get wrong. The wallet states whether its key
path is unspendable, its quorum and whether its keys are sorted. BIP 48
assigns no account to a tapscript multisig; the tests use `48'/coin'/0'/2'`
and nothing infers a path. Sparrow's descriptor library has no
`multi_a`, so the field counterparts are Nunchuk and Coldcard. Open: the
Bitcoin Core round trip and its fixtures, a taproot multisig account for
a loaded key, the review's unspendable-key-path row, and the Add a
wallet kind. One BIP 387 vector with a hardened wildcard makes
rust-miniscript panic rather than refuse, recorded in the vectors'
README.

**The USB stick.** Six cases §16.101's watcher covered unobserved are
in `tools/stick-test.py`: two sticks of one label are mounted at
`/mnt/usb/KINGSTON` and `KINGSTON-2`; a stick nobody named at
`/mnt/usb/sdb1`; of the labels a computer can write and a directory name
cannot hold, every one reaches a plain name under `/mnt/usb` and nothing
else, a label of only punctuation falling back to the node name; an
ext4 partition is ignored because `blkid` does not call it vfat, and a
bare FAT filesystem over a whole disk is ignored one step earlier
because the watcher reads `/sys/class/block/*/partition` and a whole
disk has no such file, a rule the entry had not stated; a stick pulled
on its first read leaves the app on screen and nothing mounted. The save
target was the design's mistake: a save goes to `places().first()`, the
device's own medium while it is mounted, whichever stick the transaction
came from, and a `files.rs` test states it. And `wait_for_app` had taken
the firmware's still logo for the app, which hid what looked like the
release image not reaching the app in QEMU at all. It does: run on a
quiet box the release image boots to the app in six seconds and passes
every case, and what the stream saw was the firmware taking longer than
the ninety-second wait under a load average near one hundred while
seven builds ran. The test's stricter wait stays.

**Pass R5 landed.** Two of the seven streams reach the screens, and a
third reaches Scan and Export. Add a wallet's kind Choice is six rows —
Single-sig · Multisig · Taproot multisig · MuSig2 · FROST · Recovery —
and the two new ones build, review, register, list and export like every
other kind.

**Taproot multisig needed no new account.** The brief asked for a
`MultisigScriptType::Taproot` at `48'/coin'/account'/2'`, which is the
account `NativeSegwit` already derives; a second variant with the same
index would have been one path under two names, and `ALL` and `name()`
would have had to lie about which. So a taproot multisig reads a
cosigner at BIP 48's `2'`, as the crosscheck tests already do, and
`build::Kind::SegWit` answers for it. The template is
`tr(<H>,sortedmulti_a(k,@0/**,…))` with BIP 341's NUMS point written out
in the text, where `WalletPolicy` leaves a plain internal key: it is not
a key of the wallet and never appears among its keys. The kind has no
script step — a tapscript multisig is taproot by definition — so Continue
on its Keys step goes straight to "How many must sign?". The review
carries "Key path · unspendable" for any wallet whose internal key is
`H`, which is a fact of the wallet rather than of one key, and the
quorum row comes from `tapscript_quorum` because `quorum()` is `None`
for every `Template::Tree`.

**A recovery wallet is two key lists on one screen.** `WalletWizard`
keeps a second `Vec<BuiltKey>` and a `gather_later` flag that the two
key steps set, so `keys()`, `add`, `remove` and `can_add` all answer for
the step now drawn and one Keys screen serves both paths. The flow is
Now, its threshold where it has more than one key, Later, its threshold
on the same condition, the wait, the form, the review. A key already on
one path is a dimmed row on the other with the reason "on the other
path", because `osk_bip::recovery` refuses a key that spends both ways
and a dead Continue would not say why. The wallet is built by
`RecoveryPolicy::to_wallet_policy`, not by a template, which is the one
place the wizard's `policy()` does not go through `from_parts`.

**Everything with a timelocked alternative is a recovery wallet now.**
`RecoveryPolicy::from_wallet_policy` is asked on every reviewed wallet,
so the kind row, the wallet label and the Wallets subtitle all name
Recovery wherever it recognises one — including `wallet-tree.policy`,
the taproot tree fixture, whose second leaf opens after 4 320 blocks.
That is what the reading is worth: "Miniscript" and "Taproot tree" were
the names for a wallet the device could describe only by its shape, and
a wallet it can describe by what it does should say that instead. Two
tests in `miniscript_wallet.rs` changed to the rule they now encode. The
generic spend-path rows stay under the recovery rows, because the key
rows below are lettered from them.

**The four rows BIP 129 asks for.** A descriptor record read by Scan
becomes the ordinary wallet review with `InspectDoc::bsms` filled in:
the paths line, the record's own first address, and which key of the
list is this device's, by the same letter the key rows carry. The M of N
is the quorum row that was already there. A key record is refused where
it was read, with "a signer's key record, not a wallet"; an encrypted
one with "an encrypted record". Export on a multisig wallet gains "BSMS
record", written from the wallet and the device's own first address, and
the info button on that screen opens Coordinator files rather than Xpubs
and privacy.

**What the design got wrong or left unsaid.** R4's map read a taproot
multisig as `(Template::Tree, Some(_))`, which can never match, so that
kind's section was unreachable; it is `tapscript_quorum()` now. The kind
Choice's order is a fact three tests and the layout walk index into by
number, and inserting two kinds moved FROST from 3 to 4 — the indices
are constants in `wallet_wizard.rs` now rather than literals. "Wallets
row value 2 of 3 · Taproot" turned out to be `wallet_label`, which
already had that shape, so the two new kinds are two arms of it and not a
new row. And a recovery wallet's wait is stated twice, in days on its own
row and in blocks beside it, because days are what a person chooses and
blocks are what the script holds.

**What this pass did not reach, and why.** The SLIP-39 and Codex32 load
sources and the Shamir and Codex32 backups are not built. The Load
wizard is BIP-39 all the way down — `u16` indices into one list, a
`Mnemonic`, a checksum step, a passphrase that is BIP-39's — and SLIP-39
is a second word-entry engine over 1 024 words, gathering share after
share against a header's count, with a passphrase that is not checked;
Codex32 is a bech32 string entry with its own keyboard and its own share
gathering. Either is a pass of its own, and half of either is worse than
none, so the Learn page "Backups in other forms" still says the device
reads neither, which is true. The Export that writes a BIP 129 *signer*
record is not built either: it needs a token typed on the hex keyboard,
a description, and the account node's secret key, which is a flow and
not a format. "Type a number" of days is not offered; the four fixed
waits are. A second recovery path in the wizard is not offered, and the
review already reads a descriptor with two. And the taproot multisig
test checks the wallet's addresses against `osk_bip::tapmulti` rather
than signing a PSBT, because no such fixture is committed and §16.106
had already checked signing against the BIP's vectors.

**Pass D2 landed.** The two things R5 left: a second recovery path in
the wizard, and a wait typed in days. `core/osk-bip/src/recovery.rs`
gains `MAX_DAYS` (455, `MAX_DELAY / BLOCKS_A_DAY`, with the arithmetic
in its doc comment); `build.rs` turns the wizard's one recovery path
into a `Vec<LaterPath>` of keys, threshold and wait, with `gather_later`
an index into it; `views/build.rs` gains the "Another path later?"
Choice, the Days pad and the Delay Choice's fifth row; `lib.rs`, `ids
.rs`, `learn_map.rs`, `strings/en.rs`, `docs/DESIGN.md` §5, `docs/learn/
23-wallet-kinds.md` and three test files follow. Five tests in
`wallet_wizard.rs`, and the info map's recovery walk in `info.rs`.

**Three paths, each opening after the last.** `MAX_WIZARD_PATHS` is 3.
The wizard asks "Another path later?" after every wait — No checked, as
a wallet with the paths it has is finished — and stops asking once
there are three, or once a path already waits the longest a timelock can
state. Each later wait must be longer than the one before it: the codec
refuses two paths that open on the same block, and a path that opens
before an earlier one is an earlier one, so the rows at or below the
previous wait are dimmed with "not after 30 days" and the pad refuses
the same number on its error line. That makes the wizard's order the
codec's order, so `recovery` is written shortest wait first without
sorting and the review reads the two the same way.

**The typed wait is a pad, not an entry.** "Type a number of days" opens
a §5 Pad on `KeyboardKind::Pin` with a count field: three digits at most,
✓ live from 1 to 455, the blocks the number is on the caption line, the
refusal on the same line when there is one. A Pad rather than an Entry
because the value is a count, which is what the dice and PIN pads are,
and because the digits keyboard is the whole alphabet a day count has.

**What the design got wrong or left unsaid.** The brief asked for the
blocks line and the error line as two lines; §4.3's Pad reserves one
caption line under the field, and it carries the blocks when nothing is
wrong and the refusal when something is. Neither would fit above the
other without a second reserved line on every pad in the product, so
they share the one. The brief also did not say what the wait of a path
after a 365-day one defaults to, and there is no offered wait longer:
such a path opens on the typed row a day later, which keeps §4.2's rule
that a question opens on a default that can be taken. And a wait of zero
days on the first path is refused with "not after 0 days" rather than a
bound of its own, because the floor on every path is the wait before it
and the first path's is zero.

**Pass D3 landed on 2026-09-19:** the Bitcoin Core round trip behind the
taproot multisig, which R5 left as a fixture task. Bitcoin Core 31.1 on
regtest imported `tr(H,sortedmulti_a(2,…))` over the three fixture keys
at BIP 48's `2'` account, watch-only, funded it, and built the spend;
this tree signed with two of the three keys; Core finalized that PSBT
and `testmempoolaccept` allowed txid
`42096a2e92f0ae357cef0e3403cb528876e7f1fe9ab0dcbd629016d697efe64d`, 206
vB, 821 WU, 1 550 sat, effective fee rate 0.000 075 24 BTC/kvB. The
fixtures are `wallet-tapmulti.policy`, `wallet-tapmulti-first.psbt` and
`wallet-tapmulti-signed.psbt`, the run is
`tools/scripts/tapmulti-regtest.sh`, and the signed one is rewritten by
`cargo run -p osk-psbt --example tapmulti`, now part of
`just psbt-fixtures`.

**Core needs no help with `sortedmulti_a`.** The brief allowed for Core
refusing the sorted form and the run falling back to `multi_a` with the
keys pre-sorted. Core 31.1 imports `sortedmulti_a` as written, and the
checksum it computed, `#uygt73j8`, is `osk_bip::descriptor`'s, so the
descriptor this tree writes is the descriptor Core reads. The address
Core funded is `WalletPolicy::address_at(regtest, receive, 0)`, which is
a second reading of the sorted-at-derivation rule BIP 387 states and
§16.106 implemented.

**Core writes an origin for the NUMS point.** The input's
`taproot_bip32_derivs` holds four entries, not three: one per wallet key
at `m/48h/1h/0h/2h/0/0` naming the leaf hash, and a fourth for the
internal key itself — master fingerprint `7c461e5d`, path `m`, empty
`leaf_hashes`. Core treats the descriptor's raw internal key as a key
with an origin of its own, although nobody has it and it can never sign.
Neither BIP 371 nor BIP 387 says an unspendable internal key should be
left out, so this is Core being consistent rather than wrong, and it is
what a reader must expect. `WalletPolicy` already keeps a plain internal
key out of the wallet's keys, so the reader passed it over unchanged; a
reader that counted taproot origins to count cosigners would read this
wallet as four of five. The rest of the input is what the design
predicted: one `tap_scripts` entry holding the `multi_a` leaf over the
three derived x-only keys in sorted order at leaf version 192, and a
33-byte control block that is the NUMS point with the parity bit and no
Merkle path, the tree being one leaf.

**Signing and finalizing are two layers, and the fixture is the seam.**
`osk_psbt::sign` writes one `tap_script_sig` per key under the leaf's
hash and finalizes nothing, so the committed signed fixture is not
final: it is what a coordinator collecting signatures is handed, and
Core finalizes it. The Sign flow finalizes on top of that, because two
of three satisfies the leaf, so the device hands over the finished
transaction rather than a PSBT. A third key on the same device signs too
— `sign` does not stop at the threshold — and the finalizer drops the
spare, reaching the same txid Core accepted.

**The `2'` account still stands, checked again.** BIP 48 defines script
types `1'` (p2sh-p2wsh) and `2'` (p2wsh) and nothing for taproot;
Sparrow's descriptor library has no taproot multisig and no `3'`; and
Coldcard's released firmware (`shared/multisig.py` on master) has p2sh,
p2wsh and p2sh-p2wsh only. Checked 2026-09-19: no standard and no
shipping counterpart assigns a taproot multisig account, so the `2'` R5
chose stands, and if a `3'` convention is published the reader takes
both.

### 16.107 SLIP-39 on the screens: a key of its own kind, in groups and thresholds (2026-09-18)

**Why.** `osk_bip::slip39` landed below the screens (§16.106) and pass
R5 left the screens for a pass of their own, because a SLIP-39 share is
a second word entry over a second list. The owner's ask is the whole
protocol, not the 2-of-3 case: several groups, each with its own count
and threshold, and a threshold over the groups. Principle 9 says so too.

**What SLIP-39 is to this device.** Two facts decide the shape. The
master secret a set of shares gives back is the BIP-32 seed itself, 16
or 32 bytes, and the codec takes nothing else; a BIP-39 key's seed is 64
bytes, and its entropy read as a master secret is a different wallet on
every SLIP-39 device. So a SLIP-39 backup of a BIP-39 key is not
offered, ever, and Learn says why. And a SLIP-39 key has no words: it is
its master secret, and shares are the only form it is ever written in.
That makes it the first key with no mnemonic, which every screen that
assumes words has to answer for.

**The model, as rules.**

1. **A SLIP-39 key is a key.** One row of Keys with its fingerprint,
   which is the master fingerprint of `Xpriv::new_master(secret)`; the
   Key row of its page says "SLIP-39 shares" or "SLIP-39 shares and a
   passphrase"; Open BIP-85 child works on it as on any key; Open with
   passphrase is not offered on it, because SLIP-39's passphrase is
   applied when the shares are read (rule 3), not to a key already
   loaded. It holds its master secret sealed as a seed is, no words,
   and nothing about the shares it came from: the identifier and the
   group plan are facts about one backup, and a new backup gets new
   ones.
2. **Load a key › SLIP-39 shares.** The source Choice gains the row.
   Then "How many words?" with 20 and 33, which is what Trezor asks and
   what the mnemonic's length states. Then the words of a share, on
   the same Entry screen as BIP-39 words, with candidates from the
   1024-word list by its four-letter prefixes, "Words so far" above,
   the title "Share 1 · Word 7 of 20". After the last word the share is
   checked — the RS1024 checksum, then that it belongs with the shares
   already in: same identifier, same group threshold and count, not a
   member already entered — and a share that fails is refused on the
   Result with the codec's reason, Back returning to the words. A share
   that passes is the Result "Share accepted" whose table states what
   the set now has and needs: "Groups · 1 of 2 needed", then one row
   per group seen, "Group 2 · 1 of 3 shares". Continue asks the next
   share until the set is enough for `recover`; it never asks for more
   than enough, and a set whose groups are complete goes on. Then "Add
   a passphrase?" and the passphrase as BIP-39's steps, with the two
   fingerprints computed by decrypting the encrypted master secret
   under no passphrase and under the typed one; then Confirm, then the
   PIN when it is the session's first key.
3. **The passphrase is SLIP-39's.** It is not checked: every passphrase
   opens some wallet, which is what the PassphraseConfirm's two
   fingerprints exist to show. A SLIP-39 key loaded under a passphrase
   is a passphrase key for Tier B (§16.104 rule 7): never in the blob.
4. **Add a key › Create SLIP-39 shares.** A row of its own on Add a
   key, after Create a key, and a distinct flow: the owner's direction
   is that making words and making shares are never mixed on one
   screen, as Trezor mixes them under one "backup type" question
   (2026-09-18). The row opens the Create wizard in a SLIP-39 mode
   with no "Written as?" step: the source Choice, then "How many
   words?" with 20 or 33 (128 or 256 bits), no language step, the
   entropy steps and the sanity step the same, the BIP-39 math steps
   skipped, and "Add a passphrase?" asked before the shares are made,
   since the shares carry the secret encrypted under it. Create a key
   stays the words flow and offers every BIP-39 count — 12, 15, 18,
   21 and 24 — as Load does; the three middle strengths are a pass of
   their own after S2. Then the plan: "How many groups?" (1 to
   16, 1 first); with one group, "How many shares?" and "How many must
   be present?" as the FROST counts are worded; with more, "How many
   groups must be present?" and then, per group, its two counts; a
   group of one share has threshold 1 and the codec allows no other.
   Then every share as a Words screen with the quiz, the title "Group 1
   · Share 2 of 3", exactly as the FROST deal shows its computed keys;
   then Confirm. The identifier is fresh from the shell's entropy, the
   backup is extendable, and the iteration exponent is 1, which is what
   Trezor writes; a pass that finds the reference's defaults differ
   follows the reference and says so.
5. **Backup of a SLIP-39 key.** The Backup menu offers "SLIP-39 shares"
   and nothing that needs words: no quiz of words, no SeedQR, no grid,
   no steel, no Seed XOR. The row runs rule 4's passphrase offer and
   plan over the key's master secret and shows the shares; the result
   is a new backup with a new identifier, and the old shares still
   open the same key. The encrypted backup (`osk-backup`) carries words
   and does not carry a SLIP-39 key in this entry; that is a format
   change and its own pass.
6. **Tier B.** A kept SLIP-39 key is its master secret in a keys-record
   slot, and the blob does not change size: a slot is tagged by what it
   holds, or the version moves to 5, and the pass says which and why. A
   restart brings it back as any kept key.
7. **The word entry is one engine.** The Load wizard's prefix search,
   candidate strip, two-tap rule, "Words so far" panel and keyboard mask
   are written once over a list, and the SLIP-39 list is a second list
   for the same code, not a second copy of it. The BIP-39 final-word
   filter and the "which word is wrong" diagnosis are BIP-39's: a
   SLIP-39 share is checked whole after its last word.

**Rejected.** A SLIP-39 backup of a BIP-39 key, as above. Asking the
identifier or the exponent on a screen: Trezor asks neither, and a
person who wants them has the reference implementation. Converting a
recovered master secret to BIP-39 words: a different wallet with the
same bytes, which the Learn page already warns about. A scanner route
for a share: no standard writes one as a QR, so the shares are typed.

**In two passes.** S1: the key kind, the entry engine over two lists,
Load a key › SLIP-39 shares through the passphrase and Confirm, the key
page and Backup menu for a key without words, Tier B's slot, every
`has_mnemonic` site answering for such a key, the info map, DESIGN §5.
S2: Add a key › Create SLIP-39 shares, the plan Choices, the shares with the quiz,
Backup › SLIP-39 shares, and Learn's last section saying what the device
reads and writes.

**Open in this entry.** Whether Explore should show a SLIP-39 key's
seed hex under the words it does not have. Whether the encrypted backup
should learn to carry a master secret. Whether "Type a share" belongs
among the scanner's ways in when a share is on the clipboard.

**What would reopen it.** A standard QR form for a share; a SLIP-39
device accepting a 64-byte master secret, which would make a backup of
a BIP-39 seed meaningful; a request for shares over 16 per group, which
the format cannot carry.

**Pass S1 landed (2026-09-18).** Load a key › SLIP-39 shares, the key
kind, the entry engine over two lists, the key page and Backup menu for a
key without words, Tier B's slot and the info map.

Files: `core/osk-crypto/src/sealed.rs` (`SeedBytes`), `core/osk-bip/src/
keys.rs` (`MasterKey::from_seed_bytes`), `core/osk-bip/src/slip39.rs`
(`MAX_DISPLAY_CHARS`), `opensigner/opensigner-core/src/load.rs`
(`EntryList`, `Source::Slip39`, `ShareSet`, `ShareRefusal`), `finish.rs`
(`Material`), `keep.rs` (`KeptSecret`), `lib.rs`, `learn_map.rs`,
`text.rs`, `ids.rs`, `strings/en.rs`, `views/load.rs`, `views/detail.rs`,
`views/backup.rs`, `views/keep.rs`, `views/explore.rs`,
`tests/slip39_key.rs` (new), `tests/info.rs`, `tests/common/mod.rs`,
`docs/DESIGN.md`, `docs/learn/25-other-backups.md`.

**The seed is one door, length-carrying.** `osk_crypto::SeedBytes` is a
sealable seed of 0 to 64 bytes; `LoadedKey` holds one of them whatever
the key was made from, and `MasterKey::from_seed_bytes` takes it. A key
carrying a master secret beside a seed it does not use would have given
every reader of the seed two things to ask about and every writer a
choice to get wrong; one door costs one new type and one new
constructor. `from_seed` stays for the sixty-odd callers that have a
64-byte BIP-39 seed in hand.

**The blob slot is tagged, not versioned.** A slot says what it holds by
its word count, at byte 48: 12 to 24 is a mnemonic, 0 is a SLIP-39 key
whose secret's length is at byte 49 and whose bytes are the first 16 or
32 of the slot. `BLOB_LEN`, every record size and `VERSION` 4 are
unchanged. Version 5 was the alternative and was rejected because
`header` refuses every earlier version: a bump would make every blob
already on a device unreadable, which is a real cost to every owner for a
key kind most will never load. The tag is unambiguous because no
occupied slot this build ever wrote has a word count of zero.

**What the design got wrong or left unsaid.**

- Rule 2 calls the refusal "the Checksum Result". A share that is not of
  the set fails no checksum, so the screen is titled "Share refused" and
  the codec's `Error` display is its result line; the checksum failure is
  the same screen with the same shape. One title that is true of both
  beat two screens that differ only in wording.
- Rule 2 names four strings for the accepted Result and the table needs
  six: the two labels carry templates of their own
  (`load_share_groups_value`, `load_share_group_value`), and the Result
  component needs a line under its title that is not the title again
  (§2.1), which is `load_share_enough` / `load_share_more`.
- The entry knows nothing about how many shares a set will need, so the
  wizard holds `MAX_SHARES` = 16 and refuses the seventeenth by name.
  `slip39::recover` wants *exactly* the threshold groups and exactly
  each group's threshold of shares, so the wizard keeps every consistent
  share and hands `recover` the subset it takes; room for every share the
  format allows to be present (16 groups of 16) would be some 33 KB of
  secret in the wizard for a plan nobody types.
- The step indicator still counts the language step a SLIP-39 share never
  sees, so its run of steps is 1, 2, 4, 5, 6, 7 of 7. `Step::number` is
  a method on the step and does not know the source; fixing it is a
  change to the indicator's contract, not to this pass.
- Nothing said what the key page's Backup row should show for a key with
  no quiz to pass. It shows the row with no value: the menu behind it now
  has the SLIP-39 row, and "not verified" in the caution tone would be a
  state the person cannot act on.
- §16.106's Explore question is answered by leaving the words rows off
  and keeping the seed hex, which is what the open item asked about.

**What S2 still has.** Create › "Written as?" and the plan Choices, every
share as a Words screen with the quiz, Backup › SLIP-39 shares behind the
dimmed row this pass draws, and the last section of Learn's "Backups in
other forms" when the device also writes shares.

**Pass S2 landed (2026-09-18).** Add a key › Create SLIP-39 shares,
Backup › SLIP-39 shares, the plan Choices, the randomness a split is fed
and every share as a Words screen with the quiz.

Files: `core/osk-bip/src/slip39.rs` (`encrypt`, `decrypt`, a public
`DIGEST_LENGTH`), `opensigner/opensigner-core/src/shares.rs` (new: the
plan, the randomness and the shares), `create.rs` (`CreateWizard::shares`,
`Step::Shares`, the master secret, the share gatherer), `finish.rs`
(`Material::Secret`), `backup.rs` (`BackupStep::Shares`), `quiz.rs` (over
`EntryList`), `load.rs` (`EntryList::word_count`, `word`, `candidates`),
`lib.rs`, `learn_map.rs`, `ids.rs`, `strings/en.rs`, `views/shares.rs`
(new), `views/words.rs`, `views/quiz.rs`, `views/create.rs`,
`views/backup.rs`, `views/build.rs`, `views/explore.rs`, `views/home.rs`,
`threshold.rs`, `tests/slip39_create.rs` (new), `tests/info.rs`,
`tools/lint-secrets.sh`, `docs/DESIGN.md`,
`docs/learn/25-other-backups.md`.

**Two flows, not one question.** The owner's direction during the pass
replaced the "Written as?" step this entry first described: making words
and making shares are two rows on Add a key and two flows behind them,
never one screen asking which, as Trezor asks. `CreateWizard::shares()`
bends the wizard the way `combining()` and `gathering` already do — the
same source Choice, the same entropy and sanity screens, no language
step, no BIP-39 math, and an entropy that is kept as the master secret
rather than spelled into words.

**The defaults, and what stands behind them.** Extendable, iteration
exponent 1, and an identifier of 15 bits that is fresh per backup.
`python-shamir-mnemonic` is not vendored here and nothing may be
fetched, so `generate_mnemonics`' signature could not be read in this
pass; what is in the tree is `tools/vectors/slip39/vectors.json` and its
README, which say that cases 42 to 45 are extendable and the other
forty-one are not. That is a file written over several years, not a
statement of the current default, so the choice rests on §16.107 rule 4
and on one property extendable has that matters here: an extendable
backup's master-secret encryption is not salted with the identifier, so
the two fingerprints the passphrase step shows cannot change when the
split is given its identifier later. A pass that reads the reference and
finds its defaults differ should follow the reference and say so; that
check is still open.

**The identifier comes from the session key, not from the split.** It is
public and printed on every share, so it is not the randomness §16.92 is
about. It is the first fifteen bits of SHA-256 over the session's
scramble seed, the clock and a count of the backups this session has
made, so two backups of one key are two backups — which is what the test
asserts — and the shell's own entropy is what the session key rests on.

**One plan, one array, 37 KB.** `SharePlan` is a field of `OpenSigner`
rather than of either flow, because Create and Backup ask the same
questions over a different master secret and the shares should exist
once. It holds `[Share; 256]` — sixteen groups of sixteen, which is the
format's own bound — at 110 bytes a share, `[[u8; 32]; 255]` for the
random values a split can need, and the master secret and passphrase
inline: 36,736 bytes, one allocation-free field, zeroized when the flow
is left and on drop. `slip39::split` writes straight into that array, so
nothing is copied and no temporary of the same size exists.

**The randomness is counted, not trusted.** The plan works out how many
random values the split needs — `group_threshold − 1` at the group
level, `threshold − 1` in each group — gathers exactly that many through
the source's own screens, and feeds `split`'s closure one value per
draw. It then checks three things before keeping the shares: that no
draw fell back on zeros, that every gathered value was drawn exactly
once, and that the bytes drawn equal `(t − 2) · len + len −
DIGEST_LENGTH` summed over the thresholds. `DIGEST_LENGTH` became public
for that sum. A mismatch zeroizes the plan and cancels the flow rather
than showing a share whose randomness the device cannot account for.

**What the design got wrong or left unsaid.**

- Rule 4 says "the entropy is the master secret" and leaves the
  passphrase's arithmetic open. Two fingerprints are wanted before any
  share exists, and what a reader of the shares gets is
  `decrypt(encrypt(secret, typed), theirs)` — the secret itself under
  the typed passphrase, another wallet under any other.
  `Material::Secret` computes both in `Finish::build`, which runs on the
  step that asks for them and not on any frame that draws them.
- Nothing said where the Backup path's passphrase lives. It is not the
  encrypted backup's, which is typed twice and checked, and it is not a
  `Finish`, which would bring a confirm and a PIN that do not belong
  here. The plan owns its own entry, and ✓ goes straight on, because
  SLIP-39 checks nothing.
- The brief's info-map row for "Create SLIP-39 shares" has nowhere to
  go: Add a key is one screen with one page, and it is about keys. The
  flow behind the row carries the row instead — the word count and every
  Choice of the plan open "Backups in other forms" at its SLIP-39
  section, and the source Choice keeps the randomness page it shares
  with Create a key.
- The share Words screens are in the map and will not draw the button: a
  Words screen carries the eye, and §16.105 gives one screen one control.
  The Choice before them is where the explanation is reached.
- `Quiz` had to move from `Language` to `EntryList`, which also moved its
  arrays from BIP-39's 24 words to a share's 33 and its decoy draw from
  2048 words to the list's own length.

**Still open in this entry.** Reading the reference's own defaults, as
above. Whether the plan should offer a total over sixteen shares without
a screen that says what it will cost to write them. Whether Back inside
the gathering should return one random value at a time rather than
dropping the run, as it drops a Seed XOR split's.

### 16.108 Create a key offers every BIP-39 count (2026-09-18)

**Why.** Create a key's "How many words?" has offered 12 and 24 since
the wizard was built, with 15, 18 and 21 dimmed, while Load a key has
taken all five since §16.12 and §8.1 row 9 marked the three middle
counts for v1. The owner noticed the gap beside the SLIP-39 work
(2026-09-18). The cause is `osk_entropy::Strength`, which knows two
strengths and gives every source its count of rolls, flips, digits,
cards and frames from them.

**The rule.** A new key can be any BIP-39 length. `Strength` gains 160,
192 and 224 bits, and each source's "needed" follows from the bits as
it does for the two it has: dice at log2 6 bits a roll (62, 75 and 87
rolls, rounded up as 50 and 99 were), coins one a flip, hex two digits
a byte, cards and camera frames by the rule each already states.
`Strength::for_words` answers for 15, 18 and 21; the count Choice's
five rows are all live in Create's order (12, 24, 15, 18, 21, as Load
lists them); the sanity thresholds that are stated per strength are
stated for the three new ones by the same reasoning; a Seed XOR part of
such a key is the key's length, 20, 24 or 28 bytes, as the split already
takes from the entropy; a SLIP-39 key's two strengths are untouched.
Learn's "Why 12 or 24" says the three between exist, are rarer, and are
read by every wallet that follows BIP-39.

**Not in this entry.** BIP-85 child counts (12, 18, 24 are BIP-85's own
and stay), SeedQR and CompactSeedQR (the standard's counts are its own),
the steel and grid helpers, whose rows already follow the loaded key's
count.

### 16.109 Codex32 on the screens: a string typed, a seed kept, a backup for every key (2026-09-18)

**Why.** `osk_bip::codex32` landed below the screens (§16.106) and
every pass since left it for one of its own, because a codex32 string
is a bech32 entry with its own share gathering. This entry gives it the
shape §16.107 gave SLIP-39, with the differences BIP 93 makes.

**What Codex32 is to this device.** A codex32 string encodes a BIP-32
master seed of 16, 20, 24, 28, 32 or 64 bytes, and the seed is the key:
the same door `SeedBytes` opened for SLIP-39. Three facts follow. A key
loaded from codex32 has no words and is "Codex32" on its page. Every
key this device holds has a seed one of those lengths — 64 bytes from
BIP-39 words, 16 or 32 from SLIP-39 shares, any of the six from codex32
— so **Backup › Codex32 is offered for every key**, and BIP 93 says so
itself: it is "semi-convertible" with BIP-39's 512-bit seed and warns
that the string is long (127 characters). Learn says what that backup
is: the wallet, not the words, with the passphrase inside, readable by
any codex32 device and by nothing that expects words. And the seed of
a BIP-39 key can also be split as SLIP-39 only when it is 16 or 32
bytes, which it never is; §16.107's refusal stands unchanged, and a
codex32 key of 16 or 32 bytes gets both backups.

**The model, as rules.**

1. **A Codex32 key is a key.** Its fingerprint is the master's over the
   seed; its page's Key row says "Codex32"; Open BIP-85 child works;
   Open with passphrase is not offered (BIP 93 has no passphrase);
   Backup offers Codex32 and, for a 16- or 32-byte seed, SLIP-39
   shares. It is kept on Tier B as a SLIP-39 key is, in the tagged
   slot, when its seed fits the slot's 48 bytes; a 64-byte codex32 key
   is not kept and shows no Keep row, which is recorded as open below.
2. **Load a key › Codex32.** The source Choice gains the row. Then one
   Entry screen: the string on the address keyboard's bech32 layer with
   no base58 shift and no `1`/`b` prefix keys, `ms1` already in the
   field, the typed characters shown in groups of four, keys that
   cannot continue a string dimmed, ✓ live when the length is one BIP
   93 allows and the checksum passes, the error line stating the
   codec's reason otherwise. A string whose index is `s` is the secret
   and goes on; a share is the Result "Share accepted" — "Shares · 1 of
   3 needed" — and Continue asks the next until `k` are in; a share of
   another set (identifier, threshold or length differ) or an index
   already entered is "Share refused" with the codec's reason. Then
   Confirm and the PIN. No passphrase steps: BIP 93 has none.
3. **Scan reads a codex32 string.** `classify` gains the `ms1` prefix,
   and a scanned or typed string routes to rule 2's entry with the
   string filled, since a codex32 string is text a QR can carry. A
   pasted one is refused as pasted words are: it is a seed, and §4.10
   says a secret is never pasted.
4. **Add a key › Create Codex32 shares.** A row of its own after
   Create SLIP-39 shares, the owner's rule of distinct flows (§16.107
   rule 4). The source Choice; "How long a seed?" with 128, 160, 192,
   224 and 256 bits (512 has no source that makes it and is not
   offered); the entropy and sanity steps; then "Split into shares?"
   (No, one string · Yes), and if yes "How many shares?" (2 to 31) and
   "How many must be present?" (2 to 9, and never more than the
   shares); then "Random shares from?" and the gathering per random
   share, as §16.92 and §16.107 rule 4 have it; then every string on a
   Secret screen in groups of four, each followed by "Type it back",
   the same entry as rule 2 comparing what is typed with what was
   shown, skippable with the quiz's caution, `backup_verified` when
   none was skipped; then Confirm and the PIN. The identifier is the
   first four bech32 characters of the master fingerprint's bech32
   encoding: BIP 93 leaves it undefined and asks only that it be
   distinct per seed, and a computed one is never mistyped.
5. **Backup › Codex32.** For every key: the split question and its
   counts, the randomness, the strings with "Type it back", and the
   Result "Strings made" naming the plan and the source. For a key with
   words the Result also states, as a row, that the backup is the seed
   — "Holds · the seed, not the words" — since that is a fact of the
   backup a person copying it needs, not an explanation.
6. **The entry is one engine.** The bech32 field, its grouping, its
   key mask and its checksum line are written once and used by Load,
   "Type it back" and the scanner's Type.

**Rejected.** Error correction: BIP 93's code can correct up to four
substitutions, but a correction a person must confirm is a screen of
its own, and a wrong correction silently confirmed is worse than a
refusal; it stays a later pass (§8.2). Asking the identifier: never
mistyped when computed. A 512-bit Create: no entropy source makes it.

**In two passes.** C1: the key kind, the entry engine, Load a key ›
Codex32 with share gathering, Scan's route, Tier B, the key page and
Backup menu rows, the info map, DESIGN §5, tests on BIP 93's vectors.
C2: Create Codex32 shares, Backup › Codex32 for every key, "Type it
back", Learn's Codex32 section and its last section.

**Open in this entry.** A 64-byte codex32 key on Tier B, which needs a
wider slot and therefore a blob version. Error correction. Whether a
BIP-39 key's Backup should say on its menu row that Codex32 holds the
seed, or only on the Result.

**Pass S3 landed (2026-09-18).** Create a key offers 12, 15, 18, 21 and
24, and every source counts up to whichever was chosen.

Files: `core/osk-entropy/src/lib.rs` (`Strength::Bits160`, `Bits192`,
`Bits224`, `ALL`, `for_bytes`, and `rolls`, `words`, `CardDraws::needed`
and `CameraNoise::needed` derived rather than matched),
`opensigner/opensigner-core/src/views/create.rs` (the count Choice),
`create.rs` and `backup.rs` (doc comments), `docs/learn/02-words.md`,
`05-randomness.md`, `06-where-randomness.md` and `strings/en.rs`,
`tests/create.rs`, `tests/xor.rs`, `tests/backup_helpers.rs`,
`tests/slip39_create.rs`.

**The counts, and the arithmetic behind each.** Dice: the bits over
log2 6, rounded up — 50, 62, 75, 87, and 99 at 256 bits, which is the
one exception, since the rule gives 100 and 99 rolls carry 255.91 bits,
the count Coldcard and SeedSigner take. Coins: one flip a bit, 128 to
256. Hex: two digits a byte, 32 to 64. Cards: the fewest draws whose
bits reach the strength, 25, 31, 39, 50 and 58 — the counts climb faster
than the lengths do because a draw is worth less as the deck empties.
Camera: three frames per 128 bits, rounded up, 3, 4, 5, 6 and 6, which
is the rule that already gave three and six. The device and a mix both
truncate a SHA-256 digest and needed nothing but the byte count.

**What the design got wrong or left unsaid.**

- The entry said the sanity thresholds "stated per strength" would be
  stated for the three new ones. There are none. `DICE_LONG_RUN`,
  `DICE_SEQUENTIAL`, `DICE_CHI_SQUARE_LIMIT` and `COIN_LONG_RUN` are
  properties of a sequence, not of a length, and `DICE_SKEW_MIN` is a
  floor of fifty rolls below which chi-square means nothing — every new
  length asks for more than fifty, so the check is live for all of them.
- The strengths are now derived, not matched. `words` is the bits plus a
  checksum bit per four bytes over eleven bits a word; `rolls` counts up
  against `BITS_PER_ROLL`; `CardDraws::needed` counts up against
  `card_bits`; `CameraNoise::needed` scales `CAMERA_FRAMES_128`. A
  five-armed `match` per source would have been five places to state the
  same rule, and the two counts the sources already had are what the
  derivations reproduce.
- `views/create.rs::count_step` drew `load::COUNTS` rather than the
  wizard's own `counts()`, and the filter to 12 and 24 was hiding the
  consequence: in the SLIP-39 mode §16.107 added, the screen labelled
  its two rows 12 and 24 while the taps set 20 and 33. Dropping the
  filter made that visible, so the view now reads `w.counts()` and a
  SLIP-39 share's question says 20 and 33.
- Nothing said what `RawEntropy::strength`, `RawHex::strength` and
  `SeedXor::strength` should do with a length that is none of the five.
  They go through one `Strength::for_bytes`, and `SeedXor::push` refuses
  a part whose length is not a BIP-39 one, which is what the old
  `!= 16 && != 32` did for the two lengths it knew.
- `MAX_ROLLS`, `MAX_FLIPS` and `MAX_HEX` already hold the new counts (87
  of 200, 224 of 256, 56 of 64), and `MAX_CARDS` is still two decks,
  because 58 remains the largest count.
- Learn's counts would not fit one sentence in three places, and the
  Markdown the Learn pages are written in has no tables, so each page
  gained a short paragraph listing the counts rather than a table.

**Not done here.** BIP-85's child counts, SeedQR and CompactSeedQR, and
SLIP-39's two strengths are untouched, as the entry says. The step
indicator still counts the language step a SLIP-39 share never sees
(§16.107's open item).

**Pass C1 landed (2026-09-18).** Load a key › Codex32, the key kind, the
entry engine, Scan's route, Tier B's slot, the key page and Backup menu
rows, the info map and DESIGN §5.

Files: `core/osk-bip/src/codex32.rs` (`Default` for `Codex32`),
`core/osk-ui/src/widgets/keyboard.rs` (`KeyboardKind::Codex32`),
`core/osk-ui/src/state.rs`, `core/osk-codec/src/classify.rs`
(`PayloadKind::Codex32`), `opensigner/opensigner-core/src/codex32.rs`
(new: `Codex32Entry`, `codex32_keys`), `load.rs` (`Source::Codex32`,
`KeyKind`, `LoadedKey::seed_len`, `from_codex32`), `finish.rs`
(`Material::Seed`), `keep.rs` (`keeps_secret`, the tagged slot's kind
byte), `lib.rs` (`codex32_scanned`, the kept keys), `learn_map.rs`,
`text.rs` (`grouped`), `ids.rs`, `strings/en.rs`, `views/load.rs`,
`views/detail.rs`, `views/backup.rs`, `tests/codex32_key.rs` (new),
`tests/info.rs`, `tests/common/mod.rs`, `tools/lint-secrets.sh`,
`docs/DESIGN.md`, `docs/learn/25-other-backups.md`.

**The keyboard is a kind, not a mode of the address one.** A keyboard's
layers are the shift and symbol keys the person presses; which keys exist
at all is the kind. A codex32 string has no `1`, no `b` and no base58
behind a shift, and that never changes while it is typed, so a mode would
have been a flag threaded through `rows`, `keys` and every caller to say
something fixed. `KeyboardKind::Codex32` is one arm in the three
functions that switch on kind, and it shares `key_bit`'s address
numbering, so one mask serves both and `codex32::codex32_keys` reads
beside `verify::address_keys` without a second numbering to keep in step.

**The slot is tagged twice.** A codex32 key is kept where a SLIP-39 key
is: the word count at byte 48 is zero, byte 49 is the seed's length, and
the seed is the first bytes of the slot. Two things changed. The lengths
a slot reads are now BIP 93's — 16, 20, 24, 28 and 32 — and 64 is not
among them, so a 512-bit codex32 key is not keepable and its page draws
no Keep row. And byte 47 says which kind the seed is, 0 for SLIP-39 and 1
for codex32, because a 16- or 32-byte seed could be either and a key that
came back as the wrong kind would state the wrong fact on its page and
offer the wrong backup. Byte 47 is past the longest seed a slot holds and
is zero in every slot written before this pass, which is what SLIP-39
reads as. `BLOB_LEN`, every record size and `VERSION` 4 are unchanged.

**What the design got wrong or left unsaid.**

- Rule 2 says the characters are drawn in groups of four and does not say
  where the string itself lives. A 127-character string does not fit a
  field, so the screen takes the address entry's shape: the string whole
  above the entry group, which is §4.3's one value whose tail is not
  enough, and the field under it in the same groups of four.
- `LoadedKey` had no way to answer "which backups?" without unsealing the
  seed, which the Backup menu and the Keep row both ask before anything
  is opened. The length is a plain field now: how long a seed is is not a
  secret, and every reader of it was asking a question about the kind of
  key rather than about its bytes.
- Rule 1 says Backup offers Codex32 for every key. A row whose flow does
  not exist is dimmed, and §4.11 gives a dimmed row a reason; this one
  has none until C2, exactly as S1 drew SLIP-39's row. A SLIP-39 key's
  menu now carries the same dimmed row.
- §4.10 says a secret is never pasted, and `is_secret_payload` refuses
  pasted words, a SeedQR and an xpriv. Rule 3 said a pasted codex32
  string routes to the entry, which contradicts that rule: a codex32
  string is a seed. The pass as landed refuses the paste as it refuses
  pasted words, and the string reaches the entry from the camera and
  from the scanner's Type; rule 3 is read that way from now on.
- Nothing said what the threshold `0` does to the keys. It fixes the
  index: after a `0` the only live key is `s`, which is the codec's
  `SecretIndexRequired` stated as a dimmed keyboard rather than as an
  error a person reads after typing 48 characters.
- The refused Result carries no rows. SLIP-39's states the word count;
  a codex32 refusal has nothing to state but the codec's reason, and the
  string it is about is one chevron away.
- The step indicator counts worse here than it does for SLIP-39: a
  codex32 load runs 1, 4, 5, 7 of 7, because there is no count, no
  language and no passphrase. `Step::number` still does not know the
  source, which is §16.107's open item unchanged.

**What C2 still has.** Add a key › Create Codex32 shares and Backup ›
Codex32 for every key, behind the dimmed row this pass draws; "Type it
back" through the same `Codex32Entry`; the identifier computed from the
master fingerprint; the Result's "Holds · the seed, not the words"; and
Learn's Codex32 section, whose last section now says the device reads
codex32 and does not yet write it.

**Pass C2 landed (2026-09-19).** Add a key › Create Codex32 shares,
Backup › Codex32 on every key, "Type it back", the info map, DESIGN §5
and Learn's Codex32 section.

Files: `core/osk-bip/src/codex32.rs` (`identifier_for`, `symbols_for`,
`seed_symbols`), `core/osk-ui/src/screens/secret.rs` (a second action),
`opensigner/opensigner-core/src/codex32.rs` (`Codex32Plan`),
`create.rs` (`CreateWizard::codex32`, `Step::Codex32`, `strengths`,
`codex32_made`), `backup.rs` (`BackupStep::Codex32`), `lib.rs`,
`learn_map.rs`, `ids.rs`, `strings/en.rs`, `views/codex32.rs` (new),
`views/create.rs`, `views/backup.rs`, `views/home.rs`,
`views/load.rs`, `tests/codex32_create.rs` (new), `tests/info.rs`,
`tests/common/mod.rs`, `tools/lint-secrets.sh`, `docs/DESIGN.md`,
`docs/learn/25-other-backups.md`.

**The identifier, and its arithmetic.** `codex32::identifier_for` takes
the master fingerprint over the seed and returns the first twenty of its
thirty-two bits as four 5-bit groups through `CHARSET`. The fingerprint
`73c5da0a` is `01110 01111 00010 11101` — 14, 15, 2, 29 — and those
places in the bech32 alphabet are `w0za`, which is the unit test's
hand-computed case. It is the seed's own, so two backups of one key
share an identifier, which is the opposite of what SLIP-39's does
(§16.107's is fresh per backup, because a SLIP-39 identifier salts the
encryption and a person holding two sets must not mix them). Mixing two
codex32 sets of one seed is not a hazard the identifier has to prevent:
the strings still differ, a set of `k` from two splits fails the
interpolation check the codec already makes, and the test asserts that
the shares of a second split are all new.

**The randomness, and its arithmetic.** BIP 93's "Generating Shares"
asks for `k - 1` random shares, each filled with `ceil(bitlength / 5)`
characters, and `Codex32::split` takes exactly `(k - 1) * ceil(8L / 5)`
bytes of `fresh` and uses the low five bits of each. So a `k`-of-`n`
split of an `L`-byte seed consumes `(k - 1) * ceil(8L / 5)` bytes: 26
characters a random share at 16 bytes, 32 at 20, 39 at 24, 45 at 28, 52
at 32 and 103 at 64.

The plan does not gather that many. It gathers `L` bytes per random
share, through the source's own screens, and writes them into the
share's payload with `codex32::seed_symbols` — the same 8-to-5 packing
`from_seed` uses for the secret, the last group padded with zero bits.
A random share is therefore the codex32 payload of a freshly gathered
seed of the same length, every gathered byte is used and none is
invented. The alternative was gathering `ceil(8L/5)` bytes and throwing
away three bits in every five, which would have asked for four runs of
a source where one does. The cost is that the 0 to 4 padding bits of
each random share are zero rather than random, exactly as the secret's
own are; the secret they interpolate to is unchanged, and derived
shares carry arbitrary padding as before.

A run of a source makes at most 256 bits, so a 64-byte seed takes two
runs per random share (`ceil(L / 32)`), at `Strength::for_bytes` of
whatever the share still needs. The plan counts `(k - 1) * L` bytes and
`(k - 1) * ceil(L / 32)` runs before the first screen opens, and
`Codex32Plan::split` asserts in debug that the bytes gathered and the
characters handed to the codec are both what it counted, zeroizing and
cancelling on a mismatch, as §16.107's S2 does.

**The plan is a second struct in the same file.** `Codex32Plan` sits
beside `Codex32Entry` in `opensigner-core/src/codex32.rs` rather than in
a new file, because "Type it back" **is** a `Codex32Entry` and the plan
owns one: a separate file would have put one flow's state in two places
and added a second entry in `lint-secrets.sh` for the same subject.
Generalising `SharePlan` was the other option and was rejected: the two
share the source Choice and the shape of the gathering and nothing else
— no groups, no passphrase, no word grid, no quiz, a different count of
random values and a different secret type — so one struct would have
been two structs behind a flag. It holds the seed (64 bytes), the plan,
the gathered bytes (`8 × 64`), `[Codex32; 31]` at 125 bytes a string,
and the entry: about 5.5 KB, one allocation-free field of `OpenSigner`,
zeroized when the flow is left and on drop. `Codex32::split`'s `Vec` is
the one heap value in the path; its elements are `ZeroizeOnDrop` and it
is built at its final capacity, so nothing is reallocated.

**What the design got wrong or left unsaid.**

- The brief puts "Skip" beside the ✓ on the type-back entry. §4.3 says
  an Entry screen has no bottom action — "✓ on the keyboard is the way
  on" — so there is nowhere beside the ✓ for it to stand. It stands on
  the Secret screen before it, beside Continue, which is where the
  quiz's own Skip stands: on the screen that offers the quiz, not inside
  it. §5's Secret line now allows that second action.
- A wrong character cannot reach "Not the same string". One character
  wrong is a string whose checksum fails, so ✓ is dead and the line
  under the field carries the codec's reason, exactly as Load's entry
  has it. "Not the same string" is for a string that parses and is not
  the one shown — the set's other share, typed by mistake — which is
  what the test types.
- Nothing said what "Random shares from?" means for a seed that is not
  split. It means nothing: no random share exists, so Backup skips the
  question and the Result states the plan without a source row.
- The Result's Done leaves the flow where every other backup's does, on
  the key's Backup menu rather than on the key page. The brief says the
  key page; making this one flow pop two screens would have been a
  special case in a menu every other row returns to.
- A 127-character string needs no pager. The comparison ladder takes it
  down to `CHUNK_MIN_SIZE` and it fits the 268 dp panel, as the
  111-character master private key already does, so the Secret screen
  draws it whole on every class.
- BIP 93's own "fresh secret" method would have let a person make a seed
  by writing `k` random shares and never holding the secret. This flow
  takes the other road — gather entropy, encode the secret, split it —
  because the device has to hold the seed anyway to make it a key.
- `Strength::for_bytes` maps 16, 20, 24, 28 and 32 one-to-one onto the
  five strengths §16.108 added, so "How long a seed?" is `Strength::ALL`
  with a `{} bits` label and needed no list of its own. 512 bits is not
  offered, as rule 4 says.
- The device step is left for the plan rather than for a finishing step,
  which the entropy request did not expect: `create_entropy_asked` stayed
  set and the plan's first run of "This device" was never answered. The
  dispatch now calls `sync_create` before `sync_codex32`.
- The gathering's title still says nothing about which random share it
  is making. `share_random_of` and `share_group` are in `en.rs` and no
  view draws them; §16.107's S2 left that, and codex32 leaves it the
  same way rather than wiring one of the two flows and not the other.

**Not done here.** Error correction, the 64-byte key on Tier B, and a
scanner route for a string beyond what C1 built.

**Still open in this entry.** The 64-byte key on Tier B and error
correction, both as the entry has them.

### 16.110 A key's account for a coordinator, and the transaction's keys (2026-09-19)

**Why.** Two gaps the launcher model left. First: a person joining a
multisig that a coordinator builds has to hand it this device's
account key at BIP 48's path, and BIP 129's round 1 is that key signed
as a record; but R1 decided "a key has no export of its own" and served
the two account forms from a single-sig wallet's Export, which gives
that wallet's own account (84' or 86'), never the multisig one (48'),
and there is no wallet yet when the ceremony starts. §16.106 built the
signer record and R5 left its export unbuilt for this reason. Second:
the Sign review says "No key loaded" or "none can sign" when a
transaction names keys this device does not hold, and stops there; the
wallet's Keys review has had the return path to Add a key with the
fingerprint check since R3 (§16.104 rule 6), and the review has not.

**The model, as rules.**

1. **A key exports its public accounts.** The key page gains an
   **Account key** row between the Key row and Backup. It opens the
   Choice **"Which account?"** with a row per account, the path under
   the name as the recovery waits carry their blocks: Legacy
   (`m/44h/coin/0h`), Nested SegWit (`49h`), SegWit (`84h`), Taproot
   (`86h`, which is also the account a MuSig2 wallet takes), Legacy
   multisig (`m/48h/coin/0h/0h`), Nested multisig (`1h`), SegWit
   multisig (`2h`, which is also the account a taproot multisig takes,
   §16.106 pass R5). Continue opens the Export screen with the Format
   row — **Account key**, **SLIP-132** for a single-sig account, **BSMS
   key record** for a multisig one — the string as a reference row,
   "Show QR" and Copy, exactly as a wallet's Export. The account key
   carries its origin, `[73c5da0a/48h/0h/0h/2h]xpub…`, since that is
   what every coordinator asks for. R1's sentence is revised: a key
   exports its public accounts, and a single-sig wallet's Export keeps
   the two forms for its own.
2. **The BSMS key record.** The format asks for the session token on
   the hex keyboard, with a **None** preset row that writes `00`
   (`bsms::NO_ENCRYPTION`), then the description on the name keyboard
   with this device's fingerprint already in the field, then shows the
   record as text and as a QR. It is signed with the account node's
   own secret key, as BIP 129's vectors are, through a door on the
   master key that derives the account's private key for the length of
   one call and never holds it. A token other than `00` is written
   into the record and the record is not encrypted: §16.106 records
   that this tree has no AES, and Learn says the record must then be
   carried where a token would not be needed.
3. **The transaction's keys.** The Sign review's key context row, when
   the transaction names a key this device does not hold, opens
   **the transaction's Keys review**: the wallet's Keys review shape
   (§5 Menu) over the master fingerprints the transaction's origins
   name, one row each, the key glyph where the key is loaded and the
   eye with "not loaded" where it is not, a FROST input's member as the
   wallet's review shows it. A not-loaded row opens Add a key with the
   return and the fingerprint check of §16.104 rule 6, and coming back
   re-inspects the transaction so the review continues with the key.
   The dead Continue's reason stays as it is. A transaction that names
   no origin at all has no such row, since there is nothing to name.

**Rejected.** An Export row on the key page beside Backup: the account
is a fact about a key like its fingerprint, and one row that asks which
account is enough. Listing MuSig2 or Taproot multisig as accounts of
their own: they are `86h` and `48h/…/2h` and a second row for the same
path would be one path under two names, which R5 refused for the same
reason. Guessing the missing key's kind on the Sign review: rule 6
says the device never infers that a missing key is a passphrase key.

**In one pass, D1.** Rules 1 to 3, the info map rows (Which account?
and the Export → Xpubs and privacy; the BSMS steps → Coordinator files;
the transaction's Keys review → Transactions), Learn's Coordinator
files page saying the device writes a key record, DESIGN §5's Menu,
Choice and Entry lines.

**Open in this entry.** An encrypted BSMS record, which needs AES-256
and HMAC in the tree. Whether the description should be typed at all
or fixed to the fingerprint; it is typed because coordinators show it
as the cosigner's label.

**Pass D1 landed.** Rules 1 to 3, the info map, Learn's two pages and
DESIGN §5's three lines. A key's page carries "Account key" between the
Key row and Backup wherever the key has a master; it opens the Choice
"Which account?", whose rows carry the path under the name for the
network the device is set to, and Continue opens
`Screen::KeyExport(key, account, step)`. That screen is the wallet
export's own screen over a second owner: `export_value`,
`format_offered`, the QR page, Compare and Copy all take an `Exported`,
which is either a wallet or one key's account, and the wallet's three
formats are untouched. The account key is written `[73c5da0a/48h/0h/0h/2h]xpub…`
with the `h` §2.1 asks for; `ExportFormat` gained `BsmsSigner`, and the
formats a key's account offers are the account key always, SLIP-132 for
a single-signature account that is not Taproot, and the key record for a
BIP-48 one.

BIP 129's key record is a flow, so its two values are steps of the
export screen rather than screens of their own, as the QR page already
was: choosing the format opens the session token on the hex keyboard
with "None · 00" as the one row above the field, ✓ live only on a token
`bsms::is_token` accepts; ✓ opens the description on the passphrase
keyboard with this device's fingerprint already in it, ✓ dead past
eighty characters with the limit on the error line; ✓ writes the record
and the screen goes back to the export. The chevron walks the same three
steps backwards and puts the format back to the account key. The record
is signed through `MasterKey::with_multisig_account_secret`, which
derives the BIP-48 node, hands the `SecretKey` and the node's non-secret
view to one closure and erases the derived key when the call returns;
`bsms::signer_record` refuses a secret that is not the account's, so the
record verifying is the proof that the door derived the right node.

The transaction's Keys review is `Screen::SignKeys(reading)`, drawn by
the same row builder a wallet's Keys review uses, over the master
fingerprints the transaction's origins name — the multisig cosigners in
script order first, then any other origin the inputs state — or, for a
FROST spend, the group's members by public share. The Sign review's key
context row carries `ids::SIGN_KEYS` now and opens it; where no loaded
key signs anything, the row names the keys the transaction asks for, so
there is something to tap, and a transaction that names no origin has no
row at all. `AddKeyReturn` carries a `ReturnTo` — a wallet or the
transaction — beside the member it named, so R3's fingerprint check
serves both.

**Three things the entry got wrong or left unsaid.** BIP 48 defines two
script types, `1'` and `2'`, and the seventh row the entry asks for,
"Legacy multisig" at `m/48h/coin/0h/0h`, is a path no standard defines
and no coordinator asks for; `MultisigScriptType` says so in its own
doc comment, and inventing a variant for it would have written a path
this device cannot defend. The Choice offers six accounts, not seven.
Second: the key record writes BIP 129's hardened apostrophe and every
other screen writes `h`, so the same key reads two ways on one Export
screen. The record's four lines are what the signature covers and what
the BIP's vectors are written in, so the record keeps `'`; the Account
key format keeps `h`. Third: the entry says the info map covers "Which
account?", but §16.105 leaves the slot empty on every overlay, and a
Choice is an overlay; the map covers the Export screen behind it and the
record's two Entry steps, which is where the button can be drawn.

**What the transaction costs to keep.** Every screen change clears the
Sign flow, which is what keeps a PSBT from outliving its screen, so a
detour to Add a key would have thrown the transaction away. The flow is
taken out whole into a `KeyDetour` beside the stack while Add a key
runs, and `SignFlow::reinspect` puts it back inspected again with the
keys there are now, after the screen has settled. The review opens the
way a Learn page does — pushed without `entered` — and leaves the same
way, for the same reason. `Screen::SignKeys` is `ScreenKind::WalletKeys`
and `Screen::KeyExport` is `ScreenKind::Export`, because each is the
same screen of the app over a second subject, as `LearnTopic` is
`LearnPage`.

**Not checked against BIP 129's own signatures.** The BIP publishes nine
key records and no seed for the keys in them, so a record this device
writes cannot be compared byte for byte with one of the BIP's. What is
checked is the round trip the coordinator makes: the record parses, its
signature recovers the key it names, and that key is the account key the
same screen's other format shows. `core/osk-bip/tests/bsms.rs` already
verifies all seven of the BIP's extended-key records against their own
signatures.

**Open after this pass.** An encrypted key record, which still needs
AES-256-CTR and HMAC-SHA256. A second account index: every account here
is `0'`, which is what the account Choice states and what a coordinator
asks for first. And the key context row is off a 268 dp panel by
§4.4, so on that class the transaction's Keys review has no way in; the
dead Continue's reason is all that panel says about a missing key.

### 16.111 What a signature on a PSBT is checked for (2026-09-19)

**Why.** The inspector counts the signatures already on a transaction
by key and verifies none of them. A signature another device made can
be wrong, can reuse a nonce, or can carry hidden randomness, and the
review says nothing about any of it. EntropyLab's PSBT inspector checks
all three without signing; the owner asked for the same here
(2026-09-19). Roadmap rows 26 to 30.

**The rules.**

1. **Every signature present is verified.** For each input, each ECDSA
   partial signature and each Schnorr signature — `tap_key_sig`,
   `tap_script_sigs`, and the signatures inside a finalized
   `scriptSig` or witness — is checked against the public key it is
   under and the sighash the input's fields give. The review's
   Signatures row states the count and, under it, one row per
   signature: the key's fingerprint or public key, "valid", "invalid"
   or "cannot check" with the reason (no previous output, an unsupported
   script). An invalid signature is a danger card: the transaction has
   been altered since it was signed, or the signer is broken.
2. **A repeated nonce is a danger.** Two ECDSA signatures under one
   public key with the same `r`, anywhere in the transaction, block the
   flow: the key is recoverable from them. Schnorr likewise on `R`.
3. **A deterministic signature is recognised.** Where a present
   signature is under a key this device holds, the device recomputes it
   under RFC 6979 with low R, under RFC 6979 first-nonce, and, for
   Schnorr, under BIP-340 with zero aux, and the row says "deterministic
   · low R", "deterministic · first", "deterministic · BIP-340" or "not
   deterministic". Not deterministic is an information card, not a
   refusal: random aux is the BIP's own default and hides nothing by
   itself, but a person comparing two devices wants to know.
4. **Two PSBTs compared.** Tools gains "Compare transactions": two
   PSBTs read one after the other, the decoded transaction, the signing
   state and the metadata diffed separately, so a reordered map is not a
   difference; the Result lists what differs by input and output.
5. **An inscription envelope is stated.** A tap leaf or witness holding
   `OP_FALSE OP_IF "ord"` is an information card naming the content type
   and size; nothing is rendered.

**In one pass, V1**, rules 1 to 5, in `osk-psbt::verify` with tests over
the committed fixtures (every fixture's signatures valid; a fixture with
one byte of a signature changed invalid; two signatures with one nonce
built for the test; the device's own signatures deterministic under the
setting they were made with), and the rows and cards on the Sign review
and Decode.

**Pass V1 landed.** `osk-psbt::verify` reads every signature a
transaction carries — the partial-signature fields and the ones inside a
final `scriptSig` or witness, parsed back the way `finalize` writes them
— and checks each against the key it is under and the sighash the
input's own fields give, sharing `sign`'s sighash path rather than
copying it: the ECDSA side goes through `bitcoin`'s signer path with the
type the signature itself states, and the taproot side through the same
prevout set `sign` commits to. A finalized input has had its redeem and
witness scripts cleared, so they are read back out of the final fields
before the sighash is computed. The verdict is `Valid`, `Invalid` or
`Unchecked` with the reason, and where the witness alone does not say
which key made a signature — a multisig witness, a `multi_a` leaf — the
key is whichever of the script's own keys verifies it. `inspect` runs
the check, so an invalid signature is a danger card and a nonce used
twice under one key is another; an inscription envelope is an
information card; a signature that could not be checked is a caution.
The determinism check is the one the inspection cannot make, because it
has no key: the Sign and Decode flows run it with the master keys they
are given and add the information card themselves.

The review's Inputs page gained the Signatures row, and it opens the
Signatures page, which is now a Record over every signature the
transaction carries rather than over the ones this pass wrote: the input
and the key as the label, the verdict as the value, and the nonce rule
after it for a key this device holds. It is the same page from the Sign
review, from Decode and from the Sign result, and the row still opens
the signature whole on Compare. The determinism verdict replaced the
bare "nonce checked" row it used to carry: recomputing the signature
under the rule the setting names is the stronger statement of the same
fact, and where nothing reproduces it — a taproot signature made with
fresh randomness — "not deterministic" is what is true, where "nonce
checked" claimed more than it had checked. Signing twice is still a
refusal inside `sign`.

Tools gained "Compare transactions": the scanner's ways in for the
first, then for the second, then a Result of "Same transaction" or
"Different" with one row per field that differs. Every comparison is of
decoded contents, so a PSBT whose maps were written in another order is
the same PSBT. `tests/verify.rs` covers the fixtures' own signatures,
every script type signed and then finalized and read back out of the
witness, one byte changed, two inputs signed under one nonce built with
a fixed `k`, each nonce rule recognised, the comparison and an
inscription envelope; `tests/sign.rs` and `tests/tools.rs` in the core
cover the page, the two dangers and the tool.

**Open.** The cross-session nonce history, which is state and waits on
§16.112. Anti-exfil transcripts, which wait on row 9.

### 16.112 Encrypted notes and interoperable exports: the history and a proposal (2026-09-19)

**The history, from the session logs.** On 2026-09-10 the owner asked
whether an off-the-shelf tool could open the backup, and named KDBX and
GPG. The comparison then: KDBX 4 has Argon2id and an ecosystem an heir
already has (KeePassXC, KeePassDX, KeePassium), at the cost of an outer
header, an HMAC block stream, ChaCha20 and an XML inner database; age
is a tenth of the work but scrypt only and a tool for technical people;
our own format is the lightest and needs a published spec and a
reference decryptor. The recommendation was KDBX 4 if "off the shelf"
was the requirement, and never several formats. The decision of
2026-09-11 (`.thoughts/decisions.md` §3) was our own format for the key
backup, and that is `osk-backup` (§16.78, §16.96), with its spec and
`tools/backup/decrypt.py`. On 2026-09-16 the owner reopened the wider
use cases — descriptors, address lists, seed phrases, KeePass or LUKS
for a reader who is not OpenSigner — and the assessment was: our format
where the reader is OpenSigner; plain files for what has an
interoperable form (the descriptor, BIP-329 labels, the words); KeePass
for an heir and age or PGP for the technical, each a decision of its
own; LUKS is a volume, not a file. Nothing was decided or built after
that. On 2026-09-19 the owner stated the principle: the device does not
refuse storage or state, it encrypts it; and named the use cases: a note
such as wallet instructions for an heir under a password, encrypted QR
and file exports, and KeePass as one of the forms people will want.

**The proposal.**

1. **One container for everything the device encrypts**: `osk-backup`
   generalised from words to a typed payload — words, a master seed, a
   note, a wallet sheet (descriptor, names, a note), a nonce history —
   one spec, one reference decryptor, one decrypt path to audit, as
   the 2026-09-10 advice had it. Padded in steps so a file's size says
   little. Read back by Scan or Read a file, which asks the passphrase
   and opens the payload where it belongs.
2. **Notes.** A note is typed on the device (the name keyboard) or read
   from a plain file, shown on a Document screen, and exported
   encrypted as a QR or a file. A wallet's page gains a "Recovery sheet":
   the descriptor, the wallet's name and a note, exported plain or
   encrypted, which is the heir's document. A free-standing note lives
   under Tools › Notes. On Tier B, notes and sheets are kept in the
   blob in a record of their own.
3. **KDBX 4 as an export**, write only: a database with one entry per
   item (title, notes, a password field for the secret), Argon2id, the
   HMAC block stream, ChaCha20, no compression, so an heir opens it in
   a KeePass app with no knowledge of this project. Every primitive is
   already in the tree; the writer is a few hundred lines and a test
   against `pykeepass` where it is installed, as the backup's decryptor
   test is. Reading KDBX is not proposed.
4. **Not proposed.** age (scrypt only, no heir has it) and PGP (a
   separate key type and a packet format) — the 2026-09-16 assessment
   stands.

**Decided (owner, 2026-09-19).** Both formats: `osk-backup` and KDBX 4.
Wherever the device exports something, the person chooses the form —
a QR where the data is small enough for one, an `osk-backup` file, or a
KDBX 4 file — and KDBX is offered for keys and seed words too, not only
for notes, on the stated understanding that the container's security
is then the keys' security: Learn says so and asks for a strong
passphrase under Argon2id. The Argon2id memory cost is the person's
choice per device, with the trade-offs stated and a recommendation:
more memory makes every guess cost more and every open slower, and a
file made at a cost this device cannot allocate cannot be opened here;
the recommendation follows the device's memory. A file states its own
cost in its authenticated header, so any device with the memory opens
any file. Passes: E1 the typed container, notes, the recovery sheet,
the Format choice and the memory setting; E2 the KDBX 4 writer and its
rows; E3 notes and sheets kept on Tier B.


### 16.113 Silent Payments: the receiving side (2026-09-19)

**Why.** BIP-352 is final and shipping in wallets, and it is the one
sizeable standard EntropyLab has that this device does not; the owner
accepted it (2026-09-19, §8.4 row 31). Receiving needs no chain: the
address, its labels, the descriptor for a scanner and the verification
of an output are arithmetic over a key. Sending an output to a silent
payment address needs the sum of the input private keys and BIP-375's
PSBT fields, which are a draft, so sending waits.

**The facts from the BIP.** Scan key at `m/352h/coin/account/1h/0`,
spend key at `m/352h/coin/account/0h/0`, both hardened. The address is
bech32m with the prefix `sp` on mainnet and `tsp` off it, version `q`,
then `ser_P(B_scan) || ser_P(B_m)`, 116 characters on mainnet. A label
`m` gives `B_m = B_spend + hash_BIP0352/Label(ser_256(b_scan) ||
ser_32(m))·G`; `m = 0` is the change label and is never handed out.
Receiving: `input_hash = hash_BIP0352/Inputs(outpoint_L || A)` over
the lexicographically smallest outpoint and the sum `A` of the eligible
inputs' public keys (P2TR except with `H` as internal key, P2WPKH,
P2SH-P2WPKH, P2PKH); `ecdh = input_hash · b_scan · A`; `t_k =
hash_BIP0352/SharedSecret(ser_P(ecdh) || ser_32(k))`; `P_k = B_spend +
t_k·G` for `k` from 0, and a labelled output is `P_k` plus a known
label point. BIP-392, a draft, gives the descriptor `sp(KEY)` with
`spscan1q…` (the scan private key and the spend public key, bech32m)
or `spspend1q…` (both private), and `sp(KEY,KEY)`. BIP-321 carries the
address as `bitcoin:?sp=…`, and BIP-353 puts that URI in a TXT record
at `user.user._bitcoin-payment.domain`, DNSSEC-signed. The vectors are
`bip-0352/send_and_receive_test_vectors.json` with `reference.py`.

**The model.** A silent payments wallet is a wallet kind over one key:
Add a wallet › **Silent payments**, the Keys step over the loaded keys,
no script step (outputs are taproot by definition), the review naming
the kind, the key and the address, then Add this wallet. Its page:
**Address** (the `sp1q…` string as text and QR, with the BIP-321 URI
as a second form), **Labels** (label 1 upward as addresses of their
own; the change label is not offered), **Check a payment** (a raw
transaction read by Scan or file, with the previous transactions a
taproot input needs, as §16.98's mechanism asks for them; the Result
names each output that pays this wallet, its amount and its label, or
says none does), **Export** (the `sp(spscan…)` descriptor for a
scanner, which carries the scan private key by design and is marked as
a secret screen; the BIP-321 URI; the BIP-353 record text for a domain
the person types), Keys and Forget. The wallet is kept on Tier B as any
wallet is; Sign has no row, since sending is not built.

**In one pass, SP1.** `osk_bip::silent` with the derivation, the
address, labels, the scanning arithmetic and the BIP-392 encodings,
against every vector; `osk_codec::classify` reading an `sp1q…` address
and a `bitcoin:?sp=` URI; the wallet kind and its page; Learn's page
"Silent payments" (what it is, what the scan key gives away, why the
device does not send yet); the info map.

**Open.** Sending, on BIP-375. Whether "Check a payment" should also
take Bitcoin Core's verbose JSON, which carries the previous outputs
inline, as EntropyLab does.

**Pass E1 landed.** `osk-backup` is version 3: the plaintext is a kind
byte, a length and a payload, padded to the next 256 bytes, and the
kinds are words (the version-2 plaintext unchanged), a master seed, a
note and a recovery sheet. A version-2 file still opens. The format is
written down byte for byte in `docs/BACKUP.md`, which the 2026-09-11
decision asked for and never got; `tools/backup/decrypt.py` reads both
versions and all four kinds, and `tools/vectors/backup/` holds a
version-3 vector per kind beside the two version-2 ones. A key with no
words — SLIP-39, codex32 — now has an Encrypted backup row, which seals
its master seed; words and a seed share a padding step, so a file that
holds a key is 345 bytes whichever door the key came through.

The Argon2id memory is the person's choice: Settings › Backup memory
offers 64 MiB, 256 MiB and 1 GiB, each row stating its trade-off, with
the recommendation following the memory the shell reports through the
new `DisplayInfo::memory_mib` — 256 MiB at a gigabyte or more, 64 MiB
below. Android reads `ActivityManager.MemoryInfo`, the Pi and the
desktop read `/proc/meminfo`, and the snapshot shell reports nothing so
that a picture does not change with the machine that took it. The
setting persists with the others and is absent from the file until it is
chosen. Every `seal` uses it; the kept-key blob keeps its own 64 MiB.
`open` now accepts any cost this device can allocate rather than any
cost it would have written: above 4 GiB a file is refused by its stated
cost alone, below it the allocation is attempted with `try_reserve` and
a failure is "Needs {} MiB" rather than an abort.

Tools › Notes types a note on the name keyboard or reads one from a
plain file, shows it whole on a Document and exports it; a wallet's page
gains Recovery sheet, which is its descriptor, its name and a note,
exported plain or sealed; and one Choice, "Which form?", stands before
the passphrase of every encrypted export, with KDBX 4 dimmed until E2.
A sealed note or sheet read back by Scan opens on the same Document, and
a sheet offers "Add this wallet".

Two things the design did not say. A `KIND_SEED` payload does not record
whether the key was read from SLIP-39 shares or from a codex32 string,
because the seed cannot tell and the distinction buys nothing: either
way the key is its master secret and has no words, and that is how it
comes back. And the memory Choice gets no info button: §16.105 leaves
the app bar's trailing slot empty on every overlay, and a Setting Choice
is an overlay. The other three screens the map was asked for — Notes,
the sheet, "Which form?" — carry it.

**Pass E2 landed.** `opensigner-core`'s `kdbx.rs` writes a KDBX 4.0
database: the outer header with ChaCha20 as the cipher, no compression,
a master seed, a 12-byte encryption IV and the Argon2id parameters as a
VariantDictionary; the header's SHA-256 and its HMAC-SHA-256; the HMAC
block stream over the encrypted body; the inner header with ChaCha20 as
the inner random stream; and one XML document with a group named
OpenSigner and an entry per item carrying Title, Notes and a Protected
Password. The composite key is the passphrase alone, the Argon2id cost
is the device's setting, and every secret is built in a buffer that
zeroizes, so the file is listed in `tools/lint-secrets.sh`. 4.0 rather
than 4.1 because 4.1 adds only XML this writer has no use for. KeePass's
two pages are copied into `tools/reference/kdbx4/` with the date and
their SHA-256. The one new name in the graph is `chacha20`, which
`chacha20poly1305` already builds on and does not re-export; the lock
file does not change.

"Which form?" now offers KDBX 4 live for a key's words, a master seed, a
note and a recovery sheet; the passphrase steps are the ones every
sealed export has; a KDBX Result offers Save and no QR, because the file
is kilobytes and a KeePass app reads files. The brief named a KDBX file
after what it holds — the fingerprint, the note's first line, the
wallet's name — so an heir finds it among others; the orchestrator
overruled that on review, because §16.96's rule exists exactly so that
a file on a card says nothing about what it holds, and a fingerprint in
a file name is that leak. Every KDBX file is offered as
`opensigner.kdbx`; the entry's title inside the file carries the
fingerprint, the note's first line or the wallet's name, which is what
an heir with the passphrase sees on opening it.

The check is `tools/backup/read-kdbx.py`, which opens a file through
`pykeepass` where it is installed and through a reader of its own over
`cryptography` otherwise, and says which ran; the vector is
`tools/vectors/backup/backup-kdbx-words-12-english.kdbx`. `pykeepass`
4.2 opened what the tree writes. `keepassxc-cli` is not on this machine,
so nothing was checked against KeePassXC itself.

Three things the design did not say. A note has no title — nothing on
the device asks for one — so its first line titles the KDBX entry and a
note that starts blank is titled "Note". A note and a recovery sheet go
in the entry's Notes with no Password, because neither is a secret field
and Notes is where a KeePass app shows text; only a key's words or seed
are the Protected field. And the entries carry a zero timestamp rather
than a made-up date, because the device has no clock.

**Pass SP1 landed.** `osk_bip::silent` is BIP-352's receiving side:
the scan key at `m/352h/coin/account/1h/0` and the spend key at
`…/0h/0`, the bech32m address with the version character and the two
compressed points, the labels and the change label, `input_hash`, the
shared secret, `t_k`, `P_k` and the search that walks `k` upward while
outputs match and stops when none does. The eligible-input rule is the
BIP's: the public key of a P2TR input comes from the output it spends
(unless the control block's internal key is `H`), and the other three
come from the witness or from the scriptSig, read with the sliding
33-byte window BIP-352 requires for a malleated P2PKH. The sending side
is there too, as far as the vectors need it and no further — no PSBT,
no signing — because a payment this module makes is the only way to
prove a payment this module finds. All 28 cases of
`send_and_receive_test_vectors.json` pass from both sides, including
the zero intermediate sum, the point at infinity, the NUMS input, the
uncompressed keys that are skipped and the 2,324-output transaction
that stops at `K_max`.

The wallet kind is `Template::Silent` over no key expression at all.
A silent payments wallet has no extended key, no descriptor a
miniscript library reads and no address at an index, so `WalletPolicy`
holds an `osk_bip::silent_wallet::SilentWallet` beside the template and
its text is that record — the magic, the network, the key's origin, how
many labels have been handed out, and the address, which carries both
public keys. `parse_any` reads it, so the blob keeps it and a restart
brings it back, which is how every other wallet is kept.

The page is Address, Labels, Check a payment, Export, Keys and Forget.
The Address screen gained a §4.2 value row for the form it is shown in,
which switches between the `sp1q…` string and BIP-321's
`bitcoin:?sp=…`; Labels is an Addresses-shaped list up to twenty, each
label opening the same Address screen; Check a payment reads a raw
transaction or a PSBT and says which outputs pay this wallet, with the
amount and the label of each; Export offers the scan descriptor on a
Secret screen, the URI, and the record under a user name and a domain.
`classify` reads an `sp1q…` address and a `bitcoin:?sp=` URI, and Check
an address answers "A silent payment address; it can be shown, not
checked" and shows it whole, because no wallet's chain of addresses
holds one and searching would be a lie either way it came out.

**Five things the design got wrong or left unsaid.**

The mechanism §16.98 was cited for does not exist. That entry decides
when a SegWit input with no previous transaction is refused; nothing in
the tree asks for a previous transaction, because a PSBT either carries
one or does not. So Check a payment grew the asking: a raw transaction
states no previous output at all, not only for its taproot inputs, and
the flow lists the transaction it is waiting for and reads it by the
identifier the input names. A PSBT needs none of that, since
`witness_utxo` and `non_witness_utxo` are the previous outputs.

An input whose key is in its signature data, with no signature data, is
refused rather than skipped. BIP-352 skips an input it can read no key
from, which is right for a broadcast transaction and wrong for an
unsigned PSBT: skipping there would compute a shared secret from fewer
inputs and report "Not paid here" for a transaction that pays. The
Result says the transaction is not signed instead.

Labels and Check a payment need the scan private key, so they need the
wallet's key to be loaded. The record is public data and cannot hold
the scan key — that is the one secret this wallet has — so a silent
payments wallet whose key is not loaded shows its address and nothing
else: its label rows carry §4.11's "not loaded", and a check says so.
The design did not say what such a wallet shows; this is the answer
that keeps the record public.

BIP-392 publishes no test vectors — its "Test Vectors" section reads
TBD and its examples elide the payload as `sp(spscan1q...)` — and
BIP-321's examples carry the placeholder `sp1qsilentpayment`. So there
is no published string to check either encoding against, and what the
tests check is the form each BIP specifies, over an address BIP-352's
own vectors publish. `reference.py` is kept under
`tools/vectors/bip352/` for reading and is not run: it imports a
vendored `secp256k1lab` and two helper modules that are not part of
that download, and the vectors beside it are its own published output.

A silent payments wallet's `to_descriptor` is its address. `checksum`
is what `names` keys a wallet's name by and what the kept list tracks a
wallet by, and it is taken over the descriptor, so a wallet with no
descriptor needs one string in the descriptor character set that
identifies it. BIP-392's `sp(spscan1q…)` would be that string and is
the scan private key, so the address is used instead. Nothing presents
it as a descriptor: Export offers the three forms this wallet has and
none of anyone else's.

**What is still open.** Sending, on BIP-375. `spspend1q…`, which is
both private keys and which this build neither writes nor reads,
because nothing here spends a silent payment output. Whether Check a
payment should also take Bitcoin Core's verbose JSON, which carries the
previous outputs inline and would answer the asking above in one file.

### 16.114 BIP-85's other applications, and a Bitcoin Core import file (2026-09-19)

**Why.** BIP-85 has applications beyond seed words — an HD-seed WIF
(2'), an extended private key (32'), raw hex (128169'), and passwords
in base64 (707764') and base85 (707785') — and Coldcard and EntropyLab
derive all of them; this device derives words only (§16.67). And a
wallet on Bitcoin Core is registered with `importdescriptors`, a JSON
file every other export here almost is; §8.4 row 32 and §8.1 row 13,
accepted 2026-09-19.

**The rules.** The key page gains a **BIP-85** row opening "Which
application?" — Words, WIF, Extended private key, Hex, Password
(base64), Password (base85). Words goes to the existing child-key flow
(Open BIP-85 child), since a child of words is a key and lands in Keys;
the others derive a value to transcribe, not a key: the index on the
digits pad (as the child flow asks it), Hex asking "How many bytes?"
(16, 32, 64) and a password its length (base64 20 to 86, default 21;
base85 10 to 80, default 12) on the pad, then the value on a Secret
screen with the eye, titled by the application. Nothing derived this
way is kept or loaded. Every application is checked against the BIP's
own vectors, downloaded into `tools/vectors/bip85/`.

A wallet's Export gains the format **Bitcoin Core import**: the
`importdescriptors` JSON with the wallet's multipath descriptor
(`<0;1>`, checksummed), `active: true`, `internal` absent since one
multipath descriptor covers both, `range` `[0, 999]`, and `timestamp`
from a Choice "Rescan from?" — "The start · finds old coins, slow"
(`0`) or "Now · a new wallet" (`"now"`). Written as a file and, where
it fits, a QR; no `wallet.dat`, which is Core's own SQLite and not a
format to write here.

**In one pass, M1.**

**Pass M1 landed.** `osk_bip::bip85` derives all five applications
beside 39': `child_wif` (2'), `child_xprv` (32'), `child_hex` (128169',
16 to 64 bytes), `child_password_base64` (707764', 20 to 86) and
`child_password_base85` (707785', 10 to 80). Each returns a fixed-size
buffer that zeroizes on drop — `WifAscii` and `XprivAscii` from
`keys.rs`, `HexBytes` and `Password` from `bip85.rs` — and the module is
in `tools/lint-secrets.sh`'s heap-text list now, which the entry said it
already was and it was not. The extended private key is built through
`MasterKey::from_chain_code_and_key`, a depth-0 key from BIP-85's
reversed order (chain code first, private key second), so the erasure
discipline stays inside `keys.rs`. Base64 writes through
`base64::encode_into`, which encodes into a caller's buffer without
padding, so the one alphabet in the tree serves both. The BIP's text is
in `tools/vectors/bip85/` with `extract.py` beside it, and
`core/osk-bip/tests/bip85.rs` derives every published vector from the
BIP's own master key.

The key page carries **BIP-85** under Account key wherever the key has a
master, which is every key including a passphrase key, a child key, a
SLIP-39 key and a codex32 one. It opens "Which application?"; Words
pushes the child-key flow untouched, Hex asks "How many bytes?" (16, 32,
64) and a password asks "Length" on the digits pad with the
application's bounds on the caption line and the BIP's own default in
the field; then "Index" on the same pad, default 0; then a §5 Secret
screen titled "WIF · 0", with the eye and Done and no Copy. The flow is
`Screen::Bip85(key)` over a `Bip85Flow` beside it, as the child flow is,
and the value is derived where it is drawn and held nowhere: leaving the
screen is the end of it.

A wallet's Export gains **Bitcoin Core import** wherever the wallet has
a descriptor. The format opens "Rescan from?" — "The start · finds old
coins, slow" or "Now · a new wallet" — and the Export screen then shows
the JSON as its reference row with "Save to file"
(`<name or checksum>-core-import.json`) and the QR row every format has.
`osk_bip::core_import::import_descriptors` writes it.

**Checked against Bitcoin Core 31.1.** On regtest, a descriptor wallet
made with `createwallet` and private keys disabled accepted the file for
the 2-of-3 fixture at both rescan points and for a single-signature
`wpkh` wallet:
`importdescriptors "$(cat …-core-import.json)"` answered
`[{"success": true}]`, `getwalletinfo` reported a keypool of 1 000
external and 1 000 internal addresses, and `listdescriptors` showed the
one multipath descriptor as two active descriptors, `internal: false`
and `internal: true`, each over `[0, 999]` — which is the reason the
file writes no `internal` field of its own. Core rewrites the origin's
apostrophes as `h` and re-checksums each chain separately, so the two
descriptors it lists carry neither the string nor the checksum the file
handed it; the multipath descriptor and its checksum are what the file
is compared against, not what Core stores. `timestamp: 0` is stored as
`1`, Core's floor, and rescans from the genesis block.

**Three things the design or the BIP left unsaid.** BIP-85's base85
application names no alphabet — it says "Base85 encode all 64 bytes" and
stops — and the two in use disagree. Its published password
`` _s`{TW89)i4` `` is in RFC 1924's alphabet and cannot be in Ascii85's,
which runs from `!` to `u` and holds neither `` ` `` nor `{`, so that is
the alphabet written out in the module with the reasoning beside it.
Second: the entry asks for the derived value "as mono text" for a WIF, an
xprv and a password and "grouped in fours" for hex, but §4.5 gives this
tree one ladder for every long string, secret or public, and it groups
in fours; a second geometry for one screen would be a second ceiling on
the same ramp, so every application's value is the one comparison
string. Third: the entry says the index is asked "as the child flow asks
it", and the child flow asks it on an Entry with a fact row stating the
fingerprint the index would derive. There is no key here to name, so
both numbers are asked on the Pad screen, which is what §5's own line
for this pass says.

**Open after this pass.** BIP85-DRNG and the applications built on it
(RSA, RSA GPG), the DICE application and the Nostr one; the vectors for
all four are in the downloaded text and the extractor skips them. A
derived xprv is not loadable as a key, by the entry's own rule, so a
person who wants one on this device derives the words instead.

### 16.115 The other published dice procedures (2026-09-19)

§8.1 row 17 as built (pass P1). Create a key's Dice row opens a second
Choice, **"Which procedure?"**, after the language step, and the rolls
are read the way the checked row says. Every algorithm comes from the
source quoted in `tools/reference/dice/README.md`, with that source's
commit, the file's SHA-256 and the date it was read; the repositories
were cloned only to read and nothing was vendored.

**The three rows.**

- **Hashed (Coldcard, SeedSigner)**, checked when the Choice opens, is
  what this device has done since §16.17: SHA-256 of the rolls as the
  ASCII digits `1`–`6`, truncated. Coldcard's `docs/rolls.py` was run
  against this tree's own fifty-roll transcript and printed the digest
  the tests assert, so the two are byte-identical; SeedSigner's
  `generate_mnemonic_from_dice` is the same two lines, and EntropyLab
  names both signers on the row it calls "Base 10 [0-9]".
- **Six as zero, hashed (Keystone)** writes every rolled 6 as `0` and
  then hashes the same way. It is EntropyLab's "Dice [1-6]", which its
  own text says is Keystone's method and iancoleman's "Dice" mode — the
  mode SeedSigner's source comment warns is *not* the hashed one. The
  two vectors in the README were computed by running EntropyLab's
  `hodlDiceEntropy` with method `coleman`.
- **Words chosen by the dice (BitBox)** names a word outright: five
  rolls of 1–4 as base-4 digits, most significant first, then a sixth
  roll as a coin, 1–3 heads and 4–6 tails, giving 4⁵ × 2 = 2048. A 5 or
  a 6 in a word's first five places is a reroll and is not kept, and the
  pad's caption line says so rather than swallowing the key. Twelve
  words take 66 rolls, twenty-four take 138. The words appear on the
  masked panel as their six rolls complete them, as the dice passphrase
  tool's do.

**The last word is chosen, not rolled.** **Superseded by §16.139:** the
dice roll the last word too, and the checksum takes its low bits. As
first built, direct selection named every
word but the last. The last carries the checksum, so only
`2^(11 − checksum bits)` words can stand there — 128 at twelve words, 8
at twenty-four — and a **"Which last word?"** Choice lists them in index
order with the first checked. That is what both sources do: EntropyLab
offers the candidates as a row of options, and the BitBox02 firmware's
own comment says it restricts the last word to the checksum-valid subset
"so that users can generate a seed using only the device and no external
software, allowing seed generation via dice throws". The consequence is
that the last word's free bits are the person's choice rather than the
dice's — 7 bits at twelve words, 3 at twenty-four — and Learn says so.
`osk_bip::bip39::last_word_candidates` already computed that list and
needed no change.

**The sanity step.** For the two hashing procedures it is unchanged. For
direct selection a chi-square over six faces would flag every honest
run, because 5 and 6 never survive a word's first five places, so the
transcript is split the way EntropyLab's own fairness analysis splits
it: a chi-square over the four faces of the word positions (3 d.o.f.,
the 0.999 quantile 16.266, `DICE_FOUR_CHI_SQUARE_LIMIT`) and the coin
skew test this device already applies to flips, over the sixth rolls. A
run counting up or down means nothing across two interleaved streams and
is not tested. The table states the rolls, the words, the bits, the
chi-square, heads, tails and the longest run.

**Where the Choice is and is not.** It stands after the language step,
so a row can state the rolls it takes at the length that was chosen. A
SLIP-39 key and a codex32 key take the same source steps but have no
BIP-39 words, so the direct-selection row is dimmed there with "no
words". A mix takes each source's commitment, which is SHA-256 of the
rolls as ASCII whatever else is on offer, so a mix's dice step has no
procedure Choice; neither has a Seed XOR part gathered by §16.92's
rule, whose rolls are hashed.

**What the brief got wrong, and what the sources actually say.** The
brief named four procedures. Reading the sources collapsed them to
three.

- *"Direct selection of words by dice: rolls read as a base-6 number
  that indexes the 2048-word list."* No source does this. 2048 is not a
  power of 6, so a base-6 reading needs rejection sampling, and neither
  EntropyLab nor SeedSigner nor BitBox does it that way. EntropyLab's
  two direct-selection procedures read base-4-plus-a-coin (BitBox) and
  D8 × D16 × D16 (D++), both of which divide 2048 exactly.
- *"SeedSigner's 'dice as words' mode."* SeedSigner has no such mode.
  `mnemonic_generation.py` has exactly two dice-adjacent entry points:
  the hashed procedure, and `calculate_checksum`, which completes an 11-
  or 23-word mnemonic the person typed. Its coin-flip mode also hashes
  the ASCII `0`/`1` string rather than packing the bits, which is a
  difference from this device's coin source (§16.17) and is left alone.
- *"Dice as coin flips: 1 to 3 heads, 4 to 6 tails, one bit a roll,
  packed as this device packs coins — BitBox02's method."* The
  BitBox02 firmware contains no dice procedure at all; the only mention
  is the last-word restriction quoted above. The "1 to 3 heads, 4 to 6
  tails" reading is real, but it is the *sixth* roll of BitBox's
  diceware word, not a whole procedure of one bit a roll. So this row
  and the previous one are the same procedure, and it is built once.
- *"Base-10 digits hashed: EntropyLab's 'Base 10 hashed'."* That is
  EntropyLab's name for the Coldcard/SeedSigner procedure this device
  already had. What is hashed is the digits as ASCII, all of them
  entered, with no truncation of the input; only the digest is cut to
  the strength. The row the brief was reaching for is the genuinely
  different fourth EntropyLab dice method, "Dice [1-6]", and that is
  what shipped in its place.

**What is not built.** EntropyLab's **D++** rolls one D8 and two D16 per
word and ends the checksum pick with a per-length sequence (a D8 and a
D16 at twelve words, two D8 at fifteen, a D16 and a coin at eighteen,
one D16 at twenty-one, one D8 at twenty-four). It needs dice this
device's pad does not have and a "which dice do you have?" question the
design has not asked, so it is left out; `hodlDPlusRolls` is named in
the README for whoever builds it.

**Reopen** when a pad for other dice exists, which is what D++ waits on,
or if a source changes a procedure, which the README's digests make
visible.

### 16.116 A Lightning node key from a backup (2026-09-19)

§8.1 row 18 as built (pass P2). Tools gains **Lightning node key**: a
Choice, **"From?"**, with "An LND cipher seed" and "A loaded key". The
first takes twenty-four words on the Load wizard's entry over the
English list and then a passphrase; the second takes a key already in
the session. Either way the Result states one public key — the identity
the Lightning network knows a node by — and "Show secret" opens the
private key behind it. Nothing is loaded into Keys, nothing is signed,
and no cipher seed is ever made here. A node key is hot by nature: it
lives on a machine that is online all the time, and what this device
can usefully say about one is *which node this backup is*.

**The codec.** `osk_bip::aezeed` is LND's version 0 scheme in full:
twenty-four eleven-bit indices packed into 33 bytes, a version byte, a
CRC-32C over the first 29, a five-byte scrypt salt, and 23 bytes of AEZ
ciphertext whose plaintext is a one-byte internal version, a two-byte
birthday in days since the genesis block, and sixteen bytes of entropy.
That entropy is the BIP-32 seed itself — LND hands it straight to
btcwallet, with no stretching — and `node_key` derives
`m/1017'/coin'/6'/0/0` from it, LND's `BIP0043Purpose` and
`KeyFamilyNodeKey`, with the coin type 0 on mainnet and 1 everywhere
else as LND's own chain table says. `ldk_node_key` is ldk-node's: a
master key over the 64-byte BIP-39 seed, that master's *private key* as
LDK's own 32-byte seed, a second master key over it, and `m/0'`.
Everything a decode touches is zeroized, and both files are in
`tools/lint-secrets.sh`.

**Three primitives written rather than taken.** No crate is added; none
is justified in `docs/deps/`, because none was needed.

- **AEZ v5** (`osk_bip::aez`, ~450 lines). There is no maintained Rust
  AEZ. The scheme is AES's round function used as a public permutation —
  no whitening key, MixColumns in the last round too — under two key
  schedules, plus `AEZ-hash`, `AEZ-prf`, `AEZ-core` and `AEZ-tiny`. It is
  a line-for-line port of the reference code as Yawning Angel's Go
  implementation holds it, which is the implementation LND calls. Every
  vector the reference publishes passes: `extract.json`, `hash.json`,
  `prf.json` and all 1036 encryptions, which are every message length
  from 0 to 511 bytes at two expansions.
- **scrypt** (`osk_crypto::scrypt`, ~90 lines). The tree already had
  PBKDF2-HMAC-SHA-256, which is scrypt's whole outer structure; what
  was left was Salsa20/8, `BlockMix` and `ROMix`. RFC 7914's first two
  vectors pass. The one allocation is `ROMix`'s `V`, 32 MiB at aezeed's
  `N = 32768, r = 8`, zeroized before it is freed — which is why the
  decode runs once, on ✓, and not on every frame.
- **BLAKE2b** (`osk_crypto::blake2b`, ~120 lines). AEZ's `Extract`
  hashes any key that is not already 48 bytes, and LND's key is
  scrypt's 32, so every aezeed decode runs it. RFC 7693's own vectors
  pass.

**The screen, and where it departs from the brief.** "Show secret" was
specified as one Secret screen carrying both the entropy and the node
private key. A §5 Secret screen has one panel and its rows are facts,
not secrets, so two secrets cannot both be behind the eye on one
screen's panel. The screen instead carries the node private key and,
where the source was a cipher seed, a second action beside Done —
"Entropy" — that swaps the panel and retitles the screen, with "Node
private key" swapping back. One screen, one eye, both secrets, and
nothing unmasked that should not be.

**The word entry is Load's, with one rule suspended.** An aezeed is not
a BIP-39 mnemonic: its twenty-four words carry no BIP-39 checksum, and
its own check is the CRC-32C inside the ciphertext. So `LoadWizard`
gained `set_free_last`, which suppresses the final-word mask — the
filter that restricts the twenty-fourth word to the ones BIP-39's
checksum allows — and the wizard's Checksum screen is never drawn here:
the moment the last word lands the indices are taken and the wizard is
dropped. A cipher seed's real refusals are the ones the Result states:
"The words do not check out" (the CRC-32C), "Wrong passphrase" (AEZ's
four-byte expansion, which cannot tell a wrong passphrase from bytes
that were never an aezeed) and "Not a version 0 cipher seed".

**A loaded key needs words.** ldk-node takes a BIP-39 seed and nothing
else, so the "A loaded key" row counts only keys with a 64-byte seed
and is dimmed "no key with words" where there are none. A SLIP-39 or
codex32 key has a 16- or 32-byte master secret, which ldk-node has no
derivation for.

**What the sources got wrong or left unsaid.**

- **LND's own test vectors are not decodable by LND.**
  `aezeed/cipherseed_test.go` ends with an `init()` that sets
  `scryptN = 16` for the whole package, and `scryptN` is a package
  variable, so `version0TestVectors` — the twenty-four words the
  package publishes — were enciphered under a key no released wallet
  derives. They are vectors of the scheme, not of the product. Hence
  `decode_at_cost`, which the published words are checked through at
  `N = 16`, beside `decode` at 32768 over the same three seeds
  re-enciphered at the production cost by running LND's own package
  here. Those three are in `tools/reference/aezeed/README.md` §4.
- **The comment beside `EncipheredCipherSeedSize` says tau is 8 bytes.**
  The constant beside it, `CipherTextExpansion`, is 4, and 4 is what
  the code passes. 33 = 1 + 19 + 4 + 5 + 4 only with 4.
- **No published node-key vector exists.** `lncli` documents no worked
  example from a seed to a node public key, so the vector the tests
  assert was computed once here by running LND's `aezeed` package and
  btcd's `hdkeychain` at commit `88959aec`, and is recorded with the
  command in the reference README. Go is not a toolchain this
  repository builds with; it was installed locally for the pass and
  nothing in the tree depends on it.
- **ldk-node's `network` does not reach the node key.** `derive_xprv`
  and the builder both pass `config.network`, but the master key is
  used only for its private key and chain code, and LDK's own
  `KeysManager::new` hard-codes `Network::Testnet` for the second
  master. Mainnet and testnet give the same node id, which the tests
  assert.
- **LND's word list is BIP-39's English list, to the byte.**
  `aezeed/wordlist.go` says it is "the *same* word list that's
  recommend for use with BIP0039"; the 2048 words of that literal hash
  to `2f5eed53…`, which is the digest
  `core/osk-bip/src/wordlists/english.rs` already records. So the tool
  types a cipher seed on a list this device already carries, with the
  same keyboard and the same candidate strip.

**What is not built, deliberately.** No aezeed is created, no channel
backup is read or written, and nothing Lightning is signed. `aezeed`'s
`ChangePass` — re-enciphering a seed under a new passphrase — is a
writing operation and is not here either.

**Reopen** if LND publishes a version 1 external scheme, which would
change the byte after the words and nothing else; or if a maintained
Rust AEZ crate appears, at which point `osk_bip::aez` becomes a
dependency decision rather than a file.

### 16.117 A vanity address, ground from a key's own dials (2026-09-19)

§8.1 row 19 as built (pass P3). A key's page gains **Vanity address**:
"How?" (Passphrase counter · Account index), "Which script type?", the
prefix on the address keyboard with the type's fixed characters already
in the field, then the run — "Tried", "Rate", "Expected", Stop — and
the find: the address, what reaches it, and "Use it". The passphrase
dial's find opens a passphrase key exactly as "Open with passphrase"
does; the account dial's adds the single-sig wallet at that account.
Leaving discards the find.

**Two dials, and what each costs.** `osk_bip::vanity::grind` takes the
key, the dial, the script type, the prefix, the network, a cursor and a
budget, and answers with what it tested and the first candidate that
matched. The passphrase dial derives a whole key per candidate — PBKDF2
over the words, 2048 rounds of HMAC-SHA-512, then the BIP-32 path — and
the account dial one hardened child of a `purpose'/coin'` node derived
once per call. On this box, in release, the two run at **670 and 6 700
candidates a second**; in the debug profile `just` builds, 600 and
8 000, because the work is in `sha2` and `libsecp256k1` and not in the
loop. **No Pi was measured**: none is attached to this box, so the
figure for Tier A hardware is not stated anywhere, and what the Choice's
second line says instead is what one candidate costs — "a key derived
each try", "an account derived each try" — with "slow here" at the
trailing edge on a Tier A device. The screen states the rate it is
actually managing, which is the honest number wherever it runs.

**No thread, a budget a frame.** The core has one thread, so the run is
a cursor and a budget: each `Event::Tick` spends a budget of candidates
and comes back, and the next budget is resized from the wall clock so
that a tick stays near 100 ms and Stop lands promptly. The first budget
is eight, which is the tick the rate is first measured over. The find
does not depend on the budget, because the order is the counter's;
`core/osk-bip/tests/vanity.rs` grinds one prefix in slices of three and
in one run of sixty-two and asserts the same counter.

**EntropyLab's counter, and where it stops agreeing.** The alphabet is
`a`–`z`, `A`–`Z`, `0`–`9` with `a` as zero, most significant character
first, quoted from `vanity-wasm/src/lib.rs` in
`tools/reference/vanity/README.md`. EntropyLab's screen asks for a
passphrase length and grinds one width; this device asks no such
question, so it grinds width 1 (62 counters), then width 2 (3 844), and
so on. Inside a width the two orders are the same counter, and the two
vectors the tests assert are EntropyLab's own output, from running its
committed WebAssembly once here: over BIP-39's zero-entropy words and
the prefix `bc1qq`, the passphrase dial's first match is counter 8,
counter text `i`, `bc1qqv2kx2d9tv4d3epztk59dc5kymkl8pk8scu6xz`, and the
account dial's is account 31,
`bc1qqpat9khft6dnm9qp0nnrvpyvmyg2ytshn7gglv`.

**The expected figure is EntropyLab's arithmetic too.** Each free
character is one of the encoding's 32 (bech32) or 58 (base58), and a
first free place that carries fewer is counted at what it carries.
EntropyLab has one such place, a silent payment code's parity
character; this device has one of its own, which is the finding below.

**What the design got wrong or left unsaid.**

- **A key's passphrase is not kept, so there is no base to extend.**
  The brief's `Passphrase { base: the key's passphrase }` cannot be
  filled in: §16.67 is explicit that a passphrase key is the session's
  and the passphrase is not written down anywhere, so the base is
  always empty. The parameter stays in the engine, because it is what
  the scheme is, and the screens pass nothing; the passphrase dial is
  dimmed with "has a passphrase" on a key that already carries one,
  since grinding its words would name a sibling of its parent rather
  than a child of it.
- **A grind has no total, so §4.14's bar cannot be drawn.** §4.14's
  progress is "a bar with a count '2 of 5' above it". A grind counts
  towards no end: the expected figure is a mean of a memoryless
  process, and a bar drawn against it would state a progress that does
  not exist. The run is a §5 Record — three facts and Stop — and the
  count stands without a bar.
- **A legacy address on a test network has no fixed prefix.** The
  brief names `bc1q`, `bc1p`, `1` and `3`, which are mainnet's. On
  testnet, signet and regtest a P2PKH address begins with `m` or `n`,
  so the field starts empty and `vanity::first_free` restricts the
  first character to those two — without which a person could ask for
  `1…` on testnet and wait forever. P2SH is `2`, and the bech32 prefixes
  follow the chain's own HRP (`tb1q`, `bcrt1p`).
- **`Account { purpose }` is the script type's purpose.** A second name
  for it would let the two disagree, and an address at
  `m/49'/0'/n'` that a `wpkh` wallet then claims is no one's address.
  `Method::Account` carries nothing and the script type decides.
- **A counter is part of a passphrase.** The brief puts the passphrase
  suffix on the Result. §4.10 says whatever the entropy becomes is a
  secret, so the Result states the address and how many candidates it
  took, and "Show it" opens a §5 Secret screen with the counter behind
  the eye — §16.116's shape, for the same reason.

**Not in this entry.** No multi-threading (the core has one thread and
the shells drive the ticks), no GPU, no change to the passphrase rules,
no silent payment codes (EntropyLab grinds those; this device's dials
are a key's own), and no grind over a script type the device does not
already build a wallet from.

**Reopen** if a shell gains a way to hand the core spare frames, at
which point the budget could grow without the tick growing with it; or
if EntropyLab changes its alphabet or its odometer, which
`tools/reference/vanity/README.md` would show as a changed digest.

**E3, the design (2026-09-19).** Notes and recovery sheets kept on Tier
B live in a fourth record of the blob, after the wallets record: a
count and eight slots of 4 352 bytes, each one item as the container's
kind-3 or kind-4 plaintext (the kind, the length, the bytes), sealed
under the same key as the wallets record with a nonce label of its
own, and zero where no item is. That lengthens `BLOB_LEN`, so the
version moves to 5 — and, unlike §16.53's rule that an earlier version
is refused, a version-4 blob is read: its header and three records are
at the same offsets, the notes record is simply absent, and the first
write after opening rewrites it as version 5. Phones already hold kept
keys, and a bump that lost them would cost every owner for a notes
feature. Version 3 and below stay refused. A note or a sheet is kept
when the person says so on its Document ("Keep on this device", a
toggle as a wallet's row is), off by default, and comes back to Tools ›
Notes or the wallet's Recovery sheet after a restart; Forget removes
it from the record; Wipe and the duress PIN take the record with the
rest.

**Pass E3 landed.** The blob is version 5: a fourth record after the
wallets record, sealed under the same key with a nonce label of its own,
holding a count and eight slots of 4,352 bytes. A slot is one item as
the `osk-backup` container writes it — the kind byte, a little-endian
`u16` length and the payload — so a kept note and a note in a file are
the same bytes through the same encoder and decoder
(`encrypted::sheet_payload` and `encrypted::payload_of`, both factored
out of the file paths for this). `keep_notes.rs` is the item side of it,
as `keep_wallets.rs` is the text side of the wallets record; it is in
`tools/lint-secrets.sh`, because a note is what a person wrote.

A version-4 blob is read: its header and its three records are at the
same offsets and its bytes are untouched, so it opens under the same PIN
and the same element answer, and `open` answers with no items. The first
`rewrite` after that grows the buffer, writes the notes record and sets
the version byte to 5. `rewrite` therefore takes a `&mut Vec<u8>` rather
than a slice.

**What the design did not say: the version byte cannot be authenticated
any more.** Every record is bound to the header as associated data, and
the duress record can only be sealed by the duress PIN's own element
answer, which an upgrade does not have. Changing byte 0 from 4 to 5
would therefore leave an upgraded blob with a duress record nothing
opens — a duress PIN that silently stops wiping, which is worse than
anything the byte was protecting. So the associated data now carries the
constant 4 in that position whatever the blob says, which is
byte-for-byte what a version-4 blob was sealed under. The version byte
still decides the layout and `header` still checks it against the blob's
length, so an edited byte opens nothing; what an attacker gains is the
ability to truncate the notes record off a blob undetected, which is a
strictly smaller version of deleting the blob, something they could
always do. A test keeps a duress PIN across the upgrade.

The screens are the design's: a note's Document and a wallet's Recovery
sheet carry §4.2's "Keep on this device", the same label and the same
shape a wallet's page carries, absent on a device that keeps nothing and
dead with "8 kept" once the record is full — which needed `Row::Toggle`
to carry §4.11's reason, as `components::toggle_row` already could.
Flipping it on or off goes through `sync_kept`, the path the wallets use.
Forget on a kept note takes it off the device as well as out of hand.

Three things the design left unsaid.

A kept sheet whose wallet is not in use is listed under Tools › Notes by
the wallet's name, with "Recovery sheet" as the row's value, and opens
on the same Document an arrived sheet opens on — so its one action is
"Add this wallet", after which the sheet is on that wallet's page again
and off the Notes list. The sheet carries its note back with it.

A kept sheet follows its wallet. A name given or a note written after
the sheet was kept is in the record at the next write, because the
record is built from the live wallet where one matches the kept
descriptor; a sheet with no wallet is left exactly as it was, which is
what makes it still a document.

A note is identified by its text. Nothing on the device titles a note,
so "is this note kept?" is "are these bytes in the record?", and the
Notes list shows the note in hand once rather than twice when it is also
a kept one. A slot holds 4,349 bytes and `MAX_NOTE` is 4,096
*characters*, so a note written entirely in a script of four-byte
characters would not fit a slot; `KeptNote::new` refuses it and the
toggle does not go on. No keyboard on the device types such a note
today, and the case is not worth a second length rule on the entry.

The Android shell was checked and asserts nothing about the blob's
length: `KeptSecret.kt` reads the wrapped bytes and requires only that
they are longer than the GCM IV, so the record's growth is invisible to
it. `fuzz/fuzz_targets/keep_open.rs` was already stale against
`Kept`'s current shape and is brought back to the new signatures; it is
outside the workspace and `just` does not build it.

### 16.118 A word is taken by a tap and by nothing else (2026-09-21)

**Why.** The owner typed words on the Pi's panel and found the word
committing itself the moment its last letter landed: the screen moved on
without an act of theirs. §16.50 had already put a tap in front of every
candidate on `small` and left one exception, "a spelled word that is
typed out in full still commits itself on every class". The owner's
direction is that there is no exception: a person finishing a word wants
to see it and take it, on the panel above all, and on a phone as well.
And ✓ has no work left once that is so, since it only ever accepted the
one word a tap could also take. Two more things on the same screen: the
field and the candidate cells on `small` are taller than the panel can
afford, and the strip shows one row of three where two rows would show
the person more of what their letters left.

**The rules.**

1. **Nothing commits a word but a tap on its candidate.** A word typed
   out in full is the one candidate left and stays in the strip until it
   is tapped. The checksum filter leaving one word for the last place,
   a reading leaving one character, a prefix leaving one word: every one
   of them is a candidate to tap. §16.50's two-tap rule on `small` is
   unchanged, and a lone candidate is already selected there, so it
   takes the one tap it always did. On `mobile` and `wide` one tap
   accepts, as before; the lone candidate keeps its outline there.
2. **✓ leaves every wordlist keyboard.** The BIP-39 letter keyboard, the
   kana grid, the jamo keyboard, the pinyin keyboard and the 注音
   keyboard carry no ✓. The backspace takes the bottom-right cell of
   each: the last cell of the BIP-39 letter rows at the 1.5-unit edge
   width, the seven letters of row three filling the rest; the grid's
   bottom-right cell on the kana keyboard; the right end of the last row
   on the other three, where ✓ stood. Shift stays at the left of the
   jamo row. Every other keyboard — passphrase, path, hex, address,
   codex32, the pads — keeps its ✓, because on those the field is the
   value and nothing else can take it.
3. **A physical keyboard's Enter takes the selected candidate**, or the
   one candidate left, and does nothing with several, which is what
   Enter did through ✓ and is not an auto-commit: it is a key the person
   pressed. Tab reaches the cells, as §4.15 already allows for chips.
4. **The `small` entry group is shorter.** The field is 40 dp there, a
   letter key's own height; a candidate cell is 36 dp; the strip is two
   rows of three. The floor for a candidate cell on `small` is therefore
   36 dp, and §3's sentence says so; chips keep 44. On `mobile` and
   `wide` the field, the cell and the strip are unchanged. The panel is
   268 × 358 dp: the app bar, a 40 dp field, two 36 dp rows and three
   40 dp key rows come to about 320 dp with the gaps and the caption
   line, which leaves room and no scrolling.

**Rejected.** Keeping ✓ as a second way to take the lone candidate: two
ways to do one thing on a screen whose whole point is that a word is
taken deliberately, and a key that is dead most of the time. A confirm
step after the word instead of before it: the strip is that step.
Dropping the two-tap rule on `small` now that every class taps: §16.50's
reason, a fingertip covering a 4.3 mm cell, has not changed.

**Where the rule is written once.** `LoadWizard`'s entry is the engine
under Load, Create's word steps, a SLIP-39 share, an aezeed's words, the
word list's search and Explore's typed words, so the change to it is one
change. The quiz picks from candidates already and is untouched.

**What the tests and scripts encode.** `load.rs`'s `type_word` helper
and the test named for auto-commit become the rule they now state: a
fully typed word is the only candidate and a tap takes it. The snapshot
scripts that type a word and rely on the commit — twenty-one of them —
tap the candidate after the letters; a `word` script command in the
snapshot shell that types the letters and taps cell 0 is the one place
to write that, since a fully typed word is always cell 0.

**In one pass, W1.** Rules 1 to 4, the DESIGN §4.3 and §3 lines, the
tokens, the keyboard layouts, the snapshot shell's `word` command and
every script, and the tests.

**Pass W1 landed (2026-09-21).** Rules 1 to 4 as written. `load.rs`'s
`commit_exact` is gone with its three callers, so no key commits a word
on any class or any list; `commit_only` became `commit_selected`, which
takes the selected candidate or the one left and is reached only from a
physical Enter. The BIP-39 row three is the seven letters and the
1.5-unit backspace; the jamo, pinyin and 注音 keyboards end their last
row with the backspace; `tokens::field(class)` is 40 dp on `small`;
`CANDIDATE_CELL_SMALL` is 36 dp and the strip there is two rows of
three. The snapshot shell gained `word "abandon"`, which types the
letters and taps the candidate, and eighteen scripts use it. 1248 tests.

**What the design got wrong or left unsaid.**

- **The kana grid has one cell too many.** The 五十音 leaves six cells
  over and, with ✓ gone, five things to put in them, so one cell is
  empty whatever is decided; and DESIGN §4.3 still said ん takes the
  わ column's last cell, which is the cell rule 2 gives the backspace.
  As built: the backspace bottom-right, ん above it, and the わ
  column's う cell empty. `KANA_GRID` is `[[Option<KeyInput>; 10]; 5]`
  and a tap on the empty cell does nothing.
- **The empty cell breaks UX §6's "every pixel belongs to a key"**,
  which `hit_rects_tile_the_region_out_to_its_edges` enforces. The
  alternatives — a two-cell backspace, a double-width kana — break
  "every key one unit wide", so the test allows exactly one gap, in the
  kana grid, and nowhere else.
- **`tokens::FIELD` had a third reader**: the dots field over the PIN
  pad. DESIGN §4.3's Text field row states no exception, so the PIN
  field is 40 dp on `small` as well, which the entry did not say.
- **DESIGN §3's token sentence did not carry the 36 dp floor** the entry
  said it did; it does now.
- **The read lists needed the tap on every class**, not only the
  spelled ones: a reading that left one character used to commit on
  `mobile` and `wide`, and the two Chinese scripts tapped it only under
  `on small`. Both taps are unconditional now.
- **`word` cannot always tap cell 0.** A lone candidate is drawn as one
  chip under the strip's own id with no cell rectangle, so the command
  falls back to the chip, as the test harness already did.
- **Enter has one script line covering it**: `load-key.txt`'s last word
  still ends with `key enter`, and nothing else exercises rule 3 in a
  snapshot run.

**Open.** Two snapshot scripts fail at HEAD before and after this pass
and are not this pass's: `create-key.txt` line 34 (no widget 634) and
`key-detail.txt` line 79 (`BACKUP_SHOW_QR`, one screen behind since
§16.112's "Which form?" Choice). They need a script pass of their own.

### 16.119 The check runs in a minute, and a pass looks at one screen (2026-09-21)

**Why.** Pass W1 (§16.118), a change of a few lines in the entry engine
and the keyboard rows, took the implementer 53 minutes and the
orchestrator another 50 around it. The owner's verdict: the workflow is
wrong, and the check is too slow. Measured on this box, warm, nothing to
compile:

| What | Wall | Note |
|---|---|---|
| `cargo test --workspace` | 3 min 10 s | 108 test binaries run one after another; 188 s of test time in all, 1417 s of CPU |
| `cargo nextest run --workspace` | 1 min 41 s | the same tests, binaries in parallel; the floor is the slowest single test, 36 s |
| `cargo test --workspace --doc` | 2 s | 12 doc tests |
| compile after a one-file change, tests | 7 s | |
| `cargo clippy --workspace --all-targets` after the same change | 3 s | |
| `cargo build -p opensigner-snapshot --release` after the same change | 38 s | fat LTO on one core, paid by `just snapshots` on every change |
| `just snapshots` | 27 scripts × 4 sizes | 108 runs, most of them of screens the change did not touch |

Where the CPU goes: `tests/keep.rs` 303 s and the two backup files 81 s,
because the app hard-codes the kept-key blob's 64 MiB Argon2id and the
backup's 64 MiB setting and every keep, unlock, seal and open in a test
pays it (`keep.rs` says "a test writes its own through `Header::cost`",
and no test can, since the cost is chosen inside `start_keeping`);
`tests/layout.rs` 260 s and `tests/explore.rs` 81 s, where sixteen tests
walk every screen at every size inside one `#[test]`, so the two longest
run 36 s each on one core while fifteen cores wait. The median test is
56 ms.

**The rules.**

1. **`just test` is nextest, then the doc tests.** Binaries run in
   parallel. `cargo install cargo-nextest --locked` is a prerequisite in
   the README. The check runs once per pass, at the end; while working,
   an implementer runs the one test file it touched. CLAUDE.md and the
   implementer's own instructions say so.
2. **A screen is looked at one script at a time.** `just snap <script>
   <size>` renders one script at one of the four sizes into
   `out/snapshots/<size>/`, in the dev profile, which is the build the
   tests already made and in which the renderer is already optimised
   (Cargo.toml's dev profile). `just snapshots` uses the same profile
   and is for a change to the layout engine, not to a screen.
3. **A test pays a test's KDF cost.** `opensigner-core` gains a
   `test-hooks` feature, enabled for its own tests through a
   self-dev-dependency and by no shell, under which the harness sets the
   Argon2id cost the app uses for the blob and for backups to Argon2id's
   own minimum. Argon2id's parameters are stored in every header, so
   nothing else changes: a blob or a file written cheap opens cheap, and
   the cost `DEVICE_PARAMS` states is still what the device writes. The
   tests that are about the cost — the header's parameters, "Needs
   65536 MiB", the 256 MiB file that opens — keep the real one.
4. **A test that walks every size is one test per size**, where it
   costs more than a few seconds, so that nextest spreads it. What each
   asserts does not change.

**Target.** `just` under a minute on this box, warm; a screen change
under ten minutes end to end.

**In one pass, T1**: rules 3 and 4, measured before and after with
`time just`. Rules 1 and 2 are the justfile and the two instruction
files, done by the orchestrator in this entry's commit.

**Pass T1 landed (2026-09-21), and found the entry's diagnosis wrong.**
Rules 3 and 4 are built: `opensigner-core` has a `test-hooks` feature,
on for its own tests through a self dev-dependency and on for no shell
(`cargo tree -e features` shows it nowhere else), under which
`OpenSigner::set_kdf_cost` puts the blob's and the backup's Argon2id at
`keep::MIN_PARAMS`; the harness sets it, `Harness::at_device_cost` does
not, and the two tests that assert the cost of bytes the app wrote use
that one. Fourteen size-walking tests in `layout.rs` and `explore.rs`
are one test per size through an `at_every_size!` macro; 1236 tests
became 1278 and nothing asserted changed. What it bought: 1 min 38 s to
1 min 33 s. The entry read the suite as bound by its slowest test; it
is bound by total CPU — 1408 test-seconds over sixteen threads is 94 s,
and nextest was already packing them — and Argon2id was about 50 s of
that, not 384: the kept-key file alone measured 264 s at the device cost
and 221 s at the minimum.

**Where the CPU actually goes: painting pixels nobody reads.** The
orchestrator measured it after the pass by skipping `canvas.clear` and
`draw_tree` in `render()` and running two files. `tests/keep.rs`: 16.7 s
to 0.36 s. `tests/explore.rs`: 23 s for its longest test to 0.25 s for
all seventeen. The core paints a full frame on every event — a phone
frame is 2.5 million pixels through tiny-skia — and a harness tap is
two events plus a reveal, so a flow test is a few hundred paintings of
frames no test looks at. Eleven tests read pixels; the other 1267 do
not.

5. **The core paints when the frame is read, not when an event lands.**
   `render()` keeps what it does before the paint — build the tree,
   solve the layout, keep the focus, note whether a QR is visible,
   scroll into view, push `Command::Draw` — and keeps the tree beside
   the layout instead of painting; `App::frame` becomes `&mut self`,
   paints the kept tree onto the canvas if an event has solved a new
   layout since the last paint, and returns the pixels. A shell reads
   the frame once per frame it draws, so it paints once per drawn frame
   rather than once per event, which is also what a phone wants while
   a finger drags. The pixel-moving scroll (§16.36) works on a canvas
   that holds the last painted frame, so it is taken only when nothing
   has been solved since that paint; otherwise the full solve runs as it
   does today. `Command::Draw` keeps its meaning: read the frame and
   blit it. The proof is that every snapshot is byte-identical before
   and after, which is the one legitimate use of `just snapshots`.

**In one pass, T2**: rule 5, across the core, the contract crate and
the four shells that read a frame.

**Pass T2 landed (2026-09-21).** Rule 5 as written. `App::frame` takes
`&mut self` and paints there when an event has solved a layout since
the last paint; `render` keeps the tree beside the layout and sets the
flag instead of painting; `pixels_can_move` requires the canvas to hold
the frame of the layout in hand, so the pixel-moving scroll is taken
only after a paint and falls through to the full solve otherwise. The
four shells and the FFI follow the signature; the Android side calls
the same FFI function and did not change. Every snapshot is
byte-identical: 1 526 files at the four sizes compared, 0 differences.
`just`, warm: **1 min 45 s to 14 s**, the workspace tests 94 s to 9 s,
`tests/layout.rs`'s 97 tests in 1.4 s. The slowest test is now a
3.3 s BIP-39 cross-check in `osk-bip`; nothing in the top ten touches
the renderer.

**What the design got wrong or left unsaid.**

- **An invariant the paint relies on, stated here for later changes:**
  `frame` paints with `self.ui` as it is when read, not as it was when
  the layout was solved. That is correct only because every event that
  changes what `draw_tree` reads — focus, a press, a reveal, a tick that
  moved anything — is an event that renders, and an event that changes
  nothing pushes no `Draw`. The 1 526 identical files and the eleven
  pixel-reading tests are the evidence; a future event that changes
  `UiState` without rendering would break it silently.
- **The scroll fast path's tests had to act like a shell.** With the
  precondition on the paint, `drag` and the wheel test would have fallen
  through to the full render every time and stopped testing the path
  they are named for; both call the harness's `blit`, which reads the
  frame as a shell does on `Draw`. Nothing asserted changed.
- **Nothing had to paint eagerly.** `Draw` is pushed only by `render`
  and `scroll_moved`, so a frame is never announced without a solve
  behind it, and `Event::Display` needed no special case.
- **The tree kept beside the layout holds the screen's strings**,
  revealed words among them, until the next render replaces it, where
  before it was dropped at the end of `render`. Lock, wipe and every
  screen change render, so the replacement is immediate, and neither
  the old drop nor the new replacement zeroizes; `LoadedKey` holds the
  same words sealed. A longer life for a copy, not a new exposure;
  `lint-secrets.sh` passes. Open under §5: whether lock and wipe should
  clear the tree rather than replace it.
- **`just snapshots` stops at the first script that fails**, so the two
  scripts broken since §16.112 (`create-key.txt`, `key-detail.txt`)
  stopped the reference render at the sixth script; the pass rendered
  with a loop that continues. Both scripts were one screen behind —
  Create's "Which procedure?" (§16.115) and Backup › Encrypted's "Which
  form?" (§16.112) — and are fixed in the commit after this one; the
  full recipe then runs end to end, 1 776 files in 3 minutes.

### 16.120 A key shows it was pressed, and shows it a moment after the finger lifts (2026-09-21)

**Why.** The owner, typing words on the panel after §16.118: a key
gives no sign of being pressed, so a finger that covers most of a key
cannot tell which one it hit until the letter appears in the field; and
the same is true of a candidate cell, which is what §16.50's two-tap
rule on `small` was working around. Trezor answers both with a brief
highlight that outlasts the lift, so the person sees the key after the
finger is off it. Android's ripple is the same idea. What this device
does today: rows, buttons, chips and tiles draw a pressed fill while a
finger is on them and nothing after; keys and candidate cells draw
nothing at all (`UiState::is_pressed` says so in its doc comment).

**The rules.**

1. **A key under a finger is drawn pressed.** The key face's fill mixes
   `PRESSED_MIX` towards white, as a pressed row does; the primary
   key (✓ on the keyboards that keep one) and the raised keys
   (backspace, shift, symbols) mix from their own fill. A dead key is
   never drawn pressed. The pressed key is the one under the press
   point, found as the release finds it (`keyboard::key_at`), so a
   finger that slides off a key before lifting stops showing it.
2. **A candidate cell under a finger is drawn pressed**, with the same
   fill a pressed chip has; a selected cell (the accent) stays as it
   is. The cell is the one under the press point as the release finds
   it.
3. **The pressed look outlasts the lift by `PRESS_LINGER_MS`**, 100 ms,
   on everything that draws one: keys, candidate cells, rows, buttons,
   chips, tiles, the pager's arrows. `UiState` records the target and
   the point at the release; the drawing treats it as pressed until
   the window closes; the tick that closes it asks for a redraw, as the
   reveal flash does (`REVEAL_FLASH_MS`). Every shell ticks at 50 ms,
   so the window closes within two ticks. A hold and a reveal are not
   taps and get no linger: the reveal re-masks on the lift and the hold
   completes under the finger. A linger never shows on a screen other
   than the one it was pressed on: a screen change clears it, since ids
   repeat from screen to screen.
4. **No animation.** The look is on or off, one colour, one window. A
   ripple would cost frames the panel does not have to spare and would
   say nothing the fixed window does not.

**What §16.118 already settled, restated because the owner asked for
it again:** a word is taken by a tap on its candidate and by nothing
else, on every list — BIP-39 in every language, SLIP-39, an aezeed's
words — and a word typed out in full is the one candidate left, drawn
selected on `small` and outlined on `mobile` and `wide`, waiting for its
tap.

**Snapshots.** A `tapid` in a script is a press and a release with no
time passing, so a snapshot taken right after one shows the tapped
widget in its linger where it is still on screen: a checked Choice row,
a toggle, a candidate. That is what a shell would draw at that instant,
so it is what the snapshot shows; a script that wants the settled
screen ticks past the window first. The reference snapshots change
accordingly and are looked at, not diffed for identity.

**In one pass, W2.** Rules 1 to 4, `docs/DESIGN.md` §4.3's Keyboard
and Candidates rows, tests that a pressed key and a pressed cell are
drawn so and that the linger closes on the tick.

**Pass W2 landed (2026-09-21), with two additions the owner made during
it.** Rules 1 to 4 as written, and: a selected cell shows its press and
its linger too, the accent mixed `PRESSED_MIX` towards white as a
pressed primary button is — rule 2's "stays as it is" is withdrawn —
and the strip is a fixed grid of `candidates_per_row` equal cells on
every class but `wide`, filled from the left and blank after, the lone
candidate in the first cell, so a cell is the same width whatever the
count; `Widget::outlined_chip` and the harness's chip fallbacks went
with the full-width lone chip. `Press` carries the key and the cell
found under the press point at `Down`, by the arithmetic the release
uses; `linger` is recorded at `Up` for a tap, a key and a cell, never
for a hold or a reveal or a moved finger; `tick` closes it as the flash
is closed; `OpenSigner::entered` clears it, and deliberately not the
press, since a completed hold navigates with the finger still down.
`draw_keyboard` mixes the pressed key's own fill. Eight tests. 1 285.

**What the design got wrong or left unsaid.**

- **The strip had no size fallback.** The Chip drops a label to the
  caption size when it does not fit; the strip did not, and a fixed
  grid cannot widen a cell, so the Japanese lists would have clipped.
  The strip now takes the Chip's rule, one size for the whole row.
  English, Korean and both Chinese lists never drop on `small`;
  Japanese's six-kana words do, and read fine. The size is chosen per
  row, so a second row of short words can be larger than a first row
  that dropped — visible in `load-ja-03-word-1-in`. One size for both
  rows needs text metrics the component does not have; open.
- **No script shows a Choice row in its linger**, because every Choice
  tap in the scripts is followed by Continue, which changes the screen
  and clears it. The eye before `load-06b-word-2-eye` is the nearest.

### 16.121 The reading keyboards on the panel, and one size for a strip (2026-09-21)

**Why.** The owner, on the panel: Tools › Word list in Traditional
Chinese draws the "Search by" mode row underneath the text field. The
arithmetic on a 268 × 358 dp panel: the app bar (56), the entry group
with the one-character strip (a 40 dp field, the caption line, two
32 dp rows of characters: about 132), and a four-row keyboard at the
40 dp key (160 plus the bottom reserve) leave nothing above the field.
§4.3's rule — drop the keys to the 36 dp floor, then draw the row on
one line — runs out at the second step with about 2 dp of room, and
the row is drawn anyway, over the field. The same is true of the
pinyin keyboard; the address and codex32 keyboards have four rows too
but no strip and fit. The kana grid met the same problem first and
answered it with `KEY_HEIGHT_GRID`: five rows at 28 dp on `small`.

And, from pass W2: the candidate strip picks its text size per row, so
a second row of short words can be drawn larger than a first row that
dropped to the caption size, which the Japanese list shows.

**The rules.**

1. **A reading keyboard on `small` takes the grid's key height.** The
   pinyin and 注音 keyboards, whose entry group carries the two-row
   one-character strip, use `KEY_HEIGHT_GRID` with `KEY_MIN_HEIGHT_GRID`
   as their floor on `small`, as the kana grid does: four rows of keys
   over two rows of characters are the same height problem as five rows
   of kana. With 112 dp of keys the mode row fits on two lines with
   room to spare and §4.3's one-line fallback is never reached there.
   `mobile` and `wide` are unchanged.
2. **Nothing is drawn over the field, and a test says so.** For every
   Entry screen a layout walk reaches, at both `small` reference sizes,
   the rectangle of the block above the field does not intersect the
   field's. §4.3 already states the rule; it now has a test.
3. **A strip is one text size.** The two rows of a candidate strip are
   one widget with two rows, measured together, so one size holds for
   every word shown, dropping to the caption size when any word does
   not fit its cell. The rows' ids, the hit-test arithmetic and the
   paging cell are unchanged.

**Not proposed: a condensed or a mono face.** A mono face is wider
than the proportional one, not narrower, and would fit fewer letters
per cell. A condensed Latin face (Noto Sans Condensed is under the same
licence) would fit the longest English word with more to spare, but no
list drops size on the panel but Japanese, and Noto Sans CJK has no
condensed cut, so the one list that drops would keep dropping. The
per-row inconsistency is rule 3's, not the face's. Reopen if a language
is added whose words do not fit at the chip size.

**In one pass, W3.** Rules 1 to 3, the DESIGN §4.3 Pinyin and 注音
rows' key height, a test per rule.

**Pass W3 landed (2026-09-21).** Rules 1 to 3. The pinyin and 注音
keyboards take `KEY_HEIGHT_GRID` and `KEY_MIN_HEIGHT_GRID` on `small`,
where `draw_keyboard` already used the dense keycap; the strip's two
rows each carry the other's words and measure both against the same
cell, so one size holds across the strip; and
`no_entry_screen_is_drawn_over_its_field` walks every Entry screen a
test can reach — the word list and Load's entry in all ten languages,
the codex32 string, the passphrases, the path editor, the address, the
vanity prefix, the BSMS token and description, a note, the cipher
seed's words — at both `small` sizes and asserts the block above the
field never meets it. Stashing rule 1 makes it fail on the two Chinese
word lists with the rectangles named; restoring it passes. 1 287 tests.

**What the design got wrong or left unsaid.**

- **"The one-line fallback is never reached there" was wrong by a dp.**
  With the 14 dp bottom reserve and the caption line counted, the grid
  floor leaves 54.8 dp above the field and a two-line row is 56, so the
  mode row is drawn on one line, as §4.3's fallback says, and clear of
  the field. That is the rule holding, not failing; two lines there
  would need a token change and is not worth one.
- **The test needed a hook on what is drawn**, not on the space:
  `screens::BLOCK` is the space the block is given, and the row sits in
  a child that overflows it, so a test against `BLOCK` passed on HEAD
  with the row over the field. `screens::ABOVE` tags the drawn row, the
  typed value, the facts region and the path editor's region, in
  wrapper columns where the node already carried an id, since an id
  overwrites.
- **"One widget with two rows" would have changed the contract**: a
  candidate action numbers a cell within its strip and the two row ids
  are what the app answers to, so the rows stay two widgets that each
  know the other's words, which measures the same thing.
- **The implementer spawned an Explore agent**, against CLAUDE.md, and
  says it caught itself at once, could not stop it, and used nothing
  from it. Recorded because the rule was broken; the pass's content was
  checked as any other.

### 16.122 Leaving the app on a phone does not lock it; the timer does (2026-09-21)

**Why.** §16.48 made the Android shell lock the session the moment the
activity stops, as the cheap and safe answer to a shell that had been
ticking in the background. The owner, using the phone: switching to
another app and back costs the whole state every time, which is too
aggressive; a minute or a few is the right grace. The auto-lock timer
in Settings already says how long an untouched session may stand — 30
seconds, 2, 5 or 15 minutes, 2 by default — and there is no reason a
session left for another app should be held to a shorter rule than a
session left on the table.

**The rules.**

1. **Leaving stops the ticker and locks nothing.** `onStop` stops the
   50 ms ticks, so a backgrounded app costs no battery, and sends no
   `Event::Lock`. The picker exception goes with it, since there is
   nothing to except.
2. **Returning ticks once with the wall clock, and the core decides.**
   `onStart` sends one tick on `elapsedRealtime`, as it does today,
   before the first frame. The core's own timers apply the elapsed
   time: past the auto-lock deadline the session is locked; past the
   auto-wipe deadline the keys are gone; short of both, the screen is
   the one the person left. §16.48's clock rule is what makes this
   honest and is unchanged.
3. **The window is the auto-lock setting**, not a setting of its own.
   One timer, one meaning: how long an untouched session stands.
4. **The recents thumbnail stays blank** (`FLAG_SECURE`), so a session
   left unlocked in the task switcher shows nothing.

**Rejected.** A separate "lock when leaving" setting: a second timer
for the same question. Locking after a fixed grace of one minute
regardless of the setting: a person who chose 30 seconds would get a
longer window by leaving than by staying.

**In one pass, A1.** Rules 1 and 2 in `MainActivity.kt` and
`SignerView.kt`, the comments that say why, and one sentence in Learn's
Devices page saying a phone follows the same timer whether the app is
in front or not. `Event::Lock` stays in the contract for the shells
that have a reason to send it.

**Pass A1 landed (2026-09-21).** Rules 1 and 2. `SignerView.background()`
clears the on-screen flag and stops the ticker and sends nothing;
`onStop` calls it with no picker exception; `foreground()` is as it
was. The Learn sentence is on the Devices page. `just` green, the APK
built.

**What the design got wrong or left unsaid.**

- **The first tick lands before the first frame by construction.**
  `SignerView.send` delivers the event and drains the commands on the
  calling thread, and a `Draw` only schedules a frame, so the tick
  `onStart` sends is applied before `onStart` returns and the first
  frame on return is drawn from the state the deadlines left. Nothing
  can fail if that ordering changes; it rests on `send` being
  synchronous. Recorded so a later change to `send` knows.
- **The picker case is handled better than the exception handled it.**
  A file picked before the auto-lock deadline and delivered after it
  arrives with the lock on: the lock is a session flag over the same
  screen, so `file_event` still finds the Sign flow waiting and loads
  the transaction behind the lock screen, and the review is there after
  the PIN. Signing needs the master keys the lock dropped, so nothing
  can be signed until then. Before, the exception kept the session
  unlocked for as long as the picker stood.
- **The ticker never kept the timers.** The deadlines are absolute on
  the tick clock, so one tick after an hour leaves the session where
  seventy thousand would have; §16.48's trade was the removal of one
  event send, and §16.122 reverses that send.
- **Rule 3 says more than it means.** Auto-wipe has its own setting
  and keeps it; the rule means no new setting is added.

### 16.123 A candidate tap reaches the Seed XOR part it was typed for (2026-09-22)

**Why.** The owner, on the phone: on the Seed XOR entry the keyboard
looks like the BIP-39 one, but a tap on a candidate flashes the cell
and takes nothing. It is the same keyboard and the same strip. The
fault is the core's event dispatch: `Action::Candidate` is handed to
the Load wizard and to the word-list tool and to nothing else
(`OpenSigner::apply`), and Seed XOR parts are typed inside the Create
wizard, which owns a Load entry of its own for that step
(`CreateWizard::entry`). A physical keyboard's Enter takes the word
there, because `KeyboardInput` is routed to the part entry, and that is
how the XOR tests type their words, so they stayed green while the
screen was broken.

**The rules.**

1. **Every candidate tap reaches the entry whose strip is on screen.**
   The Load wizard, the Create wizard's part entry at `Step::XorPart`,
   and the word-list search, through the one `candidate_tap` the Load
   wizard already uses; a part whose last word landed this way
   computes its fingerprint as one landed by Enter does
   (`part_landed`).
2. **A test types a word the way a finger does.** The XOR tests take
   each word with a tap on its candidate, as `type_word` does. One
   test keeps the Enter path, since §16.118 rule 3 makes Enter a way
   in.

**In one pass, X1.** Rules 1 and 2. No screen changes.

**Pass X1 landed (2026-09-22).** Rules 1 and 2. `Action::Candidate`
goes to the Load wizard, else to the gatherer's Create wizard through
`CreateWizard::candidate`, which taps the part entry and lands the part
as Enter does and is a no-op off `Step::XorPart`, else to the word-list
search. `load_words_accepted`, the count the harness reads to know a
tap took a word, now reads the part entry too. The typing test takes
every word by its candidate on both the phone and the panel, and the
fifteen-word test keeps Enter. The candidate test fails on the old
dispatch and passes on the new. 1 287 tests.

**What the design got wrong or left unsaid.**

- **"Stayed green because of Enter" is half the reason.** Before
  §16.118 a word typed in full committed itself, so a test that typed
  the letters never needed the tap; the tests were written then and
  kept Enter after. Only on the current tree is a tap-taking test a
  regression test.
- **The harness's count of accepted words was blind to the part
  entry.** It read the Load wizard alone, so rule 2's test could not
  have passed even with the tap routed. Widened.
- **No `entry_mut`.** Handing out the part entry would let a caller
  tap without landing the part; the method keeps `part_landed`
  private.
- **The Backup flow holds a Create wizard too**, as its gatherer of
  random parts. Its sources never reach `Step::XorPart`, so the bug
  was never there, and the dispatch goes through the gatherer anyway
  so that it cannot be.

### 16.124 A pressed surface changes colour, and the after-image fades (2026-09-22)

**Why.** The owner, after §16.120: the pressed look, and the selected
candidate's pressed look, are too subtle. The numbers agree. A pressed
neutral surface mixes `PRESSED_MIX` (0.18) towards white, which takes a
key from `#1C1C1E` to about `#454547` on a black screen, one step past
the raised backspace's resting `#3A3A3C`; a pressed accent surface goes
from `#FF9F0A` to a slightly paler orange. Both are a brightness step in
the same hue, read through a fingertip, on a 2.8-inch panel. A colour
that nothing else on the screen has is what reads at a glance, and a
short fade after the lift is what Trezor and Android's ripple give:
the eye catches the after-image, not the instant.

**The rules.**

1. **A pressed neutral surface takes the accent tint.** Its fill is its
   own resting fill mixed `PRESSED_TINT` towards the accent — a key
   from `surface`, the raised keys from `surface_raised`, a candidate
   cell, a row, an unselected chip, a tile, a secondary button, a Text
   button from the background, the pager's arrows. `surface_raised`
   stays the resting fill of the raised keys and is no longer a
   pressed colour.
2. **A pressed accent surface mixes further towards white.**
   `PRESSED_MIX` rises to 0.35: the primary button, ✓, the selected
   candidate cell, a selected chip (which today ignores the press), the
   danger button from its own red.
3. **The linger is two steps.** `PRESS_LINGER_MS` becomes 200: the look
   at full strength for the press and the first `PRESS_LINGER_FULL_MS`
   (100) after the lift, then at half strength — both fractions halved
   — for the rest. `UiState` reports the strength of a press rather
   than a bool; the tick that ends each step asks for a redraw, as the
   one tick did. Every shell ticks at 50 ms, so each step lands within
   two ticks.
4. **Still no ripple.** Two steps, two colours, one window. A growing
   circle at 20 frames a second is a stutter, and the panel's frame is
   not free.

**Rejected.** A stronger white mix alone: still the same hue as the
raised keys, still a guess under a finger. An outline on the pressed
cell: the outline already means "the one candidate a tap takes"
(§4.3) and cannot carry a second meaning on the same strip.

**In one pass, P1.** Rules 1 to 4, the tokens, `docs/DESIGN.md` §3's
theme line and §4.3's Keyboard and Candidates rows, the `osk-ui` tests
that say a pressed key is told apart from a resting raised key and that
the look fades in two steps and is gone after. The reference snapshots
change wherever a `tapid` precedes a snap and are looked at.

**Pass P1 landed (2026-09-22).** Rules 1 to 4 as written. `UiState`
reports a `PressLook`, `Full` or `Half`, in place of a bool, from
`pressed`, `pressed_key` and `pressed_cell`; the tick asks for a redraw
at both boundaries. Two helpers in `widgets`, `pressed_neutral` and
`pressed_accent`, are the whole rule, and every pressed site goes
through one of them: keys and the raised keys from their own fill, ✓
as an accent surface, candidate cells, rows, chips, tiles, the pager's
arrows, and every button style. `surface_raised` is a resting fill and
nothing's pressed colour. DESIGN §3's theme line names it "raised
surface" and states the two rules and the two steps. Tests assert that
fills differ from their resting selves and from the raised keys, and
that the look is half after the first step and gone after the second;
no test names a colour. 1 288 tests.

**What the design got wrong or left unsaid.**

- **Rule 1 lists keys and rule 2 lists ✓, and ✓ is a key.** Rule 2
  wins: ✓ is the accent key and mixes towards white, which is what
  §16.120 said too.
- **A Text button's resting fill is the background**, which is black,
  so its press is a dark brown disc: visible, weaker than a pressed
  surface. Making it stronger would take a fill of its own, not a
  different fraction. Left as it is.
- **The app bar's eye shows no press at all.** It is a `Ring`, not an
  `IconButton`, and never drew one; the pager's arrows beside it now
  tint. Two marks in one bar answer a finger differently. Open.
- **A selected chip now shows its press**, as rule 2 says; nothing
  depended on the old look.
- **The half step needs a tick inside each 100 ms step.** Every shell
  ticks at 50 ms. A shell ticking slower than 100 ms would go from
  full to nothing in one redraw; the code is right in that case and
  the fade is simply not seen.
- **Disabled surfaces draw no press**, as before; the entry did not
  say so.

### 16.125 "Load from?" in the order it is used, and two rows that were dead (2026-09-22)

**Why.** The owner: "Type the words" is the default and the row most
people want, and it is sixth, below the fold on the panel; and Word
numbers and Hex entropy are dead rows, which the backlog's item 3 has
promised since the start. The Create wizard already types hex on the
hex keyboard, and the backup words screen already prints the number
beside each word, so a key can be loaded from either with what is
built.

**The rules.**

1. **The rows, most used first**, the default checked and at the top:
   Type the words · Scan a SeedQR · Read an encrypted backup · SLIP-39
   shares · Seed XOR parts · Codex32 · Word numbers · Hex entropy ·
   Secure element, the last dimmed with "needs Tier B" as today. Ids
   do not change, so no script and no test moves; `docs/DESIGN.md`
   §5's source row says the new order.
2. **Word numbers** is a source. Its steps are the typed words' own:
   count, language, then one Entry per word titled "Word 7 of 12", the
   digits typed on the digit pad into the field, the strip showing the
   one word the number names as soon as the digits name one — the
   number the backup words screen prints, 1 to 2048 — and a tap on it
   taking the word, as §16.118 has every word taken. A digit that
   cannot lead to a number in range is dead: `0` on an empty field,
   any digit after 2048, any digit after a number over 204. Backspace
   on an empty field steps back to the previous word, as the words
   step does. The pad has no ✓. After the last word, the checksum step
   as typed words reach it, and the checksum's own rules — the last
   word filtered to the ones the checksum allows, a failure named —
   apply, since a number is a word.
3. **Hex entropy** is a source. Its steps are count, language, then
   one Entry titled "Hex entropy" on the hex keyboard, drawn as the
   Create wizard's hex step is: the digits masked one by one half a
   second after each (§4.10), ✓ dead until the count's digits are in —
   32, 40, 48, 56 or 64 for 12 to 24 words — and ✓ taking them. The
   words follow from the entropy and the language, the checksum step
   reports them valid, and the key finishes like any other. A key
   loaded from entropy is a key that already existed: no sanity check,
   no quiz, as §16.82's XOR combine.
4. **The Learn page the source Choice maps to says one sentence for
   each new row**, and nothing on the working screens explains either.

**Rejected.** A ✓ on the digit pad taking the number: the word is what
is being taken, and one idiom for taking a word is the point of
§16.118. Showing the number's word in the field beside the digits: the
strip is where a word waits, on every other list.

**In one pass, L1.** Rules 1 to 4: `Source::Numbers` and `Source::Hex`
in the Load wizard, the two ids the dead rows get, the two Entry
screens in `views/load.rs`, the strings, two scripts `load-numbers.txt`
and `load-hex.txt` that walk each source to the checksum, tests that a
key loaded by numbers and by hex is the key the same words load, that
a dead digit is dead, and that the rows are in the order rule 1 gives.

**Pass L1 landed (2026-09-22).** Rules 1 to 4. `Source::Numbers` and
`Source::Hex` on the Load wizard, both at `Step::Words` with the source
deciding the keys and the screen as codex32 does. Numbers: the digit
pad, the one word the digits name as the strip's one candidate through
`for_each_candidate`, so the two-tap rule, the checksum filter and its
fallback are the typed words' own; a digit is live iff what it makes
is 1 to 2048, which is rule 2's list in one sentence; backspace on an
empty field steps back and re-opens the previous word's number. Hex:
`RawHex` on the wizard, the Create step's masking, ✓ live at the
count's digits, `submit_hex` building the words and landing on the
checksum result; Back from the entry forgets the digits and returns to
the language, Back from the result returns to the emptied digits. The
digit pad honours a key mask now (`key_bit` has a `Pin` arm), and the
three other pads pass `ALL_KEYS` so nothing changes on them. The rows
are in rule 1's order; DESIGN §5's Choice and Entry rows say so; the
Learn words page has one sentence per new row. Two scripts,
`load-numbers` and `load-hex`, in the snapshot run. Six tests. 1 294.

**What the design got wrong or left unsaid.**

- **"The pad has no ✓" is not built literally.** The digit pad lays ✓
  in its bottom-right cell for the lock, the stored key, Days, Length
  and Index, and taking it away changes those. On this screen ✓ is
  dead throughout, which is what a dead key looks like everywhere else.
  A digit pad without a ✓ would be a kind of its own. Open.
- **The brief's example number for "about" was 2; it is 4.** The list
  is abandon 1, ability 2, able 3, about 4. The script types 4.
- **Rule 2 says the checksum filters the last word and that a failure
  is named**, which only coexist through the fallback the typed words
  already have: when no candidate is allowed, the word is offered
  anyway so the checksum screen can name it. That is what is built.
- **Rule 3 said nothing about Back.** Decided as above.
- **The number field is not masked**, as the letters of a word are
  not; the words-so-far panel above stays masked.

### 16.126 One mono face for everything read character by character, and a narrower one (2026-09-22)

**Why.** The owner: anything that is data — hex, fingerprints, the
seed words, the keycaps, the candidates, the word list's pages — is
better read in a monospace face that tells 0 from O and 1 from l and
I. What ships: Noto Sans for text, Noto Sans SemiBold for titles,
labels, buttons and every keycap, Noto Sans Mono for data, an icon
face and a CJK fallback. Noto Sans Mono already has the slashed zero
and distinct 1, l and I. The gap is where it is not used: a
fingerprint set as a title or a row label (the key page's title, the
Keys row, "Which key?", the wallet's Keys step, a wallet row's
"73c5da0a · SegWit"), every keycap including the hex, path and digit
pads, the candidate strip and the entry field on a wordlist, the word
list tool's rows, the number and count fields on the pads. §16.121
kept the candidates proportional because a mono face fits fewer
letters per cell; the owner's answer is to measure and, if the cell
is too tight, take a narrower mono.

**The measurement.** Noto Sans Mono advances 0.6 em; Noto Sans
Regular sets "abstract" at 3.83 em, the mono at 4.80. The longest
Latin words: English, Czech and Portuguese 8 letters, Italian 9
("avvolgere", 151 words over 8). A candidate cell on `small` is
(268 − 2·`PAD` − 2·`CANDIDATE_GAP`) / 3 = 76 dp, 64 dp inside its
padding: at `CHIP_TIGHT` (13) eight mono letters are 62.4 dp, which
fits by 2 dp, and nine are 70.2, which does not fit even at `CAPTION`
(64.8). On the phone, 411 dp wide, a cell is 92 dp, 80 inside: eight
letters at `BODY` (16) are 76.8 and fit, nine are 86.4 and drop a
size. A 0.5 em mono sets nine letters at 58.5 dp on `small` and 72 on
the phone: every list fits at the size it has today, nothing drops.

**The rules.**

1. **Anything read character by character is set in the mono face.**
   Fingerprints wherever they appear, including a title and a row
   label and a wallet's shape string; hex, paths, addresses, xpubs and
   every comparison string, as today; the seed words and the words
   panel, as today; the candidate strip and the entry field on every
   wordlist; the word list tool's rows; every keycap on every keyboard
   and pad — letters, digits, hex, path, passphrase — and the number,
   count and dots fields on a pad. Prose, labels, buttons and titles
   that are words stay in the text faces. Kana, jamo, bopomofo and
   hanzi keycaps and candidates are in the CJK face, as they are.
2. **The mono face is Iosevka**, the default Regular cut, under the
   OFL: 0.5 em advance, slashed zero, 1, l and I distinct. It replaces
   Noto Sans Mono as the `mono` family; the baked face keeps the same
   ASCII and Latin-1 subset and the same symbols, so it costs what the
   old face cost, about 40 KB. Its source is not vendored: the baker
   downloads the release archive and checks a pinned SHA-256, as it
   does the CJK faces, and only the `.outl` is committed. The arrows
   and triangles the baker borrowed from Noto Sans Mono come from the
   new face or stay borrowed, whichever the baker finds cleaner.
3. **Sizes do not change.** `candidate_label`, the keycap sizes, the
   mono ramp and the words panel keep their values; the pass renders
   the strips and the panel and reports what fits, and the rule for a
   word too long for its cell (§16.120, drop to the caption size for
   the whole strip) stays as the fallback nothing should reach.
4. **DESIGN §3 says the rule**: the faces, and one sentence that
   anything read character by character is set in mono; §4.3's
   Keyboard and Candidates rows and §4.4's Key row lose "proportional".

**Rejected.** Keeping Noto Sans Mono and accepting the drops: the
panel's English cell would fit its longest word by 2 dp and Italian
would not fit at all. A condensed proportional face for the
candidates: the owner's point is the mono face's distinct glyphs,
and one data face is one rule.

**In one pass, F1.** Rule 2 first, so the rest is rendered in the
face that ships: the baker's source entry and digest, `just fonts`,
the new `mono.outl`. Then rule 1 through the existing `mono` flags
and `Font::mono`, one site at a time; `candidate_font` and the
keycap fonts become mono. Rule 4. A test that every Latin list's
longest word is drawn in a candidate cell at the strip's own size on
`small` and on the phone, and one that a fingerprint used as a row
label is drawn in the mono face. Render `load-key` and `load-key-ja`
at 480x640 and 1080x2340, `tools-wordlist` at 480x640, and
`key-detail` at 1080x2340, and look.

**Pass F1 landed (2026-09-22).** Rules 1, 2 and 4. The `mono` family is
Iosevka 34.8.1, the default Regular cut, 0.5 em advance: the baker
downloads the release archive into a git-ignored `tools/fonts/iosevka/`,
checks its SHA-256 and unpacks `Iosevka-Regular.ttf` from it, as it does
the CJK subsets. The arrows and triangles stay borrowed from the
vendored Noto Sans Mono, which is no longer a face of its own; borrowing
was the smaller change. The subset is the same 201 glyphs, and
`mono.outl` went from 39,862 to 67,237 bytes: Iosevka's outlines carry
more points per glyph than Noto Sans Mono's, so the face costs 27 KB
more than the entry expected, not the same.

Rule 1 reached `candidate_font`, every keycap font including the special
keys, the dice and card count field, the wordlist entry fields, the word
page's number and index rows, the Keys row's fingerprint, the key
page's app-bar title, the word page's title and its pager's label, and
the wallet rows' shape value. The kana, jamo and 注音 keycaps and
candidates fall back to the CJK face as before. Four seams were added
for it: a `ListRow` with a mono title, a `Frame` with a mono app-bar
title (`screens::menu_mono_title`, `screens::record_mono_title`), a menu
row with a mono value (`Row::MonoMenu`), and a pager that names its page
in mono (`components::pager_mono`).

Every Latin list's widest word is drawn at the strip's own size on the
268 dp panel and on the phone; nothing drops to the caption size. The
entry's arithmetic left out the class type scale (§3: `mobile` sets type
at 110%), which costs the phone's cell about 8 dp of what it counted. At
the phone's own density the nine-letter words still fit, but by one or
two dp rather than the eight the entry expected, and at one pixel per dp
the rounding of 17.6 px up to 18 is enough to drop them. The test
renders each class at its own pixels and density for that reason.

### 16.127 Keys rows say what a key is made of, and a key's page holds what is done with it (2026-09-22)

**Why.** The owner, on the phone: the Keys row is a fingerprint in
the top-left corner of a tall row and nothing else, and looks wrong;
the wallet rows beside it carry a glyph, a kind, a network and a
shape. And "Open with passphrase" sits on Add a key while "BIP-85" sits
on the key's page, so what is done with one key and what makes a new
key are mixed on one menu. §16.104 rule 1 made Keys a flat list of
fingerprints that never says how a key was made, so that a fact
about a secret's construction was not public metadata; it also put
the two "Open" rows on Add a key with a "Which key?" picker each.
Since then the pickers and the wallet's Keys step have shown
"mainnet · passphrase" under every key, so Keys hides what its
neighbours show. The network is not the key's: setting the device's
network relabels every loaded key at once (`OpenSigner::set_network`),
so it belongs to the wallet and the device and comes off every key
row.

**The rules.**

1. **A Keys row is the fingerprint glyph, the fingerprint in mono,
   and what the key is made of.** The subtitle is the key page's own
   "made of" string — "12 words", "24 words and a passphrase",
   "BIP-85 child", "SLIP-39 shares", "Codex32" — and no network. The
   glyph is `Icon::Fingerprint`, which says the eight hex characters
   are a fingerprint; it goes in front of the fingerprint on every
   key row: Keys, the wallet's Keys step and Keys review, the
   member rows. §16.104's "never says how a key was made" is
   withdrawn: the list is behind the lock, the same words are one tap
   away on the pickers today, and a second fingerprint on the list
   already tells anyone who knows the parent's that a derived key
   exists. §16.104's rejection of nesting a key under its parent
   stands: nothing records which key a passphrase key or a child came
   from.
2. **A wallet row's glyph is the wallet or the eye.** `Icon::Wallet`
   where this device holds a key of the wallet, `Icon::Eye` where it
   holds none, in place of the key glyph, so a key glyph is never on a
   wallet row and a wallet glyph never on a key row. The member row's
   "computes this share" mark becomes the wallet glyph too. §4.4's
   two-glyph paragraph is reworded.
3. **A key's page holds what is done with that key.** "Open with
   passphrase" moves onto the key page, after BIP-85, and opens that
   key's passphrase entry with no picker. "Open BIP-85 child" leaves
   Add a key: the key page's BIP-85 row already opens the child-key
   flow as its Words application. Add a key is Load a key · Create a
   key · Create SLIP-39 shares · Create Codex32 shares. The two
   "Which key?" pickers go, with `PickFor` if nothing else uses it.
4. **DESIGN** §4.4's Key row says the glyph, the mono fingerprint and
   the made-of line and applies to Keys; §5's Keys and Add a key
   entries say the new rows; the tests that lock the old list and the
   old menu (`launcher.rs`, `open.rs`, `flows.rs`) are changed to the
   rules above, not deleted.

**Rejected.** A "show key details" setting: a setting for a row that
states nothing secret. The network on the row: not the key's.

**In one pass, K1.** Rules 1 to 4, and the Learn sentence that says
a key row states what a key is made of.

**Pass K1 landed (2026-09-22).** Rules 1 to 4. A Keys row is
`Icon::Fingerprint`, the fingerprint in mono (§16.126's row) and the
key page's own made-of string; the pickers' and the wallet's Keys
step's rows carry the same, through `key_choice_row` given an icon;
`key_subtitle` and the "passphrase" row note are gone, so no key row
names the network. Wallet rows and member rows carry `Icon::Wallet`
or the eye; the key glyph is on no wallet row. "Open with passphrase"
is a row of the key page, opening that key's entry with no picker;
Add a key is the four ways a key is made. The implementer kept one
case the entry missed: a wallet's or a transaction's not-loaded row
sends the person to Add a key for one key in particular, and that key
may be behind a passphrase or a BIP-85 index, so under that question
alone Add a key still carries the two "Open" rows and their "Which
key?" picker (§16.104 rule 6). The Learn words page says what a row
states. Seven tests changed to the rules, none deleted. 1 296.

**What the design got wrong or left unsaid.**

- **Rule 3 said the pickers go.** They go from Add a key's own menu
  and stay under the not-loaded row's question, where a key is asked
  for by fingerprint and the person may have to make it. `PickFor`
  stays for that.
- **The pass was finished by the orchestrator.** The implementer's
  worktree hung on a `git reset` after it had saved its diff; the diff
  was applied onto `main` three-way, the two conflicts with §16.126's
  pass (the key row's mono title against its glyph, and DESIGN §5's
  Menu row against §16.128's Choice row) resolved by hand, and the
  check run here.

### 16.128 A one-key wallet moves its check, and Back walks the wizard (2026-09-22)

**Why.** Two bugs the owner hit on the phone in Add a wallet. On
Single-sig, tapping a second key while one is checked does nothing:
`build_toggle_key` refuses because the kind is full, the row is not
dimmed as Silent payments' rows are, and the tap is dropped; the
person has to uncheck first. And Back from the Keys step with a key
checked opens "Discard this wallet?", whose own Back returns to the
Keys step, whose Back opens the prompt again: the only exits are
Discard or unchecking every key.

**The rules.**

1. **On a kind that takes one key, a tap on another key moves the
   check.** Single-sig and Silent payments: the checked key is
   replaced, no row is dimmed for being the second, and "one key" as a
   reason goes. A kind that is full of several keys (FROST at its
   count) dims the rows it cannot take with "full".
2. **Back walks the steps and keeps what was gathered.** Back from
   Keys goes to the kind step with the keys kept; a kind change that
   cannot use them clears them, the same kind keeps them. The discard
   question is asked once, on Back from the first step when keys have
   been gathered, and answers Discard or Keep.
3. **The prompt has no Back chevron.** The system back on it is Keep,
   so no path returns to the prompt from the prompt.

**In one pass, W1.** Rules 1 to 3, tests that the check moves, that
Back from Keys lands on the kind step with the keys kept, and that
Back from the prompt leads out through Keep.

**Pass W1 landed (2026-09-22).** Rules 1 to 3. `WalletKind::one_key`
names the two kinds a wallet is built on one key; on those,
`build_toggle_key` clears the gathered key before it adds, so a tap on
an unchecked row moves the check, and `build_key_reason` dims nothing.
A kind that is full of several keys dims the rows it cannot take with
"full" (`build_reason_group_full`), which covers FROST at its count and
a multisig wallet at fifteen keys; `build_reason_one_key` is gone.
`back()` walks the steps: from Keys to the kind step, or to Quorum for
FROST, with the keys kept, and from the kind step to the discard
question when anything has been gathered. The question has no chevron,
and its system back is Keep. DESIGN §5's Choice entry says both. Three
tests, one of them the old `back_with_keys_gathered_asks_first` rewritten
to the new rule. 1 296.

**What the design got wrong or left unsaid.**

- **"A kind change that cannot use them clears them" is not what the
  code distinguishes.** `set_kind` already clears the keys on any
  change of kind and keeps them when the same kind is chosen again, and
  it clears the recovery paths with them. Making the rule sharper would
  mean deciding, per pair of kinds, which keys survive; the minimal
  rule that keeps the wizard consistent is the one already there, so
  the entry's clause is read as "a change of kind clears them".
- **A recovery wallet's Later step had the same loop** the entry
  describes for Keys, and the entry says nothing about it. Back from a
  path with keys opened the same question, whose Keep returned to the
  path. It now behaves as the empty path always did: the first path's
  chevron returns to the primary Keys step with both lists kept, and a
  later path's chevron drops that path and returns to "Another path
  later?", which is what un-does the step that offered it. The discard
  question is now reached from the kind step alone.
- **Silent payments keeps one reason.** Rule 1 takes "one key" away,
  but a loaded key whose words this device does not hold still cannot
  be a silent payments wallet, and that row stays dimmed with "not
  usable".

### 16.129 A wallet from a key's page, with the passphrase in the flow (2026-09-22)

**Why.** The owner: there is no way from a key to a wallet, and no
way to add a passphrase inside the wallet flow, which is where most
software puts it. §16.104 rule 1 kept wallet screens off the key
page, and the wizard has one constructor and no way to start with a
key in hand. The owner wants one streamlined path: a key, a
passphrase or not, a kind, a wallet.

**The rules.**

1. **A key's page has "Add a wallet"**, the same words as the Wallets
   screen's action, after the BIP-85 and passphrase rows. It starts
   the one wizard with this key in hand.
2. **Its first step is "Passphrase?"**, a Choice with No checked and
   Yes. Yes opens the key's passphrase entry, the same screen as the
   page's own "Open with passphrase", whose fact row already shows the
   new fingerprint live; its title says a key is added. The new key
   lands on Keys as a peer, as it does today, and the wizard continues
   over it. No takes the loaded key.
3. **Then the kind, and the Keys step only where more keys can
   follow.** A kind that takes one key — Single-sig, Silent payments
   — skips the Keys step. Every other kind opens it with this key
   checked and the rest to add. From there the wizard is the wizard.
4. **Wallets › Add a wallet is unchanged**: no key is in hand there,
   so no passphrase step.
5. **Back** from the passphrase step and from the kind step returns
   to the key page; §16.128's discard question applies from the kind
   step once keys beyond the one in hand have been gathered.

**In one pass, W2**, after K1 and W1. Rules 1 to 5, DESIGN §5's Menu
entry for the key page and §5's Choice entry for "Passphrase?", tests
that a single-sig wallet is built from a key's page in three taps
past the passphrase step, that Yes adds a peer key and builds over
it, and that a multisig from a key opens its Keys step with the key
checked.

**Pass W2 landed (2026-09-22).** Rules 1 to 5. The key page's "Add a
wallet" row, with the wallet glyph the same label carries on Wallets,
starts `WalletWizard::from_key`, which opens on "Passphrase?". Yes sets
the wizard aside while the key's own passphrase entry is up, titled
"New key with passphrase"; its ✓ puts the new key on Keys as "Open with
passphrase" does and brings the wizard back on the kind step over the
new key. Continue on the kind checks the key in hand where the kind can
use it; Single-sig and Silent payments then go past the Keys step, and
Back from the step after returns to the kind. Back from the kind asks
the discard question only for keys beyond the one in hand
(`gathered_beyond_hand`). Four tests in `wallet_wizard.rs`, one in
`codex32_key.rs`. 1 303.

**What the design got wrong or left unsaid.**

- **A key without words has no passphrase to add.** The entry put
  "Passphrase?" first on every key; on a SLIP-39 or Codex32 key Yes
  could do nothing. The question is asked only where this device holds
  the key's words; elsewhere the wizard opens on the kind.
- **After Yes, the way out is the new key's page.** The entry is
  replaced by the new key's page, so Back or Discard from the kind step
  lands there and not on the parent's.
- **The "Opened" result is skipped** inside the wizard: the entry's
  fact row has already shown the new fingerprint.
- **A key in hand the kind cannot use** (a 12-word key for FROST, a key
  without words for Silent payments) is not checked, and the Keys step
  opens with its row dimmed and the reason, even for a one-key kind.
- **A change of kind clears the key in hand** with the rest (§16.128);
  it is checked again on the next Continue, and anything else gathered
  is gone.
- **§16.104's "no wallet screens on the key page"** is replaced: DESIGN
  §5 now says the key page carries no row of a wallet already in use.

### 16.130 The panel sets its text in Noto Sans SemiCondensed, and a row's label sits one gap after its icon (2026-09-22)

**Why.** On the 240×320 panel the menu rows cut their labels off:
"Create SLIP-39 shar", "Create Codex32 shar", "Open with passphra".
A menu row on the 268 dp panel leaves its label 144 dp between the
icon and the chevron, the label started a full padding step (16) after
the icon, and Noto Sans SemiBold at 16 dp is wider than that for a
label of twenty characters. The row already drops the label to 14 dp
when it does not fit; that was not enough.

**Measured.** Widths of the 1 245 strings in `strings/en.rs` and of
the clipped labels, against Noto Sans: Noto Sans SemiCondensed 91 %,
Roboto 93–95 %, Source Sans 3 88–89 % but 98 % at the same x-height,
IBM Plex Sans Condensed 90–91 %, Inter 101–102 %, Noto Sans Condensed
82–83 %. The clipped labels needed about 10 %.

**The rules.**

1. **A row's label starts one gap (8) after its icon**, not a padding
   step, on every class.
2. **`small` sets its text in Noto Sans SemiCondensed**, Regular and
   SemiBold, through `tokens::narrow_text`, which `Scale::for_class`
   records and `Font::sized` reads. The mono face and the icons are
   the same on every class; `mobile` and `wide` keep Noto Sans. The
   SemiCondensed cuts are the same release with the same vertical
   metrics, so no line box and no row height changes.

**Rejected.** Noto Sans Condensed: every label fits at 16 dp, but at
143 dpi its letters close up ("12 words", "Create a key"). Roboto: 5 to
7 % narrower, and a different face beside Noto Sans CJK. Source Sans 3:
narrow only because its lowercase is smaller.

**Landed (2026-09-22), by the orchestrator.** Rules 1 and 2. Two faces
baked from `NotoSans-SemiCondensed.ttf` and
`NotoSans-SemiCondensedSemiBold.ttf`, 76 KB together; the other faces
bake to the same bytes. `fonts::TEXT_FAMILIES` names every face text is
drawn in, and the glyph tests walk it, so the narrow cuts are held to
every wordlist and keycap as the others are. DESIGN §3 names the cut
and §4's Menu row the gap; the fonts README lists the files.

**What the design got wrong or left unsaid.**

- **Three labels still drop to 14 dp on the panel** in SemiCondensed:
  "Create SLIP-39 shares", "Create Codex32 shares", "Open with
  passphrase". They are whole, but a menu can carry two label sizes.
  Wrapping a label that does not fit at 14 dp onto a second line, which
  a 52 dp row has the height for, is the fallback for longer labels and
  for translations; it is not built.
- **No test says a label is drawn whole.** The app reports placed
  rectangles that overflow (`overflow()`), not text cut by its row's
  clip; a check for that needs the canvas to record it.

### 16.131 The fingerprint glyph stands before every fingerprint shown as a value (2026-09-22)

**Why.** The owner, on §16.127: the fingerprint glyph "would not be a
bad idea … we could use it in other places too next to the
fingerprint hex". §16.127 put it on key rows, key choosers, a wallet's
Keys row and the member rows, where it is the row's bullet. Everywhere
else a fingerprint is a value — "Key" over `73c5da0a` on the Sign
review and in Explore, the key a result names, a record table's key
row, the Sign with chips, the live fingerprint on the passphrase entry
— and there nothing says the eight characters are a fingerprint.
§3's "a row that states a value carries no glyph" is about a row's
bullet; this glyph is part of the value.

**The rules.**

1. **A fingerprint shown as a value is the fingerprint glyph and the
   eight hex characters.** The glyph stands immediately before the
   characters, inside the value: at the value's line height, one small
   gap (`GAP_SMALL`) before the first character, in the value's colour.
   It applies wherever `fingerprint_hex` produces the value of a row, a
   fact row, a record table cell, the key context row or a chip, and
   to each fingerprint of a value that lists several.
2. **Not twice, and not inside a longer string.** A row whose bullet is
   already the fingerprint glyph (key row, key chooser, member row)
   does not repeat it in its value. A fingerprint inside a longer
   string — an origin `[73c5da0a/84h/0h/0h]`, a descriptor, an account
   key — carries no glyph; the string is read character by character.
   An app bar title that is a fingerprint (the key page) carries none:
   the title is the page's name, and the panel's title has no room.
3. **One mechanism.** `osk-ui` gives a value a leading glyph (a row's
   value below or beside, a labelled value, a record cell, a chip); the
   views ask for it where the value is a fingerprint. Measurement
   includes the glyph, so a value that fitted still fits or steps down
   the way it does today.
4. **DESIGN** §4.4's Fingerprint entry says it; §4.4's Fingerprint glyph
   entry says "at the start of a row, or before the characters of a
   fingerprint shown as a value".

**Test.** On the Sign review, the key context row's value is drawn
with the glyph before the fingerprint (a user-facing check through the
app's drawn content, not a geometry comparison).

**Pass C1 landed (2026-09-22).** Rules 1 to 4. `Widget::with_value_glyph`
puts a glyph before a text, a chip, a labelled value or a row's value,
measured with it; a value of several parts joined by " · " takes one
before each. `components::fingerprint_glyph` gives the glyph only where
every part is eight hex characters, so "Typed words" and "Member 2 of
3" take none. The key context, Sign with and Then with chips, the
cosigner chips, the result and record rows of keep, backup, XOR,
finish, detail, message, verify, sign and explore carry it; Lightning's
key row, which was in the body face, is mono now. Test in `sign.rs`.

**What the design got wrong or left unsaid.**

- **Rule 2 named the member row's bullet as the fingerprint glyph**; it
  is the wallet or the eye (§4.4). Read as "a row whose bullet marks
  the key", member rows and a wallet's Keys review take no second glyph.
- **"At the value's line height"** is drawn at the value's em, so the
  glyph stands as tall as the characters, not the line box.
- **A value mixing fingerprints and stand-ins** (a threshold key context
  listing shares and "Member n of m") takes no glyph.
- **A record cell's characters start one glyph to the right** of the
  column's other values.
- **A fingerprint as a QR caption** (the group record, the encrypted
  backup's code) is not in rule 1's list and has none.

### 16.132 Word numbers' pad has no ✓, and what is pressed on black shows it (2026-09-22)

**Why.** Three things left open by §16.124 and §16.125. Word numbers
types each word's number on the digit pad and takes the word from the
strip, so the pad's ✓ never does anything, but it is drawn, dead. And
the pressed look of anything drawn on the black background — a Text
button, the app bar's arrows and ⓘ — is the black mixed towards the
accent, a dark brown that barely shows; the app bar's eye, drawn as a
ring while a reveal runs, shows no pressed look at all.

**The rules.**

1. **A digit pad that has nothing to confirm has no ✓.** Word numbers'
   pad is the PIN pad's grid with the ✓ cell left empty — no key drawn
   and no target — so the digits and ⌫ stay where the PIN pad has
   them. The PIN, the Days, the Index and every other pad with a ✓
   keep it.
2. **Pressed on black is the raised surface, tinted.** A control drawn
   on the background — Text and muted Text buttons, the app bar's icon
   buttons — fills, while pressed, with the raised surface (`#3A3A3C`)
   mixed `PRESSED_TINT` towards the accent, with the same two-step
   linger (§16.124). Rows and secondary buttons, which sit on a surface,
   are unchanged.
3. **The eye's ring is pressed like the other app bar buttons.**

**Tests.** Word numbers' pad offers no ✓ to tap; the PIN pad still
does.

**Pass C1 landed (2026-09-22).** Rules 1 to 3. Word numbers' pad sets a
new mask bit, `DONE_ABSENT`, which drops the ✓ after the row is laid
out, so the digits and ⌫ keep their columns and the empty cell has no
target; physical Enter does nothing there. `pressed_on_background`
gives Text buttons, icon buttons and the eye's ring the raised surface
tinted. Test in `load_numbers.rs`.

**Left unsaid.** On a wide pad the empty cell stays at the bottom
right; the pad is not re-centred.

### 16.133 A row label that does not fit takes a second line (2026-09-22)

**Why.** §16.130 made the panel's labels fit by a narrower cut and a
smaller gap, and recorded the rest: a label wider than its row at the
label size is cut off at the row's edge with no mark, and nothing
tests for it. Longer labels will come, and a translation runs about a
third longer than English.

**The rules.**

1. **A row's title steps down, then wraps.** A title that does not fit
   at its size is drawn at the label size (today's step); one that
   still does not fit wraps at a space onto a second line at the label
   size. The row grows to the two lines where its height does not hold
   them; a 52 dp row holds two label lines. A title with a subtitle
   under it wraps the same way, and the row grows.
2. **A word longer than the row** is still cut, at the row's edge; no
   row is drawn over another.
3. **The canvas records text it cuts.** Drawing text that runs past its
   clip is recorded as the `overflow()` rectangles are, so a test can
   ask whether a screen drew every label whole.

**Test.** Every menu row label of the key page, Add a key, Wallets,
Settings and Tools is drawn whole at 240×320, and a long label on a
row wraps to two lines rather than being cut.

**Pass C1 landed (2026-09-22).** Rules 1 to 3. `Widget::row_text`
computes a row's text column once for measure and draw; a title steps
to the label size, then breaks at the last space that lets the first
line fit, and the row's height takes both lines. The canvas records a
string whose ink runs past the left or right of its clip
(`Canvas::cut_texts`), and `clipped_texts()` returns them. Tests:
`every_menu_label_is_drawn_whole_on_the_smallest_panel` (key page, Add
a key, Wallets, Add a wallet, Settings, Tools at 240×320; it failed on
"Create Codex32 shares" before the wrap) and a wrap test in `osk-ui`.
1 307 with W2.

**What the design got wrong or left unsaid.**

- **The one-line row** (value beside its label) does not wrap; the
  entry did not say.
- **Text cut at the top or bottom of a clip is not recorded**: that is
  how a scrolled list looks.

### 16.134 An encrypted backup says who opens it, what it is sealed with, and where it is read back; a public code saves as a PNG (2026-09-22)

**Why.** The owner's four points on the encrypted exports of §16.112,
held until now. "Which form?" lists QR · OSKB file · KDBX 4 · Plain
text file: "OSKB" is a name only this project uses, nothing says which
program opens each form, and QR is a way to show a file, not a form
(the result already has "Show as QR"). The results after sealing say
Memory and nothing else about the encryption, so an heir holding the
file does not know what to look for. Reading a backup back is a row of
Load a key ("Read an encrypted backup"); Tools › Notes calls the same
thing "Read a file", and a wallet's Recovery sheet, which exports one,
has no way to read one. And a public code — an account key, a wallet
export, a signed transaction — can be shown but not saved as a picture
to print or send.

**The rules.**

1. **"Which form?" is Encrypted backup · KDBX 4 · Plain text**, each
   with the programs that open it as its value: "OpenSigner ·
   decrypt.py", "a KeePass app", "any text editor · not encrypted".
   "OSKB file" is renamed "Encrypted backup"; the QR row goes, since
   the result offers "Show as QR" where the file fits one code. Plain
   text stays off a key's backup (§16.112). Encrypted backup stays
   checked. The file names do not change.
2. **Every result after sealing states the encryption.** The key
   backup's result and the note's and sheet's "Sealed" result carry,
   after Memory: "Format" "OSKB 3" (or "KDBX 4"), "Cipher"
   "XChaCha20-Poly1305" (KDBX: "ChaCha20"), "Key derivation" "Argon2id
   · 3 passes · 1 lane" from the parameters actually used, and "Read
   with" as rule 1's value. Values only; the Learn page on backups says
   where BACKUP.md and decrypt.py are.
3. **"Read an encrypted backup" is one label in three places.** Load a
   key's source row (unchanged), Tools › Notes (in place of "Read a
   file", which opens the same scanner; a plain note still reads), and
   a wallet's Recovery sheet, after Export. All three open the scanner
   with its "Read a file" tile, and route as today.
4. **A public code has "Save as PNG".** Every QR screen whose content
   is not a secret — the account key and wallet exports, the signed
   transaction, the message signature, the threshold group record, an
   address, a silent payment address, and the encrypted backup's and
   sealed file's code (ciphertext) — has a "Save as PNG" row. It saves
   the one static code, black modules on white with the four-module
   quiet zone, at a whole number of pixels per module; where the
   content is too dense for one code the row is dimmed with the reason
   the Animated toggle already uses. SeedQR, CompactSeedQR and the
   grid, which are secrets, have none.
5. **The PNG is written in `osk-codec`, `no_std`, with no new crate**:
   one-bit greyscale, deflate in stored blocks, the CRC-32 the fountain
   code already has and an Adler-32. The shell API gains
   `FileKind::Png`, which Android saves as `image/png` and the desktop
   shell as a `.png` name.
6. **DESIGN** §5's Choice entry for "Which form?", the Result entries
   for the backup and Sealed results, the Menu entries for Notes and
   the Recovery sheet, and the QR entry's action rows.

**Tests.** Which form? lists the three forms with their values; the
sealed results state the cipher and key derivation; "Read an encrypted
backup" from Notes and from a Recovery sheet reads a sealed sheet back
to "Add this wallet"; a saved PNG, decoded (the `decode` feature's QR
reader in a test), gives back the code's payload; the SeedQR screen
offers no PNG.

**In one pass, B1.**

**Pass B1 landed (2026-09-22).** Rules 1 to 6. `Form::ALL` is Encrypted
backup · KDBX 4 · Plain text, each row's second line the programs that
open it; nothing jumps to a QR after sealing, and the Result shows "Show
as QR" where the file fits one code. The key backup's and the Sealed
results add Format, Cipher, Key derivation and Read with, read from the
header of the file just made. Notes and a Recovery sheet carry "Read an
encrypted backup" (`SHEET_READ`), which reads a sealed sheet on to "Add
this wallet". `osk_codec::png::qr_png` writes the one-bit PNG, eight
pixels to a module; `FileKind::Png` reaches Android as `image/png`.
Every public code screen has "Save as PNG", dimmed with "too dense"
past one byte-mode code at level L (2 953 bytes); the saved state shows
under the label for a few seconds, as "Copied" does. The Learn page on
encrypted backups says where BACKUP.md and decrypt.py are. Tests in
`backup_file.rs`, `account_key.rs` (the PNG decoded back to the account
key) and `scan.rs` (no PNG on a SeedQR); two in `tools.rs` changed to
the rule. The orchestrator added three fixes found in the renders,
below. 1 314.

**What the design got wrong or left unsaid.**

- **The panel has no room for the PNG row beside the Animated toggle.**
  With the code, the toggle and the progress row, the row pushed the
  toggle over the code's quiet zone on 240×320. On `small` a code screen
  with the toggle carries no PNG row; the encrypted backup's and sealed
  file's codes, which have no toggle, keep it. The Address screen on
  `small` has none either (§4.5 puts the whole address first).
- **"XChaCha20-Poly1305" broke mid-word** on the panel's value column.
  The wrapper now breaks a word too long for the line after its last
  hyphen that fits, and between characters only where none does.
- **Signing by hold had stopped completing in the sign script.** A tick
  that ended a press look's linger step returned before it looked at
  the hold, so a hold whose completing tick was also the tick a linger
  ended was lost and the lift cancelled it. The tick now reports a
  completed hold first. On a device ticking at 60 fps the two rarely
  coincide; the script's two long ticks made them coincide every time.
- **Rule 6's DESIGN entries did not exist as named**: there were no
  Result entries for the backup and Sealed results (added), and the
  Recovery sheet is a Document, not a Menu. Its new row is the last
  row above the note, since Export is the footer button.
- **The plain-text Result** carried Memory and a Show as QR of the
  plaintext labelled as the encrypted form; it now shows Bytes alone.
- **Key derivation states passes and lanes, not memory**, which stays on
  the Memory row above it.

### 16.135 An audit of every script's frames, and the faults it found (2026-09-23)

**Why.** The owner, on the phone: on the "New wallet" review from a
key's page the wallet glyph and "Key" run together, and the
descriptor is not in mono; "I can't check every single screen and I'm
wondering if there are more screens with these types of
inconsistencies." The tests check behaviour and the snapshots are
looked at a few at a time, so a fault on a screen nobody opened by
hand stays.

**What was built.** The canvas can record where each string and icon
leaves its ink and in which face (`Canvas::record_ink`, off on a
device). `osk_ui::audit::check` reads that record for three faults: two
inks that cross or sit closer than 3 dp on one line, a fingerprint,
checksum, address, extended key, path or descriptor drawn in a text
face, and (from §16.133) text cut at the side of its clip.
`OpenSigner::audit` paints the frame whole and asks. The snapshot
tool's `--audit` asks after every script line, prints each fault once,
and saves the frame it was found on as `audit-<script>-line-<n>.png`.
`just audit` runs every script at the four reference sizes, and lists
the kinds of screen no script reaches. It is not part of `just`.

**What it found, and the fixes.** 238 reports, about twenty faults:

- **A flat row's glyph touched its label** (the review's Key row): a
  flat row took its icon gap from its padding, which is none. The gap
  is `GAP` on every row.
- **Data in a text face.** The descriptor's summary on the review and
  the export menu (now `Record::mono_value` and `Row::MonoValue`,
  shown whole in mono rather than elided); a wallet's page title where
  the wallet is called by its key (`wallet_called_by_key`); key rows in
  every chooser, dimmed ones and a scanned cosigner's path included
  (`Item` flags, `with_mono_subtitle`); a wallet's Keys review; an input
  row's value; a label or caption that names a key (`names_a_key`: a
  part between " · " is eight hex characters), in a row, a table's
  label column, a code's caption and a comparison's label.
- **Cut.** A table label now wraps at a space; the cosigner chips go one
  to a line on `small` and two elsewhere; a row's text clip leaves the
  small gap on its left, where the hook of a J or j reaches.
- **Touching.** The SeedQR grid's column numbers ran together on the
  panel ("101112"); a label must leave `QR_GRID_LABEL_GAP` in its cell
  before the larger size is kept.
- **A script** (`load-key-zh-hans`) tapped twice for a reading with one
  character on the panel, where one tap takes it.

After the fixes `just audit` reports nothing on any script at any size.

**What it does not see.** Anything that is not ink in the wrong place:
a screen laid out unlike its neighbours, a wrong word, a wrong colour.
And the self-test refusal, which only the core's own tests can raise.

**Coverage, the same day.** Twenty-six kinds of screen were reached by
no script. Six scripts now reach all but the self-test refusal:
`key-more` (Created, Which key?, BIP-85, Vanity, Lightning, a wallet's
name), `keep` (Keep, the duress PIN, the stored key and its removal),
`notes` (Notes, a note, sealing, its code, a recovery sheet), `silent`
(the silent payments screens), `compare-tx` and `boot` (the secure-boot
refusals). The snapshot tool gained `restart tier= secure= boot=`,
which opens a new app on a device with that tier, secure hardware and
boot, the element an in-memory HMAC key and one blob that outlives the
app. The audit then found, and these are fixed:

- **A fingerprint inside a sentence** in the text face: "This
  passphrase gives 19e4837c, not 32534671". A single line of text now
  sets any fingerprint in it in the mono face on the line's baseline,
  and the caption reads "Opens 19e4837c, not 32534671", which fits the
  panel's one line.
- **A 12-character BIP-85 password ran off the right edge**: a string
  short enough not to be chunked was centred as if four characters
  wide. It is centred by the width the plan measured.
- **A second title line too wide for its row** now ends in an ellipsis
  at a word, rather than cut at the edge.
- **A note was named on Notes as if it were data**, elided in groups of
  four ("Keys in … not ary."): it is named by its first line, cut after
  a whole word with an ellipsis.

### 16.136 Candidates are sized for eight letters, and a longer word fits its own cell (2026-09-23)

**Why.** The owner: "we can still bump candidates a point size … there's
a decent amount of padding on the candidates", then "why don't we just
size them for 8 letters and allow >8 letters to resize?" The strip took
one size for both rows, chosen so that Italian's nine-letter words fit
(13 dp on `small`, 16 on `mobile`), and dropped the whole strip to the
caption size whenever a word on it did not fit (§16.121). Every Latin
list but Italian tops out at eight letters, so most words sat small in
wide cells.

**The rule.** The strip's size is the largest, up to the class's own
(`BODY`, 16 dp, on `small`; `CANDIDATE_LABEL_MOBILE`, 18 dp, on
`mobile`), at which `CANDIDATE_FIT_CHARS` (eight) mono characters fit a
cell inside its padding, measured on the display so that a density's
rounding cannot clip an eight-letter word. A word wider than that takes
the size at which it fits, stepping by `CANDIDATE_SIZE_STEP`, in its own
cell only; the cells beside it keep the strip's size. §16.121's "both
rows measured together" is withdrawn, and with it the strip's
`other_row`.

**Landed, by the orchestrator.** The words go from 13 to 16 dp on the
panel and from 16 to 18 dp on the phone, where "abstract" now fills its
cell. The test that the longest word of every Latin list is drawn at
the strip's own size becomes the test that it is drawn whole in its
cell, at 240×320, 480×640 and the phone.

### 16.137 Word entry on a 268 dp panel has no eye (2026-09-23)

**Why.** The owner: the eye on the word entry screen on the small panel
"seems like it does nothing". It did nothing. §4.3 leaves the words
panel off word entry on `small` — even one column of it is taller than
the room above the keyboard — but `words_panel_fits` answered yes on
`small` so that the view would offer the eye, on the reading that the
eye itself would show the words. Nothing was built to show them, so a
tap started the 30 s ring over a panel that was not drawn.

**The rule.** Where word entry draws no words panel, the app bar has no
eye, on `small` as on any class whose list does not fit. The words
accepted are read whole on the Words screen once the last is in, as a
pad's run is on the screen it ends on (`pad_words_panel_fits` already
answered no on `small`). The typing test that the eye goes with the
panel covers both panel sizes; `load-key` and `load-key-ko` tap the eye
on `mobile` and `wide` only.

**Rejected.** The eye showing the words on `small` by taking the
keyboard's place while revealed: a new layout for the screen, for what
the Words screen at the end already shows.


### 16.138 The file formats and the kept-key blob are crates of their own, and `rqrr` is a path dependency (2026-10-04)

**Why.** The owner: someone forking OpenSigner should find the logic in
reusable crates rather than in the app. Most of it already was, in
`core/`; what was not sat in `opensigner-core` beside the screens, and
`opensigner-core` cannot be built without `osk-ui`.

**The rule.** Code that defines bytes, or answers a question with no
screen in it, lives in a `core/` crate; `opensigner-core` holds the
flows, their state and their wording.

| What | Was | Is |
|---|---|---|
| `osk-backup` (`.oskb`): seal, open, the four kinds | `opensigner-core/src/encrypted.rs` | `core/osk-backup/src/oskb.rs` |
| The KDBX 4 writer | `opensigner-core/src/kdbx.rs` | `core/osk-backup/src/kdbx.rs` |
| Argon2id `Cost`, `DEVICE_PARAMS`, `MIN_PARAMS` | `opensigner-core/src/keep.rs` | `osk_backup` |
| The kept-key blob: layout, records, PIN, Argon2id and element keys | `opensigner-core/src/keep.rs` | `core/osk-keep/src/lib.rs` |
| The blob's wallets and notes records, wallet names | `keep_wallets.rs`, `keep_notes.rs`, `names.rs` | `osk_keep::{wallets, notes, names}` |
| The signed-message text form | `opensigner-core/src/message.rs` | `osk_psbt::message::{signed_text, parse_signed, SignedText}` |
| Address prefixes and parsing | `opensigner-core/src/verify.rs` | `osk_bip::address` |
| A wallet's address search | `OpenSigner::check_address` | `WalletPolicy::find_address` |
| A transaction and its previous outputs, read for a check | `opensigner-core/src/silent.rs` (and a second raw reader in `tools.rs`) | `osk_psbt::transaction` |

What stays in the app is what a person types or what the app does next:
the backup passphrase entry (`pass_entry.rs`, with `MIN_PASSPHRASE`),
the kept-key exchange and its PIN pads (`keep.rs`: `Flow`, `Step`,
`Landing`), a note's typing buffer, the address keyboard's mask, and
the silent-payment Check's answer. `osk-keep` depends on `osk-backup`
for `Cost` only, so a fork that stores keys another way takes
`osk-backup` alone. The `.oskb` and KDBX vector tests, with the Python
second readers, run in `core/osk-backup/tests/vectors.rs`; the app's
tests keep what a person sees.

`rqrr` is the workspace dependency `{ path = "third_party/rqrr" }`
rather than a `[patch.crates-io]`, in the root manifest and in the fuzz
workspace's. Cargo applies a patch only in the workspace being built, so
a project depending on `osk-codec` by git got the crates.io release and
its `Perspective::map` assertion (M5a); a path is followed from any
workspace. `cargo fmt --all` formats path dependencies, so
`third_party/rqrr/rustfmt.toml` turns formatting off for the copy.

**Rejected.** The kept-key blob inside `osk-backup`: one is a file
anyone with the passphrase opens anywhere and the other is one device's
storage under its PIN and its element, and a fork may want the first
without the second. Publishing the vendored `rqrr` to crates.io under
its own name: needed only once these crates are published there, and an
outside service. A note telling forks to copy the `[patch]`:
the failure is silent.

**Not moved yet.** The Tools calculators (`tools.rs`: hashes, the
descriptor checksum facts, the miniscript compiler's facts, key facts),
the start-up self-test's vector set (`selftest.rs`), and `inspect.rs`'s
reading of key origins through `osk_ui::descriptor::tokens`. All three
moved in §16.139.

### 16.139 The dice roll the last word; the entropy lists, the Learn pages, the self-test and the calculators are core crates' (2026-10-07)

**Why.** Two reports from people building on these crates. Someone
using `osk-entropy`: direct selection allowed no rolls for the last
word, so its free bits were the person's choice rather than the
dice's. The fork Faraday: it read `SOURCE_ROWS`, `MIX_SOURCES`,
`COUNTS` and the Learn text out of `opensigner-core`, and wanted the
self-test's vectors (BIP-39, BIP-32, signing, KDFs) in `core/` so it
can run them before it accepts a key. §16.138's list of what had not
moved yet is the rest of this entry.

**The last word is rolled.** Direct selection now takes six rolls for
every word, the last included: 72 rolls at twelve words, 144 at
twenty-four. `DiceRolls::entropy_under` writes the named words' 11-bit
indices one after another and cuts them to the strength's bits, so
the last word's high `11 − checksum` bits come from the dice (7 at
twelve words, 3 at twenty-four) and BIP-39 writes the checksum into
the rest. The key's last word is therefore the rolled word with its
low bits replaced, which is how SeedSigner's `calculate_checksum`
completes a final word the person picked. The "Which last word?"
Choice, its ids (`CREATE_LAST_WORD_BASE`, `CREATE_LAST_WORD_CONTINUE`)
and its strings are gone, and the procedure's row says "72 rolls" like
the others. The pad's bit count stops at the strength, since the
last group's low bits carry nothing. Vectors are in
`tools/reference/dice/README.md`: embit's BIP-39, which is what
`calculate_checksum` calls, over EntropyLab's lookup.

A key made under EntropyLab's or BitBox's own procedure, where the
person picks candidate `k` of the checksum's list, is the key whose
last group's high bits are `k`; `111111` gives the first candidate.

**Rejected.** Rolling only the free bits, three rolls of 1–4 and a coin
at twelve words and a different short sequence at each length, as
EntropyLab's D++ finishes its last word: fewer rolls, but a sixth kind
of group a person must learn, and a last word that is not looked up in
the same table. Keeping the Choice with the rolled word checked: it
offers back the choice the report was about.

**What moved.**

| What | Was | Is |
|---|---|---|
| `Source`, `SOURCE_ROWS`, `MIX_SOURCES` | `opensigner-core/src/create.rs` | `osk_entropy` |
| The BIP-39 word counts in the order they are listed | `load::COUNTS` | `osk_entropy::WORD_COUNTS` |
| The Learn pages: `LearnPage`, `LearnSection`, the 27 pages, `learn_pages()` | `strings/en.rs` | `core/osk-learn`: `Page`, `Section`, `Learn`, `EN`, `Learn::pages()`, `PAGES` |
| The start-up self-test | `opensigner-core/src/selftest.rs` | `core/osk-selftest` |
| Argon2id, three copies | `oskb.rs`, `kdbx.rs`, `osk-keep` | `osk_backup::argon2id` |
| `hashes`, `Hashes` | `tools.rs` | `osk_bip::hashes` |
| `checksum_facts`, `ChecksumFacts` | `tools.rs` | `osk_bip::descriptor` |
| A descriptor's script type and its origins' fingerprints | `inspect.rs`, through `osk_ui::descriptor::tokens` | `osk_bip::descriptor::{script_type, origin_fingerprints}` |
| `compile`, `PolicyFacts`, `PolicyScript` | `tools.rs` | `osk_bip::compile` |
| `key_facts`, `KeyFacts`, `KeyReading`, an xpub's chain | `tools.rs`, `inspect.rs` | `osk_bip::slip132::{key_facts, network_kind}` |
| `read_input`, `ReadAs` | `tools.rs` | `osk_codec::encodings` |
| `raw_transaction` | `tools.rs` | `osk_psbt::transaction::wrap_raw` |
| `hrp` | `tools.rs` | `osk_bip::address::hrp`, made public |

`Source` and the lists are re-exported nowhere: the app names them by
their `osk_entropy` paths. `COUNTS` is `WORD_COUNTS` because the app
already has a `dice::WORD_COUNTS` for the diceware passphrase and
`osk_entropy::COUNTS` would not say what it counts.

What stays in the app is what is typed and what a screen does with it:
the calculator's field (`tools::Calculator`), `Tool`, Compare
transactions' state, the Units tool (its `Denomination` is `osk-ui`'s),
the Learn "Try it" rows and the map from a screen to its page
(`learn_map.rs`), and the self-test's failure screen.

**The Learn pages.** `osk-learn` is `no_std` with no dependencies: a
page is a title and its sections, and `Learn::pages()` lists them in
reading order. `Strings` holds `learn: &'static osk_learn::Learn`, and
the app reads `s.learn.words` where it read `s.learn_words`.
`tools/learn/sync.py` reads `core/osk-learn/src/en.rs` and `lib.rs`;
`docs/learn/` is still where the text is edited. `CLAUDE.md`'s rule
that every user-facing string is in `strings/en.rs` names the two
exceptions, the Learn pages and the self-test's check names.

**The self-test.** `osk-selftest` runs the six checks it had and two
more, scrypt (RFC 7914's first vector, N = 16) and Argon2id (256 KiB,
two passes, one lane; the answer argon2-cffi and OpenSSL agree on,
recorded in `tools/vectors/kdf/README.md`). PBKDF2-HMAC-SHA512 was
already checked by the BIP-39 seed vectors and HMAC-SHA512 by BIP-32.
The Argon2id check calls `osk_backup::argon2id`, which is now the one
function the `.oskb` file, the KDBX database and the kept-key blob all
stretch through; before this there were three copies of it, and the
kept-key one allocated its memory without `try_reserve`. A cost the
device cannot spare the memory for is now `None` from
`osk_keep::challenge`, which already meant "these parameters are not
usable", rather than an abort.

**Smaller changes.**

- The Encodings tool offers a hex string under `bcrt` on regtest, where
  it offered `tb`: `osk_bip::address::hrp` is the one the address
  checks already used.
- The tests of the moved functions against published vectors moved with
  them: `core/osk-bip/tests/calculators.rs`,
  `core/osk-codec/tests/encodings.rs`,
  `core/osk-psbt/tests/transaction.rs`. The app's tests keep what a
  person sees.
- The `psbt_parse` fuzz target compiles again: it built
  `osk_psbt::Context` without the three fields added since. All ten
  targets build, and `psbt_parse` ran 60 seconds without a crash.

### 16.140 Kernel hardening from the Faraday fork's review (2026-10-07)

**Why.** The Faraday fork reviewed its x86 stick kernel, which starts from
this tree's `boards/x86_64-uefi` at `c418768`, against the Kernel Self
Protection Project's recommendations and sent the findings here. Checked
against this tree's own built configs, its x86 findings were right. It
was wrong that `init_on_free` was already on here (that is Faraday's
command line, not ours), it treated the Pi as the same kernel when the Pi
is the Raspberry Pi fork's 6.1 on 32-bit ARM, and its Landlock section
described Faraday's processes rather than this image's.

**Both boards** (`common/linux.fragment`, enforced by
`common/kernel.required` and `common/kernel.forbidden`):

- Zeroing on allocation and on free, built in as the default:
  `INIT_ON_ALLOC_DEFAULT_ON`, `INIT_ON_FREE_DEFAULT_ON`. The app erases
  its own secrets; this covers the copies the kernel makes on the way.
- Heap and copy hardening: `HARDENED_USERCOPY`, `FORTIFY_SOURCE`,
  `SLAB_FREELIST_HARDENED`, `SLAB_FREELIST_RANDOM`,
  `SHUFFLE_PAGE_ALLOCATOR`, and `SLAB_MERGE_DEFAULT` off.
- Stopping on corruption: `BUG_ON_DATA_CORRUPTION`,
  `SCHED_STACK_END_CHECK`, `PANIC_ON_OOPS`. `PANIC_TIMEOUT` is 0, so a
  panic halts.
- `ZERO_CALL_USED_REGS` and `SECURITY_DMESG_RESTRICT`.
- Yama, and Lockdown forced to confidentiality from early boot.
  `CONFIG_LSM` is named as `"lockdown,yama"`: the Pi's defconfig sets an
  empty list, which builds both modules and starts neither.
- Off: `IO_URING`, `AIO`, `SYSVIPC`, `KEYS`, `CROSS_MEMORY_ATTACH`,
  `KCMP`, `BINFMT_MISC`, `CRASH_DUMP`, `GPIO_CDEV` and `COREDUMP`. The
  shell's only dependency that talks to the kernel is `libc`, and the
  Pi's touch controller is the kernel's Goodix driver. On the Pi, three
  defconfig options selected two of these back on, and the build's check
  stopped it: `GPIO_SYSFS` selects `GPIO_CDEV`, and `FS_ENCRYPTION` and
  `INTEGRITY` select `KEYS`. All three are off; no filesystem here is
  encrypted and nothing measures files. `COREDUMP` was a
  Faraday change the review did not propose here: the shell aborts on a
  panic, and the only writable filesystem is the exchange partition.

**The x86 board only:** `RANDOM_KMALLOC_CACHES`, `LIST_HARDENED` and
`RANDOMIZE_KSTACK_OFFSET_DEFAULT`, which the Pi's kernel does not have;
`LEGACY_VSYSCALL_NONE`; and off, `IA32_EMULATION`, `MODIFY_LDT_SYSCALL`,
`X86_16BIT`, `X86_IOPL_IOPERM`, `DEVPORT`, `LEGACY_TIOCSTI`, `HIDRAW`,
`USB_HIDDEV`, virtio, the SCSI CD-ROM driver, the 22 vendor HID drivers
`x86_64_defconfig` builds, `HOTPLUG_PCI`, `ACPI_TABLE_UPGRADE`,
`EFI_CUSTOM_SSDT_OVERLAYS` and `EFI_RUNTIME_MAP`. A vendor's extra keys
are what the HID removal costs; `HID_GENERIC` and `HID_MULTITOUCH` still
read every keyboard, mouse and touchpad.

**`rcS`:** `/proc` is mounted `nosuid,nodev,noexec,hidepid=invisible`,
`/sys` `nosuid,nodev,noexec`, and `/dev` remounted `nosuid,noexec`;
`kernel.kptr_restrict` is 2 and `kernel.yama.ptrace_scope` 3.

**Not done.**

- `RANDSTRUCT` and `GCC_PLUGIN_STACKLEAK`: Buildroot's `linux/linux.mk`
  turns `GCC_PLUGINS` off for every kernel it builds.
- `EFI_DISABLE_PCI_DMA`: the kernel's help says it "will cause failures
  with some poorly behaved hardware and should not be enabled without
  testing". It waits for the stick's hardware tests.
- `PERF_EVENTS` on x86, which the architecture selects.
- Landlock for the app and `prctl(PR_SET_DUMPABLE, 0)` in the shell.
  These are changes to the shell's code, and Landlock's rule set has to
  be tested against the stick that is mounted after the app starts.

**Tested here.** Both release images build, which is when
`check-kernel-config.sh` holds the lists against the generated
`.config` (stick: 113 options out, 71 in; Pi: 64 out, 38 in). The stick
image passes `tools/stick-test.py` in QEMU: it boots into the app, takes
a mouse and a keyboard, and every USB stick case still mounts, with the
new `rcS` mounts in place. Hardware tests on each board (keyboard,
touchpad, webcam, stick, power-off) are the Faraday maintainer's, who
asked for these changes.

**What `kernel-hardening-checker` still reports** (run from its
repository on both configs; 66 failures on x86, 49 on the Pi). Most are
not available here: options the Pi's 6.1 ARM kernel does not have, the
GCC plugins Buildroot turns off, Clang's CFI, UBSAN. Some are what the
image is: `FB` and `VT` draw the display, `STAGING` holds the Pi's
camera, and `SECCOMP_FILTER` and `SYN_COOKIES` need `NET`. The `DEBUG_*`
options and SELinux are not wanted on a device. Left for a later round,
each cheap and each needing a rebuild and a boot: `LDISC_AUTOLOAD`,
`PROC_PAGE_MONITOR`, `LATENCYTOP`, `CACHESTAT_SYSCALL`,
`PROVIDE_OHCI1394_DMA_INIT` and `RSEQ` off; `DEFAULT_MMAP_MIN_ADDR`
raised; `ARCH_MMAP_RND_BITS` at its maximum; `PROC_MEM_NO_FORCE`,
`STATIC_USERMODEHELPER` and `KFENCE`; and Landlock with the shell's
rule set.

### 16.141 The boot import, and one line of fact on its sheet (2026-10-08)

**Why.** At boot the app read the boot stick's vault files into the
Inbox, and pulling the stick opened the first locked vault's passphrase
prompt. Anything else on the stick needed a stick visit before the pull,
and a PNG's QR codes were read one picture at a time behind a **Read the
QR codes** button, with its checkbox greyed out. The owner's ask: at
boot, copy everything on the stick into memory, say so and ask for the
stick to be removed; then offer the vaults to unlock; then list the
wallets the whole set holds, with their keys and whether they can sign,
and the files, to choose from; one button imports the choice and wipes
the rest.

**What it does** (`faraday-core/src/boot_import.rs`, its sheet in
`boot_import_screen.rs`; README, "Starting a session: the boot import";
`PLAN.md` §5.4). The first look at the boot stick in a power-on (the
condition the settings file is read under) copies every file: vaults
into the Inbox, where the vault screens unlock them, the rest into a
holding area beside it, a PNG as what its codes hold. `Sheet::Import`
over Home says what was copied and asks for the stick to be removed;
once it is out, it lists the vaults with **Unlock**, the wallets found
in the files and the open vaults once each by descriptor with "Can
sign", "k of n keys here · m more needed" or "Watch-only" and the files
that carry them, the keys no wallet uses, and every file for the Inbox.
**Import** loads through the Inbox's loader and the vaults' own, moves
the chosen files into the Inbox, and drops the rest, wiped as it drops.
Pulling the boot stick no longer opens a passphrase prompt; a lock drops
what was not imported; a later insertion is an ordinary visit. On a
visit, a PNG is ticked like any file and read on Import, and the **Read
the QR codes** button is gone.

**The owner's decision: one line of fact on a working sheet.** The sheet
carries, above its buttons, "Files not chosen are wiped from memory. To
bring one in later, insert the stick again." It says what the button
does and what follows from it, in plain statement, which §16.46 allows
anywhere; it is not an explanation of why, which §16.37 keeps in Learn.

### 16.142 Faraday upgrades another Faraday stick, and never signs inside the upgrade (2026-10-08)

**Why.** Upgrading a stick meant writing the new image over the whole
stick, which empties its data partition, or replacing `BOOTX64.EFI` on
another computer, which puts the vaults on that computer: it can copy
them to guess at the passphrase offline, delete them, and plant files. The
owner's ask: Faraday copies itself onto a stick that already holds
Faraday and a data partition, and the data partition is not touched.

**What it does** (Faraday's `PLAN.md` §5.5; not built yet). Settings →
**Upgrade a Faraday stick**, in the clean state: the stick Faraday
started from is read, then its boot partition is written raw over the
target's, read back and compared. The source is accepted only if it
carries the running kernel's release string, into which the build puts
the Faraday version and commit; every stick has the same partition
UUIDs, so unplugging and replugging the boot stick changes nothing. A
new process, `faraday-boot` (`ofboot`, uid 203), does the copy and reads
no FAT; the app can only ask it to copy, never hand it bytes.
`faraday-grant` hands it partitions named `OSKBOOT` only while the app
publishes an upgrade marker.

**The owner's decisions.**

- *The exception to "no boot partition is handed out" is small.* That
  rule (Faraday's `PLAN.md` §4.3) is there so a Faraday compromised while
  running cannot plant itself on a stick. Here the app never supplies
  the bytes; a source stick that was tampered with already controls the
  machine it booted; and without Secure Boot any computer the stick is
  plugged into can rewrite the boot partition anyway.
- *No signing inside the upgrade.* Signing there would have the app hand
  `faraday-boot` a file of its own making, checked by Authenticode
  digest against the source, and `faraday-boot` would parse PE and write
  FAT. Instead an owner with Secure Boot signs each release once through
  a spare stick (sign on the vault stick, copy the signed file onto the
  spare's boot partition on a computer, boot the spare, upgrade the
  vault stick from it), so the vault stick never meets an online
  computer. A Learn page and the README take the person through it.
- *A boot-partition-only flash for development* (`just
  faraday-stick-boot`), so a test stick keeps its vaults across builds.
  It is not offered to users: it puts the stick on the build computer.

### 16.143 A multi-choice list beside Choice, for the backup's plan (2026-10-09)

**Why.** Faraday's backup became a plan, then a checklist of only what
the plan needs (Faraday's `docs/WALLETS.md` §5, the owner's proposal A).
The plan's questions each take more than one answer: the seeds on paper
and in a vault, the wallet description on a sheet and in software,
several programs at once. Choice (§4.2) checks one row; a row of chips
for more is ruled out on every class.

**Decision** (owner: ok). DESIGN §4.2 gains **Multi choice**: the same
full-width rows as Choice, a checkbox at each row's start instead of the
accent check, any number ticked, Continue under the list, a row that may
not be ticked dimmed and inert. Nothing has to be ticked, so Continue is
never dimmed for it. Several lists may share a page, each under its own
label, where they answer one question (the software and its form). On a
small panel the plan asks one question per page, as every step flow
pages there. It is built in Faraday's core (`Ui::multi_list`); `osk-ui`
gets a component when an OpenSigner screen needs one.

### 16.144 On the Pi, "SD card" where the app says "stick" (2026-10-09)

**Why.** The Pi has no USB storage (PLAN.md §3), so every medium it sees
is an SD card, yet the app called it a stick on every screen: Home's
status line, Stick visit, the Pull and Lock sheets, the vault screens,
the boot import, the family guide.

**Decision** (owner: go-ahead). Faraday's core keeps
`Medium { Stick, SdCard }` (`faraday-core/src/medium.rs`), told to it by
the shell at start; nothing in the core looks at the architecture, so the
snapshot shell renders either wording at any size and the desktop app
stays "stick". The stick shell says SD card in its Pi build (armv7) or
when the image recorded the exchange partition on `/dev/mmcblk0`
(`/etc/opensigner/exchange`); the snapshot shell takes `--sd-card`.
Every string naming the medium takes its words from `Medium`: the noun
("stick" / "SD card"), the plural, the article form ("a stick" / "an SD
card"), the sentence-start forms ("Stick" / "SD card", "A stick" / "An SD
card"), the full name where the kind matters ("USB stick"), the visit's
name ("Stick visit" / "SD card visit") and the boot medium's ("Boot
stick" / "Boot SD card"); no string builds "a {noun}" by hand. The boot
medium's name moved out of `faraday-storage` into the app, which names a
`boot` medium itself whatever its volume label. Spelling: "SD card",
capital SD and lowercase card, "SD cards", never "card" alone; so the
PC's Files foot now reads "Waiting for a stick" where it read "Waiting
for a stick or card". Code names and file names are unchanged
(`StickInfo`, `stick_settings.rs`, `faraday-settings.txt`).

The medium's glyph follows it: `Icon::Drive` on the PC, `Icon::SdCard` on
the Pi, Font Awesome 6 Free Solid `sd-card` (U+F7C2) added back to the
icon face after §16.91 removed `MicroSd` as unused. `osk-ui` is shared
with upstream OpenSigner: the new variant, its place in `Icon::ALL`, its
code point and name in `core/osk-ui/src/widgets/icon.rs`, the
`tools/fontbake` entry and the rebaked `core/osk-ui/assets/icon.outl` are
an upstream change, offered to OpenSigner with this one.

Not settled here: the family guide's "Start from the SD card" card still
walks through a PC's boot menu, which a Pi does not have; its steps for a
Pi are the owner's to write. The online desktop app's mainnet warning
still says "booted from a stick", since it names the PC image to use, not
a medium the app sees.
