# What OpenSigner supports

The protocols, formats and features in the tree as of 2026-09-19, at
commit `eb71a68`. "Reads" means the device accepts it by scan, file,
paste or typing; "writes" means the device produces it as text, a QR or
a file. Where something is partly done, the row says what is missing.
The decisions behind each row are in `PLANNING.md` §16; the roadmap and
status column are in `PLANNING.md` §8.

Nothing here talks to a network. There is no networking code in any
crate or shell.

## Standards

| Standard | What it covers here | Status |
|---|---|---|
| BIP-32 | Hierarchical keys; master fingerprints; the key explorer's derivation at any path | Done |
| BIP-39 | Seed words, 12 to 24, all ten wordlists (English, Japanese, Korean, Spanish, Chinese Simplified and Traditional, French, Italian, Czech, Portuguese), each typed on a keyboard for its script; passphrases | Done |
| BIP-43/44/49/84/86 | Single-sig accounts: legacy, nested SegWit, SegWit, Taproot | Done |
| BIP-48 | Multisig accounts at `1h` (nested) and `2h` (native SegWit); `2h` is also used for a taproot multisig, since no standard assigns one | Done |
| BIP-85 | Child seed words (39'), HD-seed WIF (2'), extended private key (32'), hex (128169') and passwords in base64 (707764') and base85 (707785'), from the key page's BIP-85 row | Done |
| BIP-93 (Codex32) | A master seed as one checksummed string or as shares of a k-of-n split; read, created and written for every key | Done except error correction; a failing checksum is refused |
| SLIP-39 | Shamir shares over the 1024-word list, in groups with thresholds, with the passphrase; read, created and written | Done for SLIP-39 keys; a BIP-39 key is never split as SLIP-39, because its seed is 64 bytes |
| SLIP-132 | `ypub`/`zpub` account-key encodings, read and written | Done |
| BIP-137 | Legacy message signatures, including the Electrum and Trezor header ranges for nested and native SegWit | Done |
| BIP-322 | Message signatures, the "simple" form, for SegWit and Taproot addresses | Done |
| BIP-174 | PSBT version 0: full inspection, signing, finalizing | Done; PSBT version 2 (BIP-370) not built |
| BIP-340/341/342 | Schnorr signatures, Taproot key path and script path | Done |
| BIP-350 | bech32m addresses | Done |
| BIP-380 to 386 | Output descriptors with checksums, `multi`/`sortedmulti`, `tr` with trees, miniscript in `wsh` and `tr` | Done |
| BIP-387 | `multi_a` and `sortedmulti_a` tapscript multisig, checked against Bitcoin Core | Done |
| BIP-388 | Wallet policies (a template over `@0/**` keys and their list), read and written | Done |
| BIP-327/328/390 | MuSig2 key aggregation, aggregate xpubs, `musig()` descriptors; two-round signing checked against Bitcoin Core | Done; a `tr(musig(…), sortedmulti_a(…))` fallback tree not built |
| BIP-352 | Silent payments, receiving: the scan and spend keys at the BIP's paths, the `sp1q…` address and its labels, and the outputs of a transaction that pay a wallet, checked against every published vector | Done; sending waits for BIP-375 |
| BIP-392 | The `sp(spscan1q…)` descriptor a silent payments scanner is handed | Done for `spscan`; `spspend`, which is both private keys, is neither written nor read |
| BIP-321 | The `bitcoin:?sp=…` URI for a silent payment address | Done for `sp`; no other parameter is written or read |
| BIP-353 | The TXT record line that publishes that URI at `user.user._bitcoin-payment.domain` | Written for a name and a domain the person types; nothing is resolved and no DNSSEC proof is checked |
| BIP-445 | FROST threshold signing, trusted-dealer key generation, checked against the BIP's reference implementation | Done as a one-device flow (see Wallets) |
| BIP-129 (BSMS) | Descriptor records read and written; key records written, signed by the account key, with a session token | Done except encrypted records, which are refused by name |
| BC-UR | `crypto-psbt` and `bytes`, static and animated | Done; `crypto-hdkey` and `crypto-account` not built |
| SeedQR, CompactSeedQR | Read by scan; written on screen and as a hand-drawable grid | Done |
| Seed XOR | Coldcard-compatible split into 2 to 4 parts and combine | Done |
| RFC 6979 | Deterministic ECDSA nonces, low-R or first; Schnorr with zero or fresh auxiliary randomness, chosen in Settings | Done |
| EFF diceware | Passphrase generation from dice, long and short lists | Done |
| Miniscript | Policies compiled to descriptors; spend paths read on the review | Done |
| LND aezeed | A cipher seed's twenty-four words read to their entropy, internal version and birthday (scrypt, AEZ v5, CRC-32C), and LND's node key at `m/1017'/coin'/6'/0/0` | Read only; no cipher seed is ever made here |
| ldk-node | The node secret an ldk-node wallet derives from a BIP-39 phrase | Read only |

