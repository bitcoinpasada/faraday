//! The one worker thread: Argon2id and QR encoding never run while
//! drawing (`docs/COMPUTE.md` §1). A job goes in over an `mpsc` channel,
//! the answer comes back, and the shell is woken through a closure.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use aqrv::{Header, Item, NotOpened, Refused, Size, Step};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::keys::{MasterKey, Network};
use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use zeroize::Zeroizing;

/// What the thread is asked.
pub enum Job {
    /// Time one 64 MiB run.
    Time,
    /// Seal an item and draw its code.
    Make {
        /// Which request this is, so a cancelled one is told apart.
        id: u64,
        /// The item.
        item: Item,
        /// The passphrase.
        passphrase: Zeroizing<String>,
        /// The memory step.
        step: Step,
        /// The size.
        size: Size,
        /// Random bytes to draw from, from the shell's entropy channel.
        random: Zeroizing<Vec<u8>>,
    },
    /// Tell what a file's bytes are: an envelope's own bytes, or a PNG
    /// with a code in it.
    Read {
        /// Which request this is.
        id: u64,
        /// The file's bytes.
        bytes: Vec<u8>,
    },
    /// Open an envelope with a passphrase and read its item.
    Open {
        /// Which request this is.
        id: u64,
        /// The envelope's bytes.
        envelope: Vec<u8>,
        /// The passphrase.
        passphrase: Zeroizing<String>,
    },
    /// Work out every value of the opening.
    Workings {
        /// Which request this is.
        id: u64,
        /// The envelope's bytes.
        envelope: Vec<u8>,
        /// The passphrase.
        passphrase: Zeroizing<String>,
    },
}

/// What a line of an opened item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The BIP-39 words.
    Words,
    /// The master fingerprint.
    Fingerprint,
    /// A codex32 string.
    String,
    /// Its threshold.
    Threshold,
    /// Its identifier.
    Identifier,
    /// Its share index.
    Index,
    /// It is a secret.
    Secret,
    /// It is a share.
    Share,
    /// A passphrase's or note's text.
    Text,
    /// A raw key as hex.
    Key,
}

/// An item opened, as lines to show (`docs/AQRV.md` §7).
pub struct Shown {
    /// The size and step the file's header gave.
    pub header: Header,
    /// What to show, in order.
    pub lines: Vec<(Label, Zeroizing<String>)>,
}

/// What a line of the working is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    /// The passphrase's bytes.
    Pw,
    /// Argon2id's output.
    Prk,
    /// The encryption key.
    KEnc,
    /// The MAC key.
    KMac,
    /// The keystream.
    Keystream,
    /// `P`.
    P,
    /// `C`.
    C,
    /// The HMAC's message.
    Message,
    /// A kind's tag, by the kind's byte.
    Tag(u8),
}

/// The working: each value as lower-case hex.
pub type Working = Vec<(Value, Zeroizing<String>)>;

/// Why no code was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    /// The library refused the item or the passphrase.
    Refused,
    /// Argon2id's memory could not be had.
    NotEnoughMemory,
}

/// A code made.
pub struct Made {
    /// The envelope's bytes.
    pub bytes: Vec<u8>,
    /// The QR code of those bytes, kept to draw from.
    pub matrix: QrMatrix,
    /// The 47-character text form the 32-byte size's code holds (§3).
    pub text: Option<String>,
    /// The size.
    pub size: Size,
    /// The memory step.
    pub step: Step,
    /// The code as a PNG picture.
    pub png: Vec<u8>,
}

/// What the thread answers.
pub enum Answer {
    /// The 64 MiB run's time.
    Timed(Duration),
    /// A made code, or why not, for request `id`.
    Made(u64, Box<Result<Made, Fail>>),
    /// What a file is: the envelope and its header, or nothing.
    Read(u64, Option<(Vec<u8>, Header)>),
    /// An item opened, or why not.
    Opened(u64, Box<Result<Shown, NotOpened>>),
    /// The working, or why not.
    Working(u64, Box<Result<Working, NotOpened>>),
}

/// The thread's two ends.
pub struct Worker {
    jobs: Sender<Job>,
    answers: Receiver<Answer>,
}

impl Worker {
    /// Starts the thread; `wake` is called after each answer is sent.
    pub fn start(wake: impl Fn() + Send + 'static) -> Worker {
        let (jobs, job_rx) = channel::<Job>();
        let (answer_tx, answers) = channel::<Answer>();
        std::thread::spawn(move || run(&job_rx, &answer_tx, &wake));
        Worker { jobs, answers }
    }

