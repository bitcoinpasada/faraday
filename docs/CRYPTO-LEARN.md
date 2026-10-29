# How Faraday does it: one manifest for every cryptographic action

**Kind:** Spec
**Status:** Specified (F148), not built
**Open:** the manifest and its entries, the inventory lint, fingerprints and the re-stamp recipe, the step-by-step order of a flow's blocks (§8.1, §8.5), the guide's pictures and text of §8.7, the ⋮ row's leading icon (§8.3), the cross-check stamp read from the manifest, the website's rendering, §18's constructions on the website (§18.7) and their `vault-slot` and `vault-plain` tables (§18.2; the manifest, check.py, gen.py and Learn's drawing of them are built, F159)

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
| **Cross-check** | First, for an own construction, its label "Own construction, not yet independently reviewed", and for an action with an area, who wrote the reference and, where it is Faraday's own model, that it is not an independent check and which part Faraday wrote (F160, §10.3). Then what the F79 cross-check does for this action (owner, 2026-10-28), one line each: the area; the Faraday code it drives; the second implementation it compares against and how that one is itself checked first; the published vectors it runs; what its random cases vary; each known difference, where Faraday refuses what the second implementation accepts; when it runs (whenever the crypto changes, and with a new seed and about ten times the cases before every release); and the last passing run for this release: its seed and its case count for the area. | The independence lines are read from the action's `independence` and `label` in the manifest (§10.3); the rest from the area (§10.1), never typed in the manifest. An action with no area reads "not cross-checked: <reason>", and an area with no second implementation for a part says which part ("AEZ itself has no second implementation here"). |
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
schema = 3                   # 2: constructions (§18); 3: independence and label (§10.3)

# One table per standard, the only place its name and status are written.
[standard."BIP 174"]
name = "Partially Signed Bitcoin Transaction Format"
status = "Deployed"          # BIP 3's statuses, or "Informational", "Draft", "Convention", "Faraday's own"
checked = "2026-10-23"       # when the status was read
source = "bips master"       # or "vendored: tools/vectors/bip85"
tags = ["PSBT"]              # optional: the spellings a Tools tile names it by

# A tile tag that names no standard, with its reason (a unit, not a format).
[[tag_no_standard]]
tag = "sat"
reason = "A unit on the amount calculator; it names no standard."

# One table per STANDARDS row (§3.1), in STANDARDS' order: the row's own columns.
[[row]]
id = "psbt-v0"
section = 4                  # STANDARDS §2 to §5
what = "PSBT v0 (`core/faraday-psbt`)"
completeness = "Signs every wallet kind above; the finalizer is written here"
standard_note = "…"          # optional: a sentence after the standards its actions name

# A row that describes other rows' actions as well: its cells are theirs.
[[row]]
id = "seed-xor-tool"
section = 5
what = "Seed XOR tool"
completeness = "As in §3"
uses = ["xor"]               # may give standard, verified and own as text

# A row with no action: work not built, or work that computes nothing of its own.
[[row]]
id = "nostr-key"
section = 5
what = "Nostr key"
completeness = "Not built (`docs/NOSTR.md`)"
no_action = "Not built."     # the reason; the three cells are then given as text
standard = "BIP 85 128002' (Deployed), NIP-19, NIP-49"
verified = "None"
own = "None"

# One table per action.
[[action]]
id = "sign-ecdsa"
action = "Signs a transaction input with ECDSA, from the key the PSBT's origin names."
row = "ecdsa-nonces"
standards = ["BIP 143", "RFC 6979"]
name = "Sign with ECDSA"     # the block's heading: 2 to 6 words, plain
learn = "nonces"             # a page of faraday_learn::pages::EN, or "faraday:<id>" for one of OWN
screens_not = [{ screen = "Vaults", reason = "…" }]   # a screen whose code calls a root and does not offer it (§9.2)
screens = ["Spend", "Family"]   # Screen variants, or "starter:<page>" (write, confirm, writing, done, start)
functions = [                # the roots this entry covers (§4.4)
  "core/faraday-psbt/src/sign.rs::sign_ecdsa",
  "faraday/faraday-keys/src/psbt.rs::sign",
]
code = ["core/faraday-psbt/src/sign.rs::sign_ecdsa"]   # what the block's Code field shows
novel_rank = 3               # an action with a Novel own call: its place in STANDARDS §7
label = "own construction, not yet independently reviewed"   # schema 3, F160: every action with a novel_rank, and any other own construction

[action.implementation]
crate = "core/faraday-psbt"
libraries = ["secp256k1"]    # versions read from Cargo.lock
adds = "Low-R grinding by default; each signature verified before it is written; signed twice and compared."
written_here = "…"           # what the crate writes itself rather than takes from a library (AUDIT §3)

[action.checked]
vectors = ["psbt"]           # folders under tools/vectors/
tests = ["core/faraday-psbt/tests/all/sign.rs::rfc6979_vectors"]
crosscheck = "sign"          # tools/crosscheck/area_sign.py; or not_crosschecked = "<reason>"
independence = { class = "own-over-outside", author = "Faraday's authors (`area_sign.py`), over …", ours = "BIP-174's signer …" }
                             # schema 3, F160, with crosscheck only: class outside, own-over-outside or own-only;
                             # author, who wrote the reference; ours, the part Faraday wrote (not for outside)
selftest = ["ecdsa_vector"]
fuzz_none = "Signing takes no outside input but the PSBT; psbt_parse fuzzes that."
other = ["Core end to end (INTEROP §1)"]   # checks outside the tree: another program, other wallets' files

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

