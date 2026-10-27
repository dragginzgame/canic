use super::*;

#[test]
fn target_selection_requires_unique_role_or_exact_principal() {
    let first = RegistryEntry {
        pid: ic_platform::principal::ExternalPrincipal::from_slice(&[1]).to_text(),
        role: Some("worker-pool".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    let mut second = first.clone();
    second.pid = ic_platform::principal::ExternalPrincipal::from_slice(&[2]).to_text();
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
        Err(InstalledDeclarationError::Target(_))
    ));
    assert!(matches!(
        select_entry(&entries, "missing"),
        Err(InstalledDeclarationError::Target(_))
    ));
}
