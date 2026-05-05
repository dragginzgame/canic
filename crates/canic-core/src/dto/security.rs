use crate::dto::prelude::*;

//
// SecurityEvent
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecurityEvent {
    pub id: u64,
    pub created_at: u64,
    pub caller: Principal,
    pub endpoint: String,
    pub request_bytes: u64,
    pub max_bytes: u64,
    pub reason: SecurityEventReason,
}

//
// SecurityEventReason
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SecurityEventReason {
    IngressPayloadLimitExceeded,
}
