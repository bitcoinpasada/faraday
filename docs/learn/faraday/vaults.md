# Vaults

## What a vault is

A vault is one encrypted file, kept on a stick, that holds keys, wallets, backup plans, notes and entries. Its file ends in .fvlt. Faraday seals it at every lock and writes it to its stick when the stick is put in, replacing the copy there.

A vault is made to keep seed words: a key sealed in a vault is its words, encrypted. It can also keep a seed's BIP-39 passphrase, with the seed or alone.

## Slots and decoy passphrases

Every vault has four slots of the same size, and each passphrase opens one of them. A vault is made with one to four passphrases. A slot no passphrase opens is random bytes and looks the same as a slot in use, so the file does not say how many passphrases it has. A second passphrase can open a decoy slot that holds less; unlocking takes the same work whichever slot opens.

This does not hold against two copies of the same vault from different times: a slot whose bytes changed between them is in use. It does not hide that a vault exists. Passphrases are set only when the vault is made; changing one, the cost or the size means making a new vault and moving what it holds into it.

## Older copies

Unlock shows how many times a vault has been sealed: "Sealed 12 times · last 28 Oct 2026, 14:02". A seal is a save: Faraday seals a vault when it locks it with changes. Opening a vault and changing nothing does not count. The count is inside the encryption, so no one can change it without the passphrase.

Someone who can write to a stick can put an older copy of a vault back. They cannot read it or change what is in it, but the older copy still unlocks. Compare the count with how often you remember changing the vault: a count lower than you expect is an older copy.

Faraday remembers the counts it has seen until the computer is turned off. Unlocking an older copy after a newer one in that time stops before the vault opens and says so: "An older copy: sealed 9 times." Faraday cannot tell an older copy you kept from one someone put back. Open carries on with that copy; Keep locked leaves it locked.

A backup plan kept in one vault records the counts of the other vaults it uses. Open the vault with the plan first, then the others: a vault older than the copy the plan records is named as older, even after the computer was turned off. With one vault, its plan is in the same file and goes back with it; then the count at unlock, against what you remember, is the check.

Unlock also reads the other copies of the vault in Files, with the passphrase just typed. If one has been sealed more times, Unlock stops and says "An older copy: sealed 9 times. A copy sealed 12 times is in Files." Keep this one locked and open that copy to carry on from it.

The count cannot show every copy put back at once, at every place; a vault no backup plan names; or another passphrase's slot, whose count only that passphrase reads.

A decoy slot has its own count. Opened under pressure, a decoy sealed twice beside a vault used every day shows it is little used. Change something in the decoy now and then, so its count grows.

## The cost of a guess

The passphrase is stretched with Argon2id before it opens anything, at a memory and a number of passes chosen when the vault is made and fixed for its life. Create vault offers PCs and a Raspberry Pi (64 MiB, 3 passes) and PCs only (512 MiB, 3 passes); Customise offers more.

The cost is chosen for the weakest computer the vault will ever be opened on. A computer without the memory cannot open the vault at all, and Unlock says how much it needs before the passphrase is typed. A slow computer opens it slowly.

If the passphrase is lost, the slot it opens is lost. There is no reset. Passphrase strength, below, gives the bits of a passphrase from dice and what the cost adds.

A passphrase is read in Unicode's composed form (NFC), so an accented letter typed as one character or as a letter and a mark opens the same slot. Two passphrases that differ only in that way are one passphrase.

## Load at unlock

Unlocking a vault from the boot sheet or the Vaults tab shows Load from the vault: a card for each wallet, with its seeds that the vault holds, and a Keys card for keys in no wallet. Load ticked loads what is ticked; Not now loads nothing and leaves the vault open.

The cards start from each record's Load at unlock, on its page in the vault: Yes or No. A seed kept only as a backup can be set to No, so it stays in the vault until it is needed. A flow that asked for the unlock, such as Spend or Restore, loads what it needs instead.

## KeePass export

An entry in a vault (a title, a username, a password, a URL and notes) can leave as a KeePass database: Export for KeePass on the entry, under a passphrase of at least eight characters. The file is KDBX 4, which KeePassXC, KeePassDX, KeePassium and KeePass open. It holds that one entry, and is as safe as its passphrase.

## Reading a vault without Faraday

The format is published in Faraday's source, in docs/VAULT.md. tools/vault/open.py in the same repository opens a vault on any computer with Python 3 and either argon2-cffi with pynacl, or cryptography 44 or newer, and prints what the slot holds:

python3 tools/vault/open.py vault.fvlt

It asks for the passphrase without showing it and writes nothing.
