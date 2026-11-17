# Read a Bitcoin Core wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CoreWallet`
**Key:** core-wallet

This tool reads and checks the `wallet.dat` of a Bitcoin Core descriptor wallet, which is a SQLite database. It opens from Tools and returns there, and only reads the file; the private keys are checked but not shown.

## File

**Key:** core-wallet.file

From Tools it opens on the wallet file when one waits in Files, and on a list when several do. With none, **No Bitcoin Core wallet file waits in Files** comes with **Open Files** and the stick (a wallet file cannot come by QR, so there is no Scan). The result ends with **Another** and **Done**.

The wallet is one SQLite or Berkeley DB file. **Passphrase, if the wallet is encrypted** is the wallet's own passphrase, used only for this read. **Read** appears once you have chosen a file.

A legacy Berkeley DB wallet, a file in the write-ahead-log form, a file with its journal, log or lock beside it in Files, a wallet of another network than the one Faraday is on, and a descriptor with a key that has no origin are refused with the reason in one line.

## Result

**Key:** core-wallet.result

**Nothing read yet** shows until you read a file. The verdict is **The wallet is whole**, **The wallet is not whole** or **The passphrase does not open the wallet**. A failed record is named, and the wallet is then not whole.

**Network**, **Written by Bitcoin Core**, **Encryption**, **Private keys**, **Labels** and **Transactions** are the facts about the wallet; labels and transactions are counted, not read. **Private keys** says how many were checked, or that none were because no passphrase was given or the passphrase is wrong. Each **Descriptor** is listed with its range and next index, and **Active, receive** and **Active, change** mark the ones in use. **Records not read** and **Records not known** list the rest.

**Take to Wallets** is offered for each output type that has an active receive descriptor, with its change descriptor when there is one. It adds the wallet to Wallets with its key origins.
