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
            request: key,
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

#[test]
#[should_panic(expected = "permanent service reservation")]
fn permanent_claim_without_journal_approval_refuses_reopen() {
    let mut pools = Pools::default();
    pools.permanent.insert(super::ChildId::FIRST);
    super::validate_subnet_claims(&pools, &Default::default(), 0);
}
