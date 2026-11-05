//! A wallet's public files, made from owned public inputs
//! (`docs/WORKERS.md` §2.1): the descriptor, the sheets, the shares and
//! their pictures, the other software's files. The same making serves the
//! files a person asks for one kind at a time and the checklist's, whether
//! it runs on the worker as a [`FilesWork`] job or on the thread that asked
//! ([`made`], [`files`]); nothing here reads the app.
//!
//! A job's answer brings back the files with the toast the making would
//! have said, and is applied on the main thread: the files go For the
//! stick, the backup under way is told, the toast is said. Back or a lock
//! meanwhile drops the answer, and so does a person no longer on the screen
//! and sheet they pressed on, or a backup no longer open on the wallet.

use faraday_bip::keys::{Fingerprint, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

use crate::strings::{EN, fill};
use crate::wallet::{self, Origin, Session, Wallet, fp_text};
use crate::worker::Job;
use crate::{
    Faraday, Screen, Sheet, bstep, file_stem, pdf, picture, plan, qr_view, restore, wallet_backup,
};

/// What a wallet's files are made from, owned: its name and policy, the
/// network, the fingerprints its sheet lists and the slider's setting.
pub(crate) struct Inputs {
    name: String,
    policy: WalletPolicy,
    network: Network,
    /// Each key slot's fingerprint, as the wallet sheet lists them.
    slots: Vec<Option<Fingerprint>>,
    /// How many keys the shares leave off a sheet, where the backup under
    /// way set it.
    omit: Option<usize>,
    /// A silent payments record a loaded key contradicts: it has no
    /// public files.
    blank: bool,
}

impl Inputs {
    /// The inputs for `w` of `session`; `omit` is the backup's setting and
    /// `blank` that the wallet has no public files.
    pub(crate) fn of(session: &Session, w: &Wallet, omit: Option<usize>, blank: bool) -> Self {
        Self {
            name: w.name.clone(),
            policy: w.policy().clone(),
            network: session.network(),
            slots: session.slots(w).iter().map(|s| s.fingerprint).collect(),
            omit,
            blank,
        }
    }

    fn wallet(&self) -> Wallet {
        Wallet::new(
            self.name.clone(),
            self.policy.clone(),
            String::new(),
            Origin::Named,
        )
    }
}

/// Files made, with what the making said.
#[derive(Default)]
pub(crate) struct Made {
    pub(crate) files: Vec<(String, Vec<u8>)>,
    /// The last toast the making would have said.
    pub(crate) toast: Option<String>,
}

/// Costs what [`pictures`] and `pdf::sheet` cost, and for the multisig
/// config (2) or the split sheets (4) what `restore::rebuilds` costs per
/// set tried. The files kind `what` makes with their bytes (as
/// `Faraday::public_out` numbers them); the config or the sheets only where
/// they rebuild the wallet, and otherwise none, which the toast says.
pub(crate) fn made(c: &Compute, inp: &Inputs, what: u8) -> Made {
    let w = inp.wallet();
    let refused = match what {
        2 if wallet_backup::multisig_config(&w, None).is_some()
            && restore::checked_config(c, inp.network, &w).is_none() =>
        {
            Some(EN.backup_config_rebuilds_another)
        }
        4 if wallet_backup::splits(&w) => {
            let (m, n) = Session::quorum(&w);
            let plan = wallet_backup::split_plan(n, m, inp.omit.unwrap_or(m.saturating_sub(1)));
            (!restore::sheets_rebuild(c, inp.network, &w, &plan))
                .then_some(EN.backup_sheets_rebuild_another)
        }
        _ => None,
    };
    if let Some(why) = refused {
        return Made {
            files: Vec::new(),
            toast: Some(why.to_string()),
        };
    }
    files(c, inp, what, true)
}

/// Costs an address derivation, or `checked_config`. A wallet's public
/// text file `what` (2 the multisig config where it rebuilds the wallet, 6
/// BIP 129's).
pub(crate) fn file_text(c: &Compute, net: Network, w: &Wallet, what: u8) -> Option<String> {
    match what {
        2 => restore::checked_config(c, net, w),
        6 => {
            let first = w.policy().address_at(c, net, false, 0).ok()?;
            Some(
                faraday_bip::bsms::DescriptorRecord {
                    policy: w.policy().clone(),
                    paths: vec!["/0/*".to_string(), "/1/*".to_string()],
                    first_address: first.to_string(),
                    network: net,
                }
                .to_text(),
            )
        }
        _ => None,
    }
}

/// Costs a QR encoding (through `picture_codes`). The labelled pictures
/// among a wallet's public files `what`: for 8, the descriptor's code, in
/// parts past one code; for 4, each split share's code, one beside each
/// share's sheet and text; for 14, Specter DIY's `addwallet` in its numbered
/// parts. None for any other file.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn pictures(c: &Compute, inp: &Inputs, what: u8) -> Vec<picture::Labelled> {
    if inp.blank {
        return Vec::new();
    }
    let w = &inp.wallet();
    let stem = file_stem(&w.name);
    let net = inp.network;
    let shape = if net.is_mainnet() {
        Session::shape(w)
    } else {
        format!("{} · {}", Session::shape(w), net.name())
    };
    let keys = w.policy().keys();
    let fps = |pick: &dyn Fn(usize) -> bool| -> Vec<String> {
        keys.iter()
            .enumerate()
            .filter(|(i, _)| pick(*i))
            .filter_map(|(_, k)| k.fingerprint().map(fp_text))
            .collect()
    };
    // Each code of `text` as a picture: `name` when there is one,
    // `name{part}j-of-k` for each part when there are more.
    let codes_of = |name: &str, part: &str, text: &str, lines: Vec<String>| {
        let codes = picture::picture_codes(c, text).unwrap_or_default();
        let k = codes.len();
        let names = picture::picture_names(name, part, text);
        codes
            .into_iter()
            .zip(names)
            .enumerate()
            .map(|(j, (code, name))| {
                let mut label = if k == 1 {
                    Vec::new()
                } else {
                    vec![fill(
                        EN.public_pictures_part,
                        &[&(j + 1).to_string(), &k.to_string()],
                    )]
                };
                label.extend(lines.iter().cloned());
                picture::Labelled {
                    name,
                    code,
                    title: w.name.clone(),
                    lines: label,
                }
            })
            .collect::<Vec<_>>()
    };
    match what {
        8 => {
            let text = w.policy().to_descriptor_checksummed();
            let all = fps(&|_| true);
            let mut lines = vec![shape];
            if !all.is_empty() {
                let word = if all.len() == 1 {
                    EN.public_pictures_key
                } else {
                    EN.public_pictures_keys
                };
                lines.push(format!("{word} {}", all.join(" · ")));
            }
            if let Some((_, sum)) = text.rsplit_once('#') {
                lines.push(fill(EN.public_pictures_descriptor, &[sum]));
            }
            lines.push(EN.public_pictures_public_watch.to_string());
            codes_of(&format!("{stem}-descriptor"), "-", &text, lines)
        }
        4 => {
            let (m, n) = Session::quorum(w);
            // As the shares' sheets and text are split.
            let omit = inp.omit.unwrap_or(m.saturating_sub(1));
            let plan = wallet_backup::split_plan(n, m, omit);
            let mut out = Vec::new();
            for (i, row) in plan.iter().enumerate() {
                let Some(text) = wallet_backup::split_share(w, row) else {
                    continue;
                };
                let mut lines = vec![
                    fill(
                        EN.public_pictures_share,
                        &[
                            &(i + 1).to_string(),
                            &plan.len().to_string(),
                            &m.to_string(),
                        ],
                    ),
                    shape.clone(),
                ];
                let held = fps(&|k| row.contains(&k));
                if !held.is_empty() {
                    lines.push(fill(
                        EN.public_pictures_holds,
                        &[&held.join(" · ").to_string()],
                    ));
                }
                let off = fps(&|k| !row.contains(&k));
                if !off.is_empty() {
                    lines.push(fill(
                        EN.public_pictures_leaves,
                        &[&off.join(" · ").to_string()],
                    ));
                }
                lines.push(EN.public_pictures_not_wallet.to_string());
                lines.push(EN.public_pictures_public.to_string());
                out.extend(codes_of(
                    &format!("{stem}-share-{}-of-{n}", i + 1),
                    "-part-",
                    &text,
                    lines,
                ));
            }
            out
        }
        // Specter DIY's `addwallet` command, in Specter's numbered
        // parts, which Specter DIY scans, past one code.
        14 => qr_view::specter_diy_pictures(c, w, shape, fps(&|_| true), &stem),
        _ => Vec::new(),
    }
}

