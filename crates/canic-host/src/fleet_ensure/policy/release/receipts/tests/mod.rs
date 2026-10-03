//! Accounting cannot resolve an uncertain external effect or invalidate a terminal response.

use super::*;
use crate::fleet_ensure::view::release::receipts::{ReleaseAccountingFacts, ReleaseIntentFact};

#[test]
fn every_replay_phase_keeps_its_recovery_owner_across_all_accounting_states() {
    let phases = [
        (
            ReleaseReplayState::Committed,
            ReleaseReplayDisposition::RecordedCompletion,
        ),
        (
            ReleaseReplayState::Reserved,
            ReleaseReplayDisposition::ReservedOperation,
        ),
        (
            ReleaseReplayState::ExternalEffectInFlight,
            ReleaseReplayDisposition::EffectReconciliation,
        ),
        (
            ReleaseReplayState::ExternalEffectStatusUnknown,
            ReleaseReplayDisposition::EffectReconciliation,
        ),
        (
            ReleaseReplayState::ComponentChildLifecycleInterrupted,
            ReleaseReplayDisposition::ChildLifecycleRecovery,
        ),
        (
            ReleaseReplayState::ResponseCommitFailed,
            ReleaseReplayDisposition::ResponseRecovery,
        ),
        (
            ReleaseReplayState::CostSettlementFailed,
            ReleaseReplayDisposition::AccountingRecovery,
        ),
    ];
    let states = [
        ReleaseIntentState::Missing,
        ReleaseIntentState::Pending,
        ReleaseIntentState::Committed,
        ReleaseIntentState::Aborted,
    ];
    for (status, disposition) in phases {
        let mut facts = ReleaseReplayFacts {
            slot: [1; 32],
            operation_id: [2; 32],
            status,
            accounting: None,
        };
        let absent = assess_receipt(&facts);
        assert_eq!(absent.disposition, disposition);
        assert!(absent.pending_intents.is_empty());
        assert!(absent.missing_intents.is_empty());
        for quota in states {
            for reservation in states {
                facts.accounting = Some(ReleaseAccountingFacts {
                    quota: ReleaseIntentFact {
                        intent_id: 17,
                        state: quota,
                    },
                    reservation: ReleaseIntentFact {
                        intent_id: 29,
                        state: reservation,
                    },
                });
                let assessed = assess_receipt(&facts);
                assert_eq!(assessed.disposition, disposition);
                assert_eq!(assessed.slot, facts.slot);
                assert_eq!(assessed.operation_id, facts.operation_id);
                for (id, state) in [(17, quota), (29, reservation)] {
                    assert_eq!(
                        assessed.pending_intents.contains(&id),
                        state == ReleaseIntentState::Pending
                    );
                    assert_eq!(
                        assessed.missing_intents.contains(&id),
                        state == ReleaseIntentState::Missing
                    );
                }
            }
        }
    }
}
