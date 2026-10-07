# Bitcoin software

## Miners

Miner software uses specialized hardware for guessing random numbers to validate blocks and move the chain forward.

## Nodes

Node software runs on normal computers. You can run it at home if you have 1 TB of free space. Nodes validate transactions and keep a copy of all blocks. When you want to see your balance or initiate a transaction, you must connect to a node, either your own or someone else's.

## Hot Wallets

Wallet software runs everywhere on computers and mobile devices. Wallets connect to a node to check account balances and create transactions. Most wallets can also generate entropy and manage funds directly, in which case it is called a hot wallet. Hot wallets are not recommended for large amounts.

## Read-only Wallets

Most wallet software has an option to add a "read-only" wallet. This requires putting in something called a public key. The public key is related to your seed words, but it cannot spend funds. Despite the name, since the public key allows checking balances, it should still be considered private information that you don't want to share.

## Cold Wallets

Cold wallets generally refer to writing down your key or seed words offline, for example on paper or other archival media. Some people also include permanently offline devices or archival hardware (like a USB drive) in this category. In order to use a cold wallet, you need two separate pieces of software, typically on different devices: a coordinator and a signer.

## Coordinators

The coordinator is the internet-connected side of the cold wallet. It does not contain the keys and cannot spend funds. Most coordinators are also read-only wallets. Not every read-only wallet software can function as a coordinator.

## Signers

The signer is the offline side of the cold wallet, and it does contain the keys to spend funds. The most well known signers, often called hardware wallets, are small devices like Ledger and Trezor. Essentially any computer can be used as a signer, but sloppy usage will result in loss of funds.

We recommend using OpenSigner on a permanently offline computer, such as an old Linux laptop or a Raspberry Pi with the WiFi module removed. You can also use OpenSigner on Android or GrapheneOS, although we do not recommend using your daily driver to store more than you're willing to lose. A phone whose bootloader is unlocked, or which is rooted, is not a signer: OpenSigner refuses to run on one.

## Keys, Wallets, Coordinators and Signers in OpenSigner

OpenSigner uses four words in one fixed way, and its screens follow them.

A **key** is a secret: seed words, with or without a passphrase. It has a page of its own under Keys, with what it is made of, its backup, and Forget.

A **wallet** is public data that turns keys into addresses. It is either one key at a script type, which OpenSigner calls a single-sig wallet and lists for every key you load, or a policy over several keys, which is a multisig or MuSig2 wallet you load from a coordinator. A wallet has addresses and a descriptor, and the descriptor is what you give a coordinator. Home lists them, one row each.

Every wallet row carries a mark. A key means this device holds the private key, so it can sign for that wallet. An eye means public keys only: you can derive and compare addresses, and never spend.

A wallet can be loaded from a public key alone — a descriptor, an extended public key or a coordinator's export — and it carries the eye until one of its keys is a key you have loaded.

OpenSigner holds wallets, but it is still not a wallet in the sense the rest of this page uses the word: it has no networking code, no balances and no transaction history.

A **coordinator** is the internet-connected software that watches those addresses, shows balances and builds transactions. A **signer** is the offline software that holds the keys and signs what the coordinator built. OpenSigner is a signer.

## What a row on Home is

Wallets lists every wallet you added and Keys lists every key you loaded. A key does not get a wallet by itself: you add a wallet that uses it and choose its script type, or you load a coordinator's description.

The mark on the row says which kind it is. A key means this device holds a private key of that wallet and can sign for it. An eye means the wallet is public keys alone: addresses to derive and compare, nothing to spend.

Tapping a row opens the wallet's page, and everything you do with that wallet is there: sign, addresses, check an address, export, the key or keys it is made of, and forget.

## OpenSigner

OpenSigner needs at least one key. Create one, or load one from a backup you already have. A SeedQR loads a key in one scan; keep the code private and check for cameras before you show it.

OpenSigner keeps the key in memory while the device is on and wipes it when the device is turned off, so after that the key exists only in your written backup. Keeping a key on the device is offered only on Android phones with a secure element, behind the device lock and a PIN of its own.

On a phone, switching to another app does not lock OpenSigner: the auto-lock and auto-wipe timers run the same whether the app is in front of you or behind another one, and coming back after the auto-lock time is up shows the lock screen.

OpenSigner is not a wallet and has no networking code whatsoever. It cannot connect to the internet or a Bitcoin node. You cannot use it to view balances or initiate transactions.

To use OpenSigner, you must use a coordinator. The coordinator can communicate with OpenSigner either via QR codes or by putting files on the SD card. The coordinator will show your balances, addresses, and transfer destinations. You should verify the entire transaction on OpenSigner before signing.

A Raspberry Pi or a laptop booted from a USB stick has no secure boot: nothing checks the software before it runs. Write the image yourself and compare its hash with the published build before you use it.
