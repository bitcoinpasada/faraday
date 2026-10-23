# Codex32QR/v1-48 vectors and pictures

The Codex32QR profile's test vectors and QR pictures (`docs/CODEX32.md`
§10.3, F95): one 48-character codex32 string in upper case in a QR code
of version 3, error correction L. Read by
`core/faraday-bip/tests/all/codex32_sheets.rs` and the crypto
cross-check's codex32 area (`tools/crosscheck/area_codex32.py`).

Source: `https://github.com/FractalEncrypt/FractalEncrypt_seedsigner`,
branch `codex/codex32-error-correction`, commit
`9b8cbe0aaf527ebe9e85d0e90c0b79e2018126c8`, folder docs/codex32_qr (the paths below are in it),
and the repository's `LICENSE.md` (MIT, "Copyright (c) 2021
SeedSigner"). Fetched with `git clone` on 2026-10-23. The pictures carry
no metadata (no chunk but `IHDR`, `IDAT` and `IEND`).

| File here | In the source | SHA-256 |
|---|---|---|
| `LICENSE.md` | the repository's `LICENSE.md` | `1258a5daf0928bc18bfaa2047b8eaa34cae2a70b2f97feb0d024bc52f5ca09f5` |
| `Codex32QR_Spec.md` | `Codex32QR_Spec.md` | `fbc47a80530e29a71b3f1508419f9b065570eb1eb071d3105b43d4153ab9e4eb` |
| `test_vectors/bip93_128bit_vectors.json` | the same | `6f0982a9fed5f560b39a6b53ce0ebc60389a692e03223c5f2f6bc1071f95ffb6` |
| `test_vectors/bip93_128bit_vectors.txt` | the same | `f54ad4c1887bedee09b0685f76ca6d9775cc28faaa57c871f53f2bf3d7fd0839` |
| `test_vectors/testnet_wallet_references.json` | the same | `2ac012ea3266a1eec009f5bb405d2d548c5df35a8165c1377ac5a13c151752b2` |
| `test_vectors/bip93-v1-secret.png` | the same | `67bb1741b270e2cacf97481c848812742bca98a21463d691d686c1e73376dcd4` |
| `test_vectors/bip93-v2-secret.png` | the same | `489da613770895a2c05ba57f89cc1da2949226fbf77d720f273ec59560939d20` |
| `test_vectors/bip93-v2-share-a.png` | the same | `2b824db792dcd7773443817c8ccd587f8e050c53604db832561b50abb7859917` |
| `test_vectors/bip93-v2-share-c.png` | the same | `4961ae14ccd90b2843cc774bcaef45c19c1bdcfdd28dbf4b033ae09c9b025d2a` |
| `test_vectors/bip93-v2-share-d.png` | the same | `064b67b3c45feae7b50c9186fbf7938c541f5a9a1d37c75fa0d88dabedd9959e` |
| `test_vectors/bip93-v3-secret.png` | the same | `e10d3c171d79fcc2f151bfd73676e492f227c3042e68de15d049f4c3c9b15ee6` |
| `test_vectors/bip93-v3-share-a.png` | the same | `d76f13db9856ad6eccd7b2c6c068f10883e9d37063deae2f954e862e01ffaacf` |
| `test_vectors/bip93-v3-share-c.png` | the same | `4cf0fe63772b242dc199e4452552eadc17ad971b4b43839e53d7c5a09a809702` |
| `test_vectors/bip93-v3-share-d.png` | the same | `40e17260b365615df62a0bc999fd68b8c5f1e627bbacd21b7d4e014cd8b38fb8` |
| `test_vectors/bip93-v3-share-e.png` | the same | `de1ceac40dcceec61b073767f03d00b64e295aee5ee41fc98ce48fa245caa960` |
| `test_vectors/bip93-v3-share-f.png` | the same | `f5726dd40d76dc574bd9222abf181cde1ba9f94e7e03537fea452d58b2a7d32a` |
| `img/split-shares/love_share_a.png` | `img/Split Shares/love_share_a.png` | `1e86a63fde6cfe33758d2464e5410a583605fff520edf3aeb65f4c415b15ffea` |
| `img/split-shares/love_share_c.png` | the same folder | `b0131ae68e031833a761994395422aa1134bd0520c3baf95f6630a841501cef8` |
| `img/split-shares/wsfp_share_a.png` | the same folder | `5754517a8a528e302bc1c74b286adaea70b16a21f3d779c070113e6aa1a578c1` |
| `img/split-shares/wsfp_share_c.png` | the same folder | `1872eeffb08d4b7019db18ea02bfa4ac508b71c0dfa7e3a7bd703034f3c610df` |
| `img/split-shares/f0ur_share_a.png` | the same folder | `fb688456ccec07bb1df211c9a329b011b4a827056f47fa1d8172a80789a44ab3` |
| `img/split-shares/f0ur_share_c.png` | the same folder | `1df5adc378925be127015bec14bf83b7afc26847a3b756ecae24b9baa83ef3e5` |
| `img/split-shares/f0ur_share_d.png` | the same folder | `3a48dc282a4a2fe345e951581cad317c2a46755a646fd06503c78a0429c08133` |
| `img/split-shares/f0ur_share_e.png` | the same folder | `7ededb158bf7616da8f541819787dde77216047390cba9637745dd49fbaa263d` |
| `img/s-shares/codex32_s_share_1_seed.png` | `img/S Shares/codex32_s_share_1_seed.png` | `3739a7506979f61ebc8b50cd0941a539c8e034b1b9523eb919226a5070b9a1f3` |
| `img/s-shares/codex32_s_share_2_l0ve.png` | the same folder | `90b4805ec061922e30c23f70197f4cb9dea2e6de726b89b75b200e0544c5e79c` |
| `img/s-shares/codex32_s_share_3_wfsp.png` | the same folder | `50691eb420c1067f5b0ce2f501d6fb4d8f420a6e6110ed62bd0699161a5d22d4` |
| `extract.py` | written here | `218aa99e66cd5a2e29c9ede6f363a8847b0743cb6982032c5cb2f7da9006938a` |
| `vectors.json` | made by `extract.py` | `6f5b166cdeba79b790ea9f77450aa19214e802adc5b38075f18950ac6a15471c` |

The picture folders are renamed here without spaces; nothing else is
changed. `extract.py` reads the profile's §10 into `vectors.json`:

    cd tools/vectors/codex32qr && python3 -I extract.py > vectors.json

Vectors 1 to 3 are sets made from initial shares (`L0VE` and `WSFP` at
k = 2, `F0UR` at k = 4) with the secret they derive; vector 4 is a
secret loaded alone. Each states a testnet fingerprint, a derivation
path and the extended public key there. Vector 2's key is printed cut
short (81 characters), so `vectors.json` marks it incomplete and the
tests check its fingerprint only. The profile's §11 names two more
pictures of a secret without saying whose; their file names say `l0ve`
and `wfsp`, and they decode to the secrets of vectors 1 and 2.
`test_vectors/` repeats BIP 93's vectors 1 to 3 as pictures, each
listed with its payload in `bip93_128bit_vectors.json`.

Every picture was decoded once with `zbarimg` (zbar) to the string the
profile and `vectors.json` give it. Reading them with Faraday's own
scanner path (`third_party/rqrr`) is a test of the scanning batch
(`docs/CODEX32.md` §10.3, C6).
