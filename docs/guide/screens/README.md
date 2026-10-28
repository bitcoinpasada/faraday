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

## To write

The variants no page names yet. `tools/lint-guide.py` fails when a
variant is named by no page and is not listed here, when one is named
by two pages, and when one listed here has a page: a page written takes
its variants off this list.

- `Screen`: `WalletsStart`, `Wallets`, `Spend`, `Files`, `Visit`, `Entry`, `Backup`, `Message`, `CheckMessage`, `Create`, `Restore`, `Settings`, `SetTime`, `Randomness`, `Vaults`, `Backups`, `CreateVault`, `KeyGen`, `Bip85`, `Silent`, `Explore`, `Lightning`, `Tools`, `Unlock`, `VaultContents`, `Codes`, `Family`, `Vanity`, `Decode`, `Catalog`, `Transfer`, `BackupFlow`
- `Sheet`: `Qr`, `Scan`, `Learn`, `WordList`, `SecretOut`, `WriteOut`, `Potential`, `Chart`, `BoardList`, `BoardSees`, `BoardPartials`, `BoardMenu`, `SeedRead`, `Delete`, `Zeroing`, `Taken`, `KeyPath`, `Register`, `VaultSave`, `FileMenu`, `Rename`, `Details`, `UnlockLoad`, `PassphraseSeeds`, `GpgSign`, `KeyLengths`
