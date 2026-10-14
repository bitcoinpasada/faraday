# BitLocker guard: acceptance runs

The steps for `docs/STARTER.md` §6.6's runs 1 to 8, in a Windows 11 Pro
virtual machine, before batch W ships. Runs 1 to 6 test the extra-safe
path (BitLocker suspended); runs 7 and 8 test the default path, the
recovery key in hand and nothing suspended (F17); runs 7 and 8 are
optional, replaced by a check of the default page on Windows before
the Windows release (`docs/STARTER.md` §6.6). Each run starts from the same
saved state of the machine, follows the Starter on its screen, and is
recorded as the end of this file says. A run passes only when every
"Must see" line holds; one that does not stops the batch until the
cause is found and fixed in the code, and the run is done again from
the start.

The Home runs follow §6.6's own words: on a computer of the owner's,
with the owner, after the Pro runs pass, runs 1 and 2 of this list
only, never 3 or 5.

## The machine

- QEMU, q35, OVMF's Secure Boot build with Microsoft's keys in its
  variable store (`OVMF_CODE_4M.ms.fd`, a copy of `OVMF_VARS_4M.ms.fd`
  for this machine), a TPM 2.0 from `swtpm` whose state is kept across
  starts, no network. `tools/winvm/winvm.sh` runs exactly this, once it
  is committed; the commands below use it, and its README says how to
  set it up. Any other QEMU with the same parts serves.
- Windows 11 Pro, installed with a local administrator account, fast
  startup off (`powercfg /h off`), BitLocker on for C: with a TPM
  protector and a recovery password, the drive fully encrypted
  (`manage-bde -status C:` says "Fully Encrypted" and "Protection On").
  Keep the recovery password the install wrote (`out\recovery.txt` on
  the answer disk): run 5 needs it, and so does any run that goes wrong.
- A second account, a standard user, for run 4b.
- On the machine's answer disk or a second USB image: `Faraday Starter.exe`
  from `just faraday-windows-bin` with the release payload
  (`FARADAY_STARTER_PAYLOAD=out/payload/X.Y.Z/windows`), and nothing else of the
  Starter's.
- The stick image to start: `out/stick/faraday-x86_64-uefi.img`, given
  to the machine as a removable USB stick (`winvm.sh start --usb-stick
  …`). A second raw FAT image (`--usb-fat`) stands for a USB drive Save
  may write to.

Save the machine with BitLocker on and the Starter never run:

    winvm.sh powerdown; winvm.sh wait-off
    winvm.sh snapshot clean

Every run starts with `winvm.sh snapshot -a clean`.

## Before run 1: what the build is checked against first

The Starter was built against Microsoft's documentation and recorded
answers, with no Windows computer. Three of its readings are checked
here before anything else, from an administrator's command prompt in
the machine:

- [ ] `Faraday Starter.exe --record-bitlocker` into `C:\record-on.txt`
      (below). Its first
      line is the shell's `System.Volume.BitLockerProtection` for C:,
      read without elevation; with BitLocker on it must be `Some(1)`.
      The rest is `Win32_EncryptableVolume` as
      `faraday/shells/starter/tests/windows/*.wmi` records it: C: must be
      listed with `type 0 protection 1`, `GetConversionStatus` must
      answer `0 ConversionStatus=u32:1`, `GetKeyProtectors` must answer
      `0` and one protector ID. No line may hold a recovery password.
- [ ] After run 1's Suspend, the same command into `record-suspended.txt`:
      the shell's value must be `Some(5)` and C: `protection 0`, with
      `GetSuspendCount` answering `0 SuspendCount=u32:0` (suspended
      until resumed, as `Suspend-BitLocker -RebootCount 0` leaves it).
      If the shell's value for a suspended drive is anything but 5,
      `bitlocker::shell_protection` reads it wrong: stop and fix it.
- [ ] The key the Starter shows is the recovery password
      `manage-bde -protectors -get C: -Type RecoveryPassword` prints,
      digit for digit, and its key ID is the first eight hex digits of
      that protector's ID.

`Faraday Starter.exe` is a GUI-subsystem program: PowerShell's pipe
and `>` do not reliably capture what it prints. Run the recording as

    Start-Process -FilePath '.\Faraday Starter.exe' -ArgumentList '--record-bitlocker' -RedirectStandardOutput C:\record-on.txt -Wait -NoNewWindow

The recording's `DisableKeyProtectors` lines name its parameter
`DisableCount`, WMI's own name; PowerShell's `Suspend-BitLocker` calls
the same count `RebootCount`.

