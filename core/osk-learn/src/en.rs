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
                    "Keep two copies in two places. Before you rely on a copy, check it: the quiz asks for each word from your written copy, and Check a copy of this seed checks it later, by typing it or scanning its SeedQR.",
                ],
            },
            Section {
                heading: "What this device is",
                paragraphs: &[
                    "Faraday is a signer: it holds keys and signs transactions another program built. It has no networking code, and a stick or a Pi runs it on a system with no network, so it shows no balance and no history. Watching addresses and building transactions is the work of a coordinator, on a machine that is online.",
                ],
            },
            Section {
                heading: "What happens next",
                paragraphs: &[
                    "Make a new wallet, or add a key you already have; write the words down and check the copy with the quiz; back the wallet up with its plan; and check one address against your coordinator. To practise on a test chain first, set Network to Signet in Settings before you add a key.",
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
                    "Faraday names each loaded key by its fingerprint: eight hex digits computed from the key, the same in every wallet program.",
                ],
            },
            Section {
                heading: "Why 12 or 24",
                paragraphs: &[
                    "The word protocol is BIP-39. It has a list of 2048 words, and 2048 is 2 to the 11th, so each word carries 11 bits. Twelve words carry 132 bits: 128 bits of key and 4 of checksum. No amount of computing can guess a 128-bit number, so 12 words are enough. Twenty-four words carry 256 bits of key and 8 of checksum, for people who want a larger margin.",
                    "The three lengths in between are rarer than 12 and 24, and nothing else distinguishes them: they are built by the same rule, entropy plus a checksum of one bit per four bytes of it, and every wallet that follows BIP-39 reads them. 15 words are 160 bits, 18 are 192, and 21 are 224. Faraday makes and reads all five.",
                ],
            },
            Section {
                heading: "How they are made",
                paragraphs: &[
                    "The randomness a key is made from is called entropy. A key made from something you remember, such as a sentence or a favourite set of words, is one of a small number of possibilities however long it is, and cracking programs try those possibilities first.",
                    "Real entropy comes from a physical process such as rolling dice, flipping coins or shuffling a deck. 128 flips of a coin, or 50 rolls of a die, give 128 bits. New key turns the rolls or flips into words, so no computer has to generate the key for you.",
                    "Each word has a fixed place on the BIP-39 list. Tools › BIP-39 words lists the 2048 words, each with its number and its 11 bits.",
                    "A key's entropy is sometimes written down as hex digits rather than words. New key takes hex digits made elsewhere as its source: the digits become the key as they are, so they must already be random.",
                ],
            },
            Section {
                heading: "Keys and other secrets from one key",
                paragraphs: &[
                    "BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.",
                    "Tools › Child seeds and passwords asks which application. Words derives another BIP-39 key, which Load as a key adds to the keys loaded like any other. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16 to 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.",
                    "A derived password or WIF is as secret as the key it came from: anyone who has it can compute nothing about the parent, but they hold whatever it protects, and someone who has the parent words can recompute every child. Faraday saves the value into the open vault, sealed under its passphrase; it leaves the vault only after a warning. The same key, application and index give it again when you need it.",
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
                    "To set up a transaction for Faraday, you need a coordinator that can create Partially-Signed Bitcoin Transactions, or PSBTs. You bring the PSBT to Faraday by QR code or on a stick (on the Pi, the SD card).",
                ],
            },
            Section {
                heading: "Signing",
                paragraphs: &[
                    "Before you sign, Faraday shows what the transaction sends, the change and the fee. Change that Faraday rebuilt from the wallet's own keys is marked Verified. The Nonce check lists every signature the transaction already carries, with the key it is under and whether it verifies. Read every address and amount in full and compare them with what your coordinator shows. If anything differs, do not sign; go back to the coordinator and find out why.",
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
                    "Dice and coins are physical randomness that you can watch happen. Each roll or flip is a real event in front of you, you can count them, and no software took part. That is the whole reason Faraday asks for dice rolls or coin flips instead of generating a seed for you.",
                    "A 6-sided die gives about 2.58 bits per roll, so 50 rolls give 128 bits. A coin gives 1 bit per flip, so 128 flips give 128 bits. Use a real die and a real coin, not a phone app.",
                    "A longer key asks for more of both. 12 words take 50 rolls or 128 flips; 15 words take 62 rolls or 160 flips; 18 words take 75 rolls or 192 flips; 21 words take 87 rolls or 224 flips; 24 words take 99 rolls or 256 flips. The screen states the count it is waiting for and counts your entries against it.",
                    "While you enter dice rolls, Faraday counts how often each face came up and flags long runs and lopsided counts. That check catches a loaded die or a lazy hand. It cannot turn bad rolls into good ones. If it warns you, roll again.",
                ],
            },
            Section {
                heading: "Computer randomness",
                paragraphs: &[
                    "Every computer has a random number generator, built from a hardware chip plus the operating system. It is usually fine. The problem is that you cannot see it working. A chip that is faulty, or that was built to be predictable on purpose, produces keys that look perfectly normal and that its maker can recreate. Nobody can tell from the outside.",
                    "For this reason Faraday lists the sources you work by hand first and marks the device's own generator with the trust it asks for, because you can inspect a die and you cannot inspect a chip.",
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
                    "New key offers several ways to produce the bits a key is made of. They are not equally good. The difference between them is what you have to trust for the result to be random at all. This page says, for each source, what that is.",
                    "None of them can be checked afterwards: a key from a rigged source looks exactly like a key from a fair one. Choosing the source is the only control you have over this.",
                ],
            },
            Section {
                heading: "Dice and coins",
                paragraphs: &[
                    "With a die or a coin, the only thing you trust is the object in your hand. You can see every event, count them, and repeat the whole process with a different die. The device only records what you type.",
                    "A die gives about 2.58 bits a roll, so 50 rolls give 128 bits and 99 give 256. A coin gives one bit a flip: 128 or 256 of them. The rolls are hashed with SHA-256 and the flips are packed as bits, which is what other signers do, so you can reproduce either result on a computer you trust.",
                    "The counts for the three lengths in between follow the same arithmetic: 62 rolls or 160 flips for 15 words, 75 rolls or 192 flips for 18 words, 87 rolls or 224 flips for 21 words.",
                    "A die can also stand in for a coin: each roll is one flip, 1, 2 or 3 for tails and 4, 5 or 6 for heads, one bit a roll.",
                ],
            },
            Section {
                heading: "Three ways to read the dice",
                paragraphs: &[
                    "Other signers turn dice into a key in more than one way, and the same rolls give different keys under each. The device asks which one you are following, so that a key you made elsewhere comes out the same here.",
                    "**Hashed** is what most signers do. The rolls are written out as digits and hashed with SHA-256. Coldcard, SeedSigner and EntropyLab's \"Base 10\" row all do exactly this, so `echo -n 3246115135… | sha256sum` reproduces the result on any computer.",
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
                    "That is the whole caution, and it is why the row carries one. In a browser the generator belongs to the browser and to the page, so the website demo offers the row but puts one of its public demo keys in place of the key it makes.",
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
                heading: "Never",
                paragraphs: &[
                    "Never photograph your seed words.",
                    "Never put them in cloud storage, a password manager, a notes app, a spreadsheet or an email, not even for a moment. A Faraday vault is the exception: it is made to keep them, encrypted under its passphrase.",
                    "Never type them into a website, or into any software that asks for them.",
                    "Never read them out to anyone, whoever that person says they are.",
                ],
            },
            Section {
                heading: "Verify your backup",
                paragraphs: &[
                    "A backup you have never read back may be wrong: handwriting is misread, words get skipped, and steel gets stamped wrong. Faraday's quiz checks your written copy: it asks for every word in random order, offers four candidates each time, and checks your answers against the key in memory.",
                    "If you skip the quiz, the copy is marked as not checked. That does not mean the backup is wrong. It means nobody has checked.",
                    "Check the copy again from your physical backup about once a year, and after every move, with Check a copy of this seed. Ink fades, and you may forget where a copy is.",
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
                    "An OpenSigner backup is one small file, ending in .oskb, that OpenSigner writes: something locked under a passphrase chosen when it was made. Faraday does not write one. It reads one into a vault, where it is kept under the vault's passphrase from then on.",
                ],
            },
            Section {
                heading: "What it can hold",
                paragraphs: &[
                    "One of four things. A key's seed words. A key's master seed, which is what a SLIP-39 or a Codex32 key is and which has no words. A note. Or a wallet's recovery sheet: its descriptor, its name and a note.",
                    "A key with a wallet passphrase is backed up as its words alone. The wallet passphrase is typed again when the key is loaded.",
                ],
            },
            Section {
                heading: "Reading one into a vault",
                paragraphs: &[
                    "Copy the file onto a stick and bring it in, so it is in the Inbox, and unlock the vault it is to go into. Import into the vault on the file in the Inbox, or Import with the file's name on the vault's Bitcoin keys or Notes, asks for the backup's passphrase.",
                    "Words and a master seed become a key in the vault. A note becomes a note, and a recovery sheet becomes a recovery sheet. The file in the Inbox is not changed.",
                    "A wrong passphrase opens nothing, and Faraday says that passphrase does not open this backup. There is no other way in: without the passphrase the file is a few hundred bytes that look random.",
                ],
            },
            Section {
                heading: "The memory it needs",
                paragraphs: &[
                    "The passphrase is stretched with Argon2id at the memory the file states in its header, 64 MiB, 256 MiB or 1 GiB, chosen when it was made. A computer that cannot spare that memory cannot open it, and Faraday says how much it needs.",
                ],
            },
            Section {
                heading: "Reading one without Faraday",
                paragraphs: &[
                    "The OSKB format is written down in docs/BACKUP.md, in OpenSigner's source and in Faraday's, and tools/backup/decrypt.py in either opens a file on any computer with Python.",
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
                    "Seed XOR splits one key into two or more parts. Each part is a full set of seed words of the same length as the original: 24 words split into 24-word parts, 12 into 12-word parts. Combining the parts with the XOR operation gives the original key back.",
                    "XOR is addition without carrying, done bit by bit. Because it is its own inverse, the order of the parts does not matter and no part is more important than another. All but the last part are random, and the last one is worked out so that the parts XOR back to your key. If a random part could be guessed, the last part alone would give the key.",
                    "The scheme is Coldcard's. Parts made here combine on a Coldcard, and parts made on a Coldcard combine here.",
                    "In Faraday, a seed is split among the copies a backup makes by hand, into two or three parts; Faraday draws the random parts from its own generator. Parts are combined by Add a key, under Other forms, Type Seed XOR parts: what combining gives back is a key that already existed.",
                ],
            },
            Section {
                heading: "Every part is needed",
                paragraphs: &[
                    "All of the parts are needed to recover the key. Lose one and the key is gone, exactly as if you had lost the words themselves. Splitting into three parts and keeping them in three places makes theft harder and loss easier.",
                    "Any part on its own tells you nothing about the key. Two parts of a three-part split tell you nothing either. It is all of them or none.",
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
                    "Nothing stores the passphrase unless you keep it in a vault. It cannot be reset, recovered, or guessed. If you forget it, the seed words alone lead to a different, empty wallet.",
                    "A lost passphrase loses the funds under it for good, whatever backup of the words there is.",
                ],
            },
            Section {
                heading: "Rolling one with dice",
                paragraphs: &[
                    "A passphrase you make up yourself is weaker than it looks. Lines from songs, names with digits after them, and letter substitutions are all in the lists a cracking program tries first. Words chosen by dice are not in any such list.",
                    "Create a vault rolls one for you, under Generate with dice. You pick a list, you pick how many words, and you throw a die. Five throws name one word of the long list, four throws name one word of a short list. The device does not choose anything; it only looks the words up.",
                    "The lists are the ones the Electronic Frontier Foundation publishes, and Tools › Word lists shows each of them. The long list has 7776 words, so each word is worth 12.9 bits. The short lists have 1296 words each, so each word is worth 10.3 bits; their words are shorter to type, and every two words of the second short list are at least three edits apart, so a misread word is still recognisable.",
                    "Six words from the long list are 77 bits. That is the length to use unless you have a reason to differ: it is more than offline guessing reaches, and it is six words to write down rather than ten.",
                    "A rolled passphrase is text like any other. Create a vault uses it as the vault's passphrase; rolled the same way from the lists, it can be the BIP-39 passphrase on a key, or the password for anything else. Keep it apart from the seed words, as the rest of this page says.",
                ],
            },
            Section {
                heading: "Storing it",
                paragraphs: &[
                    "Write it down exactly, spaces and capitals included, and read it back before you use it for real. Keep it apart from the seed words, so that whoever finds one does not have both. That separation is the point of a passphrase: a stolen backup of the words alone is useless.",
                    "A vault can keep a seed's passphrase with the seed, and unlocking the vault then loads the key the two give. Kept that way, the passphrase is behind the vault's passphrase rather than apart from the words, and whoever opens that vault has the key. A passphrase kept in a vault belongs in a different vault from its seed, at another place. Faraday's backup plan warns when a seed and its passphrase are at the same place. A seed kept in a vault without its passphrase loads with Load with a passphrase, which asks for it each time and stores nothing.",
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
                    "An address shown on Faraday was derived on Faraday, from your key, on a device with no network. It is the one thing in the process you can trust.",
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
                    "Before you give out a receiving address, show the same address on Faraday and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.",
                    "You can also check an address the other way round: Tools › Check an address scans it and searches the loaded wallets for it. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.",
                ],
            },
            Section {
                heading: "Sending",
                paragraphs: &[
                    "Before you sign, read every output. Confirm that the destination address and amount match what you intended, and that the change output is marked as derived from your key. An output that Faraday cannot confirm as yours is not yours.",
                ],
            },
            Section {
                heading: "Warning cards",
                paragraphs: &[
                    "When Faraday cannot confirm something, the review shows a warning under the transaction. A danger is in red: an input whose value cannot be checked, a signature type that does not cover the outputs, a fee out of all proportion to the amount. Change it cannot rebuild from a loaded wallet is marked Unverified.",
                    "A warning you cannot explain is a reason not to sign. Go back to your coordinator and find out why.",
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
                    "An air-gapped device has no connection to any network. Faraday has no networking code, and the system a stick or a Pi starts has no network drivers at all, so nothing can reach it over a network and nothing can leave it that way. The only ways in and out are QR codes, shown on a screen and read by a camera, and files that you carry across on a stick yourself (on the Pi, the SD card).",
                    "Faraday Desktop runs on an ordinary computer that is online. It has no air gap, and a seed that holds funds does not belong there.",
                ],
            },
            Section {
                heading: "What crosses the gap",
                paragraphs: &[
                    "Into Faraday: an unsigned transaction (a PSBT), an address to check, a signed message to check, a wallet description, a vault, and your seed words when you load a key.",
                    "Out of Faraday: a public key for your coordinator, a signed transaction, a signed message, and vaults. Your seed words and private keys leave only sealed in a vault, or unprotected when you ask for that and confirm the warning.",
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
                    "An air gap does not check the transaction for you. A compromised coordinator can still hand you a PSBT that pays the wrong address, which is why you verify every address and amount on Faraday.",
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
                    "Malware on a computer can watch for an address being copied and replace it with the attacker's address. The address you paste is not the address you copied. Always compare the address on Faraday with the address that the person you are paying gave you, in full.",
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
                    "A wallet downloaded from a search advert, an app store clone, or a link in a forum post builds transactions that pay someone else, or simply uploads your seed words. Get your coordinator from the project's own website and check the download's signature before running it. Check Faraday the same way: each release lists its files' SHA-256 sums in SHA256SUMS, signed with the release key.",
                ],
            },
            Section {
                heading: "Your own mistakes",
                paragraphs: &[
                    "Most losses are not theft. They are a backup that was never tested, a passphrase that was never written down, a wallet description for a multisig that nobody kept, or funds sent to the wrong network. Faraday cannot protect you from these. The Backups, Passphrases, and Inheritance pages can.",
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
                    "Every key needs its own backup, so a 2-of-3 wallet means three sets of seed words in three places. On top of that, recovery needs the wallet description: which keys are in the wallet, the script type, the derivation path, and how many signatures are required. With every key in hand and the script type and path known, a lost description can be rebuilt; Faraday's Restore does this. With only a quorum of the seeds, two of the three, it cannot: the third key's xpub is in the description and nowhere else, so the addresses cannot be worked out and the funds cannot be found.",
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
                heading: "Using multisig with Faraday",
                paragraphs: &[
                    "Make a new wallet makes a multisig from keys loaded here and cosigners' xpubs. Restore an existing wallet reads one a coordinator made: a descriptor, a wallet file, a multisig config or a BIP 129 record, by QR code or from a file. Faraday shows every key in the wallet and which of them are loaded here.",
                    "Once loaded, Faraday recognises the addresses and change of that wallet, so a transaction you sign is checked against the wallet you loaded. A wallet is kept for the session; saved into a vault, it comes back when the vault is unlocked.",
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
                    "A common shape is one key that can spend at any time and a second key that can spend alone after a year. That is two spend paths. Faraday lists them one row each, with the keys each path needs and how long it waits.",
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
                    "Tools › Miniscript compiler compiles a policy into a descriptor. Where a policy names a key, the fingerprint of a key this device already holds stands for that key's account key, so pk(73c5da0a) is accepted and needs no xpub typed. The Keys row above the field lists the fingerprints it will take; with no key loaded there is no such row and every key must be written out.",
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
                    "Faraday shows it as a QR code or writes it as a file, together with the fingerprint and derivation path the coordinator needs to use it: Show xpub QR and Xpub file, on a key of the wallet's chart. A wallet's public files carry the account keys it is built on. Tools › Explore a key shows a key's extended public key at any path.",
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
                    "You will see the same key written with different prefixes. The prefix is a convention called SLIP-132 that tells the coordinator which script type the account uses: xpub for legacy, ypub for nested SegWit, zpub for native SegWit. The key inside is identical either way. Tools › Convert an xpub shows one key in every spelling.",
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
                    "Tools › Vanity address tries one candidate after another until the first address of the account begins with the characters you asked for. Each free character costs 32 tries on a bech32 address and 58 on a base58 one, so four characters past bc1q is about a million tries, and the device states the rate it is managing and how long that is.",
                    "There are two dials. The passphrase counter appends characters to the key's BIP-39 passphrase, which means every candidate is a different key and costs a full derivation; taking the find loads that key beside the one you started from. The account index steps through the accounts of the key you already have, which is far cheaper and adds a wallet rather than a key.",
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
                heading: "What Faraday checks on a signature that arrived",
                paragraphs: &[
                    "A transaction can reach the device with signatures another device or another person already made. Faraday checks each of them against the key it is under and refuses the transaction when one is not a signature of what is in front of you, because that means the transaction was changed after it was signed. It also looks for two signatures under one key that share a nonce, anywhere in the transaction, and refuses that too: those two signatures are the key. Where a signature is under a key this device holds, it signs the same input again under each nonce rule and says which one produced it, so you can see whether another device signed the way it says it does. The Nonce check, on the review and in Tools, lists them.",
                ],
            },
            Section {
                heading: "How Faraday picks a nonce",
                paragraphs: &[
                    "Faraday chooses no nonce with any randomness of its own. Every nonce is calculated from the key and the transaction alone, using RFC 6979 for ECDSA signatures and BIP-340 with no auxiliary randomness for Schnorr signatures. The same transaction signed with the same key always produces exactly the same signature bytes.",
                    "As a check on itself, Faraday signs every input twice and refuses to continue if the two results differ.",
                ],
            },
            Section {
                heading: "Which nonce rule",
                paragraphs: &[
                    "There are two standard ways to derive an RFC 6979 nonce. \"Low R\" retries the nonce, with a counter added as the RFC's extra data, until the signature comes out in its shortest form. \"First\" stops at the first nonce. Faraday signs with Low R, for transactions and messages alike, and has no setting for it.",
                    "Low R is what Bitcoin Core, Sparrow, and Electrum produce. First is what the RFC's own test vectors, the BIP-174 test vectors, and Trezor produce. Bitcoin Core signs messages without the retry, so a Core message signature does not match Faraday's byte for byte.",
                    "For taproot, Faraday adds no randomness to a signature, so it can be compared against another implementation that does the same. BIP-340 recommends 32 fresh bytes in every signature as a defence against fault and side-channel attacks; a signature made that way cannot be reproduced.",
                ],
            },
            Section {
                heading: "Checking Faraday against another signer",
                paragraphs: &[
                    "Because the nonce is deterministic, you can verify that Faraday is not hiding anything in it. Load the same seed words into a second implementation that signs with Low R, and for taproot with no auxiliary randomness, on another offline machine, sign the same transaction on both, and compare the signature bytes. They must match exactly. A device that was leaking your key through its nonces would fail this comparison.",
                ],
            },
            Section {
                heading: "What is not offered yet",
                paragraphs: &[
                    "There are protocols in which the coordinator contributes randomness to the nonce and then checks that the signer did not choose the nonce alone. Faraday does not implement one yet, because no coordinator currently speaks such a protocol over QR codes or PSBT files.",
                    "MuSig2 is the other case. Several keys aggregate into one, and each signer's nonce is part of one aggregate nonce, so the nonces are exchanged before anyone signs. Faraday signs in either order. When every other signer's nonce is already on the transaction, it derives its own nonce from theirs and from the transaction, signs in one pass, and keeps nothing afterwards. When a nonce is missing, it goes first: it draws its nonce, writes the public half onto the transaction for the coordinator, and holds the secret half in memory as a session. That session is the device's memory of one transaction. It ends when the device signs, when you lock it, and when it loses power, and nothing writes it to storage, because a secret nonce that survives a restart is the classic way to use one twice, and using a MuSig2 nonce twice hands over the private key. If the transaction comes back after the session has ended, the device draws a new nonce, says so, and the other signers sign again.",
                    "A FROST wallet is the one place where Faraday writes a secret nonce to a file, and the reason it is safe there is that there are not two devices. One device carries the wallet's shares between the places you keep them. At the first place it draws every signer's nonce itself, signs with the share it is holding, and saves one file that holds the transaction and the other signers' secret nonces bound to it. At the next place the same device reads that file back, checks that each stored nonce is the one the transaction already names, that the file is bound to this transaction and to this set of shares, and that the signature already on it verifies, and only then signs. Reusing a nonce leaks a share when the same nonce signs under two different challenges, and the challenge changes only if someone varies a nonce or the message; here both were fixed by the device that drew them, and any change breaks a check it can make by arithmetic. A copy of the file taken in transit is a random number with nothing to pair it with, because the signature made under it never leaves the device. None of this extends to two devices signing together: a file like this is read only by the device that wrote it.",
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
                    "A signed message is a piece of text plus a signature made with the key behind one of your addresses. It has three parts: the address, the signature, and the text. The file Faraday saves and the QR code it shows carry the address on the first line, the signature on the second line, and the message from the third line to the end.",
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
                    "Faraday signs with a single-key wallet's key, at one of its receive addresses. It produces BIP-137 signatures for legacy and SegWit addresses, and BIP-322 signatures, which work for every address type including Taproot. Choose the format your verifier expects. Most exchanges and older tools expect BIP-137.",
                ],
            },
            Section {
                heading: "Checking a signed message",
                paragraphs: &[
                    "Check a signed message, in Tools, reads a signed message from a QR code or a file, checks whether the signature matches the address and the text, and shows you the text that was checked. Read the text. A valid signature over the wrong text proves the wrong thing.",
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
                    "The route from a master key to one of its child keys, level by level, starting at the root, m. An h, or an apostrophe, marks a hardened level. The levels are purpose, coin, account, change, and index.",
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
                    "A way of picking words at random by throwing dice. Each throw of four or five dice names one word of a published list, and the words together are a passphrase. Faraday carries the three lists the Electronic Frontier Foundation publishes.",
                ],
            },
            Section {
                heading: "Entropy",
                paragraphs: &[
                    "Randomness, measured in bits. 128 bits of real entropy is unguessable. Entropy from a memorable phrase is far lower than it looks.",
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
                    "Extra text added to the seed words that produces a different key. Nothing stores it unless you keep it in a vault, which is best a different vault from its seed.",
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
                    "The 12 to 24 words that are the key. Whoever reads them controls the funds.",
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
                    "The offline software or device that holds the keys and signs transactions. Faraday is a signer.",
                ],
            },
            Section {
                heading: "SLIP-39",
                paragraphs: &[
                    "Trezor's backup scheme: Shamir's Secret Sharing over a word list of its own, with a threshold. Its shares are not BIP-39 seed words.",
                ],
            },
            Section {
                heading: "Vault",
                paragraphs: &[
                    "One file on a stick holding keys, wallets, entries and notes, encrypted under a passphrase. It has four slots, and each passphrase opens one of them.",
                ],
            },
            Section {
                heading: "Watch-only",
                paragraphs: &[
                    "A wallet whose keys are not loaded here. Its addresses can be derived and compared; nothing can be spent from it.",
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
                    "Each calculator in Tools takes what you type, works one thing out from it, and keeps nothing. None of them touches a key, and none of them stores anything, so nothing on these screens is a secret. An extended private key is refused.",
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
                heading: "Convert an xpub",
                paragraphs: &[
                    "One extended public key has several spellings. BIP-32 writes it as xpub or tpub; SLIP-132 writes the same key as ypub, zpub, upub or vpub to say which script type it is meant for. The key material is identical; only the four version bytes differ.",
                    "Give it any of them and the tool shows all of them, with the network, the depth, the fingerprint and the child number the key states about itself. Extended private keys are refused.",
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
                    "The review Sign a transaction shows, over a transaction you are reading rather than signing: what it spends and pays, field by field. Whatever wallets you have loaded are used to read it, so change of a wallet in use is recognised here as it is when you sign. Nothing is signed.",
                    "Use it to read what a coordinator produced before you sign it anywhere, or to see what a raw transaction from a block explorer actually pays.",
                ],
            },
            Section {
                heading: "Lightning node key",
                paragraphs: &[
                    "A Lightning node has an identity key, and a backup of that node is either an LND cipher seed — twenty-four words that are not a BIP-39 phrase — or the BIP-39 words an ldk-node wallet was built from. The tool reads either one and says which node it is: the public key the network knows the node by, and for a cipher seed the version and the birthday it carries.",
                    "Use it to confirm that a backup in your hand belongs to the node you think it does, before you restore it anywhere. A node key is hot by nature — it is on a machine that is online, in use, all the time — so Faraday signs nothing with it. The node's private key is a secret: it goes into the open vault, or out unprotected only after a warning.",
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
                    "In Faraday the sidebar lists them apart: Keys are the secrets, and Wallets are the rules, whether you made them here or a coordinator sent them. The kinds below are the rules the device can read. Which one suits you depends on what you are protecting against, so all of them are offered.",
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
                    "Every coordinator supports this arrangement, and it has the most software behind it. It needs the wallet description as well as the keys. With every key, and the script type and path known, a lost description can be rebuilt, as Faraday's Restore does; with only enough seeds to sign, it cannot, and those seeds alone do not find the funds. The Multisig page covers this kind in full.",
                ],
            },
            Section {
                heading: "Taproot multisig",
                paragraphs: &[
                    "The same m-of-n written as a Taproot script path. Unspent, the address looks like any other Taproot address and says nothing. Spending reveals the one script that was used and nothing else, and the transaction is smaller than the equivalent classic multisig.",
                    "When the wallet has no way to spend with a single key, the Taproot internal key is set to a NUMS point: a public key chosen so that nobody knows a private key for it. The device shows this in the review, because an internal key that has not been shown to be unspendable is a way to spend the wallet that you never agreed to.",
                    "Make a new wallet builds this kind: Taproot multisig under More kinds, then the keys and how many must sign. Fewer coordinators write this than write classic multisig, and recovery software for it is younger.",
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
                    "Make a new wallet does not offer this kind; Faraday reads one from its descriptor. A wallet Liana or another coordinator built arrives that way, including one with more than one recovery path, and Faraday names every path and every wait. Tools › Miniscript compiler writes a policy of your own as a descriptor, and says whether this device loads it.",
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
                    "A wallet made from public keys alone derives addresses, checks change and exports itself, and cannot sign. Faraday calls it watch-only. Loading a friend's descriptor to check an address they gave you is a legitimate use of the device, and so is keeping a watch-only copy of a wallet whose keys are elsewhere.",
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
                    "A dealer makes the shares. On this device that means choosing how many shares and how many must sign, picking the loaded keys that will be the chosen shares, and letting the device compute the rest; each computed share becomes a loaded key of its own, which you back up like any other key and can then forget.",
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
                    "Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. Faraday reads it, groups and all, and Add a key › Make SLIP-39 shares makes a key whose only written form is shares.",
                ],
            },
            Section {
                heading: "Codex32",
                paragraphs: &[
                    "Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1, with optional shares and a threshold, as SLIP-39 has. What is unusual about it is that the checksum and the share arithmetic were designed to be worked by hand, with the printed tables that come with the standard, on paper and without a computer.",
                    "That is the point of it. You can create a seed, verify the checksum, and combine shares without trusting any device with the secret, including this one. The cost is time and care: doing it by hand is slow, and a mistake in the arithmetic is silent until the checksum catches it.",
                    "Pick it when your objection is to trusting a computer at all.",
                    "Faraday reads and writes it. Add a key › Type a codex32 secret takes a string, or the shares of a split of one, and loads the key it holds. A backup's copies by hand can write a key made of words as codex32 shares of its seed, 2 of 3 or 3 of 5. What a codex32 backup of a key made of words holds is that seed, with the passphrase already in it, and not the words.",
                ],
            },
            Section {
                heading: "Seed XOR",
                paragraphs: &[
                    "Seed XOR splits a key into parts that are themselves ordinary seed phrases, and every part is needed. It has no threshold: two parts of a three-part split are worth nothing. In exchange it is simple enough to do with pencil and paper from the wordlist numbers, and any wallet can read a part. Seed XOR has its own page.",
                ],
            },
            Section {
                heading: "A vault",
                paragraphs: &[
                    "A vault is a file on a stick holding your words encrypted under a passphrase you choose. It is not a split at all: it is one copy that can be stored where a readable copy could not be. Its weak point is the passphrase, which is now a second thing to keep and a second thing to lose. Vaults has its own page.",
                ],
            },
            Section {
                heading: "Which one",
                paragraphs: &[
                    "If the problem is that a copy may be destroyed, more copies solve it, and no scheme on this page is needed. Two places, two copies.",
                    "If the problem is that a copy may be found, a passphrase or a vault solves it with one extra secret to keep. A threshold scheme solves it with n places to keep instead of one, and m of them to visit before you can spend.",
                    "If the problem is that you want no single person or place to be able to act alone, a threshold over the backup is the wrong tool and a multisig or FROST wallet is the right one: those keep the coins under several keys all the time, rather than putting them back under one key whenever you spend.",
                ],
            },
            Section {
                heading: "What this device reads today",
                paragraphs: &[
                    "Faraday writes and reads Seed XOR parts, keeps words in vaults, reads an OpenSigner backup into a vault, and reads and writes BIP-39 words as every wallet does.",
                    "It reads and writes SLIP-39 shares. Add a key › Type SLIP-39 shares loads the key a backup's shares hold, under its passphrase if it has one, groups and all. Add a key › Make SLIP-39 shares makes a new key and writes it as one group of shares: you choose how many shares there are and how many are needed, and Faraday shows every share in turn with the quiz.",
                    "A key made of BIP-39 words is not backed up as SLIP-39 shares, and never will be. What a set of shares gives back is the seed itself, and a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation will take.",
                    "It reads and writes Codex32. Add a key › Type a codex32 secret loads the key a string or its shares hold, and a backup's copies by hand write a key made of words as codex32 shares of its seed. A backup of a key made of BIP-39 words holds the 512-bit seed those words come to, which is 127 characters, and a wallet that expects words cannot read it back.",
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
                    "Faraday is the other half: it holds the keys and signs. Everything that crosses between the two is one of a small number of files, by QR code or on a stick (on the Pi, the SD card). This page names them.",
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
                    "Faraday writes a key record for each multisig key held here, under a wallet's public files, For the cosigners: the signed xpub record of BIP 129. The record is signed by the account key itself, and it leaves as text and as a code.",
                    "An encrypted record is a different file, hex rather than text, and Faraday writes none and reads none. A record with a token in it is still plain text, so carry it the way you would carry any other file the coordinator must not be able to change: by QR code or memory card, not through the coordinator.",
                    "Faraday reads a descriptor record wherever it reads a wallet: Scan, Files, and Restore an existing wallet. It checks the wallet's first address against the one the record states, and shows the quorum, the derivation paths the record restricts the wallet to, and which keys are loaded here. A multisig wallet's public files write the descriptor record back out, as text and as a code. Encrypted records are refused by name.",
                    "A BSMS record is worth keeping for the same reason a descriptor is: it is what recovery needs when the coordinator is gone.",
                ],
            },
            Section {
                heading: "Bitcoin Core",
                paragraphs: &[
                    "Bitcoin Core is not a coordinator with a wallet screen of its own, but it watches a wallet and builds transactions for it once it has been told the descriptor. A wallet's public files offer the Bitcoin Core import format, which is the JSON that `importdescriptors` takes: the wallet's descriptor with its checksum, marked active, over the first thousand addresses of each chain. Bitcoin Core 29 or later takes it as written; an older Core takes the file under More options, Bitcoin Core before 29.",
                    "The file asks Core to scan the whole chain from the first block, which finds coins the wallet already holds and takes a while.",
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
                    "Faraday keeps the scan key in the open vault, or sends it out through the secret sheet, because it is a secret. Export on a silent payments wallet writes it, under More options, as `sp(spscan1q…)`, which is BIP-392's descriptor for exactly this job, and that too goes through the secret sheet.",
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
                    "Check a payment does it for one transaction. Give it the transaction, and the previous transactions of any inputs whose public keys are not in the transaction itself, and the device says which outputs pay this wallet, what they pay, and which label each was paid to. It reads nothing from the network: the transaction, signed, and the ones it spends from have to be copied into Files.",
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
