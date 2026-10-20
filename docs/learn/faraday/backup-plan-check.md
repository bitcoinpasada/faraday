# A backup plan's check

## What it checks

Faraday's backup starts with a plan: where the seeds go, how many places keep paper, and where the wallet description goes. Under the plan's map it checks three things, by trying each place lost and each place found in turn. A place here is anywhere a copy is kept: a paper place, a stick of unprotected files, or the vault's stick when no paper place keeps one. Seeds that live on their own devices count as kept and never as found.

## Any one place lost

"Any one place lost: the rest rebuild the wallet" is Yes when, for every place, the other places still hold enough: a quorum of seeds, each with its passphrase if it has one, and for a wallet of more than one key the wallet description, whole or from shares that together hold every key. Watch-only software counts as a copy of the description.

"Yes, with the vault's passphrase" means at least one loss is covered only by the vault, so the vault's passphrase has to be remembered or kept somewhere else. No means one place is a single point of failure.

## One place found: can spend

"One place found: can spend" is Yes when one place on its own holds a quorum of seeds, with their passphrases. The description is not counted: it holds only xpubs, which every watch-only wallet and every share also holds, so whoever finds the seeds can get it. Anyone who finds that place can take the coins.

On the board, each place that keeps a seed says how near it brings a spend: "Can spend alone", or "Spends with 1 more place" when one other place adds enough seeds. Moving a second seed to a place that brings it nearer a spend asks first, with Keep it and Take it off.

"Only with the vault's passphrase" means the place holds a stick with the vault, and the vault holds what is missing: the finder also needs the vault's passphrase. No is what a plan should aim for.

## One place found: sees the balance

"One place found: sees the balance" is Yes when one place holds the whole wallet description, or for a wallet of one key its seed. Whoever finds it can watch every payment in and out, but spend nothing. A share of a split backup leaves keys off, so one share alone does not show the balance.

"Only with the vault's passphrase" means the description is in the vault and that place keeps the vault's stick.
