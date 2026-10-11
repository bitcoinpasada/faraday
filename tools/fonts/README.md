# Vendored fonts

Source files for the glyph outlines that `osk-ui` embeds. They are inputs
to `tools/fontbake` only; no font file and no TTF parser ships on a device
(`docs/PLANNING.md` §4.3).

| File | Family | Use |
|---|---|---|
| `NotoSans-Regular.ttf` | Noto Sans | body text |
| `NotoSans-SemiBold.ttf` | Noto Sans | titles, row labels, buttons |
| `NotoSans-SemiCondensed.ttf` | Noto Sans SemiCondensed | body text on `small` |
| `NotoSans-SemiCondensedSemiBold.ttf` | Noto Sans SemiCondensed | titles, row labels, buttons on `small` |
| `NotoSansMono-Regular.ttf` | Noto Sans Mono | the arrows and triangles the other faces lack |
| `iosevka/Iosevka-Regular.ttf` | Iosevka | everything read character by character: keycaps, candidates, fingerprints, hex, paths, addresses, chunked strings |
| `seedsigner-icons.otf` | SeedSigner icons | the `U+E9xx` icons of the `Icon` set |
| `Font_Awesome_6_Free-Solid-900.otf` | Font Awesome 6 Free Solid | the rest of the `Icon` set |
| `cjk/NotoSans{JP,KR,TC,SC}-Regular.otf` | Noto Sans CJK | the wordlist and keyboard glyphs the faces above lack |

## Iosevka, which is not committed

Iosevka publishes one archive per release, not one file per cut, and the
archive is 158 MiB. `tools/fontbake` downloads it into `iosevka/` when the
Regular TTF is not already unpacked there, checks it against the digest
pinned in `tools/fontbake/src/main.rs`, and unpacks `Iosevka-Regular.ttf`
from it. `iosevka/` is ignored by version control; only the baked
`core/osk-ui/assets/mono.outl` is committed.

Source:
`https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-Iosevka-34.8.1.zip`

| File | Bytes | SHA-256 |
|---|---|---|
| `PkgTTF-Iosevka-34.8.1.zip` | 166,007,346 | `0ea6f8a7d37444a974b45d24995b3a58f80924b71e8c51f0d91a879fa87c36be` |
| `Iosevka-Regular.ttf` (unpacked) | 10,822,640 | `8b6065f04ca4ff4ce95ae48cf9f2f31b584823557adc38ec817dbe6f3745624b` |

Version 34.8.1, the default `Iosevka` family, Regular cut: 0.5 em
advance, slashed zero, 1, l and I distinct.

## The CJK subsets, which are not committed

The four Noto Sans CJK region subsets are 23 MB together, too large to
commit. `tools/fontbake` downloads each into `cjk/` when the file is not
there and checks it against the digest pinned in
`tools/fontbake/src/main.rs`, aborting on a mismatch. `cjk/` is
git-ignored; only the baked `core/osk-ui/assets/cjk.outl` is committed,
so building the crates needs neither the fonts nor the network.

Source, Regular weight only:
`https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/<R>/NotoSans<R>-Regular.otf`

| File | Bytes | SHA-256 |
|---|---|---|
| `NotoSansJP-Regular.otf` | 4,533,028 | `dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073` |
| `NotoSansKR-Regular.otf` | 4,644,748 | `69975a0ac8472717870aefeab0a4d52739308d90856b9955313b2ad5e0148d68` |
| `NotoSansTC-Regular.otf` | 5,683,368 | `5bab0cb3c1cf89dde07c4a95a4054b195afbcfe784d69d75c340780712237537` |
| `NotoSansSC-Regular.otf` | 8,331,336 | `faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9` |

Noto Sans, its SemiCondensed cuts and Noto Sans Mono come from the `notofonts` GitHub organisation
(`notofonts.github.io`, `fonts/NotoSans/unhinted/ttf/`), unmodified release
TTFs. Noto Sans Bold was dropped when SemiBold arrived: one weight of
emphasis is enough, and every face costs binary size. The SemiCondensed cuts are
version 2.015, the release of the ordinary cuts, with the same vertical
metrics; the baker keeps them as faces of their own, and `Font::sized`
swaps them in on `small`.

## Licences

- **Iosevka**: SIL Open Font License 1.1, reproduced in `OFL.txt`.
  Copyright 2015-2026 Renzhi Li (aka. Belleve Invis, belleve@typeof.net).
  The reserved font name is not used for any derived font.
- **Noto Sans, Noto Sans SemiCondensed, Noto Sans Mono, Noto Sans CJK**: SIL Open Font License 1.1,
  reproduced in `OFL.txt`. The OFL permits embedding rasterised glyphs in a binary and
  redistribution under the same licence; the reserved font name is not
  used for any derived font.
- **seedsigner-icons.otf**: from the SeedSigner project, MIT licence,
  reproduced in `LICENSE-seedsigner-icons.txt`. Copied unmodified from the
  seedsigner-touch fork's `resources/fonts/`.
- **Font_Awesome_6_Free-Solid-900.otf**: Font Awesome Free 6. The font
  file is under SIL Open Font License 1.1; the icon designs are under
  Creative Commons Attribution 4.0 International. Both are reproduced in
  `LICENSE-fontawesome.txt`. Attribution: icons by Font Awesome
  (fontawesome.com), CC BY 4.0.

Only the glyphs the `Icon` enum names are extracted, and only their
outlines: no font file, no name table and no glyph the UI does not draw
reaches a build.

## Regenerating the outlines

From the repository root:

    just fonts

which runs `cargo run -p osk-fontbake` and rewrites
`core/osk-ui/assets/*.outl` and `core/osk-ui/src/fonts/generated.rs`. The
glyph set and the icon code points are constants in
`tools/fontbake/src/main.rs`; the outline format is documented in
`tools/fontbake/README.md`. Generated files are committed so that building
the crates never needs the fonts or the baker.

The first run of the baker after a fresh clone downloads the four CJK
subsets and the Iosevka archive; later runs read the files in `cjk/` and
`iosevka/`.
