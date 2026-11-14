use super::*;

async fn json(args: &[&str]) -> serde_json::Value {
    let cli =
        Cli::try_parse_from(["canic", "--json"].into_iter().chain(args.iter().copied())).unwrap();
    let Output::Json(value) = run(&cli).await.unwrap() else {
        panic!("expected JSON");
    };
    value
}

#[tokio::test(flavor = "current_thread")]
async fn init_named_plan_and_inspection_share_config_without_network_effects() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "fleet",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let config = root.join("canic.json");
    let config = config.to_str().unwrap();
    let init = [
        "app",
        "init",
        "--config",
        config,
        "hello",
        "--network",
        "ic",
        "--fleet",
        "demo",
    ];
    let result = json(&init).await;
    assert_eq!(result["deployed"], false);
    assert_eq!(
        result["configuration"]["apps"]["hello"]["fleets"]["demo"]["canisters"],
        serde_json::json!({})
    );
    let apps = json(&["app", "list", "--config", config]).await;
    assert_eq!(
        apps["apps"],
        serde_json::json!([{"app":"hello", "fleets":1}])
    );
    assert!(!root.join(".canic").exists());
    let without_network = Cli::try_parse_from([
        "canic", "app", "init", "--config", config, "hello", "--fleet", "demo",
    ])
    .err()
    .unwrap();
    assert!(without_network.to_string().contains("network"));
    assert!(
        Cli::try_parse_from(
            ["canic"]
                .into_iter()
                .chain(init)
                .chain(["--canister", "backend"])
        )
        .is_err()
    );
    let again = Cli::try_parse_from(["canic"].into_iter().chain(init)).unwrap();
    assert!(run(&again).await.is_err());
    let empty = json(&["fleet", "show", "--config", config]).await;
    assert_eq!(empty["configuration"]["fleet"], "demo");
    assert_eq!(empty["canisters"], serde_json::json!([]));
    assert!(!root.join(".canic").exists());
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let args = [
        "fleet",
        "plan",
        "--config",
        config,
        "--app",
        "hello",
        "--canister",
        "backend",
        "--wasm",
        wasm.to_str().unwrap(),
        "--controller",
        "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe",
    ];
    let unconfigured = Cli::try_parse_from(["canic"].into_iter().chain(args)).unwrap();
    assert!(run(&unconfigured).await.is_err());
    assert!(!root.join(".canic").exists());
    let view = Cli::try_parse_from([
        "canic", "fleet", "show", "--config", config, "--color", "never",
    ])
    .unwrap();
    let Output::Text(text) = run(&view).await.unwrap() else {
        panic!("expected text");
    };
    assert!(text.contains("No canisters declared yet"));
    let mut declared = canic_host::load_config(std::path::Path::new(config)).unwrap();
    declared
        .apps
        .get_mut("hello")
        .unwrap()
        .fleets
        .get_mut("demo")
        .unwrap()
        .canisters
        .insert("backend".into(), canic_host::CanisterConfig::default());
    std::fs::write(config, serde_json::to_vec_pretty(&declared).unwrap()).unwrap();
    let first = json(&args).await;
    let repeated = json(&args).await;
    assert_eq!(first, repeated);
    let id = first["operation"].as_str().unwrap();
    assert!(
        root.join(".canic/apps/hello/fleets/demo")
            .join(id)
            .join("plan.json")
            .exists()
    );
    let status = json(&["fleet", "status", "--config", config, id]).await;
    assert_eq!(status["plan"]["deployment"]["app"], "hello");
    assert_eq!(status["effects"]["create"], "not_submitted");
    let cli = Cli::try_parse_from([
        "canic", "fleet", "show", "--config", config, "--color", "never",
    ])
    .unwrap();
    let Output::Text(text) = run(&cli).await.unwrap() else {
        panic!("expected text");
    };
    assert!(text.contains("Fleet: demo"));
    assert!(text.contains("App: hello"));
    assert!(text.find("App: hello").unwrap() < text.find("Fleet: demo").unwrap());
    assert!(text.contains("backend: Awaiting deployment"));
    assert!(text.contains("canic fleet status --app hello --fleet demo"));
    assert!(text.contains("Configured network: IC mainnet"));
    let report = root.join("fleet.html");
    json(&[
        "fleet",
        "show",
        "--config",
        config,
        "--html",
        report.to_str().unwrap(),
    ])
    .await;
    let html = std::fs::read_to_string(report).unwrap();
    assert!(html.contains("Fleet: demo"));
    assert!(html.contains("App: hello"));
    assert!(html.find("App: hello").unwrap() < html.find("Fleet: demo").unwrap());
    assert!(html.contains("Awaiting deployment"));
    // An adopted journal may live outside the new-fleet default. Status must use
    // that original directory, without copying or changing its approved plan.
    std::fs::rename(
        root.join(".canic/apps/hello/fleets/demo"),
        root.join("original-journal"),
    )
    .unwrap();
    declared
        .apps
        .get_mut("hello")
        .unwrap()
        .fleets
        .get_mut("demo")
        .unwrap()
        .journal = Some("original-journal".into());
    std::fs::write(config, serde_json::to_vec_pretty(&declared).unwrap()).unwrap();
    let adopted_status = json(&["fleet", "status", "--config", config, id]).await;
    assert_eq!(adopted_status, status);
    assert!(!root.join(".canic/apps/hello/fleets/demo").exists());
    let fleets = &mut declared.apps.get_mut("hello").unwrap().fleets;
    fleets.insert("staging".into(), fleets["demo"].clone());
    std::fs::write(config, serde_json::to_vec_pretty(&declared).unwrap()).unwrap();
    let ambiguous = Cli::try_parse_from(["canic", "fleet", "show", "--config", config]).unwrap();
    let listed = json(&["fleet", "list", "--config", config]).await;
    assert_eq!(listed["fleets"].as_array().unwrap().len(), 2);
    let filtered = json(&["fleet", "list", "--config", config, "--app", "hello"]).await;
    assert_eq!(filtered, listed);
    assert!(
        run(&ambiguous)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("select --fleet")
    );
    let other = json(&[
        "fleet", "show", "--config", config, "--app", "hello", "--fleet", "staging",
    ])
    .await;
    assert_eq!(other["configuration"]["app"], "hello");
    assert_eq!(other["configuration"]["fleet"], "staging");
    assert_eq!(other["operations"], serde_json::json!([]));
    let wrong = Cli::try_parse_from([
        "canic", "fleet", "status", "--config", config, "--fleet", "staging", id,
    ])
    .unwrap();
    assert!(
        run(&wrong)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("does not belong")
    );
    let before = std::fs::read_dir(root.join("original-journal"))
        .unwrap()
        .count();
    let previous = "00".repeat(32);
    let outside = Cli::try_parse_from([
        "canic",
        "fleet",
        "plan",
        "--config",
        config,
        "--app",
        "hello",
        "--fleet",
        "demo",
        "--wasm",
        wasm.to_str().unwrap(),
        "--controller",
        "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe",
        "--target",
        "aaaaa-aa",
        "--previous-module",
        &previous,
    ])
    .unwrap();
    assert!(
        run(&outside)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("upgrade target does not belong")
    );
    assert_eq!(
        std::fs::read_dir(root.join("original-journal"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn help_includes_version_and_sorted_commands_at_every_level() {
    fn check(command: &clap::Command, path: &[String]) {
        for flag in ["--help", "-h"] {
            let mut args = path.to_vec();
            args.push(flag.into());
            let help = help_command().try_get_matches_from(args).unwrap_err();
            assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
            let text = help.to_string();
            assert!(text.starts_with(concat!("Canic ", env!("CARGO_PKG_VERSION"))));
            if let Some(commands) = text.split("Commands:\n").nth(1) {
                let names: Vec<_> = commands
                    .split("\n\n")
                    .next()
                    .unwrap()
                    .lines()
                    .map(|line| line.split_whitespace().next().unwrap())
                    .collect();
                assert!(names.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
            }
        }
        for child in command.get_subcommands() {
            let mut next = path.to_vec();
            next.push(child.get_name().into());
            check(child, &next);
        }
    }
    check(&help_command(), &["canic".into()]);
    let help = help_command()
        .try_get_matches_from(["canic", "help", "app", "init"])
        .unwrap_err();
    assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
}

#[test]
fn commands_reject_context_they_do_not_use() {
    for args in [
        vec!["app", "list", "--fleet", "mainnet"],
        vec![
            "app",
            "init",
            "demo",
            "--fleet",
            "mainnet",
            "--network",
            "ic",
            "--state-dir",
            "journal",
        ],
        vec!["artifact", "inspect", "app.wasm", "--app", "demo"],
        vec!["artifact", "build"],
        vec![
            "artifact",
            "build",
            "--package",
            "demo",
            "--fleet",
            "mainnet",
        ],
        vec!["fleet", "view", "report.html", "--app", "demo"],
        vec!["fleet", "view", "report.html", "--config", "canic.json"],
        vec!["fleet", "list", "--fleet", "mainnet"],
        vec!["fleet", "list", "--state-dir", "journal"],
        vec!["fleet", "list", "--watch"],
        vec!["fleet", "list", "--open"],
        vec!["fleet", "show", "--port", "8788"],
        vec!["wallet", "2vxsx-fae", "--fleet", "mainnet"],
        vec![
            "fleet",
            "plan",
            "--wasm",
            "app.wasm",
            "--controller",
            "2vxsx-fae",
            "--canister",
            "backend",
            "--target",
            "aaaaa-aa",
            "--previous-module",
            "00",
        ],
        vec![
            "fleet",
            "plan",
            "--wasm",
            "app.wasm",
            "--controller",
            "2vxsx-fae",
            "--target",
            "aaaaa-aa",
        ],
    ] {
        assert!(
            help_command()
                .try_get_matches_from(std::iter::once("canic").chain(args.iter().copied()))
                .is_err(),
            "{args:?}"
        );
    }
}
