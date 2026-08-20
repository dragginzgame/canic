//! Module: bootstrap::render
//!
//! Responsibility: render validated config models as generated Rust expressions.
//! Does not own: config validation, schema definitions, or runtime config install.
//! Boundary: host-side bootstrap tooling calls this before embedding generated source.

#[cfg(test)]
use crate::{cdk::candid::Principal, config::schema::IcpRefillPolicy};
use crate::{
    config::{
        ComponentDeploymentLabelKey, ComponentDeploymentLabelValue, FleetServiceMemberPurpose,
        schema::{
            AppConfig, AuthConfig, CanisterAuthConfig, ChainKeyRootProofConfig,
            ComponentChildConfig, ComponentChildKind, ComponentDeploymentMemberLimitConfig,
            ComponentDeploymentSpawnGrantLimitConfig, ComponentGroupComponentConfig,
            ComponentGroupDeploymentConfig, ComponentGroupIncludeConfig,
            ComponentGroupPlacementPolicyConfig, ComponentGroupSpecConfig, ComponentLimitsConfig,
            ComponentProvisioningGrantConfig, ComponentSpawnGrantConfig, ComponentSpecConfig,
            ConfigModel, CyclesFundingBudgetConfig, CyclesFundingPolicyConfig,
            DelegatedTokenConfig, DiagnosticsCanisterConfig, FleetInitMode,
            FleetServicePlacementPolicyConfig, FleetServiceTargetConfig, FleetServicesConfig,
            IndexConfig, IndexPool, LocalApplicationAuthorizationConfig, LogConfig,
            MetricsCanisterConfig, MetricsProfile, RoleAttestationConfig, RoleDeclaration,
            RoleDeclarationKind, ScalePool, ScalePoolPolicy, ScalingConfig, ServicesConfig,
            ShardPool, ShardPoolPolicy, ShardingConfig, Standards, StandardsCanisterConfig,
            TopupPolicy, Whitelist,
        },
    },
    ids::{
        AppId, BuildNetwork, CanisterRole, ComponentGroupDeploymentId, ComponentGroupMemberId,
        ComponentGroupMemberPath, ComponentGroupSpecId, ComponentSpecId, FleetServiceId,
    },
};
use proc_macro2::TokenStream;
use quote::quote;

/// config_model
///
/// Render the validated config model into a Rust expression string.
pub fn config_model(config: &ConfigModel) -> String {
    let mut source = render_config_model(config).to_string();
    source.push('\n');
    source
}

// Render the top-level configuration model into a portable Rust expression.
fn render_config_model(config: &ConfigModel) -> TokenStream {
    let standards = render_option(config.standards.as_ref(), render_standards);
    let log = render_log_config(&config.log);
    let auth = render_auth_config(&config.auth);
    let app = render_app_config(&config.app);
    let roles = render_btree_map(
        config.roles.iter(),
        render_canister_role,
        render_role_declaration,
    );
    let component_specs = render_btree_map(
        config.component_specs.iter(),
        render_component_spec_id,
        render_component_spec_config,
    );
    let component_groups = render_btree_map(
        config.component_groups.iter(),
        render_component_group_spec_id,
        render_component_group_spec_config,
    );
    let component_group_deployments = render_btree_map(
        config.component_group_deployments.iter(),
        render_component_group_deployment_id,
        render_component_group_deployment_config,
    );
    let services = render_services_config(&config.services);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ConfigModel {
            standards: #standards,
            log: #log,
            auth: #auth,
            app: #app,
            roles: #roles,
            component_specs: #component_specs,
            component_groups: #component_groups,
            component_group_deployments: #component_group_deployments,
            services: #services,
        }
    }
}

// Render an App role declaration.
fn render_role_declaration(declaration: &RoleDeclaration) -> TokenStream {
    let kind = render_role_declaration_kind(declaration.kind);
    let package = render_owned_string(&declaration.package);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::RoleDeclaration {
            kind: #kind,
            package: #package,
        }
    }
}

// Render role declaration kind.
fn render_role_declaration_kind(kind: RoleDeclarationKind) -> TokenStream {
    match kind {
        RoleDeclarationKind::Root => {
            quote!(::canic::__internal::core::bootstrap::compiled::RoleDeclarationKind::Root)
        }
        RoleDeclarationKind::Canister => {
            quote!(::canic::__internal::core::bootstrap::compiled::RoleDeclarationKind::Canister)
        }
    }
}

// Render a principal as a byte-based constructor to avoid runtime text parsing.
#[cfg(test)]
fn render_principal(principal: &Principal) -> TokenStream {
    let bytes = principal.as_slice().iter().copied();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::Principal::from_slice(&[#(#bytes),*])
    }
}

// Render a canister role using constants where possible and literals otherwise.
fn render_canister_role(role: &CanisterRole) -> TokenStream {
    match role.as_str() {
        "root" => quote!(::canic::__internal::core::bootstrap::compiled::CanisterRole::ROOT),
        "wasm_store" => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterRole::WASM_STORE)
        }
        value => quote!(::canic::__internal::core::bootstrap::compiled::CanisterRole::from(#value)),
    }
}

// Render one validated Component Spec identifier.
fn render_component_spec_id(component_spec: &ComponentSpecId) -> TokenStream {
    let value = component_spec.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentSpecId::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component Spec ID was validated at build time")
    }
}

// Render one validated Component Group Spec identifier.
fn render_component_group_spec_id(component_group: &ComponentGroupSpecId) -> TokenStream {
    let value = component_group.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupSpecId::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component Group Spec ID was validated at build time")
    }
}

