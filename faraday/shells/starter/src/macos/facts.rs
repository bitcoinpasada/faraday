//! This computer's facts on a Mac (`docs/STARTER.md` §7.1, `docs/MAC.md`
//! §6), from what `sysctl` and `uname` print, with no macOS call in it.
//!
//! - `sysctl -n hw.model`: the model identifier (`MacBookPro14,1`).
//! - `uname -m`: `x86_64` or `arm64`, the machine this Starter runs on as
//!   macOS presents it to it.
//! - `sysctl -n sysctl.proc_translated`: `1` when this process runs
//!   under Rosetta (an x86_64 Starter on an Apple Silicon Mac), `0` when
//!   native; on an Intel Mac the name does not exist and `sysctl` fails
//!   (Apple, "About the Rosetta translation environment").
//! - `sysctl -n kern.hv_vmm_present`: `1` inside a virtual machine.
//!
//! Each is `None` here when the program failed or printed nothing.

use faraday_starter::api::{Computer, Concern, Kind, Os};
use faraday_starter::concern;

/// What the programs printed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    /// `sysctl -n hw.model`.
    pub model: Option<String>,
    /// `uname -m`.
    pub machine: Option<String>,
    /// `sysctl -n sysctl.proc_translated`.
    pub translated: Option<String>,
    /// `sysctl -n kern.hv_vmm_present`.
    pub vmm: Option<String>,
}

/// What every Intel Mac's firmware gives as its maker (DMI's
/// `sys_vendor` under Linux, `SystemManufacturer` under Boot Camp), so
/// the Starter reads a Mac the same on every system.
pub const MAKER: &str = "Apple Inc.";

fn said(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// This computer as the answers describe it. A Mac is ARM when macOS
/// says `arm64`, or says this x86_64 process is translated; when
/// `uname` said nothing, the architecture this Starter was built for.
pub fn computer(a: &Answers) -> Computer {
    let translated = said(&a.translated) == Some("1");
    let machine = said(&a.machine).unwrap_or(std::env::consts::ARCH);
    let arm = translated || machine.starts_with("arm") || machine.starts_with("aarch64");
    Computer {
        os: Os::MacOs,
        maker: MAKER.to_string(),
        model: said(&a.model).unwrap_or("").to_string(),
        family: String::new(),
        uefi: !arm,
        arm,
        secure_boot: None,
        tpm_unlock: false,
    }
}

/// What is of concern (§3.1a): a virtual machine. The screens and USB
/// devices are read on a Mac in batch A3c.
pub fn concerns(a: &Answers) -> Vec<Concern> {
    if said(&a.vmm) != Some("1") {
        return Vec::new();
    }
    let model = said(&a.model).unwrap_or("");
    vec![Concern {
        kind: Kind::VirtualMachine,
        name: concern::virtual_machine("", model, true).unwrap_or_default(),
        id: String::from("vm"),
    }]
}
