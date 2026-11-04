//! Root-owned retained registration intent; no private key enters this record.

use canic_contracts::dto::portable_enrollment::{
    PortableIssuerChallenge, PortableIssuerEnrollment, PreparePortableIssuer,
};
use canic_contracts::ids::FleetKey;
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Protected Root identity and Fleet; supplied by installation, never by a client.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnrollmentDomain {
    pub authority: Principal,
    pub fleet: FleetKey,
}

/// One current Root registry and its bounded public registration intents.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootPortableEnrollmentRecord {
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub domain: Option<EnrollmentDomain>,
    pub revision: u64,
    pub generation: u64,
    pub entries: Vec<PortableIssuerEnrollment>,
    pub retired_keys: Vec<[u8; 32]>,
    pub registration_sequence: u64,
    pub registrations: Vec<PortableIssuerRegistrationRecord>,
}

/// Exact public approval and challenge; current Root authority proves completion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortableIssuerRegistrationRecord {
    pub sequence: u64,
    pub request: PreparePortableIssuer,
    pub digest: [u8; 32],
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub challenge: Option<PortableIssuerChallenge>,
}

/// Async randomness completion may publish only into this exact retained intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableIssuerPreparation {
    pub domain: EnrollmentDomain,
    pub issuer: Principal,
    pub sequence: u64,
    pub digest: [u8; 32],
}

/// Workflow either returns retained work or obtains randomness for one exact ticket.
pub enum PortableIssuerPreparationState {
    Pending(PortableIssuerPreparation),
    Ready(PortableIssuerChallenge),
}