Several actions may share a row, and each action has one row. A row
has its own actions, or names in `uses` the actions it describes as well
(the Seed XOR tool, watch-only wallets), or has none and says why in
`no_action`: a format not built (Electrum seeds, Nostr keys), or work
that computes nothing of its own (the backup plan); such a row gives its
Standard, Verified by and Faraday's own call as text, which a row with
its own actions never does. An action with no screen (`screens = []`) is
built but offered nowhere in the app (silent payments sending, the
recovery-wallet builder, F109): it has a row, a fingerprint and every
check, and no Learn block (`learn = "none"`).

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
| `docs/STANDARDS.md` §2 to §5 | Each table between two marker lines (`<!-- crypto-manifest: begin §3 -->` and its end), one row per `[[row]]`: What, Standard (status) (the standards its actions name, grouped by status, then its `standard_note`), Completeness, Verified by (the union of its actions' checks: vectors, each cross-check area by its name and what it is compared against, the `other` checks, the tests counted by crate, self-test checks, fuzz targets, the reasons an action is not cross-checked), Faraday's own call (the union of their own calls). A row that `uses` other actions takes their cells; one with no action gives its own. §0, §8 and §9 stay hand-written. |
| `docs/STANDARDS.md` §6 | The list of Conservative own calls, with and without a decision. |
| `docs/STANDARDS.md` §7 | The Novel own calls, in the rank each entry's `novel_rank` gives; actions of one rank are one item. |
| `docs/AUDIT.md` §3 | The "written here" list: each action's `implementation.written_here`, grouped by crate. |

The first generation of STANDARDS and AUDIT §3 was settled row for row
against the hand-written text it replaced, each difference against the
code (batch 5, §15).

### 3.2 Checked

| Place | Check |
|---|---|
| `docs/AUDIT.md` §1 and §4 | Every crate an action's implementation names has a row in §1, or in §4 for a crate that touches cryptographic bytes and computes none, and every row of §1 is named by an action. A §1 row says "written here" exactly when one of its crate's actions gives `implementation.written_here`; a §4 crate's actions give none. |
| `docs/AUDIT.md` §2 and `docs/KEYS.md` | Every variant of `Reveal` in `faraday/faraday-keys/src/session.rs` is named by at least one action's `secrets.reveal`, and every named reveal exists. The count in words ("fourteen named reasons") in AUDIT §2, KEYS and CLAUDE.md is the variants' count. |
| `docs/COMPUTE.md` §2.3 | Every root that table names, for the kinds `WordsParse`, `Kdf`, `MasterKey`, `KeyDerive`, `DescriptorParse`, `Address`, `PsbtParse`, `PsbtInspect`, `Sign`, `QrEncode`, `QrDecode` and `FileHash`, is covered by an action (§4.4); an action function that takes `&Compute` counts a `Work` kind the table lists. |
| `docs/CROSSCHECK.md` and `tools/crosscheck/` | An action's `crosscheck` names an area whose file exists (`tools/crosscheck/area_<name>.py`), or `not_crosschecked` gives a reason. Every area file is named by at least one action. |
| `tools/vectors/` | Every folder an action names exists; every folder under `tools/vectors/` is named by an action, or by the manifest's list of vector folders for no action, each with its reason (the Diceware lists' digests are an action's; the word lists are not). |
| Fuzz targets | A named target is a `[[bin]]` in `fuzz/Cargo.toml` or `faraday/fuzz/Cargo.toml`; `fuzz_none` gives a reason. |
| The self-test | Every check function in `core/faraday-selftest/src/lib.rs` (`bip39_vector`, `ecdsa_vector` and the rest) is named by an action's `selftest`, and every name given exists. |
| The Tools tiles | Each tag on a tile in `faraday/faraday-core/src/catalog.rs` names a standard (by number, by a spelling its `tags` lists, or by its name) of an action whose screens include the screens the tile opens or Tools, or is a `[[tag_no_standard]]` with its reason. |
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
tests left out (`#[cfg(test)]`, and a `cfg` naming one of the features
no shipped binary turns on, `testkit` and `master-for-tests`:
`tools/lint-keys-tests.sh` fails when one is on in a shipped binary),
and the crates that are in no shipped binary (`faraday-testkit`,
`faraday-snapshot`, `faraday-alloc-count`, `faraday-argon2-bench`; the
inventory's `NOT_SHIPPED_CRATES`). The costs lint and the inventory must
agree on what is shipped code; one walker makes that true by
construction. Code that is not shipped is left out, never listed under
`[[not_crypto]]`. A rule inside `costs-lint` itself was considered
and not chosen: its job is the token's discipline, its findings are
`path:line` errors, and the inventory's output is data the manifest's
checker reads. Both binaries are one package, one `syn`, no new crate.

Every crate there is read and every function of it recorded, so that
a manifest root in any of them resolves (§4.3). A **cryptographic crate**
is one whose normal dependencies include a primitive crate; rules 1, 2
and 4 apply in those, and rule 3 in all, since a crate that depends on
none can still name a primitive through a Faraday crate's re-export. The
primitive crates are `faraday-crypto`, `bitcoin`, `secp256k1`, `sha1`,
`sha2`, `hmac`, `pbkdf2`, `argon2`, `chacha20`, `chacha20poly1305`,
`ed25519-dalek`, `rsa`, `getrandom`. The list is the inventory's
`PRIMITIVE_CRATES`. Each page under `docs/deps/` for a crate in it
carries a line `**Cryptography:** computes`; the lint fails when a
dependency's page says so and the list lacks it, so a new primitive
crate cannot enter without the inventory seeing it (F71 already requires
the page).

### 4.2 The rules

Outside tests, a function is **cryptographic** when:

1. **Token** (core and the `faraday/` crates that hold keys:
   `faraday-keys`, `faraday-vault`, `faraday-pgp`, `faraday-sb`,
   `faraday-payload`, `faraday-qr`): it takes `&Compute`. Not in
   `faraday-core` or the Starter, where 600 functions pass the token
   through to glue; there rule 3 finds what computes.
2. **Reveal** (`faraday-keys`): it takes a `Reveal` or returns
   `Revealed` or `Held`.
3. **Primitive** (every crate, `faraday-core` and the Starter
   included): its signature or body names a path into a
   primitive crate, after the file's `use` lines are applied
   (`use sha2::{Digest, Sha256}` makes `Sha256::new` a primitive path),
   or a path through a Faraday crate's re-export of one:
   the inventory reads every `pub use` that names a primitive crate or
   another re-export of one (`pub use bitcoin;` in `faraday-bip`, then
   `faraday_bip::bitcoin::hashes::sha256` in `faraday-qr`; `pub use
   x as Y`), crate by crate to a fixpoint, and a path that starts with
   the re-exported name counts as the primitive's. A glob (`pub use
   bitcoin::*`) names nothing and is not followed (§17).
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
   calls in its own crate (§4.4), reaches a function of rules 1 to 3.
   A function that gains the rule makes the calls that reach it count in
   turn, so the calls are worked out again until none does. A new
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
of every crate. `--roots FILE` prints, besides, the functions whose ids
FILE lists (one `file::Type::name` per line) and what they reach in their
crate: `check.py` writes the manifest's roots and `not_crypto` functions
to such a file and runs the inventory with it, so a root in a crate the
rules do not flag (`faraday-core`'s `inbox.rs`, the Starter's
`api.rs`, `faraday-files`) resolves and is fingerprinted with its closure.
(Chosen over printing every crate with `"rules":[]`, which would put
some 15,000 lines through the checker on every run.) A root the walker
does not find is not printed and fails as "not a function the inventory
finds". The summary by rule and crate goes to
standard error, and the exit status is 1 when a source file did not
parse, since the inventory is then incomplete.

### 4.4 Coverage

An action covers its `functions` (the roots) and their **closure**: the
functions of the same crate they call, transitively, stopping at a
function that is another action's root. A call is resolved within the
crate by the rule §9.2 uses for call sites, so that a closure does not
swell with every function of a common name (`TransferArea::scanned` pulled
in some 4,900 functions of `faraday-core` by name alone, and now 37):

- A **path call** is resolved through the file's `use` lines and taken by
  its last two segments: a method of the type before the name, or a free
  function of the module before the name; a bare name is a free function
  of the calling file. Where no function is in the module the path says
  (a glob import, a name re-exported from another module), every function
  of that name stays.
- A **method call** is taken only for a method of the caller's own impl
  type, or of a type the calling file names (any identifier of the file,
  its field types included), or of a function that is itself
  cryptographic (rules 1 to 4), whose receiver's type cannot be told. For
  the names the standard library's own types use (`collect`, `push`,
  `get`, `take`, `update`, …; the inventory's `STD_METHODS`) the edge is
  kept only for the caller's own type, or for a cryptographic callee whose
  type the calling file names (`Fresh::take` from any file that names
  `Fresh`): `x.collect()` in a file that names `Faraday` is an iterator's,
  not `Faraday::collect`'s.
- A name passed as a value without a call (`.map(helper)`) is taken as a
  call. A path or call expression `name(...)` is a call even when a local
  of that name exists (`let write = write(…)`); only the bare `name` used
  as a value is the local.

That still over-approximates: a helper may fall into a closure it is not
in, which only makes the fingerprint change more often. It must not miss
a call to a cryptographic function, which is why a method call on a
receiver of unknown type keeps the edge to a cryptographic callee.

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
- **A tag that names no standard.** `[[tag_no_standard]]`: a Tools
  tile's tag that is not a standard (the amount calculator's BTC and
  sat), with `tag` and `reason`.
- **A row with no action.** A STANDARDS row with `no_action` and its
  reason: work not built, or work that computes nothing of its own. It
  holds no function, so the inventory's coverage is untouched by it.
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
manifest, the top app bar carries **Cryptography**: Material's
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
(Practice), the mode pill, **Cryptography**, the ?, then Lock and
the ⋮ where the layout has them (DESIGN §4.1).

### 8.2 What it opens

It opens Learn as the ? does, below extra-large the Learn sheet, on an
extra-large window the supporting pane (LAYOUT §6), on a tab **How
Faraday does it** placed first among the sheet's tabs. The tab is there
exactly while the bar offers the button: on a step before the action's,
Learn has no How tab. The ? opens Learn as today, on the screen's first
page, with the How tab still first among the tabs; on an extra-large
window the ? pressed while the pane is shown puts it away, whichever
tab it shows, as today. **Cryptography** pressed while the pane
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
| Compact, the Pi's panel (268 dp) | **Cryptography** goes into the ⋮ overflow menu first, before the ?, wherever the page offers it, not only when the bar's tools would reach the chevron's target (LAYOUT §8): kept on the bar it would take room from the page's name, the way-back line or the eye (owner, 2026-10-28). The ⋮ takes the ? with it. Its row reads "Cryptography", with `Icon::Method` at its start, and it is the menu's first row, above **Learn**, **Show keyboard** and **Show sheets**. On the panel the ⋮ menu is a sheet, as every menu there. |
| Compact, other windows | In the ⋮ as on the Pi, by the same rule. |
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
scrim. Its head is `Icon::Method` and **Cryptography**
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
  headline "Cross-check", then the action's independence lines (§10.3),
  each a paragraph, what is not independent in the Warning role, then
  the area's parts in §1's order, each a paragraph
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
| `tip_how` | Cryptography | The bar's button's tooltip, Faraday and the Starter |
| `bar_menu_how` | Cryptography | The ⋮ row on a compact bar, Faraday and the Starter |
| `learn_tab_how` | Cryptography | The Learn sheet's and pane's first tab |
| `starter_how_title` | Cryptography | The Starter's sheet's head |
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
1280 × 800 and the Pi's panel); the bar's Cryptography, or its
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
The closures of §4.4 take their method edges by this same rule (the types a
file names counting every identifier in it, and a cryptographic callee
kept where the receiver's type is not known), so that the two agree on
what a call names. Each line says `path` or `method`. A module that serves several screens (an area's
`mod.rs`) gives all of its area's screens; an action may narrow them
with `screens_not = [{ screen, reason }]`, each counted in
`tools/lint-crypto.counts` (`screens_not`, only falls). Built as
`docs/crypto/gen_how.py`'s `screens_check`, which `check.py check` runs: a path call
`module::name` or `Type::name` and a method call by a distinctive name
(not a name every type has) of a type the file names; it reads names, not types, so
`--check-screens --lines` shows the call each pair comes from.
A screen's Learn tabs gain the pages of the blocks it offers, after its own
(`Faraday::learn_pages`), and the reachability test (`how_reach.rs`) holds that.

## 10. The cross-check and its stamp

### 10.1 What each area says about itself

Each area file states its facts once, as class attributes beside `name`,
which the generator reads (§3.1) and the lint checks are present and
non-empty:

- `faraday`: the Faraday code it drives, as paths the lint resolves;
- `reference`: the reference it compares with, with its pinned folder
  under `tools/reference/` or its distribution package, or the area's own
  file where Faraday's authors wrote it (whether it is independent is the
  manifest's, §10.3), and `reference_checked`:
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

### 10.3 Who wrote the reference

Built (F160, schema 3). An area may compare one action with code
others wrote and another with a model of Faraday's own (`sign`'s
signatures and its finalizer), so the class is the action's, not the
area's: `independence = { class, author, ours }` in the action's
`[action.checked]`, required with `crosscheck` and refused without it.
`class` is `outside` (code Faraday did not write, used as published),
`own-over-outside` (Faraday's own model over outside code) or `own-only`
(Faraday's own model alone); `author` says who wrote the reference;
`ours` names the part Faraday wrote and is given for the last two only.
Faraday's own constructions carry `label = "own construction, not yet
independently reviewed"` at the action's top level, every action with a
`novel_rank` among them, until a decision records an outside review.

The block draws them first in Cross-check: the label in the Warning
role; for `outside`, "Reference written outside Faraday: <author>."; for
the other two, "Not an independent check: …" in the Warning role with
the part Faraday wrote, then "Reference written by <author>.". gen.py
writes the cross-check README's "Who wrote each reference" and marks a
STANDARDS Verified by cell whose area's reference is Faraday's own, and
§7's items. `docs/crypto/verified.py`, in `just lint`, fails an action
with an area and no class, and a novel action with no label.

## 11. The website's interface

The website (its own private repository) builds from a pinned Faraday
commit and already reads the guide from the pin. It reads:

- docs/crypto/actions.toml at the pin, `schema = 3` (2 added §18's
  constructions, 3 the independence of §10.3), the schema of §2.2; a
  schema it does not know stops its build;
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

### 11.1 The page /cryptography

One page on the website, generated from the pinned manifest and never
written by hand (owner, 2026-10-28):

- **The standards.** An index at its top: each standard of the
  standards table with its name and status, and the actions that follow
  it, each a link to its block.
- **The actions.** Every action's block, its nine fields as the app
  draws them, grouped by §12's groups in their order, each block at its
  own anchor (`/cryptography#<id>`). The Code field links to the file
  at the release's tag. The actions with no screen follow under "In the
  repository, not in the app".
- **Filters.** By standard, by status, by unchecked value (not checked
  against vectors, not cross-checked, no fuzz target, a reference of
  Faraday's own, an own construction's label, §10.3), by an own call
  with no decision, and by an own call's kind (Conservative, Neutral,
  Novel). With no filter, everything shows.
- **One screen.** The whole page is one scrolling page with the filters
  above it; a filter hides blocks, never moves to another page.

- **Constructions.** Each construction of §18 (the vault's account,
  passphrase strength) at its own anchor, before its actions' blocks
  (§18.7).

The page reads only what §11 lists; the hash rule above holds it.

## 12. The actions found in the code

The manifest's 92 actions, each from its entry in `docs/crypto/actions.toml`
as batch 5 left it (2026-10-28): what it does, its roots (the functions
the entry names, by module), the `Screen` variants that offer it, and
its Learn page. The inventory finds 815 cryptographic functions under
`core/` and `faraday/`, and every one is covered by an action or by one
of the six `[[not_crypto]]` entries. Screens are `Screen` variants;
an action with no screen is built and offered nowhere in the app.
Learn pages are `pages::EN`'s fields, or Faraday's own by id
(`docs/learn/faraday/<id>.md`); `secure-boot-keys` and
`checking-written` are new, their text written, not yet in the apps'
page lists.

### Keys

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| words-read | Reads seed words typed, pasted or scanned, and checks their count and checksum. | `faraday_bip::bip39`: `Mnemonic::parse`, `Mnemonic::from_indices`, `Language::typed`, `Language::candidates_typed`, `Language::candidates_pinyin`, `Language::candidates_zhuyin`, `Language::word_readings`, `fold`; `faraday_keys::words`: `parse`; `faraday_bip::hangul`: `compose`; `faraday_bip::kana`: `compose` | Entry, Restore, Files, Family | words |
| words-make | Turns 16 to 32 bytes of entropy into seed words, with the checksum bits BIP 39 adds. | `faraday_bip::bip39`: `Mnemonic::from_entropy` | KeyGen | words |
| seed | Makes the 64-byte seed from words and a passphrase by PBKDF2-HMAC-SHA512, 2048 rounds, and adds the key it gives to the session. | `faraday_bip::bip39`: `Mnemonic::to_seed`, `Mnemonic::to_seed_unchecked`; `faraday_keys::session`: `Keys::add_words`, `Keys::add_words_with`, `Keys::add_mnemonic`, `Keys::add_with_passphrase`, `Keys::add_with_suffix` | Entry, KeyGen, Restore, VaultContents, Family | passphrases |
| master | Makes a master key and its fingerprint from a seed or an extended private key, by BIP 32. | `faraday_bip::keys`: `MasterKey::from_seed`, `MasterKey::from_seed_bytes`, `MasterKey::from_xpriv`, `MasterKey::decode`, `MasterKey::from_chain_code_and_key`, `MasterKey::for_network`; `faraday_keys::session`: `Keys::add_seed`, `Keys::fingerprint_with`; `faraday_keys::identity`: `master_of`, `fingerprint`, `fingerprint_of_words`, `fingerprint_of_mnemonic`, `fingerprint_of_seed`, `master_id`, `identity`; `faraday_bip::xkey`: `decode_xpriv` | Entry, KeyGen, Restore, Files, VaultContents | xpubs |
| derive | Derives a child key and the extended public key of an account, by BIP 32 and the purposes of BIP 44, 49, 84 and 86. | `faraday_bip::keys`: `MasterKey::derive`, `MasterKey::xpub`, `MasterKey::account_xpub`, `DerivedKey::to_xpub`; `faraday_bip::account`: `AccountXpub::xpub`, `AccountXpub::path`, `AccountXpub::slip132_string`, `AccountXpub::descriptor`, `AccountXpub::chain_xpub`; `faraday_keys::operations`: `Keys::xpub_at`; `faraday_keys::identity`: `xpub_at`, `key_text_of_words`; `faraday_keys::session`: `Keys::key_text`; `faraday_keys::texts`: `key_text`, `single_account`, `account_at`; `faraday_core::explore`: `parse_path`, `implied_script`, `ExploreArea::path`, `ExploreArea::reading_now`, `ExploreArea::act`, `Faraday::explore_path`; `faraday_core::explore::addresses`: `account_of`; `faraday_core::create`: `account_of`, `path_spec`; `faraday_core::restore`: `Faraday::restore_refusal_typed`, `Faraday::restore_took` | Create, Explore, Wallets, Tools, Restore | xpubs |
| hold | Holds a key in memory blinded, on pinned pages and sealed under a session key. | `faraday_bip::keys`: `MasterKey::reblind`, `MasterKey::blinded`; `faraday_keys::session`: `Keys::blind_key`, `Keys::reblind`; `faraday_crypto::sealed`: `SessionKey::from_entropy`, `SessionKey::weak`, `SessionKey::rotate`, `SessionKey::derive`, `Sealed::seal`, `Sealed::with`, `Sealed::reseal`; `faraday_crypto::pinned`: `Pinned::new`, `Pinned::new_with`, `Pinned::drop`; `faraday_crypto::secret`: `Secret::drop` | Settings | faraday:keys-kept |
| seed-shown | Shows the master seed of a key with no words in hex, and its codex32 secret and strings where the set is known. | `faraday_keys::session`: `Keys::seed_shown` | Explore | faraday:keys-kept |
| xprv-shown | Shows an extended private key at a path below a held key. | `faraday_keys::session`: `Keys::xprv_at`; `faraday_core::explore::shown`: `Shown::xprvs`, `xprv_path`, `ExploreArea::eye`, `ExploreArea::prune` | Explore | xpubs |
| wif-shown | Shows the private key of one address as a compressed WIF. | `faraday_keys::session`: `Keys::wif_at`; `faraday_core::explore::shown`: `Shown::wif`, `address_path` | Explore | xpubs |
| words-shown | Lends a key's words or passphrase as text for a named reason, and shows them on screen or for copying by hand. | `faraday_keys::session`: `Keys::reveal`, `Revealed::empty`, `Revealed::made`, `Held::of` | Explore, Backup, BackupFlow, VaultContents | backups |
| words-identity | Tells whether two keys, or a key and a copy of its words, have the same words, by a SHA-256 of their entropy, and orders the copy check's questions. | `faraday_keys::session`: `Keys::words_id`, `Keys::same_words`, `words_id`, `Keys::words_match`, `Keys::words_alone`, `Keys::words_identity`, `Keys::words_record_id`; `faraday_psbt::silent`: `check`, `check_inputs`; `faraday_core::quiz`: `Quiz::seed`; `faraday_keys::identity`: `secret_id` | Files, BackupFlow, VaultContents, Create | words |

### Wallets

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| multisig-account | Derives a BIP 48 cosigner account and its extended public key, and lends the account key to sign a BSMS key record. | `faraday_bip::keys`: `MasterKey::multisig_account_xpub`, `MasterKey::with_multisig_account_secret`; `faraday_bip::account`: `MultisigAccountXpub::xpub`, `MultisigAccountXpub::key_at`; `faraday_keys::texts`: `multisig_account`; `faraday_keys::session`: `Keys::multisig_account` | Create, Wallets, Tools | multisig |
| address | Derives the script and the address of a wallet at a chain and an index, and looks for an address among a wallet's first addresses. | `faraday_bip::policy`: `WalletPolicy::script_at`, `WalletPolicy::script_at_with`, `WalletPolicy::address_at`, `WalletPolicy::find_address`; `faraday_bip::account`: `AccountXpub::address`, `AccountXpub::find_address`; `faraday_core::explore::addresses`: `derive`, `Kept::at`, `Kept::page`, `ExploreArea::tick`; `faraday_core::wallet`: `Wallet::script_at`; `faraday_core::drawing::kept`: `View::script_at` | Wallets, Explore, Spend, Family, Tools | verifying |
| descriptor | Reads and writes a wallet's descriptor and wallet policy, and works out and checks the descriptor checksum. | `faraday_bip::policy`: `WalletPolicy::parse`, `WalletPolicy::from_parts`, `WalletPolicy::from_descriptor`, `WalletPolicy::parse_any`, `WalletPolicy::from_xpub`, `WalletPolicy::of_silent`, `WalletPolicy::internal_key`, `WalletPolicy::tapscript_quorum`, `WalletPolicy::leaf_quorum`, `WalletPolicy::template_text`, `WalletPolicy::to_text`, `WalletPolicy::to_descriptor`, `WalletPolicy::to_descriptor_template`, `WalletPolicy::to_descriptor_checksummed`, `WalletPolicy::spend_paths`, `WalletPolicy::leaf_of`, `PolicyKey::parse`, `PolicyKey::path`, `PolicyKey::xpub`, `parse_script_descriptor`; `faraday_bip::descriptor`: `descriptor_checksum`, `verify_checksum`, `checksum_facts`, `reads_as_descriptor`, `origin_fingerprints`; `faraday_bip::spend`: `spend_paths`; `faraday_bip::core_import`: `import_descriptors`, `pair_descriptors`, `import_descriptors_pair` | Wallets, WalletsStart, Files, Restore, Create, Tools | coordinators |
| recovery | Builds a timelocked recovery wallet from its parts and reads the parts back out of one. | `faraday_bip::recovery`: `RecoveryPolicy::to_wallet_policy`, `RecoveryPolicy::from_wallet_policy` | none | none |
| tapmulti | Reads taproot multisig descriptors with multi_a and sortedmulti_a, and builds their scripts at each derivation index. | `faraday_bip::tapmulti`: `is_nums`, `multi_a_script`, `sorted_multi_a_script`, `output_script`, `derived_keys` | Wallets, Create | multisig |
| wallet-files | Reads another wallet's export (Coldcard JSON and multisig configuration files) as a wallet and checks the first address it states; writes a native SegWit single-key wallet as Coldcard's file for Wasabi. | `faraday_bip::coldcard`: `looks_like_export`, `read`, `parse`, `name`, `cosigner_key`, `cosigner_keys`; `faraday_bip::multisig_config`: `parse`, `parse_named`, `slip132_mismatch`; `faraday_bip::json`: `parse`, `Value::as_u64`, `Value::strings_named`; `faraday_core::inbox`: `kind_of_path`; `faraday_intake::single_key`: `wasabi_key`; `faraday_intake::wallet`: `network_of_kind` | Files, Restore, WalletsStart | coordinators |
| bsms | Reads BSMS key and descriptor records, checks the signature on a signer's record and writes this device's own signed record. | `faraday_bip::bsms`: `SignerRecord::parse`, `SignerRecord::verify`, `SignerRecord::verify_with`, `signer_record`, `DescriptorRecord::parse`, `DescriptorRecord::to_text`, `looks_like_record`, `is_signer_record`; `faraday_keys::session`: `Keys::bsms_record`; `faraday_keys::texts`: `bsms_record` | Create, Files, Restore | coordinators |
| compile | Compiles a spending policy to a miniscript descriptor and lists the ways it can be spent. | `faraday_bip::compile`: `compile` | Tools | spend_paths |
| musig-key | Aggregates MuSig2 public keys (BIP 327), publishes the aggregate as an extended public key (BIP 328) and reads the musig() expression of a wallet (BIP 390). | `faraday_bip::musig`: `key_agg`, `key_agg_with`, `aggregate_xpub`, `synthetic_xpub`, `MusigExpr::from_xpubs`, `MusigExpr::aggregate_at`, `MusigExpr::key_at`, `parse_musig_expr`, `parse_tr_musig`, `script_pubkey_at`, `script_pubkey_at_with`, `sort_keys`, `Participant::xpub`, `AggregateKey::untweaked`, `AggregateKey::public_key`, `AggregateKey::x_only`, `AggregateKey::serialize_x_only`, `AggregateKey::apply_tweak` | Create, Wallets | multisig |
| xpub-spellings | Writes and reads an extended public key in its SLIP-132 spelling, and takes a version only where it agrees with the script and the path around the key. | `faraday_bip::slip132`: `version_bytes`, `encode_xpub`, `decode_xpub`, `key_facts`, `network_kind`, `Script::of_path`, `named_key`, `decode`, `encode`, `plain_descriptor`, `key_line`; `faraday_bip::xkey`: `decode_xpub` | Tools, Wallets | xpubs |
| address-read | Reads an address typed or scanned, and says whether the text so far can still become an address on a network. | `faraday_bip::address`: `is_bech32_prefix`, `is_address_prefix`, `parses`, `parse`; `faraday_core`: `Faraday::address_note` | Visit, Spend, Tools | verifying |

### Randomness

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| dice | Turns dice rolls into entropy by one of three published procedures, and cautions about runs, sequences and skew. | `faraday_entropy::dice`: `DiceRolls::commitment`, `DiceRolls::entropy`, `DiceRolls::push_under`, `DiceRolls::entropy_under` | KeyGen, Randomness | randomness |
| coins-hex | Packs coin flips into entropy most significant bit first, and takes hexadecimal digits as they are. | `faraday_entropy::coins`: `CoinFlips::commitment`, `CoinFlips::entropy`; `faraday_entropy::hex`: `RawHex::parse`, `RawHex::commitment`, `RawHex::entropy` | KeyGen, Randomness | randomness |
| cards | Turns playing-card draws into entropy as SHA-256 of the draws' indices, one byte each. | `faraday_entropy::cards`: `CardDraws::commitment`, `CardDraws::entropy` | KeyGen, Randomness | randomness |
| camera | Turns camera frames into entropy as SHA-256 over the frames' SHA-256 digests. | `faraday_entropy::camera`: `CameraNoise::push`, `CameraNoise::commitment`, `CameraNoise::entropy` | KeyGen, Randomness | randomness |
| pointer-keys | Records pointer moves and key presses with time and a running count, and commits them in batches of SHA-256 for the session's randomness. | `faraday_entropy::pointer_keys`: `PointerKeys::commit`, `PointerKeys::pointer`, `PointerKeys::key` | KeyGen, Randomness | randomness |
| mix | Combines two to five sources by hashing their commitments in the order taken. | `faraday_entropy::mix`: `Mixed::push`, `Mixed::commitment`, `Mixed::entropy` | KeyGen | randomness |
| device | Hashes the 32 bytes the shell answered the entropy request with into a key's entropy. | `faraday_entropy::device`: `DeviceRandom::commitment`, `DeviceRandom::entropy`; `faraday_core::keygen`: `device_own` | KeyGen | randomness |
| system-random | Reads 32 bytes from the system's generator for the desktop app and names the source. | `window`: `entropy` | Randomness, Settings | randomness |
| fresh | Keeps the session's seed and pool from the shell's answers, mixes a person's own randomness in, never in place of the system's, and draws the seed of a MuSig2 round. | `faraday_core::fresh`: `Seed::draw`, `reseed`, `Own::add`, `Fresh::take`, `Fresh::sign_seed` | KeyGen, Randomness, Settings, CreateVault, Spend | randomness |
| booklet | Reads the codex32 booklet's dice de-biasing worksheet: ten rolls of five dice make one bech32 character. | `faraday_entropy::booklet`: `BookletChar::push` | KeyGen | other_backups |
| roller | Rolls simulated dice for Faraday Demo from the session's randomness, one value per byte with no bias. | `faraday_core::fresh::roller`: `Pool::clear`, `Pool::next` | KeyGen | faraday:demo |
| diceware | Chooses Diceware words for a passphrase from dice rolls, from the three EFF lists checked by digest. | `faraday_bip::diceware`: `List::words`, `List::word` | VaultContents, CreateVault, Tools | passphrases |

### Backups

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| seedqr-make | Makes a SeedQR or CompactSeedQR picture from a key's words. | `faraday_codec::seedqr`: `encode_seedqr`, `encode_compact`; `faraday_keys::session`: `Keys::seedqr_matrix`; `faraday_core::backup_flow`: `seedqr_picture`; `faraday_core::picture`: `labelled_png`; `faraday_core::pdf`: `probe_matrix` | Backup, BackupFlow, VaultContents | backups |
| seedqr-read | Checks that a SeedQR read by the scanner is the one a key's words make. | `faraday_keys::session`: `Keys::seedqr_reads_as` | Entry, Backup | backups |
| codex32 | Reads, checksums, splits and recovers codex32 strings, and keeps and gathers the strings of a key's set. | `faraday_bip::codex32`: `note`, `Codex32::parse`, `Codex32::to_seed`, `Codex32::from_seed`, `Codex32::split`; `faraday_keys::session`: `Keys::codex32_string`, `Keys::codex32_share_record`, `Keys::codex32_of_seed`; `faraday_keys::codex32`: `Keys::add_codex32`; `faraday_keys::vault`: `OpenVaults::codex32_of_seed`, `OpenVaults::codex32_count`, `OpenVaults::codex32_with_typed`, `OpenVaults::gather_codex32` | Entry, Backups, Backup, VaultContents | other_backups |
| codex32-make | Fills the codex32 booklet's worksheets one character at a time and makes a fresh set or derived shares from them. | `faraday_bip::codex32::worksheet`: `ChecksumWorksheet::of`, `ChecksumWorksheet::push`, `ChecksumWorksheet::finish`, `TranslationWorksheet::of`, `TranslationWorksheet::push`, `TranslationWorksheet::result`; `faraday_bip::codex32::fresh`: `fresh_set`; `faraday_keys::codex32`: `WorkSet::new`, `WorkSet::push`, `WorkSet::make`, `WorkSet::translation`, `WorkSet::given`, `WorkSet::live_translation`, `WorkSet::worksheet`, `StringCheck::of`, `StringCheck::worksheet`, `Keys::codex32_derive`, `Keys::codex32_new_set`, `Keys::codex32_rebuilds`; `faraday_keys::operations`: `Keys::codex32_shares` | KeyGen, Backup | other_backups |
| codex32-correct | Proposes the one correction a codex32 checksum justifies for a damaged string, within BIP 93's bound. | `faraday_bip::codex32::correct`: `correct` | Entry, Backups | other_backups |
| codex32qr | Writes a 48-character codex32 string as a Codex32QR code. | `faraday_codec::qr`: `codex32qr` | KeyGen, Backups | other_backups |
| slip39 | Splits a master secret into SLIP-39 shares and recovers it from them, with the passphrase's Feistel encryption. | `faraday_bip::slip39`: `split`, `recover`, `encrypt`, `decrypt`, `Share::parse`, `Share::from_indices`; `faraday_keys::words`: `slip39_split`; `faraday_keys::operations`: `slip39_seed`; `faraday_core::keygen`: `KeyGen::deal` | Backup, Entry, KeyGen | other_backups |
| xor | Splits a seed into Seed XOR parts, each a BIP 39 mnemonic, and combines parts into the seed. | `faraday_entropy::xor`: `SeedXor::push`, `SeedXor::entropy`; `faraday_keys::operations`: `Keys::seed_xor` | Backup, Entry, Tools | other_backups |
| stick-write | Makes the file a backup stick holds for a key: its words or passphrase as one line, its SeedQR as a labelled picture, a codex32 string as one line or as a Codex32QR picture. | `faraday_core::backup_flow::wallet`: `Faraday::seed_stick_bytes`; `faraday_core::backups`: `Faraday::codex32_stick_bytes` | BackupFlow, Backup | backups |
| outbox | Makes a key's words file and its SeedQR picture, standard or compact, for the secret sheet to save to the Outbox unprotected. | `faraday_core::secrets`: `Faraday::offer_seed` | Files, Backup | air_gap |
| inbox-secret | Recognises a seed or a private key in a file, a scan or pasted text, works out the fingerprint of its key, and refuses a secret where one may not go. | `faraday_core::inbox`: `Secret::fingerprint`, `words_of`, `Faraday::secret_in`, `Faraday::potential_seed`; `faraday_core::scan_key`: `key_of`, `pasted_secret`; `faraday_core::wallet`: `text_may_be_secret`; `faraday_keys::identity`: `is_wif` | Files, Visit | air_gap |

### Derived keys and tools

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| hashes-macs | Hashes and authenticates bytes with SHA-256, HMAC-SHA256 and HMAC-SHA512, and stretches a password with PBKDF2. | `faraday_crypto::hash`: `sha256`, `sha256_whole`, `hmac_sha512`, `hmac_sha256`, `pbkdf2_hmac_sha512`, `pbkdf2_hmac_sha256`, `pbkdf2_sha256` | none | tools |
| scrypt | Stretches a password with scrypt, written in this crate, for LND's aezeed. | `faraday_crypto::scrypt`: `scrypt` | none | tools |
| tools-hash | Hashes bytes with SHA-256, SHA-256d and HASH160, and reads and writes Base58Check, bech32 and bech32m text. | `faraday_bip::hashes`: `hashes`; `faraday_codec::encodings`: `read`, `read_bech32`, `from_hex`, `base58check`, `base58`, `bech32`, `bech32m`, `read_input` | Tools | tools |
| bip85 | Derives a BIP 85 child (words, a WIF, an extended private key, hex or a password) from a held key. | `faraday_bip::bip85`: `derive`, `child_mnemonic`, `child_wif`, `child_xprv`, `child_hex`, `child_password_base64`, `child_password_base85`; `faraday_keys::bip85`: `child`; `faraday_keys::operations`: `Keys::bip85_child`; `faraday_bip::base64`: `encode`, `decode` | Bip85 | tools |
| vanity | Grinds a counter on a key's passphrase or account index until its first address begins with the characters chosen. | `faraday_bip::vanity`: `grind`, `grind_passphrase`, `grind_account`; `faraday_keys::vanity`: `grind`; `faraday_keys::operations`: `Keys::vanity_grind` | Vanity | passphrases |
| aezeed | Reads an LND aezeed: 24 words, checksum, scrypt key and AEZ decipher, giving the entropy, version and birthday. | `faraday_bip::aezeed`: `decode`, `decode_at_cost`, `decode_bytes`; `faraday_bip::aez`: `extract`, `hash`, `prf`, `encrypt`, `decrypt` | Lightning | tools |
| lightning-node | Derives a Lightning node key from words (ldk-node) or from an aezeed (LND). | `faraday_bip::aezeed`: `node_key`, `ldk_node_key`; `faraday_keys::lightning`: `node_from_words`, `node_from_aezeed`, `Node::private_hex`, `Keys::node_from_aezeed`; `faraday_keys::operations`: `Keys::lightning_node` | Lightning | tools |
| silent-receive | Derives a silent payments address, labels and scan key (BIP 352, 392) and finds the payments a transaction makes to them. | `faraday_bip::silent`: `Receiver::derive`, `Receiver::new`, `Receiver::scan_public_key`, `Receiver::spend_public_key`, `Receiver::address`, `Receiver::label_tweak`, `Receiver::label_point`, `Receiver::labelled_spend_key`, `Receiver::labelled_address`, `Receiver::scan_key_text`, `Receiver::descriptor`, `address_of`, `decode_address`, `encode_address`, `scan_path`, `spend_path`, `input_public_key`, `input_sum_and_hash`, `scan`, `find_payments`; `faraday_bip::silent_wallet`: `SilentWallet::address`, `SilentWallet::to_text`, `SilentWallet::parse`; `faraday_keys::silent`: `receiver`, `Receiver::scan_key`, `Receiver::address`, `Receiver::payments_in`, `Receiver::scan_public_key`, `Receiver::spend_public_key`; `faraday_keys::operations`: `Keys::silent_receiver`; `faraday_core::silent::check`: `Check::prevouts`, `Check::tx`, `Check::waiting_for` | Silent, Wallets | silent_payments |
| silent-send | Adds this signer's ECDH shares and BIP 374 proofs to a silent payments PSBT, and sets the output scripts the shares give. | `faraday_psbt::silent`: `add_shares`, `add_shares_for_keys`, `compute_scripts`, `fill_scripts`, `scan_keys`, `signable`, `tap_origin`, `add_input`; `faraday_bip::dleq`: `secret_mul`, `generate_proof`, `generate_proof_with`, `verify_proof`, `verify_proof_with`; `faraday_bip::silent`: `sender_outputs` | none | none |

### Signing

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| psbt-read | Reads a version 0 PSBT from bytes or base64 and writes it back as it was read. | `faraday_psbt::psbt`: `Psbt::parse_bytes`, `Psbt::parse_base64`, `Psbt::to_bytes`, `Psbt::to_base64`, `Psbt::unsigned_tx`, `Psbt::inner`, `Psbt::inner_mut`, `Psbt::into_inner`; `faraday_core::wallet`: `classify_read` | Spend, Files, Decode, Family | verifying |
| psbt-v2 | Reads a version 2 PSBT as its key-value maps, checks BIP 370's shape and converts it to version 0 and back. | `faraday_psbt::v2`: `PsbtV2::parse_bytes`, `PsbtV2::parse_base64`, `PsbtV2::to_bytes`, `PsbtV2::to_base64`, `PsbtV2::to_v0`, `PsbtV2::merge_signed`, `PsbtV2::unsigned_tx` | none | none |
| psbt-review | Reads what a PSBT spends and pays: amounts, fee, verified change, warnings, and the wallet it defines for registering. | `faraday_psbt::inspect`: `inspect`, `Inspection::dangers`, `Inspection::fee_share_pct`, `Inspection::holdable`, `cosigners`, `spend_route`, `wallet_keys_at`; `faraday_psbt::classify`: `amount_unverified`; `faraday_psbt::context`: `KeyRef::with_extra`, `Context::wallet_claim`, `KeyRef::from_master`, `KeyRef::derive_key`; `faraday_psbt::register`: `offer`, `register`; `faraday_core::register`: `path_text`; `faraday_core::memory`: `stated`; `faraday_core::wallet`: `psbt_network`, `Faraday::register_act`, `Spend::collect` | Spend, Family | verifying |
| sighash-review | Reads each input's sighash type and signs only SIGHASH_ALL and SIGHASH_DEFAULT. | `faraday_psbt::inspect`: `sighash_kind`, `SighashKind::is_default`, `SighashKind::is_none`; `faraday_psbt::inspect::sighash`: `SighashKind::is_single` | Spend, Family | verifying |
| sign-ecdsa | Signs a transaction input with ECDSA, from the key the PSBT's origin names, and checks the key against the script. | `faraday_psbt::sign`: `sign`, `sign_ecdsa`; `faraday_keys::psbt`: `sign`; `faraday_keys::session`: `Keys::sign` | Spend, Family | nonces |
| sign-schnorr | Signs a taproot input with BIP 340, by the key path or under a tap leaf's hash. | `faraday_psbt::sign`: `sign_taproot`, `sign_tap_leaf` | Spend, Family | nonces |
| sign-musig | Signs as one MuSig2 participant: draws a nonce, signs with it or signs last, and aggregates the partial signatures. | `faraday_psbt::sign`: `sign_musig`; `faraday_bip::musig`: `nonce_gen`, `sign`, `partial_sig_verify`, `partial_sig_agg`, `deterministic_sign`, `nonce_agg`, `PubNonce::from_bytes`, `AggNonce::from_bytes`; `faraday_psbt::musig`: `MusigSession::new`, `write_input_participants` | Spend | nonces |
| sign-check | Checks every signature a PSBT carries, finds a nonce used twice, recomputes this device's signatures and compares two PSBTs. | `faraday_psbt::verify`: `compare`, `deterministic`, `ecdsa_mode`, `schnorr_mode`, `verify_signatures`, `repeated_nonces`; `faraday_keys::psbt`: `recompute_signatures`, `key_refs`; `faraday_keys::session`: `Keys::key_refs`, `Keys::recompute_signatures` | Spend | nonces |
| finalize | Builds the final scriptSig and witness from the partial signatures and extracts the transaction. | `faraday_psbt::finalize`: `finalize`; `faraday_core::wallet`: `Spend::finished_hex` | Spend, Family | verifying |
| message-sign | Signs a message with the key at a path, in BIP 137's compact form or as BIP 322 simple. | `faraday_psbt::message`: `sign`, `signed_text`, `virtual_txids`; `faraday_keys::message`: `sign`; `faraday_keys::session`: `Keys::sign_message`; `faraday_core::wallet`: `Session::sign_message` | Message | message |
| message-verify | Checks a signed message against an address, as BIP 137 or BIP 322 simple. | `faraday_psbt::message`: `verify`; `faraday_core::wallet`: `Session::check_message` | CheckMessage | message |
| tx-decode | Reads a raw transaction, or a PSBT's transaction, to its txid, wtxid, weight and the previous outputs its source states. | `faraday_psbt::transaction`: `read_raw`, `wrap_raw`, `is_signed`, `Spending::read`, `Spending::from_psbt`, `Spending::tx`, `Spending::waiting_for`, `Spending::add_previous`, `Spending::prevouts`, `prevout_of`; `faraday_core::decode`: `decode`, `Faraday::amounts_in`, `prevouts_of`, `same_unsigned`, `script_kind`, `btc`; `faraday_qr`: `transaction_hex` | Decode | verifying |

### Transfer

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| qr | Encodes a payload as a QR code, ISO/IEC 18004 model 2, and reads QR codes from a camera frame or a PNG. | `faraday_codec::qr`: `encode`, `encode_bounded`; `faraday_codec::decode`: `decode_luma`, `decode_picture`; `faraday_codec::png`: `qr_png`, `grey_png`; `faraday_codec::classify`: `classify` | Transfer, Spend, Family, Decode, Create, Silent, VaultContents, Tools | air_gap |
| ur | Writes and reads UR codes: bytewords with CRC-32, fountain-coded parts and the CBOR around them. | `faraday_codec::ur`: `encode_single`, `encode_psbt`, `encode_bytes`, `decode_single`, `Encoder::psbt`, `Encoder::bytes`, `Encoder::next_part`, `Decoder::receive`; `faraday_codec::ur::bytewords`: `encode`, `decode`; `faraday_codec::ur::fountain`: `Part::from_cbor`, `Part::indexes`, `Encoder::new`, `Encoder::next_part`, `Decoder::receive`, `Decoder::message`; `faraday_codec::ur::xoshiro`: `Xoshiro256::from_bytes`; `faraday_qr::send`: `ur_psbt_frames`, `ur_bytes_frames`, `ur_bytes_animated`; `faraday_qr::registry`: `read`, `hdkey`, `account_cbor` | Transfer, Spend, Family, Decode, Create, Tools | air_gap |
| bbqr | Splits a file into BBQr or numbered pNofM parts for QR codes and puts received parts together. | `faraday_qr::send`: `frames`, `parts_frames`, `numbered`, `bbqr_frames`, `animated`; `faraday_qr`: `Assembler::feed` | Transfer, Spend, Family, Decode, Create, Tools | air_gap |
| envelope | Puts a file in the faraday-file-v1 envelope with its SHA-256 and checks one received, saving a file received once. | `faraday_qr::envelope`: `pack`, `unpack`; `faraday_qr`: `classify_text` | Transfer, Files | air_gap |
| file-hash | Computes the SHA-256 of a whole file to show and keep. | `faraday_core::receipt`: `sha256_of`; `faraday_core::file_menu`: `sha256`; `faraday_core::gpg`: `sha256`, `sha256_hex`; `faraday_core`: `Item::hash` | Files, Visit | air_gap |

### Vaults

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| vault-create | Makes a vault file of four equal slots from one to four passphrases, with Argon2id, HKDF-SHA256 and XChaCha20-Poly1305, and a ChaCha20 keystream for the salt, the nonces and the unused slots. | `faraday_vault`: `create`, `derive`; `faraday_keys::vault`: `create`, `measure` | CreateVault | faraday:vaults |
| vault-unlock | Opens the slot a passphrase opens: checks the header, derives the slot key once with Argon2id and HKDF-SHA256, tries XChaCha20-Poly1305 on all four slots and parses the one whose tag verifies. | `faraday_vault`: `open`, `open_with`; `faraday_keys::vault`: `OpenVaults::unlock` | Unlock | faraday:vaults |
| vault-seal | Writes an open vault back: encrypts the open slot's records with XChaCha20-Poly1305 under a new 24-byte nonce and copies the other three slots unchanged. | `faraday_vault`: `Opened::seal` | VaultContents, BackupFlow | faraday:vaults |
| vault-reveal | Puts a secret of an open vault's record in the clear for the one reason named: a key's words or passphrase, an entry's password, TOTP secret or notes, a note's text. | `faraday_keys::vault`: `OpenVaults::reveal`, `OpenVaults::words_of` | VaultContents | faraday:vaults |
| vault-edit | Adds, changes, copies and makes public the records of an open vault, holding every secret field in a value that wipes on drop. | `faraday_keys::vault`: `OpenVaults::insert`, `OpenVaults::insert_at`, `OpenVaults::set_public_all`, `OpenVaults::set_public_all_in`, `OpenVaults::set_fields`, `OpenVaults::copy_record`, `OpenVaults::copy_out`, `OpenVaults::stage`, `OpenVaults::stage_passphrase` | VaultContents, BackupFlow | faraday:vaults |
| vault-load | Loads a key kept in an open vault into the session: its words (BIP 39) and passphrase to the seed and the master key's fingerprint (BIP 32), or its master seed, or its codex32 set. | `faraday_keys::vault`: `OpenVaults::load_from_vault`, `OpenVaults::load_key_record`, `OpenVaults::load_key_with_passphrase`, `OpenVaults::passphrase_kept_for`, `OpenVaults::warm`, `OpenVaults::word_count`, `OpenVaults::words_id`, `OpenVaults::record_word_count`, `OpenVaults::record_words_id`, `OpenVaults::same_words`, `OpenVaults::holds_session_words`, `key_identity`, `key_account`; `faraday_keys::session`: `Keys::key_record`, `Keys::seed_record_id` | VaultContents, Unlock | faraday:vaults |
| kdbx | Writes a KeePass KDBX 4.0 database of one to eight entries under a passphrase: Argon2id, SHA-256 and SHA-512 key steps, ChaCha20 over the body, HMAC-SHA-256 block authentication. | `faraday_backup::kdbx`: `write`, `verify`, `keys`, `header`, `hmac`, `chacha20`; `faraday_backup`: `argon2id`; `faraday_keys::vault`: `OpenVaults::kdbx_entry`; `faraday_core::vault`: `Faraday::vault_kdbx` | VaultContents | faraday:vaults |
| totp | Computes a TOTP code from an entry's secret at a Unix time: HMAC-SHA1, SHA-256 or SHA-512 over the time step, then dynamic truncation. | `faraday_vault::totp`: `Totp::code`; `faraday_keys::vault`: `OpenVaults::totp` | Codes | faraday:time |

### GPG and Secure Boot

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| gpg-create | Makes an OpenPGP v4 key from two fresh Ed25519 seeds, a certifying primary key and a signing subkey, with its public certificate and a kept revocation certificate. | `faraday_keys::gpg`: `create`; `faraday_pgp`: `Key::certificate`, `Key::fingerprint`, `Key::subkey_fingerprint`; `faraday_keys::vault`: `gpg_facts` | VaultContents | faraday:gpg |
| gpg-sign | Signs data with the GPG key's signing subkey: an Ed25519 detached signature over the SHA-256 or SHA-512 of the data, armoured. | `faraday_pgp`: `Key::sign`; `faraday_keys::vault`: `OpenVaults::gpg_sign_armored` | VaultContents, Files | faraday:gpg |
| gpg-revoke | Makes a revocation certificate for the GPG key and writes it as the `.rev` file GnuPG writes. | `faraday_pgp`: `Key::revocation`, `Key::revocation_file`; `faraday_keys::vault`: `OpenVaults::gpg_revocation_file`, `OpenVaults::gpg_new_revocation_file` | VaultContents | faraday:gpg |
| gpg-paperkey | Writes the GPG key's secret parts in paperkey's text form, for a person to copy onto paper. | `faraday_pgp`: `Key::paperkey`, `Key::secret_key`; `faraday_keys::vault`: `OpenVaults::gpg_paperkey` | VaultContents | faraday:gpg |
| gpg-restore | Restores a GPG key from its paperkey lines and its public certificate: checks every line's CRC, rebuilds the key from the seeds, and makes every signature of the certificate again to compare. | `faraday_pgp::certificate`: `restore`, `Certificate::read`; `faraday_keys::gpg`: `Restore::line`, `Restore::again`, `Restore::wipe`, `Restore::finish` | none | faraday:gpg |
| gpg-verify | Checks a detached OpenPGP signature over data against a certificate: the signing subkey's binding and back signature, the key's life, and the Ed25519 signature over a SHA-256 or SHA-512 digest. | `faraday_pgp::verify`: `verify_detached` | Files, VaultContents | faraday:gpg |
| sb-keys | Makes an owner's Secure Boot keys, PK, KEK and db as RSA-2048 with self-signed certificates, and the enrolment files: signature lists, PKCS #7 signed updates and certificates. | `faraday_sb`: `make`, `enrolment` | VaultContents | faraday:secure-boot-keys |
| sb-sign | Signs a boot program with the db key, as an Authenticode SHA-256 signature, only when it is the file a release's checked sums name. | `faraday_sb::release`: `sign`; `faraday_sb::pe`: `sign`, `hash_as_signed` | VaultContents | faraday:secure-boot-keys |
| sb-check | Checks an image's Authenticode signature against a db certificate: the image hash, the SHA-256 digest in the signed data and the RSA signature. | `faraday_sb::pe`: `check` | VaultContents | faraday:secure-boot-keys |

### The device and the Starter

| Id | Action | Roots | Screens | Learn |
|---|---|---|---|---|
| selftest | Runs eight fixed-vector checks at start-up and from Settings: BIP 39 words and seed in English and Japanese, BIP 32 derivation, a BIP 84 address, an ECDSA and a BIP 340 signature, scrypt and Argon2id. | `faraday_selftest`: `run`, `run_checks`, `bip39_vector`, `bip39_japanese_vector`, `bip32_vector`, `bip84_address`, `ecdsa_vector`, `schnorr_vector`, `scrypt_vector`, `argon2id_vector` | Settings | faraday:keys-kept |
| starter-write | Writes an image to a stick and checks it: the SHA-256 of the payload file against its manifest before any write, and of the decompressed bytes against the manifest as they are written. | `faraday_payload::decode`: `write_payload`, `write_image`, `hash_image` | none | faraday:checking-written |
| starter-signed | Checks a boot file the person signed against the release's own: its Authenticode image hash, its one signature at the file's end, and the signature on the file built. | `faraday_sb::pe`: `transplant`, `signer` | none | faraday:checking-written |
| starter-recovery-key | Reads a BitLocker recovery key, eight groups of six digits, for the system drive, holds it until the notes sheet is made and wipes it. | `faraday_starter::api`: `RecoveryKey::parse`; `faraday_starter::notes`: `Notes::pdf_with_key`, `Notes::sheet`; `faraday_starter`: `Starter::print_notes` | none | faraday:checking-written |

92 actions: 11 keys, 11 wallets, 12 randomness, 11 backups, 9 derived
keys and tools, 12 signing, 5 transfer, 8 vaults, 9 GPG and Secure
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
   **Cryptography** beside the ?, in the ⋮ first on the Pi (§8).
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

10. **The name on screen and the website's page.** Settled (owner,
    2026-10-28): the bar's button keeps `Icon::Method` (`fact_check`),
    and its tooltip, its ⋮ row, the Learn tab and the Starter's sheet
    read "Cryptography"; the website shows every block on one page,
    /cryptography (§11.1).

