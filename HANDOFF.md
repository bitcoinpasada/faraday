# Faraday — handoff

**As of 2026-10-06.** For a session starting cold, human or agent. Read this,
then `PLAN.md`, then the docs it names.

## What this is

Faraday is an offline key appliance: a fork of OpenSignerKit /
OpenSigner that boots from a USB stick on an x86-64 PC (or its card on a
Raspberry Pi), runs from RAM, and adds vaults, a Wallets tab, GPG, Secure Boot
and QR transfer to OpenSigner. It was called openfaraday until
2026-10-04, when the owner renamed it Faraday and deprecated Faraday OS
(the owner's amnesic Arch desktop), whose repository is now
`~/Projects/faraday-os-deprecated`. A first build exists (below).

## Where we stopped (2026-10-06)

The owner said: finish the QR work, write this handoff, stop, and **do not
build**. So the source is ahead of every output in `out/`:

- `out/stick/faraday-x86_64-uefi.img` and `out/linux/faraday` were built
  at 04:44–04:47 on 2026-10-06 (release image, no `dev=1`, booted in
  QEMU by `faraday/tools/qemu-check.py`). They have everything through
  item 34 below, and **not** item 35 (scrolling, PDFs, the scanner's
  outline, "too fine" and camera choice).
- `just` passes on the current source: fmt, clippy with warnings as
  errors, the lints, and every test (1525 of them, one skipped), after
  items 37 to 43. `out/linux/faraday` was rebuilt on 2026-10-06 with
  them (sha256 `cefc935d…`); the stick image was not.
  The snapshot tour (`cargo run -q -p faraday-snapshot -- 1366x768
  out/testkit out/snapshots/faraday/1366x768`) runs to the end.
- Nothing is committed. The owner commits.

To build when asked (`DOCKER=podman`):

    export DOCKER=podman
    just faraday-linux-bin
    just faraday-stick-image          # release; dev=1 only when asked
    python3 faraday/tools/qemu-check.py out/stick/faraday-x86_64-uefi.img out/stick/qemu-check out/testkit

The owner tests on the Dell with `hp-pull`; the dev image's kernel console
prints over the app, so test sticks are release images.

What the owner reported last and what was done about it (item 35): mouse
wheel and trackpad scrolling were inverted and jumped about half a page;
the backup sheets were not PDFs. Both are fixed in source and need the
owner's check on the Dell after the next build. The owner's other
standing requests: secrets leave only sealed in a vault unless a warning
is acknowledged (`docs/FLOWS.md` decision 6, item 27); New key's options
and cryptography come from OpenSigner, with Faraday's interface over them
(items 24, 29); a ? on every screen opens OpenSigner's Learn pages (item
26); `docs/OPENSIGNER-PARITY.md` is the plan for the rest of OpenSigner.

What is left, in the order the plan gives (`docs/OPENSIGNER-PARITY.md`
§3): upstream asks (A1: the New key option lists and the Learn text into
`core/`; A2: the self-test vectors into `core/`); BIP-39 entry for
Japanese, Korean and Chinese (on-screen keyboards); a new key made as
SLIP-39 shares; a silent payments wallet among the session's wallets;
vanity search; the desktop shell's camera choice; a descriptor scanned
during Create routed into the flow.

## Where things are

| File | What |
|---|---|
| `PLAN.md` | The plan: scope, platforms, trust model and processes, the stick rule and lock cycle, vaults, tabs, GPG, Secure Boot, interface, crates, order of work, decisions, open questions. |
| `docs/VAULT.md` | The vault file format: four equal slots, hidden-slot rules, Argon2id cost guidance, record types, write-back. |
| `docs/QR.md` | Scanner and QR sender requirements: the union of Faraday OS's file transfer and the Wallets tab's scanner, built on OpenSigner's codec. |
| `docs/FLOWS.md` | Every flow and where it leaves the person; five decisions made to remove dead ends. |
| `docs/MULTIVENDOR.md` | Proposal: a second, independent implementation of each cryptographic step, and how it would show. Not built. |
| `docs/MOTION.md` | Smooth scrolling (glide, momentum, rubber band), hover, motion, shadows, frosted sheets, light mode and reduce motion: decisions, measurements, design, what was built (item 49). |
| `docs/OPENSIGNER-PARITY.md` | Every OpenSigner crate and app feature, whether Faraday has it, and the plan for the rest. |
| `docs/AUDIT.md` | Every crate that computes cryptography, what is written here versus taken from a crate, and what each is checked against. |
| `docs/FAMILY.md` | The Spend tab: family mode through Faraday's vault, page by page, with its text. |
| `docs/WALLETS.md` | The Wallets tab: flows on OpenSigner's `osk-*` crates, wallet reassembly, shared steps, per-scheme flows, staying downstream. |
| `design/prototype/project/` | Source of the interface prototype (a Design canvas artifact, `canvas.json` plus one `.dc.html` per screen). Published at https://claude.ai/artifact/Qnw1jLVVWjBzKDNofgjRES (private to the owner). |
| `faraday/faraday-core/examples/wallet_2of3_demo.rs` | Derives the 2-of-3 the Wallets screens show (the dummy seeds bacon, zebra, summer) and the unsigned spend's txid, with osk-bip. |
| `docs/PLANNING.md`, `DESIGN.md`, `UX.md`, `FEATURES.md`, `BACKUP.md` | Upstream OpenSigner's own documents, unchanged. |
| `CLAUDE.md` | Upstream's working rules. They apply here: agents do not commit; use `trash`, not `rm`; `just` is the check. |

## Git state

- This is a full clone of `maxmoney21m/opensignerkit`; the remote is named
  `upstream`. `main` is fast-forwarded to upstream `c418768` (§16.138).
- Faraday's files (`PLAN.md`, `HANDOFF.md`, `docs/VAULT.md`,
  `docs/QR.md`, `docs/FLOWS.md`, `docs/WALLETS.md`, `design/prototype/`) are
  **untracked and uncommitted**. The owner commits.
- No GitHub fork exists; nothing has been pushed.

## Upstream

The OpenSigner developer made §16.138 for forks like this one: code that
defines bytes or answers a question without a screen lives in `core/`
crates. New since the design started: `osk-backup` (the `.oskb` format, the
KDBX writer, Argon2id `Cost`), `osk-keep` (the kept-key blob), and moves
into `osk-psbt` (signed-message text, a transaction with its previous
outputs) and `osk-bip` (address parsing, `WalletPolicy::find_address`).
`rqrr` is now a path dependency. `docs/WALLETS.md` §1 rule 5 records what is
still app-only (Tools calculators, the self-test vectors, `inspect.rs`'s key
origins through `osk-ui`).

## Decisions so far (all in the docs)

- Trust model kept from OpenSigner; no upstream file edited; variants of
  upstream parts are copies under `faraday/`; `faraday-core` names
  OpenSigner's `osk-*` types directly, with no separate adapter crate.
- USB "option B": no filesystem in the kernel; a `CAP_CHOWN`-only helper
  hands USB/SD partitions (never whole disks, never `OSKBOOT`) to an
  unprivileged FAT16/FAT32 process; root at runtime is only idle PID 1.
- No storage while a secret is in memory; Inbox and Outbox on one Files
  screen; locking restarts the app; `init_on_free=1`.
- USB devices: drivers for storage, HID, UVC and hubs only; per-interface
  authorisation; a storage device's HID interface never authorised; a new
  keyboard types a code shown on screen before it is believed.
- Vaults: four slots, hidden-slot deniability with its stated limit,
  user-chosen slot size, Argon2id presets with machine guidance, fixed file
  names, write-back replaces.
- QR: PNG/JPEG decoded only in the disk process; BBQr `Z` read, never
  written.
- Wallets tab beside the OpenSigner tab (which stays until the Wallets tab is robust);
  Windows-compatible Secure Boot policy; step-card forms on `wide`, pages
  on `small`; Faraday OS's palette, Inter and JetBrains Mono.

## Open questions

`PLAN.md` §13: which Raspberry Pis (Pi 3 panel, or also Pi 4/5 on HDMI with
a 64-bit build), and whether the Pi Zero 2 W is worth it.

## The build (as of 2026-10-05)

Built on the HP; tested on the Dell by `hp-pull`
(`~/Projects/hp-build-setup/README.md`). Outputs:
`out/stick/faraday-x86_64-uefi.img` (`just faraday-stick-image`),
`out/linux/faraday` (`just faraday-linux-bin`, the online desktop app),
`out/snapshots/faraday/<WxH>/` (`just faraday-snapshots`, a ~60-step tour
with the PDFs it prints). The HP has podman and no `docker`: run the two
container recipes with `DOCKER=podman` in the environment (`export
DOCKER=podman; just faraday-stick-image`), which the recipes and
`faraday/image/run-build.sh` pass through.

