# The dice procedures other signers publish

`docs/PLANNING.md` §16.115. Every procedure `osk_entropy::DiceProcedure`
offers is reproduced from the source named here, never from a summary.
Each function below is quoted as the file holds it, so a later change to
a source is visible as a change to this page rather than as a silent
difference of opinion.

Nothing here is built, run or vendored. The repositories were cloned to
read, and only this page is kept.

| Source | URL | Commit | Fetched |
|---|---|---|---|
| Coldcard firmware | <https://github.com/Coldcard/firmware> | `948dc108a0a12ea06ec8ee90575609eb3dc90c7b` | 2026-09-19 |
| SeedSigner | <https://github.com/SeedSigner/seedsigner> | `85cd9a0211eeb22962a5e65ba349ef6efa91bb57` | 2026-09-19 |
| EntropyLab | <https://github.com/OogaBoogaX/entropylab> | `e113d1ec0a43d45403287b724a55c83fa5699917` | 2026-09-19 |
| BitBox02 firmware | <https://github.com/BitBoxSwiss/bitbox02-firmware> | `0430249035609ec84d383461ee5288cb83a40c46` | 2026-09-19 |

| File read | SHA-256 |
|---|---|
| `docs/rolls.py` (Coldcard) | `4348a520e57df665e0ab57baa369a95ace0f9b5fba355b3f22b0b9b2c2e6cd30` |
| `docs/rolls12.py` (Coldcard) | `533daff58437cdc9a482d16cd181ba9b0fe6f86a6839b792343d39b496034c85` |
| `src/seedsigner/helpers/mnemonic_generation.py` (SeedSigner) | `1da00ac502a88ae77733681fd19f0fb61df03ebdee759b9ac78798fe9399c8fc` |
| `src/js/app.js` (EntropyLab) | `5080326e9aef1d1ab31751cd6a2b984dc12fe26c935022f9a6bbfe331a4403c2` |
| `src/rust/bitbox02-rust/src/workflow/mnemonic.rs` (BitBox02) | `b6ff60a3963c61c99f4c79cff4817068050ee3c5c4ddca76ec1c14af4d84aad9` |

## 1. Hashed — `DiceProcedure::Hashed`

The rolls written as the ASCII digits `1`–`6`, hashed with SHA-256 and
truncated to the strength. This is what this device has always done
(§16.17); the three sources below are byte-identical to it and to each
other.

Coldcard, `docs/rolls.py`, which its documentation publishes at
<https://coldcardwallet.com/docs/rolls.py>:

```python
def main():
    # Read input, remove whitespace around it
    r = input().strip()
    # Calc sha256
    h = sha256(r.encode()).digest()
```

`docs/rolls12.py` is the same file truncated to sixteen bytes.

SeedSigner, `src/seedsigner/helpers/mnemonic_generation.py`:

```python
def generate_mnemonic_from_dice(roll_data: str, wordlist_language_code: str = SettingsConstants.WORDLIST_LANGUAGE__ENGLISH) -> list[str]:
    """
        Takes a string of 50 or 99 dice rolls and returns a 12- or 24-word mnemonic.

        Uses the iancoleman.io/bip39 and bitcoiner.guide/seed "Base 10" or "Hex" mode approach:
        * dice rolls are treated as string data.
        * hashed via SHA256.

        Important note: This method is NOT compatible with iancoleman's "Dice" mode.
    """
    entropy_bytes = hashlib.sha256(roll_data.encode()).digest()

    if len(roll_data) == DICE__NUM_ROLLS__12WORD:
        # 12-word mnemonic; only use 128bits / 16 bytes
        entropy_bytes = entropy_bytes[:16]
```

EntropyLab calls the same thing "Base 10 [0-9] / Hashed rolls" and names
Coldcard and SeedSigner in its own description of it, in `src/js/app.js`:

```js
let hashInput = method === "coleman" ? hodlIanColemanDiceString(rolls) : rolls.join(""), digest = hodlSha256(new TextEncoder().encode(hashInput)), bytes = digest.slice(0, config.bytes);
```

Checked by running Coldcard's own script against this device's fifty-roll
test transcript:

```
$ echo -n 32461151351521144121541512665155412152342515356215 | python3 docs/rolls.py
b4b7bf155511dfa13688d93685fea50b2d56414be59fbde4658b84b42f355fa7
```

which is the digest `core/osk-entropy`'s `dice_entropy_is_sha256_of_the_ascii_rolls`
asserts, and `docs/rolls12.py` prints its first sixteen bytes and the
twelve words `opensigner-core`'s `fifty_dice_rolls_by_physical_keys_create_the_expected_key`
reaches.

## 2. Six as zero, hashed — `DiceProcedure::SixAsZero`

Every rolled 6 is written `0`, then the same hash and the same
truncation. EntropyLab calls it "Dice [1-6] / Hashed rolls" and states
that it is Keystone's method and iancoleman's "Dice" mode, which the
SeedSigner comment above warns is not the hashed one. `src/js/app.js`:

```js
function hodlIanColemanDiceString(rolls) {
  return rolls.map((face) => face === "6" ? "0" : face).join("");
}
```

and its own note for the row:

> Hashed rolls / Dice [1-6]: convert every 6 to 0, SHA-256 hash the
> complete mapped digit string, then use the first {bits} bits for the
> selected {words}-word seed. This matches the method used by Keystone.

Vectors, computed by running that function and `hodlDiceEntropy` out of
`src/js/app.js` with method `coleman` (the technique EntropyLab's own
`test/dice-hashed.test.mjs` uses to load a slice of the file):

