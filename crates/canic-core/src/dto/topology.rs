use crate::{dto::prelude::*, ids::FleetBinding};
use serde::{
    Deserializer,
    de::{self, SeqAccess, Visitor},
};
use std::fmt;

/// Maximum encoded bytes accepted by the state-cascade endpoint.
pub const DIRECTORY_CASCADE_MAX_BYTES: usize = 16_384;
/// Maximum entries accepted in one Directory cascade or page.
pub const DIRECTORY_ENTRY_MAX_COUNT: usize = 1_000;

//
// FleetDirectoryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetDirectoryInput {
    pub provenance: DirectoryProvenance,
    #[serde(deserialize_with = "deserialize_directory_entries")]
    pub entries: Vec<DirectoryEntryInput>,
}

//
// DirectoryProvenance
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DirectoryProvenance {
    pub fleet: FleetBinding,
    pub source_root: Principal,
}

//
// DirectoryEntryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DirectoryEntryInput {
    pub role: CanisterRole,
    pub pid: Principal,
}

fn deserialize_directory_entries<'de, D>(
    deserializer: D,
) -> Result<Vec<DirectoryEntryInput>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(BoundedDirectoryVisitor::<DirectoryEntryInput>::new())
}

struct BoundedDirectoryVisitor<T> {
    marker: std::marker::PhantomData<T>,
}

impl<T> BoundedDirectoryVisitor<T> {
    const fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<'de, T> Visitor<'de> for BoundedDirectoryVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "at most {DIRECTORY_ENTRY_MAX_COUNT} Directory entries"
        )
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let expected = sequence.size_hint().unwrap_or_default();
        if expected > DIRECTORY_ENTRY_MAX_COUNT {
            return Err(de::Error::invalid_length(expected, &self));
        }

        let mut entries = Vec::with_capacity(expected);
        while let Some(entry) = sequence.next_element()? {
            if entries.len() == DIRECTORY_ENTRY_MAX_COUNT {
                return Err(de::Error::invalid_length(entries.len() + 1, &self));
            }
            entries.push(entry);
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AppId, CanonicalNetworkId, FleetId, FleetKey};

    fn provenance() -> DirectoryProvenance {
        DirectoryProvenance {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("app"),
            },
            source_root: Principal::from_slice(&[2; 29]),
        }
    }

    fn entry(index: usize) -> DirectoryEntryInput {
        let principal_byte = u8::try_from(index % 255).expect("modulo result must fit in u8");
        DirectoryEntryInput {
            role: CanisterRole::from(format!("role_{index}")),
            pid: Principal::from_slice(&[principal_byte; 29]),
        }
    }

    #[test]
    fn directory_input_accepts_the_exact_entry_limit() {
        let input = FleetDirectoryInput {
            provenance: provenance(),
            entries: (0..DIRECTORY_ENTRY_MAX_COUNT).map(entry).collect(),
        };
        let bytes = candid::encode_one(&input).expect("encode bounded Directory");
        let decoded: FleetDirectoryInput =
            candid::decode_one(&bytes).expect("decode bounded Directory");

        assert_eq!(decoded, input);
    }

    #[test]
    fn directory_input_rejects_before_admitting_entry_overflow() {
        let input = FleetDirectoryInput {
            provenance: provenance(),
            entries: (0..=DIRECTORY_ENTRY_MAX_COUNT).map(entry).collect(),
        };
        let bytes = candid::encode_one(&input).expect("encode oversized Directory");

        assert!(candid::decode_one::<FleetDirectoryInput>(&bytes).is_err());
    }
}
