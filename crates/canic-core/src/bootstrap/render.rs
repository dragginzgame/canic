use crate::{
    cdk::candid::Principal,
    config::schema::{
        AppConfig, AppInitMode, AuthConfig, CanisterAuthConfig, CanisterConfig, CanisterKind,
        CanisterPool, ConfigModel, DelegatedTokenConfig, DiagnosticsCanisterConfig,
        DirectoryConfig, DirectoryPool, FleetConfig, IcpRefillPolicy, LogConfig,
        MetricsCanisterConfig, MetricsProfile, PoolImport, RandomnessConfig, RandomnessSource,
        RoleAttestationConfig, RoleDeclaration, RoleDeclarationKind, ScalePool, ScalePoolPolicy,
        ScalingConfig, ShardPool, ShardPoolPolicy, ShardingConfig, Standards,
        StandardsCanisterConfig, SubnetConfig, TopupPolicy, Whitelist,
    },
    ids::{CanisterRole, SubnetRole},
};
use proc_macro2::TokenStream;
use quote::quote;

// Render the validated config model into a Rust expression string.
pub fn config_model(config: &ConfigModel) -> String {
    let mut source = render_config_model(config).to_string();
    source.push('\n');
    source
}

// Render the top-level configuration model into a portable Rust expression.
fn render_config_model(config: &ConfigModel) -> TokenStream {
    let fleet = render_option(config.fleet.as_ref(), render_fleet_config);
    let controllers = render_vec(config.controllers.iter(), render_principal);
    let standards = render_option(config.standards.as_ref(), render_standards);
    let log = render_log_config(&config.log);
    let auth = render_auth_config(&config.auth);
    let app = render_app_config(&config.app);
    let app_index = render_btree_set(config.app_index.iter(), render_canister_role);
    let roles = render_btree_map(
        config.roles.iter(),
        render_canister_role,
        render_role_declaration,
    );
    let subnets = render_btree_map(
        config.subnets.iter(),
        render_subnet_role,
        render_subnet_config,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ConfigModel {
            fleet: #fleet,
            controllers: #controllers,
            standards: #standards,
            log: #log,
            auth: #auth,
            app: #app,
            app_index: #app_index,
            roles: #roles,
            subnets: #subnets,
        }
    }
}

// Render operator-facing fleet identity metadata.
fn render_fleet_config(config: &FleetConfig) -> TokenStream {
    let name = render_option(config.name.as_ref(), |name| render_owned_string(name));
    quote! {
        ::canic::__internal::core::bootstrap::compiled::FleetConfig {
            name: #name,
        }
    }
}

// Render a fleet role declaration.
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

