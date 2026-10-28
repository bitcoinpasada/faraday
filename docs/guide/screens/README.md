# Screen reference

**Kind:** Spec
**Status:** The index of the guide's screen reference; every screen and sheet of Faraday has its page below. Every page shows its pictures from `docs/guide/pictures/` and is checked against them, but for the four named on the Open line
**Open:** four pictures drawn as empty pages, to be drawn again before their pages become References: `calculators.png` ([Tools: the calculators](calculators.md)), `decode.png` ([Decode a transaction](decode.md)), `vanity.png` ([Vanity address](vanity.md)) and `settings-add-randomness.png` ([Settings](settings.md))

One page per screen, grouped as the sidebar groups them. A page's
`**Screen:**` line names the `Screen` and `Sheet` variants of
`faraday/faraday-core/src/lib.rs` it covers; small sheets share a page,
a section each; the Starter's pages name the `Page` and `Sheet`
variants of `faraday/faraday-starter/src/lib.rs` on `**Starter page:**`
and `**Starter sheet:**` lines. The walkthroughs ([`docs/GUIDE.md`](../../GUIDE.md))
say what to do; these pages say what each control on a screen does.

## Pages

| Group | Page | Covers |
|---|---|---|
| Home | [Home](home.md) | Home, the sidebar, the ⋮ menu |
| Home | [Prompts](prompts.md) | Attached to this computer, a new keyboard, the boot stick's import, Pull the stick, Cancel a flow |
| Home | [Lock and Power off](lock-and-power.md) | Lock, A stick is attached, Locking soon, Locked, Power |
| Home | [Mode and Load samples](mode.md) | Mode, Load samples |
| Vaults | [Vaults and Unlock](vaults.md) | Vaults, Unlock, Load from, Seeds with no passphrase stored |
| Vaults | [Create a vault](create-vault.md) | Create a vault, Generate with dice |
| Vaults | [Vault contents](vault.md) | Vault contents: keys, wallets, entries, notes, GPG keys, Secure Boot keys; Save to a stick, Sign with |
| Vaults | [TOTP codes](codes.md) | TOTP codes |
| Files | [Files](files.md) | Files, a file's ⋮ menu, Rename, Details, Make it a wallet, This is a secret, Save without encryption? |
| Files | [Stick visit](stick-visit.md) | Stick visit, Already on the stick, Write to a stick, Delete files, Zeroing |
| Files | [QR codes and the camera](qr-and-scan.md) | A QR code, Scan, SeedQR read, Transfer |
| Wallets | [Wallets](wallets.md) | Wallets' first page, the list and the wallet card, Another key spends this wallet alone, Register this wallet |
| Wallets | [Add a key](new-key.md) | Add a key and New key: Start, Length (and its Other menu), Randomness, the entries, Key, Words, Quiz; Make codex32 shares |
| Wallets | [Add a key: typing a key](add-a-key.md) | The words, the other forms, codex32 strings and their correction |
| Wallets | [Make a new wallet](make-a-wallet.md) | Kind, Quorum, Use vaults, Keys, Check |
| Wallets | [Restore an existing wallet](restore.md) | Kind, Quorum, Description, Use vaults, Keys, Check, Done |
| Wallets | [Spend](spend-tab.md) | The Spend tab: what is loaded, the column from the map to Put everything away, Spend from a wallet |
| Wallets | [Sign a transaction](sign.md) | Wallet, Path, Transaction, Check, Signers, Nonces, Sign, Signatures for this transaction, Finish, the panel |
| Wallets | [Back up a wallet](back-up-wallet.md) | The plan: Places (the board and its list), Software, The plan; the checklist; Add, Keep it or Take it off, partial descriptions, a chip's menu |
| Wallets | [The backup flow](backup-flow.md) | Where it will be kept, the sticks, the vaults, the key, Lock, Write the sticks, Done |
| Wallets | [Backups and the chart](backups.md) | Backups, and the chart's sheets: the wallet, a key, a place, a line, a change asked first |
| Tools | [Tools](tools.md) | Every tool by group and its search, the word list |
| Starter | [Home](starter-home.md) | The rail and the top app bar, This computer, the checklist, what can record, Faraday Demo, Update, the boot entry, Continue anyway |
| Starter | [Make](starter-make.md) | Make, For which computer?, Which stick? or Which card?, What to write? and its boot file, Confirm, Writing, Written |
| Starter | [Tools](starter-tools.md) | Tools, Transfer, Blank templates and each choice's page |
| Starter | [Start from the stick, and Learn](starter-learn-and-boot.md) | Start this computer (or this Mac) from the stick, the restart sheet, Learn |
| Starter | [Install, Appearance and About](starter-menu.md) | The ⋮ menu, Install and Update, Installed and Uninstall, Appearance, About |
| Tools | [Tools: the calculators](calculators.md) | Hashes, Encodings, Descriptor checksum, Convert key, Units, Miniscript |
| Tools | [Decode a transaction](decode.md) | The facts, inputs and outputs, the hex and its QR code |
| Tools | [Sign a message and Check a signed message](messages.md) | Address, Message, Format, Signature; the verdict, File, Address, Wallet, Message |
| Tools | [Explore](explore.md) | A key at a path, its secrets behind their eyes; Addresses, Find an address |
| Tools | [Derive with BIP-85](child-seeds.md) | Key, Application, Index, Value |
| Tools | [Silent payments](silent-payments.md) | Key, Address and its labels, Scan key, Check a payment |
| Tools | [Vanity address](vanity.md) | Key, What turns, Kind of address, Characters, Search |
| Tools | [Lightning node key](lightning.md) | ldk-node from a loaded key, LND from an aezeed, the node id and its key |
| Learn | [Learn](learn.md) | Learn's tabs and pages, beside the page on a wide window, the pages each screen gives |
| Settings | [Settings](settings.md) | Mode, Session, Time, Randomness, Appearance, Display, Signed amounts, About; Set the time; Add randomness |

## To write

The variants no page names yet. `tools/lint-guide.py` fails when a
variant is named by no page and is not listed here, when one is named
by two pages, and when one listed here has a page: a page written takes
its variants off this list.

None.
