//! Installed Root dispatch over two application-owned checkpoint participants.
//! Exercises original unit custody and interruption; it grants no snapshot authority.

use super::{
    Phase, Principal, caller_authority_status, checkpoint_protocol as protocol, increment, view,
};
use crate::pic::{CanicWasmBuildProfile, root::RootBaselineSpec};
use canic::{
    Error,
    dto::application_checkpoint::{
        RootCheckpointBeginRequest, RootCheckpointRequest, RootCheckpointStatus,
    },
};
use ic_platform::pocket_ic::PocketIc;
use ic_testkit::pic::CanisterInstallExt;
use serde::Serialize;

#[test]
pub(in crate::pic) fn qualify_root_dispatch() {
    let (fixture, wasm) = install();
    let pic = &fixture.pic;
    let root = fixture.metadata.root_id;
    let mut members = fixture
        .metadata
        .snapshot_pids
        .iter()
        .copied()
        .filter(|pid| !fixture.metadata.managed_store_pids.contains(pid))
        .collect::<Vec<_>>();
    members.sort();
    assert_eq!(
        members.len(),
        2,
        "this acceptance unit has two participants"
    );
    let authority = caller_authority_status(pic, members[0], root).unwrap();
    let begin = RootCheckpointBeginRequest {
        root_install_id: authority.authority.issuer.install_id,
        operation_id: [0xc1; 32],
        expected_sequence: 0,
    };
    let controller = Principal::anonymous();
    assert_eq!(status(pic, root), None);
    assert!(
        update(
            pic,
            root,
            Principal::self_authenticating(b"outsider"),
            "canic_root_checkpoint_begin",
            protocol::BEGIN,
            &begin
        )
        .is_err()
    );
    assert_eq!(status(pic, root), None);
    let original = update(
        pic,
        root,
        controller,
        "canic_root_checkpoint_begin",
        protocol::BEGIN,
        &begin,
    )
    .unwrap();
    assert_eq!(original.participants, 2);
    assert_eq!(original.acknowledged, 0);
    assert_eq!(original.phase, Phase::Preparing);
    assert_eq!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_begin",
            protocol::BEGIN,
            &begin
        ),
        Ok(original.clone())
    );
    let selected = RootCheckpointRequest {
        root_install_id: begin.root_install_id,
        operation_id: begin.operation_id,
        sequence: original.sequence,
    };
    qualify_partial_prepare(pic, root, &members, &selected, wasm);
    qualify_release(pic, root, &members, &selected);
    let completed = status(pic, root).unwrap();
    assert_eq!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_begin",
            protocol::BEGIN,
            &begin
        ),
        Ok(completed)
    );
    assert!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_begin",
            protocol::BEGIN,
            &RootCheckpointBeginRequest {
                operation_id: [0xc2; 32],
                ..begin
            }
        )
        .is_err()
    );
}

fn qualify_partial_prepare(
    pic: &PocketIc,
    root: Principal,
    members: &[Principal],
    request: &RootCheckpointRequest,
    wasm: Vec<u8>,
) {
    let controller = Principal::anonymous();
    let partial = update(
        pic,
        root,
        controller,
        "canic_root_checkpoint_advance",
        protocol::ADVANCE,
        request,
    )
    .unwrap();
    assert_eq!(partial.acknowledged, 1);
    assert_eq!(partial.phase, Phase::Preparing);
    assert_eq!(view(pic, members[0]).prepare_runs, 1);
    assert!(increment(pic, members[0]).is_err());
    assert_eq!(increment(pic, members[1]), Ok(1));
    pic.stop_canister(members[1], Some(root)).unwrap();
    assert!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_advance",
            protocol::ADVANCE,
            request
        )
        .is_err()
    );
    assert_eq!(status(pic, root), Some(partial.clone()));

    // Root's original unit survives independently of the participant snapshots.
    pic.wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
    pic.upgrade_canister(root, wasm, crate::pic::upgrade_args(), None)
        .unwrap();
    assert_eq!(status(pic, root), Some(partial));
    pic.start_canister(members[1], Some(root)).unwrap();
    let mut stale = request.clone();
    stale.sequence += 1;
    assert!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_advance",
            protocol::ADVANCE,
            &stale
        )
        .is_err()
    );
    let mut foreign = request.clone();
    foreign.root_install_id[0] ^= 1;
    assert!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_release",
            protocol::RELEASE,
            &foreign
        )
        .is_err()
    );
    let prepared = update(
        pic,
        root,
        controller,
        "canic_root_checkpoint_advance",
        protocol::ADVANCE,
        request,
    )
    .unwrap();
    assert_eq!(prepared.phase, Phase::Prepared);
    assert_eq!(prepared.acknowledged, 2);
    for member in members {
        assert_eq!(view(pic, *member).prepare_runs, 1);
        assert!(increment(pic, *member).is_err());
    }
    assert_eq!(
        update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_advance",
            protocol::ADVANCE,
            request
        ),
        Ok(prepared)
    );
}

