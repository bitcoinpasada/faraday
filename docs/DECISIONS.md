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
| [F2](decisions/F2.md) | Faraday upgrades another Faraday stick, and never signs inside the upgrade | 2026-10-08 | Amended by F11, F128, F244 | stick |
| [F3](decisions/F3.md) | A multi-choice list beside Choice, for the backup's plan | 2026-10-09 | Active | design |
| [F4](history/decisions/F4.md) | On the Pi, "SD card" where the app says "stick" | 2026-10-09 | Replaced by F216 | stick |
| [F5](decisions/F5.md) | Release, test release and dev builds, each named in the release string | 2026-10-10 | Amended by F128 | release |
| [F6](decisions/F6.md) | The backup before the spend | 2026-10-10 | Active | backup |
| [F7](decisions/F7.md) | TOTP codes, and a clock the person can set | 2026-10-11 | Amended by F123 | keys |
| [F8](decisions/F8.md) | Scan to sign, and a Spend tab that asks for data | 2026-10-11 | Active | wallets |
| [F9](decisions/F9.md) | Writing over a file on a stick, and an Add glyph | 2026-10-11 | Active | stick |
| [F10](decisions/F10.md) | The name as a wordmark | 2026-10-11 | Amended by F137, F140 | design |
| [F11](decisions/F11.md) | The Starter: one download that makes sticks and cards | 2026-10-12 | Amended by F12, F13, F30, F33, F128, F133 | Starter |
| [F12](decisions/F12.md) | The Starter, reviewed: what the owner changed in v0.2 | 2026-10-12 | Amended by F30, F33 | Starter |
| [F13](decisions/F13.md) | Faraday on Macs: the stick, the desktop app and the Starter | 2026-10-12 | Amended by F30 | platforms |
| [F14](decisions/F14.md) | Maker pages are published several families at once | 2026-10-12 | Active | Starter |
| [F15](history/decisions/F15.md) | Faraday Companion: an Android app with no network access | 2026-10-12 | Superseded by F34 | platforms |
| [F16](decisions/F16.md) | The hub app is Faraday Starter; the desktop app is Faraday Demo | 2026-10-12 | Amended by F119, F126, F165 | Starter |
| [F17](history/decisions/F17.md) | The BitLocker step is the recovery key; suspending is the extra-safe choice | 2026-10-13 | Superseded by F33 | Starter |
| [F18](decisions/F18.md) | Firmware sketches in Faraday Starter and the Companion | 2026-10-13 | Amended by F34, F105 | Starter |
| [F19](decisions/F19.md) | Audits from experts and AIs, published with Faraday's response | 2026-10-13 | Active | repository |
| [F20](decisions/F20.md) | Faraday Desktop's mainnet is the demo; test networks take any seed | 2026-10-14 | Amended by F81 | platforms |
| [F21](history/decisions/F21.md) | The BitLocker step is optional | 2026-10-14 | Superseded by F33 | Starter |
| [F22](decisions/F22.md) | An armored GPG public key or signature is public | 2026-10-14 | Amended by F32 | signing |
| [F23](decisions/F23.md) | The PC stick's kernel is 6.18 longterm, ahead of upstream's | 2026-10-14 | Active | stick |
| [F24](decisions/F24.md) | The stick's reminders come before the first secret, not at start | 2026-10-14 | Active | stick |
| [F25](history/decisions/F25.md) | The Companion's APK signing key is made, kept and used in Faraday | 2026-10-14 | Replaced by F99 | signing |
| [F26](decisions/F26.md) | Files of any size on a stick visit, imported whole or only for signing | 2026-10-14 | Amended by F30 | stick |
| [F27](decisions/F27.md) | Nostr keys from the seed, by BIP-85 application 128002' | 2026-10-14 | Amended by F34 | keys |
| [F28](decisions/F28.md) | QR codes for public values wherever another device reads them, and for the TOTP secret | 2026-10-14 | Amended by F33, F123, F215, F236 | QR |
| [F29](decisions/F29.md) | A copy by hand is checked by its form: typed words, or a scan of a SeedQR | 2026-10-14 | Active | backup |
| [F30](decisions/F30.md) | No released Faraday Desktop; the demo makes no secrets | 2026-10-14 | Amended by F61, F81, F83 | release |
| [F31](decisions/F31.md) | Decide where a key will be kept, then make it: one backup flow for every secret Faraday makes | 2026-10-14 | Amended by F58, F99 | backup |
| [F32](decisions/F32.md) | GPG follows GnuPG's behaviour | 2026-10-14 | Amended by F63 | signing |
| [F33](decisions/F33.md) | Secure Boot back on is the step; no BitLocker recovery key in the Starter or the Companion | 2026-10-14 | Amended by F34, F48, F49 | Starter |
| [F34](decisions/F34.md) | Faraday Companion is dropped | 2026-10-15 | Amended by F99 | platforms |
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
| [F53](decisions/F53.md) | One way to start a backup; Customize for the rest | 2026-10-20 | Amended by F122 | backup |
| [F54](decisions/F54.md) | A ⋮ on every file; files renamed | 2026-10-20 | Active | stick |
| [F55](decisions/F55.md) | The derivation path on Check, its account on Kind | 2026-10-20 | Amended by F98 | wallets |
| [F56](decisions/F56.md) | Seeds alone spend; a place says how near it brings a spend | 2026-10-20 | Active | backup |
| [F57](history/decisions/F57.md) | A new vault's slots go by the vault's name | 2026-10-20 | Superseded by F190 | vault |
| [F58](decisions/F58.md) | No idle lock while a new wallet's keys are kept nowhere; 30 and 60 minutes | 2026-10-20 | Active | backup |
| [F59](history/decisions/F59.md) | APK signing is switched off in the app; its code stays | 2026-10-20 | Replaced by F99 | signing |
| [F60](decisions/F60.md) | The boot sheet is a picker; a file row offers a vault only while one is open | 2026-10-20 | Amended by F62, F130 | stick |
| [F61](decisions/F61.md) | Faraday Desktop is published again, testnet only for real seeds | 2026-10-20 | Amended by F81, F83 | release |
| [F62](decisions/F62.md) | Unlocking a vault asks what to load | 2026-10-20 | Active | vault |
| [F63](decisions/F63.md) | Sign a file signs from a sheet that shows the time | 2026-10-20 | Active | signing |
| [F64](decisions/F64.md) | Other wallets' formats, the less common under More options | 2026-10-20 | Active | wallets |
| [F65](history/decisions/F65.md) | Upstream's code taken in house: a soft fork | 2026-10-20 | Replaced by F72 | repository |
| [F66](decisions/F66.md) | Hardened steps are written `h` | 2026-10-20 | Active | wallets |
| [F67](decisions/F67.md) | The stick's partitions and files have Faraday's names | 2026-10-21 | Amended by F128, F174 | stick |
| [F68](decisions/F68.md) | One strings file; the upstream app removed | 2026-10-21 | Active | repository |
| [F69](decisions/F69.md) | Costly work takes a token; drawing cannot mint one | 2026-10-21 | Active | speed |
| [F70](decisions/F70.md) | One crate holds every key: faraday-keys | 2026-10-21 | Amended by F125 | keys |
| [F71](decisions/F71.md) | Dependencies: what cargo-vet says is true | 2026-10-21 | Amended by F178, F180 | dependencies |
| [F72](decisions/F72.md) | Faraday's code is its own: a hard fork | 2026-10-21 | Amended by F73, F97 | repository |
| [F73](decisions/F73.md) | The upstream project is named only in LICENSE | 2026-10-21 | Amended by F97 | repository |
| [F74](decisions/F74.md) | The structure is held by `just` | 2026-10-21 | Amended by F196 | repository |
| [F75](decisions/F75.md) | A seed can be pasted on the desktop | 2026-10-22 | Active | keys |
| [F76](decisions/F76.md) | The key explorer lists an account's addresses and finds one | 2026-10-22 | Active | wallets |
| [F77](decisions/F77.md) | Back lands on the screen seen before, as it was seen | 2026-10-22 | Amended by F104, F128 | design |
| [F78](history/decisions/F78.md) | A window's layout does not follow the compositor's scale above 1.5 | 2026-10-22 | Replaced by F101 | design |
| [F79](decisions/F79.md) | Faraday's crypto is cross-checked against Python references, offline, at build time | 2026-10-22 | Amended by F160, F192, F197 | crypto |
| [F80](decisions/F80.md) | A stick or Pi card has a 64 MiB boot and a 128 MiB FAT32 data partition | 2026-10-22 | Amended by F128, F174, F182 | stick |
| [F81](decisions/F81.md) | Modes: a pressable pill, Practice on testnet, samples at every step | 2026-10-22 | Amended by F119, F143 | design |
| [F82](decisions/F82.md) | A first start opens Steps only; Guided is a choice | 2026-10-22 | Active | design |
| [F83](decisions/F83.md) | Faraday Demo is inside the Starter, and not published on its own | 2026-10-23 | Amended by F126 | release |
| [F84](decisions/F84.md) | Check a signer fails closed; unknown fields are findings there and Info in a review | 2026-10-23 | Active | crypto |
| [F85](decisions/F85.md) | Text size: Standard, Large, Larger, and the layout steps down for it | 2026-10-23 | Active | design |
| [F86](decisions/F86.md) | Your own randomness is mixed into the session's; the stick's kernel credits what it says it does | 2026-10-23 | Amended by F102, F124, F157 | security |
| [F87](decisions/F87.md) | A backup's copies carry the date they were last checked, and a recovery path's wait is stated | 2026-10-23 | Active | backup |
| [F88](decisions/F88.md) | Seed conversions are one tool, and only to forms that restore the same wallet | 2026-10-23 | Active | security |
| [F89](decisions/F89.md) | Silent payment signing counts a stated key only where the input's script commits to it | 2026-10-23 | Active | crypto |
| [F90](decisions/F90.md) | Where Faraday refuses input its standard allows | 2026-10-23 | Amended by F155 | crypto |
| [F91](decisions/F91.md) | The Starter is laid out to Material 3's layout and navigation rules | 2026-10-23 | Amended by F108 and F113 | design |
| [F92](decisions/F92.md) | Faraday is laid out to Material 3 at every window size, from the Pi to a full monitor | 2026-10-23 | Amended by F100 | design |
| [F93](decisions/F93.md) | Codex32 made by hand, any split | 2026-10-22 | Amended by F121 | keys |
| [F94](decisions/F94.md) | Codex32 error correction, opt-in | 2026-10-22 | Active | crypto |
| [F95](decisions/F95.md) | Codex32QR/v1-48: read, shown, tested | 2026-10-22 | Active | keys |
| [F96](decisions/F96.md) | Faraday Demo's dice rollers | 2026-10-22 | Active | keys |
| [F97](decisions/F97.md) | .oskb files are no longer read; the vault's record layouts are its own | 2026-10-23 | Amended by F190 | backup |
| [F98](decisions/F98.md) | A new wallet's account is asked after its quorum, not on Kind | 2026-10-23 | Active | wallets |
| [F99](decisions/F99.md) | APK signing is removed | 2026-10-23 | Active | keys |
| [F100](decisions/F100.md) | The 7-inch and 10-inch panels run in landscape, with portrait a setting | 2026-10-23 | Active | design |
| [F101](decisions/F101.md) | A window is laid out at its screen's own density; F78 is superseded | 2026-10-23 | Amended by F100 | design |
| [F102](decisions/F102.md) | A late seed is said only at a press that needs it; the Pi's generator is mixed, not credited | 2026-10-23 | Active | security |
| [F103](decisions/F103.md) | A choice that decides a step moves on by itself | 2026-10-23 | Active | design |
| [F104](decisions/F104.md) | Back and Forward move within a flow's steps | 2026-10-23 | Active | design |
| [F105](decisions/F105.md) | The boot guides give written directions, not mock-ups of firmware screens | 2026-10-23 | Active | Starter |
| [F106](decisions/F106.md) | Faraday reads PSBTs, mnemonics and vaults as their standards say, and writes back what it read | 2026-10-23 | Active | formats |
| [F107](history/decisions/F107.md) | Single key, backup in shares | 2026-10-22 | Superseded by F240 | wallets |
| [F108](decisions/F108.md) | Faraday's tokens, colour roles and components follow Material 3, shape included | 2026-10-23 | Amended by F113, F152, F188 | design |
| [F109](history/decisions/F109.md) | Out of the app until their standards mature: FROST, and MuSig2 signing across places | 2026-10-23 | Superseded by F234 | wallets |
| [F110](decisions/F110.md) | Faraday Desktop shows its screens at another screen's size | 2026-10-24 | Amended by F118 | design |
| [F111](decisions/F111.md) | The Material 3 components and patterns Faraday leaves out | 2026-10-24 | Active | design |
| [F112](decisions/F112.md) | One explanation on a working screen: the tooltip on "codex32" in Make a new wallet's Kind card | 2026-10-24 | Amended by F240 | design |
| [F113](decisions/F113.md) | Colour follows Material 3's colour system: ten seeded themes, Light and Dark, three contrast levels | 2026-10-24 | Amended by F126, F138 | design |
| [F114](decisions/F114.md) | Wide content takes the room beside the navigation; reading content keeps its column | 2026-10-24 | Active | design |
| [F115](decisions/F115.md) | A key flow opened from Make a new wallet has Cancel, back to it; a Keys card with one way on is passed over | 2026-10-24 | Amended by F240 | design |
| [F116](decisions/F116.md) | Faraday's scope: an offline signer and keeper of keys, Bitcoin first | 2026-10-25 | Active | scope |
| [F117](decisions/F117.md) | Faraday Demo matches the device; it differs only where the platform or an owner decision forces it | 2026-10-25 | Amended by F119 | design |
| [F118](decisions/F118.md) | Faraday Demo is a simulator: what is Faraday Demo's or Practice's stands in a host strip outside the app | 2026-10-25 | Amended by F119 | design |
| [F119](decisions/F119.md) | Faraday Desktop is Faraday Demo; its Demo mode is Practice | 2026-10-26 | Amended by F126 | design |
| [F120](decisions/F120.md) | Settings' theme chips are filled in their own theme's colours; the chosen one is ringed | 2026-10-25 | Active | design |
| [F121](decisions/F121.md) | No codex32 strings for a key of words | 2026-10-26 | Active | backup |
| [F122](decisions/F122.md) | Make a new wallet: one press takes a stick, and Check opens the backup | 2026-10-26 | Active | backup |
| [F123](decisions/F123.md) | A secret shows at a press on its eye, never by a hold | 2026-10-26 | Active | keys |
| [F124](decisions/F124.md) | Randomness is never collected passively; pointer and keys are a source one picks | 2026-10-26 | Active | security |
| [F125](decisions/F125.md) | Explore shows a key's secrets behind eyes | 2026-10-26 | Active | keys |
| [F126](decisions/F126.md) | Faraday Demo runs inside Faraday Starter; one theme in a session | 2026-10-26 | Active | Starter |
| [F127](decisions/F127.md) | FROST removed from the repository until its standards are published | 2026-10-26 | Active | wallets |
| [F128](decisions/F128.md) | Upgrading a stick is the Starter's; Faraday writes no boot partition | 2026-10-26 | Amended by F133 | stick |
| [F129](decisions/F129.md) | A secret saved as a plain file asks for a hold | 2026-10-26 | Active | keys |
| [F130](decisions/F130.md) | The boot import reads names and sizes until Import, then only the ticked files | 2026-10-27 | Active | stick |
| [F131](decisions/F131.md) | Zeroing a stick's free space or its partition takes a hold | 2026-10-27 | Active | stick |
| [F132](decisions/F132.md) | A tr() wallet states who can spend by the key path | 2026-10-26 | Active | wallets |
| [F133](decisions/F133.md) | The Starter writes a boot file the person signed, once it is checked to be the release's own | 2026-10-27 | Active | Starter |
| [F134](decisions/F134.md) | Faraday signs a boot program only when it is a release's own | 2026-10-27 | Active | security |
| [F135](decisions/F135.md) | A script of several keys is signed only for a registered wallet | 2026-10-27 | Amended by F141 | wallets |
| [F136](decisions/F136.md) | No way past a check without a legitimate reason | 2026-10-27 | Amended by F141 | security |
| [F137](decisions/F137.md) | The keyhole sits lower in the shield | 2026-10-26 | Amended by F140 | design |
| [F138](decisions/F138.md) | The floating action takes Material's primary colour style | 2026-10-27 | Active | design |
| [F139](decisions/F139.md) | A compact top app bar's title steps down to Title M where Title L would be cut | 2026-10-27 | Active | design |
| [F140](decisions/F140.md) | The mark in golden proportions | 2026-10-27 | Active | design |
| [F141](decisions/F141.md) | A miniscript or a taproot tree of one key is signed only through its registered wallet | 2026-10-27 | Active | wallets |
| [F142](decisions/F142.md) | A stick's pill is filled in its tone while the stick is in, its tones set by mode and contrast | 2026-10-27 | Active | design |
| [F143](decisions/F143.md) | The user guide's key is a Practice sample, and New key keeps a sample's own words | 2026-10-27 | Active | design |
| [F144](decisions/F144.md) | With no one holding an earlier build's files, the formats drop the names before Faraday's, and another program's PSBT records are signed past | 2026-10-28 | Active | wallets |
| [F145](decisions/F145.md) | One signed image for each piece of hardware, Strict and Open, and a Starter that carries none | 2026-10-28 | Amended by F147, F150, F153, F185, F186 | release |
| [F146](decisions/F146.md) | One stick numbering, "Back to" under a page's name, and the codex32 entry's step in the top app bar | 2026-10-28 | Active | design |
| [F147](decisions/F147.md) | One Pi image per screen for every Pi, the Pi 4 among them, one build per visit to the download page, and images published for testers | 2026-10-28 | Amended by F150, F185, F186, F198 | release |
| [F148](decisions/F148.md) | One manifest for every cryptographic action, its Learn block generated from it, and `just lint` failing on a function it does not cover or whose code changed since review | 2026-10-28 | Amended by F159, F197 | security |
| [F149](decisions/F149.md) | One registry of what a change brings with it, checked at landing and before the website publishes | 2026-10-28 | Amended by F151 | repository |
| [F150](decisions/F150.md) | The looser build is named Compatible, every HDMI screen shares one image, and the Pi 4 is built on the terms its reach asks | 2026-10-28 | Amended by F153, F185, F186 | release |
| [F151](decisions/F151.md) | A review rule a change triggers is attested in the commit | 2026-10-28 | Active | repository |
| [F152](decisions/F152.md) | Touch targets are tested over the snapshot tours, and a press cannot be made without its state layer | 2026-10-28 | Amended by F188 | design |
| [F153](decisions/F153.md) | No kernel patch of Faraday's own on the Pi, and a new USB touchscreen approves itself by tapping a target: nothing is trusted by name | 2026-10-28 | Amended by F185, F202, F205, F221 | security |
| [F154](decisions/F154.md) | Random bytes are taken from the source, an answer for every 32, or expanded by HKDF-Expand | 2026-10-28 | Active | security |
| [F155](decisions/F155.md) | The other refusals of input a standard allows: keys, wallet files, PSBTs, transfers, shares, the vault and the paperkey | 2026-10-28 | Active | crypto |
| [F156](decisions/F156.md) | Schnorr signatures are made with zero aux_rand | 2026-10-28 | Active | security |
| [F157](decisions/F157.md) | Wherever the system's randomness is used, the person can add their own: Add randomness at every press that draws it | 2026-10-28 | Active | security |
| [F158](decisions/F158.md) | Faraday's own constructions in the app until an outside review: cards, the camera, a mix, the vault's container and the Secure Boot generator | 2026-10-28 | Amended by F160, F189 | security |
| [F159](decisions/F159.md) | The vault is a `.fvlt` file beginning `FVLT`, its passphrase in NFC, each slot committed to its key and counting its seals, and its construction told in Learn | 2026-10-28 | Amended by F189 | security |
| [F160](decisions/F160.md) | A check counts only against code Faraday did not write; Faraday's own constructions are labelled until an outside review; Bitcoin Core in the cross-check | 2026-10-29 | Amended by F177, F189, F190, F192 | crypto |
| [F161](decisions/F161.md) | A finalized input carries the smallest valid witness by its real size, a tie broken as Bitcoin Core breaks it | 2026-10-29 | Active | crypto |
| [F162](decisions/F162.md) | Anti-exfil in both directions, Faraday as the host first, FractalEncrypt's exchange in dev builds only behind `experimental` | 2026-10-29 | Amended by F176 | crypto |
| [F163](decisions/F163.md) | The toast's text is centred | 2026-10-29 | Active | design |
| [F164](decisions/F164.md) | The manifest's screens_not count is 24 | 2026-10-29 | Amended by F237 | crypto |
| [F165](decisions/F165.md) | The stick and the Pi read Faraday Offline | 2026-10-26 | Active | design |
| [F166](decisions/F166.md) | A BIP-85 child seed's parent is kept only in a vault, when the person records it; nothing outside one says so; a wallet made over one says so once | 2026-10-29 | Active | keys |
| [F167](decisions/F167.md) | Home's Scan takes everything a scanner takes | 2026-10-29 | Amended by F173 | keys |
| [F168](decisions/F168.md) | The Nonce check page carries the rail's two touch-target exceptions every rail page carries | 2026-10-29 | Active | design |
| [F169](decisions/F169.md) | A GPG key is restored from its paperkey and its public certificate | 2026-10-29 | Active | crypto |
| [F170](decisions/F170.md) | The vault area's code stays together for auditability, not split per screen | 2026-10-29 | Active | repository |
| [F171](decisions/F171.md) | A WIF private key is swept, never kept: a sheet of its addresses, its PSBT reviewed against the loaded wallets and signed with a hold | 2026-10-29 | Active | keys |
| [F172](decisions/F172.md) | A BIP-85 password out of Faraday names neither its parent nor its index; in a vault its entry keeps both | 2026-10-29 | Active | keys |
| [F173](decisions/F173.md) | A TOTP setup code read with no vault open shows the read card: Unlock {vault}, the code carried, or Make a vault, the code not kept | 2026-10-29 | Active | keys |
| [F174](decisions/F174.md) | A stick or card does not say it is Faraday's: plain names, its own IDs, known by its layout | 2026-10-30 | Amended by F182 | stick |
| [F175](decisions/F175.md) | The vaults and the sticks are laid out together, and each stick is drawn as it comes in | 2026-10-30 | Active | backup |
| [F176](decisions/F176.md) | Anti-exfil names two levels, checked signature and protected key; Faraday as host keeps the second, with rho in memory only and aborts counted in the vault; the reveal never in a PSBT | 2026-10-30 | Active | crypto |
| [F177](decisions/F177.md) | Each Cryptography block says whose specification, whose implementation and whose check an action rests on, each own or outside | 2026-10-30 | Amended by F179 | crypto |
| [F178](decisions/F178.md) | Established implementations first: Faraday writes its own only where none exists or none fits its key rules, and says why | 2026-10-30 | Amended by F180, F191 | crypto |
| [F179](decisions/F179.md) | A Cryptography block's fields are Specification and Code; "implementation" names only a cross-check's second program | 2026-10-30 | Active | crypto |
| [F180](decisions/F180.md) | The review of 2026-10-31: getrandom trusted, two libraries used as not established, HKDF and the BIP-39 core kept, A1 through libsecp256k1, a FAT32 formatter of Faraday's own | 2026-10-31 | Active | crypto |
| [F181](decisions/F181.md) | Memory work follows the security model: no wiping allocator, no batch for copies the lock already bounds, none for Faraday Demo and the Starter | 2026-10-31 | Active | crypto |
| [F182](decisions/F182.md) | A stick Windows leaves whole: the image's first usable sector and boot FAT, a stick the Starter makes for its size, the boot partition kept as released | 2026-10-31 | Active | stick |
| [F183](decisions/F183.md) | The check runs on GitHub Actions: `just` on every push to main and every pull request, read-only, building nothing for release | 2026-10-31 | Active | repository |
| [F184](decisions/F184.md) | Work over a tenth of a second on a Pi 3B+ runs on one worker in the core; shorter work is drawn first and done on the next tick | 2026-11-02 | Active | speed |
| [F185](decisions/F185.md) | One build on every platform: the PC stick with `efi=disable_early_pci_dma`, the Pi with what Compatible carried | 2026-10-31 | Amended by F186 | release |
| [F186](decisions/F186.md) | One Pi image for every screen, found at boot | 2026-11-02 | Amended by F187, F198 | release |
| [F187](decisions/F187.md) | The Pi's screens for now: an HDMI screen and the 2.8-inch | 2026-11-02 | Active | release |
| [F188](decisions/F188.md) | Touch targets on the 2.8-inch are 44 dp, Apple's minimum | 2026-11-02 | Active | design |
| [F189](decisions/F189.md) | The vault is an age file: one passphrase, scrypt, TOML inside, five sizes, the smallest a QR code | 2026-11-02 | Amended by F190, F226 | security |
| [F190](decisions/F190.md) | The vault is a KDBX 4 file under Argon2id, and the QR vault an RFC 9580 OpenPGP message under Argon2id | 2026-11-02 | Amended by F193, F194, F206, F207, F208 and F211 | security |
| [F191](decisions/F191.md) | The switches C1 to C14 before the timing fixes, and a switch lands with its cross-check change | 2026-10-31 | Active | crypto |
| [F192](decisions/F192.md) | The cross-check's outside sources: another lineage, Faraday's second version kept only where none covers it, every judge required in `just crosscheck` | 2026-10-31 | Amended by F193 and F211 | crypto |
| [F193](decisions/F193.md) | The vault's judges: libraries over the app's primitives judge the file format only, no Node yet, and CPython's tomllib reads the vault's contents | 2026-11-03 | Amended by F211 | crypto |
| [F194](decisions/F194.md) | A KDBX file is never refused for its contents: a damaged item is Unreadable, and every TOTP form is read for the codes page | 2026-11-03 | Amended by F231 | security |
| [F195](decisions/F195.md) | A secret the person chose to show is worked out once and held while shown, and wiped when hidden | 2026-11-03 | Active | speed |
| [F196](decisions/F196.md) | A Rust file may grow to 5,000 lines, and the function counts are no longer ratchets | 2026-11-03 | Amended by F248 | repository |
| [F197](decisions/F197.md) | The full cross-check runs before a release, and the tip between releases is not kept cross-checked | 2026-11-03 | Amended by F239 | repository |
| [F198](decisions/F198.md) | The download page has one Pi tile | 2026-11-03 | Active | release |
| [F199](decisions/F199.md) | A field a signer adds is stated when its content can be checked in full against Sent, and is a finding otherwise | 2026-11-03 | Active | security |
| [F200](decisions/F200.md) | Convert a seed carries the step-card head's and the keyboard's touch-target exceptions | 2026-11-04 | Amended by F201 | design |
| [F201](decisions/F201.md) | Convert a seed's limit of step titles under the touch floor on the stick is 4 | 2026-11-04 | Active | design |
| [F202](decisions/F202.md) | A USB pointer approves itself by pressing targets in a row while nothing trusted can point, and tries at a target are counted for the session, and a device present at start is trusted | 2026-11-04 | Amended by F218 | security |
| [F203](decisions/F203.md) | A vault's name taken by a file nothing proves is a question: overwrite or save beside | 2026-11-04 | Active | security |
| [F204](decisions/F204.md) | SLIP-39's set actions are in Convert a seed; codex32's stay with the backup | 2026-11-04 | Active | keys |
| [F205](decisions/F205.md) | One kernel patch of Faraday's own on the Pi: a failed read of a later EDID block keeps the blocks before it | 2026-11-04 | Active | security |
| [F206](decisions/F206.md) | A file vault grows by itself: no size to choose, 32 KiB steps, up to 128 MiB as long as it fits in memory | 2026-11-04 | Active | security |
| [F207](decisions/F207.md) | A KeePass file keeps its entry history and recycle bin, written back and never shown | 2026-11-04 | Active | security |
| [F208](decisions/F208.md) | A file vault takes a key file, and a KeePassXC file protected by a YubiKey opens from that key's typed secret | 2026-11-04 | Active | security |
| [F209](decisions/F209.md) | Check a signer carries the step card's touch-target exception | 2026-11-04 | Active | design |
| [F210](decisions/F210.md) | The fuzz run before a release may run on GitHub Actions, started by hand, under F183's limits | 2026-11-04 | Active | repository |
| [F211](decisions/F211.md) | The QR vault's judges: PyCA's primitives, sq reads only and shares Argon2, rPGP the only outside sealer | 2026-11-04 | Active | crypto |
| [F212](decisions/F212.md) | GitHub's cache may hold compiled dependencies, never Faraday's own crates, written only by a push to main | 2026-11-04 | Active | repository |
| [F213](decisions/F213.md) | quick-xml is a library the manifest names that is not established | 2026-11-05 | Active | security |
| [F214](decisions/F214.md) | A Spec holds the design and whether it is built, and carries no work list | 2026-11-05 | Active | repository |
| [F215](decisions/F215.md) | A GPG key's paperkey has a code, for moving the key to a vault on another machine | 2026-11-05 | Active | QR |
| [F216](decisions/F216.md) | On the Pi, "card or stick" where the app says "stick" | 2026-11-05 | Amended by F235 | stick |
| [F217](history/decisions/F217.md) | A QR vault is a third form of a seed in the backup, with Open to test before the lock | 2026-11-05 | Withdrawn | backup |
| [F218](decisions/F218.md) | The list of what was plugged in at start can be dismissed, by its × or Escape, never by a held device | 2026-11-05 | Active | security |
| [F219](decisions/F219.md) | KEF, Krux's encrypted code, enters Faraday, all twelve versions read and written, checked against Krux's code and not on a device | 2026-11-05 | Active, the crate built and the screens not | crypto |
| [F220](decisions/F220.md) | What a cosigner's partial MuSig2 signature proves on the device | 2026-11-05 | Active | security |
| [F221](decisions/F221.md) | Two more kernel patches of Faraday's own on the Pi: a 32-bit framebuffer with red first, and vc4's output polling stopped on unbind | 2026-11-06 | Active | security |
| [F222](decisions/F222.md) | On a Pi's HDMI screen, fades start off | 2026-11-07 | Active | design |
| [F223](history/decisions/F223.md) | On the small panel off mainnet, the mode pill stands on the top bar's second line | 2026-11-08 | Replaced by F246 | design |
| [F224](decisions/F224.md) | A kernel patch of Faraday's own on the Pi: the bootloader's seed is not credited unless the command line says to | 2026-11-08 | Active | security |
| [F225](history/decisions/F225.md) | The open checklist card's header at 1024 × 600 is held at eight under the touch-target floor | 2026-11-08 | Withdrawn | design |
| [F226](decisions/F226.md) | A QR vault is made and opened in a tool in Tools, never kept open, and a file vault's item can be saved as one | 2026-11-08 | Active | security |
| [F227](decisions/F227.md) | The nonce check remembers the signatures it checked in an open vault, and refuses a nonce one key used before on another message | 2026-11-09 | Active, built | security |
| [F228](decisions/F228.md) | Core Lightning's hsm_secret enters Faraday as a source of Lightning node key, plain or encrypted, checked against Core Lightning's code and not on the device | 2026-11-09 | Active, built | keys |
| [F229](decisions/F229.md) | A Bitcoin Core descriptor wallet's wallet.dat is read, never written, and its records checked again, through a SQLite reader of Faraday's own | 2026-11-09 | Active, built: the SQLite reader, the wallet reader, the cross-check against Bitcoin Core and the Read a Bitcoin Core wallet tool | wallets |
| [F230](decisions/F230.md) | An OpenTimestamps proof is checked on the device against a block header the person gives, checked against python-opentimestamps | 2026-11-09 | Active, built: the reader, the cross-check against python-opentimestamps and the Check a timestamp tool | signing |
| [F231](decisions/F231.md) | A passphrase record belongs to exactly the key it names: saving another passphrase of one seed asks nothing, and only codex32 shares of one set are grouped | 2026-11-09 | Active | security |
| [F232](decisions/F232.md) | A BOLT 12 offer is read in Decode a Lightning invoice, read only, through a reader of Faraday's own, checked against Core Lightning's code | 2026-11-10 | Active | formats |
| [F233](decisions/F233.md) | Compare two PSBTs lists their differences without judging them; a fee bump is said only on Faraday's own rule, failing closed | 2026-11-10 | Active, built | signing |
| [F234](decisions/F234.md) | Work kept out of the app is held by reachability, not by a cargo feature | 2026-11-10 | Active | wallets |
| [F235](decisions/F235.md) | The stick is shown in Files, and the stick visit is no destination | 2026-11-10 | Active | stick |
| [F236](decisions/F236.md) | The online apps' Send text tool shows any text as a QR code except a secret, by a rule of Faraday's own that fails closed; bare hex is let through | 2026-11-10 | Active | QR |
| [F237](decisions/F237.md) | Transfer calls Read the words and does not offer it; the screens_not count is 23 | 2026-11-10 | Active | crypto |
| [F238](decisions/F238.md) | Named costly operations are held by an instruction count in the check: more than 25% over fails, a count is raised only in a commit that says why; a stated cost says how it was measured | 2026-11-10 | Active | speed |
| [F239](decisions/F239.md) | A release's cross-check may be made of runs at one fresh seed, the later ones running only the areas whose own inputs changed | 2026-11-10 | Active | repository |
| [F240](decisions/F240.md) | Make a new wallet asks who signs, then a single key's backup: Words (BIP-39) or Shares (codex32) on a Backup card, New key making the key in that form | 2026-11-10 | Active | wallets |
| [F241](decisions/F241.md) | A release's fuzz run may be made of two runs, the later one running only the targets whose corpus reaches what changed | 2026-11-11 | Active | repository |
| [F242](decisions/F242.md) | Change at an address index of 10,000 or more is shown as change with a Caution, not as verified change; signing stays possible | 2026-11-11 | Active | signing |
| [F243](decisions/F243.md) | One targeted check may run on GitHub Actions, started by hand: one crate's tests or one lint recipe, under F183's limits | 2026-11-11 | Active | repository |
| [F244](decisions/F244.md) | Keep data refuses the same build only when the stick's boot partition is already exactly the release's | 2026-11-11 | Active | starter |
| [F245](decisions/F245.md) | The Argon QR Vault is built as a test tool outside Faraday's apps: a library, a command line and a desktop app in `aqrv/`, test data only, into the app only after an outside review | 2026-11-11 | Active | standards |
| [F246](decisions/F246.md) | Off mainnet, the mode pill stands at the end of the top bar's first line, and a long name steps down, then wraps | 2026-11-11 | Active | design |
| [F248](decisions/F248.md) | A Rust file has no line limit; code is split by function | 2026-11-12 | Active | repository |
