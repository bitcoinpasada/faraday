# The demo, and Faraday Desktop for developers: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/DEMO.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/DEMO.md)

Spec v0.1 · 2026-10-14 · the owner's decision of 2026-10-14
(`docs/DECISIONS.md` F30), with every question answered (§13).
Batches DM1, DM2 and DM3 built 2026-10-15 (owner: before 0.1.3); DM4,
the website's, waits in its own repository.

## §1 The decision: items 1 and 2 as written (before F61)

1. Faraday Desktop is no longer a released app. Releases stop
   publishing `faraday-desktop-X.Y.Z-linux-x86_64` and
   `faraday-desktop-X.Y.Z-windows-x86_64.exe`, and the macOS file
   `faraday-desktop-X.Y.Z-macos.zip` is never published. Faraday Starter
   drops **Try the desktop app** and the desktop app in **Install**, and
   its payload stops carrying the desktop app.
2. The code and its builds stay, supported for developers and
   unpublished: `faraday/shells/desktop`, `cargo build -p
   faraday-desktop`, `just faraday-linux-bin` (whose default package is
   still `faraday-desktop`) and `just faraday-windows-bin build
   out/windows out/build/run faraday-desktop faraday`. `just` keeps
   building and testing the desktop shell; the Linux and Windows
   container recipes stay working, and the Starter's release builds go
   through the same recipes, so every release exercises them. The README
   says how to build it, in one short section (§9.3).

## §2 What "the demo" is now: the desktop bullet as written

- **A developer's build of the desktop app**, which starts on mainnet as
  the demo (F20) and may move to a test network, where it is not the
  demo.

## §2: the test-networks paragraph as written

**Test networks in a developer build** keep F20's rules: any seed, real
addresses, the warning first. `Session::demo()` is false there, so every
action in §4 is live on a test network, which is how a developer tries
Make flows without a stick. A key made there is a real key made on an
online computer; the test-network warning is the guard, as it is for
typed seeds, and these builds are never released.

## §7.1: its last sentence as written

and the screen reached shows its Make action dimmed. The batch that
builds this adds the exception to FLOWS decision 10's paragraph, in
these words.

## §8 Changes to F20 (from docs/DEMO.md)

- "Faraday Desktop starts on mainnet as the demo" reads: a developer's
  build of the desktop app starts on mainnet as the demo; the website is
  the demo for everyone else. Nothing released runs the demo except the
  website.
- New key is unchanged (§5.1): every source stays, and the next public
  seed replaces the key made. Generate with dice gains the same rule
  with the demo's public passphrase (§5.2).
- Every action in §4 is disabled while the demo is on.
- Test networks in a developer build keep any seed and real addresses,
  after the warning (owner, 2026-10-14). They are never released.
- The rest of F20 stands: the eleven public seeds, broken checksums, the
  passphrase fields ignored, mainnet refused while another key is
  loaded, `fix_network` for the website, the stick and the Pi unchanged.

## §9 Releases (from docs/DEMO.md)

*Partly reversed by F61 (owner, 2026-10-20): releases publish Faraday
Desktop again, for Linux and Windows, as
`faraday-desktop-X.Y.Z-testnet-only-linux-x86_64` and
`faraday-desktop-X.Y.Z-testnet-only-windows-x86_64.exe`, each in
`SHA256SUMS`. A release build opens on testnet and shows F20's online
warning once at start. No Starter carries, installs or offers to try
it (§10 stands). §9.1, §9.2 and §9.4 below read with that change; the
macOS desktop file stays unpublished.*

### 9.1 What a release publishes

`docs/STARTER.md` §1.1 without the macOS `faraday-desktop-` row: the
two Starters (and the macOS one when it exists), the stick image, the
Pi image, the two Faraday Desktop files (F61), the Apple Silicon image
and start-up file when they exist. `SHA256SUMS` lists the two desktop
files.

### 9.2 `faraday/tools/stage-release.sh`

- The first pass takes `out/linux/faraday` and
  `out/windows/faraday.exe`, each a release build unless `--any-build`,
  publishes them under F61's names and writes their lines to
  `SHA256SUMS`, and runs the published-file check on them. The
  one-build check compares the stick and the Pi only.
