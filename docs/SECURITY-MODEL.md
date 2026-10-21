# Security model

**Kind:** Reference
**Status:** Built

What runs on a Faraday stick or Pi, as whom, what each part can reach,
and what Faraday does not defend against. `docs/AUDIT.md` says which
crate computes which cryptography.


## One program, not a desktop

What is on the screen is not a desktop environment. There is no X11, no
Wayland, no window manager, no GPU driver, no toolkit and no browser
engine. The firmware sets up a framebuffer, the kernel hands it over, and
Faraday, one Rust program, lays out every screen, rasterises its own
fonts and writes the pixels into that framebuffer. Keys, clicks and
touches arrive as raw kernel input events, read per device.

The whole running system is the Linux kernel, BusyBox init with a short
boot script, and four Rust programs:

| Program | Runs as | Does |
|---|---|---|
| `faraday` | its own unprivileged user | The application. Holds every secret. |
| `faraday-disk` | a second unprivileged user | Reads and writes FAT on sticks and cards. Holds no key or passphrase: files pass through it to and from a stick, and are wiped once passed. Cannot read the application's memory or files. |
| `faraday-grant` | its own user, with `CAP_CHOWN` only | Hands a stick's partitions to `faraday-disk` and authorises USB interfaces, deciding from `/sys` alone. Reads nothing from a stick. |
| `faraday-boot` | a fourth unprivileged user | Upgrades a Faraday stick: copies the boot partition of the stick Faraday started from, raw, over another stick's. Has a boot partition only while the application is on its Upgrade screen, reads no FAT, and is never handed bytes to write. |

No process can trace another, and each user's processes are invisible
to the others in `/proc`.

## When root is used

- **At boot.** The kernel starts BusyBox init as root. The boot script
  mounts `/proc`, `/sys` and `/dev` with nothing on them executable,
  sets the kernel's ptrace restriction, hides kernel addresses, sets the
  USB authorisation policy, creates the RAM directories and pipes the
  programs use, and starts the other three programs. It mounts no
  storage.
- **`faraday-grant`** starts as root and, before it reads anything,
  drops to its own user keeping only `CAP_CHOWN`, empties its capability
  bounding set and sets no-new-privileges.
- **After boot**, the root processes are init and three shell loops: one
  restarts the application under its user each time it locks, the others
  restart `faraday-disk` and `faraday-boot` under their users if they
  exit. When the
  application ends, init powers the machine off. None of these reads
  anything from a stick, a camera, a keyboard or a file.
- **Nobody can become root.** There is no login prompt, no getty, no
  `su`, `sudo` or `passwd`, and root's password is locked. Every process
  that handles input runs as an unprivileged user under
  no-new-privileges.

The development image (`just dev=1 faraday-stick-image`, a separate
file ending in `-dev.img`) adds a root login on the serial port for
debugging. It is not a release image.

## What the kernel can reach

The kernel is built with a fixed list of drivers and no loadable
modules. Each board has a `kernel.forbidden` and a `kernel.required`
list, and the build fails if a forbidden option is on or a required one
is off. The required list includes the kernel's own defences: it
randomises and checks its heap, and stops on detected corruption or an
internal error rather than carrying on.

- **No networking.** The network stack is not compiled in: no Ethernet,
  Wi-Fi, Bluetooth or USB network adapter has a driver, and there is no
  socket layer for one to use.
- **No internal storage.** On a PC the SATA, NVMe and eMMC drivers are
  not built, so the computer's own disk is invisible: it cannot be read,
  written or mounted. The firmware variable filesystem is not built
  either.
- **Removable storage, by platform.**

  | | PC | Raspberry Pi |
  |---|---|---|
  | USB stick | Yes | No: the kernel has no USB storage driver |
  | SD card | Only in a USB card reader that presents itself as USB mass storage. A built-in SD slot is not read: its driver is the one many laptops use for their internal eMMC disk. | Yes, its own slot |
  | Keyboard and pointer | Built-in keyboard, touchpad and touchscreen, and USB | The touch panel only: the kernel has no HID driver |
  | Camera | A USB webcam, plugged in or built in, that offers uncompressed frames. Not a built-in camera on a MIPI lane, as in many recent thin laptops: its driver needs firmware and an image processor Faraday does not have. | Pi camera or USB webcam |

- **Why the stick comes out.** Nothing reaches a stick except through
  a stick visit, which writes only what waits For the stick, and only while
  nothing secret is open. Pulling the stick does not add that rule; it
  takes away the means to break it. With no stick attached, no fault in
  Faraday can write a secret to one, and a hostile stick has nothing to
  attack while a key or passphrase is in memory. A secret reaches a
  stick only sealed inside a vault, or unencrypted if the person asks
  for that. Faraday warns that anyone with the stick will be able to
  read it, and asks again before writing it.
