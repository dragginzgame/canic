use super::{decode_cycle_balance_response, uses_local_replica_transport};
use crate::test_support::temp_dir;
use candid::Encode;
use canic_core::dto::error::Error as CanicError;
use std::fs;

#[test]
fn decodes_cycle_balance_response_bytes() {
    let response: Result<u128, CanicError> = Ok(99_999_000_000_000);
    let bytes = Encode!(&response).expect("encode cycle balance response");

    assert_eq!(
        decode_cycle_balance_response(&bytes).expect("decode cycle balance"),
        99_999_000_000_000
    );
}

#[test]
fn named_environment_uses_its_resolved_network_class() {
    let root = temp_dir("canic-replica-query-network-class");
    fs::create_dir_all(&root).expect("create project root");
    fs::write(
        root.join("icp.yaml"),
        "networks:\n  - name: local\n    mode: managed\n\nenvironments:\n  - name: caelum-backend\n    network: local\n  - name: production\n    network: ic\n",
    )
    .expect("write icp config");

    assert!(
        uses_local_replica_transport(Some("caelum-backend"), Some(&root))
            .expect("resolve named local environment")
    );
    assert!(
        !uses_local_replica_transport(Some("production"), Some(&root))
            .expect("resolve named IC environment")
    );
    fs::remove_dir_all(root).expect("remove project root");
}

#[test]
fn explicit_http_target_uses_direct_replica_transport() {
    assert!(
        uses_local_replica_transport(Some("http://127.0.0.1:8000"), None)
            .expect("classify HTTP target")
    );
}
