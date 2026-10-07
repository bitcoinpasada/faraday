# Build container for the OpenSigner Pi image.
#
# It holds nothing but Buildroot's documented host dependencies and a
# non-root build user. Buildroot itself is not in the image: `just pi-image`
# clones it into out/pi/buildroot on the host and bind-mounts it, so the
# pinned tag is visible in the working tree and a rebuild is incremental.
# Pinned by digest, not by tag: `debian:bookworm-slim` is a name that moves
# with every point release, and a build container that changes underneath a
# build is the thing reproducibility rules out. This digest is what that
# tag pointed at on 2026-09-16.
FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171

# Buildroot's "Mandatory packages" plus what the Raspberry Pi kernel and the
# image tools need: bison/flex for the kernel's own parsers, dosfstools and
# mtools for the FAT boot partition, unzip for Waveshare's overlay pack.
RUN apt-get update && apt-get install -y --no-install-recommends \
        bash bc binutils bison build-essential bzip2 ca-certificates cpio \
        diffutils dosfstools file findutils flex gawk gcc g++ git gzip \
        libncurses-dev locales make mtools patch perl python3 rsync sed tar \
        unzip util-linux wget which xz-utils \
    && apt-get clean && find /var/lib/apt/lists -type f -delete

# Buildroot compares locale-sensitive output; C keeps that stable.
ENV LC_ALL=C
ENV LANG=C

# A named build user for anyone who wants to run the container with
# `--user`. The default is container root, which is what `just pi-image`
# uses: under rootless Docker that is the invoking user on the host, so
# everything written into the bind-mounted out/pi/ belongs to them and
# nothing in the container has host root.
RUN useradd --create-home --shell /bin/bash builder

WORKDIR /buildroot
