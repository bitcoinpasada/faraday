# BIP-39 wordlists

`gen.py` downloads the ten official wordlists from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/<name>.txt`
and writes one Rust module per language to `core/osk-bip/src/wordlists/`.
The generated modules are committed; building the crates never touches the
network.

Each module exposes `WORDS` (as published), `NFKD` (the form used for seed
derivation; a reference to `WORDS` when the list is already NFKD, which all
ten currently are), `DISPLAY` (the form a reader sees), `SHA256` (hex
digest of the source file, which the script verifies is exactly the 2048
words each followed by `\n`) and `MAX_NFKD_BYTES`.

`DISPLAY` is the NFC form of each word. The published Japanese list is
base kana plus the combining voicing marks U+3099 and U+309A and the
published Korean list is conjoining jamo, which is right for derivation
and for typing and wrong for reading; composed, they are the kana and the
Hangul syllables a person reads. Spanish and French are published
decomposed too. In the other six lists the published form is already
composed and `DISPLAY` is a reference to `WORDS`. `Language::word_display`
reads it; `word` and `word_nfkd` are unchanged.

## Mandarin readings

The script also downloads the Unicode Han Database
(`https://www.unicode.org/Public/16.0.0/ucd/Unihan.zip`, pinned by
SHA-256 in the script) and writes
`core/osk-bip/src/wordlists/readings.rs` from `kMandarin`, `kXHC1983`
and `kTGHZ2013`: the readings two modern dictionaries list for each of
the 2,821 characters in the union of the two Chinese lists. The Hanyu
Da Zidian's historical readings (`kHanyuPinyin`) are left out: nobody
types 王 as yù.

- `SYLLABLES` is every syllable those readings use, sorted by pinyin, in
  both spellings a keyboard types: toneless Hanyu Pinyin (`lv` for `lü`)
  and 注音. The conversion is a table of initials, medials and finals in
  the script; it checks that every syllable converts back to the pinyin it
  came from and fails on a reading it cannot spell.
- `READINGS` is each character and its readings, sorted by character: an
  index into `SYLLABLES` and a tone, 1 to 4 as it is marked and 5 for the
  neutral tone. The first reading is the one a dictionary heads the entry
  with. 495 characters have more than one.

`Language::readings`, `candidates_pinyin` and `candidates_zhuyin` are the
API over them.

To regenerate, from the repository root:

    just wordlists        # or: python3 tools/wordlists/gen.py && cargo fmt --all

Then run `just test`: `core/osk-bip/tests/wordlists.rs` re-hashes the
embedded words and compares against `SHA256`, `tests/crosscheck.rs`
compares every word against the `bip39` crate, and `tests/readings.rs`
checks the readings and the composed forms.
