# TOTP codes

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Codes`
**Key:** codes

Shows the current one-time code of every vault entry that holds a TOTP secret, across every open vault. It needs a vault open and the clock checked, and a code is worked out on this computer.

## Codes

**Key:** codes.list

**Now · UTC** gives the local time with seconds, or **Not known** while there is no usable clock reading; **Set the time** opens Set the time and returns here. A row without a checked clock shows **Set the time first** in place of its code. A code is worked out from the entry's secret and the time in steps of its period, so a clock wrong by more than a step gives a different code.

One row per entry: its title, under it the account (from the setup's label, else the entry's username) and, with more than one vault open, the vault's name. The code is grouped, with a bar and the seconds left of its period. An entry whose secret is a counter-based one shows **HOTP** and one that cannot be read shows **Unreadable**; neither gives a code. Pressing a row opens the entry in its vault.

## Search and add

**Key:** codes.add

**Search codes** keeps the rows whose title holds what is typed; **No codes match** shows when none does, and **No entries with a TOTP secret** when there are none. **Scan a setup code** adds an entry to the current vault from a code, and **Add an entry** opens the entry form for it.
