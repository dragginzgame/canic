//! Stable keys shared by store opens and the complete component request factory.

macro_rules! keys {
    ($( $(#[$attr:meta])* $name:ident = $key:literal; )*) => {
        $( $(#[$attr])* pub const $name: &str = $key; )*
        pub(super) fn requests() -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
            Ok(vec![$( $(#[$attr])* ic_memory::MemoryRequest::new(super::MEMORY_AUTHORITY, $name)?, )*])
        }
    };
}

keys! {
    APPLICATION_RECEIPT_ELIGIBILITY_V1 = "canic.core.application_receipt.eligibility.v1";
    ASYNC_JOB_RECOVERY_V1 = "canic.core.async_job_recovery.v1";
    #[cfg(any(test, feature = "auth-delegated-token-issuer-state"))]
    AUTH_DELEGATED_TOKEN_ISSUER_STATE_V1 = "canic.core.auth.delegated_token_issuer.state.v1";
    AUTH_LOCAL_APPLICATION_AUTHORIZATION_REPLAYS_V1 = "canic.core.auth.local_application_authorization.replays.v1";
    AUTH_LOCAL_APPLICATION_AUTHORIZATION_LOGINS_V1 = "canic.core.auth.local_application_authorization.logins.v1";
    AUTH_LOCAL_APPLICATION_AUTHORIZATION_SESSIONS_V1 = "canic.core.auth.local_application_authorization.sessions.v1";
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    AUTH_LOCAL_APPLICATION_AUTHORIZATION_STATE_V1 = "canic.core.auth.local_application_authorization.state.v1";
    #[cfg(any(test, feature = "auth-root-delegation-state"))]
    AUTH_ROOT_DELEGATION_STATE_V1 = "canic.core.auth.root_delegation.state.v1";
    AUTHORITY_RESTORE_FENCE_V1 = "canic.core.authority_restore.fence.v1";
    CALLER_AUTHORITY_HEADER_V1 = "canic.core.caller_authority.header.v1";
    CALLER_AUTHORITY_ROWS_V1 = "canic.core.caller_authority.rows.v1";
    CYCLES_FUNDING_LEDGER_V1 = "canic.core.cycles.funding_ledger.v1";
    CYCLES_ICP_REFILL_RECORDS_V1 = "canic.core.cycles.icp_refill_records.v1";
    CYCLES_TOPUP_EVENTS_V1 = "canic.core.cycles.topup_events.v1";
    CYCLES_TRACKER_V1 = "canic.core.cycles.tracker.v1";
    FLEET_ACTIVATION_V1 = "canic.core.fleet.activation.v1";
    FLEET_ADMISSION_PROJECTION_V1 = "canic.core.fleet_admission.projection.v1";
    FLEET_STATE_V1 = "canic.core.fleet.state.v1";
    INTENT_EXPIRY_INDEX_V1 = "canic.core.intent.expiry_index.v1";
    INTENT_META_V1 = "canic.core.intent.meta.v1";
    INTENT_PENDING_V1 = "canic.core.intent.pending.v1";
    INTENT_RECEIPT_BACKED_RECORDS_V1 = "canic.core.intent.receipt_backed_records.v1";
    INTENT_RECORDS_V1 = "canic.core.intent.records.v1";
    INTENT_TOTALS_V1 = "canic.core.intent.totals.v1";
    PLACEMENT_ACKNOWLEDGEMENT_INDEX_V1 = "canic.core.placement.acknowledgement_index.v1";
    PLACEMENT_INDEX_REGISTRY_V1 = "canic.core.placement.index_registry.v1";
    PLACEMENT_SCALING_REGISTRY_V1 = "canic.core.placement.scaling_registry.v1";
    REPLAY_RECEIPTS_V1 = "canic.core.replay.receipts.v1";
    RUNTIME_BINDINGS_V1 = "canic.core.runtime.bindings.v1";
    RUNTIME_CANISTER_CHILDREN_V1 = "canic.core.runtime.canister_children.v1";
    #[cfg(feature = "sharding")]
    SHARDING_ASSIGNMENTS_V1 = "canic.core.sharding.assignments.v1";
    #[cfg(feature = "sharding")]
    SHARDING_REGISTRY_V1 = "canic.core.sharding.registry.v1";
}
