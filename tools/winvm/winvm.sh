#!/usr/bin/env bash
# A Windows 11 test virtual machine for the hub app's BitLocker guard
# (docs/HUB.md §6.6): QEMU, OVMF's Microsoft-key Secure Boot firmware, a
# TPM 2.0 from swtpm, no network. See README.md here.
set -euo pipefail

DIR=${WINVM_DIR:-$HOME/winvm}
# This host's settings (any WINVM_ variable) may be kept in winvm.conf.
# shellcheck source=/dev/null
if [[ -f $DIR/winvm.conf ]]; then . "$DIR/winvm.conf"; fi
DISK=${WINVM_DISK:-$DIR/win11.qcow2}
first() { local f; for f; do if [[ -e $f ]]; then echo "$f"; return; fi; done; }
# The firmware. On a host that runs KVM itself, the distribution's Secure
# Boot build, which requires SMM. Where this Linux is itself a guest (the
# "hypervisor" CPU flag), nested KVM may be unable to enter SMM, and
# distributions build Secure Boot only into their SMM firmware: there,
# OVMF with Secure Boot and without SMM from build-ovmf.sh. It enforces
# Secure Boot and measures PCR 7 the same; only SMM's protection of the
# variable store from the guest OS is missing, which these tests do not
# need. WINVM_CODE and WINVM_SMM override the choice.
if [[ -z ${WINVM_CODE:-} ]] && ! grep -qw hypervisor /proc/cpuinfo; then
	WINVM_CODE=$(first /usr/share/OVMF/OVMF_CODE_4M.secboot.fd \
		/usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd \
		/usr/share/edk2/ovmf/OVMF_CODE.secboot.fd)
	WINVM_SMM=${WINVM_SMM:-on}
fi
CODE=${WINVM_CODE:-$DIR/ovmf/OVMF_CODE.fd}
SMM=${WINVM_SMM:-off}
# Variables with Microsoft's keys enrolled: Debian's, or a copy of them.
VARS_TEMPLATE=${WINVM_VARS_TEMPLATE:-$(first /usr/share/OVMF/OVMF_VARS_4M.ms.fd "$DIR/OVMF_VARS_4M.ms.fd")}
VARS=${WINVM_VARS:-$DIR/OVMF_VARS.fd}
TPM=${WINVM_TPM:-$DIR/tpm}
MEM=${WINVM_MEM:-4096}
CPUS=${WINVM_CPUS:-2}
ACCEL=${WINVM_ACCEL:-kvm}
if [[ $ACCEL == tcg* ]]; then CPU=${WINVM_CPU:-max}; else CPU=${WINVM_CPU:-host}; fi
VNC=${WINVM_VNC:-1}
QMP=$DIR/qmp.sock
PIDFILE=$DIR/qemu.pid
LOG=$DIR/qemu.log
SHOTS=$DIR/shots

