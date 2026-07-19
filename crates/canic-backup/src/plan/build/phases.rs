//! Module: plan::build::phases
//!
//! Responsibility: derive ordered backup operations for selected targets.
//! Does not own: target selection, registry traversal, or execution.
//! Boundary: builds operation rows consumed by backup execution journals.

use crate::plan::{BackupOperation, BackupOperationKind, BackupTarget};

pub(in crate::plan) fn build_backup_phases(targets: &[BackupTarget]) -> Vec<BackupOperation> {
    let mut phases = vec![
        operation(
            "validate-topology",
            BackupOperationKind::ValidateTopology,
            None,
        ),
        operation(
            "validate-control-authority",
            BackupOperationKind::ValidateControlAuthority,
            None,
        ),
        operation(
            "validate-snapshot-read-authority",
            BackupOperationKind::ValidateSnapshotReadAuthority,
            None,
        ),
        operation(
            "validate-quiescence-policy",
            BackupOperationKind::ValidateQuiescencePolicy,
            None,
        ),
    ];

    let mut top_down = targets.iter().collect::<Vec<_>>();
    top_down.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.canister_id.cmp(&right.canister_id))
    });
    for target in &top_down {
        phases.push(operation(
            format!("stop-{}", target.canister_id),
            BackupOperationKind::Stop,
            Some(target.canister_id.clone()),
        ));
    }
    for target in &top_down {
        phases.push(operation(
            format!("snapshot-{}", target.canister_id),
            BackupOperationKind::CreateSnapshot,
            Some(target.canister_id.clone()),
        ));
    }

    let mut bottom_up = top_down;
    bottom_up.reverse();
    for target in &bottom_up {
        phases.push(operation(
            format!("start-{}", target.canister_id),
            BackupOperationKind::Start,
            Some(target.canister_id.clone()),
        ));
    }

    for target in targets {
        phases.push(operation(
            format!("download-{}", target.canister_id),
            BackupOperationKind::DownloadSnapshot,
            Some(target.canister_id.clone()),
        ));
        phases.push(operation(
            format!("verify-{}", target.canister_id),
            BackupOperationKind::VerifyArtifact,
            Some(target.canister_id.clone()),
        ));
    }
    phases.push(operation(
        "finalize-manifest",
        BackupOperationKind::FinalizeManifest,
        None,
    ));

    phases
        .into_iter()
        .enumerate()
        .map(|(index, mut phase)| {
            phase.order = u32::try_from(index).unwrap_or(u32::MAX);
            phase
        })
        .collect()
}

fn operation(
    operation_id: impl Into<String>,
    kind: BackupOperationKind,
    target_canister_id: Option<String>,
) -> BackupOperation {
    BackupOperation {
        operation_id: operation_id.into(),
        order: 0,
        kind,
        target_canister_id,
    }
}
