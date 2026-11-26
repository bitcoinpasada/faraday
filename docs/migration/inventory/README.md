# Inventory of `main` at `d89dde5de`

This inventory records facts about the app on `main` at commit `d89dde5de`. It
names 82 features in 10 groups. For each feature it gives the screens and
sheets, what a person can do, what each action does, the state it holds and
what a lock clears, what it reaches in other areas, its rules and refusals,
the domain calls it makes, the fixtures that test it, and the bug fixes in
the history of `main` that touched it.
It holds no design. It is the input to the specification of `next`, step 2
of "How the work is done" in `docs/plan/ARCHITECTURE-PLAN.md`: the
specification and its skeleton are written from it. [`fixes.md`](fixes.md)
lists the bug fixes: 817 rows under features, and a table of the rows left
out. [`fixtures.md`](fixtures.md) lists the 193
fixture and test-data files that carry over, and the features that use them.

## How to read a feature file

Each file in `features/` starts with a title and a short paragraph on what the
feature is and who uses it. Then come ten sections, always in this order:

- **Screens and sheets**: what is drawn, and where.
- **Actions**: what a person can press, type or bring in.
- **Effects**: what each action changes, in the app and on a stick.
- **State and lock**: what the feature holds, and what a lock or a restart
  clears.
- **Reach**: what the feature calls or reads in other areas, and what calls it.
- **Rules and refusals**: what the code requires, and what it refuses.
- **Domain calls**: the calls into the cryptography, format and key crates.
- **Fixtures**: the test data that covers the feature.
- **Fixes**: the bug fixes of the history, each with what was broken and the
  rule restored.
- **Doubts**: what was not read, not confirmed, or found unclear.

Line numbers are those of `d89dde5de`. A `~` before a line number means it
was not confirmed against the code. Such numbers were off by up to 40 lines
where they were checked, so a `~` number points near the code, not at it.
A number with no `~` was read from the code. Paths are relative to the
repository at that commit.

## The index

### Lock, vaults and secrets

| feature | what it is |
|---|---|
| [The lock, idle lock and held secrets](features/lock.md) | What a Lock, an idle lock, a restart and leaving a screen clear, and keys held until their backup is done. |
| [Secrets, text checks and the passphrase field](features/secrets.md) | Where a secret may go, refusing text or files that hold one, and the passphrase field with its eye. |
| [The Vaults list and making a vault](features/vault-list-create.md) | The list of vaults, the vault area's state and the Create a vault form with its refusals. |
| [Unlocking a vault](features/vault-unlock.md) | Opening a vault with its passphrase or key file, its checks, and the memory a vault kind needs. |
| [Inside a vault](features/vault-contents.md) | The open vault's page, item panes and forms, search, the Load sheet, copying and moving items, and KeePass export. |
| [Sealing a vault on lock](features/vault-seal.md) | How an open vault is sealed and written back on a lock, and the vault area's own rules. |
| [One-time codes](features/totp-codes.md) | The TOTP codes screen and masked secrets kept in a vault. |
| [GPG keys](features/gpg-keys.md) | GPG keys kept in a vault and restored from a paperkey. |

### Making and bringing in keys

| feature | what it is |
|---|---|
| [Making a new key](features/keygen-flow.md) | The New key flow: its steps, the Key, Check and Words cards, the quiz, SLIP-39 shares and making on a Create card. |
| [Randomness and dice](features/randomness.md) | Session randomness, Add randomness, dice entry and the rollers. |
| [Codex32 shares](features/codex-shares.md) | Making codex32 shares on the worksheet and correcting a typed codex32 share. |
| [Adding a key](features/add-key.md) | Typing seed words on the on-screen word grid, key forms, the ways in to a key and the wordlist. |
| [Krux encrypted codes](features/krux-codes.md) | Reading and making Krux (KEF) encrypted codes and their background jobs. |
| [Child keys (BIP-85)](features/child-keys.md) | Deriving BIP-85 child keys and showing them. |
| [Convert a seed](features/convert.md) | Converting a seed between forms, such as splitting it into shares. |
| [Vanity address search](features/vanity.md) | Searching for an address by account number or passphrase suffix. |

