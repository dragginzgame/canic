//! Module: workflow::runtime::template::publication::cost_guard
//!
//! Responsibility: admit and settle durable publication quota/cycle permits.
//! Does not own: publication placement, store mutation, or public error DTOs.
//! Boundary: publication entrypoints reserve before effects and pass the permit inward.

use canic_core::{
    cdk::types::{Principal, TC},
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
            ic::{IcOps, mgmt::MgmtOps},
        },
        workflow::cost_guard::map_cost_guard_reserve_error,
    },
    log,
    log::Topic,
    replay_policy::CostClass,
};

pub(super) const PUBLICATION_ADMIN_COMMAND_KIND: &str = "wasm_store.admin.v1";
pub(super) const PUBLICATION_BOOTSTRAP_COMMAND_KIND: &str = "wasm_store.bootstrap_publish.v1";
pub(super) const PUBLICATION_RECOVERY_COMMAND_KIND: &str = "wasm_store.reconcile_publication.v1";

const DURABLE_PUBLICATION_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_DURABLE_PUBLICATION_OPERATIONS_PER_WINDOW: u64 = 10;
const DURABLE_PUBLICATION_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
const MIN_CYCLES_AFTER_DURABLE_PUBLICATION: u128 = TC;

///
/// PublicationCostGuard
///
/// Workflow-owned durable-publication permit and its settlement boundary.
///

pub(super) struct PublicationCostGuard {
    permit: CostGuardPermit,
}

impl PublicationCostGuard {
    /// Reserve durable-publication quota and cycle headroom before any effect.
    pub(super) fn reserve(command_kind: &'static str) -> Result<Self, InternalError> {
        let self_pid = IcOps::canister_self();
        let request = durable_publication_cost_guard_request(
            command_kind,
            self_pid,
            self_pid,
            IcOps::now_secs(),
            MgmtOps::canister_cycle_balance().to_u128(),
        );
        let permit = CostGuardOps::reserve(request).map_err(map_cost_guard_reserve_error)?;
        log!(
            Topic::Wasm,
            Info,
            "publication cost guard reserved command_kind={} reservation_id={} quota_subject={} payer={}",
            command_kind,
            permit.reservation_id,
            self_pid,
            self_pid
        );

        Ok(Self { permit })
    }

    /// Borrow the unforgeable permit required by publication effect adapters.
    pub(super) const fn permit(&self) -> &CostGuardPermit {
        &self.permit
    }

    /// Complete successful publication or recover quota/cycle state after failure.
    pub(super) fn settle<T>(self, result: Result<T, InternalError>) -> Result<T, InternalError> {
        match result {
            Ok(value) => {
                CostGuardOps::complete(&self.permit, IcOps::now_secs())?;
                Ok(value)
            }
            Err(err) => Err(CostGuardOps::recover_after_failure(
                &self.permit,
                IcOps::now_secs(),
                err,
            )),
        }
    }
}

fn durable_publication_cost_guard_request(
    command_kind: &'static str,
    quota_subject: Principal,
    payer: Principal,
    now_secs: u64,
    current_cycle_balance: u128,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::DurablePublish,
        command_kind: CommandKind::new(command_kind)
            .expect("durable publication command kind is a valid static label"),
        quota_subject,
        payer,
        now_secs,
        quota_window_secs: DURABLE_PUBLICATION_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_DURABLE_PUBLICATION_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: DURABLE_PUBLICATION_CYCLE_RESERVATION_CYCLES,
        min_cycles_after_reservation: MIN_CYCLES_AFTER_DURABLE_PUBLICATION,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::dto::error::ErrorCode;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn durable_publication_request_pins_quota_and_cycle_policy() {
        let request = durable_publication_cost_guard_request(
            PUBLICATION_ADMIN_COMMAND_KIND,
            p(1),
            p(2),
            17,
            100 * TC,
        );

        assert_eq!(request.cost_class, CostClass::DurablePublish);
        assert_eq!(
            request.command_kind.as_str(),
            PUBLICATION_ADMIN_COMMAND_KIND
        );
        assert_eq!(request.quota_subject, p(1));
        assert_eq!(request.payer, p(2));
        assert_eq!(request.now_secs, 17);
        assert_eq!(
            request.quota_window_secs,
            DURABLE_PUBLICATION_QUOTA_WINDOW_SECONDS
        );
        assert_eq!(
            request.max_operations_per_window,
            MAX_DURABLE_PUBLICATION_OPERATIONS_PER_WINDOW
        );
        assert_eq!(
            request.cycle_reservation_cycles,
            DURABLE_PUBLICATION_CYCLE_RESERVATION_CYCLES
        );
        assert_eq!(
            request.min_cycles_after_reservation,
            MIN_CYCLES_AFTER_DURABLE_PUBLICATION
        );
    }

    #[test]
    fn durable_publication_request_keeps_command_quotas_independent() {
        let admin = durable_publication_cost_guard_request(
            PUBLICATION_ADMIN_COMMAND_KIND,
            p(1),
            p(1),
            17,
            100 * TC,
        );
        let recovery = durable_publication_cost_guard_request(
            PUBLICATION_RECOVERY_COMMAND_KIND,
            p(1),
            p(1),
            17,
            100 * TC,
        );

        assert_ne!(admin.command_kind, recovery.command_kind);
        assert_eq!(admin.cost_class, recovery.cost_class);
    }

    #[test]
    fn durable_publication_request_rejects_insufficient_cycle_headroom() {
        let request = durable_publication_cost_guard_request(
            PUBLICATION_ADMIN_COMMAND_KIND,
            p(11),
            p(11),
            17,
            MIN_CYCLES_AFTER_DURABLE_PUBLICATION
                .saturating_add(DURABLE_PUBLICATION_CYCLE_RESERVATION_CYCLES)
                .saturating_sub(1),
        );

        let err = CostGuardOps::reserve(request)
            .map_err(map_cost_guard_reserve_error)
            .expect_err("publication must reject insufficient cycle headroom");

        assert_eq!(
            err.public_error().map(|public| public.code),
            Some(ErrorCode::ResourceExhausted)
        );
    }

    #[test]
    fn durable_publication_effect_adapters_require_permits() {
        let client = include_str!("../client/mod.rs");
        let chunks = include_str!("release/chunks.rs");
        let snapshot = include_str!("fleet/snapshot.rs");
        let creation = include_str!("lifecycle/creation.rs");
        let store = include_str!("store.rs");

        assert_eq!(
            client
                .matches("_publication_permit: &CostGuardPermit")
                .count(),
            3,
            "all store mutation client adapters must require the publication permit"
        );
        assert_eq!(
            chunks
                .matches("publication_permit: &CostGuardPermit")
                .count(),
            4,
            "chunk store and management effects must remain behind the publication permit"
        );
        assert_eq!(
            snapshot
                .matches("_publication_permit: &CostGuardPermit")
                .count(),
            1,
            "management chunk inventory must remain behind the publication permit"
        );
        assert_eq!(
            creation
                .matches("publication_permit: &CostGuardPermit")
                .count(),
            3,
            "publication-owned store creation must require the outer publication permit"
        );
        assert_eq!(
            store
                .matches("publication_permit: &CostGuardPermit")
                .count(),
            4,
            "publication store adapters must require the publication permit"
        );
    }
}
