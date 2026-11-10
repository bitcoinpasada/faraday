#!/usr/bin/env python3
"""Makes tests/fixtures/*.db with SQLite itself (Python's sqlite3).

    python3 -I -B faraday/faraday-sqlite/tests/make_fixtures.py

The rows are a formula the tests repeat (tests/sqlite.rs), so the files are
the only input. Rewriting them with another SQLite version changes the
files, not what they hold."""
import os
import sqlite3
import sys

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fixtures")
SCHEMA = "CREATE TABLE main(key BLOB PRIMARY KEY NOT NULL, value BLOB NOT NULL)"
SIZES = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536]
# Blob lengths around the local payload limit of a 512-byte page and past
# many overflow pages.
OVERFLOW = [0, 1, 100, 400, 477, 478, 479, 480, 481, 1000, 2000, 5000]


def key(i):
    return i.to_bytes(4, "big") + b"k" * 20


def value(i):
    return bytes((i * 7 + j) % 256 for j in range(24))


def blob(i, n):
    return bytes((i * 31 + j) % 251 for j in range(n))


def make(name, page_size, setup):
    path = os.path.join(OUT, name)
    if os.path.exists(path):
        os.remove(path)
    db = sqlite3.connect(path, isolation_level=None)
    db.execute(f"PRAGMA page_size={page_size}")
    setup(db)
    db.close()
    for ext in ("-journal", "-wal", "-shm"):
        assert not os.path.exists(path + ext), path + ext


def main():
    os.makedirs(OUT, exist_ok=True)

    def plain(db):
        db.execute(SCHEMA)
        db.execute("BEGIN")
        for i in range(300):
            db.execute("INSERT INTO main VALUES (?, ?)", (key(i), value(i)))
        db.execute("COMMIT")
        db.execute("PRAGMA application_id=-104942375")
        db.execute("PRAGMA user_version=7")

    for ps in SIZES:
        make(f"ps{ps}.db", ps, plain)

    def overflow(db):
        db.execute(SCHEMA)
        for i, n in enumerate(OVERFLOW):
            db.execute("INSERT INTO main VALUES (?, ?)", (key(i), blob(i, n)))

    make("overflow.db", 512, overflow)

    def deleted(db):
        db.execute(SCHEMA)
        db.execute("BEGIN")
        for i in range(400):
            db.execute("INSERT INTO main VALUES (?, ?)", (key(i), blob(i, 40 + i % 90)))
        db.execute("COMMIT")
        db.execute("DELETE FROM main WHERE rowid % 2 = 0 OR (rowid > 100 AND rowid < 160)")

    make("deleted.db", 1024, deleted)

    def wal(db):
        db.execute("PRAGMA journal_mode=WAL")
        db.execute(SCHEMA)
        db.execute("INSERT INTO main VALUES (?, ?)", (key(1), value(1)))

    make("wal.db", 512, wal)

    def other(db):
        db.execute(SCHEMA)
        db.execute("CREATE TABLE extra(a BLOB, b BLOB)")
        db.execute("INSERT INTO main VALUES (?, ?)", (key(1), value(1)))
        db.execute("INSERT INTO extra VALUES (?, ?)", (key(2), value(2)))

    make("other_table.db", 512, other)

    def empty(db):
        db.execute(SCHEMA)
        db.execute("CREATE TABLE extra(a BLOB, b BLOB)")

    make("empty.db", 512, empty)

    def text_value(db):
        db.execute("CREATE TABLE main(key BLOB PRIMARY KEY NOT NULL, value TEXT NOT NULL)")
        db.execute("INSERT INTO main VALUES (?, ?)", (key(1), "text"))

    make("text_value.db", 512, text_value)

    def three(db):
        db.execute("CREATE TABLE main(key BLOB, value BLOB, more BLOB)")
        db.execute("INSERT INTO main VALUES (?, ?, ?)", (key(1), value(1), b"x"))

    make("three_columns.db", 512, three)

    def freelist(db):
        db.execute(SCHEMA)
        for i in range(40):
            db.execute("INSERT INTO main VALUES (?, ?)", (key(i), blob(i, 600)))
        db.execute("DELETE FROM main WHERE rowid > 5")

    make("freelist.db", 512, freelist)
    def small(db):
        db.execute(SCHEMA)
        for i in range(1, 6):
            db.execute("INSERT INTO main VALUES (?, ?)", (key(i), value(i)))

    make("small.db", 512, small)

    def twin(db):
        # A second table of the same length of name: the tests make the two
        # names equal, which SQLite itself never writes.
        db.execute(SCHEMA)
        db.execute("CREATE TABLE mbin(key BLOB, value BLOB)")
        db.execute("INSERT INTO main VALUES (?, ?)", (key(1), value(1)))
        db.execute("INSERT INTO mbin VALUES (?, ?)", (key(2), value(2)))

    make("twin_tables.db", 512, twin)
    print(sqlite3.sqlite_version)


if __name__ == "__main__":
    sys.exit(main())
