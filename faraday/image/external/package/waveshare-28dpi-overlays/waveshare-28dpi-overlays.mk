################################################################################
#
# waveshare-28dpi-overlays
#
# Waveshare's device tree overlay for the 2.8inch DPI LCD touch, taken from the
# pack their wiki page links (https://www.waveshare.com/wiki/2.8inch_DPI_LCD)
# at a pinned sha256. Waveshare is the only source; nothing is vendored here.
#
################################################################################

WAVESHARE_28DPI_OVERLAYS_VERSION = 2024-08-08
WAVESHARE_28DPI_OVERLAYS_SOURCE = 28DPI-DTBO.zip
WAVESHARE_28DPI_OVERLAYS_SITE = https://files.waveshare.com/wiki/2.8inc-DPI-LCD
WAVESHARE_28DPI_OVERLAYS_LICENSE = GPL-2.0+ (device tree overlays)

# Only the touch overlay is used. It has no source in the kernel tree, so
# ovmerge cannot make it; the pack's compiled file is taken as it is (build.sh
# reads it from the extracted tree). The pack's panel overlays are not used:
# the panel is the kernel's vc4-kms-dpi-generic with the vendor's timings.
WAVESHARE_28DPI_OVERLAYS_FILES = \
	waveshare-touch-28dpi.dtbo

# Buildroot has no extractor for zip archives, so unzip is called directly;
# -j drops the archive's directory prefix, -o overwrites on a rebuild.
define WAVESHARE_28DPI_OVERLAYS_EXTRACT_CMDS
	unzip -q -j -o -d $(@D) \
		$(WAVESHARE_28DPI_OVERLAYS_DL_DIR)/$(WAVESHARE_28DPI_OVERLAYS_SOURCE) \
		$(addprefix 28DPI-DTBO/,$(WAVESHARE_28DPI_OVERLAYS_FILES))
endef

$(eval $(generic-package))
