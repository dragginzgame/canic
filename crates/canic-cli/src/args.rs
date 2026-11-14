//! Clap command declarations and argument conversion; execution lives in main.

use canic_host::{Error, Network};
use clap::{Args, CommandFactory as _, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "canic",
    version,
    propagate_version = true,
    about = "Manage Canic apps, fleet deployments and Wasm artifacts"
)]
pub(super) struct Cli {
    /// Emit structured JSON on stdout.
    #[arg(long, global = true)]
    pub(super) json: bool,
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
pub(super) enum ColorMode {
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub(super) fn enabled(self, terminal: bool, no_color: bool, dumb: bool) -> bool {
        match self {
            Self::Auto => terminal && !no_color && !dumb,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

#[derive(Subcommand)]
pub(super) enum Commands {
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
pub(super) struct ConfigArgs {
    /// Local application/fleet declarations.
    #[arg(long, default_value = "canic.json", value_name = "PATH")]
    pub(super) config: PathBuf,
}

#[derive(Args)]
pub(super) struct FleetScope {
    /// Override the journal (default .canic/apps/APP/fleets/FLEET); without config, .canic.
    #[arg(long)]
    pub(super) state_dir: Option<PathBuf>,
    /// Fleet config; defaults to canic.json when present. No config keeps the legacy journal flow.
    #[arg(long)]
    pub(super) config: Option<PathBuf>,
    /// Select an app; required when the config has multiple apps.
    #[arg(long, conflicts_with = "no_config")]
    pub(super) app: Option<String>,
    /// Select a fleet within the app; required when the app has multiple fleets.
    #[arg(long, conflicts_with = "no_config")]
    pub(super) fleet: Option<String>,
    /// Ignore canic.json and use the legacy .canic journal unless --state-dir is supplied.
    #[arg(long, conflicts_with = "config")]
    pub(super) no_config: bool,
}

#[derive(Subcommand)]
pub(super) enum AppCommand {
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
pub(super) struct AppConfigArgs {
    #[command(flatten)]
    pub(super) config: ConfigArgs,
    /// Select an app; required when the config has multiple apps.
    #[arg(long)]
    pub(super) app: Option<String>,
}

#[derive(Subcommand)]
pub(super) enum AppCanisterCommand {
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
pub(super) enum ArtifactCommand {
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
pub(super) enum FleetCommand {
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
        /// Named slot for creation or its first Root-owned upgrade; repeats reuse its plan.
        #[arg(long, value_name = "SLOT", conflicts_with = "no_config")]
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
        #[arg(long, value_name = "SHA256", requires = "canister")]
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
pub(super) struct CanisterConfigArgs {
    #[command(flatten)]
    pub(super) config: ConfigArgs,
    /// Select an app; required when the config has multiple apps.
    #[arg(long)]
    pub(super) app: Option<String>,
    /// Select a fleet within the app; required when the app has multiple fleets.
    #[arg(long)]
    pub(super) fleet: Option<String>,
}

#[derive(Subcommand)]
pub(super) enum CanisterCommand {
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
    pub(super) fn scope(&self) -> Option<&FleetScope> {
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
pub(super) enum NetworkName {
    Ic,
    Local,
}

#[derive(Args, Clone)]
pub(super) struct NetworkArgs {
    /// Explicit network; planning otherwise uses the selected fleet's binding.
    #[arg(long, value_enum)]
    pub(super) network: Option<NetworkName>,
    /// Local replica URL; --network local requires all three trust inputs.
    #[arg(long, required_if_eq("network", "local"))]
    pub(super) url: Option<String>,
    /// Local replica root key as hex.
    #[arg(long, required_if_eq("network", "local"))]
    pub(super) root_key: Option<String>,
    /// Local routing canister principal.
    #[arg(long, required_if_eq("network", "local"))]
    pub(super) routing_canister: Option<String>,
}

impl NetworkArgs {
    pub(super) fn into_network(self) -> canic_host::Result<Network> {
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

pub(super) fn help_command() -> clap::Command {
    fn banner(command: clap::Command) -> clap::Command {
        command
            .display_order(None)
            .before_help(concat!("Canic ", env!("CARGO_PKG_VERSION")))
            .mut_subcommands(banner)
    }
    banner(Cli::command())
}
