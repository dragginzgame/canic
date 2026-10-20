canic::start_fleet_root!();

fn require_local_controller() -> Result<(), String> {
    if canic::access::env::build_network_local().is_ok()
        && ic_platform::cdk::api::is_controller(&ic_platform::cdk::api::msg_caller())
    {
        Ok(())
    } else {
        Err("audit balance control requires a local controller".into())
    }
}

/// Set up a disposable low-reserve Root without changing its retained operations.
/// Raw test instrumentation also works while the production activation fence is closed.
#[ic_platform::cdk::update(guard = "require_local_controller")]
fn audit_recovery_balance(retain: u128) -> u128 {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    ic_platform::cdk::api::cycles_burn(excess)
}

canic::finish!();
