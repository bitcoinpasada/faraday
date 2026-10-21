# The demo, and Faraday Desktop for developers

**Status:** spec v0.1 · 2026-10-14 · the owner's decision of 2026-10-14
(`docs/DECISIONS.md` F30), with every question answered (§13).
Batches DM1, DM2 and DM3 built 2026-10-15 (owner: before 0.1.3); DM4,
the website's, waits in its own repository.

## 1. The decision

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
3. The website demo, the desktop shell compiled to WebAssembly in the
   website's own repository, is the way to try Faraday.
4. The demo lets people open every tab and screen, and every action
   that would make a secret is disabled. Bitcoin use with the eleven
   public seeds stays fully open, as F20 has it. The demo is for trying
   Faraday out, not for testing it exhaustively.

## 2. What "the demo" is now

The demo is a session whose shell called `Session::set_demo` and which
is on mainnet: `Session::demo()` is true. Two builds have one:

- **The website**, which also calls `Session::fix_network`, so it is on
  mainnet for the whole session and always the demo.
- **A developer's build of the desktop app**, which starts on mainnet as
  the demo (F20) and may move to a test network, where it is not the
  demo.

The stick, the Pi and the `Faraday` the Starter holds for
Transfer and Learn never call `set_demo`; nothing in this spec changes
them.

**The gate.** One function in `faraday-core`, `Faraday::makes_secrets()`,
is false while `Session::demo()` is true and true otherwise. Every
action in §4 asks it, in two places: the screen draws the action with
`Style::Disabled`, and the action's handler does nothing when it is
false, so a key press, a click, a shell message or a test's action all
meet the same refusal. No action in §4 has a path around it.

**Test networks in a developer build** keep F20's rules: any seed, real
addresses, the warning first. `Session::demo()` is false there, so every
action in §4 is live on a test network, which is how a developer tries
Make flows without a stick. A key made there is a real key made on an
online computer; the test-network warning is the guard, as it is for
typed seeds, and these builds are never released. F27's rule is
separate and stands: no Nostr key while `Faraday::online` is set, on
any network.

## 3. What makes a secret

An action is disabled in the demo when its result is a secret a person
could take away and use: a new seed, key, share, passphrase or password
drawn from randomness (the system's generator, dice, coins, cards, hex
digits or camera noise), and a private value derived for use outside
Faraday (BIP-85's outputs, a Lightning node's private key, a silent
payments scan key). A public seed's own words in any spelling are not a
new secret: they are public.

Two actions are open with their result replaced (owner, 2026-10-14):
New key, whose sources all run and whose key is replaced by the next
public seed (§5.1, F20), and **Generate with dice** for a vault
passphrase, whose rolls run and whose words are replaced by the demo's
public passphrase (§5.2). What they make is shown and then dropped,
never kept.

An action stays open when it shows public values, loads a public seed,
signs or checks with one, or encrypts what is already held. Encryption
draws salts and nonces and signing draws nonces; neither is a secret a
person keeps.

## 4. Disabled in the demo

