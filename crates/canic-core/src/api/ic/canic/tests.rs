use super::*;
use crate::{dto::error::ErrorCode, ids::CanisterRole};

#[test]
fn protected_internal_endpoint_descriptor_matches_roles() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ],
    );

    assert_eq!(endpoint.method(), "system_add_project_to_user");
    assert_eq!(endpoint.accepted_roles_label(), "project_hub, admin_hub");
    assert!(endpoint.accepts_role(&CanisterRole::new("project_hub")));
    assert!(endpoint.accepts_role(&CanisterRole::new("admin_hub")));
    assert!(!endpoint.accepts_role(&CanisterRole::new("user_hub")));
    assert!(endpoint.single_role().is_none());
}

#[test]
fn protected_internal_endpoint_single_role_is_available_to_generated_descriptors() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [CanisterRole::new("project_hub")],
    );

    assert_eq!(
        endpoint.single_role(),
        Some(&CanisterRole::new("project_hub"))
    );
    assert_eq!(
        endpoint.required_single_role().expect("single role"),
        CanisterRole::new("project_hub")
    );
}

#[test]
fn protected_internal_endpoint_requires_explicit_role_when_ambiguous() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ],
    );

    let err = endpoint
        .required_single_role()
        .expect_err("multi-role endpoint should require explicit caller role");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("project_hub, admin_hub"));
    assert!(err.message.contains("choose a caller role explicitly"));
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_missing_method() {
    let result =
        std::panic::catch_unwind(|| ProtectedInternalEndpoint::new("", [CanisterRole::ROOT]));

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_blank_method() {
    let result =
        std::panic::catch_unwind(|| ProtectedInternalEndpoint::new("   ", [CanisterRole::ROOT]));

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_missing_roles() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_empty_role() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("")])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_blank_role() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("   ")])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_duplicate_roles() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [
                CanisterRole::new("project_hub"),
                CanisterRole::new("project_hub"),
            ],
        )
    });

    assert!(result.is_err());
}