- `faraday/faraday-core`: the app (`osk_shell_api::App`). Screens: Home,
  Wallets (start page: what you have, what you want to do, your wallets),
  the wallet card, Sign a transaction (every wallet kind, with a Path
  step for miniscript and trees and a Nonces step for MuSig2), Back up,
  Create a wallet, Restore a wallet (descriptor, config or split shares),
  Sign a message, Check a signed message, Add a key, Files, Stick visit,
  Settings, sheets (lock, power off, QR out, camera scan in). Every flow is step cards
  (`flow.rs`) with a Guided walk-through per card (`guide.rs`).
  Cryptography is `osk-psbt`/`osk-bip`/`osk-codec`; Faraday adds
  `wallet.rs` (session, kinds, wallet matching), `create.rs`,
  `backup.rs` (split plan and audit, multisig config, sheet files, SeedQR
  structure, Check my copy), `pdf.rs` (the printed sheets, desktop only),
  `restore.rs` (merging split shares), `testkit.rs` (the testnet test
  stick, behind the `testkit` feature: twelve test wallets with MuSig2 and
  FROST in every backup form, their spends, a recovery-path spend, each
  MuSig2 round's copies, the FROST carry file, Savings' shares, the test
  keys in every key form, a signed message, the vault). Storage is a second channel beside the App interface
  (`Faraday::storage` / `poll_storage`).
- `faraday/faraday-storage`: sticks as directories; read; write with
  read-back; Inbox and Outbox across a lock; PDFs on an online shell.
- `faraday/shells/stick`: copy of `opensigner/shells/pi` at c418768,
  patched: sticks from `/proc/mounts`, `/run/faraday`, exit 75 on lock,
  HiDPI guess. `faraday/image/` overlays upstream's image tree (inittab
  app loop, `init_on_free=1 init_on_alloc=1`).
