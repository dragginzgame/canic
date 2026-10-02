//! Physical sampling uses real certified management observations; no role or account assertions.

use super::*;
use crate::{
    fleet_ensure::model::capacity_import::survey::CapacityImportSampleRecord,
    test_support::start_pocket_ic,
};
use canic_core::ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey};
use ic_agent::identity::BasicIdentity;
use ic_testkit::pocket_ic::{CanisterSettings, PocketIcBuilder};
use std::time::SystemTime;

fn authority(agent: &Agent) -> FleetReleaseAuthority {
    FleetReleaseAuthority {
        fleet: FleetBinding {
            app: AppId::from("release-observation"),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::from_der_root_trust_anchor(
                    &agent.read_root_key(),
                )
                .unwrap(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
        },
        network_root_key_sha256: Sha256::digest(agent.read_root_key()).into(),
        release_build_sha256: [2; 32],
        operation_id: [3; 32],
        operator: agent.get_principal().unwrap(),
        coordinator: Principal::from_slice(&[1]),
        cycles_ledger: Principal::from_slice(&[2]),
    }
}

fn source(
    canister_id: Principal,
    subnet: SubnetId,
    operator: Principal,
) -> CapacityImportSourceBinding {
    CapacityImportSourceBinding {
        canister_id,
        subnet,
        controllers: vec![operator],
        module_sha256: None,
        canister_version: 0,
        stopped: true,
        snapshots_size_bytes: 0,
    }
}

#[test]
fn physical_samples_reject_changed_versions_custody_and_snapshot_inventory() {
    let operator = Principal::from_slice(&[1]);
    let subnet = SubnetId::from_principal(Principal::from_slice(&[2]));
    let original = CapacityImportSampleRecord {
        binding: source(Principal::from_slice(&[3]), subnet, operator),
        cycles: 1000,
        reserved_cycles: 50,
    };
    let snapshots = vec![vec![1], vec![2]];
    let mut debited = original.clone();
    debited.cycles -= 5;
    assert_eq!(
        validate_physical_sample(
            operator, subnet, &original, &debited, &snapshots, &snapshots
        ),
        Ok(())
    );
    for mutate in [
        |sample: &mut CapacityImportSampleRecord| sample.binding.canister_version += 1,
        |sample: &mut CapacityImportSampleRecord| {
            sample.binding.controllers.push(Principal::from_slice(&[4]));
        },
        |sample: &mut CapacityImportSampleRecord| sample.binding.stopped = false,
        |sample: &mut CapacityImportSampleRecord| sample.binding.module_sha256 = Some([8; 32]),
    ] {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(matches!(
            validate_physical_sample(
                operator, subnet, &original, &changed, &snapshots, &snapshots
            ),
            Err(FleetReleaseError::Custody { .. })
        ));
    }
    for changed in [vec![vec![1]], vec![vec![1], vec![1]], vec![vec![], vec![2]]] {
        assert!(matches!(
            validate_physical_sample(operator, subnet, &original, &original, &snapshots, &changed),
            Err(FleetReleaseError::Custody { .. })
        ));
    }
    for malformed in [vec![vec![1], vec![1]], vec![vec![], vec![2]]] {
        assert!(matches!(
            validate_physical_sample(
                operator, subnet, &original, &original, &malformed, &malformed
            ),
            Err(FleetReleaseError::Custody { .. })
        ));
    }
}

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
#[expect(
    clippy::too_many_lines,
    reason = "one real management journey proves authority binding and preparation-to-observation drift refusal"
)]
fn governed_pocketic_release_physical_observation_binds_custody() {
    let mut pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    );
    pic.set_time(SystemTime::now().into());
    let identity = BasicIdentity::from_raw_key(&[17; 32]);
    let url = pic.make_live(None);
    let agent = Agent::builder()
        .with_url(url)
        .with_identity(identity)
        .with_max_response_body_size(RESPONSE_BYTES)
        .build()
        .unwrap();
    agent.set_root_key(pic.root_key().unwrap());
    let authority = authority(&agent);
    let operator = authority.operator;
    let canister = pic.create_canister_with_settings(Some(operator), None);
    pic.install_canister(
        canister,
        b"\0asm\x01\0\0\0".to_vec(),
        vec![],
        Some(operator),
    );
    pic.stop_canister(canister, Some(operator)).unwrap();
    let snapshot = pic
        .take_canister_snapshot(canister, Some(operator), None)
        .unwrap();
    let subnet = SubnetId::from_principal(pic.get_subnet(canister).unwrap());
    let binding = source(canister, subnet, operator);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let observe = || {
        runtime.block_on(async {
            prepare_with_agent(agent.clone(), &authority, &binding)
                .await
                .unwrap()
                .observe()
                .await
        })
    };
    let live = observe().unwrap();
    assert_eq!(live.snapshots, [snapshot.id]);
    assert!(live.sample.binding.stopped);
    assert_eq!(live.sample.binding.controllers, [operator]);
    assert!(live.sample.binding.module_sha256.is_some());
    assert!(live.sample.binding.snapshots_size_bytes > 0);
    assert!(live.sample.cycles > 0);
    let management = pic.canister_status(canister, Some(operator)).unwrap();
    assert_eq!(
        live.sample.reserved_cycles,
        u128::try_from(management.reserved_cycles.0).unwrap()
    );
    for change in [true, false] {
        let mut wrong = authority.clone();
        if change {
            wrong.operator = Principal::from_slice(&[9]);
        } else {
            wrong.network_root_key_sha256 = [9; 32];
        }
        assert!(matches!(
            runtime.block_on(prepare_with_agent(agent.clone(), &wrong, &binding)),
            Err(ReleaseObservationError::Authority)
        ));
    }
    let mut wrong_subnet = binding.clone();
    wrong_subnet.subnet = SubnetId::from_principal(Principal::from_slice(&[9]));
    assert!(matches!(
        runtime.block_on(prepare_with_agent(agent.clone(), &authority, &wrong_subnet)),
        Err(ReleaseObservationError::Evidence(
            FleetReleaseError::Custody { .. }
        ))
    ));
    pic.start_canister(canister, Some(operator)).unwrap();
    assert!(matches!(
        observe(),
        Err(ReleaseObservationError::Evidence(
            FleetReleaseError::Custody { .. }
        ))
    ));
    pic.stop_canister(canister, Some(operator)).unwrap();
    let prepared = runtime
        .block_on(prepare_with_agent(agent.clone(), &authority, &binding))
        .unwrap();
    pic.update_canister_settings(
        canister,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![operator, Principal::from_slice(&[9])]),
            ..CanisterSettings::default()
        },
    )
    .unwrap();
    assert!(matches!(
        runtime.block_on(prepared.observe()),
        Err(ReleaseObservationError::Management { .. })
    ));
    assert!(matches!(
        runtime.block_on(prepare_with_agent(agent.clone(), &authority, &binding)),
        Err(ReleaseObservationError::Evidence(
            FleetReleaseError::Custody { .. }
        ))
    ));
}
