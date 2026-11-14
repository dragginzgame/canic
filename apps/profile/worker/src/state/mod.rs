use canic_profile_protocol::{
    LIMITS, MAX_NAME_BYTES, MAX_PROFILES, Profile, ProfileError, ProfileResult, Put,
};
use ic_storage::{
    AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryBacking, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::{Bound, CodecError, StableBTreeMap, Storable},
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[cfg(test)]
mod tests;

pub(crate) const MAX_PAGES: u64 = 32;

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
        ic_wire::native::encode(self, LIMITS)
            .map(Cow::Owned)
            .map_err(|_| CodecError::Invalid("profile encoding"))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let record: Self = ic_wire::native::decode(&bytes, LIMITS)
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

pub(crate) struct State<M: MemoryBacking> {
    profiles: StableBTreeMap<u64, Record, M>,
    owners: StableBTreeMap<Owner, u64, M>,
    runtime: MemoryRuntime<M>,
}

impl<M: MemoryBacking> State<M> {
    pub fn open(backing: M, fresh: bool) -> Self {
        let mut runtime = MemoryRuntime::new_with_config(
            backing,
            StorageConfig {
                allocations: 2,
                extents: 3,
                pages: MAX_PAGES,
            },
        )
        .expect("admit profile backing");
        let declarations = SealedDeclarationSnapshot::new(vec![
            MemoryRequest::new("profile", "profile.rows.v1").expect("constant request"),
            MemoryRequest::new("profile", "profile.owners.v1").expect("constant request"),
        ])
        .expect("constant declarations");
        let grants = AllocationGrants::new(vec![
            MemoryAuthority::new("profile", "profile.").expect("constant namespace"),
        ])
        .expect("constant grants");
        if !fresh {
            // Refuse missing allocation identities before bootstrap can create them.
            let diagnostics = runtime
                .diagnostics()
                .expect("retained allocation integrity");
            assert_eq!(
                diagnostics.allocations, 2,
                "missing retained profile allocations"
            );
        }
        runtime
            .bootstrap(&declarations, &grants, &GenericAllocationPolicy)
            .expect("commit profile admission");
        let profiles = runtime
            .open_memory(&StableKey::parse("profile.rows.v1").expect("constant key"))
            .expect("profile lease");
        let owners = runtime
            .open_memory(&StableKey::parse("profile.owners.v1").expect("constant key"))
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
        let state = Self {
            profiles,
            owners,
            runtime,
        };
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
        assert!(
            self.runtime
                .diagnostics()
                .expect("storage integrity")
                .physical_pages
                <= MAX_PAGES
        );
    }

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
