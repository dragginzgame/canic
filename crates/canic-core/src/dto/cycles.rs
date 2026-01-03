use crate::{cdk::types::Cycles, dto::prelude::*};

///
/// CycleTrackerEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CycleTrackerEntryView {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
}
