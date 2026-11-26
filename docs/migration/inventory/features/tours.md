# Tours

The tours are scripted walks through the app's screens, run by the snapshot
tool (`faraday/shells/snapshot/`) to render PNGs of each step. They are test
drivers, not product screens: no person using the device or a wallet sees
them. Developers start one by name on the tool's command line (see the tour
runner); the default tour runs when no name is given. What matters for a
rewrite is which flows each tour walks, the states it sets up, the fixtures it
reads, and the few conditions under which it fails.

Paths below are under `faraday/shells/snapshot/src/` unless written in full.
The registry is `TOURS` in `faraday/shells/snapshot/src/tours/mod.rs:102-229`:
95 named tours, each `TourFn::Plain` or `TourFn::Costly` (`Costly` takes a
`&Compute` token; `faraday/shells/snapshot/src/tours/mod.rs:93`). Two more
tours are not in the registry: `default_tour` and `boot_tour`.

## Screens and sheets

Every tour runs at the `WxH@DPI` given on the command line unless noted
(only `components` and `dice5` fix or cap a size, and `host` fixes its
panels). The theme comes from `FARADAY_SNAP_THEME` unless noted. "Costly"
is the registry's kind. Every "defined at" line was checked against the
checkout; all files are in `faraday/shells/snapshot/src/tours/`.

### Boot, restore, add a key, cancel, busy, bands

| tour | kind | what it walks, in order | sizes / themes | defined at |
|---|---|---|---|---|
| band | Plain | Loads one test key (Bacon); adds the spending kit's unsigned PSBT as "spend.psbt"; Home shows only the PSBT waiting (band-home-psbt-only); adds a locked kdbx vault "work.kdbx" (`inbox_file_at` index 1, `faraday/shells/snapshot/src/tours/band.rs:32`); opens Restore wallet at the Kind step; shots of Home, Tools (Catalog), Check and Wallets as the Waiting band shows them | run size | `faraday/shells/snapshot/src/tours/band.rs:9` |
| again | Plain | Copies wallets from the stick; loads "savings-wallet.txt" (watch-only), shot; Backup checklist with the paper + vault-software preset, public files; restored Files box with the test vault; a 1-key wallet restored from test key 2, Backup, checklist; opens the test vault with its passphrase (vault speed 180); saves the wallet into the vault; Wallets chip list | run size | `faraday/shells/snapshot/src/tours/again.rs:12` |
| board | Costly | Loads the kit; keys 1 and 2; Savings Backup with the Paper + Vault board; Seed 1 placed on Paper 1, forms, LetGo; shares on Paper 2 and in the Vault; software; vault on a stick, a second vault added and removed, seed on the stick's Files; dark then light on the same page, then the start theme restored (`faraday/shells/snapshot/src/tours/board.rs:74-90`); an empty stick; a vault at no place; compact-only branches (list sheet, vault keeps, seed menu); the Plan page; naming a place with a vault open | run size; compact branches chosen by `is_compact()` (`faraday/shells/snapshot/src/tours/board.rs:~77, ~99, ~193`) | `faraday/shells/snapshot/src/tours/board.rs:15` |
| boardunsorted | Costly | Loads the kit; adds an "Unsorted" wsh(multi) wallet made from a sortedmulti descriptor; its backup board and list; then the MuSig2 kit wallet's board and list | run size | `faraday/shells/snapshot/src/tours/board.rs:153` |
| bootplace | Plain | Memory 15000 MiB, clock, Set time kept; one boot stick "FARADAY" (boot, serial 0x12345678, no files); Catalog GPG tile (GpgTile sheet answered MakeGpg), Continue (shot 1-insert); `UseBoot`, `Tap` (shot 2-bound); prints a line on stdout (`faraday/shells/snapshot/src/tours/boot_place.rs:6-50`) | run size | `faraday/shells/snapshot/src/tours/boot_place.rs:6` |
| boot (not in registry) | Plain-shaped, called only from the default tour (`faraday/shells/snapshot/src/tours/default.rs:~1511`) | Lock; boot stick put in, pulled, key typed; Files raises the import list (not-clean, stick); Later leaves Home with the import card; pulling leaves the list; put back; Import reads the stick; passphrase sheet; a wrong passphrase stays; Later and reopen; the right passphrase loads (boot-unlock, and boot-load-sheet if shown); on Mainnet the vault's public sample key is refused over Wallets with `Sheet::Published` (an `Err` if not, `faraday/shells/snapshot/src/tours/boot.rs:153-164`) and the tour continues; imported wallets and keys (at least 11, `faraday/shells/snapshot/src/tours/boot.rs:174`); a 2-of-3 made over test key 1 and two new keys saved into the vault; BIP-85 password; silent payments address; Explore; Lightning; Tools convert of an xpub; Lock; Start empty; Restore kind and sources | run size; the Mainnet branch only with `--network mainnet` | `faraday/shells/snapshot/src/tours/boot.rs:27` |
| addkey | Plain | Wallets start, Make a key (Start, Others); Entry words in two halves, EntryAdd; Tools: Entry, test key 2 typed, EntryAdd; Tools search "theme"; codex32 tile; Scan a seed: the unsigned spending PSBT delivered as Scanned | run size | `faraday/shells/snapshot/src/tours/add_key.rs:13` |
| cancel | Plain | Three keys; Make a new wallet Multi, path tip; Cancel before the sheet; keys assigned, Places backup Paper; Cancel sheet; CancelFlowNow to Wallets; New key from Make a new wallet with rolls, Cancel asks, Discard; a lone New key from Home with rolls, Cancel dialog | run size | `faraday/shells/snapshot/src/tours/cancel.rs:11` |
| busy | Plain | `demo_session`; Samples pressed with a wallet ticked; a busy job without a count shown moving 700 ms later; busy 2 of 5 (no tick); job ended | run size | `faraday/shells/snapshot/src/tours/busy.rs:12` |

### Check, Tools, convert, codex32, create

