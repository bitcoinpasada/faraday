# Vault contents

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `VaultContents`
**Key:** vault-contents

What an open vault holds, in categories: Bitcoin keys, Wallets, Entries, Notes, GPG keys and Secure Boot. A category is listed once it holds an item, with its count.

## Bitcoin keys

**Key:** vault-contents.bitcoin-keys

Each item shows its **Fingerprint**, its **Words** behind an eye, the **SeedQR** of English words, and whether a **BIP-39 passphrase** is **Stored**. **Save a key** adds the keys loaded in this session, and **Save with its passphrase** adds one with its BIP-39 passphrase.

**Load at unlock** is **Yes** or **No**: a key marked so loads when the vault opens. **Load into session** loads the key now, **Load with a passphrase** loads a key of words with a passphrase typed, and **Save to a stick** offers the key's files to write to a stick. **Remove codex32 set** and **Remove BIP-85 origin** drop that record from the seed at the next seal.

## Wallets

**Key:** vault-contents.wallets

Each wallet shows its chart, with where each seed is kept, and its **Descriptor**. **Save a wallet** adds a wallet loaded in this session. **Search wallets** filters the list by title.

**Open in Wallets** loads it. **Load at unlock** and **Don't load at unlock** set whether it loads when the vault opens, as for a key; a wallet whose seed is in the same vault loads with it. **Show as QR** shows the descriptor as a code, and **Save to a stick** offers its files to write.

## Entries

**Key:** vault-contents.entries

An entry is a login: **Username**, **Password**, **URL**, a **TOTP secret** and **Notes**, a secret shown only behind its eye. **Add an entry** makes one and **Edit** changes it; **Search entries** filters by title.

**Scan a code** adds entries from a code, and **Import from Files** adds the entries files in Files. **TOTP codes** opens the codes of every open vault. **Export for KeePass** seals the entry into a KeePass file under a passphrase typed twice.

## Notes

**Key:** vault-contents.notes

A note is free text, shown as **Text** behind its eye. **Add a note** makes one and **Edit** changes it.

A recovery sheet is listed here as well: its **Descriptor** and its **Note**, with **Show as QR** and **PNG** for the descriptor as a code and as a picture.

## GPG keys

**Key:** vault-contents.gpg-keys

An Ed25519 key with a signing subkey. It shows its **Fingerprint**, **Signing subkey**, **User ID**, **Valid** term, **Revocation certificate** and a **Paperkey** behind its eye. **Make a key** makes one with an **Expires** term of 1 year, 2 years, 5 years or **Never**.

**Export public key** writes the public key to the Outbox; **Show as QR** and **PNG** give it as a code and a picture. **Renew** sets a new term. **Sign a file** lists the files in Files to sign with the key. **Make a revocation** takes a **Reason**, and **Export the kept revocation** offers the certificate kept with the key. **Hold to mark the key revoked** marks it. **Restore from a paperkey** brings back a key from its paperkey.

## Secure Boot

**Key:** vault-contents.secure-boot

A set of Secure Boot keys, its **Owner GUID** and the names of its PK, KEK and db. **Make Secure Boot keys** makes the set.

**Enrolment adds** is **Windows-compatible**, which adds Microsoft's KEK and Windows CAs to the owner's keys, or **Own keys only**, which does not: Windows then does not start, nor any card or controller whose firmware Microsoft signed. **Enrolment files** writes the files to enrol. **Sign or check an image** signs or checks an EFI image.
