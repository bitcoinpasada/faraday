//! A file's hash in Check a timestamp: worked out for the one operation a
//! proof names, on the worker for a file over [`JOB_BYTES`]
//! (`docs/WORKERS.md` §2.1).

use std::sync::Arc;
use zeroize::Zeroizing;

use faraday_crypto::work::Compute;
use faraday_ots::HashOp;

use super::{Faraday, FileGiven, advance, hex, hex_text_digest, judge, ostep, st};
use crate::strings::{EN, fill};

/// The size of a file from which its hash is a job. A release build on a
/// desktop PC hashes 1 MiB under RIPEMD-160, the slowest of the three, in
/// 2.6 ms; 3 MiB is 7.8 ms, under the stand-in's line of 10 ms for the
/// Pi's 100 ms (`docs/WORKERS.md` §2.1). SHA-256 and SHA-1 are about six
/// times faster here (a Pi has no instructions for them), so one line is
/// kept for all three.
pub(crate) const JOB_BYTES: usize = 3 << 20;

/// A file to hash under one operation, owned.
pub(crate) struct HashWork {
    name: String,
    bytes: Arc<Zeroizing<Vec<u8>>>,
    op: HashOp,
}

/// A file hashed.
pub(crate) struct HashDone {
    name: String,
    bytes: Arc<Zeroizing<Vec<u8>>>,
    op: HashOp,
    digest: Vec<u8>,
}

impl HashWork {
    /// Costs a pass over the file (`HashOp::digest_whole`).
    pub(crate) fn run(self, c: &Compute) -> HashDone {
        let Self { name, bytes, op } = self;
        let digest = op.digest_whole(c, &bytes);
        HashDone {
            name,
            bytes,
            op,
            digest,
        }
    }
}

/// Costs a pass over the file: hashes `bytes` under `op` now when they are
/// at most [`JOB_BYTES`], else starts the job that does.
pub(super) fn start(
    app: &mut Faraday,
    c: &Compute,
    name: String,
    bytes: Arc<Zeroizing<Vec<u8>>>,
    op: HashOp,
) {
    let work = HashWork { name, bytes, op };
    if work.bytes.len() > JOB_BYTES {
        app.start(
            crate::worker::Job::OtsHash(Box::new(work)),
            EN.working_hashing,
        );
        return;
    }
    let done = work.run(c);
    done_with(app, c, done);
}

/// A job's answer: the file taken, or its digest added.
pub(crate) fn hashed(app: &mut Faraday, done: HashDone) {
    let c = app.compute();
    done_with(app, &c, done);
}

/// Costs what `judge` costs. Gives the digest to the file held when it is
/// this one, else takes the file with it.
fn done_with(app: &mut Faraday, c: &Compute, done: HashDone) {
    let HashDone {
        name,
        bytes,
        op,
        digest,
    } = done;
    let Some(s) = st(app) else { return };
    if let Some(f) = s.file.as_mut()
        && f.bytes.as_ref().is_some_and(|b| Arc::ptr_eq(b, &bytes))
    {
        f.digests.push((op, digest));
        judge(app, c);
        return;
    }
    let mut given = FileGiven {
        name,
        digests: vec![(op, digest)],
        bytes: Some(bytes.clone()),
        digest_only: false,
        from_text: false,
    };
    // A file that is the proof's SHA-256 written as text (from a scan or a
    // text file) is that digest, shown in the hash field with the file's
    // name as its source; a file whose own hash matches is the file.
    if let Some(p) = &s.proof {
        let digest = p.value.file_digest();
        if p.value.file_op() == HashOp::Sha256
            && given.digest(HashOp::Sha256) != Some(digest)
            && hex_text_digest(&bytes).as_deref() == Some(digest)
        {
            given.digests = vec![(HashOp::Sha256, digest.to_vec())];
            given.bytes = None;
            given.digest_only = true;
            given.from_text = true;
            given.name = fill(EN.ots_digest_from, &[&given.name]);
            s.hash_text = hex(digest);
        }
    }
    // The hash field shows only the value that is checked: a whole file
    // leaves no older typed text in it.
    if !given.digest_only {
        s.hash_text.clear();
    }
    s.file = Some(given);
    s.refused[1] = None;
    judge(app, c);
    if let Some(s) = st(app) {
        advance(s, ostep::FILE);
    }
}
