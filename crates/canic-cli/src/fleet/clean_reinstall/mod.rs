//! Select clean reset before any predecessor desired or execution contract is decoded.

use super::{
    EnsureOptions, FleetCommandError, now_nanoseconds, progress, quote_review_argument,
    render_report, resolve_from_root,
};
use canic_host::{
    fleet_ensure::{
        IcpEnsurePlatform, load_desired_fleet,
        workflow::clean_reinstall::{self as workflow, CleanReinstallReport},
    },
    icp::IcpCli,
};
use std::path::Path;

pub(super) fn run_if_selected(
    workspace: &Path,
    options: &EnsureOptions,
) -> Result<bool, FleetCommandError> {
    if options.operator_mint || options.observe_funding.is_some() || options.cancel_mint.is_some() {
        return Ok(false);
    }
    let environment = options.environment.as_deref().unwrap_or("local");
    if !workflow::selected(
        workspace,
        environment,
        &options.fleet,
        options.reinstall,
        options.apply.is_some(),
    )? {
        return Ok(false);
    }
    let desired = match workflow::retained_desired(
        workspace,
        environment,
        &options.fleet,
        options.reinstall,
    )? {
        Some(desired) => desired,
        None => load_desired_fleet(&resolve_from_root(workspace, &options.desired))?.desired,
    };
    if desired.environment != environment || desired.fleet != options.fleet {
        return Err(FleetCommandError::Usage("clean reinstall desired authority does not match the explicitly selected Fleet/environment".into()));
    }
    if options.retirement_debit_block.is_some() {
        return Err(FleetCommandError::Usage(
            "clean reinstall retains supplied canisters; --retirement-debit-block does not apply"
                .into(),
        ));
    }
    let report = execute(workspace, options, environment, &desired)?;
    render(report, options, environment)?;
    Ok(true)
}

fn execute(
    workspace: &Path,
    options: &EnsureOptions,
    environment: &str,
    desired: &canic_host::fleet_ensure::model::DesiredFleet,
) -> Result<CleanReinstallReport, FleetCommandError> {
    let session = progress::ProgressSession::new(options.json);
    session.retain_receipt(
        workspace,
        &progress::receipt::Invocation {
            command: progress::receipt::CommandKind::Ensure,
            fleet: &options.fleet,
            environment,
            desired_sha256: None,
            applied_plan_sha256: None,
            applied_review_sha256: options.apply.as_deref(),
            reinstall: true,
            next_review_command: "",
        },
    );
    let import_requests = session.sink();
    let icp = IcpCli::new(&options.icp, Some(environment.into()))
        .with_identity(options.identity.as_deref())
        .with_cwd(workspace)
        .with_timing_handler(move |event| import_requests.request(event));
    let effects = session.sink();
    let observations = session.sink();
    let requests = session.sink();
    let mut platform = IcpEnsurePlatform::new(desired.clone(), &options.icp, workspace)
        .with_identity(options.identity.as_deref())
        .with_progress_handler(move |event| effects.progress(event))
        .with_observation_handler(move |event| observations.observation(event))
        .with_request_timing_handler(move |event| requests.request(event));
    let result = (|| -> Result<_, FleetCommandError> {
        if let Some(digest) = &options.apply {
            Ok(workflow::apply(
                workspace,
                environment,
                &options.fleet,
                digest,
                &mut platform,
                &icp,
            )?)
        } else {
            Ok(workflow::review(
                workspace,
                desired,
                &resolve_from_root(workspace, &options.source),
                &resolve_from_root(workspace, &options.seed),
                now_nanoseconds()?,
                &mut platform,
                &icp,
            )?)
        }
    })();
    match &result {
        Ok(CleanReinstallReport::Infrastructure(report) | CleanReinstallReport::Fleet(report)) => {
            session.finish(Some(report));
        }
        Ok(CleanReinstallReport::Import(_)) => session.finish_without_report(true),
        Err(_) => session.finish_without_report(false),
    }
    drop(session);
    result
}

fn render(
    report: CleanReinstallReport,
    options: &EnsureOptions,
    environment: &str,
) -> Result<(), FleetCommandError> {
    let mut command = format!(
        "canic --environment {} --icp {} fleet ensure {}",
        quote_review_argument(environment),
        quote_review_argument(&options.icp),
        quote_review_argument(&options.fleet)
    );
    if let Some(identity) = &options.identity {
        command.push_str(" --identity ");
        command.push_str(&quote_review_argument(identity));
    }
    let (digest, complete, fleet_complete) = match &report {
        CleanReinstallReport::Infrastructure(report) => {
            (report.plan.plan_sha256.clone(), report.terminal, false)
        }
        CleanReinstallReport::Fleet(report) => (
            report.plan.plan_sha256.clone(),
            report.terminal,
            report.terminal,
        ),
        CleanReinstallReport::Import(record) => (
            canic_core::cdk::utils::hash::hex_bytes(
                record
                    .operation
                    .as_ref()
                    .expect("reviewed import operation")
                    .review
                    .review_sha256,
            ),
            canic_host::fleet_ensure::ops::capacity_import::publication::completed(record),
            false,
        ),
    };
    let next = if complete {
        format!("{command} --reinstall")
    } else {
        format!("{command} --apply {digest}")
    };
    match report {
        CleanReinstallReport::Infrastructure(report) | CleanReinstallReport::Fleet(report) => {
            if options.json {
                let mut value = canic_host::fleet_ensure::report_json_value(&report)?;
                value["stage"] = serde_json::json!(if report.plan.scope == canic_host::fleet_ensure::model::FleetEnsurePlanScope::InfrastructureBootstrap { "clean_reinstall_infrastructure" } else { "clean_reinstall_fleet" });
                value["phase_completed"] = serde_json::json!(complete);
                value["fleet_completed"] = serde_json::json!(fleet_complete);
                value["next_command"] = if fleet_complete {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(next)
                };
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                render_report(&report, false)?;
            }
        }
        CleanReinstallReport::Import(record) => {
            if options.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"schema_version":1,"stage":"clean_reinstall_import","completed":complete,"record":record,"next_command":next})
                    )?
                );
            } else {
                println!("Pool clearing review: {digest}");
                println!("{}", serde_json::to_string_pretty(&record.plan)?);
            }
        }
    }
    if !options.json && !fleet_complete {
        println!("Next: {next}");
    }
    Ok(())
}