- **Removable storage behind a boundary.**
  - The kernel parses no filesystem from a stick or card; it has no FAT
    driver. `faraday-disk` reads and writes FAT in Rust, as an
    unprivileged user.
  - Partitions are handed out one at a time. A whole-disk device is never
    handed out, and neither is the boot partition, so the running system
    cannot modify its own kernel.
  - No removable storage is accessible while any secret is in memory.
    The boot stick comes out before a vault is unlocked. To write a vault
    back, Faraday seals it and restarts the application, and the stick
    goes in only after that.
- **A stick plugged in while a vault is open.** Two separate parts
  refuse it, so a fault in one does not open the stick:
  - `faraday-grant` hands a partition to `faraday-disk` only while the
    application has published that it holds no secret. With a vault
    open, the partition device stays root's and mode `0600`, and no
    Faraday process can open it. If the application stops being clean,
    every partition already handed out is taken back.
  - The application shows **A stick is attached · nothing has been read
    from it** with two choices: **Not now**, which leaves the stick
    untouched, and **Lock and use stick**, which locks (below) before
    the stick is read. While a stick is attached, passphrase fields take
    no typing and no key is loaded.

  What the kernel does read from such a stick is its USB descriptors and
  its partition table. Its storage interface is the only one authorised,
  so a stick that also claims to be a keyboard types nothing.
- **USB devices.** On a PC the kernel has drivers for mass storage,
  keyboards and pointers, webcams and hubs; on a Pi, for webcams and
  hubs. Nothing else has a driver. Interfaces are authorised one at a
  time: a device with a storage interface gets only that interface, so a
  stick that also claims to be a keyboard is never authorised as one. A
  keyboard other than the computer's built-in one is ignored until it
  types a random code shown on the screen, and until it is unplugged if
  it types three codes' worth of letters without it; after three such
  keyboards, new ones are ignored until Faraday starts again. A new
  pointer is ignored until a person approves it with trusted input.
- **Also absent:** swap, suspend, hibernation, core dumps, `/dev/mem`,
  `/proc/kcore`, kexec, debugfs, BPF, kernel probes and tracing,
  io_uring, asynchronous I/O, System V IPC, the kernel keyring, one
  process reading another's memory, and Thunderbolt PCIe tunnelling. On
  a PC also the 32-bit system calls, raw HID access, virtio, the CD-ROM
  driver and the vendor-specific keyboard and mouse drivers. Kernel
  lockdown is forced to its strictest level from early boot, which
  closes whatever interface remains for a process to read or write
  kernel memory or raw hardware. The IOMMU is on from boot in strict
  mode.

## Stateless

- The root filesystem is linked into the kernel image. The firmware loads
  that one file into RAM, and nothing is mounted from the boot medium
  afterwards.
- Nothing is written to the computer. There is no driver for its disk
  and no access to its firmware variables.
- **What locking wipes, and how.** Locking seals every open vault into
  For the stick and then clears memory in two steps:
  1. The application drops every key, seed, passphrase and decrypted
     record it holds. Each is kept in a type that overwrites its memory
     with zeros when it is dropped (the `zeroize` crate's volatile
     writes, which the compiler cannot optimise away). Typed words and
     passphrases are kept in a buffer that also wipes the old copy
     whenever it has to grow, and files from a stick are wiped on their
     way through the disk process, which outlives each lock.
  2. The application process then exits and a fresh one starts. The
     kernel overwrites every page a process frees with zeros before the
     page can be used again, and zeroes it again when it is handed out
     (built into the kernel as its default, and set on the command line
     as well). This also covers copies step 1 cannot reach, such as
     temporary values a library left on the stack.

  Seed-word files and other secret files copied in from a stick are
  removed from RAM at the same time. Files, From the stick and For the
  stick, which hold only public or encrypted files, are kept for the
  fresh process, with the receipt of the last stick visit's write.
  Nothing can be paged out first: there is no swap, no hibernation and
  no core dumps.
- **Panic key.** Holding Super and S together for two seconds powers
  the machine off at once, from any screen: the panel goes black, the
  application wipes what it holds as it ends, and nothing is asked or
  saved. Settings shows it beside Power off.
- Powering off loses everything in RAM. What persists is only what is
  written to a stick: sealed vaults, signed transactions, public keys and
  certificates, and `faraday-settings.txt`, the display and idle
  settings, read from the boot stick at the next boot. Nothing read from
  that file can turn off the idle lock or change the network.

## Why not a stripped-down amnesiac Linux

An amnesiac live system makes a general-purpose desktop forget. Its
amnesia is a policy laid over a system that can still reach a network
and a disk. Faraday does not have those capabilities to begin with.
Every component that parses input is attack surface, and Faraday's
complete list of untrusted input fits in one paragraph (`PLAN.md` §4.4).

