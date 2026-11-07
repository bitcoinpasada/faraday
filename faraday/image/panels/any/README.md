# The Pi's one panel directory

The Pi image serves every screen Faraday lists and names none of them
(`docs/IMAGES.md` §3, F186). This directory is its "panel": `build.sh`
builds it like any other (`panel.conf` gives the boot logo's size, and
`buildroot.fragment` the packages that make the overlays), and the screens
are under `screens/`. A boot script, `/etc/init.d/screen` in the root
filesystem overlay, finds the screen at boot from what is in the image.

## The format of a screen

```
screens/order              the panel probes, in the order they are tried
screens/<screen>/panel.conf   the values the app is given for the screen
screens/<screen>/probe.conf   what the screen's probe expects
screens/<screen>/touch.line   the touch overlay: one line, or touch.dts
screens/<screen>/display.line the display overlay: one line
```

HDMI (`screens/hdmi`) has `panel.conf` only: it is the first check and the
fallback, so nothing is probed and no overlay is loaded.

- **`screens/order`**: one screen's directory name a line, `#` comments
  and blank lines allowed. It lists the probes after the HDMI check, in
  `docs/IMAGES.md` §3.2's order.
- **`panel.conf`**: shell variables, as `build.sh` read them from a panel
  before. No value may contain a space. `SIZE=WxH` (`WIDTH` and `HEIGHT`
  are read as well), `DPI`, `TOUCH_NAME`, `TOUCH_GRID=WxH`,
  `TOUCH_SWAP`, `TOUCH_INVERT_X`, `TOUCH_INVERT_Y` (1 to set), `NATIVE`
  (`portrait` or `landscape`) and `MOUNT` (`90` or `270`, only with
  `NATIVE`), each only when true of the screen. A `TOUCH_NAME` needs a
  `TOUCH_GRID`. HDMI's gives none of the values the screen can say
  itself, and its `TOUCH_GRID` is for a USB touch panel whose descriptor
  gives no range. The boot script turns these into the shell's arguments
  (`--size`, `--dpi`, `--touch-name`, `--touch-grid`, `--native`,
  `--mount`) exactly as `build.sh` built `/etc/faraday/args`.
- **`probe.conf`**: shell variables, for the boot script:
  - `KIND=gpio`: the controller is on the GPIO header (`dsi` is for a
    controller on the DSI connector's I2C, none today);
  - `TOUCH_NAME`: a substring of the input device's name;
  - `I2C_BUS`: a component of the input device's sysfs path naming the
    bus the touch overlay declares (`i2c@0` for Waveshare's bit-banged
    bus), or `any`;
  - `I2C_ADDR`: the controller's address in hex without `0x`, or
    several separated by spaces (quoted) when the overlay declares it at
    either;
  - `RANGE=WxH`: the range a DSI panel's controller reports, empty and
    unused for a GPIO screen.
  A value marked "to confirm" is the vendor's, built in and checked on the
  panel (`docs/IMAGES.md` §3.1).
- **`touch.line`, `display.line`**: one line each, a firmware overlay's
  name and its parameters separated by commas, without spaces. The build
  merges each with Raspberry Pi's `ovmerge` from the pinned kernel's
  overlay sources into one overlay with no parameters (a name with no
  parameters is read as followed by a comma) and compiles it.
- **`touch.dts`**: in place of `touch.line`, for an overlay with no source
  in the kernel (the 2.8-inch's). It is compiled as it is. Every fragment
  aims at a node already in the device tree: configfs refuses one aimed
  at a node the same overlay creates, which the firmware's merger allows.

## What the image holds

```
/etc/faraday/screens/order
/etc/faraday/screens/<screen>/panel.conf
/etc/faraday/screens/<screen>/probe.conf     (not HDMI)
/etc/faraday/screens/<screen>/touch.dtbo     (not HDMI)
/etc/faraday/screens/<screen>/display.dtbo   (not HDMI)
```

in the initramfs, so the release signs them with `zImage`. Nothing is
written under `/run` by the build; the boot script writes
`/run/faraday-screen/args`.

## The 2.8-inch's display settings

`screens/waveshare-28dpi/display.line` is the whole of the panel's display
setup: the firmware's `vc4-kms-dpi-generic` with Waveshare's documented
timings (`hdmi_timings` there: 480 and 640 active, horizontal porches 26,
16 and 10, vertical 25, 10 and 15, both syncs active low) and the 18-bit
pin set. The pixel clock is 22 MHz, not Waveshare's documented 32 MHz:
the mode is 532 x 690 pixels with its porches, so 32 MHz refreshes it at
87.17 Hz (11.47 ms) and 22 MHz at 59.93 Hz (16.69 ms), and the frame
grid of 60 Hz (`docs/MOTION.md` §3.2) needs the second. The colour order is its pixel format parameter,
`rgb666-padhi`. The colour fix the display needed on the firmware's
framebuffer (`dpi_output_format=0x7F216` with `framebuffer_ignore_alpha=1`)
is worked out again here: it goes in this line, and in nothing else.
