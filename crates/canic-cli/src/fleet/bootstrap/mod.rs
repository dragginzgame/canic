//! Review supplied infrastructure and publish its receipted physical identity for later import.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            parse_matches, render_usage, required_string, string_option, string_option_or_else,
            value_arg,
        },
        defaults::default_icp,
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    fleet::{
        FleetCommandError, identity_arg, now_nanoseconds, parse_digest, quote_review_argument,
        resolve_from_root,
    },
    version_text,
};
use canic_core::ids::ReleaseBuildId;
use canic_host::{
    fleet_ensure::{
        FleetEnsureReport, FleetGenerateRequest, IcpEnsurePlatform,
        dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest,
        generate_infrastructure_bootstrap,
        model::infrastructure_bootstrap::BootstrapCoordinatorSelection,
        workflow::infrastructure_bootstrap,
    },
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
};
use ic_host_fs::read::read_file_no_follow;

use clap::{ArgAction, ArgMatches, Command};
use std::{ffi::OsString, path::PathBuf};

pub(super) fn command() -> Command {
    let mut command = Command::new("bootstrap").bin_name("canic fleet bootstrap")
        .about("Review or apply explicit supplied infrastructure before Root-local pool import")
        .disable_help_flag(true)
        .arg(value_arg("fleet").required(true).help("Fleet identity"))
        .arg(value_arg("apply").long("apply").value_name("PLAN_SHA256").value_parser(parse_digest)
            .help("Resume exact retained initialization and estate publication"))
        .arg(value_arg("recover").long("recover").value_name("PLAN_SHA256").value_parser(parse_digest)
            .conflicts_with("apply").help("Review registration budget and funding after infrastructure effects complete"))
        .arg(value_arg("approve-recovery").long("approve-recovery").value_name("REVIEW_SHA256").value_parser(parse_digest)
            .requires("recover").help("Approve the exact registration recovery review and resume the retained operation"))
        .arg(value_arg("coordinator").long("coordinator").value_parser(["create", "initialize", "ready"])
            .required_unless_present_any(["apply", "recover"]).conflicts_with_all(["apply", "recover"])
            .help("Explicitly create a Coordinator, wipe a supplied ID, or verify current genesis authority"))
        .arg(value_arg("release-build").long("release-build").value_name("RELEASE_BUILD_ID")
            .value_parser(clap::value_parser!(ReleaseBuildId)).required_unless_present_any(["apply", "recover"]).conflicts_with_all(["apply", "recover"])
            .help("Finalized current release build"))
        .arg(value_arg("source").long("source").value_name("PATH").conflicts_with_all(["apply", "recover"])
            .help("Fleet policy; defaults to deployments/<fleet>.toml"))
        .arg(value_arg("seed").long("seed").value_name("PATH").conflicts_with_all(["apply", "recover"])
            .help("Explicit identity seed; defaults to deployments/<fleet>.estate.toml"))
        .arg(value_arg("json").long("json").num_args(0).action(ArgAction::SetTrue)
            .help("Print the complete reviewed plan and exact next command as JSON"))
        .arg(internal_environment_arg()).arg(internal_icp_arg()).arg(identity_arg());
    for (name, help) in [
        (
            "app-config",
            "Canonical App canic.toml for current typed authority",
        ),
        (
            "declarations",
            "Original supplied-ID custody and destructive disposition TOML",
        ),
    ] {
        command = command.arg(
            value_arg(name)
                .long(name)
                .value_name("PATH")
                .required_unless_present_any(["apply", "recover"])
                .conflicts_with_all(["apply", "recover"])
                .help(help),
        );
    }
    command.after_help("Examples:\n  canic --environment staging fleet bootstrap staging --app-config canic.toml --release-build <sha256> --coordinator initialize --declarations deployments/bootstrap.toml\n  canic --environment staging fleet bootstrap staging --apply <plan-sha256>\n\nReview retains bounded management observations. Apply clears reviewed infrastructure code/state, preserves supplied IDs and publishes the estate seed. Import held capacity through fleet import before generating ordinary workload authority. Seed coordinator = \"create\" is accepted only with explicit --coordinator create.")
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), FleetCommandError> {
    if print_help_or_version(&args, || render_usage(command), version_text()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)
        .map_err(|error| FleetCommandError::Usage(error.to_string()))?;
    execute(&matches)
}

