# BIP 129 (Bitcoin Secure Multisig Setup)

The specification text, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0129.mediawiki`.
The BIP is Complete; to refresh it, download it again and check the
digest changed for a reason.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `bip-0129.mediawiki` | 2026-09-18 | `e31e847c6b87f192086c89313f19beae3ac7ace4038e61bde983dcd8ba2e3814` | `core/osk-bip/tests/bsms.rs`, people |

BIP 129 ships no separate vector files: its examples are the `<pre>`
blocks of the text itself, and `core/osk-bip/tests/bsms.rs` reads them
out of the file rather than copying them. Four sessions are written out:

| Session | Keys | Key records | Descriptor record |
|---|---|---|---|
| `NO_ENCRYPTION` with public keys | 2 plain public keys | 2 | `wsh(sortedmulti(1,…))#rzx9dffd`, no path restrictions |
| `NO_ENCRYPTION` | 2 xpubs | 2 | `wsh(sortedmulti(2,…/**))`, `/0/*,/1/*` |
| `STANDARD` encryption | 2 xpubs | 2, plus each one's `.dat` | `wsh(sortedmulti(2,…/**))`, `/0/*,/1/*`, plus its `.dat` |
| `EXTENDED` encryption | 3 xpubs | 3, plus each one's `.dat` | `sh(wsh(multi(2,…/**)))`, `/0/*,/1/*`, plus one `.dat` per signer |

So the file carries nine key records, four descriptor records and the
encrypted forms of eight of them. The plaintext records of the two
encrypted sessions are written out beside their ciphertexts, which is
why a device that cannot decrypt can still be checked against seven of
the nine signatures and three of the four descriptor records — the
remaining pair is the session whose keys are plain public keys, which
this tree does not hold as a wallet.

The BIP gives each encrypted session's `ENCRYPTION_KEY`, `HMAC_KEY`,
`MAC`, `IV` and ciphertext, so an implementation of AES-256-CTR and
HMAC-SHA256 could be checked against it end to end. This tree has
neither primitive (`Cargo.lock` carries ChaCha20-Poly1305 and Argon2 for
the kept-key blob), so `core/osk-bip/src/bsms.rs` refuses an encrypted
record by name and the test only checks that refusal.
