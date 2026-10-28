# Lock and Power off

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/screens/sheets.rs`, `faraday/faraday-core/src/vault/mod.rs`, `faraday/faraday-core/src/lib.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `LockAsk`, `Lock`, `IdleWarn`, `Locked`, `Power`
**Group:** Home

The sheets that end a session. Each section is one sheet, under the
title it shows. A lock wipes every key and wallet loaded, seals the
open vaults, and starts Faraday again; Files and the files for the
stick stay. A power-off keeps only what is on the stick.

The lock and power sheets list three sections:

| Section | Lists |
|---|---|
| **Kept** | each vault in Files with where it stands ("Vault 1 · Unchanged", "… · Changed since written"), and the files for the stick ("0 files for the stick") |
| **Wiped from memory** | each key and wallet loaded, with where else it is ("in Vault 1", "in Vault 1 · copy checked", "copy checked"), or "nowhere else" in the error tone, listed first; **Make a new wallet · ends** while a new wallet's keys are kept nowhere yet |
| **Then** | what follows |

## Lock

**Opened from:** **Lock** at the foot of the sidebar, and a change of
mode ([Mode](mode.md#mode)), when the lock would end something: Make a
new wallet under way, or a vault open with changes, or with a key or
wallet loaded that is in no vault. Otherwise Lock locks at once,
without the sheet.

*Picture to come: `lock-and-power-lock.png`, the Lock sheet while a wallet is being made.*

| Control | What it does |
|---|---|
| **Save** | beside a key or wallet in no vault, while a vault is open: saves it into the open vault |
| **Then** | "Faraday restarts on Unlock"; for a change of mode, "A mode change keeps nothing but the mode: Files, For the stick, keys and wallets are emptied." |
| **Back to the vault** | closes the sheet |
| **Lock** | locks: seals the open vaults, wipes what is listed, and starts again |
| **Lock without sealing** | in place of **Lock** while a vault cannot be sealed: the line under the title says why ("No randomness from the system. Faraday has to restart", or that a vault's changes are over its slot), and its changes are lost |

## A stick is attached

**Opened from:** a stick plugged in while a key is loaded.

*Picture to come: `lock-and-power-stick-attached.png`, A stick is attached, with what the lock keeps and wipes.*

| Control | What it does |
|---|---|
| the stick line | the stick's name and "nothing has been read from it" |
| **Then** | "A fresh start, and the stick visit" |
| **Not now** | closes the sheet and keeps working with the stick in; nothing is read from it. A press beside the sheet does the same |
| **Lock and use stick** | locks, then opens the stick's files |

## Locking soon

**Opened from:** a time without input, before the idle lock
(**Settings → Session**, **Lock after**). No press beside the sheet
closes it.

*Picture to come: `lock-and-power-locking-soon.png`, Locking soon with its countdown.*

| Control | What it does |
|---|---|
| the line | how long there has been no input, and when it locks: "No input for 25 minutes · locks at 30 minutes without input · any key or touch keeps it" |
| **Wiped** | the keys, wallets and open vaults the lock wipes ("1 key · 1 wallet · 0 open vaults") |
| **Sealed** | the open vaults with changes, sealed for the stick. **Not sealed** in its place while the system has given no randomness: their changes are lost |
| **Kept** | the files from the stick and for the stick |
| **Then** | "Power off at 60 minutes without input, unless files wait for the stick", or "No power-off" |
| **Keep working** | closes the sheet; nothing is locked. Any key or touch does the same |

## Locked

**Opened from:** the idle lock, once it has locked.

*Picture to come: `lock-and-power-locked.png`, Locked, with files waiting for the stick.*

| Control | What it does |
|---|---|
| the line | "Locked after a time without input. Every key and vault was wiped." |
| the countdown | "Powers off in 29:40 without input", while a power-off is set |
| the files | while files wait for the stick: "2 for the stick · no power-off until written", and the first five by name |
| **Continue** | closes the sheet |

## Power

**Opened from:** the power button at the foot of the sidebar,
**Settings → Session**'s **Power off**, and the **⋮** menu's **Power
off** on a tall narrow window. On the stick its title is **Power**; in
Faraday Demo it is **Quit**, and the Demo quits at once when nothing
would be lost.

*Picture to come: `lock-and-power-power.png`, the power sheet on a stick started with Secure Boot off.*

| Control | What it does |
|---|---|
| **Turn Secure Boot back on in the BIOS before starting Windows.** | on a PC whose Secure Boot was off at start, at the top of the sheet |
| **Usual BIOS key** | with it, the BIOS key of this computer's maker, where Faraday knows it |
| the QR code | with it, the address of the steps for this computer, written under the code |
| **Kept**, **Wiped from memory** | as above; at a power-off a vault not written as it stands and the files for the stick are wiped too |
| **Then** | "The computer powers off, or restarts"; in Faraday Demo, "Faraday closes" |
| **Cancel** | closes the sheet |
| **Restart** | restarts the computer. Not in Faraday Demo |
| **Power off** | turns the computer off |
| **Quit** | Faraday Demo: closes it |
| **Hold to restart**, **Hold to power off**, **Hold to quit** | in place of the three above while files wait for the stick or an open vault has changes not saved: held until it fills. A short press does nothing |

## Why

- [Where keys are kept](../../learn/faraday/keys-kept.md)
- [Vaults](../../learn/faraday/vaults.md)
- [Secure Boot and BitLocker](../../learn/faraday/boot/secure-boot-bitlocker.md)
