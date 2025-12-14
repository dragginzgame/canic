use crate::{
    Error, ThisError,
    ids::CanisterRole,
    model::memory::directory::{DirectoryView, PrincipalList, SubnetDirectory},
    ops::{
        config::ConfigOps,
        model::memory::{
            MemoryOpsError,
            directory::{DirectoryPageDto, paginate},
            env::EnvOps,
            topology::SubnetCanisterRegistryOps,
        },
    },
    types::PageRequest,
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
        MemoryOpsError::from(err).into()
    }
}

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    /// Single source of truth: where do we get the directory?
    fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            Self::root_build_view()
        } else {
            SubnetDirectory::view()
        }
    }

    pub fn try_get(role: &CanisterRole) -> Result<PrincipalList, Error> {
        let view = Self::resolve_view();

        view.iter()
            .find_map(|(t, pids)| (t == role).then_some(pids.clone()))
            .ok_or_else(|| SubnetDirectoryOpsError::NotFound(role.clone()).into())
    }

    pub fn page(request: PageRequest) -> Result<DirectoryPageDto, Error> {
        Ok(paginate(Self::resolve_view(), request))
    }

    #[must_use]
    pub fn export() -> DirectoryView {
        Self::resolve_view()
    }

    pub fn import(view: DirectoryView) {
        SubnetDirectory::import(view);
    }

    /// Build SubnetDirectory for the current subnet from the registry.
    #[must_use]
    pub fn root_build_view() -> DirectoryView {
        let Ok(subnet_cfg) = ConfigOps::current_subnet() else {
            return Vec::new();
        };

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
