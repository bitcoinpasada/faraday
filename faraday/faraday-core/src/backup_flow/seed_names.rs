//! What a seed's files are called on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6).

/// The name seed form `what` of the seed with fingerprint `fp` is written
/// as on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6): `seed-{fp}.txt`,
/// `seedqr-{fp}.png`, `passphrase-{fp}.txt`, the fingerprint in lower case.
pub(crate) fn seed_stick_name(what: crate::plan::What, fp: &str) -> Option<faraday_text::Text> {
    let fp: faraday_text::Text = fp.chars().flat_map(char::to_lowercase).collect();
    match what {
        crate::plan::What::SeedFile(_) => Some(faraday_text::text!("seed-{}.txt", &*fp)),
        crate::plan::What::SeedQrFile(_) => Some(faraday_text::text!("seedqr-{}.png", &*fp)),
        crate::plan::What::PassphraseFile(_) => {
            Some(faraday_text::text!("passphrase-{}.txt", &*fp))
        }
        _ => None,
    }
}

/// Whether `name` is a file on a stick of one of the seeds `fps`, in any
/// form ([`seed_stick_name`]).
pub(crate) fn seed_stick_file_of(fps: &[impl AsRef<str>], name: &str) -> bool {
    fps.iter().any(|fp| {
        let fp = fp.as_ref();
        [
            crate::plan::What::SeedFile(0),
            crate::plan::What::SeedQrFile(0),
            crate::plan::What::PassphraseFile(0),
        ]
        .into_iter()
        .any(|w| seed_stick_name(w, fp).as_deref() == Some(name))
    })
}
