# The crypto cross-check

F79, `docs/CROSSCHECK.md` §10. Faraday's own Rust answers requests, and
pure-Python references recompute every answer, over every published
vector and a corpus of random inputs drawn from a fixed seed. The first
disagreement stops the area and is printed with its inputs and both
answers. Some references are code Faraday did not write; others are
models Faraday's authors wrote, over outside code or alone, and those
are not independent checks (F160): "Who wrote each reference" below says
which, per action.

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

A passing full run writes `stamp`, with each area's corpus case count
beside the seed. `tools/lint-crosscheck.sh` (in `just
lint`, about a tenth of a second) hashes the crypto sources, this
folder, the references, the vectors and the crypto crates' `Cargo.lock`
entries, and fails with "crypto changed since the last cross-check: run
`just crosscheck`" when they no longer match the stamp. Commit the stamp
with the change that made it pass. `python3 stamp.py show` lists what is
hashed.

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
(`record.py`), the release's record: commit, seed, scale and UTC date;
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
| `musig` | `faraday_bip::musig` | BIP-327's `reference.py` (`bip327/`); btclib (`btclib/`, its own curve) verifies the aggregate signature as BIP-340 | all eight files of `tools/vectors/bip327/` | sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial signatures, their verification (and a tampered one), aggregation checked as BIP-340, deterministic signing |
| `sp` | `faraday_bip::silent` | BIP-352's `reference.py` (`bip352/`) | every case of `tools/vectors/bip352/`, both sides | inputs of every kind BIP-352 reads, receivers with labels, a sender's outputs and what each receiver finds among them and decoys |
| `sps` | `faraday_psbt::silent`, `faraday_bip::dleq` | BIP-374's `reference.py` (`bip374/`) and BIP-375's validator and `test_runner.py` (`bip375/`), over `secp256k1lab` | both CSV files of `tools/vectors/bip374/` and `tools/vectors/bip375/bip375_test_vectors.json` | proofs made and verified, damaged or not, under two generators; version 2 PSBTs of one to four inputs of every kind BIP-375 reads and some it does not, one to three silent payment outputs, shares added by one signer or two, a global share, and each finished PSBT damaged in one of eighteen ways, judged by the BIP's validator |
| `bip85` | `faraday_bip::bip85` | btclib-wallet's `bip85` (`btclib/`, on btclib's own curve), its one call to `bip39_slip39` answered by python-mnemonic; and `ethankosakovsky/bip85` over python-mnemonic and a pycoin shim whose every call is btclib's (`bip85/`, `bip39/`) | every application vector of `tools/vectors/bip85/` | random master keys, indices, languages, word counts and lengths, with refusals |
| `slip39` | `faraday_bip::slip39` | python-shamir-mnemonic (`slip39/`) | all 45 of `tools/vectors/slip39/` | splits fed the same random bytes, which must match word for word; recovery from qualifying and short subsets and a changed word; the passphrase cipher |
| `codex32` | `faraday_bip::codex32` | python-codex32 (`codex32/`); the worksheet's rows written in the area from its polymod | every vector of `tools/vectors/bip93/`, the 55 invalid strings included; volvelle-website's worksheet cells (`tools/vectors/codex32-volvelle/`); the booklet's checksum, derivation, recovery, addition, translation and fusion tables (`tools/vectors/codex32-booklet/`); the Codex32QR profile's sets (`tools/vectors/codex32qr/`) | secrets, splits (the same randomness), recovery, derived shares, corrupted strings; fresh sets of every threshold up to 31 strings at 48, 74 and 127 characters, the checksum worksheets of their strings and of corrupted ones, checksums made on a sheet, translation and recovery symbols, translation worksheets |
| `codex32-correct` | `faraday_bip::codex32::correct` | python-codex32's decoder, `correction.py` at a later commit (`codex32/correction/`), asked with BIP 93's bound | python-codex32's frozen correction corpus, 57 cases (`tools/vectors/codex32-correction/`) | random strings of every length with substitutions and unreadable boxes up to and past the bound, runs of unreadable boxes around the checksum's length, a held string's header fixed, upper case, `ms1` damaged |
| `xor` | `faraday_entropy::SeedXor` with `faraday_bip::bip39` | Coldcard's own `xor` and `xor_split` (`coldcard/`, unchanged) with python-mnemonic (`bip39/`) for the words | Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's two | splits and combines in all ten languages and five lengths, with refusals; parts made by Coldcard's `xor_split` read back to their seed |
| `vault` | `faraday_vault` (create, open, seal, the slot's records and payload layouts), `faraday_backup::argon2id`, the `chacha20poly1305` crate's detached XChaCha20-Poly1305 as the vault calls it | `docs/VAULT.md` written out in `area_vault.py`, over argon2-cffi, PyNaCl and PyCA cryptography (the distribution's packages; the last for HKDF-SHA256 and ChaCha20) | RFC 5869's three SHA-256 cases, RFC 8439's five block vectors, draft-irtf-cfrg-xchacha-03's AEAD vector, the Argon2id known answer (`tools/vectors/vault-primitives/`); the vaults and the slot vectors of `tools/vectors/vault/`, among them `two-keys.fvlt`, whose one slot PyNaCl verifies under two passphrases' keys | Argon2id at small random costs and the costs it refuses; the AEAD at random lengths with one byte changed; payloads made, read and malformed; random records of all thirteen types a caller writes (type 12 is retired and refused) with the seal count, encoded, decoded and broken a rule at a time; passphrases in every Unicode form, made and opened; whole vaults of every slot size made byte for byte from a seed, opened by each passphrase, resealed with a count one higher, and refused for each header fault, a wrong passphrase, a changed byte, two slots under one key, a commitment block not zero and faulty contents |
| `bip39` | `faraday_bip::bip39` | python-mnemonic (`bip39/`) | the ten wordlists; every vector of `tools/vectors/bip39/vectors.json` in Faraday's languages; the Japanese vectors with non-ASCII passphrases | entropy to words and back in every language and length; seeds under random ASCII passphrases; a changed word, an unknown word, a word too many or too few; every valid last word |
| `bip32` | `faraday_bip::keys` (master, private derivation), `faraday_bip::xkey` (strict decoding), `bitcoin`'s `Xpub::derive_pub` | btclib-wallet's `bip32` over btclib and `ecc` (`btclib/`), on btclib's own curve with libsecp256k1 turned off, through `bip32_ref.py`; which versions a strict reader takes (BIP-32's four, from btclib's network table) is ours | BIP-32's vectors 1 to 5 (`tools/vectors/bip32/`), python-mnemonic's master keys | seeds of 16 to 64 bytes on every network, random paths with every index, public derivation against private, keys broken one rule at a time and the master-key import |
| `addr` | `faraday_bip::address::parse`, `bitcoin`'s `Address::from_script`, `faraday_bip::account` (account xpub, SLIP-132, descriptor, addresses), `faraday_bip::descriptor`'s checksum | Bitcoin Core's functional-test framework (`core/`): `segwit_addr.py`, Base58Check, `descriptors.py`; BIP 380's own checksum code as the BIP prints it (`bip380/`); btclib-wallet's `bip32` and `slip132` (`btclib/`, on btclib's own curve) for accounts' keys; Bitcoin Core v31.1's `bitcoind` on regtest for every account's address (`deriveaddresses`) and for reading Faraday's regtest descriptors (`getdescriptorinfo`) | BIP-350's and BIP-173's addresses, BIP-380's checksum cases, BIP-49's, 84's and 86's accounts (`tools/vectors/addresses/`) | scripts of every address kind on every network, written and read back, then changed; accounts of every type at random indices; random descriptors' checksums |
| `tx` | `faraday_psbt::transaction::read_raw`, `bitcoin`'s `SighashCache` as `faraday_psbt::sign` calls it | Core's `messages.py` and `script.py` (`core/`) | Core's 500 legacy sighashes, BIP-143's thirteen, BIP-341's key-path sighashes (`tools/vectors/sighash/`) | random transactions with and without witnesses (txid, wtxid, weight, bytes), cut or lengthened; every input hashed as legacy, BIP-143 P2WPKH and P2WSH, BIP-341 key and script path under every hash type |
| `psbt` | `faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` (version 2, Faraday's own) | Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (`decodepsbt`, `combinepsbt` of one PSBT as its writer, the test framework's `taproot_tree_helper`); HWI's `psbt.py` (`hwi/`) for version 2 and the version 2 fields refused in version 0, its version 0 form read by Core; the refusals neither makes written in `area_psbt.py` | every PSBT of BIP-174's, 370's and 371's test vectors (`tools/vectors/psbt-bips/`), with BIP-370's locktimes | random version 0 and 2 PSBTs with every field Faraday reads, taproot trees among them, in random order, read field for field as Core reads them and written back byte for byte as read and as Core writes them, then broken one rule at a time; each version 0 PSBT made version 2 as HWI's `convert_to_v2` makes it and back to its own bytes; BIP-370's locktime, transaction and version 0 form under every mix of required locktimes |
| `sign` | `faraday_psbt::sign` (every held key selected, deterministic Schnorr, both RFC 6979 nonce rules) and `faraday_psbt::finalize` | Bitcoin Core v31.1 on regtest (`core_node.py`): `descriptorprocesspsbt` signs from the held masters' keys, `finalizepsbt` finalizes and extracts, `testmempoolaccept` judges each finalized transaction over its funding on the node; BIP-340's reference (`secp256k1lab`) and Core's `key.py` for the deterministic nonces, over Core's `script.py` sighashes; python-ecdsa (the distribution's package) and btclib (`btclib/`, its own curve) as a second curve lineage, beside them: every ECDSA signature again under plain RFC 6979 or verified, every BIP-340 signature verified and made again; python-bip380 (`miniscript/`) reads the leaves. Faraday's refusals and the inputs a wallet's rule leaves unsigned are ours, in `area_sign.py` | BIP-174's signer, finalizer and extractor vectors; the Core-built spends of `tools/vectors/psbt/` (2-of-3 `sortedmulti`, the Liana wallet down both paths, a taproot tree by a leaf and by the key path, BIP 387's taproot multisig), signed by every subset of their keys; the finalizer vectors there (`or_b`'s smaller signature, the taproot leaves that tie by count, the taproot multisig and MuSig2 spends) | PSBTs of one to four inputs, each spending a funding output on the node: P2PKH, P2SH-P2WPKH, P2WPKH, `multi`/`sortedmulti` in P2SH, P2WSH and P2SH-P2WSH, P2TR key path, P2TR script path (`pk`, `pkh`, timelocked keys, `or_d`, `multi_a`) with the key path beside it, miniscript in P2WSH and P2SH-P2WSH; held and foreign keys, cosigners' signatures already on (Core's), every standard sighash type, timelocks met and not; Core's signatures (ECDSA byte for byte, BIP-340 by verification and the deterministic nonce), each final scriptSig and witness and the extracted transaction Core's, the cleared fields, and every finalized transaction accepted by Core's mempool; an origin that derives another key refused; sweeps: a WIF key's P2PKH, P2WPKH and P2SH-P2WPKH outputs (P2PKH alone for an uncompressed key) spent beside another key's, signed by Core from the WIF and by `faraday_psbt::sweep::sign`, byte for byte, finalized and accepted by the mempool |
| `psbt-review` | `faraday_psbt::inspect` (amounts in and out, the fee, verified change and the spoofed-change and missing-UTXO warnings), over `faraday_psbt::Psbt` and the keys and registered wallets the Sign screen holds | Bitcoin Core v31.1's `bitcoind` on regtest, offline: `decodepsbt` and `analyzepsbt` for amounts and fee, `deriveaddresses` from `tprv` descriptors for which outputs are change, `createpsbt`, `createrawtransaction` and `descriptorprocesspsbt` to make the PSBTs; the area adds each input's UTXO fields, splices a spoofed output, derives the account keys a registered wallet names (checked by Core deriving the same addresses from them), and applies the rule that an output is change when it claims a held key's origins and Core's script at the path it states is the output's own | none: Core's own decoding and derivation are the reference, over each wallet shape with change, spoofed change, a foreign claim and missing UTXO data in the prelude | PSBTs of one to three inputs and one to four outputs over single-key accounts (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR) and registered wallets (`sortedmulti` in P2WSH, a miniscript in P2WSH, a taproot tree, a taproot `sortedmulti_a`) or the same wallets unregistered: witness UTXOs, previous transactions and no UTXO data, fees from nothing to a third of the inputs, payments, change on either chain, another wallet's change, and spoofed change over the wallet's own script at another index, another wallet's and a stranger's; one key held, two, or none of the wallet's |
| `import` | `faraday_intake::wallet::read_wallet` (the wallet a file or text is: descriptors, `faraday_bip::coldcard`, `faraday_bip::multisig_config`, Caravan's, Electrum's, Cobo's, Keystone's and Wasabi's files, `faraday_bip::bsms::DescriptorRecord`), and `faraday_bip::bsms::SignerRecord` read and verified and `faraday_bip::bsms::signer_record` over the BIP-48 account node's key | Bitcoin Core v31.1's `bitcoind` on regtest, offline (`deriveaddresses`), and its `descriptors.py` (BIP-380's checksum) and `address.py`, for each wallet's receive and change scripts and every checksum; btclib-wallet 2026.10.29's BIP-32 (btclib's own curve) for every key, btclib 2026.10.24's SLIP-132 versions and addresses, and its `bms` for every key record's signature, made, verified and written byte for byte; the files are written in the area from each format's documentation, and the meaning of their fields is the area's | BIP 129's key and descriptor records (`tools/vectors/bip129/`), and other wallets' files from their test suites (`faraday/faraday-core/tests/vectors/interop/`): Sparrow's and Nunchuk's BSMS records, Coldcard Mk4's and Q's exports, a Core `listdescriptors` | single-key wallets (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR), `sortedmulti` in P2WSH, P2SH-P2WSH and P2SH of one to fifteen keys, a miniscript in P2WSH, a taproot tree and a taproot `sortedmulti_a`, on mainnet and testnet keys, each as a descriptor (multipath with and without checksum, receive and change lines, labelled, receive alone, Core's `listdescriptors`, Specter Desktop's file) or as the files that state its kind (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, Wasabi's, a key alone, a BSMS descriptor record), in BIP-32's or SLIP-132's spelling; damaged by a wrong checksum, a wrong first address, a SLIP-132 version of another script or another paths line; key records signed by btclib in each header form, with or without damage, and Faraday's own record of random seeds |
| `taproot` | `bitcoin`'s `TaprootBuilder` and `TaprootSpendInfo` (as `faraday_bip::tapmulti` and `miniscript`'s `tr()` build trees), `ScriptBuf::new_p2tr` as `faraday_psbt::sign` checks a key path, `Address::p2tr_tweaked`, `ControlBlock` decoding and `verify_taproot_commitment` | Core's `taproot_construct`, `key.py`'s tweak and `segwit_addr.py`; BIP-341's script-path check written out in `area_taproot.py` | BIP-341's `scriptPubKey` wallet vectors, all seven (`tools/vectors/sighash/`) | trees of 0 to 12 leaves in random shapes with random scripts and leaf versions over random internal keys: leaf hashes, Merkle root, tweak, output key and parity, script, address, control blocks; each control block checked, then changed eleven ways; internal keys that are no point, odd leaf versions and 0x50 refused |
| `miniscript` | `faraday_bip::policy::WalletPolicy` (a wallet's descriptor and the script it pays at an index), `faraday_bip::policy::parse_script_descriptor`, a witness script read back as `faraday_psbt` reads one | Bitcoin Core v31.1's `bitcoind` on regtest (`deriveaddresses`) for random wallets, their cosigners' keys from btclib (`btclib/`, on btclib's own curve); Antoine Poinsot's python-bip380 (`miniscript/`) for Core's vectors that state no script | Bitcoin Core's miniscript unit tests, 97 miniscripts in P2WSH and Tapscript (`tools/vectors/miniscript/`) | wallets of the Liana, Nunchuk, Sparrow and recovery shapes in `wsh()` and `sh(wsh())`, `multi`/`sortedmulti` in `sh()`, `wsh()`, `sh(wsh())`, `tr()` with trees of one to six leaves or a lone `sortedmulti_a`, over random accounts on every network, at random indices of both chains; the same made not sane, refused |
| `msg` | `faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids) | Bitcoin Core v31.1 and btclib for BIP-137, btclib-wallet and btcd for BIP-322: Core's `signmessagewithprivkey`, `verifymessage` and `signrawtransactionwithkey` (the low-R P2WPKH witness); btclib's `bms` and btclib-wallet's `bip322` (`btclib/`, btclib's own curve); python-ecdsa (the distribution's package) for the first-nonce ECDSA signature of BIP-137; btcd's `bip322` from pull request 2521 (`btcd-bip322/`, Go) | BIP-322's basic and generated vectors (hashes, txids, simple, full and proof-of-funds signatures, errors) and btcd's negative vectors; Trezor's and Electrum's BIP-137 signatures (`tools/vectors/message/`), Trezor's made again from its keys | random keys, address forms, networks and messages: virtual txids; signatures made byte for byte under both nonce rules and checked by every reference, then checked against a changed message, another address, a changed byte and a moved header; btcd's own simple signatures checked by Faraday |
| `own-randomness` | `faraday_core`'s `fresh.rs` (`Own`, the session seed and pool) through the testkit's `fresh` and `seed_draw`, with `faraday_entropy`'s dice, coin, camera and pointer-and-keys commitments | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3 written out in `area_own.py` over `hashlib` | the known answers of `faraday/faraday-core/tests/all/own_randomness.rs`: nothing added, dice before and after the system's answers, dice then coins, a batch of pointer moves and keys, a lock | random sessions of system answers, dice, coins, camera frames (some refused, some empty) and pointer moves and keys (none to several batches), locks, seed draws, fresh bytes and counts, in any order |
| `entropy` | `faraday_entropy`: dice under the three procedures, coins, hex, cards, camera frames, the device's answer, a mix | the procedures written out in `area_entropy.py` over `hashlib`, from `tools/reference/entropy/README.md` and the dice sources quoted in `tools/reference/dice/README.md`, except that the hashed dice procedure and every rolls commitment are Coldcard's own `rolls.py` and `rolls12.py` (`tools/reference/coldcard-rolls/`, run, public domain) | Coldcard's `rolls.py` digest, EntropyLab's six-as-zero and direct word selection (`tools/vectors/entropy/`) | runs of every source at every strength, short of the count, at it and past it, with faces, cards, frames and characters the pads refuse and every source past its limit; mixes of one to six sources given by their inputs |
| `roller` | Faraday Demo's dice rollers (`faraday_core`'s `fresh/roller/`, through the testkit) | `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` | every byte value as an answer: each face from 42 bytes, the four above 251 skipped; every byte as a face of every die of 2 to 20 faces (`faraday_entropy::uniform_face`); the booklet's 32 paths on six- and twenty-faced dice and its order with an equal pair (`roller.sheet`) | random answers (some mostly skipped bytes) under clicks of every size into entries of random room, with resets; the five dice of random faces (2 to 20) under clicks of every reach into codex32 shares of random count and length, answers running out part way; rolls of random dice entered by hand, equal pairs among them |
| `seedqr` | `faraday_codec::seedqr` (SeedQR, CompactSeedQR), `faraday_codec::qr`, the encoder that draws them, and `faraday_codec::qr::codex32qr` with the picture `faraday_core::picture` writes of it | SeedSigner's own encoders and `DecodeQR` (`seedsigner/`, unchanged, with embit's BIP-39 code in `embit/`); Project Nayuki's `qrcodegen.py` (`qrcodegen/`), module for module | SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and picture of the Codex32QR profile (`tools/vectors/codex32qr/`) | keys of every length and language, both codes and the words read back; digits and bytes malformed; payloads in each mode at each level and version limit; 48-character codex32 strings in either case as Codex32QR codes and the pictures written of them read back, other lengths refused |
| `bbqr` | `faraday_qr::bbqr`, `faraday_qr::specter` (numbered text parts) | Coinkite's BBQr library (`bbqr/`, with a one-table pyqrcode shim checked against `qrcodegen.py`); SeedSigner's own `SpecterWalletQrDecoder` (`seedsigner/`, unchanged) | none published beyond the specification's examples | files of every type and size split by Coinkite in each encoding and read in any order, with repeats, a part missing, another transfer's part, a conflicting repeat and a part past the count; what Faraday writes read back by `join_qrs`; numbered parts written, read in any order and refused when malformed |
| `ur` | `faraday_codec::ur` (single-part URs, the animation's parts, the decoder) | Foundation Devices' foundation-ur-py (`ur/`), URKit ported | URKit's single-part and multipart encoder vectors (`tools/vectors/ur/`) | messages as `crypto-psbt` and `bytes` at random fragment lengths, parts byte for byte; sets with fragments dropped, mixes added, shuffled, repeated and upper case, put together by both; URs of other types written, read, changed and cut |
| `vanity` | `faraday_bip::vanity` (the counter, prefixes and their expected work, the search on both dials) | EntropyLab's counter as `tools/reference/vanity/README.md` quotes it, written out in `area_vanity.py` over python-mnemonic, each candidate's address by Bitcoin Core v31.1 (`deriveaddresses` on regtest, from btclib's master key) and Core's framework, as the `addr` area writes it | EntropyLab's two finds | counters at every width's edges; prefixes of every kind on every network, valid and not; searches of random keys on both dials from random starting counters, with budgets ending before a match and after |
| `kdbx` | `faraday_backup::kdbx` (the KDBX 4 export and its own check) | pykeepass, the distribution's package (`python3-pykeepass`) | the repository's export (`tools/vectors/backup/`), written again byte for byte and opened | databases of one to eight items with markup characters, control characters, line ends and several scripts, at small Argon2id costs, every entry read back; the writer's check against pykeepass under the right passphrase, a wrong one and a changed byte |
| `lightning` | `faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read) | LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one process answering the run's requests; ldk-node's derivation as `tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` over btclib's BIP-32 (`btclib/`, on btclib's own curve) | LND's five cipher seeds, LND's two node keys, ldk's five node ids (`tools/vectors/aezeed/`) | random BIP-39 seeds and entropies on every network; random words, a wrong version with a right checksum, published seeds under a changed passphrase, a changed enciphered byte or salt with the checksum put right, a changed word, each refused by LND's code |

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
  reference as well as of Faraday; all 45 of its own vectors are run.
