# Practice, and Faraday Demo for developers

**Kind:** Reference
**Status:** Built (F30, F81, F126; Practice's aids in Faraday Demo's host strip, §14, F118)

## 1. The decision

1. Faraday Starter on Linux and Windows has Faraday Demo compiled in:
   **Open Faraday Demo** on Home shows it in the Starter's own window,
   under the Starter's top app bar (F126, which amends F30;
   F81 holds the rest). It is not installed and not a release file. Releases
   do not publish it on its own.
2. The code and its builds stay: `faraday/shells/demo`, `cargo build
   -p faraday-demo`, `just faraday-linux-bin` (whose default package
   is still `faraday-demo`) and `just faraday-windows-bin build
   out/windows out/build/run faraday-demo faraday`. `just` keeps
   building and testing the desktop shell; the Linux and Windows
   container recipes stay working, and the Starter's release builds go
   through the same recipes, so every release exercises them. The README
   says how to build it.
3. The website carries no demo; Faraday Demo, in the Starter or as its
   own download, is the way to try Faraday.
4. Practice lets people open every tab and screen, and every action
   that would make a secret is disabled. Bitcoin use with the eleven
   public seeds stays fully open, as F81 has it. Practice is for trying
   Faraday out, not for testing it exhaustively.

## 2. What Practice is now

Practice is a mode (F81, `docs/MODES.md`): a session whose
shell called `Session::set_demo`, on Testnet. `Keys::demo()` is true, and
`Faraday::mode()` is Practice. Its keys derive at coin type 1 and its
addresses are real testnet addresses, so nothing sent to a public seed's
address is bitcoin. One build has it:

- **Faraday Demo**, a developer's build or a release, which starts in
  Practice. Leaving it for Testnet, Signet or Regtest is a lock into a
  fresh session in that mode (F81). No warning sheet comes first (M3,
  `docs/MODES.md` §4); every step that takes a seed or a secret that can
  be typed draws the line "Never type a seed that holds real bitcoin
  here." under its field (not a passphrase field of Practice, §5.4).

The stick, the Pi and the `Faraday` the Starter holds for Transfer and
Learn never call `set_demo` (the Faraday Demo inside the Starter does:
it is Faraday Demo, made by `faraday_demo::desktop_app_in`): the stick has no Practice mode, and nothing in
this spec changes them.

**The gate.** One function in `faraday-core`, `Faraday::makes_secrets()`,
is false while `Keys::demo()` is true (Practice) and true otherwise. Every
action in §4 asks it, in two places: the screen draws the action with
`Style::Disabled`, and the action's handler does nothing when it is
false, so a key press, a click, a shell message or a test's action all
meet the same refusal, which is one toast (§7). No action in §4 has a path around it.

**Test networks in Faraday Demo** keep F81's rules: any seed, real
addresses, no warning sheet. `Keys::demo()` is false there, so every
action in §4 is live on a test network, which is how a developer tries
Make flows without a stick. A key made there is a real key made on an
online computer; the line under every field that takes a seed or a secret
is the guard, in every mode, and the released files are named testnet
only (F81). A seed such a vault keeps carries `faraday.online`
(`docs/VAULT.md`): on the device in Mainnet it is kept only after a hold,
and Mainnet refuses Faraday's public sample seeds when they are loaded as a BIP-32 key, by every route (typed, SeedQR or KEF scan, words file, codex32 strings, a vault), with only Remove offered; it refuses a sample's BIP-85 child too (for words typed bare, the child at index 0; for one derived or recorded, any index) and the Lightning samples (LND's cipher seed, Core Lightning's hsm_secret samples, and a sample's words typed as Core Lightning words), with only Remove or an error line. Testnet, Signet and Regtest take them all. A QR vault has nowhere to carry the mark, so Faraday Demo seals no seed into one and no marked seed or share is copied into one. Faraday Demo writes the mark on every key, BIP-85 child, codex32 share and words file it saves and on every key or share it copies or moves to another vault (on the device a copy keeps its source's mark), and a child of a marked key is marked; the mark is written by an honest Demo, and the online computer could remove it from the file.

## 3. What makes a secret

