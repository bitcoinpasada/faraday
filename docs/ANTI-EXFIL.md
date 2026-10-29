# Anti-exfil

**Kind:** Spec
**Status:** Planned, not built; experimental, for dev builds only, compiled out of every shipped app (F162)
**Open:** all of it: Faraday as the host that checks a signer (§4) first, Faraday as a signer a host checks (§5) second, the Nonce check's reading of an exchange (§6), the encoding (§7), the tests (§8), and the owner's open questions in §9

The problem it solves: a signer that chooses its own nonce can hide the
private key in it, and the signature it makes that way verifies like an
honest one (`docs/learn/13-nonces.md`). Two signatures are enough to
carry out a 12-word seed (Dark Skippy, 2024). Faraday's own nonces are
deterministic (RFC 6979 with low R, BIP-340 with zero auxiliary
randomness, F156), so a hidden key in a Faraday signature is caught by
signing the same transaction again on a second implementation that
holds the seed. Anti-exfil catches it without the seed: a second party,
the host, adds randomness to the nonce that the signer cannot choose,
and checks every signature against it.

Both directions are covered, and neither assumes Faraday is honest
(owner, 2026-10-29):

- **Faraday checks another signer** (§4): Faraday is the host, offline,
  between the coordinator and a signing device. It protects the person
  from someone else's signer.
- **Faraday signs with anti-exfil** (§5): Faraday is the signer and a
  host checks it. It protects the person from Faraday.

The checks of a returned PSBT that need no new protocol (the PSBT sent
against the one returned, the sighash a signature uses, fields a signer
added, the Nonce check failing closed) are `docs/SIGNER-CHECK.md` and
come first.

## 1. The protocol

Anti-exfil is sign-to-contract over ECDSA as libsecp256k1-zkp's
`ecdsa_s2c` module defines it (`secp256k1_ecdsa_anti_exfil_*`), wrapped
by libwally as `wally_ae_*`. Faraday follows that module at a commit at
or after `a69a662` ("Fix missing clears on secret values"). The exchange
has four stages, as FractalEncrypt's air-gapped protocol carries them
(§7). Per signature (a slot: one input, one key), with `H_data` and
`H_point` the BIP-340 tagged hashes with tags `s2c/ecdsa/data` and
`s2c/ecdsa/point`, `x` the private key, `m` the input's sighash and `n`
the curve order:

1. **Host commitment.** The host draws 32 secret bytes `rho` and sends
   only `c = H_data(rho)`.
2. **Signer commitment.** The signer derives `k1` by RFC 6979 over
   `x ‖ (m mod n) ‖ c`, `c` as the extra data, with the RFC's counter
   loop, and returns `R1 = k1·G`. It returns no signature.
3. **Host reveal.** The host sends `rho`.
4. **Signature.** The signer computes `c' = H_data(rho)`, refuses when
   `c'` differs from `c`, derives `k1` again from `c'`, and signs with
   `k = k1 + t`, `t = H_point(R1 ‖ rho)`, so its nonce point is
   `R = R1 + t·G`. A tweak `t ≥ n` is refused, never reduced. The
   signature is low-S.

The host then makes **both checks**: `r` of the signature equals the
x-coordinate of `R1 + t·G`, modulo `n`; and the signature verifies under
the public key over `m`, which the host computes from its own copy of
the PSBT, never from the signer's reply. Either alone proves nothing.

The signer is bound to `R1` before it sees `rho`, so it cannot steer
`k`. The host is bound to `rho` by `c`, and `k1` depends on `c`, so a
host cannot have one `k1` used with two values of `rho` and solve for
the key. A host that sends the same `c` again gets the same `k1`, the
same `R1` and, after the same `rho`, the same signature.

**The host needs `rho` to check, and `rho` stays secret for the life of
the key.** If a backdoored signer's `k1` is predictable to its author,
only `rho` keeps `k` secret, from then on: whoever learns `rho` later
computes `t`, then `k`, then the key, from that one signature. A saved
transcript, a backup, a PSBT that still carries `rho`, a screen
recording or a photo of the reveal QR code is a leak of the key. `rho`
is never written to a file, a PSBT, the vault or a log, and is wiped
once the exchange ends.

