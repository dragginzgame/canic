use crate::{cdk::types::Cycles, dto::prelude::*};

//
// CycleTrackerEntry
//

#[derive(CandidType, Deserialize)]
pub struct CycleTrackerEntry {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
}

//
// CycleTopupEventStatus
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum CycleTopupEventStatus {
    RequestErr,
    RequestOk,
    RequestScheduled,
}

//
// CycleTopupEvent
//

#[derive(CandidType, Deserialize)]
pub struct CycleTopupEvent {
    pub timestamp_secs: u64,
    pub sequence: u32,
    pub requested_cycles: Cycles,
    pub transferred_cycles: Option<Cycles>,
    pub status: CycleTopupEventStatus,
    pub error: Option<String>,
}
