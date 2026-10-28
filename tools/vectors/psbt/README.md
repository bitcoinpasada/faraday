# PSBT vectors

| File | What it is |
|---|---|
| `bip174.txt` | BIP-174 test vectors, extracted by `extract.py`; documented in `core/faraday-psbt/README.md` |
| `bip174-roles.txt` | The BIP-174 role vectors (creator, updater, signer, combiner, finalizer, extractor) |
| `rfc6979.txt` | RFC-6979 deterministic-nonce vectors |
| `demo-regtest.psbt` | The clean regtest spend the Sign review scripts and the QR fixtures use |
| `warn-*.psbt` | One transaction per policy warning (below) |
| `wallet-2of3.policy` | A BIP-388 wallet policy: 2 of 3, `wsh(sortedmulti(…))`, the "abandon … about" key with "zoo … wrong" and "legal … yellow" as its cosigners, on regtest |
| `wallet-2of3.psbt` | A spend of that wallet: 60 000 sat out, 39 000 back to its own change address, which is verified change only while the wallet is in use |
| `wallet-liana.policy` | A Liana-shaped miniscript wallet: `wsh(or_d(pk(@0/**),and_v(v:pkh(@1/**),older(52560))))`, the "abandon … about" key with "zoo … wrong" as the recovery key, both at the BIP-48 p2wsh path, on regtest |
| `wallet-liana.psbt` | A spend of that wallet: 60 000 sat out, 39 000 back to its own change address. The input carries the witness script and both key origins, so the first key's path is the one it takes |
| `wallet-liana-recovery.psbt` | The same spend down the recovery path: the sequence `older(52560)` wants, and only the recovery key's origin, which is what a coordinator building this spend hands over |
| `wallet-tree.policy` | A taproot tree wallet: `tr(@0/**,{and_v(v:pk(@1/**),older(4320)),pk(@2/**)})` over the three keys' BIP-86 accounts, on regtest |
| `wallet-tree.psbt` | A spend of that wallet through the leaf that wants the third key alone: one leaf script, one control block, one key origin |
| `wallet-tree-keypath.psbt` | The same spend by the key path: the internal key and the tree's root, and no leaf script at all |
| `wallet-tapmulti.policy` | BIP 387's taproot multisig: 2 of 3, `tr(H,sortedmulti_a(2,…))` over the same three keys as `wallet-2of3.policy`, at BIP 48's `2'` account, on regtest |
| `wallet-tapmulti-first.psbt` | That wallet's funded spend, as Bitcoin Core 31.1 built it: 0.5 BTC out, the rest back to its own change address, with the leaf script, its control block and four taproot key origins |
| `wallet-tapmulti-signed.psbt` | What this tree wrote from it, signed by the "abandon" key and the "zoo" key: two `tap_script_sigs` under the leaf's hash, and the input not final, because `sign` finalizes nothing |
| `wallet-musig.policy` | A BIP-390 MuSig2 wallet on mainnet: `tr(musig(@0,@1)/**)` over two BIP-86 accounts, which §16.70's address and export tests read |
| `wallet-musig-regtest.policy` | The regtest MuSig2 wallet of the Bitcoin Core round trip below: the "abandon … about" key as `@0` and a key Core generated as `@1` |
| `wallet-musig-device-first.psbt` | That wallet's funded spend, before any nonce: 0.5 BTC out, the rest back to its own change address |
| `wallet-musig-core-first.psbt` | The same spend after Core's first `walletprocesspsbt`: Core's public nonce is on it, this device's is not |
| `wallet-musig-core-first-signed.psbt` | What the device wrote from it: its own public nonce and its partial signature, both keyed by the taproot output key |
| `wallet-musig-device-first-nonce.psbt` | The other order, round 1: the funded spend with this device's public nonce on it and nothing signed |
| `wallet-musig-device-first-core.psbt` | Core's answer to it, after two `walletprocesspsbt` calls: Core's public nonce and Core's partial signature |
| `wallet-musig-device-first-signed.psbt` | The device's round 2 over that: its partial signature, the aggregate as the key-path signature, and the input final |
| `finalize-or-b.psbt` | The cross-check's seeded sign case `4fde5465…`, signed and not final: input 3 spends `wsh(or_b(pk(A),s:pk(B)))` with a 72-byte signature from A and a 71-byte one from B |
| `finalize-or-b-final.psbt` | python-bip380's finalized answer to it: input 3's witness carries B's signature and A's empty push, the smaller of the two satisfactions (`core/faraday-psbt/tests/all/satisfaction.rs`) |

