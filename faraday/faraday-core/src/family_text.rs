//! The Spend tab's words (`docs/FAMILY.md` §4): the offline system is
//! Faraday itself, the wallet usually comes from a vault, and a stick is
//! read only after a lock. Each card has a lead, always shown, and a
//! longer More about this.
//!
//! The rule for this text is Faraday's: plain statements of fact. The
//! review's warnings are not here; they are the inspector's own words, as
//! Wallets shows them.

use crate::family::{CardId, Route, page};
use crate::wallet::{Kind, step};
use crate::{Faraday, Medium, guide};

/// The map: where each stop happens, its name, and what is done there.
pub fn map(m: Medium) -> [(&'static str, &'static str, String); 6] {
    [
        (
            "Paper",
            "What you have",
            format!(
                "The Faraday {} and its passphrase, or words written down, and often a sheet \
                 describing the wallet.",
                m.noun()
            ),
        ),
        (
            "Offline",
            "Open the wallet here",
            format!(
                "Start a computer from the Faraday {} and open the wallet.",
                m.noun()
            ),
        ),
        (
            "Online",
            "Check the balance",
            "Put the wallet's description into Sparrow on an online computer.".to_string(),
        ),
        (
            "Online",
            "Write the payment",
            "Make the transaction in Sparrow. It gives you a PSBT.".to_string(),
        ),
        (
            "Offline",
            "Sign it here",
            "Bring the PSBT to Faraday, read what it does, and sign it.".to_string(),
        ),
        (
            "Online",
            "Send it",
            "Take the signed transaction back to Sparrow and broadcast it.".to_string(),
        ),
    ]
}

/// The line under the map that is never folded away.
pub const WORDS_ARE_MONEY: &str = "The words are the money. Never photograph them, and never type \
    them into a phone, a website or a password manager. The same goes for the vault's passphrase.";

/// The answers to What are you holding?: the route, its title, what it
/// looks like.
pub fn answers(m: Medium) -> [(Route, String, String); 3] {
    [
    (
        Route::Vault,
        format!("A Faraday {} and its passphrase", m.noun()),
        format!(
            "The {} this computer started from, or another one with a vault on it, and a \
             passphrase written down apart from it. The vault holds the wallet and the keys \
             that sign for it.",
            m.noun()
        ),
    ),
    (
        Route::Words,
        "Words, and nothing else".to_string(),
        "Twelve, eighteen or twenty-four ordinary English words, numbered, in a fixed order, and \
         nothing else that looks like a long code, a block of letter-and-number lines, or a QR \
         code. A single list of words is usually the whole wallet. Several lists, perhaps with \
         a line like 2 of 3, are one wallet of several keys and go here too."
            .to_string(),
    ),
    (
        Route::Paper,
        "Words and something else".to_string(),
        "As well as the words there is one long line starting wsh( or wpkh(, or lines pairing \
         short codes with xpubs, or QR codes meant to be scanned, perhaps spread across several \
         sheets. The wallet needs more than one key, and its description goes in before the \
         words."
            .to_string(),
    ),
    ]
}

/// The answer that opens the list below.
pub const UNSURE: (&str, &str) = (
    "I am not sure",
    "What each thing in the envelope looks like, and how to tell them apart.",
);

