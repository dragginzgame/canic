//! Select completed-source preparation before decoding a current desired document.
//!
//! Host workflow owns evidence, approval, journalling and effects.

use crate::fleet::{EnsureOptions, FleetCommandError, quote_review_argument};
use std::path::Path;

use canic_host::{
    fleet_ensure::{
        model::completed_handoff::preparation::CompletedPreparationReviewRecord,
        ops::retained_contract, workflow::completed_preparation as workflow,
    },
    icp::IcpCli,
};

pub(super) fn run_if_selected(
    workspace: &Path,
    options: &EnsureOptions,
) -> Result<bool, FleetCommandError> {
    let Some(environment) = options.environment.as_deref() else {
        return Ok(false);
    };
    if options.operator_mint || options.observe_funding.is_some() || options.cancel_mint.is_some() {
        return Ok(false);
    }
    let retained = workflow::review(workspace, environment, &options.fleet).map_err(failure)?;
    let published = canic_host::fleet_ensure::ops::completed_handoff::committed(
        &canic_host::fleet_ensure::EnsurePaths::under(workspace, environment, &options.fleet),
    )
    .map_err(|error| failure(workflow::CompletedPreparationError::State(error)))?
    .is_some();
    if published
        && !options.apply.as_ref().is_some_and(|approval| {
            retained
                .as_ref()
                .is_some_and(|review| &review.review_sha256 == approval)
        })
    {
        return Ok(false);
    }
    let selected = if retained.is_some() {
        options.reinstall || options.apply.is_some()
    } else if options.reinstall {
        match retained_contract::check(workspace, environment, &options.fleet) {
            Err(retained_contract::RetainedContractError::CompletedAuthorityContract {
                ..
            }) => true,
            Err(error) => {
                return Err(failure(workflow::CompletedPreparationError::Source(
                    Box::new(error),
                )));
            }
            Ok(()) => false,
        }
    } else {
        false
    };
    if !selected {
        return Ok(false);
    }
    if options.retirement_debit_block.is_some() {
        return Err(FleetCommandError::Usage(
            "completed-source preparation permits no external Ledger debit".into(),
        ));
    }
    let icp = IcpCli::new(&options.icp, Some(environment.into()))
        .with_identity(options.identity.as_deref())
        .with_cwd(workspace);
    let (review, journal) = if let Some(approval) = options.apply.as_deref() {
        let review =
            retained.ok_or_else(|| failure(workflow::CompletedPreparationError::Conflict))?;
        let journal = workflow::apply(workspace, environment, &options.fleet, approval, &icp)
            .map_err(failure)?;
        (review, Some(journal))
    } else {
        (
            workflow::plan(workspace, environment, &options.fleet, &icp).map_err(failure)?,
            None,
        )
    };
    let command = apply_command(options, &review);
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "stage": "completed_source_preparation",
                "review": review,
                "journal": journal,
                "apply_command": command,
                "maximum_operator_debit_cycles": "0",
                "deployment_complete": false,
            }))?
        );
    } else {
        println!("{}", render(&review, journal.is_some(), &command));
    }
    Ok(true)
}

fn failure(error: workflow::CompletedPreparationError) -> FleetCommandError {
    FleetCommandError::CompletedPreparation(Box::new(error))
}

fn apply_command(options: &EnsureOptions, review: &CompletedPreparationReviewRecord) -> String {
    let mut command = format!(
        "canic --environment {} --icp {} fleet ensure {}",
        quote_review_argument(&review.environment),
        quote_review_argument(&options.icp),
        quote_review_argument(&review.fleet),
    );
    if let Some(identity) = options.identity.as_deref() {
        command.push_str(" --identity ");
        command.push_str(&quote_review_argument(identity));
    }
    command.push_str(" --apply ");
    command.push_str(&review.review_sha256);
    command
}

fn render(review: &CompletedPreparationReviewRecord, prepared: bool, command: &str) -> String {
    let mut lines = vec![
        format!(
            "Fleet {}: completed-source authority preparation {}",
            review.fleet,
            if prepared { "complete" } else { "review" }
        ),
        format!("Review: {}", review.review_sha256),
        format!("Source operation: {}", review.source.operation_id),
        format!(
            "Source documents: plan={} journal={} state={}",
            review.source.plan_document_sha256,
            review.source.journal_document_sha256,
            review.source.state_document_sha256
        ),
        format!(
            "Maximum operator debit: 0 cycles; maximum observed execution burn: {} cycles",
            review.maximum_execution_burn_cycles
        ),
        format!(
            "Per authority: at most {} seal submissions and {} management observations",
            review.maximum_attempts_per_action, review.maximum_observations_per_action
        ),
    ];
    for action in &review.actions {
        if let canic_host::fleet_ensure::model::EnsureAction::SealAuthority {
            name,
            principal,
            candid_sha256,
            ..
        } = action
        {
            let binding = &review.custody.canisters[name].binding;
            lines.push(format!(
                "Seal {name}: {principal}; interface={candid_sha256}; controllers={}",
                binding
                    .controllers
                    .iter()
                    .map(candid::Principal::to_text)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
    for (name, root) in &review.inspection_roots {
        let binding = &review.custody.canisters[name].binding;
        lines.push(format!(
            "Inspect {name}: {}; subnet={}; module={}; via={}; maximum attempts={}",
            binding.principal,
            binding.subnet,
            binding.module_sha256.as_deref().unwrap_or("empty"),
            root.as_deref().unwrap_or("operator management authority"),
            review.maximum_attempts_per_action,
        ));
    }
    lines.push(format!(
        "Original native baseline: {}; receipted funding: {}; original source burn ceiling: {} cycles. Reserved cycles and other Ledger accounts do not offset missing native funds.",
        review.source_accounting.initial_native_cycles,
        review.source_accounting.recorded_funding_cycles,
        review.source_accounting.maximum_source_burn_cycles,
    ));
    lines.push("Original evidence is archived unchanged. This stage seals Coordinator/Root authority; it authorizes no payments, controller handoffs, installs or wipes. Application canisters continue running. Deployment is not complete.".into());
    lines.push(format!(
        "{}: {command}",
        if prepared {
            "Effect-free preparation replay"
        } else {
            "Apply reviewed preparation"
        }
    ));
    lines.join("\n")
}
