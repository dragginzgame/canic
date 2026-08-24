//! Module: canic_cli::medic::admission
//!
//! Responsibility: classify protected Fleet-admission convergence for Medic.
//! Does not own: status transport, policy validation, or recovery mutation.
//! Boundary: consumes the same read-only report rendered by `canic admission status`.

use crate::{
    admission::{AdmissionStatusReport, collect_status},
    medic::{
        command::MedicOptions,
        fleet::FleetMedicContext,
        report::{MedicCategory, MedicCheck, MedicSource},
    },
    support::icp_target::IcpTargetOptions,
};

pub(super) fn check_fleet_admission(
    options: &MedicOptions,
    context: &FleetMedicContext,
) -> MedicCheck {
    if context.icp_root.is_none() {
        return MedicCheck::not_evaluated(
            MedicCategory::Runtime,
            "fleet_admission_not_evaluated",
            "admission",
            "Fleet admission observation skipped because the workspace root was not resolved",
            "run from a Canic workspace root, then rerun canic medic fleet <fleet>",
            MedicSource::AdmissionStatus,
        );
    }
    let target = IcpTargetOptions {
        environment: context.environment.clone(),
        icp: options.icp.clone(),
    };
    match collect_status(options.fleet_name(), &target) {
        Ok(report) => classify_report(&report),
        Err(error) => MedicCheck::fail(
            MedicCategory::Runtime,
            "fleet_admission_unavailable",
            "admission",
            error.to_string(),
            format!(
                "run canic admission status {} and recover the retained operation before serving protected ingress",
                options.fleet_name()
            ),
            MedicSource::AdmissionStatus,
        ),
    }
}

fn classify_report(report: &AdmissionStatusReport) -> MedicCheck {
    let current = report.current_operation.as_deref();
    let conflicting_root = current.and_then(|operation| {
        report.roots.iter().find(|root| {
            root.operation_id
                .as_deref()
                .is_some_and(|root_operation| root_operation != operation)
                && root.phase.as_deref() != Some("converged")
        })
    });
    if let Some(root) = conflicting_root {
        return MedicCheck::fail(
            MedicCategory::Runtime,
            "fleet_admission_operation_conflict",
            "admission",
            format!(
                "Coordinator operation {} conflicts with Root {} operation {}",
                current.expect("conflicting Root requires current operation"),
                root.root,
                root.operation_id.as_deref().unwrap_or("none"),
            ),
            "do not open protected ingress; recover the exact retained admission operation",
            MedicSource::AdmissionStatus,
        );
    }
    if let Some(operation) = current {
        let unresolved_roots = report
            .roots
            .iter()
            .filter(|root| root.first_unresolved.is_some())
            .count();
        return MedicCheck::warn(
            MedicCategory::Runtime,
            "fleet_admission_converging",
            "admission",
            format!(
                "operation={operation}; phase={}; generation={}; roots={}; unresolved_roots={unresolved_roots}",
                report.current_phase.as_deref().unwrap_or("unknown"),
                report.generation,
                report.roots.len(),
            ),
            format!(
                "wait for convergence or inspect canic admission status {}",
                report.fleet
            ),
            MedicSource::AdmissionStatus,
        );
    }
    let unhealthy = report.roots.iter().find(|root| {
        root.active_generation != report.generation
            || root.active_policy_digest != report.policy_digest
            || root.first_unresolved.is_some()
            || !matches!(root.phase.as_deref(), None | Some("converged"))
            || (root.participant_count > 0 && root.open_count != root.participant_count)
    });
    if let Some(root) = unhealthy {
        return MedicCheck::fail(
            MedicCategory::Runtime,
            "fleet_admission_not_converged",
            "admission",
            format!(
                "Root {} is generation {} phase {} with {}/{} participants open",
                root.root,
                root.active_generation,
                root.phase.as_deref().unwrap_or("idle"),
                root.open_count,
                root.participant_count,
            ),
            "keep protected ingress fenced and recover the retained admission operation",
            MedicSource::AdmissionStatus,
        );
    }
    MedicCheck::pass(
        MedicCategory::Runtime,
        "fleet_admission_converged",
        "admission",
        format!(
            "generation={}; policy_digest={}; roots={}; Fleet_Principals={}",
            report.generation,
            report.policy_digest,
            report.roots.len(),
            report.fleet_principal_count,
        ),
        "none",
        MedicSource::AdmissionStatus,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admission::AdmissionRootReport;
    use candid::Principal;

    fn report() -> AdmissionStatusReport {
        AdmissionStatusReport {
            fleet: "demo".to_string(),
            environment: "local".to_string(),
            coordinator: Principal::from_slice(&[1; 29]),
            registry_revision: 2,
            generation: 3,
            policy_digest: "03".repeat(32),
            fleet_principal_count: 2,
            narrower_rule_count: 0,
            narrower_principal_reference_count: 0,
            current_operation: None,
            current_phase: None,
            last_operation: Some("04".repeat(32)),
            last_phase: Some("completed".to_string()),
            roots: vec![AdmissionRootReport {
                root: Principal::from_slice(&[2; 29]),
                active_generation: 3,
                active_policy_digest: "03".repeat(32),
                operation_id: Some("04".repeat(32)),
                phase: Some("converged".to_string()),
                participant_count: 2,
                pending_count: 0,
                prepared_count: 0,
                activated_count: 0,
                open_count: 2,
                first_unresolved: None,
            }],
        }
    }

    #[test]
    fn medic_distinguishes_converged_active_and_stale_admission() {
        let converged = report();
        assert_eq!(
            classify_report(&converged).code,
            "fleet_admission_converged"
        );

        let mut active = converged.clone();
        active.current_operation = Some("05".repeat(32));
        active.current_phase = Some("preparing".to_string());
        active.roots[0].operation_id = Some("05".repeat(32));
        active.roots[0].phase = Some("preparing".to_string());
        active.roots[0].pending_count = 1;
        active.roots[0].open_count = 1;
        active.roots[0].first_unresolved = Some(Principal::from_slice(&[3; 29]));
        assert_eq!(classify_report(&active).code, "fleet_admission_converging");

        let mut stale = converged;
        stale.roots[0].active_generation = 2;
        assert_eq!(
            classify_report(&stale).code,
            "fleet_admission_not_converged"
        );
    }
}
