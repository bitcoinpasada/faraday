# The air gap

## What an air gap is

An air-gapped device has no connection to any network. Faraday has no networking code, and the system a stick or a Pi starts has no network drivers at all, so nothing can reach it over a network and nothing can leave it that way. The only ways in and out are QR codes, shown on a screen and read by a camera, and files that you carry across on a stick yourself (on the Pi, the SD card).

Faraday Demo runs on an ordinary computer that is online. It has no air gap, and a seed that holds funds does not belong there.

## What crosses the gap

Into Faraday: an unsigned transaction (a PSBT), an address to check, a signed message to check, a wallet description, a vault, and your seed words when you load a key.

Out of Faraday: a public key for your coordinator, a signed transaction, a signed message, a vault, GPG certificates and signatures, and Secure Boot enrolment files and signed EFI images. Your seed words and private keys leave only sealed in a vault, or unprotected when you ask for that and confirm the warning; word and SeedQR files leave after a warning.

## What it protects against

Anything that needs a network to reach your key: remote attacks, malware on your computer reading the key out of memory, a malicious update that arrives on its own. If your coordinator is fully compromised, the attacker still does not have your key.

## What it does not protect against

An air gap does not check the transaction for you. A compromised coordinator can still hand you a PSBT that pays the wrong address, which is why you verify every address and amount on Faraday.

An air gap does nothing for your written backup, nothing against somebody who takes the device out of your hand while it is unlocked, and nothing against a camera pointed at your screen while your seed words are showing.
