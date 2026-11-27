use faraday_present::PressOf;
use faraday_templates::flow::FlowModel;

use crate::Bip85;

/// What `present` works out for the screen: the step-flow template's model (04 §5).
pub type Model<'s> = FlowModel<'s, PressOf<Bip85>>;