usage() {
	cat <<'EOF'
winvm.sh COMMAND [ARGS]

  create                     make the 64 GB disk and the firmware variables
  answer-disk IMG            make the FAT disk (label WINVM) with autounattend.xml
                             and firstlogon.ps1 from beside this script; the
                             account's password is read from WINVM_PASSWORD
  start [OPTIONS]            start the machine in the background
      --cdrom ISO            attach an installation ISO, and press a key at
                             "Press any key to boot from CD"
      --usb-stick IMG        attach a raw image as a removable USB stick
      --usb-fat IMG          attach a second raw image as a USB drive
      --secure-boot on|off   first set the firmware's Secure Boot setting
                             ("Attempt Secure Boot") in its variable store
      --setup                go into the firmware's setup menu (best effort)
  status                     running or off, and the firmware's Secure Boot
  eject                      take the ISO out of the DVD drive (a bootable disc
                             in the drive changes what BitLocker's TPM sees)
  powerdown                  press the power button (ACPI)
  quit                       pull the plug
  wait-off [SECONDS]         wait until the machine is off (default 600)
  screenshot OUT.png         save the screen as PNG
  keys KEY...                press keys; KEY is a QEMU qcode or a combination
                             such as ctrl-alt-delete; "sleep:N" waits N seconds
  click X Y                  click at a pixel of the screen (as in a screenshot)
  type TEXT                  type text (letters, digits, common punctuation)
  snapshot NAME              save the disk (qemu-img snapshot), the firmware
                             variables and the TPM's state, while off
  snapshot -l                list them
  snapshot -a NAME           go back to one, while off

Settings, as environment variables or in $WINVM_DIR/winvm.conf:
  WINVM_DIR       state directory (default ~/winvm)
  WINVM_DISK      system disk (default $WINVM_DIR/win11.qcow2)
  WINVM_CODE      OVMF code: the distribution's Secure Boot build (SMM),
                  or, inside another hypervisor, $WINVM_DIR/ovmf/OVMF_CODE.fd
                  from build-ovmf.sh (Secure Boot, no SMM)
  WINVM_SMM       on for firmware that requires SMM, off otherwise
  WINVM_VARS_TEMPLATE, WINVM_VARS
                  OVMF variables with Microsoft's keys (Debian's
                  OVMF_VARS_4M.ms.fd, or a copy of it in $WINVM_DIR), and
                  the machine's own copy
  WINVM_TPM       swtpm state directory
  WINVM_MEM, WINVM_CPUS
                  memory in MB (4096) and cores (2)
  WINVM_ACCEL     kvm (default) or tcg
  WINVM_CPU       -cpu (host under kvm, max under tcg)
  WINVM_VNC       VNC display number, on 127.0.0.1 only (1: port 5901)
EOF
}

die() { echo "winvm: $*" >&2; exit 1; }

running() { [[ -f $PIDFILE ]] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; }

# One QMP command per connection: the handshake, then the command given as
# JSON on stdin, then the reply on stdout.
qmp() {
	[[ -S $QMP ]] || die "not running (no $QMP)"
	python3 -I -c '
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
f = s.makefile("rw")
def cmd(c):
    f.write(json.dumps(c) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline())
cmd({"execute": "qmp_capabilities"})
r = cmd(json.load(sys.stdin))
if "error" in r: sys.exit("qmp: " + r["error"]["desc"])
print(json.dumps(r["return"]))
' "$QMP"
}

send_key() { # one combination, e.g. ret or ctrl-alt-delete
	local IFS=- keys=() k
	for k in $1; do keys+=("{\"type\":\"qcode\",\"data\":\"$k\"}"); done
	IFS=,
	echo "{\"execute\":\"send-key\",\"arguments\":{\"keys\":[${keys[*]}],\"hold-time\":80}}" | qmp >/dev/null
}

keys() {
	local k
	for k in "$@"; do
		case $k in
		sleep:*) sleep "${k#sleep:}" ;;
		*) send_key "$k"; sleep 0.3 ;;
		esac
	done
}

type_text() {
	local s=$1 i c k
	for ((i = 0; i < ${#s}; i++)); do
		c=${s:i:1}
		case $c in
		[a-z0-9]) k=$c ;;
		[A-Z]) k=shift-${c,,} ;;
		' ') k=spc ;; '-') k=minus ;; '_') k=shift-minus ;; '.') k=dot ;;
		',') k=comma ;; '/') k=slash ;; '\') k=backslash ;; ':') k=shift-semicolon ;;
		';') k=semicolon ;; '=') k=equal ;; '!') k=shift-1 ;; '@') k=shift-2 ;;
		'#') k=shift-3 ;; '$') k=shift-4 ;; '%') k=shift-5 ;; '*') k=shift-8 ;;
		'(') k=shift-9 ;; ')') k=shift-0 ;; '"') k=shift-apostrophe ;; "'") k=apostrophe ;;
		'>') k=shift-dot ;; '<') k=shift-comma ;; '&') k=shift-7 ;; '|') k=shift-backslash ;;
		*) die "cannot type '$c'" ;;
		esac
		send_key "$k"
		sleep 0.1
	done
}

screenshot() {
	local out
	out=$(realpath -m "$1")
	echo "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$out\",\"format\":\"png\"}}" | qmp >/dev/null
	echo "$out"
}

