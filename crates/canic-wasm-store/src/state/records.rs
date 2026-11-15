//! Versioned metadata and independently bounded chunk payloads.
use canic_wasm_store_protocol::{CHUNK_BYTES, LIMITS, MAX_CHUNKS, MAX_PINS, Quota, Release};
use ic_storage::adapter::{Bound, CodecError, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Key(pub String);
impl Key {
    pub fn new(release: &Release) -> Self {
        Self(format!("{}\0{}", release.name, release.version))
    }
    pub fn release(&self) -> Release {
        let (name, version) = self.0.split_once('\0').expect("validated release key");
        Release {
            name: name.into(),
            version: version.into(),
        }
    }
}
impl Storable for Key {
    const BOUND: Bound = Bound::Bounded {
        max_size: 129,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(self.0.as_bytes()))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let key = String::from_utf8(bytes.into_owned())
            .map_err(|_| CodecError::Invalid("release key"))?;
        let (name, version) = key
            .split_once('\0')
            .ok_or(CodecError::Invalid("release key"))?;
        if !(Release {
            name: name.into(),
            version: version.into(),
        })
        .valid()
        {
            return Err(CodecError::Invalid("release labels"));
        }
        Ok(Self(key))
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Artifact {
    pub bytes: u64,
    pub chunks: Vec<[u8; 32]>,
    pub published: bool,
    pub pins: Vec<RetainedPin>,
}
impl Artifact {
    fn valid(&self) -> bool {
        self.bytes >= 8
            && self.bytes <= (CHUNK_BYTES * MAX_CHUNKS) as u64
            && self.chunks.len() == (self.bytes as usize).div_ceil(CHUNK_BYTES)
            && self.pins.len() <= MAX_PINS
            && self.pins.iter().all(RetainedPin::valid)
            && (self.pins.is_empty() || self.published)
            && self.pins.iter().enumerate().all(|(i, pin)| {
                !self.pins[..i]
                    .iter()
                    .any(|prior| prior.owner == pin.owner && prior.operation == pin.operation)
            })
    }
    pub fn chunk_bytes(&self, index: usize) -> usize {
        (self.bytes as usize - index * CHUNK_BYTES).min(CHUNK_BYTES)
    }
}
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Settings {
    pub quota: Quota,
    pub reserved_bytes: u64,
    pub admission: Option<canic_wasm_store_protocol::Admission>,
}
impl Settings {
    fn valid(&self) -> bool {
        self.quota.valid()
            && self.reserved_bytes <= self.quota.reserved_bytes
            && self.admission.as_ref().is_none_or(|gate| {
                gate.attempt.sequence > 0
                    && gate.attempt.operation != [0; 32]
                    && !gate.owner.is_empty()
                    && gate.owner.len() <= 29
                    && gate.owner != [4]
            })
    }
}

// Pins are small obligations inside the artifact metadata, under the original lease.
// This avoids changing the four-allocation directory created by the first Store.
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct RetainedPin {
    pub owner: Vec<u8>,
    pub operation: [u8; 32],
    pub release: Release,
}
impl RetainedPin {
    fn valid(&self) -> bool {
        !self.owner.is_empty()
            && self.owner.len() <= 29
            && self.owner != [4]
            && self.operation != [0; 32]
            && self.release.valid()
    }
}
const RECORD_LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 1_048_576,
    nodes: 65_536,
    ..LIMITS
};
const METADATA_BOUND: Bound = Bound::Bounded {
    max_size: 16_384,
    is_fixed_size: false,
};
fn encode<T: Serialize>(value: &T, tag: &[u8]) -> Result<Cow<'static, [u8]>, CodecError> {
    let mut bytes = tag.to_vec();
    bytes.extend(
        ic_wire::native::encode(value, RECORD_LIMITS)
            .map_err(|_| CodecError::Invalid("store record"))?,
    );
    Ok(Cow::Owned(bytes))
}
impl Storable for Artifact {
    const BOUND: Bound = METADATA_BOUND;
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        if !self.valid() {
            return Err(CodecError::Invalid("artifact invariant"));
        }
        encode(self, b"CWA2")
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let value: Self = if let Some(payload) = bytes.strip_prefix(b"CWA2") {
            ic_wire::native::decode(payload, RECORD_LIMITS)
                .map_err(|_| CodecError::Invalid("artifact record"))?
        } else if let Some(payload) = bytes.strip_prefix(b"CWA1") {
            #[derive(Deserialize)]
            struct Legacy {
                bytes: u64,
                chunks: Vec<[u8; 32]>,
                published: bool,
            }
            let old: Legacy = ic_wire::native::decode(payload, LIMITS)
                .map_err(|_| CodecError::Invalid("legacy artifact"))?;
            Self {
                bytes: old.bytes,
                chunks: old.chunks,
                published: old.published,
                pins: vec![],
            }
        } else {
            return Err(CodecError::Invalid("artifact schema"));
        };
        if !value.valid() {
            return Err(CodecError::Invalid("artifact invariant"));
        }
        Ok(value)
    }
}
impl Storable for Settings {
    const BOUND: Bound = METADATA_BOUND;
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        if !self.valid() {
            return Err(CodecError::Invalid("settings invariant"));
        }
        encode(self, b"CWS2")
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let value: Self = if let Some(payload) = bytes.strip_prefix(b"CWS2") {
            ic_wire::native::decode(payload, LIMITS)
                .map_err(|_| CodecError::Invalid("settings record"))?
        } else if let Some(payload) = bytes.strip_prefix(b"CWS1") {
            #[derive(Deserialize)]
            struct Previous {
                quota: Quota,
                reserved_bytes: u64,
            }
            let old: Previous = ic_wire::native::decode(payload, LIMITS)
                .map_err(|_| CodecError::Invalid("previous settings record"))?;
            Self {
                quota: old.quota,
                reserved_bytes: old.reserved_bytes,
                admission: None,
            }
        } else {
            return Err(CodecError::Invalid("settings schema"));
        };
        if !value.valid() {
            return Err(CodecError::Invalid("settings invariant"));
        }
        Ok(value)
    }
}

pub(super) struct Payload(pub Vec<u8>);
impl Storable for Payload {
    const BOUND: Bound = Bound::Bounded {
        max_size: CHUNK_BYTES as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(&self.0))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        if bytes.is_empty() || bytes.len() > CHUNK_BYTES {
            return Err(CodecError::Invalid("chunk size"));
        }
        Ok(Self(bytes.into_owned()))
    }
}