// Render one validated Component Group deployment identifier.
fn render_component_group_deployment_id(deployment: &ComponentGroupDeploymentId) -> TokenStream {
    let value = deployment.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupDeploymentId::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component Group deployment ID was validated at build time")
    }
}

// Render one validated Component Group member identifier.
fn render_component_group_member_id(member: &ComponentGroupMemberId) -> TokenStream {
    let value = member.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupMemberId::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component Group member ID was validated at build time")
    }
}

// Render one validated deployment-label key.
fn render_component_deployment_label_key(key: &ComponentDeploymentLabelKey) -> TokenStream {
    let value = key.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentDeploymentLabelKey::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component deployment label key was validated at build time")
    }
}

// Render one validated deployment-label value.
fn render_component_deployment_label_value(value: &ComponentDeploymentLabelValue) -> TokenStream {
    let value = value.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentDeploymentLabelValue::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Component deployment label value was validated at build time")
    }
}

// Render one validated Fleet service identifier.
fn render_fleet_service_id(service: &FleetServiceId) -> TokenStream {
    let value = service.as_str();
    quote! {
        ::canic::__internal::core::bootstrap::compiled::FleetServiceId::try_from(
            ::std::string::String::from(#value)
        ).expect("embedded Fleet service ID was validated at build time")
    }
}

// Render one validated flattened Component Group member path.
fn render_component_group_member_path(path: &ComponentGroupMemberPath) -> TokenStream {
    let members = render_vec(path.as_slice().iter(), render_component_group_member_id);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupMemberPath::try_from(
            #members
        ).expect("embedded Component Group member path was validated at build time")
    }
}

// Render one typed Fleet-service member purpose.
fn render_fleet_service_member_purpose(purpose: FleetServiceMemberPurpose) -> TokenStream {
    match purpose {
        FleetServiceMemberPurpose::Authority => quote! {
            ::canic::__internal::core::bootstrap::compiled::FleetServiceMemberPurpose::Authority
        },
        FleetServiceMemberPurpose::Replica => quote! {
            ::canic::__internal::core::bootstrap::compiled::FleetServiceMemberPurpose::Replica
        },
        FleetServiceMemberPurpose::PoolMember => quote! {
            ::canic::__internal::core::bootstrap::compiled::FleetServiceMemberPurpose::PoolMember
        },
    }
}

// Render one strict checked-in Component Group declaration.
fn render_component_group_spec_config(config: &ComponentGroupSpecConfig) -> TokenStream {
    let components = render_btree_map(
        config.components.iter(),
        render_component_group_member_id,
        render_component_group_component_config,
    );
    let groups = render_btree_map(
        config.groups.iter(),
        render_component_group_member_id,
        render_component_group_include_config,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupSpecConfig {
            components: #components,
            groups: #groups,
        }
    }
}

// Render one direct Component occurrence in a checked-in group.
fn render_component_group_component_config(config: &ComponentGroupComponentConfig) -> TokenStream {
    let component_spec = render_component_spec_id(&config.component_spec);
    let service = render_option(config.service.as_ref(), render_fleet_service_id);
    let service_purpose = render_option(config.service_purpose.as_ref(), |purpose| {
        render_fleet_service_member_purpose(*purpose)
    });
    let labels = render_btree_map(
        config.labels.iter(),
        render_component_deployment_label_key,
        render_component_deployment_label_value,
    );
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupComponentConfig {
            component_spec: #component_spec,
            service: #service,
            service_purpose: #service_purpose,
            labels: #labels,
        }
    }
}

// Render one exact flattened-member reduction declaration.
fn render_component_deployment_member_limit_config(
    config: &ComponentDeploymentMemberLimitConfig,
) -> TokenStream {
    let member = render_component_group_member_path(&config.member);
    let maximum_descendants = render_option(config.maximum_descendants.as_ref(), |value| {
        render_u32_literal(*value)
    });
    let maximum_registry_bytes = render_option(config.maximum_registry_bytes.as_ref(), |value| {
        render_u64_literal(*value)
    });
    let spawn_grants = render_vec(
        config.spawn_grants.iter(),
        render_component_deployment_spawn_grant_limit_config,
    );
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentDeploymentMemberLimitConfig {
            member: #member,
            maximum_descendants: #maximum_descendants,
            maximum_registry_bytes: #maximum_registry_bytes,
            spawn_grants: #spawn_grants,
        }
    }
}

// Render one exact spawn-grant reduction declaration.
fn render_component_deployment_spawn_grant_limit_config(
    config: &ComponentDeploymentSpawnGrantLimitConfig,
) -> TokenStream {
    let parent_role = render_canister_role(&config.parent_role);
    let child_role = render_canister_role(&config.child_role);
    let maximum_instances_per_parent = render_u32_literal(config.maximum_instances_per_parent);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentDeploymentSpawnGrantLimitConfig {
            parent_role: #parent_role,
            child_role: #child_role,
            maximum_instances_per_parent: #maximum_instances_per_parent,
        }
    }
}

// Render one configuration-only Component Group inclusion edge.
fn render_component_group_include_config(config: &ComponentGroupIncludeConfig) -> TokenStream {
    let component_group = render_component_group_spec_id(&config.component_group);
    let service_purpose = render_option(config.service_purpose.as_ref(), |purpose| {
        render_fleet_service_member_purpose(*purpose)
    });
    let labels = render_btree_map(
        config.labels.iter(),
        render_component_deployment_label_key,
        render_component_deployment_label_value,
    );
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupIncludeConfig {
            component_group: #component_group,
            service_purpose: #service_purpose,
            labels: #labels,
        }
    }
}

