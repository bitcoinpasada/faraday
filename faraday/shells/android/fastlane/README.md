# F-Droid metadata

`metadata/android/en-US/` is F-Droid's fastlane layout (`docs/COMPANION.md`
§8): the title, the short and full descriptions, and four screenshots
rendered by `just companion-snap 1080x2340` (home, send, received, wallet
descriptions). F-Droid reads it from the tag it builds.

Opening the merge request to F-Droid's data repository is the owner's.

## TODO, both need the APK certificate

The owner makes the APK signing key in Faraday and commits its certificate
as `faraday/companion-cert.pem` (`docs/APK-SIGNING.md` §12). Then:

- **`AllowedAPKSigningKeys`** goes in F-Droid's data repository, in the
  app's metadata file, not in this repository. It is the certificate's
  SHA-256 as lower-case hex, from
  `openssl x509 -in faraday/companion-cert.pem -outform der | sha256sum`.
- **The fingerprint paragraph of the top `README.md`'s "Verifying a
  release"** (`docs/APK-SIGNING.md` §9): the same fingerprint in full, and
  `apksigner verify --print-certs faraday-companion-X.Y.Z-android.apk`,
  whose "Signer #1 certificate SHA-256 digest" prints it.