- `codex32`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; three rules BIP 93 states that the
  reference does not enforce (a new secret's threshold, the master
  seed's sizes, the order shares are handed out) are written in the area
  from the BIP's text, and the worksheet's rows from BIP 93's polymod
  alone.
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
  second BIP-32 master.
- `bip32`: the wheels are hashed against `references.sha256` before any
  run; `check_driver` runs btclib against BIP-32's vectors 1 to 5 and
  the invalid keys of vector 5 before anything else.
- `addr`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; every descriptor checksum is also
  asked of BIP 380's own code, as the BIP prints it; Core's release
  tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked.
- `tx`: its vendored files are hashed against `references.sha256` before
  any run, and a published vector's answer is required of the reference
  as well as of Faraday; each published signature hash is checked
  against the reference first.
- `psbt`: Core's release tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked; HWI's vendored files are hashed against
  `references.sha256` before any run; a published vector's answer is
  required of the side that decides. Not compared: whether a
  `PSBT_GLOBAL_VERSION` of 0 is written: Core reads a version 0 PSBT
  alike with it or without it, and Faraday writes it back as read.
- `sign`: Core is the pinned release, hashed against its pin before
  every run; the vendored files are hashed against `references.sha256`;
  a published vector's answer is required of Core as well as of Faraday;
  every ECDSA and BIP-340 signature Core makes is checked to verify
  under the hash the area computes before that hash is used for
  Faraday's deterministic nonces.
- `psbt-review`: Core's release tarball is checked against its pinned
  SHA-256 (`tools/reference/core/release.pin`) before every run, and the
  node's version is asked; every PSBT is one Core made and read, and
  every change decision is one of Core's derived scripts. Not compared:
  Faraday's other warnings (high fee, dust, address reuse, sighash,
  network, locktime) and thresholds, which are Faraday's own rules; the
  estimated virtual size and fee rate (Core's estimate needs signing
  data the PSBT does not carry).
