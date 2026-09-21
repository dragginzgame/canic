//! Module: fleet_ensure::workflow::funding_observation
//!
//! Responsibility: review and consume a bounded descendant observation pass in the Ensure journal.
//! Boundary: the platform qualifies live authority; workflow persists each attempt before calling.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        funding_observation::{
            FundingChildAccountingRecord, FundingObservationAuthorityRecord,
            FundingObservationError, FundingObservationOutcomeRecord,
            FundingObservationRequestRecord, FundingObservationReviewRecord,
        },
    },
    ops::{
        self, EnsurePaths,
        funding_observation::{self as records, validation},
    },
    policy::startup_funding::relay_quote,
    view::startup_funding::StartupRootFunding,
    workflow::{EnsureWorkflowError, verified_plan, verify_journal},
};

#[cfg(test)]
pub(in crate::fleet_ensure) use tests::qualify;
pub(in crate::fleet_ensure) use validation::{total, verify};

/// Fresh read-only membership evidence. The platform must bind selected code, signer and network.
pub struct FundingObservationSnapshot {
    pub configuration_source: String,
    pub root: StartupRootFunding,
    pub authority: FundingObservationAuthorityRecord,
}

/// Adapter for one reviewed pass; snapshot never performs Root relays or trusts saved preview data.
pub trait FundingObservationPlatform {
    type Error: std::error::Error + 'static;

    /// Requalify complete current membership, selected artifacts, operator authority and native balance.
    fn snapshot(
        &mut self,
        plan: &FleetEnsurePlan,
        desired: &crate::fleet_ensure::model::DesiredFleet,
        root: &str,
    ) -> Result<FundingObservationSnapshot, Self::Error>;

    /// Inspect one child through Root and verify its installed artifact and controllers.
    fn child_balance(
        &mut self,
        plan: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
    ) -> Result<u128, Self::Error>;

    /// Read one parent ledger, using at most one Root relay, without retrying a failed call.
    fn child_usage(
        &mut self,
        plan: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
        native_cycles: u128,
    ) -> Result<FundingChildAccountingRecord, Self::Error>;
}

/// Retain one exact review per Root and operation. Existing reviews cannot replenish attempts.
pub fn review<P: FundingObservationPlatform>(
    paths: &EnsurePaths,
    root: &str,
    platform: &mut P,
) -> Result<FundingObservationReviewRecord, EnsureWorkflowError<P::Error>> {
    let _lock = ops::lock_operation(paths)?;
    let (plan, mut journal) = load::<P::Error>(paths)?;
    let desired = records::resolved(&plan, &journal)?;
    if let Some(record) = journal.funding_observations.get(root)
        && (record.approved || !record.attempts.is_empty() || journal.funding_reviews.iter().any(|review|
            matches!(&review.pause, crate::fleet_ensure::model::FundingPauseRecord::Native(pause)
                if pause.observation_quote.as_ref().is_some_and(|source| source.review_sha256 == record.review_sha256))))
    {
        return Ok(record.clone());
    }
    require_in_progress(&journal)?;
    let snapshot = platform
        .snapshot(&plan, &desired, root)
        .map_err(EnsureWorkflowError::Platform)?;
    let record = candidate(&plan, &desired, root, snapshot)?;
    validation::record(&plan, &desired, &record)?;
    if journal.funding_observations.get(root) == Some(&record) {
        return Ok(record);
    }
    records::retain(&mut journal, root, record.clone());
    ops::write_journal(paths, &journal)?;
    Ok(record)
}

