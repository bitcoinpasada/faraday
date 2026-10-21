//! The icon and keyboard widgets.
//!
//! [`Icon`] names the outline icons the icon font carries. [`keyboard`]
//! holds the key rows of every keyboard layout and the key and modifier
//! types a keyboard reports.

pub mod icon;
pub mod keyboard;

pub use icon::Icon;
