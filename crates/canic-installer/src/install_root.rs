use crate::release_set::{
    config_path, configured_release_roles, dfx_call, dfx_root, emit_root_release_set_manifest,
    load_root_release_set_manifest, resolve_artifact_root, resume_root_bootstrap,
    root_release_set_manifest_path, stage_root_release_set, workspace_root,
};
use canic_core::protocol;
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub root_canister: String,
    pub network: String,
    pub ready_timeout_seconds: u64,
}

///
/// BootstrapStatusSnapshot
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct BootstrapStatusSnapshot {
    ready: bool,
    phase: String,
    last_error: Option<String>,
}

///
/// InstallTimingSummary
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InstallTimingSummary {
    create_canisters: Duration,
    build_all: Duration,
    emit_manifest: Duration,
    fabricate_cycles: Duration,
    install_root: Duration,
    stage_release_set: Duration,
    resume_bootstrap: Duration,
    wait_ready: Duration,
}

const LOCAL_ROOT_TARGET_CYCLES: u128 = 9_000_000_000_000_000;

impl InstallRootOptions {
    // Resolve the current local-root install options from args and environment.
    #[must_use]
    pub fn from_env_and_args() -> Self {
        Self {
            root_canister: env::args()
                .nth(1)
                .or_else(|| env::var("ROOT_CANISTER").ok())
                .unwrap_or_else(|| "root".to_string()),
            network: env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string()),
            ready_timeout_seconds: env::var("READY_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(120),
        }
    }
}

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    let total_started_at = Instant::now();
    let mut timings = InstallTimingSummary::default();

    println!("Installing root against DFX_NETWORK={}", options.network);
    require_dfx_running(&options.network)?;
    let mut create = Command::new("dfx");
    create
        .current_dir(&dfx_root)
        .args(["canister", "create", "--all", "-qq"]);
    let create_started_at = Instant::now();
    run_command(&mut create)?;
    timings.create_canisters = create_started_at.elapsed();

    let build_targets = local_install_build_targets(&workspace_root, &options.root_canister)?;
    let mut build = dfx_build_targets_command(&dfx_root, &build_targets);
    let build_started_at = Instant::now();
    run_command(&mut build)?;
    timings.build_all = build_started_at.elapsed();

    let emit_manifest_started_at = Instant::now();
    let manifest_path =
        emit_root_release_set_manifest(&workspace_root, &dfx_root, &options.network)?;
    timings.emit_manifest = emit_manifest_started_at.elapsed();

    timings.fabricate_cycles =
        maybe_fabricate_local_cycles(&dfx_root, &options.root_canister, &options.network)?;

    let mut install = Command::new("dfx");
    install.current_dir(&dfx_root).args([
        "canister",
        "install",
        &options.root_canister,
        "--mode=reinstall",
        "-y",
        "--argument",
        "(variant { Prime })",
    ]);
    let install_started_at = Instant::now();
    run_command(&mut install)?;
    timings.install_root = install_started_at.elapsed();

    let artifact_root = resolve_artifact_root(&dfx_root, &options.network)?;
    let manifest =
        load_root_release_set_manifest(&root_release_set_manifest_path(&artifact_root)?)?;
    assert_eq!(
        manifest_path,
        root_release_set_manifest_path(&artifact_root)?
    );
    let stage_started_at = Instant::now();
    stage_root_release_set(&dfx_root, &options.root_canister, &manifest)?;
    timings.stage_release_set = stage_started_at.elapsed();
    let resume_started_at = Instant::now();
    resume_root_bootstrap(&options.root_canister)?;
    timings.resume_bootstrap = resume_started_at.elapsed();
    let ready_started_at = Instant::now();
    let ready_result = wait_for_root_ready(&options.root_canister, options.ready_timeout_seconds);
    timings.wait_ready = ready_started_at.elapsed();
    if let Err(err) = ready_result {
        print_install_timing_summary(&timings, total_started_at.elapsed());
        return Err(err);
    }

    print_install_timing_summary(&timings, total_started_at.elapsed());
    println!("Root installed successfully");
    println!(
        "Smoke check: dfx canister call {} canic_ready",
        options.root_canister
    );
    Ok(())
}

// Resolve the local install build set from the root canister plus the
// configured ordinary roles owned by the root subnet.
fn local_install_build_targets(
    workspace_root: &Path,
    root_canister: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut targets = vec![root_canister.to_string()];
    targets.extend(configured_release_roles(&config_path(workspace_root))?);
    Ok(targets)
}

// Spawn the local `dfx build` step for only the configured root install targets
// without overriding the caller's selected build profile environment.
fn dfx_build_targets_command(dfx_root: &Path, targets: &[String]) -> Command {
    let mut command = Command::new("dfx");
    command.current_dir(dfx_root).arg("build").args(targets);
    command
}

