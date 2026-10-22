# Anti-exfil

**Kind:** Spec
**Status:** Planned, not scheduled
**Open:** all of it, built only when the owner says; the owner's choices in §9 first.

The problem it solves: a signer that chooses its own nonce can hide the
private key in it, and the signature it makes that way verifies like an
honest one (`docs/learn/17-nonces.md`). Faraday's nonces are
deterministic (RFC 6979 with low R, BIP-340 with no auxiliary
randomness), so a hidden key is caught by signing the same transaction
again on a second implementation that holds the seed. Anti-exfil
catches it without the seed: the host takes part in making the nonce
and checks every signature with public information only.

Two forms are planned, each on its own:

- **A, verify a transcript**: Faraday checks a signature another device
  (a Blockstream Jade) made with anti-exfil (§4, §5).
- **B, sign with it**: Faraday is the signer and a host checks it (§6).

Neither is needed for the signer checks that need no new protocol
(comparing the PSBT sent with the one returned, the sighash a returned
signature uses, fields a signer added, the Nonce check failing closed);
those are planned on their own and come first.

## 1. The protocol

Anti-exfil is sign-to-contract over ECDSA as libsecp256k1-zkp
(`anti_exfil`, `ecdsa_s2c`) and libwally (`wally_ae_*`) define it. Per
signature, with `H_data` and `H_point` the two tagged hashes those
libraries name (`s2c/ecdsa/data`, `s2c/ecdsa/point`):

1. The host draws 32 random bytes `e` and sends only `c = H_data(e)`.
2. The signer derives `k1` from its key, the message and `c` (RFC 6979,
   `c` as the extra data) and returns `R1 = k1·G`, its nonce
   commitment.
3. The host sends `e`.
4. The signer signs with `k = k1 + t`, `t = H_point(R1 ‖ e)`, so its
   nonce point is `R = R1 + t·G`, and returns the signature.
5. The host checks that the signature verifies under the public key
   and that its `r` is the x-coordinate of `R1 + t·G`.

The signer is bound to `R1` before it sees `e`, so it cannot steer the
final nonce. The host is bound to `e` before it sees `R1`, and `k1`
depends on `c`, so a host cannot have one `k1` used with two values of
`e` and solve for the key.

## 2. What it does and does not cover

- Covers: a key carried out in the nonces of signatures that reach the
  chain, checked by a party that holds no seed.
- Does not cover: a seed made from bad entropy, a key carried out by
  any other channel (a field the signer adds, a change output it
  swaps), or a host that colludes with the signer. Step 5 shows that
  the arithmetic holds, not that `e` was random: the protection is as
  good as the party that drew `e`.
- ECDSA only. No standard form exists for BIP-340 signatures, and
  Jade's protocol has none.

## 3. The values a transcript holds

Per signature: the input's index, the public key, `e`, `R1`, and the
signature. The message is the input's sighash, which Faraday computes
from the PSBT itself through `core/faraday-psbt/src/verify.rs`, never
read from the transcript. A transcript that holds `c` and not `e`
proves nothing and is refused.

## 4. Form A: where a Jade transcript comes from

Jade runs this protocol only with a host over USB or Bluetooth (its
`sign_tx` messages with anti-exfil turned on). No host saves the
values, so a transcript has to come from a host Faraday provides. The
Starter is that host: it runs on the online computer, already moves
files to and from the device by QR code (`docs/STARTER.md`), and would
gain a Jade connection over USB serial. Two arrangements:

1. **The Starter draws `e`.** It runs steps 1 to 5 with the Jade,
   checks the result itself, and sends the signed PSBT to Faraday with
   the transcript in proprietary fields (§7) by QR. Faraday checks
   again, offline, against the sighash it computes. This adds a second
   check on a trusted device; it cannot show that `e` was random, since
   the Starter is on the online computer.
2. **Faraday draws `e`** (the stronger form). The Starter only relays:
   Faraday shows `c` as a QR code; the Starter gives it to the Jade and
   shows back `R1`; Faraday scans `R1` and shows `e`; the Starter gives
   `e` to the Jade and shows back the signed PSBT; Faraday checks step
   5. The randomness and the check are both offline, so malware on the
   online computer can neither choose `e` nor fake the check. Four QR
   crossings instead of two, while the Jade holds its signing session
   open.

The Jade's own QR mode is not a source: as far as is known it signs
PSBTs without anti-exfil. That is checked against Jade's firmware
before this is built.

## 5. Form A: on Faraday

A transcript arrives inside the PSBT (§7). The review's signature
checks gain one verdict per anti-exfil signature: checked, or failed
(a Danger card: the signer did not use the host's randomness). The
Nonce check lists it beside the RFC 6979 recomputation, which does not
apply to such a signature. In arrangement 2 Faraday also keeps `e` for
the transaction it is signing for, in memory only, and refuses a
transcript whose `e` is not the one it drew.

## 6. Form B: Faraday signs with anti-exfil

Faraday is the signer in steps 2 and 4.

- Transport: two round trips across the air gap. A PSBT with `c` per
  input goes in; Faraday returns it with `R1` per input; it comes back
  with `e`; Faraday returns it signed.
- State: nothing secret is kept between the trips. `k1` is derived
  again from the key, the message and `c`; Faraday refuses an `e`
  whose `c` does not match, and refuses a second `c` for an input it
  has already committed to in this session, so one `k1` never meets two
  values of `e`.
- Host: no coordinator speaks this today. The Starter is the host, as
  in §4: it adds `c`, reads `R1`, adds `e` and checks every signature,
  between the wallet that builds the PSBT and Faraday.
- Two machines: the stick starts its own computer, so the Starter runs
  on another one, or the device is the Pi, as Transfer is used today.
- One PSBT carries every input's values, so a transaction of any size
  takes the same four crossings.
- Signing without anti-exfil stays the default: a PSBT that carries no
  `c` is signed as today.

This is the protection Jade gives with its host: the same scheme, a
signer checked by a machine it does not control, ECDSA only, the host
from the same project as the signer. Faraday's nonce before the host's
part is RFC 6979, so without a host its signatures stay reproducible
from the seed, as they are today.

## 7. PSBT fields

No BIP defines fields for anti-exfil. The plan is proprietary pairs
under one prefix, per input: the host commitment `c`, the signer
commitment `R1`, the host entropy `e`, each keyed by the public key it
is for. Accepting fields beyond BIP-174 is the owner's decision,
recorded in DECISIONS. If a standard appears, Faraday reads it and the
proprietary form goes.

## 8. Tests

A cryptographic change, so tested against published vectors and
another implementation: libsecp256k1-zkp's and libwally's anti-exfil
vectors for steps 1 to 5; a transcript recorded from a real Jade for
form A; form B's signatures checked by libwally's `wally_ae_verify`.
Refusals tested: a transcript without `e`, an `e` that does not match
`c`, an `R1` that does not match the signature, a second `c` for one
input.

## 9. Questions for the owner

1. Is either form wanted, and which first?
2. Proprietary PSBT fields (§7): yes or no.
3. Should the Starter talk to a Jade at all? It makes the Starter a
   hardware wallet host on the online computer, a new role for it.
4. Form A: arrangement 1, 2, or both?
