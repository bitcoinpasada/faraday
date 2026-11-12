//! The integration tests of faraday-host, one module per subject: Faraday
//! Demo's host, the strip it offers for each screen, and the Pi panel as
//! the preview frames it.

// The shell and the helpers faraday-core's own tests share.
#[path = "../../../faraday-core/tests/all/common.rs"]
mod common;
mod desktop_host;
mod pi_panel;
mod strip_codex32_dice;
mod strip_codex32_rest;
mod strip_codex32_roller;
mod strip_demo;
mod strip_keygen;
mod strip_paste_a_seed;
mod strips;
