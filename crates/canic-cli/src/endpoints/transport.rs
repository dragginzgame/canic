use crate::{
    cli::defaults::local_environment,
    endpoints::{
        EndpointsCommandError, EndpointsOptions,
        model::{EndpointReport, EndpointSourceKind},
    },
};
use canic_host::{
    icp_config::resolve_current_canic_icp_root, protocol_binding::load_installed_wire_schema,
    release_set::load_built_wire_schema,
};
use std::path::Path;

pub(super) fn endpoint_report(
    options: &EndpointsOptions,
) -> Result<EndpointReport, EndpointsCommandError> {
    endpoint_report_at(&resolve_current_canic_icp_root()?, options)
}

pub(super) fn endpoint_report_at(
    root: &Path,
    options: &EndpointsOptions,
) -> Result<EndpointReport, EndpointsCommandError> {
    let (built, source_kind) = if let Some(id) = options.release_build {
        (
            load_built_wire_schema(root, id, &options.canister)?,
            EndpointSourceKind::Built,
        )
    } else {
        let environment = options
            .environment
            .clone()
            .unwrap_or_else(local_environment);
        (
            load_installed_wire_schema(root, &environment, &options.fleet, &options.canister)?,
            EndpointSourceKind::Installed,
        )
    };
    let schema = crate::endpoints::model::read_schema(&built.schema)?;
    Ok(EndpointReport {
        source: built.path.display().to_string(),
        source_kind,
        release_build_id: Some(built.release_build_id),
        endpoints: schema.methods,
        types: schema.types,
    })
}
