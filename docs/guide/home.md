# Home

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Home`
**Key:** home
**Words:** key, wallet

Home is the first screen after the self-test and after every lock.
It leads with the next thing to do, then one row for what is going on (a
vault open, a task left part way, files to write), then only the wallets
that need something and the keys no wallet uses, each list ending in a row
that opens all of them; while nothing is loaded, it shows the tiles that
start each task instead.

## Search

**Key:** home.search

A search bar stands under Home's buttons (every window but the Pi's small
panel). Typing finds a tool, a Settings card by any word on it or a glossary word and shows the matches under the bar,
the buttons above staying; a press opens a tool, a card works there as it does on Settings, a word opens at its definition in the guide. Escape clears the search.

## The power button

**Key:** home.power

The power button at the top right of the top app bar, beside the ?, opens a menu: **Lock** while a secret is held, **Restart** where the computer offers it, and **Power off** (**Quit** in the desktop app, which has no Restart). **Restart** and **Power off** open the power sheet, which lists what is kept and what is wiped. The Pi's small panel has **Lock** and **Power off** at the foot of Home's menu instead.
