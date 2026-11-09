# BIP 87 multisig path text

BIP 87 ("Hierarchy for Deterministic Multisig Wallets", Complete),
verbatim, from the bitcoin/bips repository at commit
`24e96e870fffaa257b465ce1f0370c14aac588e8`
(`https://raw.githubusercontent.com/bitcoin/bips/24e96e870fffaa257b465ce1f0370c14aac588e8/bip-0087.mediawiki`).
The BIP defines the path `m/87'/coin_type'/account'` for a multisig key,
with the script left to the descriptor, and carries no test vectors of its
own: its example is a descriptor template with placeholders.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `bip-0087.mediawiki` | 2026-11-09 | `c5e9806d644d4737166e3e840509969bd3a027bec8eb86ecceafbaf78b49b942` | nothing; the text the path is built from |

`faraday/faraday-core/tests/all/create_wallet.rs` checks a wallet made at
this path against rust-bitcoin's own derivation of the same test seeds,
which Faraday did not write.