- `import`: Core's release tarball against its pinned SHA-256 before
  every run; btclib's wheels against the hashes btclib-wallet's lock
  file names (`references.sha256`), with libsecp256k1 off; BIP 129's own
  records and the files Sparrow, Nunchuk, Coldcard and Core wrote,
  judged in the prelude. Not compared: the wallet's name a file carries;
  Electrum's encrypted files and BSMS's encrypted records, which Faraday
  refuses by name; a silent payments record; the kind of a refusal.
- `taproot`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; each of BIP-341's wallet vectors is
  checked against the reference first.
- `miniscript`: its vendored files are hashed against
  `references.sha256` before any run, and a published vector's answer is
  required of the reference as well as of Faraday; Core's release
  tarball is checked against its pinned SHA-256
  (`tools/reference/core/release.pin`) before every run, and the node's
  version is asked.
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
- `own-randomness`: the model is written in the area from
  `docs/OWN-RANDOMNESS.md` §2.3 over `hashlib`, and the known answers of
  `faraday/faraday-core/tests/all/own_randomness.rs` are required of it
  as well as of Faraday.
- `entropy`: the hashed dice procedure is Coldcard's own scripts run,
  their files hashed against `references.sha256` before any run; the
  rest is written in the area over `hashlib` from the statements of
  `tools/reference/entropy/README.md` and
  `tools/reference/dice/README.md`, and the published dice answers are
  required of them as well as of Faraday.
