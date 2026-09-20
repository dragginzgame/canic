//! Passive collection orchestration with an optimistic authority recheck.

use crate::{
    icp::IcpCli,
    observatory::{
        ObservatoryError,
        model::ObservatoryOptions,
        ops::{self, transport::IcpObservatoryTransport},
        policy,
        view::{ObservatoryComparisonView, ObservatorySnapshotView},
    },
};
use std::path::Path;

/// Compare two bounded local private reports; this path has no transport or workspace discovery.
pub fn compare_files(
    before: &Path,
    after: &Path,
    maximum_bytes: usize,
) -> Result<ObservatoryComparisonView, ObservatoryError> {
    let before = ops::comparison::read_snapshot(before, maximum_bytes)?;
    let after = ops::comparison::read_snapshot(after, maximum_bytes)?;
    ops::comparison::compare(&before, &after)
}

/// Observe one selected Fleet without a mutation lock, retries or controller changes.
pub fn snapshot(
    root: &Path,
    icp: &IcpCli,
    options: &ObservatoryOptions,
) -> Result<ObservatorySnapshotView, ObservatoryError> {
    policy::validate_options(options)?;
    if icp.environment() != Some(options.environment.as_str()) {
        return Err(ObservatoryError::Profile);
    }
    let source = ops::authority(root, options);
    let mut transport = IcpObservatoryTransport {
        icp,
        root,
        environment: &options.environment,
        maximum_response_bytes: options.maximum_response_bytes,
        attempted_queries: 0,
        deadline: std::time::Instant::now()
            + std::time::Duration::from_secs(u64::from(options.maximum_collection_secs)),
        compatibility: None,
        query_timeout: std::time::Duration::from_secs(u64::from(options.query_timeout_secs)),
    };
    let mut snapshot = ops::collect(
        source.as_ref().map_err(Clone::clone),
        options,
        &mut transport,
    )?;
    ops::observe_local_operation(root, options, &mut snapshot);
    if let Ok(before) = source {
        let after =
            ops::authority(root, options).map_err(|_| ObservatoryError::AuthorityChanged)?;
        if before != after {
            return Err(ObservatoryError::AuthorityChanged);
        }
    }
    Ok(snapshot)
}
