# Hardware: boards, platforms and screens

**Kind:** Reference
**Status:** Built: every platform and screen Faraday is built for, and the boards considered and not supported

Where Faraday runs, on which screen, with what layout and motion, how it
reads codes and files, and how it is kept off the network. Each row
points to the document that holds the detail; this one collects them.
The website's hardware page follows this document and is never ahead of
it.

## 1. Platforms

| Platform | Image or file | Runs on | State |
|---|---|---|---|
| The stick | `faraday-stick-X.Y.Z-x86_64-uefi.img.xz` | a 64-bit PC with UEFI firmware; an Intel Mac (`docs/MAC.md` §1, §3) | released; booted in QEMU with OVMF (`faraday/image/README.md`, "The boards") |
| The Pi | `faraday-pi-X.Y.Z-pi3-waveshare-28dpi.img.xz` | a Raspberry Pi 3B or 3B+ with the Waveshare 2.8-inch DPI panel | released, tested |
| The Pi Zero 2 W | `faraday/image/boards/pi02w` | a Raspberry Pi Zero 2 W with the same panel | built, not released, not tried: nobody has booted it |
| The Apple Silicon stick | `docs/MAC.md` §4 | Macs with M1 or M2 | planned: its Spec is `docs/MAC.md` |
| Faraday Demo | inside the Linux and Windows Starters (`docs/DEMO.md` §1) | Linux and Windows, x86_64 | released, for trying Faraday: not air-gapped |
| The Starter | `Faraday-Starter-X.Y.Z-Linux.tar.gz`, `-Windows.exe` (`docs/STARTER.md` §1.1) | Linux and Windows, x86_64; macOS specified (`docs/MAC.md` §6) | released on Linux and Windows |

The Starter writes sticks and Pi cards and holds no keys. Faraday Demo
runs on a computer that is on a network. It is a simulator with
Practice's samples (`docs/DEMO.md`), not a signing device.

## 2. Screens and layout

A screen's size class comes from its width in dp at its density
(`docs/LAYOUT.md` §2), divided by the text size factor; a larger text
steps the layout down. Navigation by class is LAYOUT §3.

| Screen | Pixels | Density | dp | Class, navigation |
|---|---|---|---|---|
| Waveshare 2.8-inch DPI panel (the Pi) | 480 × 640, portrait natively | 286 dpi (`faraday/image/panels/waveshare-28dpi/panel.conf`) | 268 × 358 | compact and short: Home is the menu, every other page has its bar with a back chevron; no Rotation (`docs/ORIENTATION.md` §5) |
| A 7-inch panel, landscape (a small tablet, a panel PC or a portable monitor on the stick) | 1024 × 600 | 170 dpi, from its EDID | 964 × 565 | expanded and low: the navigation rail |
| A 7-inch panel, portrait | 600 × 1024 | 170 dpi | 565 × 964 | compact: the navigation bar at the foot |
| A 10-inch panel, portrait | 800 × 1280 | 150 dpi | 853 × 1365 | expanded: the rail |
| A 10-inch panel, landscape | 1280 × 800 | 150 dpi | 1365 × 853 | large: the expanded rail |
| A PC monitor or laptop screen on the stick | its firmware mode | `faraday.dpi=160`, doubled at 2800 pixels wide or more (F101) | its pixels, or half | expanded to extra-large: the rail or the expanded rail |
| Faraday Demo's window | 1280 × 800 at first, any size | the system's scale | its size | any class, laid out again at every size; the Size menu shows the panels above (`docs/PREVIEW-SIZE.md`) |

A 7-inch or 10-inch panel starts in landscape, and Settings › Display ›
Rotation turns it and its touch (`docs/ORIENTATION.md`). A screen of
11 inches or less takes its density from its EDID (ORIENTATION §2.1).
There is no 7-inch or 10-inch panel for the Pi: `faraday/image/panels/`
holds the 2.8-inch panel and the PC's firmware framebuffer.

## 3. Motion

Every platform starts with full motion. Reduce motion (Settings ›
Appearance) is off on a first start (`docs/MOTION.md` §3.5). Where frames
are slow, the automatic gate turns fades off by itself: it does so when
the steps are more than 35 ms apart, and brings them back after 24 fast
frames in a row (`docs/MOTION.md`).

## 4. Input, camera and storage