// Render one independently scalable Component Group source deployment.
fn render_component_group_deployment_config(
    config: &ComponentGroupDeploymentConfig,
) -> TokenStream {
    let component_group = render_component_group_spec_id(&config.component_group);
    let service_purpose = render_option(config.service_purpose.as_ref(), |purpose| {
        render_fleet_service_member_purpose(*purpose)
    });
    let labels = render_btree_map(
        config.labels.iter(),
        render_component_deployment_label_key,
        render_component_deployment_label_value,
    );
    let member_limits = render_vec(
        config.member_limits.iter(),
        render_component_deployment_member_limit_config,
    );
    let initial_placements = render_u32_literal(config.initial_placements);
    let maximum_placements = render_u32_literal(config.maximum_placements);
    let placement = render_component_group_placement_policy_config(&config.placement);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupDeploymentConfig {
            component_group: #component_group,
            service_purpose: #service_purpose,
            labels: #labels,
            member_limits: #member_limits,
            initial_placements: #initial_placements,
            maximum_placements: #maximum_placements,
            placement: #placement,
        }
    }
}

// Render one Component Group deployment density and spread envelope.
fn render_component_group_placement_policy_config(
    config: &ComponentGroupPlacementPolicyConfig,
) -> TokenStream {
    let maximum_per_root = render_u32_literal(config.maximum_per_root);
    let minimum_distinct_roots = render_u32_literal(config.minimum_distinct_roots);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentGroupPlacementPolicyConfig {
            maximum_per_root: #maximum_per_root,
            minimum_distinct_roots: #minimum_distinct_roots,
        }
    }
}

// Render the top-level application service namespace.
fn render_services_config(config: &ServicesConfig) -> TokenStream {
    let fleet = render_fleet_services_config(&config.fleet);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::ServicesConfig {
            fleet: #fleet,
        }
    }
}

// Render every Fleet-service target in canonical map order.
fn render_fleet_services_config(config: &FleetServicesConfig) -> TokenStream {
    let targets = render_btree_map(
        config.targets.iter(),
        render_fleet_service_id,
        render_fleet_service_target_config,
    );
    quote! {
        ::canic::__internal::core::bootstrap::compiled::FleetServicesConfig {
            targets: #targets,
        }
    }
}

// Render one mode-specific Fleet-service target declaration.
fn render_fleet_service_target_config(config: &FleetServiceTargetConfig) -> TokenStream {
    match config {
        FleetServiceTargetConfig::AuthorityReplica {
            role,
            component_spec,
            authority_deployment,
            authority_member,
            placement,
        } => {
            let role = render_canister_role(role);
            let component_spec = render_component_spec_id(component_spec);
            let authority_deployment = render_component_group_deployment_id(authority_deployment);
            let authority_member = render_component_group_member_path(authority_member);
            let placement = render_fleet_service_placement_policy_config(placement);
            quote! {
                ::canic::__internal::core::bootstrap::compiled::FleetServiceTargetConfig::AuthorityReplica {
                    role: #role,
                    component_spec: #component_spec,
                    authority_deployment: #authority_deployment,
                    authority_member: #authority_member,
                    placement: #placement,
                }
            }
        }
        FleetServiceTargetConfig::ActivePool {
            role,
            component_spec,
            placement,
        } => {
            let role = render_canister_role(role);
            let component_spec = render_component_spec_id(component_spec);
            let placement = render_fleet_service_placement_policy_config(placement);
            quote! {
                ::canic::__internal::core::bootstrap::compiled::FleetServiceTargetConfig::ActivePool {
                    role: #role,
                    component_spec: #component_spec,
                    placement: #placement,
                }
            }
        }
    }
}

// Render one Fleet-service-wide density and spread envelope.
fn render_fleet_service_placement_policy_config(
    config: &FleetServicePlacementPolicyConfig,
) -> TokenStream {
    let maximum_members_per_root = render_u32_literal(config.maximum_members_per_root);
    let minimum_distinct_roots = render_u32_literal(config.minimum_distinct_roots);
    quote! {
        ::canic::__internal::core::bootstrap::compiled::FleetServicePlacementPolicyConfig {
            maximum_members_per_root: #maximum_members_per_root,
            minimum_distinct_roots: #minimum_distinct_roots,
        }
    }
}