# A click at a pixel of the screen, through the USB tablet, whose
# coordinates run 0 to 32767 across the screen whatever its size.
click() {
	local probe=$DIR/.probe.ppm size w h
	echo "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$probe\",\"format\":\"ppm\"}}" | qmp >/dev/null
	size=$(head -c 32 "$probe" | tr -s ' \n' ' ' | cut -d' ' -f2,3)
	read -r w h <<<"$size"
	local x=$(($1 * 32767 / (w - 1))) y=$(($2 * 32767 / (h - 1)))
	echo "{\"execute\":\"input-send-event\",\"arguments\":{\"events\":[{\"type\":\"abs\",\"data\":{\"axis\":\"x\",\"value\":$x}},{\"type\":\"abs\",\"data\":{\"axis\":\"y\",\"value\":$y}}]}}" | qmp >/dev/null
	sleep 0.1
	echo '{"execute":"input-send-event","arguments":{"events":[{"type":"btn","data":{"down":true,"button":"left"}}]}}' | qmp >/dev/null
	sleep 0.1
	echo '{"execute":"input-send-event","arguments":{"events":[{"type":"btn","data":{"down":false,"button":"left"}}]}}' | qmp >/dev/null
}

# The firmware's own setting, read from its variable store: the newest
# valid SecureBootEnable variable (OVMF's "Attempt Secure Boot").
sb_state() {
	python3 -I -c '
import struct, sys
d = open(sys.argv[1], "rb").read()
name = "SecureBootEnable\0".encode("utf-16-le")
state, i = "unknown", d.find(name)
while i != -1:
    h = i - 60  # an authenticated variable header is 60 bytes
    sid, st = struct.unpack_from("<HB", d, h)
    ns, ds = struct.unpack_from("<II", d, h + 36)
    if sid == 0x55AA and st == 0x3F and ds == 1:
        state = "on" if d[i + ns] else "off"
    i = d.find(name, i + 1)
print(state)
' "$VARS"
}

create() {
	mkdir -p "$DIR" "$TPM" "$SHOTS"
	[[ -e $DISK ]] || qemu-img create -f qcow2 "$DISK" 64G
	[[ -e $VARS ]] || cp "$VARS_TEMPLATE" "$VARS"
}

answer_disk() {
	local img=$1 here
	here=$(dirname "$(realpath "$0")")
	[[ -n ${WINVM_PASSWORD:-} ]] || die "set WINVM_PASSWORD"
	[[ -e $img ]] && die "$img exists"
	truncate -s 64M "$img"
	printf 'label: dos\nstart=2048, type=c\n' | sfdisk -q "$img"
	mkfs.vfat -F 32 -n WINVM --offset 2048 "$img" $((63 * 1024)) >/dev/null
	sed "s/@PASSWORD@/$WINVM_PASSWORD/g" "$here/autounattend.xml" >"$DIR/autounattend.xml"
	mcopy -i "$img@@1M" "$DIR/autounattend.xml" "$here/firstlogon.ps1" ::/
	shred -u "$DIR/autounattend.xml" # the password stays only on the disk image
	mdir -i "$img@@1M" ::/
}

