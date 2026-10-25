# The crypto cross-check

F79, `docs/MULTIVENDOR.md` §10. Faraday's own Rust answers requests, and
independent pure-Python references recompute every answer, over every
published vector and a corpus of random inputs drawn from a fixed seed.
The first disagreement stops the area and is printed with its inputs
and both answers.

## Running it

    just crosscheck              the stamp run: fixed seed, about 3 minutes
    just crosscheck --fresh      before a release: a new seed, ten times the cases
    just crosscheck --area musig one area (no stamp is written)
    just crosscheck --seed S     reproduce a run

`run.sh` builds the Rust command with `cargo build --frozen` and runs
`crosscheck.py` in a network namespace of its own (`unshare --user
--net`), so nothing can be fetched. `crosscheck.py` refuses to run with
a network interface up, and refuses when a vendored reference does not
match `references.sha256`, or when an unlisted `.py` file sits in a
reference folder.

A passing full run writes `stamp`. `tools/lint-crosscheck.sh` (in `just
lint`, about a tenth of a second) hashes the crypto sources, this
folder, the references, the vectors and the crypto crates' `Cargo.lock`
entries, and fails with "crypto changed since the last cross-check: run
`just crosscheck`" when they no longer match the stamp. Commit the stamp
with the change that made it pass. `python3 stamp.py show` lists what is
hashed.

Under `--fresh`, a disagreement is kept in `found/` with its seed and
inputs, to become a vector.

## The self-test

Before every run, each area runs against the Rust command started with
`FARADAY_CROSSCHECK_SABOTAGE=<op>`, which changes every answer to that
one operation. The run fails unless every area reports that operation
as a disagreement: a cross-check that cannot fail is worse than none.

## What runs

