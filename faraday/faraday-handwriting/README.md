# faraday-handwriting

A Rust rewrite of the Haller handwritten-BIP39-word networks: the word
network, the word detector (both in integer arithmetic) and the dictionary
decoder, the clean-up of a grey crop to the word network's ink map, and the
reading of a page (words, line numbers, order, form). No screen, camera,
string or action uses it.

Public API: `clean_crop` (grey crop to an ink map of at most 256 by 32 and the
word's box), `unread_ink`, `read_crop` (crop and word list to `Reading::Clear`
or `Reading::Unclear`), `word_logits`, `log_softmax`, `word_scores` (BIP39
English), `word_scores_in` (any list of lowercase a-z words; the `Scores` keep
the list, so `Scores::best` names words of it), `read_word`, `detector_maps`,
`bip39_words`, `slip39_words`. `examples/read_word.rs` reads a binary PGM.

Word lists: the English BIP39 list is the one checked against the published
vectors. Faraday's SLIP-39 list (1,024 words) is accepted by the API and
nothing is claimed about how well the networks read it.

## Image clean-up

`clean_crop` follows Haller's `preprocess.py` (`recipes/haller-1/evalsim/preprocess.py` of the haller
commit under Sources): block-mean
reduction of a tall crop, ink against the local paper brightness, a threshold
from the darkest strokes, removal of long horizontal runs (ruled lines), the
heaviest block of rows and the groups of columns, then a triangle-filter
resize to 32 rows. The result is not bit-identical to Pillow's: the resize
rounds each pass to bytes in f64 rather than in Pillow's fixed point, the
reduction rounds a block mean to the nearest integer, and a crop wider than
4,096 columns is also reduced (so that the work is bounded for any crop; the
box is returned in crop coordinates and clamped to the crop).

`read_crop` answers `Clear` only when the list holds at least two words, the
best word leads the second by `CLEAR_MARGIN`, the best word's own likeliest
alignment to the frames (a Viterbi path over the same states as the scores) is
within `CLEAR_FREE_GAP` of the network's own free reading (the best class at
each frame, a gap of 0 or more), and the faint ink left out of the word is
known and under `CLEAR_UNREAD_INK`. The ink is not known when the clean-up
finds no rows or columns in its stricter mask, or when a line it removes is on
the rows of the word or within two of them (a line through a word cuts it in
two): such a crop is never clear (the rows `clean_crop` reads must be the ones it
looks at: if they differ, the ink is not known). `Unclear` lists only
words that have a score. The page reader's list fit uses the same likeliest
alignment of the best word (below). These three constants are provisional: they were set on the 62
published strips (cleaned again as crops), not on real handwriting, and are to
be measured on real handwriting. Of those strips, 31 read `Clear` before this
rule and 30 after it, and the test holds that every `Clear` reading is the word
the strip was written as (its label, leaving off a line number); the strips
whose label is no BIP39 word (9) or that hold noise never read `Clear`. No
checksum is used.

No compute token (`docs/COMPUTE.md`): like the QR scanner's worker, the
functions run on a worker thread of the caller's.

## Sources

- Weights, vectors and the C reference: github.com/haller-project/seedsigner-haller,
  commit ee63ab365642a516037aab06e1e0c45636bca5f6
  (`src/seedsigner/resources/handwriting/`, `.../native/handwriting.c`).
- Licence and notices: github.com/haller-project/haller, commit
  8ae7296a4d33703ec661848ad927079e3fbe9eaf (see `NOTICE`).
- `haller-1-capitals.npz` SHA-256
  `2e9275f1acd0d347437e856160430e96a283ec484a48ab96a2ea1ca91531fe14`;
  `haller-1-detector.npz` SHA-256
  `b33f6d02f1af87a4aff1cecc73368cc2230c2fa68d0eda977a112df437d501cc`.
- Image clean-up: `recipes/haller-1/evalsim/preprocess.py` at the same haller
  commit.
- `faraday/faraday-handwriting/tools/weights.py` checks both, and writes `weights/word.bin`,
  `weights/detector.bin`, `weights/logadd.bin` (the log-add table as f32 bit
  patterns) and `tests/data/*.bin` (the vectors, inflated).

## What is tested

- The logits of all 62 strips and the maps of both detector images equal the
  published SHA-256 bit for bit.
- The 2,048 scores of 61 of the 62 strips hash equal too. In the other one
  they differ in the last bits; the best five words and scores of all 62
  strips equal the published ones. The cause, found by swapping functions
  in a test: strip 54's log-softmax differs from the platform's (glibc)
  because our `ln` gives a different last bit for one of its sums; with our
  `exp` and the platform's `ln` the hash equals. Matching it needs glibc's
  own `logf` algorithm and tables, which is not a small change, so the count
  stays 61.
- Synthetic crops (a dark mark on light paper, uneven light, a ruled line,
  a mark cut by the edge, huge, tiny, empty, flat and noisy crops) and the
  strips run through the crop path.
- The five flat `.bin` files (weights and vectors) have the SHA-256 the test
  holds; a word list past its letter bound is refused; `fuzz/` has a target
  (`handwriting_crop`) for the crop functions and the network.
- Not held by a test or lint, a hand check: the example built in release has
  no undefined libm symbol (`nm -D -u`) and does not need libm.

## Reading a page

