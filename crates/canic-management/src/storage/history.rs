//! Store completed upgrades independently of the mutable child record.
use super::{Artifact, Journal, Metadata, Record};
use canic_management_protocol::{application, stored};

pub(crate) trait Paged: Record {
    fn len(&self) -> usize;
    fn head(&self) -> Self;
    fn entry(&self, index: usize) -> Result<Vec<u8>, &'static str>;
    fn append(&mut self, bytes: &[u8]) -> Result<(), &'static str>;
}

fn encode<T: serde::Serialize>(
    value: &T,
    limits: ic_wire::Limits,
) -> Result<Vec<u8>, &'static str> {
    ic_wire::encode(value, ic_wire::Encoding::Payload, limits).map_err(|_| "upgrade record budget")
}
fn decode<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    limits: ic_wire::Limits,
) -> Result<T, &'static str> {
    ic_wire::decode(bytes, ic_wire::Encoding::Payload, limits).map_err(|_| "invalid upgrade record")
}

impl Paged for Journal {
    fn len(&self) -> usize {
        self.2.len()
    }
    fn head(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), vec![])
    }
    fn entry(&self, index: usize) -> Result<Vec<u8>, &'static str> {
        encode(&self.2[index], Self::LIMITS)
    }
    fn append(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        self.2.push(decode(bytes, Self::LIMITS)?);
        Ok(())
    }
}

impl Paged for Metadata {
    fn len(&self) -> usize {
        self.history.len()
    }
    fn head(&self) -> Self {
        Self {
            maintenance: self.maintenance.clone(),
            bootstrap: self.bootstrap,
            history: vec![],
        }
    }
    fn entry(&self, index: usize) -> Result<Vec<u8>, &'static str> {
        encode(&self.history[index], Self::LIMITS)
    }
    fn append(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        self.history.push(decode(bytes, Self::LIMITS)?);
        Ok(())
    }
}

impl Paged for Artifact {
    fn len(&self) -> usize {
        self.sources().map_or(0, |s| s.upgrades.len())
    }

    fn head(&self) -> Self {
        let Some(s) = self.sources() else {
            return Self::Empty;
        };
        let sources = stored::Sources {
            creation: s.creation.clone(),
            upgrades: vec![],
        };
        match self {
            Self::Application(_, l) => Self::Application(
                sources,
                application::Lifecycle {
                    creation: l.creation.clone(),
                    upgrades: vec![],
                },
            ),
            _ => Self::Stored(sources),
        }
    }

    fn entry(&self, index: usize) -> Result<Vec<u8>, &'static str> {
        let s = self.sources().ok_or("missing upgrade source")?;
        let maintenance = match self {
            Self::Application(_, l) => {
                Some(l.upgrades.get(index).ok_or("missing upgrade lifecycle")?)
            }
            _ => None,
        };
        encode(&(&s.upgrades[index], maintenance), Self::LIMITS)
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let (delivery, maintenance): (stored::Delivery, Option<application::Maintenance>) =
            decode(bytes, Self::LIMITS)?;
        match (self, maintenance) {
            (Self::Stored(s), None) => s.upgrades.push(delivery),
            (Self::Application(s, l), Some(m)) if m.valid() => {
                s.upgrades.push(delivery);
                l.upgrades.push(m);
            }
            _ => return Err("upgrade source and lifecycle differ"),
        }
        Ok(())
    }
}
