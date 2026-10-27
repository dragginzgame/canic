//! Boundary projection; allocation mutation remains in the fixture workflow.

impl From<&crate::FixtureChildAllocation> for crate::protocol::ChildAllocation {
    fn from(value: &crate::FixtureChildAllocation) -> Self {
        Self {
            request_id: value.request_id,
            parent: value.parent.into(),
            canister_role: value.canister_role.clone(),
            extra_arg: value.extra_arg.clone(),
            child: value.child.into(),
            settled: value.settled,
            acknowledged: value.acknowledged,
        }
    }
}
