use crate::{
    InternalError,
    ops::{
        ic::{IcOps, mgmt::CanisterInstallMode},
        runtime::install_source::ApprovedModuleSource,
        runtime::metrics::{
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            },
            provisioning::{
                ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            },
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::{
        ic::provision::{
            ProvisionWorkflow,
            metrics::{
                record_canister_op, record_canister_op_failure, record_provisioning,
                record_provisioning_failure,
            },
            policy::validate_registration_policy,
        },
        prelude::*,
        runtime::install::ModuleInstallWorkflow,
    },
};

/// Install WASM and initial state into a new canister.
pub(super) async fn install_canister(
    pid: Principal,
    role: &CanisterRole,
    parent_pid: Principal,
    module_source: &ApprovedModuleSource,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::Install,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    record_canister_op(
        role,
        CanisterOpsMetricOperation::Install,
        CanisterOpsMetricOutcome::Started,
        CanisterOpsMetricReason::Ok,
    );

    let payload = match ProvisionWorkflow::build_nonroot_init_payload(role, parent_pid) {
        Ok(payload) => payload,
        Err(err) => {
            record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
            record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);
            return Err(err);
        }
    };
    let module_hash = module_source.module_hash().to_vec();

    // Register before install so init hooks can observe the registry; roll back on failure.
    if let Err(err) = validate_registration_policy(role, parent_pid) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Install,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Install,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::Topology,
        );
        return Err(err);
    }

    let created_at = IcOps::now_secs();
    if let Err(err) = SubnetRegistryOps::register_unchecked(
        pid,
        role,
        parent_pid,
        module_hash.clone(),
        created_at,
    ) {
        record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
        record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);
        return Err(err);
    }

    if let Err(err) = ModuleInstallWorkflow::install_with_payload(
        CanisterInstallMode::Install,
        pid,
        module_source,
        payload,
        extra_arg,
    )
    .await
    {
        record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
        record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);

        let removed = SubnetRegistryOps::remove(&pid);
        if removed.is_none() {
            log!(
                Topic::CanisterLifecycle,
                Warn,
                "⚠️ install_canister rollback: {pid} missing from registry after failed install"
            );
        }

        return Err(err);
    }

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ install_canister: {pid} ({role}, source={}, size={}, chunks={})",
        module_source.source_label(),
        module_source.payload_size(),
        module_source.chunk_count(),
    );

    record_canister_op(
        role,
        CanisterOpsMetricOperation::Install,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::Ok,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Install,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );

    Ok(())
}
