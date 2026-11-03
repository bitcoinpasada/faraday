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
tool fails the test instead of skipping it, and the OVMF Secure Boot run
(`faraday/tools/sb-ovmf-check.py`). Not with `--area` or `--identity`.
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
one) gives PyPy every area it can import (the `vault` and `kdbx` areas,
which need the distribution's packages for CPython, stay on CPython).
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
(`record.py`), the release's record: the git tree it checked (what
`git rev-parse <tag>^{tree}` gives for the release), seed, scale and UTC date;
each area with its cases, answers, known differences and result; each
reference folder with its commit and the SHA-256 of its files; Bitcoin
Core's version and tarball SHA-256; and "every answer agreed" or the
first disagreement. `faraday/tools/stage-release.sh` stages it as
`crosscheck-<version>.txt` beside `SHA256SUMS`, listed in it.

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
| `slip39` | `faraday_bip::slip39` | python-shamir-mnemonic (`slip39/`), its `recover_ems`, `split_ems` and `_interpolate` for a set's EMS, a new split of it and a member at an index; bip39_slip39 (`bip39-slip39/`) answering every operation beside it; the refusals of a member at an index before the arithmetic are ours (`derive_refused`) | all 45 of `tools/vectors/slip39/`, each again through its EMS and its decryption | splits fed the same random bytes, which must match word for word; recovery from qualifying and short subsets and a changed word; the passphrase cipher; a set's EMS from a quorum, refused from a short or changed set, split again on the same random bytes (some under a new identifier where extendable) and a member added or a lost one made again at an index, each recovering the first set's secret, and a member refused at an index given, past 15, from a short quorum and across two groups |
| `codex32` | `faraday_bip::codex32` | python-codex32 (`codex32/`); rust-codex32 at a pinned commit (`rust-codex32/`), a program outside the workspace: parsing, checksums, the threshold and a threshold of 0's share index, `interpolate_at` for a fresh set's shares; volvelle-website's checksum worksheet run as a program over every worksheet of 48 and 74 characters; the worksheet of 127 characters written in the area from python-codex32's polymod | every vector of `tools/vectors/bip93/`, the 55 invalid strings included; volvelle-website's worksheet cells (`tools/vectors/codex32-volvelle/`); the booklet's checksum, derivation, recovery, addition, translation and fusion tables (`tools/vectors/codex32-booklet/`); the Codex32QR profile's sets (`tools/vectors/codex32qr/`) | secrets, splits (the same randomness), recovery, derived shares, corrupted strings; fresh sets of every threshold up to 31 strings at 48, 74 and 127 characters, the checksum worksheets of their strings and of corrupted ones, checksums made on a sheet, translation and recovery symbols, translation worksheets |
| `codex32-correct` | `faraday_bip::codex32::correct` | python-codex32's decoder, `correction.py` at a later commit (`codex32/correction/`), asked with BIP 93's bound | python-codex32's frozen correction corpus, 57 cases (`tools/vectors/codex32-correction/`) | random strings of every length with substitutions and unreadable boxes up to and past the bound, runs of unreadable boxes around the checksum's length, a held string's header fixed, upper case, `ms1` damaged |
| `xor` | `faraday_entropy::SeedXor` with `faraday_bip::bip39` | Coldcard's own `xor` and `xor_split` (`coldcard/`, unchanged) with python-mnemonic (`bip39/`) for the words | Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's two | splits and combines in all ten languages and five lengths, with refusals; parts made by Coldcard's `xor_split` read back to their seed |
| `hash` | `faraday_bip::hashes` and `faraday_codec::encodings` (`read`, the writers and `from_hex`), and `faraday_core::file_menu::sha256` | Bitcoin Core v31.1's functional-test framework (`core/`): `ripemd160.py`, `address.py` and `segwit_addr.py`, with `hashlib` for SHA-256 and `bytes.fromhex` for hex | FIPS 180-4's SHA-256 examples, RIPEMD-160 of `abc`, BIP-173's key and its HASH160, Bitcoin Core's first `key_io_valid.json` address, BIP-173's and BIP-350's bech32 strings and their valid and invalid segwit addresses (`tools/vectors/addresses`) | random bytes of every length from 0 to 130 and to 300 hashed (SHA-256, SHA-256d, HASH160) and as a file, files of 4 KiB, 64 KiB and 1 MiB with a byte more and less; random bytes written and read as Base58Check, Base58, bech32 and bech32m, Core's writing read by Faraday, segwit addresses of every version Core's framework writes, and texts changed, cut or made of random characters read by both |
| `vault` | `faraday_vault` (create, open, seal, the slot's records and payload layouts), `faraday_backup::argon2id`, the `chacha20poly1305` crate's detached XChaCha20-Poly1305 as the vault calls it | `docs/VAULT.md` written out in `area_vault.py`, over argon2-cffi, PyNaCl and PyCA cryptography (the distribution's packages; the last for HKDF-SHA256 and ChaCha20) | RFC 5869's three SHA-256 cases, RFC 8439's five block vectors, draft-irtf-cfrg-xchacha-03's AEAD vector, the Argon2id known answer (`tools/vectors/vault-primitives/`); the vaults and the slot vectors of `tools/vectors/vault/`, among them `two-keys.fvlt`, whose one slot PyNaCl verifies under two passphrases' keys | Argon2id at small random costs and the costs it refuses; the AEAD at random lengths with one byte changed; payloads made, read and malformed; random records of all thirteen types a caller writes (type 12 is retired and refused) with the seal count, encoded, decoded and broken a rule at a time; passphrases in every Unicode form, made and opened; whole vaults of every slot size made byte for byte from a seed, opened by each passphrase, resealed with a count one higher, and refused for each header fault, a wrong passphrase, a changed byte, two slots under one key, a commitment block not zero and faulty contents |
| `bip39` | `faraday_bip::bip39` | python-mnemonic (`bip39/`); bip39_slip39 (`bip39-slip39/`) answering every operation beside it | the ten wordlists; every vector of `tools/vectors/bip39/vectors.json` in Faraday's languages; the Japanese vectors with non-ASCII passphrases | entropy to words and back in every language and length; seeds under random ASCII passphrases; a changed word, an unknown word, a word too many or too few; every valid last word |
| `bip32` | `faraday_bip::keys` (master, private derivation), `faraday_bip::xkey` (strict decoding), `bitcoin`'s `Xpub::derive_pub` | btclib-wallet's `bip32` over btclib and `ecc` (`btclib/`), on btclib's own curve with libsecp256k1 turned off, through `bip32_ref.py`, and `hdkeychain` from btcd v0.27.0 (`btcd/`, Go, decred's secp256k1); the versions a strict reader takes (BIP-32's four, from btclib's network table), a version whose kind differs from the key's data, and a depth-0 key with a parent or an index are ours | BIP-32's vectors 1 to 5 (`tools/vectors/bip32/`), python-mnemonic's master keys | seeds of 16 to 64 bytes on every network, random paths with every index, public derivation against private, keys broken one rule at a time and the master-key import, each judged by btclib and by `hdkeychain` |
| `addr` | `faraday_bip::address::parse`, `bitcoin`'s `Address::from_script`, `faraday_bip::account` (account xpub, SLIP-132, descriptor, addresses), `faraday_bip::descriptor`'s checksum, `faraday_bip::policy::WalletPolicy::from_descriptor` (an account descriptor's kind, text and script at an index) | Bitcoin Core's functional-test framework (`core/`): `segwit_addr.py`, Base58Check, `descriptors.py`; BIP 380's own checksum code as the BIP prints it (`bip380/`); btclib-wallet's `bip32` and `slip132` (`btclib/`, on btclib's own curve) for accounts' keys, and its descriptors (`descriptors`) for each account descriptor's kind, text, script at an index and checksum, and as a third checksum judge; Bitcoin Core v31.1's `bitcoind` on regtest for every account's address (`deriveaddresses`) and for reading Faraday's regtest descriptors (`getdescriptorinfo`) | BIP-350's and BIP-173's addresses, BIP-380's checksum cases, BIP-49's, 84's and 86's accounts (`tools/vectors/addresses/`) | scripts of every address kind on every network, written and read back, then changed; accounts of every type at random indices, their descriptors read and changed; random descriptors' checksums |
| `tx` | `faraday_psbt::transaction::read_raw`, `bitcoin`'s `SighashCache` as `faraday_psbt::sign` calls it | btclib's `Tx.parse` and `sig_hash` (`btclib/`); Bitcoin Core v31.1's `decoderawtransaction` on regtest (`core_node.py`) for the transactions; Core's `messages.py` and `script.py` (`core/`) for the signature hashes; the refusal of a transaction with no inputs is ours | Core's 500 legacy sighashes, BIP-143's thirteen, BIP-341's key-path sighashes (`tools/vectors/sighash/`) | random transactions with and without witnesses (txid, wtxid, weight, bytes) read by btclib and Core's `decoderawtransaction`, cut or lengthened; every input hashed as legacy, BIP-143 P2WPKH and P2WSH, BIP-341 key and script path under every hash type, by btclib and the framework |
| `psbt` | `faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` (version 2, Faraday's own) | Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (`decodepsbt`, `combinepsbt` of one PSBT as its writer, the test framework's `taproot_tree_helper`); HWI's `psbt.py` (`hwi/`) for version 2 and the version 2 fields refused in version 0, its version 0 form read by Core; btclib-wallet's `Psbt` (`btclib/`, on btclib's own curve) for both versions: `parse`, `lock_time`, `tx`, `to_v0` and `to_v2`, which also decides the refusals of a taproot signature whose sighash byte is 0, of bytes after the last map and of a stream that ends inside a map | every PSBT of BIP-174's, 370's and 371's test vectors (`tools/vectors/psbt-bips/`), with BIP-370's locktimes, read by btclib too | random version 0 and 2 PSBTs with every field Faraday reads, taproot trees among them, in random order, read field for field as Core reads them and written back byte for byte as read and as Core writes them, then broken one rule at a time; each version 0 PSBT made version 2 as HWI's `convert_to_v2` and btclib's `to_v2` make it, and back to its own bytes; BIP-370's locktime, transaction and version 0 form under every mix of required locktimes, also btclib's `lock_time`, `tx` and `to_v0`; one version 0 PSBT in eight carries fields btclib refuses and btclib is not asked of it |
| `sign` | `faraday_psbt::sign` (every held key selected, deterministic Schnorr, both RFC 6979 nonce rules) and `faraday_psbt::finalize` | Bitcoin Core v31.1 on regtest (`core_node.py`): `descriptorprocesspsbt` signs from the held masters' keys, `finalizepsbt` finalizes and extracts, `testmempoolaccept` judges each finalized transaction over its funding on the node; BIP-340's reference (`secp256k1lab`) and Core's `key.py` for the deterministic nonces, over Core's `script.py` sighashes; python-ecdsa (the distribution's package) and btclib (`btclib/`, its own curve) as a second curve lineage, beside them: every ECDSA signature again under plain RFC 6979 or verified, every BIP-340 signature verified and made again; btclib-wallet's finalizer (`psbt.finalize` with its miniscript solver) finalizes every input Faraday finalizes again and gives the same scriptSig and witness; for each sweep, btclib reads the WIF (scalar, compression, public key) and signs and finalizes the sweep on its own curve, and python-ecdsa makes the first-nonce signature of each input again, byte for byte; python-bip380 (`miniscript/`) reads the leaves. Faraday's refusals and the inputs a wallet's rule leaves unsigned are ours, in `area_sign.py` | BIP-174's signer, finalizer and extractor vectors; the Core-built spends of `tools/vectors/psbt/` (2-of-3 `sortedmulti`, the Liana wallet down both paths, a taproot tree by a leaf and by the key path, BIP 387's taproot multisig), signed by every subset of their keys; the finalizer vectors there (`or_b`'s smaller signature, the taproot leaves that tie by count, the taproot multisig and MuSig2 spends) | PSBTs of one to four inputs, each spending a funding output on the node: P2PKH, P2SH-P2WPKH, P2WPKH, `multi`/`sortedmulti` in P2SH, P2WSH and P2SH-P2WSH, P2TR key path, P2TR script path (`pk`, `pkh`, timelocked keys, `or_d`, `multi_a`) with the key path beside it, miniscript in P2WSH and P2SH-P2WSH; held and foreign keys, cosigners' signatures already on (Core's), every standard sighash type, timelocks met and not; Core's signatures (ECDSA byte for byte, BIP-340 by verification and the deterministic nonce), each final scriptSig and witness and the extracted transaction Core's, the cleared fields, and every finalized transaction accepted by Core's mempool; an origin that derives another key refused; sweeps: a WIF key's P2PKH, P2WPKH and P2SH-P2WPKH outputs (P2PKH alone for an uncompressed key) spent beside another key's, signed by Core from the WIF and by `faraday_psbt::sweep::sign`, byte for byte, finalized and accepted by the mempool |
| `sign-check` | `faraday_psbt::verify::verify_signatures` (every signature of a PSBT, in a partial field or a final scriptSig or witness, checked against its signature hash) and `faraday_psbt::verify::repeated_nonces` (one `r` or `R` under one key in more than one place) | python-ecdsa (the distribution's package, its own curve) and btclib (`btclib/`, libsecp256k1 off) verify every signature over btclib's signature hashes (`sig_hash.py`); btclib's script engine (`verify_input`, standardness rules on) runs every complete input; Bitcoin Core v31.1's `finalizepsbt` and `testmempoolaccept` judge the complete inputs and the transaction, and are named as sharing libsecp256k1 with Faraday; the reuse of a nonce is ours (`reused`, in `area_sign_check.py`) | none: BIP-340's and ECDSA's published vectors are in the areas that sign (`sign`, `msg`); the signatures here are made for the PSBT in the case, with verdicts given by two verifiers | transactions of one to three inputs of legacy P2PKH, P2WPKH, P2WSH multisig (1 of 2 to 3 of 3), taproot key path and taproot script path, each with its signatures partial (in the PSBT's fields) or complete (in a final scriptSig or witness), good, under the wrong key, over the wrong sighash type, with a high S, or malformed, with every sighash type; nonces reused on purpose across inputs that share a key (ECDSA with `k` and `-k` too, Schnorr), one nonce under two keys, and one key under fresh nonces |
| `psbt-review` | `faraday_psbt::inspect` (amounts in and out, the fee, verified change and the spoofed-change and missing-UTXO warnings), over `faraday_psbt::Psbt` and the keys and registered wallets the Sign screen holds | Bitcoin Core v31.1's `bitcoind` on regtest, offline: `decodepsbt` and `analyzepsbt` for amounts and fee, `deriveaddresses` from `tprv` descriptors for the addresses the PSBTs pay, `createpsbt`, `createrawtransaction` and `descriptorprocesspsbt` to make the PSBTs; btclib's descriptor wallet (`update_psbt_output` and `index_of`, `btclib/`, its own curve) for which outputs are change; a watch-only descriptor wallet on the same node (`getaddressinfo`, `walletprocesspsbt`) for where each change claim's script is; the area adds each input's UTXO fields, splices a spoofed output, derives the account keys a registered wallet names (checked by Core deriving the same addresses from them), and keeps the rule that a script of several keys is change only for a registered wallet | none: Core's decoding and btclib's change decision are the references, over each wallet shape with change, spoofed change, a foreign claim and missing UTXO data in the prelude | PSBTs of one to three inputs and one to four outputs over single-key accounts (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR) and registered wallets (`sortedmulti` in P2WSH, a miniscript in P2WSH, a taproot tree, a taproot `sortedmulti_a`) or the same wallets unregistered: witness UTXOs, previous transactions and no UTXO data, fees from nothing to a third of the inputs, payments, change on either chain, another wallet's change, and spoofed change over the wallet's own script at another index, another wallet's and a stranger's; one key held, two, or none of the wallet's |
| `import` | `faraday_intake::wallet::read_wallet` (the wallet a file or text is: descriptors, `faraday_bip::coldcard`, `faraday_bip::multisig_config`, Caravan's, Electrum's, Cobo's, Keystone's and Wasabi's files, Specter DIY's `addwallet`, `faraday_bip::bsms::DescriptorRecord`), and `faraday_bip::bsms::SignerRecord` read and verified and `faraday_bip::bsms::signer_record` over the BIP-48 account node's key | Bitcoin Core v31.1's `bitcoind` on regtest, offline (`deriveaddresses`), and its `descriptors.py` (BIP-380's checksum) and `address.py`, for each wallet's receive and change scripts and every checksum; btclib-wallet 2026.10.29's BIP-32 (btclib's own curve) for every key, btclib 2026.10.24's SLIP-132 versions and addresses, and its `bms` for every key record's signature, made, verified and written byte for byte; CPython's `json` for the value of every JSON file read and for a key named twice and a nesting past 32 refused (those two rules are the reader's own, written beside it); the files are written in the area from each format's documentation, and the meaning of their fields is the area's | BIP 129's key and descriptor records (`tools/vectors/bip129/`), and 93 real files from the wallets' own test suites, 62 of them a wallet and 31 refused by both sides (`faraday/faraday-core/tests/vectors/interop/`: Sparrow's and BlueWallet's; `tools/vectors/wallet-files/`: Coldcard's firmware tests, Caravan's and Liana's): BSMS records, Coldcard setup files and exports, Caravan, Electrum, Cobo, Keystone, Jade, Specter Desktop and Specter DIY files, descriptors and a Core `listdescriptors` | single-key wallets (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR), `sortedmulti` in P2WSH, P2SH-P2WSH and P2SH of one to fifteen keys, a miniscript in P2WSH, a taproot tree and a taproot `sortedmulti_a`, on mainnet and testnet keys, each as a descriptor (multipath with and without checksum, receive and change lines, labelled, receive alone, Core's `listdescriptors`, Specter Desktop's file, Specter DIY's `addwallet` with and without its keys' derivations) or as the files that state its kind (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, Wasabi's, a key alone, a BSMS descriptor record), in BIP-32's or SLIP-132's spelling; damaged by a wrong checksum, a wrong first address, a SLIP-132 version of another script or another paths line; key records signed by btclib in each header form, with or without damage, and Faraday's own record of random seeds |
| `taproot` | `bitcoin`'s `TaprootBuilder` and `TaprootSpendInfo` (as `faraday_bip::tapmulti` and `miniscript`'s `tr()` build trees), `ScriptBuf::new_p2tr` as `faraday_psbt::sign` checks a key path, `Address::p2tr_tweaked`, `ControlBlock` decoding and `verify_taproot_commitment` | Core's `taproot_construct`, `key.py`'s tweak and `segwit_addr.py` for the trees; btclib's `check_output_pubkey` (`btclib/`, on its own curve) and btcd v0.27.0's `txscript.ParseControlBlock` and `VerifyTaprootLeafCommitment` (`btcd/`, Go) for the control blocks; the rule that a leaf version is not 0x50 is ours | BIP-341's `scriptPubKey` wallet vectors, all seven (`tools/vectors/sighash/`) | trees of 0 to 12 leaves in random shapes with random scripts and leaf versions over random internal keys: leaf hashes, Merkle root, tweak, output key and parity, script, address, control blocks; each control block checked, then changed eleven ways, each judged by btclib and by btcd; internal keys that are no point, odd leaf versions and 0x50 refused |
| `compile` | `faraday_bip::compile::compile` (a concrete policy to a `wsh()` or `tr()` descriptor, the keys it names, the ways it can be spent and the script it pays at an index), over rust-miniscript's policy compiler | Bitcoin Core v31.1's `bitcoind` on regtest (`getdescriptorinfo` and `deriveaddresses`), with its `descriptors.py` (BIP-380's checksum); btclib-wallet's descriptors and miniscript (`btclib/`, on btclib's own curve) for the kind, checksum, script at an index and keys; Antoine Poinsot's python-bip380 (`miniscript/`) for the miniscript's validity, sanity and script, and its fragments for the lift to a formula, which is ours (`lift`, in `area_compile.py`) | none: no compiler states a script for a policy (rust-miniscript, sipa's and the Go and Python ones choose differently), so the answers are judged by what the descriptor is and means | policies of `and`, `or` (with weights), `thresh`, `pk`, `after` and `older` of one to six keys, each key written as an account key with its origin, compiled for `wsh()` and for `tr()` (a key beside the rest, which the compiler takes as the internal key), on mainnet and testnet keys, read at random indices of both chains; refusals: a threshold of 0 or above its parts, a lock of 0, a key twice, nothing typed |
| `miniscript` | `faraday_bip::policy::WalletPolicy` (a wallet's descriptor and the script it pays at an index, its kind, text and spend paths), `faraday_bip::policy::parse_script_descriptor`, a witness script read back as `faraday_psbt` reads one, `faraday_bip::recovery::RecoveryPolicy` (a recovery wallet built from its parts and read back) | Bitcoin Core v31.1's `bitcoind` on regtest (`deriveaddresses`) for random wallets, their cosigners' keys from btclib (`btclib/`, on btclib's own curve); btclib-wallet's descriptors and miniscript (`btclib/`) for each descriptor's kind, text and script, Core's vectors' types and scripts, and the satisfaction of each way a wallet's coins move under its lock times; Antoine Poinsot's python-bip380 (`miniscript/`) for Core's vectors that state no script | Bitcoin Core's miniscript unit tests, 97 miniscripts in P2WSH and Tapscript (`tools/vectors/miniscript/`) | wallets of the Liana, Nunchuk, Sparrow and recovery shapes in `wsh()` and `sh(wsh())`, `multi`/`sortedmulti` in `sh()`, `wsh()`, `sh(wsh())`, `tr()` with trees of one to six leaves or a lone `sortedmulti_a`, over random accounts on every network, at random indices of both chains; the same made not sane, refused; recovery wallets of both forms with one to three recovery paths, and the ways each wallet's coins move spent at their lock times |
| `msg` | `faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids) | Bitcoin Core v31.1 and btclib for BIP-137, btclib-wallet and btcd for BIP-322: Core's `signmessagewithprivkey`, `verifymessage` and `signrawtransactionwithkey` (the low-R P2WPKH witness); btclib's `bms` and btclib-wallet's `bip322` (`btclib/`, btclib's own curve); python-ecdsa (the distribution's package) for the first-nonce ECDSA signature of BIP-137; btcd's `bip322` from pull request 2521 (`btcd-bip322/`, Go) | BIP-322's basic and generated vectors (hashes, txids, simple, full and proof-of-funds signatures, errors) and btcd's negative vectors; Trezor's and Electrum's BIP-137 signatures (`tools/vectors/message/`), Trezor's made again from its keys | random keys, address forms, networks and messages: virtual txids; signatures made byte for byte under both nonce rules and checked by every reference, then checked against a changed message, another address, a changed byte and a moved header; btcd's own simple signatures checked by Faraday |
| `own-randomness` | `faraday_core`'s `fresh.rs` (`Own`, the session seed and pool) through the testkit's `fresh` and `seed_draw`, with `faraday_entropy`'s dice, coin, camera and pointer-and-keys commitments | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3 written out in `area_own.py` over `hashlib` | the known answers of `faraday/faraday-core/tests/all/own_randomness.rs`: nothing added, dice before and after the system's answers, dice then coins, a batch of pointer moves and keys, a lock | random sessions of system answers, dice, coins, camera frames (some refused, some empty) and pointer moves and keys (none to several batches), locks, seed draws, fresh bytes and counts, in any order |
| `entropy` | `faraday_entropy`: dice under the three procedures, coins, hex, cards, camera frames, the device's answer, a mix | the procedures written out in `area_entropy.py` over `hashlib`, from `tools/reference/entropy/README.md` and the dice sources quoted in `tools/reference/dice/README.md`, except that the hashed dice procedure and every rolls commitment are Coldcard's own `rolls.py` and `rolls12.py` (`tools/reference/coldcard-rolls/`, run, public domain) | Coldcard's `rolls.py` digest, EntropyLab's six-as-zero and direct word selection (`tools/vectors/entropy/`) | runs of every source at every strength, short of the count, at it and past it, with faces, cards, frames and characters the pads refuse and every source past its limit; mixes of one to six sources given by their inputs |
| `roller` | Faraday Demo's dice rollers (`faraday_core`'s `fresh/roller/`, through the testkit) | `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` | every byte value as an answer: each face from 42 bytes, the four above 251 skipped; every byte as a face of every die of 2 to 20 faces (`faraday_entropy::uniform_face`); the booklet's 32 paths on six- and twenty-faced dice and its order with an equal pair (`roller.sheet`) | random answers (some mostly skipped bytes) under clicks of every size into entries of random room, with resets; the five dice of random faces (2 to 20) under clicks of every reach into codex32 shares of random count and length, answers running out part way; rolls of random dice entered by hand, equal pairs among them |
| `seedqr` | `faraday_codec::seedqr` (SeedQR, CompactSeedQR), `faraday_codec::qr`, the encoder that draws them, and `faraday_codec::qr::codex32qr` with the picture `faraday_core::picture` writes of it | SeedSigner's own encoders and `DecodeQR` (`seedsigner/`, unchanged, with embit's BIP-39 code in `embit/`); Project Nayuki's `qrcodegen.py` (`qrcodegen/`), module for module; segno 1.6 (`python3-segno`), an encoder of another lineage, at Faraday's mask and at its own; Pillow and CPython's zlib, which read every PNG Faraday writes; zbar (`zbarimg`) and zxing-cpp, which decode the QR code in every picture and return its payload as bytes (`qr_decode.py`) | SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and picture of the Codex32QR profile (`tools/vectors/codex32qr/`), each decoded by zbar and zxing-cpp | keys of every length and language, both codes and the words read back; digits and bytes malformed; payloads in each mode at each level and version limit; 48-character codex32 strings in either case as Codex32QR codes and the pictures written of them read back and decoded by zbar and zxing-cpp, other lengths refused; the pictures of a key's SeedQR and CompactSeedQR decoded by both and read by Faraday to the key's words; greyscale pictures and QR pictures of random size and scale read by Pillow and zlib |
| `bbqr` | `faraday_qr::bbqr`, `faraday_qr::specter` (numbered text parts) | Coinkite's BBQr library (`bbqr/`, with a one-table pyqrcode shim checked against `qrcodegen.py`); krux's `qr.py` and `bbqr.py` (`krux/`, unchanged, with three small shims for the firmware's QR, base32 and deflate modules); SeedSigner's own `SpecterWalletQrDecoder` (`seedsigner/`, unchanged) | Coinkite's BBQr test files (`tools/vectors/bbqr/`): the 26 parts of a phone's scan, which read as a `U` file with the text Coinkite's test expects, and its ten PSBTs and transactions, each split in every encoding and read back to the file's bytes | files of every type and size written by krux (at the sizes of QR versions 1 to 20) and split by Coinkite in each encoding, read in any order, with repeats, a part missing, another transfer's part, a conflicting repeat and a part past the count, against Coinkite and krux; what Faraday writes, part for part krux's own writing at the same size, read back by `join_qrs` and by krux; numbered parts written by krux, written again by Faraday, read in any order against SeedSigner's decoder and krux and refused when malformed |
| `ur` | `faraday_codec::ur` (single-part URs, the animation's parts, the decoder) | Foundation Devices' foundation-ur-py (`ur/`), URKit ported | URKit's single-part and multipart encoder vectors (`tools/vectors/ur/`) | messages as `crypto-psbt` and `bytes` at random fragment lengths, parts byte for byte; sets with fragments dropped, mixes added, shuffled, repeated and upper case, put together by both; URs of other types written, read, changed and cut |
| `vanity` | `faraday_bip::vanity` (the counter, prefixes and their expected work, the search on both dials) | EntropyLab's counter as `tools/reference/vanity/README.md` quotes it, written out in `area_vanity.py` over python-mnemonic, each candidate's child key derived by btclib on its own curve, and its address by Bitcoin Core v31.1 for the script and address text only (`deriveaddresses` on regtest, from the bare key) and Core's framework, as the `addr` area writes it | EntropyLab's two finds | counters at every width's edges; prefixes of every kind on every network, valid and not; searches of random keys on both dials from random starting counters, with budgets ending before a match and after |
| `kdbx` | `faraday_backup::kdbx` (the KDBX 4 export and its own check) | pykeepass, the distribution's package (`python3-pykeepass`) | the repository's export (`tools/vectors/backup/`), written again byte for byte and opened | databases of one to eight items with markup characters, control characters, line ends and several scripts, at small Argon2id costs, every entry read back; the writer's check against pykeepass under the right passphrase, a wrong one and a changed byte |
| `lightning` | `faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read) | LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one process answering the run's requests (the aezeed also asked of a second build of each side, Faraday's with the aes crate's software round and LND's without assembly); ldk-node's derivation as `tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` over btclib's BIP-32 (`btclib/`, on btclib's own curve) | LND's five cipher seeds, LND's two node keys, ldk's five node ids (`tools/vectors/aezeed/`) | random BIP-39 seeds and entropies on every network; every aezeed case again with the aes crate's software round against LND's AEZ without assembly; random words, a wrong version with a right checksum, published seeds under a changed passphrase, a changed enciphered byte or salt with the checksum put right, a changed word, each refused by LND's code |

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
- `vault`: `check_primitives` runs before anything else: HKDF against
  RFC 5869's three SHA-256 cases, ChaCha20 against RFC 8439's five block
  vectors, PyNaCl against draft-irtf-cfrg-xchacha-03's AEAD vector,
  argon2-cffi against `tools/vectors/kdf/`'s known answer; the versions
  of argon2-cffi, PyNaCl and cryptography are written into the stamp;
  the passphrase in NFC is `unicodedata`'s, and the area stops where
  Python's Unicode is newer than `unicode-normalization`'s.
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
  key; MuSig2 partial signatures (area `musig`); a finalized P2SH input;
  the order of the signatures within a reuse report.
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
  their files hashed against `references.sha256` before any run; the
  rest is written in the area over `hashlib` from the statements of
  `tools/reference/entropy/README.md` and
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
- `vanity`: the search is written in the area from the README's
  quotation of EntropyLab's counter, and EntropyLab's two finds are
  required of it as well as of Faraday.
- `kdbx`: pykeepass is the distribution's package and the area stops and
  names it when it is missing, so a run without it fails; the versions
  are written into the stamp.
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
  and ldk's published answers are required of it as well as of Faraday.
  Not compared: ldk-node's node key is compared with a derivation
  written here, not with ldk's code (Rust, not vendored): only ldk's
  five published answers are outside it; LND's key path numbers are
  written in the Go program from `keychain/derivation.go`, as LND
  derives through btcwallet, which is not vendored.
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
    claim. A PSBT with an unproved claim is not asked of btclib-wallet.
    The corpus also makes inputs whose claim is proved, which all three
    skip.
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
  - Both refuse a record of type 7 or 9 without its required fields, a
    sheet past 16 KiB, an empty passphrase at create, and a second type
    14 or type 10 record, as `docs/VAULT.md` §7 and §8 say (owner,
    2026-10-23).
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
  - A descriptor with a second `#` in its body: btclib refuses it, as
    Bitcoin Core's parser does, where BIP 380's code and Core's
    framework check the last nine characters; its checksum is asked of
    those two only. btclib reads descriptors Faraday takes for no
    account (`pk`, `combo`, `multi`, `addr`, `raw`) and other chains
    than `<0;1>`; the area asks it for the four account descriptors with
    both chains only.
- `tx`
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
  - A bug in Faraday, held until it is fixed
    (KNOWN_BUG_TAPROOT_SIGHASH_ZERO): a finalized taproot signature, key
    path or script path, of 65 bytes ending in the sighash byte 0x00.
    BIP 341 leaves the byte out for the default type, and Bitcoin Core,
    python-ecdsa and btclib call the signature malformed;
    `verify_signatures` calls it valid. Counted for exactly these
    signatures; every other disagreement fails, and the run passes when
    Faraday refuses them.
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
    it (`Empty Miniscript compilation`); Faraday refuses it too, and the
    area counts the refusal. python-bip380 writes `v:multi_a` as
    `OP_NUMEQUAL OP_VERIFY` where BIP 379 and Core write
    `OP_NUMEQUALVERIFY`, so a leaf with one is not compared to
    python-bip380's script (counted); the output key, which commits to
    the leaf, is Core's and btclib's.
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
    refuses; its deflate reads a 256-byte window on a device, where the
    shim reads the largest. A set with an empty last part is read by
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
- `vanity`
  - Each candidate's child key is derived by btclib on its own curve,
    not by Bitcoin Core, which shares libsecp256k1's curve maths with
    Faraday; Core is given the bare key and judges the script and the
    address text only.
- `kdbx`
  - KDBX: pykeepass strips the characters XML 1.0 cannot hold from a
    protected value after decrypting it; Faraday writes a Password's
    bytes as they are. The area expects pykeepass's reading and counts
    the items that differ. pykeepass also opens a file whose major
    version is neither 3 nor 4 without reading past it, deriving a key
    or checking a hash; KeePass refuses such a file, and the area counts
    a file as opened only when pykeepass read a KDBX 4 body.
  - pykeepass runs the key derivation the header states before it checks
    the header's SHA-256, so a changed bit in the Argon2 iterations
    (2^44 passes) holds it for good; KeePass and KeePassXC check the
    hash first and refuse at once. The area asks pykeepass first with a
    key given and nothing decrypted, which checks the version and the
    header's hash and derives nothing, and only then with the
    passphrase.
<!-- facts:end known-differences -->

## Where each reference comes from

Each folder of `tools/reference/` an area names in its reference, the
repository it was copied from, the path there, and the tag and commit it
was copied at. The folder's own README says how it was fetched and
checked, and states the same address and commit; a folder copied from
more than one repository has a row for each.

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
| `bip39` | others' code | python-mnemonic, by Trezor (SatoshiLabs), and Python's `hashlib` and `hmac` (OpenSSL) | none | hashes-macs |
| `lightning` | others' code | LND's `aezeed` package, by Lightning Labs, over `golang.org/x/crypto`'s scrypt, vendored in `tools/reference/lnd/` | none | scrypt |
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
| `entropy` | others' code and, for a part, Faraday's second version | Coldcard's `rolls.py` and `rolls12.py` (public domain), run for the hashed procedure and every rolls commitment | EntropyLab's six-as-zero and direct word selection (JavaScript, which needs Node, written out in `area_entropy.py` from `tools/reference/dice/README.md`), the cut of Coldcard's digest to 20, 24 and 28 bytes, and the roll counts and refused faces | dice |
| `entropy` | Faraday's second version | none | the packing of flips and digits, written out in `area_entropy.py` | coins-hex |
| `entropy` | Faraday's second version | none | the card procedure, written out in `area_entropy.py` | cards |
| `entropy` | Faraday's second version | none | the frames' commitment, written out in `area_entropy.py` | camera |
| `own-randomness` | Faraday's second version | none | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3, written out in `area_own.py` | pointer-keys, fresh |
| `entropy` | Faraday's second version | none | the mix of commitments, written out in `area_entropy.py` | mix |
| `entropy` | Faraday's second version | none | the hash of the shell's answer, written out in `area_entropy.py`; with what the person added, as `area_own.py` writes it | device |
| `roller` | Faraday's second version | none | the booklet's dice worksheet, written out in `area_roller.py` from `docs/CODEX32.md` §5.5 | booklet |
| `roller` | Faraday's second version | none | Faraday Demo's roller, written out in `area_roller.py` from `docs/CODEX32.md` §5.6 | roller |
| `bip85` | others' code | btclib-wallet's `bip85`, by the btclib developers, on btclib's own curve and run as written over bip39_slip39, by the btclib developers; ethankosakovsky/bip85, by Ethan Kosakovsky, over python-mnemonic, by Trezor (SatoshiLabs), and btclib through a pycoin shim | none | bip85 |
| `vanity` | others' code and, for a part, Faraday's second version | python-mnemonic, by Trezor (SatoshiLabs), Bitcoin Core v31.1 (`deriveaddresses`), by the Bitcoin Core developers, and btclib, by the btclib developers, which derives each candidate's child keys on its own curve (Core judges the script and the address text only, and shares libsecp256k1's curve maths with Faraday's) | EntropyLab's counter, the prefix rules, the expected work and the search loop, written out in `area_vanity.py` from a quotation | vanity |
| `lightning` | others' code | Lightning Labs (LND), Yawning Angel (AEZ): compared with the processor's AES round on both sides and again with the aes crate's software round against Yawning Angel's AEZ built without assembly | none | aezeed |
| `lightning` | others' code and, for a part, Faraday's second version | LND's node key by btcd's `hdkeychain` in LND's vendored tree, by Lightning Labs and the btcd developers, for LND; btclib-wallet's `bip32`, by the btclib developers, and python-mnemonic, by Trezor (SatoshiLabs), for each step of ldk-node's key | ldk-node's node key, composed in `area_lightning.py` from the sources `tools/reference/aezeed/README.md` quotes (each step btclib's), and LND's key-path numbers (1017', coin, 6', 0, 0), written in `tools/reference/lnd/main.go` from `keychain/derivation.go` | lightning-node |
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
| `sign-check` | others' code and, for a part, Faraday's second version | python-ecdsa, by Brian Warner and Hubert Kario, and btclib, by the btclib developers, on btclib's own curve, verify every signature (ECDSA, and BIP-340 over python-ecdsa's points and btclib's `ssa`); the signature hashes are btclib's (`sig_hash.py`); btclib's script engine runs every complete input; Bitcoin Core v31.1 itself (`finalizepsbt`, `testmempoolaccept`), by Bitcoin Core's contributors, judges the complete inputs and the transaction, and shares libsecp256k1 with Faraday | the report of a reused nonce, written in `area_sign_check.py` from the definition (two signatures that verify, under one key, over different messages, with one `r` or `R`), which no outside library reports | sign-check |
| `sign` | others' code and, for a part, Faraday's second version | Bitcoin Core v31.1, by the Bitcoin Core developers (`finalizepsbt` finalizes and extracts, `testmempoolaccept` judges the transaction over its funding on the node) | the signatures under a taproot `pkh` leaf taken off before Core finalizes, as Faraday leaves such a leaf to the coordinator, and the fields BIP-174 keeps on a finalized input, in `area_sign.py` | finalize |
| `msg` | others' code | Bitcoin Core v31.1 (`signmessagewithprivkey`, `verifymessage`, `signrawtransactionwithkey`); btclib's `bms` and btclib-wallet's `bip322`, by the btclib developers, on btclib's own curve; btcd's `bip322` (pull request 2521), by Oliver Gugger and the btcd developers; python-ecdsa, by Brian Warner and Hubert Kario, for the first-nonce ECDSA signature of BIP-137; Core shares libsecp256k1's curve maths with Faraday's | none | message-sign, message-verify |
| `tx` | others' code and, for a part, Faraday's second version | btclib's `Tx.parse`, by the btclib developers; and Bitcoin Core v31.1's `decoderawtransaction`, by Bitcoin Core's contributors | the refusal of a transaction with no inputs, which neither judge makes (Core's `decoderawtransaction` reads bytes that begin with the marker `00 01` as a legacy transaction of no inputs), written in `area_tx.py` | tx-decode |
| `seedqr` | others' code | Project Nayuki's `qrcodegen.py`, by Nayuki, and segno, by Lars Heuer, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | none | qr, codex32qr |
| `ur` | others' code | foundation-ur-py, by Foundation Devices | none | ur |
| `bbqr` | others' code and, for a part, Faraday's second version | Coinkite's BBQr library, by Coinkite, for BBQr; krux's `qr.py` and `bbqr.py`, by the krux developers, for writing and reading numbered parts; SeedSigner's `SpecterWalletQrDecoder`, by the SeedSigner developers, for the numbered parts | Faraday's refusals written in `area_bbqr.py` in front of SeedSigner's decoder (a part number of 0 or past the count, a repeat with other text, a header in upper case or missing, a count over 1,000), the one-table pyqrcode shim checked against `qrcodegen.py`, and krux's three shims (QR, base32, deflate) | bbqr |
| `hash` | others' code | Python's `hashlib` (SHA-256), over random files of every length around the 64-byte blocks and of 4 KiB, 64 KiB and 1 MiB with a byte more and less | none | file-hash |
| `seedqr` | others' code and, for a part, Faraday's second version | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki, and segno, by Lars Heuer, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define (the entropy itself) | seedqr-make |
| `seedqr` | others' code and, for a part, Faraday's second version | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki, and the pictures decoded by zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers | Faraday's rule written in `area_seedqr.py` in front of SeedSigner's reader (exactly 48 or 96 digits, 16 or 32 bytes), and the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define | seedqr-read |
| `codex32` | others' code and, for a part, Faraday's second version | volvelle-website's checksum worksheet, by Andrew Poelstra (the worksheets of 48 and 74 characters), and rust-codex32, by Andrew Poelstra (a fresh set's derived shares), beside python-codex32, by Ben Westgate | the worksheet of 127 characters, which volvelle does not lay out, written in `area_codex32.py` from python-codex32's polymod | codex32-make |
| `codex32-correct` | others' code and, for a part, Faraday's second version | python-codex32, by Ben Westgate (its `correction.py`) | the bound BIP 93 states (substitutions and unreadable boxes, one run up to the checksum's length) and a held string's fixed header, written in `area_codex32_correct.py` from the BIP's sentence | codex32-correct |
| `slip39` | others' code and, for a part, Faraday's second version | python-shamir-mnemonic, by Trezor (SatoshiLabs), and bip39_slip39, by the btclib developers (shares read and combined, secrets split under the same random bytes, the passphrase cipher, `recover_ems`, `split_ems` and `_interpolate`) | Faraday's refusals of a new member in `derive_refused` (`area_slip39.py`), written in front of the references, which have no function for them: shares of more than one group, an index outside 0 to 15, a group of threshold 1, a count other than the threshold, a repeated index, an index among the shares given | slip39 |
| `xor` | others' code and, for a part, Faraday's second version | Coldcard's `xor` and `xor_split`, by Coinkite, over python-mnemonic, by Trezor (SatoshiLabs) | the count of parts (2 to 4) and the five BIP-39 lengths, Faraday's rule written in `area_xor.py` in front of Coldcard's `xor`, which asserts only equal lengths | xor |
| `seedqr` | others' code | zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers, which decode each picture this writes and whose payload is compared with the key's words | none | stick-write |
| `seedqr` | others' code | zbar, by the zbar developers, and zxing-cpp, by the zxing-cpp developers, which decode the picture this writes and whose payload is compared with the key's words | none | outbox |
| `sps` | others' code | BIP-374/375 authors (bitcoin/bips), and btclib's `dleq.py` (BIP-374) and btclib-wallet's BIP-375 roles (Signer, Extractor), by the btclib developers, on btclib's own curve: proofs, shares, output scripts and the verdict on a PSBT; btclib-wallet counts a taproot input's NUMS claim only against a proof, so a PSBT with an unproved claim is judged by the validator alone | none | silent-send |
| `vault` | others' code and, for a part, Faraday's second version | argon2-cffi (the Argon2 designers' reference code), PyNaCl (libsodium) and PyCA `cryptography` (OpenSSL, for HKDF-SHA256 and ChaCha20), for the primitives | the container, its slot layout, key schedule and records, written in `area_vault.py` from the FVLT format's account (`docs/history/VAULT-LOG.md`) | vault-create, vault-unlock, vault-seal, vault-edit |
| `kdbx` | others' code | pykeepass, by the libkeepass project (the distribution's package) | none | kdbx |
<!-- crypto-manifest: end independence -->

## What the machine needs

`python3` 3.11 or later (the standard library only: `tomllib`,
`hashlib`), `unshare` from util-linux with unprivileged user namespaces
allowed, and the Rust toolchain; PyPy 3.11 or later if it is to go
faster (above). Nothing is installed with pip. The
`vault` area needs the distribution's Argon2, libsodium and
cryptography bindings, argon2-cffi, PyNaCl and PyCA cryptography
(Debian and Ubuntu: `python3-argon2` `python3-nacl`
`python3-cryptography`), and the `kdbx` area the distribution's
pykeepass (`python3-pykeepass`); without them each stops and names them,
and the run fails. The `lightning` area needs Go 1.27 or later
(`go version`; `run.sh` builds `tools/reference/lnd/` offline from its
vendored modules, with `GOTOOLCHAIN=local`) and stops the run when `go`
is missing or older. A passing run writes these versions into the stamp. Bitcoin
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
| `area_*.py` | one area each, with the facts it states about itself |
| `facts.py` | reads those facts, writes this README's tables between marker lines, and checks them (`just lint`) |
| `btc.py` | Base58Check and BIP-32 written from the BIPs, which `bip32`, `addr`, `psbt`, `msg`, `sign` and `miniscript` share |
| `miniscript_ref.py` | python-bip380 set up for `sign` and `miniscript`: its shims, RIPEMD-160 from the BIPs repository, and the scripts kept beside their miniscript text |
| `references.sha256` | every vendored file the areas import or read, with its SHA-256 |
| `stamp.py`, `stamp` | the stamp and its check |
| `test_harness.py` | the harness's own tests (`just crosscheck-test`): where it reads a case more quickly than its reference, it reaches the reference's answer; the report, the progress and a case run alone are the same for any number of workers and under PyPy |
