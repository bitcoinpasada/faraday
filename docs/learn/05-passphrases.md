# Passphrases

## What a passphrase does

A passphrase is extra text added to your seed words when the key is derived. The same seed words with a different passphrase produce a completely different key, with different addresses and different funds. The words alone are one wallet, the words plus "correct horse" are another, and the words plus "Correct Horse" are a third.

Some people call this the "25th word". It is not a word from the list. It can be any text made of letters, numbers, and symbols. Spaces and capital letters count.

## There is no wrong passphrase

Nothing checks a passphrase. Every passphrase you type opens some wallet. A mistyped passphrase opens an empty wallet that looks perfectly normal, with valid addresses and a zero balance.

The only sign that you typed it wrong is that the addresses are not the ones you expect. Always compare the first receiving address with one you have recorded before trusting that you are in the right wallet.

## Forgetting it is loss

Nothing stores the passphrase unless you keep it in a vault. It cannot be reset, recovered, or guessed. If you forget it, the seed words alone lead to a different, empty wallet. A lost passphrase loses the funds under it for good, whatever backup of the words there is.

## Rolling one with dice

A passphrase you make up yourself is weaker than it looks: song lines, names with digits after them and letter substitutions are in the lists a cracking program tries first. Words chosen by dice are not in any such list.

Create a vault rolls one for you, under Generate with dice. You pick a list and how many words, and you throw a die. Five throws name one word of the long list, four throws name one word of a short list. The device only looks the words up.

The lists are the ones the Electronic Frontier Foundation publishes, and Tools › Word lists shows each of them. The long list has 7776 words, 12.9 bits a word; the short lists have 1296, 10.3 bits a word. Six words from the long list are 77.5 bits, which is more than offline guessing reaches. Vaults states the strength of the one you choose.

A rolled passphrase is text like any other. Create a vault uses it as the vault's passphrase; rolled the same way, it can be the BIP-39 passphrase on a key.

## Vanity addresses

Tools › Vanity address tries one candidate after another until the first address of the account begins with the characters you asked for. Each free character costs 32 tries on a bech32 address and 58 on a base58 one, and the device states the rate it manages and how long that takes.

There are two dials. The passphrase counter appends characters to the key's BIP-39 passphrase, so every candidate is a different key and costs a full derivation; taking the find loads that key beside the one you started from. The account index steps through the accounts of the key you already have, which is far cheaper and adds a wallet rather than a key.

A vanity address is not safer, and a familiar-looking string is one an attacker can imitate: never check an address by its first characters alone. The passphrase dial leaves you a passphrase to back up, since the words alone no longer reach the funds, and a counter written down nowhere is a key lost. The counter is the one other tools use, in the same order, so a find here is a find there.

## Storing it

Write it down exactly, spaces and capitals included, and read it back before you use it for real. Keep it apart from the seed words, so that whoever finds one does not have both. That separation is the point of a passphrase: a stolen backup of the words alone is useless.

A vault can keep a seed's passphrase with the seed, and unlocking the vault then loads the key the two give. Kept that way, the passphrase is behind the vault's passphrase rather than apart from the words, and whoever opens that vault has the key. A passphrase kept in a vault belongs in a different vault from its seed, at another place. Faraday's backup plan warns when a seed and its passphrase are at the same place. A seed kept in a vault without its passphrase loads with Load with a passphrase, which asks for it each time and stores nothing.

Whoever is meant to inherit the funds must find both halves. Seed words without the passphrase open a different, empty wallet that looks correct, and an heir who has only the words will conclude that the backup is broken and stop looking. See the Inheritance page.
