# Test vectors

- `wallets/`: the Wallets tab's QR conformance kit (mainnet, every seed a
  dummy): the ten BBQr parts of its five-input 3-of-5 PSBT, that PSBT in
  base64 in `psbts.txt` (the kit's own list), and the one-input PSBT as one
  code.
- `faraday-os-*`: written by Faraday OS's `lib/faraday/qr_transfer.py`
  (`envelope()` and `frames()`), so Faraday reads what Faraday OS sends: a
  text file and a public-key file, each as BBQr `J` in encoding `Z` (Faraday
  OS compresses whenever that is shorter, which base64 inside JSON always
  is).
