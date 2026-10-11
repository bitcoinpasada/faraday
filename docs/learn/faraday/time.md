# Time and TOTP codes

## What a TOTP code is made from

A TOTP code is a hash of the vault entry's secret and the time, in
30-second steps since 1970. Two phones with the same secret and the
same time compute the same code. The secret alone is not enough: the
time has to be right too.

## Why the time has to be right

A code is good for one 30-second step either side of the right time,
about half a minute. A clock further off than that computes a
different code, which a site that checks it refuses.

## The Pi starts at 1970

The Raspberry Pi has no real-time clock. It starts counting from 1970
every power-on, which is not the right time, so Faraday treats it as
not known until it is set by hand.

## A PC's clock can be hours off

The stick reads the PC's hardware clock as UTC. A PC that runs Windows
keeps that clock in local time instead, so what Faraday reads back can
be hours off the real UTC time.

## Setting it by hand

Type the date and the next minute, and press Set as a phone's clock
turns over to that minute. The time zone is a fixed offset from UTC,
not a time zone database: change it by hand when daylight saving
starts or ends.

## Faraday never changes the computer's clock

Setting the time in Faraday keeps a correction here, in the app. It
never writes the computer's own clock, on the Pi or the stick: nothing
it does could change what another program reads from the hardware
clock.
