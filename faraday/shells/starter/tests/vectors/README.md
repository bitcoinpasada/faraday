# Vectors for the Starter's install tests

`desktop-app.sh` stands in for a desktop app an earlier Starter installed
(F30: the Starter no longer installs it): a shell script that answers
`--version` as the desktop app does and otherwise writes `started` to a
file beside itself (`$0.mark`). The install tests check that install,
update and uninstall leave it as it is. Its SHA-256 is
`7b89e3875dc5832ecbbfb9509cf5517200c8e48762c2d983beeaf01ed2094b56` and it
is 107 bytes.