/// What each thing in the envelope looks like: its look, then what it is.
pub fn looks(m: Medium) -> [(String, String); 8] {
    let looks: [(&str, &str); 7] = [
        (
            "A square black-and-white pattern",
            "A QR code: the same information in a form a camera can read. Several numbered 1 of 5 \
         and so on are one thing in parts: scan them one after another. Never scan a QR code \
         that sits beside seed words into anything but Faraday.",
        ),
        (
            "Words numbered 1 to 12, 18 or 24",
            "The seed phrase. Often headed recovery phrase, seed words or backup, or stamped into \
         steel.",
        ),
        (
            "One long line starting wpkh(, wsh(, tr(, xpub, zpub or tpub",
            "A descriptor or an xpub. It shows the addresses and the balance but cannot spend. \
         Do not post it publicly: it shows the wallet's whole history.",
        ),
        (
            "Several sheets each holding a few lines, like 73C5DA0A: xpub…",
            "One description split up on purpose, often under a heading such as this sheet carries 2 \
         of 3 wallet descriptor parts. Enough of the sheets, in any order, add up to the whole. \
         A line such as Policy: 2 of 3 says how many signatures the wallet needs.",
        ),
        (
            "A word or sentence kept apart from the list, called a passphrase or a 25th word",
            "If one exists, the words alone open a wallet that is real but empty, and the money is \
         in the wallet the words and the passphrase open. This is not the vault's passphrase: \
         the vault's passphrase opens the vault file, and this one changes which wallet the \
         words open.",
        ),
        (
            "A short line starting bc1, 3 or 1",
            "One address, not a backup.",
        ),
        (
            "A small device with a screen and buttons",
            "A hardware wallet. It is a way of using the words, not a replacement for them.",
        ),
    ];
    let [a, b, c, d, e, f, g] = looks.map(|(look, what)| (look.to_string(), what.to_string()));
    let medium = (
        format!(
            "{} labelled Faraday, and a passphrase on a separate card",
            m.a_full_cap()
        ),
        format!(
            "The {} starts the computer and carries the vault, a locked file named like \
             vault.ofv. The passphrase opens it. The vault holds the wallet and the keys that \
             sign for it.",
            m.noun()
        ),
    );
    [medium, a, b, c, d, e, f, g]
}

/// A card's title.
pub fn title(app: &Faraday, id: CardId) -> String {
    let s = match id {
        CardId::Page(p) => match p {
            page::MAP => "How to spend bitcoin",
            page::SAFE => return format!("Start from the {}", app.medium.noun()),
            page::HOLDING => "What are you holding?",
            page::OPEN => "Open the wallet",
            page::CHECK if app.family.from_card => "Load the wallet in Sparrow",
            page::WRITE if app.family.from_card => "Write the payment in Sparrow",
            page::CHECK => "Check the money is really there",
            page::WRITE => "Write the payment",
            page::BRING => "Bring the transaction here",
            _ => "Put everything away",
        },
        CardId::Step(n) | CardId::Later(n) => match n {
            step::TRANSACTION => "Read the transaction",
            step::TXID => "The transaction id",
            step::PATH => "Which way it spends",
            step::NONCES => "Nonces",
            step::SIGNERS if app.family.route == Some(Route::Paper) => {
                "Add the seeds that can sign"
            }
            step::SIGNERS => "Who signs",
            step::SIGN => "Sign it",
            step::COLLECT => "The other signatures",
            step::FINISH => "Send it",
            _ => "",
        },
    };
    s.to_string()
}

