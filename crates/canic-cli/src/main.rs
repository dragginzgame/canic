//! Canic command line interface.

mod catalog;
#[cfg(test)]
mod config_tests;
mod fleet;
mod status;
mod topology;
mod view;

use canic_host::{Error, Network, PlanInput};
use clap::{Args, CommandFactory as _, FromArgMatches as _, Parser, Subcommand, ValueEnum};
use std::{io::IsTerminal as _, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "canic",
    version,
    propagate_version = true,
    about = "Manage Canic apps, fleet deployments and Wasm artifacts"
)]
struct Cli {
    /// Emit structured JSON on stdout.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

impl ColorMode {
    fn enabled(self, terminal: bool, no_color: bool, dumb: bool) -> bool {
        match self {
            Self::Auto => terminal && !no_color && !dumb,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize apps and manage their reusable canister definitions.
    #[command(subcommand)]
    App(AppCommand),
    /// Build or inspect local Wasm artifacts.
    #[command(subcommand)]
    Artifact(ArtifactCommand),
    /// List, deploy and observe fleets, or view a saved fleet report.
    #[command(subcommand)]
    Fleet(FleetCommand),
    /// Show a mainnet wallet's default receive address and public ICP/cycles balances.
    Wallet {
        /// Public owner principal; never an identity name or seed phrase.
        principal: String,
        /// Derive only the receive address locally, without network access.
        #[arg(long)]
        address_only: bool,
    },
}

#[derive(Args)]
struct ConfigArgs {
    /// Local application/fleet declarations.
    #[arg(long, default_value = "canic.json", value_name = "PATH")]
    config: PathBuf,
}

#[derive(Args)]
struct FleetScope {
    /// Override the journal (default .canic/apps/APP/fleets/FLEET); without config, .canic.
    #[arg(long)]
    state_dir: Option<PathBuf>,
    /// Fleet config; defaults to canic.json when present. No config keeps the legacy journal flow.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Select an app; required when the config has multiple apps.
    #[arg(long, conflicts_with = "no_config")]
    app: Option<String>,
    /// Select a fleet within the app; required when the app has multiple fleets.
    #[arg(long, conflicts_with = "no_config")]
    fleet: Option<String>,
    /// Ignore canic.json and use the legacy .canic journal unless --state-dir is supplied.
    #[arg(long, conflicts_with = "config")]
    no_config: bool,
}

#[derive(Subcommand)]
enum AppCommand {
    /// Define or list reusable app canisters; affects declarations for every fleet.
    #[command(subcommand)]
    Canister(AppCanisterCommand),
    /// Add a new app and its first fleet, creating the config if missing; no deployment.
    #[command(mut_arg("network", |arg| arg.required(true)))]
    Init {
        /// Logical application name.
        #[arg(value_name = "APP")]
        name: String,
        #[command(flatten)]
        config: ConfigArgs,
        /// Name of the app's first fleet.
        #[arg(long, value_name = "FLEET")]
        fleet: String,
        #[command(flatten)]
        network: NetworkArgs,
        /// Generate a new standalone source workspace and its config in this directory.
        #[arg(
            long,
            value_name = "DIRECTORY",
            conflicts_with = "config",
            requires = "canic_source"
        )]
        source: Option<PathBuf>,
        /// Matching local Canic checkout supplying the runtime and greeting contract.
        #[arg(long, value_name = "DIRECTORY", requires = "source")]
        canic_source: Option<PathBuf>,
    },
    /// List apps declared in the local config; makes no network calls.
    List {
        #[command(flatten)]
        config: ConfigArgs,
    },
}

#[derive(Args)]
struct AppConfigArgs {
    #[command(flatten)]
    config: ConfigArgs,
    /// Select an app; required when the config has multiple apps.
    #[arg(long)]
    app: Option<String>,
}

#[derive(Subcommand)]
enum AppCanisterCommand {
    /// Add a shared definition; deploys nothing and grants no management authority.
    Add {
        #[arg(value_name = "CANISTER")]
        name: String,
        #[command(flatten)]
        selection: AppConfigArgs,
        /// Application role, independent of controllers.
        #[arg(long)]
        role: Option<String>,
        /// Parent definition in this app; does not grant control or permission to spawn.
        #[arg(long, value_name = "CANISTER")]
        parent: Option<String>,
        /// Wasm input relative to the config file; planning retains the exact bytes.
        #[arg(long, value_name = "WASM_FILE")]
        wasm: Option<PathBuf>,
    },
    /// List shared app definitions; no network or journal access.
    List {
        #[command(flatten)]
        selection: AppConfigArgs,
    },
}

#[derive(Subcommand)]
enum ArtifactCommand {
    /// Build an application Wasm using the locked Cargo graph.
    Build {
        #[arg(long, default_value = "Cargo.toml")]
        manifest: PathBuf,
        /// Cargo package to compile; independent of app/fleet declarations.
        #[arg(long, value_name = "PACKAGE")]
        package: String,
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,
        /// Atomically write the built Wasm here, replacing a previous build input.
        #[arg(long, value_name = "WASM_FILE")]
        output: Option<PathBuf>,
    },
    /// Report the SHA-256 and size of a local Wasm file; no network calls.
    Inspect {
        #[arg(value_name = "WASM_FILE")]
        wasm: PathBuf,
    },
}

#[derive(Subcommand)]
enum FleetCommand {
    /// Bind existing canister IDs or list this fleet's app-defined slots; local config only.
    #[command(subcommand)]
    Canister(CanisterCommand),
    /// Add a fleet using the app's definitions, with no existing IDs; deploys nothing.
    #[command(mut_arg("network", |arg| arg.required(true)))]
    Init {
        /// New fleet name within the selected app.
        #[arg(value_name = "FLEET")]
        name: String,
        /// Select an existing app; required when the config has multiple apps.
        #[arg(long)]
        app: Option<String>,
        #[command(flatten)]
        config: ConfigArgs,
        #[command(flatten)]
        network: NetworkArgs,
    },
    /// List configured fleets across apps; use --app to filter. Makes no network calls.
    List {
        #[command(flatten)]
        config: ConfigArgs,
        /// Filter declarations by application name.
        #[arg(long)]
        app: Option<String>,
    },
    /// Plan one canister creation or upgrade in the selected fleet; writes only local evidence.
    Plan {
        #[command(flatten)]
        scope: FleetScope,
        /// Named slot for its first deployment; repeats reuse the original plan.
        #[arg(long, value_name = "SLOT", conflicts_with_all = ["target", "no_config"])]
        canister: Option<String>,
        /// Local Wasm input; omit for a named canister with an app-defined Wasm path.
        #[arg(long, value_name = "WASM_FILE")]
        wasm: Option<PathBuf>,
        #[command(flatten)]
        network: NetworkArgs,
        /// Controller principal to bind into this exact plan.
        #[arg(long, value_name = "PRINCIPAL")]
        controller: String,
        /// Existing canister to upgrade, preserving its principal.
        #[arg(long, value_name = "PRINCIPAL", requires = "previous_module")]
        target: Option<String>,
        /// Certified module hash from the previous deployment receipt.
        #[arg(long, requires = "target")]
        previous_module: Option<String>,
        /// Admit the named child through its Root parent, pinned to this reviewed module hash.
        #[arg(long, value_name = "SHA256", requires = "canister", conflicts_with_all = ["target", "previous_module"])]
        root_module: Option<String>,
        /// Unix seconds for a new Root plan's dispatch; default is ten minutes from planning.
        /// Reopening a slot keeps its original schedule.
        #[arg(long, value_name = "UNIX_SECONDS", requires = "root_module")]
        execute_at: Option<u64>,
        /// Creation debit ceiling, including ledger fee; defaults to 1T for creation, zero for upgrade.
        #[arg(long, value_parser = clap::value_parser!(u64).range(0..=5_000_000_000_000))]
        cycles: Option<u64>,
    },
    /// Approve this exact digest and apply or resume its original signed attempts.
    Apply {
        #[command(flatten)]
        scope: FleetScope,
        #[arg(value_name = "OPERATION")]
        digest: String,
        #[arg(long)]
        identity: String,
        /// ICP CLI executable used only to sign offline. Identity passwords stay in its terminal prompt.
        #[arg(long, default_value = "icp")]
        icp_bin: PathBuf,
    },
    /// Inspect local retained progress; makes no network calls.
    Status {
        #[command(flatten)]
        scope: FleetScope,
        /// Original operation digest retained in this fleet journal.
        #[arg(value_name = "OPERATION")]
        digest: String,
    },
    /// Observe the selected fleet; optionally save, serve or watch its report.
    #[command(group(clap::ArgGroup::new("viewer").args(["open", "serve", "watch"]).multiple(true)))]
    Show {
        #[command(flatten)]
        scope: FleetScope,
        /// ANSI colours in fleet terminal output; auto honours NO_COLOR and TERM=dumb.
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
        color: ColorMode,

        /// Include module hashes and network identity fingerprints in terminal output.
        #[arg(long)]
        details: bool,
        /// Include this mainnet wallet's public default-account balances and receive address.
        #[arg(long, value_name = "PRINCIPAL")]
        wallet: Option<String>,
        /// Read a known profile manager's public journal and observe its reported worker.
        #[arg(long)]
        manager: Vec<String>,
        /// Show a declared application route ENTRY:WORKER between known targets (not an on-chain observation).
        #[arg(long, value_name = "ENTRY:WORKER")]
        link: Vec<String>,
        /// Write a new HTML report; existing files are preserved. No signed requests are exported.
        #[arg(long)]
        html: Option<PathBuf>,
        /// Serve a fresh report on localhost and launch the default browser; Ctrl-C stops the viewer.
        #[arg(long)]
        open: bool,
        /// Serve on localhost without opening a browser; Ctrl-C stops the viewer.
        #[arg(long)]
        serve: bool,
        /// Refresh the localhost view continuously; add --open to launch a browser.
        #[arg(long)]
        watch: bool,
        /// Local viewer port; zero selects a free port.
        #[arg(long, default_value_t = 8787, requires = "viewer")]
        port: u16,
    },
    /// Serve one saved HTML snapshot at a localhost URL; Ctrl-C stops the viewer.
    View {
        /// Saved HTML snapshot file, not a fleet name or live deployment.
        #[arg(value_name = "REPORT_HTML")]
        report: PathBuf,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Also launch the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Call the deployed demo's read-only, uncertified greeting query.
    Call {
        #[command(flatten)]
        scope: FleetScope,
        /// Completed hello-demo deployment operation; this is not a generic canister call.
        #[arg(value_name = "OPERATION")]
        digest: String,
        #[arg(long, default_value = "team")]
        name: String,
    },
}

#[derive(Args)]
struct CanisterConfigArgs {
    #[command(flatten)]
    config: ConfigArgs,
    /// Select an app; required when the config has multiple apps.
    #[arg(long)]
    app: Option<String>,
    /// Select a fleet within the app; required when the app has multiple fleets.
    #[arg(long)]
    fleet: Option<String>,
}

#[derive(Subcommand)]
enum CanisterCommand {
    /// Bind an app-defined slot to an existing ID for observation; does not verify control.
    Bind {
        /// Existing app canister definition.
        #[arg(value_name = "SLOT")]
        name: String,
        /// Existing canister to observe on this fleet's network; does not verify control or deploy.
        #[arg(long, value_name = "PRINCIPAL")]
        id: String,
        #[command(flatten)]
        selection: CanisterConfigArgs,
    },
    /// List local slots and explicitly configured IDs; does not inspect deployments or journals.
    List {
        #[command(flatten)]
        selection: CanisterConfigArgs,
    },
}

impl FleetCommand {
    fn scope(&self) -> Option<&FleetScope> {
        match self {
            Self::Plan { scope, .. }
            | Self::Apply { scope, .. }
            | Self::Status { scope, .. }
            | Self::Show { scope, .. }
            | Self::Call { scope, .. } => Some(scope),
            Self::Canister(_) | Self::Init { .. } | Self::List { .. } | Self::View { .. } => None,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum NetworkName {
    Ic,
    Local,
}

#[derive(Args, Clone)]
struct NetworkArgs {
    /// Explicit network; planning otherwise uses the selected fleet's binding.
    #[arg(long, value_enum)]
    network: Option<NetworkName>,
    /// Local replica URL; --network local requires all three trust inputs.
    #[arg(long, required_if_eq("network", "local"))]
    url: Option<String>,
    /// Local replica root key as hex.
    #[arg(long, required_if_eq("network", "local"))]
    root_key: Option<String>,
    /// Local routing canister principal.
    #[arg(long, required_if_eq("network", "local"))]
    routing_canister: Option<String>,
}

impl NetworkArgs {
    fn into_network(self) -> canic_host::Result<Network> {
        match self.network {
            Some(NetworkName::Ic)
                if self.url.is_none()
                    && self.root_key.is_none()
                    && self.routing_canister.is_none() =>
            {
                Ok(Network::Mainnet)
            }
            Some(NetworkName::Local) => Ok(Network::Local {
                url: self
                    .url
                    .ok_or_else(|| Error::Invalid("local URL required".into()))?,
                root_key: self
                    .root_key
                    .ok_or_else(|| Error::Invalid("local root key required".into()))?,
                routing_canister: self
                    .routing_canister
                    .ok_or_else(|| Error::Invalid("local routing canister required".into()))?,
            }),
            None => Err(Error::Invalid(
                "select --network ic or --network local explicitly".into(),
            )),
            _ => Err(Error::Invalid(
                "mainnet uses its fixed URL and compiled trust root".into(),
            )),
        }
    }
}

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

fn help_command() -> clap::Command {
    fn banner(command: clap::Command) -> clap::Command {
        command
            .display_order(None)
            .before_help(concat!("Canic ", env!("CARGO_PKG_VERSION")))
            .mut_subcommands(banner)
    }
    banner(Cli::command())
}