| Action | Screen | What it would make |
|---|---|---|
| SLIP-39 shares: its sources and the deal | Tools → SLIP-39 shares (New key in SLIP-39 mode) | a new key dealt as shares; no public seed stands in |
| SLIP-39 shares of a key held | a key's Backup menu | fresh random shares of the key |
| Split with Seed XOR (**Make them**) | a key's Backup menu; the backup checklist's Seed XOR item; the chart's Make another backup | fresh random parts |
| Codex32 shares, k of n with k of 2 or more | a key's Backup menu; the backup checklist's codex32 item; Make another backup | fresh random shares |
| **Make a GPG key** | Tools → GPG key; Vault contents' GPG kind | an OpenPGP key |
| **Make Secure Boot keys** | Tools → Secure Boot keys; Vault contents' Secure Boot kind | PK, KEK and db keys |
| **Make an APK key** (F25, when built) | Tools → APK signing key | an APK signing key |
| **Derive with BIP-85**, every application (Words, WIF, Extended private key, Hex, Password base64, Password base85) | BIP-85 (a key's page → BIP-85; Tools → Child seeds and passwords) | a child seed, key or password |
| **Open BIP-85 child** | Add a key, opened from a wallet's or a transaction's not-loaded row | a child seed loaded as a key |
| The passphrase dial | Vanity address, the How? card | a new key under a found passphrase; the demo would refuse it anyway |
| The FROST deal (the last step of Add a wallet → FROST) | Create, FROST | new share seeds, which are not public seeds |
| **Show** for the node's private key | Lightning node key | a node's private key, on screen |
| **Save into {vault}** and **Out unprotected…** for the node's private key | Lightning node key | a node's private key |
| **Save into {vault}** and **Out unprotected…** for the scan key | Silent payments | a scan key |
| The Nostr key | none | already absent under `Faraday::online` and in the demo (F27) |

Each screen above opens in the demo. Its other cards, choices and
public values work; only the actions in the table are disabled.

## 5. Open in the demo

- **Keys**: typing, scanning or reading a public seed in any form (words,
  SeedQR, CompactSeedQR, word numbers, hex entropy, SLIP-39 shares,
  Seed XOR parts, codex32, an encrypted backup), refused unless it is a
  public seed (F20); a passphrase field, ignored (F20); Explore; Account
  key; the key's page.
- **New key**: every length and source (dice, coins, cards, hex,
  camera, the generator, a mix), its words and quiz; Lock in adds the
  next public seed in place of the key made (§5.1).
- **Wallets**: every kind built from public seeds (single-key, multisig,
  Taproot multisig, MuSig2, miniscript and recovery, silent payments'
  address and labels), Restore, Check an
  address, addresses with a broken checksum (F20), the wallet card and
  its chart.
- **Spending**: Sign a transaction, the Spend tab, the nonce check,
  Decode, Compare transactions, MuSig2 and FROST signing rounds.
- **Messages**: Sign a message, Check a signed message.
- **Backups of a public seed**: Words, SeedQR, CompactSeedQR, Draw a
  SeedQR, Numbers for steel, a single codex32 string where one is
  offered, Encrypted backup,
  into a vault, the secret sheet and **Out unprotected…** for the seed
  itself, the backup plan and checklist (§5.3), Check a copy.
- **Vaults**: Create a vault with a typed passphrase, or with **Generate
  with dice**, whose words the demo's public passphrase replaces (§5.2),
  Unlock, Vault
  contents, entries, notes, TOTP codes, KeePass export, Open an
  OpenSigner backup, the stick visit and Files.
- **Vanity address** on the account dial; **Lightning node key**'s node
  id from a loaded public seed; **Silent payments**' address, labels,
  record and Check a payment.
- **Tools**: every calculator, word list and Find a tool; every tile
  opens its screen.
- **Settings**, About, Learn, themes.

### 5.1 New key in the demo

Unchanged from F20 (owner, 2026-10-14). Every length and every source
is open: the dice, coins, cards, hex, camera noise, the generator and a
mix make words as on the device, and they are shown. At Lock in the demo
adds the next public seed not loaded in place of the key made, with no
passphrase, and the Key card says which ("Lock in adds public seed {}
({}) in place of these words"). The key made is never added or kept.
SLIP-39 shares are not New key: they stay disabled (§4), since no public
seed stands in for a dealt set of shares. With all eleven public seeds
loaded, Lock in is disabled.

### 5.2 Generate with dice in the demo

**Generate with dice** on Create a vault is open: the person rolls and
types the numbers, and the words appear as on the device. When the last
word is in, the passphrase field takes the demo's public passphrase in
place of the rolled words, and the card says so in one line, "The demo
uses its public passphrase in place of these words", as New key's Key
card does. The public passphrase is a constant in
`faraday-core/src/demo.rs` beside the seeds: the website's demo
passphrase, already published on its note, moves there and the web
shell uses Faraday's. A passphrase typed by hand is kept as typed.

### 5.3 The backup plan in the demo

The plan's form choices that need a split (Seed XOR parts, codex32
shares, SLIP-39 shares) are dimmed in its multi-choice lists, so a plan
made in the demo is one whose checklist can be finished there. A plan
read from a vault made elsewhere that names such a form keeps its item,
with its Make action disabled; the item can be marked done by hand as
any copy made away from the device is.

## 6. Borderline calls (decided, §13)

| Case | Decision | Why |
|---|---|---|
| Typing a password or TOTP secret into a vault Entry | Open | Typed, not generated, and the only way to try entries, TOTP codes and KeePass export. The demo cannot tell a real password from a made-up one, as it cannot tell a funded seed on a test network; the demo's Learn page says to type nothing real |
| Typing an LND cipher seed (aezeed) on Lightning node key | Disabled: the **An LND cipher seed** row of From? is dimmed | No public aezeed exists, so whatever is typed is someone's real node seed, and the demo cannot refuse it as it refuses a seed. A loaded public seed is the other source and stays |
| Making a vault | Open, with a typed passphrase; **Generate with dice** open, its words replaced by the demo's public passphrase (§5.2) | It encrypts what the demo holds: public seeds and what was typed. Its keys are internal to the file and never shown. The rolled words are dropped, as New key's are, so the passphrase the vault takes is public |
| BIP-85 outputs | Disabled, as the owner said | Derived from a public seed, so anyone can derive them, but they look like a fresh password or seed and would be used as one |
| Lightning node private key, silent payments scan key | Disabled | Derived private keys for use in another program, as BIP-85's are |
| The secret sheet and **Out unprotected…** for a public seed | Open | Every value they can carry in the demo is a public seed's own spelling; the sources of every other secret are disabled at §4 |
| Seed XOR, codex32 and SLIP-39 splits of a public seed | Disabled | The parts are fresh randomness; a person copying them would hold something that looks like a real backup |
| A single codex32 string of a public seed (k of 1), where offered | Open | Another spelling of a public seed, with no randomness |
| Encrypted backup, KeePass export, sealed files | Open | Encryption of what is held, under a typed passphrase or password |
| Vanity address's passphrase dial | Disabled | It makes a new key under a passphrase; F20 would refuse that key on mainnet anyway |
| FROST deal | Disabled | The dealt shares are new seeds that are not public seeds |

## 7. How a disabled action looks

- DESIGN §4.11's dimmed state, drawn by `faraday-ui`'s `Style::Disabled`,
  the same as every dimmed control: its label, dimmed, in its own place
  and size.
- **No reason line.** DESIGN §4.11 gives a dimmed reason only for a
  condition the person could change on this device, and leaves a row dead
  for a condition they cannot change with its label alone. In the demo
  nothing on the screen changes it, so no "Not in the demo" appears.
  Working screens carry no explanation (CLAUDE.md).
- It holds focus and does nothing on Enter or a tap (DESIGN §4.13's
  focus row). It opens no Pull sheet and no vault lead.
- **Where the reason is.** The demo's Learn page (`docs/learn/faraday/demo.md`)
  comes first in the info button's list on every screen of §4, as it
  does today on Home, Entry, New key and Settings. Its "New key in the
  demo" section stays and gains Generate with dice (§5.2); a new section,
  "What the demo does not make", gives the list of §4 in one paragraph,
  and that a Faraday stick makes them. The website says
  the same in its own words beside the app.

### 7.1 The exception to FLOWS decisions 8 and 10

`docs/FLOWS.md` decision 8 shows nothing that loads a key disabled for a
stick being in, and decision 10 shows no disabled button where a vault
is needed. Both stand. The demo adds one exception: while
`Session::demo()` is true, an action listed in §4 is drawn dimmed, with
no reason, and does nothing. It is the only action in Faraday dimmed for
a condition the person cannot change on the device. A Tools tile that
leads to one of these screens is not dimmed: "Make a vault first" and
"Unlock a vault first" still lead into the vault and back (decision 10),
and the screen reached shows its Make action dimmed. The batch that
builds this adds the exception to FLOWS decision 10's paragraph, in
these words.

## 8. Changes to F20

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

## 9. Releases

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

## 10. Faraday Starter

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

## 11. Decisions that offer things "in Faraday Desktop"

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

## 12. Batches

Each batch's tests describe what a person or a wallet would notice.

### DM1 Faraday's demo gate (`faraday-core`)

`Faraday::makes_secrets()`; every action of §4 drawn with
`Style::Disabled` and refused in its handler; Generate with dice's
public passphrase (§5.2) and its line; the backup plan's dimmed forms (§5.3); the aezeed row
(§6); the demo Learn page first on every §4 screen and its text
(`docs/learn/faraday/demo.md`, round-tripped into `faraday-learn`); FLOWS
decision 10's exception (§7.1). APK-SIGNING and NOSTR take the gate in
their own batches.

Tests, in a new `faraday/faraday-core/tests/demo.rs`:

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

- `faraday-starter/tests/starter.rs`: Home's rows offer no Try the
  desktop app; Install offers no desktop app and installs on one press.
- `shells/starter/tests/install.rs`: Install puts no `faraday-desktop`
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

## 13. Decided

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
