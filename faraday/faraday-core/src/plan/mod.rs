//! The backup plan: its logic is the `faraday-plan` crate, re-exported
//! here under the names the app has always used (`docs/BACKUP-FLOW.md`);
//! [`cards`] draws it.

pub(crate) mod cards;
pub use faraday_plan::*;
