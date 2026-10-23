# ttf-parser

**Purpose.** Reading glyph outlines out of a font file in
`tools/fontbake` (`faraday-fontbake`), the build-time baker that turns
fonts into the outline data `faraday-draw` draws text from
(`core/faraday-draw/src/fonts/mod.rs`). `Face`, `OutlineBuilder` and
glyph metrics are the parts used.

**Where it ships.** Nowhere. The baker is run by hand with
`just` (`justfile`) and never runs on a device. Its output is data checked
into the tree; the device, desktop and Starter builds contain no font
parser.

**Why this crate.** A font file is a table-driven binary format (`glyf`,
`CFF`, `cmap`, `hmtx`) with composite glyphs and hinting data. Reading
outlines from it is a parser of its own, and doing that once at build time
with a maintained crate keeps the device free of it. `ttf-parser` is
pure Rust, safe, with no allocations, and it was already in the lock
before the baker used it, so the tool added no crate.

**Version and features.** `ttf-parser = "0.25.1"` in the workspace
`Cargo.toml`, default features, pinned to the version already in the lock.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).

**Reopen when** the baker is replaced by prebuilt outlines checked in
from another source, or when the fonts used need a table `ttf-parser`
does not read.
