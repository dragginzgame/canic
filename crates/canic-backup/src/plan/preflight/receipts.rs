//! Module: plan::preflight::receipts
//!
//! Responsibility: validate execution preflight receipt bundles.
//! Does not own: authority probing, topology observation, or quiescence enforcement.
//! Boundary: gates backup mutation on accepted topology, authority, and quiescence receipts.

use crate::plan::{
    BackupExecutionPreflightReceipts, BackupPlan, BackupPlanError, QuiescencePreflightReceipt,
    TopologyPreflightReceipt,
    validation::{
        validate_nonempty, validate_optional_nonempty, validate_preflight_id,
        validate_preflight_timestamp, validate_preflight_window, validate_required_hash,
    },
};

impl BackupPlan {
    /// Validate execution-only preflight receipts before mutation starts.
    pub fn validate_execution_preflight_receipts(
        &self,
        topology_receipt: &TopologyPreflightReceipt,
        quiescence_receipt: &QuiescencePreflightReceipt,
        preflight_id: &str,
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        self.validate_for_execution()?;
        validate_preflight_id(preflight_id)?;
        validate_preflight_timestamp("preflight.as_of", as_of)?;
        validate_topology_preflight_receipt(self, topology_receipt, preflight_id, as_of)?;
        validate_quiescence_preflight_receipt(self, quiescence_receipt, preflight_id, as_of)
    }

    /// Apply and validate the full execution preflight receipt bundle.
    pub fn apply_execution_preflight_receipts(
        &mut self,
        receipts: &BackupExecutionPreflightReceipts,
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        validate_execution_preflight_bundle(self, receipts, as_of)?;
        self.apply_authority_preflight_receipts(
            &receipts.preflight_id,
            &receipts.control_authority,
            &receipts.snapshot_read_authority,
            as_of,
        )?;
        self.validate_execution_preflight_receipts(
            &receipts.topology,
            &receipts.quiescence,
            &receipts.preflight_id,
            as_of,
        )
    }
}

fn validate_execution_preflight_bundle(
    plan: &BackupPlan,
    receipts: &BackupExecutionPreflightReceipts,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("preflight_receipts.plan_id", &receipts.plan_id)?;
    validate_preflight_id(&receipts.preflight_id)?;
    validate_preflight_timestamp("preflight_receipts.as_of", as_of)?;
    validate_preflight_window(
        &receipts.preflight_id,
        &receipts.validated_at,
        &receipts.expires_at,
        as_of,
    )?;

    if receipts.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipts.plan_id.clone(),
        });
    }

    Ok(())
}

fn validate_topology_preflight_receipt(
    plan: &BackupPlan,
    receipt: &TopologyPreflightReceipt,
    preflight_id: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("topology_receipt.plan_id", &receipt.plan_id)?;
    validate_preflight_id(&receipt.preflight_id)?;
    validate_required_hash(
        "topology_receipt.topology_hash_before_quiesce",
        &receipt.topology_hash_before_quiesce,
    )?;
    validate_required_hash(
        "topology_receipt.topology_hash_at_preflight",
        &receipt.topology_hash_at_preflight,
    )?;
    validate_optional_nonempty("topology_receipt.message", receipt.message.as_deref())?;
    validate_preflight_window(
        &receipt.preflight_id,
        &receipt.validated_at,
        &receipt.expires_at,
        as_of,
    )?;

    if receipt.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipt.plan_id.clone(),
        });
    }
    if receipt.preflight_id != preflight_id {
        return Err(BackupPlanError::PreflightReceiptIdMismatch {
            expected: preflight_id.to_string(),
            actual: receipt.preflight_id.clone(),
        });
    }
    if receipt.topology_hash_before_quiesce != plan.topology_hash_before_quiesce {
        return Err(BackupPlanError::TopologyPreflightHashMismatch {
            expected: plan.topology_hash_before_quiesce.clone(),
            actual: receipt.topology_hash_before_quiesce.clone(),
        });
    }
    if receipt.topology_hash_at_preflight != plan.topology_hash_before_quiesce {
        return Err(BackupPlanError::TopologyPreflightHashMismatch {
            expected: plan.topology_hash_before_quiesce.clone(),
            actual: receipt.topology_hash_at_preflight.clone(),
        });
    }
    if receipt.targets != plan.topology_preflight_request().targets {
        return Err(BackupPlanError::TopologyPreflightTargetsMismatch);
    }

    Ok(())
}

fn validate_quiescence_preflight_receipt(
    plan: &BackupPlan,
    receipt: &QuiescencePreflightReceipt,
    preflight_id: &str,
    as_of: &str,
) -> Result<(), BackupPlanError> {
    validate_nonempty("quiescence_receipt.plan_id", &receipt.plan_id)?;
    validate_preflight_id(&receipt.preflight_id)?;
    validate_optional_nonempty("quiescence_receipt.message", receipt.message.as_deref())?;
    validate_preflight_window(
        &receipt.preflight_id,
        &receipt.validated_at,
        &receipt.expires_at,
        as_of,
    )?;

    if receipt.plan_id != plan.plan_id {
        return Err(BackupPlanError::PreflightReceiptPlanMismatch {
            expected: plan.plan_id.clone(),
            actual: receipt.plan_id.clone(),
        });
    }
    if receipt.preflight_id != preflight_id {
        return Err(BackupPlanError::PreflightReceiptIdMismatch {
            expected: preflight_id.to_string(),
            actual: receipt.preflight_id.clone(),
        });
    }
    if receipt.quiescence_policy != plan.quiescence_policy {
        return Err(BackupPlanError::QuiescencePolicyMismatch);
    }
    if !receipt.accepted {
        return Err(BackupPlanError::QuiescencePreflightRejected);
    }
    if receipt.targets != plan.quiescence_preflight_request().targets {
        return Err(BackupPlanError::QuiescencePreflightTargetsMismatch);
    }

    Ok(())
}
