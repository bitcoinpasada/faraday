# The codex32 booklet's dice de-biasing worksheet

`faraday_entropy::BookletChar` reads rolls the way the booklet's
**Dice De-biasing Worksheet** does, so that a person with the paper sheet
and a person with Faraday get one character from the same rolls. This
page records the method and the tree, as `tools/reference/dice/README.md`
records the other dice procedures, so a later change to the booklet is
seen as a change here.

Nothing here is built, run or vendored.

| Source | URL | Revision | SHA-256 |
|---|---|---|---|
| The booklet's PostScript (SSS32.ps), black and white edition | <https://secretcodex32.com/docs/2023-03-07--bw.ps> | 2023-03-07 | `c12cc875c16667bed1f022c94f493263f5501af91081bcf797209dfbb2d77219` |
| The same booklet as PDF | <https://secretcodex32.com/docs/2023-03-07--bw.pdf> | 2023-03-07 | `0370ea863d2eae692408aeefa9b13c14283e520f45a00f7373ad933ccf418f2e` |

Licence: MIT, "Copyright (c) 2022 Blockstream", stated inside the
PostScript (the notice is `tools/vectors/codex32-booklet/LICENSE`). The
digests are those recorded in `tools/vectors/codex32-booklet/README.md`
when its tables were taken from the same files.

## The method

The worksheet is in the booklet's section "Dice De-biasing Worksheet"
(pages 18 and 19 of the PostScript's source). It turns ten rolls of five
dice into one character:

1. Five distinct dice, die 1 to die 5, each with its own track.
2. Roll all five; each die's first value is noted (the marker on its
   track).
3. Roll the same five again; each die's second value is noted.
4. A die whose two values are equal is rolled twice again, both rolls;
   the booklet: "You must redo both rolls!"
5. Each die gives one bit: 0 when its second value is lower than its
   first (the die lies left of its marker), 1 when it is higher.
6. Die 1's bit is the most significant: the character is the bech32
   character whose value is `16·b1 + 8·b2 + 4·b3 + 2·b4 + b5`, the
   booklet's tree.

The booklet's sheet also has pads for dice of 7 to 20 faces; Faraday
takes six-faced dice only.

## The tree

The tree's leaves, read left to right, are the `code` array of SSS32.ps,
the bech32 alphabet in value order:

```
Q P Z R Y 9 X 8 G F 2 T V D W 0 S 3 J N 5 4 K H C E 6 M U A 7 L
```

`faraday_entropy::BOOKLET_TREE` is these characters in lower case. The
test `all_32_paths_reach_the_booklets_leaf` in
`core/faraday-entropy/src/booklet.rs` walks the tree level by level by
halving the leaves, for all 32 bit patterns, and compares each leaf with
what `faraday_entropy::booklet_character` reads from rolls that make the
same bits.

## What was not checked here

The wording of steps 1 to 6 above is taken from `docs/CODEX32.md` §5.1,
which was written from the PostScript; this page was written without the
PostScript at hand, so the "You must redo both rolls!" quotation and the
leaf order were not re-read against the file's digest above. Before a
release, compare them with `2023-03-07--bw.ps` and correct this page if
they differ.
