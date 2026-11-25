//! Where a wallet's files go on its sticks (`docs/BACKUP-FLOW.md` §10.2,
//! §17.4): the stick of the map each of the flow's sticks is, which of a
//! seed's files a stick takes, the stick that carries the public files,
//! and a plan changed after the keys were made.

use crate::backup_flow::WalletFlow;
use crate::backup_flow::wallet::sealed_marks;
use crate::backup_flow::{BackupFlow, Step, Target, seed_stick_file_of, seed_stick_name};
use crate::file_stem;
use crate::plan;
use crate::strings::EN as FEN;
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};
use faraday_vault::records::kind as rkind;

/// The stick at a place on the map `boxes` that the flow's stick for
/// `target` is, as a list of none or one ([`crate::backup_flow::target_stick`],
/// `docs/BACKUP-FLOW.md` §17.4): each stick of the map is one stick of the
/// flow.
pub(crate) fn target_sticks(boxes: &[plan::Spot], target: Target) -> Vec<plan::At> {
    crate::backup_flow::target_stick(boxes, target)
        .into_iter()
        .collect()
}

/// Whether `name` is a seed file of this wallet: `{stem}-{fp}-words.txt`,
/// `-seedqr.png` or `-compactseedqr.png` for one of its seeds `fps`,
/// exactly.
pub(crate) fn seed_file_of(stem: &str, fps: &[impl AsRef<str>], name: &str) -> bool {
    fps.iter().any(|fp| {
        let fp = fp.as_ref();
        let lower: Text = fp.chars().flat_map(char::to_lowercase).collect();
        let head = text!("{stem}-{}-", &*lower);
        name.strip_prefix(&*head)
            .is_some_and(|rest| ["words.txt", "seedqr.png", "compactseedqr.png"].contains(&rest))
    })
}

