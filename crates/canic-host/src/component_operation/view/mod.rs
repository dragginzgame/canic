//! Passive observations supplied by the transport to the Component workflow.

use crate::component_operation::model::{ComponentAuthorityRecord, ComponentProgressRecord};

/// Fresh authority and available Root capacity; neither authorizes mutation alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentObservation {
    pub authority: ComponentAuthorityRecord,
    pub ready_assets: u32,
}

/// Current progress for the original operation, already correlated by transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentProgressObservation {
    pub progress: Option<ComponentProgressRecord>,
}
