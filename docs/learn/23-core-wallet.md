# Bitcoin Core wallets

## What is read

Read a Bitcoin Core wallet takes the `wallet.dat` of a descriptor wallet, the kind Bitcoin Core has made since version 23, kept in a SQLite database. Choose the file in Files and press Read. The result gives the network, the version of Bitcoin Core that wrote it, whether it is encrypted, each descriptor with its key origins, which descriptors are active for receiving and for change, the range and next index of each, and how many labels and transactions the file holds. The descriptors are public: Take to Wallets loads them as any descriptor with its origins is loaded.

Labels and transactions are counted, not read. A record of a kind Faraday does not know is listed by name and count.

## What is checked

Each record is checked again. The network is the one the file's header names. Each descriptor parses, its checksum is right and its id is the hash of its text as Bitcoin Core makes it. The active descriptors are ones the file holds. Each private key must give the public key its record names, and its stored hash must be right. A record that fails is named with what failed, and the wallet is not called whole.

An encrypted wallet shows what Bitcoin Core keeps in the clear without the passphrase: the descriptors, the active ones, the labels and the master key's method and rounds. Its keys are then not checked, and the screen says so. With the passphrase the master key is opened and each key is checked as above; a wrong passphrase is said as such.

## What is not done

The file is read and never written: not a byte of it changes, and no copy is made. The private keys are checked and wiped; they are not shown, not loaded as a key of the session and not written anywhere. The passphrase is kept only for the read. Faraday reads the part of SQLite that Bitcoin Core writes, with a reader of its own that refuses a file it does not know.

## What is refused

A legacy wallet, one of the Berkeley DB kind: Bitcoin Core's `migratewallet` makes a descriptor wallet of it. A SQLite file whose flags lack descriptors, or that holds legacy records. A wallet of another network than the one Faraday is on. A descriptor with a key that has no origin. A file in the write-ahead-log form, or with a journal, log or lock beside it in Files, since it is in use or holds data not yet in it. A file that is damaged or not as Bitcoin Core writes it.
