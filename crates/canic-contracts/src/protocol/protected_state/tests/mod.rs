//! Exact state framing and maximum retained restore evidence.
use super::*;
use crate::dto::{
    authority_restore as stored,
    authority_restore_status::{Phase, Status},
    state::{FleetMode, FleetStateResponse},
};
use ic_platform::{Principal, principal::ExternalPrincipal};
use ic_wire::{Error, native};

#[test]
fn authority_fence_projection_retains_each_phase_and_original_evidence() {
    let principal = ExternalPrincipal::from_slice(&[0xff; 29]);
    let phases = [
        (stored::AuthorityRestoreFencePhase::Open, Phase::Open),
        (stored::AuthorityRestoreFencePhase::Sealed, Phase::Sealed),
        (
            stored::AuthorityRestoreFencePhase::ReleaseSealed {
                review_sha256: [0xff; 32],
                recipient: principal,
            },
            Phase::ReleaseSealed {
                review_sha256: [0xff; 32],
                recipient: Principal::from(principal),
            },
        ),
    ];
    for (phase, expected) in phases {
        let status: Status = stored::AuthorityRestoreFenceStatusResponse {
            authority_canister: principal,
            phase,
            operation_id: Some([0xff; 32]),
            history_total_num_changes: Some(u64::MAX),
            changed_at_ns: Some(u64::MAX),
        }
        .into();
        assert_eq!(status.authority_canister, principal.into());
        assert_eq!(status.phase, expected);
        assert_eq!(status.operation_id, Some([0xff; 32]));
        assert_eq!(status.history_total_num_changes, Some(u64::MAX));
        assert_eq!(status.changed_at_ns, Some(u64::MAX));
        let reply = Ok::<_, crate::dto::error::Error>(status);
        let bytes =
            native::encode_response(authority_restore::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
        let decoded: Result<Status, crate::dto::error::Error> =
            native::decode_response(authority_restore::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
        assert_eq!(decoded, reply);
    }
}

#[test]
fn fleet_state_frames_preserve_modes_and_funding() {
    for mode in [FleetMode::Enabled, FleetMode::Readonly, FleetMode::Disabled] {
        for cycles_funding_enabled in [false, true] {
            let reply = Ok::<_, crate::dto::error::Error>(FleetStateResponse {
                mode,
                cycles_funding_enabled,
            });
            let bytes =
                native::encode_response(fleet_state::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
            let decoded: Result<FleetStateResponse, crate::dto::error::Error> =
                native::decode_response(fleet_state::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
            assert_eq!(decoded, reply);
        }
    }
}

#[test]
fn protected_state_requests_reject_other_methods_and_malformed_frames() {
    for (id, other) in [
        (authority_restore::METHOD_ID, fleet_state::METHOD_ID),
        (fleet_state::METHOD_ID, authority_restore::METHOD_ID),
    ] {
        let bytes = native::encode_request(id, &(), REQUEST_LIMITS).unwrap();
        native::decode_request::<()>(id, &bytes, REQUEST_LIMITS).unwrap();
        assert!(matches!(
            native::decode_request::<()>(other, &bytes, REQUEST_LIMITS),
            Err(Error::Identity)
        ));
        assert!(matches!(
            native::decode_response::<()>(id, &bytes, RESPONSE_LIMITS),
            Err(Error::Identity)
        ));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            native::decode_request::<()>(id, &trailing, REQUEST_LIMITS),
            Err(Error::Trailing)
        ));
        assert!(matches!(
            native::encode_request(id, &vec![0u8; 128], REQUEST_LIMITS),
            Err(Error::Limit)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn native_state_schema_describes_both_reply_shapes() {
    ic_wire::typescript::native_schema::<Result<Status, crate::dto::error::Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<FleetStateResponse, crate::dto::error::Error>>()
        .unwrap();
}

#[test]
fn snapshot_mutations_keep_exact_operation_and_distinct_frame_identity() {
    let request = stored::AuthoritySnapshotRequest {
        operation_id: [255; 32],
    };
    for (id, other) in [
        (snapshot_prepare::METHOD_ID, snapshot_resume::METHOD_ID),
        (snapshot_resume::METHOD_ID, snapshot_prepare::METHOD_ID),
    ] {
        let frame = native::encode_request(id, &request, snapshot_prepare::REQUEST_LIMITS).unwrap();
        let decoded: stored::AuthoritySnapshotRequest =
            native::decode_request(id, &frame, snapshot_prepare::REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, request);
        assert!(matches!(
            native::decode_request::<stored::AuthoritySnapshotRequest>(
                other,
                &frame,
                snapshot_prepare::REQUEST_LIMITS
            ),
            Err(Error::Identity)
        ));
    }
}