- `roller`: the model is written in the area from `docs/CODEX32.md` §5.5
  and §5.6 and the booklet's worksheet, and the known answers, the byte
  distribution and every byte as a face of every die are required of it
  as well as of Faraday.
- `seedqr`: its vendored files are hashed against `references.sha256`
  before any run, and a published vector's answer is required of the
  reference as well as of Faraday; SeedSigner's reader is looser than
  Faraday (it takes 49 digits, a sign inside a group, 20 to 28 bytes,
  and does not check the checksum of digits), so Faraday's own rule, the
  lengths and the digits, stands in front of it in `area_seedqr.py` and
  the checksum is embit's; SeedSigner runs in English for every
  language, the indices being the same; the profile's own pictures are
  read from their pixels and must be Faraday's code module for module.
- `bbqr`: `check_capacities` runs first, the pyqrcode shim's table of
  version capacities against `qrcodegen.py`'s; its vendored files are
  hashed against `references.sha256` before any run, and a published
  vector's answer is required of the reference as well as of Faraday;
  where Faraday refuses what SeedSigner's decoder files or misfiles (a
  part number of 0 or past the count, a repeat with other text, a header
  in upper case or missing, a count over 1,000) the refusal is written
  in the area, in front of the decoder.
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
  ldk-node's derivation is written in the area from the sources the
  README quotes, and ldk's published answers are required of it as well
  as of Faraday. Not compared: ldk-node's node key is compared with a
  derivation written here, not with ldk's code (Rust, not vendored):
  only ldk's five published answers are outside it; LND's key path
  numbers are written in the Go program from `keychain/derivation.go`,
  as LND derives through btcwallet, which is not vendored.
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
    which the reference leaves to its caller, is written in the area
    from the BIP's own sentence.
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
- `slip39`
  - SLIP-39: Faraday refuses a group threshold of 1 over several groups;
    python-shamir-mnemonic makes the shares. The area checks that this
    still holds, so a change either way is seen.
