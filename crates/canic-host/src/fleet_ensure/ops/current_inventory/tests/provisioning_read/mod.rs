//! Exercise the actual Host query and complete native evidence without replacing domain validation.
use super::*;
use canic_contracts::{
    dto::{
        component_provisioning::{
            RootComponentProvisioningFailure, RootEstateFundingRequired,
            RootProvisionedGroupPlacement,
        },
        component_read::Provisioning,
    },
    protocol::component_read as wire,
    values::provisioning_failure::{
        ProvisioningDependencyFailure, ProvisioningFailureStage, ProvisioningRetryCategory,
    },
};
use std::os::unix::fs::PermissionsExt as _;

fn evidence() -> RootComponentProvisioningStatusResponse {
    let (_, mut status) = empty_terminal_root_status();
    let (partition, member, _, _) = component_partition_fixture();
    status
        .result
        .as_mut()
        .unwrap()
        .placements
        .push(RootProvisionedGroupPlacement {
            group_placement: (*partition.group_placement).clone(),
            component_group: "group".parse().unwrap(),
            members: vec![member],
        });
    status.last_failure = Some(RootComponentProvisioningFailure {
        dependency: Some(ProvisioningDependencyFailure {
            target: status.fleet_subnet_root,
            method: "read".into(),
            step: "observe".into(),
            rejection_code: Some(5),
            rejection_detail: Some("retained rejection".into()),
        }),
        stage: ProvisioningFailureStage::ComponentRuntime,
        target: status.fleet_subnet_root,
        operation_id: status.operation_id,
        diagnostic_code: 7,
        retry_category: ProvisioningRetryCategory::Backoff,
        failed_at_ns: 8,
        consecutive_failures: 9,
        retry_at_ns: Some(10),
    });
    status.estate_funding_required = Some(RootEstateFundingRequired {
        available: u128::MAX.into(),
        attempt_count: 2,
        creation_amount: 3.into(),
        cycles_ledger: Principal::from_slice(&[4]),
        execution_margin: 5.into(),
        last_attempt_at_ns: Some(6),
        ledger_fee: 7.into(),
        management_creation_fee: 8.into(),
        operation_id: status.operation_id,
        readiness_floor: 9.into(),
        required: 10.into(),
        retry_at_ns: 11,
        root: status.fleet_subnet_root,
        shortfall: 12.into(),
    });
    status
}

#[test]
fn native_provisioning_query_preserves_evidence_and_rejects_wrong_method() {
    let directory = crate::test_support::temp_dir("native-provisioning-read");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    std::fs::write(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
set -eu
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
while [ "$1" != --args-file ]; do shift; done
shift
cp "$1" request.bin
cat response.hex
"#,
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&directory);
    let expected = evidence();
    let response: Result<Provisioning, canic_contracts::dto::error::Error> =
        Ok(expected.clone().into());
    let bytes = ic_wire::native::encode_response(
        wire::provisioning::METHOD_ID,
        &response,
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    std::fs::write(directory.join("response.hex"), hex_bytes(bytes)).unwrap();
    assert_eq!(
        query_root_component_status(&icp, expected.fleet_subnet_root, expected.operation_id)
            .unwrap(),
        expected
    );
    let request = std::fs::read(directory.join("request.bin")).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            wire::provisioning::METHOD_ID,
            &request,
            wire::REQUEST_LIMITS
        )
        .unwrap(),
        expected.operation_id
    );
    let wrong = ic_wire::native::encode_response(
        wire::child_allocation::METHOD_ID,
        &response,
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    std::fs::write(directory.join("response.hex"), hex_bytes(wrong)).unwrap();
    assert!(matches!(
        query_root_component_status(&icp, expected.fleet_subnet_root, expected.operation_id),
        Err(CurrentProtocolError::TerminalInventoryObservation {
            stage: "root_component_provisioning",
            ..
        })
    ));
    std::fs::remove_dir_all(directory).unwrap();
}