`find_words` (`find.rs`) scales a frame to the detector's input, joins the
detector's word and number kernels into boxes and looks again on the region
that holds them when the words cover under half of the frame. `read_page` and
`read_page_with` (`page.rs`) read each word and each number once, judge the
word against the English BIP39 list, the SLIP-39 list and the caller's `Extra`
list by the rule of `read_crop`, put the words in order (`order.rs`) and choose
the form.

What a page asks, never guesses (a checksum chooses no word, form or order):

- `Question::Order`: several columns without numbers that name one list, or a
  layout that could not be told; both the order down the columns and the order
  row by row are returned, and `List::order` says which one `List::words` is
  in (the one the numbers run along comes first).
- `Question::Count`: the number of words fits no form (BIP39: 12, 15, 18, 21,
  24; SLIP-39: 20, 22, 23, 25, 27, 28, 30, 31, 33; `Extra`: the caller's).
- `Question::Form`: the count fits several forms whose lists fit alike, or
  another list fits the words better than the form's by `LIST_MARGIN`, or the
  form's list fits the words worse than `FIT_FLOOR`. A list's fit is minus the
  mean, over the words, of how far below the network's free reading the best
  word's likeliest alignment is (the gap: 0 or more, so the fit is 0 or less,
  0 being as good as the free reading), each gap no more than `BEAM` and a
  word with no reading, or whose crop cannot be read, counting as `BEAM`: a
  form is never chosen by count alone. `FIT_FLOOR` and `LIST_MARGIN` are
  provisional. On the 62 published strips (cleaned again as crops) the best
  BIP39 word's gap is 0 for 28 of the 53 whose label is a BIP39 word, 2.2 on
  average and 13.5 at most, against 10.6 on average in the SLIP-39 list; strip
  61, which holds noise, is 6.2 in BIP39, and a box of random pixels about
  3.3. The floor is -5: a page asks when its mean gap is over 5, which strip
  61 (6.2) and the twelve worst strips together (7.1 on average) are, and
  the average word (2.2) is not. A page of random-pixel boxes (3.3) is not
  asked about by the floor, but no word of it reads clear, and a list none of
  whose words reads clear in any list asks `Form` too (form and list left
  open).
  Not measured on real handwriting.
- `Question::Numbers`: a number read is not its word's place plus one in the
  order returned (a restart is cut only when the numbers prove it), or a
  number stands beside no word.
- `Question::Split`: lists cut apart only by a vertical gap where any run of
  two or more of them (the whole included) fits a form, or one of which has a
  question; columns side by side where a run of them that is not all fits a
  form, or a blank band across all the columns cuts the page and a part above
  or below it, or a run of the parts, fits one; a layout that could not be told.
  The form is left open for the last two.
- A word at the frame's edge is never clear (`PageWord::edge`,
  `Page::maybe_cut_off`); ink beside a word makes it unclear; a mark counted
  as a word stays in the list as a word that does not read clearly. Nothing
  is dropped. A word made unclear this way keeps the candidates an unclear
  reading lists. A box may hold two lines when its kernels reach over more
  than `TWO_LINES_KERNEL` of the height the network gave (`Found::tall`; one
  line's kernels reach 0.45 to 0.73 on the strips of one page, and one real
  word reached over 0.75; a merged or joined pair of lines reaches more;
  provisional, and not measured on a full page of small lines) or
  it is more than `TALL_BOXES` times the page's median box: it is unclear and
  the list is asked about (`Question::Missing`, form left open).
  A page of more than `MAX_READ` words or numbers, or whose crops together
  hold more than `MAX_TOTAL_CROP_PIXELS`, is flagged `Page::too_many` and
  nothing is read; a frame with a side over `MAX_SIDE` is `Error::Size`.

Order (`order.rs`): words in boxes that overlap across the line are chained;
a chain is one column only when each word overlaps the next one down and the
left edges stay near one straight line (a slanted page's column is straight),
otherwise it is cut at the wide gaps between left edges, and a part that is
still not a column makes the whole page one list with both orders asked. A
number goes with the word on its line (nearest vertical centre, then gap) and
with no word when two are about equally near (two on one line, as far in
height and in gap; two on other lines, as far in height). Numbers, when every word has one
and they are exactly 1 to n, give the order and cut lists where the numbering
starts again; a single column whose numbers do not run down it is a
`Numbers` question. Otherwise a column is a list top to bottom, cut at a clear
vertical gap. Slanted pages are tested only on made boxes, not on photographs.

Provisional, not measured on real handwriting: `LIST_MARGIN`, `FIT_FLOOR`,
`TWO_LINES_KERNEL`, the number reach
and the gap and row constants in `order.rs`, `CROP_MARGIN`, the detector box
constants in `find.rs`, and the three `read_crop` constants.

Untested on real pages: every test page is made from the 62 published strips
pasted on light paper, so the detector has not met a photo, a shadow, a slanted
or curved page, ruled or printed layouts, or a hand that is not the strips'.
Line numbers are not tested at all on an image (there are no number strips);
`read_number` (a digit string scored against the network's digits) and the
pairing of numbers with words are tested only on boxes. A speck the detector
does not report (a 10 by 10 pixel dot) is not seen; a mark it does report is
asked about, as above.

## Left out

The
fingerprint and lowercase networks, abbreviations, the border flood fill and
the connected components of the C file.

The weights cannot be retrained to the same bits: training is not
reproducible, so these are the published files, checked by hash.

`exp` and `ln` are written here (f64 range reduction and polynomial, rounded
to f32), so the crate calls no libm.
