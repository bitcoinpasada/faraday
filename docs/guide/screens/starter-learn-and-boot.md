# Faraday Starter: Start from the stick, and Learn

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-starter/src/draw.rs`, `faraday/faraday-starter/src/notes.rs`, `faraday/faraday-starter/src/decide.rs`, `faraday/faraday-starter/src/lib.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Starter page:** `Start`
**Starter sheet:** `Restart`
**Group:** Starter

The page that walks this computer through starting from the stick, the
sheet every restart asks first, and the Starter's Learn pages. The
walkthrough is [Start a PC from the stick](../walk/start-from-the-stick.md).

## Start this computer from the stick

**Opened from:** Home's checklist (**Start this computer from the
stick**, **Turn Secure Boot back on**), and **Start this computer from
the stick** on [Written](starter-make.md#written-or-not-written). Its
way back is the destination it was opened from. On an Intel Mac it is
**Start this Mac from the stick** ([below](#on-an-intel-mac)).

*Picture to come: `starter-start.png`, Start this computer from the stick for a known maker, Secure Boot on.*

The steps shown follow this computer's state, from the top:

| Control | What it does |
|---|---|
| **A disk here is unlocked by the TPM: have its passphrase ready** | in the warning band, on Linux with a disk the TPM unlocks |
| **This computer cannot start a Faraday stick** | on an ARM computer: the only line on the page |
| **Started in legacy BIOS mode: UEFI has to be on** | in the warning band, on a computer started in legacy BIOS mode |
| the lines of what can record | as on [Home](starter-home.md#what-can-record-the-screen-or-the-keys), with **Continue anyway**, **Check again** and **Screens and devices that can record** |
| this computer's guide | a row titled by its Learn page: its maker's ("Starting an HP computer from the stick"), or **Starting a Windows computer from the stick** or **Starting a Linux computer from the stick**. A press opens it |
| **What Secure Boot and BitLocker are** | opens that Learn page |
| "An encrypted drive Faraday does not manage: …" and **I have its rescue disk or password** | on Windows, while another encryption product is not yet confirmed; the restart buttons are grey until it is pressed |

### Write down

What is needed once the computer restarts and the Starter is off the
screen, a label and its value each:

| Row | Value |
|---|---|
| **Boot menu key** | the maker's key that opens the boot menu ("F12") |
| **Usual BIOS key** | the maker's key that opens the BIOS ("F2") |
| **Secure Boot setting** | where the setting is in this maker's BIOS ("Boot Configuration → Secure Boot") |

For a maker the Starter does not know, the key rows read "Not known:
try Esc · F2 · …" with the keys to try, and the setting row **Not
known**.

| Control | What it does |
|---|---|
| **Print** | makes the sheet as a PDF (`faraday-start-from-the-stick.pdf`, in the system's temporary folder) and opens it in this computer's viewer: the three rows, a line each to circle the key that worked, and the steps. The reason, in the error tone, when the viewer cannot be opened |
| **Include the BitLocker recovery key** | beside Print, on Windows with BitLocker on or suspended on drive C:; unticked each time the Starter starts. Ticked, Print first reads the key (Windows asks once for administrator permission) and the sheet carries **BitLocker recovery key, drive C:** and its 48 digits. The key is never drawn on the page |
| **Waiting for administrator permission** | while Windows asks |

### BitLocker

On Windows with BitLocker on or suspended on the system drive, under
Write down. Nothing waits on it.

| Control | What it does |
|---|---|
| "BitLocker, drive C:" | the drive's state: **On**, "Suspended · turns back on by itself after Windows starts 3 times", **Suspended · turns back on by itself after Windows starts once**, **Suspended until resumed**, or **Suspended** |
| **Temporarily suspend BitLocker** | while on: suspends it for the next three starts of Windows (Windows asks for administrator permission). "For the next 3 times Windows starts; then it turns back on by itself" under it |
| **Resume BitLocker** | while suspended: turns it back on now |
| **Waiting for administrator permission** | while Windows asks |
| the reason | in the error tone, when BitLocker refused ([what each says](../walk/start-from-the-stick.md#17-start-this-computer-from-the-stick)) |

### The steps

| Step | Under it |
|---|---|
| 1. **Restart into the BIOS** | the button **Restart into the BIOS**: opens [the restart sheet](#restart-into-the-bios-and-restart-from-the-stick). Grey while something attached holds it, or while another encryption product is not confirmed. The reason a restart failed, in the error tone |
| 2. **Turn Secure Boot off** | **Turn Fast Boot off where it hides USB**, then **If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.** (on Linux, **If the Secure Boot setting cannot be found: exit the BIOS without saving changes. The computer starts as before.**). With Secure Boot off already, the step reads **Secure Boot is already off** and is ticked |
| 3. **Start from the stick** | **The boot menu key**; on Windows with Secure Boot off, **The boot menu key, or Use a device in Advanced startup**, and, with a Faraday stick attached, the button **Restart to choose a device**, which opens the restart sheet as Restart from the stick. With a Faraday stick attached, **In the boot menu**, "something like UEFI: SanDisk Ultra, Partition 1" |
| 4. **Turn Secure Boot back on before starting Windows** | on Linux, **Turn Secure Boot back on**. Shown always |

**On a phone**, at the foot: the QR code of this computer's guide on
the website, and its address under it.

### On an Intel Mac

**Start this Mac from the stick**: the Mac's steps, none of them a
button. A Mac with 32-bit firmware shows only **This Mac's firmware
cannot start Faraday**; a Mac with Apple silicon, only **This Mac
cannot start Faraday yet**.

| Control | What it does |
|---|---|
| **Starting an Intel Mac from the stick** | the guide's row: opens that Learn page |
| **Boot menu key** | **Option, held** |
| **macOS Recovery key** | on a Mac with the T2 chip: **Command-R, held** |
| **Startup Security** | on a Mac with the T2 chip: **macOS Recovery → Utilities → Startup Security Utility** |
| **Print** | as above; the Mac's sheet has no keys to circle |

On a Mac the table does not know, the last two rows read "macOS
Recovery key, only on a Mac with the T2 chip" and "Startup Security,
only on a Mac with the T2 chip", and the T2 steps carry **Only on a Mac
with the T2 chip**. A Mac known to have no T2 chip has only the
**Boot menu key** row and one step, **Start from the stick**.

| Step | Under it |
|---|---|
| 1. **Lower Startup Security** | on a Mac with the T2 chip: **macOS Recovery: hold Command-R at power-on**, **Utilities, Startup Security Utility: No Security, and Allow booting from external or removable media** |
| 2. **Start from the stick** | **Shut down. Hold Option at power-on and choose EFI Boot**, **A firmware password, when one is set, is asked for first** |
| 3. **Afterwards: Startup Security back to Full** | on a Mac with the T2 chip: **macOS Recovery, Startup Security Utility: Full Security, and Disallow booting from external or removable media** |

**On a phone** and its QR code close the page, as above.

## Restart into the BIOS and Restart from the stick

**Opened from:** Home's card button and the Start page's **Restart
into the BIOS** and **Restart to choose a device**. The sheet asks once
before the computer restarts.

*Picture to come: `starter-restart.png`, the Restart into the BIOS sheet with its QR code.*

| Control | What it does |
|---|---|
| **Restart into the BIOS**, **Restart from the stick** | the sheet's title, with the power mark |
| **This restarts the computer now. Save your work first.** | in the warning tone |
| **Keep the stick in.** | the stick stays in for the start that follows |
| **Windows shows Choose an option: Use a device, then the stick.** | Restart from the stick, on Windows |
| **About the boot entry** | Restart from the stick, on Linux: opens the Learn page on the firmware's Faraday stick entry |
| the QR code | this computer's guide on the website, with its address under it |
| **Cancel** | closes the sheet |
| **Restart** | restarts now. Restart into the BIOS restarts into the firmware's settings. Restart from the stick: on Linux the system asks for the password, then the computer starts from the attached stick once; on Windows, Windows' Choose an option follows |

When the restart from the stick cannot be set up on Linux, Home's card
says why and offers **Restart into the BIOS** instead.

## Learn

**Opened from:** **Learn** on the rail or the bar. A link elsewhere in
the Starter (a guide's row, **What Secure Boot and BitLocker are**,
**Screens and devices that can record**, **About the boot entry**,
**Replacing Faraday on a stick**) opens its one page as a page inside
a task, with the way back to where it was opened.

*Picture to come: `starter-learn.png`, the Learn destination with the rail.*

Learn is Faraday's own Learn, drawn as the page under the Starter's top
app bar, with the rail beside it. It holds the Starter's pages, in this
order:

- this computer's maker's page, when its maker has one (on an HP,
  [Starting an HP computer from the stick](../../learn/faraday/boot/maker-hp.md));
- [Starting a Windows computer from the stick](../../learn/faraday/boot/windows.md);
- [Starting a Linux computer from the stick](../../learn/faraday/boot/linux.md);
- [When the stick does not start](../../learn/faraday/boot/not-starting.md);
- [Replacing Faraday on a stick](../../learn/faraday/boot/replace-faraday.md);
- [What Secure Boot and BitLocker are](../../learn/faraday/boot/secure-boot-bitlocker.md);
- [Screens and devices that can record](../../learn/faraday/boot/screens-that-record.md);
- [The Faraday boot entry on Linux](../../learn/faraday/boot/boot-entry.md);
- [Starting an Intel Mac from the stick](../../learn/faraday/boot/mac-intel.md);
- [What else runs on this computer](../../learn/faraday/boot/what-else-runs.md).

The same pages are on the website, where the Start page's QR code
leads.
