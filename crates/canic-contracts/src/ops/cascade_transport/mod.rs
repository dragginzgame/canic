//! Lossless transport projections for existing topology validation and cascade workflows.
use crate::dto::{cascade as stored, cascade_transport as native};

impl From<native::TopologySnapshot> for stored::TopologySnapshotInput {
    fn from(value: native::TopologySnapshot) -> Self {
        Self {
            parents: value.parents.into_iter().map(Into::into).collect(),
            children_map: value.children_map.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<stored::TopologySnapshotInput> for native::TopologySnapshot {
    fn from(value: stored::TopologySnapshotInput) -> Self {
        Self {
            parents: value.parents.into_iter().map(Into::into).collect(),
            children_map: value.children_map.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::PathNode> for stored::TopologyPathNode {
    fn from(value: native::PathNode) -> Self {
        Self {
            pid: value.pid.into(),
            role: value.role,
            parent_pid: value.parent_pid.map(Into::into),
        }
    }
}

impl From<stored::TopologyPathNode> for native::PathNode {
    fn from(value: stored::TopologyPathNode) -> Self {
        Self {
            pid: value.pid.into(),
            role: value.role,
            parent_pid: value.parent_pid.map(Into::into),
        }
    }
}

impl From<native::Children> for stored::TopologyChildren {
    fn from(value: native::Children) -> Self {
        Self {
            parent_pid: value.parent_pid.into(),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<stored::TopologyChildren> for native::Children {
    fn from(value: stored::TopologyChildren) -> Self {
        Self {
            parent_pid: value.parent_pid.into(),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::DirectChild> for stored::TopologyDirectChild {
    fn from(value: native::DirectChild) -> Self {
        Self {
            pid: value.pid.into(),
            role: value.role,
        }
    }
}

impl From<stored::TopologyDirectChild> for native::DirectChild {
    fn from(value: stored::TopologyDirectChild) -> Self {
        Self {
            pid: value.pid.into(),
            role: value.role,
        }
    }
}
