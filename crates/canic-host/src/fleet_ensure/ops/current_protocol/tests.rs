//! Focused proof for typed current Fleet protocol compilation.

use super::*;
use crate::fleet_ensure::{
    model::{
        DesiredCanister, DesiredCanisterKind, DesiredComponentGroupPlacement,
        DesiredFleetBootstrap, DesiredFleetBootstrapRoot, DesiredFleetProtocol,
        FLEET_ENSURE_SCHEMA_VERSION,
    },
    ops::read_plan,
    workflow,
};
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    dto::fleet_subnet_root::FleetSubnetRootAuthority,
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        FleetSubnetWasmStoreAuthority, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest,
        SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
};
use flate2::{Compression, GzBuilder};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::{fs, io::Write};

const CONFIG: &str = r#"
[app]
name = "ensure_protocol_test"

[roles.root]
kind = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 4

[component_groups.cell.components.alpha]
component_spec = "alpha"

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;

#[cfg(unix)]
struct AuthorityReadsFixture {
    root: PathBuf,
    icp: IcpCli,
    desired: DesiredFleet,
    authorities: Vec<FleetSubnetRootAuthority>,
}

#[cfg(unix)]
impl AuthorityReadsFixture {
    fn new(count: u8, overlap_gate: bool) -> Self {
        let root = crate::test_support::temp_dir("root-authority-reads");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("icp");
        fs::write(
            &executable,
            crate::test_support::tool_script(
                r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@'; exit 0;;
  *" --query "*) ;;
  *) exit 2;;
esac
while [ "$1" != canister ]; do shift; done
shift
[ "$1" = call ] || exit 3
id="$2"
method="$3"
printf '%s\n' "$id" >> calls
if [ -f latency ]; then sleep 0.02; fi
case "$method" in
  canic_root_status)
    touch "$id.started"
    if [ -f "$id.gated" ]; then
      for peer in $(cat gate); do
        attempts=0
        until [ -f "$peer.started" ]; do
          attempts=$((attempts + 1))
          [ "$attempts" -lt 1000 ] || exit 4
          sleep 0.01
        done
      done
    elif [ -f gate ]; then
      for peer in $(cat gate); do [ -f "$peer.done" ] || exit 5; done
    fi;;
  canic_wasm_store_status) touch "$(cat "$id.owner").done";;
  *) exit 6;;
esac
cat "$id.json"
"#,
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let template =
            root_authorities(&active_registry(&parse_config_model(CONFIG).unwrap())).remove(0);
        let mut desired = desired(Vec::new());
        desired.canisters.clear();
        let mut authorities = Vec::new();
        let mut gate = Vec::new();
        for index in 0..count {
            let mut authority = template.clone();
            let root_id = principal(60 + index);
            let store_id = principal(80 + index);
            authority.binding.fleet_subnet_root = root_id;
            authority.wasm_store_authority.fleet_subnet_root = root_id;
            authority.wasm_store_authority.wasm_store = store_id;
            desired.canisters.push(canister(
                &format!("root-{index}"),
                DesiredCanisterKind::Root,
                root_id,
                None,
                subnet(6),
            ));
            Self::respond(
                &root,
                root_id,
                RootStatusResponseFragment::FleetAuthority(authority.clone()),
            );
            Self::respond(
                &root,
                store_id,
                StoreStatusResponse::Authority(authority.wasm_store_authority.clone()),
            );
            fs::write(root.join(format!("{store_id}.owner")), root_id.to_text()).unwrap();
            if overlap_gate
                && usize::from(index) < super::super::bounded_observations::MAX_IN_FLIGHT
            {
                gate.push(root_id.to_text());
                fs::write(root.join(format!("{root_id}.gated")), "").unwrap();
            }
            authorities.push(authority);
        }
        if overlap_gate {
            fs::write(root.join("gate"), gate.join("\n")).unwrap();
        }
        Self {
            icp: IcpCli::new(executable.to_str().unwrap(), None).with_cwd(root.clone()),
            root,
            desired,
            authorities,
        }
    }

    fn respond(root: &Path, id: Principal, response: impl CandidType) {
        let response = Ok::<_, canic_core::dto::error::Error>(response);
        fs::write(root.join(format!("{id}.json")), serde_json::json!({
            "response_bytes": canic_core::cdk::utils::hash::hex_bytes(candid::encode_one(response).unwrap()),
        }).to_string()).unwrap();
    }

    fn read(&self) -> Result<Vec<FleetSubnetRootAuthority>, CurrentProtocolError> {
        query_current_root_authorities(
            &self.icp,
            &self.desired,
            &state(),
            &self.root.join("root.did"),
            &self.root.join("store.did"),
        )
    }
}

#[cfg(unix)]
impl Drop for AuthorityReadsFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
#[test]
fn root_authority_reads_overlap_pairs_keep_order_and_drain_before_next_batch() {
    for count in [0, 1, 4, 5] {
        let fixture = AuthorityReadsFixture::new(count, true);
        assert_eq!(fixture.read().unwrap(), fixture.authorities);
        assert_eq!(fixture.icp.remote_call_count(), 2 * u64::from(count));
    }
}

#[cfg(unix)]
#[test]
fn root_authority_reads_fail_closed_drain_and_retry_fresh() {
    let mut fixture = AuthorityReadsFixture::new(5, false);
    let first = fixture.desired.canisters[0].principal.take();
    let authority = &fixture.authorities[1];
    let mut conflicting = authority.wasm_store_authority.clone();
    conflicting.wasm_module_hash[0] ^= 1;
    AuthorityReadsFixture::respond(
        &fixture.root,
        conflicting.wasm_store,
        StoreStatusResponse::Authority(conflicting),
    );
    assert!(
        matches!(fixture.read(), Err(CurrentProtocolError::RegistryPrincipalMissing { name, .. }) if name == "root-0")
    );
    assert_eq!(fixture.icp.remote_call_count(), 6);
    assert!(
        !fixture
            .root
            .join(format!(
                "{}.started",
                fixture.authorities[4].binding.fleet_subnet_root
            ))
            .exists()
    );
    fixture.desired.canisters[0].principal = first;
    assert!(matches!(
        fixture.read(),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));
    assert_eq!(fixture.icp.remote_call_count(), 14);
    let authority = &fixture.authorities[1].wasm_store_authority;
    AuthorityReadsFixture::respond(
        &fixture.root,
        authority.wasm_store,
        StoreStatusResponse::Authority(authority.clone()),
    );
    assert_eq!(fixture.read().unwrap(), fixture.authorities);
    assert_eq!(fixture.icp.remote_call_count(), 24);
}

#[cfg(unix)]
#[test]
#[ignore = "manual matched Root/Store query scheduling measurement"]
fn root_authority_reads_matched_latency_measurement() {
    for count in [1, 4, 9] {
        let fixture = AuthorityReadsFixture::new(count, false);
        fs::write(fixture.root.join("latency"), "").unwrap();
        for round in 0..5 {
            for concurrent in [round % 2 == 0, round % 2 != 0] {
                let calls_before = fixture.icp.remote_call_count();
                let started = std::time::Instant::now();
                let observed = if concurrent {
                    fixture.read().unwrap()
                } else {
                    fixture
                        .desired
                        .canisters
                        .iter()
                        .flat_map(|root| {
                            let mut desired = fixture.desired.clone();
                            desired.canisters = vec![root.clone()];
                            query_current_root_authorities(
                                &fixture.icp,
                                &desired,
                                &state(),
                                &fixture.root.join("root.did"),
                                &fixture.root.join("store.did"),
                            )
                            .unwrap()
                        })
                        .collect()
                };
                let elapsed_us = started.elapsed().as_micros();
                let calls = fixture.icp.remote_call_count() - calls_before;
                assert_eq!(observed, fixture.authorities);
                assert_eq!(calls, 2 * u64::from(count));
                println!(
                    "root_authority_reads roots={count} round={round} concurrent={concurrent} elapsed_us={elapsed_us} calls={calls} latency_ms=20"
                );
            }
        }
    }
}

#[cfg(unix)]
struct StoreStagingFixture {
    root: PathBuf,
    icp: IcpCli,
    actions: Vec<CurrentFleetProtocolAction>,
    status: TemplateStagingStatusResponse,
}