/// Consume an approved single pass. Lost replies remain consumed and are never reissued.
pub fn collect<P: FundingObservationPlatform>(
    paths: &EnsurePaths,
    root: &str,
    approved_digest: &str,
    platform: &mut P,
) -> Result<FundingObservationReviewRecord, EnsureWorkflowError<P::Error>> {
    let _lock = ops::lock_operation(paths)?;
    let (plan, mut journal) = load::<P::Error>(paths)?;
    let desired = records::resolved(&plan, &journal)?;
    let record = journal
        .funding_observations
        .get_mut(root)
        .ok_or(FundingObservationError::Unavailable)?;
    if record.review_sha256 != approved_digest {
        return Err(FundingObservationError::ApprovalMismatch.into());
    }
    // A process may have stopped after the intent write, including before submission.
    // Treat the attempt as consumed in both cases; never infer non-execution.
    if record
        .attempts
        .last()
        .is_some_and(|attempt| attempt.outcome.is_none())
    {
        records::finish(record, FundingObservationOutcomeRecord::Interrupted);
        ops::write_journal(paths, &journal)?;
        return Ok(journal.funding_observations[root].clone());
    }
    loop {
        let record = &journal.funding_observations[root];
        if record.attempts.len() == record.body.requests.len() {
            return Ok(record.clone());
        }
        require_in_progress(&journal)?;
        let snapshot = platform
            .snapshot(&plan, &desired, root)
            .map_err(EnsureWorkflowError::Platform)?;
        let balance = snapshot.root.balance.cycles();
        let fresh = candidate(&plan, &desired, root, snapshot)?;
        if fresh.body != record.body {
            return Err(FundingObservationError::AuthorityMismatch.into());
        }
        let remaining = record
            .body
            .maximum_cycles
            .checked_sub(validation::consumed(record)?)
            .and_then(|value| value.checked_add(record.body.recovery_floor_cycles))
            .ok_or(FundingObservationError::ArithmeticOverflow)?;
        if balance < remaining {
            return Err(FundingObservationError::Underfunded.into());
        }
        let record = journal
            .funding_observations
            .get_mut(root)
            .ok_or(FundingObservationError::AuthorityMismatch)?;
        records::approve(record);
        let index = records::consume(record);
        let request = record.body.requests[index].clone();
        ops::write_journal(paths, &journal)?;
        let observed = platform
            .child_balance(&plan, &request)
            .and_then(|balance| platform.child_usage(&plan, &request, balance));
        let outcome = match observed {
            Ok(usage) if validation::usage(&request, &usage) => {
                FundingObservationOutcomeRecord::Observed(usage)
            }
            Ok(_) => FundingObservationOutcomeRecord::AuthorityChanged,
            Err(_) => FundingObservationOutcomeRecord::ObservationFailed,
        };
        let mut final_balance = None;
        let outcome = if matches!(outcome, FundingObservationOutcomeRecord::Observed(_)) {
            match platform.snapshot(&plan, &desired, root) {
                Ok(snapshot) => {
                    let balance = snapshot.root.balance.cycles();
                    match candidate(&plan, &desired, root, snapshot) {
                        Ok(fresh) if fresh.body == journal.funding_observations[root].body => {
                            if balance < fresh.body.recovery_floor_cycles {
                                FundingObservationOutcomeRecord::RecoveryRequired
                            } else {
                                final_balance = Some(balance);
                                outcome
                            }
                        }
                        _ => FundingObservationOutcomeRecord::AuthorityChanged,
                    }
                }
                Err(_) => FundingObservationOutcomeRecord::ObservationFailed,
            }
        } else {
            outcome
        };
        let succeeded = matches!(outcome, FundingObservationOutcomeRecord::Observed(_));
        let record = journal
            .funding_observations
            .get_mut(root)
            .ok_or(FundingObservationError::AuthorityMismatch)?;
        records::finish(record, outcome);
        records::finish_root(record, final_balance);
        ops::write_journal(paths, &journal)?;
        if !succeeded {
            return Ok(journal.funding_observations[root].clone());
        }
    }
}

fn require_in_progress(journal: &FleetEnsureJournalRecord) -> Result<(), FundingObservationError> {
    if journal.completion != FleetEnsureCompletion::InProgress {
        return Err(FundingObservationError::OperationNotInProgress);
    }
    Ok(())
}

fn load<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
) -> Result<(FleetEnsurePlan, FleetEnsureJournalRecord), EnsureWorkflowError<E>> {
    let plan = verified_plan(ops::read_plan(paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
    let journal = ops::read_journal(paths)?.ok_or(FundingObservationError::Unavailable)?;
    let state = ops::read_state(paths, &plan.fleet)?;
    verify_journal(&journal, &plan, &plan.fleet, &state)?;
    Ok((plan, journal))
}

fn candidate(
    plan: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    root: &str,
    snapshot: FundingObservationSnapshot,
) -> Result<FundingObservationReviewRecord, FundingObservationError> {
    if snapshot.root.root != root
        || snapshot.authority.components.len()
            != snapshot
                .root
                .inventory
                .map_err(|_| FundingObservationError::Unavailable)?
                .components
    {
        return Err(FundingObservationError::AuthorityMismatch);
    }
    let quote = relay_quote::project(desired, &snapshot.root)
        .map_err(|_| FundingObservationError::Unavailable)?;
    records::review(
        plan,
        root,
        snapshot.authority,
        snapshot.configuration_source,
        quote,
    )
}

/// Return the retained review and independently reconstructed recovery quote without IC calls.
pub fn status(
    paths: &EnsurePaths,
    root: &str,
) -> Result<
    crate::fleet_ensure::view::startup_funding::FundingObservationReport,
    EnsureWorkflowError<std::io::Error>,
> {
    let _lock = ops::lock_operation(paths)?;
    let (plan, journal) = load::<std::io::Error>(paths)?;
    let record = journal
        .funding_observations
        .get(root)
        .ok_or(FundingObservationError::Unavailable)?;
    Ok(
        crate::fleet_ensure::view::startup_funding::FundingObservationReport {
            review: record.clone(),
            recovery_demand: records::demand(&plan, &journal, record),
        },
    )
}
