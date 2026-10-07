#!/bin/bash
# The image build, as it runs inside the container. `just pi-image` and
# `just stick-image` start it; there is nothing here to run on the host.
#
# One board, one panel and one variant make one image. This script is what
# combines them: it merges their Buildroot fragments into a .config, joins
# their config.txt and cmdline.txt parts, writes the shell's arguments from
# the panel's panel.conf, generates the boot logo, and then runs Buildroot.
#
#   BOARD   a directory under boards/   (default pi3)
#   PANEL   a directory under panels/   (default waveshare-28dpi)
#   DEV     1 to add variants/dev       (default 0)
#
# Nothing here knows what a Raspberry Pi is. A board says in its board.conf
# how its firmware is told where the kernel is (FIRMWARE, CMDLINE), what
# device trees it needs (DTBS), where the exchange partition is (EXCHANGE)
# and which tty a dev login goes on (CONSOLE_TTY); a panel says in its
# panel.conf what the shell must be told about the display, and says
# nothing when the display can say it itself.
#
# Paths are the container's. Each is a bind mount from out/pi/ on the host,
# except /image, which is this directory, mounted read-only:
#
#   /buildroot  the Buildroot checkout at the pinned tag  (out/pi/buildroot)
#   /output     this combination's build tree and images
#               (out/pi/<board>-<panel>[-dev]/output)
#   /dl         Buildroot's download cache                (out/pi/dl)
#   /ccache     the compiler cache, shared by every combination (out/pi/ccache)
#   /binary     the static binary from `just pi-bin`
#   /image      opensigner/shells/pi/image, read-only
set -euo pipefail

BOARD="${BOARD:-pi3}"
PANEL="${PANEL:-waveshare-28dpi}"
DEV="${DEV:-0}"

# The dev variant's boot report, on the exchange partition: the kernel log
# from rcS and the shell's own inventory after it. A laptop has no serial
# port to watch a boot on, so this file is how a machine that did not come
# up says what it found.
BOOT_REPORT=opensigner-boot.txt

image=/image
external=$image/external
gen=/output/gen

board_dir="$image/boards/$BOARD"
panel_dir="$image/panels/$PANEL"
dev_dir="$image/variants/dev"

[ -d "$board_dir" ] || { echo "no such board: $BOARD" >&2; exit 1; }
[ -d "$panel_dir" ] || { echo "no such panel: $PANEL" >&2; exit 1; }

# The directories that make up this image, innermost first. Every part
# below is assembled by walking this list in order, so a later part
# overrides an earlier one. The dev variant contributes twice: once for
# what every board's dev image has, and once for what this board's does —
# the tty a getty goes on is the board's, not the variant's.
parts=("$image/common" "$board_dir" "$panel_dir")
if [ "$DEV" = 1 ]; then
    parts+=("$dev_dir")
    if [ -d "$board_dir/dev" ]; then
        parts+=("$board_dir/dev")
    fi
fi

export BR2_DL_DIR=/dl
# Container root is the invoking user on the host under rootless Docker.
# A few autoconf scripts refuse to configure as uid 0 without this.
export FORCE_UNSAFE_CONFIGURE=1

# shellcheck source=/dev/null
. "$board_dir/board.conf"
board_name="$NAME"
board_status="${STATUS:-untested}"
board_dtbs="${DTBS:-}"
board_firmware="${FIRMWARE:-none}"
board_cmdline="${CMDLINE:-kernel}"
board_exchange="${EXCHANGE:-}"
board_console_tty="${CONSOLE_TTY:-}"
[ -n "$board_exchange" ] || { echo "$board_dir/board.conf has no EXCHANGE" >&2; exit 1; }
# RUST_TARGET, CC and CFLAGS in board.conf are `just pi-bin`'s; nothing in
# this build may see them, least of all as a compiler for the host.
unset RUST_TARGET CC CFLAGS

# The panel's values, cleared first so a board.conf that happens to set one
# of the same names cannot be read as the panel's answer.
unset NAME WIDTH HEIGHT DPI TOUCH_NAME TOUCH_GRID FB LOGO_WIDTH LOGO_HEIGHT
# shellcheck source=/dev/null
. "$panel_dir/panel.conf"
[ -n "${NAME:-}" ] || { echo "$panel_dir/panel.conf has no NAME" >&2; exit 1; }