#[cfg(unix)]
impl StoreStagingFixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let root = crate::test_support::temp_dir("store-staging-observations");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("store.did"), "service : {}").unwrap();
        let executable = root.join("icp");
        fs::write(
            &executable,
            crate::test_support::tool_script(
                r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@'; exit 0;;
  *" canic_wasm_store_catalog "*" --query "*)
    printf 'query\n' >> calls
    if [ -e fail ]; then exit 1; fi
    cat response.json;;
  *) exit 2;;
esac
"#,
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let (actions, status) = Self::subject();
        let fixture = Self {
            icp: IcpCli::new(executable.to_str().unwrap(), None).with_cwd(root.clone()),
            root,
            actions,
            status,
        };
        fixture.respond();
        fixture
    }

    fn subject() -> (
        Vec<CurrentFleetProtocolAction>,
        TemplateStagingStatusResponse,
    ) {
        let manifest = TemplateManifestInput {
            template_id: TemplateId::new("test-template"),
            version: TemplateVersion::new("test-release"),
            role: CanisterRole::new("alpha"),
            payload_hash: canic_core::cdk::utils::hash::wasm_hash(&[1, 2, 3]),
            payload_size_bytes: 3,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(0),
            created_at: 0,
        };
        let hashes = [1, 2, 3].map(|byte| canic_core::cdk::utils::hash::wasm_hash(&[byte]));
        let status = TemplateStagingStatusResponse {
            template_id: manifest.template_id.clone(),
            version: manifest.version.clone(),
            manifest: Some(manifest_response(&manifest)),
            chunk_set_present: true,
            expected_chunk_count: 3,
            expected_chunk_hashes: hashes.to_vec(),
            payload_hash: Some(manifest.payload_hash.clone()),
            payload_size_bytes: Some(3),
            stored_chunk_hashes: hashes.into_iter().map(Some).collect(),
            stored_chunk_count: 3,
            complete: true,
        };
        let mut actions = Vec::new();
        let mut preparation = Some(TemplateChunkSetPrepareInput {
            manifest: Some(manifest),
            template_id: status.template_id.clone(),
            version: status.version.clone(),
            payload_hash: status.payload_hash.clone().unwrap(),
            payload_size_bytes: 3,
            chunk_hashes: status.expected_chunk_hashes.clone(),
        });
        for (chunk_index, byte) in (0..3).zip([1, 2, 3]) {
            actions.push(CurrentFleetProtocolAction::PublishStoreChunk {
                request: TemplateChunkInput {
                    preparation: preparation.take(),
                    template_id: status.template_id.clone(),
                    version: status.version.clone(),
                    chunk_index,
                    bytes: vec![byte],
                },
            });
        }
        (actions, status)
    }

    fn respond(&self) {
        let response = Ok::<_, canic_core::dto::error::Error>(StoreCatalogResponse::Template(
            self.status.clone(),
        ));
        fs::write(self.root.join("response.json"), serde_json::json!({
            "response_bytes": canic_core::cdk::utils::hash::hex_bytes(candid::encode_one(response).unwrap()),
        }).to_string()).unwrap();
    }

    fn pending(&self) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
        let desired = desired(Vec::new());
        let steps = self
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| CompiledCurrentProtocolStep {
                action: action.clone(),
                name: format!("store-{index}"),
                target: principal(21),
            })
            .collect();
        bind_unapplied_actions(
            &self.icp,
            &self.root,
            &desired,
            &state(),
            desired.protocol.as_ref().unwrap(),
            steps,
            0,
        )
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.root.join("calls"))
            .unwrap()
            .lines()
            .count()
    }
}

#[cfg(unix)]
impl Drop for StoreStagingFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn store_staging_reads_once_per_plan_and_refreshes_changed_chunks() {
    let mut fixture = StoreStagingFixture::new();
    assert!(fixture.pending().unwrap().is_empty());
    assert_eq!(fixture.calls(), 1);
    fixture.status.stored_chunk_hashes[1] = None;
    fixture.status.stored_chunk_count = 2;
    fixture.status.complete = false;
    fixture.respond();
    let pending = fixture.pending().unwrap();
    assert_eq!(fixture.calls(), 2);
    assert_eq!(pending.len(), 1);
    assert!(
        matches!(&pending[0], EnsureAction::FleetProtocol { action, .. }
        if matches!(action.as_ref(), CurrentFleetProtocolAction::PublishStoreChunk { request }
            if request.chunk_index == 1))
    );
    fixture.status.stored_chunk_hashes[1] = Some(fixture.status.expected_chunk_hashes[1].clone());
    fixture.respond();
    // Execution-time observations must query afresh even for the same retained action.
    assert!(
        observe(&fixture.icp, &fixture.root, &pending[0])
            .unwrap()
            .applied
    );
    fixture.status.stored_chunk_hashes[1] = None;
    fixture.respond();
    assert!(
        !observe(&fixture.icp, &fixture.root, &pending[0])
            .unwrap()
            .applied
    );
    assert_eq!(fixture.calls(), 4);
}