/// Costs what the files cost: [`file_text`], `pdf::sheet`, [`pictures`].
/// A wallet's public files `what`, their pictures and PDFs made only when
/// `bytes`, else named with no bytes. A PDF that cannot be made is
/// named in the toast.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn files(c: &Compute, inp: &Inputs, what: u8, bytes: bool) -> Made {
    let mut out = Made::default();
    if inp.blank {
        return out;
    }
    let w = &inp.wallet();
    let net = inp.network;
    let stem = file_stem(&w.name);
    let files = &mut out.files;
    match what {
        // The blank templates are printed from Faraday Starter: none is
        // made here. The sheets go out as PDFs, ready to print anywhere:
        // they hold nothing secret.
        1 => {
            files.push((
                format!("{stem}-descriptor.txt"),
                format!("{}\n", w.policy().to_descriptor_checksummed()).into_bytes(),
            ));
        }
        5 => files.push((
            format!("{stem}-wallet.json"),
            wallet_backup::wallet_json(&w.name, w.policy()).into_bytes(),
        )),
        6 => {
            if let Some(record) = file_text(c, net, w, 6) {
                files.push((format!("{stem}-bsms.txt"), record.into_bytes()));
            }
        }
        7 => files.push((
            format!("{stem}-bitcoin-core.json"),
            faraday_bip::core_import::import_descriptors(
                &w.policy().to_descriptor_checksummed(),
                faraday_bip::core_import::Rescan::Start,
            )
            .into_bytes(),
        )),
        2 => {
            if let Some(t) = file_text(c, net, w, 2) {
                files.push((format!("{stem}-multisig-config.txt"), t.into_bytes()));
            }
        }
        3 if !bytes => files.push((format!("{stem}-backup.pdf"), Vec::new())),
        3 => {
            let sheet = wallet_backup::sheet_wallet_on(c, net, &inp.slots, w);
            match pdf::sheet(c, &sheet) {
                Ok(p) => files.push((format!("{stem}-backup.pdf"), p)),
                Err(e) => out.toast = Some(fill(EN.public_made_no_pdf, &[&e.to_string()])),
            }
        }
        4 => {
            let (m, n) = Session::quorum(w);
            let plan = wallet_backup::split_plan(n, m, inp.omit.unwrap_or(m.saturating_sub(1)));
            let pics = pictures(c, inp, 4);
            for (i, row) in plan.iter().enumerate() {
                if let Some(share) = wallet_backup::split_share(w, row) {
                    // The sheet to print beside each file.
                    let pdf_name = format!("{stem}-share-{}-of-{n}.pdf", i + 1);
                    if !bytes {
                        files.push((pdf_name, Vec::new()));
                    } else if let Ok(pdf) =
                        pdf::sheet(c, &wallet_backup::sheet_share(w, i + 1, plan.len(), &share))
                    {
                        files.push((pdf_name, pdf));
                    }
                    files.push((
                        format!("{stem}-share-{}-of-{n}.txt", i + 1),
                        share.into_bytes(),
                    ));
                    // And its code, to scan back.
                    let share = format!("{stem}-share-{}-of-{n}", i + 1);
                    for p in pics.iter().filter(|p| {
                        p.name
                            .strip_prefix(share.as_str())
                            .is_some_and(|rest| rest == ".png" || rest.starts_with("-part-"))
                    }) {
                        files.push((p.name.clone(), if bytes { p.png(c) } else { Vec::new() }));
                    }
                }
            }
        }
        8 | 14 => {
            for p in pictures(c, inp, what) {
                files.push((p.name.clone(), if bytes { p.png(c) } else { Vec::new() }));
            }
        }
        9 => {
            if let Some(t) = faraday_bip::core_import::import_descriptors_pair(w.policy()) {
                files.push((
                    format!("{stem}-bitcoin-core-before-29.json"),
                    t.into_bytes(),
                ));
            }
        }
        10 => {
            if let Some(t) = faraday_intake::electrum::file(w.policy()) {
                files.push((format!("{stem}-electrum.json"), t.into_bytes()));
            }
        }
        11 => {
            let net = wallet::policy_network(w.policy(), net).unwrap_or(net);
            if let Some(t) = faraday_intake::caravan::file(&w.name, w.policy(), net) {
                files.push((format!("{stem}-caravan.json"), t.into_bytes()));
            }
        }
        12 => {
            if let Some(t) = faraday_intake::specter_diy::command(&w.name, w.policy()) {
                files.push((format!("{stem}-specter-diy.txt"), t.into_bytes()));
            }
        }
        13 => {
            if let Some(t) = faraday_intake::single_key::wasabi_file(w.policy()) {
                files.push((format!("{stem}-wasabi.json"), t.into_bytes()));
            }
        }
        _ => {}
    }
    out
}

