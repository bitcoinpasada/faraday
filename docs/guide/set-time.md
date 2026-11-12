# Set the time

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SetTime`
**Key:** set-time

Sets the time by hand when this computer's clock is missing or wrong. It opens from a page that keeps the time it reads, or from Settings, and returns there.

## Now

**Key:** set-time.now

**Now** shows the date and time in the chosen zone, seconds included, or **Not known** while there is no usable clock reading. Below it a line says when the time is not this computer's clock: **No clock reading**, or **Counted from Set: no clock reading** after a time was set by hand.

**Time zone** steps in quarter hours, from -12:00 to +14:00, and is the zone the typed time is read in. It is a fixed offset, not a zone database, so it is changed by hand when daylight saving starts or ends. A Raspberry Pi has no real-time clock and starts at 1970, so its clock is not known until set; a PC's clock is read as UTC, and a PC that keeps local time reads hours off.

## Date and time

**Key:** set-time.entry

**Date** is typed as year, month and day (2025-03-09), from 2020 to 2099. **Time** is typed as 24-hour hours and minutes (14:05). The date is filled in when a time is already known, and **Time** is the field typed in first.

**Set** is offered once both are real; it counts from the typed minute at 0 seconds, so it is pressed as the clock turns over to that minute. It counts the clock on from the typed time, at 0 seconds, marks the clock checked, and returns; it never changes the computer's clock.

**Keep this time** is offered when a time is known. It marks the clock checked without changing it. **Back** returns with nothing marked.
