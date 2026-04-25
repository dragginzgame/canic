use crate::{
    cdk::candid::Principal,
    config::schema::{
        AppConfig, AppInitMode, AuthConfig, CanisterConfig, CanisterKind, CanisterPool,
        ConfigModel, DelegatedAuthCanisterConfig, DelegatedTokenConfig, DelegationProofCacheConfig,
        DelegationProofCacheProfile, DirectoryConfig, DirectoryPool, LogConfig, PoolImport,
        RandomnessConfig, RandomnessSource, RoleAttestationConfig, ScalePool, ScalePoolPolicy,
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
    let controllers = render_vec(config.controllers.iter(), render_principal);
    let standards = render_option(config.standards.as_ref(), render_standards);
    let log = render_log_config(&config.log);
    let auth = render_auth_config(&config.auth);
    let app = render_app_config(&config.app);
    let app_index = render_btree_set(config.app_index.iter(), render_canister_role);
    let subnets = render_btree_map(
        config.subnets.iter(),
        render_subnet_role,
        render_subnet_config,
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::ConfigModel {
            controllers: #controllers,
            standards: #standards,
            log: #log,
            auth: #auth,
            app: #app,
            app_index: #app_index,
            subnets: #subnets,
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
    let ecdsa_key_name = render_owned_string(&config.ecdsa_key_name);
    let max_ttl_secs = render_option(config.max_ttl_secs.as_ref(), |value| quote!(#value));
    let proof_cache = render_delegation_proof_cache_config(&config.proof_cache);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DelegatedTokenConfig {
            enabled: #enabled,
            ecdsa_key_name: #ecdsa_key_name,
            max_ttl_secs: #max_ttl_secs,
            proof_cache: #proof_cache,
        }
    }
}

// Render the proof-cache tuning block.
fn render_delegation_proof_cache_config(config: &DelegationProofCacheConfig) -> TokenStream {
    let profile = render_option(config.profile.as_ref(), |profile| {
        render_delegation_proof_cache_profile(*profile)
    });
    let shard_count_hint = render_option(config.shard_count_hint.as_ref(), |value| quote!(#value));
    let capacity_override =
        render_option(config.capacity_override.as_ref(), |value| quote!(#value));
    let active_window_secs = config.active_window_secs;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DelegationProofCacheConfig {
            profile: #profile,
            shard_count_hint: #shard_count_hint,
            capacity_override: #capacity_override,
            active_window_secs: #active_window_secs,
        }
    }
}

// Render the proof-cache sizing profile enum.
fn render_delegation_proof_cache_profile(profile: DelegationProofCacheProfile) -> TokenStream {
    match profile {
        DelegationProofCacheProfile::Small => {
            quote!(
                ::canic::__internal::core::bootstrap::compiled::DelegationProofCacheProfile::Small
            )
        }
        DelegationProofCacheProfile::Standard => {
            quote!(::canic::__internal::core::bootstrap::compiled::DelegationProofCacheProfile::Standard)
        }
        DelegationProofCacheProfile::Large => {
            quote!(
                ::canic::__internal::core::bootstrap::compiled::DelegationProofCacheProfile::Large
            )
        }
    }
}

// Render the role-attestation config subtree.
fn render_role_attestation_config(config: &RoleAttestationConfig) -> TokenStream {
    let ecdsa_key_name = render_owned_string(&config.ecdsa_key_name);
    let max_ttl_secs = config.max_ttl_secs;
    let min_accepted_epoch_by_role = render_btree_map(
        config.min_accepted_epoch_by_role.iter(),
        |role| render_owned_string(role),
        |epoch| quote!(#epoch),
    );

    quote! {
        ::canic::__internal::core::bootstrap::compiled::RoleAttestationConfig {
            ecdsa_key_name: #ecdsa_key_name,
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
    let auto_create = render_btree_set(config.auto_create.iter(), render_canister_role);
    let subnet_index = render_btree_set(config.subnet_index.iter(), render_canister_role);
    let pool = render_canister_pool(&config.pool);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::SubnetConfig {
            canisters: #canisters,
            auto_create: #auto_create,
            subnet_index: #subnet_index,
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
    let topup_policy = render_option(config.topup_policy.as_ref(), render_topup_policy);
    let randomness = render_randomness_config(&config.randomness);
    let scaling = render_option(config.scaling.as_ref(), render_scaling_config);
    let sharding = render_option(config.sharding.as_ref(), render_sharding_config);
    let directory = render_option(config.directory.as_ref(), render_directory_config);
    let delegated_auth = render_delegated_auth_canister_config(&config.delegated_auth);
    let standards = render_standards_canister_config(&config.standards);

    quote! {
        ::canic::__internal::core::bootstrap::compiled::CanisterConfig {
            kind: #kind,
            initial_cycles: #initial_cycles,
            topup_policy: #topup_policy,
            randomness: #randomness,
            scaling: #scaling,
            sharding: #sharding,
            directory: #directory,
            delegated_auth: #delegated_auth,
            standards: #standards,
        }
    }
}

// Render the canister kind enum.
fn render_canister_kind(kind: CanisterKind) -> TokenStream {
    match kind {
        CanisterKind::Root => {
            quote!(::canic::__internal::core::bootstrap::compiled::CanisterKind::Root)
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
fn render_topup_policy(policy: &TopupPolicy) -> TokenStream {
    let threshold = render_cycles(policy.threshold.to_u128());
    let amount = render_cycles(policy.amount.to_u128());

    quote! {
        ::canic::__internal::core::bootstrap::compiled::TopupPolicy {
            threshold: #threshold,
            amount: #amount,
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
fn render_delegated_auth_canister_config(config: &DelegatedAuthCanisterConfig) -> TokenStream {
    let signer = config.signer;
    let verifier = config.verifier;

    quote! {
        ::canic::__internal::core::bootstrap::compiled::DelegatedAuthCanisterConfig {
            signer: #signer,
            verifier: #verifier,
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
