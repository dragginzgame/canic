//! Qualify an installed, extracted-package CLI against one disposable consumer and PocketIC.
//!
//! The CLI owns build, generation, review and execution; the harness owns only the test network.

use super::*;
use canic_core::cdk::utils::hash::sha256_hex;
use canic_host::fleet_ensure::model::DesiredCanisterKind;
use std::{collections::BTreeSet, io::Read, process::Output};

struct Consumer {
    root: PathBuf,
    cli: PathBuf,
    target: PathBuf,
    cli_sha256: String,
}

impl Consumer {
    fn from_environment() -> Self {
        let result = Self {
            root: std::env::var_os("CANIC_PACKAGED_WORKSPACE")
                .expect("package lane consumer")
                .into(),
            cli: std::env::var_os("CANIC_PACKAGED_CLI")
                .expect("package lane installed CLI")
                .into(),
            target: std::env::var_os("CANIC_PACKAGED_TARGET")
                .expect("package lane build target")
                .into(),
            cli_sha256: std::env::var("CANIC_PACKAGED_CLI_SHA256")
                .expect("installed binary identity"),
        };
        result.verify_binary();
        result
    }

    fn verify_binary(&self) {
        assert_eq!(
            sha256_hex(&std::fs::read(&self.cli).unwrap()),
            self.cli_sha256
        );
    }

    fn run(&self, arguments: &[String]) -> Output {
        let output = Command::new(&self.cli)
            .args(arguments)
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", &self.target)
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap();
        let diagnostics = self.root.join(".canic/diagnostics/packaged-cli");
        std::fs::create_dir_all(&diagnostics).unwrap();
        std::fs::write(diagnostics.join("stdout"), &output.stdout).unwrap();
        std::fs::write(diagnostics.join("stderr"), &output.stderr).unwrap();
        output
    }

    fn json(&self, arguments: &[String]) -> serde_json::Value {
        let output = self.run(arguments);
        assert_success(&output);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "one JSON stdout result required: {error}\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }
}

#[test]
#[ignore = "explicit extracted-package deployment lane; run make test-packaged-downstream-cli"]
fn installed_package_build_deploy_recover_and_replay() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let consumer = Consumer::from_environment();
    let build_args = strings(&["build", "downstream", "--json"]);
    let build = consumer.json(&build_args);
    assert_eq!(build["event"], "build_completed");
    assert_eq!(build["reused"], false);
    let reused = consumer.json(&build_args);
    assert_eq!(reused["reused"], true);
    assert_eq!(build["release_build_id"], reused["release_build_id"]);
    assert_eq!(build["release_manifest"], reused["release_manifest"]);
    let release = build["release_build_id"].as_str().unwrap();
    let (wrapper, operator, _) = prepare_isolated_icp(&consumer.root);
    let mut pic = build_management_pic();
    let subnet = pic.topology().get_app_subnets()[0];
    let config = consumer.root.join("apps/downstream/canic.toml");
    let infrastructure = prepare_network(&pic, operator, subnet, &config);
    let key = pic.root_key().unwrap();
    let replica = LocalReplicaTarget {
        environment: "local".into(),
        root_key: hex_bytes(&key),
        url: pic.make_live(None).to_string(),
    };
    let route = completed_reset::local_icp_route(&consumer.root, &wrapper, &replica);
    prepare_generation(&consumer, &config, &key, operator, subnet, &route, release);
    consumer.verify_binary();
    let desired =
        canic_host::fleet_ensure::load_desired_fleet(&consumer.root.join("fleets/packaged.toml"))
            .unwrap()
            .desired;
    let mut args = base_args(&route);
    args.extend(strings(&["fleet", "ensure", "packaged", "--json"]));
    std::fs::write(consumer.root.join("fail-before-install"), []).unwrap();
    std::fs::write(consumer.root.join("lose-install-response"), []).unwrap();
    let replay_args = converge(&consumer, args);
    verify_installed_artifacts(&consumer, &pic, operator, &desired, &infrastructure);
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let requests: u64 = pic.query_candid(ledger, "request_count", ()).unwrap();
    let mutations = mutation_counts(&consumer.root);
    let mut offline_replay = replay_args;
    offline_replay[3] = "/missing/packaged-replay-icp".into();
    let replay = consumer.json(&offline_replay);
    assert_eq!(replay["automation"]["fleet_completed"], true);
    assert_eq!(replay["effects_applied"], 0);
    assert_eq!(mutation_counts(&consumer.root), mutations);
    assert_eq!(
        pic.query_candid::<u64, ()>(ledger, "request_count", ())
            .unwrap(),
        requests
    );
    println!(
        "packaged CLI {} built, deployed, recovered and replayed release {release}",
        consumer.cli_sha256
    );
}

