# The Starter and Faraday Demo

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; the pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** knowing where everything is in Faraday Starter, and
Faraday Demo open, ready to follow the rest of this guide in a window.

**You'll need:**

- A computer running Windows or Linux.
- Faraday Starter, downloaded and checked as
  [Get Faraday on a stick](get-faraday.md#11-download-the-starter),
  step 1.1, says.

**Time:** about 20 minutes.

Faraday Starter is an app for Windows and Linux. It writes the Faraday
stick, helps start a computer from it, prints blank backup templates,
moves files to and from the device by QR code, and holds Faraday Demo.
It runs without being installed.

## 1. Open the Starter

- Windows: open the `.exe`. The first time, Windows shows "Windows
  protected your PC": press *More info*, then *Run anyway*.
- Linux: extract the `.tar.gz` and open the file inside.

The Starter opens on **Home**. It opens in the **Cobalt** theme at
**Standard** contrast, in **Light** or **Dark** as the computer's own
setting asks.

*Picture to come: `starter-tour-home.png`, the Starter's Home with the rail, This computer and its Checklist, and the Faraday Demo card.*

The rail at the left holds the four destinations: **Home**, **Make**,
**Tools** and **Learn**. In a narrow window they are a bar across the
foot of the window. The bar at the top holds **Install** (or
**Update**) and the **⋮** menu.

A step inside a task, such as **Which stick?**, opens as a page of its
own: the rail is gone, and the top bar holds a back arrow and the
page's name. The back arrow returns to the destination the task was
opened from, at the place it was left.

## 2. Home

Home is about this computer.

| On screen | Meaning |
|---|---|
| **This computer** | the computer's maker and model, and **Secure Boot: on** or **off** |
| **Checklist** | **Make a Faraday stick**, then **Start this computer from the stick**; the step to do now is marked. **Turn Secure Boot back on** is listed while Secure Boot is off |
| the button on the card | the next step for this computer: **Make a Faraday stick**, **Restart into the BIOS** or **Restart from the stick** |
| **Faraday Demo** | the Demo's version and **Open Faraday Demo** (step 7) |
| **Update the installed Faraday** | shown only while this copy is newer than the one installed: **Installed … · this copy …** and **Update** |
| **Boot entry** | on Linux, shown only while the Faraday stick's boot entry is in this computer's boot list: **Remove the Faraday boot entry** |

The checklist is read again each time Home is drawn: plugging a
Faraday stick in ticks **Make a Faraday stick**. A restart button asks
once before the computer restarts: "This restarts the computer now.
Save your work first.", with **Restart** and **Cancel**.

The stick and the restart are the next two walkthroughs:
[Get Faraday on a stick](get-faraday.md) and
[Start a PC from the stick](start-from-the-stick.md).

## 3. Make

*Picture to come: `starter-tour-make.png`, Make with its two cards.*

| Card | Shows | Button |
|---|---|---|
| **Faraday stick** | **No Faraday stick attached**, or the attached stick's Faraday version | **Make a Faraday stick**: writes a USB stick, step by step (**Which stick?**, **What to write?**, **Confirm**, **Writing**, **Written**) |
| **Pi card** | **Version …** of the Pi image, or **Not in this Starter** | **Make a Pi card**: writes an SD card for a Raspberry Pi the same way; grey when this Starter carries no Pi image |

A disk is never chosen for you: the one USB disk attached is as often a
backup drive as a stick.

## 4. Tools

*Picture to come: `starter-tour-tools.png`, Tools with its two cards.*

| Card | Shows | Button |
|---|---|---|
| **Transfer** | **Send and Receive** | **Open Transfer**: sends a file from this computer's Downloads folder as animated QR codes for the device's camera, and receives the device's codes through this computer's camera into Downloads |
| **Blank templates** | **Words, SeedQR and codex32** | **Open blank templates**: printable sheets to copy a seed onto by hand. They hold nothing of any wallet. Choose the rows, then **Print** (opens the PDF in this computer's viewer) or **Save the PDF** (into Downloads) |

The backup walkthrough prints its 12-word template from here
([Back it up](back-up.md)).

## 5. Learn

**Learn** holds the Starter's Learn pages: starting a Windows or a
Linux computer from the stick, one page for each computer maker, what
Secure Boot and BitLocker are, what to do when the stick does not
start, and what else runs on a computer below Faraday. A page opened
from a link elsewhere in the Starter opens as a task page, with the
back arrow.

## 6. Install, Update, Appearance and the ⋮ menu

**Install**, at the top right while nothing is installed, opens a page
with one button, **Install**. Press it: the page reads **Installed in
…** with **Done**. The Starter is then in this computer's menu as
Faraday Starter. Faraday Demo is not installed separately: it is inside
the Starter.

**Update** takes the place of **Install** while this copy is newer than
the one installed. It shows **Installed version** and **This version**,
then the same **Install** button.

The **⋮** menu holds what is not a destination:

| Row | Opens |
|---|---|
| **Appearance** | **Theme**, **Mode** (**Light**, **Dark**) and **Contrast** (**Standard**, **Medium**, **High**), the same rows as Faraday's own Settings → Appearance. A choice lasts until the Starter closes; the Starter keeps no settings file |
| **About** | the version, each image the Starter carries with its build ID and SHA-256, the release key's fingerprint, and **Checking this file**: how to check the Starter file against `SHA256SUMS` and its signature |
| **Uninstall** | shown while a copy is installed: what is installed and where, and **Uninstall**, which asks **Uninstall Faraday Starter?** first. "Removes the files listed. Received files and settings stay." |
| **Remove the Faraday boot entry** | shown while the entry is in the boot list, as on Home |

*Picture to come: `starter-tour-menu.png`, the ⋮ menu open over Home.*

## 7. Open Faraday Demo

Faraday Demo is Faraday in a window: the same app as on the stick, on
an online computer. It runs on testnet only, the test network where
coins have no value: its modes are **Practice**, **Testnet**, **Signet**
and **Regtest**, never Mainnet. It has none of the stick's protections.
Never type a real key's words into it. Every field that takes a seed
says so: "Never type a seed that holds real bitcoin here."

Every walkthrough of the app in this guide, from
[First start, Home and the session](first-start.md) on, can be followed
in Faraday Demo (step 8 says which mode to choose). Writing a stick and
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

## 8. The mode pill

The pill at the top right of the app reads **Practice · Testnet**
(**Practice** alone in a narrow window). Press it to open **Mode**:

| Row | What it takes | Use it for |
|---|---|---|
| **Practice** (marked **Testnet**) | only the public sample keys and wallets; a key that is not a sample is refused | looking around with the samples: every screen opens, and every action that would make a new secret is grey |
| **Testnet** | any key | following a walkthrough with the guide's own key, `3dca6805` |
| **Signet** | any key | as Testnet, on the signet test network |
| **Regtest** | any key | as Testnet, on a local regtest network |

The mode in use is ticked. **Not now** closes the sheet.

Changing mode locks the session: it starts again in the new mode, and
Files, the files for the stick, keys and wallets are emptied. Choose
the mode before loading anything.

In Practice, making a key runs every step, then loads the next public
sample in place of the key made, and the Key card names the sample. To
make key `3dca6805` from the guide's dice rolls, choose **Testnet**.
The fingerprints are the guide's in every mode. Addresses and account
keys are testnet ones (addresses begin `tb1`, not the guide's `bc1`):
a watch-only wallet used beside the Demo is set to testnet too.

*Picture to come: `starter-tour-mode.png`, the Mode sheet with Practice ticked.*

## 9. The strip

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

**Samples** opens **Load samples** above the strip: "In Practice, the
sample passphrase is faraday", then the sample **Wallets** and
**Keys**, each with a tick box. **Select all** ticks every row. **Load
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

## 10. Back and Power off

| To | Press |
|---|---|
| return to the Starter's Home | the back arrow in the Starter's top bar. The Demo's session ends: the next **Open Faraday Demo** starts afresh, in Practice. The window's close button and the Demo's own Quit do the same |
| quit the Demo from inside it | the power button at the foot of the sidebar. The sheet is **Quit**, "Faraday closes", with **Cancel** and **Quit**. While files wait for a stick or a vault has unsaved changes, the button reads **Hold to quit**: hold it until it fills |
| wipe the session and go on | **Lock**: the Demo starts a fresh session in place, in the same mode |

Keys pressed while the Demo is on screen are the Demo's own:
<kbd>Escape</kbd> goes back inside the app, not to the Starter.

Next: [Get Faraday on a stick](get-faraday.md), or, to follow the guide
in the Demo first, [First start, Home and the session](first-start.md).
