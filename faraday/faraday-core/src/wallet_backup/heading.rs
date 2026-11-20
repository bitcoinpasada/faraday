//! A checklist item's title on its card, from the plan as it stands.

use super::kef;
use crate::Faraday;
use crate::plan;
use crate::strings::{EN, fill};

/// How many blank templates a card is for, as its title says.
pub(crate) fn templates_heading(k: usize) -> String {
    fill(
        EN.item_title_print_blank,
        &[
            &k.to_string(),
            (if k == 1 {
                EN.item_title_template_inline
            } else {
                EN.item_title_templates
            }),
        ],
    )
}

/// A checklist item's title on its card, from the plan as it stands:
/// what [`item_title`] reads first, by lookups only.
pub(crate) fn item_heading(app: &Faraday, shape: &plan::Shape, item: plan::Item) -> String {
    let Some(b) = app.backup() else {
        return String::new();
    };
    let a = &b.answers;
    let seed = |i: usize| shape.seeds.get(i).map_or("", |s| s.name.as_str());
    match item {
        plan::Item::Templates => {
            let Some(mapped) = app.kept.plan_mapped_kept(shape, a) else {
                return String::new();
            };
            let words_n = plan::templates_on(shape, &mapped);
            let c32 = plan::codex32::templates_of(a, &mapped);
            let kef = kef::templates(app, shape, &mapped);
            templates_heading(
                (words_n
                    + c32.iter().map(|&(_, _, n)| n).sum::<usize>()
                    + kef.iter().map(|&(_, n)| n).sum::<usize>())
                .max(1),
            )
        }
        plan::Item::Codex32(i) => fill(EN.c32_item_title, &[seed(i)]),
        plan::Item::Copy(i) => fill(EN.item_title_copy_seed, &[seed(i)]),
        plan::Item::Parts(i) => fill(EN.item_title_parts, &[seed(i)]),
        plan::Item::Passphrase(i) => fill(EN.item_title_passphrase, &[seed(i)]),
        plan::Item::KefCode(i) => fill(EN.item_title_kef, &[seed(i)]),
        plan::Item::Vault(v) => {
            let names: Vec<&str> = a.vault_seeds(shape, v).into_iter().map(seed).collect();
            if names.is_empty() {
                fill(EN.item_title_vault_number, &[&(v + 1).to_string()])
            } else {
                fill(
                    EN.item_title_vault,
                    &[&(v + 1).to_string(), &names.join(", ").to_string()],
                )
            }
        }
        plan::Item::SeedFiles => EN.item_title_the_seeds.to_string(),
        plan::Item::Sheets if a.split && shape.splits => EN.item_title_the_shares.to_string(),
        plan::Item::Sheets => EN.item_title_the_wallet.to_string(),
        plan::Item::PublicFiles => EN.item_title_the_public.to_string(),
        plan::Item::Sticks => app.medium.write_to().to_string(),
        plan::Item::ShowDescriptor => EN.item_title_show_descriptor.to_string(),
        plan::Item::Envelopes => EN.item_title_one_envelope.to_string(),
    }
}
