//! Disposable actor rehearsal. Testkit owns the server; normal Canic commands own deployments.
#[path = "../fixtures/local_store.rs"]
mod local_store;
#[path = "interactive/miner.rs"]
mod miner;
#[path = "../fixtures/miner_pool.rs"]
mod miner_pool_fixture;
#[path = "interactive/notebook.rs"]
mod notebook;

use canic::{
    logging::Logs,
    method::Contract,
    metrics::{ObserveChildMetrics, ReadMetrics, Report},
};
use clap::{Parser, ValueEnum};
use ic_host_process::{Input, Ownership, Policy, Stderr, run};
use ic_testkit::{
    pocket_ic::PocketIcBuilder,
    server::{Deletion, Limits, PreparedServer, Server},
};
use rustyline_async::{Readline, ReadlineEvent};
use std::{
    io::{BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

///
/// App
/// Selects the example fleet for this disposable session.
///

#[derive(Clone, Copy, ValueEnum)]
enum App {
    Counter,
    Metrics,
    MinerPool,
    Notebook,
}

#[derive(Parser)]
#[command(about = "Disposable managed example fleet; synthetic cycles, no mainnet effects")]
struct Args {
    /// Runnable example to deploy.
    #[arg(long, value_enum, default_value = "counter")]
    app: App,
    /// Session lifetime, after setup. Each invocation creates a fresh journal directory.
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..=3600))]
    seconds: u64,
}

