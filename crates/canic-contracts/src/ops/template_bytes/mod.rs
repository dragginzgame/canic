//! Lossless publication conversions keep retained action encoding independent of transport.
use crate::dto::{
    template::TemplateChunkInput,
    template_bytes::{PublishRequest, PublishRequestRef},
};
impl From<TemplateChunkInput> for PublishRequest {
    fn from(value: TemplateChunkInput) -> Self {
        Self {
            preparation: value.preparation,
            template_id: value.template_id,
            version: value.version,
            chunk_index: value.chunk_index,
            bytes: value.bytes,
        }
    }
}
impl From<PublishRequest> for TemplateChunkInput {
    fn from(value: PublishRequest) -> Self {
        Self {
            preparation: value.preparation,
            template_id: value.template_id,
            version: value.version,
            chunk_index: value.chunk_index,
            bytes: value.bytes,
        }
    }
}
impl<'a> From<&'a TemplateChunkInput> for PublishRequestRef<'a> {
    fn from(value: &'a TemplateChunkInput) -> Self {
        Self {
            preparation: value.preparation.as_ref(),
            template_id: &value.template_id,
            version: &value.version,
            chunk_index: value.chunk_index,
            bytes: &value.bytes,
        }
    }
}
