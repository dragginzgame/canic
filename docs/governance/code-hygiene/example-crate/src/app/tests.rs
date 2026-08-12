//! Boundary-level tests for App admission and accepted record ownership.

use crate::{
    app::{AppAdmission, AppExample},
    diagnostic::StyleDiagnosticCode,
};

#[test]
fn admits_app_through_app_owner() {
    let mut app = AppExample::default();

    let report = app
        .admit("app-alpha", "application")
        .expect("valid admission should succeed");

    assert_eq!(report.admission().app_id(), "app-alpha");
    assert_eq!(report.step().label(), "app-alpha");
    assert_eq!(app.record_app_id("app-alpha"), Some("app-alpha"));
    assert_eq!(app.record_subnet_label("app-alpha"), Some("application"));
}

#[test]
fn rejects_empty_app_id_without_matching_messages() {
    let err = AppAdmission::new("   ", "application").expect_err("blank App ids should fail");

    assert_eq!(err.code(), StyleDiagnosticCode::EmptyAppId);
}
