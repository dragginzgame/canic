use canic_profile_protocol::{
    LIMITS, MAX_NAME_BYTES, MAX_PROFILES, Profile, ProfileError, ProfileResult, Put,
};
use ic_storage::{
    MemoryBacking, MemoryRequest, MemoryRuntime, SealedDeclarationSnapshot, StableKey,
    adapter::{Bound, CodecError, StableBTreeMap, Storable},
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[cfg(test)]
mod tests;

/// Authority the application must explicitly grant in its Storage bootstrap.
pub const STORAGE_AUTHORITY: &str = "profile";
/// Stable namespace reserved for this application's profiles and owner bindings.
pub const STORAGE_PREFIX: &str = "profile.";
const ROWS: &str = "profile.rows.v1";
const OWNERS: &str = "profile.owners.v1";

/// Contribute the existing stable identities to the application's one bootstrap.
/// # Panics
/// Panics only if the compile-time authority or stable keys are invalid.
#[must_use]
pub fn memory_requests() -> [MemoryRequest; 2] {
    [ROWS, OWNERS].map(|key| MemoryRequest::new(STORAGE_AUTHORITY, key).expect("constant request"))
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Owner(Vec<u8>);

impl Storable for Owner {
    const BOUND: Bound = Bound::Bounded {
        max_size: 29,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(&self.0))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if bytes.len() > 29 || bytes.as_ref() == [4] {
            return Err(CodecError::Invalid("owner principal"));
        }
        Ok(Self(bytes.into_owned()))
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct Record {
    version: u8,
    profile: Profile,
}

impl Storable for Record {
    const BOUND: Bound = Bound::Bounded {
        max_size: 512,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS)
            .map(Cow::Owned)
            .map_err(|_| CodecError::Invalid("profile encoding"))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let record: Self = ic_wire::decode(&bytes, ic_wire::Encoding::Payload, LIMITS)
            .map_err(|_| CodecError::Invalid("profile encoding"))?;
        let profile = &record.profile;
        if record.version != 1
            || profile.user_id == 0
            || profile.revision == 0
            || profile.owner.len() > 29
            || profile.owner == [4]
            || profile.display_name.trim().is_empty()
            || profile.display_name.len() > MAX_NAME_BYTES
        {
            return Err(CodecError::Invalid("profile invariant"));
        }
        Ok(record)
    }
}

/// Application-owned profiles with immutable owner/user bindings and bounded storage.
/// An embedding application must authenticate the supplied owner in one fixed
/// identity namespace; an ingress caller and an issuer-qualified subject differ.
pub struct State<M: MemoryBacking> {
    profiles: StableBTreeMap<u64, Record, M>,
    owners: StableBTreeMap<Owner, u64, M>,
}

impl<M: MemoryBacking> State<M> {
    /// Create both stores in empty, explicitly admitted allocations.
    /// # Panics
    /// Traps on missing authority, nonempty allocations or storage failure.
    #[must_use]
    pub fn create(runtime: &MemoryRuntime<M>) -> Self {
        Self::load(runtime, true)
    }

    /// Strictly reopen existing stores; never initialize on missing or invalid state.
    /// # Panics
    /// Traps on missing authority, malformed stores or violated retained invariants.
    #[must_use]
    pub fn open(runtime: &MemoryRuntime<M>) -> Self {
        Self::load(runtime, false)
    }

    fn load(runtime: &MemoryRuntime<M>, fresh: bool) -> Self {
        runtime
            .verify_authority(
                &SealedDeclarationSnapshot::new(memory_requests().to_vec())
                    .expect("profile declarations"),
                STORAGE_AUTHORITY,
            )
            .expect("explicit profile authority");
        let profiles = runtime
            .open_memory(&StableKey::parse(ROWS).expect("constant key"))
            .expect("profile lease");
        let owners = runtime
            .open_memory(&StableKey::parse(OWNERS).expect("constant key"))
            .expect("owner lease");
        let profiles = if fresh {
            StableBTreeMap::create(profiles)
        } else {
            StableBTreeMap::load(profiles)
        }
        .expect("strict profile store opening");
        let owners = if fresh {
            StableBTreeMap::create(owners)
        } else {
            StableBTreeMap::load(owners)
        }
        .expect("strict owner store opening");
        let state = Self { profiles, owners };
        state.validate();
        state
    }

    fn validate(&self) {
        assert!(self.profiles.len() <= MAX_PROFILES, "profile capacity");
        assert_eq!(self.profiles.len(), self.owners.len(), "owner count");
        for entry in self.profiles.iter() {
            let id = *entry.key();
            let record = entry.value();
            assert_eq!(record.profile.user_id, id, "profile identity");
            assert_eq!(
                self.owners.get(&Owner(record.profile.owner)),
                Some(id),
                "owner binding"
            );
        }
    }

    /// Read a profile after the embedding application authenticates its owner.
    /// # Errors
    /// Refuses anonymous callers, unknown profiles and other owners.
    pub fn get(&self, caller: &[u8], user_id: u64) -> ProfileResult {
        if caller == [4] {
            return Err(ProfileError::Unauthenticated);
        }
        let record = self.profiles.get(&user_id).ok_or(ProfileError::NotFound)?;
        if record.profile.owner != caller {
            return Err(ProfileError::Forbidden);
        }
        Ok(record.profile)
    }

    /// Commit one owner-bound revision transition or return its exact replay.
    /// # Errors
    /// Refuses invalid input, capacity, owner/identity changes and stale revisions.
    /// # Panics
    /// Storage failures trap; the IC must roll back this entire message.
    pub fn put(&mut self, caller: &[u8], input: &Put) -> ProfileResult {
        if caller == [4] {
            return Err(ProfileError::Unauthenticated);
        }
        if caller.len() > 29
            || input.user_id == 0
            || input.display_name.trim().is_empty()
            || input.display_name.len() > MAX_NAME_BYTES
        {
            return Err(ProfileError::InvalidInput);
        }
        let owner = Owner(caller.to_vec());
        if self
            .owners
            .get(&owner)
            .is_some_and(|id| id != input.user_id)
        {
            return Err(ProfileError::AlreadyRegistered);
        }
        let revision = input
            .expected_revision
            .checked_add(1)
            .ok_or(ProfileError::RevisionConflict)?;
        if let Some(record) = self.profiles.get(&input.user_id) {
            let profile = record.profile;
            if profile.owner != caller {
                return Err(ProfileError::Forbidden);
            }
            if profile.revision == revision && profile.display_name == input.display_name {
                return Ok(profile);
            }
            if profile.revision != input.expected_revision {
                return Err(ProfileError::RevisionConflict);
            }
        } else {
            if input.expected_revision != 0 {
                return Err(ProfileError::RevisionConflict);
            }
            if self.profiles.len() == MAX_PROFILES {
                return Err(ProfileError::Capacity);
            }
        }
        let profile = Profile {
            user_id: input.user_id,
            owner: caller.to_vec(),
            revision,
            display_name: input.display_name.clone(),
        };
        self.profiles.insert(
            input.user_id,
            Record {
                version: 1,
                profile: profile.clone(),
            },
        );
        self.owners.insert(owner, input.user_id);
        Ok(profile)
    }
}