impl BackupFlow {
    /// Costs what `sealed_marks` costs. A plan changed after the keys were
    /// made: a vault that now holds more than it was sealed with is as one
    /// added (the round puts it in and its sticks are written again), and a
    /// vault the plan has and the flow has not made starts the round. With
    /// `advance`, a flow that has not reached the writes goes on to the
    /// lock.
    pub(crate) fn made_plan_changed(&mut self, c: &Compute, advance: bool) {
        let f = self;
        let stale: Vec<usize> = match f.wallet.as_ref() {
            Some(wf) => f
                .plan
                .used_vaults()
                .into_iter()
                .filter(|v| f.made_in.contains(v))
                .filter(|v| {
                    let had = wf
                        .sealed
                        .iter()
                        .find(|(x, _)| x == v)
                        .map_or(&[][..], |(_, s)| s.as_slice());
                    let now = sealed_marks(c, wf, *v);
                    // Anything of this wallet added to it or taken out of
                    // it: a seed, a passphrase, the description, a share
                    // (§10.2, §17.6).
                    now.iter().any(|s| !had.contains(s)) || had.iter().any(|m| !now.contains(m))
                })
                .collect(),
            None => Vec::new(),
        };
        if !stale.is_empty() {
            f.made_in.retain(|v| !stale.contains(v));
            f.sealing.retain(|s| !stale.contains(&s.vault));
            for k in 0..f.slots.len() {
                if f.vaults_of(k).iter().any(|v| stale.contains(v)) {
                    f.slots[k].written = None;
                }
            }
        }
        let missing: Vec<usize> = f
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| !f.made_in.contains(v))
            .collect();
        if !missing.is_empty() {
            f.adding = true;
            let bind = (0..f.slots.len()).any(|s| {
                let vs = f.vaults_of(s);
                f.slots[s].bound.is_none()
                    && vs.iter().any(|v| {
                        missing.contains(v)
                            && f.plan.vaults[*v].existing
                            && !f.plan.vaults[*v].found()
                    })
            });
            f.step = if bind { Step::Sticks } else { Step::Vaults };
        } else if advance && f.step < Step::Write && f.step != Step::Lock {
            f.step = Step::Lock;
        }
    }

    /// The slots whose write keeps seed `s` on a stick: those of the
    /// vaults that hold it, and the stick of files for its seed file.
    pub(crate) fn seed_slots(&self, s: usize) -> Vec<usize> {
        let Some(wf) = self.wallet.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<usize> = Vec::new();
        for v in wf.answers.vaults_made(&wf.shape) {
            if wf.answers.vault_seeds(&wf.shape, v).contains(&s) {
                out.extend((0..self.slots.len()).filter(|&k| self.vaults_of(k).contains(&v)));
            }
        }
        // A stick counts for the seed's files only where it is given one
        // of them (its words, its SeedQR, its passphrase), never from the
        // plan alone: a stick of files, or a vault's stick with files
        // beside the vault (`docs/BACKUP-FLOW.md` §17.4).
        out.extend(
            (0..self.slots.len())
                .filter(|&k| self.slots[k].files.iter().any(|n| self.is_seed_file(s, n))),
        );
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `name` is one of seed `s`'s own files: on the stick of
    /// files, or on a stick at a place in any form.
    pub(crate) fn is_seed_file(&self, s: usize, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let Some(seed) = w.shape.seeds.get(s) else {
            return false;
        };
        let fps = [&*seed.name];
        seed_file_of(&file_stem(&w.name), &fps, name) || seed_stick_file_of(&fps, name)
    }

    /// Costs a map of the plan (through `map`). Whether `name` is a seed's
    /// file the plan keeps on the stick of slot `slot`: the stick of files
    /// takes the seeds kept there; a stick at a place the seeds' files the
    /// plan keeps on that stick, beside its vault or not
    /// (`docs/BACKUP-FLOW.md` §15.2, §17.4).
    pub fn seed_file_ok_for(&self, c: &Compute, slot: usize, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let Some(target) = self.slots.get(slot).map(|s| s.target) else {
            return false;
        };
        let boxes = plan::map(c, &w.shape, &w.answers);
        if target == Target::Files {
            let fps: Vec<&str> = plan::holds_at(&boxes, plan::At::Files)
                .into_iter()
                .filter_map(|what| {
                    w.shape
                        .seeds
                        .get(what.words_file_seed()?)
                        .filter(|x| x.here)
                })
                .map(|x| &*x.name)
                .collect();
            return seed_file_of(&file_stem(&w.name), &fps, name);
        }
        target_sticks(&boxes, target)
            .into_iter()
            .flat_map(|at| plan::holds_at(&boxes, at))
            .any(|what| {
                match what.seed_stick_file_seed() {
                    Some(i) => {
                        w.shape.seeds.get(i).filter(|x| x.here).is_some_and(|x| {
                            seed_stick_name(what, &x.name).as_deref() == Some(name)
                        })
                    }
                    // A codex32 string's file (`docs/CODEX32.md` §9.5).
                    None => crate::codex32_backup::stick_name_is(&w.shape, what, name),
                }
            })
    }

    /// Costs what `seeds_as_files` and `map` cost. Whether `name` is a
    /// seed's file of the wallet this flow is for that the plan keeps: on
    /// the stick of files, or on a stick at a place.
    pub fn seed_file_ok(&self, c: &Compute, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let kept = w.answers.seeds_as_files(c, &w.shape);
        let fps: Vec<&str> = w
            .shape
            .seeds
            .iter()
            .enumerate()
            .filter(|(s, x)| x.here && kept.contains(s))
            .map(|(_, s)| &*s.name)
            .collect();
        if seed_file_of(&file_stem(&w.name), &fps, name) {
            return true;
        }
        plan::map(c, &w.shape, &w.answers)
            .iter()
            .filter(|s| matches!(s.at, plan::At::Stick(..)))
            .flat_map(|s| s.holds.iter())
            .any(|(what, _)| {
                let seed = what
                    .seed_stick_file_seed()
                    .and_then(|i| w.shape.seeds.get(i).filter(|x| x.here));
                seed.is_some_and(|x| seed_stick_name(*what, &x.name).as_deref() == Some(name))
            })
    }
}

/// Whether `w` is a secret file: a seed's words, SeedQR or passphrase, or
/// a codex32 string, as a file on a stick. It spends, read as it is, so it
/// never shares the stick that carries the public files
/// (`docs/BACKUP-FLOW.md` §10.2).
pub(crate) fn is_secret_file(w: plan::What) -> bool {
    w.seed_stick_file_seed().is_some()
        || matches!(w, plan::What::Written(c)
            if c.form == plan::KeyForm::Codex32 && c.medium == plan::Medium::File)
}

