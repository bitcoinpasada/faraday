use faraday_kernel::{AnyChange, ChangeId, Message};
use faraday_present::Pressed;

#[derive(Message)]
pub enum Msg {
    List(faraday_vault_list::Msg),
    Unlock(faraday_vault_unlock::Msg),
    Contents(faraday_vault_contents::Msg),
    Seal(faraday_vault_seal::Msg),
    Totp(faraday_totp::Msg),
    Gpg(faraday_gpg::Msg),
    Press(Press),
    #[withheld]
    Given(ChangeId, AnyChange),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    List(faraday_vault_list::Press),
    Unlock(faraday_vault_unlock::Press),
    Contents(faraday_vault_contents::Press),
    Seal(faraday_vault_seal::Press),
    Totp(faraday_totp::Press),
    Gpg(faraday_gpg::Press),
}
