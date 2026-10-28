# Faraday Demo

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; the pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** Faraday Demo open, ready to follow the rest of this
guide in a window.

**You'll need:**

- Faraday Starter, open on a computer running Windows or Linux
  ([Faraday Starter](starter-tour.md)).

**Time:** about 10 minutes.

## 1. Open Faraday Demo

Faraday Demo is Faraday in a window: the same app as on the stick, on
an online computer. It runs on testnet only, the test network where
coins have no value: its modes are **Practice**, **Testnet**, **Signet**
and **Regtest**, never Mainnet. It has none of the stick's protections.
Never type a real key's words into it. Every field that takes a seed
says so: "Never type a seed that holds real bitcoin here."

Every walkthrough of the app in this guide, from
[First start, Home and the session](first-start.md) on, can be followed
in Faraday Demo, in Practice (step 2). Writing a stick and
starting a computer from it are done in the Starter itself.

On Home, press **Open Faraday Demo** on the **Faraday Demo** card. The
Demo fills the window under the Starter's top bar, which reads
**Faraday Demo** and holds the back arrow.

*Picture to come: `starter-tour-demo-home.png`, Faraday Demo's Home in Practice, with the strip at the window's foot.*

The Demo is in two parts:

- The app: everything above the strip, exactly as the device's
  screen shows it at this size.
- The strip, along the window's foot: what exists only because this
  is a window and a demo. Its colours are the app's turned inside out
  (a dark strip under a light app), so it is never taken for part of
  the device's screen.

The Demo opens in the Starter's theme, and a theme chosen in the Demo's
Settings becomes the Starter's too, until the Starter closes.

## 2. The mode pill