Copy `record-on.txt` and `record-suspended.txt` off the machine; they
replace the hand-made recordings once checked (the volume GUIDs are the
virtual machine's own and name nothing else).

## Run 1: through the Starter, Secure Boot back on

1. Start the machine (Secure Boot on). Sign in. Run `Faraday Starter.exe`,
   choose Install, install. Attach the second USB image.
2. Home: the card says **Do the BitLocker step** once a Faraday stick
   is attached; the checklist shows BitLocker "Protected". Open it,
   and choose **Extra safe: suspend BitLocker**.
3. **Show the recovery key**: the elevation prompt; Yes.
4. The page shows the key (eight groups), its key ID, the QR code, "Or
   take a photo of this screen.", **Save to …** naming the USB image,
   and **I have kept it**. Scan the QR code with a phone: the text is
   "BitLocker recovery key for <computer>, drive C:" and the key.
5. Save. Then detach the USB image: Save is gone. Attach only the stick
   image: Save stays gone.
6. **I have kept it**: the key is hidden; two groups are asked for. Type
   one wrong and press **Suspend BitLocker**: the page says that group
   does not match, and `manage-bde -status C:` still says "Protection
   On". Type both right; **Suspend BitLocker**; the elevation prompt;
   Yes.
7. Done's lines. Then, outside the Starter:
   `manage-bde -status C:` "Protection Off";
   `type %LOCALAPPDATA%\faraday\bitlocker-suspended` (the volume and
   `secure-boot on`, nothing else);
   `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v "Faraday Starter"`
   (the installed copy, `--remind`);
   `findstr /s /m /c:"<the key's first group>" %LOCALAPPDATA%\* %TEMP%\* %USERPROFILE%\Downloads\*`
   finds nothing.
8. Home: the card offers the firmware restart. Choose it; the sheet;
   **Restart**. The machine restarts into OVMF's setup. In a virtual
   machine the button is greyed: the Starter holds the restarts there
   (`docs/STARTER.md` §3.1a, `restart_holds`), so step 9 uses `winvm.sh`.
9. Secure Boot off there (Device Manager, Secure Boot Configuration,
   Attempt Secure Boot off, F10, reset), or `winvm.sh start
   --secure-boot off --usb-stick out/stick/faraday-x86_64-uefi.img`
   with the machine off. Start the stick from the boot menu: Faraday
   starts. Power it off from Faraday.
10. Secure Boot on (`winvm.sh start --secure-boot on`, stick detached).
    Windows starts.

Must see:

- [ ] **No recovery prompt** at step 10: Windows' sign-in screen.
- [ ] At sign-in the Starter opens on its BitLocker page by itself, with
      **Resume BitLocker** (the logon reminder).
- [ ] Home, opened again by hand, shows Resume BitLocker as the next
      step.
- [ ] **Resume BitLocker**, Yes at the prompt: "BitLocker resumed";
      `manage-bde -status C:` "Protection On"; the marker file and the
      Run value are gone.
- [ ] One more restart, Secure Boot on: no recovery prompt.
- [ ] The saved file on the USB image holds the key and nothing names a
      file of the Starter's beside it.

## Run 2: Windows started with Secure Boot still off

Steps 1 to 9 of run 1, then start Windows with Secure Boot off.

Must see:

- [ ] **No recovery prompt.**
- [ ] `manage-bde -status C:` still "Protection Off".
- [ ] At sign-in nothing opens (wait a minute; screenshot the desktop).
      The Run value is still there.
- [ ] Faraday opened by hand: Home's next step is **Turn Secure Boot
      back on**; Resume BitLocker reads "Needs Secure Boot on" and is
      not offered, on Home and on the BitLocker page.
- [ ] Shut down, Secure Boot on, start: no recovery prompt; at sign-in
      the Starter opens with **Resume BitLocker**; resuming works as in
      run 1.

## Run 3: the firmware's keys reset before Secure Boot goes back on

Steps 1 to 9 of run 1. In OVMF's Secure Boot Configuration, set the
keys to their defaults (Reset Secure Boot Keys, or the custom mode's
default keys), then Secure Boot on, then start Windows.

Must see:

- [ ] **No recovery prompt.**
- [ ] Resume BitLocker offered at sign-in; resuming works; one more
      restart: no recovery prompt.

## Run 4: elevation declined

a. On the administrator's account: steps 1 and 2 of run 1 (Extra safe
   chosen), then **Show the recovery key** and **No** at the prompt. Again, through to step
   6, and **No** at Suspend's prompt.

b. Install nothing; sign in as the standard user, run `Faraday Starter.exe`,
   open the BitLocker page, **Show the recovery key**: the prompt asks
   for an administrator's password; Cancel.

Must see:

- [ ] a: "Administrator permission was declined. Nothing was changed."
      and **Show the recovery key** offered again; after the second
      No, the same line and Suspend offered again.
- [ ] b: "An administrator of this computer has to do this step." and
      the step offered again.
- [ ] In both: the Start page's firmware buttons are not offered; the
      card says **Do the BitLocker step**; `manage-bde -status C:`
      "Protection On"; no marker file, no Run value.

