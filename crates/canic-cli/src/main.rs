//! Canic command line interface.

mod args;
mod catalog;
#[cfg(test)]
mod config_tests;
mod fleet;
mod status;
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
}

async fn run(cli: &Cli) -> canic_host::Result<Output> {
    let result = match &cli.command {
        Commands::App(AppCommand::Canister(command)) => match command {
            AppCanisterCommand::List { selection } => {
                return catalog::app_canisters(
                    &selection.config.config,
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
            } => {
                let declaration = canic_host::load_config(&selection.config.config)?;
                let (app, _) = declaration.select_app(selection.app.as_deref())?;
                let config = canic_host::add_app_canister(
                    &selection.config.config,
                    app,
                    name,
                    canic_host::CanisterDefinition {
                        role: role.clone(),
                        parent: parent.clone(),
                        wasm: wasm.clone(),
                    },
                )?;
                if !cli.json {
                    return Ok(Output::Text(format!(
                        "Defined app canister {name}\nApp: {}\nConfig: {}\nShared by all app fleets. Local files only; nothing deployed.\n",
                        app,
                        ic_host_text::escape::terminal_text(
                            &selection.config.config.to_string_lossy()
                        ),
                    )));
                }
                Ok(
                    serde_json::json!({"config_path":selection.config.config,"configuration":config,"deployed":false}),
                )
            }
        },
        Commands::App(AppCommand::List { config }) => {
            return catalog::list(&config.config, None, true, cli.json);
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
                let canic_source = canic_source.as_deref().ok_or_else(|| {
                    Error::Invalid("source generation requires --canic-source".into())
                })?;
                let path = canic_host::create_starter(
                    source,
                    name,
                    fleet,
                    network.clone().into_network()?,
                    canic_source,
                )?;
                if !cli.json {
                    return Ok(Output::Text(format!(
                        "Initialized app {name}\nFleet: {fleet}\nConfig: {}\n\nRoot\n└─ Application\n\nSource created; nothing built or deployed. See the generated README for build steps.\n",
                        ic_host_text::escape::terminal_text(&path.to_string_lossy()),
                    )));
                }
                return Ok(Output::Json(
                    serde_json::json!({"config_path":path,"configuration":canic_host::load_config(&path)?,"source":source,"deployed":false}),
                ));
            }
            let declaration =
                canic_host::init_app(&config.config, name, fleet, network.clone().into_network()?)?;
            if !cli.json {
                return Ok(Output::Text(format!(
                    "Initialized app {name}\nFleet: {fleet}\nConfig: {}\nNo canisters defined yet. Use app canister add.\nLocal files only; nothing deployed or funded.\n",
                    ic_host_text::escape::terminal_text(&config.config.to_string_lossy()),
                )));
            }
            Ok(
                serde_json::json!({"config_path":config.config,"configuration":declaration,"deployed":false}),
            )
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
    let scope = command.scope();
    let config_path = scope
        .and_then(|scope| scope.config.clone())
        .unwrap_or_else(|| "canic.json".into());
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
        | FleetCommand::Status { digest, .. }
        | FleetCommand::Call { digest, .. } = command
    {
        config.check_operation(&state_dir, digest)?;
    }
    let result = match command {
        FleetCommand::Canister(command) => match command {
            CanisterCommand::Bind {
                name,
                id,
                selection,
            } => {
                let scope = canic_host::load_config(&selection.config.config)?
                    .select(selection.app.as_deref(), selection.fleet.as_deref())?;
                let selected = canic_host::bind_canister(
                    &selection.config.config,
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
                        ic_host_text::escape::terminal_text(
                            &selection.config.config.to_string_lossy()
                        ),
                        selected.config.canisters[name]
                            .id
                            .as_deref()
                            .unwrap_or("unset"),
                    )));
                }
                Ok(
                    serde_json::json!({"config_path":selection.config.config,"configuration":selected,"deployed":false}),
                )
            }
            CanisterCommand::List { selection } => {
                return catalog::canisters(
                    &selection.config.config,
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
            let declaration = canic_host::load_config(&config.config)?;
            let (app, _) = declaration.select_app(app.as_deref())?;
            let selected =
                canic_host::init_fleet(&config.config, app, name, network.clone().into_network()?)?;
            if !cli.json {
                return Ok(Output::Text(format!(
                    "Initialized fleet {name}\nApp: {app}\nConfig: {}\nUsing shared app definitions with no existing ID bindings. Nothing deployed or funded.\n",
                    ic_host_text::escape::terminal_text(&config.config.to_string_lossy()),
                )));
            }
            Ok(
                serde_json::json!({"config_path":config.config,"configuration":selected,"deployed":false}),
            )
        }
        FleetCommand::List { config, app } => {
            return catalog::list(&config.config, app.as_deref(), false, cli.json);
        }
        FleetCommand::Plan {
            canister,
            wasm,
            network,
            controller,
            target,
            previous_module,
            root_module,
            execute_at,
            cycles,
            ..
        } => {
            let selected_network = if network.network.is_some() {
                network.clone().into_network()?
            } else {
                if network.url.is_some()
                    || network.root_key.is_some()
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
                controller: controller.clone(),
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
                if let Some(root_module) = root_module {
                    let execute_at = execute_at
                        .map(|seconds| {
                            seconds
                                .checked_mul(1_000_000_000)
                                .ok_or_else(|| Error::Invalid("dispatch timestamp overflow".into()))
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
                    )?
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
            Ok(serde_json::json!({"operation": digest, "plan": plan,"state_dir":state_dir}))
        }
        FleetCommand::Apply {
            digest,
            identity,
            icp_bin,
            ..
        } => Ok(serde_json::json!(
            canic_host::apply(&state_dir, digest, identity, icp_bin).await?
        )),
        FleetCommand::Status { digest, .. } => {
            let state = canic_host::status(&state_dir, digest)?;
            if !cli.json {
                return status::render(&state, &state_dir).map(Output::Text);
            }
            Ok(state)
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
        FleetCommand::Call { digest, name, .. } => {
            let state = canic_host::status(&state_dir, digest)?;
            let receipt: canic_host::Receipt = serde_json::from_value(state["receipt"].clone())
                .map_err(|_| {
                    Error::Invalid("operation has no completed deployment receipt".into())
                })?;
            let plan: canic_host::Plan = serde_json::from_value(state["plan"].clone())
                .map_err(|_| Error::Invalid("operation plan cannot be read".into()))?;
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
    match run(&cli).await {
        Ok(Output::Text(value)) => {
            print!("{value}");
            std::process::ExitCode::SUCCESS
        }
        Ok(Output::Json(value)) => {
            if cli.json {
                println!("{value}");
            } else {
                println!("{value:#}");
            }
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            let kind = match &error {
                Error::Invalid(_) => "invalid",
                Error::Blocked(_) => "blocked",
                Error::Supplier(_) => "supplier",
                Error::Encode(_) | Error::Decode(_) => "codec",
            };
            eprintln!(
                "{}",
                serde_json::json!({"error":kind, "message":error.to_string()})
            );
            std::process::ExitCode::FAILURE
        }
    }
}
