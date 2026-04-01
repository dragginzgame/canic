use canic_installer::release_set::{
    dfx_call, emit_root_release_set_manifest, load_root_release_set_manifest,
    resolve_artifact_root, resume_root_bootstrap, root_release_set_manifest_path,
    stage_root_release_set, workspace_root,
};
use serde_json::Value;
use std::{env, process::Command, thread, time::Duration};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Execute the local reference-topology install flow against an already running replica.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root_canister = env::args()
        .nth(1)
        .or_else(|| env::var("ROOT_CANISTER").ok())
        .unwrap_or_else(|| "root".to_string());
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let ready_timeout_seconds = env::var("READY_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(120);
    let workspace_root = workspace_root()?;

    println!("Installing reference topology against DFX_NETWORK={network}");
    require_dfx_running(&network)?;
    run_command(Command::new("dfx").args(["canister", "create", "--all", "-qq"]))?;
    let mut build = Command::new("dfx");
    build.env("RELEASE", "1").args(["build", "--all"]);
    run_command(&mut build)?;
    let manifest_path = emit_root_release_set_manifest(&workspace_root, &network)?;
    let mut fabricate = Command::new("dfx");
    fabricate.args([
        "ledger",
        "fabricate-cycles",
        "--canister",
        &root_canister,
        "--cycles",
        "9000000000000000",
    ]);
    let _ = run_command_allow_failure(&mut fabricate)?;
    run_command(Command::new("dfx").args([
        "canister",
        "install",
        &root_canister,
        "--mode=reinstall",
        "-y",
        "--argument",
        "(variant { Prime })",
    ]))?;

    let artifact_root = resolve_artifact_root(&workspace_root, &network)?;
    let manifest =
        load_root_release_set_manifest(&root_release_set_manifest_path(&artifact_root)?)?;
    assert_eq!(
        manifest_path,
        root_release_set_manifest_path(&artifact_root)?
    );
    stage_root_release_set(&workspace_root, &root_canister, &manifest)?;
    resume_root_bootstrap(&root_canister)?;
    wait_for_root_ready(&root_canister, ready_timeout_seconds)?;

    println!("Reference topology installed successfully");
    println!("Smoke check: dfx canister call {root_canister} canic_ready");
    Ok(())
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

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
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
            if let Ok(registry_json) =
                dfx_call(root_canister, "canic_subnet_registry", None, Some("json"))
            {
                println!("Current subnet registry roles:");
                println!("  {}", registry_roles(&registry_json));
            }
            println!("Recent root logs:");
            print_recent_root_logs(root_canister);
            next_report = elapsed + 5;
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// Return true once root reports `canic_ready == true`.
fn root_ready(root_canister: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = dfx_call(root_canister, "canic_ready", None, Some("json"))?;
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(matches!(data.get("Ok"), Some(Value::Bool(true))))
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
    let _ = Command::new("dfx")
        .args(["canister", "call", root_canister, method])
        .status();
}
