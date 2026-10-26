//! What Create a vault says of a passphrase's strength: the side panel's
//! line for the weakest, and the pair's own under it.

use crate::strings::{EN, fill};
use crate::vault::{CreateForm, Vaults};

impl CreateForm {
    /// The side panel's strength of the weakest passphrase, in bits with
    /// the unlock cost `cost`'s: known only when the dice made every one.
    /// Empty where every passphrase is Practice's `faraday` (`demo`: Practice's
    /// passphrase is offered), which is not measured (`docs/MODES.md`
    /// §6.2): no line is drawn.
    pub(crate) fn weakest_text(&self, cost: f32, demo: bool) -> String {
        let c = self;
        let own: Vec<Option<f32>> = (0..c.phrases.len())
            .map(|i| Vaults::phrase_bits(c, i))
            .collect();
        if !c.phrases.is_empty()
            && c.phrases
                .iter()
                .all(|(a, _)| demo && a.text.as_str() == crate::demo::PASSPHRASE)
        {
            String::new()
        } else if c.phrases.iter().all(|(a, _)| a.text.is_empty()) {
            EN.vault_no_passphrase.to_string()
        } else if let Some(min) = own
            .iter()
            .map(|b| b.map(|b| b + cost))
            .collect::<Option<Vec<f32>>>()
            .and_then(|v| v.into_iter().reduce(f32::min))
        {
            fill(EN.vault_bits, &[&format!("{min:.1}")])
        } else {
            EN.vault_not_measured.to_string()
        }
    }
}
