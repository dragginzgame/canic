use super::*;
use candid::Principal;

const ROOT: Principal = Principal::from_slice(&[]);
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build a deterministic non-root principal for discovery tests.
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

// Ensure discovery projections produce valid manifest fleet sections.
#[test]
fn discovery_projects_to_valid_fleet_section() {
    let fleet = DiscoveredFleet {
        topology_records: vec![
            topology_record(ROOT, None, "root"),
            topology_record(p(2), Some(ROOT), "app"),
        ],
        members: vec![
            discovered_member("root", ROOT.to_string(), None),
            discovered_member("app", p(2).to_string(), Some(ROOT.to_string())),
        ],
    };

    let section = fleet
        .into_fleet_section()
        .expect("discovery should project");

    section.validate().expect("fleet section should validate");
    assert_eq!(section.discovery_topology_hash, section.topology_hash);
    assert_eq!(section.members.len(), 2);
}

// Ensure duplicate canisters are rejected before manifest projection.
#[test]
fn discovery_rejects_duplicate_canisters() {
    let fleet = DiscoveredFleet {
        topology_records: vec![topology_record(ROOT, None, "root")],
        members: vec![
            discovered_member("root", ROOT.to_string(), None),
            discovered_member("root", ROOT.to_string(), None),
        ],
    };

    let err = fleet
        .into_fleet_section()
        .expect_err("duplicate canisters should fail");

    assert!(matches!(err, DiscoveryError::DuplicateCanisterId(_)));
}

// Ensure discovery requires concrete member verification.
#[test]
fn discovery_requires_verification_checks() {
    let mut member = discovered_member("root", ROOT.to_string(), None);
    member.verification_checks.clear();
    let fleet = DiscoveredFleet {
        topology_records: vec![topology_record(ROOT, None, "root")],
        members: vec![member],
    };

    let err = fleet
        .into_fleet_section()
        .expect_err("missing verification should fail");

    assert!(matches!(err, DiscoveryError::MissingVerificationChecks(_)));
}

// Build one topology record for tests.
fn topology_record(pid: Principal, parent_pid: Option<Principal>, role: &str) -> TopologyRecord {
    TopologyRecord {
        pid,
        parent_pid,
        role: role.to_string(),
        module_hash: None,
    }
}

// Build one discovered member for tests.
fn discovered_member(
    role: &str,
    canister_id: String,
    parent_canister_id: Option<String>,
) -> DiscoveredMember {
    DiscoveredMember {
        role: role.to_string(),
        canister_id,
        parent_canister_id,
        subnet_canister_id: None,
        controller_hint: Some(ROOT.to_string()),
        identity_mode: IdentityMode::Fixed,
        restore_group: 1,
        verification_class: "basic".to_string(),
        verification_checks: vec![VerificationCheck {
            kind: "call".to_string(),
            method: Some("canic_ready".to_string()),
            roles: Vec::new(),
        }],
        snapshot_plan: SnapshotPlan {
            snapshot_id: format!("snap-{role}"),
            module_hash: Some(HASH.to_string()),
            wasm_hash: Some(HASH.to_string()),
            code_version: Some("v0.30.0".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}
