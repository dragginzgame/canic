//! Count retained allocations before issuing a fresh create.

use super::{Acquisition, Placement, Pools, ProjectClaim, stored};

#[test]
fn pending_allocations_stop_growth_at_the_default_ceiling() {
    let mut pools = Pools::default();
    let limit = stored::DEFAULT_SUBNET_CHILD_LIMIT;
    for ordinal in 0..limit {
        let mut key = [0; 32];
        key[..4].copy_from_slice(&ordinal.to_be_bytes());
        assert!(matches!(pools.acquisition(), Ok(Acquisition::Create)));
        pools.projects.push(ProjectClaim {
            key,
            generation: key,
            principal: None,
        });
    }
    assert!(matches!(pools.acquisition(), Err(Placement::Full)));
    pools.capacity = Some(stored::SubnetCapacity {
        root: "test-root".into(),
        max_children: limit + 1,
        allow_large_subnet: true,
    });
    assert!(matches!(pools.acquisition(), Ok(Acquisition::Create)));
}
