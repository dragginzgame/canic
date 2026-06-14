use super::local_replica_endpoint_with_port;

// Ensure direct local queries use the ICP CLI local endpoint fallback when no project port is configured.
#[test]
fn local_replica_endpoint_defaults_to_icp_cli_port() {
    assert_eq!(
        local_replica_endpoint_with_port(None, None),
        "http://127.0.0.1:8000"
    );
    assert_eq!(
        local_replica_endpoint_with_port(None, Some(8001)),
        "http://127.0.0.1:8001"
    );
    assert_eq!(
        local_replica_endpoint_with_port(Some("http://127.0.0.1:9000/"), Some(8001)),
        "http://127.0.0.1:9000"
    );
}
