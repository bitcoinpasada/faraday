# Restore a GPG key from its paperkey

**Kind:** Spec
**Status:** Specified from the owner's ask (2026-10-15); P1 to P3 built (reading a paperkey and a certificate, the restore and its tests against paperkey and GnuPG in `faraday-pgp`; the restore set in `faraday-keys`; the flow in faraday-core, `faraday/faraday-core/src/backup_flow/gpg_restore.rs` and its screen); the decision is drafted in §10 as F-next, for the owner
**Open:** batches P4 to P6 (§11.2): scanning the public key and a paperkey code, and the docs

A person whose GPG key's vault is lost, but who wrote down its paperkey,
brings the key back into a vault and signs with it again. Today the
paperkey is only shown (F32, the key's page, held to show), and the key
can be rebuilt only in GnuPG with `paperkey --pubring … --secrets …` and
`gpg --import` (`docs/learn/faraday/gpg.md`, "The paperkey"). Faraday has
no way to take a GPG secret key into a vault.

The owner's ask (2026-10-15):

- The person has the public certificate. It is published, so it is an
  input, not something Faraday rebuilds from nothing.
- The paperkey's lines are typed (or scanned) against it, and each
  line's CRC-24 points at a mistyped line.
- The result is the key's vault record (`docs/VAULT.md` §7, type 7) as
  Make would have written it, so it signs, renews and revokes as
  before.

Sources read for it, each as data and none copied into the repository:

| Source | Where | Licence | Read |
|---|---|---|---|
| paperkey 1.6, David Shaw | https://www.jabberwocky.com/software/paperkey/ (the official page: 1.6 is the latest, and the source is the repository below) | GPL-2.0 (`COPYING`; the sources say "version 2, or (at your option) any later version") | 2026-10-23 |
| paperkey's source | https://github.com/dmshaw/paperkey, tag `v1.6` (commit ce2666f, 2018-10-19) and `main` (6ddd767, 2021-01-31); `output.c`, `extract.c`, `parse.c`, `restore.c` and `paperkey.c` are the same at both | GPL-2.0 | 2026-10-23 |
| paperkey's README | `README.md` in that repository | GPL-2.0 | 2026-10-23 |
| paperkey's man page | `paperkey.1` in that repository | GPL-2.0 or later | 2026-10-23 |
| paperkey's checks | `checks/` in that repository: `roundtrip.sh`, `roundtrip-raw.sh`, `bad-crc-base16.sh`, `bad-crc-raw.sh`, `bad-primary.sh`, `bad-subkey.sh`, and the keys `papertest-{rsa,dsaelg,ecc,eddsa,100,dsa100}.{pub,sec}` | GPL-2.0 | 2026-10-23 |
| OpenPGP | RFC 4880 (§5.5.3 secret key packets, §6.1 CRC-24), RFC 9580 (version 4 keys, the EdDSALegacy algorithm) | IETF | as already cited by `faraday/faraday-pgp/src/lib.rs` |

## 1. The rules

- **Refuse what cannot be verified.** The paperkey and the certificate
  are taken only when every byte of the record that results is proven:
  the secret makes the certificate's public keys, and every signature
  of the key kept in the record is one the restored key makes itself
  (§4.3). Nothing is guessed: no fingerprint, no expiry, no user ID.
- **The record is Make's.** A key restored is the type 7 record that
  Make wrote for it: the same seeds, dates, user IDs, expiry, and the
  certificate and kept revocation byte for byte (§5.2). What follows
  from the record (Sign a file, Renew, Make a revocation, the paperkey
  shown) is unchanged.
- **GnuPG's and paperkey's behaviour** where they have one (F32): the
  line format, the CRC checks and their words are paperkey's (§2).
  Where Faraday refuses what paperkey would take, §10 records it.
- **Offline.** The certificate comes in by a stick or a camera, never a
  network (§3).
- **The backup flow** (F31). Restore takes the backup flow's steps, as
  Restore of a wallet does: the plan, the sticks, the vaults, then the
  key at step 4, typed (`docs/BACKUP-FLOW.md` §1, "Restore, which makes
  nothing new, takes the same steps with its seed typed or scanned at
  step 4").
- **The key boundary** (F70). The paperkey's bytes live in
  `faraday-keys` from the moment each line is checked; no new reveal is
  added (§6).

## 2. The format

paperkey keeps only the secret parts of an OpenPGP secret key, because
"the secret key contains a complete copy of the public key" and the
public key "generally doesn't need to be escrowed" (README, "What
paperkey does"). To restore, the bytes are entered again "and paperkey
uses them to transform your existing public key into a secret key". So
the public certificate is needed with the paperkey: the paperkey holds
no public key, no user ID, no date and no signature, only, per key, its
fingerprint and its secret part. Without the certificate the secret
cannot be turned back into a key that others recognise: the creation
times and user IDs that the fingerprint and the self-signatures cover
are not in it.

### 2.1 The data

As `output.c`'s `output_file_format` states it, and `extract.c` writes
it:

| Field | Bytes | What |
|---|---|---|
| a | 1 | The paperkey format's version: 0 |
| b | 1 | The key or subkey's OpenPGP version: 4 |
| c | 20 | The key's fingerprint (version 4) |
| d | 2 | The length of e, big-endian |
| e | d | The secret part of the secret key packet (RFC 4880 §5.5.3): from the string-to-key usage octet to the end of the packet |

Fields b to e repeat for each subkey, the primary key first. paperkey
does no cryptography: a key protected by a passphrase gives a paperkey
protected by the same passphrase (README, "Security"; the man page,
"Security considerations"). paperkey 1.6 reads version 4 keys only; it
says "Version 3 (PGP 2.x style) keys are not supported." and refuses
any other version without a message (`parse.c`, `extract_secrets`).

For a key Faraday makes, e is the unprotected form Faraday writes
(`Key::secret_part`, `faraday/faraday-pgp/src/lib.rs`): the usage octet
0, the seed as an MPI (its bit count, then the seed with its leading
zero bytes dropped), and the two-octet sum of the MPI's bytes. The
whole is at most 1 + 2 × (1 + 20 + 2 + 37) = 121 bytes.

### 2.2 The text form (base16)

The form paperkey prints by default and Faraday shows
(`Key::paperkey`):

- each line is its number, right-aligned in three places and a colon
  (`"%3u: "`), then bytes in upper-case hex each followed by a space
  (`"%02X "`), then the CRC-24 of that line's bytes in six hex digits
  (`"%06lX"`);
- a line holds `(output_width − 11) / 3` bytes; the width is 78 unless
  `--output-width` says otherwise, so 22 bytes;
- after the last data line, one more line: its number and the CRC-24 of
  the whole data, with no bytes (`"%3u: %06lX"`);
- paperkey puts comment lines first, each starting `#`: "Secret
  portions of key" and the fingerprint, the date extracted, the
  version, the file format above, and "Each base16 line ends with a
  CRC-24 of that line." Faraday's own display has no comment lines.

A key of Faraday's is seven lines: line 1 is fields a, b and c of the
primary key (22 bytes, all public), lines 2 to 6 the rest, line 7 the
whole data's CRC.

The CRC-24 is OpenPGP armour's (RFC 4880 §6.1: initial value 0xB704CE,
polynomial 0x1864CFB), `faraday_pgp::crc24`.

How paperkey reads it back (`parse.c`, `read_secrets_file`): it skips
lines starting `#` and empty lines; the number before the colon must be
the next line's ("Error: missing line number %u (saw %u)"); each line's
last token is its CRC, compared with the CRC of its bytes ("CRC on line
%d does not match (%06lX!=%06lX)"); a line with a CRC and no bytes is
the whole-data CRC ("CRC of secret does not match", and with none,
"CRC of secret is missing"). `--ignore-crc-error` goes on regardless;
Faraday has no such choice.

### 2.3 The raw form

`--output-type raw` writes the data unchanged and then the whole data's
CRC-24 as three bytes, high byte first (`output.c`). It is meant "for
feeding to barcode generators or the like" (the man page). Reading it
back takes the last three bytes as the CRC (`parse.c`). It has no line
CRCs.

### 2.4 How paperkey restores

`restore.c`: it reads the entries (a version other than 0 is "Cannot
handle secrets file version %d"; a field b other than 4 is "Corrupt
data in secret image"), then walks the public keyring and, for each
public key or subkey whose fingerprint matches an entry, writes the
public packet with the secret part appended and its tag changed to a
secret one (6 to 5, 14 to 7); user IDs and signatures are copied as
they are. A public key with no entry, or an entry with no public key,
is passed over in silence. paperkey does not check that the secret
makes the public key it is joined to; Faraday does (§4.3).

## 3. Where the public key comes from

Faraday is offline. The certificate reaches it as any public file does:

- **The stick.** The file Faraday wrote when the key was made or
  renewed, `{name}-{fp8}.asc`, kept on the stick of public files or any
  other; copied in on a stick visit, it is in Files' From the stick as a
  public file (F22).
- **A file from elsewhere.** An armoured export (`gpg --armor --export
  FINGERPRINT`), or a key server's download saved on another computer
  and copied onto a stick: the same row in Files.
- **A scanned code.** Faraday shows the public key as one code (**Show
  as QR**, `docs/QR.md` §4); the scanner reads it back as a public key
  (P4, §11.2) and files it in From the stick as a public file.

What is read is one ASCII-armoured `PGP PUBLIC KEY BLOCK` that F22's
reader takes as public (`faraday_pgp::read_public`: every packet to its
exact end, the checksum line there and right). A binary export
(`gpg --export` without `--armor`) is not read today (Q7).

## 4. The flow

### 4.1 Ways in

- **Files.** A public key in From the stick whose certificate is of the
  shape Faraday restores carries **Restore from its paperkey** in its ⋮,
  beside Show as QR (decision 24: a file carries the action that uses
  it).
- **Vault contents' GPG kind**: **Restore from a paperkey**, under
  **Make a key**.
- **Tools' GPG key** tile: its line becomes "Make, restore, sign,
  revoke, paperkey".

From Vault contents or Tools, a sheet **Restore a GPG key** lists every
public key in Files that reads as a certificate (§4.3, the certificate's
checks), a row each, its user ID and its fingerprint in groups of four,
and **Scan the public key**. With none: "Copy its public key in on a
stick visit first." and **Scan the public key**. A row pressed, or the
Files action, opens the flow on that certificate.

In Practice the ways in are dimmed, as Make is (`docs/DEMO.md` §4); Q9.

### 4.2 The cards

The backup flow (`Screen::BackupFlow`, DESIGN §5, step flow), titled
**Restore a GPG key**, its chip the fingerprint's last eight digits.
The certificate is chosen before it opens, so its steps are Make's
seven:

0. **The public key**, as the flow's head card, closed to a line once
   read: user ID, fingerprint, signing subkey, created, expires
   (DESIGN §4.11, Fact), and "Revoked on {date}" where the certificate
   carries a revocation by the key; then **Line 1 of its paperkey**,
   the 22 bytes that line must read (`00 04` and the fingerprint) as a
   code box (§4.3), so the person can tell at a glance that the paper
   in hand is this key's. A certificate refused (§4.4) shows the reason
   as an error line and has no **Continue**; the flow opens only on a
   certificate that reads.
1. **Where it will be kept**, 2. **Bring in the sticks**, 3. **Vaults**:
   Make's (`docs/BACKUP-FLOW.md` §4.1 to §4.3), two places or more (Q3).
   The certificate is public and is kept in the flow's state across
   step 2's lock, with the plan (F31).
4. **Type the paperkey** in place of the name and expiry form (§4.5).
   With every line checked, **Restore the key** (Primary): §4.3's
   checks, then the record saved into every vault the plan keeps it in,
   in one action; the Result reads "Restored into Vault 1", a row per
   vault, and "{n} signatures by other keys are not kept" where §4.3
   set any aside. A key already in an open vault is refused here:
   "This key is in {vault}", with the vault's button; nothing is saved.
5. **Lock**, 6. **Write the sticks**, 7. **Done**: Make's. Done leads to
   the key's page.

On the small panel each step is a page, "Step 4 of 7", as Make's are.

### 4.3 What is checked, and when

Each check runs at the earliest point it can, and a check that fails
stops there with its reason.

**The certificate** (when it is chosen; public, costs nothing secret):

1. One armoured public key block that F22's reader takes.
2. A version 4 primary key with the EdDSALegacy algorithm (22) on
   Ed25519, and exactly one subkey, version 4, EdDSALegacy on Ed25519,
   whose binding signature gives it the signing flag (0x02). Anything
   else is refused by what it is (§4.4).
3. At least one user ID.
4. Each signature is sorted by its issuer: by the primary key (a user
   ID's certification, the subkey's binding, a key revocation), or by
   another key (a certification someone else made). Signatures by other
   keys are set aside: Faraday cannot check them, the record does not
   keep them, and the Result counts them (Q4).

**Each line** (as it is completed, §4.5; in `faraday-keys`, §6):

5. Its digits are pairs, then six for the CRC; the CRC matches its
   bytes ("CRC on line {n} does not match").
6. Line 1: the format version 0, key version 4, and the fingerprint the
   certificate's primary key ("This paperkey is for key {FPR}", the
   fingerprint read from it in groups of four).
7. Once the primary's length is read: the string-to-key usage octet is
   0 ("Protected by a passphrase", §4.4).
8. Once the second entry's header is read: version 4 and the
   certificate's subkey fingerprint ("This paperkey's subkey is {FPR}").
   A third entry is refused: "The paperkey holds {n} keys; the public
   key has 2".
9. The last line: the whole data's CRC ("CRC of secret does not
   match"). Every line is asked for, the CRC line included, so a
   paperkey is never taken without it ("CRC of secret is missing" is
   never reached).

**The key** (at **Restore the key**; costs Ed25519 work):

10. Each secret part: an MPI of at most 256 bits, nothing after its
    checksum, the checksum its bytes' sum ("The paperkey's secret does
    not read").
11. Each seed, its leading zeros put back to 32 bytes, makes the public
    point of the certificate's key with that fingerprint ("The secret
    does not match the public key"). paperkey does not check this; a
    paperkey whose CRCs were made over wrong secret bytes would pass
    paperkey and give a key that cannot sign.
12. **Every signature of the key's, rebuilt.** Ed25519 is deterministic,
    so the restored key makes the same bytes again. For each signature
    by the primary key in the certificate Faraday reads its time, and
    for a self-signature its key expiry, for a revocation its reason
    and text; it makes the signature Faraday makes from those
    (`Key::certificate` at that time and expiry, `Key::revocation` with
    that reason), and the two must be equal, byte for byte. Where one
    differs: "Not a certificate Faraday made for this key" (Q1). The
    newest self-signature gives the record's expiry and is the one the
    record's certificate holds; older ones, rebuilt and equal, are not
    kept (a key server may hold a key's earlier self-signatures beside
    its renewed ones).

So the record holds nothing that was not made again from the seeds and
compared: no signature is verified by trusting a parser; each is made
afresh and matched.

### 4.4 What is refused

| What came in | Refused with |
|---|---|
| A file that is not one armoured public key block F22 takes | it is not offered (Files does not show the action) |
| A key of another algorithm | "{algorithm} keys are not restored here: Faraday's are Ed25519", the algorithm as `gpg --list-keys` names it (rsa4096, dsa2048, nistp256, cv25519, ed448) |
| A key of version 3, 5 or 6 | "Version {v} keys are not restored here" |
| No subkey, two or more subkeys, a subkey that does not sign, an encryption subkey | "Not a key of Faraday's shape: an Ed25519 key with one Ed25519 signing subkey" |
| No user ID (a key server that strips unverified addresses) | "The public key has no user ID" (Q11) |
| A wrong line CRC | "CRC on line {n} does not match", the line kept for correcting |
| A line number typed that is not the line's | "This is line {n}, not line {m}" |
| A paperkey for another key | "This paperkey is for key {FPR}" |
| A subkey that differs | "This paperkey's subkey is {FPR}" |
| More or fewer keys than the certificate | "The paperkey holds {n} keys; the public key has 2" |
| A secret protected by a passphrase (usage 254, 255 or any other non-zero) | "Protected by a passphrase: Faraday restores keys with none" |
| The whole data's CRC wrong | "CRC of secret does not match" |
| A secret that does not read | "The paperkey's secret does not read" |
| A secret that does not make the public key | "The secret does not match the public key" |
| A signature of the key's that Faraday does not make the same | "Not a certificate Faraday made for this key" |
| The key already in an open vault | "This key is in {vault}" |

Each is an error line (DESIGN §4.3) under what it is about: the
certificate's card, the line, or the step's button.

### 4.5 Typing a line

Step 4's card holds the lines as rows, one row per line, in the mono
face:

- the line's number, "Line 3 of 7" (the count shows once the lengths
  are read; line 3 at the latest);
- the line being typed: its bytes as pairs with a space between, as on
  paper, then a wider gap and the CRC's six digits, the caret where
  typing goes (DESIGN §4.3, Text field). Lines too wide for the window
  wrap after 11 bytes;
- a line checked: "Line {n} · checked" in `OK`, and nothing of its
  bytes; line 1 also shows the fingerprint it names, which is public. A
  checked line is not shown again (§6); **Type line {n} again** in its
  ⋮ clears it and the lines after it;
- the line with an error: its digits as typed and the error line under
  it in `ERR`.

Keys:

- On the screen, the existing hex keyboard (`KeyboardKind::Hex`,
  `core/faraday-draw/src/widgets/keyboard.rs`), docked as the passphrase
  keyboard is (DESIGN §4.3, Keyboard), with **Next line** beside the
  field.
- On a keyboard, 0 to 9 and A to F in either case. Spaces are taken and
  ignored. A colon makes what came before it on the line its number,
  checked against the line's (so a person who types "  3: " as printed
  is not misread). Any other character: "Only 0–9, A–F, spaces and the
  line's number".
- A line ends when **Next line** or Enter is pressed, or by itself at
  22 bytes and a CRC that matches, and at the length the headers give
  for the last data line. Another line width than 78 is taken: a line
  is what the person ends with Next line.
- Backspace inside a line edits it; at the start of an empty line it
  does nothing (a checked line is not reopened by a key; §6).

No line is guessed or corrected (Q8). A mistyped line stays as typed,
with its error, until it is changed.

### 4.6 Signing after a restore

The restored key signs at once, as a restored wallet's keys do
(`docs/BACKUP-FLOW.md` §11): its signatures were already out before
the vault was lost, and the paperkey it came from is a copy that
exists. Q2 asks the owner whether it should wait for the plan's writes
as Make's does instead (F31).

## 5. The record

### 5.1 Its fields

`docs/VAULT.md` §7, type 7, as Make writes it:

| Field | From |
|---|---|
| 1 primary seed | the paperkey's first entry |
| 2 primary creation time | the certificate's primary key packet |
| 3 subkey seed | the paperkey's second entry |
| 4 subkey creation time | the certificate's subkey packet |
| 5 user IDs | the certificate's user ID packets, in their order |
| 6 expiry | the newest self-signature's key expiry (0 where it has none) |
| 7 certificate | `Key::certificate` at the newest self-signature's time: the bytes compared in §4.3 step 12 |
| 8 revocation certificate kept | `Key::revocation` at the primary's creation time with no reason and no text: the bytes Make wrote, since Make makes it at once (`faraday/faraday-keys/src/gpg.rs`, `create`) |
| 9 marked revoked | the time of a revocation by the key in the certificate, where there is one |

### 5.2 The same record as Make's

For a key Make made and never renewed, the record restored is Make's
byte for byte: the seeds, the dates, the user ID and expiry, the
certificate (made at the creation time) and the kept revocation (made
at the creation time). For a key renewed, field 7 is the renewed
certificate, as the record held after Renew. A key made before the
revocation was kept with it (F32's "a key with none makes one when its
page is first opened") had one made at a later time; the restore makes
it at the creation time, which revokes the key the same way.

Nothing needs the clock: every time in the record is read from the
certificate. Restore does not ask for the time first (decision 58).

## 6. How the key is held

- **The line being typed** is transient input (F70; `docs/KEYS.md` §2):
  it stays in the field where it arrives, under lint-secrets' rules,
  and is wiped when the line is handed in or the step closes.
- **A line ended** is handed at once to `faraday-keys`: a new restore
  set beside the codex32 work set (`WorkSet`, `docs/KEYS.md` §3), in
  faraday-keys' `gpg` module (`faraday/faraday-keys/src/gpg.rs`). It
  decodes the digits, checks the CRC and the headers (§4.3 steps 5 to
  9), and keeps the line's bytes in wiped memory, or returns the reason
  and keeps nothing (the field keeps its text for correcting). What it
  tells faraday-core is public: the line count, which lines are
  checked, the fingerprints read, the lengths, the error.
- **The key** is made inside faraday-keys at **Restore the key**
  (`Restore::finish`, costs Ed25519 work, takes the compute token) and
  comes back as Make's does: a `vault::MadeKey`, saved into each vault
  of the plan through the existing copy (`IntoRecord for &MadeKey`),
  with the GPG facts and certificate beside it. Nothing secret crosses
  the boundary.
- **No new reveal.** The lines checked are never shown again, so
  nothing leaves to be drawn; the paperkey of the restored key is
  shown, as for any key, through the existing `Reveal::GpgPaperkey`
  (`faraday/faraday-keys/src/session.rs`). The reasons of F70 (fourteen
  since F125) stand.
- **Wiped** when the flow is cancelled, at the idle lock and at step 5's
  lock, with every other secret. The flow's state keeps the certificate
  and the plan (public) across a lock; the lines typed are not kept,
  and a lock before **Restore the key** asks for them again.
- `tools/lint-secrets.sh` rule 4(c) gains the new secret-taking
  functions of `faraday-pgp` (the paperkey reader and the restore) and
  the restore set, so that only faraday-keys calls them.

## 7. Code

### 7.1 Where things live

| Where | What |
|---|---|
| `faraday/faraday-pgp/src/` (a new module, paperkey.rs) | The text and raw readers (§2.2, §2.3): a line's digits to bytes and CRC; entries from the data. No state, nothing kept |
| `faraday/faraday-pgp/src/` (a new module, certificate.rs) | Reading a certificate into packets by role (§4.3 steps 1 to 4), naming an unsupported algorithm as GnuPG does; the rebuild and compare (step 12); `restore`, which joins entries and certificate into a `Key` and the record's public values |
| `faraday/faraday-keys/src/gpg.rs` | The restore set: `Restore::new(certificate)`, `line(n, digits)`, `finish(c)`; wiped on drop |
| `faraday/faraday-core/src/gpg.rs` | The certificate's facts for the cards; the refusals' strings |
| `faraday/faraday-core/src/backup_flow/` | `Kind::Gpg` with the restore's certificate in the flow's state; step 4's card and its keys; the sheet of §4.1 |
| `faraday/faraday-core/src/vault/screen.rs`, `faraday/faraday-core/src/screens/catalog.rs` | **Restore from a paperkey**; the tile's line |
| `faraday/faraday-strings/src/en.rs` | §7.3 |

### 7.2 Costs and drawing

- Reading a certificate is a parse of a few hundred bytes: not costly.
  A line handed in costs a CRC over 25 bytes at most: not costly, and
  done in the event, not in drawing.
- `Restore::finish` costs two Ed25519 public keys and one signature
  rebuild per signature of the key (three Ed25519 signatures for a
  certificate made once, and one more for the kept revocation); its doc's first line says so and it counts
  itself through `c: &Compute` (`docs/COMPUTE.md`).
- Drawing reads the restore's public state kept on the flow (the lines'
  states, the count, the error), updated at each event; the
  certificate's facts are read once when it is chosen and kept with it.
  The budget tests in `faraday/faraday-core/src/work/budget/` gain a
  key press in step 4 that does no costly work.

### 7.3 Strings

In `faraday/faraday-strings/src/en.rs`, prefix `gpg_restore_`:

| Name | Text |
|---|---|
| `gpg_restore_action` | Restore from a paperkey |
| `gpg_restore_files_action` | Restore from its paperkey |
| `gpg_restore_title` | Restore a GPG key |
| `gpg_restore_none` | Copy its public key in on a stick visit first. |
| `gpg_restore_scan` | Scan the public key |
| `gpg_restore_public_key` | The public key |
| `gpg_restore_line_one` | Line 1 of its paperkey |
| `gpg_restore_card` | Type the paperkey |
| `gpg_restore_line` | Line {n} |
| `gpg_restore_line_of` | Line {n} of {total} |
| `gpg_restore_checked` | Line {n} · checked |
| `gpg_restore_next` | Next line |
| `gpg_restore_again` | Type line {n} again |
| `gpg_restore_button` | Restore the key |
| `gpg_restore_into` | Restored into {vault} |
| `gpg_restore_set_aside` | {n} signatures by other keys are not kept |
| `gpg_restore_chars` | Only 0–9, A–F, spaces and the line's number |
| `gpg_restore_number` | This is line {n}, not line {m} |
| `gpg_restore_crc` | CRC on line {n} does not match |
| `gpg_restore_crc_all` | CRC of secret does not match |
| `gpg_restore_other_key` | This paperkey is for key {fpr} |
| `gpg_restore_other_subkey` | This paperkey's subkey is {fpr} |
| `gpg_restore_count` | The paperkey holds {n} keys; the public key has 2 |
| `gpg_restore_protected` | Protected by a passphrase: Faraday restores keys with none |
| `gpg_restore_unreadable` | The paperkey's secret does not read |
| `gpg_restore_mismatch` | The secret does not match the public key |
| `gpg_restore_not_ours` | Not a certificate Faraday made for this key |
| `gpg_restore_algorithm` | {algorithm} keys are not restored here: Faraday's are Ed25519 |
| `gpg_restore_version` | Version {v} keys are not restored here |
| `gpg_restore_shape` | Not a key of Faraday's shape: an Ed25519 key with one Ed25519 signing subkey |
| `gpg_restore_no_user_id` | The public key has no user ID |
| `gpg_restore_in_vault` | This key is in {vault} |
| `gpg_restore_format` | This paperkey is in format {v}; paperkey writes format 0 |
| `gpg_restore_not_public` | Not a public key Faraday reads |

The revoked line reuses the key page's. The Learn page carries the why
(§9).

## 8. Tests

Each test says what a person or another tool would notice. Fixtures are
made by the real tools in a throwaway `GNUPGHOME`, with
`--faked-system-time` for every time, as F32's tests are, and kept in a
new folder tools/vectors/paperkey/ with a README naming each file's
command, the tools' versions (paperkey 1.6, GnuPG 2.4) and its SHA-256,
as `tools/vectors/bip93/` records its files. The tests that run the
tools themselves fail where they are missing, as
`faraday/faraday-pgp/tests/gnupg.rs` does; nothing skips them.

**Against paperkey** (`faraday-pgp`, a new tests/paperkey.rs):

1. The lines Faraday shows for a key, typed back, give the entries the
   key was made from, for seeds with leading zero bytes (a shorter MPI,
   as F32's test has).
2. paperkey 1.6's text output (`paperkey --secret-key`) and raw output
   (`--output-type raw`) of `Key::secret_key` read to the same entries
   as Faraday's own lines; the fixtures are those outputs, comment
   header included.
3. paperkey 1.6's restore (`paperkey --pubring cert.gpg --secrets
   lines.txt`) of the lines Faraday shows gives `Key::secret_key`, byte
   for byte: Faraday reads what paperkey reads, and paperkey agrees with
   Faraday's restore.
4. One digit changed on each line in turn: Faraday refuses on that
   line, and paperkey refuses with "CRC on line {n}" for the same `n`.
   The whole-data CRC changed: both refuse ("CRC of secret does not
   match"). The raw form with a byte changed: refused, as paperkey's
   `bad-crc-raw.sh` refuses it.
5. paperkey's own EdDSA check key (`papertest-eddsa`, an Ed25519 primary
   that certifies and signs, with no subkey) read through paperkey's own
   output: one entry with that key's fingerprint, and the restore
   refuses it as not of Faraday's shape. The key is read from
   paperkey's repository at test time only where Q10 allows; otherwise
   a key of the same shape made by GnuPG stands in.

**Refusals, with keys made by GnuPG 2.4:**

6. `gpg --quick-generate-key` with its defaults (an Ed25519 primary and
   a cv25519 encryption subkey): refused by its shape, naming cv25519.
7. An `rsa3072` key: refused by its algorithm.
8. An Ed25519 key with one Ed25519 signing subkey and a passphrase: its
   paperkey refused as protected.
9. The same shape with no passphrase: refused as "Not a certificate
   Faraday made for this key" (or restored, if Q1 says so).
10. The lines of key A with the certificate of key B: refused at line 1.
    A's primary with B's subkey (the certificate of a key whose subkey
    was replaced): refused at the subkey's header.
11. A paperkey whose secret bytes are another seed with CRCs made over
    them: refused as not matching the public key. What paperkey and
    `gpg --import` do with the same lines is run and recorded in the
    test, so the difference from paperkey is shown, not assumed.

**Certificates as they arrive:**

12. A Faraday key renewed, its renewed certificate: the record has the
    new expiry and that certificate.
13. The two certificates of a key, before and after a renewal, merged by
    `gpg --import` of both and `gpg --armor --export`: the newest
    self-signature gives the record; the older one is rebuilt, equal and
    not kept.
14. A certificate with a revocation by the key (made with **Make a
    revocation**, imported into gpg and exported): restored, marked
    revoked at its time; it signs nothing.
15. A certificate a second GnuPG key has certified (`gpg
    --quick-sign-key`): restored; the certification is set aside and
    counted.

**The record** (`faraday-keys`, `faraday/faraday-keys/tests/vault.rs`):

16. A key made by `gpg::create`, its paperkey revealed and typed back
    against its certificate: the record restored equals Make's, field
    for field and byte for byte, the kept revocation included.
17. The restored key signs a file with the same bytes as the original
    record does at the same time, and `gpg --verify` takes it.
18. Renew and Make a revocation on the restored record give the same
    certificate and revocation as on the original.

**The flow** (`faraday/faraday-core/tests/all/gpg.rs`):

19. A certificate in Files, Restore from its paperkey, the plan, the
    vaults, the lines typed as key presses: a mistyped line shows its
    error and does not advance; once fixed, the flow goes on; **Restore
    the key** saves into every vault of the plan; the key's page shows
    the same fingerprint and signs at once (or after the writes, by
    Q2).
20. A lock at step 4 wipes the lines; the flow comes back to step 4
    with the certificate and asks for line 1 again.
21. The key already in an open vault: refused, nothing saved.
22. A budget test: a key press on step 4 does no costly work.

**Fuzzing:** two new targets in `fuzz/fuzz_targets/`, beside
`pgp_armor`: the paperkey text and raw readers, and the certificate
reader with the restore over arbitrary entries (`fuzz/README.md`).

**Snapshots:** a tour, gpg-restore, of the sheet, the head card with
line 1, step 4 with a line checked, one typed and one in error, and a
refusal, at the sizes `docs/LAYOUT.md` names.

F79's cross-check has no OpenPGP reference; GPG is judged by GnuPG and
paperkey themselves (F32), and these tests are that.

## 9. Learn and Guide

`docs/learn/faraday/gpg.md`, "The paperkey", gains how a key comes back:
the public key from the stick, a file or a code, the lines typed with
their CRCs, why the public key is needed, and that only keys Faraday
made, with no passphrase, are restored. The GnuPG commands stay for a
computer that is not Faraday.

## 10. Decision draft: F-next

The number is assigned when it is committed.

**F-next A GPG key is restored from its paperkey and its public
certificate** (draft, 2026-10-23)

**Why.** A GPG key made in Faraday cannot be made again from seed
words. Its paperkey is shown so that it can be written down (F32), but
only GnuPG could turn it back into a key: a person who lost the key's
vault had no way to bring it back into Faraday.

**Decision** (the owner's ask, 2026-10-15; the spec is
`docs/GPG-PAPERKEY.md`).

- A GPG key is restored from its paperkey's lines and its public
  certificate, through the backup flow (F31), as a wallet is restored:
  the plan, the sticks, the vaults, then the lines typed at step 4. The
  certificate comes in as a public file (F22) or a scanned code.
- The lines are paperkey's base16 form, each checked by its CRC-24 as
  it is ended, and the whole data by its own; the line format and the
  refusals' words are paperkey 1.6's.
- The result is the type 7 record Make wrote: the certificate and the
  kept revocation are made again from the seeds and the times read from
  the certificate.
- Faraday refuses what paperkey would take and Faraday cannot verify:
  a key of another algorithm or shape; a secret protected by a
  passphrase; a secret that does not make the certificate's public key;
  a certificate whose signatures by the key are not the ones Faraday
  makes for it; and a CRC that does not match (paperkey's
  `--ignore-crc-error` has no counterpart). Signatures by other keys
  are not kept.
- The restored key's bytes are held by `faraday-keys` from each line
  checked, and the key enters the vaults as a made key does. No reveal
  is added to F70's list.
- A restored key signs at once (the owner's answer to Q2 may change
  this).

## 11. Plan

### 11.1 Rules for the implementing agent

- CLAUDE.md holds. Agents do not commit. While working, run the one
  test module touched (`cargo nextest run -p faraday-pgp --test
  gnupg`, and the new paperkey tests); `just` once at the end.
- Nothing cryptographic lands without its rows of §8 passing against
  the real `paperkey` and `gpg`.
- No upstream file is copied into the repository unless Q10 says so;
  fixtures are made by the tools, and their README records how.
- No path in a doc that does not exist; new files are named in plain
  text until they exist.
- Never name a person other than the public author of a standard or a
  tool.

### 11.2 Batches

| Batch | What | Model | Estimate |
|---|---|---|---|
| P1 | `faraday-pgp`: the paperkey readers (text, raw), the certificate reader with GnuPG's algorithm names, the rebuild and compare, `restore`; tests 1 to 15 against paperkey 1.6 and GnuPG 2.4, the fixtures and their README; the two fuzz targets; lint-secrets rule 4(c) | Opus | 2 days |
| P2 | `faraday-keys`: the restore set (`Restore::new`, `line`, `finish`), wiped on drop, the lock's wipe; tests 16 to 18 | Sonnet | 1 day |
| P3 | `faraday-core`: the ways in (Files' action, Vault contents, the Tools line), the sheet, the head card with line 1, step 4's lines and the hex keyboard, the refusals, the Result, signing per Q2, Practice dimmed; strings; tests 19 to 22; the snapshot tour | Opus | 3 days |
| P4 | Scanning the public key: an armoured public key code filed in From the stick as a public file, named `{name}-{fp8}.asc`; its routing row in `docs/QR.md` §2 | Sonnet | 0.5 day |
| P5 | Only if Q5 says yes: a paperkey code (base16 text or raw) scanned at step 4 fills the lines, each still checked | Sonnet | 0.5 day |
| P6 | Docs: the Learn page (§9), `docs/FLOWS.md` GPG, `docs/QR.md`, `docs/DEMO.md` §4, `docs/VAULT.md` §7's note, `docs/KEYS.md` §3, the decision recorded with its number, then this file's plan to docs/history/GPG-PAPERKEY-LOG.md (new) and its Kind to Reference | Sonnet | 0.5 day |

About 7.5 days. P1 before P2, P2 before P3; P4 and P5 after P3; P6
last.

### 11.3 Edit list

| File | Change |
|---|---|
| `faraday/faraday-pgp/src/lib.rs` and two new modules beside it | §7.1 |
| `faraday/faraday-pgp/tests/gnupg.rs`, a new tests/paperkey.rs | §8, 1 to 15 |
| `faraday/faraday-keys/src/gpg.rs`, `faraday/faraday-keys/tests/vault.rs` | the restore set; §8, 16 to 18 |
| `faraday/faraday-core/src/backup_flow/mod.rs`, `faraday/faraday-core/src/backup_flow/steps.rs`, `faraday/faraday-core/src/backup_flow/screen.rs` | the restore's state, step 4, the head card |
| `faraday/faraday-core/src/lib.rs` | the sheet's `Sheet` variant |
| `faraday/faraday-core/src/vault/screen.rs`, `faraday/faraday-core/src/screens/catalog.rs`, `faraday/faraday-core/src/inbox.rs` | the ways in |
| `faraday/faraday-core/tests/all/gpg.rs` | §8, 19 to 22 |
| `faraday/faraday-strings/src/en.rs` | §7.3 |
| `tools/lint-secrets.sh` | rule 4(c) |
| `fuzz/fuzz_targets/` | two targets |
| `docs/learn/faraday/gpg.md`, `docs/FLOWS.md`, `docs/QR.md`, `docs/DEMO.md`, `docs/DESIGN.md` §5's sheets, `docs/README.md` | P6 |

### 11.4 Questions for the owner

Answered by the owner on 2026-10-23: every proposal below is taken (Q1
refused; Q2 at once; Q3 two or more places; Q4 set aside and counted;
Q5 P5 builds scanning; Q6 not offered; Q7 armour only; Q8 not offered;
Q9 dimmed in Practice; Q10 not copied; Q11 refused).

- **Q1. Keys made elsewhere.** A key of Faraday's shape made by GnuPG
  (an Ed25519 key that certifies, one Ed25519 signing subkey, no
  passphrase) has signatures Faraday does not make byte for byte (other
  preferences, other subpackets). Proposed: refused. The alternative is
  to verify each of its self-signatures with Ed25519 instead of
  rebuilding them, keep its certificate as it came, and let Renew write
  Faraday's own at the next renewal; it accepts more than Faraday makes,
  so it is the owner's decision.
- **Q2. Signing after a restore.** Proposed: at once, as a restored
  wallet (`docs/BACKUP-FLOW.md` §11): the paperkey is a copy that
  exists. Or wait for every place of the plan to be written, as Make's
  keys do (F31).
- **Q3. The places.** Proposed: two or more, as Make. Or one, since the
  paperkey is kept at a place of its own.
- **Q4. Signatures by other keys.** Proposed: set aside and counted, the
  restore going on. Or refuse a certificate that carries any.
- **Q5. A paperkey as a code.** paperkey's raw form is meant for codes,
  and the ask says "typed (or scanned)". Faraday never shows a paperkey
  as a code (`docs/QR.md` §7, row 23). Proposed: P5 builds scanning a
  paperkey code at step 4 (the scanner opens with no stick in, as for a
  SeedQR, decision 9). Or typing only.
- **Q6. A paperkey file from a stick.** A stick is read only with
  nothing secret open, and a secret file brought in before the flow
  does not last through step 2's lock; step 4 comes after the vaults
  are open. Proposed: not offered. A later design could read the file
  from one of the plan's sticks at step 2 and hold it in the restore
  set across the lock.
- **Q7. Binary certificates.** `gpg --export` without `--armor` writes
  packets with no armour, and F22 reads only armour. Proposed: armour
  only. Taking binary packets as public files amends F22.
- **Q8. A suggested correction.** A CRC-24 points at a line; a single
  mistyped digit could be found by trying each of the line's 44 digits
  at each other value (660 tries) and offering the one that matches, as
  codex32's opt-in correction does (F94). Proposed: not offered; the
  person reads the line again.
- **Q9. Practice.** Proposed: Restore is dimmed in Practice, as Make is. Or a
  sample key with its certificate and paperkey in `faraday/faraday-samples`,
  for the website's walk-through.
- **Q10. paperkey's check keys.** They are GPL-2.0; Faraday is MIT.
  Proposed: not copied; the tests make their own keys with GnuPG, and
  test 5 uses one of the same shape. Or copy `papertest-eddsa.pub` and
  `.sec` into tools/vectors/paperkey/ with their licence notice.
- **Q11. A certificate with no user ID.** keys.openpgp.org serves a key
  without its user ID until the address is verified. Proposed: refused;
  the person exports the key from another copy. Or the user ID typed and
  a new self-signature made by the restored key, which is a change to
  the certificate the person must then publish.