fn execute(matches: &ArgMatches) -> Result<(), FleetCommandError> {
    let environment = string_option(matches, "environment")
        .ok_or_else(|| FleetCommandError::Usage("fleet bootstrap requires --environment".into()))?;
    let fleet = required_string(matches, "fleet");
    let executable = string_option_or_else(matches, "icp", default_icp);
    let identity = string_option(matches, "identity");
    let workspace = resolve_current_canic_icp_root()?;
    if let Some(digest) = string_option(matches, "recover") {
        return recover(
            matches,
            &workspace,
            &environment,
            &fleet,
            &digest,
            &executable,
            identity.as_deref(),
        );
    }
    let (plan, completed) = if let Some(digest) = string_option(matches, "apply") {
        let plan = infrastructure_bootstrap::retained(&workspace, &environment, &fleet, &digest)
            .map_err(|error| FleetCommandError::InfrastructureBootstrap(Box::new(error)))?;
        let desired = plan
            .reviewed_desired
            .as_ref()
            .expect("verified bootstrap desired")
            .desired()
            .clone();
        let mut platform = IcpEnsurePlatform::new(desired, &executable, &workspace)
            .with_identity(identity.as_deref());
        let published = infrastructure_bootstrap::apply(
            &workspace,
            &environment,
            &fleet,
            &digest,
            &mut platform,
        )?;
        (published.plan, published.completed)
    } else {
        (
            review(
                matches,
                &workspace,
                &environment,
                &fleet,
                &executable,
                identity.as_deref(),
            )?,
            false,
        )
    };
    let mut next = format!(
        "canic --environment {} --icp {} fleet bootstrap {}",
        quote_review_argument(&environment),
        quote_review_argument(&executable),
        quote_review_argument(&fleet)
    );
    if let Some(identity) = &identity {
        next.push_str(" --identity ");
        next.push_str(&quote_review_argument(identity));
    }
    next.push_str(" --apply ");
    next.push_str(&plan.plan_sha256);
    render(plan, completed, matches, &next)
}

fn recover(
    matches: &ArgMatches,
    workspace: &std::path::Path,
    environment: &str,
    fleet: &str,
    digest: &str,
    executable: &str,
    identity: Option<&str>,
) -> Result<(), FleetCommandError> {
    let plan = infrastructure_bootstrap::retained(workspace, environment, fleet, digest)
        .map_err(|error| FleetCommandError::InfrastructureBootstrap(Box::new(error)))?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .expect("verified bootstrap desired")
        .desired()
        .clone();
    let mut platform =
        IcpEnsurePlatform::new(desired, executable, workspace).with_identity(identity);
    let mut command = format!(
        "canic --environment {} --icp {} fleet bootstrap {} --recover {}",
        quote_review_argument(environment),
        quote_review_argument(executable),
        quote_review_argument(fleet),
        digest
    );
    if let Some(identity) = identity {
        command.push_str(" --identity ");
        command.push_str(&quote_review_argument(identity));
    }
    if let Some(approved) = string_option(matches, "approve-recovery") {
        infrastructure_bootstrap::registration_recovery::approve(
            workspace,
            environment,
            fleet,
            digest,
            &approved,
            &mut platform,
        )?;
        let published =
            infrastructure_bootstrap::apply(workspace, environment, fleet, digest, &mut platform)?;
        command.push_str(" --approve-recovery ");
        command.push_str(&approved);
        return render(published.plan, published.completed, matches, &command);
    }
    let review = infrastructure_bootstrap::registration_recovery::review(
        workspace,
        environment,
        fleet,
        digest,
        now_nanoseconds()?,
        &mut platform,
    )?;
    command.push_str(" --approve-recovery ");
    command.push_str(&review.review_sha256);
    if matches.get_flag("json") {
        let mut args =
            super::automation::command(environment, executable, identity, "bootstrap", fleet);
        args.extend([
            "--recover".into(),
            digest.into(),
            "--approve-recovery".into(),
            review.review_sha256.clone(),
        ]);
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1, "stage": "infrastructure_registration_recovery", "completed": false,
                "registration_recovery": canic_host::fleet_ensure::registration_recovery_json_value(&review)?, "apply_command": command,
                "automation": super::automation::bootstrap(&plan, false, Some(review.review_sha256.clone()), false, args),
            }))?
        );
    } else {
        println!("retained_plan_sha256: {}", review.plan_sha256);
        println!("recovery_review_sha256: {}", review.review_sha256);
        println!("registration_actions: {}", review.protocol_actions.len());
        println!(
            "registration_execution_burn_cycles: {}",
            review.successor_burn_cycles
        );
        println!(
            "recovery_execution_burn_cycles: {}",
            review.recovery_burn_cycles
        );
        println!(
            "maximum_execution_burn_cycles: {}",
            review.maximum_execution_burn_cycles
        );
        println!(
            "additional_funding_cycles: {}",
            review.additional_funding_cycles
        );
        println!(
            "additional_ledger_fee_cycles: {}",
            review.additional_ledger_fee_cycles
        );
        for action in &review.funding_actions {
            println!("funding: {}", serde_json::to_string(action)?);
        }
        println!("apply: {command}");
    }
    Ok(())
}

