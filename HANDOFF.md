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
§3), as of 2026-10-07: only signing silent payment outputs, which waits
on upstream (BIP-375). The upstream asks and BIP-39 entry for Japanese,
Korean and Chinese are done (item 52), the start-up self-test (item 53)
and the silent payments wallet (item 54).

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

- The repository was restarted with fresh history (origin is
  `bitcoinpasada/faraday`); upstream `maxmoney21m/opensignerkit` is the
  remote `upstream` again (re-added 2026-10-07), with no shared history.
  Upstream files are synced by applying `git diff <last> upstream/main`
  (`git apply --3way`, then `git reset` to leave the index alone): last
  synced to `96e2144` (§16.140, kernel hardening) on 2026-10-07; before
  that `fa1ae03` (§16.139) and `c418768`.
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
      files: every wallet as `-wallet.txt`, `-wallet.json` and
      (multisig) `-multisig-setup.txt`, with its spends; Savings' split
      shares; descriptor QR PNGs for
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

50. [x] 2026-10-07, owner, New key and motion:
    - Randomness lists its options (`keygen::Way`, `WAYS`) in three
      groups (`keygen::Group`): "Your own entropy, words verifiable by
      hand" (Recommended, open, chosen from the start on Dice · Flip
      mode; also Coin flips and BitBox), "Your entropy, computer
      generates words" (closed: dice hashed, six as zero, hex, cards,
      camera, mix) and "Made by this device". For SLIP-39 shares the
      first group is not shown: shares are the device's work, so nothing
      there is checkable by hand, and BitBox is not offered.
      `KWay`/`KGroup` replace `KSource`/`KProc`/`KDiceFlip`.
    - BitBox (direct selection) rolls the last word too, as upstream
      §16.139 does it (synced 2026-10-07): six rolls a word, the last
      included (72 at 12 words, 144 at 24); the key keeps the last rolled
      word's high bits (7 at 12, 3 at 24) and the checksum replaces the
      rest (`osk_entropy::DiceRolls::entropy_under`). Faraday's own
      version, which rolled only those high bits, is gone; it made the
      same keys. The Rolls card shows each word's rolls over their bits,
      the 11 bits as its number, and the word, with Guided text on how.
    - Typed is the default entry everywhere on the desktop; a future Pi
      shell would want Buttons. The word-list sheet closes on a press
      outside it.
    - No cross-fades anywhere (`docs/MOTION.md` §5 item 3.9).
    - Words show live everywhere, alike in New key and a vault's
      passphrase dice: a typed roll or flip is taken as it is typed
      (`KeyGen::entered` records every one, buttons too; "Take these"
      and `KTake` are gone), the last word shows as soon as every entry
      is in, and each word is a pill (`Ui::word_pill`) that opens it in
      its list, with one note (`wordlist::PRESS_A_WORD`) in place of an
      "In the list" button per word.

51. [x] 2026-10-07, owner: Create a vault shows the bits the unlock
    cost adds (each preset, and under the choice) and each passphrase's
    estimated strength, dice bits plus cost bits, with the weakest in the
    summary panel; a typed passphrase's own bits are not measured
    (`docs/VAULT.md` §3.1, "The bits shown"). The cost rows are two
    lines so they fit a narrow column. `tests/vault_passphrase.rs`.

52. [x] 2026-10-07, owner: upstream §16.139 synced (`fa1ae03`, see "Git
    state"). Faraday now reads `Source`, `SOURCE_ROWS`, `MIX_SOURCES`
    and `WORD_COUNTS` from `osk_entropy`, the Learn pages from
    `osk_learn::EN`, and the Tools calculators from `osk_bip`,
    `osk_codec`; BitBox's last word is upstream's (item 50).
    Add a key offers all ten BIP-39 lists: Japanese, Korean and both
    Chinese lists are typed on OpenSigner's kana, jamo, pinyin and 注音
    keyboards (`osk_ui::widgets::keyboard`), drawn by Faraday, through
    OpenSigner's own word entry (`opensigner_core::load::LoadWizard`,
    `forms::word_typer`): keys that lead to no word are off, the words
    the keys can be are pills, a press takes one. Pinyin also takes the
    computer's keyboard (letters, then the tone 1 to 5). The keys shrink
    to fit 768 high with 24 words. `tests/languages.rs`.

