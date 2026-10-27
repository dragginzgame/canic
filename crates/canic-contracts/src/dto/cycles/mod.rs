use crate::dto::prelude::*;

pub use crate::values::cycles::{CycleTopupEventStatus, CycleTopupFailureDisposition};
pub use crate::{
    cycles::BC, cycles::Cycles, cycles::CyclesConversionError, cycles::CyclesParseError,
    cycles::KC, cycles::MC, cycles::QC, cycles::TC,
};

//
// CycleTrackerEntry
//

#[derive(Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleTrackerEntry {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
}

//
// CycleTopupEvent
//

#[derive(Deserialize, serde::Serialize)]
pub struct CycleTopupEvent {
    pub timestamp_secs: u64,
    pub sequence: u32,
    pub requested_cycles: Cycles,
    pub transferred_cycles: Option<Cycles>,
    pub status: CycleTopupEventStatus,
    pub error: Option<String>,
    pub parent_failure: Option<CycleTopupFailure>,
}

/// Exact parent funding failure; its containing event supplies the failure time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CycleTopupFailure {
    pub parent: Principal,
    pub operation_id: [u8; 32],
    pub public_error_code: u16,
    pub disposition: CycleTopupFailureDisposition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_topup_status_roundtrips_through_wire() {
        let event = CycleTopupEvent {
            timestamp_secs: 42,
            sequence: 7,
            requested_cycles: Cycles::new(1_000_000),
            transferred_cycles: Some(Cycles::new(999_000)),
            status: crate::values::cycles::CycleTopupEventStatus::RequestOk,
            error: None,
            parent_failure: None,
        };

        let bytes = crate::protocol::authority::encode(&event).expect("encode cycle top-up event");
        let decoded: CycleTopupEvent =
            crate::protocol::authority::decode(&bytes).expect("decode cycle top-up event");

        let dto_status: CycleTopupEventStatus =
            crate::values::cycles::CycleTopupEventStatus::RequestOk;

        assert_eq!(decoded.timestamp_secs, 42);
        assert_eq!(decoded.sequence, 7);
        assert_eq!(decoded.requested_cycles, Cycles::new(1_000_000));
        assert_eq!(decoded.transferred_cycles, Some(Cycles::new(999_000)));
        assert_eq!(decoded.status, dto_status);
        assert_eq!(decoded.error, None);
    }
}
