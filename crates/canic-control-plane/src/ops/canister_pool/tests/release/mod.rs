//! Pool release observation remains available during retained work and leaves it unchanged.

use super::*;
use crate::storage::stable::canister_pool::capacity_import::PoolImportBootstrapRecord;
use canic_contracts::dto::pool::CanisterPoolCreationProgress;

#[test]
fn release_observes_uncertain_and_blocked_creation_without_readmission() {
    CanisterPoolStore::clear();
    let authority = creation_authority_for([9; 32]);
    CanisterPoolOps::begin_creation(&config(), authority.clone(), 10).unwrap();
    for progress in [
        CanisterPoolCreationProgressRecord::Intent {
            uncertain_result: true,
        },
        CanisterPoolCreationProgressRecord::Blocked {
            failure: CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow,
        },
        CanisterPoolCreationProgressRecord::Created {
            block_index: 77,
            canister_id: principal(50),
        },
    ] {
        let mut state = CanisterPoolStore::state();
        state.creation.as_mut().unwrap().progress = progress;
        CanisterPoolStore::set_state(state);
        let before = CanisterPoolStore::state();
        let observed = CanisterPoolOps::release_status(authority.root).unwrap();
        let creation = observed.creation.as_ref().unwrap();
        assert_eq!(creation.operation_id, authority.operation_id);
        assert_eq!(creation.ledger_amount, authority.ledger_amount);
        assert_eq!(creation.ledger_fee, authority.ledger_fee);
        assert_eq!(creation.created_at_time_ns, authority.created_at_time_ns);
        match progress {
            CanisterPoolCreationProgressRecord::Intent { .. } => assert!(matches!(
                creation.progress,
                CanisterPoolCreationProgress::Intent {
                    uncertain_result: true
                }
            )),
            CanisterPoolCreationProgressRecord::Blocked { .. } => assert!(matches!(
                creation.progress,
                CanisterPoolCreationProgress::Blocked {
                    failure: CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow
                }
            )),
            CanisterPoolCreationProgressRecord::Created { .. } => assert!(matches!(
                creation.progress,
                CanisterPoolCreationProgress::Created {
                    block_index: 77,
                    ..
                }
            )),
            CanisterPoolCreationProgressRecord::WaitingForFunding { .. } => unreachable!(),
        }
        let bytes = candid::encode_one(&observed).unwrap();
        assert_eq!(
            candid::decode_one::<crate::dto::root::RootPoolReleaseResponse>(&bytes).unwrap(),
            observed
        );
        assert_eq!(CanisterPoolStore::state(), before);
        assert!(CanisterPoolOps::release_status(principal(99)).is_err());
        assert_eq!(CanisterPoolStore::state(), before);
    }
}

#[test]
fn release_keeps_bootstrap_store_and_sources_with_pending_handoff() {
    CanisterPoolStore::clear();
    let mut state = CanisterPoolStore::state();
    state.bootstrap_import = Some(PoolImportBootstrapRecord {
        review_sha256: [1; 32],
        install_id: [2; 32],
        operator: principal(1),
        store: principal(2),
        sources: vec![principal(3), principal(4)],
    });
    state.handoff = Some(CanisterPoolHandoffRecord {
        canister_id: principal(5),
        recipient: principal(6),
        prepared_at_ns: 99,
    });
    CanisterPoolStore::set_state(state.clone());
    let observed = CanisterPoolOps::release_status(principal(7)).unwrap();
    let hold = observed.bootstrap.as_ref().unwrap();
    assert_eq!(hold.install_id, [2; 32]);
    assert_eq!(hold.store, principal(2));
    assert_eq!(hold.sources, [principal(3), principal(4)]);
    assert_eq!(observed.handoff.as_ref().unwrap().recipient, principal(6));
    assert_eq!(observed.handoff.as_ref().unwrap().prepared_at_ns, 99);
    assert_eq!(
        CanisterPoolOps::release_status(principal(7)).unwrap(),
        observed
    );
    assert_eq!(CanisterPoolStore::state(), state);
}