## The miniscript wallets' addresses

The first two receive and change addresses of each, checked against
Bitcoin Core v29.4.0 `deriveaddresses` on this machine on 2026-09-13 and
against `miniscript` used directly (`core/faraday-bip/tests/all/miniscript.rs`).

| Wallet | Chain | 0 | 1 |
|---|---|---|---|
| `wallet-liana` | receive | `bcrt1qcn77knrrdsk3wpjmndhe4xuntfqq3ft7xtr0yu23grfeqgsxkn2qjk0fz5` | `bcrt1qk93r8dd9jsf9ev4lzdzknj49se3fkmq9hju0tx8mwuhchfahn28q0l7hyz` |
| `wallet-liana` | change | `bcrt1q5kpvx7hd28xgmxq2rv5cvvwu2dtm5qfhxzsdg7300jjy3r694r2qn045yn` | `bcrt1qemhk2n6cfm5w9xqhuzp94wgwprclmwvkk2ex9fksr55chlmjnugq083cg5` |
| `wallet-tree` | receive | `bcrt1p0sf2utyzdhg8gcek3zjrh638jyns5dnfk0g6l25a9aem2v9lwjhqm20kl5` | `bcrt1pn4y4wpq7zdvjq8kyy5r6mfkckxkdvtwqcunp2mh80mqf4x47qu2q7gaumk` |
| `wallet-tree` | change | `bcrt1pw60dya6xsx3y3xh35xu5r3xzdyh2j942kaz9gg8qtfhg7dz7t0usv0ge4q` | `bcrt1pnxumakwwk2yr6ejukntjxkf00ap8e06t6axel3jlsqjng9l9yuqqncymvc` |

Core also accepted both descriptors' checksums, `#d9zz2z8e` for the
`wsh` one and `#00m3rdde` for the `tr` one, which is a second reading of
`faraday_bip::descriptor`.

## The taproot recovery wallets' addresses

`faraday_bip::recovery` builds a recovery wallet from keys and delays. Its
`wsh` answer for one key now and one key after 52 560 blocks is
`wallet-liana.policy` character for character, so its addresses are the
rows above. Its two taproot answers are new descriptors, and their first
two receive and change addresses were checked against Bitcoin Core
v31.1.0 `deriveaddresses` on this machine on 2026-09-18
(`core/faraday-bip/tests/all/recovery.rs`):

- **key path**, one primary key and one recovery key after 52 560
  blocks, the primary on the key path —
  `tr([73c5da0a/48'/1'/0'/2']tpubDFH9d…/<0;1>/*,and_v(v:pk([3f635a63/48'/1'/0'/2']tpubDFPtP…/<0;1>/*),older(52560)))#3ah698nn`
- **unspendable**, a two-of-two primary and one recovery key after
  52 560 blocks, so every path is a leaf and the internal key is
  BIP-341's NUMS point over a chain code made from the leaves' keys —
  `tr(tpubD6NzVbkrYhZ4Xc7sHx4R7YdxNd5kU29D9btbVAQwegYJ1xvbyxWZ2qQHxheE51nqZEtHMUsKk6E4XZhSbVrLCX4vJgBfcowgHeRSNweREV7/<0;1>/*,{…})#nzn0mjun`

