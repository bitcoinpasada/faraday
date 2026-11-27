# OpenTimestamps vectors

Example proofs of the OpenTimestamps client, and the headers of the Bitcoin
blocks they name. The proofs are not changed. The client is
LGPL-3.0-or-later (`LICENSE.md`, the client's own).

Source of `examples/`: `https://github.com/opentimestamps/opentimestamps-client`,
branch master at commit `f3a9e92e4dcb89ed93396d71ec808b3290a438c4`, the
folder `examples/`, fetched as the files of that commit. `bitcoin.pdf` and
`gdp2q25-2nd.pdf` are not kept (the proofs state their digests, so they are
checked from those); the client's `sha1/` folder holds the two files of a
SHA-1 collision and the proof of both (`a-or-b.ots`). `invalid/` holds three
proofs the reference refuses.

Source of `headers/`: for each block height an attestation names, the block's
80-byte header as hex (`<height>.hex`) from a public block explorer's raw
header of that block, and the block's hash as the explorer gives it
(`<height>.hash`). A test checks that each header hashes to its recorded hash.

| Height | Used by |
|---|---|
| 129405 | `empty.ots` |
| 358391 | `hello-world.txt.ots`, `bad-stamp.txt.ots` |
| 455605 | `different-blockchains.txt.ots` |
| 465751 | `bitcoin.pdf.ots` |
| 466906 | `sha1/a-or-b.ots` |
| 912095 | `gdp2q25-2nd.pdf.ots` |

Most of the client's older proofs go through a RIPEMD-160 (the calendar's own
step), so Faraday names that hash and does not count the path; the reference
counts it. `different-blockchains.txt.ots` and `gdp2q25-2nd.pdf.ots` are the
two examples that reach a block through SHA-256 only.

