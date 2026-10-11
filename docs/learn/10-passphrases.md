# Passphrases

## What a passphrase does

A passphrase is extra text added to your seed words when the key is derived. The same seed words with a different passphrase produce a completely different key, with different addresses and different funds. The words alone are one wallet, the words plus "correct horse" are another, and the words plus "Correct Horse" are a third.

Some people call this the "25th word". It is not a word from the list. It can be any text made of letters, numbers, and symbols. Spaces and capital letters count.

## There is no wrong passphrase

Nothing checks a passphrase. Every passphrase you type opens some wallet. A mistyped passphrase opens an empty wallet that looks perfectly normal, with valid addresses and a zero balance.

The only sign that you typed it wrong is that the addresses are not the ones you expect. Always compare the first receiving address with one you have recorded before trusting that you are in the right wallet.

## Forgetting it is loss

The passphrase is stored nowhere. It cannot be reset, recovered, or guessed. If you forget it, the seed words alone lead to a different, empty wallet.

IF YOU LOSE YOUR PASSPHRASE, THE FUNDS UNDER IT ARE GONE FOREVER, BACKUP OR NO BACKUP.

## Rolling one with dice

A passphrase you make up yourself is weaker than it looks. Lines from songs, names with digits after them, and letter substitutions are all in the lists a cracking program tries first. Words chosen by dice are not in any such list.

Tools › Dice passphrase rolls one for you. You pick a list, you pick how many words, and you throw a die. Five throws name one word of the long list, four throws name one word of a short list. The device does not choose anything; it only looks the words up.

The lists are the ones the Electronic Frontier Foundation publishes. The long list has 7776 words, so each word is worth 12.9 bits. The short lists have 1296 words each, so each word is worth 10.3 bits; their words are shorter to type, and the second short list has no two words within three edits of each other, so a misread word is still recognisable.

Six words from the long list are 77 bits. That is the length to use unless you have a reason to differ: it is more than offline guessing reaches, and it is six words to write down rather than ten.

A rolled passphrase is text like any other. Use it as the BIP-39 passphrase on a key, or as the password for anything else. Keep it apart from the seed words, as the rest of this page says. OpenSigner stores nothing: the words are on the screen while you are reading them and gone when you leave.

## Storing it

Write it down exactly, spaces and capitals included, and read it back before you use it for real. Keep it apart from the seed words, so that whoever finds one does not have both. That separation is the point of a passphrase: a stolen backup of the words alone is useless.

When you keep a key on an Android device, OpenSigner stores the encrypted seed words only. The passphrase is never stored anywhere, and you type it again each session: open the key's wallet on Home, then Key, and choose "Open passphrase". The key that appears carries the same words behind the passphrase you typed, and it is gone when you close the app.

Make sure that whoever is meant to inherit the funds can find both halves. See the Inheritance page.
