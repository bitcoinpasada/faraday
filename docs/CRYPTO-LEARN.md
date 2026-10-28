# How Faraday does it: one manifest for every cryptographic action

**Kind:** Spec
**Status:** Specified (F148), not built
**Open:** the manifest and its entries, the inventory lint, fingerprints and the re-stamp recipe, the generated Learn blocks, STANDARDS and AUDIT §3 generated from the manifest, the bar's How Faraday does it action and its Learn tab and the Starter's sheet as §8 designs them (its icon, its strings, §8.5's drawing), the two Learn pages of §8.4 and §12 in the apps' page lists (their text is written), the reachability test, the cross-check stamp read from the manifest, the website's rendering

Every screen that does something cryptographic opens a block that says
how Faraday does it: the standard, the code, what checks it, how the
secret is handled, and where Faraday decided for itself. The block is
generated from one manifest in the repository. The same manifest
generates or checks every other place that describes a cryptographic
function: `docs/STANDARDS.md`, `docs/AUDIT.md` §1 and §3, the cross-check's
areas, the vectors, the fuzz targets, `docs/KEYS.md`'s reveals,
`docs/COMPUTE.md`'s costs, the Tools tiles' standard tags, the self-test's
checks and the website. `just lint` finds every cryptographic function in
the code by a mechanical rule, and fails when one has no entry, or when
its code changed since its entry was last reviewed. The owner's choices
are F148.

`docs/STANDARDS.md` already says it is the source for these blocks and
their manifest. Neither exists yet; this spec builds them.

## 1. The block

A block describes one action: what a person sees happen, not one
function. Its fields are fixed, in this order, each present on every
block:

| Field | What it holds | Rules |
|---|---|---|
| **Action** | One line: what is computed, from what. "Signs a transaction input with ECDSA, from the key the PSBT's origin names." | One sentence, no "we", no explanation. |
| **Standards** | Each standard by number and name, with its status: "BIP 174, Partially Signed Bitcoin Transaction Format, Deployed"; "RFC 6979, Deterministic Usage of DSA and ECDSA, Informational". | Names and statuses come from the manifest's standards table (§2.2), the one place a status is written. A convention or Faraday's own format says so: "Coldcard's seed-xor document, a convention"; "None: Faraday's own (F86)". |
| **Implementation** | Faraday's own crate and module in `core/` or `faraday/`, or the library by name and version ("secp256k1 0.29.1, through bitcoin 0.32.102"), and what Faraday adds over it. | A library's version is read from `Cargo.lock` when the block is generated, never typed. |
| **Checked against** | The vector folders under `tools/vectors/`, the tests by name, the self-test check, the fuzz target. | A missing check is written "not checked against vectors", "no fuzz target: <reason>"; never left out. |
| **Cross-check** | What the F79 cross-check does for this action (owner, 2026-10-28), one line each: the area; the Faraday code it drives; the second implementation it compares against and how that one is itself checked first; the published vectors it runs; what its random cases vary; each known difference, where Faraday refuses what the second implementation accepts; when it runs (whenever the crypto changes, and with a new seed and about ten times the cases before every release); and the last passing run for this release: its seed and its case count for the area. | Read from the area (§10.1), never typed in the manifest. An action with no area reads "not cross-checked: <reason>", and an area with no second implementation for a part says which part ("AEZ itself has no second implementation here"). |
| **Code** | Release · file · function, for the functions the entry names as its code; the repository's address as text, `github.com/bitcoinpasada/faraday`. | No line numbers. The release is the build's own release string (F5), filled when the block is drawn, so a version change does not regenerate the blocks. |
| **Secret handling** | Which `faraday-keys` reveal the action goes through (`Reveal::…`), or "none: no secret leaves `faraday-keys`"; constant time or not, and for which part; what is wiped, and when. | "Constant time" is stated only where the code or the library documents it; otherwise "not stated by the library". |
| **Limits** | What the action does not do, or refuses: lengths, script kinds, sighashes, cost limits. | Facts from the code and the entry's standard; one line each. |
| **Faraday's own calls** | Where Faraday is stricter than the standard or departs from it, each with its kind (Conservative, Neutral, Novel; STANDARDS §1) and the decision that records it, or "no decision". | A call with no decision is written as such; F148 does not make one. |

Facts only: plain statements, as every text in the product (CLAUDE.md).
The block's field labels are strings in `faraday/faraday-strings/src/en.rs`;
its values are the manifest's, carried by `faraday-learn` as the Learn
pages' text is.

## 2. The manifest

### 2.1 Form and place

One TOML file, docs/crypto/actions.toml (new), with a lock file beside
it, docs/crypto/actions.lock (new), that only the re-stamp recipe writes
(§5). TOML because three readers need it and none should parse Rust:
`tools/learn/sync.py`'s neighbours in Python (`tomllib`, in the standard
library since 3.11, as `tools/crosscheck/stamp.py` already uses), the
website's tools in Python, and people reading a diff. No crate is added:
the Rust side never reads the TOML (§4.3).

A Rust table in `faraday-learn` was considered and not chosen: the
website and the cross-check stamp would each need a Rust parser or a
second export, and an entry's prose in Rust string literals is what
`tools/learn/sync.py` exists to avoid.

### 2.2 Schema

```toml
schema = 1

# One table per standard, the only place its name and status are written.
[standard."BIP 174"]
name = "Partially Signed Bitcoin Transaction Format"
status = "Deployed"          # BIP 3's statuses, or "Informational", "Draft", "Convention", "Faraday's own"
checked = "2026-10-23"       # when the status was read
source = "bips master"       # or "vendored: tools/vectors/bip85"

# One table per STANDARDS row (§3.1): the row's own columns.
[[row]]
id = "psbt-v0"
section = 4                  # STANDARDS §2 to §5
what = "PSBT v0"
completeness = "Signs every wallet kind above; the finalizer is written here"

# One table per action.
[[action]]
id = "sign-ecdsa"
action = "Signs a transaction input with ECDSA, from the key the PSBT's origin names."
row = "ecdsa-nonces"
standards = ["BIP 143", "RFC 6979"]
learn = "nonces"             # a page of faraday_learn::pages::EN, or "faraday:<id>" for one of OWN
screens = ["Spend", "Family"]
functions = [                # the roots this entry covers (§4.4)
  "core/faraday-psbt/src/sign.rs::sign_ecdsa",
  "faraday/faraday-keys/src/psbt.rs::sign",
]
code = ["core/faraday-psbt/src/sign.rs::sign_ecdsa"]   # what the block's Code field shows

[action.implementation]
crate = "core/faraday-psbt"
libraries = ["secp256k1"]    # versions read from Cargo.lock
adds = "Low-R grinding by default; each signature verified before it is written; signed twice and compared."

[action.checked]
vectors = ["psbt"]           # folders under tools/vectors/
tests = ["core/faraday-psbt/tests/all/sign.rs::rfc6979_vectors"]
crosscheck = "sign"          # tools/crosscheck/area_sign.py; or not_crosschecked = "<reason>"
selftest = ["ecdsa_vector"]
fuzz_none = "Signing takes no outside input but the PSBT; psbt_parse fuzzes that."

[action.secrets]
reveal = "none"              # or ["Reveal::AddressKey"]
constant_time = "libsecp256k1's signing is constant time in the key and nonce."
wiped = "The derived private key is dropped at the end of the input's signing; SecretKey wipes on drop."

[action.limits]
lines = ["Only SIGHASH_ALL and DEFAULT are signed."]

[[action.own]]
text = "A key comes from the PSBT's origin only; a multisig key with no origin is refused."
kind = "Conservative"
decision = "F90"

[action.review]              # touchpoints named for review (§3.3)
places = ["core/faraday-psbt/README.md#What is refused", "docs/learn/13-nonces.md"]
fingerprint = "…"            # in [action.review]; written by the re-stamp recipe only (§5)
reviewed = "2026-11-02"
```

Several actions may share a row; every row has at least one action. An
action with no screen (`screens = []`) is built but offered nowhere in
the app (silent payments sending, MuSig2 across places, F109): it has a
row, a fingerprint and every check, and no Learn block (`learn = "none"`).

## 3. The touchpoints

Each place that describes a cryptographic function is one of three
kinds. The lint fails when a generated place is stale, when a checked
place disagrees, or when a named place no longer exists.

### 3.1 Generated

Written by docs/crypto/gen.py (new) from the manifest; `just lint` runs
it in `check` mode, which regenerates in memory and fails on any
difference, as `tools/learn/sync.py check` does for the Learn pages.

