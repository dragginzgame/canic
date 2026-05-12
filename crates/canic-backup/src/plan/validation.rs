use super::{
    BackupOperation, BackupOperationKind, BackupPlan, BackupPlanError, BackupScopeKind,
    ControlAuthority, ControlAuthoritySource,
};
use candid::Principal;
use std::{collections::BTreeSet, str::FromStr};

impl BackupPlan {
    /// Validate the backup plan as a dry-run/planning artifact.
    pub fn validate(&self) -> Result<(), BackupPlanError> {
        validate_nonempty("plan_id", &self.plan_id)?;
        validate_nonempty("run_id", &self.run_id)?;
        validate_nonempty("fleet", &self.fleet)?;
        validate_nonempty("network", &self.network)?;
        validate_principal("root_canister_id", &self.root_canister_id)?;
        validate_optional_principal(
            "selected_subtree_root",
            self.selected_subtree_root.as_deref(),
        )?;
        validate_nonempty(
            "topology_hash_before_quiesce",
            &self.topology_hash_before_quiesce,
        )?;
        validate_root_scope(self)?;
        validate_targets(self)?;
        validate_selected_scope(self)?;
        validate_phase_order(&self.phases)
    }

    /// Validate the backup plan before any live mutation can run.
    pub fn validate_for_execution(&self) -> Result<(), BackupPlanError> {
        self.validate()?;

        for target in &self.targets {
            if !target.control_authority.is_proven() {
                return Err(BackupPlanError::UnprovenControlAuthority(
                    target.canister_id.clone(),
                ));
            }
            if !target.snapshot_read_authority.is_proven() {
                return Err(BackupPlanError::UnprovenTargetSnapshotReadAuthority(
                    target.canister_id.clone(),
                ));
            }
            if self.requires_root_controller
                && target.canister_id != self.root_canister_id
                && !target.control_authority.is_proven_root_controller()
            {
                return Err(BackupPlanError::MissingRootController(
                    target.canister_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

fn validate_root_scope(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    if plan.selected_scope_kind == BackupScopeKind::MaintenanceRoot {
        if plan.root_included {
            return Ok(());
        }
        return Err(BackupPlanError::MaintenanceRootExcludesRoot);
    }

    if plan.root_included {
        return Err(BackupPlanError::RootIncludedWithoutMaintenance);
    }

    Ok(())
}

fn validate_targets(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    if plan.targets.is_empty() {
        return Err(BackupPlanError::EmptyTargets);
    }

    let mut target_ids = BTreeSet::new();
    for target in &plan.targets {
        validate_principal("targets[].canister_id", &target.canister_id)?;
        validate_optional_principal(
            "targets[].parent_canister_id",
            target.parent_canister_id.as_deref(),
        )?;
        validate_optional_nonempty("targets[].role", target.role.as_deref())?;
        validate_optional_nonempty(
            "targets[].expected_module_hash",
            target.expected_module_hash.as_deref(),
        )?;
        validate_control_authority(&target.control_authority)?;

        if !target_ids.insert(target.canister_id.clone()) {
            return Err(BackupPlanError::DuplicateTarget(target.canister_id.clone()));
        }
        if !plan.root_included && target.canister_id == plan.root_canister_id {
            return Err(BackupPlanError::RootIncludedWithoutMaintenance);
        }
    }

    validate_operation_targets(&plan.phases, &target_ids)
}

pub(super) fn validate_control_authority(
    authority: &ControlAuthority,
) -> Result<(), BackupPlanError> {
    match &authority.source {
        ControlAuthoritySource::Unknown
        | ControlAuthoritySource::RootController
        | ControlAuthoritySource::OperatorController => Ok(()),
        ControlAuthoritySource::AlternateController { controller, reason } => {
            validate_principal("targets[].control_authority.controller", controller)?;
            validate_nonempty("targets[].control_authority.reason", reason)
        }
    }
}

fn validate_selected_scope(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootFleet => {
            if plan.selected_subtree_root.is_some() {
                return Err(BackupPlanError::NonRootFleetHasSelectedRoot);
            }
            Ok(())
        }
        BackupScopeKind::Member | BackupScopeKind::Subtree | BackupScopeKind::MaintenanceRoot => {
            let Some(selected_root) = &plan.selected_subtree_root else {
                return Err(BackupPlanError::EmptyField("selected_subtree_root"));
            };
            if plan
                .targets
                .iter()
                .any(|target| &target.canister_id == selected_root)
            {
                Ok(())
            } else {
                Err(BackupPlanError::SelectedRootNotInTargets(
                    selected_root.clone(),
                ))
            }
        }
    }
}

fn validate_operation_targets(
    phases: &[BackupOperation],
    target_ids: &BTreeSet<String>,
) -> Result<(), BackupPlanError> {
    if phases.is_empty() {
        return Err(BackupPlanError::EmptyPhases);
    }

    let mut operation_ids = BTreeSet::new();
    for (index, phase) in phases.iter().enumerate() {
        validate_nonempty("phases[].operation_id", &phase.operation_id)?;
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if phase.order != expected {
            return Err(BackupPlanError::OperationOrderMismatch {
                operation_id: phase.operation_id.clone(),
                order: phase.order,
                expected,
            });
        }
        if !operation_ids.insert(phase.operation_id.clone()) {
            return Err(BackupPlanError::DuplicateOperationId(
                phase.operation_id.clone(),
            ));
        }
        if let Some(target) = &phase.target_canister_id {
            validate_principal("phases[].target_canister_id", target)?;
            if !target_ids.contains(target) {
                return Err(BackupPlanError::UnknownOperationTarget {
                    operation_id: phase.operation_id.clone(),
                    target_canister_id: target.clone(),
                });
            }
        }
    }

    Ok(())
}

fn validate_phase_order(phases: &[BackupOperation]) -> Result<(), BackupPlanError> {
    let topology = preflight_position(phases, BackupOperationKind::ValidateTopology, "topology")?;
    let control = preflight_position(
        phases,
        BackupOperationKind::ValidateControlAuthority,
        "control_authority",
    )?;
    let read = preflight_position(
        phases,
        BackupOperationKind::ValidateSnapshotReadAuthority,
        "snapshot_read_authority",
    )?;
    let quiescence = preflight_position(
        phases,
        BackupOperationKind::ValidateQuiescencePolicy,
        "quiescence_policy",
    )?;
    let preflight_cutoff = [topology, control, read, quiescence]
        .into_iter()
        .max()
        .expect("non-empty preflight positions");

    for (index, phase) in phases.iter().enumerate() {
        if index < preflight_cutoff && phase.kind.is_mutating() {
            return Err(BackupPlanError::MutationBeforePreflight {
                operation_id: phase.operation_id.clone(),
            });
        }
    }

    Ok(())
}

fn preflight_position(
    phases: &[BackupOperation],
    kind: BackupOperationKind,
    label: &'static str,
) -> Result<usize, BackupPlanError> {
    phases
        .iter()
        .position(|phase| phase.kind == kind)
        .ok_or(BackupPlanError::MissingPreflight(label))
}

impl BackupOperationKind {
    const fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Stop | Self::CreateSnapshot | Self::Start | Self::DownloadSnapshot
        )
    }
}

pub(super) fn validate_nonempty(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    if value.trim().is_empty() {
        Err(BackupPlanError::EmptyField(field))
    } else {
        Ok(())
    }
}

pub(super) fn validate_optional_nonempty(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), BackupPlanError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Ok(()),
    }
}

pub(super) fn validate_principal(field: &'static str, value: &str) -> Result<(), BackupPlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| BackupPlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

pub(super) fn validate_required_hash(
    field: &'static str,
    value: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty(field, value)?;
    if value.len() == 64 && value.chars().all(|char| char.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(BackupPlanError::InvalidTopologyHash {
            field,
            value: value.to_string(),
        })
    }
}

pub(super) fn validate_preflight_id(value: &str) -> Result<(), BackupPlanError> {
    validate_nonempty("preflight_id", value)
}

pub(super) fn validate_preflight_window(
    preflight_id: &str,
    validated_at: &str,
    expires_at: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    let validated_at_seconds =
        validate_preflight_timestamp("preflight_receipts[].validated_at", validated_at)?;
    let expires_at_seconds =
        validate_preflight_timestamp("preflight_receipts[].expires_at", expires_at)?;
    let as_of_seconds = validate_preflight_timestamp("preflight_receipts.as_of", as_of)?;

    if validated_at_seconds >= expires_at_seconds {
        return Err(BackupPlanError::PreflightReceiptInvalidWindow {
            preflight_id: preflight_id.to_string(),
        });
    }
    if as_of_seconds < validated_at_seconds {
        return Err(BackupPlanError::PreflightReceiptNotYetValid {
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            as_of: as_of.to_string(),
        });
    }
    if as_of_seconds >= expires_at_seconds {
        return Err(BackupPlanError::PreflightReceiptExpired {
            preflight_id: preflight_id.to_string(),
            expires_at: expires_at.to_string(),
            as_of: as_of.to_string(),
        });
    }

    Ok(())
}

pub(super) fn validate_preflight_timestamp(
    field: &'static str,
    value: &str,
) -> Result<u64, BackupPlanError> {
    validate_nonempty(field, value)?;
    value
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .ok_or_else(|| BackupPlanError::InvalidTimestamp {
            field,
            value: value.to_string(),
        })
}

fn validate_optional_principal(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), BackupPlanError> {
    match value {
        Some(value) => validate_principal(field, value),
        None => Ok(()),
    }
}
