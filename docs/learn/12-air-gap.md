# The air gap

## What an air gap is

An air-gapped device has no connection to any network. OpenSigner has no networking code at all, so nothing can reach it over a network and nothing can leave it that way. The only ways in and out are QR codes, shown on a screen and read by a camera, and files that you carry across on an SD card or a USB stick yourself.

## What crosses the gap

Into OpenSigner: an unsigned transaction (a PSBT), an address to check, a signed message to check, a multisig wallet description, and your seed words when you load a key.

Out of OpenSigner: a public key for your coordinator, a signed transaction, a signed message. Your seed words and your private keys never cross the gap in either direction, except when you deliberately load them.

## What it protects against

Anything that needs a network to reach your key: remote attacks, malware on your computer reading the key out of memory, a malicious update that arrives on its own. If your coordinator is fully compromised, the attacker still does not have your key.

## What it does not protect against

An air gap does not check the transaction for you. A compromised coordinator can still hand you a PSBT that pays the wrong address, which is why you verify every address and amount on OpenSigner.

An air gap does nothing for your written backup, nothing against somebody who takes the device out of your hand while it is unlocked, and nothing against a camera pointed at your screen while your seed words are showing.
