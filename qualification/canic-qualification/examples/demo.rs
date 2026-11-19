//! Disposable actor rehearsal. Testkit owns the server; normal Canic commands own deployments.
use clap::Parser;
use ic_host_process::{Input, Ownership, Policy, Stderr, run};
use ic_testkit::{
    pocket_ic::PocketIcBuilder,
    server::{Deletion, Limits, PreparedServer, Server},
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(about = "Disposable managed counter fleet; synthetic cycles, no mainnet effects")]
struct Args {
    /// Session lifetime, after setup. Each invocation creates a fresh journal directory.
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..=3600))]
    seconds: u64,
}

fn cli(directory: &Path, args: &[&str]) -> Result<()> {
    let mut command = Command::new("canic");
    command.current_dir(directory).args(args).arg("--json");
    let mut output = Vec::new();
    let report = run(
        &mut command,
        Policy {
            ownership: Ownership::Group,
            input: Input::Null,
            stdout_bytes: 65_536,
            stderr: Stderr::Capture(65_536),
            elapsed: Duration::from_secs(30),
        },
        &AtomicBool::new(false),
        |_, bytes| {
            output.extend_from_slice(bytes);
            Ok(())
        },
    );
    if report.failure.is_some()
        || !report.cleanup.is_empty()
        || !report.status.is_some_and(|status| status.success())
    {
        return Err(format!(
            "Canic setup refused: {report:?}; output: {:?}",
            String::from_utf8_lossy(&output)
        )
        .into());
    }
    Ok(())
}

fn shell(path: &Path) -> Result<String> {
    let text = path.to_str().ok_or("demo paths require UTF-8")?;
    if text.contains(['\r', '\n']) {
        return Err("demo paths cannot contain newlines".into());
    }
    Ok(format!("'{}'", text.replace('\'', "'\"'\"'")))
}

async fn session(
    owner: &mut ic_testkit::server::Instance,
    repo: &Path,
    seconds: u64,
) -> Result<()> {
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
    let example = repo.join("apps/examples/counter");
    let mut config = canic_host::load_config(&example.join("canic.toml"))?;
    let app = config
        .apps
        .get_mut("ex-counter")
        .ok_or("missing counter example")?;
    app.fleets.clear();
    for canister in app.canisters.values_mut() {
        let recipe = canister
            .build
            .as_mut()
            .ok_or("counter component needs a build recipe")?;
        recipe.manifest = example.join(&recipe.manifest).canonicalize()?;
    }
    canic_host::init_config(&directory.join("canic.toml"), &config)?;
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
        "Local counter fleet session: {url}\nRoot, separate Wasm Store and counter; synthetic cycles only."
    );
    println!(
        "In another terminal:\n\n  cd {}\n  canic app build\n  canic fleet plan --identity anonymous\n  canic fleet apply OPERATION --identity anonymous\n  canic fleet call --canister ex-counter --identity anonymous read\n  canic fleet call --canister ex-counter --identity anonymous increment\n  canic fleet show\n",
        shell(&directory)?
    );
    println!(
        "Edit {} and rebuild with canic app build.",
        shell(&repo.join("apps/examples/counter/src/actor.rs"))?
    );
    println!(
        "After a rebuild, fleet plan selects publication, then upgrade. Apply each returned operation. Repeat apply on the same operation while Root is pending; wait for completion before calling the app."
    );
    println!(
        "Copy each OPERATION from its plan. Failed/uncertain attempts must be resumed, not replaced."
    );
    println!(
        "Press Enter here to stop (or Ctrl-C). Session expires after {seconds}s.\nLocal files are retained; new sessions create fresh instances and journal directories."
    );
    std::io::stdout().flush()?;
    // An owned OS thread may block on terminal input. Unlike spawn_blocking,
    // it does not hold Tokio shutdown open after the deadline or Ctrl-C.
    let (send, receive) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = std::io::stdin().read(&mut [0]);
        let _ = send.send(result);
    });
    tokio::select! {
        input = receive => { input??; }
        signal = tokio::signal::ctrl_c() => { signal?; }
        () = tokio::time::sleep(Duration::from_secs(seconds)) => { println!("Session lifetime reached."); }
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
        session(&mut owner, &repo, args.seconds),
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
    println!("Simulator stopped; instance deletion attempted. Session files retained.");
    Ok(())
}