fn prepare_network(
    pic: &PocketIc,
    operator: Principal,
    subnet: Principal,
    config: &Path,
) -> [(DesiredCanisterKind, Principal); 3] {
    let create = |cycles, controllers| {
        pic.create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(cycles),
                placement: Some(CreateCanisterPlacement::SubnetId(subnet)),
                settings: Some(CanisterSettings {
                    controllers: Some(controllers),
                    ..CanisterSettings::default()
                }),
            },
        )
        .unwrap()
    };
    let coordinator = create(COORDINATOR_INSTALL_CYCLES, vec![operator]);
    let root = create(ROOT_INSTALL_CYCLES, vec![operator]);
    let mut controllers = vec![root, operator];
    controllers.sort();
    let store = create(ROOT_INSTALL_CYCLES, controllers.clone());
    let config = AppConfigSnapshot::load(config).unwrap();
    let readiness = config
        .model()
        .component_specs
        .values()
        .map(|spec| spec.initial_cycles.to_u128())
        .max()
        .unwrap();
    let funding = canic_host::fleet_ensure::fresh_pool_creation_funding(readiness).unwrap();
    let pools = [
        create(funding, controllers.clone()),
        create(funding, controllers.clone()),
    ];
    let ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap();
    pic.create_canister_with_id(None, None, ledger).unwrap();
    pic.add_cycles(ledger, 2_000_000_000_000_000);
    pic.install_canister(
        ledger,
        build_journey_cycles_ledger_wasm(),
        encode_one(CyclesLedgerStubInitArgs {
            canister_ids: [coordinator, root, store]
                .into_iter()
                .chain(pools)
                .collect(),
            expected_controllers_by_index: Some(vec![
                vec![operator],
                vec![operator],
                controllers.clone(),
                controllers.clone(),
                controllers,
            ]),
            expected_root: root,
            expected_subnet: subnet,
            initial_balances: Some(vec![CyclesLedgerStubAccountBalance {
                balance: Nat::from(2_000_000_000_000_000_u128),
                owner: operator,
            }]),
            pending_first_index: None,
            withdrawal_fee: Some(Nat::from(0_u8)),
        })
        .unwrap(),
        None,
    );
    [
        (DesiredCanisterKind::Coordinator, coordinator),
        (DesiredCanisterKind::Root, root),
        (DesiredCanisterKind::Store, store),
    ]
}

fn prepare_generation(
    consumer: &Consumer,
    config: &Path,
    key: &[u8],
    operator: Principal,
    subnet: Principal,
    route: &Path,
    release: &str,
) {
    let trust = consumer.root.join("pocketic-root.der");
    std::fs::write(&trust, key).unwrap();
    canic_host::network::enroll_network(canic_host::network::NetworkEnrollmentOptions {
        workspace_root: &consumer.root,
        environment: "local",
        root_key: &trust,
        fingerprint: &sha256_hex(key),
    })
    .unwrap();
    std::fs::create_dir_all(consumer.root.join("deployments")).unwrap();
    std::fs::write(
        consumer.root.join("deployments/packaged.toml"),
        generated_journey_policy(operator, subnet, 1, 1, config),
    )
    .unwrap();
    let mut args = base_args(route);
    args.extend(strings(&[
        "fleet",
        "generate",
        "packaged",
        "--app-config",
        "apps/downstream/canic.toml",
        "--release-build",
        release,
        "--fresh",
        "--management-creation-fee-cycles",
        "0",
    ]));
    assert_success(&consumer.run(&args));
}

