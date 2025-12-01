use crate::{
    Error, ThisError,
    model::memory::directory::{DirectoryView, PrincipalList, SubnetDirectory},
    ops::model::memory::{
        MemoryOpsError,
        directory::{DirectoryPageDto, paginate},
    },
    types::CanisterType,
};

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
    #[must_use]
    pub fn export() -> DirectoryView {
        SubnetDirectory::export()
    }

    pub fn import(view: DirectoryView) {
        SubnetDirectory::import(view);
    }

    pub fn page(offset: u64, limit: u64) -> Result<DirectoryPageDto, Error> {
        Ok(paginate(Self::export(), offset, limit))
    }

    /// Fetch principals for a canister type from the current AppDirectory.
    pub fn try_get(ty: &CanisterType) -> Result<PrincipalList, Error> {
        let entry = SubnetDirectory::get(ty)
            .ok_or_else(|| SubnetDirectoryOpsError::NotFound(ty.clone()))?;

        Ok(entry)
    }
}
