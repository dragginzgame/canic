use crate::{
    Error,
    cdk::call::Call,
    env::sns::{SnsRole, SnsType},
    interface::prelude::*,
    spec::sns::{ListNeurons, ListNeuronsResponse, Neuron, NeuronId},
};

/// Convenience: page through all results and return every neuron for `owner`.
pub async fn list_sns_neurons_for_principal(
    sns: SnsType,
    owner: Principal,
) -> Result<Vec<Neuron>, Error> {
    const PAGE_SIZE: u32 = 100;

    let mut out: Vec<Neuron> = Vec::new();
    let mut last: Option<NeuronId> = None;

    loop {
        let page = list_sns_neurons_for_principal_page(sns, owner, PAGE_SIZE, last.clone()).await?;
        if page.neurons.is_empty() {
            break;
        }

        // advance cursor
        last = page.neurons.last().and_then(|n| n.id.clone());

        out.extend(page.neurons.into_iter());

        if !out.len().is_multiple_of(PAGE_SIZE as usize) {
            break; // fewer than requested on this page ⇒ done
        }
    }

    Ok(out)
}

/// Fetch a single page of neurons owned by `owner` from a given SNS.
pub async fn list_sns_neurons_for_principal_page(
    sns: SnsType,
    owner: Principal,
    page_size: u32,
    start_at: Option<NeuronId>,
) -> Result<ListNeuronsResponse, Error> {
    let gov_canister = sns.principal(SnsRole::Governance)?;

    let list_neurons_arg = ListNeurons {
        of_principal: Some(owner),
        start_page_at: start_at,
        limit: page_size,
    };

    let res = Call::unbounded_wait(gov_canister, "list_neurons")
        .with_arg(list_neurons_arg)
        .await?
        .candid::<ListNeuronsResponse>()?;

    Ok(res)
}
