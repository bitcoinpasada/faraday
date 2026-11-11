# The crypto cross-check

F79, `docs/CROSSCHECK.md` §10. Faraday's own Rust answers requests, and
pure-Python references recompute every answer, over every published
vector and a corpus of random inputs drawn from a fixed seed. The first
disagreement stops the area and is printed with its inputs and both
answers. Some references are others' code, code Faraday did not write;
others are Faraday's second version, a second implementation
Faraday's authors wrote, over others' code or alone, and those are not
checked against an outside source (F160, F177, F179): "Others' code and
Faraday's second version" below says which, per action.

## Running it

    just crosscheck              the stamp run: fixed seed, about 90 s on 12 threads
    just crosscheck --fresh      before a release: a new seed, ten times the cases
    just crosscheck --area musig one area (no stamp is written)
    just crosscheck --seed S     reproduce a run (with its --scale: 10 for --fresh)
    just crosscheck --jobs 4     four workers (default: one a core)
    just crosscheck --area tx --seed S --scale 10 --case corpus:1234
                                 one corpus case alone, as a report names it
    just crosscheck-status       how far a running cross-check has got
    just crosscheck-test         the harness's own tests (no Rust)
    just crosscheck-identity     the same report under 1 and 4 workers (and PyPy)

`run.sh` first checks that every outside tool is there (`judges.py`:
dosfstools, mtools, `sfdisk`, `sgdisk`, `xz`, 7-Zip (`7zz` or `7z`), QEMU, OVMF's Secure Boot firmware,
systemd-boot) and stops naming a missing one; their versions go into the
stamp. A full run then runs the judged test files (faraday-fat's `judged`,
`formatted`, faraday-files' `foreign`, the Starter's `ids`, `image_finish`,
`outside` and `stick_layout`) with `FARADAY_JUDGES_REQUIRED=1`, under which a missing
tool fails the test instead of skipping it. The OVMF Secure Boot run
(`faraday/tools/sb-ovmf-check.py`) runs beside the areas, and a failure of it
stops the run; each boot ends once its serial log holds the answer, or fails
at its timeout. Not with `--area` or `--identity`.
`run.sh` builds the Rust command with `cargo build --frozen` and runs
`crosscheck.py` in a network namespace of its own (`unshare --user
--net`), so nothing can be fetched. `crosscheck.py` refuses to run with
any interface there but loopback, and refuses when a vendored reference
does not match `references.sha256`, or when an unlisted `.py` file sits
in a reference folder.

## Bitcoin Core

An area that states `CORE = True` (`addr` so far) asks Bitcoin Core
v31.1 itself, through RPC (`core_node.rpc()`, with the vendored
`test_framework/authproxy.py`). When any area run has it, the run
starts one `bitcoind` before the self-test and stops it at the end
(`core_node.py`). The node runs on regtest with `-networkactive=0
-listen=0 -connect=0`, no DNS or fixed seeds, no wallet, a scratch
datadir, and RPC on loopback only, with a cookie. The run first brings up
`lo`, the namespace's one interface. The node is stopped and its
folders deleted on every way out, and killed if the run dies.

Core is not vendored. `tools/reference/core/release.pin` pins the
release tarball's SHA-256 for x86_64 and aarch64 Linux. Fetch it once,
with the network:

    python3 -I -B tools/crosscheck/fetch_core.py            this machine's tarball
    python3 -I -B tools/crosscheck/fetch_core.py --from DIR files already downloaded

