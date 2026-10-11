# osk-psbt

PSBT (BIP-174, version 0) parsing, inspection, policy warnings, signing and
finalization. `no_std` + `alloc`, no `unsafe`. Dependencies: `bitcoin` and
`osk-bip`.

## What is inspected

`inspect(&psbt, &Context { network, keys })` returns an `Inspection` with
everything the Sign screens need (`docs/UX.md` §7.3), without a private
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
counted as a payment to others. Inputs are verified the same way; a
multisig input is "ours" when a script key's origin names a loaded
fingerprint, and the real key check happens when signing.

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
| `UnusualSighash` | Caution | `SINGLE` or any `ANYONECANPAY` |
| `UnusualSighash` | Danger | `NONE` (outputs not committed to) or a non-standard value |
| `ChangeSpoof`, `UtxoMismatch`, `AmountUnverified` | Blocked | the coordinator is lying or hiding what it spends; `sign` refuses with or without `force`. `UtxoMismatch` covers the BIP-174 signer checks (wrong `non_witness_utxo`, redeem/witness script not matching the program, outputs above inputs); `AmountUnverified` is a SegWit v0 input with no `non_witness_utxo` in a transaction with more than one input |
| `MissingUtxo`, `NoParticipatingKey`, `NetworkMismatch` | Danger | see above |
| `UnverifiedChange`, `UnknownDerivation`, `AddressReuse`, `NonStandardScript`, `UnsupportedInput` | Caution | `NonStandardScript` is `Info` for a zero-value OP_RETURN |
| `MixedScriptTypes`, `LocktimeInFuture` | Info | the signer has no clock; a set locktime is reported, not judged |

## Signing

`sign(&mut psbt, &[&MasterKey], &selection, &ctx, force)`:

- derives each input's key at the path the PSBT states for the selected
  fingerprint, and refuses (`KeyMismatch`) if the derived key is not the
  key named or is not in the script;
- ECDSA nonces are RFC 6979 (libsecp256k1); Schnorr is BIP-340 with zero
  aux-rand (`docs/PLANNING.md` §16.14);
- signs twice and refuses (`Nondeterministic`) unless the bytes match;
  verifies every signature against the public key before writing it
  (`VerifyFailed` otherwise); returns the bytes per input for comparison
  with another implementation;
- refuses (`Danger`) while the inspection has any danger warning unless
  `force` is set; the UI asks for the distinct confirm gesture first;
- refuses (`Blocked`) while the inspection has any blocked warning,
  whatever `force` says.

The derived key lives in the signing function's frame and is erased on
drop (`osk_bip::keys::DerivedKey`). `bitcoin` forces two copies for
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
order (`docs/PLANNING.md` §16.100):

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

## Threshold (FROST)

A threshold wallet is a FROST group whose one key is a plain key: to a
coordinator it is `tr(XPUB/<0;1>/*)` over the group key's synthetic
extended public key, and the chain sees a single-signature taproot
spend. `docs/PLANNING.md` §16.103 is the design; `osk_bip::frost` is BIP
445's arithmetic and `osk_bip::threshold` is the group record.

### The `osk` records

Two BIP-174 proprietary records, in each input's `proprietary` map under
the identifier `osk`:

| Record | prefix | subtype | Key data | Value |
|---|---|---|---|---|
| public nonce | `osk` | `0x00` | participant's public share (33) ‖ taproot output key (33) | 66-byte `PubNonce` |
| partial signature | `osk` | `0x01` | the same | 32-byte `PartialSig` |

Nothing of the group is in the PSBT. The threshold, the group key and
the share list come from the registered record, so an input carrying
`osk` records for a group no registered wallet has is blocked as
**Threshold wallet not registered**; a second implementation reading
these records needs the record too, which is the design. The secret
nonce is never in a PSBT. `strip` runs before a finished transaction
leaves the device, so a coordinator sees the PSBT it built and the
transaction and never the session.

### Which inputs it signs

A `P2trKey` input whose internal key's `tap_key_origins` entry carries a
registered threshold wallet's fingerprint — the synthetic xpub's — and a
two-step unhardened `chain/index`, which is what Bitcoin Core and Sparrow
write for `tr(XPUB/<0;1>/*)`. Then, as for MuSig2: the internal key must
be the synthetic xpub at that chain and index; the output key must be
`script_at`'s; the tweaks from the group key to the output key are one
plain tweak per step and then BIP 341's x-only tap tweak with no merkle
root, and a merkle root is refused. The signer set of an input is the
participants with a public-nonce record, which must be exactly `t`
distinct public shares of the record. Any mismatch blocks the input by
name.

