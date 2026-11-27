# Decode a transaction

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Decode`
**Key:** decode
**Words:** psbt

This page takes a transaction apart, field by field, for one that you only want to read. Nothing is signed.

## Opening

**Key:** decode.opening

Opened from Tools, it opens on the transaction when exactly one waits in Files, and on a list of the waiting transactions to choose from when several do. With none it says **No transaction waits in Files** and offers **Scan**, **Open Files** and, while a stick is attached, Bring in from that stick. The result ends with **Another**, which shows the list again (from a file), and **Done**, which goes back to where Decode was opened.

## Type it

**Key:** decode.type

**Type it** opens a field for a transaction or a PSBT typed or pasted as hex or base64. **Next** reads it as a file of the same bytes is read, and refuses what such a file is refused. The source line says **Typed text**.

The text is not put in Files. **Another** on the result shows the field with the text again; **Back** leaves the page and drops it, as a lock does.

## Summary

**Key:** decode.summary

Under the title you see where the transaction came from. **Transaction id** and **Witness id** are its two hashes, **Version** and **Locktime** are its fields, and **Size** gives bytes, virtual bytes and weight units.

**Fee** is the amounts in minus the amounts out, in sats and in sat/vB. **Amounts in** names the PSBT the input amounts came from, because a transaction does not hold them. With none here the row says **Not here**, and **Fee** says **Not stated: the amounts going in are not here**. PSBTs of it that state different amounts are not used, and the row says **PSBTs of it state different amounts**.

## Inputs

**Key:** decode.inputs

**Inputs** lists each input by its number, the outpoint it spends and, where a PSBT states it, its amount. The line under the outpoint gives the sequence as a number with what it means, and the witness as **Witness** with its items and their sizes in bytes, or **No witness**. A scriptSig size is added when the input has one.

## Outputs

**Key:** decode.outputs

**Outputs** lists each output by its number, its address and its amount. Under the address are its script type and whose it is. An output to a wallet loaded here is marked **Change**, back to that wallet, or back to that wallet as a receive address, with the address's index. Any other output reads **Pays out of the loaded wallets**.

A transaction whose outputs, or whose stated input amounts, total over 21,000,000 BTC is refused, not decoded.

## The hex

**Key:** decode.hex

**Show the hex** shows the transaction's bytes in hexadecimal and **Hide the hex** closes them. **Show as QR** shows the transaction as a code. Neither appears when the screen was opened from a place that refuses the transaction.