| File | SHA-256 |
|---|---|
| `examples/bad-stamp.txt` | `7e3717bbe020f53cdc6c40154a1a8e55bddc13a28c8bb3c82e9ee64b81b44872` |
| `examples/bad-stamp.txt.ots` | `ffea851e316c968e5142df3ea8aa668621d2fec30153e0bd0ef912782e2af7cc` |
| `examples/bitcoin.pdf.ots` | `3d3ec15890e6ee66cfe6a84d4bb2805fa66e59a569b4fad86d4b8c2a71859620` |
| `examples/different-blockchains.txt` | `62c8b090faa21ee5f2e75399d4909e1e27a00ade7dca8f219c6fd34f54de3494` |
| `examples/different-blockchains.txt.ots` | `f5c9b3c73ee7858e8349d3c1f9c4c78fa13596d7cbc42dd577bd1bb0699891b2` |
| `examples/empty` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `examples/empty.ots` | `3bc98a0c89639e3e0b717701877a849bb8b38147402dfb19643cb3bd39894535` |
| `examples/gdp2q25-2nd.pdf.ots` | `0a63adf8a7b47b68d42aec05f365f78a631e64f734b8af5bc594188ee2d751e9` |
| `examples/gdp2q25-2nd.pdf.ots.txt` | `923e9abd1598ac2cdb002709674d9906b8d80c3009d1b716dee56ae82f84255a` |
| `examples/hello-world.txt` | `03ba204e50d126e4674c005e04d82e84c21366780af1f43bd54a37816b6ab340` |
| `examples/hello-world.txt.ots` | `be61ecb940db2e4a6f89fbed59c8d4f7f74fa756cbc1fad1ad7aabcb2bf59c30` |
| `examples/incomplete.txt` | `05c4f616a8e5310d19d938cfd769864d7f4ccdc2ca8b479b10af83564b097af9` |
| `examples/incomplete.txt.ots` | `95673f2878cbcfdddded530bd2ad5f3fb04e13d2e659760d7c1386346fdd036c` |
| `examples/known-and-unknown-notary.txt` | `d288b2ee212b01e3e5f6d333df3a4d53f292cc3f07b09013c0b40c8e7dcb9c03` |
| `examples/known-and-unknown-notary.txt.ots` | `f49b05b941c4d2512b0f0569b57c2592d501ef6036f7965d667eae07b80721aa` |
| `examples/merkle1.txt` | `d32fee9a827f5a0d580f80beb7edce662dd99fcd6591e4ef8a6244403df0b7c9` |
| `examples/merkle1.txt.ots` | `90ba8251d3f3c67e7a0f1610503912b522deeac4662267f8b38d7e81d697c300` |
| `examples/merkle2.txt` | `8bd5a5f07b4451c29756df5eb51d194fb5b20c7e89812d877bbad30d871c582f` |
| `examples/merkle2.txt.ots` | `3603fe0d3de5cde8408c1ce2c79210c9428a4e30c24a79b702f99ff1702f02f5` |
| `examples/merkle3.txt` | `3966a69fc344107bb4f30c574305d9b4508df2cb8e65dd46007a2cfb116b9579` |
| `examples/merkle3.txt.ots` | `ad1f4b14c09dc037afbbc3bea91c3e464df668b994c034bdf11d3eda5d3fc082` |
| `examples/two-calendars.txt` | `efaa174f68e59705757460f4f7d204bd2b535cfd194d9d945418732129404ddb` |
| `examples/two-calendars.txt.ots` | `1f0fe002dc50f6bc96bcab1fffa9d22f8fa71ac1f99b91bd5ece04d06d8b4d64` |
| `examples/unknown-notary.txt` | `dcc21d1d1f42a436a2a07fc915dec04db41b83c898845948c3d664b6660f4f91` |
| `examples/unknown-notary.txt.ots` | `8e9b2d5941745ce822e01c8aedf3725671960d0932699107d4531db7df3c841e` |
| `examples/invalid/bad-major-version.ots` | `7e8c585cc6669f216cff6ee2a322cd2e8856d5d878f72aa2b89803188e37a747` |
| `examples/invalid/exceeds-max-msg-length.ots` | `0adc53bb9768ae961bd40861298af030e059f1915163abbe9177eac389a61905` |
| `examples/invalid/invalid-file-digest-type.ots` | `68aa269dc4ff27a9b76891d9a4423408f10d0413c2c429f9237ce9bfa2ca690b` |
| `examples/sha1/a` | `842a2c7d2f85b25998d5e43fcced0ba3ca570ee0d36bedb23a815d79e614f646` |
| `examples/sha1/a-or-b.ots` | `0ac37bbc1b8545636597183c7ec2c9f37ff44bf3363d0aa845bc02cfa6e0b5eb` |
| `examples/sha1/b` | `cac8644dba1a9aef70cc268f3794036a2be5b5107109ad742247858fd1a36990` |
| `headers/129405.hash` | `a9a747010c15f047682dcb93537eb530205263e1857c96e2804445a00c3895ff` |
| `headers/129405.hex` | `960f79d8c800e1419208c55cd0e4fc00cb503367d9e59e82b072f3c732e33440` |
| `headers/358391.hash` | `4c86b47a62a3cd077fba564c1012354737b6394f9603b34b7f1d7f52704cb4f5` |
| `headers/358391.hex` | `8555f84a925c3fc4ae1a60c736f4057e1f00dee0c7620bc665b2b47d1acd5be0` |
| `headers/455605.hash` | `2734085b99640d20781c6756a7316df495c121ccac89d088896cbdfc6979ba31` |
| `headers/455605.hex` | `0eb527b418ffa755b6e9dcc8dedeca888b9c62d6e21ceb4dabc7fdeb52443dca` |
| `headers/465751.hash` | `7bd22ad135d73f012b8dbda92ae9917e59f4e2a8efdd6ac21611b20e9601b5c9` |
| `headers/465751.hex` | `d0850424f80192ca6d3b8412b03188469d4cf7fe6fd35891547dd98ef08b9c6a` |
| `headers/466906.hash` | `ed28e331869ce832db2cd141eeb64eeec66f5875004d53def06b6b337487e00f` |
| `headers/466906.hex` | `4b4a9e4a9dc8b5375edca374eece081fd862ffef3a0b940e00874188695bd22b` |
| `headers/912095.hash` | `77e30173192687f9e1395d75af7dbc406770747a5f75f57ff4a92bfbb3c6fdd2` |
| `headers/912095.hex` | `e021d9171f27302ab379771140614f07e8bdee6feb8b3f7d5422fdd56481b3f8` |
