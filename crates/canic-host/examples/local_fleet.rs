//! Public-library consumer: one foreground local Fleet owner with bounded JSON-line commands.

use canic_host::{
    durable_io,
    fleet_ensure::{FleetGenerateRequest, model::DesiredFleet},
    frontend::{self, model::FrontendEnvironmentInput},
    local_fleet::{
        model::{LocalAllocationInput, LocalFleetConfig},
        workflow::LocalFleetSession,
    },
};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

/// A caller selects all effects explicitly; the example owns no application build recipe.
#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Advance {
        session: String,
        seconds: u32,
    },
    Allocate {
        input: LocalAllocationInput,
    },
    Converge {
        desired: PathBuf,
    },
    Discover {
        fleet: String,
    },
    Frontend {
        fleet: String,
        input: PathBuf,
        out: PathBuf,
    },
    Generate {
        app_config: PathBuf,
        fleet: String,
        release: String,
        seed: PathBuf,
        source: PathBuf,
        out: PathBuf,
    },
    Restart {
        session: String,
    },
    Seed {
        source: PathBuf,
        seed: PathBuf,
        creation_fee_cycles: u128,
    },
    Shutdown {
        session: String,
    },
    Status,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, root, config, icp] if mode == "run" => {
            let root = Path::new(root).canonicalize()?;
            let config: LocalFleetConfig = serde_json::from_slice(
                &durable_io::read_regular_bytes(&root.join(config), 65_536)?)?;
            run(&root, &config, icp)
        }
        [mode, root, name, session] if mode == "reset" => {
            LocalFleetSession::reset(Path::new(root), name, session)?;
            emit(&serde_json::json!({"reset": "complete", "session": session}))
        }
        _ => Err("usage: local_fleet run <workspace> <config.json> <icp-executable> | reset <workspace> <name> <session>".into()),
    }
}

fn run(root: &Path, config: &LocalFleetConfig, icp: &str) -> Result<(), Box<dyn Error>> {
    let mut session = LocalFleetSession::open(root, config)?;
    emit(&session.status()?)?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = Vec::new();
        if input.by_ref().take(65_537).read_until(b'\n', &mut line)? == 0 {
            let identity = session.status()?.session_id;
            session.shutdown(&identity)?;
            return Ok(());
        }
        if line.len() > 65_536 {
            return Err("command exceeds 64 KiB".into());
        }
        let command: Command = serde_json::from_slice(&line)?;
        if let Command::Shutdown { session: identity } = command {
            match session.shutdown(&identity) {
                Ok(()) => return emit(&serde_json::json!({"shutdown": "complete"})),
                Err(error) => {
                    emit(&serde_json::json!({"error": error.to_string()}))?;
                    continue;
                }
            }
        }
        // A failed command leaves the same live owner available for inspection and exact retry.
        if let Err(error) = execute(&mut session, root, icp, command) {
            emit(&serde_json::json!({"error": error.to_string()}))?;
        }
    }
}

fn execute(
    session: &mut LocalFleetSession,
    root: &Path,
    icp: &str,
    command: Command,
) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Advance {
            session: identity,
            seconds,
        } => {
            session.advance_time(&identity, seconds)?;
            emit(&session.status()?)
        }
        Command::Allocate { input } => emit(&session.allocate(&input)?),
        Command::Converge { desired } => {
            let desired: DesiredFleet = toml::from_slice(&durable_io::read_regular_bytes(
                &root.join(desired),
                4 * 1024 * 1024,
            )?)?;
            emit(&session.converge_fleet(root, &desired, icp)?)
        }
        Command::Discover { fleet } => emit(&session.discover(root, &fleet)?),
        Command::Frontend { fleet, input, out } => {
            let input: FrontendEnvironmentInput = serde_json::from_slice(
                &durable_io::read_regular_bytes(&root.join(input), 65_536)?,
            )?;
            let status = session.status()?;
            let gateway_matches =
                input.api_origin.trim_end_matches('/') == status.gateway.trim_end_matches('/');
            if input.environment != status.environment || !gateway_matches {
                return Err(
                    "frontend must select this local session's environment and gateway".into(),
                );
            }
            emit(&frontend::workflow::prepare_handoff(
                root,
                &fleet,
                &input,
                &root.join(out),
            )?)
        }
        Command::Generate {
            app_config,
            fleet,
            release,
            seed,
            source,
            out,
        } => {
            let environment = session.status()?.environment;
            let generated = session.generate_fleet(&FleetGenerateRequest {
                app_config: &root.join(app_config),
                environment: &environment,
                fleet: &fleet,
                icp_executable: icp,
                release_build_id: release.parse()?,
                root,
                seed: &root.join(seed),
                source: &root.join(source),
            })?;
            durable_io::write_bytes(
                &root.join(out),
                toml::to_string_pretty(&generated.desired)?.as_bytes(),
            )?;
            emit(&generated.desired)
        }
        Command::Restart { session: identity } => {
            session.restart(&identity)?;
            emit(&session.status()?)
        }
        Command::Seed {
            source,
            seed,
            creation_fee_cycles,
        } => {
            canic_host::fleet_ensure::initialize_fresh_estate_seed(
                &canic_host::fleet_ensure::FreshEstateSeedRequest {
                    cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai",
                    management_creation_fee_cycles: creation_fee_cycles,
                    seed: &root.join(seed),
                    source: &root.join(source),
                },
            )?;
            emit(&serde_json::json!({"seed": "retained"}))
        }
        Command::Shutdown { .. } => unreachable!("shutdown is handled in run"),
        Command::Status => emit(&session.status()?),
    }
}

fn emit(value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, value)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}