### Wallets, seeds and addresses

| feature | what it is |
|---|---|
| [Seeds to a wallet](features/seeds.md) | Typed seeds turned into a wallet, the seed passphrase sheet and sweeping a private key. |
| [Checking a seed or a copy](features/seed-checks.md) | My seed makes my wallet, Check a seed, and Check my copy. |
| [Wallets in the session](features/wallets.md) | The wallets and keys held in memory, the Wallets tab, wallet cards and chart, names, identity and multisig checks. |
| [Making a new wallet](features/create-wallet.md) | The Make a new wallet flow and its screen. |
| [Restoring a wallet](features/restore-wallet.md) | Restoring a wallet from seeds, shares or a configuration. |
| [Public wallet files](features/public-files.md) | The public files a wallet or key carries and which stick holds them. |
| [Reading a Bitcoin Core wallet](features/core-wallet.md) | Bringing in a wallet from a Bitcoin Core wallet file. |
| [Addresses: check, find and explore](features/address-check-explore.md) | Check an address, Find an address, Explore and the addresses list. |
| [Silent payments](features/silent-payments.md) | Receiving with silent payments (BIP-352). |

### Signing, transactions and tools

| feature | what it is |
|---|---|
| [Signing a transaction](features/spend-signing.md) | Opening a PSBT as a spend, reviewing it, collecting and finishing signatures, MuSig2 and registering a wallet. |
| [Decoding and checking transactions](features/decode.md) | Decode a transaction, the nonce check, Check a signer and comparing transactions. |
| [Signing and checking messages](features/messages.md) | Sign a message and Check a signed message. |
| [Spending from a backup](features/spend-backup.md) | The Spend tab: spending from a backed-up wallet. |
| [The Tools page and calculators](features/tools.md) | The Tools catalog and its calculators, Lightning keys and invoices. |
| [The Check page](features/check-page.md) | The Check tile grid and the checks it opens. |

### Backups and sticks coming and going

| feature | what it is |
|---|---|
| [Backing up a key](features/backup-flow.md) | The guided backup of a key to sticks: the plan, bringing in sticks, making the key, Lock and write. |
| [Backing up a wallet](features/wallet-backup.md) | The Back up checklist of a wallet and its sheets. |
| [The Backups list](features/backups-list.md) | Every wallet's backups, their places, checks and status. |
| [Codex32 backup item](features/codex-backup.md) | The codex32 item of the backup checklist and its stick files. |
| [The backup board](features/backup-board.md) | The board of a wallet's backup pieces and their lines. |
| [Sticks coming and going](features/stick-visit.md) | A stick arriving and leaving, writing to it and copying in, names taken and secrets in the way. |
| [Deleting files and zeroing a stick](features/delete-and-zero.md) | Delete files and Zeroing on a stick. |

### Screens, navigation, drawing, test kit and tooling

| feature | what it is |
|---|---|
| [The app and its event loop](features/app-shell.md) | The app's event loop, every press and power. |
| [Scrolling, motion and fades](features/motion.md) | Scrolling, moved frames, toasts, cross-fades and glides. |
| [Step flows](features/flow-container.md) | The flow card, Back and Forward through steps, and Resume or Cancel of flows begun. |
| [The screen frame and navigation](features/screen-frame.md) | The screen frame, notices, the sidebar and what every screen shares. |
| [Home, menu pages and search](features/menu-pages.md) | Home and the other menu pages and the bar search. |
| [The pane container](features/pane-container.md) | How a page's panes are placed and a page turn is timed. |
| [The session pane](features/session-pane.md) | The pane that shows the keys, wallets and what waits in the session. |
| [Sheets and shared parts](features/sheets-parts.md) | Sheets drawn over screens and the shared screen parts. |
| [The guide](features/guide.md) | The Guide button, its routing and its side pane. |
| [Settings and the clock](features/settings-time.md) | Settings, the settings file on the boot stick, setting the time and the version. |
| [Drawing and costly work](features/drawing-compute.md) | What drawing reads, costly-work counters, budgets and background jobs. |
| [The small panel](features/small-panel.md) | The compact layout and screens for a small panel. |
| [Typing and the caret](features/typing.md) | The typing caret, field selection and showing invisible characters. |
| [Test kit](features/test-kit.md) | The test kit and leak tests, not shipped. |
| [Build, CI and tooling](features/tooling.md) | Builds, images, CI, fuzzing, cross-checks, releases and dependencies. |

