//! What a seed's files are called on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6).

/// The name seed form `what` of the seed with fingerprint `fp` is written
/// as on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6): `seed-{fp}.txt`,
/// `seedqr-{fp}.png`, `passphrase-{fp}.txt`, the fingerprint in lower case.
pub(crate) fn seed_stick_name(what: crate::plan::What, fp: &str) -> Option<String> {
    let fp = fp.to_lowercase();
    match what {
        crate::plan::What::SeedFile(_) => Some(format!("seed-{fp}.txt")),
        crate::plan::What::SeedQrFile(_) => Some(format!("seedqr-{fp}.png")),
        crate::plan::What::PassphraseFile(_) => Some(format!("passphrase-{fp}.txt")),
        _ => None,
    }
}

/// Whether `name` is a file on a stick of one of the seeds `fps`, in any
/// form ([`seed_stick_name`]).
pub(crate) fn seed_stick_file_of(fps: &[String], name: &str) -> bool {
    fps.iter().any(|fp| {
        [
            crate::plan::What::SeedFile(0),
            crate::plan::What::SeedQrFile(0),
            crate::plan::What::PassphraseFile(0),
        ]
        .into_iter()
        .any(|w| seed_stick_name(w, fp).as_deref() == Some(name))
    })
}
