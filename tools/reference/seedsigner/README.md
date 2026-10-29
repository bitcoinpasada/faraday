# SeedSigner's QR code, vendored

SeedSigner's own encoders and decoder for SeedQR, CompactSeedQR and
Specter's numbered text parts, for the crypto cross-check's `seedqr` and
`bbqr` areas (`tools/crosscheck/area_seedqr.py`, `area_bbqr.py`, F79,
F160): `SeedQrEncoder` and `CompactSeedQrEncoder` (`encode_qr.py`),
`DecodeQR` with `SeedQrDecoder` and `SpecterWalletQrDecoder`
(`decode_qr.py`), and the files they import, copied unchanged. MIT
(`LICENSE.md`, from the repository's root); `helpers/ur2/` is Blockchain
Commons' code under BSD-2-Clause-Patent (`helpers/ur2/LICENSE`), which
`decode_qr.py` and `encode_qr.py` import.

Source: `https://github.com/SeedSigner/seedsigner`, commit
`85cd9a0211eeb22962a5e65ba349ef6efa91bb57` (dev, 2026-09-04), folder
`src/seedsigner/`. Fetched with `git fetch` of that commit, which checks
every object against the commit hash. The files not listed below
(the views, the GUI, the hardware, `controller.py` and the rest of
`helpers/`) are not copied; nothing here imports them.

Not copied, and not provided: the packages `decode_qr.py`, `encode_qr.py`
and `seed.py` import at the top and the code run here never calls, which
SeedSigner's `requirements.txt` pins by hash: `pyzbar` (a camera reader),
`urtypes`, `qrcode` and `Pillow` (the QR image), and embit's key, PSBT and
network modules. `shims/outside_imports.py` (written for the
cross-check, not SeedSigner's) registers each as a module whose every
name is a class that fails when made or called, so a path that reached
one fails instead of answering. embit's `bip39` and its wordlist, which
the SeedQR reader does call, are real (`tools/reference/embit/`).

SeedSigner has the English wordlist only (`Seed.get_wordlist` fails for
any other), so the areas run it in English and compare wordlist
indices, which are the same in every language.

What SeedSigner reads more loosely than Faraday, and how the area keeps
Faraday's rule, is in the facts of `seedqr` and `bbqr`
(`tools/crosscheck/README.md`).

| File | SHA-256 |
|---|---|
| `LICENSE.md` | `1258a5daf0928bc18bfaa2047b8eaa34cae2a70b2f97feb0d024bc52f5ca09f5` |
| `seedsigner/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `seedsigner/helpers/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `seedsigner/helpers/l10n.py` | `611cbe564315817964999f3437eecc6a988e2b968a8169066b146e0d1f784686` |
| `seedsigner/helpers/qr.py` | `ed12c8c01002e52af9946e2c0b579c4fd692dee81612dabbc77e73fb1f0bdca8` |
| `seedsigner/helpers/ur2/LICENSE` | `2f1b5bb0af989ba3080b7eae752992e65873e294db9fa13bef1858c96fc91113` |
| `seedsigner/helpers/ur2/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `seedsigner/helpers/ur2/bytewords.py` | `b652f89758fb84d2beafaeb01e77a8af5f3e7a201d5b3fa1473e01ef8418bfc1` |
| `seedsigner/helpers/ur2/cbor_lite.py` | `313c06f8d23542fcd7eaa0c25ccb57d568571ba7afd6e630c5d9d418c2265913` |
| `seedsigner/helpers/ur2/constants.py` | `d7a4ccd6c624c69aec6983667399c6566dddb60134349e43443ac05b67956495` |
| `seedsigner/helpers/ur2/crc32.py` | `888541118b69e5a3f587e1556f0787f6b644ff263811c6913843eb18b68513da` |
| `seedsigner/helpers/ur2/fountain_decoder.py` | `4285d087cb3dcfb740c9c92329c4ba805cc3044f1a874fb0daa33405ef4fec3f` |
| `seedsigner/helpers/ur2/fountain_encoder.py` | `155812c2f58fc22888b6b2ec601d40fe405f6a4cbc0be58794c8052bf411b25a` |
| `seedsigner/helpers/ur2/fountain_utils.py` | `cbbc87708a1ad1f4e5494f04a4c29ca585946fa55fb0a4dd92711d7d40c20d42` |
| `seedsigner/helpers/ur2/random_sampler.py` | `dcb23762815edd2a0b0fe48fba02fac5b3aba3d67c1f27bafe6823b1dda04b59` |
| `seedsigner/helpers/ur2/ur.py` | `b51393572a556ff4426412783b1902d1afc6241f26870ecf649eefb719880929` |
| `seedsigner/helpers/ur2/ur_decoder.py` | `61fc0d5319f6915aab118bcd13e6aa25e0e021c5b4dcc6af2222ff30a14e9c6a` |
| `seedsigner/helpers/ur2/ur_encoder.py` | `2a05145e5961347404ccf7230727eee85efa2822f0b39890ae42e6fe47202c1b` |
| `seedsigner/helpers/ur2/utils.py` | `565724921749e1ed0e92b6beb38d4f00df2ec373b105928bed1f8c30da297c6d` |
| `seedsigner/helpers/ur2/xoshiro256.py` | `d4d700a789502a4ba21f4a3a24a3c8381f13a3c7fb896b0e19dffc6948aef1d4` |
| `seedsigner/models/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `seedsigner/models/decode_qr.py` | `ca35d03c9ede201893a01e5faceb96d58449e1600a28e564ae29d6a2b0248912` |
| `seedsigner/models/encode_qr.py` | `cba66b93b19ed0831e27bec44c49e6e488cf1634f2b82e5c5f3e9cffe9695957` |
| `seedsigner/models/qr_type.py` | `56b4789b1bc55e2b129d0a1fe873f7b2ff2d7ddafe62f8fce6c0e4d3814b5230` |
| `seedsigner/models/seed.py` | `4f612eeae59dd972bd9187818c6648a70efe7e258f4d5c81662f9175060a7658` |
| `seedsigner/models/settings.py` | `864853cc67cd59d60c220bceb922c502fe06db3993de3de76acee11604e23c13` |
| `seedsigner/models/settings_definition.py` | `f7619d0f4ec2984213e9e10c1f75f9a6651c58f522b79bad680352206cb3bb71` |
| `seedsigner/models/singleton.py` | `caaa41a60af389c8b5322d510536d310664fa4ef53e8957c9fab7f021c4e71ac` |
| `shims/outside_imports.py` (written here) | `153873d64149f8a162df40ebe2bbae56e458845a50f6ef83dd8aea19d15d4338` |
