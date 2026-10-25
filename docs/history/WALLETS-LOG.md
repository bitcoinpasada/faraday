# The Wallets tab: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/WALLETS.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.
Where a section was rewritten to what the code does, its old text is
here.

## Header (from docs/WALLETS.md)

Built 2026-10-05: every flow in §4 and §5 is in
`faraday/faraday-core`, for spending, creating, backing up, restoring,
messages and the camera — every single-key, multisig, Taproot multisig,
miniscript, tree, MuSig2 and FROST wallet with recovery paths
(`docs/history/HANDOFF.md`, work queue).

## §6 Guided mode: its first paragraph and list as written

The same steps and the same engine serve someone inheriting bitcoin, so
the same paper gives the same addresses and byte-identical signatures.
What changes is wording, order and what is folded away:
- a first page of large choices ("What are you holding?") whose answer is
  the navigation;
- the public half first and the seeds last: load the wallet, check the
  money, add the transaction, then the seeds, then sign;
- the review in sentences, with the full table one choice away;
- warnings word for word as in full mode, from the same function.

## §6 Guided mode: its last paragraph as written

This is the **Spend** tab (`docs/SPEND-FROM-BACKUP.md`, built
2026-10-06): one column from what is in the envelope to a sent
transaction, through the vault's unlock and load, on the same session and
spend as the Wallets tab.

## §7 Tools as written

**Read a transaction** (no wallet needed), **Is this address
mine?**, the address table, exports (descriptor, BIP-388 policy, BSMS, Core
`importdescriptors`, Sparrow `.json`), the nonce check, **Do these agree?**,
BIP-85 (hidden until turned on: children of one seed are not
independent signers, and none are offered as one), and **Just show me**
(a 3-of-5 test wallet from five repeated-word seeds, marked as a
test). Then, in §9's order: the key explorer, silent payments
(receive and check), codex32, SLIP-39 and Seed XOR backups, the Lightning
node key, Learn.

## §8 Tests as written

- **The conformance corpus**: `qr-testing/` and
  `qr-testing-testnet/` restore to the stated first addresses by every
  route; the ten `bad-*` PSBTs are each refused for the reason
  `bad-psbts.txt` states; the 3-of-5 split rebuilds byte-identically from
  any three sheets; the five repeated-word seeds give the same descriptor
  and first address.
- **Flow tests** drive each flow through its steps for every kind in the
  capability table, from every entry point of §2.
- **`core/`'s own tests** run first, unchanged.
- Agreement checks against the Coldcard Q simulator and SeedSigner's
  libraries are reproduced for the flows that produce the same outputs.

## §9 Order of work (from docs/WALLETS.md)

1. The capability table; the wallet card and the
   reassembly model (§2); Load and Signers.
2. Spending for single-sig and multisig: Check, Transaction with every check
   of §3.3, Txid, Sign, Collect, Finish, Agree. Run against the conformance
   corpus.
3. Creating and backing up single-sig and multisig, with the split plan.
4. Guided mode.
5. Taproot multisig, miniscript and Taproot tree, recovery.
6. MuSig2 and FROST, with §4.2.
7. Messages; the Wallets tab's tools; then the other tools.

## §10 Decided 2026-10-04 (from docs/WALLETS.md)

1. **The tab beside the Wallets tab** was deleted with its app (F68).
2. **Shared logic in `core/`**: done for the file
   formats, the kept-key blob, messages, addresses and transactions (§1,
   rule 5).
3. **Signed-amount memory** is sealed into the open vault by default, and
   kept in `/run` across locks (§3.3).

## §3 Shared steps: the upstream app's labels (from docs/WALLETS.md)

Removed 2026-10-22: neither is in Faraday (F68). The Check row ended
"**Is this address mine?**." (Faraday's is **Check an address**), and
the table had the row:

| **Agree** | Paste another device's signed PSBT; compare the signatures byte for byte: **identical** or **differs**, with both shown. |
