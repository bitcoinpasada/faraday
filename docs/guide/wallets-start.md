# Wallets: start

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `WalletsStart`
**Key:** wallets-start

The first page of Wallets: the wallets loaded and every way to bring one in. Choosing a wallet opens it.

## Your wallets

**Key:** wallets-start.wallets

Wallets are listed with the ones this computer can spend first, each with its shape and a marker: **Can spend · 2 of 3** (the numbers vary), **1 of 3 here** for keys present but short of the quorum, **Watch only**, **Does not match its key**, or **2 of 3 here · another key can spend** where a key outside the quorum can also spend. With none loaded the list reads **None loaded**. The wide page shows four, then **+ 2 more** (the number varies), which opens the next wallet.

## Ways in

**Key:** wallets-start.ways

With no wallet, no open vault and nothing unfinished, four choices stand side by side: **Make a new wallet** (**Single key or multisig**), **Restore an existing wallet** (**From its backups**), **Spend from a backup, step by step** (**A stick, words, or paper**) and **Add a key** (**Keys first, wallets later**). Otherwise only **Spend from a backup, step by step** and **Add a key** are offered here.

The first locked vault in Files is named with its file name and that it is locked, and **Unlock** opens it.

## Keys without a wallet

**Key:** wallets-start.keys

On a small panel, once a wallet is loaded, each key that no wallet uses is listed by its fingerprint with **Make a wallet from this key**.
