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

An Ed25519 key with a signing subkey. It shows its **Fingerprint**, **Signing subkey**, **User ID**, **Valid** term, **Revocation certificate** and a **Paperkey** behind its eye. **Make a key** makes one with an **Expires** term of 1 year, 2 years, 3 years, 5 years or **Never**.

**Export public key** writes the public key to the Outbox; **Show as QR** and **PNG** give it as a code and a picture. **Renew** sets a new term. **Sign a file** lists the files in Files to sign with the key. **Make a revocation** takes a **Reason**, and **Export the kept revocation** offers the certificate kept with the key. **Hold to mark the key revoked** marks it. **Restore from a paperkey** brings back a key from its paperkey.

## GPG key terms and revocation

**Key:** vault-contents.gpg-keys-term

The User ID is the name and email typed when the key was made and cannot change; a different one is a different key. The term counts in 365-day years. **Renew** counts the new term from today and signs the certificate again; others see it after they import the renewed certificate. A revoked key is not renewed.

**Export the kept revocation** writes the certificate made with the key as `name-FINGERPRINT.rev`, with a colon before its first line of dashes so it cannot be imported by accident; whoever has it can revoke the key. **Make a revocation** takes a **Reason** (**No reason specified**, **Key has been compromised**, **Key is superseded**, **Key is no longer used**) and a description, and leaves the kept one. **Hold to mark the key revoked** marks it revoked in the vault; it then signs and renews nothing.

## GPG signing and the paperkey

**Key:** vault-contents.gpg-keys-sign

**Sign a file** lists each chosen file with its size and the start of its SHA-256, for comparing with another computer's, and the **Signature time**, the time of this computer's clock. The signature is `name.asc`, checked with the public key. Nothing is signed with a key that has expired, was revoked, or was made after the clock's time, or with no known time.

The **Paperkey** is the key's secret numbers in text, shown behind its eye. With the public key it rebuilds the secret key. **Show as QR** shows it as one code for another vault to scan; it is never saved as a file.

## Secure Boot

**Key:** vault-contents.secure-boot

A set of Secure Boot keys, its **Owner GUID** and the names of its PK, KEK and db. **Make Secure Boot keys** makes the set.

**Enrolment adds** is **Windows-compatible**, which adds Microsoft's KEK and Windows CAs to the owner's keys, or **Own keys only**, which does not: Windows then does not start, nor any card or controller whose firmware Microsoft signed. **Enrolment files** writes the files to enrol. **Sign or check an image** signs or checks an EFI image.

## Secure Boot keys made

**Key:** vault-contents.secure-boot-keys

The set is three RSA-2048 keys, PK, KEK and db, each with a self-signed certificate valid for 30 years, named by the **Owner name** typed followed by PK, KEK or db, or "Faraday" when none was typed. The db key signs boot programs; the PK and the KEK sign only the enrolment files. The certificates carry the time they were made, so the time is asked first.

The private keys are one vault record and cannot be made again, so the set is made in the backup flow and kept at two places or more; until every place is written, **Enrolment files** reads **Back up first**.

## Secure Boot enrolment files

**Key:** vault-contents.secure-boot-files

**Enrolment files** writes nine files for the stick: PK.auth, KEK.auth and db.auth, the signed updates; PK.esl, KEK.esl and db.esl, the signature lists; and PK.cer, KEK.cer and db.cer, the certificates. Firmware setup screens take different ones. Faraday writes no setting of the computer.

**Windows-compatible** adds four of Microsoft's certificates: two KEK CAs to the KEK list and Windows Production PCA 2011 and Windows UEFI CA 2023 to db. Microsoft's third-party CA is not added.

**Sign or check an image** signs a release's boot program with the db key only when the release's signed sums list it and the release is at least as new as this Faraday; otherwise it names the refusal. Check reads whether the file is signed by this vault's db key, signed by another key, changed since it was signed, or not signed.
