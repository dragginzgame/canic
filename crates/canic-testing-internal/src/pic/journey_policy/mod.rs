//! Module: pic::journey_policy
//!
//! Responsibility: derive admission limits for native and PocketIC policy fixtures.
//! Boundary: test-only policy input construction; production admission remains authoritative.

mod tests;

use canic_host::release_set::AppConfigSnapshot;
use std::collections::BTreeMap;

/// Grouped fixtures admit their largest group; ungrouped fixtures retain declared limits.
pub(in crate::pic) fn component_admissions(
    configuration: &AppConfigSnapshot,
) -> BTreeMap<String, u32> {
    let model = configuration.model();
    model
        .component_specs
        .iter()
        .map(|(name, spec)| {
            let members = model
                .component_groups
                .values()
                .map(|group| {
                    u32::try_from(
                        group
                            .components
                            .values()
                            .filter(|member| &member.component_spec == name)
                            .count(),
                    )
                    .expect("configured group membership fits admission capacity")
                })
                .max()
                .unwrap_or(spec.maximum_instances);
            (name.to_string(), members)
        })
        .collect()
}
