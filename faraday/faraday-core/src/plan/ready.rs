//! The ready plans the backup board offers first (`docs/BACKUP-FLOW.md`
//! §17.25): each a whole placement a person takes as it stands or then
//! customises.

use faraday_plan::{
    Answers, At, Preset, Shape, What, stored_map, wallet, with_added, with_vault_stick,
};

/// One ready plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ready {
    /// Each seed's words on paper, nothing else: The standard way.
    PaperOnly,
    /// Paper, and the seeds and the description in a vault on a stick.
    PaperAndVault,
    /// Paper, and the key and the description in two vaults, each on a
    /// stick at a place of its own.
    TwoVaults,
}

impl Ready {
    /// Every ready plan, the plainest first.
    pub const ALL: [Ready; 3] = [Ready::PaperOnly, Ready::PaperAndVault, Ready::TwoVaults];

    /// The plans offered for `shape`: with a vault already set up before
    /// the keys the plan already has it, so none is offered; a vault plan
    /// needs a seed loaded here to hold, and two vaults are for one key
    /// (each holds a copy of it).
    pub fn offered(shape: &Shape) -> Vec<Ready> {
        if shape.vaults > 0 {
            return Vec::new();
        }
        Ready::ALL
            .into_iter()
            .filter(|r| match r {
                Ready::PaperOnly => true,
                Ready::PaperAndVault => !shape.nothing_to_vault(),
                Ready::TwoVaults => shape.vaultable().len() == 1 && !shape.multi(),
            })
            .collect()
    }

    /// The button's label.
    pub fn label(self) -> &'static str {
        match self {
            Ready::PaperOnly => faraday_strings::EN.plan_ready_paper,
            Ready::PaperAndVault => faraday_strings::EN.plan_ready_vault,
            Ready::TwoVaults => faraday_strings::EN.plan_ready_two_vaults,
        }
    }

    /// The answers of this plan for `shape`, read-only as a new plan is.
    pub fn answers(self, shape: &Shape) -> Answers {
        match self {
            Ready::PaperOnly => {
                let mut a = Answers::preset(shape, Preset::Standard);
                // The seed alone gives the wallet back: the words alone
                // at the place, no sheet (`docs/BACKUP-FLOW.md` §19.6).
                if shape.words_alone() {
                    a.wallet[wallet::PAPER] = false;
                }
                a
            }
            Ready::PaperAndVault => Answers::preset(shape, Preset::PaperVault),
            Ready::TwoVaults => {
                let mut a = Answers::preset(shape, Preset::PaperVault);
                a.add_place(shape);
                let seeds = shape.vaultable();
                let mut m = stored_map(shape, &a);
                for i in seeds {
                    m = with_added(&m, At::Vault(1), What::Seed(i));
                }
                m = with_added(&m, At::Vault(1), What::Wallet);
                let m = with_vault_stick(&m, 1, 1);
                a.set_map(m);
                a
            }
        }
    }

    /// Whether `a` is this plan as it stands.
    pub fn is(self, shape: &Shape, a: &Answers) -> bool {
        stored_map(shape, &self.answers(shape)) == stored_map(shape, a)
    }
}
