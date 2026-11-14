//! Application-owned profile client; ICP CLI owns identity and network submission.
mod growth;
use canic_profile_protocol::{
    BIND, CHECK, CheckResult, GET, INFO, Info, LIMITS, PUT, ProfileResult, Put, ROUTE, Route,
    RouteResult,
};
use clap::{ArgGroup, Parser, Subcommand};
use ic_host_process::{
    child::OwnedChild,
    tool::{OutputLimit, OutputLimits, SuccessfulExit, communicate_child},
};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Parser)]
#[command(version, about = "Use the Canic profile demo", group(ArgGroup::new("destination").required(true).args(["canister", "entry", "manager"])))]
struct Cli {
    /// Deployed profile worker principal.
    #[arg(long)]
    canister: Option<String>,
    /// Discover the worker through this entry's certified replicated resolution.
    #[arg(long)]
    entry: Option<String>,
    /// One-slot autonomous worker manager.
    #[arg(long)]
    manager: Option<String>,
    /// ICP identity; enter its password only in ICP CLI's terminal prompt.
    #[arg(long, default_value = "canic-mainnet")]
    identity: String,
    /// ICP network name or explicit local URL.
    #[arg(long, default_value = "ic")]
    network: String,
    /// Explicit root key when selecting a local URL.
    #[arg(long)]
    root_key: Option<String>,
    /// ICP CLI executable. No shell command is evaluated.
    #[arg(long, default_value = ".tools/ic/bin/icp")]
    icp_bin: PathBuf,
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Retain an exact growth approval and Wasm; makes no network mutations.
    GrowthPlan {
        #[arg(long)]
        wasm: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(10..=3600))]
        delay_seconds: u64,
    },
    /// Approve the retained growth frame; the manager admits at most one slot.
    GrowthApprove { approval: PathBuf },
    /// Read public, uncertified manager evidence without making effects.
    GrowthStatus,
    /// Observe uncertain upload/install outcomes; never repeat an issued effect.
    GrowthReconcile,
    /// Read the entry's route using replicated resolution (requires --entry).
    Route,
    /// Bind an entry once; requires its controller identity and --entry.
    Bind { worker: String },
    /// Check worker readiness through the entry; requires controller and --entry.
    Check,
    /// Create or change your profile. Exact revision/value replay has no new effect.
    Put {
        #[arg(value_parser = clap::value_parser!(u64).range(1..))]
        user_id: u64,
        #[arg(long, default_value_t = 0)]
        revision: u64,
        #[arg(long)]
        name: String,
    },
    /// Read your profile through an authenticated but uncertified query.
    Get { user_id: u64 },
    /// Read public, uncertified application readiness and fixed limits.
    Info,
}