fn cli(directory: &Path, args: &[&str]) -> Result<serde_json::Value> {
    let mut command = Command::new("canic");
    command.current_dir(directory).arg("--json").args(args);
    let mut output = Vec::new();
    let mut diagnostic = Vec::new();
    let report = run(
        &mut command,
        Policy {
            ownership: Ownership::Group,
            input: Input::Null,
            stdout_bytes: 65_536,
            stderr: Stderr::Capture(65_536),
            elapsed: Duration::from_secs(120),
        },
        &AtomicBool::new(false),
        |stream, bytes| {
            match stream {
                ic_host_process::Stream::Stdout => output.extend_from_slice(bytes),
                ic_host_process::Stream::Stderr => diagnostic.extend_from_slice(bytes),
            }
            Ok(())
        },
    );
    if report.failure.is_some()
        || !report.cleanup.is_empty()
        || !report.status.is_some_and(|status| status.success())
    {
        return Err(format!(
            "canic {} failed ({report:?}): {} {}",
            args.join(" "),
            String::from_utf8_lossy(&output),
            String::from_utf8_lossy(&diagnostic)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output)?)
}

fn fleet_call(directory: &Path, target: &str, command: &[&str]) -> Result<serde_json::Value> {
    let mut args = vec![
        "fleet",
        "call",
        "--canister",
        target,
        "--identity",
        "anonymous",
    ];
    args.extend_from_slice(command);
    let mut call = cli(directory, &args)?;
    Ok(call
        .get_mut("result")
        .ok_or("missing fleet call result")?
        .take())
}

async fn session(
    owner: &mut ic_testkit::server::Instance,
    repo: &Path,
    seconds: u64,
    app_name: App,
) -> Result<()> {
    let build_example = |name: &str| -> Result<PathBuf> {
        let example = repo.join("apps/examples").join(name);
        println!("Building local {name} fleet (synthetic cycles)…");
        cli(&example, &["app", "build"])?;
        Ok(example.join("artifacts"))
    };
    let (name, first, second, app_commands) = match app_name {
        App::MinerPool => {
            return miner::session(owner, &build_example("miner-pool")?, seconds).await;
        }
        App::Notebook => {
            return notebook::session(
                owner.client(),
                &build_example("notebook")?.join("notebook.wasm"),
                seconds,
            )
            .await;
        }
        App::Counter => ("counter", "increment", "read", "increment, read"),
        App::Metrics => ("metrics", "ping", "checksum", "ping [text], checksum TEXT"),
    };
    let url = owner.client_mut().make_live(None).await.to_string();
    let key = owner
        .client()
        .root_key()
        .await
        .ok_or("missing local trust root")?;
    let key: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    let subnet = owner
        .client()
        .topology()
        .await
        .get_nns()
        .ok_or("missing root subnet")?
        .to_text();
    let routing = owner.client().create_canister().await.to_text();
    let parent = repo.join("target/demo");
    std::fs::create_dir_all(&parent)?;
    let directory = parent.join(format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    ));
    std::fs::create_dir(&directory)?;
    let example = repo.join("apps/examples").join(name);
    let mut config: toml::Table = std::fs::read_to_string(example.join("canic.toml"))?.parse()?;
    let app = config["apps"][name]
        .as_table_mut()
        .ok_or("missing example app")?;
    app.remove("fleets");
    for (_, canister) in app["canisters"]
        .as_table_mut()
        .ok_or("missing example components")?
        .iter_mut()
    {
        let manifest = &mut canister["build"]["manifest"];
        let absolute = example
            .join(manifest.as_str().ok_or("missing example manifest")?)
            .canonicalize()?;
        *manifest = toml::Value::try_from(absolute)?;
    }
    std::fs::write(directory.join("canic.toml"), toml::to_string(&config)?)?;
    cli(
        &directory,
        &[
            "fleet",
            "init",
            "local",
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &key,
            "--root-subnet",
            &subnet,
            "--routing-canister",
            &routing,
        ],
    )?;
    println!(
        "Local {name} fleet · synthetic cycles · {}",
        directory.display()
    );
    println!("Building and deploying Root, Store and {name}…");
    let built = cli(&directory, &["app", "build"])?;
    let components = built["canisters"]
        .as_array()
        .ok_or("missing built components")?;
    let exports = |name: &str| -> Result<_> {
        let component = components
            .iter()
            .find(|row| row["canister"] == name)
            .ok_or("missing built component")?;
        let wasm = component["wasm"].as_str().ok_or("missing built Wasm")?;
        Ok(canic_host::endpoints(&directory.join(wasm))?)
    };
    let root = exports("root")?;
    let codec = components
        .iter()
        .any(|row| row["canister"] == "root" && row["client"].is_string());
    let root_command = |name| codec && root.iter().any(|e| e.name == name && e.mode == "update");
    let logs = root_command(Logs::NAME);
    let metrics = root_command(ObserveChildMetrics::NAME)
        && exports(name)?
            .iter()
            .any(|e| e.name == ReadMetrics::NAME && e.mode == "query");
    let plan = cli(&directory, &["fleet", "plan", "--identity", "anonymous"])?;
    let operation = plan["operation"].as_str().ok_or("plan missing operation")?;
    let fleet: canic_host::FleetPlan = serde_json::from_value(plan["fleet_plan"].clone())?;
    let child_slot = fleet
        .children
        .iter()
        .find(|child| child.deployment.canister == name)
        .ok_or("missing demo child in fleet approval")?
        .slot
        .index()
        .to_string();
    println!("Applying {operation}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        let applied = cli(
            &directory,
            &["fleet", "apply", operation, "--identity", "anonymous"],
        )?;
        let child = applied["children"]
            .as_array()
            .and_then(|children| children.iter().find(|child| child["name"] == name))
            .ok_or("missing planned child")?;
        let outcome = &child["outcome"];
        if let Some(id) = outcome["canister"].as_str() {
            println!("{name} deployed at {id}");
            break;
        }
        let status = &outcome["status"];
        if status["phase"]
            .as_str()
            .is_none_or(|phase| matches!(phase, "Expired" | "ReserveBlocked"))
            || !status["failure"].is_null()
            || tokio::time::Instant::now() >= deadline
        {
            return Err(format!("deployment incomplete for {operation}: {status}").into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let mut commands = vec![app_commands];
    if logs {
        commands.push("logs [before]");
    }
    if metrics {
        commands.push("metrics [blob|db|entropy|jobs|timers]");
    }
    commands.push("exit");
    let hint = commands.join(", ");
    println!("Commands: {hint}");
    let command = |line: &str| -> Result<String> {
        let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if logs && verb == "logs" {
            let mut args = vec!["logs"];
            args.extend(rest.split_whitespace());
            return Ok(serde_json::to_string_pretty(&fleet_call(
                &directory, "root", &args,
            )?)?);
        }
        if metrics
            && verb == "metrics"
            && matches!(rest.trim(), "blob" | "db" | "entropy" | "jobs" | "timers")
        {
            return Ok(serde_json::to_string_pretty(&fleet_call(
                &directory,
                "root",
                &["observe-child-metrics", child_slot.as_str(), rest.trim()],
            )?)?);
        }
        if metrics && line == "metrics" {
            let mut rows = Vec::new();
            let mut after = None::<String>;
            loop {
                let mut args = vec!["observe-child-metrics", child_slot.as_str(), "endpoints"];
                args.extend(after.as_deref());
                let report = serde_json::from_value(fleet_call(&directory, "root", &args)?)?;
                let Report::Endpoints(page) = report else {
                    return Ok("Endpoint measurements unavailable".into());
                };
                rows.extend(page.measurements);
                let Some(next) = page.next else {
                    break;
                };
                if after.as_ref().is_some_and(|previous| previous >= &next) {
                    return Err("metrics cursor did not advance".into());
                }
                after = Some(next);
            }
            return Ok(serde_json::to_string_pretty(&rows)?);
        }
        if ![first, second].contains(&verb)
            || (!rest.is_empty() && !matches!(verb, "ping" | "checksum"))
        {
            return Ok(format!("Commands: {hint}"));
        }
        let mut args = vec![verb];
        if !rest.is_empty() {
            args.push(rest.trim_start());
        }
        Ok(serde_json::to_string_pretty(&fleet_call(
            &directory, name, &args,
        )?)?)
    };
    run_prompt(name, seconds, async |line: &str| command(line)).await
}

async fn run_prompt(
    name: &str,
    seconds: u64,
    mut command: impl AsyncFnMut(&str) -> Result<String>,
) -> Result<()> {
    run_live_prompt(name, seconds, None, async |line: Option<&str>| {
        command(line.expect("ordinary prompt has no timer")).await
    })
    .await
}

async fn run_live_prompt(
    name: &str,
    seconds: u64,
    cadence: Option<Duration>,
    command: impl AsyncFnMut(Option<&str>) -> Result<String>,
) -> Result<()> {
    tokio::select! {
        input = prompt(name, cadence, command) => input?,
        signal = tokio::signal::ctrl_c() => { signal?; }
        () = tokio::time::sleep(Duration::from_secs(seconds)) => { println!("Session lifetime reached."); }
    }
    Ok(())
}

async fn prompt(
    name: &str,
    cadence: Option<Duration>,
    mut command: impl AsyncFnMut(Option<&str>) -> Result<String>,
) -> Result<()> {
    let mut timer = cadence.map(tokio::time::interval);
    if let Some(timer) = &mut timer {
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    }
    let mut pulse = async || {
        if let Some(timer) = &mut timer {
            timer.tick().await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let mut terminal = terminal
        .then(|| Readline::new(format!("{name}> ")))
        .transpose()?;
    let (send, mut receive) = tokio::sync::mpsc::channel(1);
    if terminal.is_none() {
        // A blocked pipe reader must not hold Tokio shutdown open on expiry.
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines() {
                if send.blocking_send(line).is_err() {
                    break;
                }
            }
        });
    }
    loop {
        let next_line = async {
            if let Some((editor, _)) = &mut terminal {
                return Ok(match editor.readline().await? {
                    ReadlineEvent::Line(line) => Some(line),
                    _ => None,
                });
            }
            Ok::<_, Box<dyn std::error::Error>>(receive.recv().await.transpose()?)
        };
        let line = tokio::select! {
            biased;
            line = next_line => {
                let Some(line) = line? else { break };
                match line.trim() {
                    "" => continue,
                    "exit" => break,
                    _ => {}
                }
                if let Some((editor,_)) = &mut terminal { editor.add_history_entry(line.clone()); }
                Some(line)
            }
            () = pulse() => None,
        };
        let reply = command(line.as_deref().map(str::trim))
            .await
            .unwrap_or_else(|e| e.to_string());
        if !reply.is_empty() {
            if let Some((_, output)) = &mut terminal {
                writeln!(output, "{reply}")?;
            } else {
                println!("{reply}");
            }
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("qualification workspace layout")?
        .to_path_buf();
    let prepared = PreparedServer::check(&canic_qualification::server_directory())
        .map_err(|error| format!("run make qualification-setup first: {error}"))?;
    let mut server = Server::start(
        prepared,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(args.seconds + 240),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )?;
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await?;
    let outcome = tokio::time::timeout(
        Duration::from_secs(args.seconds + 180),
        session(&mut owner, &repo, args.seconds, args.app),
    )
    .await;
    let deletion = owner.close(Duration::from_secs(10)).await;
    let stopped = server.stop()?;
    if deletion != Deletion::Attempted
        || stopped.abandoned != 0
        || !stopped.process.cleanup.is_empty()
        || !matches!(
            stopped.process.failure,
            Some(ic_host_process::Failure::Cancelled)
        )
    {
        return Err(format!("simulator cleanup incomplete: {deletion:?}; {stopped:?}").into());
    }
    outcome??;
    println!("Simulator stopped; instance deletion attempted.");
    Ok(())
}
