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

/// Check the same IPv4 loopback gateway used by local replica status queries.
///
/// A TCP listener counts as present even if HTTP status is unhealthy or malformed.
/// Only an explicit connection refusal establishes absence; other errors propagate.
/// This observation grants no process ownership or termination authority.
pub fn local_replica_gateway_listening(port: u16) -> std::io::Result<bool> {
    use std::{
        io::ErrorKind,
        net::{Ipv4Addr, SocketAddr, TcpStream},
        time::Duration,
    };
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    match TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::ConnectionRefused => Ok(false),
        Err(error) => Err(error),
    }
}
