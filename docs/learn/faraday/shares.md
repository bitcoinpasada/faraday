# Shares of a wallet description

## The wallet description

A multisig wallet is defined by its wallet description: the account key (xpub) of every signer, the number of signatures a spend needs, and the script type. Every address of the wallet is made from all of its keys at once. Working out any one address, and so finding the wallet's coins, needs every key in the description. A description missing even one key gives no addresses at all.

The seeds sign. The description finds the coins. A multisig backup needs both: a quorum of seeds and the whole description.

## What a share is

A share is the wallet description with some keys left off. Faraday writes each share as a multisig config holding the keys it keeps, the quorum and the script type, on its own sheet.

The backup plan's Places question gives each place either the whole wallet sheet or its own share. The whole sheet is the default. With shares, the slider sets how many keys are left off each share.

## What leaving keys off gives

An m-of-n wallet has n shares. With k keys left off each, share 1 holds keys 1 to n - k, share 2 holds keys 2 to n - k + 1, and so on round the list. Each key is then on n - k shares and missing from k of them.

k can be at most m - 1. Each key is missing from at most m - 1 shares, so any m shares together hold every key, and any group of signers able to spend can also rebuild the description. With k of 1 or more, no single share holds every key, so whoever finds one share cannot work out an address and cannot see the balance.

With k of 0, every share is the whole description.

## 2-of-3 and 3-of-5

A 2-of-3 can leave 0 or 1 key off each share. With 1, each share holds two of the three keys. Any two shares hold all three, and one share alone sees nothing.

A 3-of-5 can leave 0, 1 or 2 keys off each share. With 1, each share holds four keys, and any two shares hold all five. With 2, each share holds three keys. Any three shares hold all five, and some pairs do too: share 1 holds keys 1, 2 and 3, share 4 holds keys 4, 5 and 1. The table under the slider lists each share's keys, and the lines under it say how few shares rebuild the wallet.

## Not secret sharing

Shares are not Shamir's Secret Sharing or SLIP-39. Those split a secret. A share holds account keys, which cannot sign: no number of shares can spend. Spending needs a quorum of seeds, wherever the shares are.

Shares decide who can see the balance and the history of payments, not who can spend. A share holder learns the account keys on that share, and nothing about the wallet's addresses until enough shares come together.

## Rebuilding from shares

Load or restore a wallet in Faraday reads shares from Files. Choose the shares under Shares of a split backup; once they hold every key between them, Rebuild the wallet puts the whole description back together and loads it.

Other software, such as Sparrow or a multisig coordinator, needs the whole description and does not read shares. Whoever rebuilds the wallet uses Faraday, or rebuilds the description in Faraday first and takes the whole description from there.
