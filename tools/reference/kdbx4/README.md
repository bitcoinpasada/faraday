# The KDBX 4 format, as KeePass publishes it

The two pages `core/osk-backup/src/kdbx.rs` and
`tools/backup/read-kdbx.py` were written against, copied here verbatim
so that the writer can be read against the description it was built
from, and so that a later change to the pages is visible as a change to
these files rather than as a silent difference of opinion
(`docs/PLANNING.md` §16.112 pass E2).

Nothing here is built, run or parsed. They are HTML pages for a person
to read.

| File | Page | URL |
|---|---|---|
| `kdbx.html` | KDBX File Format Specification (4.1), which is the complete one | <https://keepass.info/help/kb/kdbx.html> |
| `kdbx_4.html` | KDBX 4, which describes what changed from KDBX 3.1 and why | <https://keepass.info/help/kb/kdbx_4.html> |

Fetched 2026-09-19.

| File | SHA-256 |
|---|---|
| `kdbx.html` | `76c02271617446f86d7ccc5b1891b38f9b54fec57706eccba70ac43eea954744` |
| `kdbx_4.html` | `4536a9920225560e03367ac50839d02e68f5629e75ff1fdfdc59256a28f36cb8` |

The tree writes KDBX 4.0. The specification page describes 4.1; the two
differ only in XML elements this writer does not use, and every KDBX 4
reader takes a 4.0 file.
