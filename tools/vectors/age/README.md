# age passphrase vectors

Test vectors for the age file format (age-encryption.org/v1, C2SP),
taken from the C2SP testkit, for the passphrase recipient Faraday reads
(`faraday/faraday-vault/src/age.rs`). Opened by
`faraday/faraday-vault/tests/age.rs`.

Source: <https://github.com/C2SP/CCTV>, directory `age/testdata`, the
`main` branch as of 2026-11-01 (commit
b88c4b206254ac543df240097e0f1fcc6a664821). The testkit is offered under
0BSD, CC0 1.0 or the Unlicense, to the user's choice, so copies need no
attribution; the files are unchanged.

## Taken

Every vector whose recipient stanza is `scrypt`, in `testdata/`: 25 files
named `scrypt*`, and `armor_scrypt`.

| Files | Expect | What the test checks |
|---|---|---|
| `scrypt` | success | the payload's SHA-256 is the vector's |
| `armor_scrypt` | success, armored | refused as armor (Faraday reads the binary form only) |
| `scrypt_bad_tag`, `scrypt_no_match`, `scrypt_work_factor_wrong`, `scrypt_uppercase` | no match | refused as a wrong passphrase, or as a file for another recipient |
| `scrypt_double`, `scrypt_and_x25519` | header failure | an scrypt stanza beside another stanza is refused |
| `scrypt_extra_argument`, `scrypt_salt_*`, `scrypt_not_canonical_*`, `scrypt_long_file_key`, `scrypt_work_factor_*` (other than `_wrong`) | header failure | each refused as a malformed header or stanza; `scrypt_work_factor_23` as too costly |

Each file is a few lines of `key: value`, an empty line, then the age
file; `tools/vectors/age/testdata/` keeps the testkit's format.

## Also taken: header, MAC and payload

The vectors that test the header grammar, the header MAC and the 64 KiB
STREAM payload are taken too, 56 files: `stream_*`, `hmac_*`, `stanza_*`,
`header_crlf`, `empty`, `version_unsupported`, and `x25519`,
`x25519_grease` and `x25519_multiple_recipients` (whose stanzas this
reader does not read, but whose header, MAC and payload it does). Their
files that state `compressed: zlib` are kept compressed, as the testkit
has them. A test in `faraday/faraday-vault/src/age.rs` runs each through
the header parse, the header MAC under the vector's `file key` and the
payload, and judges it against `expect`: header failure, HMAC failure,
payload failure, or success with the payload's SHA-256. A stanza of
another type is not opened there: the file key comes from the vector.

## Not taken

The vectors whose result depends on the recipient or on armor beyond the
passphrase stanza: `armor_*` except `armor_scrypt`, and `x25519_*` (other
than the three above) and `hybrid*`, which test an X25519 or ML-KEM
recipient or the armor's grammar. Faraday refuses armor and reads no other
recipient. Armor itself is recognised and refused
(`armor_scrypt`, and a test in `tests/age.rs`).

## Update

    curl -sL https://codeload.github.com/C2SP/CCTV/tar.gz/refs/heads/main

in an empty directory outside the repository; copy the files named above
from `age/testdata/`, and change the commit above.
