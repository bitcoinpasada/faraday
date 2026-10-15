# Faraday Companion: the Android shell

The Kotlin shell of Faraday Companion (`docs/COMPANION.md`), copied from
upstream's `opensigner/shells/android` at d26b84e. It drives the core
(`faraday/faraday-companion`) through `libfaraday_companion.so`
(`faraday/faraday-companion-ffi`), blits the frame the core draws, and
does for it what only the platform can: the camera, the document picker
and creator, the clipboard's read for Paste, the received files' store,
the kept store behind the phone's unlock, Share and the share target, the
screen kept on while a code shows.

Platform API and the Kotlin standard library only: `dependencies {}` is
empty, and the manifest asks for `CAMERA` and `USE_BIOMETRIC` and nothing
else. `tools/lint-companion-android.sh` (in `just lint`) checks both,
that the manifest declares only the activity and the unexported share
provider with the launcher's and SEND's filters, and that the Kotlin
opens no link and writes no clipboard;
`just companion-apk` checks the built APK's permissions with
`aapt2 dump permissions`.

    just companion-lib    the core for arm64-v8a and x86_64, into app/src/main/jniLibs
    just companion-apk    out/android/faraday-companion-debug.apk
    just companion-run    on the emulator, with a screenshot in out/android/companion/
    just companion-icon   the launcher icon drawn again from Faraday's mark

## Kept from upstream

The one Activity (portrait, `singleTask`, every configuration change
declared); the view that blits the core's frame and feeds it touches and
Back (`CompanionView.kt`, from `SignerView.kt`); `Camera.kt` unchanged but
for its package and its thread's name; the document picker and creator;
`FLAG_SECURE`; `allowBackup` false and the data-extraction rules
excluding every domain; minSdk 26, targetSdk 36, the two ABIs, `vcsInfo`
and the dependency blob off, the pinned build tools and NDK, the Gradle
wrapper and the two plugins; the Keystore wrapping-key handling of
`KeptSecret.kt`, as `ReceivedStore.kt`.

## Left out

- Everything of `KeptSecret.kt` that keeps a signing key: the MAC key,
  StrongBox, key attestation and the verified-boot reading, the kept
  blob. The received store's key is a plain AES-GCM Keystore key that
  needs no unlock; the kept store's prompt follows upstream's
  `secureMac`, with a cipher in place of the MAC.
- The secure element's MAC, the tier, entropy, the settings file, the
  vibrator, the total-memory reading.
- The clipboard write ("Copy").
- `BuildConfig`: the version About shows comes from the package manager.
- OpenSigner's look: its name, black window, orange key icon. The window
  and the system bars are Faraday's page colour in the theme on show, and
  the launcher icon is Faraday's mark on its dark tile.

## Added

- The received files' store, AES-256-GCM under its own Keystore key, in
  the app's private files (`ReceivedStore.kt`); a store that cannot be
  opened is handed to the core as one it cannot read, so it is not written
  over until the person deletes it.
- The theme chosen on About, kept in the app's private files.
- The picked file's display name, handed to the core with its bytes.
- The wall clock at start and at every return to the foreground, and
  About's facts from the package manager: the version name and code, the
  signing certificate's SHA-256 and each permission asked for with whether
  it is granted.
- `FLAG_KEEP_SCREEN_ON` while a code shows.

- Wallet descriptions (`KeptStore.kt`, §7): the store
  in AES-256-GCM under a random data key, the data key wrapped by a
  Keystore key bound to the phone's unlock (Android 11 and later: every
  use, strong biometric or the device credential, through `BiometricPrompt`
  with the cipher as its `CryptoObject`; Android 8 to 10: 30 seconds after
  the system's confirm-credential screen). One prompt opens the store and
  the data key stays in memory until `onStop`, which wipes it and tells the
  core `Locked`, unless the system's prompt is what stopped the activity.
  The store's version and the count Home shows while locked sit beside
  it, unencrypted; a store of version 1 (which held BitLocker keys) is
  deleted at the first start.
- Share (§6.3): `ShareProvider.kt`, a platform `ContentProvider`, not
  exported, serving the one file from memory under a random address, with
  a read grant for that address alone; revoked when the person comes back
  to the app, and when the activity ends.
- The share target (§6.3): `ACTION_SEND` for one file or text; only the
  stream's bytes and display name, or the text, are read from the intent.