    /// Asks the thread for `job`.
    pub fn ask(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    /// The next answer, if one is ready.
    pub fn poll(&self) -> Option<Answer> {
        self.answers.try_recv().ok()
    }

    /// The next answer, waiting for it.
    pub fn wait(&self) -> Option<Answer> {
        self.answers.recv().ok()
    }
}

/// The thread's body: answers each job until the app is gone.
fn run(job_rx: &Receiver<Job>, answer_tx: &Sender<Answer>, wake: &dyn Fn()) {
    // A token cannot cross threads, so the thread mints its own.
    let c = Compute::new(|_| {});
    while let Ok(job) = job_rx.recv() {
        let answer = answer(&c, job);
        if answer_tx.send(answer).is_err() {
            break;
        }
        wake();
    }
}

/// Costs a 64 MiB Argon2id run (`Job::Time`), an Argon2id run at the
/// step and a QR encoding (`Job::Make`), a PNG decode (`Job::Read`), an
/// Argon2id run and a fingerprint (`Job::Open`) or an Argon2id run
/// (`Job::Workings`).
fn answer(c: &Compute, job: Job) -> Answer {
    match job {
        Job::Time => {
            let started = Instant::now();
            let item = Item::Note(Zeroizing::new("x".to_owned()));
            let mut random = |b: &mut [u8]| b.fill(0x5a);
            let _ = aqrv::seal(c, &item, "timing", Step::Mib64, Size::Small, &mut random);
            Answer::Timed(started.elapsed())
        }
        Job::Make {
            id,
            item,
            passphrase,
            step,
            size,
            random,
        } => Answer::Made(
            id,
            Box::new(make(c, &item, &passphrase, step, size, &random)),
        ),
        Job::Read { id, bytes } => Answer::Read(id, read(c, &bytes)),
        Job::Open {
            id,
            envelope,
            passphrase,
        } => Answer::Opened(id, Box::new(open(c, &envelope, &passphrase))),
        Job::Workings {
            id,
            envelope,
            passphrase,
        } => Answer::Working(id, Box::new(working(c, &envelope, &passphrase))),
    }
}

/// An envelope's own bytes (a `.aqrv` file), or the first code in a PNG
/// that is one.
fn read(c: &Compute, bytes: &[u8]) -> Option<(Vec<u8>, Header)> {
    if let Some(header) = aqrv::recognise_file(bytes) {
        return Some((bytes.to_vec(), header));
    }
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return None;
    }
    let codes = faraday_files::qr_in_png(c, bytes).ok()?;
    codes.iter().find_map(|code| {
        let envelope = aqrv::recognise_code(code)?;
        let header = aqrv::recognise(&envelope)?;
        Some((envelope, header))
    })
}

fn open(c: &Compute, envelope: &[u8], passphrase: &str) -> Result<Shown, NotOpened> {
    let opened = aqrv::open(c, envelope, passphrase)?;
    Ok(Shown {
        header: opened.header,
        lines: lines(c, &opened.item),
    })
}

fn working(c: &Compute, envelope: &[u8], passphrase: &str) -> Result<Working, NotOpened> {
    let w = aqrv::workings(c, envelope, passphrase)?;
    let size = aqrv::recognise(envelope).map_or(Size::Small, |h| h.size);
    let mut out: Working = vec![
        (Value::Pw, hex(&w.pw)),
        (Value::Prk, hex(&w.prk)),
        (Value::KEnc, hex(&w.k_enc)),
        (Value::KMac, hex(&w.k_mac)),
        (Value::Keystream, hex(&w.keystream)),
        (Value::P, hex(&w.p)),
        (Value::C, hex(&w.c)),
    ];
    if let Some(m) = &w.message {
        out.push((Value::Message, hex(m)));
    }
    for (kind, tag) in &w.tags {
        out.push((Value::Tag(*kind as u8), hex(&tag[..size.tag_len()])));
    }
    Ok(out)
}

