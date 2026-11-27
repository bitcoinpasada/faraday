//! The `default` tour: Home as the device starts on it, and its search over the guide.

use faraday_fixture_adapters::Scripted;
use faraday_port::Text;

use crate::runner::Tour;

pub fn run(t: &mut Tour) -> Result<(), String> {
    t.step("home")?;
    t.press("faraday-home.Focus")?;
    t.input(&[Scripted::Text(Text::of("seed"))]);
    t.step("typed")?;
    t.press("faraday-home.Clear")?;
    t.step("cleared")
}
