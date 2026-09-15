//! Module: fleet_ensure::ops::funding
//!
//! Responsibility: construct and identify exact supplemental funding records.
//! Boundary: no planning decisions or remote effects.

use crate::fleet_ensure::{
    model::{
        EffectRecord, EffectState, EnsureAction, EstateFundingRequiredRecord, FleetEnsurePlan,
        FundingPauseRecord, FundingReviewRecord, NativeFundingRequiredRecord,
    },
    ops::action_sha256,
};
use canic_core::cdk::utils::hash::sha256_hex;

pub(in crate::fleet_ensure) fn review(
    pause: &EstateFundingRequiredRecord,
    created_at_time: u64,
) -> FundingReviewRecord {
    let mut record = FundingReviewRecord {
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
        pause: FundingPauseRecord::Estate(pause.clone()),
        review_sha256: String::new(),
    };
    record.review_sha256 = digest(&record);
    record
}

/// Validated native quote inputs mapped into the durable review by ops.
pub(in crate::fleet_ensure) struct NativeFundingQuote<'a> {
    pub plan: &'a FleetEnsurePlan,
    pub root: &'a str,
    pub root_principal: &'a str,
    pub provisioning_action_sha256: String,
    pub available_cycles: u128,
    pub minimum_cycles: u128,
    pub funding_margin_cycles: u128,
    pub cycles_ledger: &'a str,
    pub ledger_fee_cycles: u128,
    pub shortfall_cycles: u128,
}

pub(in crate::fleet_ensure) fn native_pause(
    quote: NativeFundingQuote<'_>,
) -> NativeFundingRequiredRecord {
    NativeFundingRequiredRecord {
        available_cycles: quote.available_cycles,
        cycles_ledger: quote.cycles_ledger.into(),
        funding_margin_cycles: quote.funding_margin_cycles,
        ledger_fee_cycles: quote.ledger_fee_cycles,
        minimum_cycles: quote.minimum_cycles,
        operation_id: quote.plan.operation_id.clone(),
        plan_sha256: quote.plan.plan_sha256.clone(),
        provisioning_action_sha256: quote.provisioning_action_sha256,
        root: quote.root.into(),
        root_principal: quote.root_principal.into(),
        shortfall_cycles: quote.shortfall_cycles,
    }
}

pub(in crate::fleet_ensure) fn native_review(
    pause: NativeFundingRequiredRecord,
    created_at_time: u64,
) -> FundingReviewRecord {
    let mut record = FundingReviewRecord {
        action: EnsureAction::Fund {
            pool_funding: None,
            amount: pause.shortfall_cycles,
            created_at_time,
            expected_post_cycles: pause.minimum_cycles + pause.funding_margin_cycles,
            funding_deficit_cycles: pause.minimum_cycles - pause.available_cycles,
            funding_margin_cycles: pause.funding_margin_cycles,
            ledger: pause.cycles_ledger.clone(),
            name: pause.root.clone(),
            principal: pause.root_principal.clone(),
        },
        effect: None,
        pause: FundingPauseRecord::Native(pause),
        review_sha256: String::new(),
    };
    record.review_sha256 = digest(&record);
    record
}

fn digest(record: &FundingReviewRecord) -> String {
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
        publication_attempts: 0,
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