/// One step of making a set of files, in the order the files go out.
pub(crate) enum Part {
    /// The files of kind `what` ([`made`]).
    Kind(u8),
    /// The checklist's public file `what`: its labelled `picture` where the
    /// form chosen has one, its text where it has the text or the picture
    /// came to nothing; the text's files first, then the picture.
    Public {
        what: u8,
        picture: Option<Picture>,
        text: bool,
    },
    /// Files already made (the account keys').
    Ready(Vec<(String, Vec<u8>)>),
}

/// The picture of a public file.
pub(crate) enum Picture {
    /// The descriptor's code (kind 8).
    Descriptor,
    /// The code of the wallet's text file `what` (2 or 6), under `title`,
    /// named `name`, with `lines` beneath it.
    Code {
        what: u8,
        title: String,
        name: String,
        lines: Vec<String>,
    },
}

/// Costs what the steps cost. The files of `parts`, in order.
pub(crate) fn parts_made(c: &Compute, inputs: &Inputs, parts: Vec<Part>) -> Made {
    let w = inputs.wallet();
    let mut out = Made::default();
    let add = |out: &mut Made, m: Made| {
        out.files.extend(m.files);
        out.toast = m.toast.or(out.toast.take());
    };
    for part in parts {
        match part {
            Part::Kind(what) => add(&mut out, made(c, inputs, what)),
            Part::Public {
                what,
                picture,
                text,
            } => {
                let mut shown = Made::default();
                match picture {
                    Some(Picture::Descriptor) => shown = made(c, inputs, 8),
                    Some(Picture::Code {
                        what,
                        title,
                        name,
                        lines,
                    }) => {
                        shown.files.extend(
                            file_text(c, inputs.network, &w, what)
                                .and_then(|t| qr_view::QrView::text(c, &title, &t).ok())
                                .and_then(|v| v.public(&name, lines).png(c)),
                        );
                    }
                    None => {}
                }
                if shown.files.is_empty() || text {
                    add(&mut out, made(c, inputs, what));
                }
                add(&mut out, shown);
            }
            Part::Ready(files) => out.files.extend(files),
        }
    }
    out
}

