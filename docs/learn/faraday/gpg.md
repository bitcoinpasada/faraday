# GPG keys

## What the key is

A GPG key made here is an Ed25519 certification key with an Ed25519
signing subkey. The certification key certifies the user ID and the
subkey; the subkey signs files. The fingerprint identifies the
certification key, and the signing subkey has its own. The key's secret
parts live in one vault record. Only the public certificate, signatures
and the files below leave it.

## What the user ID is

The user ID is the name and the email typed when the key was made. It
is part of the public certificate: everyone who gets the certificate
reads it. It is permanent. A copy of the certificate already sent
elsewhere keeps the user ID it had, and a different name or email means
a different key.

## How long the key lasts

A new key is valid for 3 years, which is GnuPG's own default, a year
being 365 days. After that date the key signs nothing, and others see
it as expired. Renew counts the new term from today and signs the
certificate again. Others see the new term only after they import the
renewed certificate. A key marked revoked is not renewed.

## The revocation certificate

The revocation certificate is made with the key and kept in the key's
vault record, as GnuPG keeps one beside its keyring. Whoever has it can
revoke the key for everyone who imports it, and a revocation cannot be
taken back. Faraday never writes it as a public file: it goes out only
through the secret sheet, as `name-FINGERPRINT.rev`, and sealed in the
vault otherwise.

## Using the revocation certificate

The file has GnuPG's explanation first and a colon in front of the
first line of dashes, so that it cannot be imported by accident.

1. Open the file in a text editor and remove the colon before
`-----BEGIN PGP PUBLIC KEY BLOCK-----`.

2. Run `gpg --import name-FINGERPRINT.rev`. The key is then revoked in
that keyring.

3. Send the revoked key to the places that have the key: `gpg --export
FINGERPRINT`, to a key server or to the people who use it.

The certificate kept with the key states no reason. Make a revocation
asks for one, as `gpg --generate-revocation` does: 0 no reason
specified, 1 key has been compromised, 2 key is superseded, 3 key is no
longer used, and an optional description. It makes a new certificate and
leaves the kept one as it was. Hold to mark the key revoked marks the
key revoked in the vault, as importing its certificate marks it in
GnuPG: the key then signs and renews nothing.

## Signing a file

A signature covers exactly the bytes of the file. The page shows each
file's name, size and SHA-256, and the text of a small text file. The
SHA-256 is for comparing with the one another computer prints for the
same file before the signature is made, because a signature on a bad
file cannot be withdrawn. The signature is `name.asc` beside the file. `gpg --verify
name.asc name` checks it with the public certificate.

The signature states the time of this computer's clock. Sign shows
that time with the files before the hold, and Set the time there
changes it; with no known time nothing is signed. Faraday signs
nothing with a key that has expired, was revoked, or was made after the
clock's time, and says so in GnuPG's words.

## The paperkey

The paperkey is the secret numbers of the key in text. With the public
certificate it rebuilds the secret key: `gpg --dearmor` on the
certificate, then `paperkey --pubring cert.gpg --secrets paperkey.txt
--output secret.gpg`. It is shown while held and made only then.
Anyone who reads a copy of it has the key, so a printed paperkey is kept
as a secret is.

## Restoring a key from its paperkey

A lost key comes back into a vault from its paperkey and its public
certificate. The certificate is copied into Files first, on a stick
visit. Restore from its paperkey is in the certificate's ⋮ in Files,
and Restore from a paperkey is on the GPG kind in Vault contents. The
first card shows the public key and the digits line 1 of its paperkey
must read.

A paperkey made here is seven lines. Lines 1 to 5 hold 22 bytes each,
line 6 holds the rest, and line 7 is six digits alone: the check of the
whole key. Each of lines 1 to 6 ends in six digits, the check of that
line. A full line is 50 digits: 44 for the bytes, 6 for the check.

What is typed on a line:

- The digits 0 to 9 and A to F, in either case. The keyboard on the
  screen has only these.
- Spaces are not needed. Spaces typed anywhere are ignored, so a line
  can be typed in pairs as printed, in groups of four, or as one run of
  digits.
- The line's number and colon, as printed ("1:"), may be typed or left
  out. Where typed, the number is checked against the line.
- Line 1 starts with 00 04, the paperkey's format and the key's
  version, then the key's fingerprint. All three are typed: the line's
  check covers them.
- Lines starting with # in a file the paperkey program printed are not
  typed.

When the last digit of a line is typed and its check holds, the line is
taken and the next one opens by itself. The card then shows "Line 1 ·
checked" and, for line 1, the fingerprint it names; the digits are not
shown again. The count, "Line 3 of 7", shows once the key's length is read, by
line 3 at the latest. Line 6 also ends by itself, at that length, and
line 7 at its sixth digit.

A line that does not end by itself has a digit wrong, missing or extra.
Next line, or Enter, checks it as it is and shows why it was not taken,
"CRC on line 2 does not match" for a wrong digit. The digits stay as
typed and Backspace changes them; nothing is guessed or corrected.
Type line 2 again, under the lines checked, clears the last line
checked and the ones after it. A paperkey printed by the paperkey program with
another --output-width has lines of another length: each ends with
Next line.

With every line checked, Restore the key reads the secret, checks that
it makes the public key, and makes every signature of the key again,
which must equal the certificate's byte for byte. The key is then saved
in each vault of the plan.

## Faraday Demo

Faraday Demo is online and testnet-only; a key made on it meets
whatever else runs on that computer. A key made on the stick meets no
network.