#[cfg(unix)]
#[test]
fn combined_store_preparation_requires_exact_manifest_and_chunk_evidence() {
    let mut fixture = StoreStagingFixture::new();
    assert!(fixture.pending().unwrap().is_empty());
    let manifest = fixture.status.manifest.take().unwrap();
    fixture.respond();
    let pending = fixture.pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert!(
        matches!(&pending[0], EnsureAction::FleetProtocol { action, .. }
        if matches!(action.as_ref(), CurrentFleetProtocolAction::PublishStoreChunk { request } if request.preparation.is_some()))
    );
    fixture.status.manifest = Some(manifest.clone());
    fixture.status.manifest.as_mut().unwrap().role = CanisterRole::new("different");
    fixture.respond();
    assert_eq!(fixture.pending().unwrap().len(), 1);
    fixture.status.manifest = Some(manifest);
    fixture.status.expected_chunk_hashes[0] = vec![0; 32];
    fixture.respond();
    assert_eq!(fixture.pending().unwrap().len(), 1);
    fixture.status.template_id = TemplateId::new("other");
    fixture.respond();
    assert!(matches!(
        fixture.pending(),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
}

#[cfg(unix)]
#[test]
fn store_staging_failures_are_not_reused_and_candid_is_revalidated() {
    let fixture = StoreStagingFixture::new();
    let desired = desired(Vec::new());
    let action = bind_action(
        &fixture.root,
        &desired,
        &state(),
        desired.protocol.as_ref().unwrap(),
        fixture.actions[0].clone(),
        principal(21),
        "manifest".into(),
        0,
    )
    .unwrap();
    let mut observations = StoreStagingObservations::default();
    fs::write(fixture.root.join("fail"), []).unwrap();
    assert!(matches!(
        observe_with_staging(&fixture.icp, &fixture.root, &action, &mut observations),
        Err(CurrentProtocolError::Transport(_))
    ));
    fs::remove_file(fixture.root.join("fail")).unwrap();
    assert!(
        observe_with_staging(&fixture.icp, &fixture.root, &action, &mut observations)
            .unwrap()
            .applied
    );
    fs::write(
        fixture.root.join("store.did"),
        "service : { changed : () -> () }",
    )
    .unwrap();
    assert!(matches!(
        observe_with_staging(&fixture.icp, &fixture.root, &action, &mut observations),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
    assert_eq!(fixture.calls(), 2);
}

#[cfg(unix)]
#[test]
fn store_staging_observations_bind_each_query_identity() {
    let mut fixture = StoreStagingFixture::new();
    let mut observations = StoreStagingObservations::default();
    let template = &fixture.status.template_id;
    let version = &fixture.status.version;
    for (index, target, path, digest, template, version) in [
        (
            1,
            principal(21),
            "store.did",
            "first",
            template.clone(),
            version.clone(),
        ),
        (
            2,
            principal(11),
            "store.did",
            "first",
            template.clone(),
            version.clone(),
        ),
        (
            3,
            principal(21),
            "other.did",
            "first",
            template.clone(),
            version.clone(),
        ),
        (
            4,
            principal(21),
            "store.did",
            "changed",
            template.clone(),
            version.clone(),
        ),
        (
            5,
            principal(21),
            "store.did",
            "first",
            TemplateId::new("other"),
            version.clone(),
        ),
        (
            6,
            principal(21),
            "store.did",
            "first",
            template.clone(),
            TemplateVersion::new("other"),
        ),
    ] {
        fixture.status.template_id = template.clone();
        fixture.status.version = version.clone();
        let manifest = fixture.status.manifest.as_mut().unwrap();
        manifest.template_id = template.clone();
        manifest.version = version.clone();
        fixture.respond();
        let resolved = ResolvedProtocolAction {
            action: &fixture.actions[0],
            target,
            candid_path: fixture.root.join(path),
            candid_sha256: digest,
        };
        for _ in 0..2 {
            observations
                .query(&fixture.icp, &resolved, &template, &version)
                .unwrap();
            assert_eq!(fixture.calls(), index);
        }
    }
}

#[test]
fn activation_source_requires_exact_completed_prefix_and_issued_provisioning() {
    assert_activation_source_review(1, Some(0), false);
}

#[test]
fn activation_source_thirty_rows_without_publication_counters_reach_review() {
    // CANIC-166 reports 29 Applied rows followed by one Issued provisioning row.
    assert_activation_source_review(29, None, false);
}

#[test]
fn activation_source_completed_bootstrap_without_fixtures_remains_receipt_only() {
    assert_activation_source_review(29, None, true);
}

#[test]
fn activation_source_retains_recorded_publication_attempts_through_handoff() {
    assert_activation_source_review(1, Some(3), false);
}

#[expect(
    clippy::too_many_lines,
    reason = "the source evidence fixture binds a real typed provisioning plan to its issued prefix"
)]
fn assert_activation_source_review(
    completed_prefix: usize,
    publication_attempts: Option<u32>,
    completed_bootstrap: bool,
) {
    use crate::fleet_ensure::ops::{EnsurePaths, EnsureStateError, reinstall::source};
    use canic_core::dto::component_registry::RootComponentRegistryPreparationRequest;

    let config = parse_config_model(CONFIG).expect("config");
    let registry = active_registry(&config);
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("configuration");
    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let placements = resolve_placements(&desired, &state(), &registry).expect("placements");
    let compiled =
        compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
            .expect("provisioning");
    let root = &registry.fleet_subnet_roots[0];
    let expected = RootComponentRegistryStatusResponse {
        fleet_subnet_root: root.fleet_subnet_root,
        prepared_against_registry: compiled.request.plan.fleet_registry.clone(),
        release_set: root.active_release_set,
        component_topology_digest: root.component_topology_digest,
        next_allocation_sequence: 1,
        reserved_component_instances: 0,
        committed_component_instances: 0,
        managed_descendants: 0,
        known_created_component_canisters: 0,
        encoded_bytes: 0,
        initial_inventory: None,
    };
    let prepare = CurrentFleetProtocolAction::PrepareComponentRegistry {
        request: RootComponentRegistryPreparationRequest {
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [43; 32],
                manifest_payload_size_bytes: 1,
            },
            expected_fleet_registry: expected.prepared_against_registry.clone(),
        },
        expected,
    };
    let mut actions = std::iter::repeat_n(prepare, completed_prefix)
        .chain([
            CurrentFleetProtocolAction::ProvisionComponents {
                request: compiled.request,
                plan_hash: compiled.plan_hash,
            },
            CurrentFleetProtocolAction::ObservePoolReadiness {
                minimum_ready: 1,
                readiness_floor: Cycles::new(1),
            },
        ])
        .enumerate()
        .map(|(index, action)| EnsureAction::FleetProtocol {
            action: Box::new(action),
            candid: "root.did".to_string(),
            candid_sha256: "a".repeat(64),
            maximum_execution_burn_cycles: 1,
            name: format!("action-{index}"),
            principal: root.fleet_subnet_root.to_text(),
        })
        .collect::<Vec<_>>();
    if completed_bootstrap {
        let EnsureAction::FleetProtocol { action, .. } = &mut actions[0] else {
            unreachable!("protocol fixture");
        };
        **action = CurrentFleetProtocolAction::BootstrapStore {
            expected: canic_core::dto::root_store::RootStoreBootstrapResponse {
                fleet_subnet_root: root.fleet_subnet_root,
                wasm_store: candid::Principal::anonymous(),
                release_set: root.active_release_set,
                catalog: Vec::new(),
                fixtures: Vec::new(),
            },
            request: RootStoreBootstrapRequest {
                operation_id: [44; 32],
                manifest_payload_size_bytes: 1,
            },
        };
    }
    let digest = "a".repeat(64);
    let temp = crate::test_support::temp_dir("activation-source");
    fs::create_dir_all(&temp).expect("source workspace");
    let source_wasm = temp.join("source-root.wasm");
    fs::write(&source_wasm, b"source module").expect("source artifact");
    let mut plan = serde_json::json!({
        "schema_version": 1, "scope": "full", "fleet": "source", "environment": "local",
        "operation_id": canic_core::cdk::utils::hash::hex_bytes([42; 32]),
        "plan_sha256": digest, "canisters": [{"actions": []}],
        "conservation": { "maximum_new_funding_cycles": "0", "maximum_operator_debit_cycles": "0",
            "maximum_unavoidable_fee_cycles": "0", "scheduled_transfer_cycles": "0", "maximum_execution_burn_cycles": "0" },
        "protocol_actions": crate::fleet_ensure::json::to_value(&actions).expect("actions"),
        "reviewed_desired": { "desired": {
            "operator": desired.operator, "cycles_ledger": desired.cycles_ledger,
            "canisters": [{ "name": "source-root", "kind": "root", "principal": root.fleet_subnet_root.to_text(),
                "wasm": source_wasm, "subnet": "aaaaa-aa", "controllers": [desired.operator], "controller_canisters": [] }],
        } },
    });
    let mut journal = serde_json::json!({
        "schema_version": 1, "completion": "in_progress", "fleet": "source",
        "operation_id": plan["operation_id"], "plan_sha256": digest,
        "initial_controlled_cycles": "0", "initial_operator_cycles": "0",
        "initial_estate_funding_cycles_by_root": {}, "stalled_observations": 0,
        "successor_phases": [], "funding_observations": {}, "funding_reviews": [], "estate_funding_required": null,
        "effects": actions[..=completed_prefix].iter().enumerate().map(|(index, action)| serde_json::json!({
            "action_sha256": crate::fleet_ensure::ops::action_sha256(action),
            "state": if index < completed_prefix { "applied" } else { "issued" },
            "maintenance_attempts": 2, "created_principal": null, "destination_post_cycles": null,
            "destination_pre_cycles": null, "post_cycles": null, "pre_cycles": null,
            "pre_canister_version": null, "progress_identity": null, "receipt": null,
        })).collect::<Vec<_>>(),
    });
    if let Some(attempts) = publication_attempts {
        for effect in journal["effects"].as_array_mut().expect("effects") {
            effect["publication_attempts"] = attempts.into();
        }
    }
    if completed_bootstrap {
        // Freeze the completed source bytes before the fixture field existed.
        // Hash that independent serialized fixture, never a default-filled action.
        let current = String::from_utf8(
            crate::fleet_ensure::json::to_vec(&actions[0]).expect("bootstrap fixture"),
        )
        .expect("JSON UTF-8");
        assert_eq!(current.matches(",\"fixtures\":[]").count(), 1);
        let source = current.replace(",\"fixtures\":[]", "");
        journal["effects"][0]["action_sha256"] =
            canic_core::cdk::utils::hash::sha256_hex(source.as_bytes()).into();
        plan["protocol_actions"][0]["action"]["expected"]
            .as_object_mut()
            .expect("bootstrap receipt")
            .remove("fixtures");
    }
    let paths = EnsurePaths::under(&temp, "local", "source");
    fs::create_dir_all(paths.plan.parent().expect("parent")).expect("directory");
    fs::write(&paths.plan, serde_json::to_vec(&plan).expect("plan")).expect("write plan");
    fs::write(
        &paths.journal,
        serde_json::to_vec(&journal).expect("journal"),
    )
    .expect("write journal");
    let mut source_state =
        crate::fleet_ensure::ops::read_state(&paths, "source").expect("empty source state");
    source_state.topology.insert(
        "source-root".to_string(),
        crate::fleet_ensure::model::FleetEnsureTopologyRecord {
            kind: DesiredCanisterKind::Root,
            module_hash: Some(canic_core::cdk::utils::hash::sha256_hex(b"source module")),
            parent: None,
            protocol_binding: None,
            role: None,
        },
    );
    crate::fleet_ensure::ops::write_state(&paths, &source_state).expect("write source state");
    let evidence = source::read(&paths, "local", "source").expect("source evidence");
    assert_eq!(evidence.operation_id, plan["operation_id"]);
    assert_eq!(evidence.plan_sha256, digest);
    assert_eq!(evidence.provisioning, actions[completed_prefix]);
    let preparations = &actions[usize::from(completed_bootstrap)..completed_prefix];
    assert_eq!(evidence.registry_preparations, preparations);
    assert_eq!(
        evidence.plan_document_sha256,
        canic_core::cdk::utils::hash::sha256_hex(&fs::read(&paths.plan).expect("plan bytes"))
    );
    assert_eq!(
        evidence.journal_document_sha256,
        canic_core::cdk::utils::hash::sha256_hex(&fs::read(&paths.journal).expect("journal bytes"))
    );
    let mut requested = desired;
    crate::fleet_ensure::ops::reinstall::adoption::tests::assert_review_handoff(&paths, &evidence);
    requested.fleet = "source".to_string();
    requested.environment = "local".to_string();
    let mut platform = crate::fleet_ensure::tests::MockPlatform::new(requested.clone(), []);
    assert_unreadable_activation_review(&paths, &requested, &evidence, &mut platform);
    let error = crate::fleet_ensure::workflow::plan_reinstall(
        &temp,
        &requested,
        &"b".repeat(64),
        "source",
        2,
        &mut platform,
    )
    .expect_err("source evidence cannot authorize a reset");
    assert!(matches!(
        error,
        crate::fleet_ensure::workflow::EnsureWorkflowError::PartialActivationResetUnavailable {
            operation_id, source_document_sha256,
        } if operation_id == evidence.operation_id && source_document_sha256 == evidence.plan_document_sha256
    ));
    assert_eq!(
        fs::read(&paths.journal).expect("unchanged journal"),
        serde_json::to_vec(&journal).expect("journal")
    );
    for pointer in [
        "/effects/0/state",
        "/effects/1/state",
        "/effects/0/action_sha256",
        "/effects/1/action_sha256",
        "/operation_id",
        "/plan_sha256",
    ] {
        let mut changed = journal.clone();
        let invalid_value = match pointer {
            "/effects/1/state" if completed_prefix == 1 => "applied",
            "/effects/0/state" | "/effects/1/state" => "issued",
            _ => "changed",
        };
        *changed.pointer_mut(pointer).expect("field") = invalid_value.into();
        fs::write(
            &paths.journal,
            serde_json::to_vec(&changed).expect("changed journal"),
        )
        .expect("write changed journal");
        assert!(matches!(
            source::read(&paths, "local", "source"),
            Err(EnsureStateError::InvalidActivationSource)
        ));
    }
    assert!(matches!(
        crate::fleet_ensure::workflow::retained_in_progress_plan::<std::io::Error>(
            &temp, "local", "source",
        ),
        Err(crate::fleet_ensure::workflow::EnsureWorkflowError::RetainedPlanUnreadable { .. })
    ));
    assert_eq!(
        fs::read(&paths.plan).expect("unchanged plan"),
        serde_json::to_vec(&plan).expect("plan")
    );
    fs::remove_dir_all(temp).expect("remove fixture");
}

fn assert_unreadable_activation_review(
    paths: &crate::fleet_ensure::ops::EnsurePaths,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    evidence: &crate::fleet_ensure::model::FleetActivationSourceRecord,
    platform: &mut crate::fleet_ensure::tests::MockPlatform,
) {
    let documents = [&paths.plan, &paths.journal, &paths.state];
    let before = documents.map(|path| fs::read(path).expect("source bytes"));
    assert!(matches!(
        read_plan(paths),
        Err(crate::fleet_ensure::ops::EnsureStateError::Decode { .. })
    ));
    let failed_document = match crate::fleet_ensure::ops::read_journal(paths) {
        Ok(Some(_)) => &paths.plan,
        Err(crate::fleet_ensure::ops::EnsureStateError::Decode { path, .. }) => {
            assert_eq!(path, paths.journal);
            &paths.journal
        }
        other => panic!("unexpected retained journal admission: {other:?}"),
    };
    let failures = [
        workflow::retained_in_progress_plan::<crate::fleet_ensure::tests::MockError>(
            &paths.workspace,
            &desired.environment,
            &desired.fleet,
        )
        .expect_err("unreadable source cannot resume"),
        workflow::plan(
            &paths.workspace,
            desired,
            &"b".repeat(64),
            &desired.fleet,
            2,
            platform,
        )
        .expect_err("ordinary planning cannot supersede source"),
        workflow::apply(
            &paths.workspace,
            desired,
            &"b".repeat(64),
            &desired.fleet,
            &evidence.plan_sha256,
            platform,
        )
        .expect_err("ordinary apply cannot execute source"),
    ];
    for (error, failed_document) in
        failures
            .into_iter()
            .zip([failed_document, failed_document, &paths.plan])
    {
        assert!(matches!(
            error,
            workflow::EnsureWorkflowError::RetainedActivationReviewRequired {
                operation_id, plan_sha256, source_document_sha256, source,
            } if operation_id == evidence.operation_id
                && plan_sha256 == evidence.plan_sha256
                && source_document_sha256 == evidence.plan_document_sha256
                && matches!(source.as_ref(), crate::fleet_ensure::ops::EnsureStateError::Decode { path, .. } if path == failed_document)
        ));
    }
    assert_eq!(
        documents.map(|path| fs::read(path).expect("retained bytes")),
        before
    );
    assert_eq!(
        platform.mutation_count(&crate::fleet_ensure::ops::action_sha256(
            &evidence.provisioning
        )),
        0
    );
}

#[test]
fn fresh_fleet_registry_prepare_classifies_typed_unavailable_status() {
    let error = CanisterProtocolError::Response {
        canister: Principal::anonymous(),
        method: protocol::CANIC_ROOT_STATUS,
        source: crate::icp::IcpJsonResponseError::Rejected(
            canic_core::dto::error::Error::from_registered(
                canic_core::diagnostics::codes::STATE_UNAVAILABLE,
            ),
        ),
    };

    assert!(component_registry_status_unavailable(&error));
}

#[test]
fn typed_placements_compile_to_exact_active_root_batches() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let registry = active_registry(&config);
    let desired = desired(vec![
        placement("cells", 1, "root-two"),
        placement("cells", 0, "root-one"),
    ]);
    let state = state();

    let placements = resolve_placements(&desired, &state, &registry).expect("resolve Roots");
    let compiled =
        compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
            .expect("compile exact typed plan");
    let plan = &compiled.request.plan;

    assert_eq!(plan.batches.len(), 2);
    assert!(
        plan.batches
            .is_sorted_by_key(|batch| batch.root.fleet_subnet_root)
    );
    let assignments = plan
        .batches
        .iter()
        .map(|batch| {
            (
                batch.root.fleet_subnet_root,
                batch.placements[0].group_placement.ordinal,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(assignments, vec![(principal(10), 1), (principal(20), 0)]);
    assert_eq!(
        plan.operation,
        FleetComponentProvisioningOperation::FreshInstall
    );
}

#[test]
fn duplicate_or_incomplete_placement_authority_fails_closed() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let registry = active_registry(&config);
    let state = state();

    let duplicate = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 0, "root-two"),
    ]);
    assert!(matches!(
        resolve_placements(&duplicate, &state, &registry).and_then(|placements| {
            compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
        }),
        Err(CurrentProtocolError::InvalidPlacement(_))
    ));

    let incomplete = desired(vec![placement("cells", 0, "root-one")]);
    assert!(matches!(
        resolve_placements(&incomplete, &state, &registry).and_then(|placements| {
            compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
        }),
        Err(CurrentProtocolError::InvalidPlacement(_))
    ));
}

