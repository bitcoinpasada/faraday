# faraday-psbt

PSBT (BIP-174, version 0) parsing, inspection, policy warnings, signing and
finalization. `no_std` + `alloc`, no `unsafe`. Dependencies: `bitcoin` and
`faraday-bip`.

## Written here, checked against, secrets

Written in this crate rather than taken from another: the PSBT inspection
and verification logic (`inspect`, `verify`, `classify`), signing
orchestration (`sign`), finalization (`finalize`), BIP-137 and BIP-322
message signing (`message`), and the MuSig2 signing rounds over
`faraday-bip`'s arithmetic (`musig`). PSBT parsing and secp256k1 are
`bitcoin`'s.

Checked against: `tools/vectors/psbt/` (BIP-174 vectors, RFC 6979,
Bitcoin Core's regtest PSBTs for MuSig2 and taproot multisig), read by
`tests/all/vectors.rs`, `fixtures.rs`, `wallet_fixtures.rs`, `musig.rs`
and `tapmulti.rs`; `tools/vectors/message/` (`tests/all/message.rs`).

`unsafe`: none (the workspace forbids it).

Secrets are wiped in `sign.rs`, `message.rs` and `verify.rs`
(`non_secure_erase` on the `Keypair` and its tweaked copy). The MuSig2
session's nonces wipe in `musig.rs`. Derived keys are
`faraday_bip::keys::DerivedKey`.

## What is inspected

`inspect(&psbt, &Context { network, keys })` returns an `Inspection` with
everything the Sign screens need, without a private
key:

| Field | Contents |
|---|---|
| `version`, `locktime`, `rbf` | tx version; locktime as none / block height / unix time; whether any input signals BIP-125 |
| `inputs[]` | outpoint, value, script kind, whether ours, stated key origin, sighash type, who already signed, sequence, finalized |
| `outputs[]` | address for the context network (or `non-standard script <hex>`), value, `Recipient` / verified `Change` / `UnverifiedChange`, dust flag |
| `fee`, `fee_rate_sat_vb`, `vsize_estimate` | inputs − outputs; rate from an estimated virtual size (below) |
| `total_in`, `total_out`, `amount_to_others`, `change_total`, `is_self_transfer` | totals; unverified change counts as "others" |
| `participating_keys` | loaded keys that can sign at least one input |
| `multisig` | `m`, `n`, cosigners in script order with fingerprint / signed / ours |
| `register` | the wallet the PSBT defines, offered for registering (below), when the only blocks are `WalletNotRegistered` |
| `warnings[]` | ranked `Info` / `Caution` / `Danger` / `Blocked`, blocks first |

Supported input scripts: p2pkh (BIP-44), p2sh-p2wpkh (BIP-49), p2wpkh
(BIP-84), p2tr key path (BIP-86), and `multi` / `sortedmulti` inside
p2sh, p2wsh or p2sh-p2wsh. Everything else is shown as `P2trScript` or
`Unknown` with an `UnsupportedInput` caution.

The virtual size estimate adds, per unsigned input, a signed size assumed
from its script kind (`ScriptKind::estimated_weight`: 72-byte ECDSA or
65-byte Schnorr signatures, 33-byte keys), and uses the actual size of
finalized inputs. It is accurate to about one vbyte per input.

## How change is verified

`Context.keys` holds, per loaded key, its fingerprint and account xpubs
(`KeyRef::from_master` gives the four single-sig accounts). An output is
verified change only if:

1. its `bip32_derivation` or `tap_key_origins` names a loaded fingerprint;
2. the path is `account/{0,1}/index` below one of that key's accounts;
3. the key derived from the account xpub equals the key the PSBT names; and
4. the address the account's script type produces for it equals the
   output's scriptPubKey exactly (for taproot: the stated internal key,
   no script tree, BIP-86 tweak).

