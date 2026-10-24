# BIP 380's checksum code, vendored

The descriptor checksum's reference code as BIP 380 prints it, for the
crypto cross-check's `addr` area (`tools/crosscheck/area_addr.py`,
F79), beside Bitcoin Core's `descriptors.py` (`tools/reference/core/`).
Standard library only; it imports nothing.

Source: `https://github.com/bitcoin/bips`, commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (the commit `bip327/` was
taken at), file `bip-0380.mediawiki`, SHA-256
`34e6510bb2eba9445a68eacf3d24d5a4e5f24481477488d5552f674ac81dd5fe`.
Fetched with `git fetch` of that commit, which checks every object
against the commit hash.

`descsum.py` is the BIP's two `<pre>` blocks of Python, copied line for
line: lines 140 to 180 of the file (the character sets, the generator,
`descsum_polymod`, `descsum_expand` and `descsum_check`), an empty
line, and lines 192 to 196 (`descsum_create`). Nothing is added or
changed.

| File | SHA-256 |
|---|---|
| `descsum.py` | `e693773a979e7a4c09bf8721d4bef50c6cc008069252d55c9ebe8779e9e37b03` |
