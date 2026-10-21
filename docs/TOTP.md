# TOTP codes and the time

**Status:** built 2026-10-11: batch 1 (the clock) `9a6f4a3`, batch 2
(the codes) in the commit after it; spec v0.1 · 2026-10-11 · decided by
the owner (`docs/DECISIONS.md` F7). Where the build and this file differ
once built, `docs/FLOWS.md` and the code are the record.

The problem it solves: a vault entry already holds a TOTP secret (record
type 6, field 6, `docs/VAULT.md` §7), scanned from a setup QR, typed, or
imported from the Inbox, but Faraday shows only the secret, never the
code (`PLAN.md` §6.4: "it does not generate TOTP codes"). A code is
HMAC over the Unix time in 30-second steps, so it is right only when the
clock is. The Pi has no real-time clock and starts at 1970. The stick reads
the PC's hardware clock as UTC, and a PC that runs Windows keeps that
clock in local time, so the stick can be hours off. Every displayed time
is UTC today.

## 0. Rules for the implementing agent

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

## Decisions recorded (owner, 2026-10-11)

1. Faraday shows TOTP codes for the entries in open vaults. The secrets
   stay where they are: type 6 field 6, in the vault. Nothing new is
   stored outside a vault.
2. The time can be set by hand, in local time, to the minute.
3. The time zone is a fixed offset from UTC, chosen in Settings and kept
   in `faraday-settings.txt`. No time zone database; the person changes
   the offset when daylight saving starts or ends.
4. Where the clock is not known, the codes screen opens Set the time
   first.

