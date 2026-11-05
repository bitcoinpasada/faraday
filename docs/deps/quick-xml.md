# quick-xml

**Cryptography:** none

**Purpose.** The tokenizer under the XML document of a KDBX 4 file
(`faraday/faraday-vault/src/kdbx/xml`, `docs/VAULT.md` §7, §7.1). It
splits the document into start tags, end tags, text, references and
comments, and refuses a document that is not well formed (a mismatched end
tag, an unclosed element, a stray `&`). Everything else is Faraday's own:
which references and declarations are refused (a processing instruction is carried),
the nesting cap, and the model that keeps what it does not read as the bytes
it came in.

**Why this crate.** It is already in the graph (`wayland-scanner` reads
protocol files with it), at 0.41, so taking the same version adds no crate.
It reads from a byte slice with no allocation per event, reports the byte
offset of each event (which is how carried bytes are kept exactly), and
passes an entity reference as its own event instead of expanding it, so the
reader sees every reference and refuses any but XML's five and character
references; it does not read DTDs or define entities.

**Cost.** One edge, `quick-xml` 0.41 with `default-features = false`: no
`serialize`, `encoding` or async features, so no `serde`, `encoding_rs` or
`tokio`. The crate uses `std`; `faraday-vault` is `no_std` and links it on
the targets Faraday builds for, all of which have `std`.

**Reopen when** the document must be read where `std` is missing, or the
crate changes how it reports references or offsets.

**Review.** An exemption, not reviewed by a human (`supply-chain/config.toml`, F71); not read line by line here. It is a `[[not_established]]` library of the cryptography manifest by F213.
