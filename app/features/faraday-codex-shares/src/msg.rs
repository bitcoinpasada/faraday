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
    /// Load a published sample.
    Sample(SampleAsk),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}

/// A published sample to load.
// Spec: 09 §4.4 names `SampleAsk` without a shape; it belongs to the kernel, which no
// feature may widen, so each feature that asks holds its own.
#[derive(Message)]
pub enum SampleAsk {}
