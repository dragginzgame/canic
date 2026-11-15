//! Single-message transitions. Storage failure traps; no partial typed-error mutation.
use canic_wasm_store_protocol::{
    Chunk, Error, Info, MAX_PINS, Manifest, PAGE_SIZE, Pin, PinReceipt, Put, Quota, Read, Release,
    Status, digest,
};
use ic_storage::{
    AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryBacking, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::{StableBTreeMap, StableCell},
};
use records::{Artifact, Key, Payload, RetainedPin, Settings};
use sha2::{Digest, Sha256};
use std::ops::Bound::{Excluded, Unbounded};
mod admission;
#[cfg(test)]
mod admission_tests;
mod preservation;
mod records;
#[cfg(test)]
mod tests;

pub(crate) struct State<M: MemoryBacking> {
    releases: StableBTreeMap<Key, [u8; 32], M>,
    artifacts: StableBTreeMap<[u8; 32], Artifact, M>,
    chunks: StableBTreeMap<[u8; 32], Payload, M>,
    settings: StableCell<Settings, M>,
    _runtime: MemoryRuntime<M>,
}
impl<M: MemoryBacking> State<M> {
    pub fn open(backing: M, fresh: bool) -> Self {
        let mut runtime = MemoryRuntime::new_with_config(
            backing,
            StorageConfig {
                allocations: 4,
                extents: 4096,
                pages: 16_384,
            },
        )
        .expect("Store backing");
        let names = [
            "store.releases.v1",
            "store.artifacts.v1",
            "store.chunks.v1",
            "store.settings.v1",
        ];
        let declarations = SealedDeclarationSnapshot::new(
            names
                .iter()
                .map(|name| MemoryRequest::new("store", name).expect("constant request"))
                .collect(),
        )
        .expect("Store declarations");
        let grants = AllocationGrants::new(vec![
            MemoryAuthority::new("store", "store.").expect("constant grant"),
        ])
        .expect("Store grants");
        if !fresh {
            assert_eq!(
                runtime.diagnostics().expect("Store integrity").allocations,
                4,
                "missing Store allocations"
            );
        }
        runtime
            .bootstrap(&declarations, &grants, &GenericAllocationPolicy)
            .expect("Store admission");
        let mut memories = names
            .map(|name| {
                runtime
                    .open_memory(&StableKey::parse(name).expect("constant key"))
                    .expect("Store lease")
            })
            .into_iter();
        let releases = memories.next().expect("release lease");
        let artifacts = memories.next().expect("artifact lease");
        let chunks = memories.next().expect("chunk lease");
        let settings = memories.next().expect("settings lease");
        let state = Self {
            releases: if fresh {
                StableBTreeMap::create(releases)
            } else {
                StableBTreeMap::load(releases)
            }
            .expect("release catalog"),
            artifacts: if fresh {
                StableBTreeMap::create(artifacts)
            } else {
                StableBTreeMap::load(artifacts)
            }
            .expect("artifact catalog"),
            chunks: if fresh {
                StableBTreeMap::create(chunks)
            } else {
                StableBTreeMap::load(chunks)
            }
            .expect("chunk storage"),
            settings: if fresh {
                StableCell::create(
                    settings,
                    Settings {
                        quota: Quota::default(),
                        reserved_bytes: 0,
                        admission: None,
                    },
                )
            } else {
                StableCell::load(settings, 16_384)
            }
            .expect("Store settings"),
            _runtime: runtime,
        };
        // Bounded metadata-only maintenance checks. Payloads are verified on publication/read.
        assert!(
            state.releases.len() <= state.settings.get().quota.releases,
            "release quota"
        );
        let reserved: u64 = state
            .artifacts
            .values()
            .map(|artifact| artifact.bytes)
            .sum();
        assert_eq!(
            reserved,
            state.settings.get().reserved_bytes,
            "artifact reservation custody"
        );
        for entry in state.releases.iter() {
            assert!(
                state.artifacts.contains_key(&entry.value()),
                "missing release artifact"
            );
        }
        state
    }

