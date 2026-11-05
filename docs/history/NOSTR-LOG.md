# The Nostr key: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/NOSTR.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/NOSTR.md)

Spec v0.2 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F27; §12 records the answers). Not built, and not
scheduled: it is built only when the owner says so.

## §0 Rules for the implementing agent (from docs/NOSTR.md)

- `CLAUDE.md` applies in full: every dimension a
  token; labels, not prose, on working screens (the explanations are
  §9's Learn page); tests for user-facing behaviour only; `just` once at
  the end of the batch; no `rm`; no commits; no agents.
- Nothing in `core/` changes: the BIP-85 application list there stays
  as it is.
- On screen the nsec is handled as BIP-85's WIF and extended private
  key are today (§4.4). Unlike them it has no **Out unprotected**: the
  ncryptsec (§6) is the only form in which it leaves the device, apart
  from the sealed vault file itself. A change that would need it
  handled otherwise stops and asks the owner.
- Nothing that can leave the device carries the key's fingerprint, or
  an identity or account number: no file name, no label under a code,
  no file's content. Files are named by the npub's first characters
  (§2.5). The fingerprint stays inside the vault (§5).
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## §12 Decided by the owner (2026-10-14), items 7 and 10 (from docs/NOSTR.md)

7. Superseded by 13.
10. Superseded: the Companion is gone (F34).

## §11 Batch (from docs/NOSTR.md)

The labels are the ones decisions and old commits use.

- **N1, the Nostr key.** §2's `nostr.rs` files, in `faraday-core` and
  `faraday-intake`; §3's application and tile; §4's cards, Find a pair
  and the Value card; §5's vault entry, §5.1's page actions and Path
  row, §5.2 and §5.3; §6's encrypted export, its prompt and the file's
  exposure; §6.6's file kind; §7's rules for Faraday Demo and the
  demo; §8's strings; §9's Learn page; §10's vectors and tests; the
  Nostr rows of `docs/QR.md` (§7's table) brought to these file names.
  Renders: the tile, the Identity and account card, Find a pair, the
  Value card, the export prompt and a saved entry's page at `1280x800`
  and `480x640`. It touches the device app.