/// A card's lead: what to do on it, always shown while it is open.
pub fn lead(app: &Faraday, id: CardId) -> String {
    let s: &str = match id {
        CardId::Page(page::MAP) => {
            "Six stops, three on this computer and three on an online one. Nothing moves until \
             the last."
        }
        CardId::Page(page::SAFE) if app.online => {
            return format!(
                "This is the desktop copy of Faraday, which runs on an ordinary computer that has \
                 been online. It is for testing. For real money, start a computer from the \
                 Faraday {}.",
                app.medium.noun()
            );
        }
        CardId::Page(page::SAFE) => {
            return format!(
                "This computer is running Faraday from the {}. It has no network, and it forgets \
                 everything when it is switched off.",
                app.medium.noun()
            );
        }
        CardId::Page(page::HOLDING) => {
            "Lay out everything in the envelope first. Several items is normal and does not mean \
             anything is missing."
        }
        CardId::Page(page::OPEN) => return open_lead(app),
        // From the wallet's card: what to press, no more.
        CardId::Page(page::CHECK) if app.family.from_card => {
            "In Sparrow: File, New Wallet, then scan the wallet QR. Its first address is receive \
             address 0 here."
        }
        CardId::Page(page::WRITE) if app.family.from_card => {
            "In Sparrow's Send tab: the address, the amount and a fee. Then Create Transaction, \
             Finalize Transaction for Signing and Show QR."
        }
        CardId::Page(page::CHECK) => {
            "Load this same wallet in Sparrow on your online computer to check the balance. The \
             code below is the wallet's description: enough for Sparrow to find every address \
             the wallet uses and add up what is on all of them. It cannot spend. Then check that \
             the first address Sparrow shows is character for character the same as receive \
             address 0 below. If it matches, two separate programs have read the same backup \
             and agreed. If it does not match, stop: something was typed or scanned wrongly."
        }
        CardId::Page(page::WRITE) => {
            "This part happens on the online computer. In Sparrow, open the wallet's Send tab, \
             enter the address to pay and the amount, and choose a fee. Press Create Transaction, \
             then Finalize Transaction for Signing, then Show QR: an animated code, which is what \
             Faraday reads. Sparrow can also save it as a .psbt file."
        }
        CardId::Page(page::BRING) => {
            return format!(
                "Two ways. Scanning is the simpler: nothing has to lock. Press Scan and hold \
                 Sparrow's QR code up to this computer's camera; an animation is read as it \
                 loops. Or save the .psbt onto {a} and copy it in. Faraday reads {a} only while \
                 no vault or key is open, so with one open it locks first: it seals the vault, \
                 forgets the keys and starts again. Copy the PSBT in, remove the {noun}, and \
                 unlock with the same passphrase. This tab comes back on this page.",
                a = app.medium.a(),
                noun = app.medium.noun()
            );
        }
        CardId::Page(_) => {
            return format!(
                "Lock Faraday. The vault is sealed again and the keys are forgotten. If a file \
                 you still need waits for the {noun}, plug {a} in and the {noun} visit writes \
                 it. Then switch off, take the {noun} out, and put it back with the papers.",
                a = app.medium.a(),
                noun = app.medium.noun()
            );
        }
        CardId::Later(_) => "This opens once the transaction is here.",
        CardId::Step(step::TRANSACTION) => {
            "Read the summary before you sign, every time: what leaves, what comes back, and the \
             fee. Change is the rest coming back to the same wallet, not money leaving. Signing \
             is the one step that cannot be taken back. If this is not what you meant to do, a \
             different amount, an address you do not recognise, or a warning of any kind, do not \
             sign. Declining costs nothing."
        }
        CardId::Step(step::TXID) => {
            "Write down the first and last few characters of this id. Sparrow shows the same id \
             for the transaction it wrote; after sending, the id is how you follow it."
        }
        CardId::Step(step::SIGNERS) if signers_missing(app) => {
            "Every key that signs is listed. A key not here yet comes from its own place: type \
             its words, scan its SeedQR, or unlock the vault that holds it. Each is checked \
             against the wallet, so a key from another wallet is refused. A key held by someone \
             else does not have to come here at all: they sign the same PSBT on their own device, \
             and their signed copy is collected after this device signs."
        }
        CardId::Step(step::SIGNERS) if app.family.route == Some(Route::Paper) => {
            "One seed at a time: put its words in, add it, and the count goes up. Order does not \
             matter and the same seed twice does no harm. Each list is checked against the \
             wallet's own keys, so one belonging to a different wallet is refused. A seed kept \
             as a QR code beside the words does not have to be typed: scan it. If that backup \
             has a separate passphrase, type it in the passphrase box before adding."
        }
        CardId::Step(step::SIGN) => {
            "Signing adds the signature of every key loaded here. For a wallet that needs \
             signatures from elsewhere, the next card lists who still has to sign and takes \
             their signed copies; each is checked against its key and this transaction before \
             it counts."
        }
        CardId::Step(step::FINISH) => {
            return format!(
                "The finished transaction is what the network accepts. Show the signed PSBT as a \
                 QR code; in Sparrow, on the same transaction, press Scan QR and hold it up to \
                 the webcam, then Broadcast Transaction. Or carry the finished transaction, which \
                 waits in Files, on {}. It is safe to hand to anyone: nobody can change \
                 where the money goes without breaking the signatures, and a broken transaction \
                 is rejected, not redirected.",
                app.medium.a()
            );
        }
        CardId::Step(n) => {
            let s = app.spend.as_ref();
            let kind = s
                .and_then(|s| s.wallet)
                .and_then(|w| app.session.wallets.get(w))
                .map(|w| Kind::of(&w.policy));
            return match s {
                Some(s) => guide::spend(n, kind, app.spend_needed(), s, app.medium),
                None => String::new(),
            };
        }
    };
    s.to_string()
}

