# Release sums fixtures

What GnuPG makes when a release's `SHA256SUMS` is signed, for the check
Faraday runs before it signs a boot program (`docs/decisions/F134.md`):
`faraday-pgp`'s detached-signature verification and `faraday-sb`'s
release check are tested on them.

`make.sh` made every file but `faraday-0.1.7/`, with GnuPG 2.4.9, from
the repository root:

    tools/vectors/release-sums/make.sh

Each `gpg` call runs in a throwaway home with `--faked-system-time
1780000000!` (2026-05-28), so every key and signature was made at that
time. The keys are random: a new run makes other keys and other
signatures, and the tests' fingerprint of the test release key changes
with them. None of these keys guards anything.

| File | Made by | Read by |
|---|---|---|
| `BOOTX64.EFI` | Python in `make.sh`: the PE file `faraday-core`'s `testkit::test_efi()` makes, byte for byte (a test holds the two equal) | the release check, the app's tests |
| `SHA256SUMS` | `make.sh`: release 0.9.0's first line, a stand-in image line, and `BOOTX64.EFI`'s `sha256sum` under the name a release publishes it as, `faraday-0.9.0-BOOTX64.EFI` | all |
| `release-key.asc` | `gpg --quick-generate-key 'Release <release@example.com>' ed25519 cert 5y`, `gpg --quick-add-key FPR ed25519 sign 5y`, `gpg --armor --export FPR` (primary key `BAFE0CDD021B72A843668183662846B976716584`) | all |
| `other-key.asc` | the same for `Other <other@example.com>` | `faraday-pgp` |
| `SHA256SUMS.asc` | `gpg --local-user release@example.com --armor --detach-sign SHA256SUMS` (SHA-512, GnuPG's choice for an Ed25519 key) | all |
| `SHA256SUMS.sha256.asc` | the same with `--digest-algo SHA256` | `faraday-pgp` |
| `SHA256SUMS.other.asc` | the same with `--local-user other@example.com` | all |
| `SHA256SUMS.textmode.asc` | the same by the release key with `--textmode` (signature type 0x01, refused) | `faraday-pgp`, `faraday-sb` |
| `variants/NAME.sums`, `variants/NAME.sums.asc` | sums written by `make.sh`, each signed by the release key as `SHA256SUMS` is: the boot line twice with different sums (`twice-different`) and twice the same (`twice-same`), only another version's file (`other-version`), no version line (`no-version`), the version line second (`version-second`), release 0.1.0 (`older`), a `sha256sum` binary-mode line (`binary-mode`) and CRLF line ends (`crlf`) | `faraday-sb`, `faraday-core` (`older`) |
| `faraday-0.1.7/SHA256SUMS`, `faraday-0.1.7/SHA256SUMS.asc` | Faraday 0.1.7's published files, signed by the release key `faraday/release-key.asc`; `gpg --verify` reports a good signature made 2026-10-25 by subkey `347494985B8677F9BB93F58047154E304577FC3E` | all |

`make.sh` ends by running `gpg --verify` on the two signatures by the
release key; both are good.

The tests are `faraday/faraday-pgp/tests/verify.rs`,
`faraday/faraday-pgp/tests/verify_refusals.rs`,
`faraday/faraday-sb/tests/release.rs` and
`faraday/faraday-core/tests/all/secure_boot_sign.rs`; the fuzz targets
`fuzz/fuzz_targets/pgp_verify.rs` and
`faraday/fuzz/fuzz_targets/release_sums.rs` take them as their fixed
key and seeds.