#[test]
fn live_root_authority_compiles_one_deterministic_registry_sequence() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let active = active_registry(&config);
    let genesis = FleetRegistryOps::compile_genesis(
        &active.authority.binding.fleet.app,
        active.authority.clone(),
        &topology,
        active.admission.clone(),
    )
    .expect("genesis Registry");
    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&active);

    let sequence =
        compile_current_registry_sequence(&desired, &state(), &topology, &genesis, &authorities)
            .expect("compile canonical Registry sequence");

    assert_eq!(sequence.current_stage, CurrentRegistryStage::Genesis);
    assert_eq!(sequence.joins.len(), 2);
    assert_eq!(
        sequence
            .joins
            .iter()
            .map(|join| join.request.entry.fleet_subnet_root)
            .collect::<Vec<_>>(),
        vec![principal(20), principal(10)]
    );
    assert_eq!(sequence.joins[0].request.expected_registry.revision, 1);
    assert_eq!(sequence.joins[1].request.expected_registry.revision, 2);
    assert_eq!(sequence.activation_request.expected_registry.revision, 3);
    assert_eq!(sequence.active_registry, active);

    let one_join = compile_current_registry_sequence(
        &desired,
        &state(),
        &topology,
        &sequence.joins[0].resulting_registry,
        &authorities,
    )
    .expect("recognize exact Joining prefix");
    assert_eq!(one_join.current_stage, CurrentRegistryStage::Joining(1));

    let terminal =
        compile_current_registry_sequence(&desired, &state(), &topology, &active, &authorities)
            .expect("recognize exact Active Registry");
    assert_eq!(terminal.current_stage, CurrentRegistryStage::Active);
}

