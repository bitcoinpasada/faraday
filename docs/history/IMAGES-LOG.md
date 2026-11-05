# Images for each piece of hardware: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/IMAGES.md` on 2026-11-05. Each keeps its
original number. The step numbers in it are the ones old commits use.

## 9. Order of work (from docs/IMAGES.md)

1. One Pi image for every supported Pi: the 3B, 3B+ and Zero 2 W
   device trees on one card (built: the Zero 2 W's board directory is
   folded into the Pi 3's), tried on the 3B and the Zero 2 W.
2. The Pi 4 on the same images: its 32-bit kernel, firmware and device
   tree on every Pi card, the system the same. First, for the owner:
   what it adds to the kernel's reach (USB behind PCIe, a boot loader in
   the board's EEPROM, against `docs/SECURITY-MODEL.md`), and whether
   the 2.8-inch image boots on the board with the Pi's kernel. Approved by the owner (F150) on these
   terms: no boot loader update is ever on a card; SPI_SPIDEV and
   SPI_BCM2835 are forbidden in the kernel; the Pi 4 is described as close
   to a PC, its boot code on the board, and its owner is told to restore
   Raspberry Pi's boot loader with Raspberry Pi Imager before first use
   (a wire to test point TP5 locks it in hardware).
   Paused by the owner, as macOS is: nothing is built for it until
   the owner takes it up again.
3. The Pi's kernel: its fragment, its forbidden and required lists,
   and the image that boots with it, starting with the monitor (HDMI
   with a USB keyboard), which needs no new panel (built:
   `faraday/image/boards/pi3/`; the 2.8-inch was tried on its own
   image, which the one image replaces, step 9). Left: trying an HDMI
   monitor on a Pi 3B+, on the one image (step 11).
4. The PC's one build: the command line with
   `efi=disable_early_pci_dma`, one image and one boot program (built:
   `just faraday-stick-image`).
5. The release recipes, `SHA256SUMS` lines and the image list (built:
   `just faraday-release-stage` stages every released image of
   `faraday/builds.toml`, its boot partition's line and `IMAGES`, and
   refuses images of two builds; its second pass adds the Starters,
   built with `IMAGES`). Left: the
   size and staging time of a full set, measured on the next release's
   builds.
6. The Starter: no images, finding and checking them, the offers.
   Built: the Starter carries no payload; the release's image list
   (`IMAGES`, named by `FARADAY_STARTER_IMAGES` at build) is embedded and
   is all a found file is checked against; the finder
   (`faraday/shells/starter/src/finder.rs`): Downloads
   and dropped files are looked at, and each file whose name is on the
   list is hashed off the window's thread and found, named as not
   matching, or named as another release's with the version needed; the
   Starter keeps the answer (`Starter::image_findings`). The writer
   writes from a found file (`Source::Found`, from `Starter::image`): the `.xz`'s sum
   is checked before a byte is written, and the size and SHA-256 of
   what is written against the list's line; a
   file changed since it was found is refused with its path and sum. The
   offers (Make a Faraday stick and Make a Pi card, For which computer?,
   Not found) are built. Left: the finder tried by a person on a
   computer with a real release's images, which a release brings.
7. Each screen's values, from its documentation, and tried on the
   panel to be marked tested: the 10-inch HDMI (with the USB
   touchscreen rule, §2) on the HDMI image today, and the 2.8-inch and
   an HDMI monitor on the one image (steps 9 to 11).
8. The website's download page (§7), built around the images released
   today, with a tile for each platform as it is built.
9. **The one Pi image, with both screens** (§3, F186, F187; built):
   - the panel directory `any` with a directory per screen (the 2.8-inch
     and HDMI),
     each with its values, what its probe expects, and its touch and
     display overlay lines, a vendor's value marked to confirm (§3.1);
   - the overlays merged at build time with `ovmerge` and placed in the
     initramfs (§3.3);
   - the boot script, `/run/faraday-screen/args`
     and the inittab reading it (§3.4);
   - the kernel options and their `kernel.required` and
     `kernel.forbidden` lines (§3.5), the ribbon camera's kept;
   - `faraday/builds.toml` (one Pi image, under the Pi's one
     tile), `faraday/tools/stage-release.sh`, the Starter's one
     Pi offer, `docs/HARDWARE.md` (both screens built, not tried on the
     one image; §4's camera), `docs/STARTER.md`, `docs/ORIENTATION.md`,
     `docs/DEVELOPING.md`, `docs/SECURITY-MODEL.md`;
   - the per-screen panel directories `waveshare-28dpi` and `hdmi`, and
     their release files, go.

   No Pi image is released from the one image until the 2.8-inch has
   been tried on it (step 11).
10. **Tests without a screen**, run by the builder:
    - the boot script's choices against recorded and faked probe
      answers: HDMI connected with the 2.8-inch attached; its GT911
      answering; a controller that answers late, or under another name or
      address; a rebind that leaves no `/dev/fb0`; nothing found. Each
      gives the screen expected, or HDMI, and the arguments the shell
      is given;
    - the image under QEMU, which has no HDMI output and no panel:
      every probe finds nothing, the Pi falls back to HDMI, configfs is
      unmounted, and the app starts (and exits, with no framebuffer to
      draw on). The machine is `raspi2b` with the card's
      `bcm2710-rpi-3-b.dtb`, kernel and command line given directly, and
      the card a copy of the image grown to a power of two in size:
      `raspi3b` shows nothing from this 32-bit kernel. QEMU does not run
      the Pi's firmware, so this does not test `bootcode.bin` reading
      the card.
11. **Each screen tried on the one image**, the 2.8-inch and an HDMI
    monitor (the owner, with the dev image, whose boot report carries each
    probe's lines, §3.4). A screen tried becomes tested in
    `docs/HARDWARE.md` and on the website, and its values fact; a value
    found wrong is put right in its directory. What each panel answers:
    - **Every panel.** Whether the display overlay applied through
      configfs and vc4's unbind and bind bring the panel up with
      `/dev/fb0` at its mode and the boot logo (§3.3); how long its
      probe takes when the panel is there and when it is not, which
      sets the probe's wait; the touch grid and axes the shell needs
      (`--touch-grid`, and swap or invert if any).
    - **The 2.8-inch.** Whether its GT911 answers on the touch overlay's
      bus before the DPI overlay is applied (whether the controller is
      powered and out of reset without the panel set up); the pixel
      format parameter of `vc4-kms-dpi-generic` that gives the right
      colours with the format the shell writes; that `DRM_PANEL_SIMPLE`
      is in the built configuration; Waveshare's timings on the
      kernel's DPI driver; the panel on a Zero 2 W. If the GT911
      answers but the run-time overlay does not come up, the owner
      decides on the fallback (§3.3).
    - **HDMI.** Whether `/sys/class/drm/card0-HDMI-A-1/status` reads
      `connected` by the time the boot script runs, on the 3B+ (with
      `GPIO_RASPBERRYPI_EXP`), the 3B and the Zero 2 W; that a monitor
      attached beside the 2.8-inch wins and the panel stays dark with its
      touch unbound; that a monitor plugged in after a boot with nothing
      found is set up by its hotplug and the app draws on it.
    - **The camera beside each screen**, the 2.8-inch included: the
      ribbon camera (Camera Module v1 or v2) on the firmware's stack
      with `gpu_mem=128`, and a USB webcam's frames. Beside the 2.8-inch on the kernel's DPI driver, if the firmware's
      stack does not run: whether unicam and the sensor's V4L2 driver
      do, with the ISP or without it (§3.3).