- `codex32`
  - Codex32: Faraday recovers only from exactly `k` shares without the
    secret, and derives a share only from exactly `k` strings at an
    unused index; the reference also takes more strings or a used index.
  - The codex32 threshold and seed sizes, which the reference leaves to
    its caller, are written in the area from the BIP's own sentences.
- `codex32-correct`
  - Codex32 correction: python-codex32 reads a character outside the
    bech32 alphabet as unreadable, and fills more than 8 unreadable
    boxes that are not one run when its equations allow; Faraday refuses
    both (`docs/CODEX32.md` §8.4). The corpus damages strings with
    bech32 characters and `?` only.
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
- `tx`
  - Legacy sighash: `bitcoin` hashes the script code as given, Core
    removes every OP_CODESEPARATOR first. Faraday's legacy spends
    (P2PKH, P2SH multisig) have none.
  - A raw transaction whose witness stack counts more items than the
    transaction has bytes is refused in the area: Core's framework reads
    past the end as zeros and would take hours to write such a stack
    back. Faraday's full reading refuses it too, since every item is
    written back as at least one byte, and the harness's own tests check
    the shortcut.
- `psbt`
  - Version 0 made version 2: HWI's `convert_to_v2` leaves out the
    `PSBT_OUT_SCRIPT` of an output whose script is empty, which BIP-370
    requires; Faraday writes it, and the rest of the PSBT must be the
    same.
  - Bitcoin Core reads a version 0 PSBT with a version 2 field as valid,
    the field as unknown; Faraday refuses it, as BIP-370 does, and so
    does HWI, which decides it.
  - A taproot signature of 65 bytes whose sighash byte is 0, which
    neither Core nor HWI refuses, and bytes after a version 2 PSBT's
    last map or a stream that ends inside one, which HWI reads: Faraday
    refuses each, as BIP-341 and BIP-174 do, and the refusal is checked
    by a rule written in the area.
  - PSBT version 2 (`faraday_psbt::v2`): a BIP-370 key with key data is
    an unknown key; BIP-174's and BIP-371's fields inside are checked
    only when the PSBT is bridged to version 0, except a taproot
    signature whose sighash byte is 0, refused on reading. HWI checks
    them on reading; the area requires HWI to read the PSBT without
    those pairs, to Faraday's locktime and transaction. Where HWI reads
    a value shorter than its length says and makes a version 0 PSBT of
    it, which Faraday does not, Core must refuse the version 0 PSBT of
    the PSBT's own pairs.
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
- `psbt-review`
  - Change of a script of several keys, a miniscript or a taproot tree:
    Faraday verifies it only for a registered wallet that holds a loaded
    key, and counts it as paid to others otherwise, which Core has no
    notion of. The area expects the same refusal, and does not compare
    the spoof warning for an unregistered wallet (Faraday's reading of
    such a claim depends on the account path it states).
