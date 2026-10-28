# Get Faraday on a stick

**Kind:** Reference
**Status:** Written, checked against the app's screens and strings, and pictured from `just guide-pictures` (`docs/guide/pictures/`)

**You end with:** a USB stick holding Faraday, written by Faraday
Starter and read back.

**Before you start:** [Faraday Starter](starter-tour.md).

**You'll need:**

- A PC with a USB port, running Windows or Linux.
- A USB stick of 256 MB or larger whose contents can be lost.

**Time:** about 15 minutes.

Faraday runs from a USB stick on a PC. Faraday Starter, an app for
Windows and Linux, writes the stick and helps start the PC from it.
Faraday Demo runs Faraday in a window on testnet, for trying it out
([Faraday Demo](faraday-demo.md)).

### 1.1 Download the Starter

From the release page, download `Faraday-Starter-X.Y.Z-Windows.exe` or
`Faraday-Starter-X.Y.Z-Linux.tar.gz`, and `SHA256SUMS` and
`SHA256SUMS.asc`. Check them as the README's "Verifying a release" says.

The check proves that the file is the one the release key signed. It
does not prove who holds the release key: compare the key's fingerprint
with one from a second place.

### 1.2 Open the Starter

- Windows: open the `.exe`. Windows shows "Windows protected your
  PC" the first time: press *More info*, then *Run anyway*.
- Linux: extract the `.tar.gz` and open the file inside.

The Starter opens on its [Home](../screens/starter-home.md).

