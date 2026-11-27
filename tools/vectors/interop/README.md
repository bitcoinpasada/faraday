# Other wallets' files

Files other wallets write, taken from those wallets' own test suites on
2026-10-07, for `tests/all/interop.rs`. Public keys and addresses only; the
seeds behind them are the projects' published test seeds.

| Folder | From | Licence |
|---|---|---|
| `sparrow/` | `sparrowwallet/sparrow`, `src/test/resources/com/sparrowwallet/sparrow/io/` (Coldcard, Caravan, Cobo, Electrum, Jade, Keystone, Specter DIY and Specter Desktop files, and BIP 129 records) | Apache-2.0 |
| `bluewallet/` | `BlueWallet/BlueWallet`, `tests/unit/fixtures/` and the strings in `tests/unit/multisig-hd-wallet.test.js` (Coldcard Mk4 and Q, Nunchuk, Unchained, Caravan, Casa, Cobo, Electrum, and Sparrow's exports) | MIT |

Renamed on the way in: BlueWallet's `coldcardmk4/descriptor.txt`,
`new-wasabi.json` and `sparrow-export.json` are `coldcardmk4-descriptor.txt`,
`coldcardmk4-wasabi.json` and `coldcardmk4-generic.json` (the last is
Coldcard's generic export, which Sparrow also writes).
`casa-descriptor.txt`, `sparrow-crypto-output.txt` and
`cobo-crypto-account.txt` are strings from BlueWallet's test file.

The cross-check's `import` area (`tools/crosscheck/area_import.py`) also
reads `sparrow/bsms/`, `bluewallet/nunchuck-bsms.txt`, the two Coldcard
exports and Core's `listdescriptors.json` in `core/`, with each wallet's
addresses from Bitcoin Core.

Not every file here is one Faraday reads: `docs/INTEROP.md` lists which
are refused and why.

## Where each file came from, and its SHA-256

Fetched on 2026-10-07; every file checked on 2026-11-03 to be byte for byte the
one in the repository's tree at these commits (git's blob hash, from GitHub's tree
listing). BlueWallet's `casa-descriptor.txt` and `cobo-crypto-account.txt`,
and `sparrow-crypto-output.txt`, are strings copied from `tests/unit/multisig-hd-wallet.test.js`
at the commit, not files; the `listdescriptors.json` in `core/` (a wallet named
`core-hot`) is not from a test suite and its origin is not recorded: it is
the shape of Core's `listdescriptors`, with Core's checksums, which the
`import` area checks with Core's own code.

| Folder | Repository | Commit | Licence file |
|---|---|---|---|
| `sparrow/` | <https://github.com/sparrowwallet/sparrow> | `a7d6733daba66b0c30d1c6b0afcfea8739184ab8` | `tools/vectors/wallet-files/sparrow/LICENSE` |
| `bluewallet/` | <https://github.com/BlueWallet/BlueWallet> | `3631c47d000e9faf702a9bcf9c9b3f7a4c1bfcfe` | `tools/vectors/wallet-files/bluewallet/LICENSE` |

| File | SHA-256 |
|---|---|
| `bluewallet/caravan-multisig.json` | `d9522473b0e30797ea53e395624b4c86ed54fcd0b430580c8d1fb6d2a2930344` |
| `bluewallet/casa-descriptor.txt` | `123fb6ff8cbb3acb0bb868cd49e9cc37640ada1c517aaf6d7eed2ac8b3f6123c` |
| `bluewallet/cobo-crypto-account.txt` | `8180c5d0c046c1b920ec7f2d20a3bb2b0d8af44c04be4a2f9359d9c7ef84ed31` |
| `bluewallet/coldcardQ.json` | `e9f21f04a52518f4994ccebce1f6f8a3ce2db5e562f4b839e4c6833e4655f963` |
| `bluewallet/coldcardmk4-descriptor.txt` | `f75f6e46ad372752470b9280b2a13a1f5a6fa8dc8dfc4e7d00c9ab1bddab6a7b` |
| `bluewallet/coldcardmk4-generic.json` | `026e1adfd15c6987f3a4acece03f80e2e35c4b99c8ebff4c03fcd5b2e51fe047` |
| `bluewallet/coldcardmk4-wasabi.json` | `6a1c6b1038bd17fc4272c241d3d3045201e582daf3f8e02ba2b636ebac7d2ab9` |
| `bluewallet/fromsparrow-coldcard.txt` | `b8df4a59452b479af5488ad77afbcefabf206b8d61dfa112aeec70762db3baa9` |
| `bluewallet/fromsparrow-electrum.json` | `d1413ef68262fb07cb99ab06bc8a1a5e955c11431c9753d1afd342823efe8339` |
| `bluewallet/fromsparrow-specter.json` | `8afcdfcdd43dad0f8a1f2398bbf8916656767e4271ddee0bb1948e49ce6e013d` |
| `bluewallet/nunchuck-bsms.txt` | `259b4aa22efe1f6a479a5064565e728bc1e318dea46a3297bc325d99dba3b1f3` |
| `bluewallet/nunchuk-export.json` | `c5e87e36216a295c1ded3463595329d416601b4090c68ef663b9cf2c19af004f` |
| `bluewallet/quicklook-preview-sample.psbt` | `f026b0f2b946437e5ae0fda005cc137c3dc65dbf75b647192aee60140924ac67` |
| `bluewallet/quicklook-preview-sample.txn` | `bbd88c81b888a8291da0dc6d423690f0666fc8164606b9c31266ededa00ed7b9` |
| `bluewallet/skeleton-cobo.txt` | `d78efcb68ec687b259ea905075e7058aea09ceb43955dbc622e087503701eebe` |
| `bluewallet/skeleton-coldcard.txt` | `942e94c802e58eb1377e6b1ebbcae37ff1c26245edcb797bb001906ee7fe2c4f` |
| `bluewallet/skeleton-electrum-hex-only.txt` | `fc0299d9970f56cda8c202b682de9e8f6772fda82f684b7c2232b411394a632e` |
| `bluewallet/skeleton-walletdescriptor.txt` | `589d129e923fd51aaaec168e72b2362247a6394b08d5540d93350efb99de9100` |
| `bluewallet/sparrow-crypto-output.txt` | `fec70e4af766050c4416d3b315ebcead5e3486c55de48824f00f3b5efe4f74f6` |
| `bluewallet/unchained.json` | `35fc358af5bb09876d48980265fc193e5cb1b537b239be45266de31bf03b2d79` |
| `core/` `listdescriptors.json` | `ba524c6756cfb480ce766b4d32d8dcda1314ea7c1c70a372a0f94a75fbc3a7e6` |
| `sparrow/bsms/multisig-1.bsms` | `295bdeee42b412bc09a347acb6cc268e47b6d8440eaddb361fdccfb3d8315222` |
| `sparrow/bsms/multisig-10.bsms` | `bfdec2a801ba09ba83c2c50471084e4dfefc776f85c59d966667d6dab035a7a3` |
| `sparrow/bsms/multisig-11.bsms` | `712fae55f188d70f88999cdc9a73a9cae513eebf474ea181e7321a54ec0e88d6` |
| `sparrow/bsms/multisig-12.bsms` | `d8959c13601a5ccffab2f26e220a99e3076e0892d23bd35d55a489877826b71f` |
| `sparrow/bsms/multisig-13.bsms` | `b4aac21ed5a598e9d8efb8b0930e450b1d2b8df6efab5c321c4b08181da5f2a8` |
| `sparrow/bsms/multisig-14.bsms` | `1688ac204d3c0f15da1dd252a3768631e2de5311c200f3c44a440dd4ba35305f` |
| `sparrow/bsms/multisig-15.bsms` | `b55d777cb23ef7e27c14fb750ac3a09588d1d365a9da2e07b3355e95332071c4` |
| `sparrow/bsms/multisig-16.bsms` | `2be959323123943f81b06be32fbface5a5ac9a74df5c71a5bfd96e23889542de` |
| `sparrow/bsms/multisig-17.bsms` | `5ecfd3674741d5fb0a44ad160456390284f7ee759b6e875b48c5a8bd909ed372` |
| `sparrow/bsms/multisig-2.bsms` | `2fdc121f5d3655e5de266708486a83c6934421fa4d55e1b24fdf2e55271334f5` |
| `sparrow/bsms/multisig-3.bsms` | `258ed2684da014289df95c6f7ab4367530f01223654e3995371dbc0bac4cfd57` |
| `sparrow/bsms/multisig-4.bsms` | `b8ad2bd0bba218ec7de0753e53d18722bb7331a32b50d2d0261a61a9b141436b` |
| `sparrow/bsms/multisig-5.bsms` | `d8d09bf2de19d00254a09e194e52de8b1000e57682a241659b8003350572d06a` |
| `sparrow/bsms/multisig-6.bsms` | `098d545bc60d890b13a805574cc840185b24f9ca85958989cc72cc31a4be647a` |
| `sparrow/bsms/multisig-7.bsms` | `db149a6451eb51b4591bb1b32cae64e3661eed5b983c2d643373e9640e3f2229` |
| `sparrow/bsms/multisig-8.bsms` | `3d359d1fe97ee152539101a01d8f9ccaa1c2fbebddcd1f9e383a35ef8eeffca0` |
| `sparrow/bsms/multisig-9.bsms` | `1148c93a7a029c698ac4c36741b88c48ce00a7efa455f928b34f9a11b37a368d` |
| `sparrow/caravan-multisig-export-1.json` | `8d40d0b5190f7850af320f9b8b2475f7785c0ccfb78404a5fe25e1f74235d0b0` |
| `sparrow/caravan-multisig-export-2.json` | `7e8b020d8af44411720faae3d1e9a4ff4567125b65ac61b1a2919f92608fd0aa` |
| `sparrow/caravan-multisig-export-3.json` | `bcba4564794f86626b9d20ef61fed61c11f5b2d930561028262fb594df00d823` |
| `sparrow/cc-multisig-export-1.txt` | `8fcef73372e025a9bc094a6a73a62ba060ef4f7a5645f692d58156e254dc6762` |
| `sparrow/cc-multisig-export-2.txt` | `22b0d21023bf6f5ce40a618cd7e9a2438eb8f331631e04b1281c808224a9d0cd` |
| `sparrow/cc-multisig-export-missingkey.txt` | `9abafcf136cebdd4fef5d2fcf175156569281e4ddb04308e64351c6b8791207f` |
| `sparrow/cc-multisig-export-multideriv.txt` | `e56942ec550e175ddbc855cb72333511030cff25b125eee5897cbd209c987d5a` |
| `sparrow/cc-multisig-export-nopolicy.txt` | `22cf4e6ebc80c1b26e580b8ca500985aa53fae76e24f03bd43fcf1216324ad9c` |
| `sparrow/cc-multisig-keystore-1.json` | `224db7d29b38abc2b7601d0075805e38b4af7d470f385780a50a64dc40201242` |
| `sparrow/cc-multisig-keystore-2.json` | `ebb3ecb80efb86138c7703581b44831acb7970770d1f699482ec826e709e1ef2` |
| `sparrow/cc-singlesig-keystore-1.json` | `b71b6f2945de6123009d33bfd25908e0e960a301ac5c357356ad3055d6afb5a8` |
| `sparrow/cc-singlesig-sp-keystore-1.json` | `b580419e2bc77345dd66e590660fee66ac93bb415ad39c43b93d200938c64e12` |
| `sparrow/cobo-singlesig-keystore-1.json` | `f4ae6421a17501194475d5708519bdae4c4b3671e37c245e747f07c92ba544dc` |
| `sparrow/descriptor-labelled.txt` | `df40d8fe0c34b4287a9fbe36e624acfc89918887d5e48c076bc59cd5784f9a5a` |
| `sparrow/descriptor-multipath.txt` | `444aadd26fd6fee4d4ff26476506611b72e18eaa7fadb3d442adadda908db9d6` |
| `sparrow/descriptor-receive-change1.txt` | `7efe8546519dd16b367c6c4edbeb0e60beae7735d0a21b9bcec7acf9c8846950` |
| `sparrow/descriptor-receive-change2.txt` | `e362c472216a5632d516a5fa4c7e3c044f982b8d020ac8ff50dd2dd1261abbde` |
| `sparrow/descriptor-receive.txt` | `ebbbc623545825644a47e257411395c004875381009158fb9ef67a7932120b70` |
| `sparrow/electrum-multisig-wallet.json` | `3694a0edfffebddf39cc395c312acbeb03c68ec787a71aa190917fb51ab3a7c4` |
| `sparrow/electrum-singlesig-wallet.json` | `82ee225d3be938b8bd6cad955556a1e094f4428c2ad46af0f7654aceda749c26` |
| `sparrow/jade-keystore.txt` | `7622498994f4d1fc466f018cae9efc70d03c9cd554db3ef5f457d508c93d5158` |
| `sparrow/keystone-singlesig-keystore-1.txt` | `8ebd99cb389ef6b5f23ccfb190f0b2bd8ba76ecd34204a63a482c8fe075dda1c` |
| `sparrow/keystone-singlesig-sp-keystore-1.txt` | `a07571045ee7896f7b980892ffe942bd0fd0645b11b2f92d0a712b86cff7fe98` |
| `sparrow/specter-diy-export.txt` | `a3abf71de97676d446b110706edd3bb242a12ca8d829134ed71a4065d2a06006` |
| `sparrow/specter-diy-keystore.txt` | `d5f72730a1ae95977097c5a6d4113d254cdfc23af185fb6125f32211bccf3210` |
| `sparrow/specter-diy-sp-keystore.txt` | `a726f6f83e60693be99796b0a88a5dfb0c2f79ffde45c0ee4ee5aa4d11757ca4` |
| `sparrow/specter-multisig-wallet.json` | `5e720b09840a7ca6f79ffceadb807ab84782ebc542711e41613505ce92fc63f4` |
| `sparrow/specter-wallet.json` | `d8715c06667ba526fb117550f9b72424978ba20302fd4b45f43fdeef2d04b117` |
