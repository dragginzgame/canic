use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    ids::{
        CanonicalNetworkId, CyclesFundingBudget, FleetAdmissionPolicy,
        FleetAdmissionPolicyTemplate, FleetBinding, FleetCoordinatorRootFundingPolicy,
        FleetFundingProfile, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
        SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
};

use crate::fleet_install_plan::PlannedSubnetPlacementCostEvidence;

pub fn coordinator_root_funding_policy() -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        funding_profile: FleetFundingProfile::SingleSubnet,
        minimum_reserve_cycles: Cycles::new(30_000_000_000_000),
        budget: CyclesFundingBudget {
            window_secs: 90 * 24 * 60 * 60,
            maximum_cycles: Cycles::new(30_000_000_000_000),
        },
        maximum_automatic_grants: 4,
        maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
    }
}

pub fn fleet_subnet_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(10_000_000_000_000),
            target_balance: Cycles::new(30_000_000_000_000),
            cooldown_secs: 30 * 24 * 60 * 60,
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
        },
        icp_refill: None,
    }
}

pub fn fleet_admission_policy_template() -> FleetAdmissionPolicyTemplate {
    compile_fleet_admission_policy_template(vec![Principal::from_slice(&[1; 29])], Vec::new())
        .expect("test Fleet admission policy template")
}

pub fn fleet_admission_policy(fleet: FleetBinding) -> FleetAdmissionPolicy {
    bind_initial_fleet_admission_policy(fleet, &fleet_admission_policy_template())
        .expect("test Fleet admission policy")
}

pub fn placement_cost(subnet: SubnetId) -> PlannedSubnetPlacementCostEvidence {
    PlannedSubnetPlacementCostEvidence {
        subnet,
        catalog_sha256: None,
        subnet_specialization: "not_required".to_string(),
        node_count: 13,
        cost_multiplier_numerator: 1,
        cost_multiplier_denominator: 1,
        acknowledge_fiduciary_cost: false,
        warning: None,
    }
}

// Build a unique temporary directory path for host tests that own cleanup.
pub fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

#[cfg(unix)]
pub fn create_fifo(path: &Path) {
    let status = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()
        .expect("run mkfifo");
    assert!(status.success(), "mkfifo failed with {status}");
}

// Write one deterministic local-network authority for tests that exercise
// environment-profile resolution without contacting a live replica.
pub fn write_local_network_authority(project_root: &Path, environment: &str) -> CanonicalNetworkId {
    let root_key = local_root_key();
    let network = local_network_id();
    let authority = project_root
        .join(".canic")
        .join("networks")
        .join(network.to_string());
    fs::create_dir_all(authority.join("trust")).expect("create network authority");
    fs::write(authority.join("trust/root-key.der"), &root_key).expect("write root key");
    fs::write(
        authority.join("enrollment.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "root_key_digest": canic_core::cdk::utils::hash::sha256_hex(&root_key),
            "enrolled_at": 1,
            "source_profile": environment,
        }))
        .expect("encode enrollment"),
    )
    .expect("write enrollment");
    let profile = project_root
        .join(".canic")
        .join("environment-profiles")
        .join(environment)
        .join("network.json");
    fs::create_dir_all(profile.parent().expect("profile parent")).expect("create profile dir");
    fs::write(
        profile,
        serde_json::to_vec_pretty(&serde_json::json!({
            "canonical_network_id": network,
        }))
        .expect("encode profile"),
    )
    .expect("write profile");
    network
}

pub fn local_network_id() -> CanonicalNetworkId {
    CanonicalNetworkId::from_der_root_trust_anchor(&local_root_key())
        .expect("canonical local network ID")
}

fn local_root_key() -> Vec<u8> {
    let mut root_key = vec![
        0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x02, 0x01, 0x03, 0x61, 0x00,
    ];
    root_key.extend_from_slice(&[9; 96]);
    root_key
}
