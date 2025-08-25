use crate::spec::prelude::*;

#[derive(CandidType, Deserialize)]
pub struct ListNeurons {
    pub of_principal: Option<Principal>,
    pub limit: u32,
    pub start_page_at: Option<NeuronId>,
}

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct NeuronId {
    pub id: ByteBuf,
}

#[derive(CandidType, Deserialize)]
pub struct ListNeuronsResponse {
    pub neurons: Vec<Neuron>,
}

#[derive(CandidType, Deserialize)]
pub struct Neuron {
    pub id: Option<NeuronId>,
    pub staked_maturity_e8s_equivalent: Option<u64>,
    pub maturity_e8s_equivalent: u64,
    pub cached_neuron_stake_e8s: u64,
    pub created_timestamp_seconds: u64,
}
