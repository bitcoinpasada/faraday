# Sticks and vaults, laid out together: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/STICK-SETUP.md` on 2026-11-02, when the last
of its work was built. Each keeps its original number; the source keeps
a one-line stub under the same heading.

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

## 8. What changes with it

- `docs/BACKUP-FLOW.md`: §17.6 (the first placement, §6.1), §10.2 items 2 and 3 (the Sticks and vaults
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
- A 2-of-3 set up with three vaults on three sticks opens on the board
  with three places, each keeping one key's paper and the stick of that
  key's vault, and Continue live with nothing moved; after Customize
  backup, a stick moved to another place and a seed added to a vault
  are kept in the plan.