/// The first stick of place `p` on a wallet's map `sticks`, in the flow's
/// order, that keeps no secret file ([`is_secret_file`]): the stick there
/// that may carry the public files.
pub(crate) fn public_stick_at(sticks: &[plan::Spot], p: usize) -> Option<Target> {
    (0..plan::sticks_at(sticks, p))
        .filter(|&k| {
            !plan::holds_at(sticks, plan::At::Stick(p, k))
                .into_iter()
                .any(is_secret_file)
        })
        .filter_map(|k| crate::backup_flow::stick_target(sticks, p, k))
        .min_by_key(|t| t.key())
}

impl BackupFlow {
    /// Costs what `wallet_replan_on` costs. The stick that would carry the
    /// public files, with the first secret file the plan keeps on it,
    /// were this flow locked on `answers` of `shape` and their map `m`;
    /// none when the plan can be written (`docs/BACKUP-FLOW.md` §10.2).
    pub(crate) fn public_clash(
        &self,
        c: &Compute,
        shape: &plan::Shape,
        answers: &plan::Answers,
        m: &plan::Mapped,
    ) -> Option<(Target, plan::What)> {
        if !plan::public_stick_on(answers, m) {
            return None;
        }
        let mut f = self.clone();
        if let Some(wf) = f.wallet.as_mut() {
            wf.shape = shape.clone();
            wf.answers = answers.clone();
        }
        f.wallet_replan_on(c, m);
        let t = f.slots.get(f.public_slot()?)?.target;
        let what = target_sticks(&m.map, t)
            .into_iter()
            .flat_map(|at| plan::holds_at(&m.map, at))
            .find(|w| is_secret_file(*w))?;
        Some((t, what))
    }
}

/// Costs what `read_wallet` costs per description in Files. The ids of
/// the wallets of checksum `cs`, each once, from the first place that has
/// one: the wallets loaded; the open vaults' records (a wallet, a plan's
/// wallet, a share's wallet); Files, the untrusted side, only where
/// neither gives any. A flow that kept its wallet's id
/// ([`flow_wallet_ids`]) does not ask. One id is the wallet; two or more
/// are never a pick.
fn wallet_ids_of(
    app: &crate::Faraday,
    c: &Compute,
    cs: &str,
) -> Vec<faraday_bip::policy::WalletId> {
    let mut ids: Vec<faraday_bip::policy::WalletId> = Vec::new();
    for wl in app.session.wallets.iter().filter(|wl| wl.checksum() == cs) {
        if !ids.contains(&wl.id()) {
            ids.push(wl.id());
        }
    }
    if !ids.is_empty() {
        return ids;
    }
    for o in &app.vaults.open {
        for (_, r) in app.vaults.records(o.id).iter() {
            if matches!(r.kind(), rkind::WALLET | rkind::PLAN | rkind::WALLET_SHARE)
                && let Some(d) = app.record_described(c, r)
                && d.checksum == cs
                && !ids.contains(&d.policy.id())
            {
                ids.push(d.policy.id());
            }
        }
    }
    if !ids.is_empty() {
        return ids;
    }
    let from_files = app
        .inbox
        .iter()
        .chain(app.outbox.iter())
        .filter(|it| it.kind == crate::wallet::FileKind::Wallet)
        .filter_map(|it| std::str::from_utf8(&it.bytes).ok().map(Text::of));
    for t in from_files {
        if let Ok(p) = crate::wallet::read_wallet(c, &t)
            && p.checksum() == cs
            && !ids.contains(&p.id())
        {
            ids.push(p.id());
        }
    }
    ids
}

/// Costs what `wallet_ids_of` costs, none for a flow that kept its
/// wallet's id. The ids of the wallet a flow is for: its own id where it
/// has one (a loaded twin never stands in), else those of its checksum.
pub(crate) fn flow_wallet_ids(
    app: &crate::Faraday,
    c: &Compute,
    wf: &super::WalletFlow,
) -> Vec<faraday_bip::policy::WalletId> {
    match (wf.id, wf.checksum.as_deref()) {
        (Some(id), _) => vec![id],
        (None, Some(cs)) => wallet_ids_of(app, c, cs),
        (None, None) => Vec::new(),
    }
}

