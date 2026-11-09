# BOLT #11 invoice vectors

`invoice-test.json` is the machine-readable list of BOLT #11's examples
(its "Examples" and "Examples of Invalid Invoices"), byte for byte the file
`bolt11/invoice-test.json` of <https://github.com/lightning/bolts> (master,
fetched 2026-11-09). Each entry has a `description`, `valid` and the
`invoice` text. Every example was signed with the private key
`e126f68f7eafcc8b74f54d269fe206be715000f94dac067d1c04a8ca3b2db734`, whose
node key is `03e7156ae33b0a208d0744199163177e909e80176e55d97a2f221ede0f934dd9ad`.
One exception: the "high-S" example carries the recovery id of the low-S
form of its signature, so recovering the key from it gives
`02d0139ce7427d6dfffd26a326c18be754ef1e64672b42694ba5b23ef6e6e7803d`, not that node key.
The cross-check's `lightning` area (`tools/crosscheck/area_lightning.py`)
and the `faraday-codec` tests (`tests/bolt11.rs`) read it.