![The Starter's Home with no stick attached, Make a Faraday stick outlined](../pictures/1-1-starter-home.png)

The rail at the left holds **Home**, **Make**, **Tools** and **Learn**;
in a narrow window they are a bar at the foot. The bar at the top holds
**Install** and the **⋮** menu (**About**). A step such as **Which
stick?** opens as a page of its own, with no rail: a back arrow at the
top left, the page's title, and under it the name of the page the arrow
goes back to (here **Home**).

| On screen | Meaning |
|---|---|
| **This computer** | the PC's maker and model, and **Secure Boot: on** or **off** |
| **Checklist** | **1 Make a Faraday stick**, then **2 Start this computer from the stick**; the step to do now is highlighted |
| the button on the card | the next step for this PC: here **Make a Faraday stick** |
| **Faraday Demo**, **Version …**, **Open Faraday Demo** | the card under it opens [Faraday Demo](faraday-demo.md) |

The card says so when this PC cannot start the stick, and when
something attached to it can record the screen or the keys:

| Card says | Meaning | Do |
|---|---|---|
| **This computer cannot start a Faraday stick** | an ARM PC, such as a Windows laptop with a Snapdragon processor | make the stick here and start another PC from it |
| **Started in legacy BIOS mode: UEFI has to be on** | the PC started in legacy mode | set the BIOS's boot mode to UEFI before step 1.8 |
| "… is a TV. A TV can record what it shows: use another screen or computer." | a screen is a TV | use another screen, then **Check again** |
| "… is a KVM switch. It can capture the screen and type: connect the screen and keyboard directly." | a KVM switch is attached | connect the screen and keyboard directly, then **Check again** |
| "… is a virtual machine. Faraday must start on the real computer, …" | the Starter runs in a virtual machine | run the Starter on the real PC |

While a TV, a KVM switch or a virtual machine is listed, the restart
buttons stay grey. **Continue anyway** lets the steps go on.

Press **Make a Faraday stick**.

### 1.3 Which stick?

Plug the stick in. Every USB disk attached is listed with its size and
what is on it ([Which stick?](../screens/starter-make.md#which-stick-or-which-card)). Nothing is chosen for you.

![Which stick? with two USB Flash Drives listed, the empty 32 GB one outlined](../pictures/1-2-which-stick.png)

Press the stick to write. Check the size: a backup drive is listed the
same way as a stick.

| Row says | Meaning |
|---|---|
| **Empty** | nothing is on the stick |
| a volume name, then **files on it** | the stick holds files; writing erases them |
| **Faraday stick ·** and a version | the stick already holds Faraday |
| dimmed, **Smaller than the image: needs …** | the stick is too small; it cannot be pressed |
| dimmed, **Used by this computer: mounted at …**, **Used by this computer: swap**, **Used by this computer: another device is built on it**, **Not checked: what uses it could not be read** or **Holds this computer's system** | the PC uses this disk; it cannot be pressed |

With no USB disk attached the page reads **No USB stick attached · 256
MB or larger**. The list follows sticks as they are plugged in and
pulled out: a stick not listed is pulled out and plugged in again.

### 1.4 What to write?

![What to write? for an empty stick, New Faraday stick and The release's (unsigned) chosen, Continue outlined](../pictures/1-3-what-to-write.png)

**New Faraday stick** (Everything on it is lost) is chosen, the only
choice for this stick ([What to write?](../screens/starter-make.md#what-to-write)).
Under **Boot file**, **The release's (unsigned)** is chosen; **One I
signed** is for a boot file signed with your own Secure Boot keys
([Your own Secure Boot keys](secure-boot-keys.md)). Press **Continue**.

A stick that already holds Faraday also offers keeping its data, for
upgrading: [Replace Faraday, keep the data](starter-tools.md#5-replace-faraday-keep-the-data).

### 1.5 Confirm

![Confirm, the stick's rows and the image's SHA-256 above Erase and write](../pictures/1-4-confirm.png)

Check **Stick**, **Size** and **On it** against the stick in your hand,
then press **Erase and write**. On Windows, Windows asks for
administrator permission: answer *Yes*. On Linux the system may ask
for your password.

Every row: [Confirm](../screens/starter-make.md#confirm).

| Row | What it is |
|---|---|
| **Device**, **Serial** | the system's name for the disk, and its serial number |
| **Writes** | **New Faraday stick** |
| **Boot file** | **The release's (unsigned)** or **One I signed** |
| **Build** | the Faraday version written |
| **SHA-256 of the image** | the image's sum; it is the `.img` line of the release's `SHA256SUMS` |

**Not the stick confirmed: it was removed or changed** under the rows
means the stick was pulled out or another disk took its place. The
button is grey. Press **Choose again** and choose the stick again.

### 1.6 Writing, then Written

![Writing, a bar and Written 67 MB of 202 MB](../pictures/1-5-writing.png)

The page is **Writing**, with a bar and how much is written. The
Starter writes the image, then reads it back to check it
([Writing](../screens/starter-make.md#writing)). Leave the
stick in until the result shows.

| Result | Meaning | Press |
|---|---|---|
| ![Written, In the boot menu and Start this computer from the stick](../pictures/1-6-written.png) | **Written**: the stick holds Faraday and was read back. "Keep the stick in to start this computer from it. To start another computer, pull it out." **In the boot menu** is roughly the name the PC's boot menu will show for the stick. | **Start this computer from the stick**, with the stick still in; **Done** goes back to Home |
| ![Not written, a read-back error in red above Try again](../pictures/1-6-not-written.png) | **Not written**: the reason is in red (below). Nothing on the stick can be trusted. | **Try again**, or as the reason's row says |

| Red line begins | Meaning | Do |
|---|---|---|
| "Reading back …: what was read differs from what was written at byte …" | the stick did not keep what was written | **Try again**; if it fails again, use another stick |
| "Reading back … failed", "Writing failed", "Flushing … failed" | the system reported an error from the stick | pull the stick out, plug it in, **Try again**; then another USB port or stick |
| "… is no longer the disk that was confirmed …", "The disk is no longer attached." | the stick was pulled out or replaced; nothing was written | plug it in and start again from **Make a Faraday stick** |
| "A partition on it is mounted at …", "Unmounting it failed" | the system is using the stick; nothing was written | close any window showing the stick's files, **Try again** |
| "Administrator permission was declined. Nothing was changed." | the permission prompt was declined | **Try again** and answer *Yes* |
| "The payload file does not match the manifest", "The .xz file is damaged", "The decompressed image's SHA-256 is …, not the expected one" | the Starter's own copy of Faraday is damaged | download the Starter again and check it (step 1.1) |