// Top up local root cycles only when the current balance is below the target floor.
fn maybe_fabricate_local_cycles(
    dfx_root: &Path,
    root_canister: &str,
    network: &str,
) -> Result<Duration, Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(Duration::ZERO);
    }

    let current_balance = root_cycle_balance(dfx_root, root_canister)?;
    let Some(fabricate_cycles) = required_local_cycle_topup(current_balance) else {
        println!(
            "Skipping local cycle fabrication for {root_canister}; balance {} already meets target {}",
            format_cycles(current_balance),
            format_cycles(LOCAL_ROOT_TARGET_CYCLES)
        );
        return Ok(Duration::ZERO);
    };

    println!(
        "Fabricating {} cycles locally for {root_canister} to reach target {} (current balance {})",
        format_cycles(fabricate_cycles),
        format_cycles(LOCAL_ROOT_TARGET_CYCLES),
        format_cycles(current_balance)
    );

    let mut fabricate = Command::new("dfx");
    fabricate.current_dir(dfx_root);
    fabricate.args([
        "ledger",
        "fabricate-cycles",
        "--canister",
        root_canister,
        "--cycles",
        &fabricate_cycles.to_string(),
    ]);
    let fabricate_started_at = Instant::now();
    let _ = run_command_allow_failure(&mut fabricate)?;

    Ok(fabricate_started_at.elapsed())
}

