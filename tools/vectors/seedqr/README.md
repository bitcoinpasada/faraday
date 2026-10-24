# SeedQR vectors

`vectors.json` is the "Test SeedQRs" section of SeedSigner's SeedQR
specification, the file docs/seed_qr/README.md of
<https://github.com/SeedSigner/seedsigner> at commit
`85cd9a0211eeb22962a5e65ba349ef6efa91bb57` (SHA-256 of the file
`44fe1a3232b5b60f2c4aa6a74a74025fbda69df4d1e4d452e95ccac0b7011a3c`):
nine keys, each as its words, its SeedQR digit stream and its
CompactSeedQR bytes (the specification's bitstream written as hex). The
cross-check's `seedqr` area (`tools/crosscheck/area_seedqr.py`) reads
it; the specification's text is what that area's reference is written
from.