/// What an answer goes on to do once its files are For the stick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Then {
    /// A wallet's files of one kind.
    Out,
    /// The backup's checklist opens.
    Checklist,
    /// The checklist's Make the files: the backup's screen is brought up
    /// to date with them.
    Files,
}

/// A set of files to make on the worker: the steps and what they are made
/// from, owned.
pub(crate) struct FilesWork {
    inputs: Inputs,
    parts: Vec<Part>,
    wallet: usize,
    then: Then,
    screen: Screen,
    sheet: Option<Sheet>,
}

/// A set of files made.
pub(crate) struct FilesDone {
    made: Made,
    wallet: usize,
    then: Then,
    screen: Screen,
    sheet: Option<Sheet>,
}

impl FilesWork {
    /// Costs what the steps cost. The files made, as the main thread's
    /// making ([`made`]) makes them.
    pub(crate) fn run(self, c: &Compute) -> FilesDone {
        let Self {
            inputs,
            parts,
            wallet,
            then,
            screen,
            sheet,
        } = self;
        let made = parts_made(c, &inputs, parts);
        FilesDone {
            made,
            wallet,
            then,
            screen,
            sheet,
        }
    }
}

/// Starts making wallet `wallet`'s files `parts` on the worker; `then` is
/// where the answer goes on.
pub(crate) fn start(app: &mut Faraday, wallet: usize, parts: Vec<Part>, then: Then) {
    let c = app.compute();
    let Some(inputs) = app.public_inputs(&c, wallet) else {
        return;
    };
    let work = FilesWork {
        inputs,
        parts,
        wallet,
        then,
        screen: app.screen,
        sheet: app.sheet,
    };
    app.start(Job::Files(Box::new(work)), EN.working_files);
}

