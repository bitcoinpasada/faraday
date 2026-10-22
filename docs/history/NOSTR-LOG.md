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