| Wallet | Chain | 0 | 1 |
|---|---|---|---|
| key path | receive | `bcrt1pw3s5nepc02jlyvk8hqqjs3ujwatfagv46t782nk9r9eh97grnwnsmgpq7z` | `bcrt1p4autzdywsrar9ea5k28acfetg0m6lm8ghwphdgklr2fphuefpxlspu9j37` |
| key path | change | `bcrt1pzsezgsyppy7md9v3m788trauf8ext7lthk7yezhprrd42u63c92sjtnn7m` | `bcrt1puh67whutvxxcpumrunw6l7h7q8avlprcx9xvjs3qnyqcqcfjgndszym3gv` |
| unspendable | receive | `bcrt1pswh84u67t65juskhtdgg6racc76txy69pl75t2axf400j2x0x3kq0u5qxn` | `bcrt1paglrj4pjtjwm2fu4c8pkzz5fp7z0a4aed3u7ua7ahzzuwj4s0xhqsqvp2l` |
| unspendable | change | `bcrt1pqqzwemhxyn255eg6nle9nydaudllrq8u3uwja57nxrc27p55m5xqe9fcc6` | `bcrt1p8uunumevnv4y7dqw6wk08cdppxh0r8paksx4vx8g42twlyychnxsma6g5w` |

Core accepted both checksums too.

## What Bitcoin Core checked about the signing

On 2026-09-13, on a throwaway regtest node (Bitcoin Core v29.4.0 on this
machine, no network), both descriptors were imported into watch-only
wallets, funded, and spent. Core built each PSBT with
`walletcreatefundedpsbt`; this crate signed and finalized it; Core then
judged the finished transaction with `testmempoolaccept`, which runs the
script interpreter over it.

| Spend | Witness | `testmempoolaccept` |
|---|---|---|
| `wsh(or_d(pk(A),and_v(v:pkh(B),older(52560))))` through `pk(A)` | signature, witness script | allowed, 161 vB |
| The same script through `and_v(v:pkh(B),older(52560))`, sequence 52 560 | signature, key, the empty dissatisfaction, witness script | allowed, 170 vB |
| `tr(A,{…,pk(C)})` by the key path, signed with A | one signature | allowed, 142 vB |
| The same tree through the `pk(C)` leaf, signed with C | signature, leaf script, control block | allowed, 168 vB |

The recovery row needed a coin 52 560 blocks old, so the regtest chain
was mined out to that height first. Tried against the same coin before
it matured, Core answered `non-BIP68-final`, and the device itself
refuses earlier than that: the finalizer's interpreter rejects the
satisfaction and the transaction stays partial, which is the
"Timelock not met" caution on the review.

The addresses Core derived for the funded wallets were the first receive
addresses in the table above, which is a second reading of
`WalletPolicy::script_at`.

The wallet files are written by
`cargo run -p faraday-psbt --example wallet` (also `just psbt-fixtures`) and
checked byte for byte by the same test as the warning fixtures.
tools/scripts/wallet.txt reads the policy, registers it and signs the
transaction.

## The MuSig2 wallet, and what Bitcoin Core 31.1 checked

Bitcoin Core 31.1 signs `musig()` descriptor wallets and is the only
counterpart that speaks BIP-373's fields today. On 2026-10-28, on a throwaway regtest node (Core v31.1.0 on
this machine, no network), a two-key `musig()` wallet was built with Core
holding the second key, funded, and spent in both orders: Core's nonce
first with the device signing last and no session, and the device's nonce
first with the session signing when the transaction came back.

`tools/scripts/musig-regtest.sh` is the run. Core's key is generated
afresh on every `setup`, so a rerun builds another wallet and every
fixture below changes with it; the fixtures here are one run, and the two
orders share its unsigned transaction.