### The stick shell, devices, storage and files

| feature | what it is |
|---|---|
| [Files, Inbox and the file menu](features/files-inbox.md) | The Files page, the Inbox of what came from a stick, file records and the file menu. |
| [Importing from the boot stick](features/boot-import.md) | Bringing in files found on the boot stick at start. |
| [Storage and the disk process](features/storage.md) | The storage protocol between app and shell, the disk process and saving files. |
| [The stick shell](features/stick-shell.md) | The device shell: boot, lock, restart and power-off, the command pump, disk work and drawing. |
| [Devices and input trust](features/devices-inputs.md) | The device check, input devices held back until trusted, the physical keyboard and the panic chord. |
| [Pointer, touch and focus](features/pointer-touch.md) | The pointer, cursor, touchscreen, focus and tooltips. |
| [Scanning and QR codes](features/scanner-qr.md) | The camera, the scanner sheet, reading QR codes and showing them. |
| [Developer card and diagnostics](features/dev-diagnostics.md) | The dev card's timings and boot report, diagnostics and the self-test. |

### The Starter, Faraday Demo, desktop shells and Secure Boot

| feature | what it is |
|---|---|
| [Secure Boot keys](features/secure-boot.md) | Making and keeping Secure Boot keys and signing boot images on the device. |
| [Sending files and text as codes](features/transfer.md) | The online app's Send file as codes, Receive and Send text. |
| [The desktop window shells](features/desktop-shells.md) | The window shell and host on a desktop: window, input, paste, file drop and folders. |
| [Practice and Faraday Demo](features/practice-demo.md) | Practice mode's refusals, modes, Faraday Demo's window and its samples. |
| [Making a Faraday stick](features/starter-make-stick.md) | The Starter's Make a Faraday stick flow, image finding, writing and checking a stick. |
| [Starting from the stick](features/starter-start.md) | The Starter's Start page, Secure Boot, BitLocker, restart and things attached. |
| [The Starter app](features/starter-app.md) | The Starter's frame, Home, install, appearance, guide, templates, Demo and transfer. |

### Tours

| feature | what it is |
|---|---|
| [Tours](features/tours.md) | The scripted tours through screens. |
| [The tour runner](features/tour-runner.md) | The tour driver, its command line, the secret-leak check and touch-target report. |

### Components (faraday-ui and faraday-draw)

| feature | what it is |
|---|---|
| [Presses and hits](features/ui-press.md) | The UI frame, hit and press registration and state layers. |
| [Buttons and choice controls](features/ui-controls.md) | Buttons, FAB, chips, switches, hold-to-confirm, choice lists and appearance rows. |
| [Text and text fields](features/ui-text.md) | Text drawing, selection, caret, type roles, fields and list items. |
| [Menus, sheets, dialogs and snackbar](features/ui-sheets.md) | Menus, parts, sheets, dialogs, snackbar, tooltip and the stick pill. |
| [Navigation rail, bar and tabs](features/ui-navigation.md) | Tabs, the navigation rail and bar and the top app bar. |
| [Scrolled regions and shadows](features/ui-regions.md) | Scrolled regions, scroll bars, fades and shadow. |
| [Theme, palette and brand](features/ui-theme.md) | Theme, mode, contrast, text size, palette and brand shapes. |
| [Guide pages and flow text](features/ui-guide-text.md) | Guide blocks, flow paragraphs and demo items. |
| [The on-screen keyboard](features/keyboard.md) | The on-screen keyboard panel, its layouts and key geometry. |
| [The drawing base](features/draw-base.md) | Canvas, surface, blending, call list, geometry, text, shapes, fonts and icons. |
| [Design tokens](features/tokens.md) | The token file's values and helpers and which tokens each part uses. |
| [Layout rules](features/layout-rules.md) | The layout rules the components enforce. |

