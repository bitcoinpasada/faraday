//! A tool or test-support crate: the leak flows. Every flow runs its runtime with
//! `Workers::Inline` (a runtime lane is a thread the scan cannot arm; 05 §9, 07 §6.2).