| | Amnesiac live Linux | Faraday |
|---|---|---|
| What runs | A display server, a desktop, a browser and hundreds of packages | The kernel, init and three Rust programs |
| Network | Stack and drivers present; disabled, firewalled or routed | Not compiled in |
| The computer's own disk | Driver present; the system can mount it | No driver |
| Kernel drivers | Loadable modules for most hardware | A fixed list; no module loading |
| Filesystems on sticks | Parsed by the kernel, often mounted automatically | Parsed by an unprivileged Rust process |
| Administrator access | An optional administrator password, `sudo` | No login, no `su`, no `sudo` |
| USB devices | Any class a driver exists for | Storage, input, camera and hub only, authorised interface by interface; a new keyboard proves a person is at the screen |

**A live system with networking compiled out.** Removing networking
from the kernel removes one subsystem. The rest of the kernel is decided
by the software above it. systemd requires control groups; a graphical
desktop draws through the kernel's GPU driver stack; browser sandboxes
use namespaces and seccomp; mounting a stick needs filesystem drivers in
the kernel. These are where many Linux privilege-escalation bugs have
been found, and they cannot be compiled out without breaking the
programs that need them. Above the kernel, a desktop and a browser bring
in font engines, image decoders, a JavaScript engine, D-Bus, udev and
polkit, all of which parse untrusted data and run as the same user as
the wallet. Faraday's kernel carries only what one program needs, and
the build fails if a forbidden option is turned on. The secrets are in a
process no other program shares a user with.

Faraday still runs the Linux kernel, with its USB, input and camera
drivers, and its own Rust dependencies. A bug in a driver Faraday keeps
is still a risk. What Faraday removes is everything else.

## Vault cryptography

- A vault is one file. Its header (format version, Argon2id cost and a
  random 32-byte salt) is not secret. Everything after it is ciphertext.
- **Key.** Argon2id version 1.3 over the passphrase and the salt, then
  HKDF-SHA256 with a fixed label, gives a 256-bit key. Faraday creates
  vaults with at least 64 MiB of memory and 2 passes; presets go up to
  2 GiB, chosen for the weakest machine that will open the vault.
- **Cipher.** XChaCha20-Poly1305 with a random 24-byte nonce, drawn fresh
  every time the vault is sealed. The header is authenticated as
  associated data, so a changed salt or cost makes the file fail to open
  rather than changing what a guess costs.
- **Fixed size.** The contents are padded to a size chosen at creation,
  from 64 KiB to 4 MiB. The file does not grow as entries are added and
  does not show how much it holds.
- **Limits before work.** A reader checks the stated cost against fixed
  limits before allocating memory, and checks the authentication tag
  before parsing anything inside.
- **Writing back.** A sealed vault is written as a new file, read back
  and compared byte for byte, then renamed over the previous copy.
- **The passphrase is what protects it.** Doubling the Argon2id cost adds
  one bit; one more word from the EFF long list adds 12.9. The built-in
  dice generator makes word passphrases, and a six-word one is 77.5 bits.
- **Readable without Faraday.** The format is published in
  `docs/VAULT.md`, with test vectors in `tools/vectors/vault/`.
  `tools/vault/open.py`, a Python reader over standard libraries, opens a
  vault with no Faraday code.

## Why this construction holds up

The pieces above are chosen so that the usual ways a passphrase-sealed
file goes wrong are absent by construction, not patched after the fact.

- **Standard primitives, used in the standard way.** Argon2id and
  XChaCha20-Poly1305 are the current recommendations for passphrase
  stretching and authenticated encryption, and both come from vetted
  crates rather than anything invented here (`docs/AUDIT.md`). The work
  is in how they are combined; none of it is new cryptography.
- **Memory-hard stretching.** Argon2id makes each guess cost memory as
  well as time, which is what denies an attacker the advantage of
  massively parallel hardware. It raises the floor under a passphrase; it
  does not replace a strong one.
- **Stateless sealing.** A 24-byte nonce is large enough to draw at
  random on every seal with no meaningful chance of collision, so correct
  use needs no counter and no saved state. A frequent source of AEAD
  failures — a reused or mismanaged nonce — cannot arise here.
- **Nothing is compressed before encryption.** Compressing secrets before
  sealing them can leak their content through ciphertext length; this
  format does not, and the fixed slot size means a file's length reveals
  only its size bucket, never how much is stored or what kind of thing it
  is. An unused slot is random bytes, indistinguishable from one in use.
- **Open, not obscure.** The scheme is written down byte for byte, has
  test vectors, and is opened by an independent reader, so its security
  rests on the passphrase and the primitives alone — not on the format
  staying secret, and a vault outlives the program that wrote it.

## What this does not defend against

- A malicious Faraday binary. The defence is reproducible builds, checked
  by people other than the author (below).
- Compromised firmware, hardware implants, and cold-boot attacks on RAM.
- A weak passphrase.