#[test]
fn fresh_fleet_generated_store_controllers_resolve_exact_principals() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let active = active_registry(&config);
    let genesis = FleetRegistryOps::compile_genesis(
        &active.authority.binding.fleet.app,
        active.authority.clone(),
        &topology,
        active.admission.clone(),
    )
    .expect("genesis Registry");
    let mut desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    for store in desired
        .canisters
        .iter_mut()
        .filter(|canister| canister.kind == DesiredCanisterKind::Store)
    {
        store.controllers = vec![principal(50).to_string()];
        store.controller_canisters = vec![store.parent.clone().expect("Store Root")];
    }

    compile_current_registry_sequence(
        &desired,
        &state(),
        &topology,
        &genesis,
        &root_authorities(&active),
    )
    .expect("resolve generated Store controller names");

    let missing_store = desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Store)
        .expect("Store");
    let missing_store_name = missing_store.name.clone();
    missing_store.controller_canisters = vec!["missing-root".to_string()];
    assert!(matches!(
        compile_current_registry_sequence(
            &desired,
            &state(),
            &topology,
            &genesis,
            &root_authorities(&active),
        ),
        Err(CurrentProtocolError::RegistryStoreControllerPrincipalMissing {
            store,
            controller,
        }) if store == missing_store_name && controller == "missing-root"
    ));
}

#[test]
fn registry_sequence_rejects_store_and_registry_authority_drift() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let active = active_registry(&config);
    let genesis = FleetRegistryOps::compile_genesis(
        &active.authority.binding.fleet.app,
        active.authority.clone(),
        &topology,
        active.admission.clone(),
    )
    .expect("genesis Registry");
    let mut mismatched_desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&active);

    mismatched_desired
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "store-one")
        .expect("Store")
        .principal = Some(principal(99).to_string());
    assert!(matches!(
        compile_current_registry_sequence(
            &mismatched_desired,
            &state(),
            &topology,
            &genesis,
            &authorities,
        ),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));

    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let mut drifted = genesis;
    drifted.revision = 9;
    assert!(matches!(
        compile_current_registry_sequence(&desired, &state(), &topology, &drifted, &authorities,),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));
}

#[test]
fn provisioned_registry_requires_its_exact_component_operation_receipt() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("Component deployment configuration");
    let active = active_registry(&config);
    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let state = state();
    let authorities = root_authorities(&active);
    let placements = resolve_placements(&desired, &state, &active).expect("resolve placements");
    let compiled =
        compile_current_component_provisioning(&configuration, &active, [42; 32], &placements)
            .expect("compile Component operation");
    let mut published = active;
    published.revision = published.revision.checked_add(1).expect("next revision");
    let published_version = registry_version(&topology, &published).expect("published version");
    let status =
        canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse {
            operation_id: compiled.request.operation_id,
            plan_hash: compiled.plan_hash,
            fleet_registry: compiled.request.plan.fleet_registry.clone(),
            configuration_digest: compiled.request.plan.configuration_digest,
            operation: compiled.request.plan.operation.clone(),
            phase: FleetComponentProvisioningPhase::RuntimesActivated,
            directory_confirmation_root_count: 2,
            root_batch_count: 2,
            accepted_root_count: 2,
            acceptance_in_flight_root: None,
            provisioned_root_count: 2,
            current_root: None,
            provisioning_in_flight_root: None,
            directory_confirmed_root_count: 2,
            current_synchronization: None,
            current_publication: None,
            publication_in_flight_root: None,
            runtime_activated_root_count: 2,
            current_activation: None,
            activation_in_flight_root: None,
            pending_root_failure: None,
            estate_funding_required: None,
            group_placement_count: 2,
            component_count: 2,
            planned_at_ns: 1,
            roots_accepted_at_ns: Some(2),
            components_provisioned_at_ns: Some(3),
            published_fleet_registry: Some(published_version),
            service_topology_published_at_ns: Some(4),
            directories_confirmed_at_ns: Some(5),
            runtimes_activated_at_ns: Some(6),
        };

    assert_retry_timestamp_is_not_durable_progress(&status);
    assert_provisioning_progress_is_bounded(&status);

    let sequence = compile_current_registry_sequence_with_status(
        &desired,
        &state,
        &topology,
        &published,
        &authorities,
        Some(&status),
    )
    .expect("recognize exact published successor");
    assert_eq!(sequence.current_stage, CurrentRegistryStage::Provisioned);
    require_component_status_matches(&status, &compiled.request, compiled.plan_hash)
        .expect("bind exact compiled operation");

    let mut drifted_registry = published;
    drifted_registry.revision = drifted_registry
        .revision
        .checked_add(1)
        .expect("drifted revision");
    assert!(matches!(
        compile_current_registry_sequence_with_status(
            &desired,
            &state,
            &topology,
            &drifted_registry,
            &authorities,
            Some(&status),
        ),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));

    let mut drifted_status = status;
    drifted_status.plan_hash[0] ^= 1;
    assert!(matches!(
        require_component_status_matches(&drifted_status, &compiled.request, compiled.plan_hash),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));
}

fn assert_provisioning_progress_is_bounded(
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
) {
    let mut status = status.clone();
    status.phase = FleetComponentProvisioningPhase::ActivatingRuntimes;
    status.root_batch_count = 2;
    status.accepted_root_count = 2;
    status.provisioned_root_count = 1;
    status.directory_confirmed_root_count = 3;
    status.directory_confirmation_root_count = 4;
    status.runtime_activated_root_count = 0;
    status.component_count = 7;
    let observed = component_provisioning_observation(false, &status).unwrap();
    assert!(!observed.applied);
    assert_eq!(observed.retry, EffectRetry::None);
    assert_eq!(
        observed.provisioning_progress,
        Some(crate::fleet_ensure::dto::FleetProvisioningProgress {
            pending_root_failure: None,
            phase: FleetComponentProvisioningPhase::ActivatingRuntimes,
            root_batch_count: 2,
            accepted_root_count: 2,
            provisioned_root_count: 1,
            directory_confirmed_root_count: 3,
            directory_confirmation_root_count: 4,
            runtime_activated_root_count: 0,
            component_count: 7,
        })
    );
    assert_eq!(unavailable_observation().provisioning_progress, None);
}