# A soldered-on panel knows its size and its density and the shell is told
# both; a firmware framebuffer reports its size in sysfs and no firmware
# reports a density, so a panel that gives no WIDTH says nothing and the
# shell reads sysfs and the kernel command line (PLANNING.md 16.94).
if [ -n "${WIDTH:-}" ] || [ -n "${HEIGHT:-}" ]; then
    for v in WIDTH HEIGHT; do
        [ -n "${!v:-}" ] || { echo "$panel_dir/panel.conf has WIDTH or HEIGHT but not both" >&2; exit 1; }
    done
fi
if [ -n "${TOUCH_NAME:-}" ] || [ -n "${TOUCH_GRID:-}" ]; then
    for v in TOUCH_NAME TOUCH_GRID; do
        [ -n "${!v:-}" ] || { echo "$panel_dir/panel.conf has TOUCH_NAME or TOUCH_GRID but not both" >&2; exit 1; }
    done
fi

variant="$BOARD-$PANEL"
what="$board_name with the $NAME"
if [ "$DEV" = 1 ]; then
    variant="$variant-dev"
    what="$what, dev variant"
fi
echo "==> $variant: $what"
if [ "$board_status" != tested ]; then
    echo "    $BOARD is $board_status: no one has booted this combination"
fi

mkdir -p "$gen"

# Generated files are only replaced when their content changes, so their
# timestamps mean what the staleness checks below take them to mean.
install_gen() {
    local dst="$1"
    cat > "$dst.new"
    if [ -f "$dst" ] && cmp -s "$dst.new" "$dst"; then
        find "$dst.new" -delete
    else
        mv "$dst.new" "$dst"
    fi
}

# ---- config.txt: the board's part, then the panel's. A board whose
# firmware reads a configuration file is the only kind that has one.
if [ "$board_firmware" = rpi ]; then
    for p in "${parts[@]}"; do
        if [ -f "$p/config.txt" ]; then
            cat "$p/config.txt"
        fi
    done | install_gen "$gen/config.txt"
fi

# ---- the kernel command line: one line of words. Comments and line breaks
# are stripped, and a word prefixed with "-" removes the word it names from
# what came before, which is how the dev variant drops `quiet`.
words=()
for p in "${parts[@]}"; do
    [ -f "$p/cmdline.txt" ] || continue
    for w in $(sed 's/#.*//' "$p/cmdline.txt"); do
        if [ "${w#-}" != "$w" ]; then
            keep=()
            for k in ${words[@]+"${words[@]}"}; do
                [ "$k" = "${w#-}" ] || keep+=("$k")
            done
            words=(${keep[@]+"${keep[@]}"})
        else
            words+=("$w")
        fi
    done
done
cmdline="${words[*]}"

# Where that line goes is the board's business. A Pi's firmware reads it
# from cmdline.txt on the boot partition. A kernel that is its own EFI
# application has no bootloader to be passed one, so the line is compiled
# in and CONFIG_CMDLINE_OVERRIDE makes it the only one.
if [ "$board_cmdline" = file ]; then
    echo "$cmdline" | install_gen "$gen/cmdline.txt"
else
    install_gen "$gen/cmdline.fragment" <<EOF
# Generated by build.sh from the parts' cmdline.txt files. Do not edit.
CONFIG_CMDLINE_BOOL=y
CONFIG_CMDLINE="$cmdline"
CONFIG_CMDLINE_OVERRIDE=y
EOF
fi

# ---- genimage.cfg: the layout of the medium this board boots from, which
# is the board's file. A board with device trees has them put in where the
# template says @DTBS@; post-image.sh reads the generated copy.
dtb_list=""
for d in $board_dtbs; do
    if [ -n "$dtb_list" ]; then
        dtb_list="$dtb_list, "
    fi
    dtb_list="$dtb_list\"$d.dtb\""
done
sed "s|@DTBS@|$dtb_list|" "$board_dir/genimage.cfg" \
    | install_gen "$gen/genimage.cfg"

