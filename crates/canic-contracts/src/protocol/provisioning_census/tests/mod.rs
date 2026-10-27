//! Native discovery keeps failure provenance, owner identity and original deadlines intact.
use super::*;
use crate::{
    dto::{
        error::Error,
        provisioning_census::{DependencyFailure, Entry, Failure, Page},
        root::{
            RootProvisioningReleaseKey as Key, RootProvisioningReleasePhase as Phase,
            RootProvisioningReleaseResponse,
        },
    },
    values::provisioning_failure::{ProvisioningFailureStage, ProvisioningRetryCategory},
};

#[test]
fn census_preserves_both_owners_and_complete_protected_failure() {
    let principal = ic_platform::Principal::from_slice(&[7]);
    for (key, phase) in [
        (Key::Provisioning([1; 32]), Phase::Publishing),
        (
            Key::DirectorySynchronization([1; 32]),
            Phase::DirectorySynchronizing,
        ),
    ] {
        let page = Page {
            root: principal,
            active_provisioning: Some([1; 32]),
            active_directory_synchronization: Some([1; 32]),
            next_after: Some(key),
            entry: Some(Entry {
                key,
                plan_hash: [2; 32],
                phase,
                delivery_in_flight: Some(principal),
                last_failure: Some(Failure {
                    dependency: Some(DependencyFailure {
                        target: principal,
                        method: "remote_method".into(),
                        step: "delivery".into(),
                        rejection_code: Some(5),
                        rejection_detail: Some("protected rejection".into()),
                    }),
                    stage: ProvisioningFailureStage::ComponentRuntime,
                    target: principal,
                    operation_id: [1; 32],
                    diagnostic_code: 81,
                    retry_category: ProvisioningRetryCategory::Backoff,
                    failed_at_ns: 12,
                    consecutive_failures: 3,
                    retry_at_ns: Some(24),
                }),
            }),
        };
        let canonical = RootProvisioningReleaseResponse::from(page.clone());
        assert_eq!(Page::from(canonical), page);
        let bytes = ic_wire::native::encode_response(
            METHOD_ID,
            &Ok::<_, Error>(page.clone()),
            RESPONSE_LIMITS,
        )
        .unwrap();
        let observed: Result<Page, Error> =
            ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
        assert_eq!(observed.unwrap(), page);
        let bytes = ic_wire::native::encode_request(METHOD_ID, &Some(key), REQUEST_LIMITS).unwrap();
        let cursor: Option<Key> =
            ic_wire::native::decode_request(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(cursor, Some(key));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn census_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<Option<Key>>().unwrap();
    ic_wire::typescript::native_schema::<Result<Page, Error>>().unwrap();
}
