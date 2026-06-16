use canic_core::bootstrap::compiled::MetricsProfile;

pub(super) fn randomness_source_label(source: impl std::fmt::Debug) -> String {
    format!("{source:?}").to_ascii_lowercase()
}

pub(super) const fn metrics_profile_label(profile: MetricsProfile) -> &'static str {
    match profile {
        MetricsProfile::Leaf => "leaf",
        MetricsProfile::Hub => "hub",
        MetricsProfile::Storage => "storage",
        MetricsProfile::Root => "root",
        MetricsProfile::Full => "full",
    }
}

pub(super) const fn metrics_profile_tiers_label(profile: MetricsProfile) -> &'static str {
    match profile {
        MetricsProfile::Leaf => "core,runtime,security",
        MetricsProfile::Hub => "core,placement,runtime,security",
        MetricsProfile::Storage => "core,runtime,storage",
        MetricsProfile::Root | MetricsProfile::Full => {
            "core,placement,platform,runtime,security,storage"
        }
    }
}