## How it was made

The code of `main` at `d89dde5de` was read area by area, in slices small
enough to read in full. The fix commits of its history were read, and each
was given what it broke and the rule it restored. Each area's accounts were
then merged, and their claims spot-checked against the code. The result was
grouped into features, and each fix was placed under the feature it is about.

## What it does not cover

- **Test code.** The tests of `main` were not read. Only their fixtures and
  test data are listed, in [`fixtures.md`](fixtures.md). Several features say
  so and name no fixture: [Backing up a key](features/backup-flow.md),
  [Backing up a wallet](features/wallet-backup.md),
  [The Backups list](features/backups-list.md),
  [Codex32 backup item](features/codex-backup.md),
  [The backup board](features/backup-board.md),
  [Sticks coming and going](features/stick-visit.md) and
  [Deleting files and zeroing a stick](features/delete-and-zero.md).
- **Side channels.** Timing that depends on a secret, and copies of a secret
  left in freed memory, on the stack or in a library's buffer, are outside the
  security scope (`docs/SECURITY-MODEL.md`). They are not recorded, and fix
  rows about them are left out.
- **Fixtures no feature names.** 75 of the 193 fixtures are used by no
  feature: the rows marked `-` in [`fixtures.md`](fixtures.md).
- **Fix rows left out.** Build-only fixes, rows outside the security scope and
  rows a feature file names as belonging elsewhere are in the last table of
  [`fixes.md`](fixes.md). Each feature's Doubts section says which rows it
  moved or left out.
- **Callers of the components.** The component features were read in
  `faraday-ui` and `faraday-draw` only. Their callers in the app were not
  traced, so their Reach sections are partial: every component feature, from
  [Presses and hits](features/ui-press.md) to
  [Layout rules](features/layout-rules.md).
- **Build, CI and tooling.** No code was read for
  [Build, CI and tooling](features/tooling.md). It rests on the fix rows and
  on the files existing, and has no checked line numbers.
- **Files not read.** Some files were read by no one, or only at the points
  cited: `faraday/faraday-core/src/stick/wipe.rs`
  ([Files, Inbox and the file menu](features/files-inbox.md)),
  `faraday/faraday-core/src/screens/visit.rs` and the Wallets start page
  ([The screen frame and navigation](features/screen-frame.md),
  [Home, menu pages and search](features/menu-pages.md)),
  `faraday/faraday-core/src/fade/bands.rs`
  ([Scrolling, motion and fades](features/motion.md)), the GPG restore flow
  ([GPG keys](features/gpg-keys.md)), and the tools the Tools page opens,
  such as the Lightning node key and the invoice decoder
  ([The Tools page and calculators](features/tools.md)).
- **Test data named in code but not kept.** Files under `tools/vectors/` that
  the code, the test kit or the tours read are not on the kept fixture list:
  [Randomness and dice](features/randomness.md),
  [Test kit](features/test-kit.md), [Tours](features/tours.md).
- **Fix rows read from the message alone.** Some fix rows rest on their commit
  message, and some name crate paths that no longer exist (`core/osk-bip`,
  `faraday/faraday-hub`, `faraday/shells/hub`). The Doubts sections name them:
  for example [Signing a transaction](features/spend-signing.md),
  [Wallets in the session](features/wallets.md),
  [Starting from the stick](features/starter-start.md).

