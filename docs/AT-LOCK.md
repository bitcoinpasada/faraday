# What holds a secret: one rule for clean, the idle lock and Lock

**Kind:** Spec
**Status:** Specified; none of it is built: the clean state, the idle lock's test and Lock's clears are still three lists kept by hand.
**Open:** the `AtLock` trait, the exhaustive listing of `Faraday` and of each area that holds areas, the three answers derived from it, and the leak tour that walks it (§3 to §7)

## 1. The requirement

A new area of Faraday's state, or a new field of one, does not compile
until whoever adds it has said whether it holds a secret and what Lock
does with it (owner, 2026-11-19: a future tool must not pass without
being cleared at Lock). Three answers are derived from that one
statement, and from nothing else:

- **clean** (`docs/SECURITY-MODEL.md`): this process has never held or
  typed a secret, so a stick may be handed to the disk process
  (`Faraday::clean`, `clean_now`, `note_secrets`, `faraday-core/src/idle.rs`);
- **lockable**: a lock would wipe something, so the idle warning and the
  idle lock come, and the idle power-off waits (`Faraday::lockable`,
  `idle.rs`, read in `state.rs`'s idle tick);
- **the lock's clears**: what `lock_finish` (`faraday-core/src/vault/seal.rs`)
  wipes before the process ends.

Today each is a list written by hand: `secret_typed`
(`faraday-core/src/edit.rs`) and `Faraday::holds_secret`
(`faraday-core/src/vault/mod.rs`) for clean, `lockable` for the idle lock,
and the calls in `lock_finish`. §2 shows where they disagree.

**Why the clears matter though the process ends.** Lock is two steps
(`docs/SECURITY-MODEL.md`, "What locking wipes"): the application drops
what it holds, then the process exits and the kernel zeroes its pages.
The first step is what the leak tests see: their scan of every live
block after Lock (`docs/SECRET-LEAKS.md` §1 point 2) runs in the test
harness, where nothing exits, and so does every test that locks and goes
on. `save_boxes_for_lock` writes, between the clears and the exit, what
the fresh process is given; a lock can also be stopped after it is asked
(`Faraday::not_kept`, `idle.rs`, which clears `restart` when the state
could not be kept), and then the same process goes on. The Demo shell
serves the saved state to a fresh app in the same process
(`shells/demo/src/lib.rs`, `Command::Exit` with `restart_requested`).

## 2. The three lists today, field by field

Read from the code at the time of writing. "Clean" is `secret_typed` or
`holds_secret`; "Lockable" is `lockable`; "Lock" is a call in
`lock_finish`. A field not named holds nothing secret (layout, settings,
timers, device status, the guide, search text) or is cleared by all three
alike.

| Field | What secret it can hold | Clean | Lockable | Lock |
|---|---|---|---|---|
| `lightning` | words, passphrase, the node key | yes (any state) | no | **no** |
| `tools.seed_check` (My seed makes my wallet) | words, passphrase | yes | no | **no** |
| `tools.kef_tool` (the Krux code tool) | the typed code, its passphrase, the opened payload | yes | no | **no** |
| `tools.core_wallet` (Read a Bitcoin Core wallet) | the passphrase; the file read | passphrase only | no | **no** |
| `tools.tools` (the calculator's typed line) | an extended private key typed and refused | **no** | no | **no** |
| `tools.mscheck` (Check a multisig setup) | a seed answer | **no** | no | **no** |
| `tools.convert` (Convert a seed) | the typed seed, the value held | **no** | no | yes |
| `tools.qr_tool` (the QR vault tool) | note, passphrase, the made code | note and passphrase only, not the code | no | yes |
| `spend_backup` (Spend from a backup, and its Sweep) | the seeds typed for it | **no** | no | **no** |
| `vanity` | a passphrase a find names, on show | **no** | no | **no**; to be decided by the area: whether its search state keeps the passphrase |
| `silent` (silent payments) | none: a fingerprint and the DNS record's text | no | no | no; public, not a disagreement |
| `camera` | a frame on the Scan sheet, which may picture a seed | through the Scan sheet only | no | **no** (closed by the hide before it) |
| `camera.parts` | a transfer in parts (BBQr, numbered, UR) half read, kept after the sheet closes | **no** | no | **no** |
| `camera.carried` | a TOTP setup code carried to Unlock | **no** | no | **no** |
| `camera.scan`'s read card | the code a card read, waiting under the Pull or devices sheet for Scan again | **no** (the Scan sheet is not up) | no | **no** |
| `vaults`' dice rolls for Create a vault | the rolls that fix the new vault's passphrase, kept after the form closes | **no** | no | **no** |
| `inbox` | seed files from a stick; a seed being made a wallet and its passphrase | the potential's passphrase only | no | the files dropped at lock, not the potential |
| `vaults` | open records, the unlock passphrase, the prompt, a new vault's phrases | yes | open vaults | the signing text only; the rest by the seal's end |
| `create` (a new vault's passphrases) | passphrase, again | **no** | no | yes |
| `restore` | seed shares | **no** | no | yes |
| `bip85` | a child key or its words | **no** | no | yes |
| `keygen`, `entry`, `seed_read`, `secrets`, `backup` | words, rolls, passphrases, codes | yes | no | yes |
| `session` | the keys | yes | yes, and its wallets | yes |
| `wallets` | none: a spend is public | no | yes (a spend) | yes |
| `fresh` (own randomness) | the session's seed and pool | **no** | no | what was added only; to be decided by the area: the seed and the pool |
| `flows` (backup flows) | a flow's seed and restore data | **no** | no | by the lock's flows step before `lock_finish` |
| `drawing`, `kept`, `canvas` | text and glyphs kept for frames | no | no | charts only; to be decided by each area |
| `memory`, `visit`, `outbox`, `later`, `worker`, `backups_area`, `transfer` (Demo) | to be decided by each area | no | no | `visit`'s name question only |

Each bold **no** beside a secret is a field one list counts and another
does not. Lockable reads no typed secret at all: a session that has only
typed words, or only a tool's state, never warns or locks when idle; on
the device it powers off at the power-off time instead, and in Faraday
Demo, which never powers itself off, it stays.

## 3. The two ways, against the requirement

**Whole areas set to `Default` in `lock_finish`** (tools, vanity,
silent, lightning, spend backup, the camera, keeping their few public
fields). It closes today's gaps in the lock's list, and a field added
inside one of those areas is cleared without anyone listing it. It does
not meet the requirement: a new top-level field of `Faraday`, or an area
not named, passes, and clean and lockable stay lists by hand. It
answers only the third of §1's answers.

**One trait per area, called from an exhaustive destructuring of
`Faraday` with no rest pattern** (`let Faraday { canvas, f, …, transfer, … } = self;`
with no `..`). A new field is a compile error in that pattern until it
is written there either as an area that answers the trait or as public.
It meets the requirement only if it recurses: the tools the survey found
missed (the Krux code tool, the Core wallet tool, the seed check) are
fields of `ToolsArea`, not of `Faraday`, so **every area that holds
areas destructures its own struct the same way**. The `Default` reset is
then the usual body of an area's `lock`, keeping its public fields by
name, so the first way is the inside of the second, not its rival.

**Recommended: the trait with exhaustive destructuring, recursing into
every area that holds areas, with the `Default` reset as the body of
`lock`.** `Faraday` has about ninety fields; the `demo`-only `transfer`
is written in the pattern under its own `#[cfg(feature = "demo")]`.

## 4. The trait

One file, `faraday-core/src/at_lock.rs` (a name of its function, as
`docs/STRUCTURE-PASS.md` §7 asks).

```rust
/// What an area holds, for the clean state, the idle lock and Lock.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Holds {
    /// A key or an open vault the session keeps: what the screens read
    /// as `Faraday::holds_secret` (Home's fresh state, a stick's offers).
    pub kept: bool,
    /// A secret on its way: typed, rolled, scanned, shown or opened in a
    /// tool. Makes the process not clean; read by the fade as
    /// `secret_typed`.
    pub typed: bool,
    /// Something a lock wipes that is not secret: a wallet of the
    /// session, a spend under way.
    pub session: bool,
}

impl Holds {
    pub const NOTHING: Self = Self { kept: false, typed: false, session: false };
    pub fn any(self) -> bool { self.kept || self.typed || self.session }
    pub fn or(self, o: Self) -> Self {
        Self { kept: self.kept || o.kept, typed: self.typed || o.typed, session: self.session || o.session }
    }
}

/// An area of Faraday's state that can hold a secret or the session's
/// own state. No method has a default body: writing the impl is the
/// decision.
pub(crate) trait AtLock {
    /// What it holds now.
    fn holds(&self) -> Holds;
    /// Lock: everything it holds for the session goes; a public setting
    /// it keeps across a lock stays.
    fn lock(&mut self);
}

/// An area or value that holds nothing a lock wipes. Implemented by name
/// for each type, beside the type, so calling a field public is a claim
/// made where its type is written, and a reviewer reads it there.
pub(crate) trait Public {}
impl Public for bool {}
impl Public for f32 {}
impl Public for u64 {}
impl<T: Public> Public for Option<T> {}
impl<T: Public> Public for std::cell::Cell<T> {}
// … the other plain types the struct holds, each by name.

pub(crate) fn public<T: Public>(_: &T) {}
```

The listing of `Faraday`, one for reading and one for the lock. Each is
compile-checked against the struct; both are written from one macro so
they cannot classify a field differently:

```rust
macro_rules! faraday_areas {
    (public: [$($p:ident),* $(,)?], areas: [$($a:ident),* $(,)?],
     demo: [$($d:ident),* $(,)?]) => {
        impl Faraday {
            /// What every area holds.
            pub(crate) fn holds(&self) -> Holds {
                let Faraday { $($p,)* $($a,)* #[cfg(feature = "demo")] $($d,)* } = self;
                $( public($p); )*
                let mut h = Holds::NOTHING;
                $( h = h.or(AtLock::holds($a)); )*
                #[cfg(feature = "demo")]
                $( h = h.or(AtLock::holds($d)); )*
                h
            }
            /// Lock's clears, area by area.
            pub(crate) fn lock_areas(&mut self) {
                let Faraday { $($p,)* $($a,)* #[cfg(feature = "demo")] $($d,)* } = self;
                $( let _ = $p; )*
                $( AtLock::lock($a); )*
                #[cfg(feature = "demo")]
                $( AtLock::lock($d); )*
            }
        }
    };
}

faraday_areas! {
    public: [f, w, h, compact, pointer, out, clock_drawn, restart, restart_computer,
             last_display, settings, set_time, idle, tainted, inputs, now_ms, screen,
             sheet, sheet_offset, panel, stick, pin_h, list_offset, glance, scroll,
             guide, typing, trail, disclosed, fade, selftest, online, seed_line, held,
             shell_sticks, clipboard, medium, secure_boot, maker, maker_family,
             maker_model, devices, next_mode, samples, sample_area, in_progress],
    areas: [canvas, drawing, kept, vaults, memory, backups_area, fresh, session,
            decode, inbox, outbox, entry, wallets, visit, import, qr, spend_backup,
            vanity, backup, message, create, restore, keygen, wordlist, bip85, silent,
            explore, lightning, tools, secrets, camera, seed_read, later, worker, flows],
    demo: [transfer],
}
```

The split between `public` and `areas` above is §2's reading; each area
named "to be decided by the area" in §2 is decided where its impl is
written, and moves to `public` only with a `Public` impl beside its type.

The three answers, each one line:

```rust
pub fn holds_secret(&self) -> bool { self.holds().kept }
pub(crate) fn secret_typed(&self) -> bool { self.holds().typed }
pub(crate) fn lockable(&self) -> bool { self.holds().any() }
// clean_now: !tainted && !camera_on() && !(kept || typed); camera_on is
// the camera's own `typed` once the camera area answers it: a frame
// on the Scan sheet, its parts half read, a code carried to Unlock and a
// read card waiting under another sheet all count, whether or not the
// Scan sheet is up.
```

Where one area's answer needs another's (Make a new wallet's Keys card
counts only once something is entered, which reads `entry`), the area
that owns the fact answers it: `entry` answers for what is typed in it,
`keygen` for its own rolls and card. No answer reads a sibling.

**What changes for a person.** Clean and lockable, derived from every
area, count what §2 marks **no** today: Convert a seed, a new vault's
passphrases, restore's shares, a BIP-85 child, the calculator's typed
line, Spend from a backup. A stick is held back while one of them holds
a secret, and an idle session with only typed words or a tool's secret
warns and locks at the lock time rather than powering off at the
power-off time (and in Faraday Demo it locks rather than staying). This
is the intended change: it is what the requirement asks.

## 5. Three areas in full

**`ToolsArea`, an area that holds areas** (`faraday-core/src/tools/mod.rs`).
Its own fields are destructured with no rest pattern, so a new tool is a
compile error here:

```rust
impl AtLock for ToolsArea {
    fn holds(&self) -> Holds {
        let ToolsArea { tools, catalog_find, home_find, home_focus, home_bar_y,
                        home_follow, convert, kef_tool, qr_tool, ots, seed_check,
                        mscheck, core_wallet, invoice, address_check } = self;
        public(catalog_find); public(home_find); public(home_focus);
        public(home_bar_y); public(home_follow); public(ots); public(invoice);
        public(address_check);
        let calc = Holds { typed: tools.as_ref().is_some_and(|t| !t.typed.is_empty()),
                           ..Holds::NOTHING };
        calc.or(convert.holds()).or(kef_tool.holds()).or(qr_tool.holds())
            .or(seed_check.holds()).or(mscheck.holds()).or(core_wallet.holds())
    }
    fn lock(&mut self) {
        let ToolsArea { tools, catalog_find: _, home_find: _, home_focus: _,
                        home_bar_y: _, home_follow: _, convert, kef_tool, qr_tool,
                        ots: _, seed_check, mscheck, core_wallet, invoice: _,
                        address_check: _ } = self;
        *tools = None;
        convert.lock(); kef_tool.lock(); qr_tool.lock(); seed_check.lock();
        mscheck.lock(); core_wallet.lock();
    }
}
```

**`LightningArea`, a plain secret area** (`faraday-core/src/lightning/mod.rs`):

```rust
impl AtLock for LightningArea {
    fn holds(&self) -> Holds {
        // Any state: the words, the passphrase or the node key.
        Holds { typed: self.state.is_some(), ..Holds::NOTHING }
    }
    fn lock(&mut self) { *self = Self::default(); }
}
```

**`InboxArea`, an area partly kept across Lock** (`faraday-core/src/inbox.rs`).
What is public or encrypted stays for the fresh process; the rest goes:

```rust
impl AtLock for InboxArea {
    fn holds(&self) -> Holds {
        Holds {
            typed: self.potential().is_some()
                || self.files().iter().any(|i| i.dropped_at_lock()),
            session: false,
            kept: false,
        }
    }
    fn lock(&mut self) {
        // A picture goes as what its codes hold, and as Other when none read.
        self.retain(|i| !i.dropped_at_lock());
        self.clear_potential();
        self.hide_secrets();
    }
}
```

Each counts the Inbox's secret files as typed: today they taint the
process when read and are dropped at Lock, and the Inbox answers both.

## 6. `lock_finish` after the rule, and the clears being added now

`lock_finish` keeps, in its order, the steps that are not one area's:
the boot import's and visit's wipes where they read another area, the
signing text and `more::drop_gone` before `save_boxes_for_lock`, the
wallet list marked changed, then `restart` and `Command::Exit` last. In
place of its list of area calls it calls `self.lock_areas()`.

A clear added to `lock_finish` today (`app.lightning.clear()`, a tool's
`clear()`) is one area's lock written in the wrong place. When the trait
is built, each such line becomes the body of that area's `lock` and is
deleted from `lock_finish`; a line that clears a sub-field
(`app.tools.qr_tool.clear()`) becomes the body of the sub-area's `lock`,
reached through its holder's. The same goes for each `holds_secret` an
area has today (`seeds`, `seed_check`, `kef_tool`, `qr_tool`,
`core_wallet`, `wallet_backup`'s Krux form): it becomes the area's
`holds`, widened to everything the area's `lock` wipes (the QR vault
tool's made code; the Core wallet tool's file).

## 7. What holds the rule

**The compiler** holds that every field is decided: the pattern in
`faraday_areas!` and in each holder's impl, and the missing trait impl.
No test states it: a test does not state a design rule.

**The tests** hold what a person notices, and stay as they are:

- the leak flows that end in Lock, with the scan of every live block
  (`faraday-core/tests/leaks/`, `docs/SECRET-LEAKS.md` §1 point 2, §4);
- the tests that a lock wipes an area's state: the QR vault tool's
  (`tests/all/qr_tool.rs`, a lock wipes what was typed, a lock forgets
  what was opened), own randomness's, the GPG restore's, the sidebar's, the
  visit's questions, a vault's kept and wiped lines (`tests/all/vault_seal.rs`)
  and Spend from a backup after Lock (`tests/all/spend_backup.rs`);
- the idle tests for lockable (`tests/all/idle.rs`);
- the clean tests: a text typed in the Krux code tool leaves the session
  not clean (`tests/all/kef_tool.rs`), a stick read and written only by a
  clean process (`tests/all/backup_flow.rs`), the visit's write on a clean
  visit (`tests/all/visit.rs`).

**The leak tour.** The macro also writes `pub enum Area { … }`, one name
per field in `areas`, and `Area::ALL` (under the `testkit` feature). A
leak test, `faraday-core/tests/leaks/areas.rs`, matches on `Area` with no
wildcard arm, so a new area does not compile there until it has a flow:
for each area, the flow opens its screen through the testkit and plants
the §3 secret of `docs/SECRET-LEAKS.md` as a person would (typed, scanned
or loaded), checks that the session is no longer clean and that the idle
warning comes, presses Lock, and scans every live block. An area that
holds no secret of its own (a wallet's session state) has an arm that
says so and plants nothing. The flows join §4's list of
`docs/SECRET-LEAKS.md`, and the `Area` arm names the flow that covers it.
