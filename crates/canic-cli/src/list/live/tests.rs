use super::*;

// Ensure a panicked live-query worker cannot silently remove its canister.
#[test]
fn live_query_worker_panic_is_reported_for_its_canister() {
    let registry = vec![RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    }];

    let values =
        collect_visible_entry_values(&registry, None, OBSERVATION_ERROR.to_string(), |_| {
            panic!("simulated query worker panic")
        })
        .expect("visible registry entries should resolve");

    assert_eq!(
        values.get("aaaaa-aa").map(String::as_str),
        Some(OBSERVATION_ERROR)
    );
}
