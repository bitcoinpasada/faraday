# LND's aezeed, AEZ v5, and the two Lightning node keys

`docs/PLANNING.md` §16.116. `osk_bip::aezeed` and `osk_bip::aez` are
written from the sources named here, never from a summary. Each file
read is listed with the commit it was read at and its SHA-256, so a
later change to a source shows up as a change to this page.

Only one thing is vendored: `aez-testdata/`, the seven JSON vector files
the AEZ reference implementation publishes, copied byte for byte and
read by `core/osk-bip/tests/aez.rs`. Everything else was cloned to read
and nothing else is kept.

| Source | URL | Commit | Read |
|---|---|---|---|
| LND | <https://github.com/lightningnetwork/lnd> | `88959aec3d7a9d11a509714d9bee97c29ee02269` | 2026-09-19 |
| Yawning Angel's AEZ | <https://github.com/Yawning/aez> | `e49e68abd344eba4a888cee91bc7370df2d14e27` | 2026-09-19 |
| ldk-node | <https://github.com/lightningdevkit/ldk-node> | `43dcb60f01824173069fef80276a3e802baeed89` | 2026-09-19 |
| rust-lightning | <https://github.com/lightningdevkit/rust-lightning> | `ccd7f501a53ab325f65c9dc96577246de32c65f2` | 2026-09-19 |

| File read | SHA-256 |
|---|---|
| `aezeed/cipherseed.go` (LND) | `3b50ef5e48b07355ee13a85c24ddab6966844739920f389016f7be0f1f7ddf20` |
| `aezeed/cipherseed_test.go` (LND) | `a6fd6c2c1394292cabeb9fd34213f21c2351c4750dba346c16844721e816d659` |
| `aezeed/wordlist.go` (LND) | `879adfbee0360133c8e7d5a67ade6fdd7fdd90be990a7413401b6b0011fbf9a3` |
| `keychain/derivation.go` (LND) | `dd33f649a16a7687a8f065b7856e76c274b4accfb824a074db089d921c4a079c` |
| `aez.go` (AEZ) | `6d16f4332f86d34baefcfcb0359d1dd63e32318ffcb5887d63f5d6a11f8606de` |
| `round_vartime.go` (AEZ) | `188153bd5d81e488f326a4b6c1ed6341afc9ebce02ea9ab95ee2237d6a2d5368` |
| `aez_test.go` (AEZ) | `13e03951c05ca29ab3902a7ea9097ba3686c5fb50e73fdcc33ead860f343f108` |
| `src/builder.rs` (ldk-node) | `fac69acfd7b43e135005905b78c6ee288ded626527d32deec90427c9e8fdf3e4` |
| `src/entropy.rs` (ldk-node) | `c9bdcc0980e447541b7a72d6d5dd45749c6dde78bc376e3403ff075322d1df7d` |
| `lightning/src/sign/mod.rs` (rust-lightning) | `39e1be56c26c2c9c593e69bba0066bf813a7056277e28951acc3f72ed87d7b77` |

| Vendored vector file | SHA-256 |
|---|---|
| `aez-testdata/encrypt.json` | `7290077373d9222a6b5a7382029e794be376e4810320ad0cdc6f366ca737bda5` |
| `aez-testdata/encrypt_16_byte_key.json` | `7d9253a7838d1c57fd7353db9f615be53ba8e6571ab4c1122cd8b6493603c0d4` |
| `aez-testdata/encrypt_33_byte_ad.json` | `4e4d6d52e28b2cc61576c3e7ecccfda7455bc8de4c2218a355db5af5c3a752f4` |
| `aez-testdata/encrypt_no_ad.json` | `5bed304851e87ca8f882302ee7fd65742d8cac6d5e71a2684cf3c315b24b85af` |
| `aez-testdata/extract.json` | `ec52a5587e1f1eedb136a2c62bfd831cce1a0bd8714b3eaafb86bcced9be8e30` |
| `aez-testdata/hash.json` | `0524ae265ecdae10da4191f7df574c644ddd386d84637f4386496246d5854d1e` |
| `aez-testdata/prf.json` | `133d5cb126b122b525f4c4179f732ac983abf34a80718d86b45a8959a3480e3c` |

## 1. The cipher seed's layout

`aezeed/cipherseed.go`:

```go
	// EncipheredCipherSeedSize is the size of the fully encoded+enciphered
	// cipher seed. …
	//  * 1 byte version || 23 byte enciphered seed || 5 byte salt || 4 byte checksum
	//
	// With CipherSeedVersion we encipher as follows: we use
	// scrypt(n=32768, r=8, p=1) to derive a 32-byte key from an optional
	// user passphrase. We then encipher the plaintext seed using a value
	// of tau (with aez) of 8-bytes (so essentially a 32-bit MAC).
	EncipheredCipherSeedSize = 33
```

