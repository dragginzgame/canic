//! Module: canic_cli::cli::defaults
//!
//! Responsibility: provide shared CLI default values.
//! Does not own: global option parsing, command dispatch, or environment selection policy.
//! Boundary: returns stable defaults used when callers omit command options.

const DEFAULT_ICP: &str = "icp";
const LOCAL_NETWORK: &str = "local";

/// Default ICP CLI executable name used when `--icp` is omitted.
pub fn default_icp() -> String {
    DEFAULT_ICP.to_string()
}

/// Default ICP network used when `--network` is omitted.
pub fn local_network() -> String {
    LOCAL_NETWORK.to_string()
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Keep omitted --icp behavior tied to the ordinary ICP executable.
    #[test]
    fn default_icp_is_icp() {
        assert_eq!(default_icp(), "icp");
    }

    // Keep omitted --network behavior tied to the local replica.
    #[test]
    fn local_network_is_always_local() {
        assert_eq!(local_network(), "local");
    }
}
