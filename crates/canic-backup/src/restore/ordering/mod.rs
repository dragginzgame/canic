//! Translate Canic's serial restore plan into Backup-owned explicit dependencies.
//!
//! Backup validates and orders the graph; this adapter grants no effect authority.

use ic_backup::model::effect_graph::{
    EffectGraphError, EffectGraphRecord, EffectNodeRecord, EffectNodeRequest,
};

/// Preserve the plan's serial order while delegating graph bounds and identity validation.
pub(super) fn serial_effect_graph(
    sequences: impl Iterator<Item = usize>,
) -> Result<Option<EffectGraphRecord>, EffectGraphError> {
    let mut sequences: Vec<_> = sequences.map(|sequence| sequence as u64).collect();
    if sequences.is_empty() {
        return Ok(None);
    }
    sequences.sort_unstable();
    let nodes = sequences
        .iter()
        .enumerate()
        .map(|(index, &sequence)| {
            EffectNodeRecord::new(EffectNodeRequest {
                operation_sequence: sequence,
                depends_on: index
                    .checked_sub(1)
                    .map(|previous| sequences[previous])
                    .into_iter()
                    .collect(),
            })
        })
        .collect::<Result<_, _>>()?;
    EffectGraphRecord::new(nodes).map(Some)
}