## 2. What it protects, and what it does not

- **Protects:** a key carried out in the nonces of signatures that reach
  the chain, by a signer that follows the exchange, checked by a host
  that holds no seed and keeps `rho` secret.
- **Does not protect against:**
  - a signer that shows one address or amount and signs another: the
    signer's own review and the host's review of the transaction are
    still needed;
  - a malicious host that builds a bad transaction: anti-exfil checks
    the nonce, not the payment;
  - a seed made from bad entropy;
  - a key carried out by any other channel: a field the signer adds, a
    swapped change output, timing, a refusal to sign chosen to leak a
    bit at a time, extra frames of an animated QR code;
  - a host that colludes with the signer, or a host whose `rho` leaks
    (§1);
  - one ordinary signature by the same key, made outside the exchange:
    a malicious signer only has to make the exchange fail until the
    person signs without it (§4.4);
  - Taproot: no form of anti-exfil covers BIP-340 signatures. The
    Schnorr module for libsecp256k1 (pull request 1140) is unmerged,
    Jade's and BitBox's protocols are ECDSA only, and FractalEncrypt's
    fails a request with any Taproot input. Faraday's Taproot signatures
    stay covered only by the deterministic comparison.
- **Scope:** ECDSA over SegWit v0 (P2WPKH, P2SH-P2WPKH, P2WSH and
  P2SH-P2WSH standard multisig), explicit `SIGHASH_ALL`, PSBT v0. A
  request with any other input that names a key in the exchange fails
  whole.

## 3. Experimental, in dev builds only

FractalEncrypt's exchange is not a published standard: it is an alpha,
written mostly by AI models and reviewed by AI models, with no human
audit, and no BIP or registered encoding carries it (§7). By
`docs/STANDARDS.md` §0 an unpublished standard is built and tested in
the repository and compiled out of every shipped app (F162).

