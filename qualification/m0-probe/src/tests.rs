use crate::{FUND, FUNDING_STATUS, FundingOutcome, FundingRequest, FundingStatus};
use crate::{Fault, LIMITS, MAX_MANAGEMENT_REQUEST_BYTES, Phase, START, STATUS, Status};

#[test]
fn maximum_retained_evidence_remains_observable() {
    for phase in [Phase::Issued, Phase::Recorded] {
        let state = Status {
            phase,
            fault: Some(Fault::AfterResultWrite),
            attempts: u8::MAX,
            request: vec![u8::MAX; MAX_MANAGEMENT_REQUEST_BYTES],
            reserved_cycles: u128::MAX,
            child: Some(vec![u8::MAX; 29]),
            refunded_cycles: Some(u128::MAX),
            later_work_entered: true,
        };
        for method in [START, STATUS] {
            let frame = ic_wire::native::encode_response(method, &state, LIMITS)
                .expect("maximum evidence must fit the declared status budget");
            let decoded: Status = ic_wire::native::decode_response(method, &frame, LIMITS)
                .expect("maximum evidence must remain decodable");
            assert_eq!(decoded, state);
        }
    }
}

#[test]
fn maximum_funding_evidence_remains_observable() {
    for outcome in [
        None,
        Some(FundingOutcome::Deposited),
        Some(FundingOutcome::Rejected { code: u32::MAX }),
    ] {
        let state = FundingStatus {
            phase: Phase::Recorded,
            intent: Some(FundingRequest {
                target: vec![u8::MAX; 29],
                cycles: u128::MAX,
                fault: Fault::LaterWork,
            }),
            attempts: u8::MAX,
            request: vec![u8::MAX; MAX_MANAGEMENT_REQUEST_BYTES],
            reserved_cycles: u128::MAX,
            outcome,
            refunded_cycles: Some(u128::MAX),
            later_work_entered: true,
        };
        for method in [FUND, FUNDING_STATUS] {
            let frame = ic_wire::native::encode_response(method, &state, LIMITS)
                .expect("maximum funding evidence must fit the declared budget");
            let decoded: FundingStatus = ic_wire::native::decode_response(method, &frame, LIMITS)
                .expect("maximum funding evidence must remain decodable");
            assert_eq!(decoded, state);
        }
    }
}
