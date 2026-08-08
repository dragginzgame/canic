pub mod error {
    pub use crate::error::{InternalError, InternalErrorClass, InternalErrorOrigin};
}

pub mod config {
    pub mod schema {
        pub use crate::config::schema::ComponentChildKind;
    }

    pub use crate::config::{
        ComponentDeploymentConfiguration, ComponentDeploymentPurpose, ComponentGroupDeploymentSpec,
        ComponentGroupPlacementPolicy, ComponentProvisioningGrant, ComponentTopology, ConfigModel,
        FlattenedComponentGroupDeploymentMember, FleetServiceTopology,
    };
}

pub mod format {
    pub use crate::shared_support::format::byte_size;
}

pub mod model {
    pub mod replay {
        pub use crate::model::replay::{CommandKind, ReplayCostGuardSettlement};
    }
}

pub mod policy {
    pub mod component_allocation {
        pub use crate::domain::policy::pure::component_allocation::{
            ComponentAllocationPolicyError, PeerComponentProvisioningInput,
            PeerComponentProvisioningReadiness, TopLevelComponentAllocationDecision,
            TopLevelComponentAllocationInput, authorize_peer_component_provisioning,
            reserve_top_level_component,
        };
    }

    pub mod component_child_allocation {
        pub use crate::domain::policy::pure::component_child_allocation::{
            ComponentChildAllocationDecision, ComponentChildAllocationInput,
            ComponentChildAllocationPolicyError, ComponentChildAllocationReadiness,
            ComponentRegistryVersionEvidence, reserve_component_child,
        };
    }
}

pub mod ops {
    pub mod config {
        pub use crate::ops::config::ConfigOps;
    }

    pub mod component_runtime {
        pub use crate::ops::component_runtime::ComponentRuntimeOps;
    }

    pub mod component_provisioning_plan {
        pub use crate::ops::component_provisioning_plan::{
            ComponentProvisioningPlacementAuthority, ComponentProvisioningPlanOps,
            ComponentProvisioningScaleOutAuthority, MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
            MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES,
            MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS,
            MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
            MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS,
            MAX_FLEET_SUBNET_ROOT_COMPONENT_ACTIVATION_PAYLOAD_BYTES,
            MAX_FLEET_SUBNET_ROOT_COMPONENT_PUBLICATION_PAYLOAD_BYTES,
            MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES,
            MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES,
            RootComponentProvisioningBatchValidation,
        };
    }

    pub mod component_provisioning_receipt {
        pub use crate::ops::component_provisioning_receipt::{
            RootComponentProvisioningAcceptanceReceiptAuthority,
            RootComponentProvisioningProvisionedReceiptAuthority,
            RootComponentProvisioningPublishedReceiptAuthority,
            RootComponentProvisioningReceiptOps,
            RootComponentProvisioningRuntimesActiveReceiptAuthority,
        };
    }

    pub mod cost_guard {
        pub use crate::ops::cost_guard::{
            CostGuardPermit, CostGuardRequest, CostGuardReserveError, CostGuardReservePublicKind,
        };
    }

    pub mod fleet_registry {
        pub use crate::ops::fleet_registry::{
            FleetRegistryOps, MAX_FLEET_REGISTRY_CANONICAL_BYTES,
        };
    }

    pub mod fleet_service_binding {
        pub use crate::ops::fleet_service_binding::FleetServiceBindingOps;
    }

    pub mod ic {
        pub use crate::ops::ic::IcOps;

        pub mod call {
            pub use crate::ops::ic::call::CallOps;
        }

        pub mod cycles_ledger {
            pub use crate::{
                infra::ic::cycles_ledger::{
                    CyclesLedgerCreateCanisterError, CyclesLedgerCreateCanisterSuccess,
                },
                ops::ic::cycles_ledger::CyclesLedgerOps,
            };
        }

        pub mod mgmt {
            pub use crate::ops::ic::mgmt::{
                CanisterSettings, CanisterStatus, CanisterStatusObservation, CanisterStatusType,
                MgmtOps, UpdateSettingsArgs,
            };
        }

        pub mod nns {
            pub use crate::ops::ic::nns::registry::NnsRegistryOps;
        }

        pub mod build_network {
            pub use crate::ops::ic::build_network::BuildNetworkOps;
        }
    }

    pub mod runtime {
        pub mod bootstrap {
            pub use crate::ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps};
        }

        pub mod env {
            pub use crate::ops::runtime::env::EnvOps;
        }

        pub mod ready {
            pub use crate::ops::runtime::ready::ReadyOps;
        }

        pub mod install_source {
            pub use crate::ops::runtime::install_source::ApprovedModuleSource;

            /// Resolve one Store-backed chunk source for control-plane installation.
            pub async fn resolve_approved_module_source(
                role: &crate::ids::CanisterRole,
            ) -> Result<ApprovedModuleSource, crate::error::InternalError> {
                crate::ops::runtime::install_source::ModuleSourceRuntimeApi::approved_module_source(
                    role,
                )
                .await
            }
        }

        pub mod init_payload {
            pub use crate::ops::runtime::init_payload::wasm_store_init_args;
        }
    }
}

pub mod view {
    pub mod fleet_activation {
        pub use crate::view::fleet_activation::{
            FleetActivationTransition, FleetActivationWasmStoreView,
        };
    }
}

pub mod workflow {
    pub mod cost_guard {
        pub use crate::workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error};
    }

    pub mod runtime {
        pub mod fleet_activation {
            pub use crate::workflow::runtime::fleet_activation::FleetActivationWorkflow;
        }

        pub mod install {
            pub use crate::workflow::runtime::install::ModuleInstallWorkflow;
        }
    }

    pub mod ic {
        pub use crate::workflow::ic::IcWorkflow;
    }

    pub mod rpc {
        pub use crate::workflow::rpc::{
            RootCapabilityAuthority, RootCapabilityCallerAuthority,
            RootCapabilityLifecycleExecutor, RootCapabilityMemberAuthority,
            RootCapabilityParentAuthority, RootComponentChildProvisionRequest,
            RootComponentChildRecycleOutcome, RootComponentChildRecycleRequest,
        };
    }

    pub mod topology {
        pub mod guard {
            pub use crate::workflow::topology::guard::TopologyGuard;
        }
    }

    pub mod state {
        /// Apply one root Fleet-state command to an exact caller-supplied child inventory.
        pub async fn execute_fleet_command_to(
            cmd: crate::dto::state::FleetCommand,
            root_children: &[crate::cdk::types::Principal],
        ) -> Result<crate::dto::state::FleetCommandResponse, crate::error::InternalError> {
            crate::workflow::state::FleetStateWorkflow::execute_command_to(cmd, root_children).await
        }
    }
}