```text
tools/scripts/musig-regtest.sh setup <datadir>
cp <datadir>/wallet-musig-*.p* tools/vectors/psbt/
cargo run -p faraday-psbt --example musig -- tools/vectors/psbt
# Core first: the device signed last, Core finishes.
tools/scripts/musig-regtest.sh finish <datadir> \
    tools/vectors/psbt/wallet-musig-core-first-signed.psbt
# The device first: Core answers round 1, twice (see below).
tools/scripts/musig-regtest.sh answer <datadir> \
    tools/vectors/psbt/wallet-musig-device-first-nonce.psbt
tools/scripts/musig-regtest.sh answer <datadir> <that call's psbt>
# The second call's PSBT is wallet-musig-device-first-core.psbt.
cargo run -p faraday-psbt --example musig -- tools/vectors/psbt
tools/scripts/musig-regtest.sh accept <datadir> \
    tools/vectors/psbt/wallet-musig-device-first-signed.psbt
tools/scripts/musig-regtest.sh stop <datadir>
```

`setup` imports
`tr(musig([73c5da0a/86h/1h/0h]tpub…,[5e8169da/86h/1h/0h]tpub…)/<0;1>/*)`,
funds it with 1 BTC, and asks for a 0.5 BTC spend; Core's `tprv` stays in
the datadir.

**Core first, the device signing last.**

| Step | Who | Result |
|---|---|---|
| `walletcreatefundedpsbt` | Core | `wallet-musig-device-first.psbt`, one input, two outputs, 1 550 sat fee |
| `walletprocesspsbt` | Core | `wallet-musig-core-first.psbt`: Core's public nonce, no partial signature |
| `sign` | this crate | `wallet-musig-core-first-signed.psbt`: this device's public nonce and partial signature, the PSBT not complete |
| `walletprocesspsbt` | Core | `complete: true` — Core read this device's nonce and partial signature, wrote its own, and aggregated |
| `finalizepsbt` | Core | `complete: true` |
| `testmempoolaccept` | Core | `allowed: true`, txid `f7616435c000b521129945c872f669fa3e2bc6d627103acdfe547388962b571d`, 154 vB, 1 550 sat |

**The device first, with the session.**

| Step | Who | Result |
|---|---|---|
| `sign` round 1 | this crate | `wallet-musig-device-first-nonce.psbt`: this device's public nonce alone, nothing signed, the secret nonce held in the session |
| `walletprocesspsbt` | Core | `complete: false`, and the input carries two nonces and no partial signature: **Core writes only its nonce on this call**, although every nonce is now there |
| `walletprocesspsbt` | Core | `complete: false`, two nonces and one partial signature: `wallet-musig-device-first-core.psbt`. A third call changes nothing, because what is missing is this device's signature |
| `sign` round 2 | this crate | `wallet-musig-device-first-signed.psbt`: this device's partial signature, both aggregated, and the aggregate written as `tap_key_sig` |
| `finalizepsbt` | Core | `complete: true` |
| `testmempoolaccept` | Core | `allowed: true`, txid `f7616435c000b521129945c872f669fa3e2bc6d627103acdfe547388962b571d`, 154 vB, 616 WU, 1 550 sat |

Both orders spend the same unsigned transaction, so both end at the same
txid; the witnesses differ, because the two runs aggregate different
nonces, and so do the wtxids (`3d413ccd…` and `832cfb12…`).

The three PSBTs this device wrote are written by
`cargo run -p faraday-psbt --example musig -- tools/vectors/psbt`, that is
`just psbt-fixtures`, under `Aux::Deterministic` and the example's fixed
session seed, and are checked byte for byte by
`core/faraday-psbt/tests/all/musig.rs`, which also carries the rounds the whole
way with a second signer played in the test, so that the aggregate
signature this crate builds finalizes the input. The files are recorded
as they came out of the run:

| File | SHA-256 |
|---|---|
| `wallet-musig-regtest.policy` | `fee6addc49714a89f0a97d9a807068718e3538dc53d5574ffd4330b8e71103f2` |
| `wallet-musig-device-first.psbt` | `07c2d9e03fdb89b12420bbdbf523d1f4615aef56bd49010676ce870b5f7b027d` |
| `wallet-musig-core-first.psbt` | `dcfda85105aabe3597146ab9b3e676cad0eecc7b0373c7789bda2b6f3bde5d75` |
| `wallet-musig-core-first-signed.psbt` | `fa64fccc09f367129806734e3dc57cfdef8d63a58a9609e18c53a53ab03c140e` |
| `wallet-musig-device-first-nonce.psbt` | `b5f3949f555f8eda24e39a96079ee344e164076cccc60b68fa3a9ef5fe4b8a39` |
| `wallet-musig-device-first-core.psbt` | `90c5c7f3a7e673b6a4cfa6735474e27c27cb014c5c25d8f3e83c74bbaff47cc6` |
| `wallet-musig-device-first-signed.psbt` | `62d366e3c618714db2c03292f3700c78c9b3b7805abcdafb3a42a643eb45ea7c` |

## The taproot multisig, and what Bitcoin Core 31.1 checked

The wallet is `tr(H,sortedmulti_a(2,@0/**,@1/**,@2/**))` over the three
keys `wallet-2of3.policy` uses, at BIP 48's `2'` account on regtest —
the wallet Add a wallet › Taproot multisig builds. Bitcoin Core 31.1
imports `sortedmulti_a` as it stands: no rewriting to `multi_a` with the
keys pre-sorted was needed, and the checksum Core computed for the
descriptor, `#uygt73j8`, is the one `faraday_bip::descriptor` computes.

The run was on 2026-09-19, on a throwaway regtest node (Core v31.1.0
from `~/.local/opt/bitcoin-31.1/bin`, no network), whose datadir was
removed afterwards. Core holds no key of this wallet: it imports it
watch-only, funds it, builds the spend, and judges the finished
transaction.

```text
tools/scripts/tapmulti-regtest.sh setup <datadir>
cp <datadir>/wallet-tapmulti-first.psbt tools/vectors/psbt/
cargo run -p faraday-psbt --example tapmulti -- tools/vectors/psbt
tools/scripts/tapmulti-regtest.sh accept <datadir> \
    tools/vectors/psbt/wallet-tapmulti-signed.psbt
tools/scripts/tapmulti-regtest.sh stop <datadir>
```

| Step | Who | Result |
|---|---|---|
| `importdescriptors` | Core | the committed policy as one checksummed descriptor, watch-only, `success: true` |
| `getnewaddress` | Core | `bcrt1p6srr8gqakft982yss7sh0sl8urqjj8fk5l2n9dumnj2fljvfdf2swezdl4`, which is `WalletPolicy::address_at(regtest, receive, 0)` |
| `walletcreatefundedpsbt` | Core | `wallet-tapmulti-first.psbt`, one input, two outputs, 1 550 sat fee |
| `sign` | this tree | `wallet-tapmulti-signed.psbt`: two `tap_script_sigs` under the leaf's hash, the input not final |
| `finalizepsbt` | Core | `complete: true` — Core builds the witness from this tree's two signatures |
| `testmempoolaccept` | Core | `allowed: true`, txid `42096a2e92f0ae357cef0e3403cb528876e7f1fe9ab0dcbd629016d697efe64d`, wtxid `25dd5c65…`, 206 vB, 821 WU, 1 550 sat, effective fee rate 0.000 075 24 BTC/kvB |

**What Core wrote for the input.** One `tap_scripts` entry: the
`multi_a` leaf over the three derived x-only keys in sorted order, leaf
version 192, and the one-byte-plus-32 control block
`c050929b74…ce803ac0`, which is the NUMS point with the parity bit and
no Merkle path, because the tree is one leaf. The Merkle root is
therefore the leaf hash itself,
`92ba1dd56b0e2b68cf04df9679c27962401fffcef313d70eaaa027146d146319`.

`taproot_bip32_derivs` holds **four** entries, not three: one per
wallet key, at `m/48h/1h/0h/2h/0/0` under its own master fingerprint and
naming that leaf hash, and a fourth for the NUMS internal key itself,
with master fingerprint `7c461e5d`, path `m` and an empty `leaf_hashes`.
Core writes an origin for a key nobody has, because the descriptor's
internal key is a raw key and Core derives a fingerprint for it anyway.
`WalletPolicy` already leaves a plain internal key out of the wallet's
keys, so the reader passed it over and needed no change; a reader that
counted origins to count cosigners would read this wallet as four.

