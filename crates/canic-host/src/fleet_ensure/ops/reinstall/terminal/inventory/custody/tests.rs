//! Certificate projection and governed HTTP custody checks. No legacy runtime is executed.

use super::*;
use crate::fleet_ensure::{
    CompletedCanisterInventoryView, CompletedReceiptAuditView, model::FleetTerminalSourceRecord,
};
use canic_core::{
    cdk::utils::hash::sha256_hex,
    ids::{AppId, FleetBinding, FleetId, FleetKey, ReleaseBuildId, ReleaseBuildNonce},
};
use ic_agent::{Identity, identity::BasicIdentity};
use ic_certification::{HashTree, fork, labeled, leaf, pruned};

fn certificate(
    principal: Principal,
    controllers: &[Principal],
    module: Option<HashTree>,
) -> Certificate {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(controllers, &mut bytes).unwrap();
    let controllers = labeled(b"controllers", leaf(bytes));
    let fields = module.map_or_else(
        || controllers.clone(),
        |module| fork(controllers.clone(), module),
    );
    Certificate {
        tree: labeled(b"canister", labeled(principal.as_slice(), fields)),
        signature: Vec::new(),
        delegation: None,
    }
}

#[test]
fn certified_absence_is_distinct_from_unknown_or_malformed_module() {
    let principal = Principal::from_slice(&[1]);
    let owner = Principal::from_slice(&[2]);
    let empty = certificate(principal, &[owner], None);
    assert_eq!(
        project(&empty, b"test root", principal)
            .unwrap()
            .module_sha256(),
        None
    );
    let installed = certificate(
        principal,
        &[owner],
        Some(labeled(b"module_hash", leaf([3; 32]))),
    );
    assert_eq!(
        project(&installed, b"test root", principal)
            .unwrap()
            .module_sha256(),
        Some(hex_bytes([3; 32]).as_str())
    );
    for module in [pruned([0; 32]), labeled(b"module_hash", leaf([3; 31]))] {
        assert!(matches!(
            project(
                &certificate(principal, &[owner], Some(module)),
                b"test root",
                principal
            ),
            Err(CompletedCustodyError::IncompleteCertificate { .. })
        ));
    }
}

#[test]
fn controller_evidence_rejects_duplicates_empty_values_and_wrong_target() {
    let principal = Principal::from_slice(&[1]);
    let owner = Principal::from_slice(&[2]);
    for owners in [Vec::new(), vec![owner, owner]] {
        assert!(matches!(
            project(
                &certificate(principal, &owners, None),
                b"test root",
                principal
            ),
            Err(CompletedCustodyError::IncompleteCertificate { .. })
        ));
    }
    assert!(matches!(
        project(&certificate(principal, &[owner], None), b"test root", owner),
        Err(CompletedCustodyError::IncompleteCertificate { .. })
    ));
    let additional = Principal::from_slice(&[3]);
    let observed = project(
        &certificate(principal, &[additional, owner], None),
        b"test root",
        principal,
    )
    .unwrap();
    assert_eq!(observed.controllers(), &[owner, additional]);
}

struct FixtureCanister {
    name: &'static str,
    principal: Principal,
    subnet: SubnetId,
    kind: DesiredCanisterKind,
    root: Option<&'static str>,
    parent: Option<&'static str>,
    module_sha256: Option<String>,
}

fn canister(
    name: &'static str,
    principal: Principal,
    subnet: SubnetId,
    kind: DesiredCanisterKind,
    root: Option<&'static str>,
    parent: Option<&'static str>,
    module_sha256: Option<String>,
) -> FixtureCanister {
    FixtureCanister {
        name,
        principal,
        subnet,
        kind,
        root,
        parent,
        module_sha256,
    }
}

