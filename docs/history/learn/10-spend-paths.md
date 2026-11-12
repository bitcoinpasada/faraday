# Spend paths and timelocks

**Kind:** History
**Status:** The Learn pages' text as it was edited before the in-app guide (`docs/guide/`) replaced them

## What a spend path is

Some wallets are not "2 of 3" or "1 of 1". Their script says several different ways the coins can be spent, and a transaction uses one of them. Each way is a spend path: a set of keys that must sign, and sometimes a wait that must have passed first.

A common shape is one key that can spend at any time and a second key that can spend alone after a year. That is two spend paths. Faraday lists them one row each, with the keys each path needs and how long it waits.

## older and after

A timelock written as older counts from the coins arriving. "Key B after 52 560 blocks" means that particular coin must have been sitting in the wallet for that many blocks before key B can move it. The count is per coin and starts when the coin was received, not when the wallet was made and not when you last used the device. If you spend and the change comes back, the change is a new coin and its clock starts again. A recovery plan that assumes otherwise fails at the moment it is needed.

A timelock written as after is a point on the chain, not a wait: a block height the chain must reach, or a date its clock must pass. It is the same moment for every coin in the wallet.

Blocks are not minutes. Ten minutes a block is the average the network aims for, not a promise, so the durations the review shows beside a block count are approximate and are said as such.

## Moving coins before the wait ends

Each coin's wait starts at its own block, the block it arrived in. A coin received a year ago and a coin received last week reach the end of the same wait on different days. A spend that moves every coin to a new address of the wallet makes new coins, and every wait starts again from the block that spend is mined in.

Blocks come about every ten minutes on average and arrive faster while mining grows, so a wait counted in blocks can end days earlier than the days shown beside it say. The wallet card's Move each coin within is 90 % of the shortest wait, in whole days, and is the time to move each coin by. A wait counted in 512-second units is shown in days without "about", and the same 90 % applies.

After moving every coin, press I moved every coin on the wallet's card or on its sheet in the chart. It writes today's date into the vault, and the card then shows Coins last moved with the days since and where the wait ends. The row turns to the warning colour once the days since reach the margin. The date is your word: this device cannot check it against the chain. The same margin sets how often the wallet's backups are due for a check, the shorter of 12 months and the margin.

This device is not connected to the chain. It cannot see when a coin arrived or whether it was moved. The wallet sheet's table, Received on, Amount and Move before, is where deposits and their dates are written down by hand. A wallet whose only wait is a point on the chain, an after, has no Move each coin within: moving the coins does not change it.

## Who chooses the path

The coordinator builds the transaction, and the transaction is what picks the path: the sequence and locktime fields it sets, and the signatures it collects. This device does not choose. It reads what arrived, shows you the wallet and its paths, and signs what you approve.

So a spend path you can see in the review is not a spend path that will work today. Whether the wait has passed is a fact about the chain, and this device is not connected to it. Tools → Miniscript compiler writes a policy of your own as a descriptor; the Tools page describes it.
