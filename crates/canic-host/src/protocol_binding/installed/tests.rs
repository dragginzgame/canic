use super::*;

#[test]
fn target_selection_requires_unique_role_or_exact_principal() {
    let first = RegistryEntry {
        pid: ic_platform::Principal::from_slice(&[1]).to_text(),
        role: Some("worker-pool".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    let mut second = first.clone();
    second.pid = ic_platform::Principal::from_slice(&[2]).to_text();
    let entries = vec![first, second];
    assert_eq!(
        select_entry(&entries[..1], "worker-pool").unwrap(),
        &entries[0]
    );
    assert_eq!(
        select_entry(&entries, &entries[1].pid).unwrap(),
        &entries[1]
    );
    assert!(matches!(
        select_entry(&entries, "worker-pool"),
        Err(InstalledCandidError::Target(_))
    ));
    assert!(matches!(
        select_entry(&entries, "missing"),
        Err(InstalledCandidError::Target(_))
    ));
}

#[test]
fn retained_declaration_bytes_must_match_the_installed_profile() {
    use super::super::{RegistryProtocolBinding, verify_protocol_binding};
    use canic_core::role_contract::{RoleCapabilityKey, derive_protocol_profile_hashes};
    let bytes = b"service : { current : () -> (); }";
    let role = canic_contracts::ids::CanisterRole::from("app");
    let capabilities = [RoleCapabilityKey::Runtime].into();
    let hashes = derive_protocol_profile_hashes("release", &role, &capabilities, bytes);
    let mut binding = RegistryProtocolBinding {
        release_identity: "release".into(),
        role,
        capabilities,
        candid_sha256: hashes.candid_sha256,
        protocol_profile_digest: hashes.protocol_profile_digest,
    };
    verify_protocol_binding("target", &binding, bytes).unwrap();
    assert!(matches!(
        verify_protocol_binding("target", &binding, b"service : {}"),
        Err(ProtocolBindingError::CandidHashMismatch { .. })
    ));
    binding.capabilities.clear();
    assert!(matches!(
        verify_protocol_binding("target", &binding, bytes),
        Err(ProtocolBindingError::ProfileDigestMismatch { .. })
    ));
    binding.release_identity.clear();
    assert!(matches!(
        verify_protocol_binding("target", &binding, bytes),
        Err(ProtocolBindingError::MissingReleaseIdentity { .. })
    ));
}