fn source(
    network: CanonicalNetworkId,
    operator: Principal,
    entries: Vec<FixtureCanister>,
) -> CompletedEstateInventoryView {
    let canisters = entries
        .into_iter()
        .map(
            |FixtureCanister {
                 name,
                 principal,
                 subnet,
                 kind,
                 root,
                 parent,
                 module_sha256,
             }| {
                (
                    name.into(),
                    CompletedCanisterInventoryView {
                        principal,
                        subnet,
                        kind,
                        root: root.map(str::to_string),
                        parent: parent.map(str::to_string),
                        module_sha256,
                        protocol_binding: None,
                        originally_declared_controllers: Vec::new(),
                        recorded_cycles: 0,
                    },
                )
            },
        )
        .collect::<BTreeMap<String, _>>();
    CompletedEstateInventoryView {
        receipts: CompletedReceiptAuditView {
            documents: FleetTerminalSourceRecord {
                operation_id: "11".repeat(32),
                plan_sha256: "22".repeat(32),
                plan_document_sha256: "33".repeat(32),
                journal_document_sha256: "44".repeat(32),
                state_document_sha256: "55".repeat(32),
                phase_document_sha256: BTreeMap::new(),
            },
            effect_count: 1,
            phase_count: 0,
            source_operator: operator.to_text(),
            cycles_ledger: Principal::from_slice(&[8]).to_text(),
            initial_controlled_cycles: 0,
            initial_operator_cycles: 0,
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            recorded_funding_cycles: 0,
            recorded_operator_debit_cycles: 0,
            original_maximum_execution_burn_cycles: 0,
        },
        coordinator_registry: crate::fleet_ensure::CompletedCoordinatorMembershipView {
            fleet: FleetBinding {
                app: AppId::from("custody_test"),
                fleet: FleetKey {
                    canonical_network_id: network,
                    fleet_id: FleetId::from_generated_bytes([6; 32]),
                },
            },
            coordinator: canisters["coordinator"].principal,
            coordinator_subnet: canisters["coordinator"].subnet,
            epoch: 1,
            revision: 3,
            roots: Vec::new(),
        },
        fleet: FleetBinding {
            app: AppId::from("custody_test"),
            fleet: FleetKey {
                canonical_network_id: network,
                fleet_id: FleetId::from_generated_bytes([6; 32]),
            },
        },
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32])),
        coordinator: canisters["coordinator"].principal,
        canisters,
        recorded_controlled_canister_cycles: 0,
    }
}

#[test]
fn membership_requires_unique_ids_and_same_subnet_root_custody() {
    let operator = Principal::from_slice(&[1]);
    let coordinator = Principal::from_slice(&[2]);
    let root = Principal::from_slice(&[3]);
    let child = Principal::from_slice(&[4]);
    let subnet = SubnetId::from_principal(Principal::from_slice(&[5]));
    let mut source = source(
        CanonicalNetworkId::ic_mainnet(),
        operator,
        vec![
            canister(
                "coordinator",
                coordinator,
                subnet,
                DesiredCanisterKind::Coordinator,
                None,
                None,
                None,
            ),
            canister(
                "root",
                root,
                subnet,
                DesiredCanisterKind::Root,
                None,
                Some("coordinator"),
                None,
            ),
            canister(
                "child",
                child,
                subnet,
                DesiredCanisterKind::Component,
                Some("root"),
                Some("another_parent"),
                None,
            ),
        ],
    );
    assert_eq!(owners(&source, operator).unwrap()["child"], root);
    source.canisters.get_mut("child").unwrap().subnet = SubnetId::from_principal(child);
    assert!(matches!(
        owners(&source, operator),
        Err(CompletedCustodyError::Membership)
    ));
    source.canisters.get_mut("child").unwrap().subnet = subnet;
    source.canisters.get_mut("child").unwrap().principal = root;
    assert!(matches!(
        owners(&source, operator),
        Err(CompletedCustodyError::Membership)
    ));
}

