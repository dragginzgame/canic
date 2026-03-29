use crate::{
    InternalError,
    cdk::types::WasmModule,
    ids::TemplateId,
    ops::{prelude::*, runtime::RuntimeOpsError},
};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use thiserror::Error as ThisError;

///
/// Runtime embedded template payload registry
///
/// In-memory registry mapping template identifiers to embedded payload bytes
/// for inline template installs used by test and bootstrap-only flows.
///

static TEMPLATE_PAYLOAD_REGISTRY: LazyLock<Mutex<HashMap<TemplateId, WasmModule>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

///
/// EmbeddedTemplatePayloadOpsError
///

#[derive(Debug, ThisError)]
pub enum EmbeddedTemplatePayloadOpsError {
    #[error("template payload '{0}' not found")]
    TemplatePayloadNotFound(TemplateId),
}

impl From<EmbeddedTemplatePayloadOpsError> for InternalError {
    fn from(err: EmbeddedTemplatePayloadOpsError) -> Self {
        RuntimeOpsError::EmbeddedTemplatePayloadOps(err).into()
    }
}

///
/// EmbeddedTemplatePayloadOps
///

pub struct EmbeddedTemplatePayloadOps;

impl EmbeddedTemplatePayloadOps {
    /// Fetch a template payload for the given template identifier, if present.
    #[must_use]
    pub fn get(template_id: &TemplateId) -> Option<WasmModule> {
        TEMPLATE_PAYLOAD_REGISTRY
            .lock()
            .expect("template payload registry poisoned")
            .get(template_id)
            .cloned()
    }

    /// Fetch a template payload or return an error if missing.
    pub fn try_get(template_id: &TemplateId) -> Result<WasmModule, InternalError> {
        Self::get(template_id).ok_or_else(|| {
            EmbeddedTemplatePayloadOpsError::TemplatePayloadNotFound(template_id.clone()).into()
        })
    }

    /// Import one embedded template payload at startup.
    pub fn import(template_id: TemplateId, bytes: &'static [u8]) {
        let wasm = WasmModule::new(bytes);

        TEMPLATE_PAYLOAD_REGISTRY
            .lock()
            .expect("template payload registry poisoned")
            .insert(template_id.clone(), wasm);

        log!(
            Topic::Wasm,
            Info,
            "tpl.payload.import {} ({} bytes)",
            template_id,
            bytes.len()
        );
    }
}
