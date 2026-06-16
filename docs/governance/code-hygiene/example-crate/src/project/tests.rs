//! Boundary-level tests for project admission and accepted record ownership.

use crate::{
    diagnostic::StyleDiagnosticCode,
    project::{ProjectAdmission, ProjectExample},
};

#[test]
fn admits_project_through_project_owner() {
    let mut project = ProjectExample::default();

    let report = project
        .admit("project-alpha", "application")
        .expect("valid admission should succeed");

    assert_eq!(report.admission().project_id(), "project-alpha");
    assert_eq!(report.step().label(), "project-alpha");
    assert_eq!(
        project.record_project_id("project-alpha"),
        Some("project-alpha")
    );
    assert_eq!(
        project.record_subnet_label("project-alpha"),
        Some("application")
    );
}

#[test]
fn rejects_empty_project_id_without_matching_messages() {
    let err =
        ProjectAdmission::new("   ", "application").expect_err("blank project ids should fail");

    assert_eq!(err.code(), StyleDiagnosticCode::EmptyProjectId);
}