Decided in this spec (architect, 2026-10-11, inside the owner's four):

5. The time is asked for only when the codes screen is opened: never
   at boot, never anywhere else. Where the clock is known but comes
   from the computer's hardware clock (the stick, the Pi after a set),
   opening the codes screen also opens Set the time first, with **Keep
   this time** one press away; once answered, the codes screen opens
   directly for the rest of the power-on. That is how a Windows PC's
   clock, hours off, is caught before a wrong code is typed somewhere. The desktop app reads its host's
   network-synced clock and is not asked.
6. The code is shown plainly, not hold-to-reveal: it is valid for one
   period, and an authenticator app shows it the same way. The secret
   stays hold-to-reveal.
7. Faraday never writes the computer's clock: no `settimeofday`, no
   `hwclock`, nothing the kernel's `RTC_SYSTOHC` could copy back to the
   hardware clock. A time set by hand lives in the app only.

---

# Batch 1: the clock

## 1. The clock model

Today `vaults.unix_secs` holds the last `StorageEvent::Clock` and
`clock_now()` (`faraday-core/src/vaults.rs`, "pub(crate) fn clock_now")
adds the milliseconds since it arrived. Every caller keeps using
`clock_now()`; what it returns changes.

### 1.1 A clock before 2020 is not known

`CLOCK_FLOOR = 1_577_836_800` (2020-01-01T00:00:00Z), the same floor
`faraday/shells/stick/src/files.rs` uses for file dates. The core keeps
the shell's raw reading in every case, but `clock_now()` returns `None`
when there is no time set by hand and the raw reading is below the
floor. On the Pi, which reports about 1970 after boot, GPG, Secure Boot
and the "at 14:02" toasts then say the clock is not known, as they
already do when no `Clock` event has arrived, instead of stamping 1970.

### 1.2 A time set by hand

Setting the time stores an **offset** in seconds: the time set minus
the shell's clock at that moment (`raw + elapsed`). From then on,
`clock_now()` is the shell's clock plus the offset, whether the raw
reading is above the floor or not. The shell keeps ticking and the
`Clock` events keep updating the raw reading; the offset carries the
correction across them.

The offset survives a lock. The stick and Pi shells start a new app
process at every lock (exit 75), so the offset goes into what the app
keeps across a lock (`kept()` and `restore_kept()` in
`faraday-core/src/memory.rs`) as its own item, `"clock"`, whose text is
`offset=<i64>` and `checked=0|1` on separate lines. It does **not** go
into `settings_body()`: an offset is meaningless after a power-off,
when the Pi's clock restarts at 1970. A power-off loses it.

The offset is a plain number, not a secret, and lives outside the
session's secrets.

### 1.3 Checked this power-on

`clock_checked: bool`, true once the person has pressed **Set** or
**Keep this time** (§3) in this power-on, and kept across a lock with
the offset. The desktop app (`app.online`) counts as checked whenever
its clock is known. §5 uses this, and so does everything else that
keeps the time it reads (HANDOFF item 68, 2026-10-11): GPG's Make a
key, Renew, Make a revocation, a signature's QR and PNG, and Secure Boot's Make keys and enrolment files. With the clock
not checked (the stick and the Pi), pressing one opens Set the time
(§3) over the same page, which it returns to, and does nothing else;
the person then presses again. A GPG key's creation time is part of its
fingerprint, so a wrong clock is not put right later. Signing an image
with the db key reads no clock and does not ask. GPG's Sign a file does
not ask either (F63): its sheet shows the time the signatures will
carry, with **Set the time**, which returns to the sheet, and only its
hold signs, disabled while the clock is not known.

## 2. The time zone

### 2.1 The setting

`tz_min: i16`, minutes east of UTC, default 0. It takes values from
−720 to +840 (−12:00 to +14:00) in steps of 15. That covers every
offset in use, including +05:30 and +05:45.

- **Settings** gains a section **Time** after **Session**. Under its
  title the line **For TOTP codes, GPG and Secure Boot signatures, and
  backup check dates** (everything that reads `clock_now`). It holds a
  value row **Now**, the live local date and time (`2026-10-11
  14:02:37`) or `Not known`; a row **Time zone** showing `UTC+02:00`
  (`UTC+00:00` for zero, `UTC−03:30` with a minus sign) between a −
  and a + button that step 15 minutes and stop at the ends; and the
  button **Set the time**, which opens §3.
- The compact (small panel) Settings gets the same three rows, laid out
  as its other sections are.

### 2.2 In the settings file and the kept set

`settings_body()` (`faraday-core/src/stick_settings.rs`) gains a line
`tz=+02:00`: the sign always written, two-digit hours, two-digit
minutes. `apply_setting()` takes `tz` only when it has that exact form,
the minutes are 00, 15, 30 or 45, and the value is in range; anything
else is skipped as an unknown value is now. The file is untrusted
(`stick_settings.rs` header): a wrong zone gives wrong local times and
wrong codes, never a less protected session, so it may be read from the
file. The stick-settings comment and `HANDOFF.md` item 62's key list
name it.

A stick file written before this change has no `tz` line. The zone stays
at its default, and the next stick visit offers Settings ticked because
the body now differs. That is expected.

### 2.3 Every time shown is local

One function in `vaults.rs`, beside `time_of_day`, turns Unix seconds
into local seconds (`unix + tz_min × 60`). Every date or time **drawn
on screen or written for the person** goes through it:

- `vaults::time_of_day` callers: `backups.rs` ("On STICK, 14:02"),
  `vaults.rs` ("seen 14:02"), `lib.rs` ("Written to STICK at 14:02");
- `chart_edit.rs`'s today's date, which goes into the plan's
  `checked … 2026-10-10` notes;
- the GPG screens' "From 2026-10-11" dates (`gpg::date` where it is
  drawn).

What goes **into a format** stays UTC: a GPG packet's creation time, a
signature's time, Secure Boot timestamps, the TOTP counter. Fix the
`time_of_day` doc comment ("as the computer's clock keeps it (UTC on
Faraday)") to say it is local.

## 3. Set the time

A new screen, `Screen::SetTime`, of the Enter intent (DESIGN §5):
title **Set the time**, the entry group at the bottom, its context
centred above it.

