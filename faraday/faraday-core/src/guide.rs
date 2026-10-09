//! The written walk-through each step of a flow shows in Guided mode
//! (`docs/WALLETS.md` §6). Steps only (formerly Full) hides it.
//!
//! The rule for this text: say what is on the screen, what to compare it
//! with, and what happens next. No reassurance and no theory; Learn has
//! the theory.

use crate::SpendState;
use crate::wallet::{Kind, step};

/// The walk-through for one step of a spend.
pub(crate) fn spend(n: u8, kind: Option<Kind>, needed: usize, s: &SpendState) -> String {
    let cosigned = kind.is_some_and(Kind::has_cosigners);
    match n {
        step::WALLET => match kind {
            Some(_) => "This is the wallet the transaction spends from. Faraday found it by rebuilding the \
                wallet's address from the key paths the transaction carries, so the match is exact. If \
                the name is not the wallet you meant to spend from, stop here."
                .to_string(),
            None => "No wallet loaded here matches this transaction. You can still sign it from the key \
                paths it carries, but Faraday cannot tell change from a payment to someone else. Load the \
                wallet first if you have its description."
                .to_string(),
        },
        step::CHECK => "Open the same wallet in the software that made this transaction and compare these \
            addresses with the ones it shows. If they match, both programs are looking at the same wallet, \
            and the change address is one you control."
            .to_string(),
        step::TRANSACTION => "Read each line against what you meant to do. Sends is the amount leaving and \
            where it goes. Change is the rest coming back; Verified means Faraday rebuilt that address from \
            the wallet's own keys. The fee goes to the miner. If anything here is not what you expected, \
            stop: nothing has been signed yet."
            .to_string(),
        step::NONCES => "MuSig2 signs in two rounds and every key takes part. In the first, each signer \
            adds a nonce: a one-time number its signature will use. The easiest order is for this device to \
            sign last: add the copies on which the others put their nonce, then sign. Or go first, share \
            this device's nonce and carry the PSBT to them; keep this session open until their signatures \
            come back, because locking forgets the nonce."
            .to_string(),
        step::PATH => "This wallet can be spent more than one way. Each path names the keys it needs and \
            how long it waits: a recovery path opens only after the coins have sat unspent that many blocks. \
            Faraday reads which paths this transaction can take from its own sequence numbers; whether the \
            coins are old enough on the chain is something only the network can say."
            .to_string(),
        step::TXID => "This is the transaction's id, fixed before anyone signs. Signatures do not change \
            it, so you can write it down now and look for it after the transaction is broadcast."
            .to_string(),
        step::SIGNERS => match kind {
            Some(Kind::Threshold) => format!(
                "Any {needed} of these shares sign together, and on the chain the result is one ordinary \
                 Taproot signature. Shares sign one after another, each at its own place. The first share \
                 to sign chooses who follows: pick {} here. After that the transaction names them, and \
                 no other share can join.",
                if needed == 2 { "one other".to_string() } else { format!("{} others", needed - 1) }
            ),
            Some(Kind::Miniscript) | Some(Kind::Tree) => "This wallet has more than one way to spend. \
                Each key below can sign; which keys are enough depends on the path this transaction \
                takes, and some paths wait for a timelock. The last step says whether the script is \
                satisfied."
                .to_string(),
            _ => format!(
                "{needed} {} keys must sign. Keys marked Signs here are loaded on this device. The others are \
                 on other devices or with other people; each of them signs the same PSBT and hands it back.",
                if needed == 1 { "of these" } else { "of the" }
            ),
        },
        step::SIGN => {
            if !s.spend.signed_here.is_empty() {
                "Signed. Each signature was checked against its key before it was kept.".to_string()
            } else if kind == Some(Kind::Threshold) {
                "Signing as a share draws a fresh secret nonce for each share that signs, adds this share's \
                 part, and puts the next share's nonce in a carry file. The carry file is a secret until \
                 that share has signed: keep the stick with you and sign the same day. The last share to \
                 sign checks every part before it combines them."
                    .to_string()
            } else if kind == Some(Kind::MuSig) {
                "Round two: this device adds its partial signature over everyone's nonces. The others do the \
                 same on their copies; when theirs come back, Faraday combines all of them into one signature."
                    .to_string()
            } else if cosigned {
                format!(
                    "This adds the signature of every key loaded here. On its own it moves nothing: the \
                     transaction needs {needed} {} before it can be broadcast.",
                    if needed == 1 { "signature" } else { "signatures" }
                )
            } else {
                "Signing uses the key loaded on this device. Faraday checks the signature before keeping \
                 it, and nothing leaves the device until you put a file in the Outbox."
                    .to_string()
            }
        }
        step::COLLECT => "Get the other signatures. Put the PSBT in the Outbox to carry it on a stick, or \
            let each cosigner sign the copy they already have. When a signed copy comes back, copy it in on \
            a stick visit and add it here. Each signature is checked against its key, and a copy of any \
            other transaction is refused."
            .to_string(),
        step::FINISH => {
            if s.spend.finished.is_some() {
                "There are two results. The signed PSBT goes back to the wallet that made the transaction, \
                 which can broadcast it. The finished transaction can be broadcast as it is, from any node \
                 you trust. Either one is enough."
                    .to_string()
            } else if kind == Some(Kind::Threshold) && (s.carry_out.is_some() || s.out_signed) {
                "Put the carry file in the Outbox and write it to a stick. At the next share's place, \
                 copy it into Faraday with that share loaded, open it from Files and sign. The share that \
                 signs last gets the finished transaction."
                    .to_string()
            } else {
                "The transaction is finished when enough keys have signed. Until then it cannot be \
                 broadcast."
                    .to_string()
            }
        }
        _ => String::new(),
    }
}

