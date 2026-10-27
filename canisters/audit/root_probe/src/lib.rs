mod protocol;

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

canic::__canic_register_raw_wire_schema!(
    "audit_recovery_balance",
    Update,
    protocol::BALANCE,
    u128,
    u128,
    protocol::REQUEST_LIMITS,
    protocol::RESPONSE_LIMITS
);

canic::finish!();

/// Decode a native request without opening the production activation fence.
fn decode_retain(bytes: Vec<u8>) -> u128 {
    ic_wire::native::decode_request(protocol::BALANCE, &bytes, protocol::REQUEST_LIMITS)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}

/// Reply with the exact method identity and native balance shape.
fn encode_balance(balance: u128) -> Vec<u8> {
    ic_wire::native::encode_response(protocol::BALANCE, &balance, protocol::RESPONSE_LIMITS)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}