#[test]
#[ignore = "governed PocketIC proof uses certified HTTP state on two real subnets"]
#[expect(
    clippy::too_many_lines,
    reason = "one certified-read journey covers code, controllers, placement, signer and network drift without updates"
)]
fn governed_pocketic_completed_estate_certified_custody() {
    use ic_testkit::pic::PocketIcBuilder;
    let mut pic = crate::test_support::start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet()
            .with_application_subnet(),
    );
    let identity = BasicIdentity::from_raw_key(&[42; 32]);
    let operator = identity.sender().unwrap();
    let subnets = pic.topology().get_app_subnets();
    let coordinator = pic.create_canister_on_subnet(Some(operator), None, subnets[0]);
    let root = pic.create_canister_on_subnet(Some(operator), None, subnets[1]);
    let child = pic.create_canister_on_subnet(Some(operator), None, subnets[1]);
    let idle = pic.create_canister_on_subnet(Some(operator), None, subnets[1]);
    let extra = Principal::self_authenticating(b"observed additional controller");
    let wasm = b"\0asm\x01\0\0\0";
    for principal in [coordinator, root, child, idle] {
        pic.add_cycles(principal, 2_000_000_000_000);
        if principal != idle {
            pic.install_canister(principal, wasm.to_vec(), Vec::new(), Some(operator));
        }
    }
    pic.set_controllers(child, Some(operator), vec![root, extra])
        .unwrap();
    pic.set_controllers(idle, Some(operator), vec![root])
        .unwrap();
    let key = pic.root_key().unwrap();
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&key).unwrap();
    let subnet = SubnetId::from_principal(subnets[1]);
    let mut source = source(
        network,
        operator,
        vec![
            canister(
                "coordinator",
                coordinator,
                SubnetId::from_principal(subnets[0]),
                DesiredCanisterKind::Coordinator,
                None,
                None,
                Some(sha256_hex(wasm)),
            ),
            canister(
                "root",
                root,
                subnet,
                DesiredCanisterKind::Root,
                None,
                Some("coordinator"),
                Some(sha256_hex(wasm)),
            ),
            canister(
                "child",
                child,
                subnet,
                DesiredCanisterKind::Component,
                Some("root"),
                Some("application_hub"),
                Some(sha256_hex(wasm)),
            ),
            canister(
                "idle",
                idle,
                subnet,
                DesiredCanisterKind::Pool,
                Some("root"),
                Some("root"),
                None,
            ),
        ],
    );
    let before = [coordinator, root, child, idle].map(|id| {
        pic.canister_status(
            id,
            Some(if id == child || id == idle {
                root
            } else {
                operator
            }),
        )
        .unwrap()
        .version
    });
    let url = pic.make_live(None);
    let agent = Agent::builder()
        .with_url(url.to_string())
        .with_identity(identity)
        .with_max_response_body_size(RESPONSE_BYTES)
        .build()
        .unwrap();
    agent.set_root_key(key);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let observed = runtime.block_on(observe(&agent, &source)).unwrap();
    assert_eq!(observed.canisters().len(), source.canisters.len());
    assert_ne!(
        observed.canisters()["coordinator"].subnet(),
        observed.canisters()["root"].subnet()
    );
    assert!(observed.canisters()["child"].controllers().contains(&extra));
    assert_eq!(observed.canisters()["idle"].module_sha256(), None);
    assert_eq!(
        before,
        [coordinator, root, child, idle].map(|id| pic
            .canister_status(
                id,
                Some(if id == child || id == idle {
                    root
                } else {
                    operator
                })
            )
            .unwrap()
            .version)
    );
    source.canisters.get_mut("root").unwrap().subnet = SubnetId::from_principal(subnets[0]);
    // Match the Root-local rows so local membership is valid, then reject certified placement.
    source.canisters.get_mut("child").unwrap().subnet = SubnetId::from_principal(subnets[0]);
    source.canisters.get_mut("idle").unwrap().subnet = SubnetId::from_principal(subnets[0]);
    assert!(matches!(
        runtime.block_on(observe(&agent, &source)),
        Err(CompletedCustodyError::SubnetMismatch { .. })
    ));
    for name in ["root", "child", "idle"] {
        source.canisters.get_mut(name).unwrap().subnet = subnet;
    }
    source.canisters.get_mut("child").unwrap().module_sha256 = Some("ff".repeat(32));
    assert!(
        matches!(runtime.block_on(observe(&agent, &source)), Err(CompletedCustodyError::ModuleMismatch { canister }) if canister == child)
    );
    source.canisters.get_mut("child").unwrap().module_sha256 = Some(sha256_hex(wasm));
    pic.set_controllers(child, Some(root), vec![extra]).unwrap();
    assert!(
        matches!(runtime.block_on(observe(&agent, &source)), Err(CompletedCustodyError::ControllerMismatch { canister, owner }) if canister == child && owner == root)
    );
    source.receipts.source_operator = extra.to_text();
    assert!(matches!(
        runtime.block_on(observe(&agent, &source)),
        Err(CompletedCustodyError::ReaderMismatch)
    ));
    source.receipts.source_operator = operator.to_text();
    source.fleet.fleet.canonical_network_id = CanonicalNetworkId::ic_mainnet();
    assert!(matches!(
        runtime.block_on(observe(&agent, &source)),
        Err(CompletedCustodyError::ReaderMismatch)
    ));
}
