//! The `power` tour: Home, the Power sheet from the top bar, and Cancel putting it away.

use crate::runner::Tour;

pub fn run(t: &mut Tour) -> Result<(), String> {
    t.step("home")?;
    t.press("faraday-device-product.Power")?;
    t.step("power-sheet")?;
    t.press("faraday-power.Away")?;
    t.step("away")
}