## Keys

- Load a key from typed words, a SeedQR or CompactSeedQR, an encrypted backup, Seed XOR parts, SLIP-39 shares, or a Codex32 string or its shares.
- Create a key from dice, coins, typed hex, a shuffled deck, camera noise, the device's own generator, or a mix of sources, with a sanity check on the entropy and the math shown. 12, 15, 18, 21 or 24 words.
- Dice are read by whichever published procedure you follow: hashed as ASCII digits (Coldcard, SeedSigner), every six written as a zero and then hashed (Keystone), or words chosen outright by the rolls (BitBox), where five rolls of 1–4 and a coin roll name each word, the last one too, and the checksum takes the last word's low bits.
- Create SLIP-39 shares and Create Codex32 shares as flows of their own, with the same entropy sources.
- Open a passphrase over a loaded key, or a BIP-85 child of one. A passphrase key is never stored.
- Derive one of BIP-85's other applications from a key: a WIF, an extended private key, 16, 32 or 64 bytes of hex, or a password in base64 or base85, shown on a masked screen and kept nowhere.
- Grind a vanity address from a key's page: the device turns one of the key's own dials — a counter appended to its BIP-39 passphrase, or the account index — until the first address of the chosen script type begins with the characters asked for, states the rate it is managing and how long that prefix should take, and stops on the first find. The counter is EntropyLab's, in its order, so a find here is a find there. Taking the find opens the passphrase key or adds the single-sig wallet at that account; leaving discards it.
- Every key is a row of Keys with its master fingerprint; its page says what it is made of, and carries Backup, Account key, Keep on this device (Tier B) and Forget.
- Account key: any of the six accounts (four single-sig, two BIP-48) exported with its origin, as SLIP-132 for a single-sig account, or as a signed BSMS key record for a multisig one.
- Several keys loaded at once; the session PIN; auto-lock and auto-wipe timers; wipe on exit; a duress PIN on a device that keeps a key.

## Backups

- Words, with their wordlist numbers; the quiz over every word in random order, available at any time.
- SeedQR and CompactSeedQR on screen, and as a grid to copy by hand.
- A steel-plate layout.
- Seed XOR parts, with the random parts made from a source the person chooses.
- An encrypted backup file or QR, XChaCha20-Poly1305 under an Argon2id passphrase.
- SLIP-39 shares of a SLIP-39 key, under a new plan with a new identifier.
- Codex32 for every key, split or as one string, with "Type it back" as the check.

## Wallets

A wallet is a policy over keys and is added explicitly; the device makes none up. Six kinds are built on the device; more are read.

