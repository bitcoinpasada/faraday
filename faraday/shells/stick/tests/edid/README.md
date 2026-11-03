# EDIDs of real screens

The EDIDs the stick shell's density and layout tests read
(`faraday/shells/stick/src/edid_screens.rs`, rule `docs/ORIENTATION.md`
§2.1): one file per screen, as the kernel gives it in
`/sys/class/drm/<connector>/edid`. Test data only: nothing here is
compiled into an app.

Source: the public collection `https://github.com/linuxhw/EDID`, commit
`9c0c1bffc9c0f1cb2044115149a5ecb1652803f8` (2026-01-08), under the
collection's licence, Creative Commons Attribution 4.0 International
(`LICENSE-CC-BY-4.0.txt`, the collection's own `LICENSE`). The EDIDs are
the collection's contributors' probes of their own screens, collected by
linuxhw. Each was fetched on 2026-11-03 as data into an empty directory
outside the repository and never executed; the hex dump of each was
turned into bytes, every block's checksum was checked, and what was
altered is in the last column.

Paths are under `Digital/` in the collection.

| File | Screen | Source path | Altered |
|---|---|---|---|
| `waveshare-8.9in-2560x1600.edid` | Waveshare, 2560 × 1600, 192 × 120 mm | `WaveShare/HLT0913/DE20873DE0F8` | nothing |
| `waveshare-11.9in-2560x1600.edid` | Waveshare 11.5, 2560 × 1600, 256 × 160 mm; the screen's size is an aspect ratio (bytes 21 and 22) | `WaveShare/HLT0913/0DCD4991446C` | nothing |
| `waveshare-10.1in-1920x1200.edid` | Waveshare, 1920 × 1200, 216 × 135 mm; the screen's size is an aspect ratio | `WaveShare/HLT2A3B/FC0A3B5BA151` | serial-number descriptor (`0100923`) blanked, checksum recomputed |
| `waveshare-portrait-1080x1920.edid` | Waveshare WS_FHDOLED, 1080 × 1920 portrait, picture size 1920 × 1080 mm, screen size zero | `WaveShare/HLT3433/68F4E39957AB` | serial number (bytes 12 to 15, `00888888`) zeroed, checksum recomputed |
| `hannstar-7in-800x480.edid` | HannStar HSD070IDW1, 7 inch, 800 × 480, 154 × 84 mm | `HannStar/VJK5308/75B7A83AC433` | serial number (bytes 12 to 15) zeroed, checksum recomputed |
| `displaylink-7in-1024x600.edid` | DisplayLink 7 inch, 1024 × 600, 153 × 90 mm; the screen's size in cm is wrong (8 × 5) | `DisplayLink/DSL0010/6D55827AA3E1` | serial number (bytes 12 to 15) zeroed and serial-number descriptor (`MTH…`) blanked, checksum recomputed |
| `cpt-7in-1280x800.edid` | CPT CLAA070WB01, 7 inch, 1280 × 800, 152 × 94 mm | `CPT/CPT1CEB/3A3AE50AD664` | nothing |
| `eve-10.1in-1280x800.edid` | Eve Spectrum HDMI panel, 10.1 inch, 1280 × 800, 217 × 136 mm | `Eve Spectrum/EVE1001/21AF3E362C2C` | nothing (the `VGG1280_HDMI` descriptor is a product tag) |
| `vizta-10.1in-1024x600.edid` | HDMI panel, 10.1 inch, 1024 × 600, 223 × 125 mm | `VIZTA/JRY1010/3B5A280BEBA6` | nothing |
| `vizta-10.1in-1024x600-wrong-image-size.edid` | the same kind of panel with a picture size of 276 × 207 mm, not its shape; screen size 220 × 130 mm | `VIZTA/JRY1100/C9786FA904E6` | nothing |
| `valve-7in-800x1280-placeholder.edid` | Steam Deck panel, 800 × 1280, picture size 60 × 60 mm (a placeholder), screen size 100 × 150 mm | `Valve/VLV3001/25C545529ECB` | nothing |
| `samsung-10.79in-1920x1080.edid` | Samsung panel, 1920 × 1080, 239 × 134 mm: 10.79 inch | `Samsung/SDC4C48/A00EEA5274D5` | nothing |
| `boe-10.99in-2160x1440.edid` | BOE panel, 2160 × 1440, 232 × 155 mm: 10.99 inch | `BOE/BOE1440/B7E87B3C8157` | serial number (bytes 12 to 15, `60000000`) zeroed, checksum recomputed |
| `boe-11.02in-2160x1440.edid` | BOE panel, 2160 × 1440, 233 × 155 mm: 11.02 inch | `BOE/BOE0A0A/1867B2139F4F` | nothing |
| `mstar-11.6in-1920x1080.edid` | MStar HDMI screen, 1920 × 1080, 256 × 144 mm: 11.6 inch | `MStar/MST1560/6B152D95A982` | nothing |
| `dell-24in-1920x1080.edid` | Dell desk monitor, 1920 × 1080, 527 × 296 mm | `Dell/DEL074A/601AEBC71A5F` | nothing |
| `samsung-tv-1920x1080.edid` | Samsung television, 1920 × 1080, 930 × 530 mm | `Samsung/SAM0000/2D56A8334AA9` | nothing |
| `sunplus-tv-1920x540.edid` | Sunplus television, 1920 × 540 mode, picture size 160 × 90 mm | `Sunplus/SPV1BBC/19C39EF230DD` | nothing |

Serial numbers of 0 or 1 (`01 00 00 00`, `01 01 01 01`) are the
placeholders makers write and were kept. Names in the descriptors are
the models' names. No Waveshare 5-inch or 7-inch HDMI panel is in the
collection: the 7-inch ones here are other makers' panels of that size.
