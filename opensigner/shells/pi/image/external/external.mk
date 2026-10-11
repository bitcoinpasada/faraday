include $(sort $(wildcard $(BR2_EXTERNAL_OPENSIGNER_PI_PATH)/package/*/*.mk))

# The boot logo. build.sh generates a PPM the size of the panel this image
# is for, with the OpenSigner mark in the middle of a black field, into
# $(BASE_DIR)/gen. The kernel compiles its logo from
# drivers/video/logo/logo_linux_clut224.ppm, so the way to change it is to
# replace that file in the unpacked source, which is what this hook does.
# CONFIG_LOGO and CONFIG_LOGO_LINUX_CLUT224 in common/linux.fragment are
# what make the kernel draw it.
#
# It runs at post-patch, once per unpacked kernel tree. A regenerated logo
# in an existing build tree therefore needs the tree unpacked again:
# `make linux-dirclean` in that variant's output directory.
OPENSIGNER_BOOT_LOGO = $(BASE_DIR)/gen/logo.ppm

define OPENSIGNER_INSTALL_BOOT_LOGO
	$(Q)if [ -f $(OPENSIGNER_BOOT_LOGO) ]; then \
		cp $(OPENSIGNER_BOOT_LOGO) \
			$(@D)/drivers/video/logo/logo_linux_clut224.ppm ; \
	fi
endef

LINUX_POST_PATCH_HOOKS += OPENSIGNER_INSTALL_BOOT_LOGO

# The board's defconfig, with every module dropped.
#
# Kconfig turns a `=m` into `=y` when CONFIG_MODULES is off. The bcm2709
# defconfig has 1203 `=m` lines, so `# CONFIG_MODULES is not set` in
# common/linux.fragment did not drop those drivers: it built every one of
# them into the kernel — the whole network stack, Bluetooth, sound, DRM,
# USB gadget, DVB, infrared, IIO and six filesystems (PLANNING.md 16.93).
#
# The rule is that no option is built in because a defconfig listed it as
# a module, so this hook rewrites the defconfig in the unpacked tree
# before Buildroot reads it: the `=m` lines go, and CONFIG_MODULES is off
# from the start, so a tristate that is not named resolves to n rather
# than to y. What the device needs is then named, explicitly and with a
# reason, in the three linux.fragment files, and image/check-kernel-
# config.sh fails the build if any of it is missing or anything forbidden
# came back.
#
# It runs at post-patch, once per unpacked kernel tree, which is before
# the kconfig step (pkg-kconfig.mk orders .stamp_dotconfig after
# linux-patch). A tree unpacked by an older build still has the original
# defconfig; build.sh notices and runs `linux-dirclean`.
OPENSIGNER_KERNEL_DEFCONFIG = \
	$(LINUX_ARCH_PATH)/configs/$(LINUX_KCONFIG_DEFCONFIG)

define OPENSIGNER_DROP_KERNEL_MODULES
	$(Q)if [ -f $(OPENSIGNER_KERNEL_DEFCONFIG) ]; then \
		sed -i -e '/^CONFIG_[A-Za-z0-9_]*=m$$/d' \
		       -e 's/^CONFIG_MODULES=y$$/# CONFIG_MODULES is not set/' \
			$(OPENSIGNER_KERNEL_DEFCONFIG) ; \
		grep -q '^# CONFIG_MODULES is not set$$' \
			$(OPENSIGNER_KERNEL_DEFCONFIG) \
			|| echo '# CONFIG_MODULES is not set' \
				>> $(OPENSIGNER_KERNEL_DEFCONFIG) ; \
	fi
endef

LINUX_POST_PATCH_HOOKS += OPENSIGNER_DROP_KERNEL_MODULES
