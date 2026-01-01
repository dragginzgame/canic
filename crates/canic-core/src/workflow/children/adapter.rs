pub fn topology_children_from_snapshot(
    snapshot: SubnetRegistrySnapshot,
    parent: Principal,
) -> Vec<TopologyNodeView> {
    snapshot
        .entries
        .into_iter()
        .filter_map(|(pid, entry)| {
            (entry.parent_pid == Some(parent))
                .then(|| canister_summary_to_topology_node(pid, &CanisterSummary::from(&entry)))
        })
        .collect()
}