The script requires `SHA256SUMS` to carry good signatures from at least
five of the builders in `guix.sigs`' `builder-keys/` (at the pinned
commit, fetched with git), and no bad one. It requires the tarball to
match both `SHA256SUMS` and the pin. It keeps the tarball in
`FARADAY_BITCOIN_CORE` (default `~/.cache/faraday/bitcoin-core`).
Every run hashes the tarball against the pin and unpacks `bitcoind` and
`bitcoin-cli` from it into a scratch folder (about two seconds with the
node's start). Missing, mismatched, or a node that does not answer, not
offline, or of another version: the run fails before the self-test,
naming it. The stamp's `bitcoin-core` line records the node's version
and the tarball's name and SHA-256. `stamp.py check` fails when they
are not the pinned release's.

The full run is for a release; the tip between releases is not kept
cross-checked (F197). A change to an area's code runs that area
(`--area`, which writes no stamp). A passing full run writes `stamp`,
with each area's corpus case count beside the seed; it is committed with
the release. `tools/lint-crosscheck.sh` (in `just lint-release`, about a
tenth of a second) hashes the crypto sources, this folder, the
references, the vectors and the crypto crates' `Cargo.lock` entries, and
fails with "crypto changed since the last cross-check: run `just
crosscheck`" when they no longer match the stamp. `python3 stamp.py show`
lists what is hashed.

Under `--fresh`, a disagreement is kept in `found/` with its seed,
case and inputs, to become a vector.

## Workers, and why the report does not depend on them

Each area is three parts: its prelude (the published vectors and fixed
cases), its corpus loops (`LOOPS` in the area), and a coda. Every corpus
case draws from a random stream of its own, seeded with
`<seed>:<area>:<loop>:<case number>`, so a case is the same whichever
process runs it and whatever ran before it. The run cuts each loop into
at most 50 batches by case number and starts a pool of workers, one a
core unless `--jobs` says otherwise. A worker process serves one area
(two areas' references import modules of one name, two `reference`s and
two `bip32`s, which one process cannot hold); a freed worker keeps to
its area while that area has about the most work left for each worker
on it, and otherwise moves to the one that has, so the heaviest areas
are spread over the most workers and every core stays busy to the end.
The answers are put back together in case order: an area's result is
its first batch, in case order, that did not pass, as if one worker had
run its cases in turn and stopped at the first disagreement; an area
whose loops cannot be read, or that ran fewer cases than planned, fails.
The report, the counts and the verdict are therefore the same for any
number of workers, on any machine, and a reviewer's run with a release's
seed and scale gives the release's report. Only the seconds differ (an
area's seconds are its batches' added together). `just
crosscheck-identity` runs the real areas at a small scale under one and
four workers, and under PyPy when it is found, and compares the reports
byte for byte; `just crosscheck-test` does the same with areas of its
own, without the Rust command.

Seeds from before this (2026-10-27), when each area drew every case from
one stream, reproduce their runs only on their own release tags.

On the build machine (12 threads), the stamp run takes about 90 s and a
`--fresh` run about 10 minutes, every worker busy until the last
minute; the same work run an area a process took 15 minutes.

Each worker is one Python process, with one Rust process beside it while
it runs a batch, and keeps the processes of the last two areas it ran: a
worker's Python took at most about 125 MB in a `--fresh` run, so the
run needs about 250 MB a worker at most, 3 GB on twelve. On a machine
short of memory, or shared with other work, give fewer: `--jobs 4`.

### PyPy

The references are pure Python. Under PyPy, `codex32` and `seedqr` run
about twice as fast, and the areas whose time goes to big-integer `pow`
(the curve arithmetic of `musig`, `sp`, `bip32` and the rest) run slower
than under CPython. So when `pypy3` (3.11 or later) is on the `PATH`,
the run gives PyPy those two areas (`--python auto`, the default), which
takes a `--fresh` run from about 11 minutes to 10 on the build machine;
`--python cpython` uses CPython alone, `--python pypy` (or a path to
one) gives PyPy every area it can import (the `vault` area,
which needs the distribution's packages for CPython, stays on CPython).
Pillow and zxing-cpp have no PyPy build: under PyPy the `seedqr` area
asks a CPython helper process (`qr_helper.py`) to decode the pictures, the
same decoders on the same pictures; a helper that dies stops the run. An
area PyPy cannot import runs under CPython, and the log says so.
The answers and the report are the same either way, and the stamp
records which Python ran what. PyPy is not needed: get it from the
distribution (Debian and Ubuntu: `pypy3`; Fedora: `pypy3.11`; Arch:
`pypy3`) or from pypy.org's downloads, checked against the SHA-256 on
pypy.org/checksums.html, and put its `bin/` on the `PATH`.

## Watching a run

A run writes `out/crosscheck/`: `progress.log`, one line at each event
(the build, each area's self-test, each area's start and end with its
counts, each disagreement as it is found with its case number, the
verdict) and a line on the whole run at least every ten seconds (cases
done, cases a second, workers busy, an estimate of the time left);
`progress.json`, the same state rewritten at most once a second; and
`report.txt`, the report. From another terminal, or over ssh:

    just crosscheck-status             the state now
    just crosscheck-status --follow    again every five seconds until the run ends
    just crosscheck-status --log       the log so far (or: tail -f out/crosscheck/progress.log)

Writing them is a few small writes a second at most; it changes no case
and no answer.

A full `--fresh` run also writes `out/crosscheck-record.txt`
(`record.py`), the release's record: the git tree it checked without
`docs/crypto/actions.toml` and `actions.lock`, whose fingerprints are stamped
from the run's result (`python3 -I -B tools/crosscheck/record.py tree <tag>`
gives it for the release), seed, scale and UTC date;
each area with its cases, answers, known differences and result; each
reference folder with its commit and the SHA-256 of its files; Bitcoin
Core's version and tarball SHA-256; and "every answer agreed" or the
first disagreement, or that the OVMF Secure Boot run failed. A signal to
`run.sh` ends both that run and the areas. `faraday/tools/stage-release.sh`
stages the record as `crosscheck-<version>.txt` beside `SHA256SUMS`, listed in it.

## The self-test

Before every run, each area runs against the Rust command started with
`FARADAY_CROSSCHECK_SABOTAGE=<op>`, which changes every answer to that
one operation. The run fails unless every area reports that operation
as a disagreement: a cross-check that cannot fail is worse than none.

## What runs

The table, the references' checks and the known differences below are
written by `facts.py` from the attributes each `area_*.py` class states
beside its `name` (`faraday`, `reference`, `reference_checked`,
`vectors`, `corpus`, `differences`, `not_compared`): edit the area, then
run `python3 -I -B tools/crosscheck/facts.py write`. `just lint` runs its
`check`.

<!-- facts:begin what-runs -->
| Area | Faraday | Reference (`tools/reference/`) | Published vectors | Random corpus |
|---|---|---|---|---|
| `musig` | `faraday_bip::musig` | BIP-327's `reference.py` (`bip327/`); btclib's MuSig2 (`btclib/`, its own curve) in full, answering every operation beside it and verifying the aggregate signature as BIP-340; btclib-wallet's BIP-373 roles over the Core round trip's PSBTs; Bitcoin Core v31.1 (`deriveaddresses`) for `musig()` descriptors | all eight files of `tools/vectors/bip327/`; the valid and invalid descriptors of `tools/vectors/bip390/` (Core) | sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial signatures, their verification (and a tampered one), aggregation checked as BIP-340, deterministic signing; `tr(musig(…))` wallets of 2 to 4 keys, as xpubs on both chains or as plain keys, whose scripts Core pays too |
| `sp` | `faraday_bip::silent` | BIP-352's `reference.py` (`bip352/`); btclib-wallet's `silent_payments.py` (`btclib/`, its own curve) answering every operation beside it | every case of `tools/vectors/bip352/`, both sides, each answered by btclib-wallet too | inputs of every kind BIP-352 reads, receivers with labels, a sender's outputs and what each receiver finds among them and decoys |
| `sps` | `faraday_psbt::silent`, `faraday_bip::dleq` | BIP-374's `reference.py` (`bip374/`) and BIP-375's validator and `test_runner.py` (`bip375/`), over `secp256k1lab`; btclib's `dleq.py` and btclib-wallet's BIP-375 roles (`btclib/`, its own curve) answering beside them | both CSV files of `tools/vectors/bip374/` and `tools/vectors/bip375/bip375_test_vectors.json`, each judged by btclib too | proofs made and verified, damaged or not, under two generators; version 2 PSBTs of one to four inputs of every kind BIP-375 reads and some it does not, one to three silent payment outputs, shares added by one signer or two, a global share, and each finished PSBT damaged in one of eighteen ways, judged by the BIP's validator |
| `bip85` | `faraday_bip::bip85` | btclib-wallet's `bip85` (`btclib/`, on btclib's own curve) run as written over bip39_slip39 (`bip39-slip39/`); and `ethankosakovsky/bip85` over python-mnemonic and a pycoin shim whose every call is btclib's (`bip85/`, `bip39/`) | every application vector of `tools/vectors/bip85/` | random master keys, indices, languages, word counts and lengths, with refusals |
| `slip39` | `faraday_bip::slip39` | python-shamir-mnemonic (`slip39/`), its `recover_ems`, `split_ems` and `_interpolate` for a set's EMS, a new split of it and a member at an index; bip39_slip39 (`bip39-slip39/`) answering each SLIP-39 operation beside it (not the two codex32 operations); python-codex32 (`codex32/`) for the codex32 side of the conversions and btclib (`bip32_ref.py`) for the fingerprint; the refusals of a member at an index before the arithmetic are ours (`derive_refused`), and so are the lengths the fixed 1 of 1 refuses, which no second judge covers | all 45 of `tools/vectors/slip39/`, each again through its EMS and its decryption | splits fed the same random bytes, which must match word for word; recovery from qualifying and short subsets and a changed word; the passphrase cipher; a set's EMS from a quorum, refused from a short or changed set, split again on the same random bytes (some under a new identifier where extendable) and a member added or a lost one made again at an index, each recovering the first set's secret, and a member refused at an index given, past 15, from a short quorum and across two groups; random seeds of 16 and 32 bytes through the fixed 1 of 1 against both libraries under the fingerprint's first 15 bits, recovered through both, and the lengths it refuses, against Faraday's own rule; the seed written as codex32 by Faraday and read by python-codex32, and a SLIP-39 set with a passphrase recovered by Faraday and by python-shamir-mnemonic, written as codex32 by Faraday and by python-codex32, the second read by Faraday, the same bytes both ways, the identifier's 20 bits written and the fingerprint (btclib's) compared on both sides |
| `codex32` | `faraday_bip::codex32` | python-codex32 (`codex32/`); rust-codex32 at a pinned commit (`rust-codex32/`), a program outside the workspace: parsing, checksums, the threshold and a threshold of 0's share index, `interpolate_at` for a fresh set's shares; volvelle-website's checksum worksheet run as a program over every worksheet of 48 and 74 characters; the worksheet of 127 characters written in the area from python-codex32's polymod | every vector of `tools/vectors/bip93/`, the 55 invalid strings included; volvelle-website's worksheet cells (`tools/vectors/codex32-volvelle/`); the booklet's checksum, derivation, recovery, addition, translation and fusion tables (`tools/vectors/codex32-booklet/`); the Codex32QR profile's sets (`tools/vectors/codex32qr/`) | secrets, splits (the same randomness), recovery, derived shares, corrupted strings; fresh sets of every threshold up to 31 strings at 48, 74 and 127 characters, the checksum worksheets of their strings and of corrupted ones, checksums made on a sheet, translation and recovery symbols, translation worksheets |
| `codex32-correct` | `faraday_bip::codex32::correct` | python-codex32's decoder, `correction.py` at a later commit (`codex32/correction/`), asked with BIP 93's bound | python-codex32's frozen correction corpus, 57 cases (`tools/vectors/codex32-correction/`) | random strings of every length with substitutions and unreadable boxes up to and past the bound, runs of unreadable boxes around the checksum's length, a held string's header fixed, upper case, `ms1` damaged |
| `xor` | `faraday_entropy::SeedXor` with `faraday_bip::bip39` | Coldcard's own `xor` and `xor_split` (`coldcard/`, unchanged) with python-mnemonic (`bip39/`) for the words | Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's two | splits and combines in all ten languages and five lengths, with refusals; parts made by Coldcard's `xor_split` read back to their seed |
| `hash` | `faraday_bip::hashes` and `faraday_codec::encodings` (`read`, the writers and `from_hex`), `faraday_core::file_menu::sha256`, and `faraday_crypto::hkdf_sha256` and `hkdf_sha256_expand`, `hmac_sha256`, `hmac_sha512`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` and `scrypt` | Bitcoin Core v31.1's functional-test framework (`core/`): `ripemd160.py`, `address.py` and `segwit_addr.py`, with `hashlib` for SHA-256 and `bytes.fromhex` for hex; PyCA cryptography's `HKDF` and `HKDFExpand` (OpenSSL) for the one HKDF; `hmac`, `hashlib.pbkdf2_hmac` and `hashlib.scrypt` (OpenSSL) for HMAC, PBKDF2 and scrypt | FIPS 180-4's SHA-256 examples, RIPEMD-160 of `abc`, BIP-173's key and its HASH160, Bitcoin Core's first `key_io_valid.json` address, BIP-173's and BIP-350's bech32 strings and their valid and invalid segwit addresses (`tools/vectors/addresses`) | random bytes of every length from 0 to 130 and to 300 hashed (SHA-256, SHA-256d, HASH160) and as a file, files of 4 KiB, 64 KiB and 1 MiB with a byte more and less; random bytes written and read as Base58Check, Base58, bech32 and bech32m, Core's writing read by Faraday, segwit addresses of every version Core's framework writes, and texts changed, cut or made of random characters read by both |
| `vault` | `faraday_vault`'s file (`file::create`, `open` and `seal`, the document model and the items of §7), its payload layouts and a passphrase's NFC form with the Unicode version it is made under, `file::create_holding` (an export), and `faraday_backup::argon2id` | pykeepass and `keepassxc-cli` (the distribution's packages) for the vault file, both ways, and keepass-rs for the file format only; `docs/VAULT.md` §7 written out in `area_vault.py`, over argon2-cffi and PyCA cryptography's Argon2id (OpenSSL's) (the distribution's packages) and Python's `unicodedata`, for the payloads | the Argon2id known answer (`tools/vectors/vault-primitives/`); the export vector of `tools/vectors/backup/`, read with a byte changed refused, and with each bit of the version field and bit 44 of the Argon2 iterations changed refused; every file of `tools/vectors/vault/`, read by Faraday and by both outside readers | Argon2id at small random costs and the costs it refuses; payloads made, read and malformed; passphrases in NFC, by case and at random; vaults Faraday writes at small costs with items of every kind of §7 and seals once more, read by pykeepass (every group, entry, field, protected mark and custom data item) and `keepassxc-cli` (listing and every field), under the passphrase, its NFD form, a wrong one and a changed byte; exports (`file::create_holding`) of one to eight entries whose Title, Notes and Password hold control characters, read the same way;  files pykeepass makes at random (ciphers, KDFs, compression, tags, icons, custom properties, attachments, history, custom data) and `keepassxc-cli` makes (groups, entries, notes, attachments), opened in Faraday and sealed once more with nothing of the other program's lost; keepass-rs (format only, not counted for the primitives) reads the vaults Faraday writes and seals, with a wrong passphrase, the NFD form and a changed byte refused, and writes files (AES-256 or ChaCha20, Argon2d or Argon2id, compression or not, groups, entries, tags, icons, properties, attachments, custom data) that Faraday opens and seals once more |
| `bip39` | `faraday_bip::bip39` | python-mnemonic (`bip39/`); bip39_slip39 (`bip39-slip39/`) answering every operation beside it | the ten wordlists; every vector of `tools/vectors/bip39/vectors.json` in Faraday's languages; the Japanese vectors with non-ASCII passphrases | entropy to words and back in every language and length; seeds under random ASCII passphrases; a changed word, an unknown word, a word too many or too few; every valid last word |
| `bip32` | `faraday_bip::keys` (master, private derivation), `faraday_bip::xkey` (strict decoding), `bitcoin`'s `Xpub::derive_pub` | btclib-wallet's `bip32` over btclib and `ecc` (`btclib/`), on btclib's own curve with libsecp256k1 turned off, through `bip32_ref.py`, and `hdkeychain` from btcd v0.27.0 (`btcd/`, Go, decred's secp256k1); the versions a strict reader takes (BIP-32's four, from btclib's network table), a version whose kind differs from the key's data, and a depth-0 key with a parent or an index are ours | BIP-32's vectors 1 to 5 (`tools/vectors/bip32/`), python-mnemonic's master keys | seeds of 16 to 64 bytes on every network, random paths with every index, public derivation against private, keys broken one rule at a time and the master-key import, each judged by btclib and by `hdkeychain` |
| `addr` | `faraday_bip::address::parse`, `bitcoin`'s `Address::from_script`, `faraday_bip::account` (account xpub, SLIP-132, descriptor, addresses), `faraday_bip::descriptor`'s checksum, `faraday_bip::policy::WalletPolicy::from_descriptor` (an account descriptor's kind, text and script at an index) | Bitcoin Core's functional-test framework (`core/`): `segwit_addr.py`, Base58Check, `descriptors.py`; BIP 380's own checksum code as the BIP prints it (`bip380/`); btclib-wallet's `bip32` and `slip132` (`btclib/`, on btclib's own curve) for accounts' keys, and its descriptors (`descriptors`) for each account descriptor's kind, text, script at an index and checksum, and as a third checksum judge; Bitcoin Core v31.1's `bitcoind` on regtest for every account's address (`deriveaddresses`) and for reading Faraday's regtest descriptors (`getdescriptorinfo`) | BIP-350's and BIP-173's addresses, BIP-380's checksum cases, BIP-49's, 84's and 86's accounts (`tools/vectors/addresses/`) | scripts of every address kind on every network, written and read back, then changed; accounts of every type at random indices, their descriptors read and changed; random descriptors' checksums |
| `tx` | `faraday_psbt::transaction::read_raw`, `faraday_core::decode::decode`'s txid, wtxid and output addresses, `bitcoin`'s `SighashCache` as `faraday_psbt::sign` calls it | btclib's `Tx.parse` and `sig_hash` (`btclib/`); Bitcoin Core v31.1's `decoderawtransaction` on regtest (`core_node.py`) for the transactions; Core's `messages.py` and `script.py` (`core/`) for the signature hashes; the refusal of a transaction with no inputs is ours | Core's 500 legacy sighashes, BIP-143's thirteen, BIP-341's key-path sighashes (`tools/vectors/sighash/`) | random transactions with and without witnesses (txid, wtxid, weight, bytes) read by btclib and Core's `decoderawtransaction`, cut or lengthened, and shown by Faraday's Decode view (txid, wtxid, each output's address, none where Core gives none) against Core's; every input hashed as legacy, BIP-143 P2WPKH and P2WSH, BIP-341 key and script path under every hash type, by btclib and the framework |
| `psbt` | `faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` (version 2, Faraday's own) | Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (`decodepsbt`, `combinepsbt` of one PSBT as its writer, the test framework's `taproot_tree_helper`); HWI's `psbt.py` (`hwi/`) for version 2 and the version 2 fields refused in version 0, its version 0 form read by Core; btclib-wallet's `Psbt` (`btclib/`, on btclib's own curve) for both versions: `parse`, `lock_time`, `tx`, `to_v0` and `to_v2`, which also decides the refusals of a taproot signature whose sighash byte is 0, of bytes after the last map and of a stream that ends inside a map | every PSBT of BIP-174's, 370's and 371's test vectors (`tools/vectors/psbt-bips/`), with BIP-370's locktimes, read by btclib too | random version 0 and 2 PSBTs with every field Faraday reads, taproot trees among them, in random order, read field for field as Core reads them and written back byte for byte as read and as Core writes them, then broken one rule at a time; each version 0 PSBT made version 2 as HWI's `convert_to_v2` and btclib's `to_v2` make it, and back to its own bytes; BIP-370's locktime, transaction and version 0 form under every mix of required locktimes, also btclib's `lock_time`, `tx` and `to_v0`; one version 0 PSBT in eight carries fields btclib refuses and btclib is not asked of it |
| `sign` | `faraday_psbt::sign` (every held key selected, deterministic Schnorr, both RFC 6979 nonce rules) and `faraday_psbt::finalize` | Bitcoin Core v31.1 on regtest (`core_node.py`): `descriptorprocesspsbt` signs from the held masters' keys, `finalizepsbt` finalizes and extracts, `testmempoolaccept` judges each finalized transaction over its funding on the node; BIP-340's reference (`secp256k1lab`) and Core's `key.py` for the deterministic nonces, over Core's `script.py` sighashes; python-ecdsa (the distribution's package) and btclib (`btclib/`, its own curve) as a second curve lineage, beside them: every ECDSA signature again under plain RFC 6979 or verified, every BIP-340 signature verified and made again; btclib-wallet's finalizer (`psbt.finalize` with its miniscript solver) finalizes every input Faraday finalizes again and gives the same scriptSig and witness; for each sweep, btclib reads the WIF (scalar, compression, public key) and signs and finalizes the sweep on its own curve, and python-ecdsa makes the first-nonce signature of each input again, byte for byte; python-bip380 (`miniscript/`) reads the leaves. Faraday's refusals and the inputs a wallet's rule leaves unsigned are ours, in `area_sign.py` | BIP-174's signer, finalizer and extractor vectors; the Core-built spends of `tools/vectors/psbt/` (2-of-3 `sortedmulti`, the Liana wallet down both paths, a taproot tree by a leaf and by the key path, BIP 387's taproot multisig), signed by every subset of their keys; the finalizer vectors there (`or_b`'s smaller signature, the taproot leaves that tie by count, the taproot multisig and MuSig2 spends) | PSBTs of one to four inputs, each spending a funding output on the node: P2PKH, P2SH-P2WPKH, P2WPKH, `multi`/`sortedmulti` in P2SH, P2WSH and P2SH-P2WSH, P2TR key path, P2TR script path (`pk`, `pkh`, timelocked keys, `or_d`, `multi_a`) with the key path beside it, miniscript in P2WSH and P2SH-P2WSH; held and foreign keys, cosigners' signatures already on (Core's), every standard sighash type, timelocks met and not; Core's signatures (ECDSA byte for byte, BIP-340 by verification and the deterministic nonce), each final scriptSig and witness and the extracted transaction Core's, the cleared fields, and every finalized transaction accepted by Core's mempool; an origin that derives another key refused; sweeps: a WIF key's P2PKH, P2WPKH and P2SH-P2WPKH outputs (P2PKH alone for an uncompressed key) spent beside another key's, signed by Core from the WIF and by `faraday_psbt::sweep::sign`, byte for byte, finalized and accepted by the mempool |
| `sign-check` | `faraday_psbt::verify::verify_signatures` (every signature of a PSBT, in a partial field or a final scriptSig or witness, checked against its signature hash) and `faraday_psbt::verify::repeated_nonces` (one `r` or `R` under one key in more than one place) | python-ecdsa (the distribution's package, its own curve) and btclib (`btclib/`, libsecp256k1 off) verify every signature over btclib's signature hashes (`sig_hash.py`); btclib's script engine (`verify_input`, standardness rules on) runs every complete input and, under consensus's rules alone, every final input of the table (`CONSENSUS`); Bitcoin Core v31.1's `finalizepsbt` and `testmempoolaccept` judge the complete inputs and the transaction, and are named as sharing libsecp256k1 with Faraday; the reuse of a nonce is ours (`reused`, in `area_sign_check.py`) | none: BIP-340's and ECDSA's published vectors are in the areas that sign (`sign`, `msg`); the signatures here are made for the PSBT in the case, with verdicts given by two verifiers | transactions of one to three inputs of legacy P2PKH, P2WPKH, P2WSH multisig (1 of 2 to 3 of 3), taproot key path and taproot script path, each with its signatures partial (in the PSBT's fields) or complete (in a final scriptSig or witness), good, under the wrong key, over the wrong sighash type, with a high S, or malformed, with every sighash type; nonces reused on purpose across inputs that share a key (ECDSA with `k` and `-k` too, Schnorr), one nonce under two keys, and one key under fresh nonces; and, for the verdict on a final input, a table of about ninety rows, each in every wrapping its script allows: `and_v`, `thresh`, `pk`, `pkh`, an `or_d` recovery path with `older`, 2 of 2 over [A,A], 2 of 3 over [A,A,B] and [A,B,A], the dummy empty and not, junk, repeated, swapped and extra signatures, 2 of 17 and 17 of 20, uncompressed keys, bare P2PK and multisig, P2PKH and P2WPKH true and forged, a hash time lock on both branches and three false spends of it, a redeem script of 500 and of 517 bytes, a high S and sighash bytes no standard defines, with keys drawn for each case; and about thirty taproot rows, run under TAPROOT: a key path and a script path on a Tap miniscript leaf, true and forged, an annex signed and not, SIGHASH_SINGLE with no output of its index, a control block that does not commit to the output key, a 65-byte element ending 0x00, and the leaves not run (another leaf version, a hash lock, OP_CODESEPARATOR, OP_DROP) |
| `psbt-review` | `faraday_psbt::inspect` (amounts in and out, the fee, verified change and the spoofed-change and missing-UTXO warnings), over `faraday_psbt::Psbt` and the keys and registered wallets the Sign screen holds | Bitcoin Core v31.1's `bitcoind` on regtest, offline: `decodepsbt` and `analyzepsbt` for amounts and fee, `deriveaddresses` from `tprv` descriptors for the addresses the PSBTs pay, `createpsbt`, `createrawtransaction` and `descriptorprocesspsbt` to make the PSBTs; btclib's descriptor wallet (`update_psbt_output` and `index_of`, `btclib/`, its own curve) for which outputs are change; a watch-only descriptor wallet on the same node (`getaddressinfo`, `walletprocesspsbt`) for where each change claim's script is; the area adds each input's UTXO fields, splices a spoofed output, derives the account keys a registered wallet names (checked by Core deriving the same addresses from them), and keeps the rule that a script of several keys is change only for a registered wallet | none: Core's decoding and btclib's change decision are the references, over each wallet shape with change, spoofed change, a foreign claim and missing UTXO data in the prelude | PSBTs of one to three inputs and one to four outputs over single-key accounts (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR) and registered wallets (`sortedmulti` in P2WSH, a miniscript in P2WSH, a taproot tree, a taproot `sortedmulti_a`) or the same wallets unregistered: witness UTXOs, previous transactions and no UTXO data, fees from nothing to a third of the inputs, payments, change on either chain, another wallet's change, and spoofed change over the wallet's own script at another index, another wallet's and a stranger's; one key held, two, or none of the wallet's |
| `import` | `faraday_intake::wallet::read_wallet` (the wallet a file or text is: descriptors, `faraday_bip::coldcard`, `faraday_bip::multisig_config`, Caravan's, Electrum's, Cobo's, Keystone's and Wasabi's files, Specter DIY's `addwallet`, `faraday_bip::bsms::DescriptorRecord`), and `faraday_bip::bsms::SignerRecord` read and verified and `faraday_bip::bsms::signer_record` over the BIP-48 or BIP-87 account node's key | Bitcoin Core v31.1's `bitcoind` on regtest, offline (`deriveaddresses`), and its `descriptors.py` (BIP-380's checksum) and `address.py`, for each wallet's receive and change scripts and every checksum; btclib-wallet 2026.10.29's BIP-32 (btclib's own curve) for every key, btclib 2026.10.24's SLIP-132 versions and addresses, and its `bms` for every key record's signature, made, verified and written byte for byte; CPython's `json` for the value of every JSON file read and for a key named twice and a nesting past 32 refused (those two rules are the reader's own, written beside it); the files are written in the area from each format's documentation, and the meaning of their fields is the area's | BIP 129's key and descriptor records (`tools/vectors/bip129/`), and 93 real files from the wallets' own test suites, 62 of them a wallet and 31 refused by both sides (`faraday/faraday-core/tests/vectors/interop/`: Sparrow's and BlueWallet's; `tools/vectors/wallet-files/`: Coldcard's firmware tests, Caravan's and Liana's): BSMS records, Coldcard setup files and exports, Caravan, Electrum, Cobo, Keystone, Jade, Specter Desktop and Specter DIY files, descriptors and a Core `listdescriptors` | single-key wallets (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR), `sortedmulti` in P2WSH, P2SH-P2WSH and P2SH of one to fifteen keys, a miniscript in P2WSH, a taproot tree and a taproot `sortedmulti_a`, on mainnet and testnet keys, each as a descriptor (multipath with and without checksum, receive and change lines, labelled, receive alone, Core's `listdescriptors`, Specter Desktop's file, Specter DIY's `addwallet` with and without its keys' derivations) or as the files that state its kind (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, Wasabi's, a key alone, a BSMS descriptor record), in BIP-32's or SLIP-132's spelling; damaged by a wrong checksum, a wrong first address, a SLIP-132 version of another script or another paths line; key records signed by btclib in each header form, with or without damage, and Faraday's own record of random seeds |
| `taproot` | `bitcoin`'s `TaprootBuilder` and `TaprootSpendInfo` (as `faraday_bip::tapmulti` and `miniscript`'s `tr()` build trees), `ScriptBuf::new_p2tr` as `faraday_psbt::sign` checks a key path, `Address::p2tr_tweaked`, `ControlBlock` decoding and `verify_taproot_commitment` | Core's `taproot_construct`, `key.py`'s tweak and `segwit_addr.py` for the trees; btclib's `check_output_pubkey` (`btclib/`, on its own curve) and btcd v0.27.0's `txscript.ParseControlBlock` and `VerifyTaprootLeafCommitment` (`btcd/`, Go) for the control blocks; the rule that a leaf version is not 0x50 is ours | BIP-341's `scriptPubKey` wallet vectors, all seven (`tools/vectors/sighash/`) | trees of 0 to 12 leaves in random shapes with random scripts and leaf versions over random internal keys: leaf hashes, Merkle root, tweak, output key and parity, script, address, control blocks; each control block checked, then changed eleven ways, each judged by btclib and by btcd; internal keys that are no point, odd leaf versions and 0x50 refused |
| `compile` | `faraday_bip::compile::compile` (a concrete policy to a `wsh()` or `tr()` descriptor, the keys it names, the ways it can be spent and the script it pays at an index), over rust-miniscript's policy compiler | Bitcoin Core v31.1's `bitcoind` on regtest (`getdescriptorinfo` and `deriveaddresses`), with its `descriptors.py` (BIP-380's checksum); btclib-wallet's descriptors and miniscript (`btclib/`, on btclib's own curve) for the kind, checksum, script at an index and keys; Antoine Poinsot's python-bip380 (`miniscript/`) for the miniscript's validity, sanity and script, and its fragments for the lift to a formula, which is ours (`lift`, in `area_compile.py`) | none: no compiler states a script for a policy (rust-miniscript, sipa's and the Go and Python ones choose differently), so the answers are judged by what the descriptor is and means | policies of `and`, `or` (with weights), `thresh`, `pk`, `after` and `older` of one to six keys, each key written as an account key with its origin, compiled for `wsh()` and for `tr()` (a key beside the rest, which the compiler takes as the internal key), on mainnet and testnet keys, read at random indices of both chains; refusals: a threshold of 0 or above its parts, a lock of 0, a key twice, nothing typed |
| `miniscript` | `faraday_bip::policy::WalletPolicy` (a wallet's descriptor and the script it pays at an index, its kind, text and spend paths), `faraday_bip::policy::parse_script_descriptor`, a witness script read back as `faraday_psbt` reads one, `faraday_bip::recovery::RecoveryPolicy` (a recovery wallet built from its parts and read back) | Bitcoin Core v31.1's `bitcoind` on regtest (`deriveaddresses`) for random wallets, their cosigners' keys from btclib (`btclib/`, on btclib's own curve); btclib-wallet's descriptors and miniscript (`btclib/`) for each descriptor's kind, text and script, Core's vectors' types and scripts, and the satisfaction of each way a wallet's coins move under its lock times; Antoine Poinsot's python-bip380 (`miniscript/`) for Core's vectors that state no script | Bitcoin Core's miniscript unit tests, 97 miniscripts in P2WSH and Tapscript (`tools/vectors/miniscript/`) | wallets of the Liana, Nunchuk, Sparrow and recovery shapes in `wsh()` and `sh(wsh())`, `multi`/`sortedmulti` in `sh()`, `wsh()`, `sh(wsh())`, `tr()` with trees of one to six leaves or a lone `sortedmulti_a`, over random accounts on every network, at random indices of both chains; the same made not sane, refused; recovery wallets of both forms with one to three recovery paths, and the ways each wallet's coins move spent at their lock times |
| `msg` | `faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids) | Bitcoin Core v31.1 and btclib for BIP-137, btclib-wallet and btcd for BIP-322: Core's `signmessagewithprivkey`, `verifymessage` and `signrawtransactionwithkey` (the low-R P2WPKH witness); btclib's `bms` and btclib-wallet's `bip322` (`btclib/`, btclib's own curve); python-ecdsa (the distribution's package) for the first-nonce ECDSA signature of BIP-137; btcd's `bip322` from pull request 2521 (`btcd-bip322/`, Go) | BIP-322's basic and generated vectors (hashes, txids, simple, full and proof-of-funds signatures, errors) and btcd's negative vectors; Trezor's and Electrum's BIP-137 signatures (`tools/vectors/message/`), Trezor's made again from its keys | random keys, address forms, networks and messages: virtual txids; signatures made byte for byte under both nonce rules and checked by every reference, then checked against a changed message, another address, a changed byte and a moved header; btcd's own simple signatures checked by Faraday |
| `own-randomness` | `faraday_core`'s `fresh.rs` (`Own`, the session seed and pool) through the testkit's `fresh` and `seed_draw`, with `faraday_entropy`'s dice, coin, camera and pointer-and-keys commitments | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3 written out in `area_own.py` over `hashlib` | the known answers of `faraday/faraday-core/tests/all/own_randomness.rs`: nothing added, dice before and after the system's answers, dice then coins, a batch of pointer moves and keys, a lock | random sessions of system answers, dice, coins, camera frames (some refused, some empty) and pointer moves and keys (none to several batches), locks, seed draws, fresh bytes and counts, in any order |
| `entropy` | `faraday_entropy`: dice under the three procedures, coins, hex, cards, camera frames, the device's answer, a mix | the procedures written out in `area_entropy.py` over `hashlib`, from `tools/reference/entropy/README.md` and the dice sources quoted in `tools/reference/dice/README.md`, except that the hashed dice procedure and every rolls commitment are Coldcard's own `rolls.py` and `rolls12.py` (`tools/reference/coldcard-rolls/`, run, public domain) and hex digits become bytes through CPython's `bytes.fromhex`; SeedSigner's `mnemonic_generation.py` (`tools/reference/seedsigner/`) over embit's BIP-39 (`tools/reference/embit/`) reads the hashed procedure at 50 and 99 rolls and direct selection's last word back | Coldcard's `rolls.py` digest, EntropyLab's six-as-zero and direct word selection (`tools/vectors/entropy/`) | runs of every source at every strength, short of the count, at it and past it, with faces, cards, frames and characters the pads refuse and every source past its limit; mixes of one to six sources given by their inputs |
| `roller` | Faraday Demo's dice rollers (`faraday_core`'s `fresh/roller/`, through the testkit) | `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` | every byte value as an answer: each face from 42 bytes, the four above 251 skipped; every byte as a face of every die of 2 to 20 faces (`faraday_entropy::uniform_face`); the booklet's 32 paths on six- and twenty-faced dice and its order with an equal pair (`roller.sheet`) | random answers (some mostly skipped bytes) under clicks of every size into entries of random room, with resets; the five dice of random faces (2 to 20) under clicks of every reach into codex32 shares of random count and length, answers running out part way; rolls of random dice entered by hand, equal pairs among them |
| `seedqr` | `faraday_codec::seedqr` (SeedQR, CompactSeedQR), `faraday_codec::qr`, the encoder that draws them, and `faraday_codec::qr::codex32qr` with the picture `faraday_core::picture` writes of it | SeedSigner's own encoders and `DecodeQR` (`seedsigner/`, unchanged, with embit's BIP-39 code in `embit/`); Project Nayuki's `qrcodegen.py` (`qrcodegen/`), module for module; segno 1.6 (`python3-segno`), an encoder of another lineage, at Faraday's mask and at its own; Pillow and CPython's zlib, which read every PNG Faraday writes; zbar (`zbarimg`) and zxing-cpp, which decode the QR code in every picture and return its payload as bytes (`qr_decode.py`) | SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and picture of the Codex32QR profile (`tools/vectors/codex32qr/`), each decoded by zbar and zxing-cpp | keys of every length and language, both codes and the words read back; digits and bytes malformed; payloads in each mode at each level and version limit; 48-character codex32 strings in either case as Codex32QR codes and the pictures written of them read back and decoded by zbar and zxing-cpp, other lengths refused; the pictures of a key's SeedQR and CompactSeedQR decoded by both and read by Faraday to the key's words; greyscale pictures and QR pictures of random size and scale read by Pillow and zlib |
| `bbqr` | `faraday_qr::bbqr`, `faraday_qr::specter` (numbered text parts) | Coinkite's BBQr library (`bbqr/`, with a one-table pyqrcode shim checked against `qrcodegen.py`); krux's `qr.py` and `bbqr.py` (`krux/`, unchanged, with three small shims for the firmware's QR, base32 and deflate modules); SeedSigner's own `SpecterWalletQrDecoder` (`seedsigner/`, unchanged) | Coinkite's BBQr test files (`tools/vectors/bbqr/`): the 26 parts of a phone's scan, which read as a `U` file with the text Coinkite's test expects, and its ten PSBTs and transactions, each split in every encoding and read back to the file's bytes | files of every type and size written by krux (at the sizes of QR versions 1 to 20) and split by Coinkite in each encoding, read in any order, with repeats, a part missing, another transfer's part, a conflicting repeat and a part past the count, against Coinkite and krux; what Faraday writes, part for part krux's own writing at the same size, read back by `join_qrs` and by krux; numbered parts written by krux, written again by Faraday, read in any order against SeedSigner's decoder and krux and refused when malformed |
| `ur` | `faraday_codec::ur` (single-part URs, the animation's parts, the decoder), `faraday_crypto::crc32`, `faraday_codec::cbor` and `faraday_qr::registry` | Foundation Devices' foundation-ur-py (`ur/`), URKit ported; CPython's `zlib.crc32`; cbor2 | URKit's single-part and multipart encoder vectors (`tools/vectors/ur/`) | messages as `crypto-psbt` and `bytes` at random fragment lengths, parts byte for byte; sets with fragments dropped, mixes added, shuffled, repeated and upper case, put together by both; URs of other types written, read, changed and cut; CRC-32 of random messages and of the parts; the parts and single-part payloads read by cbor2; registry types built with cbor2 and read; random and mutated CBOR through Faraday's reader beside cbor2 |
| `totp` | `faraday_vault::totp` (`parse`, `Totp::code`, `left` and `period`) | pyotp 2.9 (python3-pyotp): `TOTP.at` and `generate_otp` for the code, `parse_uri` for a URI's key, digits, period and hash | RFC 6238 Appendix B: the codes of the SHA-1, SHA-256 and SHA-512 seeds at its six times, 8 digits | random keys of 1 to 300 bytes in `otpauth://totp/` URIs and, from 10 bytes, as bare keys, digits 6 to 8, periods 1 to 3600 (and past it), the three hashes, times from 0 to 2^64 - 1 with the edges of each period, the code, the seconds left and the period; the URIs written in other spellings and damaged (a character changed, added, removed, a cut, a pair dropped or doubled), read by both; the pinned URIs |
| `vanity` | `faraday_bip::vanity` (the counter, prefixes and their expected work, the search on both dials, for the four script types and a silent payment code) | EntropyLab's counter as `tools/reference/vanity/README.md` quotes it, written out in `area_vanity.py` over python-mnemonic, each candidate's child key derived by btclib on its own curve, and its address by Bitcoin Core v31.1 for the script and address text only (`deriveaddresses` on regtest, from the bare key) and Core's framework, as the `addr` area writes it; a silent payment code by the BIP-352 reference's own bech32m from btclib's keys at BIP-352's paths | EntropyLab's two finds | counters at every width's edges; prefixes of every kind on every network, valid and not; searches of random keys on both dials from random starting counters, with budgets ending before a match and after |
| `qrvault` | `faraday_vault::qr` (the QR vault sealed and opened, with `faraday_pgp::message`) | Sequoia's `sq`, the distribution's package, over Nettle (judges the primitives except Argon2, which it shares with the app's lineage); rPGP (`tools/crosscheck/rpgp/`, a program outside the workspace, on RustCrypto like the app: judges the format only); PyCA cryptography's Argon2id, HKDF-SHA-256 and AES-256-OCB (OpenSSL's), which judge the primitives, under the OpenPGP framing written in the area from RFC 9580 and `docs/VAULT.md` §1.2 (Faraday's second version, not another program's); the QR TOML `qr_toml` written in the area from §2.1, each opened item also read by CPython's `tomllib` | the seven vaults of `tools/vectors/vault/qr/` opened by `sq`, rPGP and the PyCA judge to the item each holds and by Faraday; the four refused messages refused by Faraday and by the PyCA judge; the PyCA judge refuses a vector with a byte changed in each field | an item of every kind of §2.1 (12 and 24 words, a codex32 share, a passphrase, a note with markup, control characters and several scripts, the empty vault) sealed by Faraday and opened by `sq`, rPGP, the PyCA judge and Faraday, and sealed by rPGP and by the PyCA judge (memory 2^13 to 2^16 KiB) and opened by Faraday, each opened item read by `tomllib`; each memory step of §3 (2^16 to 2^20 KiB) sealed by Faraday at scale 1, each message equal byte for byte to the PyCA judge's seal of the same random bytes; items at the size limits (a note of 216 and 217 bytes, a passphrase of 193 and 194, a 24-word seed of the longest words); a passphrase in NFD or NFC either way; a wrong passphrase; a 15-word seed and a document of two keys sealed by rPGP, refused by Faraday; the messages `sq` writes, refused by Faraday and the PyCA judge; two items sealed by rPGP in chunks of 64 bytes (chunk size octet 0), opened by Faraday |
| `kef` | `faraday_kef` (`seal`, `seal_with_key`, `open`, `open_with_key`, `suggest_versions`), `faraday_codec::kef::unwrap` | krux's `kef.py` (`krux/`, unchanged), by the krux developers, on three small shims for the firmware's AES, SHA-256 and PBKDF2, and deflate modules (PyCA cryptography's AES, `hashlib`, zlib at `wbits` -10) | krux's test data for versions 5, 15 and 20 (`tools/vectors/kef/`): a CTR code under `key`, a GCM code under `test key` at 100,000 rounds, and the version 5 code opened by `abc` and by a wrong passphrase that passes its 3-byte check | each of the twelve versions sealed by Faraday and by krux, byte for byte for the ten that do not compress and opened by the other for all twelve, over entropy of 16 and 32 bytes, short and long text, text up to 1,024 bytes, text ending in NUL, text whose hidden check ends in 0x00, repeated blocks, ids of 0, 8, 252 and random lengths, every edge of the count, and wrong IV lengths; codes under a passphrase at 10,000 to 30,000 rounds, one of 200 characters with an id of 252 bytes, and keys of raw bytes (to 200 bytes) krux seals and Faraday opens; codes from a writer that skipped the NUL rules, with 1 to 8 NULs at the end; envelopes cut or changed; the suggested versions for each mode at every length to 1,024 |
| `sqlite` | `faraday_sqlite::Database` (`open`, `tables`, `rows`) | SQLite itself, through CPython's `sqlite3` module (the version is printed at the start of the area), by the SQLite developers | none; the reader's tests read databases SQLite made (`faraday/faraday-sqlite/tests/`) | random one-table databases at each page size from 512 to 65,536, with blobs from empty to several pages (overflow chains of every length), several transactions with rows deleted between them, and a second table in some; the same with one byte changed |
| `core_wallet` | `faraday_core::core_wallet::read_file` (a wallet.dat read whole through `faraday_sqlite`, the records read and checked, the private keys checked in `faraday-keys`) | Bitcoin Core v31.1's `bitcoind` on regtest, offline: it makes each wallet (`createwallet`, `encryptwallet`, `importdescriptors`, `getnewaddress` with labels, `generatetoaddress`), says what it holds (`listdescriptors` and `listdescriptors true`, `getwalletinfo`), opens the passphrase (`walletpassphrase`) and loads or refuses a changed file (`loadwallet`) | the three wallet.dat files Core made (`tools/vectors/core-wallet/`), plain, encrypted and migrated from a legacy wallet by `migratewallet` (made with the older release of `tools/reference/core/legacy.pin`), with what Core said of them | fresh wallets of eight shapes (plain, encrypted, watch-only, blank, with labels, with an imported descriptor, with an imported uncompressed key, with imported multisig and miniscript descriptors of keys with an origin and further hardened steps, a bare master key with hardened steps, and a hash before a key), with and without a transaction, each read with the right passphrase, a wrong one and none; and the same files with a byte flipped in a record, a record dropped or pages cut off |
| `lightning` | `faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read), `faraday_codec::bolt11::decode` (a BOLT #11 invoice read), `faraday_codec::bolt12::decode` (a BOLT #12 offer read), `faraday_keys::hsm_secret` (Core Lightning's node key from its hsm_secret, plain encrypted or the mnemonic form, and from BIP-39 words) | LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one process answering the run's requests (the aezeed also asked of a second build of each side, Faraday's with the aes crate's software round and LND's without assembly); ldk-node's derivation as `tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` over btclib's BIP-32 (`btclib/`, on btclib's own curve); for invoices, the `bolt11` package by lnbits (2.2.0, `tools/reference/bolt11/`: its wheels over the bech32, base58 and bitstring packages, with a stand-in for its libsecp256k1 binding `coincurve` written over btclib's curve), and invoices written in `bolt11_enc.py` from BOLT #11 and signed with btclib; for offers, Core Lightning's own offer decoder, `devtools/bolt12-cli` built from its source release by its own configure and make (`tools/reference/bolt12/`), and offers written in `bolt12_enc.py` from BOLT #12; Core Lightning's code (`tools/reference/cln/`: ccan's HKDF-SHA256 compiled as it is, and its `common/hsm_secret.c` functions written out over the system libsodium, the BIP-39 words and seed of the mnemonic form from python-mnemonic), one process answering the run's requests | LND's five cipher seeds, LND's two node keys, ldk's five node ids (`tools/vectors/aezeed/`); BOLT #11's 31 examples, valid and invalid (`tools/vectors/lightning-bolt11/`); BOLT #12's 53 offer vectors and 12 format strings, valid and invalid (`tools/vectors/lightning-bolt12/`) | random BIP-39 seeds and entropies on every network; every aezeed case again with the aes crate's software round against LND's AEZ without assembly; random words, a wrong version with a right checksum, published seeds under a changed passphrase, a changed enciphered byte or salt with the checksum put right, a changed word, each refused by LND's code; 150 invoices written with every kind of field (amounts in every multiplier, description or its hash, payee key, expiry, final cltv, fallbacks of every version, route hints, features, metadata, fields BOLT #11 does not define) and signed, read by Faraday and by the package field by field, then each with one Bech32 character changed, with one letter in upper case, and with two of these spoiled: a repeated p or x, a wrong-length p, s, h or n, no p, no s, both or neither of d and h, an unknown required feature, a leading zero value in a longer x, c or 9, a leading zero or a fraction of a millisatoshi in the amount, a bad multiplier, an unknown network, a high-S signature, a field cut short, a text over 1023 characters, a description that is not UTF-8; for Core Lightning, random plain and encrypted hsm_secrets under random passphrases (opened by Faraday and by libsodium, node ids compared), a secret sealed by libsodium and opened by Faraday and by Core Lightning's functions, a changed byte, a wrong passphrase and a wrong length each refused by both, random mnemonic-form files with and without a passphrase (node ids compared; a wrong passphrase, a passphrase the file does not have and broken words each refused by both), and random BIP-39 words with a passphrase; 250 rounds of BOLT #12 offers, each an offer written with every kind of field (chains, metadata, a currency or an amount in millisatoshis, description, features, expiry, blinded paths, issuer, quantity, issuer id, unknown odd fields; `+` joins, upper case and a `lightning:` prefix) read by Faraday and by Core Lightning's decoder field by field and by the writer's own record, then one with a Bech32 character changed, one in mixed case, and three of thirty-eight spoils: types out of order or repeated, a field cut short or longer than the stream, a BigSize, truncated integer or feature field not minimal, a type outside the offer's ranges or an unknown even one, a text that is not BOLT #1's, an amount without a description, of zero, or a currency without an amount, no issuer id and no path, a path of no hops or a bad point, chains empty or of a wrong length, padding of more than four bits, a bad `+`, a wrong prefix, characters outside Bech32, and F232's stricter cases (a text over 4,296 characters, a non-zero padding bit, a chain twice or none known, a currency not in ISO 4217, an amount above 21 million bitcoin, an expiry past the year 9999, a first node byte that is no key, an unassigned character) |
| `ots` | `faraday_ots` (`Proof::parse`, `Proof::verify`, `Header::parse`), asked through the three requests `ots.read`, `ots.verify` and `ots.header` | python-opentimestamps' core modules, vendored unchanged in `tools/reference/opentimestamps/`, by the OpenTimestamps developers, on three small shims for what it imports and the repository does not vendor (Keccak-256, RIPEMD-160 over Bitcoin Core's pure-Python hash, and four names of python-bitcoinlib that only its printing routine uses); block headers are read by the area's own transcription of Bitcoin Core's `SetCompact` and proof-of-work test | the OpenTimestamps client's example proofs (all 22, three of them invalid) and the headers of the six blocks they name, each proof walked against each header | proofs built through the reference's classes with every operation, the file hashed with SHA-256, SHA-1 or RIPEMD-160, side branches ending in pending, Bitcoin, Litecoin, Ethereum and unknown attestations, and chains to 255 operations, walked against a header that commits to the main path and one that does not; merkle trees of up to 40 leaves; proofs with a byte changed, inserted, deleted, cut or added to; every limit either side of its edge; headers over every kind of target and lengths other than 80 |

### How each reference is checked

What each area checks its reference by before any case, and what it
leaves uncompared.

- `musig`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday.
- `sp`: its vendored files are hashed against `references.sha256` before
  any run, and a published vector's answer is required of the reference
  as well as of Faraday.
- `sps`: its vendored files are hashed against `references.sha256`
  before any run, the validator is required to give each of BIP-375's 43
  vectors its published verdict and the reference to give each of
  BIP-374's rows its published answer.
- `bip85`: their vendored files and wheels are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of each reference as well as of Faraday. Not compared: the
  BIP's reference is not asked the passwords, which it does not have,
  nor hex bytes of a length outside the BIP's range, which it does not
  refuse: btclib alone answers those.
- `slip39`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday, and of bip39_slip39 too; all 45 of
  its own vectors are run.
- `codex32`: its vendored files are hashed against `references.sha256`
  before any run (the Rust programs' sources included, built offline by
  `run.sh` with the toolchain on the PATH), and a published vector's
  answer is required of the reference as well as of Faraday;
  rust-codex32 and python-codex32 must give the same answer where both
  are asked; two rules BIP 93 states that neither enforces (the master
  seed's sizes, the order shares are handed out) are written in the area
  from the BIP's text, and the worksheet of 127 characters from BIP 93's
  polymod alone.
- `codex32-correct`: its vendored files are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of the reference as well as of Faraday; it is asked with the
  bound BIP 93 states, written in the area from that sentence and
  nothing else.
- `xor`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; the count of parts (2 to 4) and the
  five BIP-39 lengths are Faraday's rule, written in the area in front
  of Coldcard's `xor`, which asserts only equal lengths.
- `hash`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; the order `read` tries the spellings
  in, the trimming of the text, hex as an even number of hex digits,
  plain Base58 (which Core's framework does not read; the area's own few
  lines) and which prefixes make a segwit address are Faraday's rules,
  written in the area in front of the reference. Not compared: the file
  hash is `file_menu::sha256` over a whole byte slice (it reads in no
  buffer, so no read size is probed); the Files screen's reading of a
  file from a stick is the files areas'.
- `vault`: `check_primitives` runs before anything else: argon2-cffi and
  PyCA (44 or later over OpenSSL 3.2 or later, or the area stops)
  against `tools/vectors/kdf/`'s known answer; pykeepass and
  `keepassxc-cli` are the distribution's packages and the area stops and
  names a missing one, so a run without it fails; their versions are
  written into the stamp; the area stops where Python's Unicode is newer
  than `unicode-normalization`'s. Not compared: keepass-rs does not
  count for the primitives and takes no part in the NFD or
  wrong-passphrase cases of files it did not read; the QR vault and
  OpenPGP are other areas.
- `bip39`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; python-mnemonic's own
  `to_hd_master_key` is checked against the seed's master key, as a
  second BIP-32 master; its wheel is hashed too, and a published
  vector's answer is required of bip39_slip39 as well.
- `bip32`: the wheels and the vendored Go tree are hashed against
  `references.sha256` before any run; `check_driver` runs btclib against
  BIP-32's vectors 1 to 5 and the invalid keys of vector 5 before
  anything else, and every vector is asked of `hdkeychain` too.
- `addr`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; every descriptor checksum is also
  asked of BIP 380's own code, as the BIP prints it, and of btclib's;
  Core's release tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked.
- `tx`: its vendored files and the btclib wheels are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of the reference as well as of Faraday; each published
  signature hash is checked against the framework first.
- `psbt`: Core's release tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked; HWI's vendored files and btclib's wheels are hashed
  against `references.sha256` before any run; a published vector's
  answer is required of the side that decides. Not compared: whether a
  `PSBT_GLOBAL_VERSION` of 0 is written: Core reads a version 0 PSBT
  alike with it or without it, and Faraday writes it back as read.
- `sign`: Core is the pinned release, hashed against its pin before
  every run; the vendored files are hashed against `references.sha256`;
  a published vector's answer is required of Core as well as of Faraday;
  every ECDSA and BIP-340 signature Core makes is checked to verify
  under the hash the area computes before that hash is used for
  Faraday's deterministic nonces.
- `sign-check`: its vendored files and the btclib wheels are hashed
  against `references.sha256` before any run; Core's release tarball is
  checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked; the self-test makes Faraday's verdict wrong and its
  report of a nonce wrong, and each is caught. Not compared: A PSBT with
  no previous output for an input (Faraday leaves the signature
  unchecked); signatures under a `pkh` leaf or a script that names no
  key; MuSig2 partial signatures (area `musig`); the order of the
  signatures within a reuse report.
- `psbt-review`: Core's release tarball is checked against its pinned
  SHA-256 (`tools/reference/core/release.pin`) before every run, and the
  node's version is asked; every PSBT is one Core made and read, and
  every change decision is btclib's whole-script comparison at the chain
  and index the output states, which Core's own wallet must find at the
  same place. Not compared: Faraday's other warnings (high fee, dust,
  address reuse, sighash, network, locktime) and thresholds, which are
  Faraday's own rules; the estimated virtual size and fee rate (Core's
  estimate needs signing data the PSBT does not carry).
- `import`: Core's release tarball against its pinned SHA-256 before
  every run; btclib's wheels against the hashes btclib-wallet's lock
  file names (`references.sha256`), with libsecp256k1 off; BIP 129's own
  records and the real files of Sparrow, BlueWallet (Nunchuk's,
  Coldcard's and Unchained's among them), Coldcard, Caravan and Liana,
  judged in the prelude. Not compared: the wallet's name a file carries;
  Electrum's encrypted files and BSMS's encrypted records, which Faraday
  refuses by name; a silent payments record; the kind of a refusal.
- `taproot`: its vendored files, the btclib wheels and the vendored Go
  tree are hashed against `references.sha256` before any run, and a
  published vector's answer is required of the reference as well as of
  Faraday; each of BIP-341's wallet vectors is checked against the
  reference first.
- `compile`: its vendored files are hashed against `references.sha256`
  before any run; Core's release tarball is checked against its pinned
  SHA-256 (`tools/reference/core/release.pin`) before every run, and the
  node's version is asked; the self-test makes Faraday's descriptor
  wrong and the lift wrong, and each is caught. Not compared: The script
  itself: no other compiler makes rust-miniscript's choice, so only that
  it is valid, sane and means the policy is judged; hash locks, locks in
  seconds and the time flag of `older`, which the corpus does not
  generate; a policy that names a key by a fingerprint token of a loaded
  key; policies of more than six keys.
- `miniscript`: its vendored files are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of the reference as well as of Faraday; btclib reads Core's
  97 miniscripts, in both contexts, before any random wallet (valid
  exactly when Core says, its properties, sane, script); Core's release
  tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked. Not compared: The witness bytes of a satisfaction
  (btclib assembles one and area `sign` compares Faraday's finalizer); a
  wallet with more than 64 ways to spend, for which Faraday states none;
  hash locks and locks in seconds, which the corpus does not generate;
  and a wallet btclib cannot read (none yet).
- `msg`: the wheels and the Go tree are hashed against
  `references.sha256` before any run, Core's release against its pin;
  BIP-322's basic and generated vectors and btcd's negative vectors are
  required of both BIP-322 references, each in the state the vector
  names, and Trezor's and Electrum's BIP-137 signatures of btclib. Not
  compared: a taproot signature's bytes (BIP-340's auxiliary randomness
  is the signer's: Faraday's is verified by both BIP-322 references, and
  btcd's by Faraday); the kind of a refusal (Malformed, Invalid,
  Unsupported…), Faraday's own words, checked by its tests; Core's
  reading of a BIP-137 header outside 27 to 42.
- `own-randomness`: the second implementation is written in the area
  from `docs/OWN-RANDOMNESS.md` §2.3 over `hashlib`, and the known
  answers of `faraday/faraday-core/tests/all/own_randomness.rs` are
  required of it as well as of Faraday.
- `entropy`: the hashed dice procedure is Coldcard's own scripts run,
  their files hashed against `references.sha256` before any run;
  SeedSigner's and embit's files are hashed the same way and their
  functions run; the rest is written in the area over `hashlib` from the
  statements of `tools/reference/entropy/README.md` and
  `tools/reference/dice/README.md`, and the published dice answers are
  required of them as well as of Faraday.
- `roller`: the second implementation is written in the area from
  `docs/CODEX32.md` §5.5 and §5.6 and the booklet's worksheet, and the
  known answers, the byte distribution and every byte as a face of every
  die are required of it as well as of Faraday.
- `seedqr`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; SeedSigner's reader is looser than
  Faraday (it takes 49 digits, a sign inside a group, 20 to 28 bytes,
  and does not check the checksum of digits), so Faraday's own rule, the
  lengths and the digits, stands in front of it in `area_seedqr.py` and
  the checksum is embit's; SeedSigner runs in English for every
  language, the indices being the same; the profile's own pictures are
  read from their pixels and must be Faraday's code module for module;
  segno is asked for Faraday's version, level and mode and its mask
  forced, and its mask is read back from it, its padding quirk being
  modelled in Nayuki's code; rqrr reads Faraday's and segno's codes; the
  self-test makes each decoder's answer wrong (a payload's last byte
  changed) and must catch it. Not compared: an empty payload, which
  segno refuses.
- `bbqr`: `check_capacities` and `check_krux_capacities` run first, the
  pyqrcode shim's table of version capacities and krux's two tables
  against `qrcodegen.py`'s; the vendored files are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of the reference as well as of Faraday; where Faraday refuses
  what SeedSigner's decoder files or misfiles (a part number of 0 or
  past the count, a repeat with other text, a header in upper case or
  missing, a count over 1,000) the refusal is written in the area, in
  front of the decoder. Not compared: krux's writing of file types
  Faraday does not write (`C`, `X`, `B`, `R`, `S`, `E` are refused by
  both and compared as refusals); a set over 1,295 parts, which krux
  cannot number; the old cases of Faraday's writing at 1, 5, 7 and 40
  bytes a part, gone because writing is now sized by QR version, the
  size krux chooses.
- `ur`: its vendored files are hashed against `references.sha256` before
  any run, and a published vector's answer is required of the reference
  as well as of Faraday; foundation-ur-py is itself a port of URKit
  checked against URKit's own tests, whose vectors the area also runs.
- `totp`: RFC 6238 Appendix B's codes are required of pyotp as well as
  of Faraday, and each pinned URI's answer from each side is stated in
  the area. Not compared: `from_fields` (KeePass 2's `TimeOtp-*` fields
  and KeePassXC's `TOTP Seed` and `TOTP Settings`) and `normalise` (what
  the entry form writes into field 6), which no pyotp reader covers: the
  crate's tests hold them; `grouped`, the display grouping of a code.
- `vanity`: the search is written in the area from the README's
  quotation of EntropyLab's counter, and EntropyLab's two finds are
  required of it as well as of Faraday.
- `qrvault`: `sq` is the distribution's package, built over Nettle (the
  run stops otherwise, `judges.py`), its version written into the stamp;
  rPGP is built from the crates.io release
  `tools/crosscheck/rpgp/Cargo.lock` names; PyCA must be 44 or later
  over OpenSSL 3.2 or later or the area stops, and its Argon2id,
  HKDF-SHA-256 and AES-OCB are checked against the repository's Argon2id
  known answer, RFC 5869 test case 1 and RFC 7253's first vector first;
  `sq` and rPGP open the repository's vault vectors, and so does the
  PyCA judge, which reads strictly and refuses anything but a QR vault.
  Not compared: OpenPGP.js, which `docs/VAULT.md` §10 names, is not run;
  `sq` does not seal a QR vault (below); rPGP judges the format only.
  §1.2's wider reading, which only Faraday's reader is held to, is not
  compared with a judge: any literal date, padding packets after the
  SEIPD packet and the literal data, and every length form RFC 9580
  allows; of the chunk size octets only 0 (rPGP seals several chunks,
  Faraday opens them) and 10 are run.
- `kef`: the vendored files are hashed against `references.sha256`
  before any run; krux's own vectors (`tools/vectors/kef/`) are required
  of the reference as well as of Faraday; every plaintext Faraday seals
  is opened by krux's `decrypt`, and every plaintext krux seals by
  Faraday, to the plaintext that went in. Not compared: the text forms
  and the classifier (krux's `kef.py` holds neither), the Reveal variant
  and the worker; whether Faraday notes that the check passed only after
  putting NULs back; a key entered as hex that Faraday writes under (it
  refuses a passphrase that is not printable ASCII), so keys of raw
  bytes are compared only when Faraday opens.
- `sqlite`: the rows SQLite returns for the bytes are the answer, with
  the table names of `sqlite_schema`; the databases are made by SQLite,
  and the cases that SQLite reads and Faraday refuses are asked of
  Faraday alone. Not compared: none; index pages are not read by Faraday
  and the rows are SQLite's answer.
- `core_wallet`: Core's release tarball is checked against its pinned
  SHA-256 (`tools/reference/core/release.pin`) before every run and the
  node's version is asked; every wallet.dat is one Core wrote and every
  answer is one Core gave of the same wallet. Not compared: labels and
  the block locator (counted by Faraday; Core's address book is asked
  only for a purpose record Faraday refuses), the records Faraday counts
  and does not read, and the transactions themselves.
- `lightning`: LND's code reads the aezeed (wordlist, version, CRC-32C,
  scrypt, AEZ) and derives the node key, and its published answers
  (`tools/vectors/aezeed/`) are required of it as well as of Faraday;
  the Go program is built offline from the vendored tree
  (`GOFLAGS=-mod=vendor`, `GOTOOLCHAIN=local`) and its version is
  written into the stamp; the area stops when the program is missing;
  the software-round pass runs only on programs `soft.py` has checked to
  be soft (the tags `noasm` and `purego`; no AES-NI instruction in
  Faraday's command) and the command says it was built soft; ldk-node's
  derivation is written in the area from the sources the README quotes,
  and ldk's published answers are required of it as well as of Faraday;
  the `bolt11` package's wheels and its stand-in are hashed against
  `references.sha256` before any run, and each of BOLT #11's 31
  published examples is required to be read by Faraday as the BOLT says
  (a valid one read, an invalid one refused), the package reading the
  valid ones to the same fields; Core Lightning's HKDF is its compiled
  code, its salt, limits and calls are those of `hsm_secret.c`,
  libsodium itself seals and opens the file, and Core Lightning's two
  published node ids for words are required of it as well as of Faraday;
  the area stops when the program is missing and the stamp records
  libsodium's version; Core Lightning's release archive is hashed
  against `tools/reference/bolt12/release.pin` by the fetch and again by
  every build, the area stops when `bolt12-cli` is missing, and each of
  the BOLT #12 vectors is required to be read by Faraday as the vector
  says (a valid one read, an invalid one refused) and read by Core
  Lightning's decoder to the same fields where Faraday reads it. Not
  compared: ldk-node's node key is compared with a derivation written
  here, not with ldk's code (Rust, not vendored): only ldk's five
  published answers are outside it; LND's key path numbers are written
  in the Go program from `keychain/derivation.go`, as LND derives
  through btcwallet, which is not vendored; the `bolt11` package is not
  asked the fields it does not read (fields BOLT #11 does not define, a
  fallback of a witness version above 16, a second or later fallback),
  which are compared with the writer's own list; its signature check
  runs on btclib's curve, not libsecp256k1's, and refuses a high-S
  signature as libsecp256k1 does; `hsm_secret.c` itself is not compiled
  (it needs tal and libwally): its functions are written out in
  `main.c`, and for the mnemonic form libwally's two answers, whether
  the words validate and their BIP-39 seed, are python-mnemonic's, which
  takes 12 to 24 words and normalises the words and passphrase (NFKD)
  where libwally takes 27 and 30 words too and normalises nothing, so
  the area judges only English 12 to 24-word ASCII cases and Faraday
  refuses 27 and 30 words (F228); a NUL in the mnemonic form's words is
  not compared (Core Lightning reads the text up to it, Faraday refuses
  the file); the words' derivation (`lightning-hsmtool generatehsm`) is
  BIP-39's seed, whose first 32 bytes are taken by both sides here; Core
  Lightning's `bolt12-cli` reads one Bech32 string, so the area takes
  out a `+` between two Bech32 characters and the white space after it,
  and a `lightning:` prefix, before asking it, and a text with a NUL
  byte cannot be given to it (Faraday refuses it); its output is read as
  text (a short channel id as `BxTxO/direction`, chain hashes reversed).
- `ots`: the vendored files are hashed against `references.sha256`
  before any run; the client's example proofs
  (`tools/vectors/opentimestamps/`) are read by both and the four that
  reach a block through SHA-256 and RIPEMD-160 alone are required to be
  committed by both; the shims' hashes are checked against known values
  before the corpus; each limit is built either side of its edge and
  both must agree. Not compared: the reference's printing of a tree and
  its stream readers; the order of a node's entries (the walk is
  compared as a set of paths); whether a header is in the chain;
  calendars, which neither side asks.
<!-- facts:end what-runs -->

<!-- facts:begin known-differences -->
### Known differences

Each is a place where Faraday and the reference differ, or differed once
and now agree. The corpus keeps out of it, or the area checks it to
still hold exactly and counts it in the run's output ("N of them a known
difference"), and says why.

- `sp`
  - BIP-352: Faraday's sender arithmetic is test-only and has no K_max,
    so the vector that exceeds it is checked on the receiving side only.
  - BIP-352's skip of a transaction spending SegWit version 2 or above,
    which the reference and btclib-wallet leave to their caller, is
    written in the area from the BIP's own sentence.
  - BIP-392 publishes no reference code or vectors: its `spscan`
    expression is written and read with the BIP-352 reference's bech32m
    and curve, as BIP-392 states the payload, and the `sp()` around it
    is read in the area from the BIP's text. Faraday's own, expected
    from its rule: an `sp()` with no key origin, which BIP-392 allows,
    is refused, as is an origin other than BIP-352's path.
- `sps`
  - BIP-375: Faraday takes a key a PSBT states for a P2PKH, P2SH-P2WPKH
    or P2WPKH input only where the input's script commits to it (F89);
    the validator takes the first key stated, so a PSBT it accepts with
    an unbound key is refused by Faraday.
  - BIP-375: Faraday accepts the scripts of a scan key's outputs
    numbered in sorted order of their spend keys, as the BIP's text
    says, and in output order, as its vectors do; the validator accepts
    output order only.
  - BIP-375: Faraday refuses an input whose redeem script is not the one
    its P2SH script hashes, and one whose previous transaction is not
    the one spent; the validator reads both fields without checking
    them.
  - BIP-375: btclib-wallet takes a taproot input's NUMS internal key as
    a reason to skip the input only against a proof (a merkle root or a
    control block that tweaks the NUMS point to the output key), and
    counts the input otherwise; Faraday and the validator skip on the
    claim. A PSBT with an unproved claim is not asked of btclib-wallet;
    the run prints how many PSBTs it judged and how many it was not
    asked. A proved claim only in a control block, with no internal key
    set, is not detected: btclib-wallet would skip the input, Faraday
    would count it, and the run would stop on it. The corpus also makes
    inputs whose claim is proved, which all three skip.
- `slip39`
  - SLIP-39: Faraday refuses a group threshold of 1 over several groups;
    python-shamir-mnemonic makes the shares. The area checks that this
    still holds, so a change either way is seen.
- `codex32`
  - Codex32: Faraday recovers only from exactly `k` shares without the
    secret, and derives a share only from exactly `k` strings at an
    unused index; the reference also takes more strings or a used index.
  - The codex32 seed sizes, which the references leave to their caller,
    are written in the area from the BIP's own sentence. Volvelle's
    sheet works the checksum itself, so a string of 48 or 74 characters
    whose checksum is not the one the sheet works is judged by that
    alone (`holds` false), not by its rows.
- `codex32-correct`
  - Codex32 correction: python-codex32 reads a character outside the
    bech32 alphabet as unreadable, and fills more than 8 unreadable
    boxes that are not one run when its equations allow; Faraday refuses
    both (`docs/CODEX32.md` §8.4). The corpus damages strings with
    bech32 characters and `?` only.
- `hash`
  - bech32 groups: a string that is not a segwit address of bc, tb or
    bcrt is read as bytes, all its data groups together, and the bits
    that do not fill a byte are dropped. BIP-173 has a padding rule only
    for a segwit address, so Core's `convertbits`, which applies it to
    any data, refuses some of these strings, and the area reads them as
    Faraday does.
  - bech32 length: Core's decoder refuses more than 90 characters
    (BIP-173); Faraday accepts any length the checksum covers. The area
    reads longer strings the same way.
- `vault`
  - pykeepass strips the characters XML 1.0 cannot hold from a protected
    value after decrypting it; Faraday keeps a Password's characters as
    they are put in (and leaves them out of a Title or Notes when it
    writes). The `export` loop expects pykeepass's reading of a Password
    and Faraday's own as put in, and counts the passwords that differ.
  - pykeepass opens a file whose major version is neither 3 nor 4
    without reading past it, deriving a key or checking a hash; KeePass
    refuses such a file, and the area counts a file as opened only when
    pykeepass read a KDBX 4 body (`readers.open_kdbx`). It also runs the
    key derivation the header states before it checks the header's
    SHA-256, so a changed bit in the Argon2 iterations (2^44 passes)
    holds it for good; KeePass and `keepassxc-cli` refuse at once, and
    the area asks pykeepass first with a key given and nothing
    decrypted.
  - The vault's passphrase is put in NFC (§3); KeePass apps do not. A
    vault under an NFC passphrase opens in Faraday under its NFD form
    and in neither outside reader. The area counts the NFD tries and
    expects exactly that.
  - Faraday's reader is stricter than the outside readers' in what it
    takes of a file (§4, §7.1): a block stream that ends before its
    empty last block (`keepassxc-cli` reads it), an XML document with an
    element after the root (`keepassxc-cli`), a protected value not
    canonical Base64, nesting past 64, another root element, a DOCTYPE
    and another encoding (pykeepass reads these). `WIDER` names each and
    the reader that opens it; the area expects exactly that set and
    counts those opens as known differences.
  - keepass-rs writes files Faraday refuses (§4): Twofish, AES-KDF and a
    Salsa20 inner stream (`keepass_rs.REFUSED`). The area makes one of
    each, expects Faraday to refuse it and counts the refusals as known
    differences.
  - KeePassXC 2.7.10 (and its development branch, November 2026) fails a
    save of a ChaCha20 file whose plaintext is a multiple of 1024 bytes,
    "Writing the database failed: Cannot process 0 length data.", and
    writes nothing: `SymmetricCipherStream` encrypts each full block of
    1024 bytes as it fills and then the empty rest, which
    `SymmetricCipher::process` refuses (AES-256's last block is padded).
    Each save puts a random value of 128 to 1022 hex digits in the file
    (`KPXC_RANDOM_SLUG`), so it is chance, compressed or not, about one
    save in 450 at most. `readers.Kpxc` makes a command that writes
    again, once, from the file in a new shell, only on exactly that
    message and for a ChaCha20 file left byte for byte as it was; the
    save made again is judged as any other, a second such failure in a
    row fails the run, and each is counted as a known difference. The
    prelude shows the condition (`kpxc_zero_length`): on a file whose
    plaintext but for the random value is known (its ciphertext's length
    less the value pykeepass reads), 1500 saves that cannot reach a
    multiple of 1024 bytes do not fail, of saves that can, one must fail
    within 20,000, and the same file under AES-256 does not fail; when
    KeePassXC no longer fails, the run fails and the handling is to go.
  - keepass-rs decodes a custom data item's value as Base64 when it can,
    and the program encodes the bytes again (`keepass-rs/src/main.rs`).
    A value that is valid Base64 but not canonical (padding or trailing
    bits the encoder would not write) comes back as another text, so
    keepass-rs's reading of such a value differs from the file's and
    from pykeepass's. The area compares custom data as they are, so such
    a value is a known difference of keepass-rs, not a disagreement
    about the format.
- `bip39`
  - Both read a BIP-39 sentence in NFKD, so composed words and the
    Japanese vectors as published parse on both sides (owner,
    2026-10-23).
  - A passphrase typed in Faraday is printable ASCII, at most 256 bytes;
    python-mnemonic takes any text. Faraday's vector tests and start-up
    self-test give a non-ASCII passphrase as its NFKD bytes, and the
    area checks that a typed one, or one past 256 bytes, is still
    refused.
- `bip32`
  - `hdkeychain` reads a key of any version, a key whose version is a
    private one over a public key's data (or the reverse), and a depth-0
    key with a parent fingerprint or an index. Faraday refuses all
    three, as btclib refuses the last; where one applies and btclib
    refuses, `hdkeychain` is not asked (`uncovered` in `area_bip32.py`),
    so Faraday's strict reader is not retired
- `addr`
  - A descriptor with a second `#` in its body: Faraday and btclib
    refuse it, as Bitcoin Core's parser does, where BIP 380's code and
    Core's framework check the last nine characters; its checksum is
    asked of btclib only. btclib reads descriptors Faraday takes for no
    account (`pk`, `combo`, `multi`, `addr`, `raw`) and other chains
    than `<0;1>`; the area asks it for the four account descriptors with
    both chains only.
- `tx`
  - Decode refuses a transaction whose outputs total over 21,000,000
    BTC, as Core does not in `decoderawtransaction`; the corpus asks
    Decode only of transactions within that total.
  - Legacy sighash: `bitcoin` hashes the script code as given, Core and
    btclib remove every OP_CODESEPARATOR first. Faraday's legacy spends
    (P2PKH, P2SH multisig) have none.
  - Core's `decoderawtransaction` reads bytes whose marker `00 01` it
    takes as a count of no inputs and one output, a legacy transaction
    of no inputs; Faraday refuses a transaction with no inputs, and the
    area counts a case where btclib reads inputs of such bytes as this
    difference.
  - BIP-341's refusals (SIGHASH_SINGLE without an output, a hash type it
    does not list) are btclib's: the framework hashes those anyway and
    is not asked.
- `psbt`
  - Version 0 made version 2: HWI's `convert_to_v2` leaves out the
    `PSBT_OUT_SCRIPT` of an output whose script is empty, which BIP-370
    requires; Faraday and btclib write it, and the rest of the PSBT must
    be the same.
  - Bitcoin Core reads a version 0 PSBT with a version 2 field as valid,
    the field as unknown; Faraday refuses it, as BIP-370 does, and so do
    HWI, which decides it, and btclib.
  - A taproot signature of 65 bytes whose sighash byte is 0, which
    neither Core nor HWI refuses, and bytes after a version 2 PSBT's
    last map or a stream that ends inside one, which HWI reads: Faraday
    refuses each, as BIP-341 and BIP-174 do, and so does btclib, which
    decides them (the stream ending inside a map checked, as the
    specification marked it not checked).
  - PSBT version 2 (`faraday_psbt::v2`): a BIP-370 key with key data is
    an unknown key; BIP-174's and BIP-371's fields inside are checked
    only when the PSBT is bridged to version 0, except a taproot
    signature whose sighash byte is 0, refused on reading. HWI checks
    them on reading; the area requires HWI and btclib to read the PSBT
    without those pairs, to Faraday's locktime and transaction. Where
    HWI reads a value shorter than its length says and makes a version 0
    PSBT of it, which Faraday does not, Core must refuse the version 0
    PSBT of the PSBT's own pairs, and btclib refuses the PSBT.
  - btclib reads some PSBTs that Core and Faraday refuse (a key with key
    data on a field that takes none, for one): btclib is not asked to
    refuse what Core refuses, only what Core and HWI read and Faraday
    refuses.
  - btclib refuses what the BIPs allow: an ECDSA signature that is not
    strict DER with a low S and an r that is a point's x, a sighash type
    of the seven it defines, a BIP-375 field in a version 0 global map,
    a previous transaction and a witness UTXO that disagree. About one
    version 0 PSBT in eight carries such a field, and btclib is not
    asked of it or of what is made from it. Pairs btclib does not write
    back as read (an empty final witness) and what it writes in their
    place are left out of the conversions it is compared on, and leaf
    hashes are compared sorted, as it writes them.
- `sign`
  - PSBT signing: Faraday refuses to sign a SegWit v0 input with no
    previous transaction in a transaction of more than one input; Core
    signs it, and BIP-174's own signer vector is such a PSBT. The
    refusal is ours, checked and counted. Faraday holds a finalized
    input to the same rule and refuses the PSBT when such an input is
    final, except a taproot one, whose final witness is the
    coordinator's.
  - PSBT signing, wallets: Faraday signs an input spent from a script of
    several keys (a `multi` of two or more, a miniscript naming two or
    more, a taproot script path whose carried leaves name two or more
    with its internal key, or are not its whole tree) only for a
    registered wallet that holds one of its keys and whose script it is;
    Core signs any. The area refuses, by a rule of its own, where no
    wallet the request registers holds a held key and gives the input,
    and counts it only where Faraday's refusal is that the wallet is not
    registered. A miniscript or a taproot tree of one key is signed only
    through such a wallet, and is otherwise left unsigned while the rest
    is signed (F141); the area takes Core's signatures off it. Half the
    corpus's `multi`, miniscript and taproot-tree inputs come with their
    wallet registered, so their signatures are still compared. A
    registered tree's every leaf has its wait met, since Faraday refuses
    a path that can never be valid, which the area does not work out.
  - PSBT signing: Faraday refuses a PSBT any input of which asks for a
    `NONE` or `SINGLE` sighash, and one that none of the held keys can
    sign: dangers no hold accepts (F136). Core signs the first and
    answers the second with nothing signed. The area expects both
    refusals by a rule of its own, counting a held key as taking part as
    Faraday's inspection does (a miniscript or a taproot tree only
    through a wallet in use), checks that Faraday's refusal names the
    danger, and counts them.
  - Taproot `pkh` leaves: Faraday signs a taproot leaf that checks its
    key by hash but does not finalize it: `miniscript` 12's finalizer
    looks the key up among the ECDSA origins and panics, and would push
    33 bytes where the leaf hashed 32. Core finalizes such a leaf; the
    area takes the signatures under it off before Core finalizes, and
    counts each input Core would otherwise have finalized another way.
  - btclib's finalizer writes no taproot leaf that is neither a single
    key nor `multi_a`, and refuses to choose among leaves a PSBT signs:
    an input of another leaf is judged by Core alone (counted as a known
    difference), and btclib is given only the signatures of the leaf
    Faraday spent. It writes a plain multisig from the smallest m
    signatures on hand where Core and Faraday write the first m
    signers': btclib is given only the signatures Faraday's input
    carries, so it judges the script written from them and Core judges
    which were chosen.
- `sign-check`
  - A partial signature that is not a strict DER signature with a
    defined sighash byte (or a taproot signature of 64 bytes, or 65 with
    a sighash byte that is defined and not 0x00): Bitcoin Core reads the
    PSBT and leaves the input unfinalized, Faraday refuses the PSBT or
    does not call the signature valid. Counted.
  - A final input whose script Faraday does not run (a hash time lock,
    which is no miniscript: a wrong preimage, the key of the other
    branch, a sequence below the lock, and its true spends; a P2SH
    puzzle; a segwit `pk` of an uncompressed key, which Faraday's segwit
    reading refuses, though a segwit multisig of such keys is read by
    hand and answers `ok`; a bare P2PK of a key off the curve; a taproot
    leaf of a version other than 0xc0, a taproot hash lock, a taproot
    leaf with OP_CODESEPARATOR or OP_DROP): Faraday answers `unjudged`,
    whatever the engine decides. Such a script reads not sound in the
    nonce check and is not refused, and only these rows may answer so: a
    miniscript or a multisig answering `unjudged` fails the area.
    Counted.
  - A legacy (bare or P2SH) multisig with a junk element below the
    dummy: consensus leaves it on the stack and accepts, Faraday refuses
    it. No wallet writes one, and the refusal stays. Counted.
  - What no wallet writes and no node relays, which consensus accepts: a
    P2SH redeem script or signature pushed by PUSHDATA1 where a direct
    push fits; a P2SH `or_i` selector `01 01` (a push of one byte, not
    OP_1); a P2WSH `or_i` selector `[2]` (not `[1]` or empty); an `or_d`
    dissatisfied by a wrong DER signature, not an empty element; an
    element left below a P2SH `pk`. Faraday's reading of the final
    fields refuses each, and the refusal stays. Counted.
  - A final input with a high S, or with a sighash byte no standard
    defines (0x00, 0x04, 0x05, 0x84), in a signature the script engine
    accepts under consensus's rules: Faraday refuses it (a mismatch, or
    an invalid signature) and consensus takes it. Faraday keeps to the
    standard here on purpose: a node relays neither, so a transaction
    carrying one does not reach a miner. Counted.
- `psbt-review`
  - Change of a script of several keys, a miniscript or a taproot tree:
    Faraday verifies it only for a registered wallet that holds a loaded
    key, and counts it as paid to others otherwise, which btclib's
    descriptor wallet has no notion of. The area expects the same
    refusal, and does not compare the spoof warning for an unregistered
    wallet (Faraday's reading of such a claim depends on the account
    path it states).
- `import`
  - Still ours, expected from Faraday's rule: a SLIP-132 version that
    names another script than the file's (F155), a BSMS paths line other
    than /0/*,/1/* or No path restrictions, which Faraday reads as the
    two chains (F64), a BSMS descriptor that is not a template, a key
    record whose description is over eighty characters or whose token is
    not BIP 129's; and which account of a Coldcard export, which pair of
    a `listdescriptors`, and what each field of every format means, the
    area's reading of each format's documentation; a checksum in Specter
    DIY's `addwallet`, which Specter DIY drops unread and Faraday
    checks, is refused when Core's code calls it wrong; and the reading
    of each format's documentation.
  - Real files both sides refuse, each by a named rule: a Caravan file
    or an Electrum multisig file whose keys lack a path or a
    fingerprint, a Caravan quorum that is not the keys listed, a
    multisig setup file with no policy, a key missing or no format line
    (Coldcard's own P2SH setup files omit Format, which Faraday
    refuses), a BSMS record with a paths line other than /0/*,/1/*, a
    derivation other than /** or <0;1>, a missing line, a first address
    that is not Core's or not an address, a Coldcard cosigner key file,
    an Unchained key file and a Coldcard stored setting (a key, not a
    wallet), and a UR text (area `ur`). Left out and named: a silent
    payments record (area `sp`) and a PSBT or transaction file (areas
    `psbt`, `tx`).
  - Formats with no published file in a test suite: Nunchuk's own files
    (only BlueWallet's copies of them), Specter DIY's beyond the one
    Sparrow keeps, Keystone's beyond Sparrow's, and Core's
    `listdescriptors` beyond the one in the interop vectors, whose
    source is not recorded; Electrum's published test wallets carry a
    seed and are not taken. Those formats stay the area's own reading
    (Faraday's second version), as above.
  - A key record's signature in any of BIP-137's header forms is
    accepted when it recovers the record's key: btclib is asked with the
    address that form names (Faraday reads no address from the record).
- `taproot`
  - btclib and btcd hash a leaf of version 0x50 like any other, where
    BIP-341's text and Faraday refuse it (the annex tag): a control
    block of that version is judged by the area alone, and neither is
    asked
- `compile`
  - A single key as a `tr()` policy: rust-miniscript's `compile_tr`
    takes the key as the internal key, is left with no leaf and refuses
    it (`Empty Miniscript compilation`); Faraday refuses it too, in its
    own words, and the area counts the refusal. python-bip380 writes a
    `v:` over a `multi_a`, at any depth, as `OP_NUMEQUAL OP_VERIFY`
    where BIP 379 and Core write `OP_NUMEQUALVERIFY`, so such a leaf
    must equal btclib's script but for that (counted); the output key,
    which commits to the leaf, is Core's and btclib's. python-bip380
    types `and_b`'s `e` as `f_X s_X f_Y s_Y` where BIP 379 has `e_X e_Y
    s_X s_Y`, so it calls `thresh(1,X,a:and_b(pk(A),s:pk(B)))` malleable
    where BIP 379, Core and btclib call it sane. Its `thresh` leaves the
    first part out of `e`, `s` and `m`, so it never calls a `thresh`
    expressive and calls an `or_c` over one malleable, where the same
    three call it sane. Its `0` is not `s` where BIP 379's is, so `l:X`
    and `u:X` lose X's `s`. Such a miniscript is asked of python-bip380
    again with those three rules as BIP 379's, must be sane then and is
    counted; any other reason python-bip380 gives stops the area. Its
    `thresh` of 1 never sets its lock fields, so reading one inside a
    `thresh` of 2 or more raises a TypeError, where BIP 379, Core and
    btclib read it. Such a miniscript is read again with those fields
    set from its parts, as Core keeps them, and is counted; any other
    error stops the area.
- `miniscript`
  - Miniscript: Faraday reads `sortedmulti_a` only as a `tr()`'s one
    leaf; BIP 387 allows it deeper in a tree, and Core reads it there.
    Two of Bitcoin Core's sane miniscripts are refused, as `miniscript`
    refuses them: `or_b(0,a:0)`, which has no satisfaction, and a `d:`
    used as a unit under Tapscript. Each is checked and counted; btclib
    is not asked the deeper `sortedmulti_a`. btclib says nothing of
    "needs a signature" for a malleable miniscript (BIP 379 does not),
    so that property is compared where Core calls the miniscript
    non-malleable.
- `msg`
  - BIP-137: Faraday reads the address form from the address rather than
    the header's range and tries a P2PKH address in both key encodings
    (`faraday_psbt::message`), as Electrum and Sparrow do; btclib holds
    the ranges (with Electrum's for SegWit) and Core reads the key
    encoding from the header. Counted where btclib accepts the signature
    with the header the address's form names, or Core with the other key
    encoding.
  - BIP-322: Faraday refuses the full form and proof of funds, which
    both references make and accept (for P2PKH and P2SH-P2WPKH addresses
    they sign in the full form); counted where both accept and Faraday
    refuses the format.
  - BIP-322: Faraday checks simple signatures for P2WPKH and P2TR
    addresses only; a P2WSH script both references accept is refused as
    an unsupported address, and counted.
  - Between the references: btclib's interpreter holds witness v0
    scripts alone to MINIMALIF, so it accepts btcd's two negative
    vectors of a legacy P2SH script, in the full form, whose `OP_IF`
    takes a non-minimal true; btcd must call them invalid and Faraday,
    which reads no full form, refuse them, and each is counted.
- `entropy`
  - SeedSigner's `generate_mnemonic_from_dice` cuts to 16 bytes only at
    exactly 50 rolls and takes every other count as 32 bytes, and its
    `calculate_checksum` completes 12 and 24 words only: SeedSigner
    judges the hashed procedure at 50 and 99 rolls and direct
    selection's last word at 12 and 24 words (at 15, 18 and 21 embit's
    checksum through `generate_mnemonic_from_bytes`), compared as the
    entropy embit reads back from those words, the checksum itself being
    area `bip39`'s; it has no six-as-zero, no cut to 20, 24 or 28 bytes,
    and its dice take no direct selection, which stay the area's own
- `seedqr`
  - SeedQR digits: SeedSigner reads a string with 1 to 3 digits more
    than 48 or 96 (it drops the tail) and one with a sign or a space
    inside a group of four (`int()` takes them); Faraday refuses both (5
    cases, each checked on both sides and counted). SeedSigner takes no
    CompactSeedQR of 20, 24 or 28 bytes either, and strips checksum bits
    for 12 and 24 words only; the corpus keeps Faraday's 15, 18 and 21
    word compact form on the entropy itself.
  - Codex32QR: Faraday picks the mask Nayuki's encoder picks; the
    profile's encoder chose another for 6 of its 22 pictures. For those
    the area swaps the mask of Faraday's code for the picture's with
    `qrcodegen.py`'s masking, requires the picture, and counts it.
  - segno 1.6 writes a zero codeword where the pad codewords begin when
    the data, terminator included, ends on a codeword boundary with room
    left (its `write_padding_bits` adds 8 zero bits where none are due;
    ISO/IEC 18004 7.4.10 adds none): its code is Nayuki's with that
    codeword changed, which rqrr reads, counted as a known difference,
    the padding; Faraday's is the standard's.
  - segno picks a mask by its own penalty rule, and for some payloads
    another than Nayuki's and Faraday's; the area requires that code to
    be Faraday's under the mask segno reports (Nayuki's masking swapped
    to it) and rqrr to read it, and counts each as a known difference,
    the mask choice, which a reader ignores.
- `bbqr`
  - BBQr: Faraday reads the file types `P`, `T`, `U` and `J` and frames
    of at most 4,096 characters; Coinkite's library reads every type
    BBQr defines and a version 40 part of 4,296 characters. Both
    refusals are checked and counted, and the corpus splits at version
    39 at most. Faraday also refuses a set whose parts are not one
    length but the last, which Coinkite never writes.
  - Numbered parts: SeedSigner matches `P` and `OF` in upper case;
    Faraday does not (checked and counted). Faraday also refuses a part
    number of 0 or past the count, a count over 1,000, a repeat with
    other text and a part with no header, which SeedSigner's decoder
    files, fails on or takes whole; those refusals are written in the
    area, in front of the decoder.
  - krux (checked and counted): reads the file types `P`, `T`, `J` and
    `U` only, decodes `J` and `U` as UTF-8 in `2` and `Z` only, stops at
    160 KiB of characters (Coinkite's `signed.txn` is over it, in `H`),
    and can write a last part longer than the others, which Faraday
    refuses; its deflate reads a 1,024-byte window on a device, which
    the shim reads and writes. A set with an empty last part is read by
    Coinkite and krux and refused by Faraday, which refuses any part
    with no data.
- `ur`
  - UR: Faraday has no minimum fragment length, writes a one-fragment
    message as a part like any other, and refuses to animate an empty
    message; foundation-ur-py checks a UR type one character's way (`'a'
    <= type <= 'z'`). The area asks the reference with a minimum of 1,
    makes parts with its fountain encoder, and keeps its types to
    letters, digits and `-` not starting with `z`. Faraday's decoder
    reduces mixes by each other whichever arrived first, and so
    sometimes finishes a set URKit's does not; the area then checks the
    message is the one sent and counts the set.
  - CBOR: Faraday's reader refuses indefinite lengths, floats, simple
    values other than false, true and null, nesting beyond 16 levels, a
    key given twice and a stray break, which cbor2 reads; cbor2 reads
    tags below 300 or from 1000 on as types of its own. The area counts
    each such input as a known difference, and any other difference is a
    failure.
- `totp`
  - Where Faraday reads and pyotp refuses: a scheme, type or `algorithm`
    in other than lower (upper for the algorithm) case; a parameter name
    pyotp does not know, a `counter` on a TOTP, a label issuer that the
    `issuer` parameter does not repeat; Base32 with spaces or hyphens (a
    bare key) or `=` in the wrong place, or a length that Base32 cannot
    have. The area asks both about the URI with that difference taken
    out and requires the same answer, and Faraday's answer to the
    original to be that answer.
  - Where Faraday refuses and pyotp reads: a period over 3600 seconds
    (Faraday's rule), a parameter of the key URI format written twice or
    in a case other than lower or with a percent-escape in its name
    (pyotp keeps the last, or reads the decoded name), a parameter name
    with a tab or line break in it (pyotp strips them), a pair with no
    `=` or no value, a type with no `/` before its `?`, a value written
    percent-encoded (Faraday reads it as it is), a fragment after the
    last value, a bare key of fewer than 16 characters, and
    `encoder=steam`, which pyotp reads as a Steam code of five
    characters and Faraday refuses, making no such code. The area asks
    both about the URI with a period, repeated parameter, name, pair or
    type difference taken out and requires the same answer; the percent
    escapes, the fragment, the short key and Steam's code are pinned one
    URI at a time.
  - Where both read and the keys differ: an escaped `&` or `=` in a
    value, such as an issuer's `a%26secret%3DB`. pyotp unquotes the
    whole URI first and reads a second `secret` from it; Faraday reads
    the value as written, one issuer and the key before it, which is the
    reading the URI's writer meant. Pinned one URI at a time, where the
    pin judges the kind and not the key.
  - HOTP: pyotp reads `otpauth://hotp/` as a HOTP; Faraday refuses it,
    naming it HOTP (`NotTotp::Hotp`) and reading no code.
  - Times: pyotp works from a `datetime`, which ends in year 9999; past
    it the area asks `generate_otp` for the counter, `time // period`.
- `vanity`
  - Each candidate's child key is derived by btclib on its own curve,
    not by Bitcoin Core, which shares libsecp256k1's curve maths with
    Faraday; Core is given the bare key and judges the script and the
    address text only.
- `qrvault`
  - QR vault: `sq` 1.3.1 writes a SKESK v4 and a SEIPD v1 for a password
    under either `--profile`, never the SKESK v6 and SEIPD v2 of RFC
    9580, so `sq` cannot seal a QR vault; rPGP and the PyCA judge seal
    the other direction, and the area asserts that Faraday and the PyCA
    judge refuse what `sq` writes. The PyCA judge's framing is the
    area's own Python; OpenPGP.js is not run.
- `kef`
  - KEF: Faraday refuses an empty plaintext, a compressed version over
    1,024 bytes, an id of 253 bytes or more when reading (krux's
    `unwrap` does not check it), a count over 550,000 and a passphrase
    that is not printable ASCII when it works out the key, and a stream
    that inflates to over 102,400 bytes; krux does none of these. Each
    is asked of krux and counted. Faraday does not suggest a compressed
    version over 1,024 bytes.
  - KEF: a stream that inflates to exactly 102,400 bytes is read by
    Faraday and refused by krux's firmware (its read is capped at the
    limit and raises when it is asked for more before the stream has
    ended); one of 102,401 bytes or more is refused by both. A stream
    whose matches reach back more than 1,024 bytes is read by Faraday
    (any window to 32 KiB) and refused by krux's 1,024-byte window;
    Faraday writes none.
  - KEF: Faraday's `unwrap` refuses at once a payload that holds nothing
    but the IV, the exposed check and the hidden check; krux's `unwrap`
    takes it apart and its decryption then fails. Both end in a code
    that does not open.
  - KEF: version 7 (ECB with deflate) is refused for a repeated block by
    the side whose compressor makes a stream with one; the stream of the
    other has none, and that side's code is opened by both.
  - KEF: the compressed versions are not equal byte for byte, as
    Faraday's deflate and zlib's write different streams. Each opens the
    other's; krux's decryption inflates under zlib at `wbits` -10.
- `sqlite`
  - SQLite reads the write-ahead-log form, auto-vacuum databases and
    UTF-16 databases, and a database with a table of any shape; Faraday
    reads the rollback-journal form with UTF-8 text and a table of two
    blob columns only, and refuses the rest. Each is asked of Faraday
    and counted.
  - SQLite parses the CREATE statements of the schema and refuses a
    database whose text is not valid SQL; Faraday reads a table's name
    and root page from its schema row and does not parse the statement.
    A changed byte in the schema page that SQLite refuses as a malformed
    schema, and Faraday reads, is not a mismatch.
- `core_wallet`
  - Faraday gives each descriptor as Core lists it (the account key from
    the `walletdescriptorlhcache` records, the master key's fingerprint
    and the hardened path as the origin, checksum included), and the
    area compares that whole text, with Core's `listdescriptors` without
    private keys, for every shape.
  - Faraday checks what Core does not look at when it loads a wallet: a
    private key's stored hash and public key, a descriptor's id and
    checksum, and flags and records Core's loader ignores. A changed
    file Core loads and Faraday does not call whole is counted, and must
    name a record and fault of the listed set; nothing is asked of Core
    there beyond that it loads, except for a purpose record Faraday
    cannot read, where Core must list the address's purpose as unknown.
  - Core runs SQLite's integrity check when it loads a wallet and
    refuses a file in which a record's key was changed (the unique index
    beside the table still holds the old key), and refuses one whose
    `walletdescriptorcache` record was dropped; Faraday reads the table
    only, so it reads such a file whole when the changed key is still a
    record it accepts. Core also refuses a wallet whose block locator
    ends in another chain's genesis block, as it attaches the wallet to
    its node's chain; Faraday reads no chain and does not read the
    locator's hashes. Each is asked of Core in the prelude.
  - Faraday refuses a wallet of another network than its mode and a file
    with a journal, log or lock beside it, which Core opens (and
    recovers); the area reads regtest wallets with nothing beside them,
    so neither arises.
- `lightning`
  - BOLT #11 invoices: the package skips a p, h, s or n field of the
    wrong length, reads the first of a repeated p or s, and accepts both
    or neither of a description and its hash, mixed case, an amount
    finer than a millisatoshi and a leading zero value in x, c or 9, all
    of which BOLT #11 makes a refusal or says "should" of and Faraday
    refuses; it also accepts what BOLT #11 leaves open, which Faraday is
    stricter on: a repeated x, c, m, 9, d, h or n, a text over 1023
    characters, an amount with a leading zero, and a required feature
    bit Faraday does not know; the area counts each refusal as known,
    naming its reason
  - BOLT #11 route hints: the package reads a hop's fees and delta as
    signed integers (a fee above 2^31 reads negative); the area reads
    them modulo 2^32 and 2^16 before comparing, and keeps a hint to
    three hops, as the package drops the last hop of a hint of a
    multiple of five
  - BOLT #11 fallbacks: the package reads only the first fallback
    address and writes the others in bech32 for every witness version,
    no bech32m; the area compares Faraday's whole list of fallbacks with
    the writer's f fields of versions 0 to 18, in order, and its fields
    of an unknown version by the writer's list
  - BOLT #11 features: the package keeps one state per feature, so a
    feature with both its required and its supported bit set reads as
    supported only; the writer sets one bit of each pair
  - BOLT #12 offers: Core Lightning's `bolt12-cli` passes the decoder no
    feature set, so it reads an offer with an even feature bit that
    Faraday refuses (BOLT #9 defines no offer feature); it also reads
    feature bytes with a leading zero byte, which BOLT #9 makes not
    minimal, and a description, issuer or currency holding a character
    of Unicode's Cc, Cf, Co, Zl or Zp categories, which BOLT #1 refuses
    and Faraday refuses; and it refuses what Faraday reads when the
    currency is missing from its ISO 4217 table (the list of 2018) or an
    amount in bitcoin has a first chain Faraday does not know (it names
    the unit by the first chain alone), which Faraday reads as BOLT #12
    says; the area counts each as known, naming the reason, and every
    other difference fails the run
  - BOLT #12 offers, F232's stricter cases: Faraday refuses what Core
    Lightning reads (a text over 4,296 characters, a non-zero padding
    bit, whitespace after a `+` that is not a space, tab or line break,
    a chain twice, no known chain, a currency not in the ISO 4217 list
    Faraday holds, an amount over 21 million bitcoin, an expiry past the
    year 9999, a first node that is no key, an unassigned character);
    the area requires Faraday's refusal and counts it as known
- `ots`
  - OTS: a Bitcoin path through SHA-1 or Keccak-256, or a file taken
    with SHA-1, is counted by the reference when it reaches the header's
    merkle root; Faraday names the first such hash and does not count
    it. Each is asked of the reference and counted.
  - OTS: an Ethereum attestation is verified by the reference against a
    block's transactions root; Faraday lists it by height and does not
    check it. A Litecoin one is listed by both (the reference raises
    when asked to verify it).
  - OTS: an attestation repeated at one node is collapsed by the
    reference and listed twice by Faraday.
  - OTS: Faraday refuses a proof over 64 KiB, a tree of more than 16,384
    entries and a tree whose operations make more than 16 MiB of values;
    the reference reads each (a tree of 17 appends of one byte to a
    value of 4,095 bytes, each followed by 250 reversals, makes 17.5 MiB
    in 8.6 KB; 65 chains of 252 reversals behind distinct appends make
    over 16,384 entries in 17 KB).
<!-- facts:end known-differences -->

## Where each reference comes from

Each folder of `tools/reference/` an area names in its reference, the
repository it was copied from, the path there, and the tag and commit it
was copied at. The folder's own README says how it was fetched and
checked, and states the same address and commit; a folder copied from
more than one repository has a row for each. A row whose Upstream reads
`none (Faraday's own)` is a folder that is not a copy: Faraday wrote it, or
it holds a page quoting other sources, and its other cells read `none`, the
Path naming the outside tool that generated it where one did.

| Folder | Upstream | Path | Tag | Commit |
|---|---|---|---|---|
| `bip327/` | <https://github.com/bitcoin/bips> | `bip-0327/reference.py` | none | `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` |
| `bip352/` | <https://github.com/bitcoin/bips> | `bip-0352/` | none | `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` |
| `bip374/` | <https://github.com/bitcoin/bips> | `bip-0374/reference.py` | none | `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` |
| `bip375/` | <https://github.com/bitcoin/bips> | `bip-0375/` | none | `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` |
| `bip380/` | <https://github.com/bitcoin/bips> | `bip-0380.mediawiki` | none | `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` |
| `bip39/` | <https://github.com/trezor/python-mnemonic> | `src/mnemonic/` | `v0.21` | `d4b106cdec196202d44628026fcb8fedc8ea50c1` |
| `bip85/` | <https://github.com/ethankosakovsky/bip85> | `bip85/` | none | `435a0589746c1036735d0a5081167e08abfa7413` |
| `bip85/` | <https://github.com/keis/base58> | `base58/` | `v2.1.1` | `11c293f4479fafffbc7b766fcb703a835c02dccd` |
| `btclib/` | <https://github.com/btclib-org/btclib-wallet> | none | `v2026.10.29` | `25f9638e25790816102bcc91261e3bf33384ac21` |
| `btclib/` | <https://github.com/btclib-org/btclib> | none | `v2026.10.24` | `e69bb4a6275f6c069d037e5c371010158c655dc6` |
| `btclib/` | <https://github.com/btclib-org/ecc> | none | `v2026.10.23` | `7790699f682f52ec172f180b7b49e6b277ee201f` |
| `slip39/` | <https://github.com/trezor/python-shamir-mnemonic> | `shamir_mnemonic/` | `v0.3.0` | `08bbb74bdcb58c918981836cb3ccf401e57ecfb4` |
| `codex32/` | <https://github.com/BenWestgate/python-codex32> | `src/codex32/` | `v0.6.1` | `4bb90c018c7c515c3d54cf669b89beb42fa26227` |
| `codex32/correction/` | <https://github.com/BenWestgate/python-codex32> | `src/codex32/` | none | `f19d4627c5725a9f6cfa995a1f89fe62e1becb4c` |
| `coldcard/` | <https://github.com/Coldcard/firmware> | `testing/xor.py` | none | `a772156db3fa74cbc54db02d35230d30cf4789dc` |
| `coldcard-rolls/` | <https://github.com/Coldcard/firmware> | `docs/` | none | `948dc108a0a12ea06ec8ee90575609eb3dc90c7b` |
| `core/` | <https://github.com/bitcoin/bitcoin> | `test/functional/test_framework/` | `v31.1` | `9be056a8a72b624dae9623b2f7bded92c2a21c91` |
| `hwi/` | <https://github.com/bitcoin-core/HWI> | `hwilib/` | none | `e63a0af2bf5c7c60b8cc71d99aba59068e8be0f0` |
| `miniscript/` | <https://github.com/darosior/python-bip380> | `bip380/` | none | `fb61971d9128e663f110ea2734c1d023e7e0266b` |
| `btcd-bip322/` | <https://github.com/guggero/btcd> | `bip322/` | none | `503719c950953906a8fad9286b3b51aef5ff152b` |
| `seedsigner/` | <https://github.com/SeedSigner/seedsigner> | `src/seedsigner/` | none | `85cd9a0211eeb22962a5e65ba349ef6efa91bb57` |
| `embit/` | <https://github.com/diybitcoinhardware/embit> | `src/embit/` | `v0.8.0` | `84cce66fb831fa6d625fb73f28e03605f3c04e28` |
| `qrcodegen/` | <https://github.com/nayuki/QR-Code-generator> | `python/qrcodegen.py` | none | `3c6d0b3cefb4e049dc337e82237c9644399716a8` |
| `bbqr/` | <https://github.com/coinkite/BBQr> | `python/bbqr/` | none | `8dc7ef07d0d520763cc0001a885ca8d29ac8719a` |
| `ur/` | <https://github.com/Foundation-Devices/foundation-ur-py> | `ur/` | none | `2c104058d2f6421aaf41f5e919392d3e5a5d2268` |
| `lnd/` | <https://github.com/lightningnetwork/lnd> | `aezeed/` | none | `88959aec3d7a9d11a509714d9bee97c29ee02269` |
| `cln/` | <https://github.com/ElementsProject/lightning> | `common/hsm_secret.c`, `hsmd/libhsmd.c`, `ccan/` | `v26.06.10` | `e8b1609e67575036bd80aec32605e830f8f651cc` |
| `bolt12/` | <https://github.com/ElementsProject/lightning> | the source release archive (not vendored: its SHA-256 is in `release.pin`), built into `devtools/bolt12-cli` | `v26.06.10` | `e8b1609e67575036bd80aec32605e830f8f651cc` |
| `bolt11/` | <https://github.com/lnbits/bolt11> | `bolt11-2.2.0-py3-none-any.whl` | `v2.2.0` | `51bd3b97aeed28a9c4bb0e10dbe8d9414a79d01b` |
| `bolt11/` | <https://github.com/fiatjaf/bech32> | `bech32-1.2.0-py3-none-any.whl` | none | `231e4d88b15f3dc8faf7d339f365b84f6ab5cbcc` |
| `bolt11/` | <https://github.com/keis/base58> | `base58-2.1.1-py3-none-any.whl` | `v2.1.1` | `11c293f4479fafffbc7b766fcb703a835c02dccd` |
| `bolt11/` | <https://github.com/scott-griffiths/bitstring> | `bitstring-3.1.9-py3-none-any.whl` | `bitstring-3.1.9` | `c84ad7f02818caf7102d556c6dcbdef153b0cf11` |
| `bolt11/` | none (Faraday's own) | none | none | none |
| `aezeed/` | <https://github.com/Yawning/aez> | `testdata/` | none | `e49e68abd344eba4a888cee91bc7370df2d14e27` |
| `bip39-slip39/` | <https://github.com/btclib-org/bip39-slip39> | `bip39_slip39-2026.10.24-py3-none-any.whl` | `v2026.10.24` | `f75580077b2046d491abec1ef2a372080031d920` |
| `opentimestamps/` | <https://github.com/opentimestamps/python-opentimestamps> | `opentimestamps/core/` | `v0.5.0` | `8c3dc6fbdf6fcba054e78643e9df5eccd8c9fce4` |
| `booklet-dice/` | none (Faraday's own) | none | none | none |
| `btcd/` | <https://github.com/btcsuite/btcd> | none | `v0.27.0` | `dd169fe0c32a56087d28b92fff3b8cf4fb43477a` |
| `dice/` | none (Faraday's own) | none | none | none |
| `entropy/` | none (Faraday's own) | none | none | none |
| `kdbx4/` | <https://keepass.info/help/kb/kdbx.html> | `kdbx.html` | none | none |
| `kdbx4/` | <https://keepass.info/help/kb/kdbx_4.html> | `kdbx_4.html` | none | none |
| `krux/` | <https://github.com/selfcustody/krux> | `src/krux/` | `v26.09.0` | `2b20e3370169731a0d89839a4113ad9a1b732078` |
| `material-color-utilities/` | <https://github.com/material-foundation/material-color-utilities> | `typescript/` | none | `eeaf82b8e11bf20f6d8da7c76336575b69e79e01` |
| `rust-codex32/` | <https://github.com/apoelstra/rust-codex32> | none | none | `3943cb52b2343333386fac0cd38b8ae5a4d6de02` |
| `signed-efi/` | <http://archive.ubuntu.com/ubuntu/pool/main/s/shim-signed/shim-signed_1.59+15.8-0ubuntu2_amd64.deb> | `usr/lib/shim/shimx64.efi.signed.latest` | `1.59+15.8-0ubuntu2` | none |
| `signed-efi/` | <http://archive.ubuntu.com/ubuntu/pool/main/s/shim-signed/shim-signed_1.59+15.8-0ubuntu2_amd64.deb> | `usr/lib/shim/fbx64.efi` | `1.59+15.8-0ubuntu2` | none |
| `specter/` | none (Faraday's own) | none | none | none |
| `vanity/` | none (Faraday's own) | none | none | none |

## Others' code and Faraday's second version

What each action's cross-check compares Faraday with, by its class
(F160, F177, F179): `outside`, others' code, an implementation others
wrote; `outside-and-own`, others' code and, for the part named,
Faraday's second version, a second implementation Faraday's authors
wrote; or `own`, only Faraday's second version, from the specification. A part compared only
with an implementation Faraday's authors wrote is not checked against an
outside source: it catches a slip between the two implementations, not a
misreading of the specification both share. The table is written by `docs/crypto/gen.py`
from the `independence` table of each action in
`docs/crypto/actions.toml`, which the app's and the website's
Cryptography blocks read too: edit the manifest, then run
`python3 -I -B docs/crypto/gen.py write`. `just lint` fails on an action
with an area and no class (`docs/crypto/check.py`).

<!-- crypto-manifest: begin independence -->
| Area | Compared with | Others' code | Faraday's second version | Actions |
|---|---|---|---|---|
| `bip39` | others' code | python-mnemonic, by Trezor (SatoshiLabs), and bip39_slip39, by the btclib developers (a second reader: wordlists, entropy, indices, checksums and the seed in the ten languages) | none | words-read, words-make, seed, vault-load |
| `bip32` | others' code and, for a part, Faraday's second version | btclib-wallet's `bip32`, by the btclib developers, on btclib's own curve; and `hdkeychain` from btcd v0.27.0, by the btcd developers, on decred's secp256k1 | the three rules of a strict reader that `hdkeychain` does not make: which versions it takes (BIP-32's xprv and tprv, xpub and tpub, from btclib's network table; btclib reads SLIP-132's too and `hdkeychain` any version), a version of the other kind than the key's data, and a depth-0 key with a parent fingerprint or an index, in `area_bip32.py` and `bip32_ref.py` | master |
| `bip32` | others' code | btclib-wallet's `bip32`, by the btclib developers, on btclib's own curve; and `hdkeychain` from btcd v0.27.0, by the btcd developers, on decred's secp256k1 | none | derive, xprv-shown |
| `bip32` | others' code | btclib-wallet's `bip32`, by the btclib developers, on btclib's own curve | none | multisig-account, wif-shown |
| `codex32` | others' code and, for a part, Faraday's second version | python-codex32, by Ben Westgate, and rust-codex32, by Andrew Poelstra (which decides the threshold of 0 or 2 to 9 and a threshold of 0's share index) | two rules BIP 93 states that neither enforces (the seed's lengths, the order shares are handed out), written in `area_codex32.py` from the BIP's text | seed-shown, codex32 |
| `slip39` | others' code and, for a part, Faraday's second version | python-shamir-mnemonic, by Trezor (SatoshiLabs), and bip39_slip39, by the btclib developers, for the core functions these wrappers call (shares read, `recover_ems`, `split_ems`, `derive_member`), judged in the slip39 area | `slip39::check_gathered`, Faraday's own rule on the shares typed (one set, no member twice, no more members than the threshold), which no judge in `tools/crosscheck` runs, and the held set and the read-back, which are tested here against the published vectors | slip39-set-converted |
| `hash` | others' code | Python's `hmac` and `hashlib.pbkdf2_hmac` (OpenSSL) for HMAC and PBKDF2, each over SHA-256 and SHA-512, in area `hash` | none | hashes-macs |
| `hash` | others' code | PyCA `cryptography`'s `HKDF` and `HKDFExpand` (OpenSSL) | none | hkdf |
| `hash` | others' code | Python's `hashlib.scrypt` (OpenSSL) in area `hash`, and LND's `aezeed` package, by Lightning Labs, over `golang.org/x/crypto`'s scrypt, vendored in `tools/reference/lnd/`, in area `lightning` | none | scrypt |
| `addr` | others' code | Bitcoin Core v31.1 itself (`deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; btclib-wallet's `bip32` and `slip132`, by the btclib developers, on btclib's own curve, for the account keys, and its descriptors (`parse`, `script_pub_key`, `checksum`) for each account descriptor's kind, text, script at an index and checksum; BIP 380's own checksum code, as the BIP prints it; python-mnemonic, by Trezor (SatoshiLabs), for the seeds | none | address |
| `miniscript` | others' code | Bitcoin Core v31.1 itself (`deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; python-bip380, by Antoine Poinsot, for Core's vectors that state no script; btclib-wallet's `bip32`, by the btclib developers, for the cosigners' keys, and its descriptors and miniscript, by the btclib developers, on btclib's own curve, for each descriptor's kind, text and script at an index, Core's vectors' types, sanity and scripts, and the satisfaction of each way a wallet's coins move under its lock times | none | descriptor |
| `miniscript` | others' code | Bitcoin Core v31.1 itself (`deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; python-bip380, by Antoine Poinsot, for Core's vectors that state no script; btclib-wallet's `bip32`, by the btclib developers, for the cosigners' keys, and its descriptors and miniscript, by the btclib developers, on btclib's own curve, for each descriptor's kind, text and script at an index, Core's vectors' types, sanity and scripts, and the satisfaction of each way a wallet's coins move under its lock times (and, for recovery wallets, each built from its parts) | none | recovery |
| `taproot` | others' code and, for a part, Faraday's second version | btclib's `check_output_pubkey`, by the btclib developers, on btclib's own curve; `ParseControlBlock` and `VerifyTaprootLeafCommitment` of btcd v0.27.0, by the btcd developers; and Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors, for the trees | the rule that a leaf version is not 0x50, which neither btclib nor btcd makes, written in `area_taproot.py` | tapmulti |
| `import` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1 (`deriveaddresses`, and BIP-380's checksum in its `descriptors.py`), by the Bitcoin Core developers, for every wallet's addresses and checksums; btclib-wallet's BIP-32 and btclib's SLIP-132 versions, by the btclib developers, for the keys; and 93 real files from the wallets' own test suites (Sparrow, BlueWallet, Coldcard, Caravan, Liana), read in the same area | the files the area writes and what each format's fields mean, and the formats no test suite publishes a file for (Nunchuk's, Specter DIY's and Keystone's beyond Sparrow's copies) (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, Keystone's, Wasabi's, Specter Desktop's, Specter DIY's `addwallet`, Core's `listdescriptors`), written in `area_import.py` from each format's documentation, which account of a Coldcard export and which pair of a `listdescriptors` is the wallet, and F155's SLIP-132 refusals | wallet-files |
| `import` | others' code and, for a part, Faraday's second version | btclib's `bms` and btclib-wallet's BIP-32, by the btclib developers, for every key record's signature, made, verified and written; Bitcoin Core v31.1 (`deriveaddresses`), by the Bitcoin Core developers, for a descriptor record's addresses | BIP 129's record lines read in `area_import.py`, and the rules no outside code has: the paths line (/0/*,/1/* or No path restrictions, read as the two chains, F64), a descriptor that is not a template, a description over eighty characters, a token not BIP 129's, and the header forms a signature is accepted in | bsms |
| `compile` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1 itself (`getdescriptorinfo`, `deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; python-bip380, by Antoine Poinsot, for the miniscript's validity, sanity and script; btclib-wallet's descriptors and miniscript, by the btclib developers, on btclib's own curve, for the descriptor's kind, checksum, script at an index and the miniscript's script | the lift of a miniscript to a boolean formula over its keys and waits, written in `area_compile.py` from the fragments' meanings (rust-miniscript's own lift is not used), and the rules by which a policy is refused (a threshold of 0 or above its parts, a lock of 0, a key twice, nothing typed), which no outside compiler states | compile |
| `musig` | others' code | BIP-327's `reference.py`, by the BIP's authors, and btclib's MuSig2 (`ecc.ecc.musig2`), by the btclib developers, on its own curve, for key aggregation, tweaks and the aggregate signature; Bitcoin Core v31.1's `musig()` descriptors through `deriveaddresses`, by the Bitcoin Core developers, Core's musig module is libsecp256k1's, as Faraday's point operations are, so it judges BIP-390's descriptors and addresses, not the curve maths | none | musig-key |
| `addr` | others' code | btclib-wallet's `slip132` and `bip32`, by the btclib developers | none | xpub-spellings |
| `addr` | others' code | Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | none | address-read |
| `hash` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1's functional-test framework, by Bitcoin Core's contributors (`ripemd160.py` for HASH160, `address.py` for Base58Check, `segwit_addr.py` for bech32 and bech32m), and Python's `hashlib` (SHA-256 and SHA-256d) and `bytes.fromhex` (hex) | plain Base58 with no checksum, which the framework does not read, the order the spellings are tried in, the trimming of the text, hex as an even number of hex digits only, and which prefixes make a segwit address, in `area_hash.py`; bech32 text that is not a segwit address is read as all its data groups at any length by Faraday and by the area alike, a stated difference | tools-hash |
| `entropy` | others' code and, for a part, Faraday's second version | Coldcard's `rolls.py` and `rolls12.py` (public domain), run for the hashed procedure and every rolls commitment; SeedSigner's `mnemonic_generation.py` (MIT) over embit's BIP-39, run for the hashed procedure at 50 and 99 rolls and for direct selection's entropy, read back from its completed words, at every length | EntropyLab's six-as-zero and direct word selection (JavaScript, which needs Node, written out in `area_entropy.py` from `tools/reference/dice/README.md`; the wordlist indices from faces are Faraday's rule, SeedSigner has no dice selection), the cut of Coldcard's digest to 20, 24 and 28 bytes, and the roll counts and refused faces | dice |
| `entropy` | others' code and, for a part, Faraday's second version | CPython's `bytes.fromhex`, by the CPython developers, for hex digits read as bytes, against `base16ct` in Faraday | the packing of flips, the commitments, and which characters and how many digits hex takes, written out in `area_entropy.py` | coins-hex |
| `entropy` | Faraday's second version | none | the card procedure, written out in `area_entropy.py` | cards |
| `entropy` | Faraday's second version | none | the frames' commitment, written out in `area_entropy.py` | camera |
| `own-randomness` | Faraday's second version | none | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3, written out in `area_own.py` | pointer-keys, fresh |
| `entropy` | Faraday's second version | none | the mix of commitments, written out in `area_entropy.py` | mix |
| `entropy` | Faraday's second version | none | the hash of the shell's answer, written out in `area_entropy.py`; with what the person added, as `area_own.py` writes it | device |
| `roller` | Faraday's second version | none | the booklet's dice worksheet, written out in `area_roller.py` from `docs/CODEX32.md` §5.5 | booklet |
| `roller` | Faraday's second version | none | Faraday Demo's roller, written out in `area_roller.py` from `docs/CODEX32.md` §5.6 | roller |
| `bip85` | others' code | btclib-wallet's `bip85`, by the btclib developers, on btclib's own curve and run as written over bip39_slip39, by the btclib developers; ethankosakovsky/bip85, by Ethan Kosakovsky, over python-mnemonic, by Trezor (SatoshiLabs), and btclib through a pycoin shim | none | bip85 |
| `vanity` | others' code and, for a part, Faraday's second version | python-mnemonic, by Trezor (SatoshiLabs), Bitcoin Core v31.1 (`deriveaddresses`), by the Bitcoin Core developers, and btclib, by the btclib developers, which derives each candidate's child keys on its own curve (Core judges the script and the address text only, and shares libsecp256k1's curve maths with Faraday's) | EntropyLab's counter, the prefix rules, the expected work and the search loop, written out in `area_vanity.py` from a quotation | vanity |
| `lightning` | others' code | the `bolt11` package (lnbits), over the bech32 and bitstring packages, with a stand-in for its libsecp256k1 binding over btclib; Faraday's stricter refusals are stated differences of the area | none | bolt11 |
| `lightning` | others' code and, for a part, Faraday's second version | Core Lightning's `bolt12-cli`, by the Core Lightning developers (Blockstream), built from its v26.06.10 source release | the offers the area reads, written in `bolt12_enc.py` from BOLT #12, and the stricter refusals of F232 and what Core Lightning's command does not check (a feature set, minimal feature bytes, BOLT #1's refused characters, its 2018 currency table), which are stated differences of the area | bolt12 |
| `ots` | others' code | python-opentimestamps, by the OpenTimestamps developers (the reference client's own reader and verifier of proofs) | none | ots |
| `lightning` | others' code | Lightning Labs (LND), Yawning Angel (AEZ): compared with the processor's AES round on both sides and again with the aes crate's software round against Yawning Angel's AEZ built without assembly | none | aezeed |
| `lightning` | others' code and, for a part, Faraday's second version | LND's node key by btcd's `hdkeychain` in LND's vendored tree, by Lightning Labs and the btcd developers, for LND; btclib-wallet's `bip32`, by the btclib developers, and python-mnemonic, by Trezor (SatoshiLabs), for each step of ldk-node's key | ldk-node's node key, composed in `area_lightning.py` from the sources `tools/reference/aezeed/README.md` quotes (each step btclib's), and LND's key-path numbers (1017', coin, 6', 0, 0), written in `tools/reference/lnd/main.go` from `keychain/derivation.go` | lightning-node |
| `lightning` | others' code | Blockstream and the Core Lightning developers (ccan's HKDF-SHA256, the salt, limits and calls of common/hsm_secret.c written out in tools/reference/cln/main.c), the libsodium developers (Argon2id and the secretstream), python-mnemonic, by Trezor (SatoshiLabs) (the mnemonic form's BIP-39 words and seed) | none | hsm-secret |
| `core_wallet` | others' code | Bitcoin Core (the pinned release's bitcoind on regtest), which makes each wallet and says what it holds (listdescriptors, getwalletinfo, walletpassphrase) and loads or refuses each changed file (loadwallet) | none | core-wallet-keys |
| `sp` | others' code | BIP-352's `reference.py`, by the BIP's authors, over `secp256k1lab`, and btclib-wallet's `silent_payments.py`, by the btclib developers, on btclib's own curve: addresses, labels, input keys and hash, the sender's outputs and the receiver's scan; for BIP 392's spscan key, which has no published code or vectors, that reference's bech32m and curve and btclib's | none | silent-receive |
| `psbt` | others' code | Bitcoin Core v31.1 (`decodepsbt`, and `combinepsbt` as its writer), by Bitcoin Core's contributors; HWI's `psbt.py`, by Ava Chow and HWI's contributors, for the version 2 fields BIP-370 refuses in version 0; btclib-wallet's `Psbt.parse`, by the btclib developers, which also refuses a 65-byte taproot signature whose sighash byte is 0 (BIP-341) | none | psbt-read |
| `psbt` | others' code and, for a part, Faraday's second version | HWI's `psbt.py` (reading, `compute_lock_time`, `get_unsigned_tx`, `convert_to_v0`, `convert_to_v2`), by Ava Chow and HWI's contributors, with both version 0 forms read by Bitcoin Core v31.1; btclib-wallet's `Psbt` (`parse`, `lock_time`, `tx`, `to_v0`, `to_v2`), by the btclib developers, which also refuses bytes after the last map, a stream that ends inside a map and a 65-byte taproot signature whose sighash byte is 0 | the choice of the pairs Faraday leaves to the bridge to version 0, and the version 0 PSBT of a PSBT's own pairs that Core judges where HWI repairs a short value, written in `area_psbt.py` | psbt-v2 |
| `psbt-review` | others' code and, for a part, Faraday's second version | btclib-wallet's descriptor wallet (`update_psbt_output`, `index_of`), by the btclib developers, on btclib's own curve; Bitcoin Core v31.1 (`decodepsbt`, `analyzepsbt`, `deriveaddresses`, and a watch-only descriptor wallet's `getaddressinfo` and `walletprocesspsbt`), by the Bitcoin Core developers | the rule that a script of several keys, a miniscript or a taproot tree is change only for a registered wallet, which outputs claim a held key, and the account keys a registered wallet names, derived in the area | psbt-review |
| `tx` | others' code | btclib's `sig_hash` (`legacy`, `segwit_v0` and `taproot`), by the btclib developers; and Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | none | sighash-review |
| `sign` | others' code | Bitcoin Core v31.1, by the Bitcoin Core developers, which signs from descriptors of the same WIF what Faraday reads from it and shares libsecp256k1's curve maths with Faraday's; btclib, by the btclib developers, which decodes the WIF and works out its public key on its own curve | none | wif-read |
| `sign` | others' code | Bitcoin Core v31.1, by the Bitcoin Core developers, which signs the same sweep from `pkh()`, `wpkh()` and `sh(wpkh())` descriptors of the WIF (`descriptorprocesspsbt`), finalizes it and judges it (`testmempoolaccept`); BIP-143's published P2SH-P2WPKH example signs to its transaction; Core shares libsecp256k1's curve maths with Faraday's, so python-ecdsa, by Brian Warner and Hubert Kario, signs each sweep again under the first RFC 6979 nonce byte for byte, and btclib, by the btclib developers, signs and finalizes each on its own curve | none | sweep-sign |
| `sign` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1, by the Bitcoin Core developers (`descriptorprocesspsbt` signs, `testmempoolaccept` judges the spend), with Core's `key.py` and `script.py` for the first RFC 6979 nonce; python-ecdsa, by Brian Warner and Hubert Kario (no Bitcoin code), which makes that nonce's signature again byte for byte and verifies the low-R ones; Core shares libsecp256k1's curve maths with Faraday's | Faraday's refusals (F136, F141) and the inputs a wallet's rule leaves unsigned, the script code of the hash the first RFC 6979 nonce is checked under, in `area_sign.py`; the keys of those checks are btclib's | sign-ecdsa |
| `sign` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1, by the Bitcoin Core developers (`descriptorprocesspsbt` signs, `testmempoolaccept` judges the spend), with BIP-340's reference (`secp256k1lab`) and Core's `script.py` for the zero-randomness nonce; btclib, by the btclib developers, on its own curve, which verifies each signature and makes it again byte for byte; Core shares libsecp256k1's curve maths with Faraday's | Faraday's refusals (F136, F141) and the inputs a wallet's rule leaves unsigned, in `area_sign.py`; the keys of the zero-randomness check are btclib's | sign-schnorr |
| `musig` | others' code | BIP-327's `reference.py`, by the BIP's authors, and btclib's MuSig2 (`ecc.ecc.musig2`) and btclib-wallet's BIP-373 roles (Updater, Signer, Finalizer), by the btclib developers, on btclib's own curve: nonces, signing, verification and aggregation, and the Core round trip's PSBTs read, verified, signed last and finalized again; the Core 31.1 round trip stays as a judge of BIP-373 and the descriptor and shares libsecp256k1's scalar code with Faraday's | none | sign-musig |
| `sign-check` | others' code and, for a part, Faraday's second version | python-ecdsa, by Brian Warner and Hubert Kario, and btclib, by the btclib developers, on btclib's own curve, verify every signature (ECDSA, and BIP-340 over python-ecdsa's points and btclib's `ssa`); the signature hashes are btclib's (`sig_hash.py`); btclib's script engine runs every complete input; Bitcoin Core v31.1 itself (`finalizepsbt`, `testmempoolaccept`), by Bitcoin Core's contributors, judges the complete inputs and the transaction, and shares libsecp256k1 with Faraday | the report of a reused nonce, written in `area_sign_check.py` from Faraday's own rule, failing closed (signatures that verify with one `r` or `R`, differing in key or in message), which no outside library reports | sign-check |
| `sign` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1, by the Bitcoin Core developers (`finalizepsbt` finalizes and extracts, `testmempoolaccept` judges the transaction over its funding on the node) | the signatures under a taproot `pkh` leaf taken off before Core finalizes, as Faraday leaves such a leaf to the coordinator, and the fields BIP-174 keeps on a finalized input, in `area_sign.py` | finalize |
| `msg` | others' code | Bitcoin Core v31.1 (`signmessagewithprivkey`, `verifymessage`, `signrawtransactionwithkey`); btclib's `bms` and btclib-wallet's `bip322`, by the btclib developers, on btclib's own curve; btcd's `bip322` (pull request 2521), by Oliver Gugger and the btcd developers; python-ecdsa, by Brian Warner and Hubert Kario, for the first-nonce ECDSA signature of BIP-137; Core shares libsecp256k1's curve maths with Faraday's | none | message-sign, message-verify |
| `tx` | others' code and, for a part, Faraday's second version | btclib's `Tx.parse`, by the btclib developers; and Bitcoin Core v31.1's `decoderawtransaction`, by Bitcoin Core's contributors | the refusal of a transaction with no inputs, which neither judge makes (Core's `decoderawtransaction` reads bytes that begin with the marker `00 01` as a legacy transaction of no inputs), written in `area_tx.py` | tx-decode |
| `seedqr` | others' code | Project Nayuki's `qrcodegen.py`, by Nayuki, and segno, by Lars Heuer, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | none | qr, codex32qr |
| `ur` | others' code | foundation-ur-py, by Foundation Devices, and CPython's `zlib.crc32` and cbor2, by the cbor2 developers, which judge the one CRC-32 and the one CBOR (the parts' and payloads' CBOR and the registry's types) | none | ur |
| `bbqr` | others' code and, for a part, Faraday's second version | Coinkite's BBQr library, by Coinkite, for BBQr; krux's `qr.py` and `bbqr.py`, by the krux developers, for writing and reading numbered parts; SeedSigner's `SpecterWalletQrDecoder`, by the SeedSigner developers, for the numbered parts | Faraday's refusals written in `area_bbqr.py` in front of SeedSigner's decoder (a part number of 0 or past the count, a repeat with other text, a header in upper case or missing, a count over 1,000), the one-table pyqrcode shim checked against `qrcodegen.py`, and krux's three shims (QR, base32, deflate) | bbqr |
| `kef` | others' code and, for a part, Faraday's second version | Krux's `kef.py`, by the Krux developers, vendored unchanged and run under CPython, for all twelve versions, written and read | Faraday's refusals listed as known differences in `area_kef.py` (an empty plaintext, an id of 253 bytes, a text that inflates past the limit, a passphrase or count Krux's field does not offer, suggestions over 1,024 bytes), and krux's three shims (`ucryptolib`, `uhashlib_hw` and deflate at wbits -10, over hashlib, PyCA and zlib) | kef |
| `sqlite` | others' code and, for a part, Faraday's second version | SQLite itself, by the SQLite developers, through CPython's `sqlite3`, which makes each database and returns its rows | Faraday's refusals asked of Faraday alone in `area_sqlite.py` (the write-ahead-log form, auto-vacuum and UTF-16 databases, which SQLite reads) | sqlite |
| `hash` | others' code | Python's `hashlib` (SHA-256), over random files of every length around the 64-byte blocks and of 4 KiB, 64 KiB and 1 MiB with a byte more and less | none | file-hash |
| `seedqr` | others' code and, for a part, Faraday's second version | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki, and segno, by Lars Heuer, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define (the entropy itself) | seedqr-make |
| `seedqr` | others' code and, for a part, Faraday's second version | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | Faraday's rule written in `area_seedqr.py` in front of SeedSigner's reader (exactly 48 or 96 digits, 16 or 32 bytes), and the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define | seedqr-read |
| `codex32` | others' code and, for a part, Faraday's second version | volvelle-website's checksum worksheet, by Andrew Poelstra (the worksheets of 48 and 74 characters), and rust-codex32, by Andrew Poelstra (a fresh set's derived shares), beside python-codex32, by Ben Westgate | the worksheet of 127 characters, which volvelle does not lay out, written in `area_codex32.py` from python-codex32's polymod | codex32-make |
| `codex32-correct` | others' code and, for a part, Faraday's second version | python-codex32, by Ben Westgate (its `correction.py`) | the bound BIP 93 states (substitutions and unreadable boxes, one run up to the checksum's length) and a held string's fixed header, written in `area_codex32_correct.py` from the BIP's sentence | codex32-correct |
| `slip39` | others' code and, for a part, Faraday's second version | python-shamir-mnemonic, by Trezor (SatoshiLabs), and bip39_slip39, by the btclib developers (shares read and combined, secrets split under the same random bytes, the passphrase cipher, `recover_ems`, `split_ems` and `_interpolate`) | Faraday's refusals of a new member in `derive_refused` (`area_slip39.py`), written in front of the references, which have no function for them: shares of more than one group, an index outside 0 to 15, a group of threshold 1, a count other than the threshold, a repeated index, an index among the shares given; the refused lengths of the fixed 1 of 1; the identifier rules `one_of_one_identifier` and `codex32_identifier`, which the libraries take as a parameter | slip39 |
| `xor` | others' code and, for a part, Faraday's second version | Coldcard's `xor` and `xor_split`, by Coinkite, over python-mnemonic, by Trezor (SatoshiLabs) | the count of parts (2 to 4) and the five BIP-39 lengths, Faraday's rule written in `area_xor.py` in front of Coldcard's `xor`, which asserts only equal lengths | xor |
| `seedqr` | others' code | zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers, which decode each picture this writes and whose payload is compared with the key's words | none | stick-write |
| `seedqr` | others' code | zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers, which decode the picture this writes and whose payload is compared with the key's words | none | outbox |
| `sps` | others' code | BIP-374/375 authors (bitcoin/bips), and btclib's `dleq.py` (BIP-374) and btclib-wallet's BIP-375 roles (Signer, Extractor), by the btclib developers, on btclib's own curve: proofs, shares, output scripts and the verdict on a PSBT; btclib-wallet counts a taproot input's NUMS claim only against a proof, so a PSBT with an unproved claim is judged by the validator alone | none | silent-send |
| `vault` | others' code and, for a part, Faraday's second version | argon2-cffi (the Argon2 designers' reference code), for Argon2id, and Python's `unicodedata`, for the NFC form | the payload layouts of `docs/VAULT.md` §7, written in `area_vault.py` from that document; the KDBX file itself is read by pykeepass, `keepassxc-cli` and keepass-rs in the area (an outside reader each) | vault-unlock, vault-seal, vault-edit |
| `qrvault` | others' code and, for a part, Faraday's second version | Sequoia's sq, by the Sequoia PGP developers (judges the primitives except Argon2, which it shares with the app's lineage, and opens Faraday's messages), rPGP, by the rPGP developers (judges the format only, on the app's lineage: opens Faraday's messages and seals the ones Faraday opens), and PyCA cryptography over OpenSSL (judges Argon2id, HKDF-SHA-256 and AES-256-OCB) | the OpenPGP framing around PyCA's primitives, read and written strictly in `area_qrvault.py` from RFC 9580 and `docs/VAULT.md` §1.2, and the QR TOML (`qr_toml`) from §2.1, read beside by CPython's `tomllib`; Faraday's second version of the framing, not another program's | vault-qr-pgp |
| `qrvault` | others' code and, for a part, Faraday's second version | Sequoia's sq and rPGP, as for the OpenPGP message the code is | none beyond the message's own | qr-vault-seal, qr-vault-open |
| `vault` | others' code | pykeepass, by the libkeepass project, and KeePassXC's keepassxc-cli (the distribution's packages), and argon2-cffi and PyCA's cryptography over OpenSSL for the Argon2id | none | kdbx |
| `totp` | others' code | pyotp, by the pyotp developers, which judges the code, the seconds left, the period and the reading of an otpauth URI's key, digits, period and hash; Faraday's stricter and wider readings of a URI are stated differences of the area | none | totp |
<!-- crypto-manifest: end independence -->

## What the machine needs

`python3` 3.11 or later (the standard library only: `tomllib`,
`hashlib`), `unshare` from util-linux with unprivileged user namespaces
allowed, and the Rust toolchain; PyPy 3.11 or later if it is to go
faster (above). Nothing is installed with pip. The
`vault` area needs the distribution's Argon2 binding, argon2-cffi
(Debian and Ubuntu: `python3-argon2`), PyCA cryptography 44 or later over
OpenSSL 3.2 or later (`python3-cryptography`), pykeepass (`python3-pykeepass`) and
KeePassXC's `keepassxc-cli` (`keepassxc`); without them it stops and names them,
and the run fails. The `lightning` area needs Go 1.27 or later
(`go version`; `run.sh` builds `tools/reference/lnd/` offline from its
vendored modules, with `GOTOOLCHAIN=local`) and stops the run when `go`
is missing or older, and a C compiler, `pkg-config` and libsodium's headers
(Debian and Ubuntu: `build-essential`, `pkg-config`, `libsodium-dev`) for
Core Lightning's program (`tools/reference/cln/`, compiled by `run.sh`); it
stops when they are missing. The area also builds Core Lightning's offer
decoder, `devtools/bolt12-cli`, from the release archive
`tools/reference/bolt12/release.pin` pins, fetched once with the network by
`python3 -I -B tools/crosscheck/fetch_cln.py` into
`~/.cache/faraday/core-lightning` (`FARADAY_CLN_SOURCE` for another folder)
and built by `tools/crosscheck/build_bolt12_cli.sh` with Core Lightning's own
`./configure` and `make`, which need `autoconf`, `automake`, `libtool`,
`gettext`, `python3-mako`, `lowdown`, `jq`, `unzip`, `libsqlite3-dev` and
`zlib1g-dev` (Debian and Ubuntu) besides the compiler, `make` and libsodium; the
run stops when the archive or a package is missing. A passing run writes these versions into the stamp. Bitcoin
Core v31.1's release tarball, fetched once with `fetch_core.py`, which
needs `gpg` and `git` (above).

## Files

| File | What |
|---|---|
| `Cargo.toml`, `src/` | `faraday-crosscheck`, the Rust command: one JSON request a line in, one answer out; never shipped |
| `run.sh` | builds the command offline and runs `crosscheck.py` with no network |
| `crosscheck.py` | the self-test, the batches on the pool of workers, the progress, the report, the stamp |
| `record.py` | the release's record, `out/crosscheck-record.txt`, written by a full `--fresh` run |
| `status.py` | reads a run's progress (`just crosscheck-status`) |
| `identity.py` | the real areas under several worker counts and Pythons, reports compared (`just crosscheck-identity`) |
| `common.py` | the Rust process, the comparison, the reference check |
| `core_node.py` | Bitcoin Core: the pinned tarball checked and unpacked, the regtest node started and stopped, its RPC client |
| `fetch_core.py` | fetches the pinned Core release once and checks it against `guix.sigs`' signatures and the pin |
| `fetch_cln.py` | fetches Core Lightning's pinned source release once and checks it against the pin |
| `build_bolt12_cli.sh` | builds Core Lightning's `bolt12-cli` from that release, for the `lightning` area |
| `area_*.py` | one area each, with the facts it states about itself |
| `facts.py` | reads those facts, writes this README's tables between marker lines, and checks them (`just lint`) |
| `btc.py` | Base58Check and BIP-32 written from the BIPs, which `bip32`, `addr`, `psbt`, `msg`, `sign` and `miniscript` share |
| `miniscript_ref.py` | python-bip380 set up for `sign` and `miniscript`: its shims, RIPEMD-160 from the BIPs repository, and the scripts kept beside their miniscript text |
| `references.sha256` | every vendored file the areas import or read, with its SHA-256 |
| `stamp.py`, `stamp` | the stamp and its check |
| `test_harness.py` | the harness's own tests (`just crosscheck-test`): where it reads a case more quickly than its reference, it reaches the reference's answer; the report, the progress and a case run alone are the same for any number of workers and under PyPy |