The pill at the top right of the app reads **Practice · Testnet**
(**Practice** alone in a narrow window). Press it to open [**Mode**](../screens/mode.md#mode):

| Row | What it takes | Use it for |
|---|---|---|
| **Practice** (marked **Testnet**) | only the public sample keys and wallets, the guide's key `3dca6805` among them; a key that is not a sample is refused | following the guide with its key, and looking around with the samples: every screen opens, and every action that would make a new secret is grey |
| **Testnet** | any key | experiments of your own, with a key of your own |
| **Signet** | any key | as Testnet, on the signet test network |
| **Regtest** | any key | as Testnet, on a local regtest network |

The mode in use is ticked. **Not now** closes the sheet.

Changing mode locks the session: it starts again in the new mode, and
Files, the files for the stick, keys and wallets are emptied. Choose
the mode before loading anything.

In Practice, making a key runs every step, then loads the next public
sample in place of the key made, and the Key card names the sample. The
guide's dice rolls make the words of a sample, so Practice keeps them:
the strip reads **These words are sample Guide key · 3dca6805**, and
Lock in loads key `3dca6805`. In **Load samples** the guide's key is
the first of the **Keys**, **Guide key · 3dca6805**; its words typed in
**Add a key** load it too. Choose **Testnet** for experiments of your
own.
The fingerprints are the guide's in every mode. Addresses and account
keys are testnet ones (addresses begin `tb1`, not the guide's `bc1`):
a watch-only wallet used beside the Demo is set to testnet too.

*Picture to come: `starter-tour-mode.png`, the Mode sheet with Practice ticked.*

## 3. The strip

The strip holds, from the left:

| Control | What it does |
|---|---|
| **Boot stick · in** | the stick Faraday started from. A press pulls it out or puts it back in |
| **Stick 2 · out** | a backup stick. A press puts it in or pulls it out. A backup that writes several sticks adds a pill for each |
| **Sticks · 3** | where the pills do not all fit: a menu of every stick, in or out, each with its dot |
| **Samples** | in Practice: opens **Load samples** |
| **Size · This window** | opens the Size menu: **This window**, **Pi panel**, **Narrow window**, **Half screen**, **7-inch panel**, **10-inch panel**, **Desktop window**, **Full monitor**. Another size shows the app at that screen's size, centred above the strip |
| **Practice**, then the screen's aids | in Practice only, on a screen that has them (below) |

A stick the app asks for, for example when a sheet says to pull every
stick or a backup asks for its next stick, has its pill outlined and
pulsing.

Each strip control that opens a menu opens it upwards from the strip:
Sticks, Size, **More**, **Play options** and **Rolls per click · 1**. A
choice acts and closes the menu; a press outside closes it. Where the
window is narrow, the strip shortens labels, moves aids into **More**,
and scrolls sideways.

### The sticks are folders

Each stick is a folder in `faraday-sticks` in the home folder
(`~/faraday-sticks` on Linux, `%USERPROFILE%\faraday-sticks` on
Windows):

| Folder | Stick |
|---|---|
| `BOOTSTICK` | the boot stick. It starts empty; put there the files Faraday is to find at start, such as a vault. It is in when the Demo opens, and pulling it brings its files in, as on the device |
| `STICK-2`, `STICK-3`, … | the backup sticks, made empty when each is first put in |

What the app writes to a stick is a file in its folder, so a stick
pulled and put back still holds it. The Demo never moves, renames or
removes anything in these folders, and uninstalling the Starter leaves
them.

### Load samples

**Samples** opens [**Load samples**](../screens/mode.md#load-samples) above the strip: "In Practice, the
sample passphrase is faraday", then the sample **Wallets** and
**Keys**, each with a tick box. The first of the Keys is the guide's
key, **Guide key · 3dca6805**. **Select all** ticks every row. **Load
ticked** loads them; **Not now** closes the sheet. With a stick in, the
app first asks for it to be pulled, as on the device.

*Picture to come: `starter-tour-samples.png`, Load samples open over the strip.*

### The screen's aids

In Practice, a screen with a stand-in for the person's own input hands
it to the strip, in a section of its own colour headed **Practice**:

| Aid | Where it is offered | What it does |
|---|---|---|
| **Use a sample** | **Add a key**, a typed key, the sample vault, a cosigner's key in Restore, and others | puts a public sample in the field |
| **Use faraday** | every passphrase field | types `faraday`, Practice's one passphrase |
| **Paste words** | a key's words (on the test networks too) | puts the clipboard's words in whole |
| the die, **Roll** and **Rolls per click · 1** | the dice of a new key or of a vault's passphrase | rolls for you (1, 5, 10, To the end, or a number typed) |
| **Load sample seed**, **Load sample PSBT** and the like | the scanner, and the Spend tab | a public sample in place of a scan |
| **Fill words** | Check my copy | types the sample's words back |
| **Use a sample**, **Previous**, **Next**, **Play** and **Play options** | the entry of Make codex32 shares | fills the entry with BIP 93's published sample set, or steps through the computer's rolls; once the entry is done, the sample takes the place of the strings entered, and **Lock in** loads the sample's key |

A line beside them says what Practice puts in place of the person's own,
such as "Practice puts sample … in place of these strings". On Testnet,
Signet and Regtest the strip holds the sticks and Size, and **Paste
words** where a key's words are typed.

*Picture to come: `starter-tour-aids.png`, Add a key in Practice, with Use a sample and Paste words in the strip.*

## 4. Back and Power off

| To | Press |
|---|---|
| return to the Starter's Home | the back arrow in the Starter's top bar. The Demo's session ends: the next **Open Faraday Demo** starts afresh, in Practice. The window's close button and the Demo's own Quit do the same |
| quit the Demo from inside it | the power button at the foot of the sidebar. The sheet is **Quit**, "Faraday closes", with **Cancel** and **Quit**. While files wait for a stick or a vault has unsaved changes, the button reads **Hold to quit**: hold it until it fills |
| wipe the session and go on | **Lock**: the Demo starts a fresh session in place, in the same mode |

Keys pressed while the Demo is on screen are the Demo's own:
<kbd>Escape</kbd> goes back inside the app, not to the Starter.

Next: [First start, Home and the session](first-start.md), followed in
the Demo, or [Get Faraday on a stick](get-faraday.md).
