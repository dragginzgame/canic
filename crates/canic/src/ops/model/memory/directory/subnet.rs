use crate::{
    Error, ThisError,
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
    types::CanisterType,
};
use std::collections::BTreeMap;

///
/// SubnetDirectoryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetDirectoryOpsError {
    #[error("canister type {0} not found in subnet directory")]
    NotFound(CanisterType),
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

    pub fn try_get(ty: &CanisterType) -> Result<PrincipalList, Error> {
        let view = Self::resolve_view();

        view.iter()
            .find_map(|(t, pids)| (t == ty).then_some(pids.clone()))
            .ok_or_else(|| SubnetDirectoryOpsError::NotFound(ty.clone()).into())
    }

    pub fn page(offset: u64, limit: u64) -> Result<DirectoryPageDto, Error> {
        Ok(paginate(Self::resolve_view(), offset, limit))
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
        let mut map: BTreeMap<CanisterType, PrincipalList> = BTreeMap::new();

        for entry in entries {
            let ty = entry.ty.clone();

            if subnet_cfg.subnet_directory.contains(&ty) {
                map.entry(ty).or_default().0.push(entry.pid);
            }
        }

        map.into_iter().collect()
    }
}
