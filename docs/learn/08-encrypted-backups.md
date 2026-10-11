# Encrypted backups

## What it is

An encrypted backup is something of yours locked under a passphrase you choose, as one small file or one QR code. OpenSigner writes it, and OpenSigner or the short script published with it reads it back. Nothing about it is this app's secret: the format is written down, and the two ciphers it uses are standard ones.

It is a second copy, beside the words on paper or steel, that can be kept where a written copy cannot: on a memory card in a drawer, printed as a QR and posted to yourself, or kept by a family member who cannot read it.

## What it can hold

Four things. A key's seed words. A key's master seed, which is what a SLIP-39 or a Codex32 key is and which has no words. A note: anything you type on the device or read in from a plain text file, such as instructions for whoever comes after you. And a wallet's recovery sheet: its descriptor, what you call it, and a note — the document an heir is handed.

Every export asks which form you want it in before it asks for the passphrase. An encrypted backup is a file that OpenSigner and its decrypt.py script open. A KDBX 4 file is one that a KeePass app opens. A note and a recovery sheet can also be exported as plain text, which any text editor opens and nothing locks; a key never can. Where an encrypted backup fits one QR code, the screen after it is made offers to show it as one.

## The KDBX form

KDBX 4 is the file format KeePass uses. Choose it and what you are exporting becomes a small password database that KeePassXC, KeePassDX, KeePassium, KeePass itself and anything else that reads the format will open, on a computer or a phone, with no copy of OpenSigner and no knowledge of this project. It is meant for the person who comes after you.

What you get is one entry. A key's words become the entry's password, with the word count and the wordlist in its notes and the fingerprint as its title; a key with no words becomes its master seed as hex. A note becomes an entry's notes. A recovery sheet becomes an entry titled with the wallet's name, holding the descriptor and your note.

The passphrase you type is the database's master password, and it is the only thing locking it: OpenSigner writes no key file. The Argon2id memory you chose in Settings is written into the file, so the KeePass app opening it pays the same cost per guess that OpenSigner would.

Putting your words in a KeePass database makes them as safe as that database, which is as safe as the passphrase over it. That is the whole of it. Use a strong passphrase, and treat the file the way you would treat the words themselves.

OpenSigner writes KDBX files and does not read them. A KDBX file cannot be scanned back into the device, and there is no QR for one: it is a few kilobytes, and the apps that read it read files. If you want a backup this device can take back, use the encrypted backup.

## The passphrase is the backup

Whoever knows the passphrase and holds the file has your key. Whoever holds the file without the passphrase has a few hundred bytes that look random.

A file that holds a key is the same 345 bytes whether it holds twelve words or twenty-four or a master seed, and every file is offered under the same name. Longer things — a note, a sheet — are padded up in steps of 256 bytes, so a file's size says roughly how much is in it and no more. A file on its own does not say which wallet it belongs to or how long the seed is; you learn what is inside it by opening it.

Locking your words in a file makes the file as strong as its passphrase and no stronger. Whatever the container, the keys inside it are only as safe as the passphrase over them, so use a strong one.

If you lose the passphrase, you cannot open the file either. There is no reset and no recovery: the words inside cannot be reached any other way. Write the passphrase down and keep it somewhere the file is not.

## What makes a good one

Length is what counts. Four or five random words from a list beat a short phrase with symbols in it, and they are far easier to copy out correctly later. OpenSigner's dice passphrase tool under Tools rolls one for you.

A passphrase you already use somewhere else is not a passphrase: it is in a breach list. So is a name, a date, or a line from a song.

OpenSigner asks for at least eight characters and checks nothing beyond that. Eight characters is a minimum, not a recommendation.

## How it is protected

Your passphrase is not the key. The file carries a random salt, and the key is derived from the passphrase and that salt with Argon2id, which is deliberately slow and memory-hungry: a machine guessing passphrases against your file pays for that memory and three passes over it on every guess.

The words are then encrypted with XChaCha20-Poly1305, which also authenticates the file's own header. A file with a changed header, or a wrong passphrase, fails to open rather than opening to the wrong words.

A weak passphrase stays weak. The slow derivation only raises the cost of each guess.

## How much memory to pay

Settings has a Backup memory row: 64 MiB, 256 MiB or 1 GiB. More memory costs an attacker more per guess and costs you a longer wait each time you open the file. It also decides where the file can be opened: a device that cannot spare that much memory cannot open it at all, and says how much it needs.

Every file states its own cost in its header, so any device with the memory opens any file whatever its own setting is. OpenSigner recommends 256 MiB on a device with at least a gigabyte of memory and 64 MiB below that, and the recommended row says so.

## A key with a passphrase

If the key you are backing up has a wallet passphrase, the backup holds the words the passphrase is applied to, not the passphrase itself. Restoring gives you the words back; you type the wallet passphrase again as you always do. The result screen says so while it is on screen.

## Reading one back

Scan the QR, or read the file in, and OpenSigner asks the passphrase. "Read an encrypted backup" is a row in three places: Load a key, Tools › Notes, and a wallet's Recovery sheet. Each opens the same scanner, with Read a file under it. Words and a master seed land in the flow that adds a key, so the key is yours again. A note opens as a document. A recovery sheet opens as a document too, and offers to register the wallet its descriptor names.

## Reading one without OpenSigner

The screen after a backup is made states its format ("OSKB 3" or "KDBX 4"), its cipher, the key derivation with the passes and lanes it was made with, and what reads it. The OSKB format is written down in docs/BACKUP.md in OpenSigner's source repository, and tools/backup/decrypt.py in the same repository opens an OSKB file on any computer with Python. A KDBX 4 file needs neither: any KeePass app opens it.