## 17. What this does not catch

- A function defined inside a `macro_rules!` body is not read: none is
  defined there in a crate with a cryptographic function as of
  2026-10-28. A function nested in another function's body is hashed as part of the outer one and
  not listed on its own.
- Trailing commas are normalised; other token changes rustfmt makes
  (parentheses it adds or removes) change the hash.
- A method call in `faraday-core` on a value whose type the file never
  names is not a call site (§9.2).
- In a closure (§4.4), a method call on a value whose type the calling file
  never names, to a function that is not cryptographic, is not an edge,
  and a call to a method named like a standard-library one (`collect`,
  `push`, `get`, `update`, …) on a type the file only names is not one
  either, unless the callee is cryptographic; the function it would have
  reached is then not in that action's fingerprint through that call.
  Where it matters, the called function is a root of the action, or the
  caller is changed to name the type.
- A `pub use` glob of a primitive crate (`pub use bitcoin::*`) and a
  `pub use` of a name a macro generates are not followed (§4.2 rule 3);
  none is in the tree as of 2026-10-28.
- A function generated by a macro, or reached only through a trait
  object or a function pointer, is invisible to the walker; the
  primitive it calls is still found where the macro's expansion is
  written in a file of the tree.
- A call resolved as §4.4 says may cover a function in the wrong action's
  closure (a free function of a module the path does not pin down, a glob
  import); its code is still fingerprinted, under that action.