| tour | kind | what it walks, in order | sizes / themes | defined at |
|---|---|---|---|---|
| check | Plain | Check list; Backup row; Dice row; Files beside the pane (Check and Tools): wallet, text, PSBT, loaded wallet; Check an address found; Message at a path (sign a message, then its address). Resets the session mid-tour with `testkit::session()` (`faraday/shells/snapshot/src/tours/check.rs:~78, ~126`) | run size and theme | `faraday/shells/snapshot/src/tours/check.rs:9` |
| check-address | Plain | Empty; typed load-first; typed found (spending wallet index 2); refused ("hello"); address list from addresses.txt; scanner with a camera frame; scan read; file row with the scanner beside; guide beside the scanner | run size | `faraday/shells/snapshot/src/tours/check_address.rs:17` |
| convert | Plain | Convert a seed: From (empty, with the open vault's key), To, xprv masked then shown, Check wrong then right, Save, SecretUnprotected warning; words and SeedQR masked then shown; Seed XOR parts (settings, make, shown, check part, save set and warning); Explore way-in; SLIP-39 k of n (settings, groups, result, check share); aezeed From, entry, result; SLIP-39 set Add, Replace, Rotate (limit, destroy, shape), result | run size | `faraday/shells/snapshot/src/tours/convert.rs:25` (`slip_tour` ~139, `aezeed_tour` ~181, `set_tour` ~230) |
| chartedit | Costly | Chart: lost mark on a place's word line; exposed mark on place 1 (both shots taken only if the node is found, `faraday/shells/snapshot/src/tours/chart_edit.rs:87, 95`); marked card (one-key wallet, phrase vault); Backups place sheet; 2-of-3 with a cosigner holder named (Alice's Coldcard) and its key; holder key sheet | the compact panel shows the chart on its own page | `faraday/shells/snapshot/src/tours/chart_edit.rs:15` |
| createfrom | Costly | Create: multisig Keys with the vault locked, unlock (an `Err` if unlock does not return to Create, `faraday/shells/snapshot/src/tours/create_from.rs:69-71`), vault open; slot from a vault key; New wallet from an open multisig; Quorum; BIP 87 path on and off; slot cleared; fresh single key Check path, Path change; fresh single key dice (12 words, partial rolls, full, typed) | compact vs wide (`faraday/shells/snapshot/src/tours/create_from.rs:~51-57`) | `faraday/shells/snapshot/src/tours/create_from.rs:18` |
| copy | Costly | Copy: backup as paper; checklist; templates; reveal; check with no camera; typing; camera listed; scan differs; scan matched | run size | `faraday/shells/snapshot/src/tours/copy.rs:9` |
| core-wallet | Plain | Read a Bitcoin Core wallet: empty inbox; wallet.dat (vector `tools/vectors/core-wallet/encrypted.dat`); result after passphrase "faraday"; scrolled to the pairs | run size | `faraday/shells/snapshot/src/tours/core_wallet.rs:14` |
| codex32 | Plain | Make codex32 (BIP 93 vector 2 "name", 2 of 3): Start; scheme Other, then 2 of 3; identifier; entropy dice; 20-face dice; share A rolled; panels; cell checksum, lookup, dice; share C; sheet tabs 0 to 5; string typed with a differing box; Codex32QR; Skip six times; made; Check; Lock; then `typed_in` (Add a key codex32 form, check, Make the rest, Add a share with vector 3 A, C, E); a USB receiver attached in one shot | run size; `worksheet` (`faraday/shells/snapshot/src/tours/codex32.rs:~88`) is also used by `themes` and `tooltips` | `faraday/shells/snapshot/src/tours/codex32.rs:112` |
| codex32form | Plain | Add a key codex32 form: empty; a share whose checksum fails; Suggest corrections ticked; proposal; retype; taken; already in; another set's share (the clash shot only if still on Entry, `faraday/shells/snapshot/src/tours/codex32_form.rs:~71`); Keep; Check holds; Check fails | run size | `faraday/shells/snapshot/src/tours/codex32_form.rs:39` |
| codex32correct | Costly | Correction screens drawn directly with `screen::preview`, not through the app (tick, pairs, retype, repairs, unreadable, from checksum, caution, refused, fingerprint corrected and repaired); vector 1 secret | `t.size()`, `t.dpi` | `faraday/shells/snapshot/src/tours/codex32_correct.rs:49` |
| codex32backup | Plain | A key with two strings of vector 2; set facts; wallet from key 0; backup checklist; templates; item list; typed back; Codex32QR; rotate marks and destroy; leave; a round after a lock: not here, type, refused. Uses a pushed `BackupFlow::with_plan` forced to `Step::Make` | run size | `faraday/shells/snapshot/src/tours/codex32_backup.rs:58` |
| createshares | Plain | Make a new wallet: Kind; Single key backup; a tooltip on "(codex32)" via `Event::Hover`, found from the drawn text boxes; Shares (codex32) keys; New key opens on Scheme; Cancel; More kinds; Taproot single key; Words on Keys; Other ways opens key cards (hosted) | run size | `faraday/shells/snapshot/src/tours/create_shares.rs:14` |
| createslots | Plain | Multisig Keys slots: no key; three keys held; one picked (others dimmed); all filled; slot 2 reopened with Change | run size | `faraday/shells/snapshot/src/tours/create_slots.rs:10` |
| components | Plain | Shared faraday-ui pieces, no app state: rest, focused, pressed (stick pill), menu popup. Draws on a fixed 720x900 canvas at 160 dpi (`faraday/shells/snapshot/src/tours/components.rs:17-29`) | fixed 720x900@160; scheme from `FARADAY_SNAP_THEME`, the default theme if unset or unknown (`faraday/shells/snapshot/src/tours/components.rs:33-38`) | `faraday/shells/snapshot/src/tours/components.rs:205` |
| compact | Plain | Small panel: Home; Wallets empty; stick sheet; pull; Entry; Create kind; keygen (12 words, source, 128 rolls with fixed xorshift seed `0x2545_f491`, Key, passphrase typed and selected with Shift+Left, hardware line, eye, Lock in); backup plan Paper + vault; Home loaded; Entry typing and SLIP-39; then `seeds_tour` | small panel size chosen by the caller | `faraday/shells/snapshot/src/tours/compact.rs:10` |

### The default tour and the desktop and device tours

