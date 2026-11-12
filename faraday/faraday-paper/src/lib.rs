//! What Faraday prints and what it reads of a display, with no app in it:
//! the PDF sheets and their writer ([`pdf`]), the EDID rules ([`edid`])
//! and the build ID ([`build`]). faraday-core and the Starter use them.

pub mod build;
pub mod edid;
pub mod pdf;
