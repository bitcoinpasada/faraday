# Faraday Companion

## What it keeps

Faraday Companion keeps three things on the phone. Received holds the files that came from the Faraday device or another app: PSBTs, finished transactions, wallet descriptions, account keys, files in the Faraday file envelope, and text you chose to keep. Wallet descriptions holds the descriptions you kept there, each under a name you give it. BitLocker keys holds the recovery keys Faraday Starter showed as codes.

Wallet descriptions and BitLocker keys open only after the phone's own unlock: its PIN, pattern, password or fingerprint, asked by the system each time either is opened, and again after the app has been in the background. A phone with no screen lock cannot keep them, and Received works as before.

## What it refuses

Seed words, SeedQR and CompactSeedQR codes, SLIP-39 shares, codex32 strings, extended private keys, private keys, encrypted backups, sealed vaults, KeePass databases and TOTP secrets are refused, whether they come by the camera, by Paste or from another app. The Companion names what it was and keeps nothing of it. Those stay on the Faraday device.

A web address is shown as text and is never opened. The one exception is the address of one of the website's guides to starting a computer from the stick, faradayoffline.app/start/ and the guide's name: the Companion opens its own copy of that page, so the website is never asked for it.

## No network access

An Android app can reach a network only if its manifest asks for the INTERNET permission. The system enforces this below the app, whatever its code does. The Companion asks for two permissions: the camera, and the unlock prompt. Anyone can list them from the APK file with Android's build tools:

aapt2 dump permissions faraday-companion-X.Y.Z-android.apk

F-Droid shows the same list on the app's page. About lists the permissions the phone has granted, read from the system.

Data leaves the Companion only when you press Share, which hands one file to an app you choose, or Save, which writes one file where you choose. That app may be online. BitLocker keys are never shared, saved or shown as a code.

## What a wallet description reveals

A wallet description is the wallet's public keys and how they are combined. With it, any wallet program can find every address of the wallet, and so its balance and its whole history. It cannot spend: spending needs the private keys, which a description does not hold.

## The first address

A kept description's record shows its first receive address. It tells two copies of a wallet apart: the same wallet has the same first address everywhere.

It is worked out by the same code as Faraday's, so it is not the check of the Spend tab's step 5. That check is two separate programs, Faraday and a wallet on the online computer, reading the same backup and showing the same first address.

## The BitLocker recovery key

If Windows started without asking for the recovery key, the guard worked as expected and the copy kept here is no longer needed. It can be deleted for privacy at any time: open the key and press Back in Windows, or Delete.

Faraday Starter shows the key again before its firmware steps the next time Faraday is set up on that computer, and it can be scanned in again then. The key stays valid until it is changed in Windows, whether or not it is deleted here.

## Nothing goes to a backup

The Companion is excluded from Android's cloud backup and from moving to a new phone. To move to a new phone, scan or share each thing across again. Uninstalling the app deletes everything it kept.

## Installing it

Android on Google-certified phones installs apps only from developers registered with Google, with an identity document: from 30 September 2026 in Brazil, Indonesia, Singapore and Thailand, and everywhere in 2027. Faraday is published under a pseudonym and is not registered. On a phone that asks, the Companion installs through Android's advanced flow, which needs developer options, a restart and a day's wait, or with adb from a computer. Phones running an Android without Google's certification install it as any other APK.
