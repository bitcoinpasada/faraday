# rsa

**Cryptography:** computes

**Purpose.** RSA-2048 for Faraday's Secure Boot keys
(`faraday/faraday-sb`, `docs/history/PLAN.md` §8): UEFI Secure Boot verifies PK, KEK
and db signatures and Authenticode signatures with RSA PKCS #1 v1.5 over
SHA-256, so the platform key, key exchange key and db key are RSA-2048
and every certificate, `.auth` update and signed image Faraday writes
carries one RSA signature. Faraday-only.

**Why this crate.** It is the RustCrypto implementation, `no_std` with
`alloc`, constant time over `crypto-bigint`, and its keys zeroize. Version
0.10 is the one on the graph's RustCrypto generation (`digest 0.11`,
`sha2 0.11`, `signature 3`, `rand_core 0.10`); the stable 0.9 line would
bring a second hashing stack. Both 0.9 and 0.10 carry RUSTSEC-2023-0071,
the Marvin timing side channel (the advisory lists 0.10.0-rc.18 as still
affected, with no patched version); `deny.toml` ignores it because the
advisory is not reachable in Faraday, which never decrypts with RSA and
signs with blinding. Key generation's prime search is not constant time
(`docs/ESTABLISHED.md` §4.10). 0.10 is still a release candidate, so the
version is pinned exactly
(`=0.10.0-rc.18`). Writing RSA key generation by hand (prime search over a
big-integer library) was the alternative and is not one.

**Cost.** With `default-features = false` and `sha2` and `encoding`:
`rsa`, `crypto-bigint`, `crypto-primes`, `cpubits`, `ctutils`, `cmov`,
`num-traits`, `rand_core 0.10`, and through `encoding` the `pkcs1`,
`pkcs8`, `spki`, `der` and `const-oid` crates that read and write the
private keys a vault keeps (PKCS #8) and the public key a certificate
names. `encoding` saves writing PKCS #1 and PKCS #8 by hand; the
certificates around them are `x509-cert`'s (`docs/deps/x509-cert.md`),
the CMS and Authenticode structures `der` types (`docs/deps/der.md`),
checked by `openssl` and `sbverify`.

**Reopen when** 0.10.0 is released (drop the pin), or when Secure Boot
moves to keys firmware verifies some other way.

**Review.** Trusted by publisher (github:RustCrypto/RSA) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.