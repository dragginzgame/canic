use crate::{cdk::types::Cycles, dto::prelude::*};

///
/// CycleTrackerEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CycleTrackerEntry {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
}
