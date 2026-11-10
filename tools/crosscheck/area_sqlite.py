"""The SQLite reader (`faraday_sqlite`, F229), against SQLite itself.

SQLite, through CPython's `sqlite3` module, is the implementation Faraday did
not write: it makes each database, with `PRAGMA page_size` at each of the
eight page sizes from 512 to 65,536, and reads it back (`deserialize`, then
`SELECT key, value FROM main ORDER BY rowid`). Faraday's reader is given
the same bytes and returns exactly the same rows in the same order, and the
same table names.

The tables are of the one shape Bitcoin Core makes, `main(key BLOB PRIMARY
KEY NOT NULL, value BLOB NOT NULL)`, filled with random blobs from empty to
several pages (so that overflow pages of every length occur, at every page
size), in several transactions with random rows deleted between them (so
that freeblocks, a freelist and interior pages occur), and in a few cases
beside a second table. The area prints the SQLite version it ran against.

What SQLite reads and Faraday refuses is asked of Faraday directly and
counted: the write-ahead-log form, auto-vacuum databases and UTF-16
databases, each by the refusal that names it.

A second loop changes one byte of such a database. If Faraday reads the
result, SQLite must read it too and return the same rows; a database SQLite
refuses and Faraday reads is a mismatch. Where Faraday refuses and SQLite
reads (Faraday is stricter, as for a key held by two rows) the case is
counted as a refusal, not compared.
"""

import os
import sqlite3
import sys
import tempfile

from common import Area, Mismatch, reference

PAGE_SIZES = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536]
SCHEMA = "CREATE TABLE main(key BLOB PRIMARY KEY NOT NULL, value BLOB NOT NULL)"
# Bytes of blobs one database holds, at most.
BUDGET = 400_000


def image(db):
    """The database file as bytes."""
    return db.serialize()


