# UR vectors

`vectors.json` holds URKit's encoder vectors as foundation-ur-py's
`test.py` states them (commit `2c104058d2f6421aaf41f5e919392d3e5a5d2268`,
`tools/reference/ur/README.md`): the single-part `ur:bytes` of a
50-byte message, and the first twenty parts of a 256-byte message in
fragments of at most 30 bytes. Each message is `make_message(length,
"Wolf")` of its `test_utils.py`: xoshiro256** seeded with SHA-256 of
`Wolf`, as BCR-2020-005's tests make theirs, wrapped as a CBOR byte
string. The cross-check's `ur` area (`tools/crosscheck/area_ur.py`)
reads it.
