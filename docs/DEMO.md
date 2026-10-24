# The demo, and Faraday Desktop for developers

**Kind:** Reference
**Status:** Built (F30, F61, F81); the website's part is in its own repository

## 1. The decision

1. Faraday Starter on Linux and Windows carries Faraday Desktop for its
   own system: **Open Faraday Desktop** on Home, and the desktop app
   beside the Starter from **Install**, in its payload (F83, which
   amends F30 and F61). Releases do not publish it on its own; the
   macOS Starter and the macOS desktop app carry none.
2. The code and its builds stay: `faraday/shells/desktop`, `cargo build
   -p faraday-desktop`, `just faraday-linux-bin` (whose default package
   is still `faraday-desktop`) and `just faraday-windows-bin build
   out/windows out/build/run faraday-desktop faraday`. `just` keeps
   building and testing the desktop shell; the Linux and Windows
   container recipes stay working, and the Starter's release builds go
   through the same recipes, so every release exercises them. The README
   says how to build it.
3. The website demo, the desktop shell compiled to WebAssembly in the
   website's own repository, is the way to try Faraday.
4. The demo lets people open every tab and screen, and every action
   that would make a secret is disabled. Bitcoin use with the eleven
   public seeds stays fully open, as F20 has it. The demo is for trying
   Faraday out, not for testing it exhaustively.

## 2. What "the demo" is now

The demo is **Demo**, a mode (F81, `docs/MODES.md`): a session whose
shell called `Session::set_demo`, on Testnet. `Keys::demo()` is true, and
`Faraday::mode()` is Demo. Its keys derive at coin type 1 and its
addresses are real testnet addresses, so nothing sent to a public seed's
address is bitcoin. Two builds have it:

- **The website**, which also calls `Session::fix_network`, so it is in
  Demo for the whole session.
- **Faraday Desktop**, a developer's build or a release, which starts in
  Demo. Leaving it for Testnet, Signet or Regtest is a lock into a
  fresh session in that mode (F81). No warning sheet comes first (M3,
  `docs/MODES.md` §4); every step that takes a seed or a secret that can
  be typed draws the line "Never type a seed that holds real bitcoin
  here." under its field (not a passphrase field of Demo, §5.4).

The stick, the Pi and the `Faraday` the Starter holds for Transfer and
Learn never call `set_demo`: the stick has no Demo mode, and nothing in
this spec changes them.

**The gate.** One function in `faraday-core`, `Faraday::makes_secrets()`,
is false while `Keys::demo()` is true (Demo) and true otherwise. Every
action in §4 asks it, in two places: the screen draws the action with
`Style::Disabled`, and the action's handler does nothing when it is
false, so a key press, a click, a shell message or a test's action all
meet the same refusal. No action in §4 has a path around it.

**Test networks in Faraday Desktop** keep F20's rules: any seed, real
addresses, no warning sheet. `Keys::demo()` is false there, so every
action in §4 is live on a test network, which is how a developer tries
Make flows without a stick. A key made there is a real key made on an
online computer; the line under every field that takes a seed or a secret
is the guard, in every mode, and the released files are named testnet
only (F61). F27's rule is
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
passphrase, whose rolls run and whose words are replaced by `faraday`,
the one passphrase of the demo (§5.2). What they make is shown and then
dropped, never kept.

An action stays open when it shows public values, loads a public seed,
signs or checks with one, or encrypts what is already held. Encryption
draws salts and nonces and signing draws nonces; neither is a secret a
person keeps.

## 4. Disabled in the demo

Every action below is dimmed in Demo and live in every other mode,
Testnet, Signet and Regtest included.

