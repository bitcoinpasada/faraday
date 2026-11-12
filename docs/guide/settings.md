# Settings

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Settings`
**Key:** settings

The settings of the session and of how the app looks, in sections: Mode, Session, Time, Randomness, Appearance, Display, Signed amounts, Nonce history and About.

## Mode

**Key:** settings.mode

The network that keys, wallets and addresses are made for. The row shows the mode in use and opens a sheet with a row per mode the app offers; **Not now** closes the sheet.

The device offers **Mainnet**, **Testnet**, **Signet** and **Regtest**. Faraday Demo offers **Practice** in place of Mainnet.

## Session

**Key:** settings.session

Shows how many keys are loaded. **Lock** and **Remove every key** appear while a secret is held; **Power off**, or **Quit** in the desktop app, ends the session.

**Remove every key** unloads the keys and keeps the wallets. **Lock after** locks after the minutes with no input, and **Power off after** powers off after them, on the device only. Each is set to **2 min**, **5 min** or more, or **Never**. A line names any input device ignored until it is unplugged.

## Time

**Key:** settings.time

Shows the clock now and the **Time zone**. **Set the time** opens the page that sets the clock by hand. TOTP codes, GPG and Secure Boot signatures and backup check dates read this clock.

## Randomness

**Key:** settings.randomness

Shows what the session's randomness is made from: the system's, and the camera frames, dice rolls, coin flips, cards and keys pressed that were added. **Add randomness** opens the page where more is added.

## Appearance

**Key:** settings.appearance

**Theme** is chosen from ten named themes, from **Cobalt** to **Monochrome**. **Mode** is **Light** or **Dark**. **Contrast** is **Standard**, **Medium** or **High**. **Text size** is **Standard**, **Large** or **Larger**. **Reduce motion** shows changes at once, with no slide or fade.

## Display

**Key:** settings.display

**Scale** sets the size of text and controls: **Auto**, which is the size for the screen, or 75, 90, 110, 125, 150 or 200 %.

**Rotation** is **Landscape** or **Portrait**, offered only where the screen can rotate. Landscape suits the backup board and lists beside their detail; Portrait suits typing words in.

## Signed amounts

**Key:** settings.signed-amounts

The amounts each signed input stated are remembered, and the same transaction stated with other amounts is refused. The line gives how many transactions are remembered.

**Sealed into an open vault** keeps them past power-off. **Kept until power-off only** keeps them for the session.

## Nonce history

**Key:** settings.nonce-history

Each signature checked is remembered by its key, its nonce and its message; a nonce one key used on another message is refused. The line gives how many signatures are remembered.

**Kept in an open vault** stores them in the open vault. **Off** keeps none.

## About

**Key:** settings.about

The version, how keys are brought in, and where vaults and files move. **Self-test** gives the result of the self-test that runs at every start, before any key is accepted, and **Run again** repeats it.
