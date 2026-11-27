use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::content::Content;
use faraday_present::{ChoiceLine, FlowPress, ShowFlow};

use crate::{Answer, Press, Refusal, Spend, Step};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

impl ShowFlow for Spend {
    type Press = Press;

    fn title(&self, _s: Step, _show: &Show<'_>) -> Line {
        todo!()
    }

    fn choice(&self, _a: &Answer, _show: &Show<'_>) -> ChoiceLine {
        todo!()
    }

    fn card<'s>(
        &'s self,
        _view: &View<'s>,
        _show: &Show<'s>,
    ) -> Content<'s, FlowPress<Step, Press>> {
        todo!()
    }
}
