use crate::dto::prelude::*;

pub use crate::cdk::types::{BC, Cycles, CyclesConversionError, CyclesParseError, KC, MC, QC, TC};
pub use crate::domain::cycles::CycleTopupEventStatus;

//
// CycleTrackerEntry
//

#[derive(CandidType, Deserialize)]
pub struct CycleTrackerEntry {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_topup_status_roundtrips_through_candid() {
        let event = CycleTopupEvent {
            timestamp_secs: 42,
            sequence: 7,
            requested_cycles: Cycles::new(1_000_000),
            transferred_cycles: Some(Cycles::new(999_000)),
            status: crate::domain::cycles::CycleTopupEventStatus::RequestOk,
            error: None,
        };

        let bytes = candid::encode_one(&event).expect("encode cycle top-up event");
        let decoded: CycleTopupEvent =
            candid::decode_one(&bytes).expect("decode cycle top-up event");

        let dto_status: CycleTopupEventStatus =
            crate::domain::cycles::CycleTopupEventStatus::RequestOk;

        assert_eq!(decoded.timestamp_secs, 42);
        assert_eq!(decoded.sequence, 7);
        assert_eq!(decoded.requested_cycles, Cycles::new(1_000_000));
        assert_eq!(decoded.transferred_cycles, Some(Cycles::new(999_000)));
        assert_eq!(decoded.status, dto_status);
        assert_eq!(decoded.error, None);
    }
}