An action is disabled in Practice when its result is a secret a person
could take away and use: a new seed, key, share, passphrase or password
drawn from randomness (the system's generator, dice, coins, cards, hex
digits or camera noise), and a private value derived for use outside
Faraday (BIP-85's outputs, a Lightning node's private key, a silent
payments scan key). A public seed's own words in any spelling are not a
new secret: they are public.

Two actions are open with their result replaced (owner, 2026-10-14):
New key, whose sources all run and whose key is replaced by the next
public seed (§5.1, F81), and **Generate with dice** for a vault
passphrase, whose rolls run and whose words are replaced by `faraday`,
the one passphrase of Practice (§5.2). What they make is shown and then
dropped, never kept.

An action stays open when it shows public values, loads a public seed,
signs or checks with one, or encrypts what is already held. Encryption
draws salts and nonces and signing draws nonces; neither is a secret a
person keeps.

## 4. Disabled in Practice

Every action below is dimmed in Practice and live in every other mode,
Testnet, Signet and Regtest included.

| Action | Screen | What it would make |
|---|---|---|
| SLIP-39 shares: its sources and the deal | Tools → SLIP-39 shares (New key in SLIP-39 mode) | a new key dealt as shares; no public seed stands in |
| SLIP-39 shares of a key held | a key's Backup menu | fresh random shares of the key |
| Split with Seed XOR (**Make them**) | a key's Backup menu; the backup checklist's Seed XOR item; the chart's Make another backup | fresh random parts |
| Codex32 shares, k of n with k of 2 or more | a key's Backup menu; the backup checklist's codex32 item; Make another backup | fresh random shares |
| **Make a GPG key** | Tools → GPG key; Vault contents' GPG kind | an OpenPGP key |
| **Restore from a paperkey** | Vault contents' GPG kind; a public key's ⋮ in Files | an OpenPGP secret key in a vault |
| **Make Secure Boot keys** | Tools → Secure Boot keys; Vault contents' Secure Boot kind | PK, KEK and db keys |
| **Derive with BIP-85**, every application (Words, WIF, Extended private key, Hex, Password base64, Password base85) | Tools → Child seeds and passwords | a child seed, key or password |
| The passphrase dial | Vanity address, the How? card | a new key under a found passphrase; Practice would refuse it anyway |
| **Show** for the node's private key | Lightning node key | a node's private key, on screen |
| **Save into {vault}** and **Out unprotected…** for the node's private key | Lightning node key | a node's private key |
| **Save into {vault}** and **Out unprotected…** for the scan key | Silent payments | a scan key |

Each screen above opens in Practice. Its other cards, choices and
public values work; only the actions in the table are disabled.

## 5. Open in Practice

- **Keys**: typing, scanning or reading a public seed in any form (words,
  SeedQR, CompactSeedQR, SLIP-39 shares, Seed XOR parts, codex32), refused unless it is a
  public seed (F81); a passphrase, chosen and never typed: it is `faraday`
  (§5.4); **Load samples**,
  the pill beside the Practice pill, which loads ticked sample wallets and
  sample keys in every form, with the wallets' own keys (F81, `docs/MODES.md`
  §3); **Use a sample**, in the "Demo only" card (§14) of
  every step that takes a key (Add a key, New key's typed key, the Spend
  tab's words, a cosigner's account key) and of the scanner, which puts
  the next public sample in as if typed or scanned (§6 of
  `docs/MODES.md`); Explore; Account key; the key's page.
- **Codex32** (`docs/CODEX32.md` §12): Type codex32 strings takes the
  strings of BIP 93's published sets (the samples, `docs/MODES.md` §7)
  and refuses a set that is not public as any key is; **Check a string**
  answers for any string, since it loads and keeps nothing; **Suggest
  corrections** is open on the same form; a Codex32QR code of a public
  seed's string is read and shown; **Make the rest** and **Add a share**
  take the published sets only. Make codex32 shares opens and its entry
  runs, the sheets filling from the person's rolls or typing, or from
  **Use a sample**, in the entry's "Demo only" box (CODEX32 §12); at the entry's Continue the strings made are dropped
  and the sample set, BIP 93's vector 2, stands in their place on every
  later card, with the line "Practice puts sample NAME in place of these
  strings"; its Lock in loads the sample's key. Strings from the
  person's own rolls never leave the entry.