// Render a subnet role using constants where possible and literals otherwise.
fn render_subnet_role(role: &SubnetRole) -> TokenStream {
    match role.as_str() {
        "prime" => quote!(::canic::__internal::core::bootstrap::compiled::SubnetRole::PRIME),
        value => quote!(::canic::__internal::core::bootstrap::compiled::SubnetRole::from(#value)),
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
    let icrc103 = standards.icrc103;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::Standards {
            icrc21: #icrc21,
            icrc103: #icrc103,
        }
    }
}

// Render the log retention configuration.
fn render_log_config(config: &LogConfig) -> TokenStream {
    let max_entries = config.max_entries;
    let max_entry_bytes = config.max_entry_bytes;
    let max_age_secs = render_option(config.max_age_secs.as_ref(), |value| quote!(#value));

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
    let network = render_owned_string(&config.network);
    let max_ttl_secs = render_option(config.max_ttl_secs.as_ref(), |value| quote!(#value));

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DelegatedTokenConfig {
            enabled: #enabled,
            root_canister_id: #root_canister_id,
            ic_root_public_key_raw_hex: #ic_root_public_key_raw_hex,
            network: #network,
            max_ttl_secs: #max_ttl_secs,
        }
    }
}

// Render the role-attestation config subtree.
fn render_role_attestation_config(config: &RoleAttestationConfig) -> TokenStream {
    let max_ttl_secs = config.max_ttl_secs;
    let min_accepted_epoch_by_role = render_btree_map(
        config.min_accepted_epoch_by_role.iter(),
        |role| render_owned_string(role),
        |epoch| quote!(#epoch),
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
    let init_mode = render_app_init_mode(config.init_mode);
    let whitelist = render_option(config.whitelist.as_ref(), render_whitelist);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::AppConfig {
            init_mode: #init_mode,
            whitelist: #whitelist,
        }
    }
}

// Render the initial app mode enum.
fn render_app_init_mode(mode: AppInitMode) -> TokenStream {
    match mode {
        AppInitMode::Enabled => {
            quote!(::canic::__internal::core::bootstrap::compiled::AppInitMode::Enabled)
        }
        AppInitMode::Readonly => {
            quote!(::canic::__internal::core::bootstrap::compiled::AppInitMode::Readonly)
        }
        AppInitMode::Disabled => {
            quote!(::canic::__internal::core::bootstrap::compiled::AppInitMode::Disabled)
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

// Render a subnet configuration and its canister graph.
fn render_subnet_config(config: &SubnetConfig) -> TokenStream {
    let canisters = render_btree_map(
        config.canisters.iter(),
        render_canister_role,
        render_canister_config,
    );
    let pool = render_canister_pool(&config.pool);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::SubnetConfig {
            canisters: #canisters,
            pool: #pool,
        }
    }
}

// Render the pool import config for spare canister pools.
fn render_pool_import(config: &PoolImport) -> TokenStream {
    let initial = render_option(config.initial.as_ref(), |value| quote!(#value));
    let local = render_vec(config.local.iter(), render_principal);
    let ic = render_vec(config.ic.iter(), render_principal);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::PoolImport {
            initial: #initial,
            local: #local,
            ic: #ic,
        }
    }
}

// Render the top-level canister pool config.
fn render_canister_pool(config: &CanisterPool) -> TokenStream {
    let minimum_size = config.minimum_size;
    let import = render_pool_import(&config.import);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CanisterPool {
            minimum_size: #minimum_size,
            import: #import,
        }
    }
}

// Render a single canister role configuration.
fn render_canister_config(config: &CanisterConfig) -> TokenStream {
    let kind = render_canister_kind(config.kind);
    let initial_cycles = render_cycles(config.initial_cycles.to_u128());
    let topup = render_option(config.topup.as_ref(), render_topup);
    let randomness = render_randomness_config(&config.randomness);
    let scaling = render_option(config.scaling.as_ref(), render_scaling_config);
    let sharding = render_option(config.sharding.as_ref(), render_sharding_config);
    let directory = render_option(config.directory.as_ref(), render_directory_config);
    let auth = render_canister_auth_config(&config.auth);
    let standards = render_standards_canister_config(&config.standards);
    let diagnostics = render_diagnostics_canister_config(config.diagnostics);
    let metrics = render_metrics_canister_config(config.metrics);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CanisterConfig {
            kind: #kind,
            initial_cycles: #initial_cycles,
            topup: #topup,
            randomness: #randomness,
            scaling: #scaling,
            sharding: #sharding,
            directory: #directory,
            auth: #auth,
            standards: #standards,
            diagnostics: #diagnostics,
            metrics: #metrics,
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
fn render_canister_kind(kind: CanisterKind) -> TokenStream {
    match kind {
        CanisterKind::Root => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Root)
        }
        CanisterKind::Service => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Service)
        }
        CanisterKind::Singleton => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Singleton)
        }
        CanisterKind::Replica => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Replica)
        }
        CanisterKind::Shard => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Shard)
        }
        CanisterKind::Instance => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Instance)
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

// Render the automatic top-up policy.
fn render_topup(policy: &TopupPolicy) -> TokenStream {
    let threshold = render_cycles(policy.threshold.to_u128());
    let amount = render_cycles(policy.amount.to_u128());
    let icp_refill = render_option(policy.icp_refill.as_ref(), render_icp_refill_policy);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::TopupPolicy {
            threshold: #threshold,
            amount: #amount,
            icp_refill: #icp_refill,
        }
    }
}

