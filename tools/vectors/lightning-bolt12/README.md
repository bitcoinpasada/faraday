# BOLT #12 offer vectors

`offers-test.json` (53 offers, valid and not) and `format-string-test.json`
(the text form: case and `+`) are byte for byte the files `bolt12/offers-test.json`
and `bolt12/format-string-test.json` of <https://github.com/lightning/bolts>,
fetched 2026-11-10 from master at commit
`e20a924a24f69dd071d70091039f18d4e055b36f` ("BOLT 12: payer proofs", the last
commit to change the `bolt12/` folder). Each offer entry has a `description`,
`valid`, the `bolt12` text and, for a valid one, its `fields` (type, length,
hex); each format entry has a `comment`, `valid` and the `string`.

`signature-test.json` and `payer-proof-test.json` of the same folder are for
the forms Faraday does not read (invoice requests, invoices, payer proofs) and
are not kept here. The `faraday-codec` tests (`tests/bolt12.rs`) read both
files; the fuzz target `bolt12` has them as seeds.

`iso4217-list-one.xml` is the ISO 4217 currency list (`list-one.xml`, published
2026-09-17) of the maintenance agency, SIX, byte for byte from
<https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-one.xml>,
fetched 2026-11-10. `faraday_codec::bolt12::CURRENCIES` is every code in it with
a numeric minor unit (metals, XDR, XTS, XXX and the like say N.A. and are left
out); `tests/bolt12.rs` requires the table to match the file.
