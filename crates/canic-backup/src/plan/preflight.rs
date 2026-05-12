use super::{
    BackupExecutionPreflightReceipts, BackupPlan, BackupPlanError, BackupTarget,
    ControlAuthorityPreflightRequest, ControlAuthorityPreflightTarget, ControlAuthorityReceipt,
    QuiescencePreflightReceipt, QuiescencePreflightRequest, QuiescencePreflightTarget,
    SnapshotReadAuthorityPreflightRequest, SnapshotReadAuthorityPreflightTarget,
    SnapshotReadAuthorityReceipt, TopologyPreflightReceipt, TopologyPreflightRequest,
    TopologyPreflightTarget,
    validation::{
        validate_control_authority, validate_nonempty, validate_optional_nonempty,
        validate_preflight_id, validate_preflight_timestamp, validate_preflight_window,
        validate_principal, validate_required_hash,
    },
};
use std::collections::{BTreeMap, BTreeSet};

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
        for target in &self.targets {
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
            updates.push((target.canister_id.clone(), receipt.authority));
        }

        for (target_id, authority) in updates {
            let target = self
                .targets
                .iter_mut()
                .find(|target| target.canister_id == target_id)
                .expect("validated update target should exist");
            target.control_authority = authority;
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
        for target in &self.targets {
            let receipt = receipts.remove(&target.canister_id).ok_or_else(|| {
                BackupPlanError::MissingSnapshotReadAuthorityReceipt(target.canister_id.clone())
            })?;
            if !receipt.authority.is_proven() {
                return Err(BackupPlanError::UnprovenTargetSnapshotReadAuthority(
                    target.canister_id.clone(),
                ));
            }
            updates.push((target.canister_id.clone(), receipt.authority));
        }

        for (target_id, authority) in updates {
            let target = self
                .targets
                .iter_mut()
                .find(|target| target.canister_id == target_id)
                .expect("validated update target should exist");
            target.snapshot_read_authority = authority;
        }
        Ok(())
    }

    /// Build the typed control-authority preflight request for this plan.
    #[must_use]
    pub fn control_authority_preflight_request(&self) -> ControlAuthorityPreflightRequest {
        ControlAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            requires_root_controller: self.requires_root_controller,
            targets: self
                .targets
                .iter()
                .map(ControlAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed snapshot-read preflight request for this plan.
    #[must_use]
    pub fn snapshot_read_authority_preflight_request(
        &self,
    ) -> SnapshotReadAuthorityPreflightRequest {
        SnapshotReadAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            targets: self
                .targets
                .iter()
                .map(SnapshotReadAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed topology preflight request for this plan.
    #[must_use]
    pub fn topology_preflight_request(&self) -> TopologyPreflightRequest {
        TopologyPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            selected_scope_kind: self.selected_scope_kind.clone(),
            topology_hash_before_quiesce: self.topology_hash_before_quiesce.clone(),
            targets: self
                .targets
                .iter()
                .map(TopologyPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed quiescence preflight request for this plan.
    #[must_use]
    pub fn quiescence_preflight_request(&self) -> QuiescencePreflightRequest {
        QuiescencePreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            network: self.network.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            quiescence_policy: self.quiescence_policy.clone(),
            targets: self
                .targets
                .iter()
                .map(QuiescencePreflightTarget::from)
                .collect(),
        }
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