fn hex(bytes: &[u8]) -> Zeroizing<String> {
    Zeroizing::new(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Costs a BIP-39 seed and a master key for the fingerprint of an entropy
/// item or a codex32 secret.
fn lines(c: &Compute, item: &Item) -> Vec<(Label, Zeroizing<String>)> {
    let text = |s: &str| Zeroizing::new(s.to_owned());
    match item {
        Item::Entropy(e) => {
            let mut out = Vec::new();
            if let Ok(m) = Mnemonic::from_entropy(Language::English, e) {
                let words = Language::English.words();
                let sentence: Vec<&str> =
                    m.indices().iter().map(|i| words[usize::from(*i)]).collect();
                out.push((Label::Words, text(&sentence.join(" "))));
                if let Ok(seed) = m.to_seed(c, b"") {
                    let fp = MasterKey::from_seed(c, &seed, Network::Mainnet).fingerprint();
                    out.push((Label::Fingerprint, text(&fp.to_string())));
                }
            }
            out
        }
        Item::Codex32(s) => {
            let mut out = vec![(Label::String, text(s))];
            if let Ok(parsed) = Codex32::parse(s) {
                out.push((Label::Threshold, text(&parsed.threshold().to_string())));
                let id = parsed.identifier();
                out.push((Label::Identifier, text(&String::from_utf8_lossy(&id))));
                out.push((
                    Label::Index,
                    text(&char::from(parsed.share_index()).to_string()),
                ));
                if parsed.is_secret() {
                    out.push((Label::Secret, text("")));
                    if let Ok(seed) = parsed.to_seed()
                        && let Some(bytes) = SeedBytes::new(seed.expose().bytes())
                    {
                        let key =
                            MasterKey::from_seed_bytes(c, &Secret::new(bytes), Network::Mainnet);
                        out.push((Label::Fingerprint, text(&key.fingerprint().to_string())));
                    }
                } else {
                    out.push((Label::Share, text("")));
                }
            }
            out
        }
        Item::Passphrase(t) | Item::Note(t) => vec![(Label::Text, text(t))],
        Item::RawKey(k) => vec![(Label::Key, hex(&k[..]))],
    }
}

/// The code as a grayscale PNG, a four-module quiet zone around it and
/// eight pixels to a module.
fn picture(matrix: &QrMatrix) -> Option<Vec<u8>> {
    const SCALE: usize = 8;
    let n = matrix.size();
    let side = (n + 8) * SCALE;
    let mut pixels = vec![0xffu8; side * side];
    for row in 0..n {
        for col in 0..n {
            if matrix.module(col, row) {
                for dy in 0..SCALE {
                    let at = ((row + 4) * SCALE + dy) * side + (col + 4) * SCALE;
                    pixels[at..at + SCALE].fill(0);
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, side as u32, side as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .ok()?
        .write_image_data(&pixels)
        .ok()?;
    Some(out)
}

fn make(
    c: &Compute,
    item: &Item,
    passphrase: &str,
    step: Step,
    size: Size,
    random: &[u8],
) -> Result<Made, Fail> {
    let mut at = 0;
    let mut short = false;
    let mut draw = |out: &mut [u8]| {
        if at + out.len() <= random.len() {
            out.copy_from_slice(&random[at..at + out.len()]);
            at += out.len();
        } else {
            // Never reuse a byte: the pool ran out, so the code is not made.
            short = true;
            out.fill(0);
        }
    };
    let sealed = aqrv::seal(c, item, passphrase, step, size, &mut draw);
    let bytes = match sealed {
        Ok(_) if short => return Err(Fail::Refused),
        Ok(bytes) => bytes,
        Err(Refused::NotEnoughMemory) => return Err(Fail::NotEnoughMemory),
        Err(_) => return Err(Fail::Refused),
    };
    // The 32-byte size is carried as its text in alphanumeric mode,
    // version 2 at level L; the others as their bytes (§9).
    let text = match <&[u8; 32]>::try_from(bytes.as_slice()) {
        Ok(b) if size == Size::Tiny => Some(aqrv::to_text(b)),
        _ => None,
    };
    let matrix = match (&text, size) {
        (Some(t), _) => encode(c, Payload::Alphanumeric(t.as_bytes()), Ecc::Low),
        (None, Size::Large) => encode(c, Payload::Bytes(&bytes), Ecc::Medium),
        (None, _) => encode(c, Payload::Bytes(&bytes), Ecc::Low),
    }
    .map_err(|_| Fail::Refused)?;
    let png = picture(&matrix).ok_or(Fail::Refused)?;
    Ok(Made {
        bytes,
        matrix,
        text,
        size,
        step,
        png,
    })
}
