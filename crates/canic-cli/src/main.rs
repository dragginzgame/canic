//! Canic command line interface.

#![forbid(unsafe_code)]

mod args;
mod artifacts;
mod auth;
mod catalog;
#[cfg(test)]
mod config_tests;
mod fleet;
mod placements;
mod progress;
mod status;
mod subnet_comparison;
mod topology;
mod view;

use args::{
    AppCanisterCommand, AppCommand, ArtifactCommand, CanisterCommand, Cli, Commands, FleetCommand,
    help_command,
};
use canic_host::{Error, PlanInput};
use clap::FromArgMatches as _;
use std::{io::IsTerminal as _, path::PathBuf};

enum Output {
    Json(serde_json::Value),
    Text(String),
    Partial(Box<Output>),
}

fn error_text(error: &Error, json: bool) -> String {
    let message = error.to_string();
    if json {
        let kind = match error {
            Error::Invalid(_) => "invalid",
            Error::Blocked(_) => "blocked",
            Error::Supplier(_) => "supplier",
            Error::Encode(_) | Error::Decode(_) => "codec",
        };
        return serde_json::json!({"error":kind, "message":message}).to_string();
    }
    let mut lines = message.split('\n');
    let mut text = format!(
        "Error: {}",
        ic_host_text::escape(lines.next().unwrap_or_default())
    );
    for line in lines {
        text.push_str("\n  ");
        text.push_str(&ic_host_text::escape(line));
    }
    text
}

async fn run(cli: &Cli) -> canic_host::Result<Output> {
    let result = match &cli.command {
        Commands::App(AppCommand::Build {
            selection,
            canister,
        }) => {
            let built = canic_host::build_app(
                &selection.config.path(selection.app.as_deref())?,
                selection.app.as_deref(),
                canister.as_deref(),
            )?;
            if !cli.json {
                return Ok(Output::Text(progress::built(&built)));
            }
            Ok(serde_json::json!({"canisters":built}))
        }
        Commands::App(AppCommand::Canister(command)) => match command {
            AppCanisterCommand::List { selection } => {
                return catalog::app_canisters(
                    &selection.config.path(selection.app.as_deref())?,
                    selection.app.as_deref(),
                    cli.json,
                );
            }
            AppCanisterCommand::Add {
                name,
                selection,
                role,
                parent,
                wasm,
                manifest,
                package,
            } => {
                let path = selection.config.path(selection.app.as_deref())?;
                let declaration = canic_host::load_config(&path)?;
                let (app, _) = declaration.select_app(selection.app.as_deref())?;
                let config = canic_host::add_app_canister(
                    &path,
                    app,
                    name,
                    canic_host::CanisterDefinition {
                        role: role.clone(),
                        parent: parent.clone(),
                        wasm: wasm.clone(),
                        build: manifest.as_ref().zip(package.as_ref()).map(
                            |(manifest, package)| canic_host::BuildInput {
                                manifest: manifest.clone(),
                                package: package.clone(),
                            },
                        ),
                    },
                )?;
                if !cli.json {
                    return Ok(Output::Text(format!(
                        "Defined app canister {name}\nApp: {}\nConfig: {}\nShared by all app fleets. Local files only; nothing deployed.\n",
                        app,
                        ic_host_text::escape(&path.to_string_lossy()),
                    )));
                }
                Ok(serde_json::json!({"config_path":path,"configuration":config,"deployed":false}))
            }
        },
        Commands::App(AppCommand::List { config }) => {
            return catalog::list(config.config.as_deref(), None, true, cli.json);
        }
        Commands::App(AppCommand::Init {
            name,
            config,
            fleet,
            network,
            source,
            canic_source,
        }) => {
            if let Some(source) = source {
                let default_source = args::checkout();
                let canic_source = canic_source.as_deref().unwrap_or(&default_source);
                let path = canic_host::create_starter(
                    source,
                    name,
                    fleet,
                    network.clone().into_network()?,
                    canic_source,
                )?;
                if !cli.json {
                    return Ok(Output::Text(format!(
                        "Initialized app {name}\nFleet: {fleet}\nConfig: {}\n\nRoot\n├─ Wasm Store\n└─ Application\n\nSource created; nothing built or deployed. See the generated README for build steps.\n",
                        ic_host_text::escape(&path.to_string_lossy()),
                    )));
                }
                return Ok(Output::Json(
                    serde_json::json!({"config_path":path,"configuration":canic_host::load_config(&path)?,"source":source,"deployed":false}),
                ));
            }
            let path = config
                .config
                .clone()
                .unwrap_or_else(|| PathBuf::from("canic.json"));
            let declaration =
                canic_host::init_app(&path, name, fleet, network.clone().into_network()?)?;
            if !cli.json {
                return Ok(Output::Text(format!(
                    "Initialized app {name}\nFleet: {fleet}\nConfig: {}\nNo canisters defined yet. Use app canister add.\nLocal files only; nothing deployed or funded.\n",
                    ic_host_text::escape(&path.to_string_lossy()),
                )));
            }
            Ok(serde_json::json!({"config_path":path,"configuration":declaration,"deployed":false}))
        }

        Commands::Artifact(command) => match command {
            ArtifactCommand::Build {
                manifest,
                package,
                features,
                output,
            } => {
                let path = canic_host::build(manifest, package, features, output.as_deref())?;
                let artifact = canic_host::inspect(&path)?;
                Ok(serde_json::json!({"wasm": path, "artifact": artifact}))
            }
            ArtifactCommand::Inspect { wasm } => Ok(serde_json::json!(canic_host::inspect(wasm)?)),
        },
        Commands::Fleet(command) => return run_fleet(cli, command).await,
        Commands::Wallet {
            principal,
            address_only,
        } => {
            if *address_only {
                Ok(
                    serde_json::json!({"network":"ic / mainnet", "icp_address":canic_host::wallet_address(principal)?}),
                )
            } else {
                Ok(serde_json::json!(canic_host::wallet(principal).await?))
            }
        }
    };
    result.map(Output::Json)
}

