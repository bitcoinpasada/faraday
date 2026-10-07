#!/bin/sh
# Assemble images/opensigner-pi.img from what Buildroot built.
#
# genimage does the work; this only points it at our static configuration and
# gives it an empty rootpath, since every file in the image is already built.
set -e

# The medium's layout is generated per board, next to the images directory:
# build.sh writes it from the board's genimage.cfg with its device
# trees filled in.
GENIMAGE_CFG="$(dirname "${BINARIES_DIR}")/gen/genimage.cfg"

# The root path must exist and the temporary path must not, so a fresh
# directory for each run keeps genimage happy without deleting anything.
WORK_DIR="$(mktemp -d)"
mkdir "${WORK_DIR}/root"
trap 'chmod -R u+w "${WORK_DIR}"; find "${WORK_DIR}" -mindepth 1 -delete; rmdir "${WORK_DIR}"' EXIT

# The only thing on the second partition: a note about what it is for.
mkdir -p "${BINARIES_DIR}/exchange"
cat > "${BINARIES_DIR}/exchange/README.txt" <<'NOTE'
OpenSigner file exchange partition.

Put a file here from a computer and the device will find it: Sign ->
Read a file offers the newest .psbt on this partition, and the scanner's
Read a file offers the newest file of any kind. This note is never
offered.

What comes back is written here too. Save to file writes signed.psbt,
and never over a file already here: the second one is signed-2.psbt, the
third signed-3.psbt. It is on the disk as soon as the screen says so.
Power the device off before unplugging it.

Nothing else on this disk is read after boot: the whole operating system
is inside the kernel image on the first partition.
NOTE

genimage \
	--rootpath "${WORK_DIR}/root"  \
	--tmppath "${WORK_DIR}/tmp"    \
	--inputpath "${BINARIES_DIR}"  \
	--outputpath "${BINARIES_DIR}" \
	--config "${GENIMAGE_CFG}"
