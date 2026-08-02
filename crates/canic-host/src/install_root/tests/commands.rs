use super::*;

#[test]
fn icp_canister_command_carries_selected_environment() {
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "ic", None);

    assert_eq!(command.get_program(), "icp");
    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
            "canister",
            "status",
            "root",
            "-e",
            "ic"
        ]
    );
}

#[test]
fn local_canister_command_uses_http_target_when_configured() {
    let target = LocalReplicaTarget {
        url: "http://127.0.0.1:8000".to_string(),
        root_key: "abcd".to_string(),
    };
    let mut command = icp_canister_command(Path::new("/tmp/canic-icp-root"));
    command.env("ICP_ENVIRONMENT", "local");
    command.args(["status", "root"]);
    add_icp_environment_target(&mut command, "local", Some(&target));

    assert_eq!(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [
            "--project-root-override",
            "/tmp/canic-icp-root",
            "canister",
            "status",
            "root",
            "-n",
            "http://127.0.0.1:8000",
            "-k",
            "abcd"
        ]
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "ICP_ENVIRONMENT" && value.is_none())
    );
}

#[test]
fn install_command_uses_binary_candid_file() {
    let canister = candid::Principal::from_slice(&[44]);
    let command = icp_canister_install_binary_args_command(
        Path::new("/workspace"),
        "caelum-backend",
        None,
        canister,
        Path::new("/artifacts/root.wasm"),
        Path::new("/state/root-install-args.bin"),
    );

    assert_eq!(
        crate::icp::command_display(&command),
        format!(
            "icp --project-root-override /workspace canister install {canister} --mode=install -y --wasm /artifacts/root.wasm --args-file /state/root-install-args.bin --args-format bin -e caelum-backend"
        )
    );
}

#[test]
fn create_command_binds_subnet_and_exact_cycles() {
    let subnet = canic_core::ids::SubnetId::from_principal(candid::Principal::from_slice(&[41]));
    let command = icp_canister_create_command(
        Path::new("/workspace"),
        "staging",
        None,
        subnet,
        &crate::fleet_install_plan::PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000,
        },
    );

    assert_eq!(
        crate::icp::command_display(&command),
        format!(
            "icp --project-root-override /workspace canister create --detached --json --subnet {subnet} --cycles 2000000000000 -e staging"
        )
    );
}

#[test]
fn create_command_preserves_exact_icp_e8s() {
    let subnet = canic_core::ids::SubnetId::from_principal(candid::Principal::from_slice(&[42]));
    let command = icp_canister_create_command(
        Path::new("/workspace"),
        "ic",
        None,
        subnet,
        &crate::fleet_install_plan::PlannedCanisterCreationFunding::Icp { e8s: 1 },
    );

    assert_eq!(
        crate::icp::command_display(&command),
        format!(
            "icp --project-root-override /workspace canister create --detached --json --subnet {subnet} --with-icp 0.00000001 -e ic"
        )
    );
}

#[test]
fn install_timing_summary_uses_standard_table_format() {
    let timings = InstallTimingSummary {
        create_canisters: Duration::from_millis(1200),
        build_all: Duration::from_millis(2340),
        emit_manifest: Duration::from_millis(10),
        install_root: Duration::from_millis(20),
    };

    let table = render_install_timing_summary(&timings, Duration::from_millis(3900));

    assert_eq!(
        table.lines().take(2).collect::<Vec<_>>(),
        vec!["PHASE              ELAPSED", "----------------   -------"]
    );
    assert!(
        table.lines().any(
            |line| line.split_whitespace().collect::<Vec<_>>() == ["create_canisters", "1.20s"]
        )
    );
    assert!(
        table
            .lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == ["install_root", "0.02s"])
    );
    assert!(
        table
            .lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == ["total", "3.90s"])
    );
}

#[test]
fn root_init_args_are_written_as_binary_candid() {
    use canic_core::{
        cdk::types::Cycles,
        dto::fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
        ids::{
            ComponentTopologyDigest, CyclesFundingBudget, FleetCoordinatorBinding,
            FleetRegistryAuthority, FleetSubnetRootBinding, FleetSubnetRootLimits,
            FleetSubnetRootReleaseSet, ReleaseSetDigest, SubnetId,
        },
    };

    let activation = sample_fleet_activation_identity();
    let fleet_subnet_root = candid::Principal::from_slice(&[42]);
    let identity = FleetSubnetRootInitArgs {
        authority: FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: activation.fleet,
                        coordinator_subnet: SubnetId::from_principal(
                            candid::Principal::from_slice(&[40]),
                        ),
                        coordinator: candid::Principal::from_slice(&[41]),
                    },
                    epoch: 1,
                },
                placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[43])),
                fleet_subnet_root,
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([5; 32]),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_managed_canisters: 100,
                    maximum_registry_bytes: 1_048_576,
                    maximum_wasm_store_bytes: 10_000_000,
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 3_600,
                        maximum_cycles: Cycles::new(1_000_000_000_000),
                    },
                },
            },
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id: activation.release_build_id,
                manifest_digest: ReleaseSetDigest::from_bytes([6; 32]),
            },
            expected_module_hash: [10; 32],
        },
        install_id: activation.operation_id,
    };

    let root = temp_dir("canic-binary-root-install-args");
    let path = root.join("root-install-args.bin");
    write_candid_args(&path, &identity).expect("write binary init args");
    let bytes = fs::read(&path).expect("read binary init args");
    let decoded: FleetSubnetRootInitArgs =
        candid::decode_one(&bytes).expect("decode init identity");

    assert_eq!(decoded, identity);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn renders_exact_icp_e8s_without_float_rounding() {
    assert_eq!(icp_e8s_text(1), "0.00000001");
    assert_eq!(icp_e8s_text(10_000_000), "0.1");
    assert_eq!(icp_e8s_text(100_000_000), "1");
    assert_eq!(icp_e8s_text(123_456_789), "1.23456789");
}
