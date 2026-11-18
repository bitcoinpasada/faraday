# Set the time

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SetTime`
**Key:** set-time

Set the time lets you set the time by hand when this computer's clock is missing or wrong. It opens from a page that keeps the time it reads, or from Settings, and returns there.

## Now

**Key:** set-time.now

**Now** shows the date and time in your zone, seconds included, or **Not known** while there is no usable clock reading. Below it, a line says when the time is not this computer's clock: **No clock reading**, or **Counted from Set: no clock reading**.

**Time zone** reads **Not set** until you choose it, and **Set** is not offered before. Choose your zone once from the list: each row is an offset with its cities, such as UTC-05:00 with New York and Chicago. The stick keeps it, so you are asked once. **Other offset** steps in quarter hours for an offset the list lacks. The list does not know daylight saving: choose the new row when your clocks change.

A Raspberry Pi has no clock and starts at 1970. A PC's clock is read as UTC, so a PC that keeps local time reads hours off.

## Date and time

**Key:** set-time.entry

Type the time your phone shows, not UTC. **Date** is year, month and day (2025-03-09), from 2020 to 2099, and **Local time** is 24-hour hours and minutes (14:05) in your zone. A line then shows your time and the same time in UTC: check it against your phone before **Set**.

**Set** counts the clock on from the typed minute at 0 seconds, so press it as the clock turns over to that minute. It then marks the clock checked and returns; it never changes the computer's clock.

**Keep this time** is offered when a time is known. If **Now** matches your phone, press it: it marks the clock checked without changing it. If it does not, type your phone's time and press **Set**. **Back** returns with nothing marked.
