//! The presses whose work waits for a frame drawn first
//! (`docs/SPEED.md` §9): what [`slow`] lists, and what the frame says
//! meanwhile.

use crate::Action;
use crate::strings::EN;
use crate::{boot_import, seeds, vanity};

/// Whether a press stretches a key (PBKDF2, scrypt, Argon2) or makes more
/// than one picture or PDF, so that the frame that shows it is drawn
/// first and its work is done on the tick after (`docs/SPEED.md` §9). A
/// press that does such work and is not here fails the guard in
/// `Faraday::press_in` in every test.
pub(crate) fn slow(a: Action) -> bool {
    use crate::backup_flow::steps::FlowAction as F;
    use crate::glance::sheet::ChartAction as C;
    use crate::vault::VaultAction as V;
    matches!(
        a,
        Action::BChecklist
            | Action::BC32(crate::codex32_backup::Press::Set(
                crate::codex32_backup::SetPress::MakeNew,
            ))
            | Action::BNewVault(_)
            | Action::CNext(_)
            | Action::CPassLock
            | Action::CSlotVault(..)
            | Action::EntryAdd
            | Action::EntryRecover
            | Action::Flow(F::CopyKey | F::LockWrite | F::WalletLock)
            | Action::Import(boot_import::ImportAction::Go)
            | Action::InboxLoad
            | Action::KLock
            | Action::KNext
            | Action::Own(crate::fresh::page::Act::Skip)
            | Action::LKey(_)
            | Action::LResolve
            | Action::LoadKey(_)
            | Action::Lock
            | Action::NewWalletFrom(_)
            | Action::SamplesLoad
            | Action::ScanSample
            | Action::SamplePsbt
            | Action::PRecord(_)
            | Action::POut
            | Action::PotentialMake
            | Action::SecretHold
            | Action::SeedRead(seeds::read::Choice::Plain | seeds::read::Choice::WithPass)
            | Action::Seeds(seeds::SeedsAction::Make | seeds::SeedsAction::Sample(_))
            | Action::SelfTestRun
            | Action::Vanity(vanity::VanityAction::Use)
            | Action::Chart(
                C::Checked(..) | C::Confirm(_) | C::Do(..) | C::Edit(..) | C::PassTry(_)
            )
            | Action::Vault(
                V::AddFile(_)
                    | V::AddKind(_)
                    | V::Category(_)
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
                    | V::SaveWallet(_)
            )
    )
}

/// What the frame drawn before a slow press's work says, when the work
/// is long enough to be seen: pictures and PDFs made.
pub(crate) fn working_text(a: Action) -> Option<&'static str> {
    use crate::backup_flow::steps::FlowAction as F;
    matches!(
        a,
        Action::BChecklist | Action::SecretHold | Action::Flow(F::WalletLock)
    )
    .then_some(EN.working_files)
}