/// The walk-through for one card of the backup.
pub(crate) fn backup(n: u8, m: usize, keys: usize, seeds_here: usize) -> String {
    use crate::bstep;
    match n {
        bstep::BLANK => "Print the blank template first, anywhere: it holds no secret, only numbered \
            lines for the words and the squares every SeedQR of this length shares. Put it in the Outbox, \
            carry it to the desktop app, and print one copy per seed. You write on it by hand in the next \
            step."
            .to_string(),
        bstep::SEEDS => {
            let lead = if seeds_here == 0 {
                "None of this wallet's seeds were typed into this session, so there is nothing to copy here. \
                 Each seed is backed up on the device that holds it."
                    .to_string()
            } else {
                "Copy each seed onto its own template by hand: the words, and if you want a code to scan, \
                 the SeedQR square by square. The grey squares are already on the template. Click a row to \
                 keep your place, or move it with the arrow keys. Then scan your copy with the camera; a \
                 mistake is named by its word."
                    .to_string()
            };
            let further = if seeds_here == 0 {
                ""
            } else {
                " Paper comes first. Under the copy, Save into the open vault keeps a further copy \
                 sealed under the vault's passphrase. Save as a file writes the words or the SeedQR \
                 picture to the Outbox unprotected, only after you tick that anyone who copies the \
                 stick can spend with it: the least safe of the three. A file never holds the \
                 passphrase; the vault keeps it only when saved with its passphrase."
            };
            format!("{lead}{further} A printer is a computer with memory and often a network, so a seed never goes \
                     to one. There is no line for a passphrase: written beside the words it stops being a \
                     second factor.")
        }
        bstep::PUBLIC => "This half is xpubs only. It can spend nothing, but it is the only thing that \
            puts the keys back together into this wallet, so keep copies. Show the descriptor as a QR to \
            load the wallet into watch-only software, or put the QR in the Outbox as a picture with the \
            wallet's name, keys and checksum under it, to print or to scan later. Put the files and the \
            backup sheet in the Outbox; the sheet goes as a PDF to print. Sparrow imports the .json under Specter Desktop, \
            and the multisig config under Coldcard Multisig."
            .to_string(),
        bstep::SPLIT => format!(
            "Instead of one sheet with every key, give each of the {keys} signers a share that leaves some \
             keys off. With {} left off each, any {m} shares together hold every key, so any {m} signers can \
             rebuild the wallet, and one share alone cannot watch it. This is not secret sharing: the keys \
             are public, and the shares only decide who can see the balance. Each share goes to the \
             Outbox as a sheet to print, a text file and a picture of its QR; Restore takes the text file \
             or a scan of the picture.",
            m.saturating_sub(1)
        ),
        bstep::ENVELOPE => "One envelope per signer: the seed in that signer's hand, with the sheet or share \
            that goes with it. Store the envelopes apart. A copy kept in a vault is one more \
            copy, not a replacement for paper."
            .to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of signing a message.
pub(crate) fn message(n: u8) -> String {
    use crate::mstep;
    match n {
        mstep::ADDRESS => "A signed message proves that whoever holds the key behind one address agreed to \
            some text. Pick the wallet and the address: usually the one someone asked you to prove you \
            control. Only single-key wallets whose key is loaded here can sign."
            .to_string(),
        mstep::TEXT => "Type the message exactly as the other side gave it. A single changed character, a \
            space or a line break, makes a different message and the signature will not check."
            .to_string(),
        mstep::FORMAT => "BIP-322 is the newer format and the only one for Taproot addresses. BIP-137 is what \
            older wallets and exchanges ask for, and the only one for legacy and nested SegWit addresses. \
            Use whichever the other side asked for."
            .to_string(),
        mstep::SIGN => "Signing moves no money and spends nothing. The result is three lines: the address, \
            the signature and the message. Carry it out as a file or show it as a QR code."
            .to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of creating a wallet.
pub(crate) fn create(n: u8, kind: crate::create::NewKind, m: usize, keys: usize) -> String {
    use crate::cstep;
    let multi = kind.multi();
    if kind.threshold() {
        match n {
            cstep::QUORUM => {
                return format!(
                    "Any {m} of {keys} shares sign. On the chain the wallet is one Taproot key, and a spend \
                     looks like any single-key spend. Keep the shares in different places, or the threshold \
                     protects nothing. This build deals up to five."
                );
            }
            cstep::KEYS => {
                return format!(
                    "Choose {m} loaded keys of 24 words as the first shares. Their words become shares as \
                     they are, so a person who already keeps one of these seeds keeps only that. The deal \
                     computes the other {} from them.",
                    keys - m
                );
            }
            cstep::BUILD => {
                return "The deal computes the remaining shares and the group key, and the wallet's record \
                    lists every public share. The record is public; it is what any device needs to sign \
                    with a share or watch the wallet. The computed shares are loaded here so you can write \
                    their words down in the backup step. Lock afterwards and this device forgets every share."
                    .to_string();
            }
            _ => {}
        }
    }
    match n {
        cstep::KIND => "Choose how the wallet locks its coins. One key is simplest: lose the seed and the \
            money is gone, so its backup matters more. A multisig needs several keys to sign, so one lost or \
            stolen key is not the end of it. If another program will use this wallet, pick a kind it supports."
            .to_string(),
        cstep::QUORUM => format!(
            "{m} of {keys}: every spend needs {m} signatures. Keep the keys apart, on different devices or with \
             different people, or the quorum protects nothing."
        ),
        cstep::KEYS => {
            if multi {
                "Fill each slot. A key loaded here signs on this device. New key makes a fresh seed here; \
                 write its words down in the backup step before any money goes in. A cosigner's slot takes the \
                 account xpub their device exports, copied in on a stick visit; it never brings a secret. \
                 If a cosigner's xpub is not here yet, mark the slot for later and carry on; this wallet waits \
                 under Nothing yet on the Wallets page. Each key held here can give its xpub as a file or a \
                 QR code, so the cosigners can make the same wallet on their side."
                    .to_string()
            } else {
                "Choose the key: one loaded here, or a new one made here from fresh randomness. A new key \
                 exists only in this session until its words are written down in the backup step."
                    .to_string()
            }
        }
        cstep::BUILD => "This line is the whole wallet in xpubs. With it and the seeds, any wallet \
            software can find the coins. Make the wallet to add it to the session."
            .to_string(),
        cstep::CHECK => "Load the wallet into the software you will receive with, by the QR code or the \
            descriptor file, and compare these addresses. They must match before you send anything to it."
            .to_string(),
        cstep::BACKUP => "The paper backup is the one that survives this device: the seeds copied by hand, \
            the wallet's public sheet, and the envelopes they go in. Go through it before any money goes in."
            .to_string(),
        cstep::VAULT => "Everything secret this wallet has here, the keys held on this device, goes into a \
            vault, sealed under its passphrase; the wallet goes with them so the next session loads both in \
            one step. Secrets leave this device only inside a vault file. The paper backup stays the \
            backup; the vault is a copy."
            .to_string(),
        cstep::PUBLIC => "These files hold xpubs only: they can spend nothing, and the cosigners and \
            your watch-only software need them. They go to the Outbox as they are, for the next stick visit."
            .to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of restoring a wallet.
pub(crate) fn restore(n: u8) -> String {
    match n {
        0 => "If a transaction is waiting to be signed, bring it in first: from a stick now, while no key \
            or vault is open, or as a QR code now or later. Once a key is loaded, no stick goes in until \
            you lock."
            .to_string(),
        1 => "Start with the wallet in xpubs: its descriptor, wallet file or multisig config from a \
            stick, the wallet saved in a vault, or the descriptor or split shares on paper as QR codes. Any \
            quorum of shares holds every key; tick the ones you have and rebuild. With seed words \
            alone and no description, Type the seeds."
            .to_string(),
        2 => "Type each seed you hold back in, from its sheet. Faraday checks that the words give the key \
            that slot expects and refuses them if they do not, so a seed in the wrong envelope is caught \
            here. Seeds held by other people stay with them; they sign on their own devices. \
            From the seeds alone, add each one, then Make the wallet: how many keys and how many \
            sign, an account key (xpub) for each cosigner whose words are not here, the kind and \
            the path. The first address shows before the wallet is made."
            .to_string(),
        3 => "Compare this address with the first address written on the backup sheet, and with the wallet \
            software that holds the history. If they match, the wallet is back exactly as it was."
            .to_string(),
        _ => "The wallet is in this session. It is gone when you lock, so back it up again if the old copies \
            are lost."
            .to_string(),
    }
}

/// The walk-through for one card of New key.
pub(crate) fn keygen(n: u8, k: &crate::keygen::KeyGen) -> String {
    use crate::keygen::{Source, kstep};
    let (active, slip39, by_die) = (k.active(), k.slip39, k.by_die);
    match n {
        kstep::LENGTH if slip39 => "A SLIP-39 key is a master secret of 128 or 256 bits, never written \
            whole: it is dealt as shares of 20 or 33 words, and any of them up to the number needed \
            restore it. It has no BIP-39 words."
            .to_string(),
        kstep::WORDS if slip39 => "Write each share down on its own paper, in order, and keep the papers \
            apart: in different places or with different people. Any of them up to the number needed are \
            the key; fewer are nothing."
            .to_string(),
        kstep::QUIZ if slip39 => "Answer from your papers, not from memory: each share is asked in turn, \
            every word once in a random order. A wrong answer means that share's paper needs correcting."
            .to_string(),
        kstep::LENGTH => "12 words carry 128 bits and 24 words 256. Either is beyond guessing; most wallets \
            and signers read every length here."
            .to_string(),
        kstep::SOURCE if slip39 => "Randomness you make yourself, with dice, coins or cards, does not \
            depend on this device's software: you can make the same secret again elsewhere from the same \
            rolls. The shares are worked out from it by the device. This device's generator is the operating \
            system's; nothing on the screen can show whether it was faulty or rigged, so a key made from it \
            trusts this device. A mix combines several sources, so the key is as good as the best of them."
            .to_string(),
        kstep::SOURCE => "Randomness you make yourself, with dice, coins or cards, does not depend on this \
            device's software. In the first group every word is your entries' own bits: each one shows as \
            it comes in, and you can find it in the list by hand. In the second the device hashes your \
            entries into the words, which you can check only by making the same key again elsewhere. This \
            device's generator is the operating system's; nothing on the screen can show whether it was \
            faulty or rigged, so a key made from it trusts this device. A mix combines several sources, so \
            the key is as good as the best of them."
            .to_string(),
        kstep::ENTER => match active {
            Some(Source::Dice | Source::Coins) if by_die && slip39 => "Roll one die at a time \
                and enter the face that came up. Each roll is one flip, 1 to 3 tails and 4 to \
                6 heads, so the secret is the one a coin would make from the same flips."
                .to_string(),
            Some(Source::Dice | Source::Coins) if by_die => "Roll one die at a time \
                and enter the face that came up. Each roll is one flip, 1 to 3 tails (0) and 4 to \
                6 heads (1), so the key is the one a coin would make from the same flips. Every \
                11 flips are a word's number in the list, counted from 0, and the word shows as \
                its 11th flip comes in."
                .to_string(),
            Some(Source::Dice) if k.procedure().direct() => format!(
                "Roll one die at a time and enter the face that came up. A word takes six rolls. In \
                its first five, 1 is 00, 2 is 01, 3 is 10 and 4 is 11; a 5 or a 6 there stands for \
                nothing and is rolled again. The sixth roll is one bit: 1 to 3 is 0, 4 to 6 is 1. The \
                11 bits, in order, are the word's number in the list counted from 0: 3 1 4 2 2 5 is \
                10 00 11 01 01 1, which is 1131, the list's word 1132, \"miracle\". The last word is \
                rolled the same way; the key keeps its first {} bits, and its other {} are the \
                checksum, which the device works out from all the others and writes in their place.",
                k.last_bits(),
                k.words / 3
            ),
            Some(Source::Coins) if !slip39 => "Flip a coin and enter what came up, once per flip. \
                Catching the coin in the air is fine; choosing the side is not. Heads is 1 and tails \
                0; every 11 flips are a word's number in the list, counted from 0, and the word shows \
                as its 11th flip comes in."
                .to_string(),
            Some(Source::Dice) => "Roll one die at a time and press the face that came up. Use a real \
                die on a flat surface, not the same throw twice, and do not pick numbers in your head."
                .to_string(),
            Some(Source::Coins) => "Flip a coin and press what came up, once per flip. Catching the coin \
                in the air is fine; choosing the side is not."
                .to_string(),
            Some(Source::Cards) => "Shuffle a full deck well, then turn cards over one at a time and enter \
                each: its rank, then its suit. A card cannot come up twice from one deck; past 52, shuffle \
                the deck again and keep going."
                .to_string(),
            Some(Source::Hex) => "Type the hex digits you made elsewhere. They become the key as they are, \
                so they must already be random."
                .to_string(),
            Some(Source::Camera) => "Point the camera at something that moves or is lit unevenly and take \
                each picture. The pictures are hashed and never kept."
                .to_string(),
            _ => "This device asks the operating system's generator for 32 fresh bytes when this card \
                opens. The key is those bytes, hashed. Nothing on the screen can show whether they were random."
                .to_string(),
        },
        kstep::CHECK => "These counts catch the mistakes hands make: a die that favours one face, the same \
            result typed over and over, or a sequence counted out instead of rolled. A caution is not a \
            refusal; if one appears and you are not sure why, roll again."
            .to_string(),
        kstep::WORDS => "Write the words down in order, on paper, before any money goes to this key, and keep \
            them where only you can reach them. The words are the key: anyone who has them can spend, and \
            without them a lost device loses the money."
            .to_string(),
        _ => "Answer from your paper, not from memory: the quiz checks the copy you will rely on. Each word \
            is asked once, in a random order. A wrong answer means a word on the paper needs correcting; \
            look at the words again, fix the copy, and carry on."
            .to_string(),
    }
}
