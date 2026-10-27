use canic_contracts::{
    cycles::Cycles,
    ids::{
        CyclesFundingBudget, FleetFundingProfile, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy,
    },
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::PocketIcStartupConfig,
};

const POCKET_IC_SERVER_URL_ENV: &str = "IC_TESTKIT_POCKET_IC_URL";
const POCKET_IC_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

pub fn start_pocket_ic(builder: PocketIcBuilder) -> PocketIc {
    let server_url = std::env::var(POCKET_IC_SERVER_URL_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "{POCKET_IC_SERVER_URL_ENV} must name the governed PocketIC server; use the workspace test runner"
            )
        });
    PocketIcStartupConfig::Connect {
        server_url,
        timeout: POCKET_IC_STARTUP_TIMEOUT,
    }
    .build_instance(builder)
    .map_or_else(
        |error| panic!("start governed PocketIC instance: {error}"),
        |owned| {
            // Connect borrows the caller-owned server; no server custody is transferred.
            let (instance, server) = owned.into_parts();
            assert!(server.is_none(), "Connect must not acquire server custody");
            instance
        },
    )
}

pub fn fleet_subnet_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(10_000_000_000_000),
            target_balance: Cycles::new(30_000_000_000_000),
            cooldown_secs: 30 * 24 * 60 * 60,
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
        },
        icp_refill: None,
    }
}

// Build a unique temporary directory path for host tests that own cleanup.
pub fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

/// Resolve a temporary fixture before testing locked Cargo operations.
///
/// Cargo records inherited unused patches too, so hand-written package lists
/// are not a complete lockfile when workstation overrides are configured.
#[track_caller]
pub fn generate_fixture_lockfile(root: &Path) {
    let output = crate::cargo_command()
        .current_dir(root)
        .args(["generate-lockfile", "--offline"])
        .output()
        .expect("generate fixture lockfile");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
pub fn create_fifo(path: &Path) {
    let status = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()
        .expect("run mkfifo");
    assert!(status.success(), "mkfifo failed with {status}");
}

/// Fill fake executable responses from the maintained tool authorities.
pub fn tool_script(template: &str) -> String {
    template
        .replace("@ICP_VERSION@", icp_cli_version())
        .replace(
            "@IC_WASM_IDENTITY@",
            crate::ic_wasm::IC_WASM_VERSION_IDENTITY,
        )
        .replace(
            "@BINARYEN_IDENTITY@",
            crate::binaryen::BINARYEN_VERSION_IDENTITY,
        )
}

/// Read the repository's installed ICP version for unrelated command fixtures.
pub fn icp_cli_version() -> &'static str {
    ic_tool_pin("icp", "linux-x86_64", 1)
}

/// Project a field from the common reviewed repository setup matrix.
pub fn ic_tool_pin(tool: &str, host: &str, column: usize) -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../ci/ic-tools.tsv"
    ))
    .lines()
    .find_map(|line| {
        let fields: Vec<_> = line.split('\t').collect();
        (fields.first() == Some(&tool) && fields.get(2) == Some(&host)).then(|| fields[column])
    })
    .expect("common repository IC tool pin")
}

/// Public deterministic signer fixture; never a credential for an external network.
#[cfg(unix)]
pub fn synthetic_identity_pem(seed: u8) -> String {
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend([seed; 32]);
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut body = String::new();
    for chunk in der.as_chunks::<3>().0 {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        for shift in [18, 12, 6, 0] {
            body.push(char::from(alphabet[((value >> shift) & 63) as usize]));
        }
    }
    format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n")
}

/// Finite reply budgets for small mint, withdrawal and fee fixture responses.
pub const CANDID_REPLY_LIMITS: ic_platform::candid::DecodeLimits =
    ic_platform::candid::DecodeLimits {
        reply_bytes: 64 * 1024,
        decoding_quota: 1_000_000,
        skipping_quota: 100_000,
        type_entries: 256,
        header_bytes: 16 * 1024,
    };

/// Complete native observation for process fixtures; installed proofs use the actual Root.
pub fn inspection_status(
    status: canic_contracts::dto::canister::CanisterStatusType,
    controllers: Vec<ic_platform::principal::ExternalPrincipal>,
    module_hash: Option<Vec<u8>>,
    cycles: ic_platform::candid::contract::Nat,
) -> canic_contracts::dto::inspection::Response {
    use canic_contracts::dto::canister as dto;
    use ic_platform::candid::contract::Nat;
    let status = dto::CanisterStatusResponse {
        version: 0,
        status,
        module_hash,
        cycles,
        settings: dto::CanisterSettings {
            controllers,
            compute_allocation: Nat::from(0_u8),
            memory_allocation: Nat::from(0_u8),
            freezing_threshold: Nat::from(0_u8),
            reserved_cycles_limit: Nat::from(0_u8),
            log_visibility: dto::LogVisibility::Controllers,
            log_memory_limit: Nat::from(0_u8),
            wasm_memory_limit: Nat::from(0_u8),
            wasm_memory_threshold: Nat::from(0_u8),
            environment_variables: Vec::new(),
        },
        memory_size: Nat::from(0_u8),
        reserved_cycles: Nat::from(0_u8),
        idle_cycles_burned_per_day: Nat::from(0_u8),
        memory_metrics: dto::MemoryMetrics {
            wasm_memory_size: Nat::from(0_u8),
            stable_memory_size: Nat::from(0_u8),
            global_memory_size: Nat::from(0_u8),
            wasm_binary_size: Nat::from(0_u8),
            custom_sections_size: Nat::from(0_u8),
            canister_history_size: Nat::from(0_u8),
            wasm_chunk_store_size: Nat::from(0_u8),
            snapshots_size: Nat::from(0_u8),
        },
        query_stats: dto::QueryStats {
            num_calls_total: Nat::from(0_u8),
            num_instructions_total: Nat::from(0_u8),
            request_payload_bytes_total: Nat::from(0_u8),
            response_payload_bytes_total: Nat::from(0_u8),
        },
    };
    canic_contracts::dto::inspection::Response::InspectCanister(Box::new(
        status.try_into().unwrap(),
    ))
}

/// Hex-only ICP native output, with the same identity and budgets as the Root endpoint.
pub fn inspection_reply(
    response: Result<
        canic_contracts::dto::inspection::Response,
        canic_contracts::dto::error::Error,
    >,
) -> String {
    use canic_contracts::protocol::inspection as wire;
    canic_core::cdk::utils::hash::hex_bytes(
        ic_wire::native::encode_response(wire::METHOD_ID, &response, wire::RESPONSE_LIMITS)
            .unwrap(),
    )
}

/// Encode a complete native pool fixture with the production query identity.
pub fn pool_reply(
    response: Result<
        canic_contracts::dto::pool::CanisterPoolResponse,
        canic_contracts::dto::error::Error,
    >,
) -> String {
    let response = response.map(canic_contracts::dto::pool_inventory::Page::from);
    canic_core::cdk::utils::hash::hex_bytes(
        ic_wire::native::encode_response(
            canic_contracts::protocol::pool_inventory::METHOD_ID,
            &response,
            canic_contracts::protocol::pool_inventory::RESPONSE_LIMITS,
        )
        .unwrap(),
    )
}

pub use crate::release_set::test_artifact::{WIRE_SCHEMA, wire_wasm};