- `import`
  - Still ours, expected from Faraday's rule: a SLIP-132 version that
    names another script than the file's (F155), a BSMS paths line other
    than /0/*,/1/* or No path restrictions, which Faraday reads as the
    two chains (F64), a BSMS descriptor that is not a template, a key
    record whose description is over eighty characters or whose token is
    not BIP 129's; and which account of a Coldcard export, which pair of
    a `listdescriptors`, and what each field of every format means, the
    area's reading of each format's documentation.
  - A key record's signature in any of BIP-137's header forms is
    accepted when it recovers the record's key: btclib is asked with the
    address that form names (Faraday reads no address from the record).
- `miniscript`
  - Miniscript: Faraday reads `sortedmulti_a` only as a `tr()`'s one
    leaf; BIP 387 allows it deeper in a tree, and Core reads it there.
    Two of Bitcoin Core's sane miniscripts are refused, as `miniscript`
    refuses them: `or_b(0,a:0)`, which has no satisfaction, and a `d:`
    used as a unit under Tapscript. Each is checked and counted.
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

## Who wrote each reference

Each action's cross-check reference by its independence class (F160):
code Faraday did not write; Faraday's own model over outside code, which
names the part Faraday wrote; or Faraday's own model only. Only the first
is an independent check. The table is written by `docs/crypto/gen.py`
from the `independence` table of each action in
`docs/crypto/actions.toml`, which the app's and the website's
Cryptography blocks read too: edit the manifest, then run
`python3 -I -B docs/crypto/gen.py write`. `just lint` fails on an action
with an area and no class (`docs/crypto/check.py`).

<!-- crypto-manifest: begin independence -->
| Area | Reference | Written by | Faraday's own part | Actions |
|---|---|---|---|---|
| `bip39` | code Faraday did not write | python-mnemonic, by Trezor (SatoshiLabs) | none | words-read, words-make, seed, vault-load |
| `bip32` | Faraday's own model over outside code | btclib-wallet's `bip32`, by the btclib developers, on btclib's own curve | which versions a strict reader takes: BIP-32's xprv and tprv, xpub and tpub, from btclib's network table (btclib reads SLIP-132's too), in `bip32_ref.py` | master |
| `bip32` | code Faraday did not write | btclib-wallet's `bip32`, by the btclib developers, on btclib's own curve | none | derive, multisig-account, xprv-shown, wif-shown |
| `codex32` | Faraday's own model over outside code | python-codex32, by Ben Westgate | three rules BIP 93 states that python-codex32 does not enforce (a threshold of 0 or 2 to 9, the seed's lengths, the order shares are handed out), written in `area_codex32.py` from the BIP's text | seed-shown, codex32 |
| `bip39` | code Faraday did not write | python-mnemonic, by Trezor (SatoshiLabs), and Python's `hashlib` and `hmac` (OpenSSL) | none | hashes-macs |
| `lightning` | code Faraday did not write | LND's `aezeed` package, by Lightning Labs, over `golang.org/x/crypto`'s scrypt, vendored in `tools/reference/lnd/` | none | scrypt |
| `addr` | code Faraday did not write | Bitcoin Core v31.1 itself (`deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; btclib-wallet's `bip32` and `slip132`, by the btclib developers, on btclib's own curve, for the account keys; BIP 380's own checksum code, as the BIP prints it; python-mnemonic, by Trezor (SatoshiLabs), for the seeds | none | address |
| `miniscript` | code Faraday did not write | Bitcoin Core v31.1 itself (`deriveaddresses`) and its functional-test framework, by Bitcoin Core's contributors; python-bip380, by Antoine Poinsot, for Core's vectors that state no script; btclib-wallet's `bip32`, by the btclib developers, for the cosigners' keys | none | descriptor, recovery |
| `taproot` | Faraday's own model over outside code | Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | BIP-341's script-path check of a control block, written in `area_taproot.py` | tapmulti |
| `import` | Faraday's own model over outside code | Bitcoin Core v31.1 (`deriveaddresses`, and BIP-380's checksum in its `descriptors.py`), by the Bitcoin Core developers, for every wallet's addresses and checksums; btclib-wallet's BIP-32 and btclib's SLIP-132 versions, by the btclib developers, for the keys; Faraday's authors (`area_import.py`) for the files | the files themselves and what each format's fields mean (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, Keystone's, Wasabi's, Specter Desktop's, Core's `listdescriptors`), written in `area_import.py` from each format's documentation, which account of a Coldcard export and which pair of a `listdescriptors` is the wallet, and F155's SLIP-132 refusals | wallet-files |
| `import` | Faraday's own model over outside code | btclib's `bms` and btclib-wallet's BIP-32, by the btclib developers, for every key record's signature, made, verified and written; Bitcoin Core v31.1 (`deriveaddresses`), by the Bitcoin Core developers, for a descriptor record's addresses; Faraday's authors (`area_import.py`) for the record's rules | BIP 129's record lines read in `area_import.py`, and the rules no outside code has: the paths line (/0/*,/1/* or No path restrictions, read as the two chains, F64), a descriptor that is not a template, a description over eighty characters, a token not BIP 129's, and the header forms a signature is accepted in | bsms |
| `musig` | code Faraday did not write | BIP-327's `reference.py`, by the BIP's authors, and btclib, by the btclib developers, on its own curve, which verifies the aggregate signature as BIP-340 | none | musig-key, sign-musig |
| `addr` | code Faraday did not write | btclib-wallet's `slip132` and `bip32`, by the btclib developers | none | xpub-spellings |
| `addr` | code Faraday did not write | Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | none | address-read |
| `entropy` | Faraday's own model over outside code | Coldcard's `rolls.py` and `rolls12.py` (public domain), run for the hashed procedure and every rolls commitment; Faraday's authors (`area_entropy.py`) for the rest | EntropyLab's six-as-zero and direct word selection (JavaScript, which needs Node, written out in `area_entropy.py` from `tools/reference/dice/README.md`), the cut of Coldcard's digest to 20, 24 and 28 bytes, and the roll counts and refused faces | dice |
| `entropy` | Faraday's own model only | Faraday's authors (`area_entropy.py`), over Python's `hashlib` | the packing of flips and digits, written out in `area_entropy.py` | coins-hex |
| `entropy` | Faraday's own model only | Faraday's authors (`area_entropy.py`), over Python's `hashlib` | the card procedure, written out in `area_entropy.py` | cards |
| `entropy` | Faraday's own model only | Faraday's authors (`area_entropy.py`), over Python's `hashlib` | the frames' commitment, written out in `area_entropy.py` | camera |
| `own-randomness` | Faraday's own model only | Faraday's authors (`area_own.py`), over Python's `hashlib` | `docs/OWN-RANDOMNESS.md` §2.1 and §2.3, written out in `area_own.py` | pointer-keys, fresh |
| `entropy` | Faraday's own model only | Faraday's authors (`area_entropy.py`), over Python's `hashlib` | the mix of commitments, written out in `area_entropy.py` | mix |
| `entropy` | Faraday's own model only | Faraday's authors (`area_entropy.py`), over Python's `hashlib` | the hash of the shell's answer, written out in `area_entropy.py`; with what the person added, as `area_own.py` writes it | device |
| `roller` | Faraday's own model only | Faraday's authors (`area_roller.py`) | the booklet's dice worksheet, written out in `area_roller.py` from `docs/CODEX32.md` §5.5 | booklet |
| `roller` | Faraday's own model only | Faraday's authors (`area_roller.py`) | Faraday Demo's roller, written out in `area_roller.py` from `docs/CODEX32.md` §5.6 | roller |
| `bip85` | code Faraday did not write | btclib-wallet's `bip85`, by the btclib developers, on btclib's own curve; ethankosakovsky/bip85, by Ethan Kosakovsky, over python-mnemonic, by Trezor (SatoshiLabs), and btclib through a pycoin shim | none | bip85 |
| `vanity` | Faraday's own model over outside code | Faraday's authors (`area_vanity.py`) for the search, over python-mnemonic, Bitcoin Core v31.1 (`deriveaddresses`) and btclib for each candidate's address | EntropyLab's counter, the prefix rules, the expected work and the search loop, written out in `area_vanity.py` from a quotation | vanity |
| `lightning` | code Faraday did not write | Lightning Labs (LND), Yawning Angel (AEZ) | none | aezeed |
| `lightning` | Faraday's own model over outside code | LND's node key by btcd's `hdkeychain` in LND's vendored tree, by Lightning Labs and the btcd developers, for LND; Faraday's authors (`area_lightning.py`), over btclib-wallet's `bip32` and python-mnemonic, for ldk-node | ldk-node's node key, composed in `area_lightning.py` from the sources `tools/reference/aezeed/README.md` quotes (each step btclib's), and LND's key-path numbers (1017', coin, 6', 0, 0), written in `tools/reference/lnd/main.go` from `keychain/derivation.go` | lightning-node |
| `sp` | code Faraday did not write | BIP-352's `reference.py`, by the BIP's authors, over `secp256k1lab` | none | silent-receive |
| `psbt` | Faraday's own model over outside code | Bitcoin Core v31.1 (`decodepsbt`, and `combinepsbt` as its writer), by Bitcoin Core's contributors; HWI's `psbt.py`, by Ava Chow and HWI's contributors, for the version 2 fields BIP-370 refuses in version 0 | the refusal of a 65-byte taproot signature whose sighash byte is 0 (BIP-341), which neither Core nor HWI makes, written in `area_psbt.py` | psbt-read |
| `psbt` | Faraday's own model over outside code | HWI's `psbt.py` (reading, `compute_lock_time`, `get_unsigned_tx`, `convert_to_v0`, `convert_to_v2`), by Ava Chow and HWI's contributors, with both version 0 forms read by Bitcoin Core v31.1 | the refusals HWI does not make (bytes after the last map, a stream that ends inside a map, a 65-byte taproot signature whose sighash byte is 0), the choice of the pairs Faraday leaves to the bridge to version 0, and the version 0 PSBT of a PSBT's own pairs that Core judges where HWI repairs a short value, written in `area_psbt.py` | psbt-v2 |
| `psbt-review` | Faraday's own model over outside code | Faraday's authors (`area_review.py`), over Bitcoin Core v31.1 (`decodepsbt`, `analyzepsbt`, `deriveaddresses`), by the Bitcoin Core developers | the rule that an output is change when it claims a held key's origins and Core's script at the path it states is the output's own, the rule that a script of several keys, a miniscript or a taproot tree is change only for a registered wallet, and the account keys a registered wallet names, derived in the area | psbt-review |
| `tx` | Faraday's own model over outside code | Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | BIP-341's refusal of SIGHASH_SINGLE with no output and of a hash type it does not list, which the framework hashes anyway, written in `area_tx.py` | sighash-review |
| `sign` | code Faraday did not write | the Bitcoin Core developers: Core's key_io vectors (`tools/vectors/wif`), and in the cross-check Core v31.1 itself, which signs from descriptors of the same WIF what Faraday reads from it | none | wif-read |
| `sign` | code Faraday did not write | the Bitcoin Core developers: Core v31.1 itself signs the same sweep from `pkh()`, `wpkh()` and `sh(wpkh())` descriptors of the WIF (`descriptorprocesspsbt`), finalizes it and judges it (`testmempoolaccept`); BIP-143's published P2SH-P2WPKH example signs to its transaction | none | sweep-sign |
| `sign` | Faraday's own model over outside code | the Bitcoin Core developers: Core v31.1 itself (`descriptorprocesspsbt` signs, `testmempoolaccept` judges the spend), and Core's `key.py` and `script.py` for the first RFC 6979 nonce, and python-ecdsa, by Brian Warner and Hubert Kario (no Bitcoin code), which makes that nonce's signature again byte for byte and verifies the low-R ones; Faraday's authors (`area_sign.py`) for the refusals | Faraday's refusals (F136, F141) and the inputs a wallet's rule leaves unsigned, the script code of the hash the first RFC 6979 nonce is checked under, in `area_sign.py`; the keys of those checks are btclib's | sign-ecdsa |
| `sign` | Faraday's own model over outside code | the Bitcoin Core developers: Core v31.1 itself (`descriptorprocesspsbt` signs, `testmempoolaccept` judges the spend), with BIP-340's reference (`secp256k1lab`) and Core's `script.py` for the zero-randomness nonce, and btclib, by the btclib developers, on its own curve, which verifies each signature and makes it again byte for byte; Faraday's authors (`area_sign.py`) for the refusals | Faraday's refusals (F136, F141) and the inputs a wallet's rule leaves unsigned, in `area_sign.py`; the keys of the zero-randomness check are btclib's | sign-schnorr |
| `sign` | Faraday's own model over outside code | the Bitcoin Core developers: Core v31.1 itself (`finalizepsbt` finalizes and extracts, `testmempoolaccept` judges the transaction over its funding on the node); Faraday's authors (`area_sign.py`) for what is taken off first | the signatures under a taproot `pkh` leaf taken off before Core finalizes, as Faraday leaves such a leaf to the coordinator, and the fields BIP-174 keeps on a finalized input, in `area_sign.py` | finalize |
| `msg` | code Faraday did not write | Bitcoin Core v31.1 (`signmessagewithprivkey`, `verifymessage`, `signrawtransactionwithkey`); btclib's `bms` and btclib-wallet's `bip322`, by the btclib developers, on btclib's own curve; btcd's `bip322` (pull request 2521), by Oliver Gugger and the btcd developers; python-ecdsa, by Brian Warner and Hubert Kario, for the first-nonce ECDSA signature of BIP-137 | none | message-sign, message-verify |
| `tx` | Faraday's own model over outside code | Bitcoin Core's functional-test framework, v31.1, by Bitcoin Core's contributors | the rule that a raw transaction is read only when its bytes are exactly the serialisation of what was read, where the framework reads past a short input, written in `area_tx.py` | tx-decode |
| `seedqr` | code Faraday did not write | Project Nayuki's `qrcodegen.py`, by Nayuki | none | qr, codex32qr |
| `ur` | code Faraday did not write | foundation-ur-py, by Foundation Devices | none | ur |
| `bbqr` | Faraday's own model over outside code | Coinkite's BBQr library, by Coinkite, for BBQr; SeedSigner's `SpecterWalletQrDecoder`, by the SeedSigner developers, for the numbered parts | Faraday's refusals written in `area_bbqr.py` in front of SeedSigner's decoder (a part number of 0 or past the count, a repeat with other text, a header in upper case or missing, a count over 1,000), the writing of numbered parts, and a one-table pyqrcode shim | bbqr |
| `seedqr` | Faraday's own model over outside code | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki | the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define (the entropy itself) | seedqr-make |
| `seedqr` | Faraday's own model over outside code | SeedSigner's encoders and `DecodeQR`, by the SeedSigner developers, over embit's BIP-39 code, by Stepan Snigirev, and Project Nayuki's `qrcodegen.py`, by Nayuki | Faraday's rule written in `area_seedqr.py` in front of SeedSigner's reader (exactly 48 or 96 digits, 16 or 32 bytes), and the CompactSeedQR of 15, 18 and 21 words, which SeedSigner does not define | seedqr-read |
| `codex32` | Faraday's own model over outside code | python-codex32, by Ben Westgate | the worksheets' rows, written in `area_codex32.py` from python-codex32's polymod | codex32-make |
| `codex32-correct` | Faraday's own model over outside code | python-codex32, by Ben Westgate (its `correction.py`) | the bound BIP 93 states (substitutions and unreadable boxes, one run up to the checksum's length) and a held string's fixed header, written in `area_codex32_correct.py` from the BIP's sentence | codex32-correct |
| `slip39` | code Faraday did not write | python-shamir-mnemonic, by Trezor (SatoshiLabs) | none | slip39 |
| `xor` | Faraday's own model over outside code | Coldcard's `xor` and `xor_split`, by Coinkite, over python-mnemonic, by Trezor (SatoshiLabs) | the count of parts (2 to 4) and the five BIP-39 lengths, Faraday's rule written in `area_xor.py` in front of Coldcard's `xor`, which asserts only equal lengths | xor |
| `sps` | code Faraday did not write | BIP-374/375 authors (bitcoin/bips) | none | silent-send |
| `vault` | Faraday's own model over outside code | Faraday's authors (`area_vault.py`), over argon2-cffi (the Argon2 designers' reference code), PyNaCl (libsodium) and PyCA `cryptography` (OpenSSL, for HKDF-SHA256 and ChaCha20) | the container, its slot layout, key schedule and records, written in `area_vault.py` from `docs/VAULT.md` | vault-create, vault-unlock, vault-seal, vault-edit |
| `kdbx` | code Faraday did not write | pykeepass, by the libkeepass project (the distribution's package) | none | kdbx |
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