The comment says tau is 8 bytes; the constant beside it says 4, and 4
is what the code passes:

```go
	CipherTextExpansion = 4
```

and the whole of the deciphering:

```go
	copy(salt[:], cipherSeedBytes[saltOffset:saltOffset+SaltSize])
	cipherSeed := cipherSeedBytes[1:saltOffset]
	checksum := cipherSeedBytes[checkSumOffset:]

	freshChecksum := crc32.Checksum(
		cipherSeedBytes[:checkSumOffset], crcTable,
	)
	if freshChecksum != binary.BigEndian.Uint32(checksum) {
		return plainSeed, salt, ErrIncorrectMnemonic
	}

	key, err := scrypt.Key(pass, salt[:], scryptN, scryptR, scryptP, keyLen)
	…
	ad := extractAD(cipherSeedBytes)

	plainSeedBytes, ok := aez.Decrypt(
		key, nil, [][]byte{ad[:]}, CipherTextExpansion, cipherSeed, nil,
	)
```

with `crcTable = crc32.MakeTable(crc32.Castagnoli)`, the associated data
`version ‖ salt`, and an empty passphrase replaced:

```go
	defaultPassphrase = []byte("aezeed")
```

The plaintext is `1 byte internal version ‖ 2 bytes birthday ‖ 16 bytes
entropy`, and the birthday is counted in days:

```go
	// BitcoinGenesisDate is the timestamp of Bitcoin's genesis block.
	BitcoinGenesisDate = time.Unix(1231006505, 0)
```

## 2. The word list is BIP-39's English list

`aezeed/wordlist.go` says so, and it is true to the byte:

```go
// englishWordList is an English wordlist that's used as part of version 0 of
// the cipherseed scheme. This is the *same* word list that's recommend for use
// with BIP0039.
var englishWordList = `abandon
```

The 2048 words of that literal, each followed by a newline, hash to
`2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda`,
which is the digest `core/osk-bip/src/wordlists/english.rs` already
records for the BIP's own file. So the tool types an aezeed on the
English wordlist this device already carries.

## 3. LND's node key

`keychain/derivation.go`:

```go
//   - m/1017'/coinType'/keyFamily'/0/index
	BIP0043Purpose = 1017
	// KeyFamilyNodeKey is a family of keys that will be used to derive
	// keys that will be advertised on the network to represent our current
	// "identity" within the network.
	KeyFamilyNodeKey KeyFamily = 6
```

and the coin type, `keychain/btcwallet.go`:

```go
	CoinTypeBitcoin uint32 = 0
	CoinTypeTestnet = 1
```

which `chainreg/chainparams.go` gives mainnet and every other chain
respectively. The BIP-32 seed is the entropy itself, with no stretching:
`lnwallet/btcwallet/config.go`'s `HdSeed` is what `aezeed`'s entropy is
handed to.

### The vector, computed here once

Go is not a toolchain this repository builds with. It was installed
locally for this pass and used once, at the LND commit above, with the
package's own vectors and btcd's `hdkeychain`:

```
entropy=81b637d86359e6960de795e41e0b4cfd coin=0
  priv=fcaa0ababb2377f81f3bdbd8b4e2436fe8968b0a68b0b129c66ff848bf6e1b48
  pub=024c7005923a074fd38b16ace7be4914ec9f929717692bfb585019be13290c9b1d
entropy=81b637d86359e6960de795e41e0b4cfd coin=1
  priv=c416c16490c6f9680e294e25468f384b7161b57b4a05279c145ec05475076b5a
  pub=0226594d21c0862a11168ab07cdbc15e7c7af5ee561b741259e311f1614f4df3b7
