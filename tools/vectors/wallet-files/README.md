# Other wallets' files, from their own test suites

Real files that wallets write or read, taken unchanged from those wallets'
public repositories at the commits below, for the cross-check's `import`
area (`tools/crosscheck/area_import.py`, batch X30), which reads each with
Faraday and with its own reading of the file's format, and judges the
wallet by Bitcoin Core: the same wallet, or a refusal on both sides. Test
data only: public keys and addresses of the projects' published test
seeds, no private key, seed or password (checked by search on 2026-11-03).
Every file is the bytes at that commit, fetched over HTTPS and kept at its
path in the repository.

| Folder | Repository | Commit | Date | Licence |
|---|---|---|---|---|
| `coldcard/` | <https://github.com/Coldcard/firmware> | `1ba960460692fee7ac0ec42403eeec5910942710` | 2026-11-02 | MIT with the Commons Clause (`COPYING-CC`) |
| `caravan/` | <https://github.com/unchained-capital/caravan> | `875e68cd51b9d3b280d098117a62f4af094a952d` | 2024-02-29 | MIT (`LICENSE`) |
| `liana/` | <https://github.com/wizardsardine/liana> | `dabad663dd634cdb87166f6333f5da525898606e` | 2026-11-03 | BSD-3-Clause (`LICENCE`) |

The files Sparrow and BlueWallet publish are in
`faraday/faraday-core/tests/vectors/interop/` (its README has the table of
their commits and SHA-256); their licences, Apache-2.0 and MIT, are
`sparrow/LICENSE` and `bluewallet/LICENSE` here.

Not taken: Electrum's test wallets carry a seed and its funding
transaction (a seed is not a wallet file); Specter Desktop's and HWI's test
data are backup and derivation vectors, not a file any wallet exports;
Nunchuk (libnunchuk, nunchuk-android, nunchuk-desktop), Specter DIY and Keystone
publish no wallet file in a test suite
(Keystone's firmware repository holds BlueWallet's files, taken from
BlueWallet); Bitcoin Core's tests keep no `listdescriptors` file (the
output of a running wallet), and the one in the interop folder has no
recorded source.

