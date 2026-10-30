# Sticks and vaults, laid out together

**Kind:** Spec
**Status:** Specified (F175), not built
**Open:** all of it: the Sticks and vaults card (§2), the sticks drawn as they come in (§3), Check it later (§4), what the device tells of a stick (§5), the plan after the keys (§6), the guide (§7)

Make a new wallet and Restore ask how many vaults the backup uses, then
want one stick per vault, one at a time, in vault order. This spec puts
the vaults and the sticks on one card instead: the person says how many
of each, lays the vaults onto the sticks as on the backup board, and
then sees each stick drawn on screen as it is brought in and checked.
Every choice here is the owner's of 2026-10-30 (F175). It changes
`docs/BACKUP-FLOW.md` §4.2 and §10.2; §8 is the edit list for them.

## 1. What changes

| Now (`docs/BACKUP-FLOW.md`) | With this spec |
|---|---|
| "How many vaults?" (§10.2 item 2), then one stick per vault | One card: how many vaults and how many sticks, and which vaults go on which stick (§2) |
| One vault per stick in the setup | Several vaults on one stick allowed, with a warning; a vault on several sticks is copies (§2.4) |
| The sticks asked for in vault order, "Insert the stick for Vault 1" | In any order; each stick drawn with its state (§3) |
| Every stick read before the keys | Read before the keys where it must be; a stick for new vaults only may be checked when it is written (§4) |
| The boot stick offered only as "Use {label}", left out of the sticks the flow waits for | The boot stick can be any of the sticks (§3.3) |
| A stick known by its label | By its maker, model, size and name; the rest on a press (§5) |

The numbering is unchanged (§16, F146): the boot stick is always
**Boot stick**, and the other sticks are Stick 2, Stick 3, and so on.

A second Faraday stick, plugged in after start, is a stick like any
other: `docs/STICK-IDENTITY.md` §5.2 makes only the disk the device
started from the boot medium. This spec assumes that is built.

## 2. The Sticks and vaults card

### 2.1 Where it is

Make a new wallet and Restore, after **Use vaults** answered Yes
(`docs/BACKUP-FLOW.md` §10.2 item 1), in place of **Number of vaults**
and before **Bring in the sticks**. Its title: **Sticks and vaults**.

### 2.2 The counts

Two steppers, the one the card uses now (`Ui`'s stepper,
`faraday/faraday-core/src/backup_flow/wallet_screen.rs`, `count_card`),
side by side where the width allows, else one under the other:

- **Vaults**: 1 to 1024, as now.
- **Sticks**: 1 to 1024, the total number of sticks the backup uses,
  the boot stick counted where it is one of them. Its first value is the
  number of vaults.

Changing a count changes the layout below at once (§2.3).

### 2.3 The layout

