use super::transport;
use std::path::Path;

#[must_use]
pub fn local_replica_status_reachable_from_root(
    environment: Option<&str>,
    icp_root: &Path,
) -> bool {
    transport::get_http_status(&transport::local_replica_endpoint_from_root(
        environment,
        icp_root,
    ))
    .is_ok()
}