53. [x] 2026-10-07, owner: the start-up self-test (`osk_selftest`, the
    eight checks: BIP-39 English and Japanese, BIP-32, a BIP-84 address,
    ECDSA, BIP-340, scrypt, Argon2id) runs when the display first
    arrives, before any input, as OpenSigner runs it. No key is
    accepted until it has passed (`Faraday::may_load_keys`, and New
    key's add). A failure replaces every screen with one naming the
    check, whose only control is Exit; every other press and key does
    nothing. Settings › About shows the result with Run again. Tests
    start the app as a shell does (`testkit::started`, a 1280 × 800
    display) so the self-test has run. `tests/selftest.rs`.

54. [x] 2026-10-07, owner: a silent payments wallet among the session's
    wallets (upstream §16.113's model, over `WalletPolicy::of_silent`):
    Silent payments › Address offers "Add as a wallet"; an `osk-silent`
    record in Files loads as one (`read_wallet` reads it whole); it
    saves into a vault as its record (it has no descriptor that reads
    back). `Kind::Silent`; its one key is its slot. Its card shows the
    key, the `sp1…` address and the labels handed out, and opens its
    page, the Silent payments flow, where a label shown is a label
    handed out on the record. A new card, Check a payment, reads a
    transaction from Files (raw or a finalized PSBT, OpenSigner's
    `silent::Check`), takes the transactions it spends from out of Files,
    and names the outputs that pay the wallet, to the address or a
    label, or says none does, or that it is unsigned, or that its key is
    not loaded. Without its key the page shows the address and asks for
    the key. Sending is not built. `tests/silent.rs`.

55. [x] 2026-10-07, owner: the Secure Boot test in real firmware,
    `faraday/tools/sb-ovmf-check.py` (`just faraday-sb-ovmf`, PLAN.md §8):
    `faraday-sb` makes test keys from a fixed seed and signs systemd-boot
    with the db key (`faraday-sb/examples/ovmf_kit.rs`); OVMF in QEMU,
    in Setup Mode, takes the `.auth` updates (systemd-boot's
    `secure-boot-enroll` writes them from `loader/keys/auto`), and with
    Secure Boot enforced runs the signed image and refuses the unsigned
    one and the signed one altered by one byte, for both the own-keys
    and the Windows-compatible policy. Passed on this laptop
    2026-10-07 (edk2-ovmf, systemd-boot 261). Not part of `just`: it
    boots QEMU ten times.

56. [x] 2026-10-07, owner: interoperability with the major wallets
    (`docs/INTEROP.md`). `faraday/tools/core-check.py` (not in `just`)
    runs every test wallet against Bitcoin Core on regtest through
    Faraday's own flows (`examples/interop.rs`): Core imports the
    `-bitcoin-core.json`, addresses agree, Core's PSBT is signed and
    finished by Faraday and accepted, and Core combines two Faraday
    partial signatures for each multisig; every check passed against
    Core 31.1.0. `tests/interop.rs` reads files from Sparrow's and
    BlueWallet's test suites (`tests/vectors/interop/`). Fixed: Coldcard's
    generic export loaded as BIP-44 legacy and, through
    `osk_bip::coldcard::parse`, with the account's own fingerprint in place
    of the master's (upstream fault, an upstream ask);
    `wallet::coldcard_export` reads it, checking Coldcard's `first`
    address. `testkit::public_files`. The gaps (`docs/INTEROP.md` §4) wait
    on the owner's choice of what to build.

57. [x] 2026-10-07, owner: five more themes, Nord, Catppuccin (Mocha),
    Tokyo Night, Gruvbox and Rosé Pine (Dawn, light) (`docs/MOTION.md`
    §3.7); Settings › Appearance shows each as a tile in its own colours;
    kept as `theme=<id>`. `faraday-snapshot WxH KIT OUT themes`.
    `tests/appearance.rs`.

58. [x] 2026-10-07, owner: the boot logo shows two marks again:
    Faraday's on the left, OpenSigner's key (upstream's `mark.txt`) on the
    right, each over its name (`faraday/image/overlay/common/make-logo.py`).
    Faraday's mark is the owner's choice D, the keyhole shield, in blue
    (`faraday-mark.txt`; the other candidates and the sheet the owner chose
    from are in `design/marks/`). The sidebar draws the same grid in
    place of the shield icon (`Ui::mark`), whole pixels per square, in
    the theme's accent. Not built into an image yet. Centred on a PC by
    item 59.

59. [x] 2026-10-07, owner: `docs/INTEROP.md`'s first two gaps, and the
    logo centred. Create's key slots take Coldcard's, Passport's and
    Unchained's cosigner key files and the generic export's BIP-48 and
    BIP-45 accounts (`create::read_keys`, `key_for`: the account the
    kind needs, SLIP-132 keys made `xpub`/`tpub` by `wallet::plain_xpub`);
    the slot offers any file with such a key. A wallet reads from a
    receive and a change descriptor (one per line, or Core's
    `listdescriptors`, multisig first, else BIP-84, 86, 49, 44), a receive
    descriptor alone, keys with no derivation (Specter's .json) and a
    `Receive:` label, each descriptor checked with its checksum before it
    is made `<0;1>` (`wallet::multipath`). `tests/interop.rs`;
    `core-check.py` now checks `listdescriptors` against Core. The PC
    command line gains `fbcon=logo-pos:center`; not yet seen on a built
    image.

60. [ ] 2026-10-07, owner: the Raspberry Pi build and the small-panel
    layout. The screens are done (2026-10-08, "Left to do" 1–9 below);
    left: build the Pi image and try it on the Pi 3 and panel (10).

    **Pi build (done).** `faraday/faraday.just` gains `faraday-pi-bin`
    and `faraday-pi-image` (the stick recipes with the root `board`/
    `panel`, default `pi3` + `waveshare-28dpi`; `faraday/image/overlay/
    boards/pi3` already existed). Output `out/pi/faraday-pi3-waveshare-
    28dpi[-dev].img`. `local/remote-build.sh pi [--dev]` builds it on
    the build machine and pulls back only the image (not `out/pi/`'s Buildroot
    tree). The build machine now has the armv7 target and `gcc-arm-linux-
    gnueabihf` (installed through `/etc/sudoers.d/faraday-arm-toolchain`).
    No Pi image has been built yet: the first run was stopped on purpose
    until the screens below are done.

    **Small-panel layout (`Faraday::is_compact`).** A display under
    600 dp wide (the Pi's 480×640 @ 286 dpi is 268 dp) is laid out at its
    own density, one column, no sidebar; desktop is unchanged and every
    faraday-core test passed. New files `compact.rs` (Home, the page
    bar, sheets, the docked keyboard) and `compact_screens.rs` (wallet
    card, Add a key). Done and looked at on renders:
    - Home: status card, waiting prompts, the sidebar's places as a
      2-column grid, Lock and power; scrolls.
    - Every non-flow page has a bar: back, its name, ?, keyboard button.
    - Every step flow (`flow::column`, 11 screens) is a page per step:
      numbered dots to move between steps, controls first, the Guided
      text under "About this step"; no step open shows the step list.
    - Create a wallet and New key, every step, fit (`buttons_and_next`,
      `stepper` in screens.rs). New key starts on the dice buttons on a
      small panel (`KeyGen::typing`), typed on desktop.
    - Wallets start page as one column of rows; a wallet's card is its
      own page (`compact_screens::wallet_card`), silent wallets too.
    - Add a key: every list (English too) on OpenSigner's word keyboard;
      space, Tab, Enter and Add key take a word typed whole on a USB
      keyboard (`forms::take_typed`); a scanned SeedQR still adds.
    - Docked keyboard: OpenSigner's `KeyboardKind::Passphrase` (Path on
      Explore) at the panel's foot whenever a focused field takes typing
      (vault passphrases, BIP-39 passphrase, rename, message, SLIP-39/
      codex32, vanity prefix, the potential sheet); offered from the bar
      on Explore, BIP-85, Tools, Catalog, Lightning (`compact::Osk`).
    - Sheets: lock, write-out, idle, lock-ask, locked, network, power,
      secret, potential, new input, QR (format/speed/size as stepping
      buttons), scan, Learn, word list; a sheet taller than the panel
      scrolls (`sheet_scroll`).
    - Sign a transaction: paged steps, with the signatures and the main
      button fixed at the foot (`spend_bar`, shares `spend_status` with
      the desktop panel).
    - `faraday-snapshot` takes `WxH[@DPI]` and a `compact` tour;
      `just faraday-snap 480x640` and `faraday-snapshots` render the
      panel sizes at their real density.

    **Left to do**, seen on `faraday-snapshot 480x640@286` (full tour).
    1–9 done 2026-10-08, each looked at on renders at 480×640 and on tall
    renders (480×1800 and 480×2400 @286) that show a whole page:
    1. [x] Sign a transaction's step bodies: the Transaction table's rows
       are two lines (what and how much, then where and its note); the
       table of inputs and outputs puts each outpoint or address under its
       line; Signers (plain and threshold) and Signatures put the label
       and what it does on a second line; Finish's two cards stack, the
       signed PSBT's buttons under each other when they do not fit; the
       nonce check wraps; MuSig2's Nonces and the Path step's key chips
       wrap. New `screens::wrap_buttons` lays a row of buttons that folds
       at the column's width, each cut to fit.
    2. [x] Decode a transaction (way back in the bar, to where it was
       opened from: `screens::decode_back`) and Check a message: one
       column, each value under its name, scrolling.
    3. [x] Back up: no side panel on a small panel; Words on the seed,
       the key choice, the words in two columns with their indices, the
       SeedQR grid with the pinned row named under it, Check my copy, the
       public files one under another, Split's choices under the label,
       Backup done wrapped. `buttons_and_next` now wraps on a small
       panel, with Continue on the last row when there is room.
    4. [x] Restore: the first card's ways in and why not, and the sources
       (`restore_sources_compact`): each source's name over its buttons.
    5. [x] Spend tab: the Loaded view (`family_screen::loaded_compact`)
       with its own bar; the walk-through's holding cards, vault card,
       stick rows, words route, Check the money (addresses whole, to
       compare character by character), Bring, Who signs, Put everything
       away. Step dots in any flow with more steps than fit (11, 14) show
       a window round the open step with arrows (`flow::paged`).
    6. [x] Vaults: the list and Unlock as cards with their buttons under
       them (`vault_screens::list_compact`, `unlock_compact`); a vault's
       contents as two pages, the kinds and items as a list, a chosen item
       (or a form, prompt, Save, Sign a file, images) as its own page that
       the bar goes back from (`Vaults::item_open`,
       `VaultAction::ItemBack`, `vault_screens::contents_detail`); Create
       vault without its side panel, the summary and Create vault after
       the last step (`create_summary`, `create_button`), the dice lists
       one under another; Create a wallet's Secrets and Public files rows
       two lines (`screens::put_row`).
    7. [x] Files and Stick visit (`files_compact`, `visit_compact`): one
       scrolled column each; Files has the stick's state first, then the
       open vaults (`vault_panel_compact`), From the Inbox
       (`inbox_panel_compact`), each Inbox and Outbox file as a card with
       its buttons wrapped (`inbox_actions`, `file_card`); the visit has
       Import before Write, every stick file listed (no clipped list).
    8. [x] Tools catalog (a tile a row, Find across the panel), the
       calculators, Lightning, Explore, Settings and About (each section
       a measured card, `screens::section`), BIP-85's length and value.
       `button_rows` cuts a label wider than its row.
    9. [x] `tests/small_panel.rs`: Home offers every place; a wallet is
       made by touch alone from a key rolled on dice; a passphrase field
       brings the keyboard and stays above it. Found doing so: the
       keyboard stayed up on a vault's list with no field; a step flow's
       bar had neither ? nor the keyboard button (`compact::
       flow_bar_tools`); a keyboard put away did not come back when
       another field was pressed (it now follows which field has focus,
       `osk_field`); and a field the keyboard came up over stayed under
       it (the page now scrolls by as much, `Faraday::osk_reveal`, from
       where the press was). `OskPress` is exported for the test.
    10. [~] Built 2026-10-08 (`local/remote-build.sh pi`, release, from
        the uncommitted tree with the walk-throughs behind a tap):
        `out/pi/faraday-pi3-waveshare-28dpi.img` on the Dell, 68 MiB. Not
        yet tried on the Pi 3 and panel.
        To try the panel layout by touch on the Dell meanwhile:
        `just faraday-panel 2.8` (or `5` for a phone): the desktop app
        with `--panel INCHES [--aspect W:H] [--ppi N]` opens a window
        that measures that diagonal on the laptop's own screen (pixels
        per inch from its EDID; 277 on the XPS 13 9360), and takes the
        touchscreen's touches (`WindowEvent::Touch`, new in the desktop
        shell). Under Hyprland the recipe focuses the eDP screen first,
        so the window opens there and floats at its size.

    **Pinned forward action and scroll cues (owner, 2026-10-08).** The
    owner found Continue below the fold on Create a wallet's Kind step.
    On a small panel now:
    - A page's forward action is pinned in a bar at the panel's foot and
      the page scrolls above it (`ui.pinning`/`ui.pin`, drawn by
      `screens::draw_compact`, `PIN_H`; the page is laid out again in
      the same frame when the bar comes or goes). Every `next_button`
      and `buttons_and_next` next pins (they return whether they drew
      in place), and `screens::pin_button` for the rest: Make the
      wallet, Deal the shares, Sign the message, Save all then Continue
      (Secrets), Unlock (Vaults and the Spend tab), Create vault, a
      vault form's Save and a prompt's Open/Load/Seal, Add key, Open the
      wallet (Restore), Write to a stick (Backup done), Next share.
    - Sign a transaction's foot bar shows the open step's Continue while
      the step is read, and the sign/finish button once there.
    - While the docked keyboard is up the bar is hidden; the keyboard's
      Done is Enter and puts the keyboard away, bringing the bar back.
    - A sheet's buttons stay at its foot and its body scrolls above them
      (`compact::sheet`, `ui.sheet_pinning`).
    - More below: a deeper fade at the foot of the scrolled region, and
      the scroll indicator always shown on a small panel while the page
      goes on (no pointer to hover there).
    - Steps that pick one option keep their explicit Continue.
    Tests: `tests/small_panel.rs` checks each Create a vault step's and
    Create a wallet's Kind forward action is in view without scrolling,
    at 2.8 inches and on a 5-inch phone; they fail with pinning off.

    The owner's calls on the layout (2026-10-08):
    - **Guided text behind a tap.** On a small panel a step's or a page's
      walk-through is a row, "About this step" / "About this page", after
      the controls; a tap opens it and a second closes it
      (`Action::About`, `Faraday::about_open`, `flow::about`). The panel
      has no Guided switch, so the row is offered in Steps only too: the
      tap is the choice. The Spend tab's leads are its own text, not
      Guided text, and stay shown. Desktop unchanged.
    - **Home's order, the agent's choice:** Spend, Wallets / Vaults,
      Files / Scan, Stick visit (with a stick in) / Tools, Settings: the
      device's job first, then where keys and transactions come from,
      then the ways in and out, then what is used now and then.
    - Two lines per word in New key's "Words so far": the owner is fine
      with it as it is.
    Also fixed then: Words chosen by the dice put each word's result on a
    line under its rolls on a small panel (it ran off the panel).

61. [x] 2026-10-07, owner's batch of seventeen (desktop):
    1. Nord is the theme a first start is in (`Theme`'s default).
    2. About ends with "Visit nakamotoinstitute.org to learn about
       Bitcoin’s history, economics, and technology. Not affiliated."
       The curly apostrophe is baked since 2026-10-07, with ‘ “ ” – —
       (`SYMBOLS` in `tools/fontbake/src/main.rs`, `just fonts`).
    3. Every sheet closes on a press beside it, as its own way out does
       (`Ui::outside`, set per sheet in `screens::draw`; `sheet_box` and
       `compact::sheet` take it): Cancel for most, Not now for the stick
       lock, Ignore for a new input device; the word list keeps its own,
       the idle warning closes on any input as before. `tests/outside.rs`.
    4–5. Home's tiles read **Vaults** (now to the Vaults tab, not
       straight into Create a vault) and **Wallets**.
    6. Create's Keys step: New key on the first empty slot is the
       primary button, Continue only once every slot is filled.
    7. Randomness, first group: Dice · Words chosen by the dice (the
       default now; "(BitBox)" dropped in Faraday's `procedure_name`),
       then Dice · Flip mode, then Coin flips (`WAYS` order). "Made by
       this device" opens and closes like the others and starts closed
       (`groups_open[2]`).
    8. Words chosen by the dice stop at the last word's kept bits:
       `KeyGen::dice_needed`/`rolls_for`, six rolls a word and
       ⌈kept/2⌉ for the last (70 at 12 words, 140 at 24); the checksum
       word shows on the last roll. `KeyGen::dice_entropy` fills the
       unrolled faces with zero bits before `osk_entropy`'s
       `entropy_under`, which is unchanged (upstream §16.139 still
       rolls 72); same keys. The Learn page says a device may stop at 70.
       `tests/keygen.rs`.
    9. The overlay scrollbar is 6 wide, shows while the pointer is over
       the region's right edge, and is held and dragged by pointer or
       finger (`ui::BarGeometry`, `BAR_GRAB`, `Faraday::bar_at`,
       `bar_drag`; `docs/MOTION.md` §3.8). `tests/scroll.rs`.
    10. Carets blink (`Ui::caret`, `caret_char`, 530 ms halves, steady
        with reduce motion). `tests/caret.rs`.
    11. Vaults are named in Create's last step, now "Name and
        passphrases"; the file is `vaults::vault_stem` + `.ofv`,
        numbered when taken, `vault.ofv` when left empty (`docs/VAULT.md`
        §6, `PLAN.md` §12.1 reversed). `tests/vault_name.rs`.
    12. A vault in the Outbox is no longer offered as a QR code (no vault
        fits the envelope's 256 KiB; `qr_fits`), so the "Cannot make a
        code" toast is gone.
    13. The Outbox group reads "Written sealed · Sealed under their
        passphrases before they reach a stick".
    14. The Vaults list's open vault: **View contents** and **Lock**
        (`LockAsk`, which locks the session: every open vault).
    15. Steps only is the default and on the left, Guided on the right;
        one setting for every flow, kept as before.
    16. The in-app mark is drawn as smooth outlines (`Ui::mark`, the
        shield and keyhole at the grid's proportions, `Canvas::
        fill_contours` added to osk-ui); the boot logo keeps the grid.
    17. The sidebar's "N vault(s) open" opens Vault contents, whose Lock
        is now the primary button.

62. [x] 2026-10-07, owner: **settings on the stick.** Settings survive a
    lock (the `kept` set in `/run/faraday`, `memory.rs` `kept` and
    `restore_kept`) but not a power-off; `PLAN.md` §5.2 lists settings
    in the Outbox and nothing writes them. Build that.
    1. **The file.** `faraday-settings.txt` on a stick's data partition,
       plain text, because the theme and scale are wanted on the
       passphrase screen and the keyboard layout (FLOWS' Settings line;
       not built yet, and it goes in this file when it is) is needed to
       type the passphrase, so it cannot be inside a vault. First line
       `faraday-settings 1`, then the `key=value` lines `kept` already
       writes: `scale`, `theme`, `motion`, `guided`, `qr-ms`,
       `idle-lock`, `idle-off`. One function writes both the `kept`
       text and the file's body.
    2. **Not in the file:** `seal-amounts`, and the network. A stick's
       data partition is not covered by Secure Boot, so the file is the
       one thing someone with the stick can change while the signed
       kernel stays the same. Nothing read from it may make a fresh
       boot less protected than the defaults: the signed-amount memory
       is sealed unless turned off on the device in this session, and
       every session starts on the network it starts on now.
    3. **Reading.** At boot, from the boot stick only, by the same path
       that brings the boot stick's `.ofv` files into the Inbox
       (`lib.rs`, "The boot stick's vaults come into the Inbox by
       themselves"), and applied before the passphrase prompt. The file
       is untrusted input (`PLAN.md` §4.4 gains a line): over 4 KiB or
       a first line other than `faraday-settings 1` and it is ignored
       whole; otherwise each line is taken only if its key is known and
       its value is one Settings itself offers (`scale` 50 to 300,
       `theme` a `Theme::from_id`, `qr-ms` in `QR_SPEEDS`, `idle-lock`
       in 2, 5, 10, 30 and `idle-off` in 10, 20, 30, 60, so **never**
       (0) is not read from a file and is chosen on the device each
       session), and any other line is skipped. `restore_kept`'s
       `idle-lock` and `idle-off` take any number today; the file's
       reader checks the lists, and `restore_kept` may share it. The
       file is not an Inbox item and is not shown in Files.
    4. **Writing.** A stick visit lists one more row, **Settings**,
       for `faraday-settings.txt`: ticked on the boot stick when the
       settings differ from what was read from it at boot (or from the
       defaults when it had no file), unticked on any other stick,
       where the file would say Faraday was used. Written the way the
       disk process writes everything: a new file, read back, compared,
       renamed over the old one.
    5. Desktop shell: the same, with the test stick's folder as the
       boot stick. The test kit writes no settings file.
    6. Docs: `PLAN.md` §5.2 (settings are this file, not an Outbox
       item) and §4.4, `docs/FLOWS.md`'s Settings line and the stick
       visit, and the README's "Stateless" list once built (it now says
       settings are lost at power-off).
    Tests, a new `tests/settings_file.rs`: a boot stick with the file
    starts in its theme and scale before the passphrase; a bad value,
    an unknown key, `idle-lock=0` and `seal-amounts=0` change nothing;
    an oversized file or a wrong first line changes nothing; a setting
    changed here ticks the Settings row on the boot stick and the write
    reads back as the settings now; another stick's row is unticked.
    **Built** as specified: `faraday-core/src/stick_settings.rs` (the
    file's text, the strict per-line reader shared with the kept set,
    the boot-stick read, the visit row's default), `kept` carries
    `stick-settings` (what the boot stick holds) so a later process
    neither reads the file again nor loses what to compare;
    `faraday_files::write_settings` writes over the old file, chosen in
    `write_any` by name and first line, so the disk process and the
    desktop's folders both do it. The idle choices Settings offers are
    `stick_settings::IDLE_LOCK_CHOICES`/`IDLE_OFF_CHOICES`, which the
    Settings screen now draws from. The desktop's TESTSTICK is reported
    as the boot stick, so plugging it in now also reads its vaults into
    the Inbox and leaves Home on screen, as the device does. Tests:
    `faraday-core/tests/settings_file.rs`,
    `faraday-storage/tests/settings_write.rs`. Not yet on a device.

63. [ ] 2026-10-07, owner: **kernel hardening**, from an audit of the
    `.config` of that day's x86 stick build (Linux 6.6.84,
    `out/pi/x86_64-uefi-efi-framebuffer/output/build/linux-custom/.config`)
    against the Kernel Self Protection Project's list. The findings, with
    a reason for each option, are in `local/upstream-kernel-hardening.md`.
    **The owner gave that file to the OpenSigner developer** and is
    waiting for upstream's changes. Most of it is upstream's x86 fragment,
    which Faraday's is a copy of.
    - **Order.** When upstream's changes land, merge them (`just
      upstream-diff` lists them) and carry them into Faraday's copies under
      `faraday/image/overlay/boards/`. What upstream does not take, or that
      is Faraday's alone (the disk process, the grant helper, the clean
      marker), Faraday does in its overlay. Ask the owner before starting
      the Faraday-only part ahead of upstream.
    - **What it covers** (the file's sections):
      1. To `kernel.forbidden`, after an `strace -f` of a full session in
         the dev image under QEMU confirms nothing uses them: `IO_URING`,
         `AIO`, `SYSVIPC`, `KEYS`, `IA32_EMULATION`,
         `MODIFY_LDT_SYSCALL`, `X86_16BIT`, `X86_IOPL_IOPERM`,
         `BINFMT_MISC`, `CROSS_MEMORY_ATTACH`, `KCMP`, `LEGACY_TIOCSTI`,
         `CRASH_DUMP`, `DEVPORT`, `GPIO_CDEV`, `HIDRAW`, `USB_HIDDEV`;
         `LEGACY_VSYSCALL_NONE` in place of `_XONLY`. `PERF_EVENTS` is
         selected by x86 and stays there.
      2. `x86_64_defconfig` leftovers, to `kernel.forbidden`: `VIRTIO*`
         (including `VIRTIO_BLK`, a block driver: check that
         `sb-ovmf-check.py`, which attaches the image `if=virtio`, still
         passes; only the firmware reads that disk), `BLK_DEV_SR` and
         `CDROM`, the vendor HID drivers and `HID_PID` (keep
         `HID_GENERIC` and `HID_MULTITOUCH`; `HID_APPLE` only if Mac
         keyboards are wanted), `HOTPLUG_PCI`, `ACPI_TABLE_UPGRADE`,
         `EFI_CUSTOM_SSDT_OVERLAYS`, `EFI_RUNTIME_MAP`.
      3. To `kernel.required`: `HARDENED_USERCOPY`, `FORTIFY_SOURCE`,
         `SLAB_FREELIST_HARDENED`, `SLAB_FREELIST_RANDOM`,
         `SHUFFLE_PAGE_ALLOCATOR`, `RANDOM_KMALLOC_CACHES`,
         `LIST_HARDENED`, `BUG_ON_DATA_CORRUPTION`,
         `SCHED_STACK_END_CHECK`, `RANDOMIZE_KSTACK_OFFSET_DEFAULT`,
         `ZERO_CALL_USED_REGS`, `PANIC_ON_OOPS`,
         `SECURITY_DMESG_RESTRICT`, `INIT_ON_ALLOC_DEFAULT_ON`,
         `INIT_ON_FREE_DEFAULT_ON` (the Pi's `cmdline.txt` is on the
         card and anyone holding it can edit it); `SLAB_MERGE_DEFAULT`
         to `kernel.forbidden`.
      4. `SECURITY_YAMA` with `kernel.yama.ptrace_scope=3` set in `rcS`;
         `SECURITY_LOCKDOWN_LSM` forced to confidentiality;
         `SECURITY_LANDLOCK`, with a Landlock ruleset in the app and in
         `faraday-disk` (not the grant helper: §12 decision 8). Seccomp
         is not available, because it needs `NET`. The app calls
         `prctl(PR_SET_DUMPABLE, 0)` at start-up.
      5. Command line: `efi=disable_early_pci_dma` on x86. `rcS`: set
         `kernel.kptr_restrict=2`; mount `/proc` with
         `hidepid=invisible`; mount `/proc`, `/sys` and `/run` with
         `nosuid,nodev,noexec`, and `/dev` with `nosuid,noexec`.
    - **Tests:** the existing QEMU checks (`tools/stick-qemu.py`,
      `disk-test.py`, `sb-ovmf-check.py`) and one boot on the Dell
      (keyboard, touchpad, webcam, stick, power-off). The Pi boards'
      `.config` gets the same audit at their first build.
    - **README.** The owner's draft of the new README, with the security
      model, the comparison with amnesiac live systems and "a live
      system with networking compiled out", is
      `local/README-draft.md`. **It is not published until upstream's
      hardening has landed and been merged here**, and then the owner
      adds the discussion to it. Until then the draft states two things
      the build does not yet make true: that the only storage the
      kernel sees is USB (and the Pi's card), and that nothing but the
      listed USB classes has a driver (`VIRTIO_BLK`, `BLK_DEV_SR`).
    - **Progress (2026-10-07, late session).** Upstream took most of this
      as §16.140 (`96e2144`; read `docs/PLANNING.md` §16.140 for what it
      took, what it corrected in the note, and what it left).
      1. Done: upstream synced to `96e2144` (Git state).
      2. Done: carried into Faraday's copies. Upstream's x86 board
         additions appended to `faraday/image/overlay/boards/x86_64-uefi/`
         `kernel.forbidden`, `kernel.required`, `linux.fragment` (marked
         "Upstream hardening ... at 96e2144"); upstream's common hardening
         block appended to Faraday's `common/kernel.required`; upstream's
         `/proc`, `/sys`, `/dev` mounts, `kptr_restrict=2` and
         `ptrace_scope=3` put into Faraday's `rcS`. The Pi's common
         `linux.fragment` and `kernel.forbidden` are not overlaid by
         Faraday and come through as they are. `COREDUMP` is now in
         upstream's common list as well as Faraday's board lists; the
         duplicate is harmless.
      3. Not done, as upstream also left them: `efi=disable_early_pci_dma`
         (waits for hardware tests), Landlock for the app and
         `faraday-disk`, `PR_SET_DUMPABLE` (largely covered now by
         Yama scope 3), and §16.140's "later round" list.
      4. Done: both stick images rebuilt on the build machine from this tree
         (`out/stick/faraday-x86_64-uefi.img` 20:41,
         `-dev.img` 20:50); the build's kernel-config check passed, and
         the generated `.config` was read back for the options above.
         `faraday/tools/qemu-check.py` boots the release image into the
         app; `faraday/tools/disk-test.py` passes every check on the dev
         image (stick rule, grant helper, USB authorisation) with the new
         mounts, Yama and Lockdown. Not built: the Pi images. Not done:
         a boot on the Dell or a Pi.
      5. Done: `README.md` written from `local/README-draft.md`, with the
         hardening, and corrected where the draft claimed more than the
         tree has: no keyboard layout setting, no QR transfer tab, no
         `tools/build/linux` (so `just faraday-linux-bin` does not run),
         and no shared history with upstream.
    - Also found while drafting the README, for when this is picked up:
      `PLAN.md` §4.1 says init is the only root process after boot, but
      the app loop's shell (`inittab`) and the disk process's restart
      loop (`rcS`) are root too. Neither reads input. And `PLAN.md` §3
      says the Pi board turns on `USB_STORAGE`, `SCSI` and `HID`, but
      `boards/pi3/kernel.forbidden` still forbids them; the owner has
      not yet decided which is right.

64. [x] 2026-10-08, owner's batch (built and tested in source; not yet
    on a device, no image built):
    1. **Not air-gapped** on the desktop app: `Sheet::NotAirgapped`,
       `Action::AirgapUnderstood`, `Faraday::airgap_warned` and
       `mainnet_asked`. Mainnet chosen from Network waits for **I
       understand** (Cancel leaves testnet); mainnet reached by loading
       a mainnet wallet or PSBT brings the sheet up over it at the next
       frame, with no way past but the acknowledgement. Once a session
       (a lock starts a fresh testnet app). Never on the device.
       `tests/airgap.rs`.
    2. Scrollbar held and dragged everywhere: the word-list and Learn
       sheets drew their own thin bars and now report their regions like
       a page (`docs/MOTION.md` §3.8). The stick visit's list keeps its
       own draggable bar. Tests in `tests/scroll.rs`.
    3. Tokyo Night is the first-start theme (`Theme`'s default), on every
       shell.
    4. Laptop touchscreens on the stick: positions were scaled by the
       Pi panel's 640×480 grid. A HID touchscreen's ranges are now read
       from its report descriptor in sysfs (`shells/stick/src/hid.rs`,
       no ioctl), and the touch parser reads slot 0 only, so a second
       finger neither moves nor ends the first. Not covered: Wacom panels
       (no `HID_WACOM` in the kernel; `hid-generic` binds them) and
       Intel THC/QuickSPI touchscreens on the newest laptops (need a
       newer kernel). The verbose boot report prints each touchscreen's
       range.
    5. Panic key: Super+S held 2 s (`keyboard::PanicTimer`,
       `Stroke::Panic`, `Wake::Panic`) blanks the panel, drops the app
       and exits for `poweroff -f`, from any screen; an unbelieved
       keyboard's chord is held back like its keys. Super never types.
       Shown beside Power off in Settings (device only), and in the
       README and `PLAN.md` §5.3.
    6. Zeroize audit. Found and fixed: typed secrets (`EntryState`'s
       words and passphrase, `TextBox`, Tools, Lightning, the backup
       check, New key's entries, the potential-wallet passphrase) were
       `String`/`Zeroizing<String>`, which leave every pre-growth buffer
       unwiped; now `secret_text::SecretText`. Secret strings built word
       by word start with room (`secret_text::room`), private keys are
       hex-encoded with no temporaries (`secret_text::hex`), the
       displayed copies of typed secrets are `SecretText`
       (`Ui::fit_secret`). Inbox/Outbox `Item`s, a QR sheet's source
       and the session seed wipe on drop; SLIP-39/codex32 part lists are
       allocated whole. The disk process (which outlives locks) wipes
       every frame and file it passes, at both ends of the pipe, and its
       FAT writer's cluster buffer; PNG pixels read for QR codes are
       wiped; the stick shell wipes raw input bytes once decoded.
       Left, needing upstream or an owner decision: transient copies made
       while drawing (`format!`, glyph layout) and inside upstream crates
       are freed unwiped; within a process only a wipe-on-free global
       allocator catches those (an `unsafe` allocator, which belongs in
       `osk-crypto`); `osk-codec`'s `QrMatrix` has no wipe. The kernel's
       `init_on_free` covers all of it at each lock and at power-off.
    7. README: the "Hardened" bullet folded into the kernel, process and
       boot paragraphs; **Auditing Faraday** added (cryptographic core
       upstream and apart from the interface, and why); panic key,
       desktop warning, touchscreen, disk-process wiping.
    Still for the owner: the Pi HDMI / USB keyboard / USB storage
    question (answered in chat 2026-10-08, nothing built).

## Next steps

**START HERE (2026-10-07, late).** The owner reports that upstream has
landed the kernel hardening asked for in `local/upstream-kernel-hardening.md`
(item 63). Work, in order, each step committed by the owner when done:
1. Fetch `upstream` and sync upstream files past `fa1ae03` as "Git
   state" says (`git diff fa1ae03 upstream/main`, `git apply --3way`).
2. Carry upstream's kernel changes into Faraday's copies under
   `faraday/image/overlay/boards/` (`kernel.forbidden`,
   `kernel.required`, `linux.fragment`, `cmdline.txt`) and `rcS`.
3. Do the Faraday-only parts of item 63 that upstream did not take.
4. Build a stick image (`local/remote-build.sh stick`), check its
   `.config` against item 63's lists, run the QEMU checks.
5. Then the README (`local/README-draft.md`) can be finished.
The session that wrote this note got through steps 1, 2, 4 and 5 (item
63's "Progress" lines); step 3's remaining parts (Landlock,
`efi=disable_early_pci_dma`, §16.140's later round), the Pi images and a
boot on real hardware are what is left.

0. Item 60: the screens are done; build the Pi image and try it on the
   Pi 3 and panel when the owner asks.

1. Build when the owner asks (commands under "Where we stopped"), and have
   the owner check scrolling, the PDFs and the scan sheet on the Dell.
2. Then `docs/OPENSIGNER-PARITY.md` §3's remaining items, in order.
3. The design canvas (`design/prototype/`) has not been republished
   since 2026-10-04 and no longer matches the app; the app is the
   reference now.
4. The owner chooses which of `docs/INTEROP.md` §4's remaining gaps to
   build. At the next image build, check the boot logo is centred
   (`qemu-check.py`'s photograph).
5. The desktop QR transfer companion is not built (nor the QR transfer
   tab, `docs/FLOWS.md`); options are with the owner.
6. Hiding a vault (owner's proposal, 2026-10-07: a vault inside one JPEG
   among many) is under discussion; nothing decided.
8. Item 63, kernel hardening: waiting for upstream's changes; the new
   README (`local/README-draft.md`) waits with it.

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
