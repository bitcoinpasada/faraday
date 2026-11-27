use faraday_kernel::{Line, Refused, Show, Worded};

use crate::Refusal;

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}
