# How Faraday does it: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/CRYPTO-LEARN.md` on 2026-11-05. Each keeps its
original number. The labels in them (batch 1 to 9) are the ones decisions
and old commits use.

## 15. Order of work (from docs/CRYPTO-LEARN.md)

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

### 18.8 Order of work (from docs/CRYPTO-LEARN.md)

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
