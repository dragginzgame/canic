use super::*;
use sha2::{Digest, Sha256};
use std::fs;

fn entry(root: &std::path::Path, candid: &[u8]) -> RegistryEntry {
    let role = CanisterRole::from("app");
    let capabilities = BTreeSet::from([RoleCapabilityKey::Runtime]);
    let profile = derive_protocol_profile_hashes("0.103.0", &role, &capabilities, candid);
    let did = root.join(".icp/local/canisters/app/app.did");
    fs::create_dir_all(did.parent().expect("DID parent")).expect("create DID parent");
    fs::write(&did, candid).expect("write DID");
    RegistryEntry {
        pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        role: Some("app".to_string()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: Some(RegistryProtocolBinding {
            release_identity: "0.103.0".to_string(),
            role,
            capabilities,
            candid_sha256: Sha256::digest(candid).into(),
            protocol_profile_digest: profile.protocol_profile_digest,
        }),
    }
}

#[test]
fn exact_registry_binding_resolves_before_transport() {
    let root = crate::test_support::temp_dir("canic-host-protocol-binding");
    let entry = entry(&root, b"service : {}\n");
    let resolved =
        resolve_registry_protocol_binding(&root, "local", &entry).expect("resolve exact binding");
    assert_eq!(resolved.binding, entry.protocol_binding.expect("binding"));
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn missing_or_drifted_binding_fails_closed() {
    let root = crate::test_support::temp_dir("canic-host-protocol-binding-drift");
    let mut missing = entry(&root, b"service : {}\n");
    missing.protocol_binding = None;
    assert!(matches!(
        resolve_registry_protocol_binding(&root, "local", &missing),
        Err(ProtocolBindingError::MissingBinding { .. })
    ));

    let mut drifted = entry(&root, b"service : {}\n");
    drifted
        .protocol_binding
        .as_mut()
        .expect("binding")
        .candid_sha256 = [9; 32];
    assert!(matches!(
        resolve_registry_protocol_binding(&root, "local", &drifted),
        Err(ProtocolBindingError::CandidHashMismatch { .. })
    ));
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn immutable_infrastructure_binding_reproduces_full_profile() {
    let root = crate::test_support::temp_dir("canic-host-infrastructure-protocol-binding");
    let candid = b"service : {}\n";
    let role = CanisterRole::from("fleet_coordinator");
    let capabilities = BTreeSet::from([RoleCapabilityKey::FleetCoordinator]);
    let profile = derive_protocol_profile_hashes("0.103.0", &role, &capabilities, candid);
    let did = root.join(".icp/local/canisters/fleet_coordinator/fleet_coordinator.did");
    fs::create_dir_all(did.parent().expect("DID parent")).expect("create DID parent");
    fs::write(&did, candid).expect("write DID");
    let artifact = crate::release_set::CanicInfrastructureArtifactEntry {
        role: crate::release_set::CanicInfrastructureRole::FleetCoordinator,
        package: "canic-fleet-coordinator".to_string(),
        protocol_release_identity: "0.103.0".to_string(),
        protocol_role: role,
        protocol_capabilities: capabilities,
        release_build_id: "01".repeat(32).parse().expect("release build"),
        wasm_relative_path: "coordinator.wasm".to_string(),
        wasm_size_bytes: 1,
        wasm_sha256_hex: "01".repeat(32),
        wasm_gz_relative_path: "coordinator.wasm.gz".to_string(),
        wasm_gz_size_bytes: 1,
        wasm_gz_sha256_hex: "02".repeat(32),
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
    };

    let resolved = resolve_infrastructure_protocol_binding(&root, "local", &artifact)
        .expect("resolve infrastructure binding");

    assert_eq!(resolved.binding().release_identity, "0.103.0");
    assert_eq!(resolved.candid_path(), did);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn fleet_subnet_root_artifact_selects_the_root_protocol_role() {
    let root = crate::test_support::temp_dir("canic-host-root-protocol-binding");
    let candid = b"service : {}\n";
    let role = CanisterRole::from("root");
    let capabilities = BTreeSet::from([RoleCapabilityKey::Root]);
    let profile = derive_protocol_profile_hashes("0.103.0", &role, &capabilities, candid);
    let did = root.join(".icp/local/canisters/root/root.did");
    fs::create_dir_all(did.parent().expect("DID parent")).expect("create DID parent");
    fs::write(&did, candid).expect("write DID");
    let artifact = crate::release_set::CanicInfrastructureArtifactEntry {
        role: crate::release_set::CanicInfrastructureRole::FleetSubnetRoot,
        package: "root-package".to_string(),
        protocol_release_identity: "0.103.0".to_string(),
        protocol_role: role,
        protocol_capabilities: capabilities,
        release_build_id: "01".repeat(32).parse().expect("release build"),
        wasm_relative_path: "root.wasm".to_string(),
        wasm_size_bytes: 1,
        wasm_sha256_hex: "01".repeat(32),
        wasm_gz_relative_path: "root.wasm.gz".to_string(),
        wasm_gz_size_bytes: 1,
        wasm_gz_sha256_hex: "02".repeat(32),
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
    };

    let resolved = resolve_infrastructure_protocol_binding(&root, "local", &artifact)
        .expect("resolve root infrastructure binding");

    assert_eq!(resolved.binding().role.as_str(), "root");
    assert_eq!(resolved.candid_path(), did);
    fs::remove_dir_all(root).expect("remove temp root");
}