| Rolls | Strength | Entropy |
|---|---|---|
| `32461151351521144121541512665155412152342515356215` | 128 | `8bc19a488ea41da254da5f35c041ba3a` |
| `666666665321453214532145321453214532145321453214532145321453214532145321453214532145321453214532141` | 256 | `dc08d10f0056ddb466b66682cef727f213a4da9ff8c0997e0f3a1fe1d0dfac89` |

## 3. Words chosen by the dice — `DiceProcedure::Words`

Five rolls of 1–4 and a sixth roll as a coin name one BIP-39 word
outright: 4⁵ × 2 = 2048. A 5 or a 6 on the first five is a reroll and is
not kept; on the sixth, 1–3 is heads and 4–6 is tails. EntropyLab calls
it "BitBox diceware / Direct word selection" after the BitBox paper
lookup table. `src/js/app.js`:

```js
function hodlBitBoxLookupWord(e, t) {
  let r = 0;
  for (let n of e) r = r * 4 + (n - 1);
  return r = r * 2 + t, hodlBip39Wordlist[r];
}
```

and the loop that feeds it, which is where the reroll and the coin live:

```js
    if (diceInWord.length < 5) {
      let face = Number(input);
      if (face >= 5) {
        skippedHigh += 1;
        continue;
      }
      diceInWord.push(face);
      continue;
    }
    // The sixth roll is the coin: 1-3 is Heads, 4-6 is Tails (BitBox lookup
    // table columns: "1 2 3 heads" is the +0 column, "4 5 6 tails" is +1).
    let coin = input === "1" || input === "2" || input === "3" ? 0 : 1;
    words.push(hodlBitBoxLookupWord(diceInWord, coin));
```

The rolls name every word, the last included (§16.139). The last word
also carries the checksum, so only its high `11 − checksum bits` bits
come from the dice — 7 at twelve words, 3 at twenty-four — and the
checksum replaces the rest. That is how SeedSigner completes a final
word the person picked, in `calculate_checksum`
(`src/seedsigner/helpers/mnemonic_generation.py`):

```python
    # Convert the resulting mnemonic to bytes, but we `ignore_checksum` validation
    # because we assume it's incorrect since we either let the user select their own
    # final word OR we injected the 0000 word from the wordlist.
    mnemonic_bytes = bip39.mnemonic_to_bytes(unicodedata.normalize("NFKD", " ".join(mnemonic_copy)), ignore_checksum=True, wordlist=Seed.get_wordlist(wordlist_language_code))

    # This function will convert the bytes back into a mnemonic, but it will also
    # calculate the proper checksum bits while doing so. For a 12-word seed it will just
    # overwrite the last 4 bits from the above result with the checksum; for a 24-word
    # seed it'll overwrite the last 8 bits.
    return bip39.mnemonic_from_bytes(mnemonic_bytes).split()
```

EntropyLab and the BitBox02 firmware instead stop the rolls one word
short and let the person pick the last word from the words the checksum
leaves. Those bits are then the person's choice, not the dice's. The
firmware's comment (`src/rust/bitbox02-rust/src/workflow/mnemonic.rs`):

```rust
            // For the last word, we can restrict to a subset of bip39 words that fulfil the
            // checksum requirement. This special case exists so that users can generate a seed
            // using only the device and no external software, allowing seed generation via dice
            // throws, for example.
            if num_words == 24 {
                // With 24 words there are only 8 valid candidates. We presnet them as a menu.
```

Picking candidate `k` of that list, in index order, is the same key as
rolling a last word whose high bits are `k`; the group `111111` names
`abandon` and gives the first candidate.

Vectors. The words for each six-roll group are EntropyLab's
`hodlBitBoxLookupWord`; the entropy and the completed words are the two
embit calls `calculate_checksum` makes (embit's `bip39`, run with
`uv run --with embit`):

| Rolls | Rolled last word | Entropy | Key's last word |
|---|---|---|---|
| EntropyLab's 66-roll transcript + `432141` | `tooth` | `1b0d8ac66371b2d8ec66361b0d8ec6f2` | `torch` |
| EntropyLab's 66-roll transcript + `111111` | `abandon` | `1b0d8ac66371b2d8ec66361b0d8ec680` | `above` |
| the 72 rolls of the first row, twice | `tooth` | `1b0d8ac66371b2d8ec66361b0d8ec6f261b0d8ac66371b2d8ec66361b0d8ec6f` | `unfold` |

The 66-roll transcript is
`123411234122341233412344123415234126341231412342123413234124341235`,
whose eleven words EntropyLab's `hodlBitBoxRolls` gives as `brand hobby
ranch shoulder brass hockey ranch short brand hockey random`; its
`hodlTargetLastWords` gives 128 candidates from `above` to `zone`.

## What the sanity step counts

Direct selection keeps only 1–4 in a word's first five places, so a
chi-square over six faces would flag every honest run. EntropyLab splits
the transcript into two samples and tests each; this device does the
same, over four faces and over heads and tails:

```js
    return [
      { id: "d4", title: "D4 (1–4)", rolls: d4, labels: ["1", "2", "3", "4"] },
      { id: "coin", title: "Coin", rolls: coins, labels: ["Heads", "Tails"] }
    ];
```

## What is not here

- **EntropyLab's "D++ / Direct word selection"** rolls one D8 and two
  D16 per word (8 × 16 × 16 = 2048) and finishes the checksum word with
  a per-length sequence of D8, D16 and coin rolls. It needs dice this
  device's pad does not offer, so it is not built. The rule is in
  `hodlDPlusRolls` if it ever is.
- **A BitBox02 dice procedure in the firmware.** There is none: the
  quotation above is the whole of it. The lookup table is BitBox's paper
  diceware sheet, and EntropyLab's implementation of it is the source
  this device took.
