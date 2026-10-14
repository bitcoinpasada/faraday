# Windows test virtual machine

`winvm.sh` runs Windows 11 Pro in QEMU for the Starter's BitLocker guard
acceptance runs (`docs/STARTER.md` §6.6). The machine is q35 under KVM, with
OVMF's Secure Boot firmware (below), Microsoft's keys from Debian's
`OVMF_VARS_4M.ms.fd` (the machine's own copy), a TPM 2.0 from `swtpm`
(state kept across boots, a fresh `swtpm` per boot), 4 GB of memory, 2
cores, a 64 GB qcow2 disk, USB keyboard and
tablet, and no network (`-nic none`). The display is VNC on 127.0.0.1
only; QEMU's QMP socket is in the state directory.

Needs `qemu-system-x86_64`, `qemu-img`, `swtpm`, OVMF, `python3`, and
`sfdisk`, `mkfs.vfat` and `mtools` for the answer disk. Where the
distribution has no Microsoft-key variables (Debian's
`/usr/share/OVMF/OVMF_VARS_4M.ms.fd`), put a copy of Debian's at
`~/winvm/OVMF_VARS_4M.ms.fd`.

## The firmware

`winvm.sh` picks the firmware by what the host offers. On a host that
runs KVM itself, it is the distribution's Secure Boot build, which
requires SMM (`OVMF_CODE_4M.secboot.fd` on Debian,
`OVMF_CODE.secboot.4m.fd` on Arch). Where the host is itself a virtual
machine (the `hypervisor` CPU flag), it is OVMF from `build-ovmf.sh`,
because nested KVM may be unable to enter SMM. `WINVM_CODE` and
`WINVM_SMM` override the choice.

Where KVM runs nested under another hypervisor, it may be unable to
enter SMM. On such a host, with the distribution's Secure
Boot firmware (`OVMF_CODE_4M.secboot.fd`, which requires SMM) QEMU stops
with "KVM: entry failed, hardware error 0xffffffff" as soon as the
Windows boot loader writes a UEFI variable, whatever the `-cpu`. The
distribution's firmware without SMM (`OVMF_CODE_4M.fd`) has no Secure Boot
at all: no Secure Boot Configuration page. Emulating instead (TCG) runs
the SMM firmware, but Windows 11 setup stopped in OOBE with
SYSTEM_SERVICE_EXCEPTION (0x3B) after three and a half hours.

`build-ovmf.sh` builds OVMF (edk2, pinned tag and commit) in a podman
container pinned by digest, with `SECURE_BOOT_ENABLE=TRUE` and
`SMM_REQUIRE=FALSE`, into `~/winvm/ovmf/`. It enforces Secure Boot and
measures PCR 7 as the SMM build does; only SMM's protection of the
variable store from the guest OS is missing, which these tests do not
need.

    build-ovmf.sh            # ~/winvm/ovmf/OVMF_CODE.fd; needs podman and git

On such a host, a KVM install with this firmware was cut short when the
host itself went down two minutes into it, with nothing in its logs;
whether the nested guest caused that is not known, so the machine is run
on a host whose KVM is not nested.

## Settings

State lives in `~/winvm` (`WINVM_DIR`). Settings for one host go in
`~/winvm/winvm.conf`, as shell assignments; `winvm.sh --help` lists them.

## Install

    winvm.sh create
    WINVM_PASSWORD=... winvm.sh answer-disk ~/winvm/answer.img
    winvm.sh start --cdrom Win11.iso --usb-fat ~/winvm/answer.img

`answer-disk` puts `autounattend.xml` (Windows 11 Pro by image name, the
generic Pro install key, so it stays unactivated; en-US; disk 0 wiped and
partitioned GPT; a local administrator `faraday`; no Microsoft account; no
OOBE screens) and `firstlogon.ps1` on a small FAT disk labelled WINVM. At
the first logon the script turns off fast startup, turns BitLocker on for
C: with a TPM protector and a numerical recovery password, waits until the
drive is fully encrypted, and writes `out\recovery.txt`, `out\status.txt`
and `out\protectors.txt` to that disk. Read them with the machine off:

    mtype -i ~/winvm/answer.img@@1M ::/out/status.txt

`start --cdrom` presses Enter at "Press any key to boot from CD or DVD".
Once Windows is installed, start it without `--cdrom` (or `winvm.sh
eject` while it runs): with a bootable disc in the drive, manage-bde
refuses the TPM protector, and the disc changes what the TPM measures.
`firstlogon.ps1` ejects it before turning BitLocker on. Windows 11 may
start encrypting C: by itself before the first logon (device encryption,
with no protector); the script then only adds the protectors. Check
`out\status.txt` for "Protection On" with a TPM and a numerical password
protector. Under KVM the installation takes under an hour.

## Running

    winvm.sh start [--usb-stick stick.img] [--usb-fat files.img] [--secure-boot on|off]
    winvm.sh screenshot shot.png
    winvm.sh keys ctrl-alt-delete        # QEMU qcodes
    winvm.sh type 123456-...             # recovery password, by keys
    winvm.sh powerdown; winvm.sh wait-off
    winvm.sh status                      # and the firmware's Secure Boot setting

`--usb-stick` attaches a raw image (the Faraday stick image) as a
removable USB disk on the xHCI controller; `--usb-fat` a second one.

`--secure-boot off` and `on`, with the machine off, set the firmware's
SecureBootEnable variable, which is what "Attempt Secure Boot" on OVMF's
Device Manager → Secure Boot Configuration page writes when a person
unticks or ticks it and saves; at its next start the firmware derives
Secure Boot from it, and nothing else in the variable store changes. It
is not done by keys because OVMF's Esc/F2 hotkey opens only its boot
device menu, and keys sent through QMP reach it or not depending on the
host's load. `start --setup` tries the menu (Escape, then the boot device
menu's last entry, "EFI Firmware Setup"); look at a screenshot to see
where it landed. The setting reads "unknown" until the firmware's first
start writes the variable.

## Snapshots

    winvm.sh snapshot NAME        # with the machine off
    winvm.sh snapshot -a NAME
    winvm.sh snapshot -l

A snapshot is the qcow2 disk's internal snapshot plus copies of the
firmware variables and the TPM's state, kept together because the
BitLocker key on the disk is sealed by that TPM against measurements of
those variables.
