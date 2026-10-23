# Vaults

## What a vault is

A vault is one encrypted file, kept on a stick, that holds keys, wallets, backup plans, notes and entries. Its file ends in .ofv. Faraday seals it at every lock and writes it to its stick when the stick is put in, replacing the copy there.

A vault is made to keep seed words: a key sealed in a vault is its words, encrypted. It can also keep a seed's BIP-39 passphrase, with the seed or alone.

## Slots and decoy passphrases

Every vault has four slots of the same size, and each passphrase opens one of them. A vault is made with one to four passphrases. A slot no passphrase opens is random bytes and looks the same as a slot in use, so the file does not say how many passphrases it has. A second passphrase can open a decoy slot that holds less; unlocking takes the same work whichever slot opens.

This does not hold against two copies of the same vault from different times: a slot whose bytes changed between them is in use. It does not hide that a vault exists. Passphrases are set only when the vault is made; changing one, the cost or the size means making a new vault and moving what it holds into it.

## The cost of a guess

The passphrase is stretched with Argon2id before it opens anything, at a memory and a number of passes chosen when the vault is made and fixed for its life. Create vault offers PCs and a Raspberry Pi (64 MiB, 3 passes) and PCs only (512 MiB, 3 passes); Customise offers more.

The cost is chosen for the weakest computer the vault will ever be opened on. A computer without the memory cannot open the vault at all, and Unlock says how much it needs before the passphrase is typed. A slow computer opens it slowly.

## Passphrase strength

The passphrase matters more than the cost. The whole range of costs adds about six bits to every guess. One more word from the long dice list adds 12.9 bits.

Six words rolled with dice, Generate with dice on Create vault, are 77.5 bits, beyond reach at any cost. A short passphrase, or one used anywhere else, is not protected by any cost. Create vault shows the bits of a passphrase the dice made, with the cost's bits added, and does not measure a typed one.

If the passphrase is lost, the slot it opens is lost. There is no reset.

## Load at unlock

Unlocking a vault from the boot sheet or the Vaults tab shows Load from the vault: a card for each wallet, with its seeds that the vault holds, and a Keys card for keys in no wallet. Load ticked loads what is ticked; Not now loads nothing and leaves the vault open.

The cards start from each record's Load at unlock, on its page in the vault: Yes or No. A seed kept only as a backup can be set to No, so it stays in the vault until it is needed. A flow that asked for the unlock, such as Spend or Restore, loads what it needs instead.

## KeePass export

An entry in a vault (a title, a username, a password, a URL and notes) can leave as a KeePass database: Export for KeePass on the entry, under a passphrase of at least eight characters. The file is KDBX 4, which KeePassXC, KeePassDX, KeePassium and KeePass open. It holds that one entry, and is as safe as its passphrase.

## Reading a vault without Faraday

The format is published in Faraday's source, in docs/VAULT.md. tools/vault/open.py in the same repository opens a vault on any computer with Python 3 and either argon2-cffi with pynacl, or cryptography 44 or newer, and prints what the slot holds:

python3 tools/vault/open.py vault.ofv

It asks for the passphrase without showing it and writes nothing.