launch() { # QEMU arguments beyond the machine itself
	running && die "already running (pid $(cat "$PIDFILE"))"
	[[ -e $DISK && -e $VARS ]] || die "no disk or variables: run create first"
	mkdir -p "$TPM" "$SHOTS"

	# A fresh swtpm per boot, on the same state: it ends when QEMU closes
	# the socket.
	swtpm socket --tpm2 --tpmstate dir="$TPM" \
		--ctrl type=unixio,path="$TPM/swtpm-sock" \
		--log file="$TPM/swtpm.log" --terminate --daemon
	for _ in $(seq 50); do [[ -S $TPM/swtpm-sock ]] && break; sleep 0.1; done

	local args=(
		-name winvm
		-machine "q35,smm=$SMM"
		-accel "$ACCEL"
		-global ICH9-LPC.disable_s3=1
		-drive "if=pflash,format=raw,unit=0,readonly=on,file=$CODE"
		-drive "if=pflash,format=raw,unit=1,file=$VARS"
		-cpu "$CPU"
		-smp "$CPUS" -m "$MEM"
		-chardev "socket,id=chrtpm,path=$TPM/swtpm-sock"
		-tpmdev emulator,id=tpm0,chardev=chrtpm
		-device tpm-crb,tpmdev=tpm0
		-drive "file=$DISK,if=none,id=sys,format=qcow2,cache=writeback"
		-device ide-hd,drive=sys,bus=ide.0,bootindex=1
		-device qemu-xhci,id=xhci
		-device usb-kbd,bus=xhci.0
		-device usb-tablet,bus=xhci.0
		-nic none
		-vga std
		-display none -vnc "127.0.0.1:$VNC"
		-qmp "unix:$QMP,server=on,wait=off"
		-pidfile "$PIDFILE"
		"$@"
	)
	if [[ $SMM == on ]]; then args+=(-global driver=cfi.pflash01,property=secure,value=on); fi
	echo "--- $(date -Is) ${args[*]}" >>"$LOG"
	setsid nohup qemu-system-x86_64 "${args[@]}" >>"$LOG" 2>&1 </dev/null &
	for _ in $(seq 100); do [[ -S $QMP ]] && running && break; sleep 0.1; done
	running || die "qemu failed; see $LOG"
}

# Is the screen one of OVMF's setup pages? They are grey where the boot
# screens and the boot device menu are black.
in_setup() {
	local probe=$DIR/.probe.ppm
	echo "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$probe\",\"format\":\"ppm\"}}" | qmp >/dev/null
	python3 -I -c '
import sys
d = open(sys.argv[1], "rb").read()
parts = d.split(maxsplit=4)  # P6, width, height, maxval, pixels
w, px = int(parts[1]), parts[4]
o = (400 * w + 100) * 3
r, g, b = px[o:o + 3]
sys.exit(0 if r == g == b and 100 < r < 220 else 1)
' "$probe"
}

# Into OVMF's setup menu: Escape while the firmware starts opens its boot
# device menu, whose last entry is the setup. Best effort: look at the
# screen afterwards.
enter_setup() {
	echo '{"execute":"cont"}' | qmp >/dev/null
	for _ in $(seq 25); do send_key esc; sleep 0.3; done
	for _ in $(seq 10); do
		sleep 2
		in_setup && return 0
		keys up ret
	done
	die "the firmware's setup menu did not come up; see a screenshot"
}

# Secure Boot on or off, with the machine off: the firmware's
# SecureBootEnable variable ("Attempt Secure Boot" in its setup menu) is set
# in the variable store. At its next start the firmware derives SecureBoot
# from it, exactly as after a person unticks or ticks the box and saves;
# nothing else in the store changes. Keys are not used for this because
# the firmware's Esc/F2 hotkey opens only its boot device menu, and keys
# sent through QMP reach it or not depending on the host's load.
set_secure_boot() {
	python3 -I -c '
import struct, sys
path, want = sys.argv[1], sys.argv[2] == "on"
d = bytearray(open(path, "rb").read())
name = "SecureBootEnable\0".encode("utf-16-le")
hit, i = None, d.find(name)
while i != -1:
    h = i - 60
    sid, st = struct.unpack_from("<HB", d, h)
    ns, ds = struct.unpack_from("<II", d, h + 36)
    if sid == 0x55AA and st == 0x3F and ds == 1:
        hit = i + ns
    i = d.find(name, i + 1)
if hit is None:
    sys.exit("no SecureBootEnable variable in " + path)
d[hit] = 1 if want else 0
open(path, "r+b").write(d)
' "$VARS" "$1"
	echo "Secure Boot in the firmware: $(sb_state)"
}

