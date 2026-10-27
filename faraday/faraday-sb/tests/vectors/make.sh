#!/bin/sh
# How this folder's vectors were made (2026-10-27, Debian stable's
# sbsigntool 0.9.4 and osslsigncode 2.9): faraday-sb's tiny PE (the
# tests' `tiny_pe`) signed by sbsign and by osslsigncode under a
# throwaway test key, whose certificate is cert.der; the key is not kept.
#
#   podman run --rm -v "$PWD":/w:Z docker.io/library/debian:stable sh /w/make.sh
set -eu
apt-get update -qq >/dev/null
apt-get install -y -qq sbsigntool osslsigncode openssl python3 >/dev/null
cd /w
python3 - <<'EOF'
pe = bytearray(0x400)
def p16(at, v): pe[at:at+2] = v.to_bytes(2, 'little')
def p32(at, v): pe[at:at+4] = v.to_bytes(4, 'little')
pe[0:2] = b'MZ'; p32(0x3c, 0x40); pe[0x40:0x44] = b'PE\0\0'
coff = 0x44
p16(coff, 0x8664); p16(coff+2, 1); p16(coff+16, 240); p16(coff+18, 0x22)
opt = coff + 20
p16(opt, 0x20b); p32(opt+16, 0x1000); p32(opt+32, 0x1000); p32(opt+36, 0x200)
p32(opt+56, 0x2000); p32(opt+60, 0x200); p16(opt+68, 10); p32(opt+108, 16)
sec = opt + 240
pe[sec:sec+5] = b'.text'
p32(sec+8, 0x10); p32(sec+12, 0x1000); p32(sec+16, 0x200); p32(sec+20, 0x200); p32(sec+36, 0x60000020)
pe[0x200] = 0xc3
open('tiny.efi', 'wb').write(pe)
EOF
openssl req -new -x509 -newkey rsa:2048 -nodes -sha256 -days 36500 \
    -subj "/CN=Test sbsign db/" -keyout key.pem -out cert.pem 2>/dev/null
openssl x509 -in cert.pem -outform DER -out cert.der
sbsign --key key.pem --cert cert.pem --output tiny.sbsign.efi tiny.efi
osslsigncode sign -h sha256 -certs cert.pem -key key.pem -in tiny.efi -out tiny.osslsigncode.efi
sbsign --version || true
osslsigncode --version 2>&1 | head -1 || true
sha256sum tiny.efi tiny.sbsign.efi tiny.osslsigncode.efi cert.der
rm -f key.pem
