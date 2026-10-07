//! UEFI signature lists and the signed updates firmware takes to enrol
//! them (UEFI 2.10 §32.4 and §8.2.2): `EFI_SIGNATURE_LIST`s of X.509
//! certificates, and `EFI_VARIABLE_AUTHENTICATION_2` files (`.auth`) whose
//! PKCS #7 signature covers the variable's name, vendor GUID, attributes,
//! timestamp and data.

use alloc::vec::Vec;

use crate::Error;
use crate::der::{self, seq, set};
use crate::x509;
use crate::x509::alg;

/// A GUID in the mixed-endian layout UEFI stores.
pub const fn guid(a: u32, b: u16, c: u16, d: [u8; 8]) -> [u8; 16] {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    let c = c.to_le_bytes();
    [
        a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d[0], d[1], d[2], d[3], d[4], d[5], d[6],
        d[7],
    ]
}

/// `EFI_CERT_X509_GUID`.
pub const CERT_X509: [u8; 16] = guid(
    0xa5c0_59a1,
    0x94e4,
    0x4aa7,
    [0x87, 0xb5, 0xab, 0x15, 0x5c, 0x2b, 0xf0, 0x72],
);
/// `EFI_CERT_TYPE_PKCS7_GUID`.
pub const CERT_PKCS7: [u8; 16] = guid(
    0x4aaf_d29d,
    0x68df,
    0x49ee,
    [0x8a, 0xa9, 0x34, 0x7d, 0x37, 0x56, 0x65, 0xa7],
);
/// `EFI_GLOBAL_VARIABLE`, the vendor of PK and KEK.
pub const GLOBAL_VARIABLE: [u8; 16] = guid(
    0x8be4_df61,
    0x93ca,
    0x11d2,
    [0xaa, 0x0d, 0x00, 0xe0, 0x98, 0x03, 0x2b, 0x8c],
);
/// `EFI_IMAGE_SECURITY_DATABASE_GUID`, the vendor of db.
pub const IMAGE_SECURITY_DATABASE: [u8; 16] = guid(
    0xd719_b2cb,
    0x3d3a,
    0x4596,
    [0xa3, 0xbc, 0xda, 0xd0, 0x0e, 0x67, 0x65, 0x6f],
);
/// Microsoft's owner GUID, which its certificates are listed under.
pub const MICROSOFT_OWNER: [u8; 16] = guid(
    0x77fa_9abd,
    0x0359,
    0x4d32,
    [0xbd, 0x60, 0x28, 0xf4, 0xe7, 0x8f, 0x78, 0x4b],
);

/// Non-volatile, boot-service and runtime access, time-based
/// authenticated writes.
const ATTRIBUTES: u32 = 0x01 | 0x02 | 0x04 | 0x20;

/// Signature lists, one per certificate: each owner GUID with its
/// certificate.
pub fn signature_lists(certs: &[([u8; 16], &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (owner, cert) in certs {
        let size = 16 + cert.len();
        out.extend_from_slice(&CERT_X509);
        out.extend_from_slice(&((28 + size) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(owner);
        out.extend_from_slice(cert);
    }
    out
}

/// An `EFI_TIME` for a Unix time, in UTC.
pub fn efi_time(unix: u64) -> [u8; 16] {
    let (y, mo, d, h, mi, s) = x509::civil(unix);
    let mut t = [0u8; 16];
    t[0..2].copy_from_slice(&(y as u16).to_le_bytes());
    t[2] = mo as u8;
    t[3] = d as u8;
    t[4] = h as u8;
    t[5] = mi as u8;
    t[6] = s as u8;
    t
}

/// A signed update of variable `name` (UTF-16, as firmware names it)
/// under `vendor`, holding `data`, signed at `unix` by `signer`'s key: an
/// `EFI_VARIABLE_AUTHENTICATION_2` followed by the data.
pub fn auth(
    name: &str,
    vendor: &[u8; 16],
    data: &[u8],
    unix: u64,
    signer: &x509::Pair,
) -> Result<Vec<u8>, Error> {
    let time = efi_time(unix);
    let mut signed = Vec::new();
    for unit in name.encode_utf16() {
        signed.extend_from_slice(&unit.to_le_bytes());
    }
    signed.extend_from_slice(vendor);
    signed.extend_from_slice(&ATTRIBUTES.to_le_bytes());
    signed.extend_from_slice(&time);
    signed.extend_from_slice(data);
    let sig = x509::sign(&signer.key, &signed)?;
    // A detached SignedData with no signed attributes, as
    // `sign-efi-sig-list` writes it: the firmware hashes the data itself.
    let info = x509::signer_info(&signer.cert, None, &sig)?;
    let signed_data = seq(&[
        &der::int(1),
        &set(&[&alg(x509::SHA256)]),
        &seq(&[&der::oid(&[1, 2, 840, 113_549, 1, 7, 1])]),
        &der::tlv(0xa0, &signer.cert),
        &set(&[&info]),
    ]);
    let mut out = Vec::new();
    out.extend_from_slice(&time);
    out.extend_from_slice(&((24 + signed_data.len()) as u32).to_le_bytes());
    out.extend_from_slice(&0x0200u16.to_le_bytes());
    out.extend_from_slice(&0x0EF1u16.to_le_bytes());
    out.extend_from_slice(&CERT_PKCS7);
    out.extend_from_slice(&signed_data);
    out.extend_from_slice(data);
    Ok(out)
}
