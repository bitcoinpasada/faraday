################################################################################
#
# rpi-ovmerge
#
# Raspberry Pi's ovmerge (raspberrypi/utils, ovmerge/), at a pinned commit and
# sha256. The build runs it on the host, to merge a screen's overlay and its
# parameters into the one parameterless overlay the initramfs carries
# (docs/IMAGES.md §3.3). Two files, a script and its Python module.
#
################################################################################

RPI_OVMERGE_VERSION = 3f5fa2965292168b7e428976044f1ac2da50130a
RPI_OVMERGE_SITE = $(call github,raspberrypi,utils,$(RPI_OVMERGE_VERSION))
RPI_OVMERGE_LICENSE = BSD-3-Clause
RPI_OVMERGE_LICENSE_FILES = LICENCE

define HOST_RPI_OVMERGE_INSTALL_CMDS
	$(INSTALL) -D -m 0755 $(@D)/ovmerge/ovmerge $(HOST_DIR)/bin/ovmerge
	$(INSTALL) -D -m 0644 $(@D)/ovmerge/ovmerge_engine.py \
		$(HOST_DIR)/bin/ovmerge_engine.py
endef

$(eval $(host-generic-package))
