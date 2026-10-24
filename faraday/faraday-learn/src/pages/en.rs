//! The English Learn pages.
//!
//! `docs/learn/` holds the same text as Markdown, which is where it is
//! edited; `just learn-import` writes it here and `just lint` fails
//! while the two differ (`tools/learn/sync.py`).

use super::{Learn, Page, Section};

/// The Learn pages in English.
pub static EN: Learn = Learn {
    start_here: Page {
        title: "Start here",
        sections: &[
            Section {
                heading: "What the words are",
                paragraphs: &[
                    "A key is a very large random number, written as 12 to 24 words. The words are the key: anyone who reads them can spend the coins, and nobody can give them back once they are lost. They are never photographed, never typed into a website and never read out to anyone.",
                ],
            },
            Section {
                heading: "Plan the backup first",
                paragraphs: &[
                    "Decide where two copies will live before you make a key: two copies, in two places. Before you rely on a copy, check it. The quiz asks for each word from your written copy, and Check a copy of this seed checks it later, by typing it or scanning its SeedQR.",
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
                    "Make a new wallet, or add a key you already have; write the words down and check the copy with the quiz; back the wallet up with its plan; and check one address against your coordinator. To practise on a test chain first, choose Testnet or Signet as the Mode in Settings, or on the mode pill. Choosing a mode locks the device into a fresh session, so choose it before you load anything.",
                ],
            },
        ],
    },
    words: Page {
        title: "Seed words",
        sections: &[
            Section {
                heading: "Why 12 or 24",
                paragraphs: &[
                    "The word protocol is BIP-39: a list of 2048 words, each carrying 11 bits. There are five lengths. 12 words carry 128 bits of key, 15 carry 160, 18 carry 192, 21 carry 224 and 24 carry 256, and each length adds a checksum. 128 bits cannot be guessed, so 12 words are enough; 24 give a larger margin. Faraday makes and reads all five, and every wallet that follows BIP-39 reads them.",
                    "Each word has a fixed place on the list. Tools › BIP-39 words lists the 2048 words, each with its number and its 11 bits.",
                    "Faraday names each loaded key by its fingerprint: eight hex digits computed from the key, the same in every wallet program.",
                ],
            },
            Section {
                heading: "Keys and other secrets from one key",
                paragraphs: &[
                    "BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.",
                    "Tools › Child seeds and passwords asks which application. Words derives another BIP-39 key, which Load as a key adds to the keys loaded like any other. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16 to 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.",
                    "A derived password or WIF is as secret as the key it came from: it reveals nothing about the parent, but whoever holds it holds whatever it protects, and whoever has the parent words can recompute every child. Faraday saves the value into the open vault, sealed under its passphrase; it leaves the vault only after a warning.",
                ],
            },
        ],
    },
    randomness: Page {
        title: "Randomness",
        sections: &[
            Section {
                heading: "Every source asks for trust somewhere",
                paragraphs: &[
                    "A key is only as strong as the randomness it came from. If the process that made your seed words was predictable, whoever knows that process can make the same words, and nothing else protects the funds. A seed made from a favourite quote, a keyboard pattern or a phrase you typed yourself will be cracked.",
                    "New key offers several ways to produce the bits a key is made of: dice or coins, playing cards, camera noise, hex digits, this device's generator, and a mix of several. They are not equally good. The difference is what you have to trust for the result to be random at all. This page says, for each source, what that is.",
                    "None of them can be checked afterwards: a key from a rigged source looks exactly like a key from a fair one. Choosing the source is the only control you have over this. Nothing after the entropy adds randomness: the seed, the master key and every address are computed from the bits you started with.",
                ],
            },
            Section {
                heading: "Dice and coins",
                paragraphs: &[
                    "With a die or a coin, the only thing you trust is the object in your hand. You can see every event, count them, and repeat the whole process with a different die. The device only records what you type. Use a real die and a real coin, not a phone app.",
                    "A die gives about 2.58 bits a roll and a coin one bit a flip. For 12, 15, 18, 21 and 24 words the screen asks for 50, 62, 75, 87 and 99 rolls, or 128, 160, 192, 224 and 256 flips, and counts your entries against the number. The rolls are hashed with SHA-256 and the flips are packed as bits, which is what other signers do, so you can reproduce either result on a computer you trust.",
                    "A die can also stand in for a coin: each roll is one flip, 1, 2 or 3 for tails and 4, 5 or 6 for heads.",
                    "While you enter rolls or flips, Faraday counts how often each face came up and flags long runs, sequential runs such as 1-2-3-4-5-6, and lopsided counts. That check catches a loaded die or a lazy hand. It cannot turn bad rolls into good ones. If it warns you, roll again.",
                ],
            },
            Section {
                heading: "Three ways to read the dice",
                paragraphs: &[
                    "Other signers turn dice into a key in more than one way, and the same rolls give different keys under each. The device asks which one you are following, so that a key you made elsewhere comes out the same here.",
                    "Hashed is what most signers do, including Coldcard and SeedSigner. The rolls are written out as digits and hashed with SHA-256, so `echo -n <rolls> | sha256sum` reproduces the result on any computer.",
                    "Six as zero, hashed writes every 6 as a 0 first and then hashes the same way. Keystone uses it. It is a different convention, not a better one, and a key made under one does not appear under the other.",
                    "Words chosen by the dice does not hash at all. Five rolls of 1 to 4 and a sixth roll read as a coin pick one word out of 2048 directly, because 4 × 4 × 4 × 4 × 4 × 2 is 2048. A 5 or a 6 among the first five is rolled again. This is the BitBox paper table. It costs more rolls: Faraday asks for 72 for 12 words and 144 for 24, against 50 and 99. The result can be checked by hand against a printed wordlist, with no computer in it.",
                    "The last word also carries the checksum, so only its first bits come from the dice, and the device writes the checksum into the rest. The last word of your key is usually not the word the table names for the last rolls.",
                ],
            },
            Section {
                heading: "Playing cards",
                paragraphs: &[
                    "With a deck you trust the shuffle. A deck out of its box is not shuffled at all, and a deck a card trick has been done with may be in an order somebody knows.",
                    "Each draw is worth the log of the cards still in the deck: the first is one of 52, the second one of 51. The counts climb faster than the lengths do: 25 draws for 12 words, 31 for 15, 39 for 18, 50 for 21 and 58 for 24. More than 52 needs a second, freshly shuffled deck, which the device asks for. It refuses a card that is already out, because a card entered twice adds nothing and usually means a misread one.",
                    "The entropy is SHA-256 over the cards' numbers, in the order you drew them.",
                ],
            },
            Section {
                heading: "Camera noise",
                paragraphs: &[
                    "The sensor in a camera produces a slightly different picture every time, even of the same still scene, because of electrical noise in the sensor itself. The device hashes the pixels of the frames you take: three frames for 12 words, four for 15, five for 18 and six for 21 and 24.",
                    "With the camera you trust the sensor and the software that delivers its frames, which you cannot inspect as you can a die. A frame the camera was given rather than one it took would produce a key somebody else can produce as well, and nothing on screen would look wrong.",
                    "The device refuses a frame with fewer than 32 distinct brightness values, which is what a covered lens or a blown-out picture gives. It states the frame's size, distinct values, mean and variance as facts, not as a verdict.",
                ],
            },
            Section {
                heading: "Hex",
                paragraphs: &[
                    "A key's entropy is sometimes written down as hex digits rather than words. New key takes hex digits made elsewhere as its source: the digits become the key as they are, so they must already be random.",
                ],
            },
            Section {
                heading: "This device",
                paragraphs: &[
                    "Every computer has a random number generator, built from a hardware source plus the operating system. It is usually fine. Using it for a key asks you to trust this device completely: a generator that is faulty, or built to be predictable, produces keys that look perfectly normal and that its maker can recreate. You can check a die. You cannot check a chip.",
                    "The row names what the device reads. The stick and the Pi read /dev/random, the Linux kernel's generator, which mixes the board's hardware generator with its other sources. On the Pi the board's hardware generator feeds the kernel; the device does not read it directly, so a faulty chip is one source among several. If the read fails, the device makes no key and says so on Home.",
                    "Faraday Desktop and the Starter read the operating system's generator: getrandom(2) on Linux, ProcessPrng on Windows and getentropy(2) on a Mac. Each waits until the generator is ready.",
                    "The same generator also seeds everything else the device draws at random, whichever source made the key: a vault's keys and seal nonces, GPG and Secure Boot keys, MuSig2 and FROST nonces, a new keyboard's code and the blinding of the device's own calculations. In the website demo, the key made is replaced by one of its public demo keys, whichever source made it.",
                ],
            },
            Section {
                heading: "Mixing",
                paragraphs: &[
                    "A mix runs two to five sources in turn and combines them. The result is at least as good as the best source in it: an attacker who controls the camera but not your dice learns nothing, because the dice are still in the hash.",
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
                    "A backup is your seed words written down and kept away from every computer. If you have a passphrase, the passphrase is part of the backup too, stored separately.",
                    "Paper is enough against forgetting. Steel, stamped or punched with the words, is what survives fire and flooding.",
                ],
            },
            Section {
                heading: "What destroys a backup",
                paragraphs: &[
                    "Fire, water, damp, fading ink, a house move, a relative tidying up. Two copies in two places are far better than one perfect copy in one place. The backup plan lists the places and what each holds.",
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
                    "Nothing stores the passphrase unless you keep it in a vault. It cannot be reset, recovered, or guessed. If you forget it, the seed words alone lead to a different, empty wallet. A lost passphrase loses the funds under it for good, whatever backup of the words there is.",
                ],
            },
            Section {
                heading: "Rolling one with dice",
                paragraphs: &[
                    "A passphrase you make up yourself is weaker than it looks: song lines, names with digits after them and letter substitutions are in the lists a cracking program tries first. Words chosen by dice are not in any such list.",
                    "Create a vault rolls one for you, under Generate with dice. You pick a list and how many words, and you throw a die. Five throws name one word of the long list, four throws name one word of a short list. The device only looks the words up.",
                    "The lists are the ones the Electronic Frontier Foundation publishes, and Tools › Word lists shows each of them. The long list has 7776 words, 12.9 bits a word; the short lists have 1296, 10.3 bits a word. Six words from the long list are 77.5 bits, which is more than offline guessing reaches. Vaults states the strength of the one you choose.",
                    "A rolled passphrase is text like any other. Create a vault uses it as the vault's passphrase; rolled the same way, it can be the BIP-39 passphrase on a key.",
                ],
            },
            Section {
                heading: "Vanity addresses",
                paragraphs: &[
                    "Tools › Vanity address tries one candidate after another until the first address of the account begins with the characters you asked for. Each free character costs 32 tries on a bech32 address and 58 on a base58 one, and the device states the rate it manages and how long that takes.",
                    "There are two dials. The passphrase counter appends characters to the key's BIP-39 passphrase, so every candidate is a different key and costs a full derivation; taking the find loads that key beside the one you started from. The account index steps through the accounts of the key you already have, which is far cheaper and adds a wallet rather than a key.",
                    "A vanity address is not safer, and a familiar-looking string is one an attacker can imitate: never check an address by its first characters alone. The passphrase dial leaves you a passphrase to back up, since the words alone no longer reach the funds, and a counter written down nowhere is a key lost. The counter is the one other tools use, in the same order, so a find here is a find there.",
                ],
            },
            Section {
                heading: "Storing it",
                paragraphs: &[
                    "Write it down exactly, spaces and capitals included, and read it back before you use it for real. Keep it apart from the seed words, so that whoever finds one does not have both. That separation is the point of a passphrase: a stolen backup of the words alone is useless.",
                    "A vault can keep a seed's passphrase with the seed, and unlocking the vault then loads the key the two give. Kept that way, the passphrase is behind the vault's passphrase rather than apart from the words, and whoever opens that vault has the key. A passphrase kept in a vault belongs in a different vault from its seed, at another place. Faraday's backup plan warns when a seed and its passphrase are at the same place. A seed kept in a vault without its passphrase loads with Load with a passphrase, which asks for it each time and stores nothing.",
                    "Whoever is meant to inherit the funds must find both halves. Seed words without the passphrase open a different, empty wallet that looks correct, and an heir who has only the words will conclude that the backup is broken and stop looking. See the Inheritance page.",
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
                    "Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this, including by swapping an address as you copy it.",
                    "An address shown on Faraday was derived on Faraday, from your key, on a device with no network. It is the one thing in the process you can trust.",
                ],
            },
            Section {
                heading: "What a bad coordinator can do",
                paragraphs: &[
                    "The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.",
                    "The device therefore trusts nothing it is sent. A wallet is shown in full before it is accepted, and you accept it on this screen. A transaction is shown with every output and its amount, and an output counts as your change only when the device derives its address from a wallet you accepted. An output it cannot account for is shown as a payment.",
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
                    "Before you sign, Faraday shows what the transaction sends, the change and the fee. Read every output. Confirm that the destination address and amount match what you intended. Change that Faraday rebuilt from the wallet's own keys is marked Verified; an output that Faraday cannot confirm as yours is not yours, and if it cannot derive a change output, treat that output as a payment to somebody else.",
                    "The Nonce check lists every signature the transaction already carries, with the key it is under and whether it verifies. Compare every address and amount in full with what your coordinator shows. If anything differs, do not sign.",
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
                    "Out of Faraday: a public key for your coordinator, a signed transaction, a signed message, a vault, GPG certificates and signatures, and Secure Boot enrolment files and signed EFI images. Your seed words and private keys leave only sealed in a vault, or unprotected when you ask for that and confirm the warning; word and SeedQR files leave after a warning.",
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
                    "The same goes for a message that says your wallet must be validated, synced, migrated or upgraded, and links to a page with 12 or 24 boxes on it. The boxes send your words to whoever made the page. Real software never asks you to enter your seed words on a website.",
                ],
            },
            Section {
                heading: "Fake wallet software",
                paragraphs: &[
                    "A wallet downloaded from a search advert, an app store clone, or a link in a forum post builds transactions that pay someone else, or simply uploads your seed words. Get your coordinator from the project's own website and check the download's signature before running it. Check Faraday the same way: each release lists its files' SHA-256 sums in SHA256SUMS, signed with the release key.",
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
                    "A multisig wallet is controlled by several keys, and a transaction is valid only when a set number of them have signed. In a 2-of-3 wallet any two of the three keys can move the funds. Each key has its own seed words, ideally generated on a different device and stored in a different place.",
                ],
            },
            Section {
                heading: "What it protects against",
                paragraphs: &[
                    "One stolen key cannot spend. One destroyed backup cannot lose the funds. A single-key wallet fails on either of those, and a multisig survives both. It does not protect against a wrong address, a scam, or a wallet description that nobody wrote down.",
                ],
            },
            Section {
                heading: "What it costs",
                paragraphs: &[
                    "Every key needs its own backup, so a 2-of-3 wallet means three sets of seed words in three places. On top of that, recovery needs the wallet description: which keys are in the wallet, the script type, the derivation path, and how many signatures are required. With every key in hand and the script type and path known, a lost description can be rebuilt; Faraday's Restore does this. With only a quorum of the seeds, two of the three, it cannot: the third key's xpub is in the description and nowhere else, so the addresses cannot be worked out and the funds cannot be found.",
                    "More parts means more to keep, more to explain to an heir, and more ways to lock yourself out of your own arrangement. Kinds of wallets says when a different arrangement fits better.",
                ],
            },
            Section {
                heading: "Using multisig with Faraday",
                paragraphs: &[
                    "Make a new wallet makes a multisig from keys loaded here and cosigners' xpubs. Restore an existing wallet reads one a coordinator made: a descriptor, a wallet file, a multisig config or a BIP 129 record, by QR code or from a file. Faraday shows every key in the wallet and which of them are loaded here.",
                    "Once loaded, Faraday recognises the addresses and change of that wallet, so a transaction you sign is checked against the wallet you loaded. A wallet is kept for the session; saved into a vault, it comes back when the vault is unlocked.",
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
                    "A timelock written as older counts from the coins arriving. \"Key B after 52 560 blocks\" means that particular coin must have been sitting in the wallet for that many blocks before key B can move it. The count is per coin and starts when the coin was received, not when the wallet was made and not when you last used the device. If you spend and the change comes back, the change is a new coin and its clock starts again. A recovery plan that assumes otherwise fails at the moment it is needed.",
                    "A timelock written as after is a point on the chain, not a wait: a block height the chain must reach, or a date its clock must pass. It is the same moment for every coin in the wallet.",
                    "Blocks are not minutes. Ten minutes a block is the average the network aims for, not a promise, so the durations the review shows beside a block count are approximate and are said as such.",
                ],
            },
            Section {
                heading: "Who chooses the path",
                paragraphs: &[
                    "The coordinator builds the transaction, and the transaction is what picks the path: the sequence and locktime fields it sets, and the signatures it collects. This device does not choose. It reads what arrived, shows you the wallet and its paths, and signs what you approve.",
                    "So a spend path you can see in the review is not a spend path that will work today. Whether the wait has passed is a fact about the chain, and this device is not connected to it. Tools › Miniscript compiler writes a policy of your own as a descriptor; the Tools page describes it.",
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
                    "Faraday shows it as a QR code or writes it as a file, together with the fingerprint and derivation path the coordinator needs to use it: Show xpub QR and Xpub file, on a key of the wallet's chart. A wallet's public files carry the account keys it is built on. Tools › Explore a key shows a key's extended public key at any path. The same key is written xpub, ypub or zpub to say which script type it is for; Tools › Convert an xpub shows every spelling.",
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
                heading: "Who should have your xpub",
                paragraphs: &[
                    "Your own coordinator, on a machine you control. The other members of a multisig, who cannot build the wallet without it. Nobody else, unless you are content for them to read the whole account: not a support chat, a forum post, a block explorer, or a website offering to check your balance. What you hand over cannot be taken back.",
                ],
            },
        ],
    },
    inheritance: Page {
        title: "Inheritance",
        sections: &[
            Section {
                heading: "What an heir needs",
                paragraphs: &[
                    "If you die or become incapacitated, your funds are gone unless someone else can recover them. There is no company to contact and no court that can order the funds released.",
                    "An heir needs the seed words, the passphrase if there is one, and enough about the wallet to open it: the script type and the derivation path, or for a multisig, the whole wallet description and enough of the keys. They also need instructions written for somebody who has never done this: which software works, and what the first step is. Assume they know nothing.",
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
                    "For a single key, Faraday adds no randomness of its own. The nonce is calculated from the key and the transaction alone, using RFC 6979 for ECDSA signatures and BIP-340 with no auxiliary randomness for Schnorr signatures. The same transaction signed with the same key always produces exactly the same signature bytes. As a check on itself, Faraday signs every input twice and refuses to continue if the two results differ.",
                    "A MuSig2 round that goes first, and a FROST signing, are different: their nonces are drawn from the session seed, which comes from the system's generator, and are never reused across signing passes.",
                ],
            },
            Section {
                heading: "Which nonce rule",
                paragraphs: &[
                    "There are two standard ways to derive an RFC 6979 nonce. \"Low R\" retries the nonce, with a counter added as the RFC's extra data, until the signature comes out in its shortest form. \"First\" stops at the first nonce. Faraday signs with Low R, for transactions and messages alike, and has no setting for it.",
                    "Low R is what Bitcoin Core, Sparrow, and Electrum produce. First is what the RFC's own test vectors, the BIP-174 test vectors, and Trezor produce. Bitcoin Core signs messages without the retry, so a Core message signature does not match Faraday's byte for byte.",
                    "For taproot, Faraday adds no randomness to a single-key signature, so it can be compared against another implementation that does the same. BIP-340 recommends 32 fresh bytes in every signature as a defence against fault and side-channel attacks; a signature made that way cannot be reproduced.",
                ],
            },
            Section {
                heading: "Checking Faraday against another signer",
                paragraphs: &[
                    "Because a single-key nonce is deterministic, you can verify that Faraday is not hiding anything in it. Load the same seed words into a second implementation that signs with Low R, and for taproot with no auxiliary randomness, on another offline machine, sign the same transaction on both, and compare the signature bytes. They must match exactly. A device that was leaking your key through its nonces would fail this comparison.",
                ],
            },
            Section {
                heading: "What is not offered yet",
                paragraphs: &[
                    "There are protocols in which the coordinator contributes randomness to the nonce and then checks that the signer did not choose the nonce alone (anti-exfil). Faraday does not implement one yet, because no coordinator currently speaks such a protocol over QR codes or PSBT files.",
                    "MuSig2 exchanges nonces before anyone signs, and Faraday signs in either order. When every other signer's nonce is already on the transaction, it derives its own from theirs and from the transaction, signs in one pass, and keeps nothing. When a nonce is missing, it goes first: it draws its nonce, writes the public half onto the transaction for the coordinator, and holds the secret half in memory as a session for that one transaction. The session ends when the device signs, when you lock it, and when it loses power, and nothing writes it to storage, because a secret nonce that survives a restart is the classic way to use one twice, and using a MuSig2 nonce twice hands over the private key. If the transaction comes back after the session has ended, the device draws a new nonce, says so, and the other signers sign again. The FROST page describes the one place Faraday writes a secret nonce to a file.",
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
                    "That whoever made the signature holds the key behind that address, and that this exact text was signed with it. It moves no funds and says nothing about what the address holds.",
                    "Anyone can copy the three lines and pass them on; only the key can make a new one. If someone wants to prove something to you, ask for a signature over text that you chose, including the date, and treat a ready-made signature as proof of nothing.",
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
                heading: "Identifier",
                paragraphs: &[
                    "Four characters in boxes 5 to 8 of a codex32 string, the same in every share of one set. They name the set and are not secret.",
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
                    "Which chain a key and its addresses belong to: mainnet, testnet, signet or regtest. Funds on one network do not exist on another. Settings chooses it as the Mode: Demo, Mainnet, Testnet, Signet or Regtest.",
                ],
            },
            Section {
                heading: "NUMS point",
                paragraphs: &[
                    "A public key chosen so that nobody knows a private key for it. It is used as a Taproot internal key when a wallet is meant to be spendable only by its scripts.",
                ],
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
                heading: "Share",
                paragraphs: &[
                    "One of the pieces a key or a description is split into. A FROST wallet's key is split into n shares held as 24 words, any m of which sign; SLIP-39 and codex32 split a seed into shares; a backup can split a multisig's description into shares. One share alone can do nothing.",
                ],
            },
            Section {
                heading: "Share index",
                paragraphs: &[
                    "The character in box 9 of a codex32 string: s for the secret itself, another character for each share of the set.",
                ],
            },
            Section {
                heading: "SLIP-39",
                paragraphs: &[
                    "Trezor's backup scheme: Shamir's Secret Sharing over a word list of its own, with a threshold. Its shares are not BIP-39 seed words.",
                ],
            },
            Section {
                heading: "Translation symbol",
                paragraphs: &[
                    "In codex32, the number from 0 to 31 an input string is multiplied by, character by character, when a share or the secret is made from k strings. The codex32 booklet writes it in a second alphabet of Greek letters and signs.",
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
                    "The standalone calculators in Tools take what you type, work one thing out from it, and keep nothing. They hold no key of their own, and an extended private key typed into one is refused. Two go further: Decode reads the wallets you have loaded, and the Lightning tool reads a node's seed and can save the node's private key into the open vault.",
                ],
            },
            Section {
                heading: "Hashes",
                paragraphs: &[
                    "SHA-256, SHA-256 applied twice, and RIPEMD-160 of SHA-256, which Bitcoin calls HASH160. These three are what Bitcoin hashes with: a transaction id is SHA-256d of the transaction, and the 20 bytes inside a legacy address are HASH160 of a public key.",
                    "The field is read as hex when every character is a hex digit and there is an even number of them, and as text otherwise. The mode row forces one or the other.",
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
                    "One extended public key has several spellings. BIP-32 writes it as xpub or tpub; SLIP-132 writes the same key as ypub, zpub, upub or vpub to say which script type it is meant for: xpub for legacy, ypub for nested SegWit, zpub for native SegWit. The key material is identical; only the four version bytes differ.",
                    "Give it any of them and the tool shows all of them, with the network, the depth, the fingerprint and the child number the key states about itself.",
                ],
            },
            Section {
                heading: "Miniscript compiler",
                paragraphs: &[
                    "Tools › Miniscript compiler compiles a policy into a descriptor, and says whether this device loads it. Where a policy names a key, the fingerprint of a key this device already holds stands for that key's account key, so pk(73c5da0a) is accepted and needs no xpub typed. The Keys row above the field lists the fingerprints it will take; with no key loaded there is no such row and every key must be written out.",
                ],
            },
            Section {
                heading: "Units",
                paragraphs: &[
                    "One bitcoin is 100 000 000 sats. Type an amount in bitcoin, millibitcoin, bits or sats and the other three follow.",
                ],
            },
            Section {
                heading: "Decode a transaction",
                paragraphs: &[
                    "Decode shows the review that Sign a transaction shows, for a transaction that is only being read: what it spends and pays, field by field. The wallets you have loaded are used to read it, so change of a wallet in use is recognised as it is when you sign. Nothing is signed.",
                    "Use it to read what a coordinator produced before you sign it anywhere, or to see what a raw transaction from a block explorer actually pays.",
                ],
            },
            Section {
                heading: "Lightning node key",
                paragraphs: &[
                    "A Lightning node has an identity key, and a backup of that node is either an LND cipher seed — twenty-four words that are not a BIP-39 phrase — or the BIP-39 words an ldk-node wallet was built from. The tool reads either one and says which node it is: the public key the network knows the node by, and for a cipher seed the version and the birthday it carries.",
                    "Use it to confirm that a backup in your hand belongs to the node you think it does, before you restore it anywhere. A node key is hot by nature, on a machine that is online and in use all the time, so Faraday signs nothing with it. The node's private key is a secret: it goes into the open vault, or out unprotected only after a warning.",
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
                    "It is the cheapest arrangement to hold and to recover: seed words, a script type, and a derivation path. It is also the one where a single stolen backup is a total loss.",
                ],
            },
            Section {
                heading: "Multisig",
                paragraphs: &[
                    "The wallet names n keys and requires m of them to sign. A 2-of-3 survives one stolen key and one lost backup. Spending reveals the script and puts m signatures on the chain, so anyone reading the block sees that it was a 2-of-3 and sees the three keys. Every coordinator supports this arrangement. The Multisig page covers this kind in full.",
                ],
            },
            Section {
                heading: "Taproot multisig",
                paragraphs: &[
                    "The same m-of-n written as a Taproot script path. Unspent, the address looks like any other Taproot address and says nothing. Spending reveals the one script that was used and nothing else, and the transaction is smaller than the equivalent classic multisig.",
                    "When the wallet has no way to spend with a single key, the Taproot internal key is set to a NUMS point: a public key chosen so that nobody knows a private key for it. The device shows this in the review, because an internal key that has not been shown to be unspendable is a way to spend the wallet that you never agreed to.",
                    "Make a new wallet builds this kind: Taproot multisig under More kinds. Fewer coordinators write it than write classic multisig.",
                ],
            },
            Section {
                heading: "MuSig2",
                paragraphs: &[
                    "All n keys together produce one public key and one signature. The chain sees a single-key Taproot spend: no threshold, no key count, nothing about the arrangement. It is BIP-327, written in a wallet with the musig() key expression of BIP-390.",
                    "Signing is interactive and takes two rounds. Every signer publishes a nonce, then every signer produces a partial signature over the collected nonces, and the partials add up to one signature. Every participant must take part; there is no m of n. A nonce must never be used twice, which is why the device holds one only in memory and only for the one transaction it was drawn for.",
                    "Use it when all the signers are reachable and what the chain reveals matters; not where one signer may be unavailable.",
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
                    "A wallet can name one key that spends at any time and another that spends alone after a wait. That is two spend paths in one script, and it is how Liana's wallets are shaped. It gives you a recovery that needs no second person on the day, and an inheritance an heir can execute by waiting. It costs a timelock whose clock restarts every time the coins move, so the wallet has to be refreshed or the recovery path opens while you are still using it. Spend paths and timelocks has the details.",
                    "Make a new wallet does not offer this kind; Faraday reads one from its descriptor, including one with more than one recovery path, and names every path and every wait. Tools › Miniscript compiler writes a policy of your own as a descriptor.",
                ],
            },
            Section {
                heading: "Silent payments",
                paragraphs: &[
                    "A silent payments wallet publishes one address that never changes and never reuses a payment. The payer works out a fresh taproot output that only you can find, so nothing on the chain links two payments to the same address. It is built over one key, and a wallet has to scan the blocks to find its payments. This device checks one transaction at a time and exports the scan descriptor; it does not send to a silent payment address. Silent payments has the details.",
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
                    "A 2-of-3 multisig announces itself in every spend; a 2-of-3 FROST wallet is indistinguishable from one person with one key.",
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
                    "The shares exist together on one device for the length of that flow: that is what a dealer is.",
                ],
            },
            Section {
                heading: "Signing in two places",
                paragraphs: &[
                    "Signing takes two rounds, as MuSig2 does: every signer publishes a nonce, then every signer produces a partial signature over the collected nonces. When the signers are not in the same room, the rounds are carried in a file.",
                    "At the first place you choose which other signers will take part, and the device draws every signer's nonce, signs with the share it holds, and saves a carry file. At the next place the carry file is read. The device checks that each stored nonce is the one the transaction already names, that the file is bound to this transaction (its signature hash) and to this set of signers, and that every earlier partial signature verifies. Only then does it sign, add the partials together, and finish the transaction. Any mismatch is refused by name.",
                    "The carry file holds the transaction and one secret nonce per signer still to sign. It names no device: any device that holds a signer's share and the group record can continue it. It is a file you keep to yourself and carry between the places, not one you publish or hand to a coordinator.",
                    "Writing a secret nonce to a file is safe here because both the nonce and the message were fixed by the device that drew them. Reusing a nonce leaks a share only when the same nonce signs under two different challenges, and any change to the nonce or the transaction breaks a check the device makes by arithmetic. A copy taken in transit is a random number with nothing to pair it with, because the signature made under it never leaves the device.",
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
                    "A share's 24 words are valid seed words. Loaded as an ordinary key, with no group record in sight, they open a working single-sig wallet that you may keep funded. They act as a member of a FROST wallet only after you have also loaded the group record that lists their public share; nothing in the words shows which wallet they belong to.",
                ],
            },
            Section {
                heading: "What it is not",
                paragraphs: &[
                    "FROST is not a backup scheme: shares do not reconstruct your other wallets. It is also newer than multisig and understood by less software. A coordinator that cannot read the group record sees a single-key Taproot wallet, which is enough to watch it and build transactions, and not enough to know who has to sign.",
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
                    "Words on paper or steel is the backup almost everybody should make first. The forms on this page answer the two problems a single written copy has: it can be destroyed, and it can be found and read. Each trades something away. If you do not fully understand a scheme, you are safer with a single written copy.",
                ],
            },
            Section {
                heading: "SLIP-39",
                paragraphs: &[
                    "SLIP-39 is Shamir's Secret Sharing over a word list, published by Trezor. It splits a master secret into shares with a threshold, so any 3 of 5 shares rebuild it and two do not, and it can nest that once: groups of shares, with a threshold over the groups as well.",
                    "Its words are not BIP-39's. The list is a different one, a share is a different length from a seed phrase, and a share cannot be typed into a wallet that expects seed words. A SLIP-39 backup and a BIP-39 backup of the same wallet are not interchangeable: you choose one at the moment the key is made.",
                    "Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. Faraday reads it, groups and all, and writes it. Add a key › Type SLIP-39 shares loads the key a backup's shares hold, under its passphrase if it has one. Add a key › Make SLIP-39 shares makes a new key and writes it as one group of shares: you choose how many shares there are and how many are needed, and Faraday shows every share in turn with the quiz. A key made of BIP-39 words is not backed up as SLIP-39 shares: a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation takes.",
                ],
            },
            Section {
                heading: "Codex32",
                paragraphs: &[
                    "Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1. Box 4 of the string is the threshold, boxes 5 to 8 the identifier, box 9 the share index, and the last 13 boxes (15 at 127 characters) the checksum. A string of 48 characters holds a 128-bit seed, one of 74 characters a 256-bit seed, and one of 127 characters a 512-bit seed.",
                    "A codex32 seed can stand alone, with threshold 0, or be split into shares, k of n, with k from 2 to 9 and up to 31 shares. Any k shares of one set rebuild the seed; fewer than k tell nothing about it. The shares of one set carry the same threshold, identifier and length, and each its own index.",
                    "The checksum and the share arithmetic were designed to be worked by hand. The codex32 booklet, Shamir Secret Sharing Codex, has the worksheets, tables and paper wheels to make a seed from dice, put its checksum on it, derive shares and recover the secret, on paper and without a computer. The booklet and its single worksheets are at https://secretcodex32.com/docs/index.html. The cost is time and care: a mistake in the arithmetic is silent until the checksum catches it.",
                    "The guide to read beside this page is the codex32 user flow guide of SeedSigner's codex32 branch, which reads codex32 strings and Codex32QR codes, checks them and offers corrections: https://github.com/FractalEncrypt/FractalEncrypt_seedsigner/blob/codex/codex32-error-correction/docs/codex32_qr/Codex32_User_Flow_Guide.md",
                ],
            },
            Section {
                heading: "Making codex32 shares in Faraday",
                paragraphs: &[
                    "New key › Codex32 makes a key as the booklet does, every step on screen. Scheme sets the length, 48 or 74 characters, and the split: 2 of 3, 3 of 5, Other (a threshold of 2 to 9 and up to 31 strings), or None, the secret alone. Identifier takes four characters, typed before any entropy as the booklet orders it; Suggest fills four from this device's randomness. The identifier is not secret. One already used by the strings typed in Add a key is refused.",
                    "The entropy is your own. Roll dice follows the booklet's dice de-biasing worksheet: five dice are rolled twice; a die whose second value is lower than its first gives 0, one whose second value is higher gives 1, and a die with two equal values is rolled twice again. The five bits are one character, die 1's bit first. Type the characters takes characters made on paper, from dice or a worksheet. Characters a person makes up are not random, and a share built from them can give the seed away to someone holding fewer than k strings. Faraday's own randomness does not make the seed here.",
                    "The k initial shares are the entropy: their data boxes are the characters rolled or typed, unchanged. The other shares and the secret are derived from them. As each character is entered it drops into its box, and the checksum worksheet and the translation worksheets fill with it. Every cell opens to its working: the rolls that made it, the two cells added, the table row looked up.",
                    "After the last character, Worksheets shows each sheet whole, one tab per sheet, in the booklet's order: the dice and the checksum of each initial share, each derived share, the secret. Strings shows each string to write down and asks for it typed back; a string typed back that differs names the first box that differs. Strings made lists the split, the length, the identifier and the fingerprint. Check on another device lists sets of k strings to load on another implementation: every set must give the same fingerprint. Lock in loads the key.",
                    "On the stick and the Pi the sheets are shown while the flow is open and masked when it is left; the eye masks and shows them at any time. Faraday Desktop does not mask them: it is online, and a seed that holds real bitcoin is never made there.",
                ],
            },
            Section {
                heading: "The codex32 worksheets",
                paragraphs: &[
                    "The checksum worksheet adds the string two characters at a time and looks up the next 13 symbols in the checksum table, row by row, as the booklet prints it. Making a string, its final row is fixed and the checksum boxes are worked from the bottom up. Checking a string, every box is given and the final row is worked out: the string is valid when that row reads SECRETSHARE32, or SECRETSHARE32EX at 127 characters.",
                    "A translation worksheet makes one string from k others. Each input string has a translation symbol, a number from 0 to 31 that the booklet writes in a second alphabet of Greek letters and signs. Each input is multiplied by its symbol, character by character, and the products are added. For a derived share the symbols come from the booklet's derivation table. For the secret, each string's recovery symbols, read off the recovery wheel or table, are multiplied together, which the booklet calls fusion.",
                    "The booklet publishes checksum worksheets for 48 and 74 characters, and translation worksheets only for 48 characters at k = 2 and k = 3. For k of 4 or more, and at 74 characters, Faraday's worksheets on screen are the only worked form; each cell can still be checked by hand with the wheels or tables. No sheet exists for 127 characters.",
                ],
            },
            Section {
                heading: "Codex32 strings in",
                paragraphs: &[
                    "Add a key › codex32, also New key › Type codex32 strings, takes one string at a time, typed or scanned, and checks each at once. A string whose checksum fails is not taken, and is named by its index. A string with another identifier is refused. The same string again is already in. A different string at an index already in is shown beside the one held, with the boxes that differ, to keep one or the other. Once k strings of one set are in, the key loads.",
                    "Check a string, on the same form, says whether one string's checksum holds, what the string is, and whether it is of the set typed in. It loads and keeps nothing.",
                    "Suggest corrections, off whenever the form opens, adds ? to the keyboard for a box that cannot be read. A failing string is then decoded as BIP-93 allows: up to 4 changed boxes, or 8 unreadable ones, or a mix in which twice the changes plus the unreadable boxes come to 8 at most. A correction is shown only when it is the only one within that bound, and it is never taken as it stands. I mistyped clears the changed boxes, to be typed again from the paper. The paper is damaged gives the repairs to write on the paper, then the same typing. Unreadable on the paper takes the correction for boxes marked ?. From then on the key carries Share entry corrected or Share repairs unchecked beside its fingerprint.",
                    "A Codex32QR code is one 48-character string in a QR code, as SeedSigner's codex32 branch defines it. Faraday reads one wherever a codex32 string is scanned, and shows one for each 48-character string it makes. No code is defined for longer strings.",
                ],
            },
            Section {
                heading: "Codex32 backups",
                paragraphs: &[
                    "A backup's copies by hand write a key held as codex32 strings: none, the secret alone, or a threshold of 2 to 9 of up to 31 strings, the random shares drawn from this device. For a key made of BIP-39 words the strings hold the 512-bit seed those words come to, with the passphrase already in it, at 127 characters: a wallet that expects words cannot read them back, and they are not for checking by hand. A key loaded from a master seed gets strings of its own seed's length.",
                    "In the backup plan a seed can be kept as codex32 strings, each string placed on its own. A place with k strings of one set holds the seed. A key made as a codex32 set keeps the set's facts (threshold, identifier, length, indices; no strings), and its backup places that set rather than splitting the seed again. Add a share, Replace a lost share and Rotate the set are the set's actions.",
                ],
            },
            Section {
                heading: "Single key, backup in shares",
                paragraphs: &[
                    "Make a new wallet offers Single key, backup in shares, between Single key and Multisig. Its Quorum is the codex32 scheme, how many shares rebuild it and how many there are. Its one key is made by Make codex32 shares.",
                    "On the backup side it gives a single-key wallet much of what multisig gives: no one paper or place loses the wallet, and fewer than k papers reveal nothing. At spending it does nothing multisig does. The wallet is single-key on chain, native SegWit, one xpub to the coordinator, and to sign, the whole key is on one device. Whoever holds k shares can spend alone.",
                    "Codex32 shares are made only for single-key wallets. A slot of a multisig, Taproot multisig, MuSig2, FROST or recovery wallet offers BIP-39 words to make a key. A codex32 key already loaded can fill any slot.",
                ],
            },
            Section {
                heading: "Codex32 sheets from the Starter",
                paragraphs: &[
                    "Blank templates in Faraday Starter print codex32 sheets with Form set to codex32. Length is 48, 74 or 127 characters. Copy is String, Codex32QR or Both, the last two at 48 only. Worksheets is None, or Dice, checksum and translation: at 48 characters the dice sheet, the checksum sheet and the translation sheets for k = 2 and k = 3; at 74 the dice sheet and the 256-bit checksum sheet; none at 127. A share sheet has ms1 printed in boxes 1 to 3, boxes for the threshold, the identifier and the share index, every box numbered, a box for the fingerprint, and the alphabet with 0, 7, S and Z slashed as handwriting should be. The sheets print at true size and hold nothing until written on.",
                    "Faraday draws these sheets itself, in the booklet's layout and numbering, from BIP-93's arithmetic. The booklet's notice, at the foot of this page, is kept here and is not printed on the sheets.",
                ],
            },
            Section {
                heading: "Seed XOR",
                paragraphs: &[
                    "Seed XOR splits one key into two or more parts. Each part is a full set of seed words of the same length as the original, and combining the parts with the XOR operation gives the original key back. It is Coldcard's scheme: parts made here combine on a Coldcard, and parts made on a Coldcard combine here.",
                    "All but the last part are random, and the last is worked out so that the parts XOR back to your key. Every part is needed: it has no threshold, two parts of a three-part split are worth nothing, and losing one loses the key. In exchange it is simple enough to do with pencil and paper from the wordlist numbers, and any wallet can read a part.",
                    "Each part is also a valid set of seed words, so each is a working wallet with addresses of its own. Coins sent to an address derived from a part are held by that part alone, not by your key. Treat every part with the same care as the original words.",
                    "In Faraday, a backup splits a seed among its copies by hand into two or three parts, with the random parts drawn from the device's own generator. Add a key › Type Seed XOR parts combines them into the key that already existed.",
                ],
            },
            Section {
                heading: "A vault",
                paragraphs: &[
                    "A vault is a file on a stick holding your words encrypted under a passphrase you choose. It is not a split: it is one copy that can be stored where a readable copy could not. Its weak point is the passphrase, now a second thing to keep and a second thing to lose. Vaults has its own page.",
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
                heading: "The codex32 booklet's notice",
                paragraphs: &[
                    "The codex32 booklet, whose worksheets Faraday's codex32 sheets follow, carries this notice.",
                    "MIT License",
                    "Copyright (c) 2022 Blockstream",
                    "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:",
                    "The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.",
                    "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.",
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
                    "A coordinator is the wallet software on your online machine: Sparrow, Liana, Nunchuk, Specter, Bitcoin Core and others. It holds no keys. It watches addresses, shows balances and history, picks which coins to spend, sets the fee, builds the transaction as a PSBT, and broadcasts it once it is signed.",
                    "Faraday is the other half: it holds the keys and signs. Everything that crosses between the two is one of a small number of files, by QR code or on a stick (on the Pi, the SD card). This page names them.",
                ],
            },
            Section {
                heading: "What the coordinator sends",
                paragraphs: &[
                    "A descriptor is one line of text describing a whole wallet: the keys, the script type and the derivation paths, with an eight-character checksum. It is the most portable of these, and the one to keep with your backup.",
                    "A wallet policy is the same wallet split into a template and a list of keys, which is BIP-388's form. A multisig configuration file is a plain text list of the keys and the threshold, in the shape Coldcard introduced. Both carry the same facts as a descriptor in a different layout. A BSMS record does the same job with a setup ceremony around it, below.",
                    "A PSBT, a Partially Signed Bitcoin Transaction, is the transaction itself: the inputs, the outputs, the amounts, and the key information needed to sign, with room for each signer to add a signature.",
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
                    "Bitcoin Secure Multisig Setup, BIP-129, is a procedure for building a multisig wallet without trusting the coordinator to report the keys honestly. Each signer produces its key record, the coordinator assembles them into a descriptor record, and the record goes back to every signer to be confirmed and stored before any coins are sent. Every signer ends up holding the same wallet description, checked on its own screen.",
                    "Faraday writes a key record for each multisig key held here, under a wallet's public files, For the cosigners: the signed xpub record of BIP 129, signed by the account key itself, as text and as a code. It reads a descriptor record wherever it reads a wallet: Scan, Files, and Restore an existing wallet. It checks the wallet's first address against the one the record states, and shows the quorum, the derivation paths the record restricts the wallet to, and which keys are loaded here. A multisig wallet's public files write the descriptor record back out. Encrypted records, which are hex rather than text, are refused by name. A record with a token in it is still plain text, so carry it by QR code or memory card, not through the coordinator.",
                    "Keep a BSMS record for the same reason as a descriptor: it is what recovery needs when the coordinator is gone.",
                ],
            },
            Section {
                heading: "Bitcoin Core",
                paragraphs: &[
                    "A wallet's public files offer the Bitcoin Core import format, which is the JSON that `importdescriptors` takes: the wallet's descriptor with its checksum, marked active, over the first thousand addresses of each chain. Bitcoin Core 29 or later takes it as written; an older Core takes the file under More options, Core before 29. The file asks Core to scan the whole chain from the first block, which finds coins the wallet already holds and takes a while.",
                    "Save the file, carry it to the machine running Core, and run `bitcoin-cli -rpcwallet=<name> importdescriptors \"$(cat <file>)\"` on a wallet created with `createwallet` and private keys disabled. This file is the public wallet and nothing else.",
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
                    "What the payer does with it is arithmetic. They take the two public keys the address carries, combine them with the public keys of the coins they are spending, and work out a fresh taproot output that belongs to you. The next person who pays the same address works out a different output, because they are spending different coins.",
                    "So the address can be published and reused forever, and the chain still shows a run of unrelated taproot outputs.",
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
                heading: "What the scan key gives away",
                paragraphs: &[
                    "Handing over the scan private key hands over the ability to see every payment this wallet has ever received, and every payment it ever will. It does not hand over the ability to spend one. A watching server with your scan key knows your whole receiving history; give it only to software you run, or to a service you have decided to trust with that.",
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
                    "Check a payment does it for one transaction. Give it the transaction, and the previous transactions of any inputs whose public keys are not in the transaction itself, and the device says which outputs pay this wallet, what they pay, and which label each was paid to. It reads nothing from the network: the transaction, signed, and the ones it spends from have to be copied into Files. For a wallet that watches continuously, export the scan descriptor to software that can scan blocks.",
                ],
            },
            Section {
                heading: "Why the device does not send yet",
                paragraphs: &[
                    "Paying a silent payment address needs the private keys of every input of the transaction, added together, before the output can be worked out. A signing device handed a PSBT does not hold all of them, and the PSBT fields that would let several devices do this together are BIP-375, still a draft. Until that settles, this device receives silent payments and does not send them.",
                ],
            },
        ],
    },
};
