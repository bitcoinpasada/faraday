//! Where event code gets its token (`docs/COMPUTE.md` §3).
//!
//! Costly work takes a [`Compute`]. Code that holds `&mut Faraday` (an
//! event, a press, an act, a tick, a storage event, boot, the warm step
//! before a frame) mints one here; drawing holds only `&View` or
//! `&Faraday` and cannot. The token notes its work with this crate's
//! counter ([`crate::work`]).

use faraday_crypto::work::Compute;

use crate::Faraday;

impl Faraday {
    /// A token for costly work, for code that holds the app to change
    /// it. Cheap.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: event code holding &mut Faraday (docs/COMPUTE.md §3)"
    )]
    pub fn compute(&mut self) -> Compute {
        Compute::new(crate::work::core_count)
    }
}
