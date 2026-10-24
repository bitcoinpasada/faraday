# foundation-ur-py, vendored

Foundation Devices' Python port of Blockchain Commons' URKit (bytewords,
the fountain encoder and decoder, xoshiro256** and the UR text), for
the crypto cross-check's `ur` area (`tools/crosscheck/area_ur.py`, F79).
BSD-2-Clause Plus Patent License (`LICENSE`). Standard library only.
Its own tests (`test.py`, not copied) are URKit's, ported; the area takes
URKit's single-part and multipart vectors from them
(`tools/vectors/ur/`).

Source: `https://github.com/Foundation-Devices/foundation-ur-py`, commit
`2c104058d2f6421aaf41f5e919392d3e5a5d2268` (2025-09-17), folder `ur/`,
copied unchanged. Fetched with `git fetch` of that commit, which checks
every object against the commit hash.

Two things in it the area keeps clear of, both written in its
docstring: `ur.is_ur_type` tests one character, so a whole type is
compared as a string (`'a' <= type <= 'z'`), which refuses a type that
starts with `z` and passes characters a UR type may not have; and
`UREncoder` writes a message of one fragment as a single-part UR, which
the area does not ask of it.

| File | SHA-256 |
|---|---|
| `LICENSE` | `2f1b5bb0af989ba3080b7eae752992e65873e294db9fa13bef1858c96fc91113` |
| `ur/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `ur/bytewords.py` | `83495d89584c00a6292dfeb1020ec7b3eef034fdf9c72c89bdbfe37af66835c4` |
| `ur/cbor_lite.py` | `313c06f8d23542fcd7eaa0c25ccb57d568571ba7afd6e630c5d9d418c2265913` |
| `ur/constants.py` | `d7a4ccd6c624c69aec6983667399c6566dddb60134349e43443ac05b67956495` |
| `ur/crc32.py` | `d6434fca4c8877baddfb064d6e08b52223a21df564d9d7996522883108c2aa0d` |
| `ur/fountain_decoder.py` | `5b3182d818ef527472219ce6b1493abd70093749f1f3a2956d0f3d2bf7d95815` |
| `ur/fountain_encoder.py` | `a0181d6cc6673f687871ada8bdb0e97743996914a90dc328a1cbc452ae0dfffd` |
| `ur/fountain_utils.py` | `cbbc87708a1ad1f4e5494f04a4c29ca585946fa55fb0a4dd92711d7d40c20d42` |
| `ur/random_sampler.py` | `76db5db91a1ced9f6984b8e0a6ca51042af0396f61216a24de8611d3bf1d6459` |
| `ur/ur.py` | `b51393572a556ff4426412783b1902d1afc6241f26870ecf649eefb719880929` |
| `ur/ur_decoder.py` | `97811e6d96ccb7fde24c072729f27a1b9cd38adf1b1589e43f0e3b28d55f536a` |
| `ur/ur_encoder.py` | `65d26d72dd51f1df04dda26f560f4633ab37f117c01c9e80be4511923b307597` |
| `ur/utils.py` | `c3781bb9c727053edf7ee4a09a5e3afbbe9e2cdddb362973951fe758ee745815` |
| `ur/xoshiro256.py` | `2ee464a928eb7e11689d7266be288f97ade37039c35df3a3c62e1979b38a566c` |
