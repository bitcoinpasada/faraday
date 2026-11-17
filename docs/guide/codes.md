# TOTP codes

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Codes`
**Key:** codes

Codes shows the current one-time code of every vault entry that holds a TOTP secret, across every open vault. It needs a vault open and the clock checked; each code is worked out on this computer.

## Codes

**Key:** codes.list

**Now · UTC** gives the local time with seconds, or **Not known** while there is no usable clock reading. **Set the time** opens Set the time and returns here. Until the clock is checked, a row shows **Set the time first** in place of its code. A code comes from the entry's secret and the time, in steps of its period, so a clock wrong by more than a step gives a different code.

One row per entry: its title and, under it, the account (from the setup's label, or else the entry's username) and, when more than one vault is open, the vault's name. The code is grouped, with a bar and the seconds left in its period. An entry with a counter-based secret shows **HOTP**, and one that cannot be read shows **Unreadable**; neither gives a code. Press a row to open the entry in its vault.

## Search and add

**Key:** codes.add

**Search codes** keeps the rows whose title contains what you type. **No codes match** shows when none does, and **No entries with a TOTP secret** when there are none. **Scan a setup code**, filled while there are no entries, adds an entry to the current vault from a code, and **Add an entry** opens the entry form for one.