- The payload's `linux/` and `windows/` folders hold the two image
  `.xz` files and `MANIFEST`, nothing else. The two folders stay, since
  each Starter embeds its own and they may differ later (MAC's images).
- Before it finishes, it refuses a payload folder that holds any file
  whose name starts `faraday-desktop-`, and says which (F61: the release
  folder's two are published).
- The `faraday-release-stage` recipe's comment in `faraday/faraday.just`
  drops the desktop builds.

### 9.3 `README.md`

- **Faraday Starter**: drop "and opens Faraday Desktop for testing".
- **Build** and **Run the desktop shell** become one short section,
  "Faraday Desktop, for developers" (since F61, "Faraday Desktop": it is
  published as the testnet-only files, and a developer still builds it
  as below): it is not released; build it with
  `cargo build -p faraday-desktop --release`, or as one Linux file with
  `just faraday-linux-bin` or for Windows with `just faraday-windows-bin
  build out/windows out/build/run faraday-desktop faraday`; it runs as
  the demo on mainnet and takes any seed on a test network; it has none
  of the stick's protections and is never for a seed that holds funds.
  The sticks folders, the stick pills and the options line stay, shortened.
  "Every session starts on testnet", which contradicts F20, goes.
- **Transfer** moves under Faraday Starter: the Starter's Transfer is
  how a computer becomes the device's QR link.
- **Reproducible builds**: "the desktop app for Linux is built by `just
  faraday-linux-bin`" reads "the Starter for Linux".
- **Verifying a release**: unchanged in substance; its example file stays
  the stick image. The README has no download table; the release notes
  and the website carry the file list.
- One line for whoever installed the desktop app from an earlier
  Starter: it stays until removed by hand, at `~/.local/bin/faraday-desktop`
  and its menu entry on Linux, or `Faraday Desktop.exe` and its Start
  menu shortcut on Windows.

### 9.4 Release notes

The "Other downloads" list (STARTER §1.1) has the two desktop files
since F61, each named testnet only. The first
release after this says once: "Faraday Desktop is no longer published.
Try Faraday on the website's demo; developers build the desktop app
from source (README)."

## §10 Faraday Starter (from docs/DEMO.md)

`docs/STARTER.md` changes, when batch DM2 is built:

| Where | Change |
|---|---|
| Intro | drop "opens the desktop app for testing"; the desktop app's name stays for developer builds |
| §1.1 | the three `faraday-desktop-` rows go; "the macOS Starter carries the macOS desktop app" goes |
| §1.2 | "the Starter and the desktop app for release are built in a pinned container" reads "the Starter is built"; the desktop app keeps the same recipe, unpublished |
| §1.3 | the payload table: stick image and Pi image on both systems; the MANIFEST and staging paragraph drops the desktop app's file, the Windows desktop app's build-ID check and the `faraday-desktop-` filter; size about 36 MB |
| §2 | "The desktop app runs on Windows too" stays as a fact about the developer build and ends at the build recipe; "The Windows Starter carries the Windows desktop app, Try the desktop app and the install check box appear on Windows, and the release has …" goes. "Transfer and Learn are the desktop app's own screens" stays: they are `faraday-core`'s |
| §3.1 | the **Try the desktop app** row goes |
| §3.5 | **Install** has no check box: Install is one press, then Done |
| §5.1 | "the desktop app's cache under §5.3" goes |
| §5.2 | the desktop app's rows and the macOS `Faraday Desktop.app` go. The Starter no longer installs, lists or removes `faraday-desktop`, its menu entry or `Faraday Desktop.exe`: a developer's own build there is not the Starter's |
| §5.3 | goes |
| §5.4 | the cache goes from what Uninstall removes |
| §11, §12 | batch C's and D20's desktop parts, and MAC batch D, are marked superseded by F30 |

Strings removed from `faraday/faraday-strings/src/en.rs`:
`starter_row_desktop`, `starter_row_desktop_sub`,
`starter_install_desktop`, `starter_installed_desktop`,
`starter_installed_desktop_menu`, `starter_why_no_desktop`,
`starter_why_cache_sum`, `starter_menu_desktop_name`,
`starter_menu_desktop_comment`. In `faraday-core/src/names.rs`,
`LINUX_DESKTOP_BIN`, `MENU_DESKTOP_ID` and `WIN_DESKTOP_EXE` and its
shortcut name go once nothing uses them.

No Starter guide mentions the desktop app (`docs/learn/faraday/boot/`
checked 2026-10-14). The Starter's About lists only the images.

## §11 Decisions that offer things "in Faraday Desktop" (from docs/DEMO.md)

| Decision | What it said | Now |
|---|---|---|
| F11, F12 | the desktop app opened from the Starter, installed with it, on Windows too | not carried by the Starter; published on its own as testnet-only files since F61 (§9, §10) |
| F13, MAC §5 | the desktop app on macOS, `faraday-desktop-X.Y.Z-macos.zip` | not published; MAC batch D is dropped from the plan (decided, §13); a developer with a Mac can still build it |
| F16 | "Faraday Desktop" with DESKTOP beside the wordmark | the name and wordmark stay on developer builds |
| F18 | sketches only in the Starter | unchanged |
| F25, APK-SIGNING §4 | "Faraday Desktop offers it too, as it offers GPG and Secure Boot" | developer builds and the demo only; in the demo **Make an APK key** is disabled and, with no key, Sign an APK has nothing to sign with |
| GPG, Secure Boot | offered on the desktop app as on the stick | the same: developer builds and the demo; Make disabled in the demo |
| F26 | Faraday Desktop's memory rules | developer builds only |
| F27, NOSTR §7 | no Nostr on Faraday Desktop, on any network, nor in the demo | unchanged |
| F28 | QR codes the same on Faraday Desktop; broken checksums on its mainnet demo | unchanged; the demo's broken checksums stand |

The backup step that follows making a key (F25 and its follow-up)
never opens in the demo, since every Make is disabled there; in a
developer build on a test network it runs as on the stick, with the
build's stick folders.

## §12 Batches (from docs/DEMO.md)

Each batch's tests describe what a person or a wallet would notice.

### DM1 Faraday's demo gate (`faraday-core`)

`Faraday::makes_secrets()`; every action of §4 drawn with
`Style::Disabled` and refused in its handler; Generate with dice's
public passphrase (§5.2) and its line; the backup plan's dimmed forms (§5.3); the aezeed row
(§6); the demo Learn page first on every §4 screen and its text
(`docs/learn/faraday/demo.md`, round-tripped into `faraday-learn`); FLOWS
decision 10's exception (§7.1). APK-SIGNING and NOSTR take the gate in
their own batches.

Tests, in a new `faraday/faraday-core/tests/all/demo.rs`:

- In the demo, the GPG key screen opens and Make a GPG key is
  disabled: pressing it adds no GPG key to the open vault.
- The same for Make Secure Boot keys, Split with
  Seed XOR, codex32 shares, SLIP-39 shares, the FROST deal, the vanity
  passphrase dial, and Save into the vault and Out unprotected for a
  Lightning node key and a scan key: each screen opens, the action is
  disabled, and nothing is added to the vault or For the stick.
- In the demo, BIP-85 opens and Derive with BIP-85 is disabled for every
  application: no value is shown.
- In the demo, New key from dice rolls shows the rolled words, and Lock
  in adds bacon in place of them; the same from coins, cards, hex, the
  camera, the generator and a mix, at 12 and at 24 words; the rolled key
  is not among the keys loaded. With all eleven loaded, Lock in is
  disabled.
- In the demo, SLIP-39 shares opens and its deal is disabled.
- In the demo, a vault made with Generate with dice opens with the
  demo's public passphrase and not with the rolled words; a vault made
  with a typed passphrase opens with that passphrase.
- In the demo, a backup plan cannot choose Seed XOR, codex32 or SLIP-39
  shares, and its checklist can be finished.
- In the demo, a wallet from public seeds signs a transaction and a
  message, a vault is made with a typed passphrase and unlocked, an
  entry is added and its TOTP code shown, and an encrypted backup of a
  public seed is made.
- In the demo, the info button on the GPG key screen opens the demo's
  Learn page first.
- In a session with the demo set up and moved to testnet, Make a GPG key
  makes a key.
- On the stick (no demo), every action of §4 is live.

### DM2 Faraday Starter (`faraday-starter`, `shells/starter`, `faraday-payload`)

§10's changes and string removals.

Tests:

- `faraday-starter/tests/all/starter.rs`: Home's rows offer no Try the
  desktop app; Install offers no desktop app and installs on one press.
- `shells/starter/tests/all/install.rs`: Install puts no `faraday-desktop`
  and no Faraday Desktop menu entry; a `faraday-desktop` already in
  `~/.local/bin` is the same after Install, after Update and after
  Uninstall.
- `faraday-payload`: a payload's manifest lists only the stick and Pi
  images, and About shows those two.

### DM3 Release staging and the README

§9's changes.

Tests:

- A staging run over test builds (`--any-build --out DIR`) writes a
  `SHA256SUMS` that lists no `faraday-desktop-` file, and payload folders
  that hold only the images and `MANIFEST`.
- Staging refuses a release folder holding a `faraday-desktop-` file and
  names it.
- The desktop app still builds through `just faraday-linux-bin` and `just
  faraday-windows-bin build out/windows out/build/run faraday-desktop
  faraday` (the build is the test; it is not published).

### DM4 The website (its own repository)

Not edited from here. What it needs, once the pin moves past DM1:

- The dice stories keep working: New key's rolls and the vault
  passphrase's rolls are open (§5.1, §5.2).
- **Create a vault**: the rolled words are replaced by the demo's public
  passphrase, now Faraday's `demo` constant (§5.2); the web shell takes
  it from there instead of its own `desk::PASSPHRASE`. "Keep the rolled
  words or use the demo's" becomes the one outcome: the demo's.
- The web shell's tests that derive a BIP-85 value change to the
  demo's behaviour; a test that Make a GPG key is disabled in the web
  shell.
- A line beside the app: the demo opens every screen, makes no keys it
  keeps, and uses only the public seeds; a Faraday stick makes keys.
- **Download page**: no Faraday Desktop and no `faraday-desktop-` links;
  the Starter and the images.
- The site's About cards and "What you did" lines that name Faraday
  Desktop are reworded.
- The site's HANDOFF "What Faraday provides" section follows §8.

Order: DM1 first; DM2 and DM3 together, for the next release; DM4 after
DM1, with the pin moved to a commit that has it.

## §13 Decided (from docs/DEMO.md)

All by the owner on 2026-10-14, recorded in F30.

1. New key's sources (dice, coins, cards, hex, camera, the generator, a
   mix, 12 or 24 words) stay open in the demo; the key made is replaced
   by the next public seed, as F20 has it (§5.1). The website's dice
   stories keep working.
2. **Generate with dice** for a vault passphrase was to stay disabled
   unless a public value could replace its result. One can: the demo's
   public passphrase, moved into `faraday-core/src/demo.rs` (§5.2). It
   is open on that rule.
3. Typing a password or TOTP secret into a vault Entry stays open (§6).
4. The LND cipher seed row of Lightning node key is dimmed in the demo
   (§6).
5. A desktop app installed by an earlier Starter is left in place, and
   the README says how to remove it (§9.3, §10).
6. The desktop app on macOS, MAC batch D, is dropped from the plan;
   developers can still build it (§11, `docs/MAC.md` §5 and §8).

## §5 Open in the demo: the upstream app's items (from docs/DEMO.md)

Removed 2026-10-22: Faraday has none of these (they went with the
upstream app, F68). The lines read:

- **Keys**: typing, scanning or reading a public seed in any form (words,
  SeedQR, CompactSeedQR, word numbers, hex entropy, SLIP-39 shares,
  Seed XOR parts, codex32, an encrypted backup), …
- **Spending**: Sign a transaction, the Spend tab, the nonce check,
  Decode, Compare transactions, MuSig2 and FROST signing rounds.
- **Backups of a public seed**: Words, SeedQR, CompactSeedQR, Draw a
  SeedQR, Numbers for steel, a single codex32 string where one is
  offered, Encrypted backup,
  into a vault, …
