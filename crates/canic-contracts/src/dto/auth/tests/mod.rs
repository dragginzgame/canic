#[test]
fn auth_dtos_remain_passive_boundary_types() {
    let production_source = concat!(
        include_str!("../attestation/mod.rs"),
        include_str!("../common/mod.rs"),
        include_str!("../proof/mod.rs"),
        include_str!("../renewal/mod.rs"),
        include_str!("../enrollment/mod.rs"),
    );

    for marker in [
        "impl RoleAttestation",
        "impl SignedRoleAttestation",
        "fn verify",
        "fn sign",
        "fn resolve",
        "fn replay",
        "fn consume",
        "fn policy",
        "fn validate",
    ] {
        assert!(
            !production_source.contains(marker),
            "auth DTOs must stay passive; found marker `{marker}`"
        );
    }
}