    fn artifact(&self, release: &Release) -> Result<([u8; 32], Artifact), Error> {
        if !release.valid() {
            return Err(Error::Invalid);
        }
        let hash = self
            .releases
            .get(&Key::new(release))
            .ok_or(Error::NotFound)?;
        Ok((hash, self.artifacts.get(&hash).expect("retained artifact")))
    }
    pub fn status(&self, release: &Release) -> Result<Status, Error> {
        let (hash, artifact) = self.artifact(release)?;
        let missing = artifact
            .chunks
            .iter()
            .enumerate()
            .filter(|(_, hash)| !self.chunks.contains_key(hash))
            .map(|(index, _)| index as u32)
            .collect();
        Ok(Status {
            manifest: Manifest {
                release: release.clone(),
                hash,
                bytes: artifact.bytes,
                chunks: artifact.chunks,
            },
            published: artifact.published,
            missing,
        })
    }
    pub fn custody(&self, release: &Release) -> Result<canic_wasm_store_protocol::Custody, Error> {
        let status = self.status(release)?;
        let artifact = self
            .artifacts
            .get(&status.manifest.hash)
            .expect("retained artifact");
        Ok(canic_wasm_store_protocol::Custody {
            status,
            pins: artifact
                .pins
                .into_iter()
                .map(|pin| canic_wasm_store_protocol::Retention {
                    owner: pin.owner,
                    operation: pin.operation,
                    release: pin.release,
                })
                .collect(),
        })
    }
    pub fn prepare(&mut self, manifest: &Manifest) -> Result<Status, Error> {
        self.admission_epoch()?;
        if !manifest.valid() {
            return Err(Error::Invalid);
        }
        let key = Key::new(&manifest.release);
        if self
            .releases
            .get(&key)
            .is_some_and(|existing| existing != manifest.hash)
        {
            return Err(Error::Conflict);
        }
        let retained = self.artifacts.get(&manifest.hash);
        if retained.as_ref().is_some_and(|artifact| {
            artifact.bytes != manifest.bytes || artifact.chunks != manifest.chunks
        }) {
            return Err(Error::Conflict);
        }
        if self.releases.contains_key(&key) {
            return self.status(&manifest.release);
        }
        let mut settings = self.settings.get().clone();
        let extra = if retained.is_none() {
            manifest.bytes
        } else {
            0
        };
        if self.releases.len() >= settings.quota.releases
            || settings.reserved_bytes + extra > settings.quota.reserved_bytes
        {
            return Err(Error::Capacity);
        }
        if retained.is_none() {
            self.artifacts.insert(
                manifest.hash,
                Artifact {
                    bytes: manifest.bytes,
                    chunks: manifest.chunks.clone(),
                    published: false,
                    pins: vec![],
                },
            );
            settings.reserved_bytes += extra;
            self.settings.set(settings);
        }
        self.releases.insert(key, manifest.hash);
        self.status(&manifest.release)
    }
    pub fn put(&mut self, input: &Put) -> Result<Status, Error> {
        self.admission_epoch()?;
        let (_, artifact) = self.artifact(&input.release)?;
        let index = input.index as usize;
        let expected = artifact.chunks.get(index).ok_or(Error::Invalid)?;
        if input.bytes.len() != artifact.chunk_bytes(index) {
            return Err(Error::Invalid);
        }
        if digest(&input.bytes) != *expected {
            return Err(Error::Hash);
        }
        // Content-addressed writes cannot overwrite a published or previously accepted chunk.
        if !self.chunks.contains_key(expected) {
            self.chunks.insert(*expected, Payload(input.bytes.clone()));
        }
        self.status(&input.release)
    }
    pub fn publish(&mut self, release: &Release) -> Result<Status, Error> {
        self.admission_epoch()?;
        let (hash, mut artifact) = self.artifact(release)?;
        if artifact.published {
            return self.status(release);
        }
        let mut full = Sha256::new();
        for (index, expected) in artifact.chunks.iter().enumerate() {
            let payload = self.chunks.get(expected).ok_or(Error::Incomplete)?;
            if payload.0.len() != artifact.chunk_bytes(index) || digest(&payload.0) != *expected {
                return Err(Error::Hash);
            }
            if index == 0 && !payload.0.starts_with(b"\0asm\x01\0\0\0") {
                return Err(Error::Invalid);
            }
            full.update(&payload.0);
        }
        if <[u8; 32]>::from(full.finalize()) != hash {
            return Err(Error::Hash);
        }
        artifact.published = true;
        self.artifacts.insert(hash, artifact);
        self.status(release)
    }
    pub fn read(&self, input: &Read) -> Result<Chunk, Error> {
        let (_, artifact) = self.artifact(&input.release)?;
        if !artifact.published {
            return Err(Error::Incomplete);
        }
        let index = input.index as usize;
        let hash = artifact.chunks.get(index).ok_or(Error::Invalid)?;
        let payload = self.chunks.get(hash).expect("published payload");
        assert_eq!(
            payload.0.len(),
            artifact.chunk_bytes(index),
            "published chunk size"
        );
        assert_eq!(digest(&payload.0), *hash, "published chunk digest");
        Ok(Chunk(payload.0))
    }
    pub fn list(&self, after: Option<&Release>) -> Result<Vec<Status>, Error> {
        if after.is_some_and(|release| !release.valid()) {
            return Err(Error::Invalid);
        }
        let lower = after.map_or(Unbounded, |release| Excluded(Key::new(release)));
        self.releases
            .keys_range((lower, Unbounded))
            .take(PAGE_SIZE)
            .map(|key| self.status(&key.release()))
            .collect()
    }
    pub fn info(&self) -> Info {
        Info {
            quota: self.settings.get().quota.clone(),
            reserved_bytes: self.settings.get().reserved_bytes,
            releases: self.releases.len(),
            artifacts: self.artifacts.len(),
            chunks: self.chunks.len(),
            maintenance_candidate: cfg!(feature = "maintenance-candidate"),
        }
    }
    pub fn configure(&mut self, quota: &Quota) -> Result<Info, Error> {
        self.admission_epoch()?;
        if !quota.valid() {
            return Err(Error::Invalid);
        }
        if quota.reserved_bytes < self.settings.get().reserved_bytes
            || quota.releases < self.releases.len()
        {
            return Err(Error::Capacity);
        }
        let mut settings = self.settings.get().clone();
        settings.quota = quota.clone();
        self.settings.set(settings);
        Ok(self.info())
    }

