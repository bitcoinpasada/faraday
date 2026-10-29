# Coldcard's Seed XOR code, vendored

`xor` and `xor_split`, Coinkite's own Seed XOR helpers from the Coldcard
firmware repository's tests, for the crypto cross-check's `xor` area
(`tools/crosscheck/area_xor.py`, F160), copied unchanged: `xor.py`
imports python-mnemonic (`tools/reference/bip39/`). The licence is
Coinkite's `COPYING-CC`: the MIT permission with the Commons Clause, which
allows copying, use and distribution and takes away the right to sell
the Software. This copy is a test reference, given away with the
repository and in no build or release payload.

Source: `https://github.com/Coldcard/firmware`, commit
`a772156db3fa74cbc54db02d35230d30cf4789dc` (master, 2026-10-27), files
`testing/xor.py` and `COPYING-CC`. Fetched with `git fetch` of that
commit, which checks every object against the commit hash.

The firmware's own Seed XOR is `shared/xor_seed.py` (SHA-256
`8e2e8cf991abfee3b31b11aeb532578ff97624234fa3867a5abee8f83e5c8dd5`),
which runs on MicroPython over `ngu`, `ux` and the secure element and
cannot run here. It combines parts with `xor` from `shared/utils.py`
(SHA-256 `3bbf378452192c751570b9c8476e32a0478338536b9b479832a0fb41a0b9b036`
at the same commit), whose body is, but for blank lines, `xor` in
`testing/xor.py`; it splits with SHA-256d masks of `b'Batshitoshi ' +
secret + b'%d of %d parts'` and a last part that is the XOR of the secret
with the masks, offering 12, 18 and 24 words and 2 to 4 parts.
`testing/xor.py`'s `xor_split` makes the same split with a single SHA-256,
so its parts are not the firmware's, but the parts of any split XOR to the
secret, which is what the area asks of Faraday.

| File | SHA-256 |
|---|---|
| `COPYING-CC` | `af3e07181f14662155d76a57cffdfc24517aa279654e394a742eb4ee81adfd4e` |
| `xor.py` | `5bbdf5cf7ea61f1e0ace87de659ddc433f82548e143dd9590fe10c2ba1cc2fff` |
