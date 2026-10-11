# The secure element

## When this applies

Only when you choose to keep a key on an Android device between sessions. Otherwise your seed words are in memory while OpenSigner runs and gone when it stops, and nothing on this page matters.

Keeping a key on a phone is a convenience with a cost. Read this page before you do it.

## What the chip does

A secure element is a separate chip inside the phone that holds its own keys and never hands them out. When you keep your keys on the phone, OpenSigner encrypts each key's words under a key that lives in that chip. Every key you add afterwards is kept the same way, under the one PIN, and a key you forget is removed from the phone. Opening them again requires the chip, your phone's screen lock, and the OpenSigner PIN together. A copy of the phone's storage is useless on its own. While a key is kept, OpenSigner opens on its PIN pad and shows nothing else until the PIN is entered.

The wallets you are using are kept beside the keys, and so are the notes and recovery sheets you ask it to keep: each one carries a "Keep on this device" switch, and the phone holds eight of them. They are encrypted under the same key as the words and they go the same way — a wipe, the duress PIN or eight wrong PINs takes them with the keys.

After eight wrong PINs in a row, on any PIN pad, OpenSigner has the chip delete its keys. The stored words are unrecoverable from that point, which ends any guessing attack for good. Your written backup is still your backup.

## What it cannot do

The chip cannot check what you are signing. The phone's software still draws the screen, and a compromised phone can still show you a false address. It does nothing for your written seed words, and nothing against someone who takes the phone out of your hand while OpenSigner is unlocked.

It never holds a passphrase. The passphrase stays in your head or on paper, and you type it every time.

## The duress PIN

You can set a second PIN. Entering it wherever OpenSigner asks for a PIN, including the lock screen, deletes the stored key and opens an empty OpenSigner, as if no key had ever been kept. Nothing in the stored data shows that a duress PIN exists. If you are forced to unlock the phone, the duress PIN gives the person forcing you nothing, and your written backup still has the funds.

## Losing the phone

The stored key is bound to that one chip. A new phone cannot open the old data, and neither can you. Recovery is your written seed words, exactly as it is everywhere else. A key kept on a phone is never a backup.
