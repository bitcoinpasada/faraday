//! The `settings` tour: Home, then Settings from the navigation.

use crate::runner::Tour;

pub fn run(t: &mut Tour) -> Result<(), String> {
    t.step("home")?;
    // The destinations send one press, in the bar's order; Settings is the sixth drawn.
    t.press_nth("faraday-device-product.Open", 5)?;
    t.step("settings")
}
