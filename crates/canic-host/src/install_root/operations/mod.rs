mod activation;
mod artifact;
mod creation;
mod installation;
mod manifest;
mod phase;
mod preparation;
mod registry;

pub(super) use crate::canister_protocol::{
    call_no_arg, call_with_arg, query_no_arg, query_with_arg,
};
pub(super) use activation::{module_hash_text, require_expected_module_hash};
pub(super) use artifact::{InstallArtifact, resolve_install_artifact};
pub(super) use creation::{CreationEffectRequest, execute_or_observe_creation};
pub(super) use installation::{InstallEffectRequest, execute_or_observe_install};
pub(super) use manifest::EmitRootManifestOperation;
pub(super) use phase::InstallPhaseLabel;
pub(super) use preparation::BuildInstallTargetsOperation;
pub(super) use registry::{LiveRegistryEvidence, query_live_registry};

#[derive(Clone, Copy)]
pub(in crate::install_root) enum EffectAction {
    Execute,
    ObserveOnly,
}

impl EffectAction {
    pub(in crate::install_root) const fn from_advanced(advanced: bool) -> Self {
        if advanced {
            Self::Execute
        } else {
            Self::ObserveOnly
        }
    }
}