    fn pin_artifact(&self, caller: &[u8], pin: &Pin) -> Result<Artifact, Error> {
        if caller.is_empty() || caller.len() > 29 || caller == [4] || pin.operation == [0; 32] {
            return Err(Error::Invalid);
        }
        let status = self.status(&pin.manifest.release)?;
        if status.manifest != pin.manifest {
            return Err(Error::Conflict);
        }
        if !status.published || !status.missing.is_empty() {
            return Err(Error::Incomplete);
        }
        Ok(self
            .artifacts
            .get(&pin.manifest.hash)
            .expect("published artifact"))
    }

    pub fn pin(&mut self, caller: &[u8], pin: &Pin) -> Result<PinReceipt, Error> {
        self.admission_epoch()?;
        let mut artifact = self.pin_artifact(caller, pin)?;
        if let Some(old) = artifact
            .pins
            .iter()
            .find(|old| old.owner == caller && old.operation == pin.operation)
        {
            if old.release != pin.manifest.release {
                return Err(Error::Conflict);
            }
        } else {
            if artifact.pins.len() >= MAX_PINS {
                return Err(Error::Capacity);
            }
            artifact.pins.push(RetainedPin {
                owner: caller.to_vec(),
                operation: pin.operation,
                release: pin.manifest.release.clone(),
            });
            self.artifacts.insert(pin.manifest.hash, artifact);
        }
        Ok(PinReceipt {
            owner: caller.to_vec(),
            pin: pin.clone(),
        })
    }

    pub fn observe_pin(&self, caller: &[u8], pin: &Pin) -> Result<PinReceipt, Error> {
        let artifact = self.pin_artifact(caller, pin)?;
        let retained = artifact
            .pins
            .iter()
            .find(|old| old.owner == caller && old.operation == pin.operation)
            .ok_or(Error::NotFound)?;
        if retained.release != pin.manifest.release {
            return Err(Error::Conflict);
        }
        Ok(PinReceipt {
            owner: caller.to_vec(),
            pin: pin.clone(),
        })
    }
}
