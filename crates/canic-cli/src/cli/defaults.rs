//! Module: canic_cli::cli::defaults
//!
//! Responsibility: provide shared CLI default values.
//! Does not own: global option parsing, command dispatch, or environment selection policy.
//! Boundary: returns stable defaults used when callers omit command options.

const DEFAULT_ICP: &str = "icp";
const LOCAL_ENVIRONMENT: &str = "local";

/// Default ICP CLI executable name used when `--icp` is omitted.
pub fn default_icp() -> String {
    DEFAULT_ICP.to_string()
}

/// Default ICP environment used when `--environment` is omitted.
pub fn local_environment() -> String {
    LOCAL_ENVIRONMENT.to_string()
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

    // Keep omitted --environment behavior tied to the local replica.
    #[test]
    fn local_environment_is_always_local() {
        assert_eq!(local_environment(), "local");
    }
}
