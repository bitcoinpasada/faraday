# toml_parser

**Purpose.** Lexes and parses the TOML inside a vault (`docs/VAULT.md` §7)
into events, and decodes keys and scalars (strings with their escapes,
integers, booleans, date-times). `faraday-vault`'s `contents` module builds
the tree from the events and holds the rules TOML puts across keys.

**Why this crate.** It is the parser inside `toml_edit` and `toml`, by the
same publisher, and the one of the family that builds without `std`
(`default-features = false`, `alloc` on): `toml_edit` needs `std` through
`indexmap`, and `toml` needs `serde`, which would copy every secret string
through a derive. It forbids `unsafe` unless its `unsafe` feature is on, which it is not here.

**Cost.** One crate and `winnow` (`default-features = false`, the same
publisher), no `std`. The parser reads TOML
1.1, a superset of 1.0.0; `contents` refuses the three things 1.1 adds
(a line break or a trailing comma in an inline table, the `\x` and `\e`
escapes), so what the vault accepts is 1.0.0. The parser does not check a
date-time: it comes back as the text written, so `contents` checks the
one it reads (`sealed`) itself, in ASCII, with seconds, an offset in range
and a year of four digits. It is checked against toml-test's TOML v1.0.0
cases (`tools/vectors/toml/`).

**Secrets.** The parser decodes a string into a `Cow<str>`: borrowed from the
text where it has no escape, owned where it has. `contents` moves each into
a buffer that is wiped when it drops. Not wiped: the owned buffer of a string
with escapes, while it grows (a reallocation leaves the old copy), the token
list and the events (spans only, no text), and the error value (a description
and spans, no text). The plaintext the parser reads is the caller's, wiped by
the caller. Wiping these is later work under the Security scope
(`CLAUDE.md`), not done now.

**Reopen when** `toml_parser` stops building without `std`, or a TOML 1.0-only
mode appears (the three refusals in `contents/tree.rs` then go).

**Review.** Trusted by publisher (Ed Page, `epage`) under F71
(`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`
(`toml_parser` and `winnow`). Not read line by line here.