| Kind | Built here | Read from a coordinator |
|---|---|---|
| Single-sig, four script types | Yes | Descriptor, account key, SLIP-132 key, Coldcard's JSON account export |
| Multisig, `sortedmulti` in `wsh` or `sh(wsh)` | Yes, from loaded keys and scanned cosigners | Descriptor, BIP-388 policy, the Coldcard multisig text file that Sparrow, Nunchuk and Specter write, BSMS descriptor record |
| Taproot multisig, `tr(H, sortedmulti_a)` | Yes | Descriptor |
| MuSig2, `tr(musig(…))` | Yes | Descriptor |
| FROST, m of n over 24-word keys | Yes, dealt on the device | Group record |
| Recovery (Liana's shapes), up to three timelocked paths, SegWit or Taproot | Yes | Liana descriptor, any number of paths |
| Miniscript `wsh` and Taproot tree wallets | No | Descriptor |
| Silent payments (BIP-352), one key's scan and spend halves | Yes | — |
| Watch-only, any of the above without a key | No | Descriptor |

Every wallet has addresses, Check an address, Export (descriptor, policy, BSMS record for a multisig, and a Bitcoin Core `importdescriptors` file), a Keys review with the key or eye glyph per member, a name, and Forget. On Tier B a wallet can be kept across restarts; a wallet over a passphrase key is kept only if asked.

A silent payments wallet is the exception, because it has no chain of addresses and no descriptor: its page is the one address it publishes, the labels that are addresses of their own, Check a payment — a transaction read by code or file, with the previous transactions its inputs need, answered with the outputs that pay this wallet — and an Export of the scan descriptor, the payment URI and the DNS record. It cannot sign, because sending to a silent payment address is not built.

## Signing

- PSBTs by QR, animated BC-UR, file or paste; the review shows every output and input, the fee and rate, the sighash, locktime and RBF, and warnings for unusual sighash, high fee, dust, a wrong network or an unknown path.
- Change is verified by deriving it from a wallet in use; an output the device cannot account for is a payment.
- Single-sig, multisig, tapscript multisig, miniscript and Taproot tree inputs; MuSig2 in two rounds; FROST with one device carrying a bound file between two locations.
- Every signature is verified before it leaves; the signature bytes are shown for comparison with another implementation; a deterministic nonce is checked on the review.
- The result leaves as a finished transaction or a partly signed PSBT, as a QR or a file.
- When a transaction names a key this device does not hold, the review lists the keys and opens Add a key for a missing one.
- Messages: sign with BIP-137 or BIP-322 for a chosen address, and check a signed message.
- Networks: mainnet, testnet, signet, regtest.

## Verifying and tools

- Check an address, scanned or typed, against the wallets in use.
- Tools: the key explorer (paths, extended keys, addresses, words and bits, the seed and master key), the word list, the dice passphrase, hashes, encodings, descriptor checksums, extended-key conversion, units, miniscript policy compilation, decode a transaction, and the Lightning node key an LND cipher seed or a loaded key belongs to.
- Notes and a wallet's recovery sheet: text typed on the device or read from a plain file, shown whole, exported in the clear, as an encrypted file or QR, or as a KDBX 4 database; on Tier B, eight notes and sheets can be kept on the device with the keys.
- Learn: 26 pages on the concepts, opened from any working screen's info button at the section that screen is about.

## Devices and shells

| Shell | Tier | Camera | Keeps a key |
|---|---|---|---|
| Raspberry Pi 3 card image, 2.8" DPI panel | A | V4L2 | No |
| USB stick image for any x86-64 UEFI laptop | A | V4L2 | No |
| Android | B with a secure element, else C | Camera2 | Yes on Tier B, behind the Keystore and a biometric or credential per use |
| Desktop, Linux and macOS | C | V4L2, AVFoundation | No |
| Desktop, Windows | C | None; files only | No |
| iOS, browser (HTML) | Not built | | |

- The Pi and stick images are Buildroot builds with no shell, no login and no network; the stick reads and writes files on any FAT partition of any USB disk.
- Every screen works by touch, mouse and keyboard, at four size classes from 240×320 up.
- Reproducible builds in pinned containers for the Linux binary, the Android APK and the card image; a self-test of the vectors at every start; fuzz targets for the parsers.

## Not built, and out of scope

Not built yet: PSBT version 2; anti-exfil signing and the verification of another device's anti-exfil transcript; sending to a silent payment address (BIP-375); a cross-session nonce history; the `tr(musig, sortedmulti_a)` fallback tree; Codex32 error correction; encrypted BSMS records; encryption of the FROST carry file; `crypto-hdkey` and `crypto-account`; a legacy `sh(multi)` account; the iOS and browser shells; camera capture on Windows; translations.

Out of scope by design: any network access, balances, fee estimation, coin selection, other coins, and being a coordinator. Lightning too, but for one reading tool: a node key is derived so that a backup can be identified, and this device never holds one, signs with one or makes a cipher seed.