- **New key**: every length and source (dice, coins, cards, hex,
  the generator, a mix, with the camera among its sources), its words and quiz; Lock in adds the
  next public seed in place of the key made (§5.1).
- **Wallets**: every kind built from public seeds (single-key, multisig,
  Taproot multisig, MuSig2, miniscript and recovery, silent payments'
  address and labels), Restore, Check an
  address, addresses (real testnet addresses, F81), the wallet card and
  its chart.
- **Spending**: Sign a transaction, the Spend tab, the nonce check,
  Decode, MuSig2 signing rounds.
- **Messages**: Sign a message, Check a signed message (its Tools tile
  takes the sample message where the Inbox has none), Find an address
  (the sample address in its field).
- **Backups of a public seed**: Words, SeedQR, CompactSeedQR, a single
  codex32 string where one is offered, into a vault, the secret sheet and **Out unprotected…** for the seed
  itself, the backup plan and checklist (§5.3), Check a copy.
- **Vaults**: Create a vault, its passphrase `faraday` or **Generate with
  dice**, whose words `faraday` replaces (§5.2), a sample vault where
  none is in Files (**Use a sample**), Unlock, Vault
  contents, entries, notes, TOTP codes, KeePass export, the stick
  visit and Files.
- **QR vault** tool: Make seals a loaded public seed or a typed note under
  a passphrase of `faraday` or the person's own (§5.4); Open takes a code
  scanned, from Files or sample (**Use a sample**, §14.1), and its offers
  are the forms' own: a seed's words go to the seed sheet and a share to
  the codex32 form, which Practice takes only for a public seed (§3), a
  passphrase adds to a loaded key, a note saves into an open vault.
- **Vanity address** on the account dial; **Lightning node key**'s node
  id from a loaded public seed, and its LND cipher seed row, which takes
  only LND's published seed and offers it (**Use a sample**); **Silent payments**' address, labels,
  record and Check a payment.
- **Tools**: every calculator, word list and Find a tool; every tile
  opens its screen.
- **Settings**, About, Learn, themes.

### 5.1 New key in Practice

See `docs/MODES.md` §7.

### 5.2 Generate with dice in Practice

See `docs/MODES.md` §7.1.

### 5.4 Every passphrase field offers `faraday`

See `docs/MODES.md` §6.2.

### 5.3 The backup plan in Practice

See `docs/MODES.md` §7.2.

## 6. Borderline calls (decided)

| Case | Decision | Why |
|---|---|---|
| A password into a vault Entry; a TOTP secret | The password is typed, or `faraday` from its chip (§5.4); a TOTP secret is typed or scanned | The password field is a passphrase field of Practice; a TOTP secret is typed, and is the only way to try TOTP codes and KeePass export. Practice cannot tell a real secret from a made-up one; Practice's Learn page says to type nothing real |
| Typing an LND cipher seed (aezeed) on Lightning node key | Open in Practice, taking LND's published vector only (F81, `docs/MODES.md` §6.2), which **Use a sample** types in; its passphrase is none or `faraday` (§5.4), and the vector has none | Whatever else is typed would be someone's real node seed, so Practice refuses it as it refuses a key that is not public. A loaded public seed is the other source and stays |
| Making a vault | Open, under `faraday`; **Generate with dice** open, its words replaced by `faraday` (§5.2) | It encrypts what Practice holds: public seeds and what was typed. Its keys are internal to the file and never shown. The rolled words are dropped, as New key's are, so the passphrase the vault takes is public |
| BIP-85 outputs | Disabled, as the owner said | Derived from a public seed, so anyone can derive them, but they look like a fresh password or seed and would be used as one |
| Lightning node private key, silent payments scan key | Disabled | Derived private keys for use in another program, as BIP-85's are |
| The secret sheet and **Out unprotected…** for a public seed | Open | Every value they can carry in Practice is a public seed's own spelling; the sources of every other secret are disabled at §4 |
| Seed XOR, codex32 and SLIP-39 splits of a public seed | Disabled | The parts are fresh randomness; a person copying them would hold something that looks like a real backup |
| A single codex32 string of a public seed (k = 0, the secret alone), where offered | Open | Another spelling of a public seed, with no randomness |
| KeePass export, sealed files | Open | Encryption of what is held, under `faraday` from its chip or one the person types (§5.4). The KeePass export's passphrase is typed by the person, masked with the eye, and its minimum of 8 characters holds (§5.4) |
| Convert's **A SLIP-39 set** source | Disabled | A set's shares are typed in as a key; Practice takes none (F292) |
| Vanity address's passphrase dial | Disabled | It makes a new key under a passphrase; Practice refuses that key anyway |

