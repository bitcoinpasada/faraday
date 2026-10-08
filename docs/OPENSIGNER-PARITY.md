# OpenSigner in Faraday: what is used, what is not, and the plan

Written 2026-10-06 against upstream at c418768. One row per OpenSigner
crate, then per feature of OpenSigner's app (`opensigner-core`), saying
whether Faraday has it, through which crate, and what is left. The plan at
the end orders what is left.

Faraday's rule (`docs/WALLETS.md` §2 rule 5) is that it builds on
OpenSigner's `core/` crates, which hold the bytes and the cryptography, and
puts its own flows on top. The exceptions, where Faraday reads
`opensigner-core` itself, are listed in the last table of §1.

## 1. Crates

| Crate | What it is | In Faraday | How |
|---|---|---|---|
| `osk-crypto` | Hashes, scrypt, BLAKE2b, sealed and pinned secrets, zeroizing | Yes | `faraday-core`, `faraday-vault`, the shells |
| `osk-bip` | BIP-32/39/85/388, descriptors, policies, MuSig2, FROST, SLIP-39, codex32, BSMS, silent payments, aezeed, wordlists, vanity | Partly | Used: `bip39`, `keys`, `policy`, `multisig_config`, `musig`, `frost`, `threshold`, `spend`, `diceware`, `base64`. Not used: `bip85`, `bsms`, `codex32`, `slip39`, `silent`, `silent_wallet`, `aezeed`/`aez`, `core_import`, `slip132` (SLIP-132 `ypub`/`zpub` read), `kana`/`hangul`/`wordlists` (BIP-39 in languages other than English), `vanity`, `recovery`, `tapmulti` as a module (Faraday's TapMulti goes through `policy`), `xkey`, `account`, `address`, `descriptor` as modules |
| `osk-entropy` | Dice, coins, cards, hex, camera noise, device bytes, mixing, sanity checks | Yes, whole (2026-10-05) | New key (`faraday-core/src/keygen.rs`) |
| `osk-psbt` | PSBT inspect, sign, finalize, MuSig2 and FROST records, carry file, signed messages, verify | Yes | Spend, Message, Check message, the FROST carry |
| `osk-codec` | QR encode and decode, UR, SeedQR, PNG, classification | Yes | Scanner, QR sheet, PNG read in `faraday-files`; Faraday adds BBQr, numbered parts and the UR registry in `faraday-qr` |
| `osk-backup` | `.oskb` encrypted backup, KDBX 4 writer, Argon2id cost | Partly | `.oskb` read into a vault; KDBX not used |
| `osk-keep` | The kept-key blob, notes, remembered wallets and names | No | Faraday keeps these in its vault (`faraday-vault`) instead |
| `osk-shell-api` | Events in, commands out, between an app and its shell | Yes | Every Faraday shell |
| `osk-ui` | Canvas, fonts, tokens, widgets, layout | Partly | Canvas, fonts, icons and widgets; Faraday draws its own screens |
| `opensigner-core` | OpenSigner's app: flows and wording | Read only | Faraday does not run it. It reads its New key options (`create::SOURCE_ROWS`, `MIX_SOURCES`, `load::COUNTS`), its strings for those, and its Learn pages; the stick shell reduces camera frames with its `scan` helpers |
| `opensigner-ffi` | C ABI and JNI over the app | No | Not needed: Faraday has no phone shell |
| shells: `pi` | Framebuffer shell for the Pi and the PC stick | Copied | `faraday/shells/stick` is a copy, kept in step by hand |
| shells: `desktop` | winit desktop shell | No | Faraday has its own desktop shell |
| shells: `scanner` | QR scanning thread | Copied | `faraday/faraday-scanner`, which also reports where a code was and how fine it is |
| shells: `v4l2`, `avfoundation` | Linux and macOS cameras | Yes | Stick and desktop shells |
| shells: `snapshot` | Renders OpenSigner's screens | No | Faraday has its own tour (`faraday/shells/snapshot`) |
| shells: `android` | The phone app | No | Out of scope |
| tools: `fontbake`, `qrgen` | Font atlas, QR fixtures | Indirectly | Through `osk-ui` and the vectors |

## 2. Features of OpenSigner's app

| Feature (`opensigner-core`) | In Faraday | Notes |
|---|---|---|
| Load a key: words typed, English | Yes | Add a key |
| Load: words in the other BIP-39 languages | Partly (2026-10-06) | Spanish, French, Italian, Czech and Portuguese, typed as their ASCII fold; a key keeps its list. Japanese, Korean and Chinese need on-screen keyboards: plan B1 |
| Load: SeedQR, CompactSeedQR | Yes | Scan a SeedQR |
| Load: BIP-39 passphrase | Yes | |
| Load: SLIP-39 shares | Yes (2026-10-06) | Add a key › SLIP-39 shares, against SLIP-39's vectors |
| Load: codex32 | Yes (2026-10-06) | Add a key › codex32, against BIP 93's vectors |
| Load: Seed XOR parts | Yes (2026-10-06) | Add a key › Seed XOR parts |
| Load: `.oskb` encrypted backup | Yes | Into a vault |
| Load: an xprv | Refused on purpose | Keys come as words |
| Create a key: dice (3 procedures), coins, cards, hex, camera, mix, device | Yes (2026-10-05) | New key, options read from OpenSigner |
| Create: the backup quiz | Yes (2026-10-06) | New key's Quiz card runs `opensigner_core::quiz::Quiz`; skipping asks twice |
| Create: SLIP-39 shares, codex32, Seed XOR split | Partly (2026-10-06) | A key held here splits, in the backup's seeds step, into Seed XOR parts or codex32 shares of its seed, copied by hand; making a new key as SLIP-39 shares is left: plan B2 |
| Sign: single key, multisig, miniscript, Taproot trees, MuSig2, FROST | Yes | Spend |
| Sign: silent payment outputs | No | Plan C2 |
| Verify an address | Yes | A scanned address is checked against loaded wallets |
| Sign and check a message | Yes | |
| Explore: derivation paths and keys | Yes (2026-10-06) | Explore a key, on the Wallets page: the public side at any path, read with OpenSigner's own path parser; nothing private |
| Lightning: node key from aezeed or a BIP-39 key | Yes (2026-10-06) | Lightning node key, on the Wallets page: ldk-node from a loaded key, LND from an aezeed; the private key into a vault by default |
| Silent payments: receive address and scan key | Yes (2026-10-06) | Silent payments on the Wallets page: the address with labels, its QR and `bitcoin:` link, the public record to the Outbox, the scan key into a vault or through the secret sheet |
| BIP-85 child seeds and passwords | Yes (2026-10-06) | Derive with BIP-85, on the Wallets page: OpenSigner's six applications, the value into a vault by default |
| Vanity address search | No | Plan C4 |
| Tools: the calculators | Yes (2026-10-06) | Tools, on the Wallets page: OpenSigner's six calculators (`opensigner_core::tools`), called as they are |
| Self-test against vectors at start | No | Plan A2 |
| Kept key in a secure element | No | Faraday uses vaults; a PC has no secure element it trusts |
| Notes | Yes | Vault notes |
| Diceware passphrase (EFF list) | Yes | Vault passphrases |
| Export: `.oskb`, KDBX | Partly (2026-10-06) | A vault entry exports for KeePass as KDBX 4 under its own passphrase, a sealed file; `.oskb` export is not offered, vaults carry keys instead |
| Learn pages | Yes (2026-10-06) | The ? on each screen opens OpenSigner's pages for it |
| Settings: text size, language | Partly | Text size yes; language no (B1) |

## 3. Plan

Each item is a Faraday flow over the crate named, with tests of what a
person notices, in the order below. Nothing here edits an upstream file.

**A. Integrity first**

1. Done 2026-10-07 (upstream §16.139, synced): `SOURCE_ROWS`,
   `MIX_SOURCES` and `WORD_COUNTS` are `osk-entropy`'s and the Learn pages
   `osk-learn`'s, and Faraday reads them there.
2. Done 2026-10-07: the self-test at start (`core/osk-selftest`,
   upstream §16.139): BIP-39, BIP-32, signing and the KDFs checked when
   the display arrives, and no key accepted until they pass; a failure
   blocks the app with Exit only (HANDOFF item 53).
3. Done 2026-10-06: vault creation, GPG and Secure Boot keys take fresh
   shell bytes of their own (`faraday-core/src/fresh.rs`). A vault's seal
   nonce and the new-keyboard code keep the session's bytes and a counter:
   neither is a key, and a seal at lock must not wait.

**B. Keys in every form OpenSigner reads**

1. Done: the Latin lists 2026-10-06; Japanese (kana), Korean (jamo) and
   Chinese (pinyin, 注音) 2026-10-07, on OpenSigner's keyboards through its
   own word entry (HANDOFF item 52).
2. Loading done 2026-10-06, and a held key's Seed XOR split and codex32
   shares in the backup. A new key made as SLIP-39 shares done
   2026-10-06: New key's Length card offers BIP-39 words or SLIP-39
   shares (20 or 33 words, m of n in one group), the randomness is the
   master secret, the split takes a fresh draw of its own, each share is
   shown and quizzed, and the card states what shares reveal.
3. Done 2026-10-06: the word quiz after New key.
4. Done 2026-10-06: BIP-85, the results into a vault by default.
5. Done 2026-10-06: a vault entry exported for KeePass (`osk-backup::kdbx`,
   at OpenSigner's device cost); the file is sealed, so it goes to the
   Outbox's Sealed group without the secret sheet.

**C. The rest of the app**

1. Done 2026-10-06: Explore.
2. Receiving done 2026-10-06; the silent payments wallet among the
   session's wallets, with Check a payment, 2026-10-07 (HANDOFF item 54).
   Left: signing silent payment outputs, when upstream does (BIP-375).
3. Done 2026-10-06: the Lightning node key.
4. Tools done 2026-10-06. Vanity search done 2026-10-06
   (`faraday-core/src/vanity.rs`, `vanity_screen.rs`, `tests/vanity.rs`):
   upstream's engine and state, the account dial and the passphrase dial,
   a tick at a time; a find opens its wallet. A search runs only while its
   screen is open, and the idle lock still ends one.
5. Done 2026-10-06: BIP 129 records both ways (signed key records out,
   checked in; descriptor records out and in, first address checked) and
   Bitcoin Core's `importdescriptors` file out. SLIP-132 keys on their own
   carry no origin to build a multisig from and stay unread outside the
   multisig configs that already carry them.

**D. QR left from `docs/QR.md` §6**

1. Done 2026-10-06: camera choice, over Faraday's own storage channel,
   on the stick and in the desktop app (`faraday/shells/desktop`:
   `camera::list`, `Camera::choose`).
2. Done 2026-10-06: the found code outlined and "too fine for this
   camera", through `faraday/faraday-scanner`, Faraday's copy of
   `opensigner-scanner`.
3. Done 2026-10-06: a cosigner's key scanned while a multisig is made
   fills the slot waiting for one (marked for later, else the first empty),
   with Scan their key on the slot.