# ---- /etc/opensigner/args: what the inittab hands the shell, so one
# binary serves every panel and every laptop. No value may contain a space;
# the file is word-split by the shell that starts the app.
mkdir -p "$gen/overlay/etc/opensigner"
args=""
if [ -n "${FB:-}" ]; then
    args="$args --fb ${FB}"
fi
if [ -n "${WIDTH:-}" ]; then
    args="$args --size ${WIDTH}x${HEIGHT}"
fi
if [ -n "${DPI:-}" ]; then
    args="$args --dpi ${DPI}"
fi
if [ -n "${TOUCH_NAME:-}" ]; then
    args="$args --touch-name ${TOUCH_NAME} --touch-grid ${TOUCH_GRID}"
fi
if [ "$DEV" = 1 ]; then
    args="$args --verbose --timings --boot-report /mnt/microsd/$BOOT_REPORT"
fi
echo "${args# }" | install_gen "$gen/overlay/etc/opensigner/args"

# ---- /etc/opensigner/boot-report: the name of the file rcS writes the
# kernel log to on the exchange partition, and the shell then appends its
# own inventory to. The file exists only in the dev variant, and its
# absence is what tells rcS in a release image to write nothing.
if [ "$DEV" = 1 ]; then
    echo "$BOOT_REPORT" | install_gen "$gen/overlay/etc/opensigner/boot-report"
else
    find "$gen/overlay/etc/opensigner" -name boot-report -delete
fi

# ---- /etc/opensigner/exchange: which partition rcS mounts at
# /mnt/microsd. A card has one place it can be, so the Pi names the node;
# a stick can come up as sda or sdb, so it names the label instead and rcS
# asks findfs which node that is.
echo "$board_exchange" | install_gen "$gen/overlay/etc/opensigner/exchange"

# ---- the dev variant's inittab, which is the release one plus a getty on
# the board's console tty. The tty is the board's — ttyAMA0 on a Pi,
# ttyS0 on a PC — so the file is a template and this is where it is filled
# in.
if [ "$DEV" = 1 ]; then
    [ -n "$board_console_tty" ] || {
        echo "$board_dir/board.conf has no CONSOLE_TTY, which the dev variant needs" >&2
        exit 1
    }
    mkdir -p "$gen/overlay/etc"
    sed "s|@TTY@|$board_console_tty|g" "$dev_dir/inittab.in" \
        | install_gen "$gen/overlay/etc/inittab"
fi

# ---- the boot logo. external.mk copies it over the kernel's own logo when
# the tree is unpacked. The kernel draws a logo at the top left and does
# not scale it, so a panel whose size is known gets a logo the size of the
# panel with the mark in the middle of it, and a display whose size is not
# known until it is on gets a small square that fits any of them.
logo_w="${LOGO_WIDTH:-${WIDTH:-}}"
logo_h="${LOGO_HEIGHT:-${HEIGHT:-}}"
[ -n "$logo_w" ] || { echo "$panel_dir/panel.conf has neither WIDTH nor LOGO_WIDTH" >&2; exit 1; }
python3 "$image/common/make-logo.py" --mark "$image/common/mark.txt" \
    --width "$logo_w" --height "$logo_h" --out "$gen/logo.ppm"

# ---- the paths fragment: every Buildroot option whose value is a list of
# files, which is to say every option that depends on which parts are in
# this image. The parts themselves hold no paths.
kernel_fragments=()
busybox_fragments=()
overlays=()
users_tables=()
post_image=()
for p in "${parts[@]}"; do
    if [ -f "$p/linux.fragment" ]; then
        kernel_fragments+=("$p/linux.fragment")
    fi
    if [ -f "$p/busybox.fragment" ]; then
        busybox_fragments+=("$p/busybox.fragment")
    fi
    if [ -d "$p/rootfs-overlay" ]; then
        overlays+=("$p/rootfs-overlay")
    fi
    if [ -f "$p/users.table" ]; then
        users_tables+=("$p/users.table")
    fi
    # A part's post-image script stages what its layout wants to find, so
    # each runs before common/post-image.sh, which is what calls genimage.
    if [ -f "$p/post-image.sh" ] && [ "$p" != "$image/common" ]; then
        post_image+=("$p/post-image.sh")
    fi
