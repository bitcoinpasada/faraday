# Faraday Starter: Home

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-starter/src/draw/home.rs`, `faraday/faraday-starter/src/draw/nav.rs`, `faraday/faraday-starter/src/draw.rs`, `faraday/faraday-starter/src/decide.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Starter page:** `Home`
**Starter sheet:** `LetThrough`
**Group:** Starter

**Opened from:** the Starter opens on it; **Home** on the rail or the
bar. A press on **Home** while it is on show goes to its top.

Home is about this computer: the This computer card with its
checklist and the screen's one primary button, then a card for each
thing about this computer and this copy of the Starter that applies.

*Picture to come: `starter-home.png`, the Starter's Home with the rail, This computer and its Checklist, and the Faraday Demo card, at 1280 × 800.*

## The rail, the bar and the top app bar

| Control | What it does |
|---|---|
| **Home**, **Make**, **Tools**, **Learn** | the four destinations, on the rail at the left (on a narrow window, a bar across the foot); the one on show is on a pill. A press opens a destination as it was last left; a press on the one on show goes to its top. Make: [Make](starter-make.md#make); Tools: [Tools](starter-tools.md#tools); Learn: [Learn](starter-learn-and-boot.md#learn) |
| the wordmark | on Home, the Faraday wordmark with "Starter" beside it in grey; on Make and Tools, the destination's name |
| **Install** | at the top right while no copy is installed: opens [Install](starter-menu.md#install-and-update) |
| **Update** | in place of **Install** while this copy is newer than the one installed: opens the same page as Update |
| **⋮** | the menu: [Appearance, About, Uninstall, Remove the Faraday boot entry](starter-menu.md#the--menu) |

A page inside a task (**Which stick?**, **About**, a Learn page opened
from a link) has no rail or bar: its top app bar holds the back arrow,
the name of where it goes back to, and the page's name. Back lands on
the destination the task was opened from, at the scroll it was left
at. While a stick is written there is no way back.

## This computer

| Control | What it does |
|---|---|
| **This computer** | the card's heading |
| the computer's name | its maker and model ("ExampleMaker Laptop 14", as the computer reports them); **This computer** when neither could be read. On a Mac, its name from the Starter's table of Macs, then its model identifier, then **Apple T2 Security Chip** on a Mac that has it |
| **Secure Boot: on**, **Secure Boot: off**, **Secure Boot: not read** | whether Secure Boot is on, as the system reports it. Not shown on a Mac |
| **This computer cannot start a Faraday stick** | in the warning tone, on an ARM computer. The checklist then holds only **Make a Faraday stick**, for another computer |
| **Started in legacy BIOS mode: UEFI has to be on** | in the warning tone, on a computer started in legacy BIOS mode |
| **This Mac's firmware cannot start Faraday**, **This Mac cannot start Faraday yet** | in the warning tone, on a Mac with 32-bit firmware and on a Mac with Apple silicon |
| "Restart from the stick did not happen: …" | in the warning tone, on Linux, when the last **Restart from the stick** could not be set up, with the reason. The button is then **Restart into the BIOS** |
| "An encrypted drive Faraday does not manage: …" | in the warning tone, on Windows, when another encryption product's driver is found, by its name. The button is then **I have its rescue disk or password** |
| the reason a restart failed | in the error tone, under the facts |
| **Checklist** | see [The checklist](#the-checklist) |
| the button | the card's one primary button, the next step for this computer: see [The card's button](#the-cards-button) |

### The checklist

A numbered row for each item that applies to this computer, read again
each time Home is drawn. The row of the next step is highlighted; a
done row is ticked. A press on a row opens it.

| Row | Second line | Opens |
|---|---|---|
| **Make a Faraday stick** | **No Faraday stick attached**; with a Faraday stick attached, "Faraday stick attached · 0.1.8", or on Windows "Faraday stick attached · version not read" (ticked) | [Make a Faraday stick](starter-make.md#make-a-faraday-stick) |
| **Start this computer from the stick** | **Secure Boot: on**, **Secure Boot: off** or **Secure Boot: not read**. Never ticked: it is the next step once a stick is attached | [Start this computer from the stick](starter-learn-and-boot.md#start-this-computer-from-the-stick) |
| **Start this Mac from the stick** | on an Intel Mac, in place of the row above: **Option key at power-on**, **Startup Security, then the Option key** (a Mac with the T2 chip), or **Option key at power-on · Startup Security on a T2 Mac** (a Mac the table does not know) | [the Mac's steps](starter-learn-and-boot.md#on-an-intel-mac) |
| **Turn Secure Boot back on** | **Secure Boot: off**. Listed while Secure Boot is off, never marked as the next step; the link **What Secure Boot and BitLocker are** under it opens that Learn page | [Start this computer from the stick](starter-learn-and-boot.md#start-this-computer-from-the-stick) |

Only **Make a Faraday stick** is listed on a computer that cannot start
a stick.

### The card's button

The first that applies:

| This computer | Button | Press |
|---|---|---|
| no Faraday stick attached, or a computer that cannot start one | **Make a Faraday stick** | opens [Make a Faraday stick](starter-make.md#make-a-faraday-stick) |
| an Intel Mac | **Start this Mac from the stick** | opens [the Mac's steps](starter-learn-and-boot.md#on-an-intel-mac) |
| Windows, another encryption product found and not yet confirmed | **I have its rescue disk or password** | confirms it; the button then becomes the restart |
| Secure Boot off, and no restart from the stick has failed | **Restart from the stick** | opens [the restart sheet](starter-learn-and-boot.md#restart-into-the-bios-and-restart-from-the-stick) |
| otherwise | **Restart into the BIOS** | opens [the restart sheet](starter-learn-and-boot.md#restart-into-the-bios-and-restart-from-the-stick) |

A restart button is grey while something attached holds it (below).

## What can record the screen or the keys

Between the This computer card and the other cards, when something
attached to this computer can record what is shown or typed:

| Control | What it does |
|---|---|
| "Samsung QN65 is a TV. A TV can record what it shows: use another screen or computer." | a screen read as a TV, by its name (**The screen** when it has none). Holds the restart buttons |
| "… is a KVM switch. It can capture the screen and type: connect the screen and keyboard directly." | a KVM switch or IP-KVM, by its name (**A USB device** when it has none). Holds the restart buttons |
| "… is a virtual machine. Faraday must start on the real computer, and this app cannot see the host's screens and devices." | the Starter runs in a virtual machine (**This computer** when its firmware names none). Holds the restart buttons, with no **Continue anyway** |
| **Continue anyway** | under a TV's or a KVM's line: opens [Continue with … attached?](#continue-with--attached) |
| **Check again** | reads what is attached again now |
| **Screens and devices that can record** | opens that Learn page |
| **Wireless receiver attached. A wireless keyboard's typing can be picked up nearby: use a wired keyboard or the on-screen keyboard.** | in the warning tone, while a wireless receiver is attached. Holds nothing |
| **Logitech Bolt receiver attached. A Bolt keyboard's typing is encrypted.** | a plain line, in place of the one above, for a Logitech Bolt receiver. Holds nothing |

The same lines are at the top of [Start this computer from the
stick](starter-learn-and-boot.md#start-this-computer-from-the-stick).

## The other cards

| Card | Shown | Holds |
|---|---|---|
| **Faraday Demo** | on Windows and Linux | "Version 0.1.8", the Starter's own version; the reason in the warning tone when the Demo could not be opened; **Open Faraday Demo**, which shows [Faraday Demo](../walk/faraday-demo.md) in this window, under a top app bar that reads **Faraday Demo** with the way back to **Home** |
| **Update the installed Faraday** | while this copy is newer than the one installed | "Installed 0.1.7 · this copy 0.1.8" and **Update**, which opens [Install](starter-menu.md#install-and-update) as Update |
| **Boot entry** | on Linux, while the Faraday stick's entry is in the firmware's boot list and no Faraday stick is attached | **In this computer's boot list**, **Removing** while the removal runs, or why it failed in the warning tone; **Remove the Faraday boot entry**, which removes the entry |

On a wide window the cards sit in columns under the This computer
card, which spans them all; on a narrower one, in one column.

## Continue with … attached?

**Continue anyway** asks once, naming the item: "Continue with Samsung
QN65 attached?"

| Control | What it does |
|---|---|
| the item's line | as Home shows it, in the warning tone |
| **Until this app is closed.** | how long it lasts |
| **Only for a screen or switch that is your own, in front of you.** | the case the hold is for |
| **Hold to continue** | held to the end, lets the item through until the Starter is closed: the restart buttons open. Let go before, nothing happens |
| **Cancel** | closes the sheet |

**Why:** the Learn pages [Screens and devices that can record](../../learn/faraday/boot/screens-that-record.md)
and [What Secure Boot and BitLocker are](../../learn/faraday/boot/secure-boot-bitlocker.md).
