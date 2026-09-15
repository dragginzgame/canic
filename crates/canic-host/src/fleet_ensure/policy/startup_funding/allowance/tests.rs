use super::*;
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::config::ComponentDeploymentConfiguration,
    ids::{FleetSubnetRootReleaseSet, ReleaseBuildNonce, ReleaseSetDigest},
};

fn limits() -> FundingLimits {
    FundingLimits {
        max_per_request: 30,
        max_per_child: 100,
        cooldown_secs: 20,
    }
}

fn usage() -> StartupChildAccounting {
    StartupChildAccounting {
        observed_at_ns: 70_000_000_000,
        accounted_cycles: 95,
        last_accounted_at_secs: 50,
        pending_operations: 0,
        reserved_cycles: Some(0),
    }
}

#[test]
fn allowance_matches_runtime_caps_cooldown_and_exhaustion() {
    let mut sample = usage();
    let result = accounting(limits(), &sample).unwrap();
    assert_eq!(result.remaining_after_charges_cycles, 5);
    assert_eq!(result.next_request_policy_cap_cycles, Some(5));
    assert_eq!(result.cooldown_remaining_secs, 0);
    sample.observed_at_ns -= 1_000_000_000;
    let result = accounting(limits(), &sample).unwrap();
    assert_eq!(result.cooldown_remaining_secs, 1);
    assert_eq!(result.next_request_policy_cap_cycles, Some(0));
    sample.observed_at_ns += 1_000_000_000;
    for charged in [100, 101, u128::MAX] {
        sample.accounted_cycles = charged;
        let result = accounting(limits(), &sample).unwrap();
        assert_eq!(result.remaining_after_charges_cycles, 0);
        assert_eq!(result.next_request_policy_cap_cycles, Some(0));
    }
    sample.accounted_cycles = 0;
    assert_eq!(
        accounting(limits(), &sample)
            .unwrap()
            .next_request_policy_cap_cycles,
        Some(30)
    );
}

#[test]
fn pending_reservations_are_not_subtracted_again_or_treated_as_settled() {
    let mut sample = usage();
    sample.pending_operations = 1;
    for reserved in [Some(30), Some(0), None] {
        sample.reserved_cycles = reserved;
        let result = accounting(limits(), &sample).unwrap();
        assert_eq!(result.remaining_after_charges_cycles, 5);
        assert_eq!(result.next_request_policy_cap_cycles, None);
    }
    sample.pending_operations = 0;
    assert_eq!(
        accounting(limits(), &sample),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
    sample.reserved_cycles = Some(0);
    sample.last_accounted_at_secs = 71;
    assert_eq!(
        accounting(limits(), &sample),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
}

#[test]
fn allowance_requires_the_selected_release_spec_and_role() {
    let mut config = super::super::tests::hub_config();
    let policy = &mut config
        .component_specs
        .get_mut("hubs")
        .unwrap()
        .cycles_funding;
    policy.max_per_request = Cycles::new(30);
    policy.max_per_child = Cycles::new(100);
    policy.cooldown_secs = 20;
    let topology = ComponentDeploymentConfiguration::compile(&config)
        .unwrap()
        .component_topology;
    let spec = &topology.component_specs[0];
    let release = ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([1; 32]));
    let binding = StartupChildFundingBinding {
        release_set: FleetSubnetRootReleaseSet {
            release_build_id: release,
            manifest_digest: ReleaseSetDigest::from_bytes([2; 32]),
        },
        spec_hash: spec.spec_hash,
        parent: "root".into(),
        role: spec.component_role.clone(),
        component_spec: spec.component_spec.clone(),
    };
    let expected = project(&config, &topology, release, &binding, &usage()).unwrap();
    assert_eq!(expected.next_request_policy_cap_cycles, Some(5));
    let mut changed = binding.clone();
    changed.spec_hash = [99; 32];
    assert_eq!(
        project(&config, &topology, release, &changed, &usage()),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    let mut changed = binding.clone();
    changed.release_set.release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([3; 32]));
    assert_eq!(
        project(&config, &topology, release, &changed, &usage()),
        Err(StartupUsageUnavailable::SelectedBuildNotInstalled)
    );
    let mut changed = binding;
    changed.role = "unregistered".into();
    assert_eq!(
        project(&config, &topology, release, &changed, &usage()),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
}
