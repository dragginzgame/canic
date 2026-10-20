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
#[ic_platform::cdk::update(
    guard = "require_local_controller",
    decode_with = "decode_retain",
    encode_with = "encode_balance"
)]
fn audit_recovery_balance(retain: u128) -> u128 {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    ic_platform::cdk::api::cycles_burn(excess)
}

canic::finish!();

/// Decode the existing single-argument audit contract with the CDK skipping quota.
fn decode_retain(bytes: Vec<u8>) -> u128 {
    use ic_platform::candid::decoder::{DecoderConfig, decode_one_with_config};
    let mut config = DecoderConfig::new();
    config.set_skipping_quota(10_000);
    decode_one_with_config(&bytes, &config)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}

/// Encode the audit method's existing single Candid balance reply.
fn encode_balance(balance: u128) -> Vec<u8> {
    ic_platform::candid::contract::encode_one(balance)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}