| Place | What is generated |
|---|---|
| The Learn blocks | faraday/faraday-learn/src/how.rs (new): a `Block` per action with its nine fields, and for each Learn page the blocks it carries, in the manifest's order. Marked generated at its top; never edited by hand. Each Learn page that names an action shows its blocks after its own sections under the heading **How Faraday does it**. |
| The Cross-check field | From the action's area (§10.1) and the stamp: the area's facts and the last passing run's seed and case count. |
| `tools/crosscheck/README.md`'s "What runs" table and "Known differences" | From the areas' facts (§10.1), between marker lines, so the README and the blocks say the same. |
| `docs/STANDARDS.md` §1's status table | From the standards table. |
| `docs/STANDARDS.md` §2 to §5 | Each table between two marker lines (`<!-- crypto-manifest: begin §3 -->` and its end), one row per `[[row]]`: What, Standard (status), Completeness, Verified by (the union of its actions' checks), Faraday's own call (the union of their own calls). §0, §8 and §9 stay hand-written. |
| `docs/STANDARDS.md` §6 | The list of Conservative own calls, with and without a decision. |
| `docs/STANDARDS.md` §7 | The Novel own calls, in the rank each entry's `novel_rank` gives. |
| `docs/AUDIT.md` §3 | The "written here" list: every action whose implementation names no library for its core computation, grouped by crate. |

The first generation of STANDARDS and AUDIT §3 must say what the
hand-written text says today, row for row; batch 4 (§15) diffs the two
and an Opus review settles each difference against the code.

### 3.2 Checked

| Place | Check |
|---|---|
| `docs/AUDIT.md` §1 | Every crate an action's implementation names has a row in §1, and every row's crate is named by an action. A row's "written here" claim agrees with its actions' libraries. |
| `docs/AUDIT.md` §2 and `docs/KEYS.md` | Every variant of `Reveal` in `faraday/faraday-keys/src/session.rs` is named by at least one action's `secrets.reveal`, and every named reveal exists. The count in words ("fourteen named reasons") in AUDIT §2, KEYS and CLAUDE.md is the variants' count. |
| `docs/COMPUTE.md` §2.3 | Every root that table names, for the kinds `WordsParse`, `Kdf`, `MasterKey`, `KeyDerive`, `DescriptorParse`, `Address`, `PsbtParse`, `PsbtInspect`, `Sign`, `QrEncode`, `QrDecode` and `FileHash`, is covered by an action (§4.4); an action function that takes `&Compute` counts a `Work` kind the table lists. |
| `docs/CROSSCHECK.md` and `tools/crosscheck/` | An action's `crosscheck` names an area whose file exists (`tools/crosscheck/area_<name>.py`), or `not_crosschecked` gives a reason. Every area file is named by at least one action. |
| `tools/vectors/` | Every folder an action names exists; every folder under `tools/vectors/` is named by an action, or by the manifest's list of vector folders for no action, each with its reason (the Diceware lists' digests are an action's; the word lists are not). |
| Fuzz targets | A named target is a `[[bin]]` in `fuzz/Cargo.toml` or `faraday/fuzz/Cargo.toml`; `fuzz_none` gives a reason. |
| The self-test | Every check function in `core/faraday-selftest/src/lib.rs` (`bip39_vector`, `ecdsa_vector` and the rest) is named by an action's `selftest`, and every name given exists. |
| The Tools tiles | Each tag on a tile in `faraday/faraday-core/src/catalog.rs` is a standard of an action whose screens include the tile's screen. The BIP 370 tag STANDARDS §8 lists fails this until it is removed or backed. |
| The Learn pages a screen opens | Each action's `learn` page is among the pages `Faraday::screen_pages` gives each of its screens (`faraday/faraday-core/src/learn.rs`): checked by the reachability test (§9), since the list depends on state. |
| The screens list | §9.2: the screens whose code calls an action's functions are its `screens`. |

### 3.3 Named for review

Places whose prose depends on an action but cannot be checked
mechanically. An action names them in `review.places` as a path and,
where it is a section, its heading; the lint checks the path and the
heading exist. A changed fingerprint lists them with the rest (§5.3).

Found in the tree (§13 lists every one): `docs/SECURITY-MODEL.md`
("Where keys live in the program", "Vault cryptography", "Why this
construction holds up"), `docs/INTEROP.md`, `docs/VAULT.md`, `docs/KDBX.md`,
`docs/CODEX32.md`, `docs/TOTP.md`, `docs/QR.md`, `docs/OWN-RANDOMNESS.md`,
the crate READMEs (`core/faraday-bip/README.md`, `core/faraday-psbt/README.md`
and the others), the Learn pages' own prose (`docs/learn/13-nonces.md`
says how every nonce is derived), Faraday's own pages under
`docs/learn/faraday/`, the guide's screen pages (`docs/guide/screens/README.md`),
`tools/reference/` and each vector folder's README, and the strings in
`faraday/faraday-strings/src/en.rs` that state a construction (the
Settings explainers on how added randomness is mixed).

Release notes are written at release from what changed:
just crypto-changes (new) lists the actions whose fingerprint changed
since a tag, for the notes and for the deep cross-check before a
release (F79).

## 4. Finding the cryptographic functions

### 4.1 Where the lint looks

The inventory is a second binary in `tools/costs-lint`, crypto-inventory
(new), sharing its source walker: the crate roots under `core/` and
`faraday/`, the modules each root reaches, and what is compiled for
tests left out (`#[cfg(test)]`, the test kit's cfg). The costs lint and
the inventory must agree on what is shipped code; one walker makes that
true by construction. A rule inside `costs-lint` itself was considered
and not chosen: its job is the token's discipline, its findings are
`path:line` errors, and the inventory's output is data the manifest's
checker reads. Both binaries are one package, one `syn`, no new crate.

A **cryptographic crate** is one whose normal dependencies include a
primitive crate: `faraday-crypto`, `bitcoin`, `secp256k1`, `sha1`,
`sha2`, `hmac`, `pbkdf2`, `argon2`, `chacha20`, `chacha20poly1305`,
`ed25519-dalek`, `rsa`, `getrandom`. The list is the inventory's
`PRIMITIVE_CRATES`. Each page under `docs/deps/` for a crate in it
carries a line `**Cryptography:** computes`; the lint fails when a
dependency's page says so and the list lacks it, so a new primitive
crate cannot enter without the inventory seeing it (F71 already requires
the page).

### 4.2 The rules

In every cryptographic crate, outside tests, a function is
**cryptographic** when:

1. **Token** (core and the `faraday/` crates that hold keys:
   `faraday-keys`, `faraday-vault`, `faraday-pgp`, `faraday-sb`,
   `faraday-payload`, `faraday-qr`): it takes `&Compute`. Not in
   `faraday-core` or the Starter, where 600 functions pass the token
   through to glue; there rule 3 finds what computes.
2. **Reveal** (`faraday-keys`): it takes a `Reveal` or returns
   `Revealed` or `Held`.
3. **Primitive** (every cryptographic crate, `faraday-core` and the
   Starter included): its signature or body names a path into a
   primitive crate, after the file's `use` lines are applied
   (`use sha2::{Digest, Sha256}` makes `Sha256::new` a primitive path).
   This finds `faraday-core`'s file hashes, the Starter's image hash in
   `faraday/faraday-payload/src/decode.rs`, and every direct call. A
   path into `bitcoin` or `secp256k1` counts whatever it names, a type
   or a script builder as much as a signature check; closures and
   `[[not_crypto]]` absorb what is not cryptographic. A path into
   `faraday-crypto` counts unless its first name is one of the inventory's
   `NOT_COMPUTING`: `work` (the compute token), `Secret`, `Zeroize`,
   `ZeroizeOnDrop`, `Zeroizing`, `WipeOnFree`, `Pinned` and
   `pin_failure_to_report`, which compute nothing. Without that, every
   function that names `Compute` would meet rule 3.
4. **Public reach** (the crates of rule 1): it is `pub` and, through
   calls in its own crate, reaches a function of rules 1 to 3. A new
   public function that wraps a hash is found even when it takes no
   token. `pub` is taken literally: a method of a trait impl is not
   public here, and a function that already meets rule 1, 2 or 3 is not
   also listed under rule 4.

A function defined inside a function body, or inside a `macro_rules!`
body, is not a function of its own to the inventory (§17).

### 4.3 What the inventory prints

One JSON line per function: its file and qualified name
(the file, two colons and the function, as the manifest's `functions`
write them; a method as `Type::name`), the rules it meets, its body hash (§5.1), the functions
of its crate it calls, and the crate it is in. One more line per `#[test]`
function: its file, name, whether it is `#[ignore]`d, and the cfg it
sits under. The Python checker, docs/crypto/check.py (new), reads these
and the manifest.

A function that meets no rule is printed too, with `"rules":[]`, when a
cryptographic function of its crate calls it, directly or through others:
the closure of §4.4 passes through such helpers, and their body hashes
and callees are needed to fingerprint it. `--all` prints every function
of every cryptographic crate. The summary by rule and crate goes to
standard error, and the exit status is 1 when a source file did not
parse, since the inventory is then incomplete.

### 4.4 Coverage

An action covers its `functions` (the roots) and their **closure**: the
functions of the same crate they call, transitively, stopping at a
function that is another action's root. A call is resolved by name
within the crate: a path call by its last two segments, a method call by
its name, every function of that name in the crate taken. That
over-approximates: a helper with a common name may fall into a closure it
is not in, which only makes the fingerprint change more often; nothing
is missed. A name passed as a value without a call (`.map(helper)`) is
taken as a call; a local variable of the same name as a function is not.
A call through a `use`d name is resolved through the file's `use` lines
only: a glob import is not applied.

Every cryptographic function (§4.2) must be covered by exactly one
action, or listed as not cryptographic (§7). A function covered by none
fails: "core/faraday-bip/src/bip39.rs::Mnemonic::to_seed_v2 is
cryptographic (rule 3: sha2) and no action covers it: add it to an
action's functions, or to [[not_crypto]] with its reason". Covered by two
roots' closures is allowed; named as a root by two actions fails.

An action's root in another crate (`faraday-keys` over `faraday-psbt`)
is listed in the same action. A call across crates to another action's
root, in the same crate or another, is that action's `uses`, worked out
by the inventory and printed; it is not hand-written. A change to an
action fails every action that uses it, directly or through others, as
well as its own (owner, 2026-10-28): each is reviewed, since what it
computes may have changed (§5.2).

## 5. Fingerprints and the re-stamp

### 5.1 What is hashed

A function's **body hash** is SHA-256 over its tokens as `syn` reads
them, printed back with one space between tokens: its signature without
its name, its generic bounds, its body, and the attributes other than
`#[doc]`. A `,` that ends a group is left out. So:

| Change | Body hash |
|---|---|
| `cargo fmt`, a line wrapped, blank lines | Same: tokens only. |
| A trailing comma added or dropped (rustfmt does either) | Same: a `,` directly before a closing `)`, `]` or `}` is left out before hashing. |
| A `//` comment, a doc comment | Same: comments are not tokens; `#[doc]` is left out. |
| The function moved within its file, or to another file | Same; the entry's path no longer resolves, so the lint fails as untrue (§6), and the recipe re-points it with no review: "moved, body unchanged". |
| The function renamed | Same, with the same failure and the same re-pointing. |
| A local variable renamed | Changes. Reviewed (owner, 2026-10-28): a false positive the design accepts, since telling a rename from a change needs name resolution `syn` does not do. |
| A callee in the same crate changed | The action's fingerprint changes (§5.2). |
| Another action it uses changed, in any crate | That action's fingerprint changes, and so does this one's, through its `uses` line (§5.2). |
| A library's version | The action's fingerprint changes (§5.2). |
| A vector file | The action's fingerprint changes (§5.2). |

### 5.2 An action's fingerprint

The lock file holds, per action, one line per part: each function of
its coverage with its body hash, each library with its `Cargo.lock`
version and checksum, each vector folder with the SHA-256 of its files,
its cross-check area's file with its SHA-256 (a change to what the
cross-check does is reviewed against the block as a change to the code
is), and each action of its `uses` with that action's fingerprint. A change
therefore reaches every action above it: all of them fail together, and
each is re-stamped after its own review, the used action first.
A function's line carries its path (`fn <hash> <path>`) so the recipe
can re-point a move; the action's `fingerprint` in the manifest is
SHA-256 over those lines without the paths, sorted, so a move or a
rename keeps it (§5.1). The lint recomputes it from the inventory and the tree; a
difference fails, naming the parts that changed:

```
sign-ecdsa: the code changed since it was reviewed on 2026-11-02
  changed: core/faraday-psbt/src/sign.rs::grind_low_r
  run: just crypto-review sign-ecdsa
```

### 5.3 The re-stamp recipe

just crypto-review (new) `<id>` is the only writer of a fingerprint:

1. It prints the action's entry, the parts that changed with their diff
   since the commit that last wrote the fingerprint, and every
   touchpoint of the action together: its Learn block and page, its
   STANDARDS row, its AUDIT §1 crate row and §3 line, its reveal, its
   COMPUTE kind, its cross-check area, vectors, fuzz target, self-test
   check, Tools tags, the places named for review, and the actions that
   use it, each of which fails until it is reviewed in turn.
2. It regenerates the generated places (§3.1) from the manifest as it
   now stands.
3. It refuses, and writes nothing, while a checked place disagrees,
   while a named test, area, vector folder or fuzz target is missing,
   and, for an action with a cross-check area, while
   `tools/crosscheck/stamp.py check` fails: an entry is never re-stamped
   on code the cross-check has not seen.
4. Only then it writes the action's lock lines, its `fingerprint` and
   `reviewed`.

The diff of a re-stamp shows the fingerprint and the review date
changing beside whatever text the review changed. Re-stamping is a
command, never an edit: the lint fails a fingerprint that is not the
hash of the action's lock lines, and lock lines that are not the tree's.

## 6. Entries are true

The lint fails when:

- a file or function an action names (`functions`, `code`) does not
  exist in the inventory;
- a test in `checked.tests` is not a `#[test]` function of that name in
  that file, is `#[ignore]`d, or sits under a cfg that `just test`
  (`cargo nextest run --locked --workspace`) does not build;
- a vector folder, cross-check area, fuzz target, self-test check,
  reveal, Learn page or screen it names does not exist;
- a standard it names has no table, or a row it names has no `[[row]]`;
- a field of §1 is missing or empty;
- `decision` names a file that is not under `docs/decisions/`.

## 7. Ways past

There is none silent. Each is an entry in the manifest, with its reason,
in the diff:

- **Not cryptographic.** `[[not_crypto]]`: a function the rules find
  that computes nothing on a secret or signed data (a QR code's mask
  penalty over public bytes, the Starter's blank SeedQR grid), with
  `function` and `reason`. Their count is in tools/lint-crypto.counts
  (new) and only falls, as the layout and token ratchets do (F74);
  raising it is the owner's decision, recorded in DECISIONS.
- **Unchecked.** An action with no vectors, cross-check or fuzz target
  writes so in its block (§1). That is a fact shown to the person, not a
  way past; STANDARDS §0 still requires the check before 1.0.
- **Hold.** A check of this spec that keeps keys or money safe has no
  way past (F136). A hold here would be an entry
  `[[hold]] action, check, case, decision`, honoured by the lint only
  when its decision exists and names the case. None is foreseen; the
  first needs the owner's decision.

## 8. Reaching the block

Checked against Material 3's pages (m3.material.io): Top app bar
(Guidelines: trailing action icons, the overflow menu for what does
not fit), Icon buttons (standard,
a 40 dp container in a 48 dp target, a plain tooltip naming the action),
Tabs (primary tabs, scrollable), Lists (Specs: list item, headline
Body L in on-surface, supporting text Body M in on-surface-variant, 16 dp
side padding; Guidelines: dividers between groups), Dividers
(full-width), Bottom sheets and Side sheets, Menus, and Material
Symbols (`fact_check`). The script-drawn pages were read as Android's
Compose documentation states them (`ListItem`, `TopAppBar`,
`PrimaryScrollableTabRow`), as DESIGN §4.12's tooltip row was.

### 8.1 The bar's action

On every page and step flow whose screen offers an action of the
manifest, the top app bar carries **How Faraday does it**: Material's
standard icon button, in a 48 dp target, immediately left of the ?.
Its icon is new in `faraday-draw`'s set, `Icon::Method`: Material
Symbols' `fact_check` (a sheet with two check marks at its left and two
lines beside them), in its filled form, so that it sits beside the set's
solid glyphs (Font Awesome 6 Free Solid, the SeedSigner icons) at the
same weight. Its outline is taken from Material Symbols' published SVG
(Apache License 2.0), kept in `tools/fonts/` with its licence beside the
other faces, and baked by `tools/fontbake` into the icon face at a
code point of its own, so it is filled at every size as every `Icon`
is; it is not a `Glyph` drawn in code. Its tooltip is plain: "How
Faraday does it". It has no selected state, as the ? has none. Where a
flow offers an action on one step only, the button shows on that step
and the steps after it that the action's result is on (a signed PSBT's
step), not before.