| tour | kind | what it walks, in order (summary) | sizes / themes | defined at |
|---|---|---|---|---|
| default (no name) | Costly-shaped (takes `&Compute` and the medium) | Stick device check (only on the Stick medium; with `--sd-card` only home-empty is shot); Settings Set time; stick visit (file menu, delete sheet, copy, QR PNG read); Home pull sheet; Files; Wallets start; load the kit; Add a key typing (BIP-39, SLIP-39 share, Spanish); spend Savings 2-of-3 (transaction through finished, nonce check, hex, decode), a single-key spend, Taproot multisig, MuSig2 nonces and partials, and unconfirmed, false-partial and mended files; restore Savings from 2 shares; SeedQR scan; animated UR scan; Inheritance path; backup of Savings (paper, Paper + XOR, scan of a wrong copy, checklist, sheets, envelopes, PDF out); message sign and check; create a 2-of-3 with a dice key and a waiting slot; wallet QR decode; lock sheet; Not now; vault unlock (Load sheet), entries and a BIP-39-passphrase key; vault backup (PaperVault); GPG key in the vault and signing; TOTP; Secure Boot keys and image signing; Lock seals the vault; restart; new vault from dice; then the seeds, boot and spend tours (`faraday/shells/snapshot/src/tours/default.rs:~1510-1512`); Tools and Find; SLIP-39 plan; word lists; rolled words; keyed entry (Japanese, Korean, Chinese); silent payments wallet; MuSig2 create; idle warning; Settings and About; power sheet; Home foot; bar menu; New key cards hint; self-test forced to fail by a check `\|_\| false` (the shot is a fixed render, `faraday/shells/snapshot/src/tours/default.rs:1607-1613`) | run size; medium Stick or SdCard | `faraday/shells/snapshot/src/tours/default.rs:269` |
| pills | Plain | Delegates to `host_tour` | run size | `faraday/shells/snapshot/src/tours/desktop.rs:11` |
| demo | Plain | Online with demo samples (`testkit::demo_samples`): Entry of the sample seed "bacon", BIP-85 Value card, Lightning node key page. Sets `t.app.online` and `t.app.seed_line` directly (`faraday/shells/snapshot/src/tours/desktop.rs:~18-19`) | run size | `faraday/shells/snapshot/src/tours/desktop.rs:17` |
| modes | Plain | Home mode sheet in Practice; add-key sample chip, share step; demo passphrases; scan a sample; boot stick in; Samples sheet and scrollbar; Pull sheet; demo PSBT; backup copy check; Testnet Home and mode sheet; a mode change with an unsaved outbox file asks first; testnet passphrases; Tools; Mainnet Settings. Uses `faraday_samples::SAMPLES` (`faraday/shells/snapshot/src/tours/desktop.rs:~52`) | run size | `faraday/shells/snapshot/src/tours/desktop.rs:48` |
| devices | Plain | Receiver (Unifying, Bolt) marks on entry and Home; IP-KVM with Continue asking; VM with KVM (no way on); desktop before; desktop with a TV EDID; new USB touchscreen (tap, ignore); new USB mouse with the keyboard trusted; boot-time USB list on Home | run size | `faraday/shells/snapshot/src/tours/devices.rs:59` |
| toasts | Plain | Receiver let in as a pointer (toast); keyboard typing a wrong code | run size | `faraday/shells/snapshot/src/tours/devices.rs:171` |
| power | Plain | Home with a key; power menu (Lock, Restart, Power off); power sheet with Secure Boot on and off (sample maker and model strings) | run size | `faraday/shells/snapshot/src/tours/devices.rs:204` |
| osk | Plain | Receiver on Home; passphrase entry keyboard button; on-screen keyboard on Tools | run size | `faraday/shells/snapshot/src/tours/devices.rs:227` |
| scan | Plain | Home with a stick: pull sheet, camera; QR outlined (found, read); sidebar Scan of a sample testnet address (not on compact); viewfinder over Tools and Check | run size | `faraday/shells/snapshot/src/tours/devices.rs:249` |
| files-wide | Plain | Files with no stick: Inbox summary; Outbox; Stick tab waiting; stick in while a vault is open (Lock sheet put off) | run size (its doc says "wide"; nothing enforces it) | `faraday/shells/snapshot/src/tours/files_wide.rs:13` |
| first | Costly | Backups empty; single-key native SegWit wallet; Backups; Paper + vault plan; vault board with Vault 1's stick; templates; copy typed back; make the vault; save the vault into the backup (a 30-tick wait that breaks silently if never reached, `faraday/shells/snapshot/src/tours/first.rs:66-71`); envelopes; write to sticks; unlock the vault; checklist; done card and strip | run size | `faraday/shells/snapshot/src/tours/first.rs:17` |
| flow | Plain | GPG key tile; Where; Insert; which vault (a second stick with an unopened vault); confirm; bound; two empty sticks read alike; hold; vaults; create vault 2; lock and write; written; use another stick; second stick; done | run size | `faraday/shells/snapshot/src/tours/flow.rs:109` |
| flowbeside | Plain | A known vault opened once and locked; GPG tile; a second stick with another vault: new file beside the first, pick step | run size | `faraday/shells/snapshot/src/tours/flow.rs:63` |
| flowguide | Plain | Vaults, Create vault (closed), Guide opened, Guide closed | run size | `faraday/shells/snapshot/src/tours/flow.rs:252` |
| fromvault | Costly | Lock; boot stick (FARADAY) with a vault and the Savings PSBT; Import, pull, passphrase, unlock Load sheet; Wallets landing; Spending card, spend steps to Write; a 2-of-3 short-of-keys wallet and Signers; Scan a key, SeedQR of a missing key | run size | `faraday/shells/snapshot/src/tours/from_vault.rs:23` |
| dice5 | Plain | The five-dice roller alone: rest, tumbling, landed, both rolls with a struck pair, 20-faced dice; PNGs written directly (not via `Tour::shot`), each at most 520x560 (`faraday/shells/snapshot/src/tours/dice_five.rs:22-36`) | min(run size, 520x560) | `faraday/shells/snapshot/src/tours/dice_five.rs:50` |

### Search, hosts, flows, keys, Krux, vaults