| Area | Faraday | Reference (`tools/reference/`) | Published vectors | Random corpus |
|---|---|---|---|---|
| `musig` | `faraday_bip::musig` | BIP-327's `reference.py` (`bip327/`) | all eight files of `tools/vectors/bip327/` | sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial signatures, their verification (and a tampered one), aggregation checked as BIP-340, deterministic signing |
| `frost` | `faraday_bip::frost` | BIP 445's `frost_ref` (`bip445/`) | all six files of `tools/vectors/bip445/` | the dealer against the reference's public-share interpolation, then signing sessions as for MuSig2 |
| `sp` | `faraday_bip::silent` | BIP-352's `reference.py` (`bip352/`) | every case of `tools/vectors/bip352/`, both sides | inputs of every kind BIP-352 reads, receivers with labels, a sender's outputs and what each receiver finds among them and decoys |
| `bip85` | `faraday_bip::bip85` | `ethankosakovsky/bip85` over python-mnemonic and a hardened-BIP-32 shim (`bip85/`, `bip39/`) | every application vector of `tools/vectors/bip85/` | random master keys, indices, languages, word counts and lengths, with refusals |
| `slip39` | `faraday_bip::slip39` | python-shamir-mnemonic (`slip39/`) | all 45 of `tools/vectors/slip39/` | splits fed the same random bytes, which must match word for word; recovery from qualifying and short subsets and a changed word; the passphrase cipher |
| `codex32` | `faraday_bip::codex32` | python-codex32 (`codex32/`); the worksheet's rows written in the area from its polymod | every vector of `tools/vectors/bip93/`, the 55 invalid strings included; volvelle-website's worksheet cells (`tools/vectors/codex32-volvelle/`); the booklet's checksum, derivation, recovery, addition, translation and fusion tables (`tools/vectors/codex32-booklet/`); the Codex32QR profile's sets (`tools/vectors/codex32qr/`) | secrets, splits (the same randomness), recovery, derived shares, corrupted strings; fresh sets of every threshold up to 31 strings at 48, 74 and 127 characters, the checksum worksheets of their strings and of corrupted ones, checksums made on a sheet, translation and recovery symbols, translation worksheets |
| `codex32-correct` | `faraday_bip::codex32::correct` | python-codex32's decoder, `correction.py` at a later commit (`codex32/correction/`), asked with BIP 93's bound | python-codex32's frozen correction corpus, 57 cases (`tools/vectors/codex32-correction/`) | random strings of every length with substitutions and unreadable boxes up to and past the bound, runs of unreadable boxes around the checksum's length, a held string's header fixed, upper case, `ms1` damaged |
| `xor` | `faraday_entropy::SeedXor` with `faraday_bip::bip39` | python-mnemonic and a byte XOR (`bip39/`) | Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's two | splits and combines in all ten languages and five lengths, with refusals |
| `vault` | `faraday_vault` (create, open, seal, the slot's records and payload layouts), `faraday_backup::argon2id`, the `chacha20poly1305` crate's detached XChaCha20-Poly1305 as the vault calls it | `docs/VAULT.md` written out in `area_vault.py`, over argon2-cffi and PyNaCl (the distribution's packages), with HKDF-SHA256 and the ChaCha20 block written over the standard library | RFC 5869's three SHA-256 cases, RFC 8439's five block vectors, draft-irtf-cfrg-xchacha-03's AEAD vector, the Argon2id known answer (`tools/vectors/vault-primitives/`); the two vaults and the slot vectors of `tools/vectors/vault/` | Argon2id at small random costs and the costs it refuses; the AEAD at random lengths with one byte changed; payloads made, read and malformed; random records of all thirteen types (type 12 is retired and refused) encoded, decoded and broken a rule at a time; whole vaults of every slot size made byte for byte from a seed, opened by each passphrase, resealed, and refused for each header fault, a wrong passphrase, a changed byte, two slots under one key and faulty contents |
| `bip39` | `faraday_bip::bip39` | python-mnemonic (`bip39/`) | the ten wordlists; every vector of `tools/vectors/bip39/vectors.json` in Faraday's languages; the Japanese vectors with non-ASCII passphrases | entropy to words and back in every language and length; seeds under random ASCII passphrases; a changed word, an unknown word, a word too many or too few; every valid last word |
| `bip32` | `faraday_bip::keys` (master, private derivation), `faraday_bip::xkey` (strict decoding), `bitcoin`'s `Xpub::derive_pub` | `btc.py`, BIP-32 written from the BIP over `secp256k1lab` and `ripemd160.py` (`bip352/`) | BIP-32's vectors 1 to 5 (`tools/vectors/bip32/`), python-mnemonic's master keys | seeds of 16 to 64 bytes on every network, random paths with every index, public derivation against private, keys broken one rule at a time and the master-key import |
| `addr` | `faraday_bip::address::parse`, `bitcoin`'s `Address::from_script`, `faraday_bip::account` (account xpub, SLIP-132, descriptor, addresses), `faraday_bip::descriptor`'s checksum | Bitcoin Core's functional-test framework (`core/`): `segwit_addr.py`, Base58Check, `descriptors.py`; BIP 380's own checksum code as the BIP prints it (`bip380/`); `btc.py`; BIP-86's tweak written in the area | BIP-350's and BIP-173's addresses, BIP-380's checksum cases, BIP-49's, 84's and 86's accounts (`tools/vectors/addresses/`) | scripts of every address kind on every network, written and read back, then changed; accounts of every type at random indices; random descriptors' checksums |
| `tx` | `faraday_psbt::transaction::read_raw`, `bitcoin`'s `SighashCache` as `faraday_psbt::sign` calls it | Core's `messages.py` and `script.py` (`core/`) | Core's 500 legacy sighashes, BIP-143's thirteen, BIP-341's key-path sighashes (`tools/vectors/sighash/`) | random transactions with and without witnesses (txid, wtxid, weight, bytes), cut or lengthened; every input hashed as legacy, BIP-143 P2WPKH and P2WSH, BIP-341 key and script path under every hash type |
| `psbt` | `faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` (version 2, Faraday's own) | BIP-174, 370 and 371 written out in `area_psbt.py`, over Core's `CTransaction` | every PSBT of BIP-174's, 370's and 371's test vectors (`tools/vectors/psbt-bips/`), with BIP-370's locktimes | random version 0 and 2 PSBTs with every field Faraday reads, taproot trees among them, written back byte for byte as read, then broken one rule at a time; BIP-370's locktime under every mix of required locktimes |
| `own-randomness` | `faraday_core`'s `fresh.rs` (`Own`, the session seed and pool) through the testkit's `fresh` and `seed_draw`, with `faraday_entropy`'s dice, coin and camera commitments | `docs/OWN-RANDOMNESS.md` §2.3 written out in `area_own.py` over `hashlib` | the known answers of `faraday/faraday-core/tests/all/own_randomness.rs`: nothing added, dice before and after the system's answers, dice then coins, a lock | random sessions of system answers, dice, coins and camera frames (some refused, some empty), locks, seed draws, fresh bytes and counts, in any order |
| `msg` | `faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids) | BIP-137 and BIP-322 written out in `area_msg.py`: ECDSA with RFC 6979 as libsecp256k1 states it, Core's `script.py` sighashes, `secp256k1lab`'s BIP-340 | BIP-322's hashes, txids, simple signatures and error cases; Trezor's and Electrum's BIP-137 signatures (`tools/vectors/message/`), made again from Trezor's keys | random keys, address forms, networks and messages: signatures made byte for byte under both nonce rules and both aux choices, checked, then checked against a changed message, another address, a changed byte and a moved header |
| `entropy` | `faraday_entropy`: dice under the three procedures, coins, hex, cards, camera frames, the device's answer, a mix | the procedures written out in `area_entropy.py` over `hashlib`, from `tools/reference/entropy/README.md` and the dice sources quoted in `tools/reference/dice/README.md` | Coldcard's `rolls.py` digest, EntropyLab's six-as-zero and direct word selection (`tools/vectors/entropy/`) | runs of every source at every strength, short of the count, at it and past it, with faces, cards, frames and characters the pads refuse and every source past its limit; mixes of one to six sources given by their inputs |
| `roller` | Faraday Desktop's dice rollers (`faraday_core`'s `fresh/roller/`, through the testkit) | `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` | every byte value as an answer: each face from 42 bytes, the four above 251 skipped; every byte as a face of every die of 2 to 20 faces (`faraday_entropy::uniform_face`); the booklet's 32 paths on six- and twenty-faced dice and its order with an equal pair (`roller.sheet`) | random answers (some mostly skipped bytes) under clicks of every size into entries of random room, with resets; the five dice of random faces (2 to 20) under clicks of every reach into codex32 shares of random count and length, answers running out part way; rolls of random dice entered by hand, equal pairs among them |
| `seedqr` | `faraday_codec::seedqr` (SeedQR, CompactSeedQR), `faraday_codec::qr`, the encoder that draws them, and `faraday_codec::qr::codex32qr` with the picture `faraday_core::picture` writes of it | SeedSigner's specification written out in `area_seedqr.py` over python-mnemonic (`bip39/`); Project Nayuki's `qrcodegen.py` (`qrcodegen/`), module for module | SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and picture of the Codex32QR profile (`tools/vectors/codex32qr/`) | keys of every length and language, both codes and the words read back; digits and bytes malformed; payloads in each mode at each level and version limit; 48-character codex32 strings in either case as Codex32QR codes and the pictures written of them read back, other lengths refused |
| `bbqr` | `faraday_qr::bbqr`, `faraday_qr::specter` (numbered text parts) | Coinkite's BBQr library (`bbqr/`, with a one-table pyqrcode shim checked against `qrcodegen.py`); SeedSigner's numbered-part reader written out in `area_bbqr.py` from `tools/reference/specter/README.md` | none published beyond the specification's examples | files of every type and size split by Coinkite in each encoding and read in any order, with repeats, a part missing, another transfer's part, a conflicting repeat and a part past the count; what Faraday writes read back by `join_qrs`; numbered parts written, read in any order and refused when malformed |
| `ur` | `faraday_codec::ur` (single-part URs, the animation's parts, the decoder) | Foundation Devices' foundation-ur-py (`ur/`), URKit ported | URKit's single-part and multipart encoder vectors (`tools/vectors/ur/`) | messages as `crypto-psbt` and `bytes` at random fragment lengths, parts byte for byte; sets with fragments dropped, mixes added, shuffled, repeated and upper case, put together by both; URs of other types written, read, changed and cut |
| `vanity` | `faraday_bip::vanity` (the counter, prefixes and their expected work, the search on both dials) | EntropyLab's counter as `tools/reference/vanity/README.md` quotes it, written out in `area_vanity.py` over python-mnemonic, `btc.py` and the `addr` area's address writing | EntropyLab's two finds | counters at every width's edges; prefixes of every kind on every network, valid and not; searches of random keys on both dials from random starting counters, with budgets ending before a match and after |
| `kdbx` | `faraday_backup::kdbx` (the KDBX 4 export and its own check) | pykeepass, the distribution's package (`python3-pykeepass`) | the repository's export (`tools/vectors/backup/`), written again byte for byte and opened | databases of one to eight items with markup characters, control characters, line ends and several scripts, at small Argon2id costs, every entry read back; the writer's check against pykeepass under the right passphrase, a wrong one and a changed byte |
| `lightning` | `faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read) | ldk-node's and LND's derivations as `tools/reference/aezeed/README.md` quotes them, and the aezeed's outer layer (CRC-32C, version), written out in `area_lightning.py` over `btc.py`; AEZ itself has no second implementation here | LND's five cipher seeds, LND's two node keys, ldk's five node ids (`tools/vectors/aezeed/`) | random BIP-39 seeds and entropies on every network; random words, a wrong version with a right checksum, published seeds under a changed passphrase, a changed enciphered byte or salt with the checksum put right, a changed word |

Where a reference leaves a rule to its caller, the area writes the
BIP's own sentence and says so in its docstring: the codex32 threshold
and seed sizes, the BIP-85 length ranges and its two password
applications, BIP-352's skip of a transaction spending SegWit version 2
or above.

### Known differences

Each is a place where Faraday refuses what the reference accepts; the
corpus keeps out of it and the area says why.

- SLIP-39: Faraday refuses a group threshold of 1 over several groups;
  python-shamir-mnemonic makes the shares. `area_slip39.py` checks that
  this still holds, so a change either way is seen.
- Codex32: Faraday recovers only from exactly `k` shares without the
  secret, and derives a share only from exactly `k` strings at an unused
  index; the reference also takes more strings or a used index.
- Codex32 correction: python-codex32 reads a character outside the
  bech32 alphabet as unreadable, and fills more than 8 unreadable boxes
  that are not one run when its equations allow; Faraday refuses both
  (`docs/CODEX32.md` §8.4). The corpus damages strings with bech32
  characters and `?` only.
- BIP-445: the vectors' tweak lists whose flag list has another length
  are skipped; each of Faraday's tweaks carries its own flag.
- BIP-352: Faraday's sender arithmetic is test-only and has no K_max,
  so the vector that exceeds it is checked on the receiving side only.
- BBQr: Faraday reads the file types `P`, `T`, `U` and `J` and frames
  of at most 4,096 characters; Coinkite's library reads every type BBQr
  defines and a version 40 part of 4,296 characters. `area_bbqr.py`
  checks both refusals and counts them; its corpus splits at version 39
  at most. Faraday also refuses a set whose parts are not one length but
  the last, which Coinkite never writes.
- Numbered parts: SeedSigner matches `P` and `OF` in upper case;
  Faraday does not (checked and counted).
- Codex32QR: Faraday picks the mask Nayuki's encoder picks; the
  profile's encoder chose another for 6 of its 22 pictures. There
  `area_seedqr.py` swaps the mask of Faraday's code for the picture's
  with `qrcodegen.py`'s masking, requires the picture, and counts it.
- KDBX: pykeepass strips the characters XML 1.0 cannot hold from a
  protected value after decrypting it; Faraday writes a Password's bytes
  as they are. `area_kdbx.py` expects pykeepass's reading and counts the
  items it differs on. pykeepass also returns from opening a file whose
  major version is neither 3 nor 4 having read nothing past it, no key
  derived and no hash checked; KeePass refuses such a file, and
  `area_kdbx.py` counts a file as opened only when pykeepass read a
  KDBX 4 body.
- UR: Faraday has no minimum fragment length, writes a one-fragment
  message as a part like any other, and refuses to animate an empty
  message; foundation-ur-py checks a UR type one character's way
  (`'a' <= type <= 'z'`). `area_ur.py` asks the reference with a minimum
  of 1, makes parts with its fountain encoder, and keeps its types to
  letters, digits and `-` not starting with `z`. Faraday's decoder
  reduces mixes by each other whichever arrived first and so sometimes
  finishes a set URKit's does not; the area then checks the message is
  the one sent and counts the set.

From batches 2 to 4 the areas meet these as they come and compare them
with Faraday's reading, so each is checked to still hold exactly, and
count them in the run's output ("N of them a known difference"):

- Legacy sighash: `bitcoin` hashes the script code as given, Core
  removes every OP_CODESEPARATOR first. Faraday's legacy spends (P2PKH,
  P2SH multisig) have none.
- PSBT version 2 (`faraday_psbt::v2`): a BIP-370 key with key data is
  an unknown key; BIP-174's and BIP-371's fields inside are checked only
  when the PSBT is bridged to version 0, except a taproot signature
  whose sighash byte is 0, refused on reading.
- BIP-137: Faraday reads the address form from the address rather than
  the header's range and tries a P2PKH address in both key encodings
  (`faraday_psbt::message`), as Electrum and Sparrow do; BIP-137 read
  strictly refuses Trezor's and Electrum's "no script type" signatures.

Where the two once differed and now agree (owner, 2026-10-23), the
areas check the agreement like any other answer: the vault's required
fields of types 7 and 9, a sheet past 16 KiB, an empty passphrase at
create, and duplicate type 14 and type 10 records, all refused by both
as `docs/VAULT.md` §7 and §8 now say; a BIP-39 sentence read in NFKD,
so composed words and the Japanese vectors as published parse on both
sides; a version 0 PSBT with a version 2 field, bytes after its last
map or a taproot signature whose sighash byte is 0 refused, and every
PSBT written back byte for byte as read (PSBT_OUT_TAP_TREE's order and
a PSBT_GLOBAL_VERSION of 0 included); a version 2 PSBT's output
without PSBT_OUT_SCRIPT (unless a silent payment's), invalid required
locktimes and wrongly sized locktime fields and modifiable flags
refused on reading.

## What the machine needs

`python3` 3.11 or later (the standard library only: `tomllib`,
`hashlib`), `unshare` from util-linux with unprivileged user namespaces
allowed, and the Rust toolchain. Nothing is installed with pip. The
`vault` area needs the distribution's Argon2 and libsodium bindings,
argon2-cffi and PyNaCl (Debian and Ubuntu: `python3-argon2`
`python3-nacl`), and the `kdbx` area the distribution's pykeepass
(`python3-pykeepass`); without them each stops and names them, and the
run fails. A passing run writes their versions into the stamp.

## Files

| File | What |
|---|---|
| `Cargo.toml`, `src/` | `faraday-crosscheck`, the Rust command: one JSON request a line in, one answer out; never shipped |
| `run.sh` | builds the command offline and runs `crosscheck.py` with no network |
| `crosscheck.py` | the self-test, the areas in parallel, the stamp |
| `common.py` | the Rust process, the comparison, the reference check |
| `area_*.py` | one area each |
| `btc.py` | Base58Check and BIP-32 written from the BIPs, which `bip32`, `addr`, `psbt` and `msg` share |
| `references.sha256` | every vendored file the areas import or read, with its SHA-256 |
| `stamp.py`, `stamp` | the stamp and its check |