## 7. How a disabled action looks

- DESIGN §4.11's dimmed state, drawn by `faraday-ui`'s `Style::Disabled`,
  the same as every dimmed control: its label, dimmed, in its own place
  and size.
- **No reason line.** DESIGN §4.11 gives a dimmed reason only for a
  condition the person could change on this device, and leaves a row dead
  for a condition they cannot change with its label alone. In Practice
  nothing on the screen changes it, so no "Not in Practice" appears.
  Working screens carry no explanation (CLAUDE.md).
- A press on it shows one toast, "Not available in Practice: this is an
  online computer" (F292), and does nothing else: no Pull sheet, no vault
  lead, no reveal, nothing saved or shown. It is the exception DESIGN
  §4.11 names to a dimmed control taking no press, and only for the actions
  of §4. A refused button is drawn so; a refused eye or Show button (BIP-85's
  value, the node key, the scan key and the descriptor's output) is drawn
  as usual and answers a press with the same toast. Sweep's tile and Convert's Set source say the same.
- **Where the reason is.** Practice's Learn page
  comes first in the info button's list on every screen of §4, as it
  does today on Home, Entry, New key and Settings. Its "New key in the
  demo" section stays and gains Generate with dice (§5.2); a new section,
  "What Practice does not make", gives the list of §4 in one paragraph,
  and that a Faraday stick makes them.

### 7.1 The exception to FLOWS decisions 8 and 10

`docs/FLOWS.md` decision 8 shows nothing that loads a key disabled for a
stick being in, and decision 10 shows no disabled button where a vault
is needed. Both stand. Practice adds one exception: while
`Session::demo()` is true, an action listed in §4 is drawn dimmed, with
no reason, and a press on it shows the toast. It is the only action in Faraday dimmed for
a condition the person cannot change on the device. A Tools tile that
leads to one of these screens is not dimmed: "Make a vault first" and
"Unlock a vault first" still lead into the vault and back (decision 10),
and the screen reached shows its Make action dimmed. FLOWS decision
10's paragraph carries this exception.

## 9. Releases

The Starter carries no image: Faraday Demo is compiled into it
(F126). Staging (`faraday/tools/stage-release.sh`) takes no
Faraday Demo build and refuses a release folder holding any file named
`faraday-demo-…`. The desktop app is not published beside the
Starters.

## 14. The "Demo only" card

Everything that exists only for Practice stands outside the app, in
Faraday Demo's host strip at the window's foot, under "Practice", in the
host's own look (owner, 2026-10-25; F118, `docs/DESKTOP-HOST.md` §3),
so it never reads as part of the device's screen. The strip shows only
the aids of the screen or sheet on show. A page hands each item to the
frame where it would have drawn it (`faraday_ui::demo`), and the host
takes it into the strip; nothing practice-only is drawn in a page. (Until
then, owner 2026-10-24, they stood in an outlined "Demo only" card
beside the page; the heading keeps its number.)

**Practice is the one condition** (owner, 2026-10-24): every aid is offered
only in Practice. In Testnet, Signet, Regtest and Mainnet, on Faraday
Demo too, they are gone, not moved: the computer's dice rollers are
Practice's alone (`Faraday::roller_offered`).

### 14.1 What the strip holds

See `docs/MODES.md` §6.3.

### 14.2 What stays where it is

See `docs/MODES.md` §6.3.