// Render a string allocation explicitly so generated code stays self-contained.
fn render_owned_string(value: &str) -> TokenStream {
    quote!(::std::string::String::from(#value))
}

// Render an optional value with a caller-provided item renderer.
fn render_option<T, F>(value: Option<&T>, render: F) -> TokenStream
where
    F: Fn(&T) -> TokenStream,
{
    if let Some(value) = value {
        let rendered = render(value);
        quote!(::core::option::Option::Some(#rendered))
    } else {
        quote!(::core::option::Option::None)
    }
}

// Render a vector with a caller-provided element renderer.
fn render_vec<'a, T: 'a, I, F>(items: I, render: F) -> TokenStream
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> TokenStream,
{
    let rendered = items.into_iter().map(render).collect::<Vec<_>>();
    quote!(vec![#(#rendered),*])
}

// Render a BTreeSet with a caller-provided element renderer.
fn render_btree_set<'a, T: 'a, I, F>(items: I, render: F) -> TokenStream
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> TokenStream,
{
    let rendered = items.into_iter().map(render).collect::<Vec<_>>();
    if rendered.is_empty() {
        return quote!(::std::collections::BTreeSet::new());
    }

    quote!({
        let mut set = ::std::collections::BTreeSet::new();
        #( set.insert(#rendered); )*
        set
    })
}

// Render a BTreeMap with caller-provided key and value renderers.
fn render_btree_map<'a, K: 'a, V: 'a, I, FK, FV>(
    items: I,
    render_key: FK,
    render_value: FV,
) -> TokenStream
where
    I: IntoIterator<Item = (&'a K, &'a V)>,
    FK: Fn(&K) -> TokenStream,
    FV: Fn(&V) -> TokenStream,
{
    let entries = items
        .into_iter()
        .map(|(key, value)| (render_key(key), render_value(value)))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return quote!(::std::collections::BTreeMap::new());
    }

    let keys = entries.iter().map(|(key, _)| key);
    let values = entries.iter().map(|(_, value)| value);

    quote!({
        let mut map = ::std::collections::BTreeMap::new();
        #( map.insert(#keys, #values); )*
        map
    })
}

// Render the top-level standards feature flags.
fn render_standards(standards: &Standards) -> TokenStream {
    let icrc21 = standards.icrc21;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::Standards {
            icrc21: #icrc21,
        }
    }
}

// Render the log retention configuration.
fn render_log_config(config: &LogConfig) -> TokenStream {
    let max_entries = render_u64_literal(config.max_entries);
    let max_entry_bytes = render_u32_literal(config.max_entry_bytes);
    let max_age_secs = render_option(config.max_age_secs.as_ref(), |value| {
        render_u64_literal(*value)
    });

    quote! {
        ::canic::__internal::core::bootstrap::compiled::LogConfig {
            max_entries: #max_entries,
            max_entry_bytes: #max_entry_bytes,
            max_age_secs: #max_age_secs,
        }
    }
}

// Render the authentication configuration bundle.
fn render_auth_config(config: &AuthConfig) -> TokenStream {
    let delegated_tokens = render_delegated_token_config(&config.delegated_tokens);
    let role_attestation = render_role_attestation_config(&config.role_attestation);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::AuthConfig {
            delegated_tokens: #delegated_tokens,
            role_attestation: #role_attestation,
        }
    }
}

// Render the delegated-token config subtree.
fn render_delegated_token_config(config: &DelegatedTokenConfig) -> TokenStream {
    let enabled = config.enabled;
    let root_canister_id = render_option(config.root_canister_id.as_ref(), |value| {
        render_owned_string(value)
    });
    let ic_root_public_key_raw_hex =
        render_option(config.ic_root_public_key_raw_hex.as_ref(), |value| {
            render_owned_string(value)
        });
    let chain_key_root_proof = render_chain_key_root_proof_config(&config.chain_key_root_proof);
    let build_network = match config.build_network {
        BuildNetwork::Ic => {
            quote!(::canic::__internal::core::bootstrap::compiled::BuildNetwork::Ic)
        }
        BuildNetwork::Local => {
            quote!(::canic::__internal::core::bootstrap::compiled::BuildNetwork::Local)
        }
    };
    let max_ttl_secs = render_option(config.max_ttl_secs.as_ref(), |value| {
        render_u64_literal(*value)
    });

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DelegatedTokenConfig {
            enabled: #enabled,
            root_canister_id: #root_canister_id,
            ic_root_public_key_raw_hex: #ic_root_public_key_raw_hex,
            chain_key_root_proof: #chain_key_root_proof,
            build_network: #build_network,
            max_ttl_secs: #max_ttl_secs,
        }
    }
}

fn render_chain_key_root_proof_config(config: &ChainKeyRootProofConfig) -> TokenStream {
    let key_id = render_option(config.key_id.as_ref(), |value| render_owned_string(value));
    let derivation_path_hash_hex =
        render_option(config.derivation_path_hash_hex.as_ref(), |value| {
            render_owned_string(value)
        });
    let derivation_path_hex = render_option(config.derivation_path_hex.as_ref(), |components| {
        render_vec(components.iter(), |component| {
            render_owned_string(component)
        })
    });
    let public_key_hex = render_option(config.public_key_hex.as_ref(), |value| {
        render_owned_string(value)
    });
    let key_version = render_option(config.key_version.as_ref(), |value| {
        render_u64_literal(*value)
    });
    let min_accepted_key_version =
        render_option(config.min_accepted_key_version.as_ref(), |value| {
            render_u64_literal(*value)
        });
    let min_accepted_proof_epoch =
        render_option(config.min_accepted_proof_epoch.as_ref(), |value| {
            render_u64_literal(*value)
        });
    let min_accepted_registry_epoch =
        render_option(config.min_accepted_registry_epoch.as_ref(), |value| {
            render_u64_literal(*value)
        });
    let valid_from_ns = render_option(config.valid_from_ns.as_ref(), |value| {
        render_u64_literal(*value)
    });
    let accept_until_ns = render_option(config.accept_until_ns.as_ref(), |value| {
        render_u64_literal(*value)
    });
    let max_revocation_latency_ns =
        render_option(config.max_revocation_latency_ns.as_ref(), |value| {
            render_u64_literal(*value)
        });
    let allow_test_key = config.allow_test_key;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ChainKeyRootProofConfig {
            key_id: #key_id,
            derivation_path_hash_hex: #derivation_path_hash_hex,
            derivation_path_hex: #derivation_path_hex,
            public_key_hex: #public_key_hex,
            key_version: #key_version,
            min_accepted_key_version: #min_accepted_key_version,
            min_accepted_proof_epoch: #min_accepted_proof_epoch,
            min_accepted_registry_epoch: #min_accepted_registry_epoch,
            valid_from_ns: #valid_from_ns,
            accept_until_ns: #accept_until_ns,
            max_revocation_latency_ns: #max_revocation_latency_ns,
            allow_test_key: #allow_test_key,
        }
    }
}

