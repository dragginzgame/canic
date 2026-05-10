use canic_core::bootstrap::compiled::MetricsProfile;

pub const METRICS_TIER_CORE: u8 = 1 << 0;
pub const METRICS_TIER_PLACEMENT: u8 = 1 << 1;
pub const METRICS_TIER_PLATFORM: u8 = 1 << 2;
pub const METRICS_TIER_RUNTIME: u8 = 1 << 3;
pub const METRICS_TIER_SECURITY: u8 = 1 << 4;
pub const METRICS_TIER_STORAGE: u8 = 1 << 5;

#[must_use]
pub const fn metrics_profile_tier_mask(profile: MetricsProfile) -> u8 {
    match profile {
        MetricsProfile::Leaf => METRICS_TIER_CORE | METRICS_TIER_RUNTIME | METRICS_TIER_SECURITY,
        MetricsProfile::Hub => {
            METRICS_TIER_CORE
                | METRICS_TIER_PLACEMENT
                | METRICS_TIER_RUNTIME
                | METRICS_TIER_SECURITY
        }
        MetricsProfile::Storage => METRICS_TIER_CORE | METRICS_TIER_RUNTIME | METRICS_TIER_STORAGE,
        MetricsProfile::Root | MetricsProfile::Full => {
            METRICS_TIER_CORE
                | METRICS_TIER_PLACEMENT
                | METRICS_TIER_PLATFORM
                | METRICS_TIER_RUNTIME
                | METRICS_TIER_SECURITY
                | METRICS_TIER_STORAGE
        }
    }
}