| File | SHA-256 | What it is |
|---|---|---|
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2SH-P2WSH_MAINNET.json` | `e9dce8dd7518b38cdaad6493a22b5e1bbb0694778374bc02f724856e17643084` | Caravan's wallet configuration |
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2SH-P2WSH_TESTNET.json` | `fa5ba98a360b8249715744a782ad7dbcfc4bf28729a6d5e0577d166c22414670` | Caravan's wallet configuration |
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2SH_MAINNET.json` | `ae9dfec6b7e0b6d72885e013d02cadc3e671084072f5fae11a61bfbf90ee4e1e` | Caravan's wallet configuration |
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2SH_TESTNET.json` | `8b4fcd2ab662f84b0da220a1ace21d63d4695b734c04f94bf6d219eda56c10c4` | Caravan's wallet configuration |
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2WSH_MAINNET.json` | `9949a65ea61db25b3ce13bb4c6b62da23f2db28c096c01ad67b03fd35193c816` | Caravan's wallet configuration |
| `caravan/fixtures/caravan/bitcoin-2-of-2-P2WSH_TESTNET.json` | `2282fc2262f53308c258892c2b69acc1bb919f63905f7fb5a3882b3516b6decd` | Caravan's wallet configuration |
| `caravan/fixtures/coldcard/wc-p2sh-p2wsh_m.txt` | `e4ee69fc952857ce1e275707c158027b3c20fc98bf10711fdfb64723a0b7ceda` | Coldcard's multisig setup file for Caravan's tests |
| `caravan/fixtures/coldcard/wc-p2sh-p2wsh_t.txt` | `694a9d683870415a9794eb680c4e827591b2903c00ac915df083c4e8535baca6` | Coldcard's multisig setup file for Caravan's tests |
| `caravan/fixtures/coldcard/wc-p2sh_m.txt` | `d627828523272b40fb207eed6a36927eb9935f17f6572fb9bdf75ca5034ae68c` | Coldcard's multisig setup file for Caravan's tests |
| `caravan/fixtures/coldcard/wc-p2sh_t.txt` | `7446f5601e48b3ec8c01fdf8d8c7988c5aca152fd31f5246c087d36d1439e2c0` | Coldcard's multisig setup file for Caravan's tests |
| `caravan/fixtures/coldcard/wc-p2wsh_m.txt` | `ebae8a08cc46ab9457357d6620a8b2c917461259155302b2fcae26042f2265c6` | Coldcard's multisig setup file for Caravan's tests |
| `caravan/fixtures/coldcard/wc-p2wsh_t.txt` | `4bd979e6fd4d4f93505984136dd87d9179aab5e8193ee8fae50fbdc80c9c62bb` | Coldcard's multisig setup file for Caravan's tests |
| `coldcard/docs/sample-electrum-wallets/electrum-2-of-4.json` | `9780893a98d717d447e49d59e7aadd486b755b6070258198b0764cc84b091d3e` | Electrum's wallet file with Coldcard keys |
| `coldcard/docs/sample-electrum-wallets/electrum-single.json` | `5bbd6ed7035932e4cecb2545313105e3e69c05b77e72ee529402bd6971075621` | Electrum's wallet file with Coldcard keys |
| `coldcard/testing/data/coldcard-export.json` | `6526b066174ff77aeead8780f6a47f9f4c8c2c116bb57e9da0d092f207c791b4` | Coldcard's generic export |
| `coldcard/testing/data/multisig/ccxp-0F056943.json` | `8ba5b6199e0e3f128d0773d14117f6210ef6fcfaa7c3a46d3bd9eec363b60945` | Coldcard's cosigner key file (refused as a wallet: a key) |
| `coldcard/testing/data/multisig/ccxp-6BA6CFD0.json` | `6b6108f93a2a4727ca63f7b4da60d1d306b50caf636c91c6862b59425dc8bb98` | Coldcard's cosigner key file (refused as a wallet: a key) |
| `coldcard/testing/data/multisig/ccxp-747B698E.json` | `16ccffc7f179001c3736451edf9e3903d1d2ac86c8bbea53baf7200bd62f0c86` | Coldcard's cosigner key file (refused as a wallet: a key) |
| `coldcard/testing/data/multisig/ccxp-7BB026BE.json` | `6acedb6e8c6a17e6e2d8d35a8b1bf7ec664750f72308907763922f9a589509f2` | Coldcard's cosigner key file (refused as a wallet: a key) |
| `coldcard/testing/data/multisig/desc-p2sh-myself.txt` | `78a81aa1409e462c79b9e793621cc7cae9ec1c165bbd963041b9fc2d1c0d9890` | A descriptor Coldcard shows for a multisig wallet |
| `coldcard/testing/data/multisig/desc-p2sh-p2wsh-myself.txt` | `f9c6544943d8b91e7763fb616fa777dcf25e5682c6d65d7c744d3a41508601f4` | A descriptor Coldcard shows for a multisig wallet |
| `coldcard/testing/data/multisig/desc-p2wsh-myself.txt` | `e0f6f222bfcb7ce6e0a43f6797b99872654166e6a2a02959986e6591cae65862` | A descriptor Coldcard shows for a multisig wallet |
| `coldcard/testing/data/multisig/el-p2sh-myself.json` | `baeb69b608f724b567fc4b3a212a47a2f574dd35938d7fd8bb5ba9d19af6804c` | Electrum's multisig wallet file with Coldcard keys |
| `coldcard/testing/data/multisig/el-p2sh-p2wsh-myself.json` | `c7d4cd57e53208b74be98f046e594408ae9e6b4ea70b86136f5c5c37ced9edf7` | Electrum's multisig wallet file with Coldcard keys |
| `coldcard/testing/data/multisig/el-p2wsh-myself.json` | `42bcdbe9da3ac9de807c3017cec3fbf7dcf5d50e3af7f1bb4d346defde8ad1ef` | Electrum's multisig wallet file with Coldcard keys |
| `coldcard/testing/data/multisig/export-p2sh-myself.txt` | `5de76c918f2311bfb7d28c16a5934daf0284f3fbb161de0b0c573f2f7b0b1f9c` | Coldcard's multisig setup file |
| `coldcard/testing/data/multisig/export-p2sh-p2wsh-myself.txt` | `29e392c8ad0a72b8fdb827f24eec7783c01c0316455351d18e6f56aba388435f` | Coldcard's multisig setup file |
| `coldcard/testing/data/multisig/export-p2wsh-myself.txt` | `c2620ea270956301bc7d4184196bffc649e1a06bc67f131e6cf1e473b5a36da0` | Coldcard's multisig setup file |
| `coldcard/testing/data/multisig/setting-p2sh-myself.json` | `5b1a3b1365f37b6edd6194d46e5363708c80904f23520343cb01131135407f81` | Coldcard's stored multisig setting (not a file a wallet reads; refused) |
| `coldcard/testing/data/multisig/setting-p2sh-p2wsh-myself.json` | `a682a675d2b976608d974001954166702a1c08913a9bc6700b9a90d4c6c88ff4` | Coldcard's stored multisig setting (not a file a wallet reads; refused) |
| `coldcard/testing/data/multisig/setting-p2wsh-myself.json` | `55a7e4c758664b451b18758acc59e58a9d212c1fce1396662af4406e25ec9319` | Coldcard's stored multisig setting (not a file a wallet reads; refused) |
| `liana/liana-gui/test_assets/coldcard-export.json` | `62dbde2edebfe9425c4fdedca09058da1cf9685a0d5260a928ac4034bed2cfd8` | Coldcard's generic export, in Liana's tests |
