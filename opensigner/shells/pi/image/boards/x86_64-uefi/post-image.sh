#!/bin/sh
# Stage the EFI system partition's one file, before common/post-image.sh
# runs genimage.
#
# The firmware looks for EFI/BOOT/BOOTX64.EFI on the first FAT partition
# of a removable disk and runs it. That file is the kernel: CONFIG_EFI_STUB
# makes bzImage a PE/COFF application with its own loader, and the whole
# root filesystem is the initramfs inside it. There is nothing else on the
# partition — no bootloader, no shim, no configuration file and no blob.
set -e

esp="${BINARIES_DIR}/esp/EFI/BOOT"
mkdir -p "$esp"
cp "${BINARIES_DIR}/bzImage" "$esp/BOOTX64.EFI"