/// Whether share `num` of the flow's wallet is among what a vault held
/// before the flow (`before`'s tokens). With the wallet's id or ids (one
/// wallet's, or two of one checksum), their tokens (and the old form, by
/// checksum `cs`); with none, a token by an id too (it cannot be told whose),
/// never an old-form token of another checksum, which is plainly another
/// wallet's.
pub(crate) fn share_held_before(
    before: &[Text],
    cs: &str,
    ids: &[faraday_bip::policy::WalletId],
    num: usize,
) -> bool {
    before.contains(&text!("s{cs}:{num}"))
        || match ids {
            [] => before.iter().any(|t| {
                t.strip_prefix('s')
                    .and_then(|r| r.strip_suffix(&*text!(":{num}")))
                    .is_some_and(|who| who.len() == 64)
            }),
            ids => ids
                .iter()
                .any(|id| before.contains(&text!("s{}:{num}", &*id.to_hex()))),
        }
}

/// Costs what `read_wallet`, `add_wallet` and `sheets_rebuild` cost. Share
/// `j` of the wallet with id `id` as a type 13 record
/// (`docs/BACKUP-FLOW.md` §15.6), made from its description: the wallet
/// loaded, or its description in an open vault or Files. `None` where no
/// description is found; a refusal, the toast to show, where the plan's
/// sheets do not rebuild the wallet.
pub(crate) fn wallet_share_record(
    app: &crate::Faraday,
    c: &Compute,
    wf: &WalletFlow,
    id: faraday_bip::policy::WalletId,
    j: usize,
) -> Option<Result<faraday_vault::Record, &'static str>> {
    use faraday_vault::records::field;
    type Made = Result<(Text, usize, Text), &'static str>;
    let of_wallet = |t: &str| {
        crate::wallet::read_wallet(c, t)
            .ok()
            .is_some_and(|p| p.id() == id)
    };
    let text = |wl: &crate::wallet::Wallet| -> Option<Made> {
        let (m, n) = crate::wallet::Session::quorum_keys(wl);
        let plan = crate::wallet_backup::split_plan(n, m, wf.answers.omit);
        let share = crate::wallet_backup::split_share(wl, plan.get(j)?)?;
        if !crate::restore::sheets_rebuild(c, app.session.network(), wl, &plan) {
            return Some(Err(FEN.backup_sheets_rebuild_another));
        }
        Some(Ok((
            Text::of(&share),
            plan.len(),
            Text::of(&crate::Faraday::wallet_text(wl)),
        )))
    };
    let made = match app.session.wallets.iter().find(|wl| wl.id() == id) {
        Some(wl) => text(wl),
        None => {
            // A wallet record, or a plan record's wallet (its field 1), in
            // any open vault; or the description file.
            let desc = app
                .vaults
                .open
                .iter()
                .flat_map(|o| {
                    app.vaults
                        .records(o.id)
                        .iter()
                        .filter_map(|(_, r)| match r.kind() {
                            rkind::WALLET => r.public_text(field::WALLET),
                            rkind::PLAN => r.public_text(field::PLAN_WALLET),
                            _ => None,
                        })
                        .map(Text::of)
                        .collect::<Vec<_>>()
                })
                .chain(
                    app.inbox
                        .iter()
                        .chain(app.outbox.iter())
                        .filter(|it| it.kind == crate::wallet::FileKind::Wallet)
                        .filter_map(|it| std::str::from_utf8(&it.bytes).ok().map(Text::of)),
                )
                .find(|t| of_wallet(t))?;
            let mut s = crate::wallet::Session::on(app.session.network());
            // A throwaway session that only reads the description back:
            // no sheet (F254).
            let k = s
                .add_wallet_kept(c, &wf.name, desc.trim(), "", crate::wallet::Origin::Named)
                .ok()?;
            text(&s.wallets[k])
        }
    };
    Some(made?.map(|(share, of, desc)| {
        faraday_vault::Record::new(rkind::WALLET_SHARE)
            .with(field::SHARE, share.as_bytes())
            .with(field::SHARE_WALLET, desc.as_bytes())
            .with(field::SHARE_WALLET_NAME, wf.name.as_bytes())
            .with(
                field::SHARE_NUMBER,
                &[u8::try_from(j + 1).unwrap_or(u8::MAX)],
            )
            .with(field::SHARE_OF, &[u8::try_from(of).unwrap_or(u8::MAX)])
    }))
}