The stick's and the Pi's kernels carry only the drivers
`docs/SECURITY-MODEL.md` lists ("What the kernel can reach").

| | The stick (PC, Intel Mac) | The Pi |
|---|---|---|
| Touch, keyboard, pointer | built-in keyboard, touchpad and touchscreen; USB ones once approved | the panel's touch only: no HID driver |
| Camera, for QR codes | a USB webcam, plugged in or built in, that offers uncompressed frames; not a MIPI camera (`faraday/image/README.md`, "USB is kept for a webcam") | a Pi camera or a USB webcam |
| Files | USB sticks; an SD card only in a USB reader | its own SD slot; no USB storage |

On the stick and the Pi, FAT is read by `faraday-disk` in Rust, not by
the kernel. Faraday Demo reads files through the computer's own system
and opens its camera through V4L2, Media Foundation or AVFoundation.

## 5. How each is kept off the network

- **The stick and the Pi.** The network stack is not compiled in: no
  Ethernet, Wi-Fi, Bluetooth or USB network driver, and no socket layer
  (`docs/SECURITY-MODEL.md`, "No networking"). A Pi 3's Wi-Fi and
  Bluetooth chip is never given its firmware, so it never runs. A PC's
  own disks are invisible.
- **Faraday Demo and the Starter** run on a computer that may be on a
  network. Neither is a place for real keys.

## 6. Considered, not supported

### 6.1 Waveshare ESP32-P4-WIFI6-Touch-LCD-7B

A 7-inch 1024 × 600 IPS board with GT911 capacitive touch, MIPI-DSI and
MIPI-CSI. It has an ESP32-P4 (two RISC-V cores, up to 400 MHz, 768 KB
SRAM, 32 MB PSRAM, 32 MB flash) and an ESP32-C6 module for Wi-Fi 6 and
Bluetooth 5. It costs about US $50. Studied without a board on
2026-10-28. Not supported, and not being ported.

On its panel Faraday would be expanded and low in landscape (964 × 565
dp, the rail) and compact in portrait. It would run under Reduce motion,
because a full frame on the P4 is estimated at 50 to 100 ms.

What carries over:

- The core crates are `no_std` already.
- `faraday-core` does no I/O: its time comes from the shell's ticks.
- libsecp256k1 builds for RISC-V.
- Rust's std on ESP-IDF (`riscv32imafc-esp-espidf`, ESP-IDF 5.2 or
  later) has threads, files and time. The app would need a new shell:
  the DSI panel, GT911 touch, the CSI camera, SD and USB through
  `faraday-fat`, the chip's TRNG, and a wipe and reset at Lock.
- The TRNG draws on ADC and ring-oscillator noise and needs no radio
  running.

What stands in the way:

1. **Vault memory.** Faraday makes vaults of at least 64 MiB of Argon2
   memory (`docs/VAULT.md` §3.1), and the board has 32 MB in all. A vault
   it could open would be made at 16 MiB or less, below Light. That is
   the owner's decision, not taken.
2. **The radio.** The C6 boots its own flash at power-on, whatever the
   P4 runs. Without a radio means the C6 module desoldered, or its flash
   erased and the erase checked. No ESP32-P4 board without a C6 was
   found. ESP32-S3 boards carry the radio in the chip and are not
   candidates.
3. **The security model.** `docs/SECURITY-MODEL.md` rests on Linux:
   - separate users and processes;
   - the disk process alone touching sticks;
   - a fresh process at Lock in pages the kernel zeroes;
   - lockdown and no modules.

   One ESP-IDF image in one address space would need a model of its
   own.
4. **Secure boot.** Espressif's advisory AR2026-006 says the P4's ROM
   ECDSA check may accept invalid signatures, so RSA-3072 secure boot
   would be used. ESP-IDF is a large body of C that Faraday has not
   reviewed (F71).

Help is welcome. The work that would decide it is a measurement on the
board before any port:

- a full frame and a scroll step drawn by `faraday-draw`;
- Argon2id at 16 MiB;
- a 640 × 480 QR frame read by rqrr;
- a PSBT signed;
- a check, with a scanner, that an erased C6 stays silent.

Sources: Waveshare's wiki and FAQ for the board, Espressif's ESP32-P4
product page, ESP-IDF's random number documentation, Espressif's
security advisories, and Rust's ESP-IDF platform-support page, read
2026-10-28.
