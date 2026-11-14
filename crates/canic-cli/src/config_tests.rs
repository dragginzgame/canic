use super::*;
use clap::Parser as _;

async fn json(args: &[&str]) -> serde_json::Value {
    let cli =
        Cli::try_parse_from(["canic", "--json"].into_iter().chain(args.iter().copied())).unwrap();
    let Output::Json(value) = run(&cli).await.unwrap() else {
        panic!("expected JSON");
    };
    value
}

#[tokio::test(flavor = "current_thread")]
async fn selectors_infer_only_unique_apps_and_fleets() {
    async fn refuses(path: &std::path::Path, args: &[&str], message: &str) {
        let before = std::fs::read(path).unwrap();
        let parent = path.parent().unwrap();
        let entries = || {
            std::fs::read_dir(parent)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<std::collections::BTreeSet<_>>()
        };
        let files = entries();
        let cli = Cli::try_parse_from(
            ["canic"]
                .into_iter()
                .chain(args.iter().copied())
                .chain(["--config", path.to_str().unwrap()]),
        )
        .unwrap();
        let Err(error) = run(&cli).await else {
            panic!("expected selection refusal");
        };
        assert!(error.to_string().contains(message), "{error}");
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert_eq!(entries(), files);
    }

    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "selectors",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("custom.json");
    let config = path.to_str().unwrap();
    let report = root.join("report.html");
    let journal = root.join("journal");
    let deployment_commands: [&[&str]; 5] = [
        &["fleet", "show", "--html", report.to_str().unwrap()],
        &[
            "fleet",
            "plan",
            "--canister",
            "backend",
            "--controller",
            "aaaaa-aa",
        ],
        &["fleet", "apply", "operation", "--identity", "unused"],
        &["fleet", "status", "operation"],
        &["fleet", "call", "operation"],
    ];
    json(&[
        "app",
        "init",
        "hello",
        "--fleet",
        "mainnet",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    json(&["app", "canister", "add", "backend", "--config", config]).await;
    let app = json(&["app", "canister", "list", "--config", config]).await;
    assert_eq!(app["app"], "hello");
    assert_eq!(
        app,
        json(&[
            "app", "canister", "list", "--app", "hello", "--config", config
        ])
        .await
    );
    let bound = json(&[
        "fleet", "canister", "bind", "backend", "--id", "aaaaa-aa", "--config", config,
    ])
    .await;
    assert_eq!(bound["configuration"]["app"], "hello");
    assert_eq!(bound["configuration"]["fleet"], "mainnet");
    assert_eq!(
        json(&["fleet", "canister", "list", "--config", config]).await,
        json(&[
            "fleet", "canister", "list", "--app", "hello", "--fleet", "mainnet", "--config",
            config,
        ])
        .await
    );
    for args in [
        vec!["app", "canister", "add", "extra", "--app", "typo"],
        vec!["fleet", "init", "extra", "--network", "ic", "--app", "typo"],
        vec!["fleet", "canister", "list", "--app", "typo"],
        vec!["fleet", "list", "--app", "typo"],
    ] {
        refuses(&path, &args, "select --app from: hello").await;
    }
    refuses(
        &path,
        &[
            "fleet", "canister", "bind", "backend", "--id", "aaaaa-aa", "--fleet", "typo",
        ],
        "select --fleet from: mainnet",
    )
    .await;
    // Scope refusal precedes plan/evidence access, signing and report output.
    for command in deployment_commands {
        for (selector, message) in [
            ("--app", "select --app from: hello"),
            ("--fleet", "select --fleet from: mainnet"),
        ] {
            let mut args = command.to_vec();
            args.extend(["--state-dir", journal.to_str().unwrap(), selector, "typo"]);
            refuses(&path, &args, message).await;
            args.push("--no-config");
            assert!(Cli::try_parse_from(std::iter::once("canic").chain(args)).is_err());
        }
    }

    let staging = json(&[
        "fleet",
        "init",
        "staging",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    assert_eq!(staging["configuration"]["app"], "hello");
    assert_eq!(staging["configuration"]["fleet"], "staging");
    json(&["app", "canister", "add", "worker", "--config", config]).await;
    let app = json(&["app", "canister", "list", "--config", config]).await;
    assert!(app["canisters"].get("worker").is_some());
    for args in [
        vec!["fleet", "canister", "list"],
        vec!["fleet", "canister", "bind", "worker", "--id", "aaaaa-aa"],
    ] {
        refuses(&path, &args, "select --fleet from: mainnet, staging").await;
    }
    for command in deployment_commands {
        let mut args = command.to_vec();
        args.extend(["--state-dir", journal.to_str().unwrap()]);
        refuses(&path, &args, "select --fleet from: mainnet, staging").await;
    }
    let staging = json(&[
        "fleet", "canister", "list", "--fleet", "staging", "--config", config,
    ])
    .await;
    assert_eq!(staging["configuration"]["fleet"], "staging");

    json(&[
        "app",
        "init",
        "other",
        "--fleet",
        "othernet",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    for args in [
        vec!["app", "canister", "list"],
        vec!["app", "canister", "add", "extra"],
        vec!["fleet", "init", "extra", "--network", "ic"],
        vec!["fleet", "canister", "list", "--fleet", "staging"],
        vec![
            "fleet", "canister", "bind", "worker", "--id", "aaaaa-aa", "--fleet", "staging",
        ],
    ] {
        refuses(&path, &args, "select --app from: hello, other").await;
    }
    for command in deployment_commands {
        let mut args = command.to_vec();
        args.extend([
            "--state-dir",
            journal.to_str().unwrap(),
            "--fleet",
            "staging",
        ]);
        refuses(&path, &args, "select --app from: hello, other").await;
    }
    let listed = json(&[
        "fleet", "canister", "list", "--app", "other", "--config", config,
    ])
    .await;
    assert_eq!(listed["configuration"]["fleet"], "othernet");
    assert_eq!(
        json(&["app", "list", "--config", config]).await["apps"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        json(&["fleet", "list", "--config", config]).await["fleets"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(!root.join(".canic").exists());
    assert!(!journal.exists());
    assert!(!report.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn app_artifact_and_topology_are_shared_but_fleet_plans_remain_independent() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "shared-app",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let config = path.to_str().unwrap();
    std::fs::write(root.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();
    json(&[
        "app",
        "init",
        "hello",
        "--fleet",
        "first",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    json(&[
        "app", "canister", "add", "backend", "--app", "hello", "--role", "worker", "--wasm",
        "app.wasm", "--config", config,
    ])
    .await;
    json(&[
        "fleet",
        "init",
        "second",
        "--app",
        "hello",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    let listed = json(&[
        "app", "canister", "list", "--app", "hello", "--config", config,
    ])
    .await;
    assert_eq!(listed["canisters"]["backend"]["wasm"], "app.wasm");
    let fleets = json(&["fleet", "list", "--app", "hello", "--config", config]).await;
    assert!(
        fleets["fleets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|fleet| fleet["declared_canisters"] == 1)
    );
    let owner = "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe";
    let mut operations = Vec::new();
    for fleet in ["first", "second"] {
        let args = [
            "fleet",
            "plan",
            "--app",
            "hello",
            "--fleet",
            fleet,
            "--canister",
            "backend",
            "--controller",
            owner,
            "--config",
            config,
        ];
        let plan = json(&args).await;
        assert_eq!(json(&args).await, plan);
        let id = plan["operation"].as_str().unwrap();
        assert_eq!(
            std::fs::read(root.join(format!(".canic/apps/hello/fleets/{fleet}/{id}/module.wasm")))
                .unwrap(),
            b"\0asm\x01\0\0\0"
        );
        operations.push(id.to_string());
        let conflicting = Cli::try_parse_from(
            ["canic"]
                .into_iter()
                .chain(args)
                .chain(["--wasm", "override.wasm"]),
        )
        .unwrap();
        assert!(run(&conflicting).await.is_err());
    }
    assert_ne!(operations[0], operations[1]);
    std::fs::write(root.join("app.wasm"), b"\0asm\x01\0\0\0\0\x01\0").unwrap();
    let changed = Cli::try_parse_from([
        "canic",
        "fleet",
        "plan",
        "--app",
        "hello",
        "--fleet",
        "first",
        "--canister",
        "backend",
        "--controller",
        owner,
        "--config",
        config,
    ])
    .unwrap();
    assert!(matches!(run(&changed).await, Err(Error::Blocked(_))));
    let status = json(&[
        "fleet",
        "status",
        &operations[0],
        "--app",
        "hello",
        "--fleet",
        "first",
        "--config",
        config,
    ])
    .await;
    assert_eq!(status["effects"]["install"], "not_submitted");
}

#[tokio::test(flavor = "current_thread")]
async fn canister_declarations_retain_roles_and_parents_without_effect_authority() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "topology",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let config = path.to_str().unwrap();
    json(&[
        "app",
        "init",
        "profiles",
        "--fleet",
        "demo",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    json(&[
        "app", "canister", "add", "owner", "--role", "manager", "--app", "profiles", "--config",
        config,
    ])
    .await;
    let args = [
        "app", "canister", "add", "child", "--role", "worker", "--parent", "owner", "--app",
        "profiles", "--config", config,
    ];
    let added = json(&args).await;
    assert_eq!(
        added["configuration"]["apps"]["profiles"]["canisters"]["child"],
        serde_json::json!({"role":"worker", "parent":"owner"})
    );
    assert_eq!(added["deployed"], false);
    assert!(!root.join(".canic").exists());
    let args = [
        "fleet", "canister", "list", "--app", "profiles", "--fleet", "demo", "--config", config,
    ];
    let cli = Cli::try_parse_from(std::iter::once("canic").chain(args)).unwrap();
    let Output::Text(text) = run(&cli).await.unwrap() else {
        panic!("expected declaration tree");
    };
    assert!(text.contains("└─ child"));
    assert!(text.contains("manager"));
    let listed = json(&args).await;
    assert_eq!(
        listed["configuration"]["config"]["canisters"],
        added["configuration"]["apps"]["profiles"]["canisters"]
    );
    let app_list = Cli::try_parse_from([
        "canic", "app", "canister", "list", "--app", "profiles", "--config", config,
    ])
    .unwrap();
    let Output::Text(before) = run(&app_list).await.unwrap() else {
        panic!("expected app tree");
    };
    assert!(before.contains("└─ child"));
    json(&[
        "fleet",
        "init",
        "another",
        "--app",
        "profiles",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    let Output::Text(after) = run(&app_list).await.unwrap() else {
        panic!("expected app tree with multiple fleets");
    };
    assert_eq!(after, before);
    let bytes = std::fs::read(&path).unwrap();
    let cli = Cli::try_parse_from([
        "canic", "app", "canister", "add", "other", "--parent", "missing", "--app", "profiles",
        "--config", config,
    ])
    .unwrap();
    assert!(run(&cli).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[tokio::test(flavor = "current_thread")]
async fn app_init_adds_a_second_app_and_requires_explicit_selection() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "second-app",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let config = path.to_str().unwrap();
    let first = json(&[
        "app",
        "init",
        "first",
        "--fleet",
        "mainnet",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    let args = [
        "app",
        "init",
        "second",
        "--fleet",
        "mainnet",
        "--network",
        "ic",
        "--config",
        config,
    ];
    let added = json(&args).await;
    assert_eq!(added["deployed"], false);
    assert_eq!(
        added["configuration"]["apps"]["first"],
        first["configuration"]["apps"]["first"]
    );
    let listed = json(&["app", "list", "--config", config]).await;
    assert_eq!(
        listed["apps"],
        serde_json::json!([{"app":"first", "fleets":1}, {"app":"second", "fleets":1}])
    );
    let ambiguous = Cli::try_parse_from([
        "canic", "fleet", "show", "--fleet", "mainnet", "--config", config,
    ])
    .unwrap();
    assert!(matches!(run(&ambiguous).await, Err(Error::Invalid(_))));
    let selected = json(&[
        "fleet", "show", "--app", "second", "--fleet", "mainnet", "--config", config,
    ])
    .await;
    assert_eq!(selected["configuration"]["app"], "second");
    assert_eq!(selected["canisters"], serde_json::json!([]));
    let before = std::fs::read(&path).unwrap();
    let repeated = Cli::try_parse_from(["canic"].into_iter().chain(args)).unwrap();
    assert!(matches!(run(&repeated).await, Err(Error::Invalid(_))));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!root.join(".canic").exists());
}

#[tokio::test(flavor = "current_thread")]
async fn existing_canister_declaration_lists_exact_id_without_deployment_authority() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "existing-id",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let config = path.to_str().unwrap();
    json(&[
        "app",
        "init",
        "hello",
        "--fleet",
        "demo",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    for slot in ["existing", "another"] {
        json(&[
            "app", "canister", "add", slot, "--app", "hello", "--config", config,
        ])
        .await;
    }
    let id = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let args = [
        "fleet", "canister", "bind", "existing", "--id", id, "--app", "hello", "--fleet", "demo",
        "--config", config,
    ];
    for forbidden in ["--identity", "--network", "--state-dir", "--apply"] {
        assert!(
            Cli::try_parse_from(
                ["canic"]
                    .into_iter()
                    .chain(args)
                    .chain([forbidden, "unused"])
            )
            .is_err()
        );
    }
    let cli = Cli::try_parse_from(["canic"].into_iter().chain(args)).unwrap();
    let Output::Text(output) = run(&cli).await.unwrap() else {
        panic!("expected text")
    };
    assert!(output.contains(&format!("Configured ID: {id}")));
    assert!(output.contains("Existence and control are not verified"));
    let listed = json(&[
        "fleet", "canister", "list", "--app", "hello", "--fleet", "demo", "--config", config,
    ])
    .await;
    assert_eq!(
        listed["configuration"]["config"]["canisters"]["existing"]["id"],
        id
    );
    assert_eq!(listed["scope"], "local configuration only");
    let before = std::fs::read(&path).unwrap();
    assert!(run(&cli).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let added = json(&[
        "fleet",
        "canister",
        "bind",
        "another",
        "--id",
        "ryjl3-tyaaa-aaaaa-aaaba-cai",
        "--app",
        "hello",
        "--fleet",
        "demo",
        "--config",
        config,
    ])
    .await;
    assert_eq!(added["deployed"], false);
    assert_eq!(
        added["configuration"]["config"]["canisters"]["another"]["id"],
        "ryjl3-tyaaa-aaaaa-aaaba-cai"
    );
    assert!(!root.join(".canic").exists());
}

#[tokio::test(flavor = "current_thread")]
async fn declaration_messages_escape_paths_while_json_retains_original_paths() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "escaped-path",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    for structured in [false, true] {
        let path = root.join(format!("config-{structured}-café\n\u{1b}[2J\u{202e}.json"));
        let config = path.to_str().unwrap();
        for args in [
            vec!["app", "init", "hello", "--fleet", "demo", "--network", "ic"],
            vec![
                "fleet",
                "init",
                "staging",
                "--app",
                "hello",
                "--network",
                "ic",
            ],
            vec!["app", "canister", "add", "worker", "--app", "hello"],
        ] {
            let mut cli = Cli::try_parse_from(
                ["canic"]
                    .into_iter()
                    .chain(args)
                    .chain(["--config", config]),
            )
            .unwrap();
            cli.json = structured;
            match run(&cli).await.unwrap() {
                Output::Json(value) => {
                    assert!(structured);
                    assert_eq!(value["config_path"], config);
                    assert_eq!(value["deployed"], false);
                }
                Output::Text(value) => {
                    assert!(!structured);
                    assert!(value.contains(&format!(
                        "config-{structured}-café\\n\\u{{1b}}[2J\\u{{202e}}.json"
                    )));
                    assert!(!value.contains('\u{1b}'));
                    assert!(!value.contains('\u{202e}'));
                }
            }
        }
        let selected = canic_host::load_config(&path)
            .unwrap()
            .select(Some("hello"), Some("staging"))
            .unwrap();
        assert!(selected.config.canisters.contains_key("worker"));
    }
    assert!(!root.join(".canic").exists());
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_init_adds_a_second_fleet_without_journals_or_network_calls() {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/init-cli-tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = ic_host_fs::directory::create_unique_directory(
        &parent,
        "second-fleet",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .unwrap();
    let path = root.join("canic.json");
    let config = path.to_str().unwrap();
    json(&[
        "app",
        "init",
        "hello",
        "--fleet",
        "mainnet",
        "--network",
        "ic",
        "--config",
        config,
    ])
    .await;
    let args = [
        "fleet",
        "init",
        "staging",
        "--app",
        "hello",
        "--network",
        "ic",
        "--config",
        config,
    ];
    let result = json(&args).await;
    assert_eq!(result["deployed"], false);
    assert_eq!(result["configuration"]["app"], "hello");
    assert_eq!(result["configuration"]["fleet"], "staging");
    assert!(!root.join(".canic").exists());
    let listed = json(&["fleet", "list", "--config", config, "--app", "hello"]).await;
    assert_eq!(listed["fleets"].as_array().unwrap().len(), 2);
    let before = std::fs::read(&path).unwrap();
    let repeat = Cli::try_parse_from(std::iter::once("canic").chain(args)).unwrap();
    assert!(run(&repeat).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    for invalid in [
        vec!["fleet", "init", "--network", "ic"],
        vec!["fleet", "init", "missing-network", "--app", "hello"],
        vec![
            "fleet",
            "init",
            "local",
            "--app",
            "hello",
            "--network",
            "local",
        ],
        vec![
            "fleet",
            "init",
            "extra",
            "--app",
            "hello",
            "--network",
            "ic",
            "--state-dir",
            "journal",
        ],
    ] {
        assert!(Cli::try_parse_from(std::iter::once("canic").chain(invalid)).is_err());
    }
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
    let slot_args = [
        "app", "canister", "add", "backend", "--app", "hello", "--config", config,
    ];
    let added = json(&slot_args).await;
    assert_eq!(added["deployed"], false);
    assert!(!root.join(".canic").exists());
    let before = std::fs::read(config).unwrap();
    let duplicate = Cli::try_parse_from(std::iter::once("canic").chain(slot_args)).unwrap();
    assert!(run(&duplicate).await.is_err());
    assert_eq!(std::fs::read(config).unwrap(), before);
    let list_args = [
        "fleet", "canister", "list", "--app", "hello", "--fleet", "demo", "--config", config,
    ];
    let listed = json(&list_args).await;
    assert_eq!(
        listed["configuration"]["config"]["canisters"],
        serde_json::json!({"backend":{}})
    );
    let list = Cli::try_parse_from(std::iter::once("canic").chain(list_args)).unwrap();
    let Output::Text(text) = run(&list).await.unwrap() else {
        panic!("expected text")
    };
    assert!(text.contains("backend"));
    assert!(text.starts_with("App: hello · Fleet: demo\n\nCANISTER"));
    let mut declared = canic_host::load_config(std::path::Path::new(config)).unwrap();
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
    let status_cli =
        Cli::try_parse_from(["canic", "fleet", "status", "--config", config, id]).unwrap();
    let Output::Text(text) = run(&status_cli).await.unwrap() else {
        panic!("expected readable operation status");
    };
    assert!(text.contains(id));
    assert!(text.contains("App: hello\nFleet: demo\nCanister slot: backend"));
    assert!(text.contains("Not submitted"));
    assert!(text.contains("Receipt: none retained"));
    assert_eq!(
        json(&["fleet", "status", "--config", config, id]).await,
        status
    );
    let cli = Cli::try_parse_from([
        "canic", "fleet", "show", "--config", config, "--color", "never",
    ])
    .unwrap();
    let Output::Text(text) = run(&cli).await.unwrap() else {
        panic!("expected text");
    };
    assert!(text.contains("hello / demo · IC mainnet"));
    assert!(text.contains("backend: Awaiting deployment"));
    assert!(!text.contains("canic fleet status"));
    assert!(text.contains("IC mainnet"));
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
fn source_generation_requires_its_checkout_and_owns_its_new_config() {
    let base = [
        "canic",
        "app",
        "init",
        "hello",
        "--fleet",
        "mainnet",
        "--network",
        "ic",
    ];
    assert!(
        Cli::try_parse_from(
            base.into_iter()
                .chain(["--source", "new", "--canic-source", "."])
        )
        .is_ok()
    );
    for extra in [
        vec!["--source", "new"],
        vec!["--canic-source", "."],
        vec![
            "--source",
            "new",
            "--canic-source",
            ".",
            "--config",
            "existing.json",
        ],
        vec![
            "--source",
            "new",
            "--canic-source",
            ".",
            "--app",
            "existing",
        ],
    ] {
        assert!(Cli::try_parse_from(base.into_iter().chain(extra)).is_err());
    }
}

#[test]
fn commands_reject_context_they_do_not_use() {
    for args in [
        vec![
            "app", "canister", "add", "backend", "--app", "hello", "--fleet", "mainnet",
        ],
        vec![
            "app", "canister", "add", "backend", "--app", "hello", "--id", "aaaaa-aa",
        ],
        vec![
            "fleet", "canister", "bind", "backend", "--app", "hello", "--fleet", "mainnet", "--id",
            "aaaaa-aa", "--role", "worker",
        ],
        vec!["fleet", "canister", "add", "backend", "--app", "hello"],
        vec!["fleet", "canister", "list", "--network", "ic"],
        vec![
            "fleet",
            "canister",
            "list",
            "--app",
            "hello",
            "--fleet",
            "mainnet",
            "--state-dir",
            "journal",
        ],
        vec![
            "fleet",
            "canister",
            "add",
            "backend",
            "--app",
            "hello",
            "--fleet",
            "mainnet",
            "--network",
            "ic",
        ],
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

#[test]
fn managed_planning_arguments_require_the_named_child_context() {
    let base = ["canic", "fleet", "plan", "--controller", "2vxsx-fae"];
    for args in [
        vec!["--root-module", "00"],
        vec!["--execute-at", "123"],
        vec!["--canister", "app", "--root-module", "00", "--no-config"],
    ] {
        assert!(Cli::try_parse_from(base.into_iter().chain(args)).is_err());
    }
    assert!(
        Cli::try_parse_from(base.into_iter().chain([
            "--canister",
            "app",
            "--root-module",
            "00",
            "--execute-at",
            "123"
        ]))
        .is_ok()
    );
    assert!(
        Cli::try_parse_from(base.into_iter().chain([
            "--canister",
            "app",
            "--root-module",
            "00",
            "--target",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
            "--previous-module",
            "01"
        ]))
        .is_ok()
    );
}
