#!/usr/bin/env bash
# Builds OVMF with Secure Boot and without SMM, for winvm.sh.
#
# Why: where KVM runs nested under another hypervisor, it may be unable
# to enter SMM; the guest
# stops with "KVM: entry failed, hardware error 0xffffffff" as soon as the
# Windows boot loader writes a UEFI variable. Distributions build OVMF's
# Secure Boot firmware only with SMM required, and their firmware without
# SMM has no Secure Boot at all. This build enforces Secure Boot and
# measures PCR 7 the same way; only the protection of the variable store
# from the guest OS, which SMM provides, is missing, and the BitLocker
# tests do not need it. Microsoft's keys come from the distribution's
# OVMF_VARS_4M.ms.fd, whose 4 MB variable store has the same layout.
#
# Everything is fetched and built inside a rootless podman container,
# pinned by digest; the host gets only the source tree and the output.
#
#   build-ovmf.sh [DIR]    DIR defaults to ~/winvm/ovmf; the firmware goes
#                          to DIR/OVMF_CODE.fd and DIR/OVMF_VARS.fd
set -euo pipefail

DIR=${1:-$HOME/winvm/ovmf}
# debian:trixie-slim
IMAGE=docker.io/library/debian@sha256:918311b7b6c4c6f68b232ba516584925f6c78ad82b6fd534b98979df6438e483
TAG=edk2-stable202608
COMMIT=2970e5699ba6267f3384ffab20f96647578aebc8
SUBMODULES=(
	CryptoPkg/Library/OpensslLib/openssl
	CryptoPkg/Library/MbedTlsLib/mbedtls
	MdeModulePkg/Universal/RegularExpressionDxe/oniguruma
	MdeModulePkg/Library/BrotliCustomDecompressLib/brotli
	BaseTools/Source/C/BrotliCompress/brotli
	MdePkg/Library/BaseFdtLib/libfdt
	MdePkg/Library/MipiSysTLib/mipisyst
	SecurityPkg/DeviceSecurity/SpdmLib/libspdm
)
FLAGS=(
	-D SECURE_BOOT_ENABLE=TRUE
	-D SMM_REQUIRE=FALSE
	-D TPM2_ENABLE=TRUE
	-D NETWORK_ENABLE=FALSE
	-D FD_SIZE_4MB
)

mkdir -p "$DIR"
if [[ ! -d $DIR/src ]]; then
	git clone -q --depth 1 --branch "$TAG" https://github.com/tianocore/edk2 "$DIR/src"
	git -C "$DIR/src" submodule update -q --init --depth 1 "${SUBMODULES[@]}"
fi
[[ $(git -C "$DIR/src" rev-parse HEAD) == "$COMMIT" ]] || {
	echo "build-ovmf: $DIR/src is not $TAG ($COMMIT)" >&2
	exit 1
}

podman run --rm --network=slirp4netns -v "$DIR/src:/edk2:Z" -w /edk2 "$IMAGE" bash -ec "
	apt-get -qq update
	apt-get -qq install -y --no-install-recommends build-essential uuid-dev \
		acpica-tools nasm python3 python-is-python3 >/dev/null
	make -s -C BaseTools -j\$(nproc) >/dev/null
	export WORKSPACE=/edk2 PACKAGES_PATH=/edk2
	. ./edksetup.sh >/dev/null
	build -q -n \$(nproc) -a X64 -t GCC -b RELEASE -p OvmfPkg/OvmfPkgX64.dsc ${FLAGS[*]}
"
cp "$DIR/src/Build/OvmfX64/RELEASE_GCC/FV/OVMF_CODE.fd" "$DIR/src/Build/OvmfX64/RELEASE_GCC/FV/OVMF_VARS.fd" "$DIR/"
sha256sum "$DIR/OVMF_CODE.fd" "$DIR/OVMF_VARS.fd"
