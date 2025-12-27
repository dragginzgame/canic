use crate::dto::{
    canister::CanisterSummaryView,
    directory::DirectoryView,
    prelude::*,
    state::{AppStateView, SubnetStateView},
};

///
/// StateBundle
/// Snapshot of mutable state and directory sections that can be propagated to peers
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct StateBundle {
    // states
    pub app_state: Option<AppStateView>,
    pub subnet_state: Option<SubnetStateView>,

    // directories
    pub app_directory: Option<DirectoryView>,
    pub subnet_directory: Option<DirectoryView>,
}

impl StateBundle {
    /// Whether the bundle carries any sections.
    /// Returns true when every optional field is absent.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.app_state.is_none()
            && self.subnet_state.is_none()
            && self.app_directory.is_none()
            && self.subnet_directory.is_none()
    }

    /// Compact debug string showing which sections are present.
    /// Example: `[as ss .. sd]`
    #[must_use]
    pub fn debug(&self) -> String {
        const fn fmt(present: bool, code: &str) -> &str {
            if present { code } else { ".." }
        }

        format!(
            "[{} {} {} {}]",
            fmt(self.app_state.is_some(), "as"),
            fmt(self.subnet_state.is_some(), "ss"),
            fmt(self.app_directory.is_some(), "ad"),
            fmt(self.subnet_directory.is_some(), "sd"),
        )
    }
}

///
/// TopologyBundle
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub parents: Vec<CanisterSummaryView>,
    pub children_map: HashMap<Principal, Vec<CanisterSummaryView>>,
}
