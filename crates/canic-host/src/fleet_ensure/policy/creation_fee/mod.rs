//! Module: fleet_ensure::policy::creation_fee
//!
//! Responsibility: reject ambiguous creation-fee authority before paid effects.
//! Boundary: inspects the exact operation's direct creates and Root forecasts;
//! existing canisters on unrelated subnets do not consume creation fees.

use crate::fleet_ensure::{
    model::{CanisterPlan, DesiredFleet, EnsureAction, EstateFundingDomainPlan},
    policy::EnsurePolicyError,
};
use std::collections::BTreeSet;

/// A scalar fee can authorize creation on at most one exact target subnet.
/// Checking both compilation and retained apply keeps old reviewed documents
/// from bypassing the same restriction. No numerical fee is inferred here.
pub fn validate_creation_fee_scope(
    desired: &DesiredFleet,
    canisters: &[CanisterPlan],
    domains: &[EstateFundingDomainPlan],
) -> Result<(), EnsurePolicyError> {
    let mut subnets = BTreeSet::new();
    for action in canisters.iter().flat_map(|canister| &canister.actions) {
        if let EnsureAction::Create { subnet, .. } = action {
            subnets.insert(subnet.clone());
        }
    }
    for domain in domains {
        if domain.required_creation_count == 0 && domain.pending_creation_count == 0 {
            continue;
        }
        let root = desired
            .canisters
            .iter()
            .find(|canister| canister.name == domain.root)
            .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
                reason: format!("creation forecast has no declared Root {}", domain.root),
            })?;
        subnets.insert(root.subnet.clone());
    }
    if subnets.len() > 1 {
        return Err(EnsurePolicyError::MixedSubnetCreationFees {
            subnets: subnets.into_iter().collect(),
        });
    }
    Ok(())
}