def blob_length(rng, page_size):
    """A blob length from empty to several pages, with the edges of one
    page's local payload over-represented."""
    kind = rng.randrange(8)
    if kind < 3:
        return rng.randrange(0, 60)
    if kind < 5:
        return rng.randrange(0, page_size // 2)
    if kind == 5:
        # Around the local payload limit of a leaf, usable - 35.
        return max(0, page_size - 35 - rng.randrange(0, 50) - 30)
    if kind == 6:
        return rng.randrange(page_size - 100, 2 * page_size + 100)
    return rng.randrange(0, 4 * page_size)


def make(rng, page_size, wal=False, extra=False):
    """A random database of the one-table shape, from SQLite, and the bytes."""
    db = sqlite3.connect(":memory:", isolation_level=None)
    db.execute(f"PRAGMA page_size={page_size}")
    db.execute(SCHEMA)
    if extra:
        db.execute("CREATE TABLE extra(a BLOB, b BLOB)")
        db.execute("INSERT INTO extra VALUES (?, ?)", (rng.randbytes(5), rng.randbytes(9)))
    spent = 0
    used = set()
    for _ in range(rng.randint(1, 4)):
        db.execute("BEGIN")
        for _ in range(rng.choice([0, 1, 5, 20, 60, 150])):
            n = blob_length(rng, page_size)
            if spent + n > BUDGET:
                break
            spent += n
            k = rng.randbytes(rng.choice([1, 4, 16, 33, 40]))
            if rng.randrange(25) == 0:
                k = rng.randbytes(rng.randrange(0, 2 * page_size))
            if k in used:
                continue
            used.add(k)
            db.execute("INSERT INTO main VALUES (?, ?)", (k, rng.randbytes(n)))
        db.execute("COMMIT")
        if rng.randrange(2):
            m = rng.randint(2, 6)
            db.execute("DELETE FROM main WHERE rowid % ? = ?", (m, rng.randrange(m)))
    return db


def rows_of(data):
    """SQLite's own answer for these bytes."""
    db = sqlite3.connect(":memory:")
    db.deserialize(data)
    tables = [r[0] for r in db.execute(
        "SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY rowid")]
    rows = [[bytes(k).hex(), bytes(v).hex()]
            for k, v in db.execute("SELECT key, value FROM main ORDER BY rowid")]
    db.close()
    return {"tables": tables, "rows": rows}


class Sqlite(Area):
    name = "sqlite"
    faraday = "`faraday_sqlite::Database` (`open`, `tables`, `rows`)"
    reference = (
        "SQLite itself, through CPython's `sqlite3` module (the version is printed at "
        "the start of the area), by the SQLite developers"
    )
    reference_checked = (
        "the rows SQLite returns for the bytes are the answer, with the table names of "
        "`sqlite_schema`; the databases are made by SQLite, and the cases that SQLite "
        "reads and Faraday refuses are asked of Faraday alone"
    )
    vectors = (
        "none; the reader's tests read databases SQLite made "
        "(`faraday/faraday-sqlite/tests/`)"
    )
    corpus = (
        "random one-table databases at each page size from 512 to 65,536, with blobs "
        "from empty to several pages (overflow chains of every length), several "
        "transactions with rows deleted between them, and a second table in some; the "
        "same with one byte changed"
    )
    differences = [
        (
            "SQLite reads the write-ahead-log form, auto-vacuum databases and UTF-16 "
            "databases, and a database with a table of any shape; Faraday reads the "
            "rollback-journal form with UTF-8 text and a table of two blob columns "
            "only, and refuses the rest. Each is asked of Faraday and counted."
        ),
        (
            "SQLite parses the CREATE statements of the schema and refuses a database "
            "whose text is not valid SQL; Faraday reads a table's name and root page "
            "from its schema row and does not parse the statement. A changed byte in "
            "the schema page that SQLite refuses as a malformed schema, and Faraday "
            "reads, is not a mismatch."
        ),
    ]
    not_compared = (
        "none; index pages are not read by Faraday and the rows are SQLite's answer"
    )

    LOOPS = (("corpus", 400), ("mutation", 400))
    lax = 0

    def prelude(self):
        print(f"sqlite: SQLite {sqlite3.sqlite_version} through CPython's sqlite3 "
              f"{sys.version.split()[0]}", file=sys.stderr, flush=True)
        # The reader's own refusals, asked of Faraday alone.
        self.refused_forms()

    def refused_forms(self):
        for pragma, refusal in (("PRAGMA journal_mode=WAL", "Wal"),
                                ("PRAGMA auto_vacuum=FULL", "AutoVacuum"),
                                ("PRAGMA encoding='UTF-16le'", "TextEncoding"),
                                ("PRAGMA encoding='UTF-16be'", "TextEncoding")):
            with tempfile.TemporaryDirectory() as folder:
                path = os.path.join(folder, "x.db")
                db = sqlite3.connect(path, isolation_level=None)
                db.execute("PRAGMA page_size=4096")
                db.execute(pragma)
                db.execute(SCHEMA)
                db.execute("INSERT INTO main VALUES (x'01', x'02')")
                db.close()
                data = open(path, "rb").read()
            answer = self.rust.call("sqlite.read", db=data.hex())
            self.known += 1
            if answer != ("err", refusal):
                raise Mismatch(self.name, "sqlite.read", {"form": pragma}, answer,
                               ("err", refusal), "Faraday did not refuse the form by name")

    def case_corpus(self):
        rng = self.rng
        page_size = rng.choice(PAGE_SIZES)
        db = make(rng, page_size, extra=rng.randrange(6) == 0)
        data = image(db)
        db.close()
        want = rows_of(data)
        self.compare("sqlite.read", {"db": data.hex()}, lambda: want)

    def case_mutation(self):
        rng = self.rng
        page_size = rng.choice(PAGE_SIZES[:5])
        db = make(rng, page_size, extra=rng.randrange(6) == 0)
        data = bytearray(image(db))
        db.close()
        # Half the changes are in the header and the schema page, where a
        # byte means the most.
        if rng.randrange(2):
            at = rng.randrange(0, min(len(data), 300))
        else:
            at = rng.randrange(len(data))
        data[at] = rng.choice([0, 0xFF, data[at] ^ (1 << rng.randrange(8)),
                               rng.randrange(256)])
        data = bytes(data)
        inputs = {"db": data.hex(), "changed": at}
        got = self.rust.call("sqlite.read", db=data.hex())
        if got[0] != "ok":
            return
        want = reference(lambda: rows_of(data))
        if (want[0] == "err" and at < page_size
                and "malformed database schema" in want[1]):
            # A changed byte in the text of a CREATE statement: SQLite parses
            # it, Faraday takes the table's name and root page only.
            self.lax += 1
            return
        self.compared += 1
        if want[0] != "ok" or want[1] != got[1]:
            raise Mismatch(self.name, "sqlite.read", inputs, got, want,
                           "Faraday read a changed database SQLite does not "
                           "read the same way")
