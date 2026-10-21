################################################################################
#
# faraday
#
# The app, cross-compiled on the host by faraday.just and mounted
# into the build container. There is no source, no download and no build
# step: this package is an install rule and a check that the file is there.
#
################################################################################

# No download: an empty _SOURCE tells Buildroot's generic infrastructure that
# this package has no archive to fetch or extract.
FARADAY_VERSION = prebuilt
FARADAY_SOURCE =
FARADAY_LICENSE = GPL-3.0-or-later
FARADAY_BINARY = $(call qstrip,$(BR2_PACKAGE_FARADAY_BINARY))

define FARADAY_INSTALL_TARGET_CMDS
	test -f $(FARADAY_BINARY) || { \
		echo "faraday: $(FARADAY_BINARY) is missing;" \
		     "run 'just faraday-pi-bin' first" >&2; \
		exit 1; \
	}
	$(INSTALL) -D -m 0755 $(FARADAY_BINARY) \
		$(TARGET_DIR)/usr/bin/faraday
endef

$(eval $(generic-package))
