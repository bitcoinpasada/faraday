# TOTP codes and the time: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/TOTP.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/TOTP.md)

Built 2026-10-11: batch 1 (the clock) `9a6f4a3`, batch 2
(the codes) in the commit after it; spec v0.1 · 2026-10-11 · decided by
the owner (`docs/DECISIONS.md` F7). Where the build and this file differ
once built, `docs/FLOWS.md` and the code are the record.

The problem it solves: a vault entry already holds a TOTP secret (record
type 6, field 6, `docs/VAULT.md` §7), scanned from a setup QR, typed, or
imported from the Inbox, but Faraday shows only the secret, never the
code (`docs/history/PLAN.md` §6.4: "it does not generate TOTP codes"). A code is
HMAC over the Unix time in 30-second steps, so it is right only when the
clock is. The Pi has no real-time clock and starts at 1970. The stick reads
the PC's hardware clock as UTC, and a PC that runs Windows keeps that
clock in local time, so the stick can be hours off. Every displayed time
is UTC today.

## §0 Rules for the implementing agent (from docs/TOTP.md)

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of a batch; no `rm`; no commits; no agents.
- Two batches, in order: §1–§3 (the clock), then §4–§7 (the codes). Each
  batch is one review and one completion report. Do not start batch 2
  before batch 1 is reviewed.
- Tests go in the files named in §8, driving the app through `Action`s
  and `StorageEvent`s as the neighbouring tests do
  (`faraday/faraday-core/tests/all/settings_file.rs`, `gpg.rs`,
  `import.rs`). A test states what a person sees or can do.
- Docs in the same batch as the code they describe (§9).
- The snapshot tour (`faraday/shells/snapshot/src/main.rs`) gains the
  shots §9 names. To look: `just faraday-snap 1280x800`, and
  `480x640` and `240x320` for the small panel.
- Completion report: the items done, each with its tests and whether
  they pass; what was left out and why; docs changed; render names to
  look at. No restating the spec.

## §8 Tests (from docs/TOTP.md)

`faraday/faraday-vault/tests/totp.rs` (batch 2):

- RFC 6238 Appendix B, 8 digits, 30-second period. Seeds: ASCII
  `12345678901234567890` (SHA1), `12345678901234567890123456789012`
  (SHA256), `1234567890` × 6 + `1234` (SHA512), each given as a Base32
  `otpauth://` URI with `algorithm` and `digits=8`:

  | Time | SHA1 | SHA256 | SHA512 |
  |---|---|---|---|
  | 59 | 94287082 | 46119246 | 90693936 |
  | 1111111109 | 07081804 | 68084774 | 25091201 |
  | 1111111111 | 14050471 | 67062674 | 99943326 |
  | 1234567890 | 89005924 | 91819424 | 93441116 |
  | 2000000000 | 69279037 | 90698825 | 38618901 |
  | 20000000000 | 65353130 | 77737706 | 47863826 |

- RFC 4226 Appendix D, SHA1, 6 digits, counters 0–9 (period 1 at times
  0–9 gives the counter directly): 755224, 287082, 359152, 969429,
  338314, 254676, 287922, 162583, 399871, 520489.
- The key-URI example
  `otpauth://totp/ACME%20Co:john.doe@email.com?secret=HXDMVJECJJWSRB3HWIZR4IFUGFTMXBOZ&issuer=ACME%20Co&algorithm=SHA1&digits=6&period=30`
  reads; the same secret typed as `hxdm vjec jjws rb3h wizr 4ifu gftm
  xboz` gives the same code.
- `otpauth://hotp/…` is Hotp; `digits=9`, `algorithm=MD5`, `period=0`
  and a secret with `1` or `8` in it are Unreadable.
- `normalise` of a bare key reads back through `parse` with the same
  code as the key; of garbage is `None`.

`faraday/faraday-core/tests/all/clock.rs` (batch 1):

- A `Clock` of 1000 (the Pi at boot): making a GPG key says the clock
  is not known.
