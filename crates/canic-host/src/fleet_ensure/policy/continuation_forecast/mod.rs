//! Module: fleet_ensure::policy::continuation_forecast
//!
//! Responsibility: project known continuation work from one immutable review.
//! Boundary: no live reads, quote completion, persistent changes or effect admission.

use crate::fleet_ensure::{
    model::{
        CanisterDisposition, CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan,
        FleetEnsurePlanScope, FleetEnsureReport,
    },
    view::continuation::{
        ContinuationAuthority, ContinuationDiscovery, ContinuationForecast, ContinuationImport,
        ContinuationImportState, DependentPoolFunding,
    },
};

/// Explain the reviewed ceiling separately from dependent imports and funding.
#[must_use]
pub fn forecast(report: &FleetEnsureReport) -> ContinuationForecast {
    let plan = &report.plan;
    let complete = report.terminal && plan.scope == FleetEnsurePlanScope::Full;
    let mut result = ContinuationForecast {
        authority: if complete {
            ContinuationAuthority::Complete
        } else if plan.continuation.is_some() {
            ContinuationAuthority::WithinReviewedProtocolBounds
        } else {
            ContinuationAuthority::SeparateReview
        },
        reviewed_actions: plan
            .canisters
            .iter()
            .map(|canister| canister.actions.len())
            .sum::<usize>()
            + plan.protocol_actions.len(),
        maximum_successor_actions: plan
            .continuation
            .as_ref()
            .map(|value| value.maximum_successor_actions),
        maximum_operator_debit_cycles: plan.conservation.maximum_operator_debit_cycles,
        imports: Vec::new(),
        dependent_funding: Vec::new(),
        requires_live_discovery: Vec::new(),
    };
    if complete {
        return result;
    }
    let awaiting_initialization = append_initialization_imports(plan, &mut result.imports);
    for action in &plan.protocol_actions {
        if let EnsureAction::FleetProtocol {
            name,
            principal,
            action,
            ..
        } = action
            && let CurrentFleetProtocolAction::ReconcilePoolAsset { request, .. } = action.as_ref()
        {
            result.imports.push(ContinuationImport {
                root: principal.clone(),
                canister: name.clone(),
                principal: Some(request.canister_id.to_text()),
                state: ContinuationImportState::ReviewedReconciliation,
            });
        }
    }
    if let Some(review) = &plan.recovery_review {
        result.dependent_funding = review
            .known_pool_funding
            .iter()
            .map(|funding| DependentPoolFunding {
                root: funding.root.clone(),
                principal: funding.principal.clone(),
                amount_cycles: funding.amount_cycles,
                ledger_fee_cycles: funding.ledger_fee_cycles,
            })
            .collect();
    }
    if awaiting_initialization
        || plan.scope != FleetEnsurePlanScope::Full
        || plan.continuation.is_some()
        || plan.recovery_review.is_some()
        || !result.imports.is_empty()
    {
        result.requires_live_discovery = vec![
            ContinuationDiscovery::PoolReadiness,
            ContinuationDiscovery::PoolFundingAndCapacity,
            ContinuationDiscovery::PublicationAndProvisioning,
        ];
    }
    result
}

fn append_initialization_imports(
    plan: &FleetEnsurePlan,
    imports: &mut Vec<ContinuationImport>,
) -> bool {
    let mut awaiting_initialization = false;
    if let Some(desired) = plan.reviewed_desired.as_ref().map(|value| value.desired()) {
        for root in desired
            .bootstrap
            .iter()
            .flat_map(|bootstrap| &bootstrap.roots)
        {
            let initializes = plan.canisters.iter().any(|canister| {
                canister.name == root.root
                    && matches!(
                        canister.disposition,
                        CanisterDisposition::Create
                            | CanisterDisposition::Replace
                            | CanisterDisposition::Reinstall
                    )
            });
            if !initializes {
                continue;
            }
            awaiting_initialization = true;
            for name in &root.canister_pool_imports {
                let principal = match plan
                    .canisters
                    .iter()
                    .find(|canister| &canister.name == name)
                {
                    Some(canister) => canister.principal.clone(),
                    None => desired
                        .canisters
                        .iter()
                        .find(|canister| &canister.name == name)
                        .and_then(|canister| canister.principal.clone()),
                };
                imports.push(ContinuationImport {
                    root: root.root.clone(),
                    canister: name.clone(),
                    principal,
                    state: ContinuationImportState::PostInitializationObservation,
                });
            }
        }
    }
    awaiting_initialization
}