- `faraday/shells/desktop`: the online Faraday in a window (glibc build:
  winit loads Wayland at run time). Sticks are folders in
  `~/faraday-sticks`, F2/F3/F4; Make PDF saves to `~/faraday-print`; the
  webcam scans (camera.rs copied from upstream's desktop shell).
- `faraday/shells/snapshot`, `faraday/tools/testkit`,
  `faraday/tools/qemu-check.py` (boots the image headless and
  photographs it).
- The VM route (`of-qemu`) was dropped by the owner.
- Shared upstream files touched: `Cargo.toml` (members) and `justfile`
  (one `import?` line at the end).

## Work queue

**Builds (owner, 2026-10-05):** build only the desktop app (`DOCKER=podman just faraday-linux-bin`) until the owner asks for stick images again.

The auto-continue job reads this list and takes the first item not
marked done. Owner's direction (2026-10-04): wallet-reassembly flows across
every scheme OpenSigner signs for, with plain written guidance for each
flow; always build the USB image as well as the Linux test app; get as far
as possible before the next build.

1. [x] Display scale: on large screens the layout stops growing (a cap
   from the display's density) and is centred; small screens still fill.
   Settings gains Display scale (Auto, 75–200 %).
2. [x] Stick image: patch `faraday/shells/stick` (sticks from
   `/proc/mounts`, Inbox and Outbox in `/run/faraday`, exit 75 on
   lock), an image overlay (app loop in inittab, `/run/faraday`,
   `init_on_free=1 init_on_alloc=1`), and `just faraday-stick-image` →
   `out/stick/faraday-x86_64-uefi.img`.
3. [x] Every scheme `osk-psbt` signs, through the same spend flow:
   single-sig (pkh, sh-wpkh, wpkh, tr key path), multisig (sh, wsh,
   sh-wsh), Taproot multisig (`tr` with `sortedmulti_a`), miniscript;
   steps per kind as in `docs/WALLETS.md` §4; a test kit wallet and spend
   for each.
3b. [x] Recovery paths: a **Path** step for a miniscript or tree spend
   (which path, its timelock, whether the inputs have aged enough, from
   the PSBT's sequences), and test kits for a recovery-path spend.
3c. [x] The Wallets start page (owner: "users can get into any flow
   OpenSigner supports"): "What are you holding?" as the tab's
   first screen, every flow one press away, grouped by what the person
   has (a PSBT, a wallet description, a seed, nothing yet) and what they
   want (sign, create, back up, restore, check, message). Every flow is
   step cards like Create vault and ends on the wallet card or a result.
   Only flows that work are listed; each new flow adds its row.
3d. [x] QR out and in (camera on the stick and in the desktop app; UR single and animated; a SeedQR is refused there): a PSBT, a signed PSBT or a descriptor shown as one code
   or animated UR (`osk_codec::qr`, `osk_codec::ur::Encoder`), on a white
   card with square modules. QR in (camera) after that.
4. [x] Guided mode: a written walk-through for every step of every flow,
   shown in Guided and hidden in Full (`docs/WALLETS.md` §6). Plain,
   specific sentences.
5. [x] Messages: sign (BIP-322, BIP-137) and check a signed message (step cards; the test stick carries a signed message).
6. [x] MuSig2 and FROST. MuSig2: the Nonces step, signing last or first with the session kept until lock, partials collected and aggregated; the test stick carries each round's copies. FROST (`docs/PLANNING.md` §16.103, upstream's `osk-bip::frost`, `osk-psbt::threshold`): a 24-word key is a share wherever a loaded record lists its public share; Create › Threshold · FROST deals from chosen loaded keys (up to five shares), loads the computed shares so the backup can copy their words, and the record goes out with the descriptor; a spend's first location chooses the other shares under Signers, signs, and puts the carry file (`.osk`) in the Outbox; the next location opens it from Files, signs, and finishes. The test stick carries the Threshold record (test keys 1 and 2 are shares 1 and 2), its spend and test key 1's carry file.
7. [x] Creating a wallet and its backup (backup — blank template, seeds by hand with the SeedQR grid and Check my copy, descriptor QR, descriptor, wallet .json (Specter's form, which Sparrow imports) and multisig config files, split shares, envelope, PDFs on the desktop app; creating — every single-key and multisig kind, keys here, new keys, cosigner account keys from files, a slot left for a cosigner's key to come (the creation waits under Nothing yet), each key held here out as a file or QR; naming — Rename on the wallet card, a .json's label; restoring — from a descriptor, a multisig config, a wallet .json, or any quorum of split shares, then the seeds typed back with the slot check; a SeedQR or CompactSeedQR scanned into Add a key; PNG QR codes on a stick read into Files, JPEG not read: it needs a decoder crate, the owner's call) (`docs/WALLETS.md` §5;
   "Backing up: two parts, kept differently"), as step
   cards like Create vault:
   - seeds by hand: the ruled SeedQR grid (standard and compact,
     coordinates every five squares, structural modules grey, a pinned
     row moved with the arrow keys), the words with their indices, and
     **Check my copy** (type the digits back; it names the wrong word);
     no passphrase line, and the screen says why;
   - the wallet in public keys: the descriptor as a QR on screen (one
     code or animated), and descriptor `.txt`, Sparrow
     `.json`, multisig config `.txt` to the Outbox; for a multisig the
     split plan (each sheet omits at most M−1 keys);
   - printing happens only on the desktop app (the online Faraday): the
     device puts a backup sheet file and the blank template (word count
     only, no secret) in the Outbox; the desktop app opens them and
     writes a PDF to print. Nothing secret is ever in a file or a PDF.
   - restore flows ("Restoring from a backup"): words, SeedQR, the
     descriptor or its split sheets.
8. [x] Rename the project to Faraday (owner, 2026-10-04): the old
   Faraday OS repo at `~/Projects/faraday` becomes
   `~/Projects/faraday-os-deprecated` (its GitHub repo,
   bitcoinpasada/faraday, is the owner's to rename); this repo becomes
   `~/Projects/faraday`; crates `faraday-*`, outputs `out/linux/faraday`
   and `out/stick/faraday-x86_64-uefi.img`, recipes `faraday-*`; update
   `hp-build-setup` (README, `dell/hp-pull` path) and tell the owner to
   reinstall the Dell scripts; recreate the auto-continue job with the
   new path.
9. [x] Small fixes: scroll long lists (Stick visit, Files, the wallet
   list), compact wallet rows; a stick's volume label instead of `sda1`;
   "Boot stick" once; "1 of 1 signature"; a signed file named
   `<wallet>-signed.psbt`, not `…-unsigned-signed.psbt`.
10. [x] Build (2026-10-05 03:34): `just faraday-stick-image`, `just faraday-linux-bin`,
   `just faraday-snapshots`; tell the owner what is ready for `hp-pull`.

11. [x] Vaults (owner, 2026-10-05: "implement it in the vaults tab"):
    `faraday/faraday-vault` is `docs/VAULT.md` (format, Argon2id +
    HKDF + XChaCha20-Poly1305, four slots tried every time, create with a
    random slot order and keystream-filled unused slots, seal that copies
    the other slots byte for byte, the ten record types with their field
    rules); 9 tests, among them RFC 5869's HKDF vector and the committed
    vectors; `tools/vault/open.py` opens the vectors on its own (OpenSSL's
    Argon2id). The app: a Vaults tab (list, Unlock, Create a vault with the
    four step cards and the cost panel of the prototype, Vault contents
    with kinds, items and details, hold to show, hold to delete), keys
    marked to load and every wallet loading at unlock, save session keys
    and wallets into a vault, add and edit entries and notes, rename a
    slot, the stick rule counting an open vault, Lock sealing changed
    vaults into the Outbox (the lock sheet says which), the boot stick's
    `.ofv` files read into the Inbox, a stick visit writing a vault back
    over the file with its salt and length (`faraday-storage::write_vault`),
    shells reporting free memory, the first unlock or create timing one
    64 MiB pass. The test stick has `vault.ofv` (passphrases `test vault`
    and `decoy`).
12. [x] Vaults, the rest (2026-10-05): entries from an Inbox text file
    of `otpauth://` URIs and `field: value` lines and from a scanned TOTP
    setup code (`faraday-vault::entries`); multi-line notes; Generate with
    dice (EFF long list, five dice a word); `.oskb` import (words, a master
    seed, a note, a recovery sheet); a BIP-39 passphrase typed in Add a key,
    stored with a key only on "Save with its passphrase", used at load, and
    "Load with a passphrase" for one not stored; §6 step 4's recovery of an
    interrupted write-back (`faraday-storage::finish_vault_writes`, tested);
    Lock asks first while a vault is open and lists what is sealed and
    what is not saved; the signed-amount memory (`docs/WALLETS.md` §3.3,
    `memory.rs`): refused with no override, kept across a lock in the
    boxes' new `kept` set with the settings, sealed into open vaults as
    type-10 records (a Settings choice), tested; `tools/argon2-bench`
    (`faraday-argon2-bench`) and the laptop row of `docs/VAULT.md` §3.1.
    Not built, on purpose: the type-9 MuSig2 round record. `osk-psbt`
    never serialises a `MusigSession` (PLANNING §16.47: a secret nonce
    lives in memory only, so it cannot sign twice), and writing one into
    a vault would mean changing upstream and weakening that rule; FROST
    already carries its nonce in the carry file. The Pi rows of the timing
    table wait for the boards.
13. [x] Build with the vaults (2026-10-05 07:26): `just faraday-stick-image`,
    `just faraday-linux-bin`, `just faraday-snapshots`, qemu-check.

14. [x] GPG (`PLAN.md` §7), 2026-10-05: `faraday/faraday-pgp` writes v4
    packets for an Ed25519 certification key with an Ed25519 signing
    subkey (self-signatures, the subkey's back-signature, detached
    signatures over SHA-256/512, revocation, armour, paperkey lines).
    Judged by `gpg` 2.4 (imports, verifies, rejects other data, takes the
    revocation) and by `paperkey` 1.6 (identical lines) in
    `faraday-pgp/tests/gnupg.rs`. In the vault: Make a key (name, email,
    1/2/5 years or never; the certificate and a revocation certificate go
    to the Outbox), Export public key, Sign a file (any Inbox file to
    `NAME.asc`), Revocation certificate, Renew, Paperkey held to show; the
    key is a type-7 record. The shells report the system clock
    (`StorageEvent::Clock`) for creation times. New dependencies
    `ed25519-dalek` 3 and `sha1` 0.11, notes in `docs/deps/`.
15. [x] Secure Boot (`PLAN.md` §8), 2026-10-05: `faraday/faraday-sb` makes
    PK, KEK and db as RSA-2048 keys with self-signed certificates and an
    owner GUID, signature lists and `.auth` updates (PK by PK, KEK by PK,
    db by KEK) under the Windows-compatible policy (Microsoft KEK CA 2011,
    KEK 2K CA 2023, Windows Production PCA 2011, Windows UEFI CA 2023,
    taken from Microsoft on 2026-10-05, SHA-256 recorded and tested) or
    own keys only, Authenticode signing of an EFI image and checking one.
    Judged by `openssl` (certificates, the PKCS #7 in each `.auth`) and
    `sbverify` (Faraday's own `BOOTX64.EFI` and a minimal image) in
    `faraday-sb/tests/judged.rs`. In the vault: Make keys, Enrolment
    files, Sign or check an image (the SHA-256 shown, Hold to sign).
    Type-8 record. New dependencies `rsa` 0.10.0-rc.18 (pinned) and
    `rand_core` 0.10, notes in `docs/deps/`. Not done: the OVMF test that
    enrols the keys, boots a signed image and refuses an altered one
    (`PLAN.md` §8 Test), and a build record to compare the image hash with
    (the screen asks the person to compare it). The tests that call
    outside tools find them on PATH or as `FARADAY_PAPERKEY`,
    `FARADAY_SBVERIFY`, `FARADAY_EFI`; this session used `apt download
    paperkey sbsigntool` unpacked in the scratchpad.
16. [x] Desktop app built after GPG and Secure Boot.

17. [x] The owner's flow batch (2026-10-05), after Secure Boot:
    - [x] **No test wallets or dummy seeds in the app** (2026-10-05).
      Load the test wallets, the test-seed buttons and the test-build pill
      are gone; `faraday-core::testkit` is behind the `testkit` feature,
      which the desktop app, the snapshot tour and `faraday-testkit` turn
      on and the stick shell does not. The snapshot tour and the tests
      load wallets from the stick's files and type the words.
    - [x] **Network** (2026-10-05): the session has a network (Mainnet,
      Testnet, Signet, Regtest: `osk_bip::keys::Network::ALL`; the
      `bitcoin` crate here has no testnet4). A pill on Home, the Wallets
      start page and the wallet card opens a chooser sheet. The first
      wallet, cosigner key or PSBT whose keys say a network moves the
      session to it (tpub → Testnet unless already on a test network); one
      from the other side of mainnet/test is refused while wallets are
      loaded, and the chooser refuses that move too (test networks among
      themselves are allowed: their keys are written alike). Keys re-derive
      for the network (`MasterKey::for_network`); Create's paths take coin
      type 1; sheets and PDFs state the network. A test network shows an
      amber band along the top, a tag in the sidebar and an amber pill.
    - [x] **The test stick, redone** (2026-10-05): testnet only, 82
      files listed in the stick's `README.txt`: every wallet as
      `-wallet.txt`, `-wallet.json` and (multisig) `-multisig-setup.txt`,
      with its spends; Savings' split shares; descriptor QR PNGs for
      Savings, Spending and Nested; each test key as words `.txt`, SeedQR
      and CompactSeedQR PNGs, `.oskb` (passphrase `backup`) and four
      account-key files; the vault, whose first slot now holds test key 1
      and every test wallet. Savings' spend txid is now `e65b496f…`. The
      desktop app moves a `~/faraday-sticks/TESTSTICK` from the old kit
      aside to `TESTSTICK-old-N` and writes the new one; `faraday-testkit`
      drops an earlier kit's files from its output directory.
    - [x] **Sidebar order:** Home, Vaults, Files, Wallets, Stick visit,
      Settings.
    - [x] **Home at boot with a vault from the boot stick** (2026-10-05):
      the boot stick alone no longer opens the stick visit; Home names the
      vault ("copied from the boot stick"), says "Pull the stick to unlock",
      and offers "Bring a PSBT in from a stick" (the visit) and "Scan a
      PSBT" (now, or later from Wallets). Pulling the stick on Home opens
      the vault's passphrase prompt. Unlocking loads nothing: it lands on
      Files, where each open vault lists its wallets and keys with
      checkboxes (wallets and keys marked "load at unlock" start chosen;
      that flag is now "chosen at unlock"), "Load everything into Wallets"
      and "Load the chosen".
    - [x] **Home with no vault:** "Create a vault" and "Go to Wallets".
      Create a vault ends with "Keep it open" (opens it with the first
      passphrase, held only until that choice) or "Leave it locked" (in
      the Outbox).
    - [x] **Wallets with nothing loaded:** Create a wallet, Restore a
      wallet, and the line "A stick with a Faraday vault brings its wallets
      and keys" (or Unlock, when a vault is in Files).
    - [x] **Create a wallet:** a seventh card, Vault, after Back up: with
      a vault open, the wallet and the keys held here for it, each saved
      or "In the vault", and Save all; with none, Make a vault or Unlock
      (both come back to Create) or Not now. Back up started from Create
      goes back to Create.
    - [x] **Restore a wallet:** a first card, A transaction to sign: From
      a stick (only while no key or vault is open; the visit comes back to
      Restore when the stick is pulled), Scan a QR code, Later. The wallet
      card lists its sources: Stick (copy files in), Vault (unlock one in
      Files, or restore a wallet saved in an open vault, its keys for the
      wallet's slots loading with it), Paper (scan the descriptor or a
      share; the words in the next card). A scan from Restore stays on
      Restore.
    - Tests: `faraday-core/tests/boot.rs` (boot vault on Home, pull to
      unlock, nothing loads until chosen, Restore's first card), and the
      snapshot tour's boot scene (steps 89–97 at 1280×800).

18. [x] Desktop app built after item 17 (2026-10-05): `out/linux/faraday`,
    sha256 `550ea193…`. No stick image (owner's instruction).

19. [x] Idle timers (`PLAN.md` §12.3), 2026-10-05: lock after 5 minutes
    without input (touch, key, scroll), when a key, vault, wallet or spend
    would be wiped; power off after 10 minutes, counted across that lock
    (the locking process keeps its idle time in the `idle` kept entry),
    only when nothing is held and the Outbox is empty, and never on the
    online desktop app. After an idle lock the Locked sheet shows; with
    files in the Outbox it stays and lists them instead of powering off.
    Settings › Session: Lock after 2/5/10/30 min or Never, Power off after
    10/30/60 min or Never, kept with the settings. Tests:
    `faraday-core/tests/idle.rs`.

20. [x] The disk process (`PLAN.md` §4.3), 2026-10-05. Crates:
    `faraday-fat` (FAT16/32, root directory only; FAT32 known from the
    boot sector as Linux knows it, so mtools' small FAT32 sticks read;
    judged by mkfs.fat, fsck.fat and mtools via `FARADAY_FATTOOLS`, plus
    thousands of damaged volumes; fuzz target in `faraday/fuzz`),
    `faraday-files` (Place trait over a folder or a FAT volume, the write
    rules, vault write-back, QR from PNG, the pipe protocol with sequence
    numbers), `faraday-disk` (uid 201; `--list` explains each partition),
    `faraday-grant` (root → uid 202 with CAP_CHOWN only, empty bounding
    set, no-new-privileges; decides from /sys alone; tested on a fake
    /sys), the app's
    `clean()` and the shell's clean marker `/run/faraday-clean/clean`.
    Image: no FAT/MSDOS/ISO9660 etc. in the kernel (forbidden lists),
    GPT partitions named `OSKBOOT` and `OSKDATA`, rcS starts both helpers
    with their output in the kernel log, users ofdisk/ofgrant, a dev
    inittab whose getty runs and whose app errors show on screen
    (`printk.devkmsg=on`). `faraday/tools/disk-test.py IMAGE OUT KIT`
    boots the dev image in QEMU, logs in on ttyS0 and checks it all;
    every check passes, and the test stick's 82 files reach the visit.
    Found on the way: `eprintln!` panics once rcS's console is gone (the
    helpers use a `say!` that never panics); kmsg rate-limits userspace.
    Open:
    - Decided (owner, 2026-10-05; `PLAN.md` §12 decision 8): no seccomp
      filter, no NET, no Landlock. The grant helper decides from `/sys`
      alone (GPT name `OSKBOOT`, or the Pi card's `mmcblk0p1`) and reads
      nothing from a stick; the disk process refuses a FAT labelled
      `OSKBOOT`.
    - JPEG QR decided against (owner, 2026-10-05): PNG only.
    - The release stick image has not been rebuilt with this.

21. [x] USB devices and new input (`PLAN.md` §4.6), 2026-10-05.
    Layer 1: `USB_PRINTER`, `USB_MON`, `USB_SERIAL`, `USB_ACM` forbidden.
    Layer 2: `usbcore.authorized_default=2`; rcS sets every root hub's
    `interface_authorized_default` to 0; `faraday-grant` authorises other
    devices, then their interfaces by class (storage, HID, video, hub),
    storage alone on a device that has storage, and asks
    `drivers_probe` to bind each (authorising alone binds nothing). It
    also gives input and video nodes that appear after boot the owners
    `/etc/mdev.conf` names (no hotplug helper runs). Layer 3: the stick
    shell trusts i8042, I2C, platform and hard-wired-USB input and the
    named touch panel; any other device's events are held back
    (`Wake::Held`), announced to the app (`StorageEvent::NewInput`), its
    typed characters sent as `InputTyped`; the app's sheet shows a
    six-letter code (keyboards) or **Use this pointer** (pointers,
    approved with believed input), **Ignore this device**, and Settings
    lists ignored devices until unplugged (`faraday-core/src/inputs.rs`,
    `tests/inputs.rs`). QEMU: disk-test checks the authorisation states;
    the USB keyboard's sheet shows (`out/stick/disk-test/02-…`).
22. [x] Pi boards' overlay (pi3, pi02w), 2026-10-05: no FAT in the
    kernel, MBR partitions required, USB printer and monitor forbidden,
    boot partition labelled `OSKBOOT`, `init_on_free=1 init_on_alloc=1
    usbcore.authorized_default=2`; the disk process takes `mmcblk0` as the
    boot medium and never lists `mmcblk0p1`. Not built: Faraday has no Pi
    image recipe yet.
23. [~] QR scanning against `docs/QR.md` (2026-10-06: `pNofM` written,
    public/secret labels on every code, progress kept across closing the
    camera, an image's codes stop at a completed transfer; left: camera
    choice, outline and "too fine", which need upstream, and routing to a
    wallet in progress) (audited and mostly built
    2026-10-05; `docs/QR.md` §6 lists both halves). Built since the
    audit, in `faraday/faraday-qr` and the scanner and QR sheet: BBQr read
    (P/T/U/J in 2/H/Z) and written (2), Specter `pNofM` read, `ur:psbt`,
    raw transaction hex, `crypto-account`, `crypto-hdkey` and
    `crypto-output` read, `crypto-seed` and private keys refused, the
    Faraday file envelope both ways, a scanned address checked against the
    loaded wallets, inverted symbols (PNG and camera), "3 of 10 · missing
    2, 5, 6" progress, wrong-box wording, UR/BBQr, frame-rate and
    part-size choices on the QR sheet, §3's limits, a Create cosigner key
    as `ur:crypto-account`, and the conformance tests (the BBQr conformance vectors,
    Faraday OS's envelopes, `crypto-account` read back by `osk-codec`,
    `faraday-core/tests/scan.rs`). `miniz_oxide`: `docs/deps/`. Not
    built: camera choice, the found-code outline, "too fine for this
    camera", several PNGs stopping when a transfer completes, progress kept
    across closing the scan sheet, `qrGuard`, `pNofM` written for
    SeedSigner, keys and descriptors routed to a wallet in progress.

24. [x] New key, 2026-10-05 (`faraday-core/src/keygen.rs`,
    `keygen_screen.rs`, `tests/keygen.rs`): a five-card flow (Length,
    Randomness, Entries, Check, Words) opened from a Create slot's New key
    and from Add a key's Make a new key. The options come from OpenSigner
    itself, not a copy: `opensigner_core::create::{SOURCE_ROWS,
    MIX_SOURCES}`, `load::COUNTS` (12, 24, 15, 18, 21) and OpenSigner's
    strings for every source and procedure name; every cryptographic step
    is `osk-entropy`'s and `osk-bip`'s, as OpenSigner's own wizard calls
    them (dice under the three procedures with the BitBox last-word choice,
    coins, cards, hex, camera noise, a mix of two or more, this device's
    generator). Sources the person makes are listed first as recommended;
    this device's generator carries OpenSigner's "trusts this device" and
    "Trust: this device's software · Not checkable". The device source asks
    the shell for 32 fresh bytes (`RequestEntropy`) when its card opens,
    and the bytes go to the key alone. The old New key (the session's
    bytes hashed with a counter, always 24 words, no choice) is gone.
    `faraday-core` now depends on `opensigner-core` for those lists and
    strings, which `docs/WALLETS.md` rule 5 would have moved into `core/`;
    the ask upstream is to move `SOURCE_ROWS`, `MIX_SOURCES` and `COUNTS`
    into `osk-entropy`. Still drawn from the session's bytes and a counter:
    vault creation seeds and nonces, GPG keys and Secure Boot keys
    (`vault_draw`), and the new-keyboard code.
25. [x] Stick pointer, 2026-10-05: the arrow is sized from the panel's
    pixels, not the 160 dpi the command line gives every laptop: 12×19 up
    to 1,799 lines on the short side, doubled from 1,800 (2880×1800, 4K).
    Test sticks are built without `dev=1`, whose kernel console printed
    over the app.

26. [x] Learn on every screen, 2026-10-06 (`faraday-core/src/learn.rs`,
    `tests/learn.rs`): a ? at the top right opens OpenSigner's own Learn
    pages for the screen, read from `opensigner_core::strings::EN` (no
    copy), by wallet kind on the wallet, Spend and Create screens; tabs
    for each page, scrolled by wheel or arrow keys.
27. [x] Public and secret exports, 2026-10-06 (`faraday-core/src/
    secrets.rs`, `tests/secrets.rs`, `docs/FLOWS.md` decision 6): the
    Outbox lists Public, Sealed and Unprotected secrets apart, a stick
    visit tags each and leaves an unprotected secret unticked; a secret
    goes into the open vault by default, and to the Outbox only through
    the secret sheet once the person ticks the warning; `put_outbox`
    refuses a secret. The FROST carry goes into the vault as a type-9
    signing round with its public PSBT to the Outbox, the next device
    finds the round when it opens that PSBT, and the round leaves the
    vault once used. Create's cards after Check: Secrets into a vault,
    Public files (each to the Outbox, with key files and the descriptor
    QR), Paper backup.
28. [x] `docs/OPENSIGNER-PARITY.md`, 2026-10-06: every OpenSigner crate and
    app feature, whether Faraday has it, and the plan for the rest.

29. [x] 2026-10-06: New key's Quiz card (OpenSigner's `Quiz`, every word
    from four, skip asks twice); BIP-85 (`faraday-core/src/bip85.rs`,
    `bip85_screen.rs`, `tests/bip85.rs`: OpenSigner's six applications,
    child words load as a key, passwords into the vault as entries, other
    values as notes, out only through the secret sheet); fresh shell bytes
    for vault, GPG and Secure Boot keys (`fresh.rs`); Create's Keys row
    wraps; Files' lists scroll above the foot; a hit outside the clip is
    not pressable.

30. [x] 2026-10-06: Add a key reads SLIP-39 shares, codex32 strings and
    Seed XOR parts (`forms.rs`, `tests/forms.rs`, against SLIP-39's and
    BIP 93's vectors), and BIP-39 words from the Spanish, French, Italian,
    Czech and Portuguese lists typed as their ASCII fold
    (`tests/languages.rs`); a session key keeps its list, the vault and
    the FROST share read it, and SeedQR is offered for English words only.
    BIP 129 both ways and Bitcoin Core's import file (`tests/bsms.rs`).
    Silent payments' receiving side (`silent.rs`, `silent_screen.rs`,
    `tests/silent.rs`).

31. [x] 2026-10-06: Explore a key (`explore.rs`, `tests/explore.rs`,
    against BIP-84's and BIP-86's published addresses) and the Lightning
    node key (`lightning.rs`, `tests/lightning.rs`, against LND's and
    ldk-node's node ids), both tiles on the Wallets page.

32. [x] 2026-10-06: a cosigner's key scanned during Create fills the
    waiting slot (Scan their key on each open slot); a vault entry exports
    for KeePass as a sealed KDBX 4 file (`tests/kdbx.rs`).

33. [x] 2026-10-06: Tools (`tools.rs`, `tools_screen.rs`, `tests/tools.rs`):
    OpenSigner's six calculators with Faraday's screen.

34. [x] 2026-10-06: other paper forms in the backup's seeds step
    (`paper.rs`, `tests/paper.rs`): a held key's words as 2 or 3 Seed XOR
    parts, or its seed as codex32 shares (2 of 3, 3 of 5), from fresh
    randomness, shown to copy by hand; each typed back into Add a key is
    the same key.

35. [x] 2026-10-06, after the owner's test on the Dell:
    - **Scrolling.** The app now reads `Event::Scroll` as the shell API
      states it, pixels the content moves up (it multiplied by 40 and
      flipped the sign, so a wheel notch moved ~1,900 pixels the wrong
      way); the desktop shell sends pixels with that sign (48 a notch);
      lists (Files, Visit, the wallet card) move a row once 56 units
      have built up; New key, BIP-85, Silent payments and Create vault
      scroll too (`tests/scroll.rs`). A wheel scrolls conventionally and
      two fingers drag the content, as the stick's pointer code intends.
    - **PDFs.** The blank template and the backup sheet go to the Outbox
      as PDFs on every shell (`pdf.rs`, which used to run only on the
      desktop), listed as "PDF to print"; the tour writes both beside its
      shots.
    - **The scanner.** `faraday/faraday-scanner` is a copy of
      `opensigner/shells/scanner` at c418768 that also reports what each
      pass saw; the stick and desktop shells use it. The scan sheet
      outlines the code found (green once read, amber while not), says
      "Too fine for this camera" after three passes of a code under 2.5
      pixels a module, and on the stick offers the cameras sysfs lists
      (`camera::list`, `Camera::choose`), over Faraday's storage
      channel (`StorageEvent::Cameras`, `StorageEvent::QrSeen`,
      `Faraday::take_camera`). Tests: `faraday-scanner`'s own, and
      `tests/scan.rs`.

36. [x] 2026-10-06, after the owner's test of the desktop app (sha256
    `d7931f0e…`) on the Dell:
    - **This device's generator and Camera noise did nothing on a real
      tap.** `touch()` (every mouse and touchscreen press) called `act()`
      alone; `keygen_device()` and `keygen_camera()` — which ask the shell
      for entropy bytes or turn the camera on — ran only after a keyboard
      key or from the snapshot tour's scripted `press()`. New key's
      Generator card sat at "0 of 1" forever on a real device, since
      nothing ever asked for the bytes. Fixed: `touch()` calls both after
      every tap, as `key()` already did (`faraday-core/src/lib.rs`).
    - **Make a vault from Create, finished, stranded on the Vaults tab.**
      The wallet-creation flow's own state and the way back to it
      (`vaults.back_to`) were intact; two tests proved both "Keep it open"
      and "Leave it locked" return to Create correctly
      (`tests/create_vault.rs`). The trap was the Vaults screen's own
      "Create a vault" button, shown even while a just-made vault's
      choice was still waiting: pressing it discarded `back_to` silently.
      Hidden while that choice is pending (`vault_screens.rs`).
    - **A locked vault's passphrase, already typed in.** The owner found
      this testing the above: "Keep it open" copied the first passphrase
      typed while making the vault straight into the Unlock screen's
      field and unlocked it at once — a vault just called "locked" opened
      itself with a passphrase the person never typed there. Fixed: it now
      opens the same Unlock screen any other vault does, passphrase blank,
      nothing carried over; the button reads "Open it now" (was "Keep it
      open", which read as already open next to "It is in the Outbox,
      locked"). Two tests: the field is blank and nothing unlocks on its
      own; typing the passphrase there unlocks it and returns to Create
      (`tests/create_vault.rs`).
    - **Home's Session card landed somewhere else than the Wallets tab.**
      It always opened `Screen::Wallets` (the session/wallet-card view),
      which has no "Create a wallet" button and says "None loaded" / "No
      wallet loaded" when the session is empty; the sidebar's own Wallets
      tab opens `Screen::Start`, whose empty state is the "Create a
      wallet" / "Restore a wallet" pair. Fixed: the Session card opens
      Start when the session is empty, Wallets once there is a key or
      wallet to show (`screens.rs`, `home()`).
    - **Stick visit: pulling the stick was easy to miss, and the Inbox's
      whereabouts weren't obvious.** "Remove the stick to load keys" was
      small muted text tucked in a corner; nothing on the screen said
      copied files land in Files' Inbox. Fixed: it is now a WARN-coloured
      banner the width of the screen, with an "Open the Inbox" button
      next to it that navigates to Files (`visit()`, `screens.rs`); the
      snapshot tour's own vault-creation step was updated to match the
      passphrase fix above.
    - **Copied in, but no way to load it.** A seed's words copied in from
      a stick (`*-words.txt`) landed in the Inbox with no action but
      Remove — worse, `classify()` had no case for them at all, so they
      were rejected at the copy itself ("not a PSBT, a wallet or a
      transaction"). New `FileKind::Words` (English, checksum valid, via
      `forms::typed_mnemonic`) recognises them; an Inbox item of that kind
      offers "Load this key" (`Action::LoadKey`, refused with a stick
      attached, same as typing the words in). Stick visit's "Copy" is
      renamed "Import" throughout, and gains a second button, "Import and
      load": it copies the chosen files and loads any of them that are
      Words once the stick is pulled, with no further press — the
      "Remove the stick" banner names how many are waiting
      (`visit.load_after`, `sticks_changed`). Tests: `tests/import.rs`.
    - **Vault file format, discussed with the owner (2026-10-06): kept
      `.ofv`.** It already reuses OpenSigner's own Argon2id and
      XChaCha20-Poly1305 (`osk-backup`'s `Cost`); the only custom part is
      the four-equal-slot container, which exists so a locked vault does
      not say how many passphrases it has — a property KDBX, a
      single-key database, cannot offer. `tools/vault/open.py` already
      answers "not locked into Faraday". No code changed.
    - **Create vault's Passphrases step: the dice tool was long-list-only,
      with no strength guidance and no way to see what was typed.** Now
      offers all three EFF lists (`osk_bip::diceware::List::ALL`, OpenSigner's
      own `dice_list_*` strings), rolls and bits scale to whichever list is
      chosen, switching lists clears rolls already thrown (`vaults.dice`
      is now `(usize, TextBox, List)`), the live count turns green at 77
      bits (docs/VAULT.md §3.1's own reference point), the step's guide
      text states that reference point in words, and a "Show passphrases"
      toggle unmasks every typed field at once (`CreateForm::shown`,
      `VaultAction::CShow`) — nothing is accepted from memory alone.
      Fixed a real bug on the way: the rewritten `dice_words()` first
      passed raw ASCII bytes to `List::word()`, which wants 1–6 face
      values, so every roll silently produced no words at all; caught by
      `tests/vault_passphrase.rs` before it shipped.
    - **List scrolling (Stick visit, Files, Wallets) felt laggy.** It
      moved a whole row only once 56 pixels had built up, so a 48-pixel
      desktop wheel notch (item 35) usually produced no visible motion at
      all before a sudden jump. `list_offset` is now a continuous pixel
      value like every other scrolled screen already used
      (`faraday-core/src/lib.rs::scroll`); the three list renderers that
      used to skip/take whole rows now draw every row and clip a
      continuous shift, the same pattern Files already had for its two
      columns (`screens.rs`: `wallets()`, `visit()`). `tests/scroll.rs`
      updated for the new behaviour.
    - **A second passphrase slot had no way back.** `VaultAction::CRemovePhrase`
      already existed and worked (confirmed with a test); the button was
      just a 12px ghost-styled text link tucked in the corner, easy to
      miss entirely next to "Generate with dice". Now `Style::Secondary`,
      the same visual weight as "Add another passphrase". Test:
      `tests/vault_passphrase.rs`.
    - **Lock, explained to the owner (2026-10-06): no code change.**
      There is no PIN anywhere in Faraday; "Lock" wipes the in-memory
      session and keys (`Faraday::lock`) and restarts the app fresh —
      getting back in means a vault's own passphrase or retyping a key,
      the same as any other session start. With nothing loaded, locking
      an already-empty session is visually a no-op, which is what read as
      "nothing happens". Not actioned: a "Locked" confirmation toast on
      the fresh session, offered but not yet asked for.
37. [x] 2026-10-06: the **Spend tab** (`docs/FAMILY.md`): family
    mode as a tab of its own, through Faraday's vault unlock and load.
    Owner's answers: the tab is called Spend; page 6 keeps the short
    Sparrow lines; the desktop app is for testing and keeps every
    function; the words route offers every single-key kind. Added the
    same day: a vault may hold only part of a wallet (its descriptor and
    one key), and the other keys come from words, a SeedQR, another
    vault, or a cosigner's signed copy. Built: `family.rs`,
    `family_screen.rs`, `family_text.rs`, `tests/family.rs` (nine
    tests), the snapshot tool's `spend` tour.
38. [x] 2026-10-06, the owner's observations after testing, one by one
    (what was done follows each, after the arrow):
    1. Vault passphrase fields still have no eye to show what is typed.
       → `vault_screens::secret_box`: an eye inside every passphrase
       field (Unlock, the Spend tab, Create vault, prompts, an entry's
       secret fields); `VaultAction::ShowTyped`. `tests/vault_passphrase.rs`.
    2. Dice passphrases: a line pointing to eff.org/dice (as text, it is
       offline), the length to aim for, and that the count turns green
       when the passphrase is strong.
       → `vaults::dice_aim` under the count ("Aim for 6 words…"),
       `vaults::STRONG_BITS`.
    3. Inbox items get **Add to vault** where it fits, one press: a text
       file (say, Gmail recovery codes) becomes a note in the open vault.
       → `FileKind::Text` (plain text Faraday reads as nothing else; it
       was refused at the copy before); **Add to vault** on Text (a note,
       titled by the file), Words (a key) and Wallet (a wallet) rows;
       `VaultAction::AddFile`. `tests/inbox.rs`.
    4. Stick visit, Import into the Inbox: a **Select all**.
       → `Action::VisitInAll`; one list of readable kinds
       (`stick_kind`), which now includes `.osk` and `.oskb`. `tests/visit.rs`.
    5. Stick visit, **Read the QR codes** is broken: after the button and
       pulling the stick (F4 on the desktop app) the key is nowhere.
       → A SeedQR read off a picture was refused ("seeds go in through
       Add a key"); now it is copied in as `<picture>-words.txt` and loads
       when the stick is pulled. `tests/import.rs`.
    6. The sidebar's "No keys loaded" status lists each loaded key by
       fingerprint instead of a count, down to the Settings row at most,
       and scrolls past that.
       → `sidebar()`: fingerprint and label per key; a wheel over the
       list scrolls it (`sidebar_scroll`). `tests/sidebar.rs`.
    7. Stick visit, Import into the Inbox: a scrollbar that can be
       dragged, for long lists.
       → `Action::VisitBar`, dragged by touch or mouse (`visit_drag`).
       `tests/visit.rs`.
    8. A new test stick, the full kit kept behind an option: only the
       files a backup of a **2-of-3 Taproot multisig** over the bacon,
       zebra and summer seeds makes. Public files (the whole descriptor,
       the split descriptor sheets as .txt and .pdf, the wallet .json …)
       on the stick; the three seeds inside a vault on it with the
       passphrase `a`. Pulling the stick (F4) asks to unlock the vault
       and loads the keys.
       → `testkit::backup_files()`: Faraday's own Backup output for the
       Taproot multisig (descriptor, wallet .json, BIP 129 record, Core
       import, backup sheet PDF, blank template PDF, split sheets as .txt
       and .pdf) and `vault.ofv` (passphrase `a`, the three seeds, load at
       unlock). The desktop app's TESTSTICK is this stick; `--full-kit`
       gives the old one; `faraday-testkit --backup DIR` writes it. On the
       way: Taproot multisig had no split sheets at all; it now has them
       (`backup::split_share`, `Format: P2TR`, rebuilt by `restore::merge`
       as `tr(NUMS, sortedmulti_a(…))`), and every split sheet gets a PDF
       (`pdf` sheet kind `share`). `tests/backup_stick.rs`.
    9. Add a key: the other BIP-39 languages behind an **Other
       languages** button.
       → `Action::EntryLanguages`. `tests/languages.rs`.
    10. Add a key, SLIP-39 (and every form): offer a QR scan wherever one
        can be read.
        → **Scan a QR code** on SLIP-39, codex32 and Seed XOR
        (`Action::ScanPart`; a Seed XOR part may be a SeedQR).
        `tests/forms.rs`.
    11. The scanner's camera preview is greyscale; show colour.
        → The shells already sent the NV12 chroma; the core dropped it.
        Kept and drawn (`ScanState::chroma`). `tests/scan.rs`.
    12. Sign a transaction: after a SeedQR of an unrelated seed, the line
        and button say "collect 2 more" when none has been collected;
        drop "more" when the count collected is zero.
        → Done in the panel line and button, Finish's line and the
        wallet card.
    13. Importing `savings-share-1-of-2.txt` (a descriptor share) says
        "remove the stick to load the 3 keys just imported": they are not
        keys. And Files should see that shares belong together and offer
        **Restore the wallet** there, without going through Wallets
        first: every imported kind leads where it is naturally used.
        → The visit's banner counts only words copied for Import and load,
        and names a vault to unlock; Files offers **Restore the wallet**
        on share rows once they add up (`Action::RestoreShares`).
        `tests/inbox.rs`.
    14. The Guided / Full switch: "Full" only hides the walk-through;
        find a label that says that.
        → **Steps only**.
    15. Terminology: "key" means a seed or private key that signs. A
        descriptor is a descriptor, an account xpub is an account key or
        public key, never "key" alone. The test stick's
        `key-1-9a6a2580-multisig-account.txt`, which holds a descriptor
        line, is the example to fix; go through every file name, label
        and string.
        → Test stick names `seed-N-<fp>-…` and `xpub-N-<fp>-<kind>.txt`;
        Outbox `xpub-<fp>.txt`; "Account xpub", "Show xpub QR", "Xpubs" on
        the printed sheet; walk-through texts say xpubs. The rule is in
        `docs/FLOWS.md` decision 7.
    16. Import and load of `key-1-9a6a2580-words.txt`: nothing loaded,
        nothing to press afterwards, and a red "not a PSBT, a wallet or a
        transaction" at the foot of the visit. It should say one key was
        copied and loads when the stick is pulled, then load it.
        → The words file's `# Test key 1…` note line made the whole file
        fail the word check, so it was refused at the copy. Note lines and
        numbers before words are now read past (`forms::file_words`); the
        log and banner say the key loads on pulling the stick.
        `tests/import.rs`.
39. [x] 2026-10-06, owner: the backup test stick carries an unsigned
    spend (`taproot-multisig-unsigned.psbt`); the desktop app starts every
    session on testnet, the test stick's network (`desktop_app()` in
    `faraday/shells/desktop`); the stick's app keeps mainnet
    (`wallet::START_NETWORK`). Desktop app built after this.
40. [x] 2026-10-06, owner: loading from an unlocked vault works like a
    stick visit's import. The vault's list (`screens::vault_panel`, rows
    from `Faraday::vault_rows`) has **Select all**
    (`VaultAction::ChooseAll`), a Load button that counts what it loads
    ("Load 3 seeds and 1 wallet"), and stays on Files after loading with
    each loaded row ticked and **Go to Wallets**. The Spend tab shows the
    same list after its unlock, everything ticked, and loads nothing until
    Load. `tests/boot.rs`, `tests/family.rs`.
41. [x] 2026-10-06, owner: the sidebar's corner names what is loaded in
    two groups, **Seeds** (fingerprint and label) and **Wallets** (name
    and quorum), each with its count, up to the last tab. A group that
    does not fit shows its first rows and "+ N more" (`sidebar_caps`):
    seeds' opens Explore's list of every seed, wallets' the Wallets list.
    A seed's row opens it in Explore (`Action::ExploreKey`), a wallet's
    its card. The corner no longer scrolls. `tests/sidebar.rs`.
42. [x] 2026-10-06, owner: a wallet the seeds loaded here can sign for
    alone shows a green dot and its quorum in green in the corner; one
    with some of its seeds here a grey dot; a watch-only one none
    (`screens::sign_readiness`, which counts `wallet::needed`). Ready
    wallets are listed first, so "+ N more" never hides them. The
    Wallets list's "k of m here" is green for the same wallets.
43. [x] 2026-10-06, while the owner was away:
    - "Collect 2 signatures" / "Collect 1 more signature" from one
      function, `wallet::collect_line` (`tests/collect.rs`).
    - The Spend tab opens the description in Files by itself when there is
      one wallet (several files of the same wallet count once) or split
      sheets that add up, on the paper route and on a vault route whose
      vault holds seeds alone (`family_auto_wallet`).
    - The Spend tab's Sparrow steps checked against Sparrow's QR guide
      (sparrowwallet.com/docs/airgapped-wallet-qr.html): Create
      Transaction, Finalize Transaction for Signing, Show QR, Scan QR,
      Broadcast Transaction. It names no Output Descriptor import, so the
      check page no longer does either.
    - Parity D1: the desktop app's camera choice.
    - Parity C4: vanity addresses, a tile on the Wallets page
      (`vanity.rs`, `vanity_screen.rs`, `tests/vanity.rs`, the snapshot
      tour's `spend` mode ends with one).
    - A message for the OpenSigner developer listing what Faraday still
      takes from `opensigner-core` was given to the owner to send.

44. [x] 2026-10-06, the owner's notes from testing on the Dell:
    1. Vaults after Create a vault offered "Open it now" / "Leave it
       locked" above the new vault's own Unlock: the same choice twice.
       → The banner is gone (`vaults.created`, `KeepOpen`, `LeaveLocked`,
       `Dismiss` removed; `just_made` is the file name only, no
       passphrase). The list shows the new vault with its Unlock, a toast
       says "Vault created", and when another flow made it, a back link
       ("‹ Create a wallet", `VaultAction::Back`) leaves it locked and
       returns there. `tests/create_vault.rs`.
    2. Generate with dice was a small ghost link. → A 40-high button
       under each passphrase's label, Primary while that passphrase is
       empty, beside "or type your own below".
    3. A finished backup said nothing about what next. → Under the cards,
       once all are done, "Backup done" with the Outbox count, **Write to a
       stick** (`Action::WriteAsk`: the visit at once in a clean process,
       else `Sheet::WriteOut`, which counts what is sealed, what is not in
       a vault and what is written, then Lock), Open the Outbox, Back to
       Create a wallet, and "Put the sheets in the Outbox"
       (`Action::BSheets`) when the backup put nothing there
       (`flow::column_foot`). The panel's count reads "Put in the Outbox".
       `tests/create_vault.rs`.
    4. Files said a vault was open and, in the Outbox, "Vault, locked".
       → An Outbox vault open in this session reads "Vault, open · sealed
       and locked at Lock" and "Written after Lock"; the Sealed group's
       line is "Locked under a passphrase before they are written".
    5. A vault passphrase could be typed with a stick attached, and
       Unlock then said to remove it (the typing had also spent the clean
       state). → No passphrase, new-vault passphrase or dice roll takes
       typing or focus while a stick is attached (`vault_key`, `CFocus`,
       `FocusPassphrase`, `Pick`, `Open`, `Dice`, `vault_create_check`);
       Unlock, the Spend tab's unlock and Create vault's passphrases show
       "Remove the stick, then type the passphrase" above the vault list
       and a field reading "Remove the stick first"
       (`vault_screens::stick_banner`, `stick_field`); the Vaults list says
       "Remove the stick to unlock" on locked rows; pulling the stick on
       Unlock puts the caret in the field. Add a key, New key and BIP-85
       were already refused with a stick in. `tests/boot.rs`,
       `tests/create_vault.rs`.
    6. Hold to show in a vault drew the field's background over the words
       while held. → Measured first, words drawn last (`vault_screens::secret`).
    Tour shots added: `backup-done`, `backup-write-out`, `unlock-stick`,
    `files-outbox-vault-open`. Not built.

45. [x] 2026-10-06, owner: "no trace left for a malicious stick". Audit
    of what a lock wipes and keeps is `PLAN.md` §5.3.
    - Lock drops secret-bearing files from the Inbox before the boxes
      are saved for the next process: seed words files, FROST carry
      files, entries files and every plain text file (wider than first
      proposed: a recovery-codes file not yet added to a vault is as
      secret as one that was). `Faraday::lock`.
    - Found on the way: a secret let out unprotected was classed by its
      contents, so a Lightning node key or a BIP-85 child's words could be
      listed as Public and ticked by default on a visit, and a lock lost
      which Outbox files were secrets. `Item::secret` and
      `Item::exposure()` now carry it, the `kept` set's `secret-out` keeps
      it across a lock, and a words file is a secret by kind.
    - `COREDUMP` forbidden on x86_64-uefi, pi3 and pi02w
      (`linux.fragment` and `kernel.forbidden`; the Pi fragments set
      `EXPERT`, which the prompt needs). Checked by the image build's
      kernel-config check, not yet run: no image built since.
    Tests: `tests/secrets.rs`.

46. [x] 2026-10-06, the owner's notes from the Dell (third batch):
    1. Idle: warning sheet with countdown at 5 min (`Sheet::IdleWarn`,
       what the lock wipes, seals, keeps, then power-off), lock at 10,
       power off at 20 counted across the lock with a countdown on the
       Locked sheet; Settings offers 20. `PLAN.md` §12 decision 3,
       `tests/idle.rs`.
    2. Selecting a field: a second click on a text field, or a drag
       across it, selects all of it; Backspace empties it, a character
       replaces it (`Faraday::select_all`, `field_action`). Delete maps
       to Backspace in both shells (no Ctrl in the shell API, which is
       upstream's). Also fixed: the dice-rolls box reset the rolls when
       pressed again. `tests/select.rs`.
    3. Stick visit: Import and load goes straight to Files, which shows
       the pull-the-stick banner (`pull_line`). `tests/import.rs`.
    4. Spend tab: with anything loaded and no route taken it opens on a
       Loaded view (`family_overview`): a transaction being signed,
       each wallet ready (Spend from it, past the map and the stick to
       its first page not done, or straight to signing a PSBT in Files)
       or short of keys (where they may be), seeds with no wallet (open
       a wallet file, Restore, or spend as a single-key wallet), and
       Walk through from the start. `tests/spend_loaded.rs`.
    5. Wallets page: Your wallets first, "None loaded", "Ready to sign ·
       m of m" marker (`wallet_rows`); the page scrolls (`content_h`).
    6. Rename: the caret was the glyph "▏", which the font lacks, drawn
       as "?"; now a drawn bar (rename and message). Pressing the name
       again keeps what is typed.
    7. A missing key once the quorum is here reads "Not needed" with no
       button (wallet card, Restore's seeds card, Signers); "Watch only"
       per key became "Not here".
    8–9. Every missing key has a way in: Load from vault when an open
       vault holds it (`VaultAction::LoadKeyOf`), Add its key otherwise,
       and `missing_keys_line`: a locked vault to Unlock (coming back),
       or "Not in <vault> · perhaps another vault, a paper backup or a
       SeedQR". On the wallet card, Restore, the Signers step and the
       Spend tab. Add a key returns to the transaction. `tests/vault_keys.rs`.
    10. Files' vault panel: Wallets with their seeds, checkboxes when
        several, Load wallet with keys / Load N wallets with keys
        (`VaultAction::LoadWithKeys`, `vault_wallets_with_keys`); the
        one-by-one list folds behind Each seed and wallet.
    11. Finish ends with the nonce check (`Session::nonce_check`: OpenSigner's
        `verify_signatures`, `deterministic`, `repeated_nonces`; each
        signature valid or not, recomputed when made here, "signed
        elsewhere" otherwise, a repeated nonce called out), Show the raw
        hex, and Decode it. `tests/finish.rs`.
    12. Decode a transaction (`decode.rs`, `Screen::Decode`): txid,
        wtxid, version, locktime, size, fee when a PSBT of it is here,
        inputs with sequences and witnesses, outputs with address, type,
        and change or receive of a loaded wallet; hex and QR. From
        Finish, from any Inbox transaction (a text file of raw hex under
        any name is now a transaction, and so is a scanned one), and a
        tile on the Wallets page.
    13. Not done: MuSig2 for a Taproot multisig with every signer here.
        A `tr(NUMS, sortedmulti_a)` wallet's addresses commit to an
        unspendable internal key, so no key-path signature can spend
        them. It needs a new wallet shape, `tr(musig(all),
        sortedmulti_a(k, …))`, which OpenSigner's policies do not have
        (only `tr(musig(…))`, all of n); an upstream ask, then a Create
        kind. Waiting on the owner.

47. [x] 2026-10-06, owner:
    - `docs/MULTIVENDOR.md`: the proposal to compute each cryptographic
      result twice (a second, independent implementation), with what
      OpenSigner is made of, candidates, what can be checked, where it
      runs, costs and a staged recommendation. Not built; to discuss.
    - Shared keys: one seed in several wallets fills its place in each,
      loads once, survives removing one wallet, and signs for whichever
      wallet the transaction spends from. Already so; pinned by
      `tests/shared_keys.rs`.
    - The Inbox put together (`inbox.rs`, the From the Inbox panel on
      Files): wallets once each however many files say them (descriptor,
      .json, BIP 129, multisig config, Core import, split sheets that add
      up), seeds once each (words files, SeedQR pictures read at a visit,
      SLIP-39 shares and codex32 strings once they add up, a new
      `FileKind::SeedPart`, secret, dropped at Lock), sealed `.oskb`
      backups opened with their passphrase. A wallet loads with the seeds
      here that are its keys (Load N wallets with their seeds); a seed a
      wallet in the Inbox names also brings that wallet when it loads on
      its own (Load this key, Import and load). A seed no wallet names is a
      potential wallet (`Sheet::Potential`): a BIP-39 (or SLIP-39)
      passphrase or none, the fingerprint it gives shown, then a
      single-key kind, or the wallet here that the passphrased key turns
      out to be. Seed XOR parts read as seeds on their own; they combine in
      Add a key. `tests/inbox_found.rs`.
    - Found on the way: the same wallet read from a multisig config (`'`)
      and from a descriptor (`h`) counted as two; `wallet::same_wallet`
      compares them in one form, in the session, the vaults and the Inbox.
    - Spend tab: "How to spend bitcoin", "What you have", and a note under
      the title saying the page is for a first spend and that it opens on
      a shorter page when something is loaded (`flow::Column::note`).
    - A Tools entry in the sidebar listing every flow with its BIP
      numbers: built as item 48.

48. [x] 2026-10-06, owner:
    - An account xpub in the Inbox that no wallet here uses and no seed
      here gives is a potential watch-only wallet ("Xpubs with no wallet",
      `inbox::FoundXpub`): its kind from its path (BIP-44, 49, 84, 86),
      chosen when it has no origin; a BIP-45/48/87 account is a cosigner's
      key and is not offered. `tests/inbox_found.rs`.
    - Create › All keys · MuSig2 (`NewKind::MuSig`): `tr(musig(…)/<0;1>/*)`
      over the keys' Taproot accounts (m/86'/coin'/0'), signatures needed
      always all keys, and the quorum card says losing any one key loses
      the money. `tests/create_musig.rs`.
    - New key as SLIP-39 shares (`KeyGen::slip39`, `deal`): 20 or 33
      words, m of n in one group, extendable, iteration exponent 1, the
      split's randomness one fresh system draw stretched with SHA-256;
      each share shown, then quizzed (OpenSigner's quiz over the SLIP-39
      list); the key loads from the master secret. The Length card lists
      what shares reveal (`SLIP39_FACTS`). Also from Tools. `tests/keygen_slip39.rs`.
    - Tools (`catalog.rs`, `Screen::Catalog`, in the sidebar): every flow
      by group with its standards, Find a tool (typing on the page goes
      there; "85" finds BIP-85), a tile that needs something first says
      what and opens nothing. The Wallets page's grid is trimmed to
      Restore, Back up, Sign and Check a message, Decode, and All tools.
      `tests/catalog.rs`.

49. [x] 2026-10-06, owner: scrolling that feels native, modern visual
    polish and a light mode. Built 2026-10-07: every item in
    `docs/MOTION.md` §5, measured in §6. Needs the owner's check on the
    Dell (wheel, two-finger flick, pull past an end, sheets, light
    mode), on a fresh build: the shell API gained `Wheel`, `ScrollEnd`,
    `Hover` and `HoverEnd`, and the stick shell sends them.

## Next steps

1. Build when the owner asks (commands under "Where we stopped"), and have
   the owner check scrolling, the PDFs and the scan sheet on the Dell.
2. Then `docs/OPENSIGNER-PARITY.md` §3's remaining items, in order.
3. The design canvas (`design/prototype/`) has not been republished
   since 2026-10-04 and no longer matches the app; the app is the
   reference now.
4. For later (owner, 2026-10-06): check interoperability against major
   external wallets' backup and export files — at least Sparrow, Nunchuk,
   BlueWallet, Coldcard and Trezor.

## Build machine

The HP (WSL2 Ubuntu): it has the pinned Rust
toolchain, `just`, podman (no docker) and QEMU with OVMF. How it is kept
up, and how the Dell pulls and tests are in
`~/Projects/hp-build-setup/README.md`. Never run `wsl --shutdown` or
reboot it without asking.

## Other sources the docs cite

Copied beside this project on the HP, in `~/Projects/`:
- `~/Projects/faraday-os-deprecated`: Faraday OS source, docs, profile, package repo and
  test scripts. Left behind on the laptop as build output:
  `out/` (ISOs), `work/`, `test/run/` (disk images), and the kernel's
  `src/`, `pkg/`, source tarballs and built packages under
  `kernel/linux-faraday/`.
