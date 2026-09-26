//! Select fresh completed-estate reset authority before predecessor desired decoding.

use crate::fleet::{
    EnsureOptions, FleetCommandError, now_nanoseconds, progress, quote_review_argument,
    render_report,
};
use canic_host::{
    fleet_ensure::{
        load_desired_fleet,
        ops::IcpEnsurePlatform,
        workflow::{self, completed_reset as reset},
    },
    icp::IcpCli,
};
use std::path::Path;

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
    let retained = reset::review(workspace, environment, &options.fleet).map_err(failure)?;
    let applying = options.apply.as_ref().is_some_and(|approval| {
        retained
            .as_ref()
            .is_some_and(|review| &review.publication.review_sha256 == approval)
    });
    if !applying && retained.as_ref().is_some_and(|review| review.committed) {
        return Ok(false);
    }
    if !applying
        && (!options.reinstall
            || !reset::prepared(workspace, environment, &options.fleet).map_err(failure)?)
    {
        return Ok(false);
    }
    if options.retirement_debit_block.is_some() {
        return Err(FleetCommandError::Usage("completed-source reset requires its exact source operator balance; no external debit is authorized".into()));
    }
    let icp = IcpCli::new(&options.icp, Some(environment.into()))
        .with_identity(options.identity.as_deref())
        .with_cwd(workspace);
    if applying {
        let plan = reset::approve(
            workspace,
            environment,
            &options.fleet,
            options.apply.as_deref().expect("selected approval"),
            &icp,
        )
        .map_err(failure)?;
        let desired = plan
            .reviewed_desired
            .as_ref()
            .ok_or_else(|| failure(reset::CompletedResetError::Conflict))?
            .desired();
        let session = progress::ProgressSession::new(options.json);
        let progress = session.sink();
        let observations = session.sink();
        let requests = session.sink();
        let mut platform = IcpEnsurePlatform::new(desired.clone(), &options.icp, workspace)
            .with_identity(options.identity.as_deref())
            .with_progress_handler(move |event| progress.progress(event))
            .with_observation_handler(move |event| observations.observation(event))
            .with_request_timing_handler(move |event| requests.request(event));
        let report = workflow::apply(
            workspace,
            desired,
            &plan.desired_sha256,
            &options.fleet,
            &plan.plan_sha256,
            &mut platform,
        )?;
        render_report(&report, options.json)?;
    } else {
        let path = workspace.join(&options.desired);
        let loaded = load_desired_fleet(&path)?;
        if loaded.desired.environment != environment || loaded.desired.fleet != options.fleet {
            return Err(failure(reset::CompletedResetError::Conflict));
        }
        let review = reset::plan(
            workspace,
            &loaded.desired,
            &loaded.sha256,
            now_nanoseconds()?,
            &icp,
        )
        .map_err(failure)?;
        render_review(review, options, environment)?;
    }
    Ok(true)
}

fn render_review(
    review: canic_host::fleet_ensure::view::completed_reset::CompletedResetReviewView,
    options: &EnsureOptions,
    environment: &str,
) -> Result<(), FleetCommandError> {
    let command = apply_command(options, environment, &review.publication.review_sha256);
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1, "stage": "completed_source_reset", "review": review,
                "apply_command": command, "deployment_complete": false,
            }))?
        );
    } else {
        println!(
            "Completed-estate reset review: {}",
            review.publication.review_sha256
        );
        render_report(
            &canic_host::fleet_ensure::model::FleetEnsureReport {
                actual_conservation: None,
                effects_applied: 0,
                funding_review: None,
                plan: review.plan,
                terminal: false,
            },
            false,
        )?;
        println!(
            "Source evidence stays archived unchanged. This review authorizes current typed infrastructure reinstalls and Root-owned clearing of every retained pool/application canister."
        );
        println!("Apply reviewed reset: {command}");
    }
    Ok(())
}

fn apply_command(options: &EnsureOptions, environment: &str, digest: &str) -> String {
    let identity = options
        .identity
        .as_deref()
        .map_or_else(String::new, |identity| {
            format!(" --identity {}", quote_review_argument(identity))
        });
    format!(
        "canic --environment {} --icp {} fleet ensure {}{identity} --apply {digest}",
        quote_review_argument(environment),
        quote_review_argument(&options.icp),
        quote_review_argument(&options.fleet),
    )
}

fn failure(error: reset::CompletedResetError) -> FleetCommandError {
    FleetCommandError::CompletedReset(Box::new(error))
}