/// The spend's wallet needs more keys than are here.
fn signers_missing(app: &Faraday) -> bool {
    let Some(s) = app.spend.as_ref() else {
        return false;
    };
    let here = s
        .inspection
        .participating_keys
        .iter()
        .filter(|fp| app.session.key_label(**fp).is_some())
        .count();
    app.family.route != Some(Route::Paper) && here < app.spend_needed()
}

fn open_lead(app: &Faraday) -> String {
    let s = match app.family.route {
        Some(Route::Vault) => {
            return format!(
                "The vault comes in by itself from the {noun} this computer started from; one on \
                 another {noun} is copied in on {a} visit. Faraday opens a vault only with no \
                 {noun} attached: take the {noun} out and keep it with the papers. Then type the \
                 vault's passphrase exactly as it is written, capital letters and spaces \
                 included. Then the vault lists its wallets and seeds, all ticked: Load brings \
                 them in, or untick what you do not need first.",
                noun = app.medium.noun(),
                a = app.medium.a()
            );
        }
        Some(Route::Words) => {
            "Type the words in the order they are written, separated by spaces. Spelling and \
             order matter; nothing else does. Faraday checks them against the list of 2048 words \
             and the backup's own checksum, so a mistyped or missing word is caught here. If a \
             separate passphrase came with the words, put it in the passphrase box exactly as \
             written. If there was none, leave it empty. Several lists are one wallet of several \
             keys: add each with Add another seed, then choose how many keys there are and how \
             many sign, the kind and the path."
        }
        Some(Route::Paper) => {
            "Scan or type the long code exactly as it is written, including everything after the \
             #. It cannot move money: it opens the wallet read-only, which is enough to see every \
             address and to check the balance. The words come later, after the transaction has \
             been read."
        }
        None => "Answer What are you holding? first.",
    };
    s.to_string()
}