### The rounds

Per threshold input this device holds a share of, in this order:

1. **Already signed** — this share's partial signature is on the input:
   nothing is drawn or signed, and what is left is the aggregation.
2. **A later location** — the input carries the whole signer set's public
   nonces and a carry section was read. This share must be one of the
   signers and the section must hold its secret nonce for this input;
   then, refusing at the first failure by name, the stored nonce's public
   nonce equals the record beside it, the section's sighash equals the
   sighash computed now, the section's signer set equals the records', and
   every partial signature on the input verifies. Then it signs with the
   stored nonce, which is consumed.
3. **The first location** — no nonce is on the input and no section was
   read: the signer set is this share plus the `t - 1` others the caller
   chose, every signer's nonce is drawn from the caller's seed, tagged per
   input and participant, and every other signer's secret nonce goes into
   the carry section. `extra_in` is none for every signer: this share's
   secret already enters its own nonce through `secshare`, and mixing it
   into a nonce that is written to a file would open a path from the
   share to the stick for no gain.
4. Anything else — nonces for a set this share is not in, a section for a
   PSBT with no nonces, nonces with no section — is refused by name.
   There is no in-memory session and nothing waits.

The message is the BIP-341 key-path sighash under `SIGHASH_DEFAULT` only.
When every signer has signed, `partial_sig_agg` gives the 64-byte
signature, which is verified against the output key and written as
`tap_key_sig`.

### The carry file

One file per PSBT, and one file whole, because the shell's file channel
answers one request with one file and a picker shell cannot fetch a
sibling by name. Its magic is `OSKC`, so nothing that reads PSBTs takes
it for one, and it is saved as `partly-signed.osk`:

```text
magic "OSKC" (4)  version 0x01 (1)
section:
  signers u8, then that many public shares (33 each), in identifier order
  entries u8, then per entry:
    input index u32 BE
    public share of the signer whose nonce this is (33)
    sighash (32)
    secret nonce (64)
psbt: the rest of the file, binary PSBT bytes
```

`CarrySection` is a secret: not `Clone`, never printed, wiped on drop,
and its module is in the secret-hygiene lint's list. There is no
encryption in this version; §16.103 offers it as defence in depth and it
stays open.

## What is refused

- Taproot script-path inputs (`P2trScript`) and unknown scripts, when an
  input names a selected key: `Unsupported`.
- Inputs whose UTXO or script data fails the BIP-174 signer checks:
  `Unsupported` with the reason.
- SegWit v0 inputs (p2wpkh, p2sh-p2wpkh, p2wsh, p2sh-p2wsh) with no
  `non_witness_utxo`, in a transaction with more than one input: BIP-143
  signs the amount the PSBT states, so without the transaction it spends
  the amount is unverifiable and two rounds of the lie pay the difference
  to the miner. A transaction with one input is signed from its
  `witness_utxo` alone, because a lie there produces one signature the
  chain rejects and nothing to combine it with. Taproot commits to every
  amount it spends and needs no such transaction.
- PSBT versions other than 0, and PSBTs `bitcoin` rejects.

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
`tests/vectors.rs`, `tests/sign.rs`, `tests/inspect.rs`,
`tests/multisig.rs`, `tests/nonce.rs`.

The threshold fixtures (`wallet-threshold-*`) are the committed regtest
group, Bitcoin Core's funded spend of it, the carry file the first
location wrote and the transaction the second one finished;
`tests/threshold.rs` runs the route in both orders and every refusal, and
`tools/scripts/threshold-reference.py` replays one session against BIP
445's own reference implementation.

The MuSig2 fixtures (`wallet-musig-*`) are Bitcoin Core 31.1's own PSBTs
from a regtest round trip in both orders, with the commands and the
accepted transaction in `tools/vectors/psbt/README.md`; `tests/musig.rs`
reads them.

## Example

```text
cargo run -p osk-psbt --example inspect -- --make-test-psbt /tmp/demo.psbt
cargo run -p osk-psbt --example inspect -- --psbt /tmp/demo.psbt --words "abandon … about" --sign
```