fn assert_retry_timestamp_is_not_durable_progress(
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
) {
    let mut first_failure = status.clone();
    first_failure.phase = FleetComponentProvisioningPhase::ActivatingRuntimes;
    first_failure.pending_root_failure = Some(
        canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure {
            origin: Some(canic_core::dto::component_provisioning::ProvisioningFailureOrigin {
                failed_at_ns: 9,
                stage: canic_core::dto::component_provisioning::ProvisioningFailureStage::StoreCatalog,
                target: principal(11),
                operation_id: [9; 32],
                diagnostic_code: 61,
                retry_category: canic_core::dto::component_provisioning::ProvisioningRetryCategory::Backoff,
            }),
            fleet_subnet_root: principal(10),
            stage: canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::RuntimeActivation,
            diagnostic_code: canic_core::diagnostics::codes::STATE_CONFLICT
                .raw_code()
                .raw(),
            failed_at_ns: 10,
        },
    );
    let mut repeated_failure = first_failure.clone();
    assert_eq!(
        component_provisioning_observation(false, &first_failure)
            .unwrap()
            .provisioning_progress
            .unwrap()
            .pending_root_failure,
        first_failure.pending_root_failure,
    );
    repeated_failure
        .pending_root_failure
        .as_mut()
        .expect("failure")
        .failed_at_ns = 20;
    repeated_failure
        .pending_root_failure
        .as_mut()
        .unwrap()
        .origin
        .as_mut()
        .unwrap()
        .failed_at_ns = 19;
    assert_eq!(
        component_provisioning_observation(false, &first_failure)
            .expect("first failure observation")
            .retry,
        EffectRetry::ReplayExactIssuedCommand
    );
    assert_eq!(
        component_provisioning_observation(false, &first_failure)
            .expect("first failure identity")
            .progress_identity,
        component_provisioning_observation(false, &repeated_failure)
            .expect("repeated failure identity")
            .progress_identity,
    );

    let mut permanent = repeated_failure.clone();
    permanent.pending_root_failure.as_mut().unwrap().origin = Some(
        canic_core::dto::component_provisioning::ProvisioningFailureOrigin {
            failed_at_ns: 10,
            stage: canic_core::dto::component_provisioning::ProvisioningFailureStage::StoreStatus,
            target: principal(11),
            operation_id: [9; 32],
            diagnostic_code: 132,
            retry_category:
                canic_core::dto::component_provisioning::ProvisioningRetryCategory::ReviewRequired,
        },
    );
    assert!(
        matches!(component_provisioning_observation(false, &permanent),
        Err(CurrentProtocolError::ProvisioningReviewRequired { target, diagnostic_code: 132, failed_at_ns: 10, .. })
        if target == principal(11))
    );
    repeated_failure.pending_root_failure = None;
    assert_eq!(
        component_provisioning_observation(false, &first_failure)
            .unwrap()
            .progress_identity,
        component_provisioning_observation(false, &repeated_failure)
            .unwrap()
            .progress_identity
    );

    assert_pool_funding_pause_preserves_exact_observation(status);
}

fn assert_pool_funding_pause_preserves_exact_observation(
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
) {
    let funding = canic_core::dto::component_provisioning::RootEstateFundingRequired {
        available: Cycles::new(900),
        attempt_count: 1,
        creation_amount: Cycles::new(1_000),
        cycles_ledger: principal(90),
        execution_margin: Cycles::new(100),
        last_attempt_at_ns: Some(10),
        ledger_fee: Cycles::new(10),
        management_creation_fee: Cycles::new(500),
        operation_id: [91; 32],
        readiness_floor: Cycles::new(400),
        required: Cycles::new(1_010),
        retry_at_ns: 20,
        root: principal(10),
        shortfall: Cycles::new(110),
    };
    let mut funding_pause = status.clone();
    let mut pending = canic_core::dto::pool::CanisterPoolCreation {
        attempt_count: funding.attempt_count,
        operation_id: funding.operation_id,
        cycles_ledger: funding.cycles_ledger,
        placement_subnet: principal(11),
        root: funding.root,
        ledger_amount: funding.creation_amount.clone(),
        ledger_fee: funding.ledger_fee.clone(),
        readiness_floor: funding.readiness_floor.clone(),
        creation_execution_margin: funding.execution_margin.clone(),
        management_creation_fee: funding.management_creation_fee.clone(),
        created_at_time_ns: 1,
        last_attempt_at_ns: funding.last_attempt_at_ns,
        progress: canic_core::dto::pool::CanisterPoolCreationProgress::WaitingForFunding {
            available: funding.available.clone(),
            attempt_count: funding.attempt_count,
            last_attempt_at_ns: funding.last_attempt_at_ns,
            observed_at_ns: 11,
            required: funding.required.clone(),
            retry_at_ns: funding.retry_at_ns,
            shortfall: funding.shortfall.clone(),
        },
    };
    assert_eq!(pool_funding_required(&pending), Some(funding.clone()));
    pending.progress = canic_core::dto::pool::CanisterPoolCreationProgress::Intent {
        uncertain_result: true,
    };
    assert!(pool_funding_required(&pending).is_none());
    funding_pause.phase = FleetComponentProvisioningPhase::ProvisioningRoots;
    funding_pause.estate_funding_required = Some(funding.clone());
    let observation = component_provisioning_observation(false, &funding_pause)
        .expect("typed funding pause observation");
    assert_eq!(observation.estate_funding_required, Some(funding));
    assert_eq!(observation.retry, EffectRetry::None);
}

#[test]
fn store_authority_without_operation_receipt_does_not_complete_adoption() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let registry = active_registry(&config);
    let authority = root_authorities(&registry)
        .into_iter()
        .next()
        .expect("Root authority")
        .wasm_store_authority;
    let request = FleetSubnetWasmStoreAdoptionRequest {
        operation_id: [44; 32],
        authority: authority.clone(),
    };

    assert!(!store_adoption_applied(&request, None));
    let exact = canic_core::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse {
        operation_id: request.operation_id,
        authority: authority.clone(),
        controllers: expected_store_controllers(&authority),
        adopted_at_ns: 1,
    };
    assert!(store_adoption_applied(&request, Some(&exact)));

    let mut conflicting = exact;
    conflicting.operation_id[0] ^= 1;
    assert!(!store_adoption_applied(&request, Some(&conflicting)));
}