A claim that names a loaded key and passes step 2 but fails 3 or 4 is a
`ChangeSpoof` danger: the coordinator is lying. A claim that names an
unloaded key, or a path not under a loaded account (a BIP-48 multisig
path, another account index), is an `UnverifiedChange` caution and is
counted as a payment to others. Multisig change is verified only by
re-deriving it from a registered wallet; with none registered it is
unverified, whatever script the PSBT states for it. Inputs are verified the same way; a
multisig input is "ours" only where a loaded key's account (single-sig or
BIP-48), or the xpub a registered wallet holds for that fingerprint,
derives the script's key at the path the input states. Where neither
derives the path, the key still takes part, with an `UnknownDerivation`
caution, and the input is not called ours; where the derived key is
another, the key does not take part and the caution says so.

A scriptPubKey carries no network. `NetworkMismatch` is raised from what
does: loaded accounts for another network, global xpubs with other version
bytes, and BIP-44/49/84/86/48 paths whose coin type is not the context's.

## Warning thresholds

| Warning | Level | Rule |
|---|---|---|
| `HighFee` | Caution | fee ≥ `HIGH_FEE_CAUTION_PCT` (5 %) of the amount to others (of all outputs for a self-transfer) |
| `HighFee` | Danger | fee ≥ `HIGH_FEE_DANGER_PCT` (20 %), or fee > `HIGH_FEE_DANGER_ABS` (0.01 BTC) |
| `AbsurdFeeRate` | Danger | ≥ `ABSURD_FEE_RATE_SAT_VB` (1000 sat/vB) |
| `DustOutput` | Caution | below `bitcoin`'s `minimal_non_dust` for the script |
| `UnusualSighash` | Caution | `ALL\|ANYONECANPAY` |
| `UnusualSighash` | Danger | `NONE` (outputs not committed to), `SINGLE` on an input that has an output of its index (only that output is committed to), or a non-standard value |
| `UnusualSighash` | Blocked | `SINGLE` on an input with no output of its index: the signature would commit to nothing; `sign` refuses with or without `force` |
| `ChangeSpoof`, `UtxoMismatch`, `AmountUnverified` | Blocked | the coordinator is lying or hiding what it spends; `sign` refuses with or without `force`. `UtxoMismatch` covers the BIP-174 signer checks (wrong `non_witness_utxo`, redeem/witness script not matching the program, outputs above inputs); `AmountUnverified` is a SegWit v0 input with no `non_witness_utxo` in a transaction with more than one input |
| `WalletNotRegistered` | Blocked | an input that names a loaded key spends a script of several keys (a `multi` of two or more, a miniscript naming two or more, a taproot script path that carries leaves which, with its internal key unless that is BIP 341's unspendable point, name two or more, or which are not its whole tree; a taproot input carrying no leaf spends its key path and is not held) and no registered wallet that holds a loaded key gives its script at the chain and index it states; `sign` refuses with or without `force` |
| `MissingUtxo`, `NoParticipatingKey`, `NetworkMismatch` | Danger | see above |
| `UnverifiedChange`, `UnknownDerivation`, `AddressReuse`, `NonStandardScript`, `UnsupportedInput` | Caution | `NonStandardScript` is `Info` for a zero-value OP_RETURN |
| `MixedScriptTypes`, `LocktimeInFuture` | Info | the signer has no clock; a set locktime is reported, not judged |

## Registering a wallet from the PSBT

An input of several keys is signed only for a registered wallet
(`WalletNotRegistered` above). A PSBT whose only blocks are those, and
which carries BIP 174's global xpubs, offers its wallet
(`Inspection::register`, a `Registration`) when:

- every blocked input is a `multi` of one shape: one wrapper (p2sh,
  p2wsh or p2sh-p2wsh), one threshold, one count of keys;
- every key of those inputs states its origin, and exactly one global
  xpub has that origin less its last two steps (a chain, 0 or 1, and an
  unhardened index);
- the keys are sorted in every input (`sortedmulti`), or are in one
  order of cosigners in every input (`multi`); where both hold, each
  reading is tried against the spend's scripts and those that give them
  all are offered (`Registration::readings`, `sortedmulti` first);
- no key's origin is the master itself.

`register(&psbt, &registration, reading, yours)` writes the descriptor
(`wsh(sortedmulti(2,[fp/path]xpub/<0;1>/*,…))`), reads it as a
`WalletPolicy`, and returns it only once it gives the script of every
input of that shape at the chain and index the input states, and of every
output whose key origins name one of its chains;
`RegisterRefusal::Input(i)` or `Output(i)` names the first it does not
give, `RegisterRefusal::NotYourKey(i)` where cosigner `i` is under a
loaded fingerprint and is not what `yours` (the loaded key's own,
worked out from it) gives at its path, and `RegisterRefusal::NotYours`
where no cosigner is.

A registered wallet counts only where it holds a loaded key
(`Context::holds`: one of its keys is a loaded key's own extended key at
its path, from `KeyRef`'s accounts or `KeyRef::extra`, and none under a
loaded fingerprint is another key): one that holds none verifies no
change and lets no input of several keys be signed. A cosigner is the
person's only where the loaded key itself derives it; a wallet's xpub
under the person's fingerprint is not taken for theirs.

A key origin of more than `faraday_bip::keys::MAX_PATH_STEPS` (16)
steps anywhere in a PSBT is refused when it is parsed.

## Signing

`sign(&mut psbt, &[&MasterKey], &selection, &ctx, force)`:

- derives each input's key at the path the PSBT states for the selected
  fingerprint, and refuses (`KeyMismatch`) if the derived key is not the
  key named or is not in the script;
- ECDSA nonces are RFC 6979 (libsecp256k1); Schnorr is BIP-340 with zero
  aux-rand;
- signs twice and refuses (`Nondeterministic`) unless the bytes match;
  verifies every signature against the public key before writing it
  (`VerifyFailed` otherwise); returns the bytes per input for comparison
  with another implementation;
- refuses (`Danger`) while the inspection has any danger warning unless
  `force` is set; the UI asks for the distinct confirm gesture first;
- refuses (`Blocked`) while the inspection has any blocked warning,
  whatever `force` says.

The derived key lives in the signing function's frame and is erased on
drop (`faraday_bip::keys::DerivedKey`). `bitcoin` forces two copies for
Schnorr (`Keypair`, untweaked and tweaked); both are erased with
`non_secure_erase`.

## MuSig2

A taproot key-path input carrying BIP-373's
`PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` is a MuSig2 input. What is signed is
the BIP-341 key-path sighash under `SIGHASH_DEFAULT`, over the sorted
participants and the tweaks from the root aggregate to the output key:
one plain tweak per step of the sub-path below the BIP-328 aggregate
xpub, then the x-only tap tweak with no merkle root.

The wallet must be registered. The participant set is what the aggregate
key is, so a set that differs from every registered `tr(musig(…)/**)`
wallet's is a different wallet, and the input is blocked with "MuSig2
wallet not registered" rather than signed for a set the coordinator
chose. With the wallet in hand the input must also agree with itself:
`KeyAgg` of the sorted participants is the key the field is keyed by, the
internal key's origin carries the aggregate xpub's own fingerprint and a
two-step unhardened sub-path, the internal key is the aggregate derived
there, and the script is the wallet's own address at that chain and
index. Any mismatch blocks the input by name. A change output is verified
the same way, and one that claims the wallet and pays elsewhere is the
`ChangeSpoof` block a multisig claim would be.

The public nonce and the partial signature this crate writes are keyed by
the participant's key and the **taproot output key** in compressed form,
which is what Bitcoin Core 31.1 writes and reads; BIP-373's text reads as
the root aggregate, so another signer's entry is accepted in any of the
three forms — root aggregate, derived aggregate, output key. Both fields
travel in `bitcoin::Psbt`'s `unknown` maps, which is where a field the
crate has no type for survives a round trip. When the last partial
signature is present they are aggregated, the 64 bytes are checked as a
BIP-340 signature of the output key, and written as `tap_key_sig`, so the
input finalizes as any key-path spend.

### The three ways a participant is handled, and the session

`sign` decides per input and per participant this device holds, in this
order:

1. **Round 2** — the input carries this participant's public nonce and
   the session holds its secret nonce for this transaction: BIP-327's
   `Sign` with that nonce, which is taken out of the session by value, so
   no nonce signs twice.
2. **Signing last** — no nonce of this participant's is on the input and
   every other participant's is: BIP-327's `DeterministicSign`, which
   derives the nonce from the others' aggregate nonce, the keys, the
   tweaks and the message, and so needs nothing kept between rounds. One
   participant per transaction can go this way.
3. **Round 1** — otherwise: `NonceGen` from the session's seed, the
   public nonce written to the input and the secret kept in the session.
   Every partial signature already on the input is dropped, because each
   was made against an aggregate nonce this one is not in.

A participant whose partial signature is already on the input signs
nothing again, whatever else is there; what is left for the pass is the
aggregation.

The session is `MusigSession`: the unsigned transaction's txid, a seed,
and one secret nonce per input and participant. Each `NonceGen` call
draws its own `rand'` as `tagged_hash("OpenSigner/musig-rand", seed ‖
input index ‖ participant key)`, which is the fresh value per call
BIP-327 asks for. It is memory and nothing else: not `Clone`, never
serialised, dropped and wiped when another transaction opens a session,
and empty once every nonce it held has signed. The caller passes it to
`sign` as `&mut Option<MusigSession>` with the seed a new one would use,
and gives `inspect` what it holds as `Context::musig_session`, which is
what tells a nonce of ours the session still covers from one it does not.

One block and one caution are MuSig2's own:

- **MuSig2 wallet not registered** (blocked) — no registered wallet has
  these participants.
- **Nonce replaced · earlier signatures dropped** (caution) — the input
  carries a public nonce of this device's that no open session holds, so
  the pass draws a new one and the partial signatures made against the
  old one go with it. The device locked, powered off, or opened a session
  for another transaction. Refusing instead would leave a transaction
  nobody can finish.

A nonce or partial-signature key with a 32-byte tapleaf hash on it is a
script-path MuSig2 spend and is refused: `tr(musig())` has no tree.

## What is refused

- Taproot script-path inputs (`P2trScript`) and unknown scripts, when an
  input names a selected key: `Unsupported`.
- Inputs whose UTXO or script data fails the BIP-174 signer checks:
  `Unsupported` with the reason.
- SegWit v0 inputs (p2wpkh, p2sh-p2wpkh, p2wsh, p2sh-p2wsh) with no
  `non_witness_utxo`, in a transaction with more than one input: BIP-143
  signs the amount the PSBT states, so without the transaction it spends
  the amount is unverifiable and two rounds of the lie pay the difference
  to the miner. A finalized input is held to this too, except a taproot
  one, since its final witness is the coordinator's. A transaction with one input is signed from its
  `witness_utxo` alone, because a lie there produces one signature the
  chain rejects and nothing to combine it with. Taproot commits to every
  amount it spends and needs no such transaction.
- PSBT versions other than 0, and PSBTs `bitcoin` rejects.
- A taproot key-path input carrying proprietary records under the
  identifier `osk`, which this build does not read: blocked as
  `UnreadRecords` (F127).

`finalize` builds the scriptSig/witness for the supported kinds (`bitcoin`
has no finalizer), clears the signing fields as BIP-174 requires, and
extracts the transaction once every input is final. Descriptor-based
multisig registration, script-path taproot and anything needing
`miniscript` are later milestones.

## Vectors

`tools/vectors/psbt/` holds the BIP-174 vectors (`bip174.txt`, extracted
from the BIP text by `extract.py`; `bip174-roles.txt` for the
creator → extractor chain including the master `tprv` Bitcoin Core signed
with) and RFC 6979 cross-check vectors (`rfc6979.txt`, from python-ecdsa
and Bitcoin Core's `key_tests.cpp`). The files are blank-line separated
records of `key: value` lines; each file's header lists its keys. Tests:
`tests/all/vectors.rs`, `tests/all/sign.rs`, `tests/all/inspect.rs`,
`tests/all/multisig.rs`, `tests/all/nonce.rs`.

The MuSig2 fixtures (`wallet-musig-*`) are Bitcoin Core 31.1's own PSBTs
from a regtest round trip in both orders, with the commands and the
accepted transaction in `tools/vectors/psbt/README.md`; `tests/all/musig.rs`
reads them.

## Example

```text
cargo run -p faraday-psbt --example inspect -- --make-test-psbt /tmp/demo.psbt
cargo run -p faraday-psbt --example inspect -- --psbt /tmp/demo.psbt --words "abandon … about" --sign
```