| tour | kind | what it walks, in order | sizes / themes | defined at |
|---|---|---|---|---|
| glance | Costly | Test key 1; "Spending" wallet; wallet card with no plan (compact: At-a-glance, then 3 scrolled pages); backup Paper + vault, checklist; chart sheets (wallet, key 0, node 0, node 2, line 0); 2-of-3 "Savings" (key 2 with passphrase "glance") and cosigner key 3; vault create with passphrase "glance phrase", open, save wallet; vault item shots; plan and save; Place sheet in the vault | compact vs wide; Memory 15000; vault speed 180 | `faraday/shells/snapshot/src/tours/glance.rs:40` |
| gpg-restore | Plain | Clock; Tools GPG tile; Restore sheet; scanner frame and a non-key code; the flow on a public key at step 4; lines typed; a wrong line 2 gives an error; checked; Restore the key; the refusal under it (vectors `tools/vectors/paperkey`) | run size | `faraday/shells/snapshot/src/tours/gpg_restore.rs:23` |
| home-search | Plain | Home search bar scrolled into view (up to 3600 px); queries "send", "theme", "orange", "time", "quorum", "zzzz"; Start menu; stick box; Tools search "seal"; all seven menu pages; CreateWallet flow with search "sign" | run size | `faraday/shells/snapshot/src/tours/home_search.rs:10` |
| host | Plain | Desktop host in Practice with samples, boot stick in: Home, Size menu, samples sheet; Pi panel 268x358 dp (`faraday/shells/snapshot/src/tours/host.rs:~15`); 7-inch 964x565 dp (`faraday/shells/snapshot/src/tours/host.rs:~16`); boot-stick pull flash; stick pill; Add a key; new key dice; Create vault dice; codex32 shares (dark theme, host light); Testnet desktop and Pi panel | window from `t.size()` minus the host strip; panels fixed; theme set to dark inside codex32 | `faraday/shells/snapshot/src/tours/host.rs:161` |
| how | Plain | Spend top bar; ? opens the guide; Cryptography tab; scroll; Words row (glossary); guide closed; Settings with the guide; Spend backup tab | run size | `faraday/shells/snapshot/src/tours/how.rs:8` |
| inprogress | Plain | Memory and Clock; GPG tile; Continue; three keys; multisig wallet with three slots; Backup Paper; places; Settings; Home | run size | `faraday/shells/snapshot/src/tours/in_progress.rs:13` |
| inbox | Plain | Outbox PSBT plus five Inbox files (text, descriptor, a 4.2 MB vault, PNG, words); Select all; text sheet; unprotected; hold; words sheet; file menus | compact vs wide | `faraday/shells/snapshot/src/tours/inbox.rs:10` |
| invoice | Plain | Catalog "Decode a Lightning invoice": empty; COFFEE typed; decoded; scrolled; a spoiled checksum refused; BOLT #12 offer; a zero-amount offer refused | run size; keyboard hidden on a small panel | `faraday/shells/snapshot/src/tours/invoice.rs:26` |
| kef | Plain | Home, Scan; a code with count byte 100; Read again; a genuine Krux code (Krux's own AES-GCM code from `tools/vectors/kef/`, `faraday/shells/snapshot/src/tours/kef.rs:12-14`); passphrase sheet; "wrong"; failure | run size | `faraday/shells/snapshot/src/tours/kef.rs:16` |
| kef-ways | Plain | Home; KefTool Open; Add a key's Krux form | run size | `faraday/shells/snapshot/src/tours/kef.rs:43` |
| kef-menu | Plain | 15-word seed typed; key wallet; Seeds Make; backup; Board Customize; list; menu on the seed with the Krux row dimmed | compact vs wide | `faraday/shells/snapshot/src/tours/kef.rs:57` |
| kef-make | Costly | Test key 1 and "Spending"; Tools Krux code; Pick wallet; 15-word key (dimmed row); steps; passphrase twice; rounds 10000; label; Make on a worker (held); code card; Check; compare; Done; Another | run size; jobs run inline | `faraday/shells/snapshot/src/tours/kef.rs:81` |
| kef-step | Plain | Test seed 2, wallet, Paper backup; board: seed to Krux code, new stick, off paper; checklist item not made; form; options; Make; code card; Check; compare; checked | run size | `faraday/shells/snapshot/src/tours/kef.rs:168` |
| kept | Costly | Kit loaded; Backup Savings Paper; seed files at place 0; checklist; Templates; File; unprotected and hold; kept shot; vault.kdbx in the Inbox; open vault 0, unlock; Backup PaperVault; reveal; check; words typed | Memory 15000; vault speed 180 | `faraday/shells/snapshot/src/tours/kept.rs:22` |
| keypath | Plain | Files: taproot multisig "shared-wallet.txt" with a key not in it, its PSBT, a NUMS wallet; confirm sheet; hold; two keys typed; Wallets card; spend check at the transaction step | run size | `faraday/shells/snapshot/src/tours/key_path.rs:26` |
| keygen | Plain | Entropy fixed (0x11 bytes, `faraday/shells/snapshot/src/tours/keygen.rs:19-20`); Make a new wallet native segwit words form; New key start; length; 18 words; generator; dice "31415"; key; passphrase and Show words; word list "101" | run size | `faraday/shells/snapshot/src/tours/keygen.rs:17` |
| dicewords | Plain | New key 12 words, dice words; typing; 3 rolls, then 39 more; Words-so-far page; Guide open | run size | `faraday/shells/snapshot/src/tours/keygen.rs:79` |
| flipwords | Plain | New key 12 words, coin flips: 5, 119 more, last word; 4 checksum flips | run size | `faraday/shells/snapshot/src/tours/keygen.rs:127` |
| keys | Plain | Kit; the "abandon ... about" key; Wallets; Lock; sticks off; the key again; key wallet shapes (5 scroll steps if the width is under 700, else 2, `faraday/shells/snapshot/src/tours/keys.rs:37`); two keys; Home; one-key wallet; BIP-85 child seed result, load, record; CreateWallet over the child | by `t.size().0 < 700` | `faraday/shells/snapshot/src/tours/keys.rs:9` |
| lightning-cln | Plain | Test key 1; Lightning from WalletsStart; CLN node from the key; sample hsm_secret (`faraday/faraday-samples/data/hsm-secret-encrypted`, `faraday/shells/snapshot/src/tours/lightning_cln.rs:33`); passphrase "faraday1" wrong, then right; node id code | run size | `faraday/shells/snapshot/src/tours/lightning_cln.rs:13` |
| m3 | Plain | Three keys; Home; Settings motion toggle; power dialog; Cancel; multisig flow Cancel flow; dialog in High contrast dark and light; Set time typed with a zone; toast at 1000 ms; Tab focus ring | Cobalt default, then (monochrome, dark, high) and (cobalt, light, high) | `faraday/shells/snapshot/src/tours/m3.rs:15` |
| menu | Plain | Kit; keys 2 and 3; Savings backup Paper + vault; Customize; seed placed; compact: Add sheet, Add to, placed menu; wide: Add to menu, Move to, Tab, Enter, Down | compact vs wide | `faraday/shells/snapshot/src/tours/menu.rs:12` |
| multisig-check | Plain | Tools multisig check; scan the Savings descriptor; result; Check a seed (typed "zebra" words, passphrase step; cosigner key; ABOUT words not a key); a corrupted checksum refused; rescan; Add; hold; wallet sheet key block, tick keys | run size | `faraday/shells/snapshot/src/tours/multisig_check.rs:38` |
| nonce | Plain | Test kit spend; finished hex from test key 1; a cosigner copy with a flipped signature bit; Nonce check on the cosigner PSBT (no key) and end; key 2 loaded; recomputed; finished transaction; raw and hex; a damaged PSBT refused | run size | `faraday/shells/snapshot/src/tours/nonce.rs:68` |
| oldparts | Costly | Key 1 to Savings; vault create "old parts phrase", open; key split into three Seed XOR parts, placed; save; re-split to two (replace); save; chart board; kept old part sheet | Memory 15000; compact uses the Glance page | `faraday/shells/snapshot/src/tours/old_parts.rs:12` |
| open-tools | Plain | Decode transaction, Check signed message, Explore a key, Child seeds, Vanity, Silent payments (no key); four files in the Inbox; decode list and result; typed field open, refused, long; paste too large; message list; Hashes; Encodings; Descriptor checksum; Convert an xpub; Miniscript compiler; Units (one amount, tabs, three and six) | run size | `faraday/shells/snapshot/src/tours/open_tools.rs:27` |
| ots | Plain | Inbox: an .ots proof, its file, a block header hex (`tools/vectors/opentimestamps`); Tools Check a timestamp; proof; file; header; result | run size | `faraday/shells/snapshot/src/tours/ots_check.rs:18` |
| panes | Plain | Test vault and kit in the Inbox; Wallets (wallet 2); Files; Vaults (vault 0 open); Backups; Settings sections 4 (Appearance), 5 (Display), 1 (Lock), 6 (remembered); closed | run size | `faraday/shells/snapshot/src/tours/panes.rs:12` |
| parts | Plain | Keys 1 to 3; CreateWallet multi; keys; Backup; Customize; Partial descriptions sheet; the whole taken off every place; partials again; removed warning | run size | `faraday/shells/snapshot/src/tours/partials.rs:11` |
| passseeds | Costly | Trio vault (passphrase "trio phrase") with keys 1 to 3, "Spare", and a 2-of-3 "Trio" whose third key is a seed with passphrase "kept apart"; Vaults, open, unlock Load sheet; ticked keys loaded; wallet 0; passphrase-key list; test key 3; "wrong" refused; "kept apart" loads it (3 keys) | Memory 15000; vault speed 180 | `faraday/shells/snapshot/src/tours/pass_seeds.rs:22` |

### Spend, backup, vault, site, themes

| tour | kind | what it walks, in order | sizes / themes | defined at |
|---|---|---|---|---|
| spend | Plain | Lock; spend page empty; map; More; Next; Unsure twice; stick visit (vault.kdbx, savings-unsigned.psbt); vault pick, unlock, LoadChosen; Savings chosen from the PSBT; Check, Write, Bring, UsePsbt; transaction, signers, SignHere, sign, collect, send, finished; vanity address: key 1, prefix "qq", Start, up to 4000 ticks of 100 ms until found, an `Err` if not found (`faraday/shells/snapshot/src/tours/spend.rs:112-122`) | run size | `faraday/shells/snapshot/src/tours/spend.rs:9` |
| spend-pane | Plain | Four PSBTs from the spending kit (plain, OP_RETURN, 62-byte and 74-byte output scripts); wallet loaded; key 1 typed; each: Start spend, steps to SIGN, sign pane, "Your choices", SignHere, finish | run size | `faraday/shells/snapshot/src/tours/spend_pane.rs:22` |
| register | Plain | Savings PSBTs (with xpubs, bare, one whose third key's origin is replaced); Register this wallet refused, cancelled (wallet 2); wallet 0: Register blocked, sheet, field typing, held, registered; remove wallet 0; wallet 1: the no-xpubs case | run size | `faraday/shells/snapshot/src/tours/register.rs:23` |
| signer | Plain | Catalog Check a signer, empty; Files: sent, signed, device-added, regtest-sent, random-nonce, bump-original and bump-replacement .psbt, bump-original.tx (`tools/vectors/psbt/`); Sent and Returned picks; findings; Swap; not loaded; fee bump; a finished .tx as Returned; closed differs card | run size | `faraday/shells/snapshot/src/tours/signer.rs:20` |
| plan | Costly | Spending: board read-only, Paper and vault preset, Vault 1, LetGo, Plan tab, checklist foot. Savings: Paper, shares, What is a share?, map, checklist. A 3-of-5 "Five": shares, Omit 2. Spending re-backed with a vault; Savings with vaults; blank-template outbox files written to the output folder | run size | `faraday/shells/snapshot/src/tours/plan.rs:17` |
| public | Costly | Load the kit; signed message (sign, code, PNG); silent payments record; Create CKind 4 with key 1 and devices 1 and 2; backup checklist with software and description files at Place 1; Public files row; multisig config and BSMS codes; More options; a GPG key made in the test vault (a test name and a placeholder email address); PNG outbox items written | run size (small-panel scroll at `faraday/shells/snapshot/src/tours/public.rs:~155`) | `faraday/shells/snapshot/src/tours/public.rs:21` |
| qr | Costly | Wallet files copied; key 1: Explore extended key code, Lightning node key and node-id code; wallet card receive address code; test vault with a recovery sheet and TOTP opened; vault sheet code; TOTP as QR and Secret sheet; GPG paperkey code and Secret sheet | run size | `faraday/shells/snapshot/src/tours/qr.rs:15` |
| quick | Plain | Wallet from test seed 1, Check, Back up; Quick paper backup (Write, Show, Blank template, Seed QR, Code added, Wrote, Check my copy typed, Next); Krux-style code form (rounds 10000, passphrase twice, Make, Check, Compare); checklist; end card with the first receive address | run size | `faraday/shells/snapshot/src/tours/quick.rs:14` |
| randomness | Plain | Pointer and keys offered; getrandom entropy event; one camera; Settings Randomness row, Add randomness: Camera source with a 64x64 luma frame, Dice with ten rolls then buttons, Cards, Pointer and keys, Back; New key SLIP-39: 20 words, Coins, 128 flips, the at-a-press frame, Add randomness from the menu | run size | `faraday/shells/snapshot/src/tours/randomness.rs:16` |
| restore | Plain | Test network; copy wallets; sticks off; Restore Kind 4, Quorum, description savings-wallet.txt; Seeds: key 1 matched, a made-up seed refused, the other two Not here; Make, Check, Done; one key from its seed alone: Kind, Seeds, key 2; Account stepper, Custom path empty (bad path), Standard, Make | run size; `testkit::net()` (`faraday/shells/snapshot/src/tours/restore.rs:16`) | `faraday/shells/snapshot/src/tours/restore.rs:13` |
| roller | Plain | Demo mode (`into_demo`); Vault Create, Dice; rest, Per Five, roll, tumbling (300 ms), landed (700 ms), Stop, Count "12", To end, Reduce motion, rolled to the end; Use | run size; demo only | `faraday/shells/snapshot/src/tours/roller.rs:10` |
| rowdetails | Plain | Session with a key (Bacon) and the spending wallet; two inbox files; Tools, Wallets, Home, Files rows opened; file details with and without a stick; reset with notes.txt; tick off and on; Vaults: work.kdbx locked card and details | its doc says a non-compact window; nothing enforces it | `faraday/shells/snapshot/src/tours/row_details.rs:11` |
| seed-pick | Plain | "My seed makes my wallet" tile: no key, two keys (Savings, Spare), Other forms (SeedMakesWallet), SeedCheck Type | run size | `faraday/shells/snapshot/src/tours/seed_pick.rs:10` |
| seedfile | Costly | Key 1, Backup paper vault preset; seed file step (no vault): file sheet, scrolled; test vault unlocked; Vault step, second key, seed file sheet with the SeedQR form, Unprotected, Hold; compact toggled; SeedQR PNGs written | run size; small-panel scroll `faraday/shells/snapshot/src/tours/seedfile.rs:~41, ~86` | `faraday/shells/snapshot/src/tours/seedfile.rs:18` |
| seeds | Plain | Lock; sticks off; Restore 2-of-3 with no description: typed seeds, cosigner 0 xpub typed, shape (5 scroll steps if the width is under 700, else 2, `faraday/shells/snapshot/src/tours/seeds.rs:70`), Make, check; Lock; Spend backup tab: StartOver, MAP, intake, Type words, Scan, Cancel; No money: guess, one seed and Everything, passphrase, Taproot step; Lock | width under 700 | `faraday/shells/snapshot/src/tours/seeds.rs:15` |
| silent | Costly | Key 1; Silent payments record; DNS "matt" at "example.com"; QR; a bad domain " x"; three restarts: silent-unchecked (an honest record as a file, no key), silent-differs (scan key replaced, key 1 loaded); Load and Pick wallet; silent page; a made key with a silent wallet, silent-held | run size | `faraday/shells/snapshot/src/tours/silent.rs:14` |
| std | Plain | Key 1; Make a new wallet on Kind; single-key Check, vault question, Back up; keys 2 and 3; 2-of-3 board read-only; Software on and off; Customize; Whole off Place 3 with Share 1 dropped there (sheet) | run size | `faraday/shells/snapshot/src/tours/standard.rs:14` |
| themes | Plain | Kit; keys 2 and 3; Savings board with seed 0 on Paper 1, seed 1 chip; for every `ThemeName::ALL` x `Mode::ALL` at Standard, plus Cobalt at Medium and High in both modes: Home, Wallets, board, Settings Appearance (section 4) scrolled to the chip; then the codex32 worksheet in each scheme; Settings Appearance default, Reduce motion row, Frost Dark Standard, Bitcoin Orange Light Medium, Cobalt Dark High | all schemes (`faraday/shells/snapshot/src/tours/themes.rs:10-26`) | `faraday/shells/snapshot/src/tours/themes.rs:28` |
| tooltips | Plain | Savings board with seed 0 on Paper 1; Cobalt Standard Light and Dark: Home, a tooltip on Learn (Guide), board, a tooltip on the Off cross of Paper 1 seed 0; the codex32 worksheet eye tooltip in each mode | Cobalt Standard, Light and Dark | `faraday/shells/snapshot/src/tours/tooltips.rs:25` |
| transfer | Plain | Downloads folder filled (photo 300 KiB, tarball 100 KiB, descriptor, unsigned PSBT); the online app in Practice on the test network (`demo_samples`); Transfer: a pasted block header shown as a code; pasted seed words refused; Send PSBT as codes, a jpg refused; Receive: a 640x480 synthetic frame; BBQr parts of a descriptor and of seed words; Saved; Home; WalletsStart | run size; `testkit::net()` (`faraday/shells/snapshot/src/tours/transfer.rs:32`) | `faraday/shells/snapshot/src/tours/transfer.rs:9` |
| tour (the site tour) | Costly | Website pictures, five stops: codex32 Split 2-of-3 "name" on dice (frames at characters 4, 10, 16, 22); vault "Passwords" on dice (frames after rolls 1, 3, 5, 6); Settings Appearance in the default theme and Forest Dark; 2-of-3 with guide key 3dca6805, board customized (vault at Place 1, seed 1 SeedQR); sign a 0.012 BTC native-segwit "payment.psbt" and StartSpend. Its doc says it runs on Mainnet (`faraday/shells/snapshot/src/tours/site_tour.rs:2`); the code does not select the network: it follows `--network`, default the test network. A sixth stop belongs to the Starter and is not here | run size | `faraday/shells/snapshot/src/tours/site_tour.rs:20` |
| vaultshow | Plain | Test vault unlocked; key 0 masked, eye-revealed SeedQR, Save to a stick; loaded key; Wallets category and the Spending wallet's Save to a stick; search "sav" matches, "savx" matches none; a new entry without a title refused | run size | `faraday/shells/snapshot/src/tours/vault_show.rs:13` |
| vaultsheets | Costly | Five vaults restored (Alpha with a seed and an entry, Beta empty, Newer with a changed format value, Gamma, Saved with an extra Tags element), unlocked, Gamma locked; seed page Remove buttons, Trim dialog, Copy sheet, Move sheet and dialog, Copy from an unread entry, keep sheet (BIP-85 child plus codex32 set), secret sheet; keyed vault menu, Key file | its doc says a wide panel; nothing enforces it | `faraday/shells/snapshot/src/tours/vault_sheets.rs:181` |
| vaultstates | Plain | Wrong passphrase on Unlock; keyed vault Unlock buttons, Unlock from a flow (Cancel beside it), key-file dialog, wrong passphrase with a key file; Create vault key file (button, dialog, chip, drop); a vault saved by another program; a vault with history opens with no dialog; a vault older than the copy open (shot only if reached); unreadable items and a read-only vault; Create vault file name; Change passphrase and Memory; Lock sheet "cannot be sealed" (a filler record sized to exceed the slot, `faraday/shells/snapshot/src/tours/vault_states.rs:~335-343`); locked with changes lost. Foreign vectors from `tools/vectors/kdbx/`, passphrase "kdbx vector passphrase" | run size; vault speeds 100 and 180 | `faraday/shells/snapshot/src/tours/vault_states.rs:254` |
| vaultspair | Costly | No vault file; one vault open with the first key and a wallet saved; two open; a read-only newer vault; 14 vault files, none open | run size | `faraday/shells/snapshot/src/tours/vaults_pair.rs:86` |
| vaultway | Plain | Tools GPG tile with no vault; Create a vault for it; with the test vault locked; Unlock for it; unlocked, back to GPG keys. Compact panels find the tile by typing "gpg" after Escape; wide ones scroll to it (`faraday/shells/snapshot/src/tours/vaultway.rs:25-28`) | run size; vault speed 180 | `faraday/shells/snapshot/src/tours/vaultway.rs:12` |
| visit | Costly | Files with a 20-file wallet backup in the Outbox and a secret file, sticks in; wide: Outbox top, wheel to the end, both ends, Outbox none; compact: pages through the Outbox; an unprotected secret written to a stick; the clean process with a PSBT on a stick: Import and sign; the not-clean state (a words file in the Inbox, Write asks to lock); the same file in Faraday and on the stick (copied in, written; `faraday/shells/snapshot/src/tours/visit.rs:~249` writes a real file into the kit folder) | compact vs wide (`faraday/shells/snapshot/src/tours/visit.rs:~69, ~172, ~202, ~263`) | `faraday/shells/snapshot/src/tours/visit.rs:13` |
| walletflow | Costly | Make a new wallet, Kind, Keys with a vault-on-a-stick row; which vault; Bring in with sticks; vault unlocked from Vaults; key made, slot set; Check, Back up Paper vault preset; checklist, new vault, Unlock, save; Write to sticks; lock, reopen from kept boxes; change the plan board with the vault chip held; each stick by place written; a new single-key wallet, rolls and words; multisig Key 2 xpub typing and New words; sticks other than "TESTSTICK" replaced by fake empty sticks with id "b" or "c" (`faraday/shells/snapshot/src/tours/wallet_flow.rs:~163-167`) | run size | `faraday/shells/snapshot/src/tours/wallet_flow.rs:30` |

### Helpers that are not tours

- `faraday/shells/snapshot/src/tours/guide/mod.rs`: `stick_session` (12),
  `key_session` (27), `demo_session` (37: Testnet, samples, desktop modes),
  `single_kit` (53), `files_in` (70), `DICE_WORDS` (~92), `kind_at` (96).
- `faraday/shells/snapshot/src/tours/default.rs`: `pass_quiz`, `dice_key`, `word_lists`, `rolled_words`,
  `keyed_entry`, `silent_wallet`.
- `faraday/shells/snapshot/src/tours/desktop.rs`: `demo_psbt`, `demo_passphrases`, `testnet_passphrases`,
  `demo_copy_check`.
- `faraday/shells/snapshot/src/tours/devices.rs`: `receiver`, `kvm`. `faraday/shells/snapshot/src/tours/first.rs`: `write_sticks`.
  `faraday/shells/snapshot/src/tours/flow.rs`: `opened_once`.
- `faraday/shells/snapshot/src/tours/signer.rs:~12` `vector`; `faraday/shells/snapshot/src/tours/seedfile.rs:~117` `seedqr_on_paper`;
  `faraday/shells/snapshot/src/tours/site_tour.rs:~76` `hold_view`; `faraday/shells/snapshot/src/tours/vault_sheets.rs:~99` `setup`,
  `:~227` `more_menu`, `:~293` `secret_sheet`; `faraday/shells/snapshot/src/tours/register.rs:~14`
  `open_transaction`; `faraday/shells/snapshot/src/tours/spend_pane.rs:~14` `recipient`;
  `faraday/shells/snapshot/src/tours/codex32.rs` `worksheet` ~88, `twenty_faced` ~53, `typed_in` ~257,
  `checksum_end` ~299, `pan_to_end` ~309.

## Actions

- A tour is chosen by its registry name as the last argument of the snapshot
  tool; with no name the default tour runs (see the tour runner).
- Inside a tour, every step is an `Action` pressed through `Tour::press`, an
  `Event` (keys, hover, display, entropy) or a `StorageEvent`; the tour then
  takes a shot. Tours also read the app's internals through accessors
  (`t.app.session`, `vaults()`, `sheet`, `flow()`) and set some fields
  directly: `t.app.session` (`faraday/shells/snapshot/src/tours/check.rs:~78,
  ~126`), `t.app.online` and `t.app.seed_line`
  (`faraday/shells/snapshot/src/tours/desktop.rs:~18-19`).
- `codex32correct` draws screens with `screen::preview`, outside the app's
  event path.

## Effects

- PNGs `NN-name.png` in the output folder, one per shot (through the tour
  runner's `save`); `dice5` and `components` write their own PNGs.
- `plan` writes blank-template outbox files to the output folder; `public`
  writes PNG outbox items; `seedfile` writes SeedQR PNGs (through the app's
  `Print` and storage commands, answered by the runner).
- `visit` writes a real file into the kit folder
  (`faraday/shells/snapshot/src/tours/visit.rs:~249`).
- `bootplace` prints a line on stdout.
- No tour touches real sticks, cameras or a disk process: storage, entropy
  and cameras are answered or faked by the runner.

## State and lock

- A tour holds no state of its own beyond the `Tour` driver (see the tour
  runner); the app's state lives in the `Faraday` it drives.
- A restart inside a tour (Lock, Restart, an `Exit` command) builds a fresh
  `Faraday::new()`; the session, keys, vaults and sheets are gone, and only
  settings, the scheme and the last saved boxes carry over.
- Secrets the tours hold are test data only: the test seeds of
  `faraday_core::testkit`, the BIP-39 and BIP-93 vectors, the test vault's
  passphrase and items, and the fixed passphrases named above ("glance",
  "glance phrase", "trio phrase", "kept apart", "old parts phrase",
  "faraday", "faraday1", "kdbx vector passphrase").
- Fixed values set up by tours: storage `Memory{available_mib: 15_000}`,
  `Clock{unix_secs: 1_791_000_000}`; entropy 32 bytes of `0x5a ^ n` (or
  `0x11` in keygen, `faraday/shells/snapshot/src/tours/keygen.rs:19-20`); a
  fingerprint `[0x5d,0x38,0x83,0x76]`
  (`faraday/shells/snapshot/src/tours/default.rs:~745`); vault speeds 100
  and 180 through `testkit::vault_speed`.

## Reach

- Run by the tour runner (`faraday/shells/snapshot/src/main.rs`,
  `faraday/shells/snapshot/src/tours/mod.rs:238`, `run_named`).
- The default tour calls `seeds_tour`, `boot_tour` and `spend_tour`
  (`faraday/shells/snapshot/src/tours/default.rs:~1510-1512`); `pills` calls
  `host_tour`; `compact` calls `seeds_tour`; the codex32 `worksheet` helper is
  used by `themes` and `tooltips`.
- Between them the tours open almost every product feature: Home, Tools,
  Check, Wallets, Files, Vaults, Backups, Settings, the spend flow, key
  making, codex32, SLIP-39, Seed XOR, aezeed, Krux codes, GPG, TOTP,
  Lightning, silent payments, BIP-85, registration, the signer check,
  OpenTimestamps, transfers of the desktop app, devices, power, the guide and
  the self-test.

## Rules and refusals

All paths below are under `faraday/shells/snapshot/src/tours/`.

- `boot`: on Mainnet the vault's public sample key must be refused over
  Wallets with `Sheet::Published`; otherwise the tour returns `Err`
  (`faraday/shells/snapshot/src/tours/boot.rs:153-164`). It returns `Err` when fewer than 11 wallets and keys
  are held after the import (`faraday/shells/snapshot/src/tours/boot.rs:174`).
- `createfrom`: returns `Err` if unlocking does not return to Create
  (`faraday/shells/snapshot/src/tours/create_from.rs:69-71`).
- `spend`: the vanity search runs up to 4000 ticks of 100 ms and returns
  `Err` if no address is found (`faraday/shells/snapshot/src/tours/spend.rs:112-122`).
- `first`: the wait for the vault save is 30 ticks and breaks silently if the
  state is never reached (`faraday/shells/snapshot/src/tours/first.rs:66-71`).
- `chartedit`: the lost and exposed mark shots are taken only if the node is
  found (`faraday/shells/snapshot/src/tours/chart_edit.rs:87, 95`).
- `codex32form`: the clash shot is taken only if still on Entry
  (`faraday/shells/snapshot/src/tours/codex32_form.rs:~71`). `vaultstates`: the "vault older than the copy"
  shot is taken only if reached (`faraday/shells/snapshot/src/tours/vault_states.rs:~299-303`).
- `default`: the self-test is forced to fail by a check `|_| false`; the shot
  is a fixed render (`faraday/shells/snapshot/src/tours/default.rs:1607-1613`). On `--sd-card` only home-empty
  is shot for the device check.
- `board`: after dark then light, the start theme is restored
  (`faraday/shells/snapshot/src/tours/board.rs:74-90`).
- `components`: an unknown or unset `FARADAY_SNAP_THEME` falls back to the
  default theme (`faraday/shells/snapshot/src/tours/components.rs:33-38`); the canvas is fixed at 720x900 at
  160 dpi (`faraday/shells/snapshot/src/tours/components.rs:17-29`).
- `dice5`: each picture is at most 520x560 (`faraday/shells/snapshot/src/tours/dice_five.rs:22-36`).
- `keys` and `seeds`: the scroll steps depend on a width under 700 px, not
  on `is_compact` (`faraday/shells/snapshot/src/tours/keys.rs:37`, `faraday/shells/snapshot/src/tours/seeds.rs:70`).
- `vaultway`: compact panels find the GPG tile by typing "gpg" after Escape,
  wide ones scroll to it (`faraday/shells/snapshot/src/tours/vaultway.rs:25-28`).
- `themes`: covers every `ThemeName::ALL` x `Mode::ALL` at Standard, plus
  Cobalt at Medium and High (`faraday/shells/snapshot/src/tours/themes.rs:10-26`).
- `walletflow`: sticks other than "TESTSTICK" are replaced by fake empty
  sticks (`faraday/shells/snapshot/src/tours/wallet_flow.rs:~163-167`).
- `restore` and `transfer` use `testkit::net()` (`faraday/shells/snapshot/src/tours/restore.rs:16`,
  `faraday/shells/snapshot/src/tours/transfer.rs:32`); the site tour does not choose a network.
- Refusals the tours show (the refusals are the app's; the tours only reach
  them): a typed "hello" as an address (check-address); a made-up seed in
  Restore (restore); an empty custom path (restore); a spoiled invoice
  checksum and a zero-amount BOLT #12 offer (invoice); a corrupted
  descriptor checksum (multisig-check); a damaged PSBT (nonce); pasted seed
  words and a jpg in Transfer (transfer); a bad domain " x" (silent); a
  wrong second paperkey line (gpg-restore); a wrong Krux passphrase (kef); a
  wrong vault passphrase, with and without a key file (vaultstates); a vault
  entry without a title (vaultshow); "Register this wallet" on a wallet that
  does not match (register); the public sample key on Mainnet (boot); a
  wrong passphrase for a passphrase seed (passseeds); a wrong hsm_secret
  passphrase (lightning-cln); a codex32 share whose checksum fails and a
  share of another set (codex32form); a round after a lock (codex32backup).
- Tours otherwise assert only that screens and sheets are reached, with `Err`
  strings such as "did not reach the Outbox" (`faraday/shells/snapshot/src/tours/default.rs:~991`).
- `#![expect(clippy::disallowed_methods)]` with the reason "the snapshot
  shell renders test data and is not shipped" in `faraday/shells/snapshot/src/tours/default.rs:3-6` and
  `faraday/shells/snapshot/src/tours/from_vault.rs:3-6`.

## Domain calls

- `faraday_core::testkit` (`faraday/faraday-core/src/testkit`): `TEST_SEEDS`,
  `test_words`, `key(n, path)`, `kits()` (spending, savings, musig and
  others), `unsigned(kit)`, `files()` (the test vault `vault.kdbx`),
  `VAULT_PASSPHRASE`, `kdbx_vault*`, `kdbx_edited`, `test_vault`,
  `inbox_file`, `inbox_file_at`, `inbox_push`, `vault_speed`,
  `confirm_wallets`, `session()`, `compute()`, `net()`, `move_to`,
  `use_network`, `into_demo`, `demo_samples`, `settle`, `flow_push`,
  `hold_worker` and `press_on_worker`, `show_busy` and `end_busy`,
  `select_wallet`, `plain_files` and `plain_kept`.
- `faraday_samples::SAMPLES` (`faraday/faraday-samples`).
- `faraday_codec` (`core/faraday-codec`): `seedqr::to_digits`,
  `vault_file::is_vault_file`, `decode::decode_luma`.
- `faraday_storage` (`faraday/faraday-storage`): `read_qr_png`, `read_host`,
  `save_download`.
- `faraday_crypto::sha256` (`core/faraday-crypto`), `faraday_selftest::Check`
  (`core/faraday-selftest`), `faraday_text::Text`.
- The app itself: `faraday_core::Faraday`, `Action`, `Screen`, `Sheet`,
  `screen::preview` (codex32correct), `plan::Preset`,
  `backup_flow::BackupFlow::with_plan`.

## Fixtures

From the kept fixture list:

- `faraday/faraday-samples/data/hsm-secret-encrypted` (lightning-cln, by
  `include_bytes!`).
- `faraday/faraday-samples/Cargo.toml` and the other `faraday/faraday-samples/data/`
  files, indirectly, through `faraday_samples::SAMPLES` (demo, modes, host).

Read from the source tree but not on the kept fixture list (located by
`CARGO_MANIFEST_DIR` or `include_bytes!` paths, so the tours depend on the
source tree):

- `tools/vectors/core-wallet/encrypted.dat` (core-wallet)
- `tools/vectors/paperkey/` (`gpg-unprotected.asc`, `tools/vectors/paperkey/gpg-unprotected.txt`,
  `gpg-rsa3072.asc`; gpg-restore)
- `tools/vectors/opentimestamps/` (ots)
- `tools/vectors/psbt/` (signer)
- `tools/vectors/kdbx/` (vaultstates)
- `tools/vectors/kef/` (named as the source of the Krux code inlined in
  `faraday/shells/snapshot/src/tours/kef.rs:12-14`)

## Fixes

None.

## Doubts

- Only about 20 rows were checked against the code by the inventory; the
  rest of the walk descriptions rest on single reports. The "defined at"
  lines and the registry kinds were checked against the checkout.
- `tools/vectors/` files the tours read are not on the kept fixture list; a
  rewrite of the tours needs them or copies.
- No tour enforces a window size; `rowdetails`, `files-wide` and
  `vaultsheets` docs assume a non-compact or wide window. Which sizes the
  lint recipes run is not covered here.
- Doc and code differ in places, unresolved: `faraday/shells/snapshot/src/tours/pass_seeds.rs:~14-21` doc
  mentions a boot stick but the code does no stick action; `faraday/shells/snapshot/src/tours/m3.rs:~1-7` doc
  mentions segmented buttons; the `kept` unprotected claim relies on
  `seed_files_at`; a misfiled doc comment above `power_tour` and `osk_tour`
  (`faraday/shells/snapshot/src/tours/devices.rs:~199-204`); `faraday/shells/snapshot/src/tours/vault_sheets.rs` doc comments "Costs a map of the
  plan" look copied; `faraday/shells/snapshot/src/tours/host.rs:~190-191` doc mentions a sample action that is
  not pressed; the site tour's doc says Mainnet but the code does not switch.
- Two sample sources: `testkit::demo_samples()` (`faraday/shells/snapshot/src/tours/desktop.rs:~23`) and
  `faraday_samples::SAMPLES` (`faraday/shells/snapshot/src/tours/desktop.rs:~52`, `faraday/shells/snapshot/src/tours/host.rs:~36-45`); which is
  canonical is unclear.
- `faraday/shells/snapshot/src/tours/vaults_pair.rs:~118-126` read-only tamper closure differs from
  `faraday/shells/snapshot/src/tours/vault_states.rs:~117-124` when the `<Value>1</Value>` marker is missing.
- The `vaultstates` filler size `slot - used + 1_000` is usize arithmetic and
  would panic in a debug build if `used` exceeds `slot`
  (`faraday/shells/snapshot/src/tours/vault_states.rs:343`, confirmed).
- `faraday/shells/snapshot/src/tours/codex32.rs:~148-150` seven digit presses and `faraday/shells/snapshot/src/tours/codex32.rs:~209-214` six
  `C32::Skip` presses have no stated meaning.
- The meaning of `inbox_file_at` index 1 in `band` is unclear.
- `codex32correct` draws screens the app may not reach.
- Whether tours rely on stick state across a restart (`silent`, `spend`,
  `first`) was not checked; `restart` does not re-send the sticks.
