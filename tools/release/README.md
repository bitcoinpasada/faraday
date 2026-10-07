# Release signing

`pubkey.asc` is the OpenSigner release public key, exported with

```
gpg --armor --export <key-id> > tools/release/pubkey.asc
```

It is a public key: it verifies releases and signs nothing. Its
fingerprint is printed in `docs/VERIFY.md` and in the repository
`README.md`, and those two are what a user checks the imported key
against.

The private half is never on a build machine's disk and is never
reachable from an agent session. Signing goes through
`opensigner-release-sign`, a command the machine provides on `PATH` and
that this repository does not carry. Its contract:

- It takes one argument, the path of a manifest.
- It writes a detached, ASCII-armoured signature at `<manifest>.asc`,
  beside the manifest, made with the release key.
- It exits non-zero if it cannot, and writes no signature file.

How it reaches the key is that machine's business.

`just release-sign VERSION` calls it and then checks the result against
`pubkey.asc` with `gpg --verify`, so a signature made with the wrong key
fails there rather than on a user's machine.