- **Context:** the value row **Now**, live as in §2.1, and the **Time
  zone** row with its − and + buttons (the same actions as Settings').
- **Entry:** two fields, **Date** (`YYYY-MM-DD`) and **Time** (`HH:MM`,
  24-hour). Date starts filled with the clock's local date when the
  clock is known and empty when not; Time starts empty and has the
  focus. They take digits and the separators; the on-screen keyboard
  on the Pi as for any field.
- **Set** (primary): enabled when Date is a real calendar date between
  2020-01-01 and 2099-12-31 and Time is 00:00 to 23:59. Pressing it
  sets the local time to Date Time at **:00 seconds** at the instant of
  the press (§1.2), marks the clock checked, toasts `Time set to 14:02`
  and goes back. The person presses it as their phone's minute changes;
  the Learn page (§9) says so, the screen does not.
- **Keep this time** (secondary): shown only when the clock is known.
  It marks the clock checked, sets no offset, and goes back.
- **Back** goes back without marking anything.

"Goes back" is to the screen it was opened from: Settings, or the codes
screen (§5), which then shows its codes. The ? button opens the Learn
page of §9.

Compact layout: the same order, scrolled where it does not fit, as the
other compact Enter screens.

---

# Batch 2: the codes

## 4. Reading a TOTP secret

A new module, `faraday/faraday-vault/src/totp.rs` (no_std with alloc,
as the crate is), public as `faraday_vault::totp`. Dependencies:
`sha1 = { version = "0.11", default-features = false }`, the version
`faraday-pgp` already uses, so no new crate enters the tree; `hmac`
and `sha2` are already the crate's. No Base32 crate: the decoder is
written here (RFC 4648 §6 alphabet).

### 4.1 What it reads

`parse(field: &str) -> Result<Totp, NotTotp>`, from a type 6 field 6
value:

- An `otpauth://totp/<label>?<params>` URI. `secret` is required (Base32,
  case-insensitive, spaces and `=` padding ignored, any other character
  refused, at least one byte decoded). `algorithm` is `SHA1` (default),
  `SHA256` or `SHA512`, case-insensitive. `digits` is 6 (default), 7
  or 8. `period` is 30 (default), from 1 to 3600. Unknown parameters
  (`issuer`, `image`, …) are ignored. Any present parameter with a value
  outside these refuses the URI as `NotTotp::Unreadable`.
- An `otpauth://hotp/…` URI: `NotTotp::Hotp`. A HOTP code needs its
  counter written back to the vault on every use; not in this spec.
- A bare Base32 key, as services show "can't scan? type this key"
  (`JBSW Y3DP EHPK 3PXP`, any case, spaces or hyphens between groups),
  of at least 16 characters (80 bits, the shortest key services hand
  out, so a word or a password typed into the field is not taken for
  one): read as SHA1, 6 digits, 30 seconds.
- Anything else: `NotTotp::Unreadable`.

`Totp` holds the decoded secret in a `Zeroizing<Vec<u8>>`, the
algorithm, digits and period, and its `Debug` prints no secret bytes.

### 4.2 What it computes

- `code(&self, unix: u64) -> String`: RFC 6238 with T0 = 0, the counter
  `unix / period` as a big-endian `u64`, RFC 4226 dynamic truncation,
  modulo 10^digits, zero-padded to `digits`.
- `left(&self, unix: u64) -> u32`: seconds until the next code,
  `period − unix % period`.
- `grouped(code: &str) -> String`: six digits as `492 039`, seven as
  `1 234 567`, eight as `1234 5678`, for display only.

### 4.3 Normalising what is typed or imported

`normalise(field: &str, title: &str, account: &str) -> Option<String>`:
a URI that `parse` reads, or one it reads as HOTP, is returned
unchanged (trimmed). A bare Base32 key becomes
`otpauth://totp/<title>:<account>?secret=<KEY>&issuer=<title>`, the
title and account percent-encoded, the key upper-cased without spaces,
hyphens or padding (`:<account>` left out when the account is empty).
Anything else is `None`. `entries::parse` (Inbox import) passes its
`totp:` lines through it as well, so an imported bare key is stored as a
URI; an unreadable `totp:` value goes to the notes, as other unknown
lines already do.

## 5. The codes screen

`Screen::Codes`, of the Navigate intent (DESIGN §5, the Addresses list's
layout): title **TOTP codes**.

### 5.1 Ways in

- A Tools tile in **Vaults and other keys**, after **Vaults**: name
  **TOTP codes**, line **Codes from the entries in open vaults**, tags
  **RFC 6238**, `Go::Codes`.
- The open vault's **Entries** list gains a button **TOTP codes** beside
  its **Add an entry**.

### 5.2 What comes first

Opening it, in this order:

1. No vault open: `vault_way(Screen::Codes)`, as `Go::VaultCategory`
   does today, so Create or Unlock comes back to the codes.
2. The clock not checked this power-on (§1.3): Set the time (§3), which
   goes back to the codes.
3. The list.

Both checks happen on the way in only. The clock check is not asked
again after a lock in the same power-on.

### 5.3 The list

- At the top, the value row **Now**, local time with seconds and the
  zone (`14:02:37 UTC+02:00`), with **Set the time** on its right.
- Then one row per type 6 record whose field 6 `parse`s, in every open
  vault, vault by vault in the order they were opened, then in record
  order. A row holds:
  - the entry's title (field 1), and under it the account: the URI
    label's account part, else field 2 (username), else nothing; with
    more than one vault open, the vault's name after it (`you@example ·
    Cold storage`);
  - on the right, the code, grouped (§4.2), in the mono face at the
    size the tokens give a code on this class;
  - under the row, a progress bar (DESIGN §4.14) for the time left in
    its period, with the seconds (`12 s`) in muted text.
- A record whose field 6 reads as HOTP is listed with an **HOTP** badge
  (DESIGN §4.8) in place of the code; one that does not read, with an
  **Unreadable** badge. Neither has a bar.
- Pressing a row opens that entry in its vault (Vault contents, Entries,
  the record selected), where §6 shows the same code.
- Foot actions: **Scan a setup code** (`VaultAction::ScanEntry`, into
  the current vault) and **Add an entry** (the entry form). Saving the
  form opened from here comes back to the codes screen, with the new
  entry's row in the list.
- No rows: the line **No entries with a TOTP secret**, and the two foot
  actions.

The screen redraws on the shell's ticks whenever the second shown has
changed, and the code changes on the tick that crosses its period. It is
a vault screen: a lock or the idle lock closes it as the other vault
screens are closed. Watching codes is not input and does not hold off
the idle lock.

### 5.4 Small panel

Compact layout as the compact vault lists: one row per entry, title
over code, the account line dropped when there is no room, the bar under
each row, Now on one line at the top. Check at `480x640` and `240x320`.

## 6. The entry

- **Detail** (`vault_screens.rs`, `kind::ENTRY`): a row **Code** before
  the **TOTP secret** row, with the grouped code and `12 s`, live. When
  the clock is not checked, its value is the button **Set the time**,
  which opens §3 and comes back. HOTP and Unreadable show the badge
  instead. The secret's hold row is unchanged.
- **List** (the second line, "Password · TOTP"): unchanged. Codes are on
  the codes screen and in the detail, not in the vault's list.
- **Form**: the **TOTP secret** field takes a URI or a bare key. On save
  it goes through `normalise` with the form's Title and Username; `None`
  refuses the save with the toast **Not a TOTP key**, the form kept as
  typed. An empty field saves no field 6, as now.

## 7. Not in this spec

- HOTP codes (the counter needs a write to the vault on every use).
- Google Authenticator's export QR (`otpauth-migration://`, a protobuf
  of many secrets). Useful for moving off a phone, but a new parser of
  untrusted input; a later item if the owner wants it.
- Steam's five-character codes.
- Opening Set the time from the "This computer's clock is not known"
  toasts in GPG and Secure Boot. Settings → Time reaches it.
- Dates on files written to a stick: they come from the system clock,
  1970 on the Pi, and are not changed here (decision 7).

## 8. Tests

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

## 9. Docs and snapshots

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
- `HANDOFF.md` item 62's key list gains `tz`.
- Snapshot: `settings-time`, and `set-time` with the clock not known.

Batch 2:

- `PLAN.md` §6.4: "Faraday stores entries; it does not generate TOTP
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