The device signs with two of the three keys in one pass and finalizes
the result itself, so what the Sign flow hands over is the finished
transaction. The committed fixture is the step before that: the PSBT
`faraday_psbt::sign` writes, which is what a coordinator collecting
signatures would be given.

| File | SHA-256 |
|---|---|
| `wallet-tapmulti.policy` | `163d372f56519a3b2db3e84d7bc23292e8f8401abc3e9847b080007d154a1d8d` |
| `wallet-tapmulti-first.psbt` | `c7605ca40a20c8fbbee8e0835483a9b801b46f379e1116df8a4be350bc211ff7` |
| `wallet-tapmulti-signed.psbt` | `8243e2304a995fc7832adc4ae5c1876fcac8ee94f95ad36e6f0b8eb62e747bcc` |

The signed fixture is written by
`cargo run -p faraday-psbt --example tapmulti -- tools/vectors/psbt`, that is
`just psbt-fixtures`, and checked byte for byte by
`core/faraday-psbt/tests/all/tapmulti.rs`.

## The warning fixtures

Written by `cargo run -p faraday-psbt --example warnings`, that is
`just psbt-fixtures`, and checked byte for byte by
`core/faraday-psbt/tests/all/fixtures.rs`. All but `warn-network-mismatch.psbt`
are regtest spends of the "abandon … about" seed; addresses that are not
ours belong to "zoo … wrong". Read them with the `abandon` key loaded on
regtest.

| File | Warning | Level | What else it carries |
|---|---|---|---|
| `warn-change-spoof.psbt` | `ChangeSpoof` | blocked | the change claims our key and pays the recipient's address, so an `AddressReuse` caution comes with it |
| `warn-unverified-change.psbt` | `UnverifiedChange` | caution | |
| `warn-high-fee.psbt` | `HighFee` | danger | fee is 25 % of the 80 000 sat sent |
| `warn-absurd-fee-rate.psbt` | `AbsurdFeeRate` | danger | about 1 800 sat/vB, which is also a `HighFee` danger |
| `warn-dust-output.psbt` | `DustOutput` | caution | a 200 sat output beside a 98 000 sat one, so no fee warning joins it |
| `warn-unusual-sighash.psbt` | `UnusualSighash` | danger | `SIGHASH_NONE` |
| `warn-mixed-script-types.psbt` | `MixedScriptTypes` | info | a p2wpkh and a p2tr input |
| `warn-network-mismatch.psbt` | `NetworkMismatch` | danger | mainnet paths read on regtest; the paths are under no loaded account, so two cautions come with it |
| `warn-no-participating-key.psbt` | `NoParticipatingKey` | danger | the whole transaction belongs to the "zoo" seed |
| `warn-unknown-derivation.psbt` | `UnknownDerivation` | caution | our fingerprint at `84'/1'/5'/0/0` |
| `warn-locktime-in-future.psbt` | `LocktimeInFuture` | info | locked until block 800 000, and not replaceable |
| `warn-non-standard-script.psbt` | `NonStandardScript` | info | an OP_RETURN output |
| `warn-address-reuse.psbt` | `AddressReuse` | caution | two outputs pay one address |
| `warn-utxo-mismatch.psbt` | `UtxoMismatch` | blocked | the input's transaction is not the one it spends, which leaves it with `MissingUtxo` and `NoParticipatingKey` too |
| `warn-missing-utxo.psbt` | `MissingUtxo` | danger | no UTXO data, so `NoParticipatingKey` comes with it |
| `warn-unsupported-input.psbt` | `UnsupportedInput` | caution | a taproot input claiming a tree the PSBT gives no leaf of and no internal key for, beside an ordinary one |
| `warn-amount-unverified.psbt` | `AmountUnverified` | blocked | two p2wpkh inputs, neither with its previous transaction, so their stated amounts are committed to by nothing; one input alone would not be blocked |