- **One switch.** Every part of this spec in the app's crates is behind
  one cargo feature, `experimental`, off by default, as `unreleased`
  was for FROST (F109, removed with FROST by F127). It is on only in a
  dev build: a dev image (`just dev=1 faraday-stick-image`, a
  developer's own build, F136) or a developer's own `cargo` run. A dev
  image is never shipped: `release=1` refuses one
  (`docs/DEVELOPING.md`).
- **Held by a lint.** `tools/lint-keys-tests.sh` fails when the normal
  dependency graph of a binary Faraday ships turns `experimental` on,
  on any target, as it does for the test-only features; and
  `faraday/image/build.sh` passes the feature only with `DEV=1` and
  refuses an image without it whose app reports the feature in its
  build information. So no release and no test release carries it.
  `just` also runs clippy and the tests of the crates that have the
  switch with it on, so the work keeps building while it is out.
- **`core/` stays whole.** The arithmetic of §1 is cryptography in
  `core/`, built and tested by `just` whether or not the feature is on;
  `tools/lint-standards.toml` gains an `[[out_of_app]]` entry naming it,
  so app code cannot reach it without the feature.
- **Coming into the app** is the owner's decision, recorded in a
  decision that amends F162, once a published standard carries the
  exchange (§7).

## 4. Faraday as the host: checking a signer

Faraday is the host first (owner, 2026-10-29). It sits between the
coordinator (Sparrow, a wallet on the online computer) and a signing
device, in the QR flow: the coordinator's PSBT comes to Faraday,
Faraday runs the exchange with the signer, and the checked, signed PSBT
goes back to the coordinator. Faraday draws `rho`, offline, and `rho`
never touches an online machine. A host on the online computer (a
Sparrow fork, Blockstream's Green) has to show `rho` there or hold it in
the online computer's memory, where malware or a screen recording can
take it (§1).

### 4.1 Where it is

- **Tools, Check a signer**, beside the comparison of a sent and a
  returned PSBT of `docs/SIGNER-CHECK.md`, as its protected exchange.
- **Later, Spend**, when a cosigner signs a transaction Faraday is
  building or collecting.

### 4.2 The flow

1. The coordinator's PSBT is scanned or opened. Faraday freezes its own
   copy and computes each slot's sighash `m` itself
   (`core/faraday-psbt/src/verify.rs`). Each key the signer is to sign
   with is a slot.
2. Faraday draws a `rho` per slot (§4.3), keeps it in memory only, and
   shows the host-commitment message (stage 1) as a QR code for the
   signer to scan.
3. Faraday scans the signer's reply (stage 2), takes the `R1` of each
   slot it asked for and nothing else (§4.5), and shows the reveal
   (stage 3). The reveal screen says that the code shows a secret that
   is never photographed, recorded or kept, and the code is shown only
   while it is scanned.
4. Faraday scans the signer's signatures (stage 4), makes both checks
   of §1 per slot, wipes every `rho`, and writes the verified signatures
   into its frozen copy. The checked PSBT, with no `rho` and no field of
   the exchange in it, goes to the coordinator.

A verdict per slot: **checked**, or **failed** (a Danger: the signer did
not use the host's randomness, or its signature is not over this
transaction). One failed slot fails the whole exchange: no signature of
it leaves Faraday.

### 4.3 Where `rho` comes from

- **Faraday's own randomness** (the default). `rho` is drawn from the
  pool every other secret comes from (`faraday/faraday-core/src/fresh.rs`),
  which mixes everything the person has added into the system's
  randomness (`docs/OWN-RANDOMNESS.md` §2.3). The step holds the
  Randomness row above its press (`docs/OWN-RANDOMNESS.md` §8.2), so
  dice, coins, cards, the camera, and pointer and keys on the desktop
  app can all be added before `rho` is drawn. The press is a row to add
  to §8.1's table.
- **Yours only.** `rho` is a hash of the person's dice alone, so that
  another tool can compute it again and the person can confirm Faraday
  did not choose it: per slot, `rho = SHA-256(SHA-256(tag) ‖ SHA-256(tag)
  ‖ dice ‖ 0x00 ‖ m ‖ P)`, the BIP-340 tagged hash with `tag` the ASCII
  text `Faraday/anti-exfil/dice`, `dice` the rolls as ASCII digits 1 to
  6, `m` the slot's 32-byte sighash and `P` its 33-byte public key.
  Faraday shows each slot's `rho` for the person to compare with their
  own computation. It is Faraday's own construction, and carries F160's
  label "own construction, not yet independently reviewed". The step
  says, as a label: the rolls and `rho` are never written down or kept.

### 4.4 The host's rules

Faraday as host follows these. Each is a refusal; none has a way past
it (F136).

1. **Protected signing is required.** Once the exchange starts for a
   payment, Faraday offers no ordinary signing for it: a failure ends in
   a failed verdict, never in a fallback to a plain PSBT. Where a
   cosigner's key is marked as protected in a wallet (Spend, later),
   Faraday refuses an ordinary signature by that key. Where the person
   turns protection on, the screen says that one ordinary signature by
   the key undoes the protection of every protected one, if the signer
   is malicious.
2. **No fresh `rho` for the same payment after an abort.** A signer that
   stops after the reveal, or a reply that fails, is retried with the
   same `rho` and must return the same `R1`. A payment is its unsigned
   transaction's id. After a lock or a power-off, `rho` is gone (it is
   never saved), so that payment is not retried with Faraday's own
   randomness: a new exchange needs a new transaction from the
   coordinator. In yours-only mode the same rolls give the same `rho`,
   so the retry is the same exchange.
3. **Aborts are counted.** An abort after the reveal is counted per
   signer key: the transaction's id, the key's fingerprint, and each
   slot's `c` and `R1`, never `rho`, in the open vault, or in memory for
   the session when none is open, and the screen says which. A retry
   whose `c` or `R1` differs from the counted one is refused. The count
   is shown at every exchange with that key; past the threshold (§9)
   the screen says to move the funds to a new key.
4. **Both checks** of §1, for every slot, against the sighash Faraday
   computes.
5. **Length caps before allocation.** Every length a message declares
   (the animated QR code's sequence and length, each envelope, record
   and PSBT length, the number of slots) is checked against its cap
   before a buffer is made for it.
6. **Only the allowed fields.** From the stage 2 reply Faraday takes
   each asked slot's `R1`; from stage 4, each asked slot's signature.
   Anything else in a reply is not carried forward, and a reply with
   records for slots not asked, twice for one slot, or missing one, is
   refused. The PSBT that leaves is Faraday's frozen copy with the
   verified signatures added.

### 4.5 Which signers

Any signer that speaks the exchange over QR: FractalEncrypt's SeedSigner
and Kern forks today, Jade once it publishes anti-exfil over QR, and
another Faraday (§5). No USB path to a Jade is planned (owner,
2026-10-29: QR only); released Jade runs anti-exfil only over USB or
Bluetooth.

## 5. Faraday as the signer

Faraday signs with anti-exfil second (owner, 2026-10-29), in stages 2
and 4.

- **The nonce.** Stage 2 derives `k1` exactly as `ecdsa_s2c` does (§1)
  and returns `R1` per slot. Stage 4 derives `k1` from `H_data(rho)`,
  never from the `c` the message carries, refuses when the two differ,
  and refuses when the `R1` the request carries differs from the one it
  derives. This is a code path apart from Faraday's ordinary signing,
  which grinds low R through RFC 6979's extra data: an anti-exfil nonce
  is never ground, so its signatures are 71 to 72 bytes with the sighash
  byte, where Faraday's ordinary ones are 71 or shorter.
- **Stateless.** Nothing is kept between the stages: `k1` is derived
  again from the key, the message and `c`. A stage 4 request needs no
  stage 2 in the same power-on.
- **Whole requests.** Faraday finds every key it holds on every input.
  A request with a Taproot input, a non-SegWit-v0 input, a sighash
  other than `SIGHASH_ALL`, or a finalized input under one of its keys
  fails whole. A reply never mixes ordinary and anti-exfil signatures.
- **Two reviews, one approval.** The stage 2 screen says it signs
  nothing. The stage 4 review is the full review of the transaction and
  its approval is the signing. Both show the transaction's fingerprint
  in large type for the person to compare with the host's.
- **Ordinary signing stays.** A PSBT that asks for no exchange is signed
  as today. Faraday refusing ordinary signing for a key adds nothing
  against a malicious Faraday: only the host can enforce protection
  (§4.4, rule 1).

The hosts that would check it, and what each protects against:

| Host | Where `rho` is drawn | Protects against | Does not protect against |
|---|---|---|---|
| FractalEncrypt's Sparrow fork (today) | the online computer | a backdoored Faraday whose author reads the chain | malware on the online computer that records the reveal or reads `rho` from memory |
| Sparrow, if it adopts the exchange (the goal) | the online computer | the same | the same |
| A second Faraday device, with the person's dice (yours only, §4.3) | offline, from the dice | a backdoored Faraday on the signing device, with `rho` never online; a backdoored host Faraday cannot choose `rho`, which the person computes again with another tool | a backdoored host Faraday that misreports its checks, unless the person checks them with another tool too; one build's backdoor shared by both devices, beyond `rho`'s choice |

## 6. The Nonce check

Spend's Send it card and Tools, Nonce check, sign each input again under
each nonce rule and say which one produced a signature. An anti-exfil
signature is deliberately not an RFC 6979 nonce: no rule reproduces it.

- **With the exchange.** When a signature arrives with its anti-exfil
  exchange (Faraday was its host, §4), the Nonce check makes the two
  checks of §1 for it instead and says so: "Anti-exfil: checked".
- **Without it.** The Nonce check stays as it is now.

Until anti-exfil exists, in dev builds only (§3), nothing changes in the
shipped app.

## 7. PSBT fields and the encoding

No BIP defines anti-exfil or fields for it.

- **In the dev build:** FractalEncrypt's four-stage protocol, its
  multi-slot AEXB records in the AEXT envelope, as an animated QR code
  under the UR type `x-btc-anti-exfil`, at the version that names the
  multi-slot layout. The older single-slot layout, which shares the
  magic and version byte, is refused. A message carries its version
  and a 32-byte session id, and stage 2 and 4 replies carry no PSBT.
- **Later:** whatever QR scheme Jade publishes, and FractalEncrypt's
  once it is published as a standard.
- **The recommendation** (review, 2026-10-29): the four-stage semantics
  carried in BIP-registered PSBT input fields, the host commitment, the
  signer commitment and the host reveal, with a protocol version and a
  session id; the host rules of §4.4 written as MUSTs; the reveal field
  removed before a PSBT leaves the host-to-signer path. Faraday intends
  to help write that draft BIP.
- **No proprietary fields of Faraday's own.** Accepting fields beyond
  BIP-174 is in dev builds only (F162). Once a BIP is published,
  Faraday reads its fields and the dev build's encoding goes.
  `docs/SIGNER-CHECK.md` §6.6 then treats the exchange's fields as known
  fields with checks of their own.

## 8. Tests

A cryptographic change, so tested against published vectors and
implementations Faraday did not write (F160):

- libsecp256k1-zkp's `test_ecdsa_s2c_fixed_vectors` and the module
  itself, pinned at or after `a69a662`;
- libwally's `wally_ae_host_commit_from_bytes`, `wally_ae_signer_commit_from_bytes`,
  `wally_ae_sig_from_bytes` and `wally_ae_verify`;
- btclib's `ecc.dsa` anti-exfil functions, vendored in
  `tools/reference/btclib/`, in a cross-check area of its own. btclib
  reproduces FractalEncrypt's vectors byte for byte.

FractalEncrypt's vectors are encoding fixtures only, labelled where
they are used: alpha, AI-written and AI-reviewed, no human audit. They
are not a cryptographic reference.

Refusals tested: a reveal that does not match `c`; a changed `R1`; a
high-S or out-of-range signature; a tweak at or past `n` (injected); a
signature over another sighash; a Taproot input in the request; a
request that mixes kinds of input; a reply with an unasked, repeated or
missing slot; a declared length past its cap; a second exchange with a
new `rho` for an aborted transaction. Kept: a retry with the same `rho`
returns the same signature; no `rho` is in any PSBT, file or vault
record after an exchange.

## 9. Questions for the owner

Decided (owner, 2026-10-29; F162): both directions; Faraday as host
first, then as signer; `rho` from Faraday's own randomness with every
source, and a yours-only mode; FractalEncrypt's encoding in dev builds
only behind `experimental`; Jade's and FractalEncrypt's published
schemes later; QR only, no USB for Jade; the host rules of §4.4; the
Nonce check of §6; the references of §8; the wording of §10.

Open:

1. How many rolls yours-only mode needs at least (50 rolls of a
   six-sided die are 129 bits; 99 are 256).
2. The abort count past which the screen says to move the funds
   (libsecp256k1-zkp's header suggests a strong warning after about 20
   in a device's life).
3. Whether the stage 2 review on Faraday as signer can be left out,
   since it approves nothing.

## 10. How it is described

In the docs, Learn and later the website:

- It is experimental, in dev builds only, and not in any release.
- Anti-exfil matters, because a signing device can carry a key out in
  the signatures themselves, and it needs to move toward a standard
  quickly. Faraday supports the published schemes as they appear and
  intends to help write a BIP for one.
- What each form protects and does not, as §2 and §5's table say: it
  does not catch a signer that lies about addresses, a malicious host
  that builds a bad transaction, or bad entropy.