#[test]
fn current_desired_state_rejects_component_demand_above_pool_target() {
    let root = crate::test_support::temp_dir("current-protocol-pool-capacity");
    fs::create_dir_all(&root).expect("create test root");
    fs::write(root.join("canic.toml"), CONFIG).expect("write App config");
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let registry = active_registry(&config);
    let mut desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&registry);
    let mut roots = authorities
        .iter()
        .map(|authority| DesiredFleetBootstrapRoot {
            canister_pool_imports: Vec::new(),
            component_admissions: authority.binding.component_admissions.clone(),
            component_topology_digest: authority.binding.component_topology_digest,
            funding: authority.binding.funding.clone(),
            limits: authority.binding.limits.clone(),
            placement_subnet: authority.binding.placement_subnet,
            root: if authority.binding.fleet_subnet_root == principal(20) {
                "root-one".to_string()
            } else {
                "root-two".to_string()
            },
            store: if authority.binding.fleet_subnet_root == principal(20) {
                "store-one".to_string()
            } else {
                "store-two".to_string()
            },
        })
        .collect::<Vec<_>>();
    roots[0].limits.canister_pool.canister_cycles = Cycles::new(4_999_999_999_999);
    desired.bootstrap = Some(DesiredFleetBootstrap {
        admission_identity_origin: None,
        admission: compile_fleet_admission_policy_template(vec![principal(1)], Vec::new())
            .expect("Fleet admission template"),
        app: registry.authority.binding.fleet.app.clone(),
        canonical_network_id: registry.authority.binding.fleet.fleet.canonical_network_id,
        component_deployment_configuration: config
            .compile_component_deployment_configuration()
            .expect("Component deployment configuration"),
        coordinator: "coordinator".to_string(),
        coordinator_subnet: registry.authority.binding.coordinator_subnet,
        fleet_id: registry.authority.binding.fleet.fleet.fleet_id,
        fresh_estate: false,
        release_build_id: authorities[0].initial_release_set.release_build_id,
        root_funding: None,
        roots,
    });

    assert!(matches!(
        validate_component_pool_capacity(&root, &desired),
        Err(CurrentProtocolError::ComponentPoolCapacity(
            crate::component_topology::RootPoolCapacityError::Insufficient {
                pool_target_cycles: 4_999_999_999_999,
                required_cycles: 5_000_000_000_000,
                ..
            }
        ))
    ));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixture binds application catalog bootstrap and deterministic replay"
)]
fn fresh_fleet_store_bootstrap_is_deterministic() {
    let root = crate::test_support::temp_dir("current-store-sequence");
    let release = crate::release_build::plan_release_build(&root).expect("plan release build");
    let release_build_id = release.record.release_build_id;
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let artifact_root = root.join(".icp/local/canisters/alpha");
    fs::create_dir_all(&artifact_root).expect("create artifact root");
    let wasm_path = artifact_root.join("alpha.wasm");
    let wasm_gz_path = artifact_root.join("alpha.wasm.gz");
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d, 7];
    wasm.extend_from_slice(release_build_id.to_string().as_bytes());
    fs::write(&wasm_path, &wasm).expect("write Wasm");
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm).expect("write gzip payload");
    fs::write(&wasm_gz_path, encoder.finish().expect("finish gzip")).expect("write gzip Wasm");
    let targets = vec![crate::release_set::ApplicationArtifactBuildTarget {
        role: CanisterRole::new("alpha"),
        package: "alpha".to_string(),
        wasm_relative_path: ".icp/local/canisters/alpha/alpha.wasm".to_string(),
        wasm_gz_relative_path: ".icp/local/canisters/alpha/alpha.wasm.gz".to_string(),
    }];
    let outputs = vec![crate::release_set::ApplicationArtifactFileBuildOutput {
        role: CanisterRole::new("alpha"),
        package: "alpha".to_string(),
        release_build_id,
        wasm_path,
        wasm_gz_path,
        candid_sha256: [3; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [4; 32],
        ),
    }];
    let persisted = crate::release_set::compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist qualified union");
    persist_infrastructure_manifest(&root, release_build_id);
    let fixtures = crate::release_set::fixture::compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology,
        release_build_id,
        &[],
    )
    .expect("retain empty fixture closure");
    let infrastructure = crate::release_set::load_persisted_canic_infrastructure_artifact_manifest(
        &root,
        release_build_id,
    )
    .expect("load infrastructure receipt");
    crate::release_set::compile_and_persist_current_release_set_manifest(
        &root,
        &topology,
        release_build_id,
        &persisted,
        &infrastructure,
        &fixtures,
    )
    .expect("bind all release children");
    let registry = active_registry(&config);
    let entry = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == principal(20))
        .expect("Root entry");
    let registry_authority = registry.authority.clone();
    let binding = FleetSubnetRootBinding {
        authority: registry_authority.clone(),
        placement_subnet: entry.placement_subnet,
        fleet_subnet_root: entry.fleet_subnet_root,
        component_admissions: entry.component_admissions.clone(),
        component_topology_digest: entry.component_topology_digest,
        limits: entry.limits.clone(),
        funding: entry.funding.clone(),
    };
    let manifest = FleetSubnetRootReleaseSetManifest::project(
        &topology,
        &binding,
        &persisted.union,
        &fixtures.manifest,
    )
    .expect("project Root release set");
    let manifest_bytes =
        serde_json::to_vec(&manifest.root_store_manifest()).expect("canonical manifest");
    let authority = FleetSubnetRootAuthority {
        binding,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes(Sha256::digest(manifest_bytes).into()),
        },
        expected_module_hash: [31; 32],
        wasm_store_authority: FleetSubnetWasmStoreAuthority {
            authority: registry_authority,
            placement_subnet: entry.placement_subnet,
            fleet_subnet_root: entry.fleet_subnet_root,
            wasm_store: principal(21),
            installation_controller: principal(50),
            release_build_id,
            wasm_module_hash: [32; 32],
        },
    };
    let compiled = compile_current_store_sequence(&root, &topology, &authority, [42; 32])
        .expect("compile exact Store sequence");
    let repeated = compile_current_store_sequence(&root, &topology, &authority, [42; 32])
        .expect("repeat exact Store sequence");

    assert_eq!(compiled, repeated);
    assert!(matches!(
        compiled.actions.first(),
        Some(CurrentFleetProtocolAction::PublishStoreChunk { request }) if request.preparation.is_some()
    ));
    assert!(
        compiled
            .actions
            .iter()
            .any(|action| matches!(action, CurrentFleetProtocolAction::BootstrapStore { .. }))
    );
    assert_eq!(compiled.expected_bootstrap.catalog.len(), 1);
    let prepared_manifests = compiled
        .actions
        .iter()
        .filter_map(|action| match action {
            CurrentFleetProtocolAction::PublishStoreChunk { request } => request
                .preparation
                .as_ref()
                .and_then(|preparation| preparation.manifest.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        prepared_manifests.len(),
        compiled.expected_bootstrap.catalog.len()
    );
    assert_eq!(
        prepared_manifests[0].role,
        compiled.expected_bootstrap.catalog[0].role
    );
    assert_eq!(
        prepared_manifests[0].payload_hash,
        compiled.expected_bootstrap.catalog[0].payload_hash
    );
    assert_ne!(compiled.bootstrap_request.operation_id, [0; 32]);

    fs::remove_dir_all(root).expect("remove test root");
}

fn persist_infrastructure_manifest(root: &Path, release_build_id: ReleaseBuildId) {
    use crate::release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
        CanicInfrastructureRole,
    };

    let entries = [
        CanicInfrastructureRole::FleetCoordinator,
        CanicInfrastructureRole::FleetSubnetRoot,
        CanicInfrastructureRole::WasmStore,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, role)| {
        let index = u8::try_from(index).expect("infrastructure role index fits u8");
        let raw = [b"\0asm\x01\0\0\0".as_slice(), &[index]].concat();
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::best());
        encoder.write_all(&raw).expect("write infrastructure gzip");
        let gzip = encoder.finish().expect("finish infrastructure gzip");
        let base = format!(".icp/local/canisters/{0}/{0}", role.as_str());
        fs::create_dir_all(root.join(&base).parent().expect("artifact parent"))
            .expect("create artifact parent");
        fs::write(root.join(format!("{base}.wasm")), &raw).expect("write raw artifact");
        fs::write(root.join(format!("{base}.wasm.gz")), &gzip).expect("write gzip artifact");
        CanicInfrastructureArtifactEntry {
            role,
            package: role.as_str().to_string(),
            protocol_release_identity: "current".to_string(),
            protocol_role: CanisterRole::owned(role.protocol_role_name().to_string()),
            protocol_capabilities: BTreeSet::new(),
            release_build_id,
            wasm_relative_path: format!("{base}.wasm"),
            wasm_size_bytes: raw.len() as u64,
            wasm_sha256_hex: canic_core::cdk::utils::hash::sha256_hex(&raw),
            wasm_gz_relative_path: format!("{base}.wasm.gz"),
            wasm_gz_size_bytes: gzip.len() as u64,
            wasm_gz_sha256_hex: canic_core::cdk::utils::hash::sha256_hex(&gzip),
            candid_sha256: [3; 32],
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [4; 32],
            ),
        }
    })
    .collect();
    let manifest = CanicInfrastructureArtifactManifest {
        release_build_id,
        entries,
    };
    manifest.validate().expect("valid infrastructure manifest");
    let path = root
        .join(".canic/release-builds")
        .join(release_build_id.to_string())
        .join("infrastructure-artifact-manifest.json");
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest parent");
    fs::write(
        path,
        manifest.canonical_bytes().expect("canonical manifest"),
    )
    .expect("write infrastructure manifest");
}

fn desired(placements: Vec<DesiredComponentGroupPlacement>) -> DesiredFleet {
    DesiredFleet {
        bootstrap: None,
        canisters: vec![
            canister(
                "coordinator",
                DesiredCanisterKind::Coordinator,
                principal(30),
                None,
                subnet(1),
            ),
            canister(
                "root-one",
                DesiredCanisterKind::Root,
                principal(20),
                Some("coordinator"),
                subnet(6),
            ),
            canister(
                "store-one",
                DesiredCanisterKind::Store,
                principal(21),
                Some("root-one"),
                subnet(6),
            ),
            canister(
                "root-two",
                DesiredCanisterKind::Root,
                principal(10),
                Some("coordinator"),
                subnet(7),
            ),
            canister(
                "store-two",
                DesiredCanisterKind::Store,
                principal(11),
                Some("root-two"),
                subnet(7),
            ),
        ],
        cycles_ledger: principal(40).to_string(),
        environment: "local".to_string(),
        fleet: "protocol-test".to_string(),
        ledger_fee_cycles: "0".to_string(),
        management_creation_fee_cycles: "0".to_string(),
        material_cycle_threshold: "0".to_string(),
        maximum_observation_burn_cycles: "0".to_string(),
        maximum_stalled_observations: 2,
        maximum_update_burn_cycles: "0".to_string(),
        operator: principal(50).to_string(),
        protocol: Some(DesiredFleetProtocol {
            app_config: "canic.toml".to_string(),
            component_group_placements: placements,
            coordinator_candid: "coordinator.did".to_string(),
            root_candid: "root.did".to_string(),
            store_candid: "store.did".to_string(),
        }),
        protocol_steps: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: "coordinator".to_string(),
    }
}

