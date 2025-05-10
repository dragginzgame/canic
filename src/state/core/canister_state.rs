use crate::{
    impl_storable_unbounded,
    structures::{DefaultMemory, cell::Cell},
};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// CanisterStateError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum CanisterStateError {
    #[error("path has not been set")]
    PathNotSet,

    #[error("root_id has not been set")]
    RootIdNotSet,
}

///
/// CanisterState
///

#[derive(Deref, DerefMut)]
pub struct CanisterState(Cell<CanisterStateData>);

impl CanisterState {
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(Cell::init(memory, CanisterStateData::default()).unwrap())
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> CanisterStateData {
        self.get()
    }

    // set_data
    pub fn set_data(&mut self, data: CanisterStateData) -> Result<(), CanisterStateError> {
        self.set(data).map_err(CanisterStateError::MimicError)?;

        Ok(())
    }

    // get_type
    pub fn get_type(&self) -> Result<CanisterType, CanisterStateError> {
        let ty = self.get().ty.ok_or(CanisterStateError::PathNotSet)?;

        Ok(ty)
    }

    // set_type
    pub fn set_type(&mut self, ty: CanisterType) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.ty = Some(ty);
        self.set(state)?;

        Ok(())
    }

    // get_root_id
    pub fn get_root_id(&self) -> Result<Principal, CanisterStateError> {
        let root_id = self.get().root_id.ok_or(CanisterStateError::RootIdNotSet)?;

        Ok(root_id)
    }

    // set_root_id
    pub fn set_root_id(&mut self, id: Principal) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.root_id = Some(id);
        self.set(state)?;

        Ok(())
    }

    // get_parent_id
    #[must_use]
    pub fn get_parent_id(&self) -> Option<Principal> {
        self.get().parent_id
    }

    // set_parent_id
    pub fn set_parent_id(&mut self, id: Principal) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.parent_id = Some(id);
        self.set(state)?;

        Ok(())
    }
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Serialize, Deserialize)]
pub struct CanisterStateData {
    ty: Option<CanisterType>,
    root_id: Option<Principal>,
    parent_id: Option<Principal>,
}

impl_storable_unbounded!(CanisterStateData);