- Constant-time claims are copied from the library or the code's own
  documentation; nothing here measures timing.
- The places named for review (§3.3) are listed, not read: a stale
  sentence in SECURITY-MODEL is caught by the reviewer the recipe
  prompts, not by the lint.
- A library's behaviour changing without its version changing (a
  vendored patch) is caught by `cargo vet` and the cross-check stamp,
  not by a fingerprint.

## 18. Constructions: how a format is built

(owner, 2026-10-28; F159.) The vault, and how strong a passphrase is,
need an account of their own: the header field by field, the keys, the
slots, opening and sealing, what the format promises and what it does
not. No block holds that. This section adds it to the manifest as a
**construction**, drawn in Learn under the vault's page and on
/cryptography beside the vault's actions.

### 18.1 Why a construction, and not a field of a block

A block is facts about one action, in nine fixed fields (§1, F148). The
vault's account is about the format several actions share (vault-create,
vault-unlock, vault-seal), and much of it is why: why a changed header
cannot expose a slot, what two copies show, what the format does not
promise. That is explanation. CLAUDE.md keeps explanation off working
screens and puts it in Learn and About, which are not working screens. A
block is drawn in Learn, but its rule is facts only, so the account does
not go into a block's field. Three forms were weighed:

- **A field on the vault actions** (`action.account`): three actions
  would carry one text, or one would carry it for the others; a tenth
  field breaks §1's fixed nine and the block's drawing (§8.5).