/// Costs what `put_outbox` costs. A set of files' answer: the files go
/// For the stick and the toast is said. Dropped where the person is no
/// longer on the screen and sheet they pressed on, or the backup is no
/// longer open on the wallet the files are for.
pub(crate) fn files_made(app: &mut Faraday, done: FilesDone) {
    let FilesDone {
        made,
        wallet,
        then,
        screen,
        sheet,
    } = done;
    if (app.screen, app.sheet) != (screen, sheet) {
        return;
    }
    if then != Then::Out && app.backup.state().is_none_or(|b| b.wallet != wallet) {
        return;
    }
    app.put_public(wallet, made);
    match then {
        Then::Out => {}
        Then::Checklist => checklist_opened(app),
        Then::Files => app.backups_sync(),
    }
}

/// Costs what `backup_items_now` costs, and the account keys'
/// derivations (`account_key_files`). The wallet the backup under way is
/// for and the steps that make its files, in the order they go out: the
/// public files of the form chosen, as `backup_public_names` lists them
/// (the picture, the text, or both), and each key's account key files with
/// them. The account keys' files are made here, from the keys held, and go
/// to the worker made.
fn checklist_parts(app: &mut Faraday) -> Option<(usize, Vec<Part>)> {
    let c = app.compute();
    let b = app.backup.state()?;
    let w = b.wallet;
    let (qr, text) = (
        b.answers.form[plan::form::QR],
        b.answers.form[plan::form::TEXT],
    );
    let split = app
        .session
        .wallets
        .get(w)
        .is_some_and(|wl| b.answers.split && wallet_backup::splits(wl));
    let items: Vec<plan::Item> = app
        .backup_items_now(&c)
        .into_iter()
        .filter_map(bstep::item)
        .collect();
    let mut parts = Vec::new();
    for item in items {
        match item {
            plan::Item::Sheets => parts.push(Part::Kind(if split { 4 } else { 3 })),
            plan::Item::PublicFiles => {
                for what in app.backup_public() {
                    let picture = match what {
                        _ if !qr => None,
                        1 => Some(Picture::Descriptor),
                        2 | 6 => app.session.wallets.get(w).map(|wl| {
                            let (title, name, lines) = app.wallet_code_head(wl, what);
                            Picture::Code {
                                what,
                                title,
                                name,
                                lines,
                            }
                        }),
                        _ => None,
                    };
                    parts.push(Part::Public {
                        what,
                        picture,
                        text,
                    });
                }
                parts.push(Part::Ready(app.account_key_files(&c, w)));
            }
            _ => {}
        }
    }
    Some((w, parts))
}

/// Starts making the checklist's files on the worker, as the Make the
/// files press does: the blank template, the sheet or the shares, and the
/// files for the software chosen. Made again, each replaces its namesake.
pub(crate) fn files_start(app: &mut Faraday) {
    if let Some((w, parts)) = checklist_parts(app) {
        start(app, w, parts, Then::Files);
    }
}

/// Starts making the checklist's files on the worker; the checklist opens
/// with the answer.
pub(crate) fn checklist_start(app: &mut Faraday) {
    if let Some((w, parts)) = checklist_parts(app) {
        start(app, w, parts, Then::Checklist);
    }
}

/// Opens the checklist once its files are made: its first item not yet
/// done is open.
fn checklist_opened(app: &mut Faraday) {
    let c = app.compute();
    let first = app
        .backup_items_now(&c)
        .into_iter()
        .find(|&n| bstep::item(n).is_some_and(|it| !app.backup_item_done_now(&c, it)));
    app.backup.checklist();
    app.backup_item_open(first);
    app.backups_sync();
}
