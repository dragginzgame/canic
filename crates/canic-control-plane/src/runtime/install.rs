use async_trait::async_trait;
use canic_core::{
    api::runtime::install::{ApprovedModuleSource, ModuleSourceResolver, ModuleSourceRuntimeApi},
    control_plane_support::error::InternalError,
    ids::CanisterRole,
};

///
/// TemplateModuleSourceResolver
///

pub struct TemplateModuleSourceResolver;

#[async_trait]
impl ModuleSourceResolver for TemplateModuleSourceResolver {
    /// Resolve one approved role source through the current template-backed control-plane path.
    async fn approved_module_source(
        &self,
        role: &CanisterRole,
    ) -> Result<ApprovedModuleSource, InternalError> {
        crate::workflow::runtime::template::resolved_approved_module_source_for_role(role).await
    }
}

static TEMPLATE_MODULE_SOURCE_RESOLVER: TemplateModuleSourceResolver = TemplateModuleSourceResolver;

/// Register the template-backed resolver used by root-owned install and upgrade workflows.
pub fn register_template_module_source_resolver() {
    ModuleSourceRuntimeApi::register_module_source_resolver(&TEMPLATE_MODULE_SOURCE_RESOLVER);
}
