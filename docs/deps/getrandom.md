# getrandom

**Cryptography:** computes

**Purpose.** The desktop shell's answer to `Command::RequestEntropy`: 32 bytes from the operating system's
random number generator, consumed into the core's session key.

**Shell only.** The core has no random number generator by design (§4.2:
the shell is the only thing that talks to the OS). `getrandom` is a
dependency of Faraday's desktop shells (`faraday/shells/window`) alone;
it does not appear in any `core/` crate. The
stick shell reads `/dev/random`. The Starter's shell
(`faraday/shells/starter`) draws each stick's own disk and partition
GUIDs and FAT serials from it (`docs/STICK-IDENTITY.md` §4.2); they are
not secret.

**Why this crate.** It is the standard binding to `getrandom(2)`,
`getentropy`, `BCryptGenRandom` and friends, with the fallbacks (older
kernels, `/dev/urandom`) handled, and it is already in the dependency
graph twice through `winit`'s platform crates, so it costs nothing.

**Cost.** Version 0.4, one crate plus `libc` on Unix, already present.

**Reopen when** the desktop shell moves to a platform layer that
exposes an RNG of its own.

**Review.** Version 0.4.3 is audited by other organisations (`supply-chain/imports.lock`) only as a delta from 0.3.4, so with 0.3.4 an exemption it is one too; 0.3.4, which `ahash` in the window stack takes, is an exemption, not reviewed by a human; an AI model read it for the project, the Linux, Windows and macOS backends in full, recorded under `ai-says-safe-to-deploy`, which grants no trust (F71, `supply-chain/audits.toml`).