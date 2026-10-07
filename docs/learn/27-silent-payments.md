# Silent payments

## What a silent payment address is

A silent payment address is one string, starting `sp1q`, that you can print on a card, put on a web page or send to anyone who will ever pay you. It is 116 characters long, and it is not a Bitcoin address: nothing is ever paid to it directly, and no block explorer will show it.

What the payer does with it is arithmetic. They take the two public keys the address carries, combine them with the public keys of the coins they are spending, and work out a fresh taproot output that belongs to you. They pay that output. The next person who pays the same address works out a different output, because they are spending different coins.

So the address can be published and reused forever, and the chain still shows a run of unrelated taproot outputs. That is the whole point: address reuse is the ordinary way a person's payments get linked together, and this is the arrangement that removes the reason to reuse an address.

## What the two keys are

The address carries a scan key and a spend key. Both come from one of your keys, at paths BIP-352 sets aside: the scan key at `m/352h/0h/0h/1h/0` and the spend key at `m/352h/0h/0h/0h/0` on mainnet. Recovering the key recovers the wallet.

The scan key finds payments. The spend key spends them. They are separate so that the finding can be given away without the spending: a machine that watches the chain for you needs the scan private key and gets no ability to move a coin.

## What the scan key gives away, and to whom

Handing over the scan private key hands over the ability to see every payment this wallet has ever received, and every payment it ever will. It does not hand over the ability to spend one.

That is a real trade. A watching server with your scan key knows your whole receiving history, and if it also knows who you are, it knows who paid you. Give it to software you run, or to a service you have decided to trust with that; do not give it to anyone you would not show your bank statements to.

Export on a silent payments wallet writes it as `sp(spscan1q…)`, which is BIP-392's descriptor for exactly this job. The device shows it on a hidden panel, as it shows seed words, because it is a secret.

## Labels

A label turns the one address into several. Label 1, label 2 and so on each give a different `sp1q` string, all found by the same scan key and spendable by the same spend key. Give one to each payer and you can tell payments apart without publishing separate wallets.

Labels are derived from the scan private key, not stored, so they come back from the seed. The device hands them out in order and remembers how many it has handed out. Label 0 is reserved: it is the label a wallet uses for its own change, and it is never handed to anyone, because a payer who knew it could make a payment your wallet would file as change.

## Checking that you were paid

A silent payment leaves no address to look up. To know that a transaction paid you, someone has to do the arithmetic with the scan private key.

Check a payment does it for one transaction. Give it the transaction, and the previous transactions of any inputs whose public keys are not in the transaction itself, and the device says which outputs pay this wallet, what they pay, and which label each was paid to. It reads nothing from the network: the transaction has to be brought to it, as a PSBT or as a raw transaction, by code or by file.

For a wallet that watches continuously, export the scan descriptor to software that can scan blocks. That is what the descriptor is for.

## Why the device does not send yet

Paying a silent payment address needs the private keys of every input of the transaction, added together, before the output can be worked out. A signing device that is handed a PSBT does not hold all of them, and the PSBT fields that would let several devices do this together are BIP-375, which is still a draft.

Until that settles, this device receives silent payments and does not send them. A wallet elsewhere can pay a silent payment address today; this one cannot.