fn qualify_release(
    pic: &PocketIc,
    root: Principal,
    members: &[Principal],
    request: &RootCheckpointRequest,
) {
    let controller = Principal::anonymous();
    let releasing = update(
        pic,
        root,
        controller,
        "canic_root_checkpoint_release",
        protocol::RELEASE,
        request,
    )
    .unwrap();
    assert_eq!(releasing.phase, Phase::Releasing);
    assert_eq!(releasing.acknowledged, 0);
    for member in members {
        assert!(increment(pic, *member).is_err());
        let _ = update(
            pic,
            root,
            controller,
            "canic_root_checkpoint_advance",
            protocol::ADVANCE,
            request,
        )
        .unwrap();
        assert_eq!(view(pic, *member).release_runs, 1);
    }
    let completed = status(pic, root).unwrap();
    assert_eq!(completed.phase, Phase::Released);
    assert_eq!(completed.acknowledged, 2);
    let before = members
        .iter()
        .map(|member| view(pic, *member))
        .collect::<Vec<_>>();
    for method in [
        ("canic_root_checkpoint_advance", protocol::ADVANCE),
        ("canic_root_checkpoint_release", protocol::RELEASE),
    ] {
        assert_eq!(
            update(pic, root, controller, method.0, method.1, request),
            Ok(completed.clone())
        );
    }
    assert_eq!(
        members
            .iter()
            .map(|member| view(pic, *member))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(increment(pic, members[0]), Ok(1));
    assert_eq!(increment(pic, members[1]), Ok(2));
}

fn status(pic: &PocketIc, root: Principal) -> Option<RootCheckpointStatus> {
    let bytes = ic_wire::native::encode_request(protocol::STATUS, &(), protocol::LIMITS).unwrap();
    let reply = pic
        .query_call(
            root,
            Principal::anonymous(),
            "canic_root_checkpoint_status",
            bytes,
        )
        .unwrap();
    ic_wire::native::decode_response::<Result<Option<RootCheckpointStatus>, Error>>(
        protocol::STATUS,
        &reply,
        protocol::LIMITS,
    )
    .unwrap()
    .unwrap()
}

fn update<T: Serialize>(
    pic: &PocketIc,
    root: Principal,
    sender: Principal,
    method: &str,
    id: u64,
    request: &T,
) -> Result<RootCheckpointStatus, Error> {
    let bytes = ic_wire::native::encode_request(id, request, protocol::LIMITS).unwrap();
    let reply = pic.update_call(root, sender, method, bytes).unwrap();
    ic_wire::native::decode_response(id, &reply, protocol::LIMITS).unwrap()
}

fn install() -> (crate::pic::root::InitializedRootTopology, Vec<u8>) {
    let workspace = crate::workspace::discover().unwrap();
    let spec = RootBaselineSpec {
        progress_prefix: "checkpoint_root",
        build_config_path: workspace.join("canisters/test/managed_lifecycle_probe/checkpoint.toml"),
        workspace_root: workspace,
        artifact_watch_paths: &["icp.yaml", "scripts/ci"],
        release_roles: &["test"],
        build_network: canic::ids::BuildNetwork::Local,
        build_profile: CanicWasmBuildProfile::Fast,
        build_extra_env: vec![(
            "CANIC_REFERENCE_CANISTERS".to_string(),
            "root wasm_store test".to_string(),
        )],
        bootstrap_tick_limit: 120,
        root_setup_max_attempts: 1,
        pocket_ic_wasm_chunk_store_limit_bytes: 100 * 1024 * 1024,
    };
    let artifacts = crate::pic::root::ensure_root_release_artifacts_built(&spec);
    let wasm = crate::pic::root::load_root_wasm(&spec, &artifacts).unwrap();
    let mut fixture = crate::pic::root::setup_root_topology(&spec, &artifacts, wasm.clone());
    let second = crate::pic::fleet_registry::provision_component_request(
        &fixture.pic,
        fixture.metadata.root_id,
        canic::dto::component_registry::RootComponentAllocationRequest {
            operation_id: [0xc0; 32],
            component_spec: "test".parse().unwrap(),
        },
    );
    fixture
        .metadata
        .snapshot_pids
        .push(second.installation.unwrap().binding.canister_id);
    (fixture, wasm)
}
