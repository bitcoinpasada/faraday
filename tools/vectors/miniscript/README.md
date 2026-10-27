# Miniscript

The published miniscript vectors the crypto cross-check's `miniscript`
area (`tools/crosscheck/area_miniscript.py`) checks Faraday with.

| File | What | Source | SHA-256 |
|---|---|---|---|
| `core-miniscript.json` | 97 miniscripts with their P2WSH and Tapscript scripts where the test states them, their validity, malleability, signature and timelock-mix flags in each context, and their operation counts and stack sizes where stated | written by `extract.py` from Bitcoin Core `src/test/miniscript_tests.cpp`, tag `v31.1` (commit `9be056a8a72b624dae9623b2f7bded92c2a21c91`) | of the source text: `510cf2af017b6c7b8def36ad9a4cc486f046b25a361130a6a2ee281b4537ffc1` |

To refresh it, download the test file at the new tag, check its digest
changed for a reason, update it in `extract.py`, and run
`python3 -I tools/vectors/miniscript/extract.py miniscript_tests.cpp`.