/// A card's More about this, as separate paragraphs.
pub fn more(app: &Faraday, id: CardId) -> Vec<String> {
    let m = app.medium;
    let fixed: &[&str] = match id {
        CardId::Page(page::MAP) => {
            let mut paras = strings(&[
                "Anyone who reads the words can spend the money from anywhere in the world, and it \
             cannot be reversed. Where a wallet needs several lists, anyone with enough of them \
             can. Treat every list as if it were the only one. No legitimate bitcoin company will \
             ever ask for them: not a support line, not someone helping you, not an AI.",
                "Nothing done here is irreversible until the last step, and that step does not happen \
             in Faraday. Opening the vault, typing words, looking at addresses and reading a \
             transaction move nothing and notify no one. You can stop and come back.",
                "There is no deadline; bitcoin does not expire. Rushing is how people get robbed, by \
             scammers and by helpers acting in their own interest. Take your time, and find \
             someone you trust.",
            ]);
            paras.push(format!(
                "If any of this stops making sense, stop. Put the {} and the paper somewhere safe \
                 and get help from someone you would trust with the money itself. Waiting costs \
                 nothing.",
                m.noun()
            ));
            return paras;
        }
        CardId::Page(page::SAFE) => {
            return vec![
                "Shut the computer down completely, not sleep or restart. Unplug any network \
                 cable."
                    .to_string(),
                format!(
                    "Plug the Faraday {} in, switch the computer on and press the boot menu key \
                     straight away, repeatedly. It is usually F12, F9, Esc or F2. The first \
                     screen often names the key for a second or two.",
                    m.noun()
                ),
                format!(
                    "Pick the {} from the list that appears. If no list appears, the key was \
                     wrong or came too late: shut down and try another one.",
                    m.full()
                ),
                format!(
                    "If the {noun} is missing, or someone else has opened the envelope, do not \
                     use it. Do not use {a} you are unsure about: the words are the money, and \
                     taking longer costs nothing.",
                    noun = m.noun(),
                    a = m.a()
                ),
            ];
        }
        CardId::Page(page::OPEN) => match app.family.route {
            Some(Route::Vault) => &[
                "Unlocking takes the time shown on purpose: it makes guessing the passphrase slow.",
                "A wrong passphrase opens nothing and says so. Try again as often as you like.",
                "A vault can have more than one passphrase, and each opens its own contents. If \
                 what opens has no wallet in it, check the passphrase against the paper again.",
                "A vault may hold only part of a wallet: its description and one key, say. The \
                 other keys come from their own places, on the signing cards: words typed or \
                 scanned from a SeedQR, another vault, or a cosigner who signs on their own \
                 device and hands back the signed copy.",
                "If the vault holds keys but not the wallet they belong to, load the wallet's \
                 description from the paper below.",
            ],
            Some(Route::Words) => &[
                "The wallet opens as native SegWit, account 0: addresses starting bc1q, what most \
                 wallets make. If Sparrow shows nothing on the next page, the wallet may be another \
                 kind; each kind's first address is listed, to compare with any address on the \
                 paper.",
                "When the transaction arrives, Faraday reads which kind it spends from its own key \
                 paths and opens that kind.",
                "With several lists of words, the wallet is a multisig. The number of keys and \
                 how many must sign are often written down as 2 of 3 or similar. A key whose \
                 words are not here, a cosigner's, goes in as its account key (an xpub): scanned, \
                 from a key file in Files, or typed. Native SegWit multisig at the standard path \
                 is what most wallets make; if Sparrow shows nothing on the next page, try \
                 another kind or account.",
            ],
            Some(Route::Paper) => &[
                "If it came in pieces, do one sheet at a time. Scan or copy in what one sheet \
                 gives you; Faraday counts the keys in hand and waits. Order does not matter, a \
                 sheet entered twice does no harm, and the wallet opens by itself when the last \
                 part arrives.",
                "The number of signatures needed is printed at the top of each sheet, as 2-of-3 or \
                 Policy: 2 of 3.",
            ],
            None => &[],
        },
        CardId::Page(page::CHECK) => &[
            "In Sparrow: File, New Wallet, give it a name, then choose the way in that scans a \
             QR code or imports a descriptor, and hold this screen up to the webcam. Menu names \
             change between versions; look for an import that takes a descriptor or a scan. If \
             the code is animated, let it loop once.",
            "Only load it into wallet software you chose and trust.",
            "Faraday can work out every address the wallet owns, but not which were used. Money is \
             often spread over several addresses, including change addresses from old payments, \
             so the first few can be empty while the money is further down. Sparrow adds them \
             all up.",
            "On a phone you can also look an address up on a block explorer such as \
             mempool.space. That moves nothing; it does tell the website you looked.",
        ],
        CardId::Page(page::WRITE) => &[
            "Paying into an exchange: the exchange gives a deposit address for bitcoin in your \
             account. Copy it exactly, and compare the first and last few characters after \
             pasting.",
            "Faraday locks after a few minutes without input. That is expected. Unlock again with \
             the same passphrase and this tab opens where you left it.",
        ],
        CardId::Step(step::SIGN) => &[
            "If the envelope held a hardware wallet that came with the papers and looks \
             untouched, you can sign the same PSBT on it too and compare. Follow the device's \
             own guide to restore from the same words. Compare what both screens say before \
             approving anything: the amount, the fee and the address. They must match.",
            "Faraday on its own is enough to do the whole job.",
        ],
        CardId::Step(step::FINISH) => &[
            "Any of these sends it: Sparrow, which opens a transaction from a QR code, a file or \
             pasted text and broadcasts it; a block explorer's broadcast page, such as \
             mempool.space or blockstream.info; or someone you trust.",
            "Then check the id. Whatever sends it should report the id Faraday showed. If the id \
             you see anywhere differs, what was sent is not what you signed here: stop and find \
             out why.",
            "The first confirmation usually takes about ten minutes but can take hours if the fee \
             was low. Exchanges usually want one to six confirmations before the money shows in \
             the account.",
        ],
        CardId::Page(page::AWAY) => {
            return vec![format!(
                "When Faraday signs with a vault open, it records the amounts it signed inside \
                 that vault, so locking seals a changed copy of the vault to write back. \
                 Writing it back to the {} keeps that record; leaving it out changes nothing \
                 else.",
                m.noun()
            )];
        }
        _ => &[],
    };
    strings(fixed)
}

fn strings(paras: &[&str]) -> Vec<String> {
    paras.iter().map(|p| p.to_string()).collect()
}