- **A Learn page of Faraday's own** (`docs/learn/faraday/`): its prose is
  not in the manifest, so the website's /cryptography page would not
  render it from the pin, and no lint would hold its numbers to the code.
- **A manifest table, `[[construction]]`** (chosen): one text, in the
  one source the app's blocks and the website already read, naming the
  actions it belongs to, with its numbers and names held to the code by
  `check.py` (§18.6). It is still Learn's: it is drawn after a Learn
  page's own sections, never on a working screen.

### 18.2 Its form in the manifest

```toml
schema = 2                      # constructions are new in schema 2; the website stops on a schema it does not know

[[construction]]
id = "vault"
title = "How the vault is built"
learn = "faraday:vaults"        # the Learn page it is drawn after
actions = ["vault-create", "vault-unlock", "vault-seal"]   # /cryptography shows it before their blocks

[[construction.section]]
heading = "The header"

[[construction.section.para]]
text = "The header is the first {c:faraday/faraday-vault/src/lib.rs::HEADER_LEN} bytes. Nothing in it is secret, and none of it is a key."

[[construction.section.para]]
text = "Today the magic is OFVT."
until = "magic"                 # dropped when that Open item is built

[[construction.section.para]]
text = "The magic is FVLT."
planned = "magic"               # an item of docs/VAULT-HARDENING.md's Open line (§18.6)

[[construction.section.para]]
table = "vault-header"          # a table gen.py writes from the code (§18.6), in its place among the paragraphs

[open_item.magic]               # an item of a Spec's Open line, by the words that line names it with
spec = "docs/VAULT-HARDENING.md"
names = "the `.fvlt` extension and the magic `FVLT`"

[[name_with_digits]]
name = "Argon2id"
reason = "The function's name (RFC 9106)."
```

