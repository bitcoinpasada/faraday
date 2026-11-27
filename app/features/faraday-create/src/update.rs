use faraday_kernel::{
    Back, CancelOffer, Carried, Ctx, Ending, FlowFx, Notice, Public, StepFlow, Steps, View, Width,
};

use crate::{MakeWallet, Out, Own};

/// The flow's steps, in order.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Step {
    Kind,
    Quorum,
    BackupForm,
    Keys,
    Check,
}

/// Why a step or a message is refused (03 §9.3).
#[derive(Public)]
#[expect(
    clippy::large_enum_variant,
    reason = "the enum carries the domain's real types, which differ in size; a value is moved once"
)]
pub enum Refusal {
    NoKind,
    BackupFormUnanswered,
    SlotEmpty(u8),
    SlotLater(u8),
    VaultStickPending,
    Cosigner(CosignerRefusal),
    Session(faraday_kernel::session::AddRefusal),
    Practice,
}

/// Why a cosigner's key is refused (03 §9.3).
#[derive(Public)]
pub enum CosignerRefusal {
    OriginMissing,
    OtherPath,
    OtherScript,
    NotOneKey,
    SameKeyTwice,
}

/// The value an answered step shows in Your choices.
#[derive(Public)]
pub struct Answer;

impl StepFlow for MakeWallet {
    type Step = Step;
    type Own = Own;
    type Out = Out;
    type Refusal = Refusal;
    type Answer = Answer;

    fn steps(&self) -> Steps<Step> {
        todo!()
    }

    fn at(&self) -> Step {
        todo!()
    }

    fn answer(&self, _s: Step) -> Option<Answer> {
        todo!()
    }

    fn back_stop(&self) -> Option<Step> {
        todo!()
    }

    fn running(&self) -> bool {
        todo!()
    }

    fn begun(&self) -> bool {
        todo!()
    }

    fn cancel(&self) -> CancelOffer {
        todo!()
    }

    fn width(&self, _s: Step) -> Width {
        todo!()
    }

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (MakeWallet, FlowFx<MakeWallet>) {
        (MakeWallet::new(), FlowFx::<MakeWallet>::none())
    }

    fn forward(&mut self, _cx: &Ctx<'_>) -> Result<FlowFx<MakeWallet>, Refusal> {
        todo!()
    }

    fn back(&mut self) -> Back {
        todo!()
    }

    fn open(&mut self, _s: Step) -> bool {
        todo!()
    }

    fn own(&mut self, _m: Own, _cx: &Ctx<'_>) -> FlowFx<MakeWallet> {
        todo!()
    }

    fn notice(&mut self, _n: &Notice<'_>, _cx: &Ctx<'_>) -> FlowFx<MakeWallet> {
        todo!()
    }

    fn refuses(&self, _m: &Own, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }

    fn cancelled(&mut self) -> Ending {
        todo!()
    }
}