async fn run_fleet(cli: &Cli, command: &FleetCommand) -> canic_host::Result<Output> {
    if let FleetCommand::Auth(command) = command {
        return auth::run(command, cli.json);
    }
    if let FleetCommand::Artifact(command) = command {
        return artifacts::run(command, cli.json);
    }
    if let FleetCommand::CompareSubnets {
        selection,
        left,
        right,
    } = command
    {
        let path = selection.config.path(selection.app.as_deref())?;
        let selected = canic_host::load_config(&path)?
            .select(selection.app.as_deref(), selection.fleet.as_deref())?;
        let report = canic_host::subnet_comparison::observe(&selected, left, right).await?;
        return Ok(subnet_comparison::output(&report, cli.json));
    }
    if let FleetCommand::Placements { selection } = command {
        let path = selection.config.path(selection.app.as_deref())?;
        let selected = canic_host::load_config(&path)?
            .select(selection.app.as_deref(), selection.fleet.as_deref())?;
        let report = canic_host::placements::observe(&selected).await?;
        return Ok(placements::output(&report, cli.json));
    }
    let scope = command.scope();
    let config_path = match scope {
        Some(scope) if !scope.no_config => canic_host::resolve_config_path(
            std::path::Path::new(""),
            scope.config.as_deref(),
            scope.app.as_deref(),
        )?,
        _ => PathBuf::from("canic.json"),
    };
    let configuration = if let Some(scope) = scope
        && !scope.no_config
        && (scope.config.is_some()
            || config_path
                .try_exists()
                .map_err(|error| Error::Supplier(Box::new(error)))?)
    {
        Some(
            canic_host::load_config(&config_path)?
                .select(scope.app.as_deref(), scope.fleet.as_deref())?,
        )
    } else {
        None
    };
    if let Some(scope) = scope
        && configuration.is_none()
        && (scope.app.is_some() || scope.fleet.is_some())
    {
        return Err(Error::Invalid(
            "app/fleet selection requires a config".into(),
        ));
    }
    let state_dir = scope
        .and_then(|scope| scope.state_dir.clone())
        .unwrap_or_else(|| {
            configuration.as_ref().map_or_else(
                || PathBuf::from(".canic"),
                |config| config.journal_path(&config_path),
            )
        });
    if let Some(config) = &configuration
        && let FleetCommand::Apply { digest, .. }
        | FleetCommand::Activate { digest, .. }
        | FleetCommand::Reconcile { digest, .. }
        | FleetCommand::Cancel { digest, .. }
        | FleetCommand::Status { digest, .. }
        | FleetCommand::Call { digest, .. } = command
    {
        config.check_operation(&state_dir, digest)?;
    }
    let result = match command {
        FleetCommand::Auth(_)
        | FleetCommand::Artifact(_)
        | FleetCommand::CompareSubnets { .. }
        | FleetCommand::Placements { .. } => {
            unreachable!("artifact and observation commands handled before deployment resolution")
        }
        FleetCommand::Canister(command) => match command {
            CanisterCommand::Bind {
                name,
                id,
                selection,
            } => {
                let path = selection.config.path(selection.app.as_deref())?;
                let scope = canic_host::load_config(&path)?
                    .select(selection.app.as_deref(), selection.fleet.as_deref())?;
                let selected = canic_host::bind_canister(
                    &path,
                    &scope.app,
                    &scope.fleet,
                    name,
                    canic_host::CanisterBinding {
                        id: id.clone(),
                        ..Default::default()
                    },
                )?;
                if !cli.json {
                    return Ok(Output::Text(format!(
                        "Bound canister slot {name}\nApp: {}\nFleet: {}\nConfig: {}\nConfigured ID: {}\nObservation declaration only; no canister or journal created. Existence and control are not verified.\n",
                        selected.app,
                        selected.fleet,
                        ic_host_text::escape(&path.to_string_lossy()),
                        selected.config.canisters[name]
                            .id
                            .as_deref()
                            .unwrap_or("unset"),
                    )));
                }
                Ok(
                    serde_json::json!({"config_path":path,"configuration":selected,"deployed":false}),
                )
            }
            CanisterCommand::List { selection } => {
                return catalog::canisters(
                    &selection.config.path(selection.app.as_deref())?,
                    selection.app.as_deref(),
                    selection.fleet.as_deref(),
                    cli.json,
                );
            }
        },
        FleetCommand::Init {
            name,
            app,
            config,
            network,
        } => {
            let path = config.path(app.as_deref())?;
            let declaration = canic_host::load_config(&path)?;
            let (app, _) = declaration.select_app(app.as_deref())?;
            let selected =
                canic_host::init_fleet(&path, app, name, network.clone().into_network()?)?;
            if !cli.json {
                return Ok(Output::Text(format!(
                    "Initialized fleet {name}\nApp: {app}\nConfig: {}\nUsing shared app definitions with no existing ID bindings. Nothing deployed or funded.\n",
                    ic_host_text::escape(&path.to_string_lossy()),
                )));
            }
            Ok(serde_json::json!({"config_path":path,"configuration":selected,"deployed":false}))
        }
        FleetCommand::List { config, app } => {
            return catalog::list(config.config.as_deref(), app.as_deref(), false, cli.json);
        }
        FleetCommand::Plan {
            canister,
            wasm,
            network,
            controller,
            identity,
            signer,
            target,
            previous_module,
            root_module,
            install_args,
            blob_attempt,
            publication,
            execute_at,
            cycles,
            ..
        } => {
            let controller = match (controller, identity) {
                (Some(controller), None) => controller.clone(),
                (None, Some(identity)) => canic_host::signing_principal(identity, &signer.icp_bin)?,
                _ => return Err(Error::Invalid("select --identity or --controller".into())),
            };
            let dispatch = execute_at
                .map(|seconds| {
                    seconds
                        .checked_mul(1_000_000_000)
                        .ok_or_else(|| Error::Invalid("dispatch timestamp overflow".into()))
                })
                .transpose()?;
            if canister.is_none()
                && wasm.is_none()
                && target.is_none()
                && let Some(selected) = &configuration
            {
                if network.network.is_some()
                    || network.url.is_some()
                    || network.root_key.is_some()
                    || network.root_subnet.is_some()
                    || network.routing_canister.is_some()
                    || root_module.is_some()
                {
                    return Err(Error::Invalid(
                        "whole-fleet planning uses the configured network and Root artifact".into(),
                    ));
                }
                let result = canic_host::plan_fleet(
                    &state_dir,
                    &config_path,
                    selected,
                    &controller,
                    *cycles,
                    dispatch,
                )?;
                if !cli.json {
                    let (digest, mut text) = match &result {
                        canic_host::FleetPlanning::Create { operation, fleet_plan } =>
                            (operation, status::fleet_preview(operation, fleet_plan, &state_dir)?),
                        canic_host::FleetPlanning::AddChild { operation, .. } | canic_host::FleetPlanning::Upgrade { operation, .. } | canic_host::FleetPlanning::Maintenance { operation, .. } | canic_host::FleetPlanning::Publication { operation, .. } =>
                            (operation, status::render(&canic_host::status(&state_dir, operation)?, &state_dir)?),
                        canic_host::FleetPlanning::Unchanged { .. } => return Ok(Output::Text(
                            "No artifact changes. Matches retained completion; network state was not refreshed.\n".into()
                        )),
                    };
                    text.push_str(&progress::next(
                        command,
                        digest,
                        "apply",
                        identity.as_deref(),
                    ));
                    return Ok(Output::Text(text));
                }
                let mut output =
                    serde_json::to_value(result).map_err(|e| Error::Invalid(e.to_string()))?;
                output["state_dir"] = serde_json::json!(state_dir);
                return Ok(Output::Json(output));
            }
            if execute_at.is_some() && root_module.is_none() {
                return Err(Error::Invalid(
                    "--execute-at requires a whole-fleet plan or --root-module".into(),
                ));
            }
            let selected_network = if network.network.is_some() {
                network.as_ref().clone().into_network()?
            } else {
                if network.url.is_some()
                    || network.root_key.is_some()
                    || network.root_subnet.is_some()
                    || network.routing_canister.is_some()
                {
                    return Err(Error::Invalid("network options require --network".into()));
                }
                configuration
                    .as_ref()
                    .ok_or_else(|| {
                        Error::Invalid("plan requires a fleet config or explicit --network".into())
                    })?
                    .config
                    .network
                    .clone()
            };
            if configuration
                .as_ref()
                .is_some_and(|config| config.config.network != selected_network)
            {
                return Err(Error::Invalid(
                    "plan network differs from fleet config".into(),
                ));
            }
            let input = PlanInput {
                network: selected_network,
                controller,
                target: target.clone(),
                previous_module: previous_module.clone(),
                creation_cycles: cycles.unwrap_or(if target.is_some() {
                    0
                } else {
                    1_000_000_000_000
                }),
            };
            let (digest, plan) = if let Some(canister) = canister {
                let config = configuration.as_ref().ok_or_else(|| {
                    Error::Invalid("named deployment requires a fleet config".into())
                })?;
                let wasm = config.wasm_path(&config_path, canister, wasm.as_deref())?;
                if let Some(version) = &publication.publish_version {
                    canic_host::create_publication_plan(
                        &state_dir,
                        &wasm,
                        config,
                        &input.controller,
                        canic_host::PublicationInput {
                            canister: canister.clone(),
                            store: publication.store.clone().expect("Clap publication Store"),
                            version: version.clone(),
                            root_module: root_module.clone().expect("Clap publication Root module"),
                            store_module: publication
                                .store_module
                                .clone()
                                .expect("Clap publication Store module"),
                        },
                    )?
                } else if let Some(root_module) = root_module {
                    if config
                        .config
                        .canisters
                        .get(canister)
                        .is_some_and(|slot| slot.role.as_deref() == Some("wasm-store"))
                    {
                        if execute_at.is_some() || publication.release.is_some() {
                            return Err(Error::Invalid("Store bootstrap accepts neither an application release nor a child dispatch schedule".into()));
                        }
                        canic_host::create_store_plan(
                            &state_dir,
                            &wasm,
                            input,
                            config,
                            canister,
                            root_module,
                        )?
                    } else {
                        let execute_at = execute_at
                            .map(|seconds| {
                                seconds.checked_mul(1_000_000_000).ok_or_else(|| {
                                    Error::Invalid("dispatch timestamp overflow".into())
                                })
                            })
                            .transpose()?;
                        canic_host::create_managed_plan(
                            &state_dir,
                            &wasm,
                            input,
                            config,
                            canister,
                            root_module,
                            execute_at,
                            publication.release.as_deref(),
                            install_args.as_deref(),
                            *blob_attempt,
                        )?
                    }
                } else {
                    canic_host::create_configured_plan(&state_dir, &wasm, input, config, canister)?
                }
            } else {
                if configuration.is_some() && target.is_none() {
                    return Err(Error::Invalid(
                        "configured creation requires --canister in the selected app/fleet".into(),
                    ));
                }
                if let (Some(config), Some(target)) = (&configuration, target) {
                    config.check_target(&state_dir, target)?;
                }
                let wasm = wasm
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("this plan requires --wasm".into()))?;
                canic_host::create_plan(&state_dir, wasm, input)?
            };
            if !cli.json {
                let state = canic_host::status(&state_dir, &digest)?;
                let mut text = status::render(&state, &state_dir)?;
                text.push_str(&progress::next(
                    command,
                    &digest,
                    "apply",
                    identity.as_deref(),
                ));
                return Ok(Output::Text(text));
            }
            Ok(serde_json::json!({"operation": digest, "plan": plan,"state_dir":state_dir}))
        }
        FleetCommand::Activate {
            digest,
            identity,
            signer,
            ..
        } => {
            let outcome =
                canic_host::activate(&state_dir, digest, identity, &signer.icp_bin).await?;
            if !cli.json {
                return Ok(Output::Text(progress::applied(&outcome, command)));
            }
            Ok(serde_json::json!(outcome))
        }
        FleetCommand::Apply {
            digest,
            identity,
            signer,
            ..
        } => {
            let outcome = canic_host::apply(&state_dir, digest, identity, &signer.icp_bin).await?;
            if !cli.json {
                return Ok(Output::Text(progress::applied(&outcome, command)));
            }
            Ok(serde_json::json!(outcome))
        }
        FleetCommand::Reconcile {
            digest,
            identity,
            signer,
            ..
        } => {
            let outcome =
                canic_host::reconcile(&state_dir, digest, identity, &signer.icp_bin).await?;
            if !cli.json {
                return Ok(Output::Text(progress::applied(&outcome, command)));
            }
            Ok(serde_json::json!(outcome))
        }
        FleetCommand::Cancel { digest, .. } => {
            let cancelled = canic_host::cancel_fleet_plan(&state_dir, digest)?;
            if !cli.json {
                return Ok(Output::Text(format!(
                    "Cancelled unsubmitted fleet approval {digest}\nEvidence retained. Plan again to review a new approval.\n"
                )));
            }
            Ok(serde_json::json!(cancelled))
        }
        FleetCommand::Status { digest, .. } => {
            let state = canic_host::status(&state_dir, digest)?;
            if !cli.json {
                return status::render(&state, &state_dir).map(Output::Text);
            }
            Ok(serde_json::json!(state))
        }
        FleetCommand::View { report, port, open } => {
            view::serve(report, *port, *open)?;
            Ok(serde_json::json!({"viewer": "stopped"}))
        }
        FleetCommand::Show {
            color,
            details,
            html,
            open,
            link,
            serve,
            port,
            manager,
            wallet,
            watch,
            ..
        } => {
            let snapshot = fleet::collect(
                &state_dir,
                manager,
                wallet.as_deref(),
                configuration.as_ref(),
            )
            .await?;
            if *watch {
                fleet::publish(&snapshot, html.as_deref(), false, false, *port, link)?;
                fleet::watch(
                    &state_dir,
                    manager,
                    wallet.as_deref(),
                    link,
                    &snapshot,
                    *port,
                    *open,
                )
                .await?;
                return Ok(Output::Json(serde_json::json!({"viewer":"stopped"})));
            }
            fleet::publish(&snapshot, html.as_deref(), *open, *serve, *port, link)?;
            if !cli.json {
                let color = color.enabled(
                    std::io::stdout().is_terminal(),
                    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
                    std::env::var_os("TERM").is_some_and(|value| value == "dumb"),
                );
                return Ok(Output::Text(fleet::terminal(
                    &snapshot, link, *details, color,
                )));
            }
            Ok(serde_json::json!(snapshot))
        }
        FleetCommand::Call {
            digest,
            name,
            canister,
            ..
        } => {
            let state = canic_host::status(&state_dir, digest)?;
            let state = status::call_state(&state, canister.as_deref())?;
            let receipt = state.receipt.as_ref().ok_or_else(|| {
                Error::Invalid("operation has no completed deployment receipt".into())
            })?;
            let plan = &state.plan;
            let result = canic_host::greet(&plan.network, &receipt.canister, name).await?;
            Ok(
                serde_json::json!({"canister": receipt.canister, "certified": false, "greeting": result.message}),
            )
        }
    };
    result.map(Output::Json)
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let cli =
        Cli::from_arg_matches(&help_command().get_matches()).unwrap_or_else(|error| error.exit());
    let result = run(&cli).await;
    let (result, success) = match result {
        Ok(Output::Partial(value)) => (Ok(*value), std::process::ExitCode::FAILURE),
        other => (other, std::process::ExitCode::SUCCESS),
    };
    match result {
        Ok(Output::Text(value)) => {
            print!("{value}");
            success
        }
        Ok(Output::Json(value)) => {
            if cli.json {
                println!("{value}");
            } else {
                println!("{value:#}");
            }
            success
        }
        Ok(Output::Partial(_)) => unreachable!("partial output is unwrapped once"),
        Err(error) => {
            eprintln!("{}", error_text(&error, cli.json));
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
