use faraday_kernel::Message;
use faraday_organisms::Pressed;

#[derive(Message)]
pub enum Msg {
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// Begin Make a new wallet (06 §4.3).
    // Spec: 06 §4.3 names `home::Out::CreateWallet` without a shape.
    CreateWallet,
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