```

`lncli` publishes no worked example of a node key from a seed, so this
is the vector `core/osk-bip/src/aezeed.rs` asserts.

## 4. LND's own vectors are not decodable by LND

`aezeed/cipherseed_test.go` ends with:

```go
func init() {
	// For the purposes of our test, we'll crank down the scrypt params a
	// bit.
	scryptN = 16
	scryptR = 8
	scryptP = 1
}
```

`scryptN` is a package variable, so that `init()` applies to the whole
test binary, `version0TestVectors` included. The twenty-four words those
vectors state were therefore enciphered under `scrypt(N = 16)`, and a
released LND, which uses `N = 32768`, cannot read them. They are
vectors of the scheme, not of the product.

`osk_bip::aezeed::decode_at_cost` exists for exactly this: the published
words are checked at `N = 16`, and the same three seeds — same entropy,
same salt, same birthday, same passphrases — re-enciphered at the
production cost are checked through `decode`. Those three were produced
here by running LND's own package with `scryptN` set back:

| Passphrase | Internal version | Birthday | Words |
|---|---|---|---|
| (none) | 0 | 0 | `above judge emerge veteran reform crunch system all snap please shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body` |
| `!very_safe_55345_password*` | 0 | 3365 | `absorb century submit father path glove gloom super divert garden ice mirror wisdom grass dice kit ugly castle success suggest drink monster congress flight` |
| `hello` | 1 | 3365 | `ability toilet excite swear ostrich model long squeeze solid memory kit pepper arena equal spider beauty satoshi romance success suggest drink photo already biology` |

Each decodes back to the entropy `81b637d86359e6960de795e41e0b4cfd`
under LND's own `ToCipherSeed`.

## 5. AEZ v5

The whole of the key extraction, `aez.go`:

```go
func extract(k []byte, extractedKey *[extractedKeySize]byte) {
	if len(k) == extractedKeySize {
		copy(extractedKey[:], k)
	} else {
		h, err := blake2b.New(extractedKeySize, nil)
		…
		h.Write(k)
		tmp := h.Sum(nil)
		copy(extractedKey[:], tmp)
```

`extractedKeySize` is 48, and LND's key is scrypt's 32 bytes, so every
aezeed decode runs BLAKE2b with a 48-byte digest. That is why
`osk_crypto::blake2b` exists.

The round function, `round_vartime.go`, is AES's with MixColumns kept in
the last round and no key added before the first:

```go
func (r *roundVartime) AES4(j, i, l *[blockSize]byte, src []byte, dst *[blockSize]byte) {
	xorBytes4x16(j[:], i[:], l[:], src, dst[:])
	r.rounds(dst, 4)
}
…
	// Skip adding the initial round key.
	…
	// Always do MixColumns.
	for r := 0; r < rounds; r++ {
```

and the two key schedules:

```go
	// AES10
	copy(r.aes10Key[0:], keys[:])  // I J L
	copy(r.aes10Key[12:], keys[:]) // I J L
	copy(r.aes10Key[24:], keys[:]) // I J L
	copy(r.aes10Key[36:], iK)      // I

	// AES4
	copy(r.aes4Key[0:], jK) // J
	copy(r.aes4Key[4:], iK) // I
	copy(r.aes4Key[8:], lK) // L
```

`aes4Key` is sixteen words and only twelve are written, so the fourth
round key is all zeros. `aez.go`'s `aezHash`, `aezPRF`, `aezCore` and
`aezTiny` are ported line for line into `core/osk-bip/src/aez.rs`; a
23-byte ciphertext — which is what an aezeed is — takes the `aezTiny`
branch with an odd length, so the half-byte shift in it is on the path
every decode runs.

## 6. ldk-node's node key

`src/entropy.rs`:

```rust
	pub fn from_bip39_mnemonic(mnemonic: Mnemonic, passphrase: Option<String>) -> Self {
		let mnemonic = maybe_deref(&mnemonic);
		match passphrase {
			Some(passphrase) => Self(mnemonic.to_seed(passphrase)),
			None => Self(mnemonic.to_seed("")),
		}
	}
```

`src/builder.rs`, where the 64-byte seed becomes LDK's 32-byte one:

```rust
	let xprv = bitcoin::bip32::Xpriv::new_master(config.network, &seed_bytes).map_err(|e| {
	…
	let ldk_seed_bytes: [u8; 32] = xprv.private_key.secret_bytes();
	let keys_manager = Arc::new(KeysManager::new(
		&ldk_seed_bytes,
```

and `lightning/src/sign/mod.rs`, where that becomes the node secret:

```rust
		const NODE_SECRET_INDEX: ChildNumber = ChildNumber::Hardened { index: 0 };
		…
		match Xpriv::new_master(Network::Testnet, seed) {
			Ok(master_key) => {
				let node_secret = master_key
					.derive_priv(&secp_ctx, &NODE_SECRET_INDEX)
					.expect("Your RNG is busted")
					.private_key;
```

So it is two master keys in a row: one over the BIP-39 seed, one over
the first one's private key, and `m/0'` of the second. Neither is ever
serialised, so the network the builder passes changes nothing — LDK
itself hard-codes `Network::Testnet` at the second step.

### The vector, computed here once

`lightning` 0.2.6's `KeysManager::get_node_id`, over the BIP-39 phrase
`abandon abandon abandon abandon abandon abandon abandon abandon
abandon abandon abandon about` with no passphrase:

```
ldk_seed=1837c1be8e2995ec11cda2b066151be2cfb48adf9e47b151d46adab3a21cdf67
node_secret=fd0afbf5d42fd797b85344291a2e5867f6a3f5a1e92f8ba2d030d804188efd8e
node_id=027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c
```

with `Network::Bitcoin` and `Network::Testnet` giving the same three
values.
