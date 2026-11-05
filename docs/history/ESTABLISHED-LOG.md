# Established implementations first: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/ESTABLISHED.md` on 2026-11-05, when its last
open work (batch D1) was built: §2.5 (the inventory's name matching), §4 (the timing fixes), §5 (the switches),
§8 (the order of work) and §9.7 (the cross-check's batches). Each keeps its
original number; the source keeps a one-line stub under the same heading.

## 4. The timing fixes, and what wiping reaches

Places where a secret's value steers the time an operation takes or the
memory it reads, and what the security model already bounds of the
copies a secret leaves in freed memory (§4.8, §4.9). On an offline
device the risk is low: an attacker needs code on the device or a probe
on its power or emissions. Each timing fix comes before the switches. Where an outside crate is no better (§4.5's word
lookup, §4.4's GF(32)), the fix is Faraday's own form, written without a
branch or a lookup on a secret, with the reason in §3.

Every batch here is proved by the vectors and cross-check areas it
names giving the same bytes, the tests of the crates touched, `just
crosscheck --area NAME` where an area's code changed; the stamp and the
actions' reviews wait for the release (F197).
Each changes the manifest's `secrets.constant_time` (or `secrets.wiped`)
text of the actions named to the text given.

### 4.1 F-1: scalar arithmetic on secrets

**Where.** `core/faraday-bip/src/musig.rs`, `core/faraday-bip/src/dleq.rs`
and `core/faraday-bip/src/silent.rs`. Main has them branch-free in
Faraday's own form: `add_mod_order`, `negate_mod_order`, `reduce` and
`at_least_order` carry each borrow and carry as a number over u128 limbs
and pick a result by mask, and `mul_mod_order`'s double-and-add runs 256
fixed rounds over them. Secrets reach them in:

- MuSig2's signing (`musig.rs`, the partial signature): `k1`, `k2`
  negated, `d = g·gacc·d0`, `b·k2`, `k1 + b·k2`, `e·a·d` and the sum;
  the nonces `k` reduced from their hashes and checked against zero and
  the order;
- BIP-374's proof (`dleq.rs`, `generate_proof`): `k` reduced from its
  hash, `e·a` and `k + e·a`;
- BIP-352's sender sum (`silent.rs`, `sender_outputs`): each input key
  negated where its point is odd, and the running sum;
- BIP-352's `checked` (`silent.rs`), which compares a derived tweak
  `t_k` with zero and the order.

**Decided** (owner, 2026-10-31, F180, as F178 says): every operation on a
secret scalar goes through libsecp256k1, which is in the tree (secp256k1
0.29.1). The branch-free functions on main stay only where this section
says.

**The fix.**

- A secret 32-byte value becomes a scalar only through
  `SecretKey::from_slice` (libsecp256k1's `seckey_verify`), and a
  `Scalar` only through `Scalar::from(SecretKey)` (which copies the bytes
  and compares nothing), so no secret meets `Scalar::from_be_bytes`,
  whose comparison the crate documents as not constant time. A secret
  held as a `Scalar` is only handed to the library as a tweak; it is
  never compared, ordered or read by Faraday's code.
- `a + b` is `SecretKey::add_tweak`, `a · b` is `SecretKey::mul_tweak`,
  `-a` is `SecretKey::negate`, held in a type of Faraday's,
  `SecretScalar(Option<SecretKey>)`, where `None` is zero.
- A public factor (MuSig2's `g`, `gacc`, `b`, `e`, `a`; BIP-374's `e`)
  is a `Scalar` from its bytes and multiplies the secret as the tweak.
  Every such factor is below the order by construction (a hash reduced
  by `reduce`, or a key coefficient); `Scalar::from_be_bytes(..)` on it
  is an `expect` whose message says so. A public factor of zero gives
  zero.
- A hash reduced to a scalar (`reduce`) keeps main's branch-free form:
  `a - n` is always computed, and the borrow selects `a` or `a - n` by
  mask. It is the one piece of scalar arithmetic left written here
  (§3.4). `is_zero`, `zero_bit` and the secret form of `at_least_order`
  are gone: a nonce reduced from its hash is checked by
  `SecretKey::from_slice` (`seckey_verify`), which refuses zero and a
  value at or above the order.

A `SecretKey` cannot be zero. Where an intermediate value can be zero
it is handled where it arises: in the silent-payment sum the running
value is held as "no key yet" and an `add_tweak` that gives zero sets it
back to that; MuSig2's partial signature `s`, which BIP-327 allows to be
zero, is `nonce_part.add_tweak(key_part)`, with zero returned where the
library refuses. These branches are taken only on a value of
probability about 2⁻²⁵⁶ for keys not chosen to cancel, and the code says
so where it takes them.

The remaining arithmetic in `musig.rs` (key aggregation coefficients,
tweak accumulators, verification) works on public values only; its
functions say so in their first line and carry a `_public` suffix
(`add_mod_order_public`, `negate_mod_order_public`,
`mul_mod_order_public`, `at_least_order_public`), so that a secret passed
to one shows in review; being public they may branch.

**The limit, stated.** In rust-secp256k1 0.29.1 `SecretKey` is `Copy`,
and `add_tweak` and `mul_tweak` take `self` by value: every operation
copies the key. `SecretKey`s are therefore kept in short scopes, one
call's stack frame, and erased with `non_secure_erase` when that scope
ends (`Drop` for `SecretScalar`, and after each use in `silent.rs` and
`dleq.rs`), which reaches the last copy only; the earlier copies are
left on the stack, as the `[u8; 32]` intermediates are today. The
manifest's `secrets.wiped` of `sign-musig`, `silent-send` and
`silent-receive` says so.

**What must pass, unchanged.** Vectors `tools/vectors/bip327`,
`tools/vectors/bip328`, `tools/vectors/bip390`, `tools/vectors/bip352`,
`tools/vectors/bip374`, `tools/vectors/bip375`, `tools/vectors/psbt`;
the cross-check areas `musig`, `silent` and `silent_send`
(`just crosscheck`, which re-stamps); the tests of `faraday-bip` and
`faraday-psbt`, with a new test file, core/faraday-bip/tests/all/silent.rs, for the
sum's zero cases; `just crypto-review` for `sign-musig`, `musig-key`,
`silent-send` and `silent-receive`, and for every other action
`check.py check` names, since the renamed functions sit in a lock block
those actions share.

**Built.** Every point above is built: the secret arithmetic of
`musig.rs`, `dleq.rs` and `silent.rs` goes through libsecp256k1, the
public arithmetic carries the `_public` suffix, `reduce` is unchanged,
the `secrets.wiped` and `secrets.constant_time` texts say what is and is
not reached, the actions' lock and fingerprints are written again, and
the cross-check stamp is current.

**Secret handling that changes** (`secrets.constant_time`):

- `sign-musig`: "Every operation on a secret scalar is libsecp256k1's
  (`SecretKey` add, multiply and negate). libsecp256k1's header says its
  code is written and tested to be constant time whatever the secret
  values; it states nothing of these functions by name. A hash is
  reduced to a scalar without a branch; the code documents no other
  timing."
- `silent-send`: "Every secret multiplication goes through secp256k1's
  ECDH, and the response `k + e·a` through libsecp256k1's `SecretKey`
  add and multiply, of which its header says what `sign-musig` says."
- `silent-receive`: the sender sum and the tweak check, named the same
  way.

### 4.2 F-2: AEZ's AES round (built)

**Where.** `core/faraday-bip/src/aez.rs`, `round`, whose SubBytes was a
lookup in `SBOX` indexed by the state's secret bytes.

**Built.** `round` is `aes::hazmat::cipher_round` (aes 0.9.3,
`default-features = false`, feature `hazmat`); `SBOX` and `xtime` are
gone; `double` is masked and `mult` runs over public constants. The aes
crate documents every backend as designed to run in constant time: a
fixsliced software round with no table and no data-dependent branch,
AES-NI or VAES detected at run time on x86, ARMv8's instructions on
aarch64 where present (`src/lib.rs`, `src/hazmat.rs`: the hazmat round
takes the same paths as the cipher). Which backend runs is fixed by the
target (aes 0.9.3 `src/hazmat.rs`, `src/lib.rs`; nothing in the
repository sets `aes_backend`):

- The stick (`x86_64-unknown-linux-musl`) and the desktop app
  (`x86_64-unknown-linux-gnu`) take AES-NI where CPUID reports it, and
  the fixsliced round otherwise.
- The Pi image is `armv7-unknown-linux-gnueabihf`
  (`faraday/image/boards/pi3/board.conf`'s `RUST_TARGET`). 32-bit ARM is
  not in the crate's intrinsics `cfg`, so the Pi always runs the
  fixsliced software round, whatever its processor has: AND, OR, XOR,
  NOT and shifts by constant amounts on a machine word (cpubits 0.1.1
  makes it a `u64` on ARMv7), with no table and no data-dependent
  branch.

Both are documented constant time. The `aezeed` action says so.

That the compiled round on the Pi's Cortex-A53 holds no branch or
variable-time instruction is not measured. The owner may check it on a
Pi with the dev image; it is not a batch: a timing test of
`cipher_round` built for `armv7-unknown-linux-gnueabihf` with dudect's
method (Reparaz, Balasch and Verbauwhede, 2017: a fixed input against
random inputs, interleaved, each batch timed with
`CLOCK_MONOTONIC_RAW`, Welch's t staying under 4.5 as the count grows),
and `objdump -d` of the built round showing no conditional branch and no
load indexed by the state. Faraday has no timing harness of this kind.

### 4.3 F-3: SLIP-39's GF(256) and checksum (built)

**Built.** `core/faraday-bip/src/slip39.rs`'s `gf_mul` is eight masked
shift-and-XOR rounds modulo 0x11B, `gf_inv` is `a^254`, `Points::at`
branches only on public share indices, each Lagrange basis is a product
of quotients, and RS1024's `polymod` XORs its generator under a mask;
the `EXP` and `LOG` tables are gone. The `slip39` action says so. What is
left of SLIP-39 is its word lookup (§4.5) and its digest compare (§4.6).

### 4.4 F-4: codex32's GF(32), checksum and alphabet (batch A4)

**Where.** `core/faraday-bip/src/codex32/`:

- `gf_mul` (`mod.rs`) branches on the bits of `b` and on `a >= 32`;
  `mul32` (`correct.rs`) is the same code again as a `const fn`.
  Secret symbols reach them in interpolation (`interpolate`, the
  recovery of a secret from shares, `derive_share`, `split` and
  `fresh_set`), the worksheet's translation (`worksheet.rs`) and the
  corrector's elimination (`correct.rs`).
- `residue_step` (`mod.rs`) branches on the residue's top five bits; the
  residue is a running function of every secret character, folded by
  `polymod` and `long_polymod` (`verify_checksum` on every share or
  secret read, `append_checksum` in `from_seed`, `split` and
  `fresh_set`) and by the worksheet's rows.
- `charval` and `CHARSET` look a character up by its secret value
  (`mod.rs`, `correct.rs`); `StringCheck`
  (`faraday/faraday-keys/src/codex32.rs`) finds a character with
  `position()`, a scan that stops at it.
- `fresh_set` (`fresh.rs`) reserves room for the initial shares only and
  then pushes the derived ones past it: the reallocation frees the old
  block, with the initial shares in it, unwiped.

`INV` is not indexed by a secret: `lagrange` and the worksheet index it
by products of share indices, which are printed on every share, and the
corrector by pivots of the erased positions, which the person marked.
It stays.

**The fix.** No established crate does better: codex32 0.1.0's multiply
is a table with a zero branch, and the bech32 crate's `Fe32` multiplies
by `LOG` and `LOG_INV` tables and finds a character in `CHARS_INV`.
Faraday's own form, as SLIP-39's (§4.3):

- One `gf_mul`, a `const fn` in `mod.rs` that `correct.rs` calls in place
  of `mul32`: five rounds of
  `res ^= a & ((b >> i) & 1).wrapping_neg(); let hi = (a >> 4) & 1; a = ((a << 1) & 31) ^ (9 & hi.wrapping_neg());`
  (41 is x⁵ + x³ + 1; with bit five dropped by the mask, the reduction is
  XOR with 9).
- `residue_step`'s loop becomes
  `residue ^= generator[i] & 0u128.wrapping_sub(((b >> i) & 1) as u128)`;
  it stays a `const fn` (no `black_box`, as SLIP-39's `polymod`). The
  final compare with the constant is one compare whose result is public.
- `charval` becomes a scan of all 32 alphabet entries with a masked
  select (`v |= i & eq_mask(CHARSET[i], c)`), the character's validity
  taken from whether any entry matched; encoding a symbol is a masked
  scan likewise. `StringCheck`'s lookup calls the same function. A
  character outside the alphabet is refused as today; that the text
  typed holds such a character is not secret.
- `fresh_set` reserves the set's full count of strings
  (`scheme.strings_len()`) before the first push.

**Tests.** A test keeps the old `gf_mul` in the test module and checks
every one of the 32 × 32 products unchanged, and the old `residue_step`
against the new over every symbol and a sample of residues.

**What must pass, unchanged.** `tools/vectors/bip93`,
`tools/vectors/codex32-booklet`, `tools/vectors/codex32-volvelle`,
`tools/vectors/codex32-correction`; the cross-check areas `codex32` and
`codex32_correct`; the tests of `faraday-bip` and `faraday-keys`;
`just crypto-review` for `codex32`, `codex32-make` and `codex32-correct`.

**Secret handling that changes.** `codex32`, `codex32-make`,
`codex32-correct`: "GF(32) multiplication, the checksum's residue and the
alphabet's lookup are written without a branch or a table index on a
symbol's value, as the code documents; the code states no other
timing." (The text today says "the arithmetic is table lookups over
GF(32)", which is not so: it is shift-and-add with branches.)

About half a day.

### 4.5 F-5: word lookups (batch A5)

**Where.** The time to find a word follows its place in the list, so
each word's 10 or 11 bits are in the timing.

- `Language::index_of` and `index_of_nfkd`
  (`core/faraday-bip/src/bip39.rs`): a scan over 2048 entries that stops
  at the word, each compare stopping at the first byte that differs.
  Reached by `Mnemonic::parse`, by words pasted, scanned or read from a
  file (`faraday/faraday-core/src/scan_key.rs`), by the Inbox and scan
  classifier on every word of a secret text
  (`faraday/faraday-core/src/scan_read.rs`, over every language until one
  fails, so the time also says which list), and by aezeed's 24 words in
  English (`faraday/faraday-core/src/lightning/mod.rs`). Words typed on
  the on-screen keyboard come in as indices and do not take it.
- `slip39::index_of` (`core/faraday-bip/src/slip39.rs`): the same scan
  over 1024 words with `starts_with`, reached by `Share::parse` on every
  share typed, pasted or read from a file.
- `set11` (`bip39.rs`) writes each bit of a word's index with a branch,
  reached by `Mnemonic::entropy` and `validate_checksum`. `get11` is
  already branch-free.
- `aezeed::pack` (`core/faraday-bip/src/aezeed.rs`) branches on each bit
  of the 24 words, reached by the classifier and every decode; under the
  default passphrase "aezeed" the words are as good as the entropy.
- The sentence for PBKDF2 (`bip39.rs`, the seed's password) is gathered
  word by word with `word_nfkd(w)` by secret index: a load from the
  2048-entry table and a copy whose length follows the word.

**Decided** (owner, 2026-10-31, F180): the BIP-39 core stays Faraday's
(§3.3, §5.5): the bip39 crate's `find_word` is a binary search on four
lists and a scan that stops at the word on the others, its
`from_entropy_in` branches on each bit, and its `to_entropy_array` calls
`find_word`, so it takes the time Faraday's takes today.

**The fix.** Faraday's own form:

- `index_of` and `index_of_nfkd` read all 2048 entries for every word:
  each entry, padded to five 64-bit chunks (40 bytes, the whole chunks
  that hold `MAX_NFKD_BYTES`), is compared in full with the word, padded
  the same way: the differences of the chunks and of the length are
  joined into one value, which goes through `subtle::ConstantTimeEq`,
  and the index is kept by `ConditionallySelectable`; the result is the
  index or none. A word longer than 40 bytes is no word, without a scan.
  Measured in a release build on the English list, one lookup takes
  about 35 µs and `Mnemonic::parse` of a 24-word sentence about 0.83 ms.
  The classifier keeps trying each language, each with the full scan;
  which list a sentence is in is then given by its words, not its time.
- `slip39::index_of`: the first four letters, which identify a SLIP-39
  word, are compared with all 1024 entries' first four in constant time,
  and the rest of the typed word against the entry found, masked.
- `set11`: `let m = 1 << (7 - bit % 8); let set = 0u8.wrapping_sub(((v >> (10 - b)) & 1) as u8); buf[bit / 8] = (buf[bit / 8] & !m) | (m & set);`.
- `aezeed::pack`: `out[at / 8] |= (((word >> (10 - bit)) & 1) as u8) << (7 - at % 8);`.
- The sentence: each word is gathered by a scan of all 2048 entries,
  copying each a chunk at a time into a fixed slot of five 64-bit chunks
  under a mask, and then the slot's bytes up to the word's length into
  the `Secret` buffer. The length of each word, and so of the sentence, still follows
  the words; BIP-39 allows no padding, so this is stated, not hidden
  (it decides how many SHA-512 blocks HMAC's key takes, once per seed).

**What must pass, unchanged.** `tools/vectors/bip39` in every language,
`tools/vectors/slip39`, `tools/vectors/aezeed`; the cross-check areas
`bip39`, `slip39` and `lightning`; the comparison of Faraday's BIP-39
with the bip39 crate (`core/faraday-bip/tests/all/crosscheck.rs`), which
stays; the fuzz target `bip39_words`; the tests of `faraday-bip`,
`faraday-keys` and `faraday-core`; `just crypto-review` for
`words-read`, `words-make`, `seed`, `slip39` and `aezeed`.

**Secret handling that changes.** `words-read`, `seed`: "A word is found
by comparing it with every entry of its list in constant time (subtle),
and the entropy's bits are written without a branch; the sentence is
gathered by a masked scan, and its length follows the words, as BIP-39
requires; the code states no other timing." `slip39`: the word lookup
named the same way, beside the text of §4.3. `aezeed`: "The words are
packed without a branch on their bits", beside §4.2's text.

About a day.

### 4.6 F-6: comparisons of secrets (batch A6)

**Where.** In `faraday/faraday-keys/src/session.rs`:
`Keys::passphrase_is` compares a key's BIP-39 passphrase with a vault
record's by `==`, which stops at the first byte that differs, beside the
constant-time `passphrase_matches`; `with_same_words` compares two keys'
word texts by `==`; `seedqr_reads_as` compares a key's SeedQR digits with
the digits scanned by `==`. In `core/faraday-bip/src/slip39.rs`, the
digest check compares two 4-byte slices by `!=`. In
`faraday/faraday-pgp/src/lib.rs`, `crc24` branches on the CRC's top bit
over paperkey data, which is secret key material.

**The fix.**

- `passphrase_is` calls `passphrase_matches` (one function stays).
  Unequal lengths still return early, as subtle's slice `ct_eq`
  documents; the length is stated.
- `with_same_words` compares the kept `words_id` (SHA-256 of the
  entropy), as `same_words` does.
- `seedqr_reads_as` and SLIP-39's digest compare with
  `subtle::ConstantTimeEq`.
- `crc24`'s step is masked as `crc32.rs`'s is:
  `crc ^= 0x186_4CFB & 0u32.wrapping_sub((crc >> 24) & 1)`.

**What must pass, unchanged.** The tests of `faraday-keys`,
`faraday-bip` and `faraday-pgp`; `tools/vectors/slip39`,
`tools/vectors/paperkey`; GnuPG's and paperkey's judgements in
`faraday-pgp`'s tests; `just crypto-review` for the actions named.

**Secret handling that changes.** `gpg-paperkey`: from "Not applicable:
no secret-dependent comparison is made" to "The armour's and paperkey's
CRC-24 runs without a branch on the data; the code states no other
timing." `slip39`: the digest is compared in constant time.

About two hours.

### 4.7 F-7: BIP-352 tweaks, and contexts that are not randomized (batch A7)

**Where.** BIP-352's label tweak `hash(b_scan ‖ m)` goes through
`PublicKey::add_exp_tweak` in `labelled_spend_key`
(`core/faraday-bip/src/silent.rs`), which is
`secp256k1_ec_pubkey_tweak_add`: it computes `tweak·G` with
libsecp256k1's variable-time `ecmult`, and the tweak comes through
`Scalar::from_be_bytes`, which rust-secp256k1 0.29.1 documents as not
constant time. A label tweak with its address links the label to the
receiver: privacy, not money. `label_point` in the same file already
computes the same point through a secret key.

The output tweak `t_k` (`scan`, the receiver's, and `sender_outputs`)
also goes through `add_exp_tweak`, and stays so. libsecp256k1's own
silent payments module (v0.8.0, commit 6e2c8bc4,
`src/modules/silentpayments/main_impl.h`) computes `t_k` the same way:
`create_output_tweak` declassifies the hashed secret ("The only thing
that the attacker can do with the hashed secret is derive the final
pubkeys"), and the output is `secp256k1_eckey_pubkey_tweak_add`, the
variable-time `ecmult` (`src/eckey_impl.h`). Its label tweak it treats
as secret: `recipient_label_create` computes it with `ecmult_gen` and
does not declassify it. Faraday follows the reference in both.
(Faraday's secp256k1-sys 0.10.1 bundles an older libsecp256k1 without
this module; §6.2.) (`group_scripts` in `core/faraday-psbt/src/silent.rs`
works on shares public to the PSBT's participants and is not changed.)

`docs/KEYS.md` §2 says each master key's curve operations run in a
context randomized from the key and the session. rust-secp256k1 0.29.1's
`Secp256k1::new()` randomizes only with its `rand-std` feature, which
Faraday does not turn on, so these contexts are not randomized:
MuSig2 signing (`core/faraday-psbt/src/sign.rs`), the vanity grinds
(`core/faraday-bip/src/vanity.rs`), and silent payments
(`faraday/faraday-keys/src/silent.rs`'s label points and
`find_payments`, `core/faraday-bip/src/silent.rs`'s `decode_scan_key`).
`ecmult_gen` is constant time either way; randomization is protection
against power and emission leakage on top.

**The fix.** `labelled_spend_key` is the spend key `combine`d with
`label_point` (`SecretKey::from_slice` of the tweak, then
`PublicKey::from_secret_key`: `ecmult_gen`, constant time and blinded),
the sum of two public points, as libsecp256k1's
`create_labeled_spend_pubkey` adds a stored label point; a tweak that is
zero or at or above the order is refused as `checked` refuses it today.
`t_k` is not changed. Each context named above is the key's own
(`MasterKey::secp`, `core/faraday-bip/src/keys.rs`) or one built as
`blinded_context` builds it.

**What must pass, unchanged.** `tools/vectors/bip352`,
`tools/vectors/bip375`, `tools/vectors/bip327`; the cross-check areas
`silent`, `silent_send` and `musig`; the vanity tests; `just
crypto-review` for `silent-receive`, `silent-send`, `sign-musig` and
`vanity`. `docs/KEYS.md` §2's sentence is then true, and is not changed.

**Secret handling that changes.** `silent-receive`: "A label tweak
times G is libsecp256k1's `ecmult_gen` through a secret key, in a
randomized context; the output tweak is treated as public and
multiplied by `ecmult`, as libsecp256k1's own silent payments module
does." `silent-send`: "The output tweak is treated as public and
multiplied by `ecmult`, as libsecp256k1's own silent payments module
does."

About half a day.

### 4.8 Copies of secrets in freed memory

The secrets that matter are wiped where Faraday's own types hold them
(faraday-keys' types, `SecretText`, `Zeroizing`). Other copies are left
in freed heap: buffers that grow, plain copies, other crates' buffers.
On the stick and the Pi each is bounded by the security model: the
app's process restarts at every lock and the kernel zeroes its pages
(`init_on_free=1`), there is no swap, and the computer is powered off
after use (§4.9, F181). They are not chased one by one. Faraday Demo
and the Starter run on an online computer, and no memory or timing work
is done for them alone. Where a manifest line or a module doc claims
more than the code does, it is corrected (§7.8).

### 4.9 A wiping allocator

`core/faraday-crypto/src/wipe.rs`'s `WipeOnFree` is a global allocator
that wipes every block it frees, and the old block on every grow or
shrink (a reallocation is always a move). It is installed in no shipped
program.

**Not installed** (F181). It writes every
freed byte and moves every growing buffer, on every allocation in the
program, so its cost falls on drawing, the camera and the Starter's
writes, most on the Pi; what it adds is a wipe of copies in freed heap
before the process ends. Faraday runs on an offline computer that is
powered off after use, and `docs/SECURITY-MODEL.md` already leaves
cold-boot attacks on RAM out of what it defends against. On the stick
and the Pi, a copy in freed heap is bounded already: the kernel boots
with `init_on_free=1` (`faraday/image/boards/pi3/cmdline.txt`,
`faraday/image/boards/x86_64-uefi/cmdline.txt`), which zeroes pages as
they are freed, the app's process restarts at every lock, and there is
no swap, suspend, hibernation or core dump. The cost is not worth that
gain. The secrets that matter are wiped where they are held (faraday-keys'
types, `SecretText`, `Zeroizing`); the other copies are bounded as §4.8
says.

**What is reopened, and how.** Installing it in any program is a new
decision, made with numbers: the program's late-flow timings with and
without it, its peak memory, the full test suite run with it installed,
the allocator run under Miri, and a fuzz target that grows and shrinks
buffers through it. It is never installed in the website's build.
`wipe.rs` stays in `faraday-crypto`, tested, and the AUDIT row says it is
installed by no binary (§7.3).

**What is not reached**, the stated limits:

1. A secret held in a plain type while it is live: it is in memory, in
   the clear, until it is dropped.
2. A copy in freed heap, until its block is reused or the process ends:
   on the device, the next lock at the latest (the app restarts), or
   power-off for faraday-disk, which outlives the app's locks
   (`faraday/faraday-disk/src/main.rs`); in Faraday Demo and the Starter,
   until the program exits.
3. The stack and registers, including the copies of `Copy` types
   (rust-secp256k1 0.29.1's `SecretKey`, §4.1) and the slots a value was
   moved from; hmac's and pbkdf2's key blocks (§6.7).
4. Memory other code allocates itself: C libraries' own allocators, the
   graphics stack, the window system's text input and clipboard (on the
   desktop another process owns the clipboard), the kernel's pipe
   buffers to faraday-disk, and files written.
5. Outside the process: swap, sleep images and crash dumps. The stick
   and the Pi have none (`docs/SECURITY-MODEL.md`); for Faraday Demo and
   the Starter they are the desktop operating system's to decide.

§7.9 has `docs/KEYS.md` and the manifest say these limits.

### 4.10 Display-only and input-only

These let a secret steer time but are not fixed, because what the timing
gives is what the screen shows at the moment it is computed: someone who
reads the timing learns at most what anyone seeing the screen learns
then. The risk is lower, not nil (a timing channel can be read by
someone who cannot see the screen), and the shared functions of §4.4 to
§4.6 fix most of them anyway.

- Display-only: BIP-39 word display and the predictive keyboard
  (`Language::word`, `candidates`; the words are drawn); the last word's
  candidates (shown); dice, coin and card statistics and warnings
  (`core/faraday-entropy`; the rolls are on the pad as typed); codex32's
  worksheets and corrector (`worksheet.rs`, `correct.rs`'s
  Berlekamp-Massey and Forney steps; every value is a cell shown or a
  correction the person accepts box by box); `compare_words` and
  `another_seed` (the screen says which word differs); xprv, WIF, hex
  and BIP-85 text when shown or written out by the person's choice
  (base58ck's `Display`, BIP-85's base85 and base64 passwords); SeedQR
  and QR encoding (the mask choice and penalty scoring); TOTP's dynamic
  truncation.
- Input-only: the QR decoder over camera frames (rqrr branches on
  pixels; the secret is held up to the camera). This is so for timing
  only: the payload it decodes is a secret in a buffer, bounded as §4.8
  says.
- By design of the algorithm: scrypt's ROMix and Argon2id's second half
  read memory by passphrase-derived values (RFC 7914, RFC 9106 §3.4). The
  manifest's text for `scrypt`, `aezeed`, `vault-kdbx`, `vault-qr-pgp`
  and `kdbx` says so.
- A stated limit: RSA key generation for the Secure Boot keys
  (`faraday/faraday-sb`, `x509::generate`, rsa 0.10.0-rc.18) is not
  constant time. Its prime search, crypto-primes 0.7.2, branches on the
  candidate that becomes p or q: the strong Lucas test's ladder branches
  on every bit of `d` (`src/hazmat/lucas.rs`, `bit_vartime`), and the
  Jacobi symbol, the gcd and Miller-Rabin's loop count are `_vartime`.
  No constant-time prime search exists in Rust, and rsa has no option
  for one. It runs once per key, offline, on the device that will hold
  it; the keys guard boot integrity, not money. Signing is blinded
  (rsa's `rsa_decrypt_and_check` blinds the base with a value drawn from
  a ChaCha20 stream seeded from the system; the CRT exponentiation is
  crypto-bigint 0.7.10's constant-time `pow`).
  RUSTSEC-2023-0071 (Marvin) is a decryption-padding oracle and is not
  reachable: Faraday never decrypts with RSA, signs only digests of what
  it built, and has no network (`deny.toml` ignores it with that
  reason). Checked again when rsa 0.10.0 is released or crypto-primes
  gains a constant-time Lucas ladder.

## 5. The switches

Each subsection is one batch (§8). In every switch the function a caller
uses keeps its signature, so the choke point (AUDIT §1), the key
boundary (F70: no key leaves `faraday-keys` but through its reveals) and
the compute token (`c: &Compute`, `docs/COMPUTE.md`) are unchanged; what
Faraday refuses before the call is unchanged. Every switch also does
these, which the subsections do not repeat:

1. **Manifest (F179).** `[library.<crate>] author = "<the project's
   developers>"` is added to `docs/crypto/actions.toml` for a crate first
   named; the crate is named in `implementation.libraries` of each action
   whose functions call it directly and in `implementation.through` (with
   the Faraday crate it is reached through) of each action that reaches
   it through another crate; a library no longer reached is dropped;
   `faraday/faraday-learn/src/how.rs` is regenerated
   (`docs/crypto/gen_how.py`).
2. **Inventory.** A crate that computes cryptography is added to
   `PRIMITIVE_CRATES` (`tools/costs-lint/src/inventory.rs`) in the batch
   that makes it a direct dependency: `blake2` (C1), `x509-cert` (C5).
   `der`, `miniscript`, `base64ct`, `base16ct`, `serde_json`, `adler2`,
   `subtle` and `zeroize` compute nothing on a key and are not.
3. **Locks.** `Cargo.lock`, `fuzz/Cargo.lock` and `faraday/fuzz/Cargo.lock`
   change with any crate a crate they build gains; each subsection's "New
   in the graph" is for the main lock, and the batch updates the fuzz
   locks with `cargo update -p <crate> --offline` in each fuzz workspace.
4. **Trust.** A new trust entry is written with `start` the date of the
   first version the publisher published, `end = "2027-10-01"` and
   `notes = "F71: trusted publisher (docs/decisions/F71.md)"`, and `just
   vet-refresh` (online) records the publisher in
   `supply-chain/imports.lock` in the same commit.
5. **deps page.** A new direct dependency gets its `docs/deps/` page
   (rule `deps.page`); a crate already a direct dependency elsewhere has
   its page's purpose extended.
6. **Fingerprints.** They and the stamp wait for the release (F197);
   `just crosscheck --area NAME` runs where an area's code changed.
7. **AUDIT.** The `written_here` string of the action changes as given
   (before D1 makes it an entry), and AUDIT §3's line, and §1's row where
   named, with it (`check_audit_crates`).

### 5.1 BLAKE2b (batch C1, built)

**Built.** `core/faraday-crypto/src/blake2b.rs` is a wrapper of about 25
lines over `blake2` 0.11.0's `Blake2bVarCore`; RFC 7693's vectors and every
length 1 to 64 are in `core/faraday-crypto/tests/primitives.rs`. What
follows is the plan it was built from.

- **Files.** `core/faraday-crypto/src/blake2b.rs` (179 lines) becomes a
  wrapper of about 25 lines; `core/faraday-crypto/Cargo.toml`.
- **Crate.** `blake2` 0.11.0 (RustCrypto/hashes), `default-features =
  false, features = ["zeroize"]` (its `Drop` wipes the state under it).
  blake2 0.11.0 has no `Blake2bVar` (removed in 0.11.0); it has
  `Blake2b<OutSize>` with the size a type, and `Blake2bVarCore` with the
  size given at run time through digest's `VariableOutputCore`.
- **The wrapper.** `faraday_crypto::blake2b(data, out)` keeps its
  run-time length, 1 to 64 bytes, and its panic outside that, over
  `Blake2bVarCore`: `VariableOutputCore::new(out.len())`, a
  `BlockBuffer` for the blocks, `finalize_variable_core`, then the first
  `out.len()` bytes; the core's output buffer is wiped after the copy.
- **New in the graph.** None: blake2 0.11.0 is in `Cargo.lock` through
  argon2. The edge `faraday-crypto -> blake2` is new.
- **cargo vet.** `[[trusted.blake2]]` exists.
- **deps page.** New: docs/deps/blake2.md.
- **Manifest.** `aezeed`'s `through` gains `{ library = "blake2", crate =
  "core/faraday-crypto" }`; `[library.blake2] author = "the RustCrypto
  developers"`; `scrypt`'s `written_here` becomes "scrypt
  (`faraday-crypto::scrypt`)". AUDIT §3 loses "BLAKE2b".
- **Proof.** RFC 7693's vectors, today inline in `blake2b.rs`, move to
  `core/faraday-crypto/tests/primitives.rs`, with the existing tests at
  every length 1 to 64; `tools/vectors/aezeed`; area `lightning`.
- **Size.** About three hours.

### 5.2 HKDF: not a switch

Decided (owner, 2026-10-31, F180): HKDF stays Faraday's, `does-not-fit`
(§3.3). hkdf 0.13.0 wipes its HMAC state on drop (through `hmac/zeroize`,
`sha2/zeroize` and `block-buffer/zeroize`, which `faraday-crypto` turns
on) but returns the pseudorandom key as a plain `Output<H>`
(`GenericHkdfExtract::finalize`), discards it unwiped in
`GenericHkdf::new`, and keeps each output block T(i) in a plain
`Option<Output<H>>` it does not wipe; for a 32-byte output T(1) is the
key. Faraday's code holds the key in `Secret` and T in `Zeroizing`. A
change that wipes them is proposed to RustCrypto/KDFs, as for scrypt;
§6.7 says when the switch is made. The two copies became one in batch
C15 (§5.17). F178's trust entry for `hkdf` is not written.

### 5.3 AEZ's AES round (built)

Built with §4.2: `aes` 0.9.3 (RustCrypto/block-ciphers), feature
`hazmat`, `default-features = false`; `[[trusted.aes]]` with
`trusted-publisher = "github:RustCrypto/block-ciphers"`, start
2026-04-10, recorded in `supply-chain/imports.lock`; docs/deps/aes.md;
`aes` in `PRIMITIVE_CRATES`. AEZ's mode, `Extract`, `AES4`, `AES10` and
the doubling stay Faraday's (§3.2).

### 5.4 Scalar arithmetic

Built as §4.1 (batch A1). No new crate: secp256k1 0.29.1 is in the tree
and trusted.

### 5.5 The BIP-39 core: not a switch

Decided (owner, 2026-10-31, F180): the BIP-39 core stays Faraday's,
`does-not-fit` (§3.3), made constant time in batch A5 (§4.5), because
the bip39 crate's word lookup and bit packing take a time that follows
the words. The bip39 crate stays what the tests compare against: a
dev-dependency of `faraday-bip` at 2.2.2 under its exemption,
`core/faraday-bip/tests/all/crosscheck.rs` and area `bip39`
(python-mnemonic) as today. F178's trust entry for the bip39 crate's
publisher is not written; the crate's three publishers (stevenroose,
elsirion, who published 2.2.2, and tnull, who published 3.0.0) are
recorded in its deps page should it be taken later (§6.9). The
`words-read` written-here string becomes two entries at D1 (§3.1, §3.3);
before D1 it stays "BIP-39 mnemonics (`faraday-bip::bip39`)".

### 5.6 The RSA RNG adapter (batch C4, built)

- **Files.** `faraday/faraday-sb/src/rng.rs` (now a constructor of about 10 lines), its callers in
  `faraday/faraday-sb/src/lib.rs`.
- **Crate.** `chacha20` 0.10.2's `ChaCha20Rng`: `faraday-sb`'s chacha20
  gains feature `rng` (over `rand_core` 0.10), with `zeroize`, under
  which the generator is `ZeroizeOnDrop` and wipes its block buffer.
- **New in the graph.** None: chacha20's dependency list gains
  `rand_core` 0.10.1, in the tree, covered by imported audits.
- **cargo vet.** No change.
- **deps page.** `docs/deps/chacha20.md` adds the Secure Boot keys'
  generator to its purpose.
- **Kept in front.** One fresh 32-byte draw as the seed, taken as today
  from the system with the person's own randomness hashed in;
  `Stream::new(seed, what)` becomes `ChaCha20Rng::from_seed(seed)` then
  `set_stream(u64::from(what))` before any output, with the same `what`
  values as today (the GUID and serials, the key, the blinding).
- **Output.** `ChaCha20Rng` lays out its counter and stream as djb's
  ChaCha20 (64-bit counter, 64-bit stream), not RFC 8439's 32-bit
  counter and 96-bit nonce, and buffers its blocks, so one seed gives
  different keys than today. Keys already made are unaffected: they are
  stored in a vault. `faraday/faraday-sb/tests/golden.rs` is re-pinned;
  the test in `faraday/faraday-sb/tests/judged.rs` that checks the
  stream against RFC 8439 Appendix A.1 is replaced by one that the
  stream for a seed is the crate's, and every key made is judged by
  `openssl pkey -check` and every image by `sbverify` as today.
- **Manifest.** `sb-keys` keeps `chacha20` and `rand_core`; its `adds`
  drops the adapter; `written_here` becomes "X.509/CMS structure and
  Authenticode signing (`faraday-sb`)".
- **Size.** About half a day with the re-pinning.

### 5.7 X.509 and DER (batch C5, built)

**Built.** The hand-written DER writer is gone. `x509.rs` builds the
certificates with `x509-cert`'s `CertificateBuilder` (a profile of
Faraday's three extensions in today's order) and signs through `rsa`'s
`pkcs1v15::SigningKey<Sha256>`, which meets the builder's bounds
(`build_with_rng` with C4's blinding stream) with `faraday-sb`'s rsa
features; reads a certificate with `Certificate::from_der`. `cms.rs`
writes and reads the SignedData, SignerInfo, ContentInfo and
Authenticode structures as `der` types. The golden file is unchanged:
every certificate, `.auth` and signed image is the same bytes for the
same draw, and `openssl` and `sbverify` judge them as before. The
cross-check's change (batch X2, §9.7) is built: GnuTLS's `certtool` reads
every certificate made, `pesign` and `pesigcheck` read the real signed
binaries of `tools/reference/signed-efi/` and name their signer and image
hash as Faraday does, and python3-pefile reads the checksum and
certificate-table entry of every image. `pesigcheck` does not verify an
image Faraday signs: NSS takes only a signer certificate that is not a CA
and carries the code-signing extended key usage, and the db certificate
(§5.7's profile, kept so that every certificate is the same bytes) is a CA
with key usage alone; `pesigcheck` also aborts on an image with a byte
changed. Whether the db certificate's profile changes is the owner's call.

- **Files.** The DER writer (164 lines) and
  `faraday/faraday-sb/src/x509.rs` (336 lines); the CMS, Authenticode
  and EFI code that builds DER with that writer
  (`faraday/faraday-sb/src/pe.rs`, `faraday/faraday-sb/src/efi.rs`),
  now through `faraday/faraday-sb/src/cms.rs`.
- **Crates.** `x509-cert` 0.3.0, `default-features = false, features =
  ["builder"]` (`pem` and `std` off: Faraday writes DER), for the
  self-signed certificates and their reading; `der` 0.8, a direct
  dependency, `default-features = false, features = ["alloc", "oid"]`,
  for the CMS SignedData and Authenticode structures, written as `der`
  types (§3.2: only those structures stay Faraday's). The builder signs
  through `signature` 3's traits: the signer is rsa's
  `pkcs1v15::SigningKey<Sha256>` with its `RandomizedSigner` taking C4's
  blinding stream. Whether that type meets the builder's bound with
  `faraday-sb`'s rsa features is not checked; the batch checks it first
  and stops if it does not.
- **New in the graph.** `x509-cert` 0.3.0; its `der` features `derive`
  and `flagset` add `der_derive` 0.8.0 (a proc-macro over proc-macro2,
  quote and syn 2, in the tree) and `flagset` 0.4.7. sha1 0.11.0,
  signature 3.0.0, spki 0.8.0, const-oid 0.10.2 and der 0.8.2 are in the
  tree.
- **cargo vet.**
  - `[[trusted.x509-cert]]`, `trusted-publisher =
    "github:RustCrypto/formats"`, `start = "2026-07-09"` (0.3.0, its
    first release by trusted publishing).
  - `[[trusted.der_derive]]`, `user-id = 267 # Tony Arcieri (tarcieri)`,
    the RustCrypto person F71 already trusts for base16ct, base64ct,
    const-oid and others, `start = "2020-01-01"`.
  - `[[exemptions.flagset]]` 0.4.7, `safe-to-deploy`, with the note that
    Mozilla's audit covers 0.4.3 (F71: an exemption; no deps page, as it
    is not a direct dependency).
- **deps pages.** New: docs/deps/x509-cert.md and docs/deps/der.md.
- **Kept in front.** The certificates' fields as today (subject, serial
  from the draw, validity, the key usage and extensions written today);
  the reader's refusals of a certificate that is not the shape Faraday
  wrote, which the batch lists in docs/deps/x509-cert.md from today's
  `x509.rs` before replacing it, and each keeps a test.
- **Manifest.** `sb-keys`' `libraries` gain `x509-cert` and `der`;
  `[library.x509-cert]` and `[library.der]`, author "the RustCrypto
  developers"; `x509-cert` in `PRIMITIVE_CRATES`; `written_here`
  becomes "CMS SignedData, Authenticode's image hash and signature, EFI
  signature lists and `.auth` files (`faraday-sb`)".
- **Proof.** `faraday/faraday-sb/tests/judged.rs`: every certificate and
  `.auth`'s PKCS #7 judged by `openssl`, signed images by `sbverify`;
  `faraday/faraday-sb/tests/golden.rs` gives the same bytes for the same
  draw (DER is canonical; where x509-cert orders an extension
  differently, the golden file changes and `openssl` still judges it).
  The Starter's check of a signed boot image (`starter-signed`) reads
  the same.
- **Size.** One and a half to two days.

### 5.8 The single-key and multisig finalizer (batch C6, built)


**Built.** `build_final` calls miniscript's `finalize_inp_mut` for every script kind, with the interpreter check; a plain multisig carries the first m signers' signatures as Bitcoin Core writes them, and only a miniscript is given the smallest satisfaction by signature size. Area `sign` also asks btclib-wallet's finalizer for the scriptSig and witness of every input Faraday finalizes (batch X3, §9.7). What follows is the plan it was built from.

- **Files.** `core/faraday-psbt/src/finalize.rs` (`build_final` and
  `push`).
- **Crate.** `miniscript`, in the tree and trusted: its
  `finalize_inp_mut`, which Faraday already calls for miniscript and
  taproot inputs, also infers pk, pkh, wpkh, sh-wpkh, and multi under sh,
  wsh and sh-wsh from the PSBT's own scripts (`get_descriptor` in
  `src/psbt/finalizer.rs`). The batch stays on 12.3.7; 12.3.8
  (2026-10-27) is taken by the ordinary dependency update, not here.
- **New in the graph.** None.
- **cargo vet, deps page.** No change; `docs/deps/miniscript.md` adds
  the finalizer for every script kind.
- **Kept in front.** The interpreter check on every witness; the choice
  of the smallest satisfaction by real signature size for a miniscript
  (a `multi()` inside it keeping its k smallest signatures); a plain
  multisig, under `sh`, `wsh` or `sh(wsh)`, carrying the first m
  signers' signatures as Bitcoin Core's `finalizepsbt` writes them (§3.1);
  the signatures under a taproot `pkh` leaf taken off as today;
  the finalizer run on a copy of the input, as `satisfy` does today,
  because miniscript's `finalize_input` drops the unknown fields:
  Faraday keeps the UTXO and unknown fields and clears the signing
  fields and the hash preimages as BIP-174 requires, and the
  proprietary fields too (BIP-174 does not require keeping them).
- **Proof.** `tools/vectors/psbt` (BIP-174's finalizer and extractor
  vectors); area `sign` (Bitcoin Core's `finalizepsbt` and
  `testmempoolaccept`); the wallet fixtures
  (`core/faraday-psbt/tests/all/wallet_fixtures.rs`). The transactions
  are byte for byte the same.
- **Size.** One to two days.

### 5.9 The sealing's counter nonce, removed (batch C3, built)

**Built.** `core/faraday-crypto` has no `Sealed`, `SessionKey`, `SealError` or
`SealedBytes`, and no `chacha20poly1305` dependency; `MnemonicBytes`,
`SeedBytes`, `MAX_SEED_LEN` and `MAX_MNEMONIC_WORDS` are in
`core/faraday-crypto/src/secret.rs`. `KEY_LEN`, `NONCE_LEN` and `TAG_LEN` had
no user and went. What follows is the plan it was built from.

- **Files.** The sealing's file (653 lines, `sealed.rs`):
  `Sealed`, `SessionKey`, `SealError`, `SealedBytes`, the counter nonce,
  and `KEY_LEN`, `NONCE_LEN` and `TAG_LEN` (unless a user is found, then
  they move with it) go; `MnemonicBytes`, `SeedBytes`, `MAX_SEED_LEN` and
  `MAX_MNEMONIC_WORDS`, which `faraday-bip`, `faraday-keys` and
  `tools/crosscheck`'s Rust use, move to `core/faraday-crypto/src/secret.rs`.
  `core/faraday-crypto/src/lib.rs`'s exports and module doc; the
  comment in `core/faraday-bip/src/keys.rs` that waits for a
  `Sealed<T>`; `tools/lint-secrets.sh`: rule 2's type list loses `Sealed`
  and `SessionKey` (the allowed name `backup_flow`'s `Sealed` in
  `faraday-core` stays), and `SECRET_FILES` loses `sealed.rs` and gains
  `core/faraday-crypto/src/secret.rs` if it is not there.
- **New in the graph.** None; `faraday-crypto` drops its
  `chacha20poly1305` dependency, and `faraday-vault` has none.
- **Reason.** No shipped code builds a session key or seals a value
  (the manifest's `hold`; STANDARDS §7 item 3). Should sealing in memory
  come back, it is XChaCha20-Poly1305 with a random nonce from the same
  crate, and a decision.
- **Manifest.** `hold`'s `written_here` goes; its `libraries` drop
  `chacha20poly1305` (and `subtle` if nothing else in its functions uses
  it); its `adds` drops the sentence on the in-memory sealing and its
  `limits` "Nothing held in memory is sealed"; how.rs is regenerated.
  STANDARDS §7 loses item 3 (generated); AUDIT §1's `faraday-crypto` row
  drops `Sealed`/`SessionKey`.
- **Proof.** The tests of `faraday-crypto`, `faraday-keys` and
  `faraday-core`.
- **Size.** About two hours.

### 5.10 Base64, four copies (batch C8, built)

**Built.** `faraday_bip::base64`, the armour in `faraday-pgp`, the KDBX
writer in `faraday-backup` and the Starter's plist reader are
`base64ct` 1.8.3's. A PSBT with padding bits that are not zero, or a
missing `=`, is refused (`core/faraday-psbt/tests/all/vectors.rs`); no
BIP-174 vector or wallet fixture carries one. What follows is the plan it
was built from.

- **Files.** `core/faraday-bip/src/base64.rs` (170 lines; PSBT v0 and v2,
  BIP-322, BSMS, BIP-85's password through `encode_into`);
  `faraday/faraday-pgp/src/lib.rs` (`base64`, the armour's encoder, and
  `Base64Reader` with `base64_value`);
  `faraday/shells/starter/src/macos/plist.rs` (`base64`, `diskutil`'s
  data).
- **Crate.** `base64ct` 1.8.3 (RustCrypto/formats), `default-features =
  false, features = ["alloc"]`: `Base64::encode_string`,
  `Base64::decode_vec`, and the slice forms `encode` and `decode`. It is
  `no_std` and constant time with respect to the data (not the length).
- **The forms.** `faraday_bip::base64::{encode, encode_into, decode}`
  keep their signatures over base64ct, `DecodeError` mapped from
  `base64ct::Error`; the module's reason sentence (bitcoin's base64
  feature pulls `std`) goes. faraday-pgp's armour encoder is
  `encode_string`; `Base64Reader` is replaced by gathering the body's
  characters with CR, LF, space and tab removed into one buffer and
  `decode_vec` (armour line widths vary between writers, so
  `Decoder::new_wrapped` is not used). the vault's KDBX XML writer encodes
  protected values with `encode_string`, and its reader decodes them strictly. plist's strips ASCII white space, then
  `decode_vec`.
- **Edge cases.** base64ct refuses padding bits that are not zero and a
  missing `=`; today's decoders accept the first. Faraday refuses them
  too (RFC 4648 §3.5 allows it, and an encoding with a second spelling
  is not taken). A test feeds a PSBT with non-zero padding bits and sees
  it refused. If a BIP-174 vector or a wallet fixture carries such a
  PSBT, the batch stops and the owner is told.
- **New in the graph.** None: base64ct 1.8.3 is in the lock (through
  argon2, spki and pem-rfc7468). `faraday-bip`, `faraday-pgp`,
  `faraday-backup` and the Starter's shell gain the edge.
- **cargo vet.** `[[trusted.base64ct]]` exists (tarcieri, 267).
- **deps page.** New: docs/deps/base64ct.md, listing the four users.
- **Manifest.** `bip85`'s `secrets.constant_time`: from "not stated by
  the library" to "The password's base64 is the base64ct crate's, which
  documents its encoding as constant time with respect to the data; its
  base85 is shown (§4.10)." `[library.base64ct] author = "the RustCrypto
  developers"`; `libraries` and `through` of `psbt-read`, `psbt-v2`,
  `message-sign`, `message-verify`, `bsms`, `bip85`, `gpg-create`,
  `kdbx` and the actions §5 item 1 finds.
- **Proof.** `tools/vectors/psbt`, `tools/vectors/psbt-bips`,
  `tools/vectors/message`, `tools/vectors/bip129`, `tools/vectors/bip85`;
  areas `psbt`, `msg`, `bip85`, `vault`, `import`; GnuPG reading the
  armour in `faraday-pgp`'s tests; the Starter's macOS plist fixtures;
  RFC 4648 §10's vectors in `faraday-bip`'s tests.
- **Size.** About half a day.

### 5.11 Hex, one copy (batches C9 and X4)

- **Files.** The copies carrying secrets:
  `faraday/faraday-intake/src/secret_buf.rs` (`hex`, private keys, which
  `faraday/faraday-keys/src/lightning.rs`'s `private_hex` reaches),
  `faraday/faraday-keys/src/bip85.rs` (`hex`, a duplicate of the last,
  which goes), `core/faraday-entropy/src/hex.rs` (`RawHex`, typed
  entropy), `faraday/faraday-pgp/src/paperkey.rs` (paperkey's base16 of
  secret key material). The public copies:
  `core/faraday-codec/src/encodings.rs`, `core/faraday-psbt/src/verify.rs`,
  `core/faraday-psbt/src/sign.rs`, `faraday/faraday-core/src/backup_flow/mod.rs`,
  `faraday/faraday-core/src/backup_flow/gpg_restore.rs`,
  `faraday/faraday-core/src/backups/mod.rs`, `faraday/faraday-core/src/decode/mod.rs`,
  `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-qr/src/bbqr.rs`,
  `faraday/faraday-qr/src/envelope.rs`, `faraday/faraday-payload/src/lib.rs`,
  `faraday/shells/starter/src/windows/disks.rs`,
  `faraday/shells/starter/src/windows/elevated.rs`,
  `faraday/faraday-intake/src/single_key.rs`, `core/faraday-selftest/src/lib.rs`,
  and `faraday/faraday-vault/src/contents.rs` (`hex` and `unhex`, the
  fingerprints in a vault's TOML, added since).
  (Each was read on 2026-10-31; the batch greps for 16-entry hex tables
  and `to_digit(16)` and `from_str_radix(.., 16)` over bytes and takes
  every hit outside tests.)
- **Crate.** `base16ct` 1.0.0 (RustCrypto/formats), `default-features =
  false, features = ["alloc"]`: `lower::encode`, `lower::encode_str`,
  `mixed::decode`, `decoded_len`; `no_std`, "without data-dependent
  branches or lookup tables". hex-conservative, also trusted and in the
  lock, looks digits up in a table and is not used for this.
- **One place.** A module `faraday_crypto::hex` with
  `encode_lower(bytes, out: &mut [u8]) -> &str`,
  `encode_secret(bytes) -> Zeroizing<String>` and
  `decode_into(text, out: &mut [u8]) -> Option<usize>` (either case, the
  length checked first), over base16ct. Every crate above that depends
  on `faraday-crypto` calls it; `faraday-payload`, which does not,
  calls base16ct directly, and `faraday-pgp` depends on it directly too,
  for paperkey's `upper` and `mixed`. Upper-case output, where a
  format asks for it (paperkey), is `upper::encode`.
- **New in the graph.** None: base16ct 1.0.0 is in the lock (through
  serdect). The edges from `faraday-crypto`, `faraday-pgp` and
  `faraday-payload` are new.
- **cargo vet.** `[[trusted.base16ct]]` exists (tarcieri, 267).
- **deps page.** New: docs/deps/base16ct.md.
- **Manifest.** `[library.base16ct]`; the actions §5 item 1 finds
  (at least `gpg-paperkey`, `bip85`, `coins-hex`,
  `lightning-node`).
- **Proof.** RFC 4648 §10's base16 vectors in `faraday-crypto`'s tests;
  `tools/vectors/paperkey` (paperkey 1.6 reads what is written);
  `tools/vectors/bip85`; `tools/vectors/entropy`; the tests of every
  crate touched.
- **Size.** About half a day.

### 5.12 The descriptor checksum (batch C10)

- **Files.** `core/faraday-bip/src/descriptor.rs`: `Engine`,
  `INPUT_CHARSET`, `CHECKSUM_CHARSET`, `GENERATOR` go;
  `descriptor_checksum`, `verify_checksum` and `checksum_facts` keep
  their signatures.
- **Crate.** `miniscript`, in the tree and trusted:
  `miniscript::descriptor::checksum::desc_checksum(&str) -> Result<String,
  Error>`, which works on the text, so it serves BIP-392's `sp(..)`
  descriptors too. `descriptor_checksum` is it, its error giving `None`;
  `verify_checksum` splits at the ninth character from the end as today
  and compares `desc_checksum(body)` with the given eight (miniscript's
  own `verify_checksum` is not public).
- **New in the graph.** None.
- **cargo vet, deps page.** No change; `docs/deps/miniscript.md` adds the
  checksum.
- **Manifest.** `descriptor`'s functions keep their names; `libraries`
  already names miniscript.
- **Proof.** Area `addr` (Bitcoin Core's `getdescriptorinfo` and BIP-380's
  code); the wallet fixtures; `tools/vectors/bip392` for `sp()`.
- **Size.** About two hours. Built (batch C10).

### 5.13 The JSON reader (batch C11)

- **Files.** `core/faraday-bip/src/json.rs` (402 lines): the parser goes;
  `Value`, `Error`, `parse`, `quote` and `Value`'s accessors stay;
  `faraday/faraday-intake/src/wallet.rs` (`holds_secret`).
- **Crate.** `serde_json` 1.0.151 (dtolnay), `default-features = false,
  features = ["alloc", "float_roundtrip"]`, moved in `faraday-bip` from dev-dependencies to
  dependencies, with `serde` (`default-features = false, features =
  ["alloc"]`). Whether `faraday-bip` builds with `alloc` alone is not
  checked: the batch checks it first and stops if not.
- **Kept in front.** The 1 MiB limit, checked on the text before it is
  read. A `DeserializeSeed` for Faraday's `Value` (the visitor of §3.4)
  carries the depth and refuses past 32 (serde_json's own limit of 128
  stays behind it), and refuses an object with a key named twice
  (`Error::DuplicateKey`). Objects keep their members in the order
  written. A number becomes `Value::Number` holding the decimal text Rust
  writes for the visitor's `u64`, `i64` or `f64` (so `1e3` reads as
  `1000`); `arbitrary_precision` is not turned on, since it changes
  serde_json for every user in the workspace; `float_roundtrip` is, so that
  the decimal text reads back as the same value. Callers ask numbers only
  through `as_u64`; the test in `core/faraday-bip/tests/all/json.rs` that
  expects a number "as written" changes to this rule.
- **`holds_secret`.** An Electrum file's keystore is checked for `xprv`,
  `seed` and `keypairs` by deserializing it into a struct whose fields
  are `Option<serde::de::IgnoredAny>`: no `String` is built for any
  value, so no copy of a secret is made beyond serde_json's scratch
  buffer for a string with escapes. The manifest's `wallet-files` names
  `faraday_intake::wallet::holds_secret` in its functions, and its
  secret text, today "Nothing secret is held", says: "An Electrum file's
  keystore is checked for a private key or seed without its values being
  kept; the file's own bytes are the caller's."
- **New in the graph.** None: serde_json and its dependencies (itoa,
  memchr, serde_core, zmij) are in the lock.
- **cargo vet.** `[[trusted.serde_json]]` exists (dtolnay, 3618).
- **deps page.** New: docs/deps/serde_json.md.
- **Manifest.** `wallet-files`' `adds` changes "JSON is read by
  Faraday's own reader" to "JSON is read by serde_json, behind
  Faraday's refusal of a key named twice and of a nesting deeper than
  32"; `[library.serde_json] author = "David Tolnay"`.
- **Proof.** RFC 8259's examples and the duplicate-key and depth tests in
  `core/faraday-bip/tests/all/json.rs`; area `import`; the wallet
  fixtures; the fuzz target `json`.
- **Size.** About a day.

### 5.14 Adler-32 (batch C12)

- **Files.** `core/faraday-codec/src/png.rs`: `adler32` goes.
- **Crate.** `adler2` 2.0.1, `default-features = false`:
  `adler2::Adler32::new()`, `write_slice`, `checksum`.
- **New in the graph.** None: in the lock through miniz_oxide; the edge
  from `faraday-codec` is new.
- **cargo vet.** Established by imported audits (Google's full audit of
  2.0.0, Mozilla's and Zcash's deltas to 2.0.1).
- **deps page.** New: docs/deps/adler2.md.
- **Proof.** RFC 1950's check values in `faraday-codec`'s tests; every
  PNG the tests write read by the png crate's decoder as today.
- **Size.** Under an hour.

### 5.15 The zero check, the wipes, one Argon2id call (batch C13)

- **The zero check: withdrawn.** `committed` in
  `faraday/faraday-vault/src/lib.rs` was the FVLT container's commitment
  block, gone with that container; KDBX and the QR vault's message have
  no such block.
- **Files.** The Argon2 memory's `fill` and `black_box` in
  `core/faraday-backup/src/lib.rs`, `faraday/faraday-vault/src/kdbx/keys.rs`
  and `faraday/faraday-pgp/src/message.rs`, and those of
  `faraday/faraday-fat/src/lib.rs` (a cluster block of file data), become
  `zeroize::Zeroize::zeroize` (argon2's `Block` implements `Zeroize`).
- **One Argon2 call.** `faraday-backup`'s `argon2id`, the KDBX
  container's `argon_run` and the OpenPGP message's `s2k` are the same
  function: reserve the memory, `hash_password_into_with_memory`, wipe.
  It moves to `faraday_crypto::argon2(algorithm, password, salt, params,
  out: &mut Secret<..>)`, which takes `c: &Compute` and says "Costs an
  Argon2 derivation" in its first line, the algorithm Argon2id or, for a
  KeePass file read, Argon2d; all three call it, and their parameters
  and errors stay as they are. The FVLT container's own call
  (`derive`) went with that container and was not moved.
- **New in the graph.** None; `faraday-fat` gains `zeroize` (trusted, in
  the tree).
- **deps page.** `docs/deps/zeroize.md` adds the users.
- **Manifest.** `kdbx`, `vault-kdbx` and `vault-qr-pgp` gain `through`
  argon2 by `core/faraday-crypto`.
- **Proof.** `tools/vectors/kdbx`, `tools/vectors/kdf`,
  `tools/vectors/openpgp` (with its ignored Argon2 S2K test run); area
  `vault`, which judges the KDBX vault (the QR vault is `docs/VAULT.md` §10's
  to build); the tests of `faraday-fat`.
- **Size.** About two hours.

### 5.16 The quiz's seed (batch C14)

- **Files.** `faraday/faraday-core/src/quiz.rs`: `Quiz::seed` is
  `now_ms ^ SHA-256(word indices)[..8]`, feeding a xorshift64* whose
  outputs order the questions and pick the decoy words on screen: 64 bits
  of a hash of the words, in a generator whose output is shown.
- **The fix.** The seed is 8 bytes from the session's generator
  (`faraday-core::fresh`); the words leave it. The xorshift generator
  stays (it orders a quiz; it guards nothing).
- **Manifest.** `words-identity`'s `adds` drops the seed from the words.
- **Proof.** The quiz's tests, changed to the rule: a quiz's order does
  not follow from the words alone.
- **Size.** About an hour.

### 5.17 One copy of each kept piece (batch C15)

Built; nothing is open.

The pieces §3 keeps are each in one place:

- **GF(32) and the bech32 alphabet.** `faraday-bip` exports its
  `gf_mul` (§4.4) and the alphabet; the Starter's blank templates
  (`faraday/faraday-starter/src/sheets.rs`: `gf_mul`, `ALPHA`, `val`,
  `up`) use them. The alphabet constants in
  `faraday/faraday-vault/src/codex32.rs`, `faraday/faraday-keys/src/codex32.rs`,
  `faraday/faraday-core/src/codex32/mod.rs`,
  `faraday/faraday-core/src/codex32_correct/mod.rs`,
  `faraday/faraday-core/src/plan/codex32.rs`,
  `core/faraday-entropy/src/booklet.rs`, `core/faraday-bip/src/address.rs`
  and `core/faraday-bip/src/vanity.rs` become `faraday_bip`'s, or
  `bech32::Fe32::from_char` and `to_char` where the text is public.
- **GPT, MBR and the boot sector (built).** `gpt::Table` carries the
  header fields `faraday/shells/starter/src/ids.rs` used to read again
  after `gpt::read` (header size, entries' sector, count and size, the
  backup header's sector), and faraday-fat has a public `BootSector`
  (bytes per sector, sectors per cluster, reserved, FATs, root entries,
  FAT size, root cluster, the serial's offset and the backup boot
  sector) that `ids.rs` (`root_region`, `serial`) and `boot_sector_serial`
  both use. `Volume::open` keeps its own reads: its checks come between
  the fields. `gpt.rs`'s `BOOT_AT`, `BOOT_LEN`, `DATA_AT` and `DATA_LEN`
  are gone: the Starter uses `faraday/faraday-boot/src/layout.rs`'s
  `BOOT_START`, `BOOT_SECTORS`, `DATA_START` and `DATA_SECTORS`.
- **CBOR.** `core/faraday-codec/src/ur/`'s `cbor_bytes`,
  `cbor_byte_string` and the fountain part's CBOR, and
  `faraday-qr`'s own `cbor.rs`, become one module in
  `faraday-codec` (`src/cbor.rs`, built), which `faraday-qr` uses.
- **PDF text.** `faraday/faraday-core/src/pdf.rs`'s `pdf_text` and the
  Starter's narrower copy in `sheets.rs` become one, `faraday-core`'s.
- **HKDF (built).** `faraday/faraday-pgp/src/message.rs`'s `hkdf`, an
  extract through `faraday_crypto::hmac_sha256` before
  `hkdf_sha256_expand`, is `faraday_crypto::hkdf_sha256(salt, ikm, info,
  out: &mut [u8])` beside `hkdf_sha256_expand`, the pseudorandom key in
  a `Secret`; the message calls it with the salts it uses today. `out` is
  a plain slice, as the expand's is: both callers fill a sub-slice of a
  buffer they wipe, which a `Secret` would copy. The FVLT
  container's own `hkdf_sha256` went with that container and was not
  moved. RFC 5869's cases 1 to 3 run through the whole function in
  `core/faraday-crypto/tests/primitives.rs`.

**Proof.** The tests of every crate touched; the Starter's stick and
card tests (`tests/all/sticks.rs` in its shell); `tools/vectors/ur`,
`tools/vectors/openpgp`; areas `ur` and `sign`, and `qrvault`
(`docs/VAULT.md` §10); the blank templates'
snapshots unchanged.

**Size.** About a day. It is code more than one app uses: the device
app's and the Starter's builds both follow it.

## 8. Order of work

Each batch is one commit and one check: the module touched while
working, `just crosscheck --area NAME` where a cross-check area's code
changed, then `just` at the end. Each batch edits this document's `**Status:**` and
`**Open:**` lines to name what is left, and `docs/README.md`'s line with
them. The batch that builds the last open item moves §4, §5, §8 and §9.7 to
docs/history/ESTABLISHED-LOG.md and makes the document a Reference
(CLAUDE.md); the work kept for later (§6) is then its own Spec or the
Reference's list of conditions.

| Batch | What | Sections | Size |
|---|---|---|---|
| A0 | The inventory's name matching; the actions it moves re-stamped | §2.5 | Built |
| A1 | Scalar arithmetic on secrets through libsecp256k1, from the written batch; the re-stamps it lacks; with X8 | §4.1, §5.4, §9.7 | Built, with X8 |
| A2 | AEZ's AES round from `aes` | §4.2, §5.3 | Built |
| A3 | SLIP-39's GF(256) and checksum without a branch | §4.3 | Built |
| A4 | codex32's GF(32), checksum and alphabet without a branch; one `gf_mul`; `fresh_set` sized; with X10 | §4.4, §9.7 | About half a day, and X10's |
| A5 | Word lookups, `set11`, aezeed's `pack` and the sentence's gathering in constant time; with X11 | §4.5, §9.7 | About 1 day, and X11's |
| A6 | Secret comparisons through subtle; paperkey's CRC-24 masked | §4.6 | About 2 hours |
| A7 | BIP-352's label tweak times G through a secret key, `t_k` as libsecp256k1's own module computes it; the key's own randomized contexts; with X9 | §4.7, §9.7 | About half a day, and X9's |
| B1 | AUDIT §1's rows, the QR encoder's module doc, the storage command's comment, `docs/deps/rsa.md`'s Marvin sentence, the Secret handling lines that claim more than the code does (§7.8), KEYS §2 and the `hold` action saying what wiping does not reach | §7 | Built |
| C1 | BLAKE2b from `blake2`'s `Blake2bVarCore` | §5.1 | Built |
| C2 | (HKDF: not a switch; kept, §5.2, §6.7) | §5.2 | None |
| C3 | The sealing's counter nonce removed | §5.9 | Built |
| C4 | The RSA RNG adapter from `ChaCha20Rng`; tests re-pinned | §5.6 | Built |
| C5 | X.509 from `x509-cert`, DER from `der`; trust entries for x509-cert and der_derive, flagset's exemption; with X2 | §5.7, §9.7 | Built |
| C6 | The single-key and multisig finalizer from miniscript; with X3 | §5.8, §9.7 | Built, with X3 |
| C7 | (The BIP-39 core: not a switch; kept, §5.5, §6.9) | §5.5 | None |
| C8 | Base64 from `base64ct`, four copies | §5.10 | Built |
| C9 | Hex from `base16ct`, one module; with X4 | §5.11, §9.7 | Built, with X4 |
| C10 | The descriptor checksum from miniscript | §5.12 | Built |
| C11 | JSON from `serde_json` behind Faraday's visitor; `holds_secret` keeps no value; with X5 | §5.13, §9.7 | Built, with X5 |
| C12 | Adler-32 from `adler2`; with X6 | §5.14, §9.7 | Built, with X6 |
| C13 | The wipes through zeroize, one Argon2 call; with X7 (the zero check withdrawn: it was the FVLT container's, now gone) | §5.15, §9.7 | Built, with X7 in area `vault` over the KDBX vault and the export, and in area `qrvault` over the QR vault |
| C14 | The quiz's seed from the session's generator | §5.16 | Built |
| C15 | One copy of each kept piece; with X12 | §5.17, §9.7 | Built, with X12 |
| D1 | Schema 5: `written_here` as entries with `why`, `note`, `decision`; `[[not_established]]` for lzma-rust2 and rqrr; getrandom's trust entry; `check.py` (§1.2's three ways, the version checked) and `gen.py`; AUDIT §3 grouped; STANDARDS §10 generated and §0's bullet; the counts; `crypto.established` made active; the manifest's entries filled as §3 | §1.2, §1.4, §2, §3 | 1 to 1.5 days |
| E1 | The FAT32 formatter in faraday-fat, its `written_here` entry and the count raised under F180 | §3.2 | Built, with the Windows stick work |
| X1 | The cross-check's judges required in `just crosscheck` (§9.2, rule 4) | §9.7 | 1 to 1.5 days |
| X2 to X12 | The cross-check changes that land with C5, C6, C9, C11, C12, C13, A1, A7, A4, A5 and C15 | §9.7 | About 10.5 to 12.5 days in all, inside those commits |
| X13 to X30 | The cross-check changes standing alone (X25 withdrawn) | §9.7 | About 21.5 to 27.5 days in all |

The order (owner, 2026-10-31 and 2026-11-02; F191): A0, A1, B1, X1,
C1 to C14, then A4 to A7, then C15, then D1. A1 lands second because it
was written before this order was set. A0 comes first, because it changes
fingerprints every later batch re-stamps. C1 to C14 are independent of
each other and may land in any order; A4 to A7 come after them,
and C15 after A4 (it moves the masked `gf_mul`). Each changes the
manifest's `written_here` string of its action as it lands, and each
that names an X batch lands with it, in one commit and one passing
`just crosscheck` (§9.2, rule 6). X1 comes after B1 and before C1, so
that the judges the tied batches add are required from the first. The
X batches standing alone may land at any point after X1, in §9.7's
order. D1 comes after the switches so that the entries it writes are
§3's; before D1 lands, the website's reader of the manifest is changed
in its own repository. §6's items come
as their conditions hold. In all, A0 to D1 take about twelve to sixteen
working days, and the cross-check's
batches about 33.5 to 41.5 more, of which about 11.5 to 14 come with X1 and
the switches.

### 9.7 The batches

Each is one commit and one passing `just crosscheck`, and changes the
actions' `independence` in `docs/crypto/actions.toml`, the areas'
facts, `tools/crosscheck/README.md`'s tables (`facts.py`, `gen.py`),
`references.sha256` and the stamp, and, where it adds a tool,
`docs/DEVELOPING.md` §1. Where it retires a part of Faraday's second
version, the area's code for that part goes in the same commit and the
area's sabotage cases are kept against the outside judge. A batch with
a switch lands in that switch's commit (rule 6).

| Batch | With | What | Size |
|---|---|---|---|
| X1 | alone, after B1, before C1 | rule 4: `just crosscheck` checks every tool it needs first and names a missing one; the versions in the stamp; the judged test files run with `FARADAY_JUDGES_REQUIRED=1`; the OVMF run named in sb-keys', sb-sign's and sb-check's `other` and run | 1 to 1.5 days |
| X2 | C5 | `certtool` reads every certificate; `pesigcheck` verifies the real signed binaries (not the images Faraday signs, §5.7); real signed binaries read and their signer named; pefile's PE checksum and certificate-table entry | 1.5 to 2 days |
| X3 | C6 | btclib's finalizer beside Core in area `sign` | about 1 day |
| X4 | C9 | coins-hex's hex judged as outside (rule 5) | about an hour |
| X5 | C11 | CPython's `json` reads every wallet file the area writes and refuses a duplicate key and a depth over 32, beside the visitor | about 2 hours |
| X6 | C12 | every PNG the app writes read by Pillow and zlib | about 2 hours |
| X7 | C13 | PyCA's Argon2id beside argon2-cffi for the one Argon2 call: in area `vault` over the export's and the KDBX vault's, and in area `qrvault` over the QR vault's (built) | about 2 hours |
| X8 | A1 | btclib's MuSig2 in full and BIP-373's roles; Core's `musig()` descriptors; the Core round trip named as sharing the curve | 1.5 to 2 days |
| X9 | A7 | btclib's BIP-352, BIP-374 and BIP-375 in areas `sp` and `sps` | 1.5 to 2 days |
| X10 | A4 | rust-codex32 and volvelle's worksheet program; the threshold rule and the 48- and 74-character rows retired | about 2 days |
| X11 | A5 | bip39_slip39 vendored, in areas `bip39`, `slip39` and `bip85` | 1 to 1.5 days |
| X12 | C15 | `zlib.crc32`, cbor2 and PyCA's `HKDFExpand` against the one CRC-32, CBOR and HKDF-Expand. Built: `zlib.crc32` and cbor2 (area `ur`, and the Starter's GPT CRCs), and PyCA's `HKDFExpand` and `HKDF` (area `hash`) over the one HKDF | about 1 day |
| X13 | alone | rule 2 (built): vanity's child keys by btclib; sweep-sign and wif-read judged on btclib's curve and by python-ecdsa; Core named as sharing the curve | 1 to 1.5 days |
| X14 | alone | a released btcd vendored; `hdkeychain` for master, derive and xprv-shown; btcd's control-block check for tapmulti. Built: btcd v0.27.0 in `tools/reference/btcd/`; master's strict reader is kept (§9.4) | 1.5 to 2 days |
| X15 | alone | btclib in place of Faraday's second version for psbt-read, psbt-v2, sighash-review, tx-decode and tapmulti; Core's `decoderawtransaction`. Built: psbt-read and psbt-v2 (area `psbt`, with `psbt.v0` and `psbt.v2` in the self-test), sighash-review, tx-decode (area `tx`, with `tx.read` and `tx.sighash`) and tapmulti (area `taproot`, with `taproot.verify`) | Built |
| X16 | alone | btclib's change rule for psbt-review (built) | about 1 day |
| X17 | after X16 | a watch-only descriptor wallet on Core's regtest node (`core_node.py`, CROSSCHECK §10.4's "no wallet" changed), `ismine` and `ischange` (built: `watch_wallet`, made by the area that asks and deleted after; psbt-review asks it of every output btclib calls change or spoof, `ischange` being Core's "the wallet's, not in its address book", true of both chains, so the chain is the parent descriptor's and the position the key origin `walletprocesspsbt` writes) | 1.5 to 2 days |
| X18 | alone | btclib's descriptors and miniscript for address, descriptor and recovery | Built |
| X19 | alone | area `sign-check`. Built: partial and complete signatures of every script kind and sighash type, verified by python-ecdsa and btclib over btclib's sighashes, run by btclib's engine and judged by Core, and the nonces reused on purpose (§9.4) | Built |
| X20 | alone | area `compile`. Built: policies compiled by Faraday, taken by Core, python-bip380 and btclib, and compared with the miniscript as boolean formulas (§9.4) | Built |
| X21 | alone | segno, zbar and zxing-cpp; outside codes read by rqrr. Built: segno 1.6 as a second encoder in area `seedqr`, and zbar (`zbarimg --raw -Sbinary`) and zxing-cpp decoding every picture of it, the payloads compared as bytes (`tools/crosscheck/qr_decode.py`) | 2.5 to 3 days |
| X22 | alone | SeedSigner's dice steps. Built: `mnemonic_generation.py` over embit's BIP-39 in area `entropy` (see §9.4's dice row for what it covers) | Built |
| X23 | alone | krux's numbered parts; Coinkite's test files | Built |
| X24 | alone | `sq` in the GPG tests | Built |
| X25 | alone | (withdrawn: `keepassxc-cli` judges the vault the export is folded into, in area `vault`, F190 points 10 and 11, `docs/VAULT.md` §10) | None |
| X26 | alone | area `totp` with pyotp. Built: RFC 6238's vectors, random keys, times, digits, periods and hashes, and `parse_uri` over random and damaged URIs, with Faraday's stricter and wider readings stated and pinned (§9.4) | Built |
| X27 | alone | area `hash`; `hashlib` and `hmac` for HMAC-SHA256 and PBKDF2-HMAC-SHA256; `hashlib.scrypt`; the Base58Check assertion. Built: area `hash`, with HMAC, PBKDF2 (each over SHA-256 and SHA-512) and scrypt, and the assertion | built |
| X28 | alone | starter-write's decoders, tables, plist and IDs. Built: tests in the Starter's `outside` module, run with `FARADAY_JUDGES_REQUIRED=1` by `just crosscheck`: `xz` and 7-Zip over good and damaged streams, `sgdisk -v` beside `sfdisk --verify`, `plistlib` over the `diskutil` fixtures, `uuid` on the GUIDs on a stick | Built |
| X29 | alone | aezeed's software-round pass | Built |
| X30 | alone | more real wallet files | Built |

The batches standing alone change no app code but the one test X27
changes, and may land at any point after X1, in the order listed, which
puts what checks keys, addresses and signatures first. X1 takes 1 to
1.5 working days; X2 to X12 about 10.5 to 12.5, inside their switches'
commits; X13 to X30 about 21.5 to 27.5; in all about 33.5 to 41.5.

### 2.5 The inventory's name matching (batch A0)

The crypto inventory (`tools/costs-lint/src/inventory.rs`) decides which
functions an action's fingerprint covers. Built: a path starting with
`crate`, `super` or `self` is made absolute against the caller's module
and names the functions of that module only, or what a `pub use` there
re-exports, or nothing (`Self::f` is not moved by a `use` of the type in
another module of the file); a method whose name is in `STD_METHODS` is
an edge from a caller whose signature or body names its type, as well as
from the type's own impls. The fixture crate of `selftest.rs` holds a
check for each. `slip39` gained `Points::at`, `gf_mul` and `gf_inv`;
`codex32-make` and `codex32-correct` no longer carry SLIP-39's `gf_mul`
and `polymod`.
