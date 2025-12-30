//!
//! SNS governance candid bindings kept in a single spot with uniform naming.
//!

use crate::spec::prelude::*;

///
/// ListNeurons
/// Request payload for SNS `list_neurons` with optional principal filtering.
///

#[derive(CandidType, Deserialize)]
pub struct ListNeurons {
    pub of_principal: Option<Principal>,
    pub limit: u32,
    pub start_page_at: Option<NeuronId>,
}

///
/// NeuronId
/// Wrapper around the raw bytes identifying an SNS neuron.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct NeuronId {
    pub id: ByteBuf,
}

///
/// ListNeuronsResponse
/// Response payload containing the returned SNS neurons.
///

#[derive(CandidType, Deserialize)]
pub struct ListNeuronsResponse {
    pub neurons: Vec<Neuron>,
}

///
/// Neuron
/// Simplified view of an SNS neuron record used in ops modules.
///

#[derive(CandidType, Deserialize)]
pub struct Neuron {
    pub id: Option<NeuronId>,
    pub staked_maturity_e8s_equivalent: Option<u64>,
    pub maturity_e8s_equivalent: u64,
    pub cached_neuron_stake_e8s: u64,
    pub created_timestamp_seconds: u64,
}