// Render the role-attestation config subtree.
fn render_role_attestation_config(config: &RoleAttestationConfig) -> TokenStream {
    let max_ttl_secs = render_u64_literal(config.max_ttl_secs);
    let min_accepted_epoch_by_role = render_btree_map(
        config.min_accepted_epoch_by_role.iter(),
        |role| render_owned_string(role),
        |epoch| render_u64_literal(*epoch),
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::RoleAttestationConfig {
            max_ttl_secs: #max_ttl_secs,
            min_accepted_epoch_by_role: #min_accepted_epoch_by_role,
        }
    }
}

// Render the app-level configuration subtree.
fn render_app_config(config: &AppConfig) -> TokenStream {
    let name = render_app_id(&config.name);
    let init_mode = render_fleet_init_mode(config.init_mode);
    let whitelist = render_option(config.whitelist.as_ref(), render_whitelist);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::AppConfig {
            name: #name,
            init_mode: #init_mode,
            whitelist: #whitelist,
        }
    }
}

// Render an immutable App source identity.
fn render_app_id(app: &AppId) -> TokenStream {
    let value = render_owned_string(app.as_str());
    quote!(::canic::__internal::core::bootstrap::compiled::AppId::owned(#value))
}

// Render the initial app mode enum.
fn render_fleet_init_mode(mode: FleetInitMode) -> TokenStream {
    match mode {
        FleetInitMode::Enabled => {
            quote!(::canic::__internal::core::bootstrap::compiled::FleetInitMode::Enabled)
        }
        FleetInitMode::Readonly => {
            quote!(::canic::__internal::core::bootstrap::compiled::FleetInitMode::Readonly)
        }
        FleetInitMode::Disabled => {
            quote!(::canic::__internal::core::bootstrap::compiled::FleetInitMode::Disabled)
        }
    }
}

// Render the principal whitelist.
fn render_whitelist(whitelist: &Whitelist) -> TokenStream {
    let principals = render_btree_set(whitelist.principals.iter(), |principal| {
        render_owned_string(principal)
    });

    quote! {
        ::canic::__internal::core::bootstrap::compiled::Whitelist {
            principals: #principals,
        }
    }
}

// Render one flat Component Spec and its potential child-role catalog.
fn render_component_spec_config(config: &ComponentSpecConfig) -> TokenStream {
    let component_role = render_canister_role(&config.component_role);
    let maximum_instances = render_u32_literal(config.maximum_instances);
    let limits = render_component_limits_config(&config.limits);
    let initial_cycles = render_cycles(config.initial_cycles.to_u128());
    let topup = render_option(config.topup.as_ref(), render_topup);
    let cycles_funding = render_cycles_funding_policy(&config.cycles_funding);
    let scaling = render_option(config.scaling.as_ref(), render_scaling_config);
    let sharding = render_option(config.sharding.as_ref(), render_sharding_config);
    let index = render_option(config.index.as_ref(), render_index_config);
    let auth = render_canister_auth_config(&config.auth);
    let standards = render_standards_canister_config(&config.standards);
    let diagnostics = render_diagnostics_canister_config(config.diagnostics);
    let metrics = render_metrics_canister_config(config.metrics);
    let provisions = render_btree_map(
        config.provisions.iter(),
        render_component_spec_id,
        render_component_provisioning_grant_config,
    );
    let children = render_btree_map(
        config.children.iter(),
        render_canister_role,
        render_component_child_config,
    );
    let spawn_grants =
        render_btree_map(config.spawn_grants.iter(), render_canister_role, |grants| {
            render_btree_map(
                grants.iter(),
                render_canister_role,
                render_component_spawn_grant_config,
            )
        });

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentSpecConfig {
            component_role: #component_role,
            maximum_instances: #maximum_instances,
            limits: #limits,
            initial_cycles: #initial_cycles,
            topup: #topup,
            cycles_funding: #cycles_funding,
            scaling: #scaling,
            sharding: #sharding,
            index: #index,
            auth: #auth,
            standards: #standards,
            diagnostics: #diagnostics,
            metrics: #metrics,
            provisions: #provisions,
            children: #children,
            spawn_grants: #spawn_grants,
        }
    }
}

// Render aggregate limits for one concrete Component.
fn render_component_limits_config(config: &ComponentLimitsConfig) -> TokenStream {
    let maximum_descendants = render_u32_literal(config.maximum_descendants);
    let maximum_registry_bytes = render_u64_literal(config.maximum_registry_bytes);
    let cycles_funding = render_cycles_funding_budget_config(&config.cycles_funding);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentLimitsConfig {
            maximum_descendants: #maximum_descendants,
            maximum_registry_bytes: #maximum_registry_bytes,
            cycles_funding: #cycles_funding,
        }
    }
}

// Render one aggregate cycles-funding budget.
fn render_cycles_funding_budget_config(config: &CyclesFundingBudgetConfig) -> TokenStream {
    let window_secs = render_u64_literal(config.window_secs);
    let maximum_cycles = render_cycles(config.maximum_cycles.to_u128());

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CyclesFundingBudgetConfig {
            window_secs: #window_secs,
            maximum_cycles: #maximum_cycles,
        }
    }
}

// Render one potential Component child role.
fn render_component_child_config(config: &ComponentChildConfig) -> TokenStream {
    let kind = render_component_child_kind(config.kind);
    let initial_cycles = render_cycles(config.initial_cycles.to_u128());
    let topup = render_option(config.topup.as_ref(), render_topup);
    let cycles_funding = render_cycles_funding_policy(&config.cycles_funding);
    let scaling = render_option(config.scaling.as_ref(), render_scaling_config);
    let sharding = render_option(config.sharding.as_ref(), render_sharding_config);
    let index = render_option(config.index.as_ref(), render_index_config);
    let auth = render_canister_auth_config(&config.auth);
    let standards = render_standards_canister_config(&config.standards);
    let diagnostics = render_diagnostics_canister_config(config.diagnostics);
    let metrics = render_metrics_canister_config(config.metrics);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentChildConfig {
            kind: #kind,
            initial_cycles: #initial_cycles,
            topup: #topup,
            cycles_funding: #cycles_funding,
            scaling: #scaling,
            sharding: #sharding,
            index: #index,
            auth: #auth,
            standards: #standards,
            diagnostics: #diagnostics,
            metrics: #metrics,
        }
    }
}

