################################################################################
#
# waveshare-28dpi-overlays
#
# Waveshare's device tree overlays for the 2.8inch DPI LCD, taken from the
# pack their wiki page links (https://www.waveshare.com/wiki/2.8inch_DPI_LCD)
# at a pinned sha256. Waveshare is the only source; nothing is vendored here.
#
################################################################################

WAVESHARE_28DPI_OVERLAYS_VERSION = 2024-08-08
WAVESHARE_28DPI_OVERLAYS_SOURCE = 28DPI-DTBO.zip
WAVESHARE_28DPI_OVERLAYS_SITE = https://files.waveshare.com/wiki/2.8inc-DPI-LCD
WAVESHARE_28DPI_OVERLAYS_LICENSE = GPL-2.0+ (device tree overlays)
WAVESHARE_28DPI_OVERLAYS_INSTALL_IMAGES = YES

# The pack also carries a 4B pin map and a KMS/DRM overlay. Neither is used:
# the board is a 3B+ and the panel is driven by the firmware's DPI path.
WAVESHARE_28DPI_OVERLAYS_FILES = \
	waveshare-28dpi-3b-4b.dtbo \
	waveshare-28dpi-3b.dtbo \
	waveshare-touch-28dpi.dtbo

# Buildroot has no extractor for zip archives, so unzip is called directly;
# -j drops the archive's directory prefix, -o overwrites on a rebuild.
define WAVESHARE_28DPI_OVERLAYS_EXTRACT_CMDS
	unzip -q -j -o -d $(@D) \
		$(WAVESHARE_28DPI_OVERLAYS_DL_DIR)/$(WAVESHARE_28DPI_OVERLAYS_SOURCE) \
		$(addprefix 28DPI-DTBO/,$(WAVESHARE_28DPI_OVERLAYS_FILES))
endef

# rpi-firmware installs the stock overlays into the same directory; depending
# on it keeps the two installs in a defined order.
WAVESHARE_28DPI_OVERLAYS_DEPENDENCIES = rpi-firmware

define WAVESHARE_28DPI_OVERLAYS_INSTALL_IMAGES_CMDS
	$(foreach f,$(WAVESHARE_28DPI_OVERLAYS_FILES), \
		$(INSTALL) -D -m 0644 $(@D)/$(f) \
			$(BINARIES_DIR)/rpi-firmware/overlays/$(f)$(sep))
endef

$(eval $(generic-package))
