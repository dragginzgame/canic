//! Select clean reset before any predecessor desired or execution contract is decoded.

use super::{
    EnsureOptions, FleetCommandError, automation, now_nanoseconds, progress, quote_review_argument,
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
    if let Some(digest) = &options.cancel_reinstall {
        let environment = options.environment.as_deref().unwrap_or("local");
        let record = workflow::cancel_review(workspace, environment, &options.fleet, digest)?;
        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version": 1, "stage": "clean_reinstall_cancelled", "cancelled": true,
                    "record": record, "payment_authorized": false, "deployment_authorized": false
                }))?
            );
        } else {
            println!(
                "Cancelled unpaid reinstall review {}; archive {}. Generate current desired state from complete physical inventory, then review fleet ensure --reinstall.",
                record.plan_sha256, record.archive_sha256
            );
        }
        return Ok(true);
    }
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
    let applied_plan = match options.apply.as_deref() {
        Some(digest) => {
            canic_host::fleet_ensure::workflow::infrastructure_bootstrap::retained(
                workspace,
                environment,
                &options.fleet,
                digest,
            )
            .is_ok()
                || canic_host::fleet_ensure::ops::read_plan(
                    &canic_host::fleet_ensure::ops::EnsurePaths::under(
                        workspace,
                        environment,
                        &options.fleet,
                    ),
                )?
                .is_some_and(|plan| plan.plan_sha256 == digest)
        }
        None => false,
    };
    let session = progress::ProgressSession::new(options.json);
    session.retain_receipt(
        workspace,
        &progress::receipt::Invocation {
            command: progress::receipt::CommandKind::Ensure,
            fleet: &options.fleet,
            environment,
            desired_sha256: None,
            applied_plan_sha256: options.apply.as_deref().filter(|_| applied_plan),
            applied_review_sha256: options.apply.as_deref().filter(|_| !applied_plan),
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
    if let Ok(report) = &result {
        record_authority(&session, report);
    }
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

fn record_authority(session: &progress::ProgressSession, phase: &CleanReinstallReport) {
    let authority = match phase {
        CleanReinstallReport::Infrastructure(report) | CleanReinstallReport::Fleet(report) => {
            let fleet = matches!(phase, CleanReinstallReport::Fleet(_));
            serde_json::json!({
                "phase": if fleet { "fleet" } else { "infrastructure" },
                "desired_sha256": report.plan.desired_sha256,
                "plan_sha256": report.plan.plan_sha256,
                "operation_id": report.plan.operation_id,
                "review_sha256": null,
                "phase_completed": report.terminal,
                "fleet_completed": fleet && report.terminal,
            })
        }
        CleanReinstallReport::Import(record) => serde_json::json!({
            "phase": "import",
            "desired_sha256": null,
            "plan_sha256": null,
            "operation_id": null,
            "review_sha256": record.operation.as_ref().map(|operation| canic_core::cdk::utils::hash::hex_bytes(operation.review.review_sha256)),
            "phase_completed": canic_host::fleet_ensure::ops::capacity_import::publication::completed(record),
            "fleet_completed": false,
        }),
    };
    session
        .sink()
        .record("clean_reinstall_authority", &authority);
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
                value["automation"] =
                    serde_json::to_value(automation::ensure(&report, options, true))?;
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
                        &serde_json::json!({"schema_version":1,"stage":"clean_reinstall_import","completed":complete,"record":record,"next_command":next,
                            "automation": automation::import(digest, complete, options.apply.is_some(),
                                if complete { automation::reinstall_review_command(options, environment) } else { automation::ensure_command(options, environment) }, true)})
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