| Action | Screen | What it would make |
|---|---|---|
| SLIP-39 shares: its sources and the deal | Tools → SLIP-39 shares (New key in SLIP-39 mode) | a new key dealt as shares; no public seed stands in |
| SLIP-39 shares of a key held | a key's Backup menu | fresh random shares of the key |
| Split with Seed XOR (**Make them**) | a key's Backup menu; the backup checklist's Seed XOR item; the chart's Make another backup | fresh random parts |
| Codex32 shares, k of n with k of 2 or more | a key's Backup menu; the backup checklist's codex32 item; Make another backup | fresh random shares |
| **Make a GPG key** | Tools → GPG key; Vault contents' GPG kind | an OpenPGP key |
| **Make Secure Boot keys** | Tools → Secure Boot keys; Vault contents' Secure Boot kind | PK, KEK and db keys |
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
  SeedQR, CompactSeedQR, SLIP-39 shares, Seed XOR parts, codex32), refused unless it is a
  public seed (F20); a passphrase, chosen and never typed: it is `faraday`
  (§5.4); **Load samples**,
  the pill beside the Demo pill, which loads ticked sample wallets and
  sample keys in every form, with the wallets' own keys (F81, `docs/MODES.md`
  §3 and §10 M6); **Use a sample**, a chip inside the empty field of every
  step that takes a key (Add a key, New key's typed key, the Spend tab's
  words, a cosigner's account key) and a **Sample** tile on the scanner,
  which put the next public sample in as if typed or scanned (§6 of
  `docs/MODES.md`, §10 M7); Explore; Account key; the key's page.
- **Codex32** (`docs/CODEX32.md` §12): Type codex32 strings takes the
  strings of BIP 93's published sets (the samples, `docs/MODES.md` §7)
  and refuses a set that is not public as any key is; **Check a string**
  answers for any string, since it loads and keeps nothing; **Suggest
  corrections** is open on the same form; a Codex32QR code of a public
  seed's string is read and shown. Make codex32 shares opens and its
  entry runs; its Lock in loads nothing (CODEX32 §12 plans the sample
  in place of the strings made).
- **New key**: every length and source (dice, coins, cards, hex,
  camera, the generator, a mix), its words and quiz; Lock in adds the
  next public seed in place of the key made (§5.1).
- **Wallets**: every kind built from public seeds (single-key, multisig,
  Taproot multisig, MuSig2, miniscript and recovery, silent payments'
  address and labels), Restore, Check an
  address, addresses (real testnet addresses, F81), the wallet card and
  its chart.
- **Spending**: Sign a transaction, the Spend tab, the nonce check,
  Decode, MuSig2 and FROST signing rounds.
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
- **Vanity address** on the account dial; **Lightning node key**'s node
  id from a loaded public seed, and its LND cipher seed row, which takes
  only LND's published seed and offers it (**Use a sample**); **Silent payments**' address, labels,
  record and Check a payment.
- **Tools**: every calculator, word list and Find a tool; every tile
  opens its screen.
- **Settings**, About, Learn, themes.

### 5.1 New key in the demo

Every length and every source is open: the dice, coins, cards, hex,
camera noise, the generator and a mix make words as on the device, and
they are shown. At Lock in the demo adds the next sample of the length
chosen that is not loaded, in place of the key made (F81,
`docs/MODES.md` §7): four samples of 12 words, two each of 15, 18 and
21, eleven of 24 (`faraday-samples`). The Key card says which ("Lock in
adds sample {} ({}) in place of these words", with the sample named as
"ocean ×12"). A sample loaded with the passphrase `faraday` counts as
loaded. The key made is never added or kept. SLIP-39 shares are not New
key: they stay disabled (§4), since no sample stands in for a dealt set
of shares. With every sample of the length loaded, Lock in is disabled
for that length only.

### 5.2 Generate with dice in the demo

**Generate with dice** on Create a vault is open: the person rolls and
types the numbers, and the words appear as on the device. When the last
word is in, both passphrase fields hold `faraday` in place of the rolled
words, and the card says so in one line, "Demo puts faraday in place of
these words", as New key's Key card does. No strength line is drawn for
it. On Faraday Desktop the dice roller (F96) rolls into the
same entries for New key's dice and Generate with dice; the outcome is
unchanged: the sample, or `faraday`. `faraday` is `demo::PASSPHRASE` in `faraday-core/src/demo.rs`, the
one constant every Demo passphrase field and the samples
(`faraday-samples`, the sample vault and backup) use.

### 5.4 Every passphrase field offers `faraday`

In Demo every passphrase and password field is typed, masked and edited
as outside Demo (F81, `docs/MODES.md` §6.2): empty, with the eye and the
caution line "Never type a seed that holds real bitcoin here." under it.
An empty field also offers a chip inside it, "faraday"; a press on it
puts `faraday` in the field, drawn unmasked in a code box (DESIGN
§4.10). The chip goes when the person presses in the field or types; a
press in a field holding `faraday` lets the person clear or replace it,
and what is typed is theirs and masked. A second field that types a
passphrase back, and an unlock, offer the chip too. A field that is
optional keeps **No passphrase** in it while empty (Add a key's BIP-39
passphrase, New key's pair, the potential wallet's, an aezeed's). A key
added with a passphrase of `faraday` is the public set's second
fingerprint of the sample, which Demo takes. The KeePass export's
passphrase has no chip: `faraday` is under its minimum of 8 characters,
so the person types one. `Faraday::demo_passphrase()` is the one test.

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
| A password into a vault Entry; a TOTP secret | The password is typed, or `faraday` from its chip (§5.4); a TOTP secret is typed or scanned | The password field is a passphrase field of the demo; a TOTP secret is typed, and is the only way to try TOTP codes and KeePass export. The demo cannot tell a real secret from a made-up one; the demo's Learn page says to type nothing real |
| Typing an LND cipher seed (aezeed) on Lightning node key | Open in Demo, taking LND's published vector only (F81, `docs/MODES.md` §10 M7, M8), which **Use a sample** types in; its passphrase is none or `faraday` (§5.4), and the vector has none | Whatever else is typed would be someone's real node seed, so Demo refuses it as it refuses a key that is not public. A loaded public seed is the other source and stays |
| Making a vault | Open, under `faraday`; **Generate with dice** open, its words replaced by `faraday` (§5.2) | It encrypts what the demo holds: public seeds and what was typed. Its keys are internal to the file and never shown. The rolled words are dropped, as New key's are, so the passphrase the vault takes is public |
| BIP-85 outputs | Disabled, as the owner said | Derived from a public seed, so anyone can derive them, but they look like a fresh password or seed and would be used as one |
| Lightning node private key, silent payments scan key | Disabled | Derived private keys for use in another program, as BIP-85's are |
| The secret sheet and **Out unprotected…** for a public seed | Open | Every value they can carry in the demo is a public seed's own spelling; the sources of every other secret are disabled at §4 |
| Seed XOR, codex32 and SLIP-39 splits of a public seed | Disabled | The parts are fresh randomness; a person copying them would hold something that looks like a real backup |
| A single codex32 string of a public seed (k = 0, the secret alone), where offered | Open | Another spelling of a public seed, with no randomness |
| KeePass export, sealed files | Open | Encryption of what is held, under `faraday` from its chip or one the person types (§5.4). The KeePass export's passphrase is typed by the person, masked with the eye, and its minimum of 8 characters holds (§5.4) |
| Vanity address's passphrase dial | Disabled | It makes a new key under a passphrase; Demo refuses that key anyway |
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
- A press on it does nothing (DESIGN §4.11, Dimmed). It opens no Pull
  sheet and no vault lead.
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
and the screen reached shows its Make action dimmed. FLOWS decision
10's paragraph carries this exception.

## 8. Changes to F20

Moved to `docs/history/DEMO-LOG.md`.

## 9. Releases

A payload carries the images and Faraday Desktop for its own system
(F83): staging (`faraday/tools/stage-release.sh`) puts it in each
payload folder, and refuses a release folder holding any file named
`faraday-desktop-…`. The desktop app is not published beside the
Starters.

Moved to `docs/history/DEMO-LOG.md`.

## 10. Faraday Starter

Moved to `docs/history/DEMO-LOG.md`.

## 11. Decisions that offer things "in Faraday Desktop"

Moved to `docs/history/DEMO-LOG.md`.

## 12. Batches

Moved to `docs/history/DEMO-LOG.md`.

## 13. Decided

Moved to `docs/history/DEMO-LOG.md`.
