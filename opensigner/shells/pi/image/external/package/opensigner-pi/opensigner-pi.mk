################################################################################
#
# opensigner-pi
#
# The shell binary, cross-compiled on the host by `just pi-bin` and mounted
# into the build container. There is no source, no download and no build
# step: this package is an install rule and a check that the file is there.
#
################################################################################

# No download: an empty _SOURCE tells Buildroot's generic infrastructure that
# this package has no archive to fetch or extract.
OPENSIGNER_PI_VERSION = prebuilt
OPENSIGNER_PI_SOURCE =
OPENSIGNER_PI_LICENSE = GPL-3.0-or-later
OPENSIGNER_PI_BINARY = $(call qstrip,$(BR2_PACKAGE_OPENSIGNER_PI_BINARY))

define OPENSIGNER_PI_INSTALL_TARGET_CMDS
	test -f $(OPENSIGNER_PI_BINARY) || { \
		echo "opensigner-pi: $(OPENSIGNER_PI_BINARY) is missing;" \
		     "run 'just pi-bin' first" >&2; \
		exit 1; \
	}
	$(INSTALL) -D -m 0755 $(OPENSIGNER_PI_BINARY) \
		$(TARGET_DIR)/usr/bin/opensigner-pi
endef

$(eval $(generic-package))
