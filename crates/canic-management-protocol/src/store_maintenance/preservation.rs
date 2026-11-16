//! Versioned scan evidence. Historical Option tags and JSON fields remain exact.
use ic_wasm_store_protocol::{Release, preservation::Catalog};
use serde::{Deserialize, Serialize};

/// Bounded catalog commitment, without retaining Store's catalog in Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preservation {
    /// Exact counters and quota observed under closed admission.
    pub catalog: Catalog,
    /// Last committed release in the selected scan order.
    pub after: Option<Release>,
    /// Number of records included, including incomplete uploads and removed labels.
    pub observed: u64,
    /// Ordered metadata commitment, excluding payload bytes and management cache.
    pub digest: [u8; 32],
    /// Explicit end page and matching total count have been retained.
    pub complete: bool,
    /// Last entry was removed; false before the first page and for historical scans.
    pub after_removed: bool,
}

/// The scan contract survives before its first page and across executable changes.
/// Historical encodings are byte-identical to the former `Option<Preservation>`.
/// New evidence uses Wire discriminant 2, which historical readers refuse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scan {
    /// Historical method 113, name/version ordering and metadata-v1 digest domain.
    Legacy(Option<Preservation>),
    /// Method 117, explicit active/removed entries and metadata-v2 digest domain.
    Entries(Option<Preservation>),
}
impl Scan {
    /// Current commitment, absent until the first page commits.
    #[must_use]
    pub fn as_ref(&self) -> Option<&Preservation> {
        match self {
            Self::Legacy(value) | Self::Entries(value) => value.as_ref(),
        }
    }

    /// Whether no page has committed yet; this does not erase the selected contract.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }

    /// Start an independent verification with the same contract and no page evidence.
    #[must_use]
    pub fn empty(&self) -> Self {
        match self {
            Self::Legacy(_) => Self::Legacy(None),
            Self::Entries(_) => Self::Entries(None),
        }
    }
}

// This shape is a retained codec, not another runtime scan implementation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Legacy {
    catalog: Catalog,
    after: Option<Release>,
    observed: u64,
    digest: [u8; 32],
    complete: bool,
}
impl From<Legacy> for Preservation {
    fn from(value: Legacy) -> Self {
        Self {
            catalog: value.catalog,
            after: value.after,
            observed: value.observed,
            digest: value.digest,
            complete: value.complete,
            after_removed: false,
        }
    }
}
impl TryFrom<&Preservation> for Legacy {
    type Error = &'static str;
    fn try_from(value: &Preservation) -> Result<Self, Self::Error> {
        if value.after_removed {
            return Err("historical scan cannot encode a removed cursor");
        }
        Ok(Self {
            catalog: value.catalog.clone(),
            after: value.after.clone(),
            observed: value.observed,
            digest: value.digest,
            complete: value.complete,
        })
    }
}

#[derive(Deserialize, Serialize)]
enum Encoded<T> {
    Empty,
    Legacy(Legacy),
    Entries(T),
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entries<T> {
    entries: T,
}
impl Serialize for Scan {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Legacy(value) => {
                let value = value
                    .as_ref()
                    .map(Legacy::try_from)
                    .transpose()
                    .map_err(serde::ser::Error::custom)?;
                if serializer.is_human_readable() {
                    value.serialize(serializer)
                } else {
                    let encoded: Encoded<&Option<Preservation>> = match value {
                        None => Encoded::Empty,
                        Some(value) => Encoded::Legacy(value),
                    };
                    encoded.serialize(serializer)
                }
            }
            Self::Entries(value) => {
                if serializer.is_human_readable() {
                    Entries { entries: value }.serialize(serializer)
                } else {
                    Encoded::Entries(value).serialize(serializer)
                }
            }
        }
    }
}
impl<'de> Deserialize<'de> for Scan {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Human {
                Entries(Entries<Option<Preservation>>),
                Legacy(Option<Legacy>),
            }
            Ok(match Human::deserialize(deserializer)? {
                Human::Entries(value) => Self::Entries(value.entries),
                Human::Legacy(value) => Self::Legacy(value.map(Into::into)),
            })
        } else {
            Ok(
                match Encoded::<Option<Preservation>>::deserialize(deserializer)? {
                    Encoded::Empty => Self::Legacy(None),
                    Encoded::Legacy(value) => Self::Legacy(Some(value.into())),
                    Encoded::Entries(value) => Self::Entries(value),
                },
            )
        }
    }
}
