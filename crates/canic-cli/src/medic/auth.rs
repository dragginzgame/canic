//! Module: canic_cli::medic::auth
//!
//! Responsibility: classify auth-renewal readiness for deployment Medic reports.
//! Does not own: auth mutation, issuer resolution, or report rendering.
//! Boundary: maps the auth command summary and typed failures into Medic checks.

use crate::{
    auth::{self as auth_api, AuthCommandError, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    medic::{
        command::MedicOptions,
        report::{MedicCategory, MedicCheck, MedicSource},
    },
};

pub(super) fn check_auth_renewal(
    options: &MedicOptions,
    issuer: &str,
    network: &str,
) -> MedicCheck {
    match auth_api::renewal_medic_summary(options.deployment_name(), issuer, network, &options.icp)
    {
        Ok(summary) => auth_renewal_medic_check_from_summary(summary),
        Err(err) => auth_renewal_medic_error_check(err, options.deployment_name(), issuer),
    }
}

pub(super) fn auth_renewal_medic_error_check(
    error: AuthCommandError,
    deployment: &str,
    issuer: &str,
) -> MedicCheck {
    let (code, next, source) = match &error {
        AuthCommandError::InvalidIssuerPrincipal { .. } => (
            "auth_renewal_issuer_invalid",
            "pass a valid issuer canister principal".to_string(),
            MedicSource::Command,
        ),
        _ => (
            "auth_renewal_drift_fail",
            format!("run canic auth renewal status {deployment} --issuer {issuer}"),
            MedicSource::AuthRenewal,
        ),
    };

    MedicCheck::fail(
        MedicCategory::Auth,
        code,
        "auth_renewal",
        error.to_string(),
        next,
        source,
    )
}

pub(super) fn auth_renewal_medic_check_from_summary(
    summary: AuthRenewalMedicSummary,
) -> MedicCheck {
    match summary.status {
        AuthRenewalMedicStatus::Ready => MedicCheck::pass(
            MedicCategory::Auth,
            "auth_renewal_ready",
            "auth_renewal",
            summary.detail,
            summary.next,
            MedicSource::AuthRenewal,
        ),
        AuthRenewalMedicStatus::Warning => MedicCheck::warn(
            MedicCategory::Auth,
            "auth_renewal_drift_warn",
            "auth_renewal",
            summary.detail,
            summary.next,
            MedicSource::AuthRenewal,
        ),
    }
}
