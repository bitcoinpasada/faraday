# Codex32 error correction: the frozen corpus

python-codex32's frozen correction corpus (`docs/CODEX32.md` §8.4,
§13.5, F94): 57 damaged codex32 strings, each with the string a correct
decoder proposes or `null` where it must refuse. 29 are of 48
characters (the BIP 93 vector 1 secret) and 28 of 127 (vector 5): every
mix of up to 4 substitutions and 8 unreadable boxes (`?`) within BIP
93's bound, three runs of 13 unreadable boxes on the short checksum and
three of 15 on the long one, and three past every bound (5
substitutions at each length, 14 scattered unreadable boxes). The
corpus says it was drawn from the Haskell implementation with error
correction in BlockstreamResearch/codex32 pull request 70 (head
`610cbad30258c80cd862b3773a20f8099d25e36e`), from the generators of its
`haskell/test/Tests.hs`, each expected value by that implementation's
own correctable property.

Read by `core/faraday-bip/tests/all/codex32_correct.rs`, which runs
Faraday's decoder over every case, and by the crypto cross-check's
correction area (`tools/crosscheck/area_codex32_correct.py`), which
runs Faraday's decoder and python-codex32's over every case.

Source: `https://github.com/BenWestgate/python-codex32`, commit
`f19d4627c5725a9f6cfa995a1f89fe62e1becb4c`:
`tests/data/p70_correction_vectors.json` and the repository's `LICENSE`
(MIT), copied unchanged. Fetched from the repository's raw files at that
commit on 2026-10-23. The decoder the corpus was frozen against,
`src/codex32/correction.py` at the same commit, is vendored with the
modules it imports for the cross-check, in
`tools/reference/codex32/correction/` (its README and
`tools/crosscheck/references.sha256` give its digest,
`3fab7f0fe4ccab44030960d309d2aff5aceb0364451495483120e1341e91b962`).

| File | SHA-256 |
|---|---|
| `p70_correction_vectors.json` | `6aa552b34c0bb2878d45dee2655c331d52e40e41e61cef523415d314ad9948e5` |
| `LICENSE` | `2c451c18c8a93f46ef08fefdb5e99c2250ebdb48f8657506fa721a04de709372` |

The licence, as `LICENSE` states it: MIT, "Copyright (c) 2026", by the
author of python-codex32. Permission is granted to use, copy, modify
and distribute the files provided the copyright notice and the
permission notice are included; the full text is `LICENSE` here.

`p70_correction_vectors.json` has two keys: `source`, where the corpus
came from (the repository, the pull request, its head, the SHA-256 of
its patch, and how the cases were derived), and `cases`, each with
`damaged` (the string with its damage, `?` for an unreadable box),
`expected` (the correction, or `null`), `errors` and `erasures` (how
many boxes of each kind were damaged) and `kind` (`short` or `long`,
the checksum).
