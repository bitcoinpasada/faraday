# Signed EFI binaries as a distribution ships them

Two binaries, vendored unchanged, for `faraday/faraday-sb/tests/judged.rs`:
Faraday reads each one's Authenticode signature, names its signer and
works out its image hash, and `pesign` and `pesigcheck` (pesign) say the
same.

| File | Signed by | Source | SHA-256 |
|---|---|---|---|
| `shimx64.efi` | Microsoft Windows UEFI Driver Publisher | `usr/lib/shim/shimx64.efi.signed.latest` | `4c89145e958cf592a6f16552eadf112ef2c1c525e2435c2761e6a99fa88188b3` |
| `fbx64.efi` | Canonical Ltd. Secure Boot Signing (2022 v1) | `usr/lib/shim/fbx64.efi` | `396af1acd51bfa1348f4c71e572d7a1adbd8fbb52c844141e9efd255134afe0a` |

Both come from Ubuntu's package `shim-signed` 1.59+15.8-0ubuntu2 (amd64),
<http://archive.ubuntu.com/ubuntu/pool/main/s/shim-signed/shim-signed_1.59+15.8-0ubuntu2_amd64.deb>
(package SHA-256 `f8ed71ce2d91a304b6d5eb84997f846f331b554578bc02dbfe78e13ad8ac81a9`); the shim project is
<https://github.com/rhboot/shim> (upstream release 15.8, the source of the
unsigned build the package carries). Each file is under 2 MiB.

Licence: BSD-2-Clause (Red Hat, Intel; the package's
`usr/share/doc/shim-signed/copyright`).
