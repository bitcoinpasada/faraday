# Mode and Load samples

**Kind:** Reference
**Status:** Checked against the app's pictures in docs/guide/pictures/ (drawn by just guide-pictures) and its code and strings (`faraday/faraday-core/src/modes/screen.rs`, `faraday/faraday-core/src/samples/screen.rs`)
**Screen:** `Mode`, `Samples`
**Group:** Home

## Mode

**Opened from:** the mode pill at the top right, shown in every mode
but Mainnet (**Practice · Testnet** in Practice, **Practice** alone on
a narrow window, otherwise the mode's name), and **Settings → Mode**.

![The Mode sheet in Faraday Demo: Practice (Testnet) ticked, then Testnet, Signet, Regtest and Not now](../pictures/mode-mode.png)

*The Mode sheet in Faraday Demo: **Practice** (**Testnet**) ticked, **Testnet**, **Signet**, **Regtest**, and **Not now**; there is no **Mainnet** in the Demo.*

| Control | What it does |
|---|---|
| **Practice** (**Testnet**) | Faraday Demo only: the public sample keys and wallets, on Testnet |
| **Mainnet** | on the stick only |
| **Testnet**, **Signet**, **Regtest** | the test networks |
| the tick | marks the mode in use |
| **Not now** | closes the sheet |

A press on another mode changes to it. A change of mode is a lock:
Faraday starts again in the mode chosen, and Files, the files for the
stick, keys and wallets are emptied. When that would lose something,
[Lock](lock-and-power.md#lock) asks first.

## Load samples

**Opened from:** **Samples** in Faraday Demo's strip, in Practice. In
Faraday Demo the sheet opens above the strip.

![Load samples with the passphrase faraday, Unselect all, and the sample wallets each ticked](../pictures/mode-samples.png)

*The sheet **Load samples** with every sample ticked: the passphrase `faraday`, **Unselect all**, the **Wallets** (Savings, Spending, Taproot and the rest), **Not now** and **Load ticked**.*

| Control | What it does |
|---|---|
| **In Practice, the sample passphrase is** | followed by `faraday`, Practice's one passphrase and password |
| **Pull the stick to load the samples** | shown while a stick is attached |
| **Select all**, **Unselect all** | ticks or unticks every sample not loaded yet |
| **Wallets** | the sample wallets, each with a tick box and what it is |
| **Keys** | the sample keys, each with a tick box and what it is |
| a sample already loaded | ticked and greyed |
| **Not now** | closes the sheet |
| **Load ticked** | loads the ticked samples ("Loaded 1 wallet and 3 keys"). Greyed until one is ticked. With a stick attached, [Pull the stick](prompts.md#pull-the-stick) opens first |

## Why

- [Practice and its public seeds](../../learn/faraday/demo.md)
