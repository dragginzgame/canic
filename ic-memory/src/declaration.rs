use crate::{
    key::{StableKey, StableKeyError},
    schema::{SchemaMetadata, SchemaMetadataError},
    slot::AllocationSlotDescriptor,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

///
/// AllocationDeclaration
///
/// Data-only claim that a stable key owns an allocation slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationDeclaration {
    /// Durable stable key.
    pub stable_key: StableKey,
    /// Claimed allocation slot.
    pub slot: AllocationSlotDescriptor,
    /// Optional diagnostic label.
    pub label: Option<String>,
    /// Optional diagnostic schema metadata.
    pub schema: SchemaMetadata,
}

impl AllocationDeclaration {
    /// Build a declaration from raw parts.
    pub fn new(
        stable_key: impl AsRef<str>,
        slot: AllocationSlotDescriptor,
        label: Option<String>,
        schema: SchemaMetadata,
    ) -> Result<Self, DeclarationSnapshotError> {
        let stable_key = StableKey::parse(stable_key).map_err(DeclarationSnapshotError::Key)?;
        schema
            .validate()
            .map_err(DeclarationSnapshotError::SchemaMetadata)?;
        Ok(Self {
            stable_key,
            slot,
            label,
            schema,
        })
    }
}

///
/// DeclarationCollector
///
/// Mutable collection phase before a snapshot is sealed.
#[derive(Clone, Debug, Default)]
pub struct DeclarationCollector {
    declarations: Vec<AllocationDeclaration>,
}

impl DeclarationCollector {
    /// Add one allocation declaration.
    pub fn push(&mut self, declaration: AllocationDeclaration) {
        self.declarations.push(declaration);
    }

    /// Seal collected declarations into a duplicate-free snapshot.
    pub fn seal(self) -> Result<DeclarationSnapshot, DeclarationSnapshotError> {
        DeclarationSnapshot::new(self.declarations)
    }
}

///
/// DeclarationSnapshot
///
/// Immutable runtime declaration snapshot ready for policy and history validation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeclarationSnapshot {
    /// Runtime declarations.
    declarations: Vec<AllocationDeclaration>,
    /// Optional binary/runtime identity for generation diagnostics.
    runtime_fingerprint: Option<String>,
}

impl DeclarationSnapshot {
    /// Create and validate a declaration snapshot.
    pub fn new(declarations: Vec<AllocationDeclaration>) -> Result<Self, DeclarationSnapshotError> {
        reject_duplicates(&declarations)?;
        Ok(Self {
            declarations,
            runtime_fingerprint: None,
        })
    }

    /// Attach an optional runtime fingerprint.
    #[must_use]
    pub fn with_runtime_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.runtime_fingerprint = Some(fingerprint.into());
        self
    }

    /// Return true when the snapshot has no declarations.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// Return the number of declarations in the snapshot.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.declarations.len()
    }

    /// Borrow the sealed declarations.
    #[must_use]
    pub fn declarations(&self) -> &[AllocationDeclaration] {
        &self.declarations
    }

    /// Borrow the optional runtime fingerprint.
    #[must_use]
    pub fn runtime_fingerprint(&self) -> Option<&str> {
        self.runtime_fingerprint.as_deref()
    }

    pub(crate) fn into_parts(self) -> (Vec<AllocationDeclaration>, Option<String>) {
        (self.declarations, self.runtime_fingerprint)
    }
}

///
/// DeclarationSnapshotError
///
/// Declaration snapshot validation failure.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum DeclarationSnapshotError {
    /// Stable-key grammar failure.
    #[error(transparent)]
    Key(StableKeyError),
    /// Schema metadata encoding failure.
    #[error(transparent)]
    SchemaMetadata(SchemaMetadataError),
    /// A stable key appeared more than once in one snapshot.
    #[error("stable key '{0}' is declared more than once")]
    DuplicateStableKey(StableKey),
    /// An allocation slot appeared more than once in one snapshot.
    #[error("allocation slot '{0:?}' is declared more than once")]
    DuplicateSlot(AllocationSlotDescriptor),
}

fn reject_duplicates(
    declarations: &[AllocationDeclaration],
) -> Result<(), DeclarationSnapshotError> {
    let mut keys = BTreeSet::new();
    let mut slots = BTreeSet::new();

    for declaration in declarations {
        if !slots.insert(declaration.slot.clone()) {
            return Err(DeclarationSnapshotError::DuplicateSlot(
                declaration.slot.clone(),
            ));
        }
        if !keys.insert(declaration.stable_key.clone()) {
            return Err(DeclarationSnapshotError::DuplicateStableKey(
                declaration.stable_key.clone(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::AllocationSlotDescriptor;

    fn declaration(key: &str, id: u8) -> AllocationDeclaration {
        AllocationDeclaration::new(
            key,
            AllocationSlotDescriptor::memory_manager(id),
            None,
            SchemaMetadata::default(),
        )
        .expect("declaration")
    }

    #[test]
    fn rejects_duplicate_keys() {
        let err = DeclarationSnapshot::new(vec![
            declaration("app.users.v1", 100),
            declaration("app.users.v1", 101),
        ])
        .expect_err("duplicate key");

        assert!(matches!(
            err,
            DeclarationSnapshotError::DuplicateStableKey(_)
        ));
    }

    #[test]
    fn rejects_duplicate_slots() {
        let err = DeclarationSnapshot::new(vec![
            declaration("app.users.v1", 100),
            declaration("app.orders.v1", 100),
        ])
        .expect_err("duplicate slot");

        assert!(matches!(err, DeclarationSnapshotError::DuplicateSlot(_)));
    }
}
