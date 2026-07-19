//! Module: plan::validation
//!
//! Responsibility: validate backup plan structure and execution readiness.
//! Does not own: plan construction, preflight receipt mapping, or journaling.
//! Boundary: enforces plan invariants before dry-run or live execution.

use super::{BackupPlan, BackupPlanError, BackupScopeKind, BackupTarget, build_backup_phases};
use candid::Principal;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

const SUPPORTED_BACKUP_PLAN_VERSION: u16 = 1;

impl BackupPlan {
    /// Validate the backup plan as a dry-run/planning artifact.
    pub fn validate(&self) -> Result<(), BackupPlanError> {
        if self.plan_version != SUPPORTED_BACKUP_PLAN_VERSION {
            return Err(BackupPlanError::UnsupportedVersion(self.plan_version));
        }
        validate_nonempty("plan_id", &self.plan_id)?;
        validate_nonempty("run_id", &self.run_id)?;
        validate_nonempty("fleet", &self.fleet)?;
        validate_nonempty("environment", &self.environment)?;
        validate_principal("root_canister_id", &self.root_canister_id)?;
        validate_optional_principal(
            "selected_subtree_root",
            self.selected_subtree_root.as_deref(),
        )?;
        validate_required_hash(
            "topology_hash_before_quiesce",
            &self.topology_hash_before_quiesce,
        )?;
        validate_root_scope(self)?;
        validate_targets(self)?;
        validate_selected_scope(self)?;
        validate_target_topology(self)?;
        validate_phase_projection(self)
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
        if !target_ids.insert(target.canister_id.clone()) {
            return Err(BackupPlanError::DuplicateTarget(target.canister_id.clone()));
        }
        if !plan.root_included && target.canister_id == plan.root_canister_id {
            return Err(BackupPlanError::RootIncludedWithoutMaintenance);
        }
    }

    Ok(())
}

fn validate_selected_scope(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootDeployment => {
            if plan.selected_subtree_root.is_some() {
                return Err(BackupPlanError::NonRootDeploymentHasSelectedRoot);
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

fn validate_target_topology(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    let targets = plan
        .targets
        .iter()
        .map(|target| (target.canister_id.as_str(), target))
        .collect::<BTreeMap<_, _>>();

    for target in &plan.targets {
        validate_target_parent_chain(target.canister_id.as_str(), &targets)?;
        if let Some(parent_canister_id) = target.parent_canister_id.as_deref()
            && let Some(parent) = targets.get(parent_canister_id)
        {
            let expected = u64::from(parent.depth) + 1;
            if u64::from(target.depth) != expected {
                return Err(BackupPlanError::TargetDepthMismatch {
                    canister_id: target.canister_id.clone(),
                    parent_canister_id: parent_canister_id.to_string(),
                    expected,
                    actual: target.depth,
                });
            }
        }
    }

    if plan.selected_scope_kind == BackupScopeKind::NonRootDeployment {
        for target in &plan.targets {
            validate_target_connects_to_root(target, &plan.root_canister_id, &targets)?;
        }
        return Ok(());
    }
    let selected_root = plan
        .selected_subtree_root
        .as_deref()
        .ok_or(BackupPlanError::EmptyField("selected_subtree_root"))?;
    let selected = targets
        .get(selected_root)
        .ok_or_else(|| BackupPlanError::SelectedRootNotInTargets(selected_root.to_string()))?;
    if let Some(parent_canister_id) = selected.parent_canister_id.as_deref()
        && targets.contains_key(parent_canister_id)
    {
        return Err(BackupPlanError::SelectedRootHasInternalParent {
            selected_root: selected_root.to_string(),
            parent_canister_id: parent_canister_id.to_string(),
        });
    }

    for target in &plan.targets {
        if target.canister_id != selected_root {
            validate_target_connects_to_root(target, selected_root, &targets)?;
        }
    }
    Ok(())
}

fn validate_target_parent_chain(
    canister_id: &str,
    targets: &BTreeMap<&str, &BackupTarget>,
) -> Result<(), BackupPlanError> {
    let mut current = canister_id;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(current) {
            return Err(BackupPlanError::TargetParentCycle {
                canister_id: canister_id.to_string(),
            });
        }
        let Some(parent) = targets
            .get(current)
            .and_then(|target| target.parent_canister_id.as_deref())
            .and_then(|parent| targets.get(parent))
        else {
            return Ok(());
        };
        current = parent.canister_id.as_str();
    }
}

fn validate_target_connects_to_root(
    target: &BackupTarget,
    expected_root: &str,
    targets: &BTreeMap<&str, &BackupTarget>,
) -> Result<(), BackupPlanError> {
    let mut current = target;
    while let Some(parent_canister_id) = current.parent_canister_id.as_deref() {
        if parent_canister_id == expected_root {
            return Ok(());
        }
        let Some(parent) = targets.get(parent_canister_id) else {
            break;
        };
        current = parent;
    }

    Err(BackupPlanError::TargetDisconnected {
        canister_id: target.canister_id.clone(),
        expected_root: expected_root.to_string(),
    })
}

fn validate_phase_projection(plan: &BackupPlan) -> Result<(), BackupPlanError> {
    let expected = build_backup_phases(&plan.targets);
    if plan.phases.len() != expected.len() {
        return Err(BackupPlanError::OperationCountMismatch {
            expected: expected.len(),
            actual: plan.phases.len(),
        });
    }

    for (index, (actual, expected)) in plan.phases.iter().zip(expected).enumerate() {
        let field = if actual.order != expected.order {
            Some("order")
        } else if actual.operation_id != expected.operation_id {
            Some("operation_id")
        } else if actual.kind != expected.kind {
            Some("kind")
        } else if actual.target_canister_id != expected.target_canister_id {
            Some("target_canister_id")
        } else {
            None
        };
        if let Some(field) = field {
            return Err(BackupPlanError::OperationProjectionMismatch { index, field });
        }
    }
    Ok(())
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
