# Secure Boot keys

## The three keys

Secure Boot keys made here are three RSA-2048 keys, each with a
self-signed X.509 certificate valid for 30 years: the platform key (PK),
the key exchange key (KEK) and the signature database key (db). Their
certificates are named by the owner's name typed when they were made,
followed by PK, KEK or db, or "Faraday PK", "Faraday KEK" and "Faraday
db" when none was typed. The three are listed under one owner GUID, made with
them.

A computer's firmware keeps one PK, a KEK list and a db list. An
update to the KEK list must be signed by the PK, and an update to db
by a KEK. With Secure Boot on, the firmware starts a boot program only
when its signature verifies with a certificate in db. The db key signs
boot programs; the PK and the KEK sign only the enrolment files.

## How they are made

The keys are drawn from 32 bytes of the system's randomness. A ChaCha20
keystream over those bytes, written for Faraday, feeds the key
generation: one stream for the GUID and the certificates' serial
numbers, one for the keys, one that blinds the signing. The RSA key
generation is the `rsa` crate's, a release candidate before its 1.0.
The certificates carry the time they were made, so Faraday asks for
the time first.

## Where they are kept

The private keys live in one vault record. Only certificates,
signature lists, signed updates and signed boot programs leave it.

Secure Boot keys cannot be made again from seed words, so they are
made inside the backup flow, kept at two places or more. Until every
place is written, Enrolment files reads Back up first and nothing is
signed. Keys in a vault unlocked from a stick sign at once.

## Enrolment files

Enrolment files writes nine files For the stick: `PK.auth`, `KEK.auth`
and `db.auth`, the signed updates (PK signed by the PK, KEK by the PK,
db by the KEK); `PK.esl`, `KEK.esl` and `db.esl`, the signature lists;
and `PK.cer`, `KEK.cer` and `db.cer`, the certificates. Firmware setup
screens take different ones of these.

Windows-compatible adds four of Microsoft's certificates: Microsoft
Corporation KEK CA 2011 and Microsoft Corporation KEK 2K CA 2023 to
the KEK list, Microsoft Windows Production PCA 2011 and Windows UEFI CA
2023 to db. Windows and its updates still start. Microsoft's
third-party certificate is not added, so a boot program signed only by
it does not start. Own keys only adds none: only what the db key signed
starts, and Windows does not.

Faraday never writes a setting of the computer. The keys are enrolled
in the firmware's own setup screen, from the files on the stick.

## Signing a boot program

Sign or check an image signs a release's `BOOTX64.EFI` with the db key:
an Authenticode signature over the file's image hash, SHA-256, added at
the end of the file. Faraday signs it only when the release's
`SHA256SUMS` and `SHA256SUMS.asc` are beside it in the Inbox, the
signature over `SHA256SUMS` verifies with the release key built into
Faraday, `SHA256SUMS` lists the file under its release name with its
SHA-256, and the release is at least as new as the Faraday signing it.
Otherwise the page names the refusal and nothing is signed. To sign an
older release, sign it on that release's Faraday.

Check reads a file's signature against the vault's db certificate:
signed by this vault's db key, signed by another key, changed since it
was signed, or not signed.

## Where this comes from

The UEFI specification 2.10 (signature lists and authenticated
variables), Microsoft's Windows Authenticode Portable Executable
Signature Format, X.509 and CMS. Faraday's own design: decisions F31
and F134, in Faraday's repository, as of 2026-10-28.