// Read the current root canister cycle balance from `dfx canister status`.
fn root_cycle_balance(
    dfx_root: &Path,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let output = Command::new("dfx")
        .current_dir(dfx_root)
        .args(["canister", "status", root_canister])
        .output()?;
    if !output.status.success() {
        return Err(format!("dfx canister status failed: {}", output.status).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    parse_canister_status_cycles(&stdout)
        .ok_or_else(|| "could not parse cycle balance from `dfx canister status` output".into())
}

// Parse the cycle balance from the human-readable `dfx canister status` output.
fn parse_canister_status_cycles(status_output: &str) -> Option<u128> {
    status_output
        .lines()
        .find_map(parse_canister_status_balance_line)
}

fn parse_canister_status_balance_line(line: &str) -> Option<u128> {
    let (label, value) = line.trim().split_once(':')?;
    let label = label.trim().to_ascii_lowercase();
    if label != "balance" && label != "cycle balance" {
        return None;
    }

    let digits = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }

    digits.parse::<u128>().ok()
}

// Return the local top-up delta needed to bring root up to the target cycle floor.
fn required_local_cycle_topup(current_balance: u128) -> Option<u128> {
    (current_balance < LOCAL_ROOT_TARGET_CYCLES)
        .then_some(LOCAL_ROOT_TARGET_CYCLES.saturating_sub(current_balance))
        .filter(|cycles| *cycles > 0)
}

fn format_cycles(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + (digits.len().saturating_sub(1) / 3));
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

// Fail fast unless the requested DFX replica is already running.
fn require_dfx_running(network: &str) -> Result<(), Box<dyn std::error::Error>> {
    let result = Command::new("dfx").args(["ping", network]).output()?;
    if result.status.success() {
        return Ok(());
    }

    Err(format!(
        "dfx replica is not running for network '{network}'\nStart the target replica externally and rerun."
    )
    .into())
}

// Wait until root reports ready, printing periodic progress and diagnostics.
fn wait_for_root_ready(
    root_canister: &str,
    timeout_seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let mut next_report = 0_u64;

    println!("Waiting for {root_canister} to report canic_ready (timeout {timeout_seconds}s)");

    loop {
        if root_ready(root_canister)? {
            println!(
                "{root_canister} reported canic_ready after {}s",
                start.elapsed().as_secs()
            );
            return Ok(());
        }

        if let Some(status) = root_bootstrap_status(root_canister)?
            && let Some(last_error) = status.last_error.as_deref()
        {
            eprintln!(
                "root bootstrap reported failure during phase '{}' : {}",
                status.phase, last_error
            );
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_bootstrap_status");
            print_raw_call(root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_subnet_registry");
            print_raw_call(root_canister, "canic_subnet_registry");
            eprintln!(
                "Diagnostic: dfx canister call {root_canister} canic_wasm_store_bootstrap_debug"
            );
            print_raw_call(root_canister, "canic_wasm_store_bootstrap_debug");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_wasm_store_overview");
            print_raw_call(root_canister, "canic_wasm_store_overview");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_log");
            print_recent_root_logs(root_canister);
            return Err(format!(
                "root bootstrap failed during phase '{}' : {}",
                status.phase, last_error
            )
            .into());
        }

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_bootstrap_status");
            print_raw_call(root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_subnet_registry");
            print_raw_call(root_canister, "canic_subnet_registry");
            eprintln!(
                "Diagnostic: dfx canister call {root_canister} canic_wasm_store_bootstrap_debug"
            );
            print_raw_call(root_canister, "canic_wasm_store_bootstrap_debug");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_wasm_store_overview");
            print_raw_call(root_canister, "canic_wasm_store_overview");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_log");
            print_recent_root_logs(root_canister);
            return Err("root did not become ready".into());
        }

        if elapsed >= next_report {
            println!("Still waiting for {root_canister} canic_ready ({elapsed}s elapsed)");
            if let Some(status) = root_bootstrap_status(root_canister)? {
                match status.last_error.as_deref() {
                    Some(last_error) => println!(
                        "Current bootstrap status: phase={} ready={} error={}",
                        status.phase, status.ready, last_error
                    ),
                    None => println!(
                        "Current bootstrap status: phase={} ready={}",
                        status.phase, status.ready
                    ),
                }
            }
            if let Ok(registry_json) =
                dfx_call(root_canister, "canic_subnet_registry", None, Some("json"))
            {
                println!("Current subnet registry roles:");
                println!("  {}", registry_roles(&registry_json));
            }
            next_report = elapsed + 5;
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// Return true once root reports `canic_ready == true`.
fn root_ready(root_canister: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = dfx_call(root_canister, "canic_ready", None, Some("json"))?;
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(parse_root_ready_value(&data))
}

// Return the current root bootstrap diagnostic state when the query is available.
fn root_bootstrap_status(
    root_canister: &str,
) -> Result<Option<BootstrapStatusSnapshot>, Box<dyn std::error::Error>> {
    let output = match dfx_call(
        root_canister,
        protocol::CANIC_BOOTSTRAP_STATUS,
        None,
        Some("json"),
    ) {
        Ok(output) => output,
        Err(err) => {
            let message = err.to_string();
            if message.contains("has no query method")
                || message.contains("method not found")
                || message.contains("Canister has no query method")
            {
                return Ok(None);
            }
            return Err(err);
        }
    };
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(parse_bootstrap_status_value(&data))
}

// Accept both plain-bool and wrapped-result JSON shapes from `dfx --output json`.
fn parse_root_ready_value(data: &Value) -> bool {
    matches!(data, Value::Bool(true)) || matches!(data.get("Ok"), Some(Value::Bool(true)))
}

fn parse_bootstrap_status_value(data: &Value) -> Option<BootstrapStatusSnapshot> {
    serde_json::from_value::<BootstrapStatusSnapshot>(data.clone())
        .ok()
        .or_else(|| {
            data.get("Ok")
                .cloned()
                .and_then(|ok| serde_json::from_value::<BootstrapStatusSnapshot>(ok).ok())
        })
}

fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{:<20} {:>10}", "phase", "elapsed");
    println!("{:<20} {:>10}", "--------------------", "----------");
    print_timing_row("create_canisters", timings.create_canisters);
    print_timing_row("build_all", timings.build_all);
    print_timing_row("emit_manifest", timings.emit_manifest);
    print_timing_row("fabricate_cycles", timings.fabricate_cycles);
    print_timing_row("install_root", timings.install_root);
    print_timing_row("stage_release_set", timings.stage_release_set);
    print_timing_row("resume_bootstrap", timings.resume_bootstrap);
    print_timing_row("wait_ready", timings.wait_ready);
    print_timing_row("total", total);
}

fn print_timing_row(label: &str, duration: Duration) {
    println!("{label:<20} {:>9.2}s", duration.as_secs_f64());
}

// Print recent structured root log entries without raw byte dumps.
fn print_recent_root_logs(root_canister: &str) {
    let page_args = r"(null, null, null, record { limit = 8; offset = 0 })";
    let Ok(logs_json) = dfx_call(root_canister, "canic_log", Some(page_args), Some("json")) else {
        return;
    };
    let Ok(data) = serde_json::from_str::<Value>(&logs_json) else {
        return;
    };
    let entries = data
        .get("Ok")
        .and_then(|ok| ok.get("entries"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if entries.is_empty() {
        println!("  <no runtime log entries>");
        return;
    }

    for entry in entries.iter().rev() {
        let level = entry.get("level").and_then(Value::as_str).unwrap_or("Info");
        let topic = entry.get("topic").and_then(Value::as_str).unwrap_or("");
        let message = entry
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace('\n', "\\n");
        let topic_prefix = if topic.is_empty() {
            String::new()
        } else {
            format!("[{topic}] ")
        };
        println!("  {level} {topic_prefix}{message}");
    }
}

// Render the current subnet registry roles from one JSON response.
fn registry_roles(registry_json: &str) -> String {
    serde_json::from_str::<Value>(registry_json)
        .ok()
        .and_then(|data| {
            data.get("Ok").and_then(Value::as_array).map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .get("role")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .map_or_else(
            || "<unavailable>".to_string(),
            |roles| {
                if roles.is_empty() {
                    "<empty>".to_string()
                } else {
                    roles.join(", ")
                }
            },
        )
}

// Run one command and require a zero exit status.
fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed: {status}").into())
    }
}

// Run one command and return its status without failing the caller on non-zero exit.
fn run_command_allow_failure(
    command: &mut Command,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    Ok(command.status()?)
}

// Print one raw fallback `dfx canister call` result to stderr for diagnostics.
fn print_raw_call(root_canister: &str, method: &str) {
    let mut command = Command::new("dfx");
    if let Ok(root) = dfx_root() {
        command.current_dir(root);
    }
    let _ = command
        .args(["canister", "call", root_canister, method])
        .status();
}

#[cfg(test)]
mod tests {
    use super::{
        LOCAL_ROOT_TARGET_CYCLES, dfx_build_targets_command, local_install_build_targets,
        parse_bootstrap_status_value, parse_canister_status_cycles, parse_root_ready_value,
        required_local_cycle_topup,
    };
    use serde_json::json;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parse_root_ready_accepts_plain_true() {
        assert!(parse_root_ready_value(&json!(true)));
    }

    #[test]
    fn parse_root_ready_accepts_wrapped_ok_true() {
        assert!(parse_root_ready_value(&json!({ "Ok": true })));
    }

    #[test]
    fn parse_root_ready_rejects_false_shapes() {
        assert!(!parse_root_ready_value(&json!(false)));
        assert!(!parse_root_ready_value(&json!({ "Ok": false })));
        assert!(!parse_root_ready_value(&json!({ "Err": "nope" })));
    }

    #[test]
    fn parse_bootstrap_status_accepts_plain_record() {
        let status = parse_bootstrap_status_value(&json!({
            "ready": false,
            "phase": "root:init:create_canisters",
            "last_error": null
        }))
        .expect("plain bootstrap status must parse");

        assert!(!status.ready);
        assert_eq!(status.phase, "root:init:create_canisters");
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn parse_bootstrap_status_accepts_wrapped_ok_record() {
        let status = parse_bootstrap_status_value(&json!({
            "Ok": {
                "ready": false,
                "phase": "failed",
                "last_error": "registry phase failed"
            }
        }))
        .expect("wrapped bootstrap status must parse");

        assert!(!status.ready);
        assert_eq!(status.phase, "failed");
        assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
    }

    #[test]
    fn parse_canister_status_cycles_accepts_balance_line() {
        let output = "\
Canister status call result for root.
Status: Running
Balance: 9_002_999_998_056_000 Cycles
Memory Size: 1_234_567 Bytes
";

        assert_eq!(
            parse_canister_status_cycles(output),
            Some(9_002_999_998_056_000)
        );
    }

    #[test]
    fn parse_canister_status_cycles_accepts_cycle_balance_line() {
        let output = "\
Canister status call result for root.
Cycle balance: 12_345 Cycles
";

        assert_eq!(parse_canister_status_cycles(output), Some(12_345));
    }

    #[test]
    fn required_local_cycle_topup_skips_when_balance_already_meets_target() {
        assert_eq!(required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES), None);
        assert_eq!(
            required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES + 1_000),
            None
        );
    }

    #[test]
    fn required_local_cycle_topup_returns_missing_delta_only() {
        assert_eq!(
            required_local_cycle_topup(3_000_000_000_000),
            Some(8_997_000_000_000_000)
        );
    }

    #[test]
    fn dfx_build_command_targets_only_requested_canisters() {
        let targets = vec![
            "root".to_string(),
            "app".to_string(),
            "user_hub".to_string(),
        ];
        let command = dfx_build_targets_command(Path::new("/tmp/canic-dfx-root"), &targets);

        assert_eq!(command.get_program(), "dfx");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["build", "root", "app", "user_hub"]
        );
        assert_eq!(
            command
                .get_current_dir()
                .map(|path| path.to_string_lossy().into_owned()),
            Some("/tmp/canic-dfx-root".to_string())
        );
        assert!(
            command.get_envs().next().is_none(),
            "dfx build must not override profile env"
        );
    }

    #[test]
    fn local_install_build_targets_use_root_subnet_release_roles_only() {
        let workspace_root = write_temp_workspace_config(
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.project_registry]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.extra.canisters.oracle_pokemon]
kind = "singleton"
"#,
        );

        let targets =
            local_install_build_targets(&workspace_root, "root").expect("targets must resolve");

        assert_eq!(
            targets,
            vec![
                "root".to_string(),
                "project_registry".to_string(),
                "user_hub".to_string()
            ]
        );
    }

    fn write_temp_workspace_config(config_source: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for test temp dir")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "canic-install-root-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("canisters")).expect("temp canisters dir must be created");
        fs::write(root.join("canisters/canic.toml"), config_source)
            .expect("temp canic.toml must be written");
        root
    }
}
