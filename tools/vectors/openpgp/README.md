# OpenPGP known answers

`rfc9580.txt` holds vectors from RFC 9580 (OpenPGP), Appendix A, as
`name = hex` lines, for the QR vault's message
(`faraday/faraday-pgp/src/message.rs`, `docs/VAULT.md` §1.2):

- `a10_skesk`, `a10_seipd`: A.10, a version 6 symmetric-key encrypted
  session key and a version 2 data packet, AES-128 and OCB, encrypting
  `Hello, world!` and a padding packet under the passphrase `password`
  (its S2K is the iterated and salted one). `a10_derived_key` is the key
  the S2K gives, `a10_session_key` the session key (A.10.2), and
  `a10_plain_literal` and `a10_plain_padding` the packets in the data
  packet's chunk (A.10.4). The tests wrap and unwrap the session key and
  encrypt and decrypt the chunk with Faraday's code, byte for byte.
- `a12_3_message`, `a12_3_session_key`: A.12.3, a version 4 packet with
  the Argon2 S2K (t = 1, p = 4, m = 2^21 KiB) under `password` with
  AES-256; the S2K output decrypts the session key. The test runs 2 GiB
  of Argon2id and is ignored by default: `cargo nextest run -p
  faraday-pgp --run-ignored all -E 'test(/argon2_s2k/)'`.

- `vault_13`: a QR vault (AES-256, OCB, Argon2 memory exponent 13) under
  the passphrase `correct horse`, the file
  `fuzz/corpus/pgp_qr_vault/vault-13.pgp`. Faraday made it
  (its S2K salt is the tests' random pattern, its plaintext the tests' 226-byte
  TOML), so the test is a known answer of Faraday's own seal, not an outside
  check. A reader outside the repository (argon2-cffi and OpenSSL) opened it
  once during review; that is not reproducible here.

RFC 9580 gives no vector for the Argon2 S2K with AES-256 and OCB in a
version 6 packet together; checking that pairing against Sequoia,
OpenPGP.js and rPGP in both directions is planned in the F79 area
`vault` and not done.