- A `Clock` of 1000, then Set the time to 2026-10-11 14:02 at
  `UTC+02:00`: a GPG key made at once has creation time 2026-10-11
  12:02:00 UTC (within the test's elapsed milliseconds); Settings' Now
  reads 14:02.
- After that set, a later `Clock` event of 2000 does not move the time.
- The set time and its checked mark survive a lock (kept, restored into
  a new app).
- `tz=+05:30` in the boot stick's settings file is applied; `tz=+05:20`,
  `tz=+15:00`, `tz=5:30` and `tz=x` are not, and the rest of the file
  still is.
- With `UTC−04:00`, "Written to STICK at" shows the local time.
- Set is not offered for 2026-02-30 or 24:00.

`faraday/faraday-core/tests/all/totp.rs` (batch 2), with an entry holding
the RFC 4226 secret as a SHA1 6-digit URI, so the expected codes are
the RFC's:

- From Tools with no vault open, TOTP codes leads to Unlock and, after
  it, to the codes.
- A Pi-like clock (`Clock` below the floor, not online): the codes
  screen opens Set the time; after Set, the codes show the code for
  that instant.
- A stick-like clock (known, not online): Set the time with Keep this
  time first; Keep shows the codes; after a lock, the codes open
  directly.
- The desktop (`online`, clock known): the codes open directly.
- Two open vaults with a TOTP entry each: both rows, each with its
  vault's name; a HOTP entry shows HOTP and no code.
- The code shown changes when the ticks cross the period boundary.
- The entry form saves a bare key as a URI, and the detail shows its
  code; `hello` in the field is refused with Not a TOTP key.
- Scan a setup code from the codes screen, save: back on the codes with
  the new row.

## §9 Docs and snapshots (from docs/TOTP.md)

Batch 1:

- `docs/FLOWS.md`, "New devices and settings": Settings → Time, Set the
  time; a dated decision line (decision 56).
- `docs/learn/faraday/time.md`, a Learn page **Time and TOTP codes**,
  included by `faraday-learn/src/lib.rs` as the upgrade and shares pages
  are, opened by Set the time's ?: what a TOTP code is made from; why
  the time must be right to within about half a minute; the Pi has no
  clock and starts at 1970; a PC that runs Windows keeps its clock in
  local time, so the stick can read it hours off; type the next minute
  and press Set as the phone's minute changes; the time zone is a fixed
  offset, changed by hand at daylight saving; Faraday never changes the
  computer's own clock. Plain statements, as `CLAUDE.md` asks.
- `docs/history/HANDOFF.md` item 62's key list gains `tz`.
- Snapshot: `settings-time`, and `set-time` with the clock not known.

Batch 2:

- `docs/history/PLAN.md` §6.4: "Faraday stores entries; it does not generate TOTP
  codes" becomes the codes screen and the Code row, and names this file.
- `docs/VAULT.md` §7, type 6 field 6: "an `otpauth://` URI (a bare key
  is stored as one, `docs/TOTP.md` §4.3)".
- `docs/FLOWS.md`, "Vaults": the codes screen, its ways in, the Code
  row; decision 57.
- `README.md`'s feature table, Vaults row: "TOTP codes" after "password
  entries with TOTP secrets".
- Snapshot: `totp-codes` and `vault-entry-code`, from the entries the
  tour imports (`vault-entries.txt` already holds
  `JBSWY3DPEHPK3PXP`), taken after the tour's GPG step sends its fixed
  `Clock` (1_791_000_000), with Keep this time pressed if Set the time
  comes first. The tour's time is fixed, so the codes are the same in
  every render.

## Batch headings (from docs/TOTP.md)

The spec was in two batches: "# Batch 1: the clock" over §1–§3 and
"# Batch 2: the codes" over §4–§7, each after a rule (`---`).

## §1 The clock model: its first paragraph as written

Today `vaults.unix_secs` holds the last `StorageEvent::Clock` and
`clock_now()` (`faraday-core/src/vault/mod.rs`, "pub(crate) fn clock_now")
adds the milliseconds since it arrived. Every caller keeps using
`clock_now()`; what it returns changes.

## §2.3 Every time shown is local: its last sentence

Fix the
`time_of_day` doc comment ("as the computer's clock keeps it (UTC on
Faraday)") to say it is local.