start() {
	local cdrom="" stick="" fat="" setup=0 sb="" extra=()
	while (($#)); do
		case $1 in
		--cdrom) cdrom=$2; shift 2 ;;
		--usb-stick) stick=$2; shift 2 ;;
		--usb-fat) fat=$2; shift 2 ;;
		--secure-boot) sb=$2; shift 2 ;;
		--setup) setup=1; shift ;;
		*) die "start: unknown option $1" ;;
		esac
	done
	[[ -z $sb || $sb == on || $sb == off ]] || die "--secure-boot on|off"
	[[ -n $cdrom ]] && extra+=(-drive "file=$cdrom,if=none,id=cd,media=cdrom,readonly=on"
		-device ide-cd,drive=cd,bus=ide.1,bootindex=2)
	[[ -n $stick ]] && extra+=(-drive "file=$stick,if=none,id=stick,format=raw"
		-device usb-storage,bus=xhci.0,drive=stick,removable=on)
	[[ -n $fat ]] && extra+=(-drive "file=$fat,if=none,id=fat,format=raw"
		-device usb-storage,bus=xhci.0,drive=fat,removable=on)

	if [[ -n $sb ]]; then
		running && die "--secure-boot: the machine must be off"
		set_secure_boot "$sb"
	fi
	if ((setup)); then
		# The firmware waits 5 s for a key at power-on.
		launch -S -boot menu=on,splash-time=5000 "${extra[@]}"
		enter_setup
	else
		launch "${extra[@]}"
		if [[ -n $cdrom ]]; then
			# "Press any key to boot from CD or DVD": only when the disk
			# has nothing to boot, which is the first start of an
			# installation.
			for _ in $(seq 40); do send_key ret; sleep 0.5; done
		fi
	fi
	echo "running: pid $(cat "$PIDFILE"), VNC 127.0.0.1:$((5900 + VNC)), QMP $QMP"
}

wait_off() {
	for _ in $(seq "${1:-600}"); do running || return 0; sleep 1; done
	die "still running"
}

snapshot() {
	# The disk's qemu-img snapshot, with copies of the firmware variables
	# and the TPM's state beside it: BitLocker's key is sealed to that TPM.
	running && die "power the machine off first"
	case ${1:-} in
	-l) qemu-img snapshot -l "$DISK" ;;
	-a)
		local name=${2:?snapshot -a NAME}
		[[ -d $DIR/snapshots/$name ]] || die "no saved state for $name"
		qemu-img snapshot -a "$name" "$DISK"
		cp "$DIR/snapshots/$name/OVMF_VARS.fd" "$VARS"
		find "$TPM" -maxdepth 1 -type f -name 'tpm2-*' -delete
		cp "$DIR/snapshots/$name/tpm/"tpm2-* "$TPM/"
		;;
	'') die "snapshot NAME|-l|-a NAME" ;;
	*)
		[[ -e $DIR/snapshots/$1 ]] && die "$1 exists"
		mkdir -p "$DIR/snapshots/$1/tpm"
		qemu-img snapshot -c "$1" "$DISK"
		cp "$VARS" "$DIR/snapshots/$1/OVMF_VARS.fd"
		cp "$TPM/"tpm2-* "$DIR/snapshots/$1/tpm/"
		;;
	esac
}

cmd=${1:-}
shift || true
case $cmd in
create) create ;;
answer-disk) answer_disk "${1:?answer-disk IMG}" ;;
start) start "$@" ;;
status)
	if running; then echo "running: pid $(cat "$PIDFILE")"; else echo "off"; fi
	echo "Secure Boot in the firmware: $(sb_state)"
	;;
eject) echo '{"execute":"eject","arguments":{"device":"cd","force":true}}' | qmp >/dev/null ;;
powerdown) echo '{"execute":"system_powerdown"}' | qmp >/dev/null ;;
quit) if running; then echo '{"execute":"quit"}' | qmp >/dev/null || true; fi; wait_off 30 ;;
wait-off) wait_off "${1:-600}" ;;
screenshot) screenshot "${1:?screenshot OUT.png}" ;;
keys) keys "$@" ;;
click) click "${1:?click X Y}" "${2:?click X Y}" ;;
type) type_text "${1:?type TEXT}" ;;
snapshot) snapshot "$@" ;;
-h | --help) usage ;;
*) usage >&2; exit 2 ;;
esac