// Render one bounded parent-role to child-role spawn grant.
fn render_component_spawn_grant_config(config: &ComponentSpawnGrantConfig) -> TokenStream {
    let maximum_instances_per_parent = render_u32_literal(config.maximum_instances_per_parent);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentSpawnGrantConfig {
            maximum_instances_per_parent: #maximum_instances_per_parent,
        }
    }
}

// Render one bounded non-parent peer-Component grant.
fn render_component_provisioning_grant_config(
    config: &ComponentProvisioningGrantConfig,
) -> TokenStream {
    let maximum_instances_per_requester_per_root =
        render_u32_literal(config.maximum_instances_per_requester_per_root);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ComponentProvisioningGrantConfig {
            maximum_instances_per_requester_per_root:
                #maximum_instances_per_requester_per_root,
        }
    }
}

// Render parent-to-child cycles funding policy limits.
fn render_cycles_funding_policy(policy: &CyclesFundingPolicyConfig) -> TokenStream {
    let max_per_request = render_cycles(policy.max_per_request.to_u128());
    let max_per_child = render_cycles(policy.max_per_child.to_u128());
    let cooldown_secs = render_u64_literal(policy.cooldown_secs);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CyclesFundingPolicyConfig {
            max_per_request: #max_per_request,
            max_per_child: #max_per_child,
            cooldown_secs: #cooldown_secs,
        }
    }
}

// Render the per-canister diagnostics config.
fn render_diagnostics_canister_config(config: DiagnosticsCanisterConfig) -> TokenStream {
    let memory_ledger = config.memory_ledger;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DiagnosticsCanisterConfig {
            memory_ledger: #memory_ledger,
        }
    }
}

// Render per-canister metrics profile configuration.
fn render_metrics_canister_config(config: MetricsCanisterConfig) -> TokenStream {
    let profile = render_option(config.profile.as_ref(), |profile| {
        render_metrics_profile(*profile)
    });

    quote! {
        ::canic::__internal::core::bootstrap::compiled::MetricsCanisterConfig {
            profile: #profile,
        }
    }
}

// Render a metrics profile enum.
fn render_metrics_profile(profile: MetricsProfile) -> TokenStream {
    match profile {
        MetricsProfile::Leaf => {
            quote!(::canic::__internal::core::bootstrap::compiled::MetricsProfile::Leaf)
        }
        MetricsProfile::Hub => {
            quote!(::canic::__internal::core::bootstrap::compiled::MetricsProfile::Hub)
        }
        MetricsProfile::Storage => {
            quote!(::canic::__internal::core::bootstrap::compiled::MetricsProfile::Storage)
        }
        MetricsProfile::Root => {
            quote!(::canic::__internal::core::bootstrap::compiled::MetricsProfile::Root)
        }
        MetricsProfile::Full => {
            quote!(::canic::__internal::core::bootstrap::compiled::MetricsProfile::Full)
        }
    }
}

// Render the canister kind enum.
fn render_component_child_kind(kind: ComponentChildKind) -> TokenStream {
    match kind {
        ComponentChildKind::Singleton => {
            quote!(::canic::__internal::core::bootstrap::compiled::ComponentChildKind::Singleton)
        }
        ComponentChildKind::Replica => {
            quote!(::canic::__internal::core::bootstrap::compiled::ComponentChildKind::Replica)
        }
        ComponentChildKind::Shard => {
            quote!(::canic::__internal::core::bootstrap::compiled::ComponentChildKind::Shard)
        }
        ComponentChildKind::Instance => {
            quote!(::canic::__internal::core::bootstrap::compiled::ComponentChildKind::Instance)
        }
    }
}

// Render a cycles wrapper constructor.
fn render_cycles(value: u128) -> TokenStream {
    let value = render_u128_literal(value);
    quote!(::canic::__internal::core::bootstrap::compiled::Cycles::new(#value))
}

// Render a large integer literal with separators so generated code stays clippy-clean.
fn render_u128_literal(value: u128) -> TokenStream {
    let digits = value.to_string();
    let grouped = digits
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (index, ch)| {
            if index > 0 && index % 3 == 0 {
                acc.push('_');
            }
            acc.push(ch);
            acc
        })
        .chars()
        .rev()
        .collect::<String>();

    format!("{grouped}_u128")
        .parse()
        .expect("valid u128 literal")
}

// Render a u32 literal with separators so generated code stays clippy-clean.
fn render_u32_literal(value: u32) -> TokenStream {
    let digits = value.to_string();
    let grouped = digits
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (index, ch)| {
            if index > 0 && index % 3 == 0 {
                acc.push('_');
            }
            acc.push(ch);
            acc
        })
        .chars()
        .rev()
        .collect::<String>();

    format!("{grouped}_u32").parse().expect("valid u32 literal")
}

// Render a u64 literal with separators so generated code stays clippy-clean.
fn render_u64_literal(value: u64) -> TokenStream {
    let digits = value.to_string();
    let grouped = digits
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (index, ch)| {
            if index > 0 && index % 3 == 0 {
                acc.push('_');
            }
            acc.push(ch);
            acc
        })
        .chars()
        .rev()
        .collect::<String>();

    format!("{grouped}_u64").parse().expect("valid u64 literal")
}