// Render the optional ICP-to-cycles refill policy.
fn render_icp_refill_policy(policy: &IcpRefillPolicy) -> TokenStream {
    let enabled = policy.enabled;
    let min_hub_cycles_before_refill = render_cycles(policy.min_hub_cycles_before_refill.to_u128());
    let max_refill_e8s_per_call = policy.max_refill_e8s_per_call;
    let min_xdr_permyriad_per_icp = render_option(
        policy.min_xdr_permyriad_per_icp.as_ref(),
        |value| quote!(#value),
    );
    let ledger_canister_id = render_option(policy.ledger_canister_id.as_ref(), render_principal);
    let cmc_canister_id = render_option(policy.cmc_canister_id.as_ref(), render_principal);
    let allow_ic_system_canister_overrides = policy.allow_ic_system_canister_overrides;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::IcpRefillPolicy {
            enabled: #enabled,
            min_hub_cycles_before_refill: #min_hub_cycles_before_refill,
            max_refill_e8s_per_call: #max_refill_e8s_per_call,
            min_xdr_permyriad_per_icp: #min_xdr_permyriad_per_icp,
            ledger_canister_id: #ledger_canister_id,
            cmc_canister_id: #cmc_canister_id,
            allow_ic_system_canister_overrides: #allow_ic_system_canister_overrides,
        }
    }
}

// Render the randomness config subtree.
fn render_randomness_config(config: &RandomnessConfig) -> TokenStream {
    let enabled = config.enabled;
    let reseed_interval_secs = config.reseed_interval_secs;
    let source = render_randomness_source(config.source);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::RandomnessConfig {
            enabled: #enabled,
            reseed_interval_secs: #reseed_interval_secs,
            source: #source,
        }
    }
}

// Render the randomness source enum.
fn render_randomness_source(source: RandomnessSource) -> TokenStream {
    match source {
        RandomnessSource::Ic => {
            quote!(::canic::__internal::core::bootstrap::compiled::RandomnessSource::Ic)
        }
        RandomnessSource::Time => {
            quote!(::canic::__internal::core::bootstrap::compiled::RandomnessSource::Time)
        }
    }
}

// Render the delegated-auth role config.
fn render_canister_auth_config(config: &CanisterAuthConfig) -> TokenStream {
    let signer = config.delegated_token_signer;
    let role_attestation_cache = config.role_attestation_cache;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CanisterAuthConfig {
            delegated_token_signer: #signer,
            role_attestation_cache: #role_attestation_cache,
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
    let initial_workers = policy.initial_workers;
    let min_workers = policy.min_workers;
    let max_workers = policy.max_workers;

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

// Render the directory placement config subtree.
fn render_directory_config(config: &DirectoryConfig) -> TokenStream {
    let pools = render_btree_map(
        config.pools.iter(),
        |name| render_owned_string(name),
        render_directory_pool,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DirectoryConfig {
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
    let capacity = policy.capacity;
    let initial_shards = policy.initial_shards;
    let max_shards = policy.max_shards;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ShardPoolPolicy {
            capacity: #capacity,
            initial_shards: #initial_shards,
            max_shards: #max_shards,
        }
    }
}

// Render one keyed-instance placement pool.
fn render_directory_pool(pool: &DirectoryPool) -> TokenStream {
    let canister_role = render_canister_role(&pool.canister_role);
    let key_name = render_owned_string(&pool.key_name);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DirectoryPool {
            canister_role: #canister_role,
            key_name: #key_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::{Cycles, TC};

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn render_icp_refill_policy_preserves_system_canister_overrides() {
        let rendered = render_icp_refill_policy(&IcpRefillPolicy {
            enabled: true,
            min_hub_cycles_before_refill: Cycles::new(2 * TC),
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
    }
}
