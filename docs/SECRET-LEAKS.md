# Secret leaks: tests that fail on a secret left in memory or on screen

**Kind:** Spec
**Status:** Specified (F250); the allocator, the flow tests, the tours' check and the clippy refusal of `clone`, `to_string` and `to_owned` (each secret-holding crate's own `clippy.toml`) are built, the rest is in its Open line.
**Open:** §6's refusal of `to_vec` and `String::from`.

Faraday keeps its secrets in types that wipe on drop (`Secret`,
`SecretText`, `Zeroizing`, `faraday-keys`' holders, `docs/KEYS.md`), and
`tools/lint-secrets.sh` keeps heap text out of the files that hold them.
Neither says whether a copy was made anyway: a `Vec` that grew and left
its old block, a `to_vec` into a type that does not wipe, a string made
for the screen. This document designs tests that find such a copy by
looking for the secret itself: in freed memory, in memory still live
after Lock, and on the screen after a frame that should have forgotten
it. It does for secrets what the budget tests of `docs/SPEED.md` §7 do
for work.

What wiping does and does not reach is in `docs/KEYS.md` ("What wiping
does not reach"), `docs/SECURITY-MODEL.md` ("What locking wipes, and
how") and `docs/ESTABLISHED.md` §4.9; it is not repeated here.

Everything here is test-only. No shipped program installs the allocator
(F181 holds: §8). The decision is F250.

## 1. What is checked

A test secret is planted in a flow and the flow is run. The test fails,
and names the form and the place, when:

1. **A freed block holds a form of the secret** (§3) at the moment it is
   freed or moved by a reallocation, and was not wiped first.
2. **A block still live after Lock holds one.** At the end of a flow that
   ends in Lock, the allocator scans every live block of the armed
   thread. After Lock no secret is kept (`docs/KEYS.md`; F195 point 2),
   so what is found was kept by mistake.
3. **A frame drawn after Lock, a hide or leaving a page shows one** in
   its text, or a frame kept for it holds one (§5).

A hit gives the form that matched (§3), its offset in the block, and for
a free the backtrace of the call that freed it. The scan reads bytes and
knows nothing of the type that owned the block, so it finds a copy made
by a library as well as one made by Faraday's code.

It finds copies in Rust's heap. A copy on the stack, in a register or in
a C library's own allocator is not seen (§8).

## 2. The allocator

A global allocator for test binaries, in a crate beside
`faraday/tools/alloc-count` (the counting allocator of the frame
budgets, which `faraday-core` takes as a dev-dependency). It has the
same shape: every call goes to the system allocator, and the extra work
is done on the thread that makes the call, so tests running side by side
in one process do not see each other's blocks or secrets.

- **Armed per thread and per test.** A test arms it with the secrets it
  planted (§3) and disarms it at its end. While disarmed a call costs
  one thread-local read. A test that does not arm it is not scanned.
- **On a free or a reallocation** the block is scanned for each form
  before it goes back to the system allocator. A block shorter than the
  shortest form is not scanned.
- **Live blocks** of an armed thread are listed (address and size only,
  never a copy of the contents) so that the scan after Lock (§1.2) can
  read them. The list does not allocate from the allocator it serves.
- **Worker threads.** The core's costly work runs on its worker
  (`docs/WORKERS.md`), and a block of a job may be freed there. The
  worker's thread is armed with the secrets of the test that started the
  job.
- **A scan is a byte search**, not a parse: each form is looked for at
  every offset.

The crate is a dev-dependency only, and a lint (§10) keeps it out of
every shipped manifest.

## 3. The test secrets and their forms

Each flow plants fixed test secrets, never random ones, so a failure
reproduces. They are published test vectors (the BIP-39 and codex32
ones the cross-checks use), never a key that stands for anything. A
secret is looked for in every form the code can hold it in:

| Secret | Forms |
|---|---|
| A seed's words | the phrase as text with single spaces; the indices as `u16` (both byte orders); the entropy with its checksum bits packed |
| Entropy | its bytes |
| The BIP-39 seed | the 64 bytes |
| The master key | the 32-byte key and the chain code; the 78-byte serialised form; its Base58Check text (`xprv`, `tprv`) |
| A child private key | the 32 bytes; its WIF; its text in a descriptor or a PSBT's key record |
| A codex32 secret | the string as text; its payload bytes |
| A SLIP-39 share or an aezeed | its words and its bytes |
| The passphrase, and a vault's | its UTF-8 bytes, in NFC and in NFKD; a PIN's digits |
| A vault item's secrets | each field's bytes; a TOTP secret as bytes and as Base32 |

A single word of a phrase is never searched alone: it is a word of a
public list. The test secrets are made long and unusual, so no form of one
matches by chance; a PIN is looked for only in its own tests (§9).

## 4. Which flows run under it

Each runs in the core's headless test harness (`testkit`), armed from
the planting of the secret to the end of the flow:

- **Seed load:** a seed typed, scanned, loaded from a vault and from the
  unlock's Load sheet; the unlock's passphrase.
- **Sign:** a PSBT reviewed and signed with a single key, a multisig key
  and a MuSig2 key.
- **Show and hide:** a seed's words, its SeedQR, a BIP-85 child, a vault
  key record, a paperkey and a TOTP code opened and hidden (F195).
- **Seal and open:** a vault sealed and opened, the file vault and the
  QR vault, with a Lock pressed while the seal runs.
- **Lock:** each of the above ended by Lock, with the scan of §1.2.
- **Backup:** a codex32 set, SLIP-39 shares and a SeedQR written and
  checked.
- **The vault readers:** a vault's entries listed, opened and copied.

A flow that handles a secret is added to this list (§10). The sessions
of the `work/budget` tests are the model: the secret is planted once, in
the heaviest session the flow reaches.

## 5. The screens check

The tours (`just faraday-snap`, `faraday/shells/snapshot`) walk each
screen's states. For the frame after each of **Lock**, **a hide** (a
secret opened and then hidden) and **leaving a page that showed one**,
the check reads:

- the text the frame drew (the lines the budget tests already count),
  against every form of §3 that is text;
- every frame the core keeps across frames: a sheet's frosted copy of the
  page under it (`Ui::frost`) and the fade's previous frame
  (`faraday-core/src/fade.rs`), and any other kept image. A kept image
  holds pixels, not text, so it is checked by what drew it: a kept image
  whose screen held a secret and that is still kept after the event is a
  hit.

The tours plant the §3 secrets in the sessions that open a secret
screen. A hit fails the tour and names the tour, the step and the form.
It runs on every step of those three kinds, so a screen added later is
covered without its author listing it, as the Lock-during-seal change
had to list one screen by hand.

## 6. The types and what refuses a copy

- **No derived `Clone`, `PartialEq`, `Debug` or `Display`** on a type
  that holds a secret. A copy is written where it is needed, and a
  comparison goes through a function of the type that does not branch on
  the bytes. `lint-secrets` rule 2 holds the `Debug` derive today for a
  list of type names and one-by-one exceptions; it becomes a rule on
  every secret type, found as below.
- **`clippy::disallowed_methods`** in the crates that hold secrets
  refuses `to_vec`, `clone`, `to_owned`, `String::from` and `to_string`
  on a secret buffer (`Secret<_>`, `SecretText`, `Zeroizing<_>`, a
  `faraday-keys` holder). The repository has no such list today. The
  lint names methods, not types, so where it cannot tell the type the
  refusal covers the method in the whole module, and a legitimate use
  carries `#[expect(clippy::disallowed_methods, reason = "...")]`.
- **`lint-secrets` finds a secret file by the types it uses**, not only
  by its place in `SECRET_FILES`: a file that names a secret type
  (`Secret`, `SecretText`, `Zeroizing`, a `faraday-keys` holder,
  `SeedBytes`, rule 4a's list) is held by rule 3 (no heap text) like the
  listed ones, and a listed file that names none is reported, so the
  list holds only what it should. This closes the gap the script's own
  header states: a new module is held only once someone adds it.

## 7. Allowed entries

A hit that is known and accepted is let through by a named entry in one
file under `tools/`, read by the allocator. An entry gives:

- the **form** and the **place**: the function of Faraday's or the
  library's that frees the block, as it appears in the backtrace;
- the **reason**, in a sentence, and what bounds the copy (for instance
  the restart at every lock, F181);
- the **fix** it waits for: a path in the repository, or a statement
  that none is planned. An entry is never only a silence.

This is how F136 names a hold: a way past a check is a name with the
case it is for, and nothing passes without one. An entry no test hits
any more fails the test, so the list only falls as copies go. A new
entry is made only in a commit that says why, and passes review.

## 8. What stays review-only, and what is not done

- **Stack temporaries.** A secret that passed through a local or a
  register leaves bytes on the stack that no allocator sees. A test that
  paints the stack and looks afterwards is not proposed: its result
  depends on the compiler and the target, and what it finds is gone at
  the next call. A change to how a secret is handled locally is read for
  it (a review rule, §10).
- **Timing, and loads indexed by a secret.** Under the Security scope
  (`docs/SECURITY-MODEL.md`, "What this does not defend against") these
  are worth having and later work, recorded as open work and done when
  the app is mature. No test here claims to find one.
- **Memory seen from outside the process** (cold boot, the kernel, the
  management engines) is out of scope, as there.
- **Faraday Demo and the Starter** run on an online computer and hold
  test data: no leak work is done for them alone (F181 point 3). The
  tests run in the crates they share with the device.
- **Nothing is shipped.** The allocator is a dev-dependency, no shipped
  program installs it or any other global allocator, and F181's rule
  (`speed.no-wiping-allocator`) holds as it is. If the hits argue for
  wiping every free, that is a new decision with F181 point 5's numbers.

## 9. The owner's choices (owner, 2026-11-12)

- **Every freed copy counts**, however briefly it lived: a temporary
  made and dropped inside one function is a hit like any other.
- **A freed-block hit fails the test**, and may be let through by a
  named entry (§7) that names what bounds it, the restart at every lock
  (F181 point 2), and the fix it waits for. A block still live after
  Lock, and a secret on screen after Lock, a hide or leaving a page,
  have no entry: they always fail.
- **Short secrets.** The test secrets are long and unusual, so a form
  matches only where the secret is; a PIN is looked for only in tests of
  its own.
- **Allowed entries** live in one file under `tools/`; an entry is added
  only in a commit that says why, and passes review.

## 10. What the registry holds

`tools/impact.toml` holds this so `just lint` does and nobody has to
remember:

- a change to a flow that touches a secret (§4's list, and the files of
  `faraday-keys`, `faraday-vault` and the seed and backup screens) runs
  under the leak tests and the tour check: a **held** rule;
- a secret type follows §6: a **held** rule through clippy and
  `lint-secrets`;
- a change to an allowed entry or to the forms of §3 names its reason,
  and a manifest never lists the allocator, `faraday-leak-scan`, outside
  a `dev-dependencies` table (`lint-secrets`, rule 6): a **review** rule
  and a **held** one;
- a change to how a secret is handled in a local, indexed by or compared:
  a **review** rule that says what the tests cannot see (§8).

`docs/CHECKS.md` is generated from the registry and shows them.