fn canister(
    name: &str,
    kind: DesiredCanisterKind,
    canister_principal: Principal,
    parent: Option<&str>,
    placement_subnet: SubnetId,
) -> DesiredCanister {
    let mut controllers = vec![principal(50).to_string()];
    if kind == DesiredCanisterKind::Store {
        controllers.push(match parent {
            Some("root-one") => principal(20).to_string(),
            Some("root-two") => principal(10).to_string(),
            _ => panic!("Store test fixture requires one known Root parent"),
        });
        controllers.sort();
    }
    DesiredCanister {
        canic_init: None,
        controller_canisters: Vec::new(),
        controllers,
        drain: None,
        initial_cycles: "0".to_string(),
        init_arg: None,
        init_candid: None,
        kind,
        minimum_cycles: "0".to_string(),
        name: name.to_string(),
        parent: parent.map(str::to_string),
        presence: DesiredPresence::Present,
        principal: Some(canister_principal.to_string()),
        protocol_binding: None,
        replace: false,
        subnet: placement_subnet.to_string(),
        wasm: None,
    }
}

fn root_authorities(registry: &FleetRegistry) -> Vec<FleetSubnetRootAuthority> {
    registry
        .fleet_subnet_roots
        .iter()
        .map(|entry| FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: registry.authority.clone(),
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                component_admissions: entry.component_admissions.clone(),
                component_topology_digest: entry.component_topology_digest,
                limits: entry.limits.clone(),
                funding: entry.funding.clone(),
            },
            initial_release_set: entry.active_release_set,
            expected_module_hash: [31; 32],
            wasm_store_authority: FleetSubnetWasmStoreAuthority {
                authority: registry.authority.clone(),
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                wasm_store: if entry.fleet_subnet_root == principal(20) {
                    principal(21)
                } else {
                    principal(11)
                },
                installation_controller: principal(50),
                release_build_id: entry.active_release_set.release_build_id,
                wasm_module_hash: [32; 32],
            },
        })
        .collect()
}

fn placement(deployment: &str, ordinal: u32, root: &str) -> DesiredComponentGroupPlacement {
    DesiredComponentGroupPlacement {
        deployment: deployment.to_string(),
        ordinal,
        root: root.to_string(),
    }
}

fn state() -> FleetEnsureStateRecord {
    FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstall_action_sha256: BTreeMap::new(),
        completed_reinstall_operation_id: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: "protocol-test".to_string(),
        pending_principals: BTreeMap::new(),
        principals: BTreeMap::new(),
        retained_cycles_by_principal: BTreeMap::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        topology: BTreeMap::new(),
    }
}

fn active_registry(config: &canic_core::bootstrap::compiled::ConfigModel) -> FleetRegistry {
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("ensure_protocol_test"),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: subnet(1),
            coordinator: principal(30),
        },
        epoch: 1,
    };
    let template = compile_fleet_admission_policy_template(vec![principal(1)], Vec::new())
        .expect("Fleet admission template");
    let admission = bind_initial_fleet_admission_policy(fleet.clone(), &template)
        .expect("Fleet admission policy");
    let mut registry =
        FleetRegistryOps::compile_genesis(&fleet.app, authority.clone(), &topology, admission)
            .expect("genesis Registry");
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    for root in [principal(20), principal(10)] {
        registry = FleetRegistryOps::compile_joining(
            &authority,
            &topology,
            &registry,
            FleetSubnetRootEntry {
                placement_subnet: subnet(if root == principal(20) { 6 } else { 7 }),
                fleet_subnet_root: root,
                component_admissions: vec![admission_for(&topology)],
                component_topology_digest: topology
                    .project_for_admissions(&[admission_for(&topology)])
                    .expect("root topology")
                    .digest()
                    .expect("root topology digest"),
                active_release_set: release_set,
                limits: limits(),
                funding: crate::test_support::fleet_subnet_root_funding_authority(),
                status: FleetSubnetRootStatus::Joining,
            },
        )
        .expect("Joining root");
    }
    FleetRegistryOps::compile_active(&authority, &topology, &registry).expect("active Registry")
}

fn admission_for(
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
) -> ComponentSpecAdmission {
    let component_spec = "alpha".parse().expect("Component Spec ID");
    ComponentSpecAdmission {
        spec_hash: topology
            .get(&component_spec)
            .expect("Component Spec")
            .spec_hash,
        component_spec,
        maximum_root_instances: 2,
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 4,
        maximum_registry_bytes: 16_777_216,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 2,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 2,
            canister_cycles: Cycles::new(5_000_000_000_000),
            creation_execution_margin: Cycles::new(1_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

#[test]
fn fixture_publication_binding_reserves_reviewed_attempts_and_rejects_overflow() {
    use canic_core::dto::fixture_provisioning::{FixtureChunkUpload, FixtureSourceStatus};
    let root = crate::test_support::temp_dir("fixture-publication-budget");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("store.did"), "service : {};").unwrap();
    let mut desired = desired(Vec::new());
    let upload = CurrentFleetProtocolAction::PublishStoreFixtureChunk {
        maximum_attempts: 1,
        request: FixtureChunkUpload {
            content_id: [1; 32],
            index: 0,
            bytes: vec![2],
        },
        expected: FixtureSourceStatus {
            content_id: [1; 32],
            next_chunk: 1,
            chunk_count: 1,
            received_bytes: 1,
            complete: true,
        },
        source_bytes: 1,
    };
    let bind = |desired: &DesiredFleet, burn| {
        bind_action(
            &root,
            desired,
            &state(),
            desired.protocol.as_ref().unwrap(),
            upload.clone(),
            principal(21),
            "fixture-upload".into(),
            burn,
        )
    };
    let mut hashes = BTreeSet::new();
    for limit in [1, 3] {
        desired.maximum_stalled_observations = limit;
        let bound = bind(&desired, 7).unwrap();
        assert_eq!(bound.fixture_publication_attempt_limit(), Some(limit));
        let EnsureAction::FleetProtocol {
            maximum_execution_burn_cycles,
            ..
        } = &bound
        else {
            unreachable!()
        };
        assert_eq!(*maximum_execution_burn_cycles, u128::from(limit) * 7);
        assert!(hashes.insert(crate::fleet_ensure::ops::action_sha256(&bound)));
    }
    assert!(matches!(
        bind(&desired, u128::MAX),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
    desired.maximum_stalled_observations = 0;
    assert!(matches!(
        bind(&desired, 7),
        Err(CurrentProtocolError::ResponseMismatch)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn store_chunk_compilation_fuses_only_the_first_chunk_within_the_byte_envelope() {
    let bytes = vec![9; canic_core::CANIC_WASM_CHUNK_BYTES + 1];
    let mut actions = Vec::new();
    append_chunk_actions(
        &mut actions,
        TemplateId::new("app"),
        TemplateVersion::new("current"),
        &bytes,
        None,
    )
    .unwrap();
    assert_eq!(actions.len(), 2);
    for (index, action) in actions.iter().enumerate() {
        let CurrentFleetProtocolAction::PublishStoreChunk { request } = action else {
            panic!("chunk publication")
        };
        assert_eq!(request.chunk_index as usize, index);
        assert_eq!(request.preparation.is_some(), index == 0);
        assert!(
            candid::encode_one(request).unwrap().len()
                <= canic_core::CANIC_WASM_CHUNK_REQUEST_MAX_BYTES
        );
    }
    let mut rejected = Vec::new();
    assert!(matches!(
        append_chunk_actions(
            &mut rejected,
            TemplateId::owned("a".repeat(100_000)),
            TemplateVersion::new("current"),
            &bytes,
            None
        ),
        Err(CurrentProtocolError::Configuration(_))
    ));
    assert!(rejected.is_empty());
}
