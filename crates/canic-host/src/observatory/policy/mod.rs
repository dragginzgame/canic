//! Pure collection budgets, profile validation and freshness decisions.

use crate::observatory::{
    ObservatoryError,
    model::{ObservatoryOptions, ObservatoryProfile},
};

/// Bound collections by the maintained Fleet inventory ceiling and explicit host budgets.
pub fn validate_options(options: &ObservatoryOptions) -> Result<(), ObservatoryError> {
    for name in [&options.environment, &options.fleet] {
        crate::component_operation::policy::validate_label(name)
            .map_err(|_| ObservatoryError::Profile)?;
    }
    if options.maximum_canisters == 0
        || options.maximum_canisters > crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS
    {
        return Err(ObservatoryError::Bound("canister selection"));
    }
    if !(1024..=16 * 1024 * 1024).contains(&options.maximum_response_bytes) {
        return Err(ObservatoryError::Bound("response bytes"));
    }
    if !(1..=3600).contains(&options.freshness_secs) {
        return Err(ObservatoryError::Bound("freshness interval"));
    }
    if !(1..=60).contains(&options.query_timeout_secs) {
        return Err(ObservatoryError::Bound("query timeout"));
    }
    if !(1..=3600).contains(&options.maximum_collection_secs) {
        return Err(ObservatoryError::Bound("collection timeout"));
    }
    Ok(())
}

/// Labels are plain bounded text. They cannot select endpoints or authorize collection.
pub fn validate_profile(profile: &ObservatoryProfile) -> Result<(), ObservatoryError> {
    if profile.schema_version != 1 || profile.role_labels.len() > 128 {
        return Err(ObservatoryError::Profile);
    }
    for text in std::iter::once(&profile.title)
        .chain(profile.role_labels.values())
        .chain(profile.role_labels.keys())
    {
        if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
            return Err(ObservatoryError::Profile);
        }
    }
    Ok(())
}

/// Future timestamps are unknown; elapsed TTL never becomes a fresh observation.
#[must_use]
pub const fn is_fresh(observed_at: u64, now: u64, ttl_secs: u32) -> bool {
    now >= observed_at && now - observed_at <= ttl_secs as u64 * 1000
}
