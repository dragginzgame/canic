//! Verify the production ICP command and native response boundary.

use super::*;
use canic_contracts::dto::{
    page::Page,
    public_status::{PublicMetricsSnapshot, PublicSnapshotState},
};

#[test]
fn wire_metrics_command_uses_binary_input_and_raw_json_output() {
    let icp = IcpCli::new("icp", Some("local".into()));
    let command = icp.bounded_query_command(
        "aaaaa-aa",
        protocol::CANIC_PUBLIC_METRICS,
        Path::new("request.bin"),
        None,
    );
    let args = command
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect::<Vec<_>>();
    assert!(args.windows(2).any(|pair| pair == ["--args-format", "bin"]));
    assert!(args.windows(2).any(|pair| pair == ["--output", "hex"]));
    assert!(args.contains(&"--json"));
    assert!(args.contains(&"--query"));
    assert!(!args.contains(&"--candid"));
}

#[test]
fn wire_metrics_reply_preserves_state_and_rejects_identity_or_budget_drift() {
    let snapshot = PublicMetricsSnapshot {
        family: PublicMetricFamily::Application,
        state: PublicSnapshotState::Disabled,
        sampled_at_ns: None,
        stale_after_ns: 42,
        truncated: false,
        metrics: Page {
            entries: vec![],
            total: 0,
        },
    };
    let frame = ic_wire::native::encode_response(
        protocol::public_metrics::METHOD_ID,
        &snapshot,
        protocol::public_metrics::RESPONSE_LIMITS,
    )
    .unwrap();
    let envelope =
        serde_json::json!({"response_bytes":canic_core::cdk::utils::hash::hex_bytes(&frame)})
            .to_string();
    let decoded = decode_public_reply::<PublicMetricsSnapshot>(
        &envelope,
        4096,
        protocol::public_metrics::METHOD_ID,
        protocol::public_metrics::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(decoded.family, snapshot.family);
    assert_eq!(decoded.state, snapshot.state);
    assert_eq!(decoded.stale_after_ns, snapshot.stale_after_ns);
    assert!(matches!(
        decode_public_reply::<PublicMetricsSnapshot>(
            &envelope,
            envelope.len() - 1,
            protocol::public_metrics::METHOD_ID,
            protocol::public_metrics::RESPONSE_LIMITS
        ),
        Err(ObservationFailure::InvalidResponse)
    ));
    let wrong = ic_wire::native::encode_response(
        protocol::public_health::METHOD_ID,
        &snapshot,
        protocol::public_metrics::RESPONSE_LIMITS,
    )
    .unwrap();
    let envelope =
        serde_json::json!({"response_bytes":canic_core::cdk::utils::hash::hex_bytes(&wrong)})
            .to_string();
    assert!(matches!(
        decode_public_reply::<PublicMetricsSnapshot>(
            &envelope,
            4096,
            protocol::public_metrics::METHOD_ID,
            protocol::public_metrics::RESPONSE_LIMITS
        ),
        Err(ObservationFailure::InvalidResponse)
    ));
}

#[test]
fn wire_history_reply_preserves_cost_window_and_rejects_wrong_method() {
    let snapshot = PublicHistorySnapshot {
        state: PublicSnapshotState::Unavailable,
        unit: None,
        heap_started_at_ns: Some(123),
        canister_version: 456,
        coverage_start_ns: None,
        cadence_ns: 300,
        retention_ns: 86400,
        stale_after_ns: 600,
        truncated: false,
        series_limit: 256,
        byte_limit: 8_388_608,
        reserved_bytes: 0,
        points: Page {
            entries: vec![],
            total: 0,
        },
    };
    let frame = ic_wire::native::encode_response(
        protocol::public_history::METHOD_ID,
        &snapshot,
        protocol::public_history::RESPONSE_LIMITS,
    )
    .unwrap();
    let envelope =
        serde_json::json!({"response_bytes": canic_core::cdk::utils::hash::hex_bytes(&frame)})
            .to_string();
    let reply: PublicHistorySnapshot = decode_public_reply(
        &envelope,
        4096,
        protocol::public_history::METHOD_ID,
        protocol::public_history::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(reply.heap_started_at_ns, Some(123));
    assert_eq!(reply.canister_version, 456);
    assert_eq!(reply.state, PublicSnapshotState::Unavailable);
    assert!(matches!(
        decode_public_reply::<PublicHistorySnapshot>(
            &envelope,
            4096,
            protocol::public_metrics::METHOD_ID,
            protocol::public_history::RESPONSE_LIMITS
        ),
        Err(ObservationFailure::InvalidResponse)
    ));
}

#[test]
fn native_storage_refusal_keeps_the_remote_diagnostic_code() {
    let error = canic_contracts::dto::error::Error::from_registered(
        canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
    );
    let frame = ic_wire::native::encode_response(
        protocol::store_inventory::storage::METHOD_ID,
        &Err::<WasmStoreStatusResponse, _>(error),
        protocol::store_inventory::RESPONSE_LIMITS,
    )
    .unwrap();
    let envelope =
        serde_json::json!({"response_bytes": canic_core::cdk::utils::hash::hex_bytes(&frame)})
            .to_string();
    let reply: Result<WasmStoreStatusResponse, canic_contracts::dto::error::Error> =
        decode_public_reply(
            &envelope,
            4096,
            protocol::store_inventory::storage::METHOD_ID,
            protocol::store_inventory::RESPONSE_LIMITS,
        )
        .unwrap();
    assert_eq!(
        response_failure(IcpJsonResponseError::Rejected(reply.unwrap_err())),
        ObservationFailure::Rejected {
            code: u32::from(error.raw_code())
        }
    );
}

#[test]
fn missing_installed_schema_prevents_native_storage_transport() {
    let icp = IcpCli::new("/does-not-exist", Some("local".into()));
    let mut transport = IcpObservatoryTransport {
        icp: &icp,
        root: Path::new("/does-not-exist"),
        fleet: "local",
        environment: "local",
        maximum_response_bytes: 4096,
        attempted_queries: 0,
        query_timeout: std::time::Duration::from_secs(1),
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
        compatibility: Some(Ok(())),
    };
    let entry = RegistryEntry {
        pid: Principal::anonymous().to_text(),
        role: Some("wasm_store".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    assert!(matches!(
        transport.store(&entry),
        Err(ObservationFailure::BindingUnavailable)
    ));
    assert_eq!(transport.attempts(), 0);
}
