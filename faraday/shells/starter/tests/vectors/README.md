# Vectors for the Starter's install tests

`desktop-app.sh` stands in for the desktop app: a shell script that answers
`--version` as the desktop app does and otherwise writes
`started` to a file beside itself (`$0.mark`), so a test can see that the
cached copy was the one that ran. `desktop-app.sh.xz` is made with

    xz -9e -T1 -k -c desktop-app.sh > desktop-app.sh.xz

(xz 5.8.3). Its SHA-256 is `a96fbc9a0f71df904781e6ca23e78bbfc0052737dfcf2f49ec289c3178dd078a`;
the script's is `7b89e3875dc5832ecbbfb9509cf5517200c8e48762c2d983beeaf01ed2094b56`
and it is 107 bytes.
