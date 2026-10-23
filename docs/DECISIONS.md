# Decisions

**Kind:** Record
**Status:** Kept as written; indexed here

Faraday's own decisions, numbered F1 onward, newest last. Each is a file
of its own: `docs/decisions/F<n>.md`. A decision that was replaced,
superseded or withdrawn moves to `docs/history/decisions/` and keeps its
row here. A new decision, about `core/` as about anything else, is a new
file there and a row at the foot of this table. An amendment is a line
at the top of the decision it amends, naming the decision that amends it,
and that decision's row here says "Amended by".

Read this table, then only the decision you need.

| F# | Title | Date | Status | Area |
|---|---|---|---|---|
| [F1](decisions/F1.md) | The boot import, and one line of fact on its sheet | 2026-10-08 | Active | stick |
| [F2](decisions/F2.md) | Faraday upgrades another Faraday stick, and never signs inside the upgrade | 2026-10-08 | Amended by F11 | stick |
| [F3](decisions/F3.md) | A multi-choice list beside Choice, for the backup's plan | 2026-10-09 | Active | design |
| [F4](decisions/F4.md) | On the Pi, "SD card" where the app says "stick" | 2026-10-09 | Active | stick |
| [F5](decisions/F5.md) | Release, test release and dev builds, each named in the release string | 2026-10-10 | Active | release |
| [F6](decisions/F6.md) | The backup before the spend | 2026-10-10 | Active | backup |
| [F7](decisions/F7.md) | TOTP codes, and a clock the person can set | 2026-10-11 | Active | keys |
| [F8](decisions/F8.md) | Scan to sign, and a Spend tab that asks for data | 2026-10-11 | Active | wallets |
| [F9](decisions/F9.md) | Writing over a file on a stick, and an Add glyph | 2026-10-11 | Active | stick |
| [F10](decisions/F10.md) | The name as a wordmark | 2026-10-11 | Active | design |
| [F11](decisions/F11.md) | The Starter: one download that makes sticks and cards | 2026-10-12 | Amended by F12, F13, F30, F33 | Starter |
| [F12](decisions/F12.md) | The Starter, reviewed: what the owner changed in v0.2 | 2026-10-12 | Amended by F30, F33 | Starter |
| [F13](decisions/F13.md) | Faraday on Macs: the stick, the desktop app and the Starter | 2026-10-12 | Amended by F30 | platforms |
| [F14](decisions/F14.md) | Maker pages are published several families at once | 2026-10-12 | Active | Starter |
| [F15](history/decisions/F15.md) | Faraday Companion: an Android app with no network access | 2026-10-12 | Superseded by F34 | platforms |
| [F16](decisions/F16.md) | The hub app is Faraday Starter; the desktop app is Faraday Desktop | 2026-10-12 | Active | Starter |
| [F17](history/decisions/F17.md) | The BitLocker step is the recovery key; suspending is the extra-safe choice | 2026-10-13 | Superseded by F33 | Starter |
| [F18](decisions/F18.md) | Firmware sketches in Faraday Starter and the Companion | 2026-10-13 | Amended by F34 | Starter |
| [F19](decisions/F19.md) | Audits from experts and AIs, published with Faraday's response | 2026-10-13 | Active | repository |
| [F20](decisions/F20.md) | Faraday Desktop's mainnet is the demo; test networks take any seed | 2026-10-14 | Amended by F81 | platforms |
| [F21](history/decisions/F21.md) | The BitLocker step is optional | 2026-10-14 | Superseded by F33 | Starter |
| [F22](decisions/F22.md) | An armored GPG public key or signature is public | 2026-10-14 | Amended by F32 | signing |
| [F23](decisions/F23.md) | The PC stick's kernel is 6.18 longterm, ahead of upstream's | 2026-10-14 | Active | stick |
| [F24](decisions/F24.md) | The stick's reminders come before the first secret, not at start | 2026-10-14 | Active | stick |
| [F25](decisions/F25.md) | The Companion's APK signing key is made, kept and used in Faraday | 2026-10-14 | Amended by F31, F34, F59 | signing |
| [F26](decisions/F26.md) | Files of any size on a stick visit, imported whole or only for signing | 2026-10-14 | Amended by F30 | stick |
| [F27](decisions/F27.md) | Nostr keys from the seed, by BIP-85 application 128002' | 2026-10-14 | Amended by F34 | keys |
| [F28](decisions/F28.md) | QR codes for public values wherever another device reads them, and for the TOTP secret | 2026-10-14 | Amended by F33 | QR |
| [F29](decisions/F29.md) | A copy by hand is checked by its form: typed words, or a scan of a SeedQR | 2026-10-14 | Active | backup |
| [F30](decisions/F30.md) | No released Faraday Desktop; the demo makes no secrets | 2026-10-14 | Amended by F61, F81, F83 | release |
| [F31](decisions/F31.md) | Decide where a key will be kept, then make it: one backup flow for every secret Faraday makes | 2026-10-14 | Amended by F58 | backup |
| [F32](decisions/F32.md) | GPG follows GnuPG's behaviour | 2026-10-14 | Amended by F63 | signing |
| [F33](decisions/F33.md) | Secure Boot back on is the step; no BitLocker recovery key in the Starter or the Companion | 2026-10-14 | Amended by F34, F48, F49 | Starter |
| [F34](decisions/F34.md) | Faraday Companion is dropped | 2026-10-15 | Active | platforms |
| [F35](decisions/F35.md) | A wallet's backup plan is asked place by place | 2026-10-15 | Active | backup |
| [F36](decisions/F36.md) | Numbered stick pills on every device | 2026-10-15 | Active | stick |
| [F37](decisions/F37.md) | The backup plan is a board of places | 2026-10-15 | Active | backup |
| [F38](history/decisions/F38.md) | Access layers | 2026-10-16 | Withdrawn | backup |
| [F39](decisions/F39.md) | One page colour; steps are sections, not cards | 2026-10-16 | Active | design |
| [F40](decisions/F40.md) | The plan is what the places keep | 2026-10-16 | Active | backup |
| [F41](decisions/F41.md) | A flow in progress is always one press away | 2026-10-16 | Active | design |
| [F42](decisions/F42.md) | Restart beside Power off, with the BIOS key | 2026-10-16 | Active | stick |
| [F43](decisions/F43.md) | The cursor moves by the screen's scale | 2026-10-16 | Active | stick |
| [F44](decisions/F44.md) | Add a key is the New key flow | 2026-10-19 | Active | keys |
| [F45](decisions/F45.md) | Keys from a vault in Create; a new wallet from one you have | 2026-10-19 | Active | wallets |
| [F46](history/decisions/F46.md) | Access layers: layers first, a paper route for every layer | 2026-10-19 | Withdrawn | backup |
| [F47](decisions/F47.md) | Three things to write down before the restart, and a sheet to print | 2026-10-19 | Active | Starter |
| [F48](decisions/F48.md) | The recovery key, framed plainly; no Microsoft account row | 2026-10-19 | Amended by F49 | Starter |
| [F49](decisions/F49.md) | The Starter offers to suspend BitLocker and to print its recovery key | 2026-10-19 | Active | Starter |
| [F50](decisions/F50.md) | OSKDATA keeps its drive letter; a computer's own files are ignored by name | 2026-10-19 | Active | stick |
| [F51](decisions/F51.md) | A flow is left by Cancel inside it | 2026-10-19 | Active | design |
| [F52](decisions/F52.md) | At most one place per key; partial descriptions by one button | 2026-10-19 | Active | backup |
| [F53](decisions/F53.md) | One way to start a backup; Customize for the rest | 2026-10-20 | Active | backup |
| [F54](decisions/F54.md) | A ⋮ on every file; files renamed | 2026-10-20 | Active | stick |
| [F55](decisions/F55.md) | The derivation path on Check, its account on Kind | 2026-10-20 | Amended by F98 | wallets |
| [F56](decisions/F56.md) | Seeds alone spend; a place says how near it brings a spend | 2026-10-20 | Active | backup |
| [F57](decisions/F57.md) | A new vault's slots go by the vault's name | 2026-10-20 | Active | vault |
| [F58](decisions/F58.md) | No idle lock while a new wallet's keys are kept nowhere; 30 and 60 minutes | 2026-10-20 | Active | backup |
| [F59](decisions/F59.md) | APK signing is switched off in the app; its code stays | 2026-10-20 | Active | signing |
| [F60](decisions/F60.md) | The boot sheet is a picker; a file row offers a vault only while one is open | 2026-10-20 | Amended by F62 | stick |
| [F61](decisions/F61.md) | Faraday Desktop is published again, testnet only for real seeds | 2026-10-20 | Amended by F81, F83 | release |
| [F62](decisions/F62.md) | Unlocking a vault asks what to load | 2026-10-20 | Active | vault |
| [F63](decisions/F63.md) | Sign a file signs from a sheet that shows the time | 2026-10-20 | Active | signing |
| [F64](decisions/F64.md) | Other wallets' formats, the less common under More options | 2026-10-20 | Active | wallets |
| [F65](history/decisions/F65.md) | Upstream's code taken in house: a soft fork | 2026-10-20 | Replaced by F72 | repository |
| [F66](decisions/F66.md) | Hardened steps are written `h` | 2026-10-20 | Active | wallets |
| [F67](decisions/F67.md) | The stick's partitions and files have Faraday's names | 2026-10-21 | Active | stick |
| [F68](decisions/F68.md) | One strings file; the upstream app removed | 2026-10-21 | Active | repository |
| [F69](decisions/F69.md) | Costly work takes a token; drawing cannot mint one | 2026-10-21 | Active | speed |
| [F70](decisions/F70.md) | One crate holds every key: faraday-keys | 2026-10-21 | Active | keys |
| [F71](decisions/F71.md) | Dependencies: what cargo-vet says is true | 2026-10-21 | Active | dependencies |
| [F72](decisions/F72.md) | Faraday's code is its own: a hard fork | 2026-10-21 | Amended by F73, F97 | repository |
| [F73](decisions/F73.md) | The upstream project is named only in LICENSE | 2026-10-21 | Amended by F97 | repository |
| [F74](decisions/F74.md) | The structure is held by `just` | 2026-10-21 | Active | repository |
| [F75](decisions/F75.md) | A seed can be pasted on the desktop | 2026-10-22 | Active | keys |
| [F76](decisions/F76.md) | The key explorer lists an account's addresses and finds one | 2026-10-22 | Active | wallets |
| [F77](decisions/F77.md) | Back lands on the screen seen before, as it was seen | 2026-10-22 | Active | design |
| [F78](decisions/F78.md) | A window's layout does not follow the compositor's scale above 1.5 | 2026-10-22 | Active | design |
| [F79](decisions/F79.md) | Faraday's crypto is cross-checked against Python references, offline, at build time | 2026-10-22 | Active | crypto |
| [F80](decisions/F80.md) | A stick or Pi card has a 64 MiB boot and a 128 MiB FAT32 data partition | 2026-10-22 | Active | stick |
| [F81](decisions/F81.md) | Modes: a pressable pill, Demo on testnet, samples at every step | 2026-10-22 | Active | design |
| [F82](decisions/F82.md) | A first start opens Steps only; Guided is a choice | 2026-10-22 | Active | design |
| [F83](decisions/F83.md) | Faraday Desktop is inside the Starter, and not published on its own | 2026-10-23 | Active | release |
| [F84](decisions/F84.md) | Check a signer fails closed; unknown fields are findings there and Info in a review | 2026-10-23 | Active | crypto |
| [F85](decisions/F85.md) | Text size: Standard, Large, Larger, and the layout steps down for it | 2026-10-23 | Active | design |
| [F86](decisions/F86.md) | Your own randomness is mixed into the session's; the stick's kernel credits what it says it does | 2026-10-23 | Active | security |
| [F87](decisions/F87.md) | A backup's copies carry the date they were last checked, and a recovery path's wait is stated | 2026-10-23 | Active | backup |
| [F88](decisions/F88.md) | Seed conversions are one tool, and only to forms that restore the same wallet | 2026-10-23 | Active | keys |
| [F89](decisions/F89.md) | Silent payment signing counts a stated key only where the input's script commits to it | 2026-10-23 | Active | crypto |
| [F90](decisions/F90.md) | Where Faraday refuses input its standard allows | 2026-10-23 | Active | crypto |
| [F91](decisions/F91.md) | The Starter is laid out to Material 3's layout and navigation rules | 2026-10-23 | Active | design |
| [F92](decisions/F92.md) | Faraday is laid out to Material 3 at every window size, from the Pi to a full monitor | 2026-10-23 | Active | design |
| [F93](decisions/F93.md) | Codex32 made by hand, any split | 2026-10-22 | Active | keys |
| [F94](decisions/F94.md) | Codex32 error correction, opt-in | 2026-10-22 | Active | crypto |
| [F95](decisions/F95.md) | Codex32QR/v1-48: read, shown, tested | 2026-10-22 | Active | keys |
| [F96](decisions/F96.md) | Faraday Desktop's dice rollers | 2026-10-22 | Active | keys |
| [F97](decisions/F97.md) | .oskb files are no longer read; the vault's record layouts are its own | 2026-10-23 | Active | backup |
| [F98](decisions/F98.md) | A new wallet's account is asked after its quorum, not on Kind | 2026-10-23 | Active | wallets |