fn render(
    plan: canic_host::fleet_ensure::model::FleetEnsurePlan,
    completed: bool,
    matches: &ArgMatches,
    next: &str,
) -> Result<(), FleetCommandError> {
    let report = FleetEnsureReport {
        plan,
        terminal: completed,
        effects_applied: 0,
        funding_review: None,
        actual_conservation: None,
    };
    let plan = &report.plan;
    if matches.get_flag("json") {
        let executable = string_option_or_else(matches, "icp", default_icp);
        let identity = string_option(matches, "identity");
        let mut args = super::automation::command(
            &plan.environment,
            &executable,
            identity.as_deref(),
            "bootstrap",
            &plan.fleet,
        );
        let recovery = string_option(matches, "approve-recovery");
        if let Some(review) = &recovery {
            args.extend([
                "--recover".into(),
                plan.plan_sha256.clone(),
                "--approve-recovery".into(),
                review.clone(),
            ]);
        } else {
            args.extend(["--apply".into(), plan.plan_sha256.clone()]);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1, "stage": "infrastructure_bootstrap", "completed": completed,
                "plan": plan, "apply_command": next,
                "continuation_forecast": canic_host::fleet_ensure::policy::continuation_forecast::forecast(&report),
                "automation": super::automation::bootstrap(plan, completed, recovery.clone(), matches.get_one::<String>("apply").is_some() || recovery.is_some(), args),
            }))?
        );
    } else {
        let source = plan
            .infrastructure_bootstrap
            .as_ref()
            .expect("verified bootstrap source");
        println!("coordinator_selection: {:?}", source.coordinator);
        for (name, original) in &source.sources {
            println!(
                "supplied: {name}; id={}; subnet={}; controllers={:?}; module={:?}; native_cycles={}; reserved_cycles={}",
                original.sample.binding.canister_id,
                original.sample.binding.subnet,
                original.sample.binding.controllers,
                original
                    .sample
                    .binding
                    .module_sha256
                    .map(canic_core::cdk::utils::hash::hex_bytes),
                original.sample.cycles,
                original.sample.reserved_cycles
            );
        }
        for target in &plan.canisters {
            for action in &target.actions {
                println!("effect: {}", serde_json::to_string(action)?);
            }
        }
        super::render_report(&report, false)?;
        println!("apply: {next}");
        if completed {
            println!(
                "Infrastructure setup and estate publication complete. Next: review each Root's held set with fleet import, then fleet generate and fleet ensure."
            );
        }
    }
    Ok(())
}

fn review(
    matches: &ArgMatches,
    workspace: &std::path::Path,
    environment: &str,
    fleet: &str,
    executable: &str,
    identity: Option<&str>,
) -> Result<canic_host::fleet_ensure::model::FleetEnsurePlan, FleetCommandError> {
    let selection = match required_string(matches, "coordinator").as_str() {
        "create" => BootstrapCoordinatorSelection::Create,
        "initialize" => BootstrapCoordinatorSelection::Initialize,
        "ready" => BootstrapCoordinatorSelection::Ready,
        _ => unreachable!("Clap validates Coordinator selection"),
    };
    let source = string_option(matches, "source").map_or_else(
        || PathBuf::from(format!("deployments/{fleet}.toml")),
        PathBuf::from,
    );
    let seed = string_option(matches, "seed").map_or_else(
        || PathBuf::from(format!("deployments/{fleet}.estate.toml")),
        PathBuf::from,
    );
    let desired = generate_infrastructure_bootstrap(
        &FleetGenerateRequest {
            catalog_progress: None,
            app_config: &resolve_from_root(
                workspace,
                &PathBuf::from(required_string(matches, "app-config")),
            ),
            environment,
            fleet,
            icp_executable: executable,
            signing_identity: identity,
            release_build_id: *matches
                .get_one::<ReleaseBuildId>("release-build")
                .expect("required release build"),
            root: workspace,
            seed: &resolve_from_root(workspace, &seed),
            source: &resolve_from_root(workspace, &source),
        },
        selection,
        None,
    )?;
    let declaration_path = resolve_from_root(
        workspace,
        &PathBuf::from(required_string(matches, "declarations")),
    );
    let declarations = String::from_utf8(
        read_file_no_follow(&declaration_path, 256 * 1024).map_err(std::io::Error::from)?,
    )
    .map_err(|_| FleetCommandError::Usage("bootstrap declarations must be UTF-8 TOML".into()))?;
    let icp = IcpCli::new(executable, Some(environment.into()))
        .with_identity(identity)
        .with_cwd(workspace);
    let mut platform =
        IcpEnsurePlatform::new(desired.clone(), executable, workspace).with_identity(identity);
    Ok(infrastructure_bootstrap::review(
        &InfrastructureBootstrapReviewRequest {
            workspace,
            desired: &desired,
            coordinator: selection,
            declarations_toml: &declarations,
            seed: &resolve_from_root(workspace, &seed),
            planned_at_time: now_nanoseconds()?,
        },
        &mut platform,
        &icp,
    )?)
}