## Run 5: without the Starter, the prompt appears

From `clean`: shut down, Secure Boot off, start Windows. The Starter is
never run.

Must see:

- [ ] **The BitLocker recovery screen**, asking for the key with its key
      ID. This shows the machine reproduces the problem, so runs 1 to 3
      prove something. Type the recovery password
      (`winvm.sh type …`) to finish, then restore `clean`.

## Run 6: Uninstall while suspended

a. Steps 1 to 7 of run 1 (Secure Boot on). Installed, **Uninstall**.
   Then the same from Windows' Settings, Apps, Installed apps, Faraday Starter,
   Uninstall, which runs `Faraday Starter.exe --uninstall`.

b. Steps 1 to 9 of run 1, start Windows with Secure Boot off, open the
   installed Faraday, Installed, **Uninstall**.

Must see:

- [ ] a: the sheet says "BitLocker stays suspended until it is resumed."
      and offers **Resume BitLocker**, which resumes (Protection On).
      Uninstalling instead, without resuming: the Starter, its shortcut,
      its uninstall entry, the marker and the Run value are gone, and
      `manage-bde -status C:` still says "Protection Off".
- [ ] b: the sheet says "BitLocker stays suspended until it is resumed:
      Control Panel, BitLocker Drive Encryption, Resume protection."
      and offers no Resume. After Uninstall, the marker and the Run
      value are gone and protection is still suspended. Then Secure
      Boot on, start Windows (no recovery prompt), and Control Panel's
      Resume protection resumes it.

## Run 7 (optional): the default path, the key in hand

1. Start the machine (Secure Boot on). Sign in. Run `Faraday Starter.exe`
   (not installed). Attach the stick image.
2. Home: the card says **Do the BitLocker step**; the checklist shows
   BitLocker "Protected". Open it. The page shows "Also kept in:
   Microsoft account: aka.ms/myrecoverykey", **Show the recovery key**,
   **I have my recovery key** and **Extra safe: suspend BitLocker**.
3. **Show the recovery key**: the elevation prompt; Yes. The key, its
   key ID, the QR code and "Or take a photo of this screen."; no **I
   have kept it**, no groups asked for. Compare the key with
   `manage-bde -protectors -get C: -Type RecoveryPassword`.
4. **I have my recovery key**: the key leaves the screen; the page reads
   "Recovery key kept". Outside the Starter: `manage-bde -status C:`
   still "Protection On"; no `%LOCALAPPDATA%\faraday\bitlocker-suspended`;
   no Run value.
5. Home: BitLocker reads "Recovery key kept"; the card offers the
   firmware restart (greyed in a virtual machine, as in run 1 step 8).
   Choose it; the sheet; **Restart**.
6. Secure Boot off in OVMF's setup, save, and on the restart that
   follows choose the stick from the boot menu before Windows starts
   (or `winvm.sh start --secure-boot off --usb-stick
   out/stick/faraday-x86_64-uefi.img` with the machine off). Faraday
   starts. Power it off from Faraday.
7. Secure Boot on (`winvm.sh start --secure-boot on`, stick detached).
   Windows starts.

Must see:

- [ ] Step 2 to 4: nothing changed on the computer: "Protection On"
      throughout, no marker, no Run value.
- [ ] **No recovery prompt** at step 7: Windows' sign-in screen.
- [ ] `manage-bde -status C:` "Protection On".
- [ ] Faraday Starter opened again: BitLocker reads "Protected" (the
      confirmation was for that session only).

## Run 8 (optional): the default path, Windows started with Secure Boot off

Steps 1 to 5 of run 7. Secure Boot off in OVMF's setup, save, and let
Windows start with the stick not chosen (or `winvm.sh start
--secure-boot off` with no stick).

Must see:

- [ ] **The BitLocker recovery screen**, with the key ID Faraday Starter
      showed at step 3 of run 7.
- [ ] The key Faraday Starter showed unlocks it (`winvm.sh type …`):
      Windows starts. Then Secure Boot on, start Windows: no recovery
      prompt. Restore `clean`.

## What to record

For each run, in the completion report:

- the date, the build (`Faraday Starter.exe --version`, its SHA-256) and the
  snapshot it started from;
- each step as done, with anything that differed from this file;
- a screenshot of Windows' first screen after every start
  (`winvm.sh screenshot`), named by run and start;
- screenshots of the Starter's BitLocker page at each step it changes;
- `manage-bde -status C:` before the run, after Suspend and after
  Resume (runs 7 and 8: before the run and after I have my recovery
  key); the marker's text and the Run value where the run checks
  them;
- the two `--record-bitlocker` files of the first check;
- each "Must see" line, passed or not.

No recovery password goes into the report or any file outside the
machine: write "matched" where it was compared.