Under the counts, the vaults and the sticks as the backup board draws
them (`docs/BACKUP-FLOW.md` §17.1, §17.17): a chip per vault ("Vault
1"), and a box per stick with the stick glyph. The sticks are named
**Stick 2**, **Stick 3**, … in order; the boot stick is not placed
here (it is chosen for a stick when the sticks are brought in, §3.3),
so the card names no stick "Boot stick".

- **At first**, Vault 1 on Stick 2, Vault 2 on Stick 3, and so on; with
  more vaults than sticks the rest go on the last stick, and with more
  sticks than vaults the sticks after them hold a copy of the last
  vault. Raising a count adds a vault or a stick at the end and places
  it by the same rule; lowering one takes the last one away, and a vault
  on no stick goes back to the list of vaults.
- **Moving** is the board's (§17.2): drag a vault chip onto a stick to
  put it there, from one stick to another to move it, back onto the list
  of vaults, or by its ×, to take it off that stick. Tap to place, as
  there.
- **The ⋮ menu** on every chip, as §17.20: **Put on…** (each stick),
  **Move to…**, **Copy to…** (also kept on that stick), **Take off**.
  On the narrow column and the Pi's panel, the same ⋮.

### 2.4 Each vault: new, or one already on a stick

Each vault chip carries a segmented button (DESIGN §4.2), **New** or
**Already on a stick**, New at first. A vault already on a stick is
brought in from its stick before the keys (§17.21 item 5): which file
it is is asked when its stick is read, as now ("Which vault is Vault
1?", §4.2). This is how the card knows which sticks must be read now
(§4).

### 2.5 Checks

Under the layout, as the board's checks are (§17.5), one line each:

- a vault on no stick: "Vault 2 is on no stick". **Continue** is not
  live while one is.
- a stick holding two or more vaults: "Vault 2 and Vault 3 share
  Stick 3: losing it loses both". A warning; Continue stays live.
- a stick holding no vault: "Stick 4 holds no vault". A warning.
- a vault already on a stick that the layout puts on more than one
  stick: the stick it is read from is its first; the others take
  copies. No line.

**Continue** goes on to Bring in the sticks.

## 3. Bring in the sticks

### 3.1 The sticks drawn

The card draws a figure per stick of §2, in order, wrapping into rows:
the stick glyph, its name (Stick 2), the vaults it will hold, and its
state under them:

- **Waiting**: nothing in yet. Dim.
- **Reading**: a stick in, being read. Edged in the flow's colour, as
  the pill of the stick wanted now (§16).
- **Checked**: read and usable, with what tells it apart (§5) and, for a
  vault already on it, the file chosen. Lit, with a tick.
- **A problem**, in one line, as §4.2's usability checks: "Not FAT32",
  "Cannot be written", "Not enough room: 40 MB needed, 12 MB free",
  "This is a boot partition". The stick is not bound; another goes in
  its place.
- **Checked when written** (§4).

The pills across the top (§16) stay as they are and follow the same
states.

### 3.2 Any order

A stick put in goes to the stick the person pressed last among those
waiting, or, with none pressed, the first one waiting. The binding is
§4.2's as it is: a tap **This is Stick 3** when it reads unlike every
stick already bound, the hold when it reads the same as one, the
refusal when it is one bound to another stick. "One stick at a time" as
now. **Pull it**, then the next.

### 3.3 The boot stick

The boot stick is in already, so a waiting stick offers **Use the boot
stick** (on its figure, and in its ⋮). Chosen, that figure reads **Boot
stick**, and the boot stick is read as any stick is (its data
partition; its boot partition is never used, F128). The boot stick is
never chosen by default, and serves one stick of the layout at most.
Chosen for a stick, it frees the stick number it took: the sticks after
it are numbered again from 2, so the names stay Boot stick, Stick 2,
Stick 3 with no gap.

### 3.4 Continue

**Continue** is live once every stick is Checked or Checked when
written. It goes on to Vaults (§4.3: unlock or make each vault), as
now.

## 4. Check it later

A stick none of whose vaults is **Already on a stick** may be checked
when it is written. Its figure, while Waiting, offers **Check it
later** (and its ⋮). A stick holding a vault already on a stick does
not: that vault is read only before the keys.

The press opens a dialog (Material 3, Dialogs) before anything changes.
Its title: **Check Stick 3 later?** Its text, as statements of fact:

- A stick is not read while a key or a vault is open, so a stick not
  read now is read at the write step, after the lock.
- The vaults are made now either way. Until the stick is written, each
  is kept sealed in this device's memory only.
- At the write step the stick must be FAT32, writable, not a boot
  partition, and have room for its vaults ({size}).
- If it fails then, another stick that passes can take its place. With
  none at hand, turning this device off or restarting it loses those
  vaults, and any key kept only in them.

Its buttons: **Check it later** and **Cancel**. Chosen, the figure
reads **Checked when written**, and the stick is checked at the write
step as a stick named after step 2 is (`docs/BACKUP-FLOW.md` §4.6).
A stick put off that turns out to hold vaults loses nothing: the new
vault is written beside them (§4.6).

## 5. What tells a stick apart

On its figure once read, in this order, as much as fits on one line:
its maker and model as the stick reports them, its size ("16 GB"), and its name where it has one. A Pi card
shows the card's name and size; Faraday Demo, the folder's name.

A press on a checked figure opens a sheet (Material 3, Bottom sheets;
a dialog on a wide screen) with the rest where known: the name, the
FAT volume serial, the free room, the files at its top with their sizes,
and the vaults on it. Nothing there is a press but **Close**.

The device does not tell maker, model or size today: `faraday-disk`
reads them from `/sys` with the partitions it lists (a USB disk's
`device/vendor` and `device/model`, a card's `device/name`, and
`size`), and the stick's listing carries them
(`faraday/faraday-files/src/proto.rs`, `Stick`). Nothing is read from
the stick for them.

## 6. After the keys

The sticks of §2 are the plan's sticks: Back up's plan opens with each
of them as a stick on the board, numbered as here (F146), holding the
vaults laid on it, and each at a place of its own in order, as the
setup's sticks are given to places now (§10.2 item 6). The board moves
them from there.

The board holds one vault per stick (§17.4); this spec lets a stick
hold several. `At::Stick(p, s)` holds any number of `VaultStick(v)`,
the board draws one vault box per vault on the stick, and the write
step (§4.6) writes each vault file on the stick in one visit, as it
writes a vault and its files now.

## 7. The guide

The card's Guided text and the guide's walks (`docs/guide/walk/make-a-wallet.md`,
`docs/guide/walk/restore.md`, `docs/guide/walk/back-up.md`) say why the
sticks are laid out and read here, before the keys: a stick is not read
while a key or a vault is open; reading them now makes sure every vault
has a stick that works before there is anything to lose; nothing is
written to them until the lock; and what Check it later means (§4).

## 8. What changes with it

- `docs/BACKUP-FLOW.md`: §10.2 items 2 and 3 (the Sticks and vaults
  card in place of Number of vaults; the sticks in any order), §4.2
  (the figures, Use the boot stick in place of Use {label} for the
  setup, Check it later), §17.4 (several vaults on a stick), §16 (the
  boot stick's pill follows the stick it serves), its worked examples B
  to D (§6 of that file).
- `docs/FLOWS.md`, Make a new wallet and Restore: the card's name and
  order.
- Code: `faraday/faraday-core/src/backup_flow/wallet_screen.rs`
  (`count_card`, `create_count`, `restore_count`), `steps.rs` (the
  sticks' order, `flow_candidates` and the boot stick, Check it later),
  `mod.rs` (the setup's sticks and their vaults), `screen.rs` (the
  figures), the board's model (§17.4);
  `faraday/faraday-disk/src/lib.rs` and
  `faraday/faraday-files/src/proto.rs` (maker, model, size);
  Faraday Demo's sticks (the folder's name).
- Strings, in `faraday/faraday-strings/src/en.rs`: Sticks and vaults,
  Sticks, the checks of §2.5, New, Already on a stick, Put on…, the
  states of §3.1, Use the boot stick, Check it later and its dialog,
  the stick's sheet; retired: Number of vaults and its question once
  nothing uses them.

## 9. Tests

What a person notices:

- Three vaults and two sticks: the card places Vault 1 on Stick 2 and
  Vaults 2 and 3 on Stick 3, and says they share it; Continue is live.
- A vault taken off every stick: Continue is not live, and the line
  names it.
- Sticks put in in the order Stick 3, then Stick 2: each is bound to
  the stick it was put in for, and the figures say Checked.
- The boot stick chosen for the first stick: it is read with no stick
  put in, reads Boot stick, and the next stick is Stick 2.
- A second Faraday stick plugged in after start is taken as a stick.
- Check it later on a stick of new vaults: the dialog first; once
  chosen, Continue is live with that stick unread; at the write step the
  stick is checked, and one that fails offers another.
- No Check it later on a stick holding a vault already on a stick.
- A stick's figure shows its maker, model and size from the disk's
  fixture; its sheet shows its files.
- Two vaults on one stick are both written to it, each its own file,
  and both unlock.

## 10. Choices made here

Within F175, this spec chose the following; each is the owner's to
change:

- The card's first layout when the counts differ (§2.3): extra vaults
  on the last stick; extra sticks take copies of the last vault.
- A stick holding no vault is a warning, not a refusal (§2.5).
- The boot stick, chosen for a stick, takes that stick's place and the
  others are numbered again from 2 (§3.3).
- The stick's details open in a sheet with nothing but Close (§5).