// Render the automatic top-up policy.
fn render_topup(policy: &TopupPolicy) -> TokenStream {
    let threshold = render_cycles(policy.threshold.to_u128());
    let amount = render_cycles(policy.amount.to_u128());

    quote! {
        ::canic::__internal::core::bootstrap::compiled::TopupPolicy {
            threshold: #threshold,
            amount: #amount,
        }
    }
}

// Render the optional ICP-to-cycles refill policy.
#[cfg(test)]
fn render_icp_refill_policy(policy: &IcpRefillPolicy) -> TokenStream {
    let max_refill_e8s_per_call = render_u64_literal(policy.max_refill_e8s_per_call);
    let min_xdr_permyriad_per_icp =
        render_option(policy.min_xdr_permyriad_per_icp.as_ref(), |value| {
            render_u64_literal(*value)
        });
    let ledger_canister_id = render_option(policy.ledger_canister_id.as_ref(), render_principal);
    let cmc_canister_id = render_option(policy.cmc_canister_id.as_ref(), render_principal);
    let allow_ic_system_canister_overrides = policy.allow_ic_system_canister_overrides;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::IcpRefillPolicy {
            max_refill_e8s_per_call: #max_refill_e8s_per_call,
            min_xdr_permyriad_per_icp: #min_xdr_permyriad_per_icp,
            ledger_canister_id: #ledger_canister_id,
            cmc_canister_id: #cmc_canister_id,
            allow_ic_system_canister_overrides: #allow_ic_system_canister_overrides,
        }
    }
}

// Render the delegated-auth role config.
fn render_canister_auth_config(config: &CanisterAuthConfig) -> TokenStream {
    let issuer = config.delegated_token_issuer;
    let verifier = config.delegated_token_verifier;
    let local_application_authorization = match &config.local_application_authorization {
        None => quote!(::core::option::Option::None),
        Some(config) => {
            let config = render_local_application_authorization_config(config);
            quote!(::core::option::Option::Some(#config))
        }
    };
    let role_attestation_cache = config.role_attestation_cache;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CanisterAuthConfig {
            delegated_token_issuer: #issuer,
            delegated_token_verifier: #verifier,
            local_application_authorization: #local_application_authorization,
            role_attestation_cache: #role_attestation_cache,
        }
    }
}

fn render_local_application_authorization_config(
    config: &LocalApplicationAuthorizationConfig,
) -> TokenStream {
    let allowed_scopes = &config.allowed_scopes;
    let default_session_ttl_secs = config.default_session_ttl_secs;
    let maximum_session_ttl_secs = config.maximum_session_ttl_secs;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::LocalApplicationAuthorizationConfig {
            allowed_scopes: ::std::vec![#(#allowed_scopes.to_string()),*],
            default_session_ttl_secs: #default_session_ttl_secs,
            maximum_session_ttl_secs: #maximum_session_ttl_secs,
        }
    }
}

// Render the per-canister standards config.
fn render_standards_canister_config(config: &StandardsCanisterConfig) -> TokenStream {
    let icrc21 = config.icrc21;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::StandardsCanisterConfig {
            icrc21: #icrc21,
        }
    }
}

// Render the scaling config subtree.
fn render_scaling_config(config: &ScalingConfig) -> TokenStream {
    let pools = render_btree_map(
        config.pools.iter(),
        |name| render_owned_string(name),
        render_scale_pool,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ScalingConfig {
            pools: #pools,
        }
    }
}

// Render a stateless scaling pool definition.
fn render_scale_pool(pool: &ScalePool) -> TokenStream {
    let canister_role = render_canister_role(&pool.canister_role);
    let policy = render_scale_pool_policy(&pool.policy);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ScalePool {
            canister_role: #canister_role,
            policy: #policy,
        }
    }
}

// Render the scaling pool worker policy.
fn render_scale_pool_policy(policy: &ScalePoolPolicy) -> TokenStream {
    let initial_workers = render_u32_literal(policy.initial_workers);
    let min_workers = render_u32_literal(policy.min_workers);
    let max_workers = render_u32_literal(policy.max_workers);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ScalePoolPolicy {
            initial_workers: #initial_workers,
            min_workers: #min_workers,
            max_workers: #max_workers,
        }
    }
}

// Render the sharding config subtree.
fn render_sharding_config(config: &ShardingConfig) -> TokenStream {
    let pools = render_btree_map(
        config.pools.iter(),
        |name| render_owned_string(name),
        render_shard_pool,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ShardingConfig {
            pools: #pools,
        }
    }
}

// Render the keyed placement index config subtree.
fn render_index_config(config: &IndexConfig) -> TokenStream {
    let pools = render_btree_map(
        config.pools.iter(),
        |name| render_owned_string(name),
        render_index_pool,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::IndexConfig {
            pools: #pools,
        }
    }
}

// Render a stateful shard pool definition.
fn render_shard_pool(pool: &ShardPool) -> TokenStream {
    let canister_role = render_canister_role(&pool.canister_role);
    let policy = render_shard_pool_policy(&pool.policy);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ShardPool {
            canister_role: #canister_role,
            policy: #policy,
        }
    }
}

// Render the shard pool capacity policy.
fn render_shard_pool_policy(policy: &ShardPoolPolicy) -> TokenStream {
    let capacity = render_u32_literal(policy.capacity);
    let initial_shards = render_u32_literal(policy.initial_shards);
    let max_shards = render_u32_literal(policy.max_shards);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ShardPoolPolicy {
            capacity: #capacity,
            initial_shards: #initial_shards,
            max_shards: #max_shards,
        }
    }
}

