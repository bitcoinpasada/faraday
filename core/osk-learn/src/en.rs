//! The English Learn pages.
//!
//! `docs/learn/` holds the same text as Markdown, which is where it is
//! edited; `just learn-import` writes it here and `just lint` fails
//! while the two differ (`tools/learn/sync.py`).

use crate::{Learn, Page, Section};

/// The Learn pages in English.
pub static EN: Learn = Learn {
    start_here: Page {
        title: "Start here",
        sections: &[
            Section {
                heading: "What the words are",
                paragraphs: &[
                    "A key is a very large random number. So that a person can write one down without error, it is encoded as a list of words: 12 or 24 of them, drawn from a fixed list of 2048.",
                    "Those words are the key. Anyone who reads them can spend the coins, and nobody can give them back to you once they are lost. They are never photographed, never typed into a website, and never read out to anyone.",
                ],
            },
            Section {
                heading: "Plan the backup first",
                paragraphs: &[
                    "A backup is your words written down and kept away from every computer. Written on paper, the words are safe from being forgotten. Stamped or punched into steel, they are also safe from fire and flood. Decide where two copies will live before you make a key.",
                    "Keep two copies in two places. Before you rely on a copy, read it back into the device with the backup quiz; the device marks the key as verified once you have answered every question from your written copy.",
                ],
            },
            Section {
                heading: "What this device is",
                paragraphs: &[
                    "OpenSigner is a signer: it holds keys and signs transactions another program built. It has no networking code, so it shows no balance and no history. Watching addresses and building transactions is the work of a coordinator, on a machine that is online.",
                ],
            },
            Section {
                heading: "What happens next",
                paragraphs: &[
                    "Create a key or load one you already have, write the words down, prove the copy with the backup quiz, and check one address against your coordinator. To practise on a test chain first, set Network to signet in Settings before you make the key.",
                ],
            },
        ],
    },
    words: Page {
        title: "Seed words",
        sections: &[
            Section {
                heading: "Seed words are a key",
                paragraphs: &[
                    "A key is a very large random number. So that a person can write it down without error, it is encoded as a list of words: 12, 24, or one of three lengths between.",
                    "The words are the key. Anyone who reads them can spend the coins, and there is no bank or company that can reverse the transfer. Do not photograph them, type them into a website, send them in a message, or show them to anyone.",
                    "Keys lists one row per key you have loaded: the fingerprint that names it, and under that what the key is made of — \"12 words\", \"24 words and a passphrase\", \"BIP-85 child\".",
                ],
            },
            Section {
                heading: "Why 12 or 24",
                paragraphs: &[
                    "The word protocol is BIP-39. It has a list of 2048 words, and 2048 is 2 to the 11th, so each word carries 11 bits. Twelve words carry 132 bits: 128 bits of key and 4 of checksum. No amount of computing can guess a 128-bit number, so 12 words are enough. Twenty-four words carry 256 bits of key and 8 of checksum, for people who want a larger margin.",
                    "The three lengths in between are rarer than 12 and 24, and nothing else distinguishes them: they are built by the same rule, entropy plus a checksum of one bit per four bytes of it, and every wallet that follows BIP-39 reads them. 15 words are 160 bits, 18 are 192, and 21 are 224. OpenSigner creates and loads all five.",
                ],
            },
            Section {
                heading: "How they are made",
                paragraphs: &[
                    "The randomness a key is made from is called entropy. A key made from something you remember, such as a sentence or a favourite set of words, is one of a small number of possibilities however long it is, and cracking programs try those possibilities first.",
                    "Real entropy comes from a physical process such as rolling dice, flipping coins or shuffling a deck. 128 flips of a coin, or 50 rolls of a die, give 128 bits. OpenSigner turns the rolls or flips into words on the device, so no computer has to generate the key for you.",
                    "A backup can be written as numbers instead of words, because each word has a place from 1 to 2048 on the BIP-39 list; Load a key › Word numbers takes a key that way, one number per word.",
                    "A key's entropy is sometimes written down as hex digits rather than words; Load a key › Hex entropy takes those 32 to 64 digits and computes the words from them.",
                ],
            },
            Section {
                heading: "Keys and other secrets from one key",
                paragraphs: &[
                    "BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.",
                    "The key page's BIP-85 row asks which application. Words derives another BIP-39 key, which is added to Keys like any other and can be loaded on another wallet. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16, 32 or 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.",
                    "A derived password or WIF is as secret as the key it came from: anyone who has it can compute nothing about the parent, but they hold whatever it protects, and someone who has the parent words can recompute every child. The device shows the value on a screen that stays masked until you hold it, does not copy it, and keeps nothing: write it down or type it in, and derive it again from the same key and index when you need it.",
                ],
            },
            Section {
                heading: "Safe handling",
                paragraphs: &[
                    "There is no backup except the ones you make, there is no account, and there is nobody who can help you recover. If you lose the words, the coins are gone. If somebody else reads the words, they can take the coins. Backups has its own page; read it before you make a key.",
                ],
            },
        ],
    },
    devices: Page {
        title: "Bitcoin software",
        sections: &[
            Section {
                heading: "Miners",
                paragraphs: &[
                    "Miner software uses specialized hardware for guessing random numbers to validate blocks and move the chain forward.",
                ],
            },
            Section {
                heading: "Nodes",
                paragraphs: &[
                    "Node software runs on normal computers. You can run it at home if you have 1 TB of free space. Nodes validate transactions and keep a copy of all blocks. When you want to see your balance or initiate a transaction, you must connect to a node, either your own or someone else's.",
                ],
            },
            Section {
                heading: "Hot Wallets",
                paragraphs: &[
                    "Wallet software runs everywhere on computers and mobile devices. Wallets connect to a node to check account balances and create transactions. Most wallets can also generate entropy and manage funds directly, in which case it is called a hot wallet. Hot wallets are not recommended for large amounts.",
                ],
            },
            Section {
                heading: "Read-only Wallets",
                paragraphs: &[
                    "Most wallet software has an option to add a \"read-only\" wallet. This requires putting in something called a public key. The public key is related to your seed words, but it cannot spend funds. Despite the name, since the public key allows checking balances, it should still be considered private information that you don't want to share.",
                ],
            },
            Section {
                heading: "Cold Wallets",
                paragraphs: &[
                    "Cold wallets generally refer to writing down your key or seed words offline, for example on paper or other archival media. Some people also include permanently offline devices or archival hardware (like a USB drive) in this category. In order to use a cold wallet, you need two separate pieces of software, typically on different devices: a coordinator and a signer.",
                ],
            },
            Section {
                heading: "Coordinators",
                paragraphs: &[
                    "The coordinator is the internet-connected side of the cold wallet. It does not contain the keys and cannot spend funds. Most coordinators are also read-only wallets. Not every read-only wallet software can function as a coordinator.",
                ],
            },
            Section {
                heading: "Signers",
                paragraphs: &[
                    "The signer is the offline side of the cold wallet, and it does contain the keys to spend funds. The most well known signers, often called hardware wallets, are small devices like Ledger and Trezor. Essentially any computer can be used as a signer, but sloppy usage will result in loss of funds.",
                    "We recommend using OpenSigner on a permanently offline computer, such as an old Linux laptop or a Raspberry Pi with the WiFi module removed. You can also use OpenSigner on Android or GrapheneOS, although we do not recommend using your daily driver to store more than you're willing to lose. A phone whose bootloader is unlocked, or which is rooted, is not a signer: OpenSigner refuses to run on one.",
                ],
            },
            Section {
                heading: "Keys, Wallets, Coordinators and Signers in OpenSigner",
                paragraphs: &[
                    "OpenSigner uses four words in one fixed way, and its screens follow them.",
                    "A **key** is a secret: seed words, with or without a passphrase. It has a page of its own under Keys, with what it is made of, its backup, and Forget.",
                    "A **wallet** is public data that turns keys into addresses. It is either one key at a script type, which OpenSigner calls a single-sig wallet and lists for every key you load, or a policy over several keys, which is a multisig or MuSig2 wallet you load from a coordinator. A wallet has addresses and a descriptor, and the descriptor is what you give a coordinator. Home lists them, one row each.",
                    "Every wallet row carries a mark. A key means this device holds the private key, so it can sign for that wallet. An eye means public keys only: you can derive and compare addresses, and never spend.",
                    "A wallet can be loaded from a public key alone — a descriptor, an extended public key or a coordinator's export — and it carries the eye until one of its keys is a key you have loaded.",
                    "OpenSigner holds wallets, but it is still not a wallet in the sense the rest of this page uses the word: it has no networking code, no balances and no transaction history.",
                    "A **coordinator** is the internet-connected software that watches those addresses, shows balances and builds transactions. A **signer** is the offline software that holds the keys and signs what the coordinator built. OpenSigner is a signer.",
                ],
            },
            Section {
                heading: "What a row on Home is",
                paragraphs: &[
                    "Wallets lists every wallet you added and Keys lists every key you loaded. A key does not get a wallet by itself: you add a wallet that uses it and choose its script type, or you load a coordinator's description.",
                    "The mark on the row says which kind it is. A key means this device holds a private key of that wallet and can sign for it. An eye means the wallet is public keys alone: addresses to derive and compare, nothing to spend.",
                    "Tapping a row opens the wallet's page, and everything you do with that wallet is there: sign, addresses, check an address, export, the key or keys it is made of, and forget.",
                ],
            },
            Section {
                heading: "OpenSigner",
                paragraphs: &[
                    "OpenSigner needs at least one key. Create one, or load one from a backup you already have. A SeedQR loads a key in one scan; keep the code private and check for cameras before you show it.",
                    "OpenSigner keeps the key in memory while the device is on and wipes it when the device is turned off, so after that the key exists only in your written backup. Keeping a key on the device is offered only on Android phones with a secure element, behind the device lock and a PIN of its own.",
                    "On a phone, switching to another app does not lock OpenSigner: the auto-lock and auto-wipe timers run the same whether the app is in front of you or behind another one, and coming back after the auto-lock time is up shows the lock screen.",
                    "OpenSigner is not a wallet and has no networking code whatsoever. It cannot connect to the internet or a Bitcoin node. You cannot use it to view balances or initiate transactions.",
                    "To use OpenSigner, you must use a coordinator. The coordinator can communicate with OpenSigner either via QR codes or by putting files on the SD card. The coordinator will show your balances, addresses, and transfer destinations. You should verify the entire transaction on OpenSigner before signing.",
                    "A Raspberry Pi or a laptop booted from a USB stick has no secure boot: nothing checks the software before it runs. Write the image yourself and compare its hash with the published build before you use it.",
                ],
            },
        ],
    },
    transactions: Page {
        title: "Transactions",
        sections: &[
            Section {
                heading: "Inputs and outputs",
                paragraphs: &[
                    "A transaction takes coins you control, called inputs, and creates new coins from them, called outputs. Those new coins can be owned by you, or you can transfer ownership to someone else.",
                ],
            },
            Section {
                heading: "Change",
                paragraphs: &[
                    "You cannot spend part of an input. If you control a 10,000 sat input and spend 5,000 sats, the whole 10,000 sat input is broken up into two new coins: the 5,000 sat output (sent to someone else) and your change less the miner fee.",
                    "Check the change output of every transaction before you sign. If the device cannot derive a change output from your wallet, treat that output as a payment to somebody else.",
                ],
            },
            Section {
                heading: "Fees",
                paragraphs: &[
                    "The fee is simply the inputs less the outputs. If you spend 10,000 sats as 5,000 to somebody else plus 4,900 in change to yourself, the fee is 100 sats.",
                ],
            },
            Section {
                heading: "Setup and PSBT",
                paragraphs: &[
                    "To set up a transaction for OpenSigner, you need a coordinator that can create Partially-Signed Bitcoin Transactions, or PSBTs. You can transfer the PSBT to OpenSigner via QR code or the SD card.",
                ],
            },
            Section {
                heading: "Signing",
                paragraphs: &[
                    "Before you sign, OpenSigner lists every input and output and marks the amounts you control. The Signatures row on the inputs page lists every signature the transaction already carries, with the key it is under and whether it verifies. Read every address and amount in full and compare them with what your coordinator shows. If anything differs, do not sign; go back to the coordinator and find out why.",
                ],
            },
        ],
    },
    randomness: Page {
        title: "Randomness",
        sections: &[
            Section {
                heading: "Entropy is the whole key",
                paragraphs: &[
                    "A key is only as strong as the randomness it came from. If the process that produced your seed words was predictable, then someone who knows that process can produce the same words, and there is nothing else protecting the funds. A seed made from a favourite quote, a keyboard pattern, or a \"random\" phrase you typed yourself will be cracked.",
                ],
            },
            Section {
                heading: "Dice and coins",
                paragraphs: &[
                    "Dice and coins are physical randomness that you can watch happen. Each roll or flip is a real event in front of you, you can count them, and no software took part. That is the whole reason OpenSigner asks for dice rolls or coin flips instead of generating a seed for you.",
                    "A 6-sided die gives about 2.58 bits per roll, so 50 rolls give 128 bits. A coin gives 1 bit per flip, so 128 flips give 128 bits. Use a real die and a real coin, not a phone app.",
                    "A longer key asks for more of both. 12 words take 50 rolls or 128 flips; 15 words take 62 rolls or 160 flips; 18 words take 75 rolls or 192 flips; 21 words take 87 rolls or 224 flips; 24 words take 99 rolls or 256 flips. The screen states the count it is waiting for and counts your entries against it.",
                    "While you enter dice rolls, OpenSigner counts how often each face came up and flags long runs and lopsided counts. That check catches a loaded die or a lazy hand. It cannot turn bad rolls into good ones. If it warns you, roll again.",
                ],
            },
            Section {
                heading: "Computer randomness",
                paragraphs: &[
                    "Every computer has a random number generator, built from a hardware chip plus the operating system. It is usually fine. The problem is that you cannot see it working. A chip that is faulty, or that was built to be predictable on purpose, produces keys that look perfectly normal and that its maker can recreate. Nobody can tell from the outside.",
                    "For this reason OpenSigner lists dice first and marks the device's own generator with the trust it asks for, because you can inspect a die and you cannot inspect a chip.",
                ],
            },
            Section {
                heading: "From randomness to keys",
                paragraphs: &[
                    "Your 128 or 256 bits become the seed words. The words are then stretched by an algorithm called PBKDF2 into a 64-byte seed, using the words plus your passphrase (if any) as input. BIP-32 splits that seed into a master private key and a chain code, and every address in every account of the wallet is derived from there.",
                    "Nothing further down that chain adds randomness: every key and address in the wallet is computed from the bits you started with.",
                ],
            },
        ],
    },
    where_randomness: Page {
        title: "Where randomness comes from",
        sections: &[
            Section {
                heading: "Every source asks for trust somewhere",
                paragraphs: &[
                    "Create offers several ways to produce the bits a key is made of. They are not equally good. The difference between them is what you have to trust for the result to be random at all. This page says, for each source, what that is.",
                    "None of them can be checked afterwards: a key from a rigged source looks exactly like a key from a fair one. Choosing the source is the only control you have over this.",
                ],
            },
            Section {
                heading: "Dice and coins",
                paragraphs: &[
                    "With a die or a coin, the only thing you trust is the object in your hand. You can see every event, count them, and repeat the whole process with a different die. The device only records what you type.",
                    "A die gives about 2.58 bits a roll, so 50 rolls give 128 bits and 99 give 256. A coin gives one bit a flip: 128 or 256 of them. The rolls are hashed with SHA-256 and the flips are packed as bits, which is what other signers do, so you can reproduce either result on a computer you trust.",
                    "The counts for the three lengths in between follow the same arithmetic: 62 rolls or 160 flips for 15 words, 75 rolls or 192 flips for 18 words, 87 rolls or 224 flips for 21 words.",
                ],
            },
            Section {
                heading: "Three ways to read the dice",
                paragraphs: &[
                    "Other signers turn dice into a key in more than one way, and the same rolls give different keys under each. The device asks which one you are following, so that a key you made elsewhere comes out the same here.",
                    "**Hashed** is the first row and what most signers do. The rolls are written out as digits and hashed with SHA-256. Coldcard, SeedSigner and EntropyLab's \"Base 10\" row all do exactly this, so `echo -n 3246115135… | sha256sum` reproduces the result on any computer.",
                    "**Six as zero, hashed** writes every 6 as a 0 first and then hashes the same way. Keystone uses it, and so does the \"Dice\" mode of iancoleman's page. It is no better or worse than the first; it is a different convention, and a key made under one will not appear under the other.",
                    "**Words chosen by the dice** does not hash at all. Five rolls of 1 to 4 and a sixth roll read as a coin — 1 to 3 heads, 4 to 6 tails — pick one word out of 2048 directly, because 4 × 4 × 4 × 4 × 4 × 2 is 2048. A 5 or a 6 among the first five is rolled again. This is the BitBox paper table. It costs more rolls: 72 for 12 words against 50, and 144 for 24 against 99. The last word's final rolls only fill bits the checksum replaces, so a device may stop before them, at 70 for 12 words and 140 for 24. What you get for them is a key you can check by hand against a printed wordlist, without a computer anywhere in it.",
                    "The dice name every word, the last one too. The last word also carries the checksum, so only its first bits come from the dice — 7 of its 11 at 12 words, 3 at 24 — and the device writes the checksum into the rest. The last word of your key is usually not the word the table names for the last six rolls. SeedSigner completes a last word the same way.",
                ],
            },
            Section {
                heading: "Playing cards",
                paragraphs: &[
                    "With a deck you trust the shuffle. A deck shuffled seven times by hand is thoroughly mixed; a deck out of its box is not shuffled at all, and a deck a card trick has been done with may be in an order somebody knows.",
                    "Each draw is worth the log of the cards still in the deck: the first is one of 52, the second one of 51. Twenty-five draws carry 128 bits. 256 bits needs 58 draws, which is more than a deck holds, so the device asks for a second, freshly shuffled deck once the first is spent. It refuses a card that is already out, because a card you enter twice adds nothing and usually means you misread one.",
                    "Because each draw is worth less than the one before it, the counts climb faster than the lengths do: 25 draws for 12 words, 31 for 15, 39 for 18, 50 for 21 and 58 for 24.",
                    "The entropy is SHA-256 over the cards' numbers, in the order you drew them.",
                ],
            },
            Section {
                heading: "Camera noise",
                paragraphs: &[
                    "The sensor in a camera produces a slightly different picture every time, even of the same still scene, because of electrical noise in the sensor itself. The device hashes the pixels of the frames you take. It asks for three frames at 12 words and six at 24, and four or five for the lengths in between.",
                    "With the camera you trust the sensor and the software that delivers its frames, which you cannot inspect as you can a die. A camera that returns the same frame twice, or a frame it was given rather than one it took, would produce a key somebody else can produce as well, and nothing on screen would look wrong.",
                    "The device refuses a frame with fewer than 32 distinct brightness values, which is what a covered lens or a blown-out picture gives. It states the frame's size, its distinct values, its mean and its variance as facts, not as a verdict: those numbers cannot tell you the sensor is honest.",
                ],
            },
            Section {
                heading: "This device",
                paragraphs: &[
                    "Every computer has a random number generator, built from a hardware source plus the operating system. It is usually fine, and it is what this device uses to blind its own calculations.",
                    "Using it for a key asks you to trust this device completely. A generator that is faulty, or that was built to be predictable, produces keys that look perfectly normal and that its maker can recreate. You can check a die. You cannot check a chip.",
                    "That is the whole caution, and it is why the row carries one. In a browser the generator belongs to the browser and to the page, so the row is not offered at all.",
                ],
            },
            Section {
                heading: "Mixing",
                paragraphs: &[
                    "A mix runs two or more sources in turn and combines them. The result is at least as good as the best source in it: an attacker who controls the camera but not your dice learns nothing, because the dice are still in the hash.",
                    "Before the words appear, the device lists one commitment per source: the SHA-256 of what that source put in. The key is SHA-256 over those commitments, in that order. Write them down first. If you ever want to check that no source was quietly changed after the fact, recompute that one hash and compare.",
                ],
            },
        ],
    },
    backups: Page {
        title: "Backups",
        sections: &[
            Section {
                heading: "What a backup is",
                paragraphs: &[
                    "A backup is your seed words written down and kept away from every computer. That is all it is. If you have a passphrase, the passphrase is part of the backup too, stored separately.",
                    "Paper is enough against forgetting. Steel, stamped or punched with the words, is what survives fire and flooding. Steel backup plates are cheap compared to what they protect.",
                ],
            },
            Section {
                heading: "What destroys a backup",
                paragraphs: &[
                    "Fire. Water. Damp. Ink that fades. A house move. A relative tidying up. A safe that nobody else can open.",
                    "Two copies in two places are far better than one perfect copy in one place. If one location is lost, the other still works.",
                ],
            },
            Section {
                heading: "Where to keep it",
                paragraphs: &[
                    "Somewhere you control, that a burglar does not search first, and that a person you trust can reach if you cannot. A bank deposit box, a safe at a family member's house, or a fireproof box hidden at home are all common choices, each with their own trade-offs. Think about who could reach it and who could not.",
                ],
            },
            Section {
                heading: "NEVER",
                paragraphs: &[
                    "NEVER photograph your seed words.",
                    "NEVER put them in cloud storage, a password manager, a notes app, a spreadsheet, or an email, not even \"temporarily\".",
                    "NEVER type them into a website or into any software that asks for them.",
                    "NEVER read them out to anyone, no matter who they say they are.",
                ],
            },
            Section {
                heading: "Verify your backup",
                paragraphs: &[
                    "A backup you have never read back may be wrong: handwriting is misread, words get skipped, and steel gets stamped wrong. OpenSigner's backup quiz checks your written copy: it asks for every word in random order, offers four candidates each time, and checks your answers against the key in memory.",
                    "If you skip the quiz, the key stays marked as \"not verified\". That does not mean the backup is wrong. It means nobody has checked.",
                    "Run the quiz again from your physical backup about once a year, and after every move. Ink fades, and you may forget where a copy is.",
                ],
            },
            Section {
                heading: "A backup plan's check",
                paragraphs: &[
                    "Faraday's backup starts with a plan: where the seeds go, how many places keep paper, and where the wallet description goes. Under the plan's map it checks three things, by trying each place lost and each place found in turn. A place here is anywhere a copy is kept: a paper place, a stick of unprotected files, or the vault's stick when no paper place keeps one. Seeds that live on their own devices count as kept and never as found.",
                ],
            },
            Section {
                heading: "Any one place lost",
                paragraphs: &[
                    "\"Any one place lost: the rest rebuild the wallet\" is Yes when, for every place, the other places still hold enough: a quorum of seeds, each with its passphrase if it has one, and for a wallet of more than one key the wallet description, whole or from shares that together hold every key. Watch-only software counts as a copy of the description.",
                    "\"Yes, with the vault's passphrase\" means at least one loss is covered only by the vault, so the vault's passphrase has to be remembered or kept somewhere else. No means one place is a single point of failure.",
                ],
            },
            Section {
                heading: "One place found: can spend",
                paragraphs: &[
                    "\"One place found: can spend\" is Yes when one place on its own holds a quorum of seeds, with their passphrases, and for a wallet of more than one key the description. Anyone who finds that place can take the coins.",
                    "\"Only with the vault's passphrase\" means the place holds a stick with the vault, and the vault holds what is missing: the finder also needs the vault's passphrase. No is what a plan should aim for.",
                ],
            },
            Section {
                heading: "One place found: sees the balance",
                paragraphs: &[
                    "\"One place found: sees the balance\" is Yes when one place holds the whole wallet description, or for a wallet of one key its seed. Whoever finds it can watch every payment in and out, but spend nothing. A share of a split backup leaves keys off, so one share alone does not show the balance.",
                    "\"Only with the vault's passphrase\" means the description is in the vault and that place keeps the vault's stick.",
                ],
            },
        ],
    },
    encrypted_backups: Page {
        title: "Encrypted backups",
        sections: &[
            Section {
                heading: "What it is",
                paragraphs: &[
                    "An encrypted backup is something of yours locked under a passphrase you choose, as one small file or one QR code. OpenSigner writes it, and OpenSigner or the short script published with it reads it back. Nothing about it is this app's secret: the format is written down, and the two ciphers it uses are standard ones.",
                    "It is a second copy, beside the words on paper or steel, that can be kept where a written copy cannot: on a memory card in a drawer, printed as a QR and posted to yourself, or kept by a family member who cannot read it.",
                ],
            },
            Section {
                heading: "What it can hold",
                paragraphs: &[
                    "Four things. A key's seed words. A key's master seed, which is what a SLIP-39 or a Codex32 key is and which has no words. A note: anything you type on the device or read in from a plain text file, such as instructions for whoever comes after you. And a wallet's recovery sheet: its descriptor, what you call it, and a note — the document an heir is handed.",
                    "Every export asks which form you want it in before it asks for the passphrase. An encrypted backup is a file that OpenSigner and its decrypt.py script open. A KDBX 4 file is one that a KeePass app opens. A note and a recovery sheet can also be exported as plain text, which any text editor opens and nothing locks; a key never can. Where an encrypted backup fits one QR code, the screen after it is made offers to show it as one.",
                ],
            },
            Section {
                heading: "The KDBX form",
                paragraphs: &[
                    "KDBX 4 is the file format KeePass uses. Choose it and what you are exporting becomes a small password database that KeePassXC, KeePassDX, KeePassium, KeePass itself and anything else that reads the format will open, on a computer or a phone, with no copy of OpenSigner and no knowledge of this project. It is meant for the person who comes after you.",
                    "What you get is one entry. A key's words become the entry's password, with the word count and the wordlist in its notes and the fingerprint as its title; a key with no words becomes its master seed as hex. A note becomes an entry's notes. A recovery sheet becomes an entry titled with the wallet's name, holding the descriptor and your note.",
                    "The passphrase you type is the database's master password, and it is the only thing locking it: OpenSigner writes no key file. The Argon2id memory you chose in Settings is written into the file, so the KeePass app opening it pays the same cost per guess that OpenSigner would.",
                    "Putting your words in a KeePass database makes them as safe as that database, which is as safe as the passphrase over it. That is the whole of it. Use a strong passphrase, and treat the file the way you would treat the words themselves.",
                    "OpenSigner writes KDBX files and does not read them. A KDBX file cannot be scanned back into the device, and there is no QR for one: it is a few kilobytes, and the apps that read it read files. If you want a backup this device can take back, use the encrypted backup.",
                ],
            },
            Section {
                heading: "The passphrase is the backup",
                paragraphs: &[
                    "Whoever knows the passphrase and holds the file has your key. Whoever holds the file without the passphrase has a few hundred bytes that look random.",
                    "A file that holds a key is the same 345 bytes whether it holds twelve words or twenty-four or a master seed, and every file is offered under the same name. Longer things — a note, a sheet — are padded up in steps of 256 bytes, so a file's size says roughly how much is in it and no more. A file on its own does not say which wallet it belongs to or how long the seed is; you learn what is inside it by opening it.",
                    "Locking your words in a file makes the file as strong as its passphrase and no stronger. Whatever the container, the keys inside it are only as safe as the passphrase over them, so use a strong one.",
                    "If you lose the passphrase, you cannot open the file either. There is no reset and no recovery: the words inside cannot be reached any other way. Write the passphrase down and keep it somewhere the file is not.",
                ],
            },
            Section {
                heading: "What makes a good one",
                paragraphs: &[
                    "Length is what counts. Four or five random words from a list beat a short phrase with symbols in it, and they are far easier to copy out correctly later. OpenSigner's dice passphrase tool under Tools rolls one for you.",
                    "A passphrase you already use somewhere else is not a passphrase: it is in a breach list. So is a name, a date, or a line from a song.",
                    "OpenSigner asks for at least eight characters and checks nothing beyond that. Eight characters is a minimum, not a recommendation.",
                ],
            },
            Section {
                heading: "How it is protected",
                paragraphs: &[
                    "Your passphrase is not the key. The file carries a random salt, and the key is derived from the passphrase and that salt with Argon2id, which is deliberately slow and memory-hungry: a machine guessing passphrases against your file pays for that memory and three passes over it on every guess.",
                    "The words are then encrypted with XChaCha20-Poly1305, which also authenticates the file's own header. A file with a changed header, or a wrong passphrase, fails to open rather than opening to the wrong words.",
                    "A weak passphrase stays weak. The slow derivation only raises the cost of each guess.",
                ],
            },
            Section {
                heading: "How much memory to pay",
                paragraphs: &[
                    "Settings has a Backup memory row: 64 MiB, 256 MiB or 1 GiB. More memory costs an attacker more per guess and costs you a longer wait each time you open the file. It also decides where the file can be opened: a device that cannot spare that much memory cannot open it at all, and says how much it needs.",
                    "Every file states its own cost in its header, so any device with the memory opens any file whatever its own setting is. OpenSigner recommends 256 MiB on a device with at least a gigabyte of memory and 64 MiB below that, and the recommended row says so.",
                ],
            },
            Section {
                heading: "A key with a passphrase",
                paragraphs: &[
                    "If the key you are backing up has a wallet passphrase, the backup holds the words the passphrase is applied to, not the passphrase itself. Restoring gives you the words back; you type the wallet passphrase again as you always do. The result screen says so while it is on screen.",
                ],
            },
            Section {
                heading: "Reading one back",
                paragraphs: &[
                    "Scan the QR, or read the file in, and OpenSigner asks the passphrase. \"Read an encrypted backup\" is a row in three places: Load a key, Tools › Notes, and a wallet's Recovery sheet. Each opens the same scanner, with Read a file under it. Words and a master seed land in the flow that adds a key, so the key is yours again. A note opens as a document. A recovery sheet opens as a document too, and offers to register the wallet its descriptor names.",
                ],
            },
            Section {
                heading: "Reading one without OpenSigner",
                paragraphs: &[
                    "The screen after a backup is made states its format (\"OSKB 3\" or \"KDBX 4\"), its cipher, the key derivation with the passes and lanes it was made with, and what reads it. The OSKB format is written down in docs/BACKUP.md in OpenSigner's source repository, and tools/backup/decrypt.py in the same repository opens an OSKB file on any computer with Python. A KDBX 4 file needs neither: any KeePass app opens it.",
                ],
            },
        ],
    },
    seed_xor: Page {
        title: "Seed XOR",
        sections: &[
            Section {
                heading: "What a part is",
                paragraphs: &[
                    "Seed XOR splits one key into two, three or four parts. Each part is a full set of seed words of the same length as the original: 24 words split into 24-word parts, 12 into 12-word parts. Combining the parts with the XOR operation gives the original key back.",
                    "XOR is addition without carrying, done bit by bit. Because it is its own inverse, the order of the parts does not matter and no part is more important than another. You make all but the last part yourself, from the same sources Create a key offers — dice, coin flips, hex digits, playing cards, camera noise, this device's generator, or a mix of them — and the device works out the last one, so that the parts XOR back to your key. You choose the source because if a random part could be guessed, the last part alone would give the key.",
                    "The scheme is Coldcard's. Parts made here combine on a Coldcard, and parts made on a Coldcard combine here.",
                    "On this device, a key is split from its Backup menu, under \"Split with Seed XOR\". Parts are combined by Load a key, under \"Seed XOR parts\": what combining gives back is a key that already existed.",
                ],
            },
            Section {
                heading: "Every part is needed",
                paragraphs: &[
                    "All of the parts are needed to recover the key. Lose one and the key is gone, exactly as if you had lost the words themselves. Splitting into three parts and keeping them in three places makes theft harder and loss easier.",
                    "Any part on its own tells you nothing about the key. Two parts of a three-part split tell you nothing either. It is all of them or none.",
                    "To check a part later, load it as a key and compare its fingerprint with the one the split screen showed for it. That identifies the part without putting the key back together.",
                ],
            },
            Section {
                heading: "A part is a real key",
                paragraphs: &[
                    "Each part is a valid set of seed words, so each part is a working wallet with addresses of its own. Coldcard warns about this and it is worth repeating: if you send coins to an address derived from a part, those coins are held by that part alone, not by your key.",
                    "Treat every part with the same care as the original words. A part in a photograph, in cloud storage, or in a drawer with the other parts puts the key at risk.",
                ],
            },
            Section {
                heading: "This is not Shamir",
                paragraphs: &[
                    "Shamir's Secret Sharing, which SLIP-39 uses, makes shares with a threshold: any 2 of 3, any 3 of 5. Seed XOR has no threshold. Every part is required.",
                    "Seed XOR is far simpler in exchange. A part is ordinary BIP-39 words that any wallet can read, the arithmetic is one XOR that you can do with pencil and paper from the wordlist numbers, and no special software is needed to put the key back together.",
                ],
            },
        ],
    },
    passphrases: Page {
        title: "Passphrases",
        sections: &[
            Section {
                heading: "What a passphrase does",
                paragraphs: &[
                    "A passphrase is extra text added to your seed words when the key is derived. The same seed words with a different passphrase produce a completely different key, with different addresses and different funds. The words alone are one wallet, the words plus \"correct horse\" are another, and the words plus \"Correct Horse\" are a third.",
                    "Some people call this the \"25th word\". It is not a word from the list. It can be any text made of letters, numbers, and symbols. Spaces and capital letters count.",
                ],
            },
            Section {
                heading: "There is no wrong passphrase",
                paragraphs: &[
                    "Nothing checks a passphrase. Every passphrase you type opens some wallet. A mistyped passphrase opens an empty wallet that looks perfectly normal, with valid addresses and a zero balance.",
                    "The only sign that you typed it wrong is that the addresses are not the ones you expect. Always compare the first receiving address with one you have recorded before trusting that you are in the right wallet.",
                ],
            },
            Section {
                heading: "Forgetting it is loss",
                paragraphs: &[
                    "The passphrase is stored nowhere. It cannot be reset, recovered, or guessed. If you forget it, the seed words alone lead to a different, empty wallet.",
                    "IF YOU LOSE YOUR PASSPHRASE, THE FUNDS UNDER IT ARE GONE FOREVER, BACKUP OR NO BACKUP.",
                ],
            },
            Section {
                heading: "Rolling one with dice",
                paragraphs: &[
                    "A passphrase you make up yourself is weaker than it looks. Lines from songs, names with digits after them, and letter substitutions are all in the lists a cracking program tries first. Words chosen by dice are not in any such list.",
                    "Tools › Dice passphrase rolls one for you. You pick a list, you pick how many words, and you throw a die. Five throws name one word of the long list, four throws name one word of a short list. The device does not choose anything; it only looks the words up.",
                    "The lists are the ones the Electronic Frontier Foundation publishes. The long list has 7776 words, so each word is worth 12.9 bits. The short lists have 1296 words each, so each word is worth 10.3 bits; their words are shorter to type, and the second short list has no two words within three edits of each other, so a misread word is still recognisable.",
                    "Six words from the long list are 77 bits. That is the length to use unless you have a reason to differ: it is more than offline guessing reaches, and it is six words to write down rather than ten.",
                    "A rolled passphrase is text like any other. Use it as the BIP-39 passphrase on a key, or as the password for anything else. Keep it apart from the seed words, as the rest of this page says. OpenSigner stores nothing: the words are on the screen while you are reading them and gone when you leave.",
                ],
            },
            Section {
                heading: "Storing it",
                paragraphs: &[
                    "Write it down exactly, spaces and capitals included, and read it back before you use it for real. Keep it apart from the seed words, so that whoever finds one does not have both. That separation is the point of a passphrase: a stolen backup of the words alone is useless.",
                    "When you keep a key on an Android device, OpenSigner stores the encrypted seed words only. The passphrase is never stored anywhere, and you type it again each session: open the key's wallet on Home, then Key, and choose \"Open passphrase\". The key that appears carries the same words behind the passphrase you typed, and it is gone when you close the app.",
                    "Make sure that whoever is meant to inherit the funds can find both halves. See the Inheritance page.",
                ],
            },
        ],
    },
    verifying: Page {
        title: "Verifying",
        sections: &[
            Section {
                heading: "Why you must check addresses",
                paragraphs: &[
                    "Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this.",
                    "An address shown on OpenSigner was derived on OpenSigner, from your key, on a device with no network. It is the one thing in the process you can trust.",
                ],
            },
            Section {
                heading: "What a bad coordinator can do",
                paragraphs: &[
                    "The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.",
                ],
            },
            Section {
                heading: "Receiving",
                paragraphs: &[
                    "Before you give out a receiving address, show the same address on OpenSigner and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.",
                    "You can also check an address the other way round: open the wallet on Home, tap \"Check an address\", then scan the address, paste it or type it. A pasted address is checked against the wallet exactly as a scanned one is, and the answer means the same thing. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.",
                ],
            },
            Section {
                heading: "Sending",
                paragraphs: &[
                    "Before you sign, read every output. Confirm that the destination address and amount match what you intended, and that the change output is marked as derived from your key. An output that OpenSigner cannot confirm as yours is not yours.",
                ],
            },
            Section {
                heading: "Warning cards",
                paragraphs: &[
                    "When OpenSigner cannot confirm something, it shows a warning card. A red card is a danger: change that cannot be derived from your key, a fee out of all proportion to the amount, an address on the wrong network. A red card stops the flow until you acknowledge it.",
                    "DO NOT TAP PAST A WARNING YOU CANNOT EXPLAIN. Go back to your coordinator and find out why.",
                ],
            },
        ],
    },
    air_gap: Page {
        title: "The air gap",
        sections: &[
            Section {
                heading: "What an air gap is",
                paragraphs: &[
                    "An air-gapped device has no connection to any network. OpenSigner has no networking code at all, so nothing can reach it over a network and nothing can leave it that way. The only ways in and out are QR codes, shown on a screen and read by a camera, and files that you carry across on an SD card or a USB stick yourself.",
                ],
            },
            Section {
                heading: "What crosses the gap",
                paragraphs: &[
                    "Into OpenSigner: an unsigned transaction (a PSBT), an address to check, a signed message to check, a multisig wallet description, and your seed words when you load a key.",
                    "Out of OpenSigner: a public key for your coordinator, a signed transaction, a signed message. Your seed words and your private keys never cross the gap in either direction, except when you deliberately load them.",
                ],
            },
            Section {
                heading: "What it protects against",
                paragraphs: &[
                    "Anything that needs a network to reach your key: remote attacks, malware on your computer reading the key out of memory, a malicious update that arrives on its own. If your coordinator is fully compromised, the attacker still does not have your key.",
                ],
            },
            Section {
                heading: "What it does not protect against",
                paragraphs: &[
                    "An air gap does not check the transaction for you. A compromised coordinator can still hand you a PSBT that pays the wrong address, which is why you verify every address and amount on OpenSigner.",
                    "An air gap does nothing for your written backup, nothing against somebody who takes the device out of your hand while it is unlocked, and nothing against a camera pointed at your screen while your seed words are showing.",
                ],
            },
        ],
    },
    scams: Page {
        title: "Mistakes and scams",
        sections: &[
            Section {
                heading: "Fake support",
                paragraphs: &[
                    "Nobody needs your seed words to help you. Not a wallet company, not an exchange, not the police, not a developer. Anyone who asks for them, in a chat window, an email, a phone call, a video call, or a helpful reply to a question you posted online, is stealing your funds. There are no exceptions.",
                ],
            },
            Section {
                heading: "\"Validate your wallet\"",
                paragraphs: &[
                    "A message tells you your wallet must be validated, synced, migrated, or upgraded, and links to a page with 12 or 24 boxes on it. The boxes send your words to whoever made the page. Real software never asks you to enter your seed words on a website.",
                ],
            },
            Section {
                heading: "Clipboard swaps",
                paragraphs: &[
                    "Malware on a computer can watch for an address being copied and replace it with the attacker's address. The address you paste is not the address you copied. Always compare the address on OpenSigner with the address that the person you are paying gave you, in full.",
                ],
            },
            Section {
                heading: "Address poisoning",
                paragraphs: &[
                    "A tiny payment arrives in your wallet from an address that starts and ends with the same characters as one you have used before. It sits in your transaction history so that you copy it by mistake later. Take an address from the person you are paying, never from your own history.",
                ],
            },
            Section {
                heading: "Fake wallet software",
                paragraphs: &[
                    "A wallet downloaded from a search advert, an app store clone, or a link in a forum post builds transactions that pay someone else, or simply uploads your seed words. Get your coordinator from the project's own website and check the download's signature before running it. See the Verifying page for how to check OpenSigner itself.",
                ],
            },
            Section {
                heading: "Your own mistakes",
                paragraphs: &[
                    "Most losses are not theft. They are a backup that was never tested, a passphrase that was never written down, a wallet description for a multisig that nobody kept, or funds sent to the wrong network. OpenSigner cannot protect you from these. The Backups, Passphrases, and Inheritance pages can.",
                ],
            },
        ],
    },

    multisig: Page {
        title: "Multisig",
        sections: &[
            Section {
                heading: "What m-of-n means",
                paragraphs: &[
                    "A multisig wallet is controlled by several keys, and a transaction is valid only when a set number of them have signed. In a 2-of-3 wallet there are three keys, any two of them can move the funds, and the third is not needed. Each key has its own seed words, ideally generated on a different device and stored in a different place.",
                ],
            },
            Section {
                heading: "What it protects against",
                paragraphs: &[
                    "One stolen key cannot spend. One destroyed backup cannot lose the funds. A single-key wallet fails on either of those, and a multisig survives both.",
                ],
            },
            Section {
                heading: "What it costs",
                paragraphs: &[
                    "Every key needs its own backup, so a 2-of-3 wallet means three sets of seed words in three places. On top of that, recovery needs the wallet description: which keys are in the wallet, the script type, and how many signatures are required. The seed words alone will not find the funds. If the description is lost, the funds are lost, even with all three keys in hand.",
                    "More parts means more to keep, more to explain to an heir, and more ways to lock yourself out of your own arrangement.",
                ],
            },
            Section {
                heading: "When multisig is not the answer",
                paragraphs: &[
                    "One backup kept badly does not become safer as three backups kept worse. For most people, a passphrase, a better hiding place, or a second copy of the words solves more problems than multisig does.",
                    "Multisig protects against a stolen key and a destroyed backup. It does not protect against a wrong address, a scam, or a wallet description that nobody wrote down.",
                ],
            },
            Section {
                heading: "Using multisig with OpenSigner",
                paragraphs: &[
                    "Your coordinator exports a wallet description, called a policy. Load it into OpenSigner by QR code or file. OpenSigner shows you every key in the wallet, marks which ones are yours, and asks you to accept it.",
                    "Once accepted, OpenSigner recognises the addresses and change of that wallet, so a transaction you sign is checked against the wallet you agreed to. The policy is kept for the current session only. It is not stored on the device, and you load it again next time.",
                ],
            },
            Section {
                heading: "FROST and MuSig2",
                paragraphs: &[
                    "Both put the same arrangement under one key, so the chain sees one signature and nothing about how many keys exist; Kinds of wallets compares them with multisig, and FROST has its own page.",
                ],
            },
        ],
    },
    spend_paths: Page {
        title: "Spend paths and timelocks",
        sections: &[
            Section {
                heading: "What a spend path is",
                paragraphs: &[
                    "Some wallets are not \"2 of 3\" or \"1 of 1\". Their script says several different ways the coins can be spent, and a transaction uses one of them. Each way is a spend path: a set of keys that must sign, and sometimes a wait that must have passed first.",
                    "A common shape is one key that can spend at any time and a second key that can spend alone after a year. That is two spend paths. The review of such a wallet lists them, one row each, with the keys named A, B, C in the same order as the key rows below.",
                ],
            },
            Section {
                heading: "older and after",
                paragraphs: &[
                    "A timelock written as older counts from the coins arriving. \"Key B after 52 560 blocks\" means that particular coin must have been sitting in the wallet for that many blocks before key B can move it. The count is per coin, and it starts again whenever the coins move.",
                    "A timelock written as after is a point on the chain, not a wait: a block height the chain must reach, or a date its clock must pass. It is the same moment for every coin in the wallet.",
                    "Blocks are not minutes. Ten minutes a block is the average the network aims for, not a promise, so the durations the review shows beside a block count are approximate and are said as such.",
                ],
            },
            Section {
                heading: "When the clock starts",
                paragraphs: &[
                    "A recovery path with an older timelock starts counting when the coins were received, not when the wallet was made and not when you last used the device. If you spend from the wallet and the change comes back, the change is a new coin and its clock starts again. A recovery plan that assumes otherwise fails at the moment it is needed.",
                ],
            },
            Section {
                heading: "Writing one here",
                paragraphs: &[
                    "Tools › Miniscript compiles a policy into a descriptor. Where a policy names a key, the fingerprint of a key this device already holds stands for that key's account key, so pk(73c5da0a) is accepted and needs no xpub typed. The Keys row above the field lists the fingerprints it will take; with no key loaded there is no such row and every key must be written out.",
                ],
            },
            Section {
                heading: "Who chooses the path",
                paragraphs: &[
                    "The coordinator builds the transaction, and the transaction is what picks the path: the sequence and locktime fields it sets, and the signatures it collects. This device does not choose. It reads what arrived, shows you the wallet and its paths, and signs what you approve.",
                    "So a spend path you can see in the review is not a spend path that will work today. Whether the wait has passed is a fact about the chain, and this device is not connected to it.",
                ],
            },
        ],
    },
    xpubs: Page {
        title: "Xpubs and privacy",
        sections: &[
            Section {
                heading: "What an xpub is",
                paragraphs: &[
                    "An xpub, or extended public key, is the public key for one account of your wallet. Your coordinator needs it to find your funds, show your balance, and build transactions. It is what you give a read-only wallet.",
                    "OpenSigner exports it as a QR code, together with the fingerprint and derivation path the coordinator needs to use it.",
                    "Any account of a key is exported from the key's own page, under Account key: the four single-signature accounts and the two multisig accounts BIP 48 defines, each with its path. A single-sig wallet's Export hands over the one account that wallet is built on.",
                ],
            },
            Section {
                heading: "What it reveals",
                paragraphs: &[
                    "Every address of that account, the ones already used and every one still to come, and therefore every payment in and out of them, with amounts and dates, forever. That is the account's whole financial history. Once the xpub is tied to your name, the history is tied to your name.",
                ],
            },
            Section {
                heading: "What it cannot do",
                paragraphs: &[
                    "An xpub cannot sign, so nobody can spend with it. It does not reveal the seed words. It does not reach your other accounts, and it does not reach any wallet under a passphrase.",
                ],
            },
            Section {
                heading: "xpub, ypub, zpub",
                paragraphs: &[
                    "You will see the same key written with different prefixes. The prefix is a convention called SLIP-132 that tells the coordinator which script type the account uses: xpub for legacy, ypub for nested SegWit, zpub for native SegWit. The key inside is identical either way. OpenSigner can show it with whichever prefix your coordinator expects.",
                ],
            },
            Section {
                heading: "Who should have your xpub",
                paragraphs: &[
                    "Your own coordinator, on a machine you control. The other members of a multisig, who cannot build the wallet without it. Nobody else, unless you are content for them to read the whole account.",
                ],
            },
            Section {
                heading: "Where never to put it",
                paragraphs: &[
                    "Not in a support chat, a forum post, a block explorer, or a website offering to check your balance. What you hand over cannot be taken back.",
                ],
            },
            Section {
                heading: "Vanity addresses",
                paragraphs: &[
                    "A key's page offers Vanity address: the device tries one candidate after another until the first address of the account begins with the characters you asked for. Each free character costs 32 tries on a bech32 address and 58 on a base58 one, so four characters past bc1q is about a million tries, and the device states the rate it is managing and how long that is.",
                    "There are two dials. The passphrase counter appends characters to the key's BIP-39 passphrase, which means every candidate is a different key and costs a full derivation; taking the find opens that key beside the one you started from. The account index steps through the accounts of the key you already have, which is far cheaper and adds a wallet rather than a key.",
                    "A vanity address is not safer. It is the same kind of address, with the same key behind it, and a string that looks familiar is a string an attacker can imitate: never check an address by its first characters alone. What the passphrase dial leaves you with is a passphrase to back up, since the words alone no longer reach the funds, and a counter written down nowhere is a key lost. The account dial leaves you with a wallet at an account no other software will look for unless you tell it, which its descriptor does.",
                    "The counter is the one other tools use, in the same order, so a find here is a find there.",
                ],
            },
        ],
    },
    secure_element: Page {
        title: "The secure element",
        sections: &[
            Section {
                heading: "When this applies",
                paragraphs: &[
                    "Only when you choose to keep a key on an Android device between sessions. Otherwise your seed words are in memory while OpenSigner runs and gone when it stops, and nothing on this page matters.",
                    "Keeping a key on a phone is a convenience with a cost. Read this page before you do it.",
                ],
            },
            Section {
                heading: "What the chip does",
                paragraphs: &[
                    "A secure element is a separate chip inside the phone that holds its own keys and never hands them out. When you keep your keys on the phone, OpenSigner encrypts each key's words under a key that lives in that chip. Every key you add afterwards is kept the same way, under the one PIN, and a key you forget is removed from the phone. Opening them again requires the chip, your phone's screen lock, and the OpenSigner PIN together. A copy of the phone's storage is useless on its own. While a key is kept, OpenSigner opens on its PIN pad and shows nothing else until the PIN is entered.",
                    "The wallets you are using are kept beside the keys, and so are the notes and recovery sheets you ask it to keep: each one carries a \"Keep on this device\" switch, and the phone holds eight of them. They are encrypted under the same key as the words and they go the same way — a wipe, the duress PIN or eight wrong PINs takes them with the keys.",
                    "After eight wrong PINs in a row, on any PIN pad, OpenSigner has the chip delete its keys. The stored words are unrecoverable from that point, which ends any guessing attack for good. Your written backup is still your backup.",
                ],
            },
            Section {
                heading: "What it cannot do",
                paragraphs: &[
                    "The chip cannot check what you are signing. The phone's software still draws the screen, and a compromised phone can still show you a false address. It does nothing for your written seed words, and nothing against someone who takes the phone out of your hand while OpenSigner is unlocked.",
                    "It never holds a passphrase. The passphrase stays in your head or on paper, and you type it every time.",
                ],
            },
            Section {
                heading: "The duress PIN",
                paragraphs: &[
                    "You can set a second PIN. Entering it wherever OpenSigner asks for a PIN, including the lock screen, deletes the stored key and opens an empty OpenSigner, as if no key had ever been kept. Nothing in the stored data shows that a duress PIN exists. If you are forced to unlock the phone, the duress PIN gives the person forcing you nothing, and your written backup still has the funds.",
                ],
            },
            Section {
                heading: "Losing the phone",
                paragraphs: &[
                    "The stored key is bound to that one chip. A new phone cannot open the old data, and neither can you. Recovery is your written seed words, exactly as it is everywhere else. A key kept on a phone is never a backup.",
                ],
            },
        ],
    },
    inheritance: Page {
        title: "Inheritance",
        sections: &[
            Section {
                heading: "The problem",
                paragraphs: &[
                    "If you die or become incapacitated, your funds are gone unless someone else can recover them. There is no company to contact and no court that can order the funds released. Whoever has the seed words, the passphrase, and the wallet details has the funds. Whoever does not, does not.",
                ],
            },
            Section {
                heading: "What an heir needs",
                paragraphs: &[
                    "The seed words. The passphrase, if there is one. Enough about the wallet to open it: the script type and the derivation path, or for a multisig, the whole wallet description and enough of the keys.",
                    "They also need instructions written for somebody who has never done this: which software works, and what the first step is. Assume they know nothing.",
                ],
            },
            Section {
                heading: "What to leave and where",
                paragraphs: &[
                    "Plain-language instructions, kept where the heir will actually find them: with a lawyer, in a deposit box they can open, or with a person they already know to ask. Tell them that the instructions exist.",
                    "Keep the seed words apart from the passphrase, so that one discovery is not the whole wallet, but make sure the instructions say where both halves are.",
                ],
            },
            Section {
                heading: "What not to do",
                paragraphs: &[
                    "Do not put the seed words in your will. A will passes through many hands and can end up on public record.",
                    "Do not leave a clue that only you can follow. Do not leave one copy in one place. Do not assume that someone technical in the family will work it out.",
                ],
            },
            Section {
                heading: "Passphrases and heirs",
                paragraphs: &[
                    "A passphrase is the part of a backup you can store separately, which is what makes it useful and what makes it dangerous here. Seed words without the passphrase open a different wallet that is empty and looks correct. An heir who has only the words will conclude that the backup is broken and stop looking.",
                ],
            },
            Section {
                heading: "Test it while you can",
                paragraphs: &[
                    "Have your heir walk through a recovery once, with a small amount, while you are there to answer questions. An untested plan is a guess, and this is one you will not be able to fix later.",
                ],
            },
        ],
    },
    nonces: Page {
        title: "Nonces",
        sections: &[
            Section {
                heading: "What a nonce is",
                paragraphs: &[
                    "Every signature is made with a one-time secret number called the nonce, combined with your private key and the transaction. The nonce must be different for every signature and must never be predictable.",
                ],
            },
            Section {
                heading: "Why it matters",
                paragraphs: &[
                    "Anyone who learns the nonce used for a signature can calculate your private key from that signature. Using the same nonce twice gives the key away outright, and a nonce with a pattern in it leaks the key over time. This has happened to real wallets.",
                    "A signer that picks its nonce freely could also hide your key inside it on purpose, and a signature made that way looks exactly like an honest one. This is the one way a malicious signing device can steal funds without ever touching a network.",
                ],
            },
            Section {
                heading: "What OpenSigner checks on a signature that arrived",
                paragraphs: &[
                    "A transaction can reach the device with signatures another device or another person already made. OpenSigner checks each of them against the key it is under and refuses the transaction when one is not a signature of what is in front of you, because that means the transaction was changed after it was signed. It also looks for two signatures under one key that share a nonce, anywhere in the transaction, and refuses that too: those two signatures are the key. Where a signature is under a key this device holds, it signs the same input again under each nonce rule and says which one produced it, so you can see whether another device signed the way it says it does. The Signatures row on the review opens the list.",
                ],
            },
            Section {
                heading: "How OpenSigner picks a nonce",
                paragraphs: &[
                    "As it comes, OpenSigner chooses no nonce with any randomness of its own. Every nonce is calculated from the key and the transaction alone, using RFC 6979 for ECDSA signatures and BIP-340 with no auxiliary randomness for Schnorr signatures. The same transaction signed with the same key always produces exactly the same signature bytes.",
                    "As a check on itself, OpenSigner signs every input twice and refuses to continue if the two results differ.",
                ],
            },
            Section {
                heading: "The Nonce and Schnorr settings",
                paragraphs: &[
                    "There are two standard ways to derive an RFC 6979 nonce, and OpenSigner offers both under Settings. \"Low R\" retries the nonce, with a counter added as the RFC's extra data, until the signature comes out in its shortest form. \"First\" stops at the first nonce.",
                    "Low R is what Bitcoin Core, Sparrow, and Electrum produce. First is what the RFC's own test vectors, the BIP-174 test vectors, and Trezor produce. Bitcoin Core signs messages without the retry, so compare a Core message signature against First.",
                    "The Schnorr setting covers taproot. \"Deterministic\" adds no randomness, so the signature can be compared against another implementation. \"Fresh randomness\" mixes 32 new bytes into every taproot signature, which is what BIP-340 recommends as a defence against fault and side-channel attacks, at the cost of a signature nobody can reproduce.",
                ],
            },
            Section {
                heading: "Checking OpenSigner against another signer",
                paragraphs: &[
                    "Because the nonce is deterministic, you can verify that OpenSigner is not hiding anything in it. Load the same seed words into a second implementation on another offline machine, set OpenSigner's Nonce setting to match that software and its Schnorr setting to Deterministic, sign the same transaction on both, and compare the signature bytes on OpenSigner's Signatures screen. They must match exactly. A device that was leaking your key through its nonces would fail this comparison.",
                ],
            },
            Section {
                heading: "What is not offered yet",
                paragraphs: &[
                    "There are protocols in which the coordinator contributes randomness to the nonce and then checks that the signer did not choose the nonce alone. OpenSigner does not implement one yet, because no coordinator currently speaks such a protocol over QR codes or PSBT files.",
                    "MuSig2 is the other case. Several keys aggregate into one, and each signer's nonce is part of one aggregate nonce, so the nonces are exchanged before anyone signs. OpenSigner signs in either order. When every other signer's nonce is already on the transaction, it derives its own nonce from theirs and from the transaction, signs in one pass, and keeps nothing afterwards. When a nonce is missing, it goes first: it draws its nonce, writes the public half onto the transaction for the coordinator, and holds the secret half in memory as a session. That session is the device's memory of one transaction. It ends when the device signs, when you lock it, and when it loses power, and nothing writes it to storage, because a secret nonce that survives a restart is the classic way to use one twice, and using a MuSig2 nonce twice hands over the private key. If the transaction comes back after the session has ended, the device draws a new nonce, says so, and the other signers sign again.",
                    "A FROST wallet is the one place where OpenSigner writes a secret nonce to a file, and the reason it is safe there is that there are not two devices. One device carries the wallet's shares between the places you keep them. At the first place it draws every signer's nonce itself, signs with the share it is holding, and saves one file that holds the transaction and the other signers' secret nonces bound to it. At the next place the same device reads that file back, checks that each stored nonce is the one the transaction already names, that the file is bound to this transaction and to this set of shares, and that the signature already on it verifies, and only then signs. Reusing a nonce leaks a share when the same nonce signs under two different challenges, and the challenge changes only if someone varies a nonce or the message; here both were fixed by the device that drew them, and any change breaks a check it can make by arithmetic. A copy of the file taken in transit is a random number with nothing to pair it with, because the signature made under it never leaves the device. None of this extends to two devices signing together: a file like this is read only by the device that wrote it.",
                ],
            },
        ],
    },
    message: Page {
        title: "Signing a message",
        sections: &[
            Section {
                heading: "What a signed message is",
                paragraphs: &[
                    "A signed message is a piece of text plus a signature made with the key behind one of your addresses. It has three parts: the address, the signature, and the text. The file OpenSigner saves and the QR code it shows carry the address on the first line, the signature on the second line, and the message from the third line to the end.",
                    "Nothing about it touches the blockchain. It is text, and it is checked by whoever you hand it to.",
                ],
            },
            Section {
                heading: "What it proves",
                paragraphs: &[
                    "That whoever made the signature holds the key behind that address, and that this exact text was signed with it. People use this to prove ownership of an address to an exchange, to prove control of funds without moving them, or to sign a statement in a way that cannot be forged.",
                ],
            },
            Section {
                heading: "What it does not prove",
                paragraphs: &[
                    "It moves no funds and says nothing about what the address holds.",
                    "Anyone can copy the three lines and pass them on. Only the key can make a new one. So if someone wants to prove something to you, ask for a signature over text that you chose, including the date, and treat a ready-made signature you were handed as proof of nothing.",
                ],
            },
            Section {
                heading: "Formats",
                paragraphs: &[
                    "OpenSigner produces BIP-137 signatures for legacy and SegWit addresses, and BIP-322 signatures, which work for every address type including Taproot. Choose the format your verifier expects. Most exchanges and older tools expect BIP-137.",
                ],
            },
            Section {
                heading: "Checking a signed message",
                paragraphs: &[
                    "Verify on OpenSigner reads a signed message from a QR code or a file, checks whether the signature matches the address and the text, and shows you the text that was checked. Read the text. A valid signature over the wrong text proves the wrong thing.",
                ],
            },
        ],
    },
    glossary: Page {
        title: "Glossary",
        sections: &[
            Section {
                heading: "Account",
                paragraphs: &[
                    "One branch of a key with its own addresses and its own history. A coordinator usually watches one account at a time.",
                ],
            },
            Section {
                heading: "Address",
                paragraphs: &[
                    "Where a payment goes. It is derived from a key, so a wallet can make as many as it needs. Every address should be used once.",
                ],
            },
            Section {
                heading: "Air gap",
                paragraphs: &[
                    "No network connection of any kind. Data crosses as QR codes or as files that you carry.",
                ],
            },
            Section {
                heading: "BSMS",
                paragraphs: &[
                    "Bitcoin Secure Multisig Setup, BIP-129: a procedure in which every signer confirms the same multisig description on its own screen before the wallet is used.",
                ],
            },
            Section {
                heading: "Carry file",
                paragraphs: &[
                    "The file that carries a part-signed FROST transaction from one signer to the next, holding the transaction and the secret nonce of each signer still to sign.",
                ],
            },
            Section {
                heading: "Change",
                paragraphs: &[
                    "The part of an input that comes back to you, at an address derived from your own key.",
                ],
            },
            Section {
                heading: "Checksum",
                paragraphs: &[
                    "Bits at the end of the seed words that catch a typo. Passing the checksum does not prove the words are yours.",
                ],
            },
            Section {
                heading: "Codex32",
                paragraphs: &[
                    "BIP-93: a master seed written as one checksummed string, optionally split into shares, with arithmetic that can be done by hand.",
                ],
            },
            Section {
                heading: "Coordinator",
                paragraphs: &[
                    "The internet-connected wallet software that watches your balances, builds transactions, and sends them to the network. It has no keys.",
                ],
            },
            Section {
                heading: "Delay",
                paragraphs: &[
                    "A wait written into a wallet's script, after which a spend path that was closed becomes usable.",
                ],
            },
            Section {
                heading: "Derivation path",
                paragraphs: &[
                    "The route from a master key to one of its child keys, level by level, starting at the root, m. An h marks a hardened level. The levels are purpose, coin, account, change, and index.",
                ],
            },
            Section {
                heading: "Descriptor",
                paragraphs: &[
                    "One line of text that describes a whole wallet: which keys, which script type, which paths. Your coordinator can export one.",
                ],
            },
            Section {
                heading: "Diceware",
                paragraphs: &[
                    "A way of picking words at random by throwing dice. Each throw of four or five dice names one word of a published list, and the words together are a passphrase. OpenSigner carries the three lists the Electronic Frontier Foundation publishes.",
                ],
            },
            Section {
                heading: "Entropy",
                paragraphs: &[
                    "Randomness, measured in bits. 128 bits of real entropy is unguessable. Entropy from a memorable phrase is far lower than it looks.",
                ],
            },
            Section {
                heading: "Eye glyph",
                paragraphs: &[
                    "The mark at the start of a row that names a wallet or one of its keys: this device holds the public key only. You can derive and compare its addresses; you cannot spend from it. On a screen with a secret panel the eye is a button instead, which reveals the panel; the two never appear on one screen.",
                ],
            },
            Section {
                heading: "Fee",
                paragraphs: &["The inputs less the outputs. It goes to the miner of the block."],
            },
            Section {
                heading: "Fingerprint",
                paragraphs: &[
                    "Four bytes that identify a key, so a coordinator and a signer can confirm they mean the same one.",
                ],
            },
            Section {
                heading: "FROST",
                paragraphs: &[
                    "A scheme in which one key is split into n shares, any m of which sign, and the chain sees one key and one signature.",
                ],
            },
            Section {
                heading: "Group record",
                paragraphs: &[
                    "The public file that states a FROST wallet's threshold, its group key, the public half of every share, and its descriptor. A share without it opens nothing.",
                ],
            },
            Section {
                heading: "Hot wallet",
                paragraphs: &["Wallet software that holds keys on an internet-connected device."],
            },
            Section {
                heading: "Input",
                paragraphs: &[
                    "A coin being spent. An input is always spent whole, and the remainder returns as change.",
                ],
            },
            Section {
                heading: "Key",
                paragraphs: &[
                    "The secret a wallet is built from, held as seed words. Everything else is derived from it.",
                ],
            },
            Section {
                heading: "Key glyph",
                paragraphs: &[
                    "The mark at the start of a row that names a wallet or one of its keys: this device holds the private key, so it can sign for that wallet. A 2-of-3 with one of your keys in it carries the key glyph.",
                ],
            },
            Section {
                heading: "Locktime",
                paragraphs: &["The earliest block or time at which a transaction may confirm."],
            },
            Section {
                heading: "Multisig",
                paragraphs: &[
                    "A wallet with several keys that spends only when a set number of them sign.",
                ],
            },
            Section {
                heading: "Network",
                paragraphs: &[
                    "Which chain a key and its addresses belong to: mainnet, testnet, signet, or regtest. Funds on one network do not exist on another.",
                ],
            },
            Section {
                heading: "Node",
                paragraphs: &[
                    "Software that keeps a full copy of the blockchain and validates every transaction. A coordinator connects to one.",
                ],
            },
            Section {
                heading: "NUMS point",
                paragraphs: &[
                    "A public key chosen so that nobody knows a private key for it. It is used as a Taproot internal key when a wallet is meant to be spendable only by its scripts.",
                ],
            },
            Section {
                heading: "Output",
                paragraphs: &["A coin being created: an address and an amount."],
            },
            Section {
                heading: "Passphrase",
                paragraphs: &[
                    "Extra text added to the seed words that produces a different key. It is stored nowhere.",
                ],
            },
            Section {
                heading: "PIN",
                paragraphs: &[
                    "The number that opens a session on OpenSigner, or a key kept on an Android device.",
                ],
            },
            Section {
                heading: "Policy",
                paragraphs: &[
                    "The rule a wallet spends by: which keys, how many of them, and any wait or spend path. A descriptor is one way of writing it down.",
                ],
            },
            Section {
                heading: "PSBT",
                paragraphs: &[
                    "Partially Signed Bitcoin Transaction. What a coordinator hands to a signer, and gets back with a signature in it.",
                ],
            },
            Section {
                heading: "Recovery path",
                paragraphs: &[
                    "A spend path that becomes usable only after a delay, so that a wallet can be recovered later by keys that cannot spend from it today.",
                ],
            },
            Section {
                heading: "Sat",
                paragraphs: &["The smallest unit of bitcoin. One bitcoin is 100,000,000 sats."],
            },
            Section {
                heading: "Script type",
                paragraphs: &[
                    "The form of a wallet's addresses: legacy, nested SegWit, native SegWit, or Taproot. It decides the derivation path and the address prefix.",
                ],
            },
            Section {
                heading: "Seed",
                paragraphs: &[
                    "The 64 bytes that the seed words and passphrase are stretched into. Every key of the wallet comes from it.",
                ],
            },
            Section {
                heading: "Seed words",
                paragraphs: &[
                    "The 12 or 24 words that are the key. Whoever reads them controls the funds.",
                ],
            },
            Section {
                heading: "Share",
                paragraphs: &[
                    "One of the n pieces a FROST wallet's key is split into, held as 24 words. Any m of them sign; one alone can do nothing.",
                ],
            },
            Section {
                heading: "Signature",
                paragraphs: &[
                    "Proof that a key agreed to a transaction or a message. On its own it moves nothing.",
                ],
            },
            Section {
                heading: "Signer",
                paragraphs: &[
                    "The offline software or device that holds the keys and signs transactions. OpenSigner is a signer.",
                ],
            },
            Section {
                heading: "SLIP-39",
                paragraphs: &[
                    "Trezor's backup scheme: Shamir's Secret Sharing over a word list of its own, with a threshold. Its shares are not BIP-39 seed words.",
                ],
            },
            Section {
                heading: "Xpub",
                paragraphs: &[
                    "An account's extended public key. It reveals every address and payment of that account and can sign nothing.",
                ],
            },
        ],
    },
    tools: Page {
        title: "Tools",
        sections: &[
            Section {
                heading: "What the tools are",
                paragraphs: &[
                    "Each tool is a calculator. It takes something you scan, paste or type, works one thing out from it, and keeps nothing. No tool touches a key, and none of them stores anything, so nothing on these screens is a secret.",
                    "Every tool but Units opens on the scanner. Under the viewfinder are Read a file, Paste and Type: the string reaches the tool the same way whichever of them you use, and the answer opens as soon as the tool can work one out. Words, a seed code, an extended private key and a private key are refused here, because a secret is never pasted.",
                ],
            },
            Section {
                heading: "Hashes",
                paragraphs: &[
                    "SHA-256, SHA-256 applied twice, and RIPEMD-160 of SHA-256, which Bitcoin calls HASH160. These three are what Bitcoin hashes with: a transaction id is SHA-256d of the transaction, and the 20 bytes inside a legacy address are HASH160 of a public key.",
                    "The field is read as hex when every character is a hex digit and there is an even number of them, and as text otherwise. The mode row forces one or the other, which is how you hash the four characters \"dead\" rather than the two bytes 0xde 0xad.",
                ],
            },
            Section {
                heading: "Encodings",
                paragraphs: &[
                    "Bitcoin writes bytes in a handful of alphabets. Base58Check carries a version byte and a four-byte checksum, and spells legacy addresses and extended keys. Bech32 and bech32m carry a human-readable prefix and their own checksum, and spell segwit and Taproot addresses.",
                    "Give it a string and the tool says which one it is, what bytes it holds, and whether the checksum holds. Give it hex and it offers the same bytes in the other spellings.",
                ],
            },
            Section {
                heading: "Descriptor checksum",
                paragraphs: &[
                    "A descriptor is the text a wallet is described by. The eight characters after the # are a checksum over it, and a coordinator that reads a descriptor with a broken checksum refuses it rather than deriving the wrong addresses.",
                    "The tool computes the checksum of what you type, says whether the one that came with it holds, and says whether the descriptor parses as a wallet this device could use.",
                ],
            },
            Section {
                heading: "Convert key",
                paragraphs: &[
                    "One extended public key has several spellings. BIP-32 writes it as xpub or tpub; SLIP-132 writes the same key as ypub, zpub, upub or vpub to say which script type it is meant for. The key material is identical; only the four version bytes differ.",
                    "Give it any of them and the tool shows all of them, with the network, the depth, the fingerprint and the child number the key states about itself. A key already loaded is a row of its own on the scanner, Use a loaded key, which fills the field with that key's account key. Extended private keys are refused: the key explorer is where those are seen.",
                ],
            },
            Section {
                heading: "Units",
                paragraphs: &[
                    "One bitcoin is 100 000 000 satoshi. A millibitcoin is a thousandth of a bitcoin, and a bit is a millionth. Type an amount in any of the four and the other three follow as you type.",
                ],
            },
            Section {
                heading: "Decode a transaction",
                paragraphs: &[
                    "The same review the Sign flow shows, over a transaction you are reading rather than signing. Whatever keys and wallets you have loaded are used to read it, so change of a wallet in use is recognised here as it is when you sign; with nothing loaded the change row says so. Nothing is signed either way, there is no confirm, and the last page is Done.",
                    "Use it to read what a coordinator produced before you sign it anywhere, or to see what a raw transaction from a block explorer actually pays.",
                ],
            },
            Section {
                heading: "Lightning node key",
                paragraphs: &[
                    "A Lightning node has an identity key, and a backup of that node is either an LND cipher seed — twenty-four words that are not a BIP-39 phrase — or the BIP-39 words an ldk-node wallet was built from. The tool reads either one and says which node it is: the public key the network knows the node by, and for a cipher seed the version and the birthday it carries.",
                    "Use it to confirm that a backup in your hand belongs to the node you think it does, before you restore it anywhere. Nothing is loaded and nothing is signed. A node key is hot by nature — it is on a machine that is online, in use, all the time — so it is not a key this device holds or signs with, and what the tool derives is gone when you leave the screen.",
                ],
            },
        ],
    },
    wallet_kinds: Page {
        title: "Kinds of wallets",
        sections: &[
            Section {
                heading: "What a wallet is here",
                paragraphs: &[
                    "A key is words. A wallet is a rule about which keys may spend, together with the addresses that rule produces. One key can be in several wallets, and the same key in two wallets with different script types gives two sets of addresses and two balances.",
                    "In OpenSigner these are two screens: Keys lists the secrets, and Wallets lists the rules, whether you built them here or a coordinator sent them. The kinds below are the rules the device can read and build. Which one suits you depends on what you are protecting against, so all of them are offered.",
                ],
            },
            Section {
                heading: "Single-sig",
                paragraphs: &[
                    "One key spends. The wallet still has a choice in it: the script type, which decides the address prefix and the derivation path. Legacy, nested SegWit, native SegWit and Taproot are four different wallets over the same words.",
                    "It is the cheapest arrangement to hold, to explain and to recover: seed words, a script type, and a derivation path. It is also the one where a single stolen backup is a total loss, and a single destroyed backup with no second copy is a total loss the other way.",
                ],
            },
            Section {
                heading: "Multisig",
                paragraphs: &[
                    "The wallet names n keys and requires m of them to sign. A 2-of-3 survives one stolen key and one lost backup. The script lists all n public keys, and spending reveals the script and puts m signatures on the chain, so anyone reading the block sees that it was a 2-of-3 and sees the three keys.",
                    "Every coordinator supports this arrangement, and it has the most software behind it. It needs the wallet description as well as the keys: without it, the keys alone do not find the funds. The Multisig page covers this kind in full.",
                ],
            },
            Section {
                heading: "Taproot multisig",
                paragraphs: &[
                    "The same m-of-n written as a Taproot script path. Unspent, the address looks like any other Taproot address and says nothing. Spending reveals the one script that was used and nothing else, and the transaction is smaller than the equivalent classic multisig.",
                    "When the wallet has no way to spend with a single key, the Taproot internal key is set to a NUMS point: a public key chosen so that nobody knows a private key for it. The device shows this in the review, because an internal key that has not been shown to be unspendable is a way to spend the wallet that you never agreed to.",
                    "Add a wallet builds this kind: pick Taproot multisig, choose the keys, choose how many must sign, and the review states the quorum, the keys and that the key path cannot spend. Fewer coordinators write this than write classic multisig, and recovery software for it is younger.",
                ],
            },
            Section {
                heading: "MuSig2",
                paragraphs: &[
                    "All n keys together produce one public key and one signature. The chain sees a single-key Taproot spend: no threshold, no key count, nothing about the arrangement. It is the standard in BIP-327, and a wallet is written with the musig() key expression of BIP-390.",
                    "Signing is interactive and takes two rounds. Every signer publishes a nonce, then every signer produces a partial signature over the collected nonces, and the partials add up to one signature. Every participant must take part; there is no m of n. A nonce must never be used twice, which is why the device holds one only in memory and only for the one transaction it was drawn for.",
                    "Use it when all the signers are yours or reachable, and when what the chain reveals matters. Do not use it where one signer may be unavailable.",
                ],
            },
            Section {
                heading: "FROST",
                paragraphs: &[
                    "Any m of n shares of one key sign, and the chain sees one key and one signature, as with MuSig2. The shares are made by a dealer at creation and each one is 24 words. FROST has its own page.",
                ],
            },
            Section {
                heading: "Recovery and inheritance",
                paragraphs: &[
                    "A wallet can name one key that spends at any time and another that spends alone after a wait. That is two spend paths in one script, and it is how Liana's wallets are shaped: a primary key, and one or more recovery paths that become usable when a coin has sat untouched for a set number of blocks.",
                    "It gives you a recovery that needs no second person on the day, and an inheritance an heir can execute by waiting. It costs a timelock whose clock restarts every time the coins move, so the wallet has to be refreshed or the recovery path opens while you are still using it. Spend paths and timelocks has the details.",
                    "Add a wallet builds one: the keys that can sign now, then up to three recovery paths, each with its own keys and its own wait. Each wait has to be longer than the one before it, and you either pick one of the four offered or type a number of days, up to 455. Last comes whether the wallet pays to SegWit or to Taproot. The review names every path and every wait, in days and in blocks. A wallet Liana or another coordinator built arrives as a descriptor and is read the same way, including one with more than one recovery path.",
                ],
            },
            Section {
                heading: "Silent payments",
                paragraphs: &[
                    "A silent payments wallet publishes one address that never changes and never reuses a payment. The payer takes the address, does the arithmetic with the keys of the inputs they are spending, and pays a fresh taproot output that only you can find. Nothing on the chain links two payments to the same address, and nothing has to pass between you and the payer beforehand.",
                    "It is built over one key: the device derives a scan key and a spend key at BIP-352's paths, and the address carries both public halves. Labels give the one address several forms, so a payer can be told apart from another without publishing a second address. Silent payments has the details.",
                    "What it costs is that a wallet has to look for the payments. There is no chain of addresses to hand a coordinator, so something has to scan the blocks with the scan key. This device does the arithmetic for one transaction at a time, under Check a payment, and exports the scan descriptor for a wallet that scans continuously. Sending to a silent payment address is not built here.",
                ],
            },
            Section {
                heading: "Wallets with none of your keys",
                paragraphs: &[
                    "A wallet made from public keys alone derives addresses, checks change and exports itself, and cannot sign. Its rows carry the eye rather than the key. Loading a friend's descriptor to check an address they gave you is a legitimate use of the device, and so is keeping a watch-only copy of a wallet whose keys are elsewhere.",
                ],
            },
            Section {
                heading: "Choosing",
                paragraphs: &[
                    "Four questions decide it. How many people are involved, and must more than one of them agree? How many devices and backups will actually exist, in how many places? Does it matter what the chain shows about the arrangement? And must a coordinator, a recovery tool, or an heir's software understand the wallet years from now?",
                    "More parts mean the wallet survives one failure, and also that the plan has more ways to go wrong. A 2-of-3 kept in three places by a person who has never tested a recovery is worse than a single key on steel in two places. Pick the arrangement you will actually maintain, write it down, and test the recovery before it is needed.",
                ],
            },
        ],
    },
    frost: Page {
        title: "FROST",
        sections: &[
            Section {
                heading: "What it is",
                paragraphs: &[
                    "FROST splits one key into n shares. Any m of them can sign; fewer cannot, and no share on its own can do anything. The chain sees one public key and one signature, exactly as it would for a wallet with a single key, and learns nothing about how many shares exist or how many signed.",
                    "That is the difference from multisig. A 2-of-3 multisig announces itself in every spend. A 2-of-3 FROST wallet is indistinguishable from one person with one key.",
                ],
            },
            Section {
                heading: "The shares are keys",
                paragraphs: &[
                    "Each share is 24 words, written down and backed up the way any key is. On this device a share is a key in Keys, with a fingerprint like any other, and the wallet finds its shares by computing each loaded key's public half and matching it against the record. You never type a share number.",
                    "A share derives no addresses of its own within the wallet. It is one piece of the group's single key.",
                ],
            },
            Section {
                heading: "The group record",
                paragraphs: &[
                    "The record is a short public file that states the threshold, the group's public key, the public half of every share, and the wallet's descriptor. Without it, the device cannot use a share on its own: it cannot tell which group the words belong to, what the threshold is, or which addresses to watch.",
                    "So the record travels with every share. Keep a copy wherever you keep a share, and a copy with whoever inherits. It reveals no secret, so make as many copies as you like; if every copy is lost, the wallet cannot be rebuilt even with every share in hand.",
                ],
            },
            Section {
                heading: "Creating a wallet",
                paragraphs: &[
                    "A dealer makes the shares. On this device that means choosing how many shares and how many must sign, picking the loaded keys that will be the chosen shares, and letting the device compute the rest; each computed share is shown as words with a quiz, and each becomes a loaded key you can write down and then forget.",
                    "The shares exist together on one device for the length of that flow. That is what a dealer is, and it is the trade-off FROST makes against a key generation where the shares are never in one place.",
                ],
            },
            Section {
                heading: "Signing in two places",
                paragraphs: &[
                    "Signing takes two rounds, as MuSig2 does: every signer publishes a nonce, then every signer produces a partial signature over the collected nonces. When the signers are not in the same room, the rounds are carried in a file.",
                    "At the first device you choose which other signers will take part, the device draws every nonce, signs, and saves a carry file. At the second device the carry file is read, the stored nonce is matched to its record, the transaction's signature hash and the set of signers are checked against what the first device committed to, every earlier partial signature is verified, and only then does it sign, add the partials together, and finish the transaction. Any mismatch is refused by name.",
                    "The carry file holds the transaction and one secret nonce per signer still to sign, which is why it is a file you keep to yourself rather than one you publish.",
                ],
            },
            Section {
                heading: "A lost share",
                paragraphs: &[
                    "Lose one share of a 2-of-3 and the funds are still spendable by the other two. Any m shares together also rebuild the whole set, so a lost share can be replaced rather than lived with.",
                    "Lose more than n − m shares and the wallet is gone, with every copy of the group record intact and useless.",
                ],
            },
            Section {
                heading: "The decoy property",
                paragraphs: &[
                    "A share's 24 words are valid seed words. Loaded as an ordinary key, with no group record in sight, they open a working single-sig wallet that you may keep funded. Someone who finds the words and does not have the record finds that wallet, and nothing about the words says a group exists.",
                    "The device treats any 24 words you load as an ordinary key. They act as a member of a FROST wallet only after you have also loaded the group record that lists their public share; nothing in the words themselves shows which wallet they belong to.",
                ],
            },
            Section {
                heading: "What it is not",
                paragraphs: &[
                    "FROST is not a backup scheme. Shares do not reconstruct your other wallets, and a FROST wallet's funds are reachable only through its own script.",
                    "It is also newer than multisig and understood by less software. A coordinator that cannot read the group record sees a single-key Taproot wallet, which is enough to watch it and build transactions, and not enough to know who has to sign.",
                ],
            },
        ],
    },
    other_backups: Page {
        title: "Backups in other forms",
        sections: &[
            Section {
                heading: "Why there is more than one",
                paragraphs: &[
                    "Words on paper or steel is the backup almost everybody should make first. The forms on this page all answer the same two problems that a single written copy has: one copy can be destroyed, and one copy can be found and read by whoever finds it. Each of them trades something away to fix one of those.",
                    "Read this page before you split anything. If you do not fully understand a scheme, you are safer with a single written copy.",
                ],
            },
            Section {
                heading: "SLIP-39",
                paragraphs: &[
                    "SLIP-39 is Shamir's Secret Sharing over a word list, published by Trezor and used on Trezor devices. It splits a master secret into shares with a threshold, so any 3 of 5 shares rebuild it and two do not, and it can nest that once: groups of shares, with a threshold over the groups as well.",
                    "Its words are not BIP-39's. The list is a different one, a share is a different length from a seed phrase, and a share cannot be typed into a wallet that expects seed words. The master secret it protects is the entropy itself, stretched into a seed by SLIP-39's own rules, so a SLIP-39 backup and a BIP-39 backup of the same wallet are not interchangeable: you choose one at the moment the key is made.",
                    "Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. This device reads and writes it, groups and all: Add a key has a row that makes a key whose only written form is shares, and a key loaded from shares can be written into a fresh set of them at any time.",
                ],
            },
            Section {
                heading: "Codex32",
                paragraphs: &[
                    "Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1, with optional shares and a threshold, as SLIP-39 has. What is unusual about it is that the checksum and the share arithmetic were designed to be worked by hand, with the printed tables that come with the standard, on paper and without a computer.",
                    "That is the point of it. You can create a seed, verify the checksum, and combine shares without trusting any device with the secret, including this one. The cost is time and care: doing it by hand is slow, and a mistake in the arithmetic is silent until the checksum catches it.",
                    "Pick it when your objection is to trusting a computer at all.",
                    "This device reads and writes it. Type a string, or the shares of a split of one, and the key it holds is loaded. Add a key has a row that makes a key and writes it as codex32: you choose how long a seed to make, whether to split it, and into how many shares of which how many must be present, and the device shows every string in turn and asks you to type it back. Every key already loaded can be written as codex32 from its Backup menu, whatever it was made from, because every key is a seed. What a codex32 backup of a key made of words holds is that seed, with the passphrase already in it, and not the words.",
                ],
            },
            Section {
                heading: "Seed XOR",
                paragraphs: &[
                    "Seed XOR splits a key into parts that are themselves ordinary seed phrases, and every part is needed. It has no threshold: two parts of a three-part split are worth nothing. In exchange it is simple enough to do with pencil and paper from the wordlist numbers, and any wallet can read a part. Seed XOR has its own page.",
                ],
            },
            Section {
                heading: "Encrypted backup",
                paragraphs: &[
                    "An encrypted backup is a file or QR code holding your words under a passphrase you choose. It is not a split at all: it is one copy that can be stored where a readable copy could not be. Its weak point is the passphrase, which is now a second thing to keep and a second thing to lose. Encrypted backups has its own page.",
                ],
            },
            Section {
                heading: "Which one",
                paragraphs: &[
                    "If the problem is that a copy may be destroyed, more copies solve it, and no scheme on this page is needed. Two places, two copies.",
                    "If the problem is that a copy may be found, a passphrase or an encrypted backup solves it with one extra secret to keep. A threshold scheme solves it with n places to keep instead of one, and m of them to visit before you can spend.",
                    "If the problem is that you want no single person or place to be able to act alone, a threshold over the backup is the wrong tool and a multisig or FROST wallet is the right one: those keep the coins under several keys all the time, rather than putting them back under one key whenever you spend.",
                ],
            },
            Section {
                heading: "What this device reads today",
                paragraphs: &[
                    "OpenSigner writes and reads Seed XOR parts and its own encrypted backup, and reads and writes BIP-39 words as every wallet does.",
                    "It reads and writes SLIP-39 shares. Type the shares of a backup and the key they hold is loaded, under its passphrase if it has one. Add a key › Create SLIP-39 shares makes a new key and writes it as shares: you choose how many groups, how many of them must be present, and how many shares each group has and needs, and the device shows every share in turn with the quiz. A key already loaded from shares can be written again, under a new plan, from its Backup menu; the old shares still open it.",
                    "A key made of BIP-39 words is not backed up as SLIP-39 shares, and never will be. What a set of shares gives back is the seed itself, and a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation will take.",
                    "It reads and writes Codex32. Type the string a Codex32 backup is written as, or the shares of a split of one, and the key it holds is loaded. Add a key › Create Codex32 shares makes a new key and writes it as one string or as a set of shares, and any key already loaded can be written as Codex32 from its Backup menu. A backup of a key made of BIP-39 words holds the 512-bit seed those words come to, which is 127 characters, and a wallet that expects words cannot read it back.",
                ],
            },
        ],
    },
    coordinators: Page {
        title: "Coordinator files",
        sections: &[
            Section {
                heading: "What a coordinator is",
                paragraphs: &[
                    "A coordinator is the wallet software on your online machine: Sparrow, Liana, Nunchuk, Specter, Bitcoin Core and others. It holds no keys. It watches addresses, shows balances and history, picks which coins to spend, sets the fee, builds the transaction, and broadcasts it once it is signed.",
                    "OpenSigner is the other half: it holds the keys and signs. Everything that crosses between the two is one of a small number of files, by QR code, memory card or USB stick. This page names them.",
                ],
            },
            Section {
                heading: "What the coordinator sends",
                paragraphs: &[
                    "A descriptor is one line of text describing a whole wallet: the keys, the script type and the derivation paths, with an eight-character checksum. It is the most portable of these, and the one to keep with your backup.",
                    "A wallet policy is the same wallet split into a template and a list of keys, which is BIP-388's form. Some coordinators and signers use it because the template is short enough to read on a small screen and compare by eye.",
                    "A multisig configuration file is a plain text list of the keys and the threshold, in the shape Coldcard introduced and several coordinators now export. It carries the same facts as a descriptor in a different layout.",
                    "A BSMS record does the same job with a setup ceremony around it, below.",
                    "A PSBT is the transaction itself, partially signed: the inputs, the outputs, the amounts, and the key information needed to sign, with room for each signer to add a signature.",
                ],
            },
            Section {
                heading: "What it gets back",
                paragraphs: &[
                    "An xpub, with the fingerprint and derivation path that say where it came from. That is what a coordinator needs before it can watch anything, and it is all you should ever send.",
                    "A signed PSBT: the same transaction with your signature in it. The coordinator collects the signatures it needs, finalises the transaction and broadcasts it.",
                    "Nothing else leaves. No seed words, no private key, and no passphrase.",
                ],
            },
            Section {
                heading: "BSMS",
                paragraphs: &[
                    "Bitcoin Secure Multisig Setup, BIP-129, is a procedure for building a multisig wallet without trusting the coordinator to report the keys honestly. Each signer produces its key record, the coordinator assembles them into a descriptor record, and the record goes back to every signer to be confirmed and stored before any coins are sent.",
                    "It can be run with a shared secret token, so that a coordinator that swapped a key in would produce a record the signers cannot confirm. The point of the ceremony is that every signer ends up holding the same wallet description, checked on its own screen, rather than trusting one machine's word for it.",
                    "This device writes a key record from a key's Account key row: choose one of the two multisig accounts, choose the BSMS key record format, and the device asks for the session token and a description. None writes the token 00, which says the session is not encrypted; a token the coordinator gave you is written into the record as it stands. The record is signed by the account key itself, and it leaves as text and as a code.",
                    "An encrypted record is a different file, hex rather than text, and this build writes none and reads none. A record with a token in it is still plain text, so carry it the way you would carry any other file the coordinator must not be able to change: by QR code or memory card, not through the coordinator.",
                    "This device reads a descriptor record with Scan. The review shows the quorum, the derivation paths the record restricts the wallet to, the wallet's own first address, and which key of the list is this device's, and you accept the wallet there. A record that is one signer's key is refused where it is read: it is a key, not a wallet. Export on a multisig wallet writes the descriptor record back out, as text and as a code. Encrypted records are refused by name.",
                    "A BSMS record is worth keeping for the same reason a descriptor is: it is what recovery needs when the coordinator is gone.",
                ],
            },
            Section {
                heading: "Bitcoin Core",
                paragraphs: &[
                    "Bitcoin Core is not a coordinator with a wallet screen of its own, but it watches a wallet and builds transactions for it once it has been told the descriptor. Export on any wallet with a descriptor offers the Bitcoin Core import format, which is the JSON that `importdescriptors` takes: the wallet's descriptor with its checksum, marked active, over the first thousand addresses of each chain.",
                    "Choosing the format asks where to start scanning. The start reads the whole chain from the first block, which finds coins the wallet already holds and takes a while; Now looks at nothing before this moment, which is what a wallet that has never received anything wants.",
                    "Save the file, carry it to the machine running Core, and hand it over with `bitcoin-cli -rpcwallet=<name> importdescriptors \"$(cat <file>)\"` on a wallet created with `createwallet` and private keys disabled. Core keeps the private keys nowhere: this file is the public wallet and nothing else.",
                ],
            },
            Section {
                heading: "Why the device checks anyway",
                paragraphs: &[
                    "The coordinator is on an internet-connected machine, which is the machine most likely to be compromised. A coordinator that has been tampered with can show you one address and build a transaction paying another. Nothing the device receives is trusted for that reason.",
                    "So the device verifies. A wallet is shown to you in full before it is accepted, keys and threshold and script type, and you accept it on this screen rather than on the coordinator's. A transaction is shown with every output and its amount, and every output the device believes is your change is verified by deriving that address from a wallet you accepted. An output the device cannot account for is shown as a payment.",
                    "That is the whole reason the two programs are separate. Check the address on this screen against the one you meant to pay, and check it on the coordinator's screen too, and a compromise of either machine alone cannot move your coins.",
                ],
            },
        ],
    },
    silent_payments: Page {
        title: "Silent payments",
        sections: &[
            Section {
                heading: "What a silent payment address is",
                paragraphs: &[
                    "A silent payment address is one string, starting `sp1q`, that you can print on a card, put on a web page or send to anyone who will ever pay you. It is 116 characters long, and it is not a Bitcoin address: nothing is ever paid to it directly, and no block explorer will show it.",
                    "What the payer does with it is arithmetic. They take the two public keys the address carries, combine them with the public keys of the coins they are spending, and work out a fresh taproot output that belongs to you. They pay that output. The next person who pays the same address works out a different output, because they are spending different coins.",
                    "So the address can be published and reused forever, and the chain still shows a run of unrelated taproot outputs. That is the whole point: address reuse is the ordinary way a person's payments get linked together, and this is the arrangement that removes the reason to reuse an address.",
                ],
            },
            Section {
                heading: "What the two keys are",
                paragraphs: &[
                    "The address carries a scan key and a spend key. Both come from one of your keys, at paths BIP-352 sets aside: the scan key at `m/352h/0h/0h/1h/0` and the spend key at `m/352h/0h/0h/0h/0` on mainnet. Recovering the key recovers the wallet.",
                    "The scan key finds payments. The spend key spends them. They are separate so that the finding can be given away without the spending: a machine that watches the chain for you needs the scan private key and gets no ability to move a coin.",
                ],
            },
            Section {
                heading: "What the scan key gives away, and to whom",
                paragraphs: &[
                    "Handing over the scan private key hands over the ability to see every payment this wallet has ever received, and every payment it ever will. It does not hand over the ability to spend one.",
                    "That is a real trade. A watching server with your scan key knows your whole receiving history, and if it also knows who you are, it knows who paid you. Give it to software you run, or to a service you have decided to trust with that; do not give it to anyone you would not show your bank statements to.",
                    "Export on a silent payments wallet writes it as `sp(spscan1q…)`, which is BIP-392's descriptor for exactly this job. The device shows it on a hidden panel, as it shows seed words, because it is a secret.",
                ],
            },
            Section {
                heading: "Labels",
                paragraphs: &[
                    "A label turns the one address into several. Label 1, label 2 and so on each give a different `sp1q` string, all found by the same scan key and spendable by the same spend key. Give one to each payer and you can tell payments apart without publishing separate wallets.",
                    "Labels are derived from the scan private key, not stored, so they come back from the seed. The device hands them out in order and remembers how many it has handed out. Label 0 is reserved: it is the label a wallet uses for its own change, and it is never handed to anyone, because a payer who knew it could make a payment your wallet would file as change.",
                ],
            },
            Section {
                heading: "Checking that you were paid",
                paragraphs: &[
                    "A silent payment leaves no address to look up. To know that a transaction paid you, someone has to do the arithmetic with the scan private key.",
                    "Check a payment does it for one transaction. Give it the transaction, and the previous transactions of any inputs whose public keys are not in the transaction itself, and the device says which outputs pay this wallet, what they pay, and which label each was paid to. It reads nothing from the network: the transaction has to be brought to it, as a PSBT or as a raw transaction, by code or by file.",
                    "For a wallet that watches continuously, export the scan descriptor to software that can scan blocks. That is what the descriptor is for.",
                ],
            },
            Section {
                heading: "Why the device does not send yet",
                paragraphs: &[
                    "Paying a silent payment address needs the private keys of every input of the transaction, added together, before the output can be worked out. A signing device that is handed a PSBT does not hold all of them, and the PSBT fields that would let several devices do this together are BIP-375, which is still a draft.",
                    "Until that settles, this device receives silent payments and does not send them. A wallet elsewhere can pay a silent payment address today; this one cannot.",
                ],
            },
        ],
    },
};
