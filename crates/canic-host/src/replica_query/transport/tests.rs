use super::{local_replica_endpoint_with_port, split_http_body};

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

#[test]
fn http_status_parsing_requires_a_real_success_code() {
    let success = b"HTTP/1.1 204 No Content\r\nContent-Length: 3\r\n\r\nabc";
    assert_eq!(split_http_body(success).expect("2xx response"), b"abc");

    let false_prefix = b"HTTP/1.1 20 Invalid\r\nContent-Length: 0\r\n\r\n";
    assert!(split_http_body(false_prefix).is_err());

    let failure = b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n";
    assert!(split_http_body(failure).is_err());
}
