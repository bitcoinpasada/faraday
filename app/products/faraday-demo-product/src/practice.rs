//! What Practice takes, from the published samples (03 §11, A33; `main`'s `Samples::handouts`,
//! `public` and `nodes`). Faraday Demo builds it once and gives it in the `Mode` of a Practice
//! build; the session takes only these keys, and New key hands out these handouts.

use std::sync::OnceLock;

use faraday_kernel::{Handout, PracticeSamples};
use faraday_samples::SAMPLES;

/// The samples Practice takes, worked out once and kept for the process.
pub fn practice_samples() -> &'static PracticeSamples {
    static SET: OnceLock<PracticeSamples> = OnceLock::new();
    SET.get_or_init(|| {
        let handouts: Vec<Handout> = SAMPLES
            .keys
            .iter()
            .filter(|k| !k.word.is_empty())
            .filter_map(|k| {
                Some(Handout {
                    name: k.word,
                    words: k.count,
                    plain: k.plain?,
                    with_passphrase: k.with_passphrase?,
                })
            })
            .collect();
        let public: Vec<[u8; 4]> = SAMPLES
            .keys
            .iter()
            .flat_map(|k| [k.plain, k.with_passphrase])
            .flatten()
            .collect();
        let nodes: Vec<[u8; 33]> = SAMPLES
            .keys
            .iter()
            .filter_map(|k| k.node)
            .chain(SAMPLES.inputs.hsm.iter().map(|h| h.node))
            .chain(SAMPLES.inputs.hsm_words_nodes.iter().copied())
            .collect();
        PracticeSamples {
            handouts: handouts.leak(),
            public: public.leak(),
            nodes: nodes.leak(),
        }
    })
}

#[cfg(test)]
mod tests {
    use faraday_app_test::TestView;
    use faraday_kernel::{Carried, Feature, session::Read};
    use faraday_samples::SAMPLES;
    use faraday_session::Session;

    use super::practice_samples;

    /// Invariant: In Practice the session takes the Testnet Lightning sample node and New key has a sample to hand out.
    /// Regression: Practice is given a Mainnet node list, so LND's published cipher seed is refused, or no handouts, so New key cannot add its key.
    /// Owner: app
    #[test]
    fn practice_takes_the_testnet_lnd_sample_and_hands_out_a_sample() {
        let view = TestView::demo().with_samples(practice_samples());
        let (state, _) = Session::start(&mut Carried::empty(), &view.ctx());
        let lnd = SAMPLES.keys.iter().find_map(|k| k.node);
        assert!(lnd.is_some_and(|node| state.keys().is_public_node(&node)));
        assert!(state.keys().next_handout(12).is_some());
    }
}
