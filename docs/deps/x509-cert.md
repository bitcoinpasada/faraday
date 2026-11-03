# x509-cert

**Cryptography:** computes

**Purpose.** The self-signed X.509 certificates of the Secure Boot keys
(`faraday/faraday-sb/src/x509.rs`) and their reading: Faraday builds a
PK, KEK or db certificate with its `CertificateBuilder` and reads a
certificate's issuer, serial number, public key and common name, for the
`.auth` files, the signature lists, the signed images and the Secure Boot
screen. Faraday-only.

**Why this crate.** It is RustCrypto's X.509, by the publisher the other
format crates come from (`der`, `spki`, `const-oid`). Faraday wrote the
certificate's DER by hand before; the crate's builder signs through the
`signature` 3 traits, which `rsa`'s `pkcs1v15::SigningKey<Sha256>` meets
with its `RandomizedSigner` taking the blinding stream, so the
certificates are the same bytes for the same draw (DER is canonical) and
every one is still read by `openssl` and the images by `sbverify`.

**Cost.** Version 0.3.0 with `default-features = false` and the `builder`
feature (`pem` and `std` off: Faraday writes DER). New in the lock file:
`x509-cert`, and through the `der` features it turns on, `der_derive`
0.8.0 (a proc-macro over `proc-macro2`, `quote` and `syn`, in the tree)
and `flagset` 0.4.7. `sha1` 0.11.0, `signature` 3.0.0, `spki` 0.8.0,
`const-oid` 0.10.2 and `der` 0.8.2 are in the tree. The edge
`faraday-sb -> x509-cert` is new.

**What the reader refuses.** `x509::read` returns no certificate, and so
`common_name` has no name, `verify` is false, `signer_info` is refused
and an image carrying it reads as unreadable, for bytes that are not
one certificate in DER: empty or zero bytes, a certificate cut short (a
length that runs past the end), a sequence with fewer fields than a
certificate has, and three that Faraday's own reader let through: a
validity time out of range (an hour of 24; it read as a certificate with
no start time), a length that is not minimal (BER) and bytes after the
certificate. The crate's reader is stricter there, and the certificates
Faraday writes and those of the signed boot programs it checks (sbsign,
osslsigncode) are all DER. A certificate whose key is not an RSA public
key verifies nothing, and a signature by another key does not verify.
`faraday/faraday-sb/tests/certificates.rs` keeps a test of each.

**Reopen when** `faraday-sb` writes no certificate and reads none, or the
crate's builder no longer takes `rsa`'s signing key.

**Review.** Trusted by publisher (github:RustCrypto/formats) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here. Its `der_derive` is trusted by the same person as `const-oid` (`user-id` 267), and `flagset` is an exemption (`supply-chain/config.toml`: Mozilla's audit covers 0.4.3).