done
post_image+=("$image/common/post-image.sh")
# The generated command line is a kernel fragment on a board that compiles
# it in, and it comes last so nothing can be merged over it.
if [ "$board_cmdline" != file ]; then
    kernel_fragments+=("$gen/cmdline.fragment")
fi
# The generated overlay last: it carries /etc/opensigner/args and nothing
# a part could want to override.
overlays+=("$gen/overlay")

{
    echo "# Generated by build.sh for $BOARD + $PANEL, dev=$DEV. Do not edit."
    echo "BR2_LINUX_KERNEL_CONFIG_FRAGMENT_FILES=\"${kernel_fragments[*]}\""
    echo "BR2_PACKAGE_BUSYBOX_CONFIG_FRAGMENT_FILES=\"${busybox_fragments[*]}\""
    echo "BR2_ROOTFS_USERS_TABLES=\"${users_tables[*]}\""
    echo "BR2_ROOTFS_OVERLAY=\"${overlays[*]}\""
    echo "BR2_ROOTFS_POST_IMAGE_SCRIPT=\"${post_image[*]}\""
    if [ -n "$board_dtbs" ]; then
        echo "BR2_LINUX_KERNEL_INTREE_DTS_NAME=\"$board_dtbs\""
    fi
    if [ "$board_firmware" = rpi ]; then
        echo "BR2_PACKAGE_RPI_FIRMWARE_CONFIG_FILE=\"$gen/config.txt\""
        echo "BR2_PACKAGE_RPI_FIRMWARE_CMDLINE_FILE=\"$gen/cmdline.txt\""
    fi
} | install_gen "$gen/paths.fragment"

# ---- the .config, rebuilt whenever any input is newer than it. The
# fragments are merged in part order, so the dev variant's empty root
# password replaces the release image's locked one rather than sitting
# next to it. CONFIG_=BR2_ is what tells merge_config.sh that these are
# Buildroot symbols and not kernel ones; without it a later `# BR2_X is
# not set` would be appended as a comment and the earlier BR2_X=y would
# still win.
fragments=()
for p in "${parts[@]}"; do
    if [ -f "$p/buildroot.fragment" ]; then
        fragments+=("$p/buildroot.fragment")
    fi
done
fragments+=("$gen/paths.fragment")

stale=no
if [ ! -f /output/.config ]; then
    stale=yes
elif [ -n "$(find "${parts[@]}" "$external" "$gen/paths.fragment" -newer /output/.config -print -quit)" ]; then
    stale=yes
fi

if [ "$stale" = yes ]; then
    echo "==> configuring $variant"
    CONFIG_=BR2_ /buildroot/support/kconfig/merge_config.sh -m -O /output \
        "${fragments[@]}"
    make -C /buildroot O=/output BR2_EXTERNAL="$external" olddefconfig
fi

# external.mk drops the defconfig's `=m` lines when the kernel tree is
# unpacked, once per tree. A tree unpacked before that hook existed still
# carries the original defconfig, and Buildroot would build its modules
# into the kernel (PLANNING.md 16.93). Unpack it again.
linux_dir="$(ls -d /output/build/linux-* 2>/dev/null | grep -v linux-headers | head -1 || true)"
if [ -n "$linux_dir" ] && [ -d "$linux_dir/arch" ]; then
    defconfig_name="$(sed -n 's/^BR2_LINUX_KERNEL_DEFCONFIG="\(.*\)"$/\1/p' \
        /output/.config)_defconfig"
    defconfig_path="$(find "$linux_dir/arch" -name "$defconfig_name" \
        -print -quit)"
    if [ -n "$defconfig_path" ] \
            && grep -q '^CONFIG_[A-Za-z0-9_]*=m$' "$defconfig_path"; then
        echo "==> the unpacked kernel tree still has the defconfig's modules"
        make -C /output linux-dirclean
    fi
fi