#[cfg(test)]
mod tests {
    use super::{flow_wallet_ids, wallet_ids_of};
    use crate::testkit::{self, WALLET_ID_FIRST as FIRST, WALLET_ID_TWIN as TWIN};
    use crate::wallet::Origin;

    fn app_with(wallets: &[&str]) -> crate::Faraday {
        let c = testkit::compute();
        let mut app = crate::Faraday::new();
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        for (name, d) in ["First", "Twin"].into_iter().zip(wallets) {
            app.session
                .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
                .expect("the wallet reads");
        }
        app
    }

    /// The flow's copy of share records goes by a wallet's id only where
    /// one wallet of the checksum is here: one is its id, two or none are
    /// no id to go by.
    #[test]
    fn the_ids_of_a_checksum_are_one_none_or_two() {
        let c = testkit::compute();
        let first = crate::wallet::read_wallet(&c, &faraday_text::Text::of(FIRST)).unwrap();
        let cs = first.checksum();
        let one = wallet_ids_of(&app_with(&[FIRST]), &c, cs);
        assert_eq!(one.len(), 1);
        assert!(wallet_ids_of(&app_with(&[]), &c, cs).is_empty());
        let two = wallet_ids_of(&app_with(&[FIRST, TWIN]), &c, cs);
        assert_eq!(two.len(), 2);
        assert_ne!(two[0], two[1]);
    }

    /// A flow that kept its wallet's id goes by it: a twin loaded later
    /// never stands in, and no wallet need be loaded. One saved before
    /// the id was kept goes by what is here.
    #[test]
    fn a_flow_goes_by_its_own_id_and_a_loaded_twin_never_stands_in() {
        let c = testkit::compute();
        let first = crate::wallet::read_wallet(&c, &faraday_text::Text::of(FIRST)).unwrap();
        let mut wf = crate::backup_flow::WalletFlow::blank();
        wf.checksum = Some(faraday_text::Text::of(first.checksum()));
        wf.id = Some(first.id());
        let twin_only = app_with(&[TWIN]);
        assert_eq!(flow_wallet_ids(&twin_only, &c, &wf), vec![first.id()]);
        assert_eq!(flow_wallet_ids(&app_with(&[]), &c, &wf), vec![first.id()]);
        wf.id = None;
        let ids = flow_wallet_ids(&twin_only, &c, &wf);
        assert_eq!(ids.len(), 1);
        assert_ne!(
            ids[0],
            first.id(),
            "a flow with no id goes by what is loaded"
        );
    }

    /// A share the vault held before the flow is never taken out, whatever
    /// the flow can tell of its wallet after a restart: with no id found
    /// a share of that number held under any id counts (never an old-form token of
    /// another checksum), with one id its own does.
    #[test]
    fn a_share_held_before_is_told_by_its_number_where_no_id_is_found() {
        use super::share_held_before;
        let c = testkit::compute();
        let first = crate::wallet::read_wallet(&c, &faraday_text::Text::of(FIRST)).unwrap();
        let twin = crate::wallet::read_wallet(&c, &faraday_text::Text::of(TWIN)).unwrap();
        let cs = first.checksum();
        let before = [faraday_text::text!("s{}:2", &*first.id().to_hex())];
        assert!(share_held_before(&before, cs, &[], 2), "no id: the number");
        assert!(!share_held_before(&before, cs, &[], 1));
        let other = [faraday_text::text!("sdeadbeef:2")];
        assert!(
            !share_held_before(&other, cs, &[], 2),
            "another checksum's is plainly another wallet's"
        );
        assert!(share_held_before(&before, cs, &[first.id()], 2));
        assert!(share_held_before(&before, cs, &[first.id(), twin.id()], 2));
        assert!(
            !share_held_before(&before, cs, &[twin.id()], 2),
            "a twin's is its own"
        );
    }
}
