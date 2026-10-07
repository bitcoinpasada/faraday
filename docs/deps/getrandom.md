# getrandom

**Purpose.** The desktop shell's answer to `Command::RequestEntropy`
(`docs/PLANNING.md` §5.2, §16.21): 32 bytes from the operating system's
random number generator, consumed into the core's session key.

**Shell only.** The core has no random number generator by design (§4.2:
the shell is the only thing that talks to the OS). `getrandom` is a
dependency of `opensigner-desktop` alone; it does not appear in any
`osk-*` crate or in `opensigner-core`, and the snapshot shell answers
from a fixed seed instead so that review renders are reproducible. A Pi
shell will read `/dev/urandom` or the SoC's hardware RNG; the phone
shells will call the platform API from Kotlin or Swift.

**Why this crate.** It is the standard binding to `getrandom(2)`,
`getentropy`, `BCryptGenRandom` and friends, with the fallbacks (older
kernels, `/dev/urandom`) handled, and it is already in the dependency
graph twice through `winit`'s platform crates, so it costs nothing.

**Cost.** Version 0.4, one crate plus `libc` on Unix, already present.

**Reopen when** the desktop shell moves to a platform layer that
exposes an RNG of its own.
