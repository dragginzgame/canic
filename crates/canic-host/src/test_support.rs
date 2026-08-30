use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use canic_core::{
    cdk::types::Cycles,
    ids::{
        CyclesFundingBudget, FleetFundingProfile, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy,
    },
};
use ic_testkit::pic::{PocketIc, PocketIcBuilder, PocketIcBuilderExt, PocketIcStartupConfig};

const POCKET_IC_SERVER_URL_ENV: &str = "CANIC_POCKET_IC_SERVER_URL";
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
    builder
        .try_build(PocketIcStartupConfig::connect(
            server_url,
            POCKET_IC_STARTUP_TIMEOUT,
        ))
        .unwrap_or_else(|error| panic!("start governed PocketIC instance: {error}"))
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

#[cfg(unix)]
pub fn create_fifo(path: &Path) {
    let status = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()
        .expect("run mkfifo");
    assert!(status.success(), "mkfifo failed with {status}");
}
