//! Persist shared login pruning atomically without owning retention policy.

use super::*;
use crate::storage::stable::auth::LocalApplicationLoginTotalsRecord;
use ic_auth::portable::{LoginRevocationError, MemoryLoginRevocations};

impl ApplicationSessionStore {
    /// Prune only the exact retained ledger. Shared Auth selects expired callers;
    /// Canic commits counts and rows before the ledger publishes its heap changes.
    pub fn prune_logins(
        &self,
        ledger: &mut MemoryLoginRevocations,
        now_ns: u64,
        budget: usize,
    ) -> Result<usize, LoginRevocationError> {
        let (mut state, _) = prepare_maintenance_state(now_ns.max(ledger.observed_at_ns()))
            .map_err(|error| match error {
                ApplicationSessionStateError::ClockRollback => LoginRevocationError::Clock,
                _ => LoginRevocationError::Storage,
            })?;
        let records = ledger
            .records()
            .map(|record| {
                prepare_login(self, &record).map_err(|_| LoginRevocationError::Storage)?;
                Ok((Principal::from(record.caller()), record.encode()?))
            })
            .collect::<Result<BTreeMap<_, _>, LoginRevocationError>>()?;
        if ledger.occupancy() != (state.login_totals.admitted, state.login_totals.unknown)
            || records.len() != state.logins.len()
            || state
                .logins
                .iter()
                .any(|(caller, record)| records.get(caller) != Some(&record.bytes))
        {
            return Err(LoginRevocationError::Storage);
        }
        ledger.prune_with_commit(now_ns, budget, |callers, (admitted, unknown)| {
            let callers = callers
                .iter()
                .copied()
                .map(Principal::from)
                .collect::<BTreeSet<_>>();
            state.logins.retain(|(caller, _)| !callers.contains(caller));
            state.login_totals = LocalApplicationLoginTotalsRecord { admitted, unknown };
            // Validation refuses orphaning a session: the existing session cleanup
            // must finish its bounded work before its reservation can be removed.
            commit_validated_state(state).map_err(|_| LoginRevocationError::Storage)
        })
    }
}
