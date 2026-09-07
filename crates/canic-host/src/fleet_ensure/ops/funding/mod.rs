//! Module: fleet_ensure::ops::funding
//!
//! Responsibility: construct and identify exact supplemental funding records.
//! Boundary: no planning decisions or remote effects.

use crate::fleet_ensure::{
    model::{
        EffectRecord, EffectState, EnsureAction, EstateFundingRequiredRecord,
        EstateFundingReviewRecord,
    },
    ops::action_sha256,
};
use canic_core::cdk::utils::hash::sha256_hex;

pub(in crate::fleet_ensure) fn review(
    pause: &EstateFundingRequiredRecord,
    created_at_time: u64,
) -> EstateFundingReviewRecord {
    let mut record = EstateFundingReviewRecord {
        action: EnsureAction::FundEstate {
            amount: pause.shortfall_cycles,
            created_at_time,
            expected_post_cycles: pause.maximum_creation_debit_cycles,
            ledger: pause.cycles_ledger.clone(),
            ledger_fee_cycles: pause.ledger_fee_cycles,
            name: pause.root.clone(),
            principal: pause.root_principal.clone(),
        },
        effect: None,
        pause: pause.clone(),
        review_sha256: String::new(),
    };
    record.review_sha256 = digest(&record);
    record
}

fn digest(record: &EstateFundingReviewRecord) -> String {
    // Approval and receipt changes must not change the immutable transfer identity.
    sha256_hex(
        &serde_json::to_vec(&(&record.action, &record.pause))
            .expect("funding review contains only serializable boundary data"),
    )
}

pub(in crate::fleet_ensure) fn intent(
    action: &EnsureAction,
    source: u128,
    destination: u128,
) -> EffectRecord {
    EffectRecord {
        maintenance_attempts: 0,
        action_sha256: action_sha256(action),
        created_principal: None,
        destination_post_cycles: None,
        destination_pre_cycles: Some(destination),
        post_cycles: None,
        pre_cycles: Some(source),
        pre_canister_version: None,
        progress_identity: None,
        receipt: None,
        state: EffectState::Intent,
    }
}
