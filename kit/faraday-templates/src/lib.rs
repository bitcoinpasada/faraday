//! The kit's templates level: faraday-core's screens/hub_layout/, flow.rs, flow/,
//! screens/menu_page.rs, the frame of screens/mod.rs. Section 09 §3.3.

pub mod flow;
pub mod menu_page;
pub mod panes;
pub mod screen;

pub use faraday_atoms::{Drawn, Laid, Offered, Placement, PlainRef, Pressed, Shown, Style, Ui};

pub use faraday_organisms::content::{Block, Content, Organism};
pub use faraday_organisms::snackbar::SnackbarModel;
