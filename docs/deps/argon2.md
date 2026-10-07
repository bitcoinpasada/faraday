# argon2

**Purpose.** Argon2id, the memory-hard half of the key that decrypts a
key kept on the device (`core/osk-keep/src/lib.rs`, PLANNING §15 item
32). The storage key is
`HMAC-SHA256(Argon2id(pin, salt_pin), secure_element_mac)`; Argon2id is
what makes each of the few thousand possible PINs cost 64 MiB and three
passes rather than one hash.

**Why this crate.** It is the RustCrypto implementation, by the same
authors as `sha2`, `hmac` and `chacha20poly1305`, which are already in
the graph, and it carries the RFC 9106 test vectors. It is `no_std` and
does not need an allocator: `hash_password_into_with_memory` takes the
working memory as a `&mut [Block]` the caller owns, which is how
`keep.rs` calls it, so the 64 MiB is allocated once at the one moment a
blob is written or opened and overwritten before it is dropped. Writing
Argon2id by hand was the alternative and is not one: it is several
hundred lines of indexing rules where a mistake is silent.

**Cost.** Version 0.6 with `default-features = false` and the `zeroize`
feature: three crates, `argon2`, `blake2` and `base64ct`. Nothing else
enters the graph, because 0.6 is on the RustCrypto generation the rest of
the graph is already on: `blake2 0.11` takes `digest 0.11`, which `sha2`,
`hmac` and `pbkdf2` already bring, and `cpufeatures 0.3`, which `sha2`
already brings. Version 0.5 took `blake2 0.10`, and with it a second copy
of the hashing stack — `digest 0.10`, `block-buffer 0.10`,
`crypto-common 0.1`, `generic-array` and `cpufeatures 0.2` — five crates
that the move to 0.6 removed. The default features are off, which drops
`password-hash` (the `$argon2id$v=19$...` string format and its parser)
and `getrandom` (an RNG this project does not have and does not want).
`zeroize` wipes the intermediate hashes inside the crate; `keep.rs` wipes
the block buffer it owns.

**Reopen when** the parameters move (the 64 MiB, three passes and one
lane are in `keep::DEVICE_PARAMS` and travel in every blob's header, so
raising them costs nothing but the write), or when a device wants the
platform's own hardware-backed KDF instead — on Android, deriving inside
the secure element rather than beside it would remove this crate and the
memory it needs.
