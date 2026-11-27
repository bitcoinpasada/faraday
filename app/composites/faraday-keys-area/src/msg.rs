use faraday_kernel::{Ending, Gate, Message, ScanFor};
use faraday_organisms::Pressed;
use faraday_present::FlowPress;

use crate::Page;
use crate::SampleAsk;

#[derive(Message)]
pub enum Msg {
    KeyGen(faraday_keygen::Msg),
    Randomness(faraday_randomness::Msg),
    CodexShares(faraday_codex_shares::Msg),
    AddKey(faraday_add_key::Msg),
    Krux(faraday_krux::Msg),
    Bip85(faraday_bip85::Msg),
    Convert(faraday_convert::Msg),
    Vanity(faraday_vanity::Msg),
    SecureBoot(faraday_secure_boot::Msg),
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

/// What the composite tells its parent.
#[derive(Message)]
pub enum Out {
    Open(Page),
    Leave(Ending),
    Scan(ScanFor),
    Camera(bool),
    Reveal(Gate),
    Sample(SampleAsk),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    KeyGen(FlowPress<faraday_keygen::Step, faraday_keygen::Press>),
    Randomness(faraday_randomness::Press),
    CodexShares(faraday_codex_shares::Press),
    AddKey(faraday_add_key::Press),
    Krux(faraday_krux::Press),
    Bip85(FlowPress<faraday_bip85::Step, faraday_bip85::Press>),
    Convert(FlowPress<faraday_convert::Step, faraday_convert::Press>),
    Vanity(faraday_vanity::Press),
    SecureBoot(faraday_secure_boot::Press),
}