// Render one keyed-instance placement pool.
fn render_index_pool(pool: &IndexPool) -> TokenStream {
    let canister_role = render_canister_role(&pool.canister_role);
    let key_name = render_owned_string(&pool.key_name);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::IndexPool {
            canister_role: #canister_role,
            key_name: #key_name,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_TOPOLOGY_CONFIG: &str = r#"
[app]
name = "render_v3"

[roles.root]
kind = "root"
package = "root"

[roles.hub]
kind = "canister"
package = "hub"

[roles.instance]
kind = "canister"
package = "instance"

[roles.ledger]
kind = "canister"
package = "ledger"

[component_specs.hub]
component_role = "hub"
maximum_instances = 1

[component_specs.hub.children.instance]
kind = "instance"

[component_specs.hub.children.ledger]
kind = "instance"

[component_specs.hub.spawn_grants.hub.instance]
maximum_instances_per_parent = 100

[component_specs.hub.spawn_grants.instance.ledger]
maximum_instances_per_parent = 1

[component_specs.hub.provisions.instance]
maximum_instances_per_requester_per_root = 100

[component_specs.instance]
component_role = "instance"
maximum_instances = 100

[component_groups.shared.components.hub]
component_spec = "hub"
service = "hubs"
labels = { role = "hub" }

[component_groups.cell.components.instance]
component_spec = "instance"

[component_groups.cell.components.database]
component_spec = "instance"
service = "database"
service_purpose = "authority"

[component_groups.cell.groups.shared]
component_group = "shared"
service_purpose = "pool_member"
labels = { inclusion = "shared" }

[component_group_deployments.cell]
component_group = "cell"
labels = { workload = "project" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.cell.member_limits]]
member = ["shared", "hub"]
maximum_descendants = 50
maximum_registry_bytes = 8_388_608
spawn_grants = [
  { parent_role = "hub", child_role = "instance", maximum_instances_per_parent = 50 },
]

[services.fleet.targets.database]
role = "instance"
component_spec = "instance"
mode = "authority_replica"
authority_deployment = "cell"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.hubs]
role = "hub"
component_spec = "hub"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1
"#;

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn render_icp_refill_policy_preserves_system_canister_overrides() {
        let rendered = render_icp_refill_policy(&IcpRefillPolicy {
            max_refill_e8s_per_call: 100_000_000,
            min_xdr_permyriad_per_icp: Some(40_000),
            ledger_canister_id: Some(principal(11)),
            cmc_canister_id: Some(principal(12)),
            allow_ic_system_canister_overrides: true,
        })
        .to_string();

        assert!(rendered.contains("ledger_canister_id"));
        assert!(rendered.contains("cmc_canister_id"));
        assert!(rendered.contains("allow_ic_system_canister_overrides"));
        assert!(rendered.contains("Principal :: from_slice"));
        assert!(rendered.contains("100_000_000_u64"));
        assert!(rendered.contains("40_000_u64"));
    }

    #[test]
    fn render_large_config_literals_with_clippy_clean_separators() {
        let log = render_log_config(&LogConfig::default()).to_string();
        let component_limits =
            render_component_limits_config(&ComponentLimitsConfig::default()).to_string();

        assert!(log.contains("10_000_u64"));
        assert!(log.contains("16_384_u32"));
        assert!(component_limits.contains("16_777_216_u64"));
        assert!(component_limits.contains("3_600_u64"));
    }

    #[test]
    fn render_component_topology_and_nested_group_declarations() {
        let config = crate::config::Config::parse_toml(COMPONENT_TOPOLOGY_CONFIG)
            .expect("valid Component and Component Group topology config");
        let rendered = config_model(&config);

        assert!(rendered.contains("ComponentProvisioningGrantConfig"));
        assert!(rendered.contains("ComponentSpawnGrantConfig"));
        assert!(rendered.contains("ComponentGroupSpecConfig"));
        assert!(rendered.contains("ComponentGroupComponentConfig"));
        assert!(rendered.contains("ComponentGroupIncludeConfig"));
        assert!(rendered.contains("ComponentGroupDeploymentConfig"));
        assert!(rendered.contains("ComponentDeploymentMemberLimitConfig"));
        assert!(rendered.contains("ComponentDeploymentSpawnGrantLimitConfig"));
        assert!(rendered.contains("ComponentGroupPlacementPolicyConfig"));
        assert!(rendered.contains("ServicesConfig"));
        assert!(rendered.contains("FleetServicesConfig"));
        assert!(rendered.contains("FleetServiceTargetConfig :: ActivePool"));
        assert!(rendered.contains("FleetServiceTargetConfig :: AuthorityReplica"));
        assert!(rendered.contains("FleetServicePlacementPolicyConfig"));
        assert!(rendered.contains("embedded Component Group Spec ID was validated at build time"));
        assert!(
            rendered.contains("embedded Component Group member ID was validated at build time")
        );
        assert!(
            rendered.contains("embedded Component Group deployment ID was validated at build time")
        );
        assert!(
            rendered.contains("embedded Component Group member path was validated at build time")
        );
        assert!(rendered.contains("embedded Fleet service ID was validated at build time"));
        assert!(
            rendered
                .contains("embedded Component deployment label key was validated at build time")
        );
        assert!(
            rendered
                .contains("embedded Component deployment label value was validated at build time")
        );
        assert!(rendered.contains("FleetServiceMemberPurpose :: PoolMember"));
        assert!(rendered.contains("maximum_instances_per_requester_per_root : 100_u32"));
        assert!(rendered.contains("maximum_instances_per_parent : 100_u32"));
        assert!(!rendered.contains("initial_instances"));
    }
}