- **`para.text`**: plain statements. A number is never typed: it is a
  placeholder that gen.py fills when it generates: `{c:<file>::<CONST>}`
  for a constant (an integer expression of literals and other constants
  of that file, as `HEADER_LEN` is), `{len:<file>::<STATIC>}` for an
  array's length, `{f:<expr>}` for arithmetic over such placeholders
  (`log2` of a list's length, rounded to a tenth), and `|words` after any
  of them for a count written as a word ("four slots"). A byte string
  (a magic, a label) is `{b:<file>::<CONST>}`. A literal the code
  states inside a function (Argon2id's version, its output length, the
  lanes and the fewest passes `check_cost` takes) is `{v:<name>}`, read
  from that function's body by gen.py's `CODE_VALUES`, which fails when
  the body no longer has that shape. A function or a type is
  `{fn:<file>::<name>}`. A standard is named by its key in the
  standards table ("RFC 5869"), whose digits are its name; another name
  that holds digits ("Argon2id", a U+ code point) is in
  `[[name_with_digits]]` with its reason, as a `name` or a `pattern`.
  A paragraph's text holds no backticks.
- **`para.planned`** names an item of a Spec's Open line that the
  paragraph describes as not built, by its key in `[open_item]`, whose
  `names` are the words the Open line of `spec` names it with. The
  generator draws the paragraph led by "Planned, not built:", so its
  text begins in lower case unless its first word is a name; a number of
  that work, which no constant holds yet, is `{planned:<n>}`, allowed only
  there. **`para.until`** marks a paragraph about today's behaviour that
  the item replaces. The batch that builds the item drops its `until`
  paragraphs and its paragraphs' `planned` keys, rewording them as built,
  and the `[open_item]`.
- **`para.table`**: in place of `text`, one of the tables gen.py writes
  from the code, drawn where the paragraph stands: `vault-header` (the
  header's fields, from `Header::to_bytes`), `vault-sizes` (each of
  `SLOT_SIZES` with its file size, by `file_len`'s formula over the
  constants), `diceware-bits` (with `words = [4, 10]`: each list of
  `core/faraday-bip/src/diceware/`, its length read from its `WORDS`
  array, and the bits for each count of words). `vault-slot` (a slot's
  fields) and `vault-plain` (a slot's plaintext) are not written yet: the
  account tells a slot in its paragraphs. A table typed by hand is not
  allowed.
- **In faraday-learn:** gen.py writes `CONSTRUCTIONS` and
  `docs/crypto/gen_how.py` puts them after `BLOCKS` in
  `faraday/faraday-learn/src/how.rs`, each with its title, its page and its sections as headings,
  paragraphs and table rows (the type is
  `faraday/faraday-learn/src/construction.rs`), and the lookups
  `constructions_for_page(page)` and, by the page itself,
  `faraday_learn::constructions_after`. A Learn page draws its own
  sections, then each construction for it as more sections under the
  construction's title (`faraday_ui::how::construction`), then its
  blocks under **How Faraday does it** (§3.1). The drawing is a Learn
  page's (the title as the page's, headings and paragraphs in the page's
  type), not a block's list; a table is Material's data table: heads in
  Title S, cells in Body M, a divider under each row, columns sharing the
  width where they do not fit. Each part's height is kept for the width
  and text size it was drawn at, as a block's is, so a part wholly
  outside the view costs a lookup when the frame is drawn again.

### 18.3 The vault: how it is built

The account as it ships, by section. A sentence marked **[M]**,
**[P]**, **[C]**, **[S]** or **[N]** changes when that item lands: the
file's name and magic, the passphrase in Form C, the commitment
block and the seal count of `docs/VAULT-HARDENING.md` §2 to §5, and the
fresh seal nonce (`docs/OWN-RANDOMNESS.md`, F154). "Planned, not built"
sentences read as here until then; "Today" sentences are `until`
paragraphs, dropped when their item lands. The numbers are what the
placeholders give today.

The fresh seal nonce ([N]) had landed when the account was built
(`Faraday::vault_seal_nonce` takes a fresh answer for "vault seal" when one is
waiting, `docs/VAULT.md` §5), and no Open line names it, so the
manifest writes its sentences as built: the nonce is the first 24 bytes
of a fresh answer when one is waiting, else of SHA-256 over the label,
the seed and the count; and two seals share a nonce only when no fresh
answer was waiting at either and the system gave both sessions the same
seed. The sentences marked [N] below are as specified before it landed.

**The file**

A vault is one file: a header of 53 bytes, then four slots of *L* + 40
bytes each, 53 + 4 × (*L* + 40) bytes in all. *L* is a slot's plaintext
length, chosen when the vault is made and fixed for its life:

| *L* | File |
|---|---|
| 65 536 | 262 357 |
| 262 144 | 1 048 789 |
| 1 048 576 | 4 194 517 |
| 4 194 304 | 16 777 429 |

Every vault has four slots, whether it has one passphrase or four.

- The file's name ends in `.fvlt`.

**The header**

The header is the first 53 bytes. Its integers are little-endian.
Nothing in it is secret, and none of it is a key.

| At | Bytes | What |
|---|---|---|
| 0 | 4 | magic |
| 4 | 1 | version, 1 |
| 5 | 4 | Argon2id memory, in KiB |
| 9 | 4 | Argon2id passes |
| 13 | 4 | Argon2id lanes |
| 17 | 4 | *L* |
| 21 | 32 | Argon2id salt |

- The magic is `FVLT`.

**The keys**

The passphrase is stretched by Argon2id version 1.3 under the header's
salt, memory, passes and lanes, to 32 bytes. HKDF-SHA256 (RFC 5869),
with an empty salt and the label "Faraday vault v1 slot", makes the slot
key from them. One passphrase gives one slot key, which is tried on
every slot.

- The passphrase is first put in Unicode Normalization Form C, and
  Argon2id takes that form's UTF-8 bytes. Then "é" typed as one
  character (U+00E9) and as "e" with a combining accent (U+0065 U+0301)
  open the same slot, and
  two passphrases with one Form C are refused as the same when a vault
  is made. Nothing else is changed: case, spaces and every character are
  kept as typed, and Form C keeps full-width letters and ligatures as
  they are: "Ａ" is not "A".

**The slots**

Each slot is a 24-byte nonce, *L* bytes of ciphertext and a 16-byte
Poly1305 tag, sealed with XChaCha20-Poly1305
(draft-irtf-cfrg-xchacha-03). Its associated data is the whole header
and the slot's number, 0 to 3: 54 bytes. A slot's plaintext is a format
byte, the slot's records, an end record, and zeros to *L*.

- The plaintext starts with 32 zero bytes, before the format byte. *L*
  and the file's size do not change; the room for records is 32 bytes
  less.
- Every slot holds one seal count record: how many times the slot has
  been written, and when it was last written by the clock of the
  computer that wrote it.

A slot no passphrase opens is a nonce and *L* + 16 bytes, drawn when the
vault is made from a ChaCha20 keystream keyed by HKDF-SHA256 of the
computer's randomness: random-looking bytes the length of a used slot.
The salt, the order in which the passphrases take the slots, and each
used slot's first nonce come from the same keystream.

**Opening**

The reader checks the magic, the version, *L* against the four sizes,
the file's length against *L*, and the cost against its limits: memory
8 MiB to 4 GiB, passes 1 to 10, lanes exactly 1. A file outside them is
refused before any memory is taken. The memory a vault needs is in its
header, and Unlock shows it before the passphrase is typed. Faraday makes
vaults with at least 64 MiB and 2 passes.

The slot key is derived once and tried on all four slots. One tag
verifies: that slot opens. None: the passphrase opens nothing. More than
one: the file is refused, since Faraday never makes two slots under one
key. Every slot is tried whichever opens, so the time taken does not
show which slot a passphrase opens. A passphrase that opens none
decrypts and reads no slot, which the result shows anyway. The opened
slot's records are read, and any fault refuses the slot.

- A slot whose tag verifies and whose first 32 bytes are not zero
  refuses the file.

**Sealing**

Sealing writes the open slot again under the same slot key with a new
nonce, and copies the other three slots byte for byte. A session cannot
tell an unused slot from one another passphrase opens, so it never
rewrites one. The sealed vault is written to its stick as a new file,
read back and compared byte for byte, and then takes the old copy's
place.

- **[N]** Today the nonce is the first 24 bytes of SHA-256 over a label,
  the session's seed and the count of seals this session has made.
  Planned, not built: each seal's nonce is drawn fresh from the system's
  randomness; the seed and count are used only when no fresh answer is
  available.
- Each seal writes the slot's seal count, one more than the last, and
  the time.

**What a changed header can do**

Changing the cost or the salt gives another key, so every slot fails.
Changing *L*, the magic or the version fails the reader's checks, the
length check or every tag. A slot moved to another position fails: its
number is in its associated data. A slot moved to another vault fails:
another salt gives another key, and another header is other associated
data. The most a changed header can do is stop the vault opening, or,
within the limits, make it slow or need more memory than a computer has.

**What it promises**

Without a passphrase, one copy of a vault shows four random-looking
slots of equal size. It does not show how many passphrases exist or
which slots are in use. Nothing in a slot can be read without its
passphrase, and a changed byte makes its tag fail, so nothing in it can
be changed unnoticed.

**What it does not promise**

- **Offline guessing.** Anyone with the file has its salt and cost and
  can try passphrases on their own computers. Each guess costs one
  Argon2id at the vault's cost and tests all four slots. A slot is as
  strong as its passphrase (Passphrase strength, §18.4).
- **Rollback.** An older copy of a vault still opens. Someone who can
  write to the stick can put an older copy back; they cannot read or
  change anything in it.
  - **[S]** Built: Unlock shows a slot's seal count. An older copy is
    named when a newer one was opened, sealed or sat among the copies in
    Files since power-on, or when an open vault's backup plan records a
    newer one; the vault still opens. A stick visit reads no counts: a
    process that holds a slot key is never handed a stick. A rollback is
    not seen when no newer copy is opened or read between power-on and
    the older copy's unlock and no open vault's plan records one, or
    when a vault and its plan are rolled back together.
  - **[S]** Built: a backup plan records, for each other vault it
    places, that vault's salt, size, slot position, seal count and a
    digest of the slot. Whoever opens the slot holding the plan learns
    those vaults' salts and sizes, which slot of each holds the plan's
    seeds, and how often it was sealed.
- **The seal's nonce.** **[N]** Today the nonce comes from the session's
  seed and a count. A computer started again from a saved state, a
  virtual machine's snapshot for instance, can draw the same seed again;
  two seals under one key with one nonce show much of the slot to
  whoever has both copies, and let them forge it. Planned, not built: a
  fresh nonce from the system at every seal.
- **Two copies from different times.** A slot's bytes change only when
  it is sealed. Someone with two copies sees which slots changed, and so
  which are in use. That weakens deniability, not secrecy.
- **Availability.** A slot is checked only when it is opened. A changed
  byte in a slot no one opened is not noticed until its passphrase is
  next used, and then that slot is lost. A damaged header loses all four
  slots. A copy at another place is the remedy.
- **Key commitment.** XChaCha20-Poly1305 does not commit to its key: a
  slot can be made whose tag verifies under two passphrases, showing
  other contents under each. Making one needs both passphrases.
  - The 32 zero bytes at the start of every slot's plaintext: under a
    second key they decrypt to other bytes, and the file is refused.
- **The seal count is read by whoever opens the slot.** A decoy slot sealed twice beside one sealed daily says it
  is little used. Opening and sealing the decoy raises its count.

### 18.4 Passphrase strength

A second construction, `id = "passphrase-strength"`, on the same Learn
page (`learn = "faraday:vaults"`, which Create a vault's Learn opens) and
on /cryptography beside vault-create. It replaces the "Passphrase
strength" section of `docs/learn/faraday/vaults.md`, whose facts it
keeps. As it ships:

**Words from dice**

A passphrase of words drawn by dice from a list carries log2 of the
list's length in bits for every word. Faraday has the three lists the
EFF publishes:

| Words | EFF large list: 7 776 words, 12.9 bits a word | EFF short list 1: 1 296 words, 10.3 bits a word | EFF short list 2: 1 296 words, 10.3 bits a word |
|---|---|---|---|
| 4 | 51.7 | 41.4 | 41.4 |
| 5 | 64.6 | 51.7 | 51.7 |
| 6 | 77.5 | 62.0 | 62.0 |
| 7 | 90.5 | 72.4 | 72.4 |
| 8 | 103.4 | 82.7 | 82.7 |
| 9 | 116.3 | 93.1 | 93.1 |
| 10 | 129.2 | 103.4 | 103.4 |

The bits hold when every word is drawn by fair dice or by the computer's
randomness, and not when a person chooses a word.

**What Argon2id adds**

Each guess costs one Argon2id at the vault's memory and passes, on the
guesser's own hardware. Faraday states that work as log2(memory in KiB ×
passes) bits, which Create a vault shows beside each cost. The memory of
the vaults Faraday makes ranges over a factor of 64, which is 6 bits;
one more word from the large list is 12.9. The figure counts work, not
how much more memory costs a guesser than time, and it gives no time per
guess.

**Other passphrases**

A person may use any passphrase. Its strength is then theirs to ensure:
Faraday does not measure a typed passphrase.

**Where to learn more**

- The EFF's page on dice-generated passphrases, eff.org/dice.
- Arnold Reinhold's Diceware page, where the method began,
  theworld.com/~reinhold/diceware.html.
- NIST Special Publication 800-63B, Digital Identity Guidelines, on what
  makes a password strong.

The website links them; the device names them as text.

### 18.5 Where it is drawn

- **In the app:** after the Learn page `faraday:vaults`' own sections,
  which Vaults, Create a vault, Unlock, the vault's contents and the
  backup flow open (`faraday/faraday-core/src/learn.rs`,
  `Faraday::screen_pages`): "How the vault is built", then "Passphrase
  strength", then the page's blocks.
- **On /cryptography:** each construction at its own anchor
  (`/cryptography#vault`), before the first of its actions' blocks, in
  §12's group of its actions (Vaults), and in the index (§18.7).

### 18.6 What check.py holds

`docs/crypto/check.py check` fails when:

- a placeholder does not resolve: its file does not exist, or holds no
  constant, static or function of that name, or the constant is not an
  integer expression gen.py can evaluate;
- a paragraph's text holds a digit outside a placeholder, a standard's
  key, or a name in the manifest's `[[name_with_digits]]` list
  (Argon2id, HKDF-SHA256, SHA-256, XChaCha20-Poly1305, ChaCha20,
  Poly1305, UTF-8, U+ code points, log2, NIST Special Publication
  800-63B, each with its reason; the label "Faraday vault v1 slot" is
  `SLOT_INFO`'s bytes, a placeholder), so no number is typed;
- a `planned` or `until` names no `[open_item]`, or an item whose
  Spec's Open line no longer names it: the batch that builds an item,
  which removes it from the Open line, must rewrite its paragraphs in the
  same change; an `[open_item]` no paragraph names fails too;
- a `{planned:<n>}` stands outside a planned paragraph;
- a construction names an action that does not exist, a Learn page that
  does not exist (`learn_pages`, as for actions), or a table kind gen.py
  does not write;
- a code name in backticks inside a paragraph is not a placeholder.

`docs/crypto/gen.py` reads what the tables and placeholders name:

- `vault-header`: the offsets from `Header::to_bytes`' slices, failing
  when the code no longer has that shape; and the sizes and offsets
  `tools/vault/open.py` reads (`HEADER_LEN`, `NONCE_LEN`, `TAG_LEN`,
  `SLOT_SIZES`, the salt's slice, the four integers), which
  `faraday/faraday-vault/tests/open_py.rs` holds to the vectors;
- `vault-sizes`: `SLOT_SIZES`, `HEADER_LEN`, `SLOTS`, `NONCE_LEN` and
  `TAG_LEN`, by `file_len`'s formula; the vectors' lengths are held by
  `faraday/faraday-vault/tests/vectors.rs`;
- the cost limits: `MIN_MEMORY_KIB`, `MAX_MEMORY_KIB`, `MAX_PASSES`,
  `CREATE_MIN_MEMORY_KIB`, `CREATE_MIN_PASSES`, and lanes' 1 from
  `check_cost`;
- the label: `SLOT_INFO`'s bytes;
- `diceware-bits`: each list's `WORDS` length, so a list changed in
  `tools/diceware/gen.py`'s output changes the table.

A construction is fingerprinted as an action is (§5), over the code its
placeholders name: its lock lines in `docs/crypto/actions.lock` are
each value it reads (`val c:<file>::<CONST> 53`) and a digest of each
table, and `just crypto-review <id>` stamps it in `[construction.review]`
with no inventory. A change to a value the account states fails check
until the account is reviewed and re-stamped with the vault actions.
check.py is not yet among `just lint`'s checks; `gen.py check` holds
`how.rs`'s `CONSTRUCTIONS` to the manifest.

### 18.7 The website's /cryptography page

What changes in §11.1:

- **The index** lists the constructions above the standards, each a link
  to its anchor.
- **The actions:** each construction is drawn before the first block of
  the actions it names, in their group, with its headings, paragraphs
  and tables, a planned paragraph led by "Planned, not built:", as in
  the app.
- **Filters:** a construction shows whenever any of its actions shows; a
  filter "Planned, not built" shows the constructions with a planned
  paragraph.
- **Links:** "Where to learn more" links eff.org/dice,
  theworld.com/~reinhold/diceware.html and NIST SP 800-63B's page; the
  account's `fn` placeholders link to the file at the release's tag, as
  the Code field does.
- **The schema:** the website reads `schema = 2`; a build of it that
  knows only 1 stops, as §11 has it.

### 18.8 Order of work

One batch (Sonnet, before or after VAULT-HARDENING's batch 0, its
sentences marked): the schema of §18.2 in check.py (`TOP_KEYS` gains
`construction` and `name_with_digits`; `schema = 2`; `--self-test`
covers a placeholder that does not resolve, a typed digit, a planned
item no Open line names), gen.py's tables and placeholders, the two
constructions in docs/crypto/actions.toml with §18.3's and §18.4's text,
`CONSTRUCTIONS` in `faraday/faraday-learn/src/how.rs`, the Learn page's
drawing of them, and `docs/learn/faraday/vaults.md`'s "Passphrase
strength" section removed. Each later VAULT-HARDENING batch rewrites its
planned paragraphs (§18.6).
