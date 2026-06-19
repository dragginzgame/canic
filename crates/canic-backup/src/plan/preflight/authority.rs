//! Module: plan::preflight::authority
//!
//! Responsibility: apply authority preflight receipts to backup plan targets.
//! Does not own: authority probing, plan construction, or execution mutation.
//! Boundary: validates receipt headers before upgrading target authority evidence.

use crate::plan::{
    BackupPlan, BackupPlanError, BackupTarget, ControlAuthorityReceipt,
    SnapshotReadAuthorityReceipt,
    validation::{
        validate_control_authority, validate_nonempty, validate_optional_nonempty,
        validate_preflight_id, validate_preflight_window, validate_principal,
    },
};

use std::collections::{BTreeMap, BTreeSet};

impl BackupPlan {
    /// Apply proven authority receipts produced by execution preflights.
    pub fn apply_authority_preflight_receipts(
        &mut self,
        preflight_id: &str,
        control_receipts: &[ControlAuthorityReceipt],
        snapshot_read_receipts: &[SnapshotReadAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        self.apply_control_authority_receipts(preflight_id, control_receipts, as_of)?;
        self.apply_snapshot_read_authority_receipts(preflight_id, snapshot_read_receipts, as_of)
    }

    /// Apply proven control authority receipts for every selected target.
    pub fn apply_control_authority_receipts(
        &mut self,
        preflight_id: &str,
        receipts: &[ControlAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        let mut receipts =
            control_receipt_map(&self.plan_id, preflight_id, as_of, &self.targets, receipts)?;
        let mut updates = Vec::with_capacity(self.targets.len());
        for (index, target) in self.targets.iter().enumerate() {
            let receipt = receipts.remove(&target.canister_id).ok_or_else(|| {
                BackupPlanError::MissingControlAuthorityReceipt(target.canister_id.clone())
            })?;
            if !receipt.authority.is_proven() {
                return Err(BackupPlanError::UnprovenControlAuthority(
                    target.canister_id.clone(),
                ));
            }
            if self.requires_root_controller
                && target.canister_id != self.root_canister_id
                && !receipt.authority.is_proven_root_controller()
            {
                return Err(BackupPlanError::MissingRootController(
                    target.canister_id.clone(),
                ));
            }
            updates.push((index, receipt.authority));
        }

        for (index, authority) in updates {
            self.targets[index].control_authority = authority;
        }
        Ok(())
    }

    /// Apply proven snapshot read authority receipts for every selected target.
    pub fn apply_snapshot_read_authority_receipts(
        &mut self,
        preflight_id: &str,
        receipts: &[SnapshotReadAuthorityReceipt],
        as_of: &str,
    ) -> Result<(), BackupPlanError> {
        let mut receipts =
            snapshot_read_receipt_map(&self.plan_id, preflight_id, as_of, &self.targets, receipts)?;
        let mut updates = Vec::with_capacity(self.targets.len());
        for (index, target) in self.targets.iter().enumerate() {
            let receipt = receipts.remove(&target.canister_id).ok_or_else(|| {
                BackupPlanError::MissingSnapshotReadAuthorityReceipt(target.canister_id.clone())
            })?;
            if !receipt.authority.is_proven() {
                return Err(BackupPlanError::UnprovenTargetSnapshotReadAuthority(
                    target.canister_id.clone(),
                ));
            }
            updates.push((index, receipt.authority));
        }

        for (index, authority) in updates {
            self.targets[index].snapshot_read_authority = authority;
        }
        Ok(())
    }
}

fn control_receipt_map(
    plan_id: &str,
    preflight_id: &str,
    as_of: &str,
    targets: &[BackupTarget],
    receipts: &[ControlAuthorityReceipt],
) -> Result<BTreeMap<String, ControlAuthorityReceipt>, BackupPlanError> {
    let target_ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut receipt_map = BTreeMap::new();

    for receipt in receipts {
        validate_authority_receipt_header(AuthorityReceiptHeaderInput {
            expected_plan_id: plan_id,
            expected_preflight_id: preflight_id,
            as_of,
            target_ids: &target_ids,
            actual_plan_id: &receipt.plan_id,
            actual_preflight_id: &receipt.preflight_id,
            target_canister_id: &receipt.target_canister_id,
            validated_at: &receipt.validated_at,
            expires_at: &receipt.expires_at,
            message: receipt.message.as_deref(),
        })?;
        validate_control_authority(&receipt.authority)?;
        if receipt_map
            .insert(receipt.target_canister_id.clone(), receipt.clone())
            .is_some()
        {
            return Err(BackupPlanError::DuplicateAuthorityReceipt(
                receipt.target_canister_id.clone(),
            ));
        }
    }

    Ok(receipt_map)
}

fn snapshot_read_receipt_map(
    plan_id: &str,
    preflight_id: &str,
    as_of: &str,
    targets: &[BackupTarget],
    receipts: &[SnapshotReadAuthorityReceipt],
) -> Result<BTreeMap<String, SnapshotReadAuthorityReceipt>, BackupPlanError> {
    let target_ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut receipt_map = BTreeMap::new();

    for receipt in receipts {
        validate_authority_receipt_header(AuthorityReceiptHeaderInput {
            expected_plan_id: plan_id,
            expected_preflight_id: preflight_id,
            as_of,
            target_ids: &target_ids,
            actual_plan_id: &receipt.plan_id,
            actual_preflight_id: &receipt.preflight_id,
            target_canister_id: &receipt.target_canister_id,
            validated_at: &receipt.validated_at,
            expires_at: &receipt.expires_at,
            message: receipt.message.as_deref(),
        })?;
        if receipt_map
            .insert(receipt.target_canister_id.clone(), receipt.clone())
            .is_some()
        {
            return Err(BackupPlanError::DuplicateAuthorityReceipt(
                receipt.target_canister_id.clone(),
            ));
        }
    }

    Ok(receipt_map)
}

struct AuthorityReceiptHeaderInput<'a> {
    expected_plan_id: &'a str,
    expected_preflight_id: &'a str,
    as_of: &'a str,
    target_ids: &'a BTreeSet<&'a str>,
    actual_plan_id: &'a str,
    actual_preflight_id: &'a str,
    target_canister_id: &'a str,
    validated_at: &'a str,
    expires_at: &'a str,
    message: Option<&'a str>,
}

fn validate_authority_receipt_header(
    input: AuthorityReceiptHeaderInput<'_>,
) -> Result<(), BackupPlanError> {
    validate_nonempty("authority_receipts[].plan_id", input.actual_plan_id)?;
    validate_preflight_id(input.actual_preflight_id)?;
    validate_principal(
        "authority_receipts[].target_canister_id",
        input.target_canister_id,
    )?;
    validate_optional_nonempty("authority_receipts[].message", input.message)?;
    validate_preflight_window(
        input.actual_preflight_id,
        input.validated_at,
        input.expires_at,
        input.as_of,
    )?;

    if input.actual_plan_id != input.expected_plan_id {
        return Err(BackupPlanError::AuthorityReceiptPlanMismatch {
            expected: input.expected_plan_id.to_string(),
            actual: input.actual_plan_id.to_string(),
        });
    }
    if input.actual_preflight_id != input.expected_preflight_id {
        return Err(BackupPlanError::AuthorityReceiptPreflightMismatch {
            expected: input.expected_preflight_id.to_string(),
            actual: input.actual_preflight_id.to_string(),
        });
    }
    if !input.target_ids.contains(input.target_canister_id) {
        return Err(BackupPlanError::UnknownAuthorityReceiptTarget(
            input.target_canister_id.to_string(),
        ));
    }

    Ok(())
}
