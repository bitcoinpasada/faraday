//! The costly presses: the ones that start a job on the worker and the
//! ones that run in the press (`docs/WORKERS.md` §2.1).

use crate::Action;
use crate::{boot_import, seeds, vanity};

/// Whether a press starts a job on the worker (`docs/WORKERS.md` §2.1):
/// its work is over the line, so the screen stays and shows the busy
/// line while the worker runs it. Practice's samples
/// loaded, the backup's checklist and its Make the files, a wallet's multisig config, split
/// shares and Specter DIY pictures, and Make the code and Compare of a
/// Krux code, in Tools and on the checklist, and Make the code and Open of a QR
/// vault in Tools, Check a timestamp's file over
/// 3 MiB (`ots_check::hash::JOB_BYTES`), Read of a Bitcoin Core wallet;
/// the other presses move
/// here from [`runs_in_press`] as they are shown to need it.
pub(crate) fn starts_job(a: Action) -> bool {
    matches!(
        a,
        Action::SamplesLoad
            | Action::Collect(_)
            | Action::BChecklist
            | Action::BFilesMake
            | Action::PublicOut(_, 2 | 4 | 14)
            | Action::OtsCheck(crate::ots_check::OtsAction::File(_))
            // Compare's search of 100 addresses on each chain is 200 key
            // derivations and curve multiplications: over the line by
            // count, up to 6.5 ms in a release build (`Kind::SeedCompare`).
            | Action::SeedCheck(
                crate::seed_check::SeedAction::CompareGo
                    | crate::seed_check::SeedAction::CompareFile(_),
            )
            | Action::CoreWallet(crate::core_wallet::tool::CoreWalletAction::Read)
            | Action::QrTool(
                crate::qr_tool::QrToolAction::Make | crate::qr_tool::QrToolAction::OpenCode,
            )
            | Action::KefTool(crate::kef_tool::KefToolAction::Backup(
                crate::kef_form::KefAct::Make | crate::kef_form::KefAct::Compare,
            ))
            | Action::KefTool(crate::kef_tool::KefToolAction::Form(
                crate::kef_form::KefAct::Make | crate::kef_form::KefAct::Compare,
            ))
    )
}

/// The rule the two lists keep, held in debug builds and every test: a
/// press that starts no job and does not run in the press stretches no
/// key and makes at most one picture. `done` is the work the press did.
pub(crate) fn guard(a: Action, done: &crate::work::Counts) {
    use crate::work::Work;
    debug_assert!(
        starts_job(a)
            || runs_in_press(a)
            || (done.of(Work::Kdf) == 0 && done.of(Work::Picture) <= 1),
        "{a:?} stretched a key {} times and made {} pictures and is on neither list: \
         add it to `runs_in_press` or `starts_job` (docs/WORKERS.md, the list)",
        done.of(Work::Kdf),
        done.of(Work::Picture),
    );
}

/// Whether a press stretches a key (PBKDF2, scrypt, Argon2) or makes more
/// than one picture or PDF and is under the line (`docs/WORKERS.md`
/// §2.1), so that it runs in the press itself with no frame drawn first
/// (`docs/SPEED.md` §9). A press that does such work and is on neither
/// list fails the guard in `Faraday::press_in` in every test.
pub(crate) fn runs_in_press(a: Action) -> bool {
    use crate::backup_flow::steps::FlowAction as F;
    use crate::glance::sheet::ChartAction as C;
    use crate::vault::VaultAction as V;
    matches!(
        a,
        Action::BC32(crate::codex32_backup::Press::Set(
            crate::codex32_backup::SetPress::MakeNew,
        )) | Action::BNewVault(_)
            | Action::CNext(_)
            | Action::CPassLock
            | Action::CSlotVault(..)
            | Action::EntryAdd
            | Action::EntryRecover
            | Action::Flow(F::CopyKey | F::LockWrite | F::WalletLock)
            | Action::Import(boot_import::ImportAction::Go)
            | Action::TextSeed(boot_import::TextSeedAnswer::Load)
            | Action::InboxLoad
            | Action::WalletHold
            | Action::KefRead(crate::seeds::kef::KefAct::ReadAsSeed)
            | Action::KLock
            | Action::KNext
            | Action::Own(crate::fresh::page::Act::Skip)
            | Action::LKey(_)
            | Action::LCln(Some(_))
            | Action::LResolve
            | Action::LFile(_)
            | Action::LoadKey(_)
            | Action::Lock
            | Action::LockUnsealed
            | Action::NewWalletFrom(_)
            | Action::ScanSample
            | Action::SamplePsbt
            | Action::PRecord(_)
            | Action::ReadGo(_)
            | Action::POut
            | Action::PotentialMake
            | Action::SecretHold
            | Action::SeedRead(seeds::read::Choice::Plain | seeds::read::Choice::WithPass)
            | Action::QrTool(
                crate::qr_tool::QrToolAction::Seed | crate::qr_tool::QrToolAction::AddToKey,
            )
            | Action::Seeds(seeds::SeedsAction::Make | seeds::SeedsAction::Sample(_))
            | Action::SelfTestRun
            // Under 10 ms in a release build (`docs/WORKERS.md` §2.1).
            | Action::SeedCheck(
                crate::seed_check::SeedAction::NoPass
                    | crate::seed_check::SeedAction::UsePass
                    | crate::seed_check::SeedAction::Keep,
            )
            | Action::Vanity(vanity::VanityAction::Use)
            // Save of a Seed XOR set stretches up to five parts (one
            // PBKDF2 each), under the line by count, not yet timed on a
            // Pi 3B+.
            | Action::Convert(
                crate::convert::ConvertAction::To(_)
                    | crate::convert::ConvertAction::VaultKey(..)
                    | crate::convert::ConvertAction::Make
                    | crate::convert::ConvertAction::Save
            )
            | Action::Chart(C::Checked(..) | C::Do(..) | C::PassTry(_))
            | Action::Vault(
                V::AddFile(_)
                    | V::AddKind(_)
                    | V::Category(_)
                    | V::CopyTo
                    | V::MoveTo
                    | V::Load
                    | V::LoadAll(_)
                    | V::LoadChosen(_)
                    | V::LoadKeyOf(..)
                    | V::LoadTicked
                    | V::LoadWithKeys(_)
                    | V::OlderOpen
                    | V::Open(_)
                    | V::OpenWalletOf(..)
                    | V::PromptGo
                    | V::SaveKey(_)
                    | V::SaveKeyWithPassphrase(_)
                    | V::SaveKeyKept(..)
                    | V::KeepSave
                    | V::SaveWallet(_)
            )
    )
}
