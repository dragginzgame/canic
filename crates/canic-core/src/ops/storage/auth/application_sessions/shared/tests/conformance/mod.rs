//! Run the delivered Auth contract against Canic's stable records and rebuilt indexes.
//! Failure injection is at the transaction boundary, not a simulated IC trap.

use super::*;
use canic_contracts::ids::{FleetId, FleetKey};
use ic_auth_backend_conformance::{Backend, assert_backend_contract};

/// Disposable scenario owner; every successful operation uses the production adapter.
struct StableBackend {
    store: ApplicationSessionStore,
    fail_next: bool,
}

impl StableBackend {
    fn before_commit(&mut self) -> Result<(), SessionError> {
        if std::mem::take(&mut self.fail_next) {
            Err(SessionError::StorageFailure)
        } else {
            Ok(())
        }
    }
}

impl SessionStore for StableBackend {
    fn read<T>(
        &mut self,
        operation: impl FnOnce(&dyn SessionReader) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        self.store.read(operation)
    }

    fn admit(
        &mut self,
        session: Session,
        replay: Replay,
        change: &LoginRevocationChange,
    ) -> Result<bool, SessionError> {
        self.before_commit()?;
        self.store.admit(session, replay, change)
    }

    fn logout(&mut self, change: &LoginRevocationChange) -> Result<(), SessionError> {
        self.before_commit()?;
        self.store.logout(change)
    }
}

impl Backend for StableBackend {
    fn fresh(generation: u64, limits: SessionLimits) -> Self {
        let data = crate::storage::stable::auth::LocalApplicationAuthorizationStateData {
            authority_generation: generation,
            ..Default::default()
        };
        LocalApplicationAuthorizationState::import(data);
        invalidate_indexes();
        // Restore the empty backing before applying deliberate byte-exhaustion bounds.
        capacity::select(2048).unwrap();
        LocalApplicationAuthorizationStateOps::restore_application_session_state(0).unwrap();
        let mut profile = capacity();
        profile.sessions = limits.max_sessions();
        profile.replays = limits.max_replays();
        profile.replays_per_caller = limits.max_replays_per_caller();
        profile.stable_bytes = limits.max_encoded_bytes();
        capacity::restore_selection(profile);
        Self {
            store: ApplicationSessionStore {
                fleet: FleetKey {
                    canonical_network_id: "01".repeat(32).parse().unwrap(),
                    fleet_id: FleetId::from_generated_bytes([2; 32]),
                },
            },
            fail_next: false,
        }
    }

    fn fail_next_commit(&mut self) {
        self.fail_next = true;
    }

    fn reopen(self) -> Self {
        let data = LocalApplicationAuthorizationState::export();
        // Import writes the actual encoded stable records; rebuilding rereads that backing.
        LocalApplicationAuthorizationState::import(data.clone());
        invalidate_indexes();
        LocalApplicationAuthorizationStateOps::restore_application_session_state(
            data.observed_at_ns,
        )
        .unwrap();
        Self {
            fail_next: false,
            ..self
        }
    }

    fn advance_generation(&mut self) {
        let mut data = LocalApplicationAuthorizationState::export();
        data.authority_generation += 1;
        LocalApplicationAuthorizationState::import(data);
    }

    fn retained_logins(&self) -> Vec<LoginRevocationChange> {
        LocalApplicationAuthorizationState::export()
            .logins
            .iter()
            .map(|(_, record)| LoginRevocationChange::decode(&record.bytes).unwrap())
            .collect()
    }
}

#[test]
fn delivered_backend_contract_uses_canic_stable_storage() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    assert_backend_contract::<StableBackend>();
}