fn run(cli: &Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if matches!(
        cli.command,
        Action::GrowthPlan { .. }
            | Action::GrowthApprove { .. }
            | Action::GrowthStatus
            | Action::GrowthReconcile
    ) {
        return growth::run(cli);
    }
    if cli.manager.is_some() {
        return Err("profile commands require --entry or --canister".into());
    }
    let (method, id, request, query) = match &cli.command {
        Action::GrowthPlan { .. }
        | Action::GrowthApprove { .. }
        | Action::GrowthStatus
        | Action::GrowthReconcile => unreachable!("growth commands handled above"),
        Action::Route => (
            "resolve",
            ROUTE,
            ic_wire::native::encode_request(ROUTE, &(), LIMITS)?,
            false,
        ),
        Action::Bind { worker } => (
            "bind",
            BIND,
            ic_wire::native::encode_request(
                BIND,
                &Route {
                    worker: worker.clone(),
                },
                LIMITS,
            )?,
            false,
        ),
        Action::Check => (
            "check",
            CHECK,
            ic_wire::native::encode_request(CHECK, &(), LIMITS)?,
            false,
        ),
        Action::Put {
            user_id,
            revision,
            name,
        } => {
            if name.trim().is_empty() || name.len() > canic_profile_protocol::MAX_NAME_BYTES {
                return Err("display name must contain 1–128 UTF-8 bytes".into());
            }
            (
                "put",
                PUT,
                ic_wire::native::encode_request(
                    PUT,
                    &Put {
                        user_id: *user_id,
                        expected_revision: *revision,
                        display_name: name.clone(),
                    },
                    LIMITS,
                )?,
                false,
            )
        }
        Action::Get { user_id } => (
            "get",
            GET,
            ic_wire::native::encode_request(GET, user_id, LIMITS)?,
            true,
        ),
        Action::Info => (
            "info",
            INFO,
            ic_wire::native::encode_request(INFO, &(), LIMITS)?,
            true,
        ),
    };
    let target = if matches!(
        cli.command,
        Action::Route | Action::Bind { .. } | Action::Check
    ) {
        cli.entry.clone().ok_or("this command requires --entry")?
    } else if let Some(entry) = &cli.entry {
        let request = ic_wire::native::encode_request(ROUTE, &(), LIMITS)?;
        let reply = invoke(cli, entry, "resolve", &request, false)?;
        let route = ic_wire::native::decode_response::<RouteResult>(ROUTE, &reply, LIMITS)?
            .map_err(|error| format!("entry refused resolution: {error:?}"))?;
        route.worker
    } else {
        cli.canister.clone().expect("Clap requires destination")
    };
    let reply = invoke(cli, &target, method, &request, query)?;
    let result = match id {
        INFO => serde_json::to_value(ic_wire::native::decode_response::<Info>(
            id, &reply, LIMITS,
        )?)?,
        ROUTE | BIND => serde_json::to_value(
            ic_wire::native::decode_response::<RouteResult>(id, &reply, LIMITS)?
                .map_err(|error| format!("entry refused: {error:?}"))?,
        )?,
        CHECK => serde_json::to_value(
            ic_wire::native::decode_response::<CheckResult>(id, &reply, LIMITS)?.map_err(
                |error| format!("entry check failed: {error:?}; worker health is unknown"),
            )?,
        )?,
        _ => serde_json::to_value(
            ic_wire::native::decode_response::<ProfileResult>(id, &reply, LIMITS)?
                .map_err(|error| format!("profile refused: {error:?}"))?,
        )?,
    };
    Ok(
        serde_json::json!({"canister": target, "entry": cli.entry, "query": query, "certified": !query, "result": result}),
    )
}

fn invoke(
    cli: &Cli,
    target: &str,
    method: &str,
    request: &[u8],
    query: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let args: String = request.iter().map(|byte| format!("{byte:02x}")).collect();
    invoke_arguments(cli, target, method, Some(&args), None, query)
}

fn invoke_arguments(
    cli: &Cli,
    target: &str,
    method: &str,
    args: Option<&str>,
    file: Option<&std::path::Path>,
    query: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut command = Command::new(&cli.icp_bin);
    command.args([
        "canister",
        "call",
        "--args-format",
        if file.is_some() { "bin" } else { "hex" },
        "--output",
        "hex",
        "--network",
        &cli.network,
        "--identity",
        &cli.identity,
    ]);
    if let Some(key) = &cli.root_key {
        command.args(["--root-key", key]);
    }
    if query {
        command.arg("--query");
    }
    // Keep discovered text in positional arguments, never in ICP's option parser.
    if let Some(path) = file {
        command.arg("--args-file").arg(path);
    }
    command.args(["--", target, method]);
    if let Some(args) = args {
        command.arg(args);
    }
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    // Stay in the foreground process group so ICP's terminal password prompt works.
    let mut child = OwnedChild::spawn_direct(&mut command)?;
    let output = communicate_child(
        &mut child,
        None,
        OutputLimits {
            stdout: OutputLimit::Terminate(16384),
            stderr: OutputLimit::Terminate(16384),
            timeout: Some(Duration::from_secs(300)),
        },
        SuccessfulExit::Cleanup,
        || false,
    )?
    .require_success()?
    .require_complete()?;
    let text = std::str::from_utf8(&output.stdout)?.trim();
    let hex = text.strip_prefix("0x").unwrap_or(text);
    if !hex.is_ascii() || !hex.len().is_multiple_of(2) {
        return Err("ICP CLI returned invalid hex".into());
    }
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair)?, 16).map_err(Into::into))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()
}

fn main() -> std::process::ExitCode {
    match run(&Cli::parse()) {
        Ok(value) => {
            println!("{value:#}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{error}\nNo automatic retry was made. Inspect retained profile or growth evidence before submitting more work."
            );
            std::process::ExitCode::FAILURE
        }
    }
}
