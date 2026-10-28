# Before you rely on a key

**Kind:** Spec
**Status:** Written and checked against the app's strings and the security model
**Open:** the pictures, drawn once after the theming has landed

Faraday is an offline key store and signer: it keeps a key for one
session, and signs what it is shown. What makes a
key safe to rely on is its backup, the copy of Faraday that made it,
and what is signed with it. This page checks the first two and says
what Faraday cannot check.

**You end with:** a key restored once from its backup alone, the
Faraday download checked, and a list of what Faraday does not protect
against.

**Before you start:** [Make a key](new-key.md) and
[Back it up](back-up.md), or, for several keys,
[Make a wallet](make-a-wallet.md) as well.

**You'll need:**

- The Faraday stick, and the PC it starts.
- The backup of one place: the paper copy of the words, or the vault's
  stick and its passphrase.
- The fingerprint written down when the key was made.

**Time:** about 30 minutes.

## 1. Restore the key once from the backup alone

A backup is known to work only once a key has come back from it, with
nothing else at hand.

1. Power off Faraday. Everything in memory is wiped.
2. Put away everything that is not the backup being checked: no other
   paper, no note of the words, no second stick.
3. Start the PC from the Faraday stick again. Faraday starts with no
   key loaded.
4. Load the key from the backup alone:
   - From paper: press **Wallets → Add a key**, **Change** on
     **Start**, then **Type the words**. Type the words from the paper
     and press **Continue**.
   - From a vault: plug the vault's stick in. On **Stick visit**,
     press **Import 1 file**, then pull the stick
     ([Files, sticks and QR codes, step 2](files-and-sticks.md#2-a-stick-visit-bring-files-in)).
     In **Files**, press **Unlock** on the vault's row, type the
     passphrase, press **Unlock**, then **Load ticked**
     ([Restore a wallet, step 7.4](restore.md#74-from-a-vault)).
   - A wallet of several keys: **Restore an existing wallet**, as
     [Restore a wallet](restore.md) does it, then check it as
     [step 7.3](restore.md#73-check-then-done) says.
5. Compare the fingerprint under **Keys** in the sidebar with the one
   written down. Every character must match.
6. Press **Lock**, or power off.

| Result | Meaning | Do |
|---|---|---|
| the fingerprint matches | the backup gives this key | nothing more: the backup works |
| the words are refused | a word on the paper is wrong or missing | do not rely on the key. Correct the paper from another place's backup, and restore again |
| another fingerprint | the backup gives another key: a wrong word that still makes a valid list, a wrong passphrase, or another key's paper | do not rely on the key until a restore gives its fingerprint |

Repeat this for each place that holds enough to restore the key, and
again after a backup is changed.

## 2. Check the download

A key is only as sound as the Faraday that made it. Check the Starter
file before writing a stick from it, as the README's
[Verifying a release](../../../README.md#verifying-a-release) says:
import the release key, compare its fingerprint with one from a second
place (the README on GitHub and the website, for example), then check
the signature of `SHA256SUMS` and the file's sum. The Starter's
**About** lists the same commands under **Checking this file**, and the
SHA-256 of each image it carries.

A file that does not pass the check is not used: download it again.

## 3. What Faraday does not protect against

- A Faraday program that is not the real one. Checking the download
  (step 2) is the defence; reproducible builds let others check that
  the published files come from the published code.
- A compromised computer below Faraday. Changed firmware, hardware
  implants, and someone reading the computer's memory just after it is
  turned off.
- The management processors some computers carry under the
  operating system (Intel's Management Engine with AMT, AMD's PSP).
  Learn: [What else runs on this computer](../../learn/faraday/boot/what-else-runs.md).
- Anything that records the screen or the keys: a smart TV, a
  capture card, a KVM switch, a camera, or a person watching. Learn:
  [Screens and devices that can record](../../learn/faraday/boot/screens-that-record.md).
- A weak passphrase on a vault or a key.
- A wrong address in what is signed. Faraday shows every output and
  the fee; it cannot know whether an address is the one meant. Read
  them on Faraday's screen before signing, and check a receive address
  there. Learn: [Verifying](../../learn/06-verifying.md).
- A lost or unreadable backup. Faraday keeps nothing after power
  off: without a backup that works, the key is gone. Learn:
  [Backups](../../learn/04-backups.md).
- Someone who asks for the words. No support desk, wallet or
  person needs them. Learn: [Mistakes and scams](../../learn/08-scams.md).
