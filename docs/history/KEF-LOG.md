# KEF: Krux's encrypted code, read and written in full: how it was built

**Kind:** History
**Status:** Kept as written

Text moved out of `docs/KEF.md` on 2026-11-08, when its last open work was
built and it became a Reference. Each piece keeps its section number; the
source keeps the section.

## 11. Implementation

As the build was planned:

The manifest gains an action (`docs/crypto/actions.toml`), and with it the STANDARDS
row, the crypto Learn block and the `written_here` entry, in the commit that adds
the code, whose `independence` is others' code (Krux's).

Before the code existed, the dependency table read: CBC "read only today, encrypt
comes with it" (its page `docs/deps/cbc.md` said nothing here encrypts); `ctr`,
`aes-gcm`, `ghash` and `polyval` "new", each needing a page in `docs/deps/` and a
`cargo vet` trust entry by publisher (F71); compressing "new" to `miniz_oxide`. A
`Reveal` variant was to be reused if its meaning covered the code, otherwise added in
`faraday-keys` with its doc in the same commit.

## 15.1 The form on a seed

**What must come first:** the scanning card of this section, since every form the board keeps is
read back by Restore (`docs/BACKUP-FLOW.md` §17.12 item 5), and a code is restored from one
place's contents with its passphrase typed. For the code on Paper, also the Starter's blank template
(§15.5), since the Templates item names it.

## 15.2 The tool in Tools

The sources of text were planned in two builds. The first: typed text (no kind asked)
and a loaded wallet's descriptor. Not in the first build: a held key's BIP-39
passphrase, under a new reason `Reveal::KefText(Part)` seen in review and listed in
`docs/KEYS.md`, and a vault's note or password, under `Reveal::VaultEntry`; the
reason they waited was that each opens a new way out of `faraday-keys`.

## Old header

The Spec's Status and Open lines on the day of the move listed the built pieces one by
one: the crate, the codec module, the PBKDF2 loop, Krux's vectors, the cross-check
area, the fuzz target, the classifier's arm, `Reveal::KefCode`, the worker jobs, the
scanning card and sheet, the making backend, the form and the tool, the checklist
item, the stick files and the backup record, Learn's text, Practice's sample and the
code on Paper with Scan my copy. All were built.
