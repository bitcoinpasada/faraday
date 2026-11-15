//! Faraday's English words for the plan area.

macro_rules! words_plan {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- plan (faraday-plan/src/lib.rs) -----
    /// Text in `vault_name`. `{}`: `v + 1`, `names` joined by ", ".
    plan_vault_names: "Vault {} · {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_words: "Seed {}: words",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_seedqr: "Seed {}: SeedQR",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_passphrase_label: "Passphrase of {}",
    /// Text in `label`.
    plan_wallet_sheet: "Wallet sheet",
    /// Text in `label`. `{}`: `j + 1`, `shape.keys`.
    plan_share_count: "Share {} of {}",
    /// Text in `label`. `{}`: `medium.cap()`, `v + 1`.
    plan_with_vault: "{} with Vault {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed: "Seed {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_with_passphrase: "Seed {} with its passphrase",
    /// Text in `label`.
    plan_wallet_description: "Wallet description",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_file: "Seed {}: file",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seedqr_file: "Seed {}: SeedQR file",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_passphrase_file: "Passphrase of {}: file",
    /// Text in `label`. `{}`: the form, `form_name`.
    plan_wallet_file: "Wallet description · {}",
    /// Text in `label`. `{}`: the share, how many, the form.
    plan_share_file: "Share {} of {} · {}",
    /// Text in `label`.
    plan_software_file: "Watch-only software file",
    /// A public file's form, in `form_name`.
    plan_form_text_file: "Text file",
    /// A public file's form, in `form_name`.
    plan_form_qr_picture: "QR picture",
    /// A node's thing on the chart. `{}`: the key's number.
    glance_key_seedqr_file: "Key {} SeedQR file",
    /// A node's thing on the chart. `{}`: the key's number.
    glance_passphrase_file: "Passphrase {} file",
    /// Text in `text`.
    plan_yes_with_the_vault: "Yes, with the vault's passphrase",
    /// Text in `text`.
    plan_yes: "Yes",
    /// Text in `text`.
    plan_only_with_passphrase: "Only with the vault's passphrase",
    /// Text in `text`.
    plan_no: "No",
    /// Text in `CHECK_LINES`.
    plan_any_one_place: "Any one place lost: the rest rebuild the wallet",
    /// Text in `CHECK_LINES`.
    plan_one_place_found_can: "One place found: can spend",
    /// Text in `CHECK_LINES`.
    plan_one_place_found: "One place found: sees the balance",
    /// Text in `text`.
    plan_can_spend: "Can spend",
    /// Text in `text`.
    plan_sees_the_balance_can: "Sees the balance · can spend with the vault's passphrase",
    /// Text in `text`.
    plan_can_spend_passphrase: "Can spend with the vault's passphrase",
    /// Text in `text`.
    plan_sees_balance: "Sees the balance",
    /// Text in `text`.
    plan_sees_the_balance: "Sees the balance with the vault's passphrase",
    /// Text in `text`.
    plan_nothing: "Cannot see balance",
    /// Text in `lost_why`.
    plan_at_no_place: "at no place",
    /// Text in `lost_why`. `{}`: `v + 1`, `noun`, `stick_text(*v)`.
    plan_lost_vault_medium: "Vault {}, whose {} is {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_in: "in {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_on: "on {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_at: "at {}",
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    plan_lost_with: "with {}",
    /// Text in `lost_why`. `{}`: `seed`, `line`.
    plan_seed_is_kept_only: "Seed {} is kept only {}",
    /// Text in `lost_why`. `{}`: `seed`, `name_at(gone, name, noun)`.
    plan_lost_passphrase_only: "The passphrase of {} is kept only with {}",
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    plan_without_the_rest: "Without {}, the rest do not rebuild the wallet",
    /// Text in `name_at`. `{}`: `v + 1`.
    plan_vault: "Vault {}",
    /// Text in `name_at`. `{}`: `noun`.
    plan_the_of_files: "the {} of files",
    /// Text in `name_at`.
    plan_the_software: "the software",
    /// Text in `name_at`.
    plan_the_devices: "the devices",
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    plan_seed_is_kept_nowhere: "Seed {} is kept nowhere",
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    plan_the_passphrase: "The passphrase of {} is kept nowhere",
    /// A thing's tag on the map, as `Tag::Secret` shows.
    plan_tag_secret: "secret",
    /// A thing's tag on the map, as `Tag::Sealed` shows.
    plan_tag_sealed: "sealed",
    /// A thing's tag on the map, as `Tag::Public` shows.
    plan_tag_public: "public",
    /// A thing's mark on the chart.
    plan_mark_lost: "lost",
    /// A thing's mark on the chart.
    plan_mark_exposed: "exposed",
    /// A thing's mark on the chart: a SeedQR dropped for the words.
    plan_mark_drawn_in_part: "drawn in part",
        }
    };
}
