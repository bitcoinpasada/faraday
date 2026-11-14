# The device images

An image that boots a machine straight into Faraday's app: no shell, no
login, no network and no writable root. `build.sh` combines a board, a panel
and optionally the dev variant. What the images hold, how they are built
and how to try them is `docs/IMAGES.md`, §10 for this directory; why they
are shaped so is `docs/SECURITY-MODEL.md`; the recipes are
`docs/DEVELOPING.md`.

```
common/     what every image has: the initramfs, BusyBox, the boot logo,
            the inittab, the account the app runs as
boards/     the architecture, the kernel, the firmware, the drivers, the
            layout of the medium the machine boots from
panels/     what the shell must be told about the display (`panels/any`
            holds a directory per Pi screen; its README gives the format)
variants/   dev only: a serial login and a talking console
external/   the Buildroot br2-external tree: the two packages
```

Each board and panel combination that is built has its entry in
`faraday/builds.toml`, the repository's list of builds; `just lint` fails on
a board or panel directory no entry builds.
