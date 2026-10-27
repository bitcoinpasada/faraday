# getrandom

**Purpose.** The desktop shell's answer to `Command::RequestEntropy`: 32 bytes from the operating system's
random number generator, consumed into the core's session key.

**Shell only.** The core has no random number generator by design (§4.2:
the shell is the only thing that talks to the OS). `getrandom` is a
dependency of Faraday's desktop shells (`faraday/shells/window`) alone;
it does not appear in any `core/` crate. The
stick shell reads `/dev/random`.

**Why this crate.** It is the standard binding to `getrandom(2)`,
`getentropy`, `BCryptGenRandom` and friends, with the fallbacks (older
kernels, `/dev/urandom`) handled, and it is already in the dependency
graph twice through `winit`'s platform crates, so it costs nothing.

**Cost.** Version 0.4, one crate plus `libc` on Unix, already present.

**Reopen when** the desktop shell moves to a platform layer that
exposes an RNG of its own.

**Review.** Version 0.4.3 is covered by audits imported from other organisations (`supply-chain/imports.lock`) from 0.3.4 onward; 0.3.4, which `ahash` in the window stack takes, is audited under `safe-to-deploy-read-by-ai`: read by an AI model for the project, the Linux, Windows and macOS backends in full (F71, `supply-chain/audits.toml`).