## Doubts that cut across features

- **What a lock clears, and what only the restart after it clears.** Several
  areas keep state through their own `lock()` and lose it only when the
  process ends: `older` and `save_hold` of a vault, Decode's PSBT state,
  Convert's typed keys, `kef_timing`, the Tools page's `ots`, the chart's
  sheet state. See [The lock](features/lock.md),
  [Inside a vault](features/vault-contents.md),
  [Sealing a vault on lock](features/vault-seal.md),
  [Decoding and checking transactions](features/decode.md),
  [Convert a seed](features/convert.md),
  [Krux encrypted codes](features/krux-codes.md),
  [The Tools page and calculators](features/tools.md),
  [Wallets in the session](features/wallets.md),
  [Scrolling, motion and fades](features/motion.md).
- **A failed save at lock does not restart the process.** When the save at
  lock fails and keys wait for their backup, the stick shell drops the old app
  and builds a new one in the same process. What the restart would clear then
  stays: screen copies in the shell, values no `lock()` clears, maybe a
  waiting write. See [The lock](features/lock.md),
  [The stick shell](features/stick-shell.md),
  [Files, Inbox and the file menu](features/files-inbox.md),
  [Pointer, touch and focus](features/pointer-touch.md),
  [Backing up a key](features/backup-flow.md).
- **A typed codex32 string does not count as held.** The backup area's
  `holds()` leaves it out, so it does not raise the lock's prompt on its own.
  See [Codex32 shares](features/codex-shares.md),
  [Codex32 backup item](features/codex-backup.md).
- **Whether Faraday's secret-holding types wipe what they hold.** For several
  types this was not checked: the keys of a new key, `Parts` and `Clash` of
  the key forms, the restore pool, the Outbox item that takes a plain secret,
  and `StarterCommand`, whose `Debug` would print a recovery key. See
  [Making a new key](features/keygen-flow.md),
  [Adding a key](features/add-key.md),
  [Restoring a wallet](features/restore-wallet.md),
  [Secrets, text checks and the passphrase field](features/secrets.md),
  [Inside a vault](features/vault-contents.md),
  [Starting from the stick](features/starter-start.md).
- **Drawing writes state or does work.** Several screens write to state while
  drawing (scroll extents, camera pane, measured heights) or derive keys and
  addresses each frame. Which writes are memos and which are real state is
  not decided. See [Drawing and costly work](features/drawing-compute.md),
  [The screen frame and navigation](features/screen-frame.md),
  [The session pane](features/session-pane.md),
  [The pane container](features/pane-container.md),
  [Adding a key](features/add-key.md),
  [Codex32 shares](features/codex-shares.md),
  [Silent payments](features/silent-payments.md),
  [Addresses: check, find and explore](features/address-check-explore.md),
  [Spending from a backup](features/spend-backup.md).
- **Faraday Demo's lock was not traced.** Whether the Demo ends the app or
  rebuilds it at a lock, and what it carries over, is not known. See
  [The lock](features/lock.md),
  [Practice and Faraday Demo](features/practice-demo.md),
  [The Starter app](features/starter-app.md).
- **No list of screens per product, and no map of sheets to their drawing.**
  Which screens exist on the device, in the Demo and in the Starter was not
  listed, and which sheet opens which drawing function was not recorded. See
  [The app and its event loop](features/app-shell.md),
  [The screen frame and navigation](features/screen-frame.md),
  [Sheets and shared parts](features/sheets-parts.md).
- **Some fix rows change a decision or add behaviour.** They are not plain
  regressions, though [`fixes.md`](fixes.md) lists every row as one. See
  [Step flows](features/flow-container.md),
  [The Vaults list and making a vault](features/vault-list-create.md),
  [Unlocking a vault](features/vault-unlock.md),
  [Making a new key](features/keygen-flow.md),
  [Backing up a key](features/backup-flow.md).
