//! Native publication frames; retained action and storage shapes remain canonical.
use crate::{
    dto::template::TemplateChunkSetPrepareInput,
    ids::{TemplateId, TemplateVersion},
};
use serde::{Deserialize, Serialize};
/// One template chunk, optionally admitting its exact manifest with chunk zero.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PublishRequest {
    pub preparation: Option<TemplateChunkSetPrepareInput>,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}
/// Borrowed serialization of the same request without copying retained bytes.
#[derive(Serialize)]
pub struct PublishRequestRef<'a> {
    pub preparation: Option<&'a TemplateChunkSetPrepareInput>,
    pub template_id: &'a TemplateId,
    pub version: &'a TemplateVersion,
    pub chunk_index: u32,
    #[serde(with = "serde_bytes")]
    pub bytes: &'a [u8],
}
