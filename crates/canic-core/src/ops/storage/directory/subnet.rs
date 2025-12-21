use crate::{
    Error, ThisError,
    dto::page::{Page, PageRequest},
    ids::CanisterRole,
    model::memory::directory::{DirectoryView, PrincipalList, SubnetDirectory},
    ops::{
        config::ConfigOps,
        storage::{
            StorageOpsError, directory::paginate, env::EnvOps, topology::SubnetCanisterRegistryOps,
        },
    },
};
use std::collections::BTreeMap;

///
/// SubnetDirectoryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetDirectoryOpsError {
    #[error("canister role {0} not found in subnet directory")]
    NotFound(CanisterRole),
}

impl From<SubnetDirectoryOpsError> for Error {
    fn from(err: SubnetDirectoryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// SubnetDirectoryOps
///
/// NOTE:
/// - `export()` is intended for snapshot/state export flows and is infallible.
/// - The only legitimate runtime error is "role not present", surfaced via `try_get`.
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    /// Single source of truth: where do we get the directory?
    ///
    /// Root canisters synthesize the directory from the registry (filtered by config).
    /// Non-root canisters read the imported in-memory directory.
    #[must_use]
    fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            Self::root_build_view()
        } else {
            SubnetDirectory::view()
        }
    }

    /// Get principals for a role, if present.
    pub fn try_get(role: &CanisterRole) -> Result<PrincipalList, Error> {
        let view = Self::resolve_view();

        view.iter()
            .find_map(|(t, pids)| (t == role).then_some(pids.clone()))
            .ok_or_else(|| SubnetDirectoryOpsError::NotFound(role.clone()).into())
    }

    /// Get principals for a role, panicking if the role is missing.
    #[must_use]
    pub fn get(role: &CanisterRole) -> PrincipalList {
        Self::try_get(role).expect("subnet directory missing configured role")
    }

    /// Page through the directory view.
    #[must_use]
    pub fn page(request: PageRequest) -> Page<(CanisterRole, PrincipalList)> {
        paginate(Self::resolve_view(), request)
    }

    /// Export the directory view.
    ///
    /// # Panics
    /// Panics if called before environment/config initialization for root canisters.
    #[must_use]
    pub fn export() -> DirectoryView {
        Self::resolve_view()
    }

    pub fn import(view: DirectoryView) {
        SubnetDirectory::import(view);
    }

    /// Build SubnetDirectory for the current subnet from the registry.
    ///
    /// Root-only path. This is infallible by policy: failure to determine the current
    /// subnet config is a startup/config invariant violation, not a recoverable runtime
    /// error.
    #[must_use]
    pub fn root_build_view() -> DirectoryView {
        let subnet_cfg = ConfigOps::current_subnet();

        let entries = SubnetCanisterRegistryOps::export();
        let mut map: BTreeMap<CanisterRole, PrincipalList> = BTreeMap::new();

        for entry in entries {
            let role = entry.role.clone();

            if subnet_cfg.subnet_directory.contains(&role) {
                map.entry(role).or_default().0.push(entry.pid);
            }
        }

        map.into_iter().collect()
    }
}
