//! Canic command line interface.

use canic_host::{Error, Network, PlanInput};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Build, review and deploy one Canic application")]
struct Cli {
    /// Durable operation directory. Preserve it across interruption and recovery.
    #[arg(long, global = true, default_value = ".canic")]
    state_dir: PathBuf,
    /// Emit structured JSON on stdout.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build an application Wasm using the locked Cargo graph.
    Build {
        #[arg(long, default_value = "Cargo.toml")]
        manifest: PathBuf,
        #[arg(long, default_value = "canic-hello")]
        package: String,
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,
    },
    /// Inspect the exact bytes to be installed.
    Inspect { wasm: PathBuf },
    /// Retain a reviewable deployment plan without network mutations.
    Plan {
        #[arg(long)]
        wasm: PathBuf,
        #[command(flatten)]
        network: NetworkArgs,
        #[arg(long)]
        controller: String,
        /// Existing canister to upgrade, preserving its principal.
        #[arg(long, requires = "previous_module")]
        target: Option<String>,
        /// Certified module hash from the previous deployment receipt.
        #[arg(long, requires = "target")]
        previous_module: Option<String>,
        /// Creation debit ceiling, including ledger fee; defaults to 1T for creation, zero for upgrade.
        #[arg(long, value_parser = clap::value_parser!(u64).range(0..=5_000_000_000_000))]
        cycles: Option<u64>,
    },
    /// Approve this exact digest and apply or resume its original signed attempts.
    Apply {
        digest: String,
        #[arg(long)]
        identity: String,
        /// ICP CLI executable used only to sign offline. Identity passwords stay in its terminal prompt.
        #[arg(long, default_value = "icp")]
        icp_bin: PathBuf,
    },
    /// Inspect local retained progress; makes no network calls.
    Status { digest: String },
    /// Call the deployed demo's read-only, uncertified greeting query.
    Call {
        digest: String,
        #[arg(long, default_value = "team")]
        name: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum NetworkName {
    Ic,
    Local,
}

#[derive(Args)]
struct NetworkArgs {
    #[arg(long, value_enum)]
    network: NetworkName,
    #[arg(long, required_if_eq("network", "local"))]
    url: Option<String>,
    #[arg(long, required_if_eq("network", "local"))]
    root_key: Option<String>,
    #[arg(long, required_if_eq("network", "local"))]
    routing_canister: Option<String>,
}

impl NetworkArgs {
    fn into_network(self) -> canic_host::Result<Network> {
        match self.network {
            NetworkName::Ic
                if self.url.is_none()
                    && self.root_key.is_none()
                    && self.routing_canister.is_none() =>
            {
                Ok(Network::Mainnet)
            }
            NetworkName::Local => Ok(Network::Local {
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
            _ => Err(Error::Invalid(
                "mainnet uses its fixed URL and compiled trust root".into(),
            )),
        }
    }
}

async fn run(cli: &Cli) -> canic_host::Result<serde_json::Value> {
    match &cli.command {
        Commands::Build {
            manifest,
            package,
            features,
        } => {
            let path = canic_host::build(manifest, package, features)?;
            let artifact = canic_host::inspect(&path)?;
            Ok(serde_json::json!({"wasm": path, "artifact": artifact}))
        }
        Commands::Inspect { wasm } => Ok(serde_json::json!(canic_host::inspect(wasm)?)),
        Commands::Plan {
            wasm,
            network,
            controller,
            target,
            previous_module,
            cycles,
        } => {
            let network = NetworkArgs {
                network: network.network,
                url: network.url.clone(),
                root_key: network.root_key.clone(),
                routing_canister: network.routing_canister.clone(),
            }
            .into_network()?;
            let (digest, plan) = canic_host::create_plan(
                &cli.state_dir,
                wasm,
                PlanInput {
                    network,
                    controller: controller.clone(),
                    target: target.clone(),
                    previous_module: previous_module.clone(),
                    creation_cycles: cycles.unwrap_or(if target.is_some() {
                        0
                    } else {
                        1_000_000_000_000
                    }),
                },
            )?;
            Ok(serde_json::json!({"operation": digest, "plan": plan}))
        }
        Commands::Apply {
            digest,
            identity,
            icp_bin,
        } => Ok(serde_json::json!(
            canic_host::apply(&cli.state_dir, digest, identity, icp_bin).await?
        )),
        Commands::Status { digest } => canic_host::status(&cli.state_dir, digest),
        Commands::Call { digest, name } => {
            let state = canic_host::status(&cli.state_dir, digest)?;
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
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli).await {
        Ok(value) => {
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