fn converge(consumer: &Consumer, mut args: Vec<String>) -> Vec<String> {
    let mut faults = BTreeSet::new();
    let mut reviews = BTreeSet::new();
    let mut successor_pauses = BTreeSet::new();
    let mut approved = None;
    loop {
        let output = consumer.run(&args);
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .find(|value| value["event"] == "fleet_ensure_error")
                .unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(&output.stderr)));
            if error["code"] == "successor_review_required" {
                assert!(
                    successor_pauses.insert(args.clone()),
                    "one review per paused approval"
                );
                args = arguments(&error["next_action"]);
                continue;
            }
            let observed = [
                "failed-before-install",
                "lost-install-response",
                "lost-controller-response",
            ]
            .into_iter()
            .filter(|name| consumer.root.join(name).exists())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
            assert!(
                !observed.is_subset(&faults),
                "unexpected failure after known injected faults: {error}"
            );
            faults = observed;
            assert!(
                approved.as_ref() == Some(&args),
                "only an exact approved invocation may resume"
            );
            continue;
        }
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let automation = &result["automation"];
        assert!(automation.is_object());
        if automation["fleet_completed"] == true {
            assert!(automation["next_action"].is_null());
            assert!(!faults.is_empty());
            return approved.unwrap();
        }
        let next = &automation["next_action"];
        args = arguments(next);
        match next["kind"].as_str().unwrap() {
            "apply" => {
                // The test operator explicitly approves each new disposable-estate review.
                assert_eq!(next["requires_approval"], true);
                assert!(reviews.insert(automation["plan_sha256"].as_str().unwrap().to_owned()));
                approved = Some(args.clone());
            }
            "resume" => {
                assert_eq!(approved.as_ref(), Some(&args));
            }
            "review" => {
                assert_eq!(next["requires_approval"], false);
            }
            other => panic!("unhandled operator decision: {other}"),
        }
    }
}

fn verify_installed_artifacts(
    consumer: &Consumer,
    pic: &PocketIc,
    operator: Principal,
    desired: &DesiredFleet,
    infrastructure: &[(DesiredCanisterKind, Principal)],
) {
    for (kind, id) in infrastructure {
        let canister = desired
            .canisters
            .iter()
            .find(|canister| canister.kind == *kind)
            .unwrap();
        let bytes = std::fs::read(consumer.root.join(canister.wasm.as_ref().unwrap())).unwrap();
        let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
            let mut raw = Vec::new();
            flate2::read::GzDecoder::new(bytes.as_slice())
                .read_to_end(&mut raw)
                .unwrap();
            raw
        } else {
            bytes
        };
        let installed = pic.canister_status(*id, Some(operator)).unwrap();
        assert_eq!(hex_bytes(installed.module_hash.unwrap()), sha256_hex(&raw));
    }
}

fn mutation_counts(root: &Path) -> Vec<Vec<u8>> {
    [
        "controller-mutations.log",
        "reinstall-mutations.log",
        "funding-requests.log",
    ]
    .into_iter()
    .map(|name| std::fs::read(root.join(name)).unwrap_or_default())
    .collect()
}

fn arguments(action: &serde_json::Value) -> Vec<String> {
    assert_eq!(action["executable"], "canic");
    action["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|arg| arg.as_str().unwrap().to_owned())
        .collect()
}

fn strings(arguments: &[&str]) -> Vec<String> {
    arguments.iter().map(|arg| (*arg).to_owned()).collect()
}

fn base_args(route: &Path) -> Vec<String> {
    strings(&["--environment", "local", "--icp", route.to_str().unwrap()])
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