# ---- the kernel tarball is the one the board pinned.
#
# The board's buildroot.fragment carries the sha256 of the tarball it
# names, as a `# KERNEL_SHA256 <hex>` line. Buildroot checks a hash of its
# own only for the versions its linux.hash happens to list, and checks
# nothing at all for a tarball from anywhere else, so this is the line
# that makes the pin a pin.
echo "==> fetching the kernel source"
make -C /output linux-source
kernel_sha256="$(sed -n 's/^# KERNEL_SHA256 \([0-9a-f]\{64\}\)$/\1/p' \
    "$board_dir/buildroot.fragment")"
[ -n "$kernel_sha256" ] || {
    echo "$board_dir/buildroot.fragment has no '# KERNEL_SHA256 <hex>' line" >&2
    exit 1
}
kernel_tarball="$(basename "$(sed -n \
    's/^BR2_LINUX_KERNEL_CUSTOM_TARBALL_LOCATION="\(.*\)"$/\1/p' /output/.config)")"
kernel_file="$(find /dl/linux -name "$kernel_tarball" -print -quit)"
[ -n "$kernel_file" ] || { echo "the kernel tarball $kernel_tarball is not in /dl/linux" >&2; exit 1; }
got="$(sha256sum "$kernel_file" | cut -d' ' -f1)"
if [ "$got" != "$kernel_sha256" ]; then
    echo "the kernel tarball is not the one this board pinned:" >&2
    echo "  $kernel_file" >&2
    echo "  pinned: $kernel_sha256" >&2
    echo "  got:    $got" >&2
    exit 1
fi
echo "    $kernel_tarball sha256 $got"

# Two packages install files they did not build, so Buildroot's stamps
# say "done" however new the file is. The shell binary is one: an image
# rebuilt after `just pi-bin` would otherwise carry the previous binary.
# The firmware package is the other: it copies the assembled config.txt
# and cmdline.txt onto the boot partition. Both reinstalls are a handful
# of file copies, and the rootfs and the kernel's initramfs re-link
# follow from them.
echo "==> reinstalling the generated files and the shell binary"
reinstall=(opensigner-pi-reinstall)
if [ "$board_firmware" = rpi ]; then
    reinstall+=(rpi-firmware-reinstall)
fi
make -C /output "${reinstall[@]}"

# The kernel's .config, before the kernel is built. The check is the rule
# in image/check-kernel-config.sh: the drivers this board, this panel, the
# input devices, the camera and the medium need, and nothing else. A
# forbidden option fails here in seconds rather than in an image.
echo "==> configuring the kernel"
make -C /output linux-configure
"$image/check-kernel-config.sh" \
    "$(ls -d /output/build/linux-*/.config | grep -v /linux-headers | head -1)" \
    "${parts[@]}"

echo "==> building"
make -C /output

# The binary in the image must be the binary `just pi-bin` built. The two
# files are not identical — Buildroot's target-finalize strips the
# installed copy, which moves the section headers and takes about 130
# bytes off — so the code is what is compared: the bytes of .text, which
# stripping does not touch.
echo "==> checking the shell binary in the image"
work="$(mktemp -d)"
trap 'chmod -R u+w "$work"; find "$work" -mindepth 1 -delete; rmdir "$work"' EXIT
gzip -dc /output/images/rootfs.cpio.gz \
    | (cd "$work" && cpio --quiet -idm usr/bin/opensigner-pi)
# The toolchain installs the same objcopy under more than one name; any
# of them reads the target's ELF, so take the first.
objcopy="$(ls /output/host/bin/*-objcopy | head -1)"
text_hash() {
    "$objcopy" -O binary --only-section=.text "$1" /dev/stdout | sha256sum | cut -d' ' -f1
}
in_image="$(text_hash "$work/usr/bin/opensigner-pi")"
on_host="$(text_hash /binary/opensigner-pi)"
if [ "$in_image" != "$on_host" ]; then
    echo "the image carries a different opensigner-pi than /binary:" >&2
    echo "  in the image: $in_image" >&2
    echo "  from pi-bin:  $on_host" >&2
    exit 1
fi
echo "    .text $in_image"

echo "==> images"
ls -l /output/images