The bar's tools then read, from the start of the trailing group: Cancel,
the eye (Make codex32 shares' sheets), the keyboard button, Load samples
(Practice), the mode pill, **How Faraday does it**, the ?, then Lock and
the ⋮ where the layout has them (DESIGN §4.1).

### 8.2 What it opens

It opens Learn as the ? does, below extra-large the Learn sheet, on an
extra-large window the supporting pane (LAYOUT §6), on a tab **How
Faraday does it** placed first among the sheet's tabs. The tab is there
exactly while the bar offers the button: on a step before the action's,
Learn has no How tab. The ? opens Learn as today, on the screen's first
page, with the How tab still first among the tabs; on an extra-large
window the ? pressed while the pane is shown puts it away, whichever
tab it shows, as today. **How Faraday does it** pressed while the pane
shows the How tab puts it away; pressed while the pane shows another
tab, it moves the pane to the How tab. Escape, the pane's close button
and the mouse's back button put the pane away as they do now. When the
step changes, the How tab follows the screen's actions as the pane
follows its pages (LAYOUT §6).

The tab holds the blocks of the screen's actions, in the order the
screen offers them, the open step's first (§8.5). Learn's own page for
the action carries the same blocks after its sections (§3.1).

### 8.3 Every size class

| Class | Where the action is |
|---|---|
| Compact, the Pi's panel (268 dp) | When the bar's tools would reach the chevron's target (LAYOUT §8), **How Faraday does it** goes into the ⋮ overflow menu first, before the ?: its row reads "How Faraday does it", with `Icon::Method` at its start, and it is the menu's first row, above **Learn**, **Show keyboard** and **Show sheets**. On the panel the ⋮ menu is a sheet, as every menu there. |
| Compact, other windows | On the bar while it fits, else in the ⋮ as on the Pi, by the same order. |
| Medium, expanded, large | On the bar. |
| Extra-large | On the bar; it opens the supporting pane, and pressed again while the pane shows the How tab puts the pane away (§8.2). |

Every target stays 48 dp, and no two overlap (owner, 2026-10-26).

### 8.4 The Starter

The Starter's computing actions: writing an image and checking its
SHA-256 (`faraday/faraday-payload/src/decode.rs`), checking a boot file
the person signed (`faraday/shells/starter/src/signed.rs`, F133), and
once IMAGES is built, checking the release's image list. Its top app
bar carries the same action, the same icon and tooltip, left of the ⋮,
on the pages inside a task that perform them:

| Page (STARTER §3.2 to §3.4) | Blocks, in order |
|---|---|
| What to write | starter-write, then starter-signed while *One I signed* is offered |
| Confirm, Writing, Done | starter-write, then starter-signed when a signed boot file is chosen |
| Start this computer from the stick, while **Include the BitLocker recovery key** is offered | starter-recovery-key |

On a compact window it goes into the ⋮ as its first row, as in Faraday.
It opens a sheet in the Starter's look (`faraday-ui`), drawn as the
Starter's other sheets (`faraday_ui::sheet::scroll_sheet`, DESIGN §5):
the window's width less its margin on a compact window, centred at
most 560 dp wide otherwise (LAYOUT §5), over the page's frost and
scrim. Its head is `Icon::Method` and **How Faraday does it**
(`faraday_ui::sheet::sheet_head`); under it the blocks, drawn as §8.5
draws them, scrolling within the sheet; at its foot **Close**, a text
button. A press beside it, Escape and the back arrow close it. The
Starter has no supporting pane, so the sheet is the same at every
size. The Starter's Learn destination gets a page for them, Checking
what is written (`docs/learn/faraday/checking-written.md`, id
`checking-written`), listed after **Replacing Faraday on a stick**,
built from the same blocks after its sections.

The Starter's templates draw a blank SeedQR grid and a blank codex32
worksheet: no secret, no computation; they are `[[not_crypto]]` with
that reason. Its notes PDF carries a BitLocker recovery key as text
(F49): a secret carried, not computed, and still given a block (owner,
2026-10-28), action starter-recovery-key (§12), on Checking what is
written: where the key is read, that it is held only until the sheet is
made and wiped when dropped, and what the Starter checks of it (the
groups' shape only).

### 8.5 The block's drawing

One drawing at every size: a block is a Material list, the same list
on the 480x640 window, at 1280x800, in the extra-large supporting pane
and on the Pi's panel; only its width changes.

- **The action's name** heads the block in Title S, in on-surface, with
  16 dp of side padding (`LIST_PAD`), as a section's label is
  (`parts::section_label`). Blocks follow one another down the tab;
  between two blocks Material's full-width divider in the
  outline-variant role, with `GAP` above and below it. A screen with
  several actions (Spend offers nine) shows every block whole, the open step's first and
  the rest in the order the screen offers them; nothing is folded, so
  every field of every block is reached by scrolling the tab alone.
- **Each field** of §1 is one list item, in §1's order: its headline the
  field's label in Body L, on-surface; its supporting text the value in
  Body M, on-surface-variant (Material's list item, `Ui::list_item`,
  `LIST_2` for a one-line value, `LIST_3` for two). The Action field's
  value is one sentence and is drawn as any other.
- **A field with several entries** (Standards, Checked against, Code,
  Limits, Faraday's own calls, Cross-check) writes each entry as its own
  paragraph of the supporting text, `GAP_SMALL` (4 dp) between
  entries, each wrapped in the room the item leaves; an entry is never
  run into the next on one line.
- **Cross-check**, the longest field, is one list item like the rest:
  headline "Cross-check", then its parts in §1's order, each a paragraph
  led by its part's name in the medium face of Body M (Area, Faraday's
  code, Compared with, Vectors, Random cases, Differences, When it runs,
  Last passing run) and its value after it in the regular face. An area
  with no second implementation for a part says so in that part's
  paragraph; an action with no area has the one paragraph "not
  cross-checked: <reason>". At 268 dp the field runs to about thirty
  lines and the tab scrolls; it is not shortened.
- **Values read character by character** (a file path, a function, a
  library's version, a fingerprint, a seed) are Body M `.mono()`, as
  DESIGN §3 sets data. A path that does not fit wraps after a `/`, a
  function after `::`, and otherwise at the character that reaches the
  edge; the release string in the Code field is filled when the block is
  drawn (§1).
- **What is not checked** ("not checked against vectors", "no fuzz
  target: …", "not cross-checked: …", "no decision", "not stated by the
  library") is drawn in the Warning role (THEMES §5) with
  `Icon::Warning` at `ICON_GLYPH_SMALL` before it, so the state is not
  told by colour alone (DESIGN §3).
- **Height.** An item is as tall as its text: `LIST_1` to `LIST_3`
  while the value takes up to two lines, and beyond `LIST_3` by the
  lines it takes after that, its headline at `LIST_3_TOP` from its top.
  Material's list item stops at three lines and truncates the rest;
  Faraday cuts no fact (LAYOUT §8, "Nothing is cut off"), so this
  differs from Material, as F148 records (owner, 2026-10-28).
- **No presses.** A block is read, not acted on: its items take no
  press, have no state layer and no trailing icon. The tab scrolls as
  the Learn sheet scrolls, by wheel, drag and the arrow keys.

At 480x640 (compact) the sheet is the window's width less 16 dp
margins; at 1280x800 (expanded) the sheet's column, its text held to
the reading measure (about 70 characters, LAYOUT §8); at extra-large the pane's 400 dp less
its padding; on the Pi's panel 268 dp less 16 dp margins, where
Body L's labels stay on one line ("Faraday's own calls" is the longest)
and every value wraps.

### 8.6 The strings

Field labels and the action's name, for `faraday/faraday-strings/src/en.rs`
(each a key of its own, as the bar's ? and its ⋮ row are, `tip_learn`
and `bar_menu_learn`). They are added with the code that draws them:
the strings lint fails a key no code names.

| Key | Value | Where |
|---|---|---|
| `tip_how` | How Faraday does it | The bar's button's tooltip, Faraday and the Starter |
| `bar_menu_how` | How Faraday does it | The ⋮ row on a compact bar, Faraday and the Starter |
| `learn_tab_how` | How Faraday does it | The Learn sheet's and pane's first tab |
| `starter_how_title` | How Faraday does it | The Starter's sheet's head |
| `starter_how_close` | Close | The Starter's sheet's foot |
| `how_field_action` | Action | §1 |
| `how_field_standards` | Standards | §1 |
| `how_field_implementation` | Implementation | §1 |
| `how_field_checked` | Checked against | §1 |
| `how_field_crosscheck` | Cross-check | §1 |
| `how_field_code` | Code | §1 |
| `how_field_secret` | Secret handling | §1 |
| `how_field_limits` | Limits | §1 |
| `how_field_own_calls` | Faraday's own calls | §1 |
| `how_cc_area` | Area | Cross-check's parts (§8.5) |
| `how_cc_code` | Faraday's code | Cross-check |
| `how_cc_second` | Compared with | Cross-check |
| `how_cc_vectors` | Vectors | Cross-check |
| `how_cc_random` | Random cases | Cross-check |
| `how_cc_differences` | Differences | Cross-check |
| `how_cc_when` | When it runs | Cross-check |
| `how_cc_last_run` | Last passing run | Cross-check |
| `how_cc_seed_cases` | Seed {} · {} cases | Last passing run's value; `{}`: the seed, then the area's case count |
| `how_code_entry` | {} · {} · {} | One Code entry; `{}`: the release, the file, the function |
| `how_repository` | github.com/bitcoinpasada/faraday | The Code field's last entry |

The values are the manifest's and the cross-check's, carried by
`faraday-learn` (§1), not strings of this file.

### 8.7 The guide's pages

The bar's new button changes the guide's pictures of the screens that
offer an action, and the guide's text that names the bar's tools. Batch
7 draws them again (`just guide-pictures`) and says so in each page's
"On this screen" table, after the ?:

- Faraday's screens: `docs/guide/screens/add-a-key.md` (Entry),
  `restore.md`, `files.md`, `spend-tab.md` (Family), `new-key.md`
  (KeyGen), `vault.md` (VaultContents), `make-a-wallet.md` (Create),
  `explore.md`, `wallets.md` (Wallets, WalletsStart), `calculators.md`
  (Tools), `settings.md` (Settings, Randomness), `back-up-wallet.md`
  (Backup), `backup-flow.md`, `backups.md`, `create-vault.md`,
  `vaults.md` (Unlock), `codes.md`, `sign.md` (Spend), `child-seeds.md`
  (Bip85), `vanity.md`, `lightning.md`, `silent-payments.md`,
  `messages.md` (Message, CheckMessage), `decode.md`, `qr-and-scan.md`
  (the QR sheet, Transfer) and `stick-visit.md` (Visit), all in
  `docs/guide/screens/`.
- `docs/guide/screens/learn.md`: the How tab, first among the tabs.
- `docs/guide/screens/home.md` and `docs/guide/screens/prompts.md`:
  the ⋮ menu's first row on the Pi's panel, where they show it.
- The Starter: `docs/guide/screens/starter-make.md` (What to write,
  Confirm, Writing, Done), `starter-learn-and-boot.md` (the new Learn
  page and the Start page's BitLocker tick box) and `starter-menu.md`
  (the ⋮'s first row on a compact window).
- The walkthroughs whose pictures show those bars, found by the
  pictures `just guide-pictures` draws again.

## 9. Tests and checks of reach

### 9.1 The reachability test

A test in `faraday-core` (and one in the Starter) over the manifest's
actions: for each action and each of its screens, the app is put on that
screen, on the step that offers the action, at three sizes (480 × 640,
1280 × 800 and the Pi's panel); the bar's How Faraday does it, or its
row in the ⋮ menu on the Pi, is pressed; the Learn sheet or pane is open
on the How tab, and the action's name is among the blocks it shows. It
also opens the ? there and checks that the action's `learn` page is
among the tabs. What a person on that screen would notice: the block is
one press, or two on the Pi, away. The screens' states come from the
test kit's tour states, as the snapshot tours use them.

### 9.2 Which screens offer an action

The inventory also lists every call site in `faraday-core` of an
action's roots in `faraday-keys` and `core/` (app code reaches
cryptography only there, KEYS §2), with the module it is in. The code
map in `faraday/faraday-core/src/lib.rs` gives each module's screen; the
checker fails when a screen whose code calls an action's root is not in
that action's `screens`. A path call is resolved through the file's `use`
lines. A method call is taken by its name, for a method of a type the
calling file names in a `use` line or a path; without types, the name alone
would take every `push` and `get` of a root of that name. A method
called on a field whose type the file never names is therefore not listed.
Each line says `path` or `method`. A module that serves several screens (an area's
`mod.rs`) gives all of its area's screens; an action may narrow them
with `screens_not = [{ screen, reason }]`, each counted with the
not-crypto ratchet.

## 10. The cross-check and its stamp

### 10.1 What each area says about itself

Each area file states its facts once, as class attributes beside `name`,
which the generator reads (§3.1) and the lint checks are present and
non-empty:

- `faraday`: the Faraday code it drives, as paths the lint resolves;
- `reference`: the second implementation, with its pinned folder under
  `tools/reference/` or its distribution package, and `reference_checked`:
  how that one is checked before any case (its `check_driver`);
- `vectors`: the published vector folders it runs, or "none published"
  with what there is;
- `corpus`: what its random cases vary, one line per kind;
- `differences`: each known difference, with what the area does about
  it (checked, counted, kept out of the corpus);
- `not_compared`: a part of the action with no second implementation,
  if any.

The README's "What runs" table and "Known differences" are generated
from these (§3.1), so the area, the README and the block cannot
disagree; today's text is moved into the attributes in batch 8, word for
word, and differences settled against the area's code. A passing run
writes, per area, its case count beside the seed in the stamp; the
block's last line reads it.

### 10.2 The stamp and the fingerprints

`tools/crosscheck/stamp.py` records that the cross-check passed on this
code, for all of it at once: any change to a crypto crate forces
`just crosscheck` again. A fingerprint records that one action's text
was reviewed against its code. They fit together:

- The re-stamp of an action with an area requires the stamp current
  (§5.3), so text is reviewed only over code the cross-check passed.
- The stamp's `FILES` and `VECTORS` come from the manifest: the files
  holding any covered function outside the stamp's whole crates (today
  `fresh.rs`, the rollers and `picture.rs`, written by hand), and every
  vector folder an action with an area names. Its `CRATES`, hashed
  whole, stay as they are: coarser than the manifest, so a change
  outside every closure in a crypto crate still forces a rerun.
- When the stamp fails, `stamp.py check` prints the actions whose
  fingerprints changed, so the rerun's reviewer knows which areas and
  which texts to look at.

## 11. The website's interface

The website (its own private repository) builds from a pinned Faraday
commit and already reads the guide from the pin. It reads:

- docs/crypto/actions.toml at the pin, `schema = 1`, the schema of §2.2;
  a schema it does not know stops its build;
- `docs/STANDARDS.md` at the pin, already generated;
- the SHA-256 of actions.toml's bytes at the pin, written into each
  rendered page that shows a block or the standards.

The standards page lists the actions that are in the repository and not
in the app (an entry with no screens) under their own heading, "In the
repository, not in the app", with their blocks (owner, 2026-10-28).

To be done there, later: render the blocks on the Learn pages and the
standards page from the pinned manifest; record the manifest's hash in
the rendered output; and make its publish step (its
tools/publish-pages.sh) refuse when a page's recorded hash is not the
pin's manifest's.

## 12. The actions found in the code

From the code on 2026-10-28: the 675 functions under `core/` and
`faraday/` that take `&Compute` (about 300 in cryptographic crates,
the rest `faraday-core`'s and the Starter's glue), the 33 functions that
take a `Reveal`, the 14 `Reveal` variants, and the files that call a
primitive crate directly, seeded from STANDARDS §2 to §5. 77 actions.
The functions are the roots; batch 3 checks each against the code and
adds what the inventory finds uncovered. Screens are `Screen` variants;
"QR" is the QR sheet and the scanner, on every screen that shows or
reads a code. Learn pages are `pages::EN`'s fields, or Faraday's own by
id (`docs/learn/faraday/<id>.md`); `secure-boot-keys` and
`checking-written` are new, their text written, not yet in the apps'
page lists.

### Keys

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| words-read | Read seed words and check their checksum (BIP 39) | `faraday_bip::bip39::Mnemonic::parse`; `faraday_keys::words::parse`, `from_indices`; `faraday_intake::seed_in_code` | Entry, Restore, Files, Family | words |
| words-make | Turn entropy into words (BIP 39) | `faraday_keys::words::from_entropy` and the bip39 function it calls | KeyGen | words |
| seed | Make the seed from words and passphrase, PBKDF2-HMAC-SHA512 (BIP 39) | `Mnemonic::to_seed`, `to_seed_unchecked`; `Keys::add_words`, `add_words_with`, `add_mnemonic`, `add_with_passphrase`, `add_with_suffix` | Entry, KeyGen, Restore, VaultContents, Family | passphrases |
| master | Make the master key and its fingerprint (BIP 32) | `MasterKey::from_seed`, `from_seed_bytes`, `from_xpriv`, `decode`, `from_chain_code_and_key`; `Keys::add_seed`; `identity::master_of`, `fingerprint_of_words`, `fingerprint_of_mnemonic`, `fingerprint_of_seed`, `master_id` | Entry, KeyGen, Restore, Files, VaultContents | xpubs |
| derive | Derive a key and its extended public key (BIP 32, 44, 48, 49, 84, 86) | `MasterKey::derive`, `xpub`, `account_xpub`, `multisig_account_xpub`, `with_multisig_account_secret`; `DerivedKey::to_xpub`; `Keys::xpub_at`; `texts::key_text`, `single_account`, `multisig_account`, `account_at` | Create, Explore, Wallets, Tools | xpubs |
| hold | Hold a key in memory: blinded, pinned, sealed | `MasterKey::reblind`, `blinded`; `Keys::blind_key`, `reblind`; `faraday_crypto`'s `Sealed`, `SessionKey`, `Pinned`, `Secret` | Settings | faraday:keys-kept |
| seed-shown | Show a key's master seed, and its codex32 secret (`Reveal::SeedShown`) | `Keys::seed_shown` | Explore | faraday:keys-kept (added to Explore's pages) |
| xprv-shown | Show an extended private key (`Reveal::ExtendedPrivateKey`) | `Keys::xprv_at` | Explore | xpubs |
| wif-shown | Show an address's private key as WIF (`Reveal::AddressKey`) | `Keys::wif_at` | Explore | xpubs |
| words-shown | Show words and passphrase, and to copy by hand (`Reveal::Words`, `Passphrase`, `CopyByHand`) | `Keys::reveal`; `View::reveal` | Explore, Backup, BackupFlow, VaultContents | backups |

### Wallets

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| address | Derive an address and its script (BIP 44, 49, 84, 86, 341) | `WalletPolicy::script_at`, `script_at_with`, `address_at`, `find_address`; `Wallet::address`, `address_shown`, `script_at` | Wallets, Explore, Spend, Family, Tools | verifying |
| descriptor | Read a wallet description and its checksum (BIP 380 to 390, SLIP-132; F132's unspendable key) | `WalletPolicy::parse`, `from_parts`, `from_descriptor`, `parse_any`, `from_record`, `parse_script_descriptor`; `descriptor::checksum_facts`, `reads_as_descriptor`; `recovery::to_wallet_policy`; `coldcard::read`, `parse`; `multisig_config::parse`, `parse_named`; the `tapmulti` reader; `faraday_intake::wallet::read_wallet` | Wallets, WalletsStart, Files, Restore, Create, Tools | coordinators |
| bsms | Read BSMS records and sign the signer's (BIP 129) | `bsms::parse`, `KeyRecord::verify`; `Keys::bsms_record`; `texts::bsms_record` | Create, Files | coordinators |
| compile | Compile a spending policy to miniscript | `compile::compile` | Tools | spend_paths |
| musig-key | Aggregate MuSig2 keys (BIP 327, 328, 390) | `musig::key_agg`, `key_agg_with`, `aggregate_xpub`, `synthetic_xpub`, `aggregate_at`, `parse_tr_musig` | Create, Wallets | multisig |

### Randomness

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| dice | Dice to entropy, with the cautions | `faraday_entropy::dice`'s `push`, `entropy`, `warnings`, `word_indices` | KeyGen, Randomness | randomness |
| coins-hex | Coins and hex to entropy | `coins`, `hex` modules' `entropy` | KeyGen, Randomness | randomness |
| cards | Playing cards to entropy, SHA-256 of draw indices | `cards`' `entropy`, `commitment` | KeyGen, Randomness | randomness |
| camera | Camera noise to entropy | `camera`' `push`, `entropy`, `commitment` | KeyGen, Randomness | randomness |
| pointer-keys | Pointer and key timings to entropy (the desktop app) | `pointer_keys`' `commit` | KeyGen, Randomness | randomness |
| mix | Mix sources: SHA-256 over their commitments | `mix`' `push`, `entropy` | KeyGen | randomness |
| fresh | The session's randomness, and a person's own mixed in (F86) | `fresh.rs`: `draw`, `own_add`, `seeded`, `take` | KeyGen, Randomness, Settings, CreateVault | randomness |
| sign-seed | The signing seed: aux randomness and MuSig2's `rand'` chain | `fresh.rs`: `sign_seed`; `faraday_psbt::musig`'s chain | Spend | nonces |
| booklet | Booklet dice for codex32 | `booklet`' `push`, `character` | KeyGen | other_backups |
| roller | Faraday Demo's dice roller (F96) | `fresh/roller` | KeyGen | faraday:demo |
| diceware | A Diceware passphrase from dice, the lists checked by digest | `diceware::words`, `sha256` | VaultContents | passphrases |

### Backups

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| seedqr-make | Make a SeedQR or CompactSeedQR (`Reveal::SeedQr`) | `seedqr::encode_seedqr`, `encode_compact`; `Keys::seedqr_matrix`; `pdf.rs`'s sheets; `picture.rs`' `png` | Backup, BackupFlow, VaultContents, QR | backups |
| seedqr-read | Read a SeedQR back to words | `words::from_seedqr_digits`, `from_compact_seedqr`; `Keys::seedqr_reads_as` | Entry, Backup | backups |
| codex32 | Encode, checksum and recover codex32 (BIP 93) | `Keys::codex32_string`, `codex32_reads_as`, `codex32_share_record`, `codex32_of_seed`, `add_codex32`; `codex32::note` | Entry, Backups, Backup, VaultContents | other_backups |
| codex32-make | Make codex32 shares: worksheets, a fresh set, derived shares | `worksheet::of`, `push`, `finish`; `fresh::fresh_set`; `WorkSet::new`, `push`, `make`, `translation`; `codex32_new_set`, `codex32_derive`; `Keys::codex32_shares` | KeyGen, Backup | other_backups |
| codex32-correct | Correct a codex32 string (F94) | `correct::correct`; `codex32_correct::offer` | Entry, Backups | other_backups |
| codex32qr | Write a codex32 string as Codex32QR (F95) | `qr::codex32qr` | KeyGen, Backups, QR | other_backups |
| slip39 | Split and recover SLIP-39 shares | `slip39::split`, `recover`, `encrypt`, `decrypt`; `words::slip39_split`; `slip39_seed` | Backup, Entry | other_backups |
| xor | Seed XOR parts | `xor`' `push`, `entropy`; `Keys::seed_xor` | Backup, Entry, Tools | other_backups |
| stick-write | Write words, a passphrase or a SeedQR to a stick (`Reveal::BackupStick`) | the backup flow's `wallet_out_files`, `seed_stick_bytes` | BackupFlow, Backup | backups |
| outbox | An unprotected file to the Outbox (`Reveal::OutboxFile`) | the secret-out sheet's save | Files | air_gap |
| inbox-secret | Recognise a seed or key in a file or a scan | `inbox::fingerprint`, `potential_seed`, `secret_in`; `scan_key::key_of`, `pasted_secret` | Files, Visit | air_gap |

### Derived keys and tools

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| bip85 | Derive BIP-85 children (`Reveal::Bip85Child`) | `bip85::derive`, `child_mnemonic`, `child_wif`, `child_xprv`, `child_hex`, `child_password_base64`, `child_password_base85`; `Keys::bip85_child` | Bip85 | tools (added to Bip85's pages) |
| vanity | Grind a vanity address | `vanity::grind`, `grind_passphrase`, `grind_account`; `Keys::vanity_grind` | Vanity | passphrases |
| lightning | A Lightning node key, from words or an aezeed (`Reveal::LightningKey`) | `aezeed::decode`, `decode_at_cost`, `decode_bytes`, `node_key`, `ldk_node_key`; `aez`; `faraday_crypto::scrypt`, `blake2b`; `lightning::node_from_words`, `node_from_aezeed` | Lightning | tools |
| silent-receive | Silent payments: the address, labels, scan key and payments checked (BIP 352, 392) | `silent::derive`; `Receiver::scan_key`, `payments_in`; `silent_wallet`; `faraday_psbt::silent::check` | Silent, Wallets | silent_payments |
| silent-send | Send to silent payments, with DLEQ proofs (BIP 374, 375); not in the app | `dleq`; `faraday_psbt::silent::add_shares`, `compute_scripts`, `signable` | none | none |
| tools-hash | The calculators: SHA-256, HASH160, Base58Check, bech32, a descriptor's checksum | `hashes`; `faraday_codec::encodings` | Tools | tools |

### Signing

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| psbt-read | Read a PSBT, v0 and v2 (BIP 174, 370) | `Psbt::parse_bytes`, `parse_base64`; `v2::parse_bytes`, `parse_base64`, `to_v0` | Spend, Files, Decode, Family | verifying |
| psbt-review | Review: amounts, verified change, warnings, sighash kinds, the signed-amount memory, registering a wallet | `inspect::inspect` and its helpers; `register::offer`, `register`; `context::wallet_claim`, `from_master`, `derive_key` | Spend, Family | verifying |
| sign-ecdsa | Sign with ECDSA, RFC 6979, low-R | `sign::sign`, `sign_ecdsa`; `faraday_keys::psbt::sign`; `Keys::sign` | Spend, Family | nonces |
| sign-schnorr | Sign with Schnorr, key path and tap leaf (BIP 340, 341, 342) | `sign_taproot`, `sign_tap_leaf` | Spend, Family | nonces |
| sign-musig | Sign as a MuSig2 participant (BIP 327, 373) | `sign_musig`; `musig::nonce_gen`, `sign`, `partial_sig_verify`, `partial_sig_agg`; `faraday_psbt::musig` | Spend | nonces |
| sign-check | Verify each signature made, and sign twice to compare | `verify::deterministic`, `ecdsa_mode`, `schnorr_mode`, `verify_signatures`, `repeated_nonces`; `faraday_keys::psbt::recompute_signatures`, `key_refs` | Spend | nonces |
| finalize | Finalize a transaction | `finalize::finalize` | Spend, Family | verifying |
| message-sign | Sign a message (BIP 137, 322) | `message::sign`; `faraday_keys::message::sign`; `Keys::sign_message` | Message | message |
| message-verify | Check a signed message | `message::verify` | CheckMessage | message |
| tx-decode | Decode a transaction: txid, wtxid, weight | `transaction::read_raw` | Decode | verifying |
| musig-across | MuSig2 rounds across places, the secret nonce kept in a vault; compiled out (F109) | the vault record type 9 code | none | none |

### Transfer

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| qr | Encode and read a QR code, its error correction | `faraday_codec::qr::encode`, `encode_bounded`; `decode::decode_luma`, `decode_picture`; `faraday_files::qr_in_png` | QR, Transfer | air_gap |
| ur | UR codes: CRC32, fountain codes (BCR-2020-005, 006, 012) | `faraday_codec::ur`; `faraday_qr::send`'s `ur_psbt_frames`, `ur_bytes_frames` | QR, Transfer | air_gap |
| bbqr | BBQr and Specter pNofM | `faraday_qr::bbqr`, `specter`; `send::bbqr_frames`, `parts_frames` | QR, Transfer | air_gap |
| envelope | The file envelope `faraday-file-v1`: its SHA-256 | `transfer::read_any`, `read`, `scanned` | Transfer, Files | air_gap |
| file-hash | A file's SHA-256, shown and kept | `faraday_crypto::hash::sha256_whole`; `file_menu::sha256`; `receipt::sha256_of`; `gpg::sha256`, `sha256_hex`; `Faraday::hash` | Files, Visit | air_gap |

### Vaults

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| vault-create | Make a vault: Argon2id, HKDF-SHA256, XChaCha20-Poly1305, ChaCha20 fill | `faraday_vault::create`, `derive`, `stretch`; `faraday_keys::vault::create`, `measure`; `faraday_backup::argon2id` | CreateVault | faraday:vaults |
| vault-unlock | Unlock a slot | `faraday_vault::open`, `open_with`; `OpenVaults::unlock` | Unlock | faraday:vaults |
| vault-seal | Seal a record into a slot | `faraday_vault::seal`; `OpenVaults::sealed` | VaultContents, BackupFlow | faraday:vaults |
| vault-reveal | A vault record's secret on screen, into a form or a record (`Reveal::VaultEntry`) | `OpenVaults::reveal`, `words_of` | VaultContents | faraday:vaults |
| kdbx | Write a KeePass KDBX 4 file | `kdbx::write`, `verify`, `stretch`, `keys`; `write_kdbx`; `kdbx_entry` | VaultContents | faraday:vaults |
| totp | A TOTP code (RFC 6238) | `faraday_vault::totp::code`; `faraday_keys::totp::code_at`; `OpenVaults::totp` | Codes | faraday:time |

### GPG and Secure Boot

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| gpg-create | Make a GPG key: Ed25519 certify and sign subkey | `faraday_keys::gpg::create`; `faraday_pgp::certificate` | VaultContents | faraday:gpg |
| gpg-sign | Sign with the GPG key | `faraday_pgp::sign`; `gpg_sign_armored` | VaultContents, Files | faraday:gpg |
| gpg-revoke | Make the revocation certificate and its file | `revocation`, `revocation_file`; `gpg_revocation_file`, `gpg_new_revocation_file` | VaultContents | faraday:gpg |
| gpg-paperkey | Write the paperkey (`Reveal::GpgPaperkey`) | `faraday_pgp::paperkey`; `gpg_paperkey` | VaultContents | faraday:gpg |
| gpg-restore | Restore a GPG key from its paperkey; built in the crates, no screen yet | `gpg::Restore::line`, `finish` | none | faraday:gpg |
| gpg-verify | Read public armour and verify a detached signature | `read_public_armor`, `verify_detached` | Files, VaultContents | faraday:gpg |
| sb-keys | Make Secure Boot keys and their enrolment files | `faraday_sb::make`, `enrolment`; `secure_boot::create`; `sb_enrolment`; `rng` | VaultContents | faraday:secure-boot-keys |
| sb-sign | Check a release's sums and signature, then sign its boot program (F134) | `release::check`, `sign`; `pe`'s Authenticode; `sb_sign_image` | VaultContents | faraday:secure-boot-keys |

### The device and the Starter

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| selftest | The start-up self-test | `faraday_selftest::run` and its checks | Settings | faraday:keys-kept |
| starter-write | Write an image and check its SHA-256 | `faraday_payload::decode::write_payload`, `write_image`, `hash_image` | the Starter's Make | faraday:checking-written |
| starter-signed | Check a boot file the person signed (F133) | `signed.rs`' `choose`; `faraday_sb::pe::transplant` | the Starter's Make | faraday:checking-written |
| starter-recovery-key | Read a BitLocker recovery key for the notes PDF, hold it until the sheet is made, wipe it (F49) | `faraday_starter::api::RecoveryKey::parse` and the sheet's maker | the Starter's BitLocker step | faraday:checking-written |

77 actions: 10 keys, 5 wallets, 11 randomness, 11 backups, 6 derived
keys and tools, 11 signing, 5 transfer, 6 vaults, 8 GPG and Secure
Boot, 4 the device and the Starter.

## 13. Other places that depend on a cryptographic function

Each is a touchpoint of the actions it describes (§3): generated,
checked, or named for review.

| Place | What depends | Kind |
|---|---|---|
| `docs/STANDARDS.md` §1 to §7 | Statuses, rows, own calls, novel list | Generated |
| `docs/AUDIT.md` §1, §2, §3 | Crates, reveals, written-here list | Checked, checked, generated |
| `docs/KEYS.md` | The reveals and their count | Checked |
| `docs/COMPUTE.md` §2.3 | The token's roots | Checked |
| `docs/CROSSCHECK.md`, `tools/crosscheck/` | Areas, the stamp's files and vectors | Checked; the stamp reads the manifest (§10) |
| `tools/vectors/` and their READMEs | Vector folders | Checked; READMEs named for review |
| `fuzz/`, `faraday/fuzz/` | Fuzz targets | Checked |
| `core/faraday-selftest/src/lib.rs` | Its check names | Checked |
| `faraday/faraday-core/src/catalog.rs` | The Tools tiles' standard tags | Checked |
| `faraday/faraday-core/src/learn.rs` | Which pages each screen opens | Tested (§9.1) |
| `docs/SECURITY-MODEL.md` | Where keys live, the vault's cryptography | Named for review |
| `docs/INTEROP.md` | What is read and written, the refusals | Named for review |
| `docs/VAULT.md`, `docs/KDBX.md`, `docs/CODEX32.md`, `docs/TOTP.md`, `docs/QR.md`, `docs/OWN-RANDOMNESS.md`, `docs/WALLETS.md` §3.3, `docs/SPEND-FROM-BACKUP.md` | Their constructions and refusals | Named for review |
| The crate READMEs under `core/` | What each computes and refuses | Named for review |
| `docs/learn/` and `docs/learn/faraday/` | The prose of Nonces, Randomness, Backups in other forms, Silent payments, Vaults, GPG, Time, Where keys are kept | Named for review |
| `docs/guide/screens/README.md` and its pages | What a screen's control computes | Named for review |
| `faraday/faraday-strings/src/en.rs` | Strings that state a construction: Settings' Guided explainers, the Tools tiles' lines | Named for review |
| `tools/reference/` | The pinned sources an area was written from | Named for review |
| `docs/deps/` | A primitive crate's page: `**Cryptography:** computes` | Checked (§4.1) |
| `SECURITY.md`, `docs/AUDIT-SUBMISSIONS.md` | The scope an audit is asked to cover | Named for review |
| Release notes | What changed in the cryptography since the last release | `just crypto-changes` (§3.3) |
| The website | Learn blocks, the standards page | Rendered from the pin (§11) |

## 14. Rule lines for CLAUDE.md

Added when batch 4 puts the lint in `just lint`, beside the Learn
exception and the structure rules:

- Every cryptographic function is covered by an action of
  docs/crypto/actions.toml (F148): what the inventory finds by its rules
  (the compute token in the crates that hold keys, a reveal, a primitive
  crate named, a public function that reaches one) and no action covers
  fails `just lint`, as does an action whose code, libraries or vectors
  changed since its fingerprint. `just crypto-review <id>` is the only
  way to re-stamp: it lists every place the action touches, regenerates
  what is generated, and refuses while a named test, area or vector is
  missing. A function the rules find that computes nothing on a secret
  or signed data is a `[[not_crypto]]` entry with its reason; their count
  only falls.
- The "How Faraday does it" blocks, `docs/STANDARDS.md` §1 to §7 and
  `docs/AUDIT.md` §3 are generated from the manifest and never edited by
  hand; the blocks' values are a third exception to the one strings
  file, beside the Learn pages and the self-test's check names.
- A screen that offers an action of the manifest carries **How Faraday
  does it** in its top app bar, and the reachability test holds it.

## 15. Order of work

Each batch is its own commit; the lint enters `just lint` only when the
manifest covers every function it finds, with no transitional
exemptions.

1. **Inventory** (Sonnet). The `crypto-inventory` binary in
   `tools/costs-lint`, the walker shared with the costs lint; rules 1 to
   4 of §4.2; closures; body hashes; test functions; call sites in
   `faraday-core`; JSON lines out. A self-test over a fixture crate, as
   `tools/lint-guide.py --self-test` has, showing each rule and each row
   of §5.1's table. Check: `cargo run -q -p costs-lint --bin
   crypto-inventory` over the tree, its count of cryptographic functions
   reported.
2. **Manifest and checker** (Sonnet). The schema of §2.2;
   docs/crypto/check.py (`check`, `review <id>`, `changes <tag>`); the
   `**Cryptography:**` line on the primitive crates' `docs/deps/` pages;
   the empty manifest and the list of uncovered functions it prints.
3. **Entries** (Sonnet, several agents, one area each of §12's groups,
   at most four groups per agent). Each entry filled from the code, every
   fact checked there: the roots exist, the tests exist and run, the
   vectors and areas are the ones named, the libraries' versions are the
   lock's, a status is STANDARDS' as read on its date. Seeded from §12
   and STANDARDS §2 to §5; every function the inventory reports
   uncovered is added to an action or to `[[not_crypto]]` with its
   reason. Each agent stamps its entries with the recipe.
4. **Generators and the lint** (Sonnet). docs/crypto/gen.py for §3.1;
   STANDARDS and AUDIT §3 regenerated; the checks of §3.2; the ratchet
   file; `just lint` runs the inventory, `check.py check`, `gen.py check`;
   CLAUDE.md's lines (§14) and `docs/DEVELOPING.md` and `tools/README.md`
   for the recipes.
5. **Review** (Opus). The generated STANDARDS and AUDIT §3 against
   today's text, each difference settled against the code; the
   `[[not_crypto]]` list read entry by entry.
6. **Screens** (Opus). Designed: the icon, the bar's order, the Learn
   tab and the block's drawing at every size and the Pi, the Starter's
   sheet (§8); the strings listed (§8.6); the two new Learn pages
   written (`docs/learn/faraday/secure-boot-keys.md`,
   `docs/learn/faraday/checking-written.md`); DESIGN §4.1 and §4.12 and
   LAYOUT §8 point to §8; the guide's pages listed (§8.7). Left to
   batch 7, being code: the strings themselves, the two pages in
   faraday-learn's `OWN` table and the Starter's `LearnId`, and Bip85's
   and Explore's Learn lists (`faraday/faraday-core/src/learn.rs`).
7. **Screens built** (Sonnet). From batch 6's design (§8): the icon
   baked, the strings of §8.6, the pages registered (`secure-boot-keys`
   in `OWN` and on VaultContents' Secure Boot kind, `checking-written`
   in `OWN` and `LearnId`), `keys_kept_page()` added to Explore's
   pages and `&EN.tools` to Bip85's, the guide's pages of §8.7 drawn
   again; the reachability
   test (§9.1); the screens check (§9.2); one tour rendered per size.
8. **The cross-check** (Sonnet). Each area's facts as attributes
   (§10.1), moved from the README's table word for word; the README's
   table and differences generated; the stamp records each area's case
   count; `tools/crosscheck/stamp.py` reads `FILES` and `VECTORS` from
   the manifest and names the changed actions (§10.2). Before batch 4's
   generator, so the Cross-check field has its source.
9. **The website** (later, its own repository; §11).

The last batch to build moves this spec's plan (§12's seed table, §14,
§15, §16) to docs/history/CRYPTO-LEARN-LOG.md (new) and makes it a
Reference.

## 16. Questions for the owner

1. **The bar's action.** Settled (owner, 2026-10-28): the bar carries
   **How Faraday does it** beside the ?, in the ⋮ first on the Pi (§8).
2. **An action another action uses.** Settled (owner, 2026-10-28): a
   change fails every action that uses it as well as its own (§4.4,
   §5.2).
3. **A local variable renamed** counts as a change. Settled (owner,
   2026-10-28; §5.1).
4. **The Starter's BitLocker sheet.** Settled (owner, 2026-10-28): the
   recovery key carries a block, starter-recovery-key (§8.4).
5. **Two new Learn pages.** Settled (owner, 2026-10-28): Secure Boot
   keys, and the Starter's Checking what is written.
6. **Actions not in the app.** Settled (owner, 2026-10-28): the
   website's standards page lists them as "In the repository, not in the
   app" (§11).
7. **The repository's address.** Settled (owner, 2026-10-28):
   `github.com/bitcoinpasada/faraday`, as text in every block's Code
   field; the website links it to the release's tag.
8. **What the cross-check does.** Settled (owner, 2026-10-28): each block
   carries a Cross-check field saying what the F79 cross-check does for
   the action, read from its area (§1, §10.1).
9. **A list item taller than three lines.** Settled (owner, 2026-10-28;
   F148): a block's field is drawn whole, the item growing by its lines
   (§8.5), where Material's list item stops at three lines and truncates
   its supporting text; Faraday cuts no fact.

## 17. What this does not catch

- A function defined inside a `macro_rules!` body is not read: none is
  defined there in a crate with a cryptographic function as of
  2026-10-28. A function nested in another function's body is hashed as part of the outer one and
  not listed on its own.
- Trailing commas are normalised; other token changes rustfmt makes
  (parentheses it adds or removes) change the hash.
- A method call in `faraday-core` on a value whose type the file never
  names is not a call site (§9.2).
- A function generated by a macro, or reached only through a trait
  object or a function pointer, is invisible to the walker; the
  primitive it calls is still found where the macro's expansion is
  written in a file of the tree.
- A call resolved by name may cover a function in the wrong action's
  closure (§4.4); its code is still fingerprinted, under that action.
- Constant-time claims are copied from the library or the code's own
  documentation; nothing here measures timing.
- The places named for review (§3.3) are listed, not read: a stale
  sentence in SECURITY-MODEL is caught by the reviewer the recipe
  prompts, not by the lint.
- A library's behaviour changing without its version changing (a
  vendored patch) is caught by `cargo vet` and the cross-check stamp,
  not by a fingerprint.
