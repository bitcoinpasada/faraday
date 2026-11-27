use faraday_kernel::{Gate, Message, ScanFor};
use faraday_present::Pressed;

#[derive(Message)]
pub enum Msg {
    BootImport(faraday_boot_import::Msg),
    StickVisit(faraday_stick_visit::Msg),
    Zeroing(faraday_zeroing::Msg),
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
    Scan(ScanFor),
    Reveal(Gate),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Files(FilesPress),
    BootImport(faraday_boot_import::Press),
    StickVisit(faraday_stick_visit::Press),
    Zeroing(faraday_zeroing::Press),
}

/// What a press on the Files page itself sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum FilesPress {}
