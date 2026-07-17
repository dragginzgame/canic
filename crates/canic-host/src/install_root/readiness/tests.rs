use super::*;
use crate::replica_query::ReplicaQueryError;

#[test]
fn local_bootstrap_status_preserves_direct_replica_failure() {
    let error = root_bootstrap_status(Path::new("."), "local", "not a principal", None)
        .expect_err("local bootstrap query must report its direct replica failure");

    assert!(matches!(
        error.downcast_ref::<ReplicaQueryError>(),
        Some(ReplicaQueryError::Query(_))
    ));
}
