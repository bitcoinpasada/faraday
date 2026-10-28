# Screen reference

**Kind:** Spec
**Status:** The index of the guide's screen reference; the pages below are written, the screens and sheets under To write have no page yet
**Open:** the pages for the screens and sheets under To write; the pictures

One page per screen, grouped as the sidebar groups them. A page's
`**Screen:**` line names the `Screen` and `Sheet` variants of
`faraday/faraday-core/src/lib.rs` it covers; small sheets share a page,
a section each. The walkthroughs ([`docs/GUIDE.md`](../../GUIDE.md))
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

## To write

The variants no page names yet. `tools/lint-guide.py` fails when a
variant is named by no page and is not listed here, when one is named
by two pages, and when one listed here has a page: a page written takes
its variants off this list.

- `Screen`: `WalletsStart`, `Wallets`, `Spend`, `Entry`, `Backup`, `Message`, `CheckMessage`, `Create`, `Restore`, `Settings`, `SetTime`, `Randomness`, `Backups`, `KeyGen`, `Bip85`, `Silent`, `Explore`, `Lightning`, `Tools`, `Family`, `Vanity`, `Decode`, `Catalog`, `BackupFlow`
- `Sheet`: `Learn`, `WordList`, `Chart`, `BoardList`, `BoardSees`, `BoardPartials`, `BoardMenu`, `KeyPath`, `Register`, `KeyLengths`
