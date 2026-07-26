# Canic Configuration

This guide documents the canonical shape of `canic.toml`, the configuration file consumed by Canic build scripts and runtime helpers.

At a high level the file describes:

- App identity and package-backed roles (`app`, `roles`).
- Global settings (`controllers`, `standards`, `app`, `auth`, `log`).
- Flat Component topology under `component_specs.<name>`.
- One Component role and its direct children per Component Spec.
- Per-Component and per-child instance ceilings, cycles policy, and optional
  Component-owned scaling, sharding, and keyed binding pools.
- The implicit Fleet Subnet Root-local wasm-store behavior used by
  chunk-store-backed installs.

All fields are validated when `canic::build!` runs, so configuration drift fails
fast at compile time. Every canister crate also declares the App and role it
implements in `Cargo.toml`:

```toml
[package.metadata.canic]
app = "demo"
role = "app"
```

That package App must match `[app].name`, and the package role must
exist in `canic.toml`. `role = "root"` selects the root lifecycle and root
endpoint bundle; every other role selects the ordinary non-root lifecycle and
endpoint bundle.

---

## Runtime Config + Env Lifecycle

Canic treats config/env identity as startup invariants. Missing env data is a fatal error.

- Build time: `CANIC_CONFIG_PATH` is embedded into the Wasm and `ICP_ENVIRONMENT` is baked in (`local` or `ic`), defaulting to `local` when unset.
- Init/post-upgrade: generated lifecycle code loads the embedded TOML and parsed config model; `ConfigOps::current_*` is infallible.
- Root env: fresh root installation sets base fields from
  `CurrentRootInstallIdentity` without a registry lookup.
  - The Fleet Subnet Root sits outside every Component Spec.
  - One root may manage several admitted Component Specs.
  - `fleet_root_pid` identifies that Fleet's current root authority.
- Non-root env: children must receive a complete `EnvBootstrapArgs` in `CanisterInitPayload` from root.
  - The current transitional selector names the exact owning Component Spec;
    the frozen protected `ComponentBinding` replaces it when root-local
    allocation can supply a real concrete Component identity.
  - Missing env fields always trap (no local fallback).

---

## Global Keys

### `[roles.<role>]`

Required package declaration for every Component or direct child attached
through `component_specs`. The `root` declaration is also required whenever
Component topology is present.

- `kind = "root" | "canister"` – package role class. Only `[roles.root]` may
  use `root`.
- `package: string` – non-empty path to the role package, relative to this
  `canic.toml`.

Role declarations own package identity. The matching
`component_specs.<name>` or
`component_specs.<name>.children.<role>` entry owns permitted topology and
Component-local policy.

### `controllers = ["aaaaa-aa", ...]`

Optional list of controller principals appended to every provisioned canister.

### `[app]`

Required source identity and initial application mode.

- `name: string` – required immutable App identity used in role and build
  evidence.
- `init_mode = "enabled" | "readonly" | "disabled"` – default `enabled`.

### `[app.whitelist]`

Optional allow-list for privileged operations.

- `principals = ["aaaaa-aa", ...]` – principal text strings authorised for whitelist checks.
  - If `[app.whitelist]` or `principals` is omitted, whitelist checks deny all
    principals. An empty table is also deny-all.

### `[log]`

Configure log retention for every canister.

- `max_entries: u64` – ring buffer cap on stored log entries (default `10000`).
- `max_entries` must be `<= 100000` (larger values are rejected at config validation).
- `max_entry_bytes: u32` – maximum message size in bytes per entry; oversized entries are truncated with a `...[truncated]` suffix (default `16384`).
- `max_age_secs: u64` – optional maximum age; entries older than this (in seconds) are purged (default `null` = no age limit).

### `[auth.delegated_tokens]`

Root/issuer delegated token authentication
(cert -> chain-key root proof -> issuer proof -> token).

- `enabled: bool` – enable delegated token auth (default `false`).
- `root_canister_id: string` – optional root canister trust anchor. If omitted, runtime verification uses the initialized Canic root env.
- `ic_root_public_key_raw_hex: string` – optional raw 96-byte IC BLS root public key encoded as hex. If omitted, runtime verification uses the IC/test root-key provider for issuer canister-signature proof verification.
- `build_network: "ic" | "local"` – network class bound into delegated-auth proofs and verifier policy (default `"ic"`).
- `max_ttl_secs: u64` – optional upper bound on delegated cert/token/session TTL in seconds (default `null` = runtime default cap; must be > 0 when set).

When delegated-token verification is enabled on a non-root endpoint canister,
startup requires issuer canister-signature verification support, an effective
root canister id, the raw IC root public key for the configured network, and a
complete chain-key root proof policy. Verification uses that policy directly.

#### `[auth.delegated_tokens.chain_key_root_proof]`

Trust anchor for `RootProof::IcChainKeyBatchSignatureV1`.

These fields are required when delegated tokens are enabled:

- `key_id: string` – IC chain-key ECDSA key id, such as `"key_1"`.
- `derivation_path_hash_hex: string` – canonical 32-byte hash of the derivation path, encoded as hex.
- `derivation_path_hex: [string, ...]` – derivation path components encoded as hex strings.
- `public_key_hex: string` – SEC1 secp256k1 public key for the configured root canister, key id, and derivation path.
- `key_version: u64` – configured signing key version expected in root proof headers.
- `min_accepted_key_version: u64` – verifier floor for accepted key versions.
- `min_accepted_proof_epoch: u64` – verifier floor for root proof epochs. For the byte-free V1 hard cut, deployments must set this strictly above their highest previously accepted proof epoch before issuing new material.
- `min_accepted_registry_epoch: u64` – verifier floor for delegated-auth registry epochs. For the byte-free V1 hard cut, deployments must set this strictly above their highest previously accepted registry epoch before issuing new material.
- `valid_from_ns: u64` – first accepted proof-policy time in nanoseconds.
- `accept_until_ns: u64` – last accepted proof-policy time in nanoseconds; must be greater than `valid_from_ns`.
- `max_revocation_latency_ns: u64` – maximum accepted policy revocation lag; must be greater than zero.
- `allow_test_key: bool` – allow `test_key_1` for `build_network = "local"` (default `false`). The `ic` build network always rejects `test_key_1`.

### `[auth.role_attestation]`

Root canister-signature role-attestation settings.

- `max_ttl_secs: u64` – maximum role-attestation lifetime in seconds (default `900`, must be > 0).
- `min_accepted_epoch_by_role.<role>: u64` – optional per-role epoch floor for rejecting older attestations.

### `[standards]`

Feature toggles tied to public standards.

- `icrc21: bool` – enable the ICRC-21 consent endpoint (default `false`).

---

## Component Specs

Declare each permitted flat topology under `[component_specs.<name>]`.
The name is a bounded `ComponentSpecId` and has no physical placement or
runtime-parent meaning.

Each Component Spec declares exactly one Component directly below a Fleet
Subnet Root:

```toml
[component_specs.users]
component_role = "user_hub"
maximum_instances = 10
```

- `component_role` – required role of the Component Canister.
- `maximum_instances` – required positive Fleet-wide ceiling for concrete
  instances of this Spec.
- Component policy fields (`initial_cycles`, `topup`, `cycles_funding`,
  `scaling`, `sharding`, `binding`, `auth`, `standards`, `diagnostics`, and
  `metrics`) configure that Component.
- `limits` compiles finite aggregate child, Registry, and cycles-funding
  quotas into every concrete Component binding.
- `children.<role>` – optional direct Component Child tables.

The sum of all Component Spec `maximum_instances` values must not exceed
4,096. A Component role occurs in exactly one Spec and cannot also be a child.
A direct child role may be reused by several Specs because the one global
`[roles.<role>]` declaration fixes its package/artifact identity; ownership
still resolves through the exact Spec and concrete Component instance.

Component Specs cannot include one another. A child cannot declare another
Component, child table, or child-producing pool. `root`, `service`, and
`component` are structural roles, not accepted child `kind` values.

### Implicit `wasm_store`

Every Fleet Subnet Root has one mandatory root-local `wasm_store`. It is
bootstrapped implicitly, sits outside Component topology, and must not be
declared in `canic.toml`.

Fixed `0.18` preset:

- canister role: `wasm_store`
- kind: implicit `singleton`
- `max_store_bytes = 40000000`
- `headroom_bytes = 4000000`
- `max_templates = none`
- `max_template_versions_per_template = none`

Rules:

- do not define a `wasm_store` role as a Component or child
- ordinary deployable roles install from published chunked manifests in this store
- inline install is reserved for bootstrapping `wasm_store` itself

### `[component_specs.<name>.children.<role>]`

Each child table configures one direct non-Component Canister owned by each
concrete Component instance. The role is derived from the table key; do not
declare `role`, `type`, `owner_component`, another Component, or another
`children` table.

- `kind = "singleton" | "replica" | "shard" | "instance"` – required
  lifecycle class for the direct child.
- `maximum_instances` – required positive ceiling per owning Component
  instance. A `singleton` must use exactly `1`.
- `initial_cycles = "5T"` – cycles to allocate when provisioning (defaults to 5T).
- `topup.threshold = "10T"` – minimum cycles before requesting a top-up
  (default `10T` when the `topup` table is present).
- `topup.amount = "5T"` – cycles to request when topping up (default `5T`
  when the `topup` table is present; it cannot exceed half the threshold).
  Omit `topup` entirely to disable auto top-ups.

Cycle amount fields use exact decimal `K`, `M`, `B`, `T`, or `Q` shorthand.
They must resolve to a whole number of cycles within `u128`; Canic does not
round, truncate, or saturate them.
- `scaling` – optional table that defines stateless replica pools.
- `sharding` – optional table that defines stateful shard pools.
- `auth.delegated_token_issuer = true` – mark this role as a delegated-token issuer; Canic requires local issuer canister-signature support for token issuance.
- `auth.delegated_token_verifier = true` – mark this role as a delegated-token
  verifier; the role contract requires the matching verifier feature and the
  global delegated-token trust policy.
- `auth.role_attestation_cache = true` – start the role-attestation key cache for canisters that verify root-signed role attestations. Delegated-token endpoint verification itself is driven by endpoint guards and `auth.delegated_tokens`, not this flag.
- `standards.icrc21 = true` – enable the canister-local ICRC-21 endpoint. This
  is separate from the global `[standards]` setting.
- `diagnostics.memory_ledger = true` – opt this role into the controller-only `canic_memory_ledger` recovery diagnostic. The endpoint is omitted by default to keep the shared Candid/runtime surface smaller.
- `metrics.profile = "leaf" | "hub" | "storage" | "root" | "full"` – override
  the role-derived metrics profile.

The same cycles, auth, standards, diagnostics, and metrics fields also apply
directly to the Component Spec table. Only the Component itself may own
`scaling`, `sharding`, or `binding` pools, and each pool target must be a
direct child declared by that same Spec.

#### Component aggregate limits

Every Component has finite aggregate limits in addition to each child's own
policy:

```toml
[component_specs.<name>.limits]
maximum_children = 4096
maximum_registry_bytes = 2097152

[component_specs.<name>.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "1000T"
```

- `maximum_children` bounds the total direct-child instances owned by one
  concrete Component (default `4096`). It may be lower than the sum of
  independent per-role ceilings to express shared aggregate capacity, but
  must be positive when the Spec declares children.
- `maximum_registry_bytes` bounds that Component's canonical Registry
  (default `2097152`, and must be positive).
- `cycles_funding.window_secs` and `maximum_cycles` form a positive aggregate
  budget above per-child request, cumulative, and cooldown limits (defaults
  `3600` and `"1000T"`).

One Component Spec may declare at most 256 distinct direct-child roles. The
complete canonical Fleet Component Topology is bounded to 2 MiB.

#### Parent cycles funding

`cycles_funding` limits cycle requests made by this role to its parent. It is
always active as policy; omitted values use finite defaults.

- `max_per_request = "5T"` – maximum granted by one request.
- `max_per_child = "100T"` – cumulative parent budget for one child.
- `cooldown_secs = 60` – minimum time between grants for the child.

`max_per_request` must not exceed `max_per_child`, and all three values must be
positive.

The `wasm_store` role is reserved and implicit.
Do not add it under `component_specs.*`.

#### Scaling Pools

Scaling pools model interchangeable replicas with simple bounds on how many to keep alive.

```toml
[component_specs.<name>.scaling.pools.<pool>]
canister_role = "replica_role"
policy.initial_workers = 1
policy.min_workers = 2
policy.max_workers = 16
```

Fields:

- `canister_role` – direct child role in the same Component Spec with
  `kind = "replica"`.
- `policy.initial_workers` – workers to create during canister startup warmup (default `1`).
- `policy.min_workers` – minimum workers to keep alive (default `1`).
- `policy.max_workers` – positive hard cap on workers (default `32`), no
  greater than that child's `maximum_instances`.

#### Placement Binding Pools

Placement binding pools place keyed stateful instances.

```toml
[component_specs.<name>.binding.pools.<pool>]
canister_role = "instance_role"
key_name = "project"
```

- `canister_role` – direct child role in the same Component Spec with
  `kind = "instance"`.
- `key_name` – non-empty logical key name used by keyed placement admission.

#### Sharding Pools

Sharding pools manage stateful shards that own capacity-bounded partitions.

```toml
[component_specs.<name>.sharding.pools.<pool>]
canister_role = "shard_role"
policy.capacity = 1000
policy.max_shards = 64
```

Fields:

- `canister_role` – direct child role in the same Component Spec with
  `kind = "shard"`.
- `policy.capacity` – per-shard capacity (default `1000`, must be > 0).
- `policy.initial_shards` – shards created by initial warmup (default `1`; may
  be `0`, but cannot exceed `max_shards`).
- `policy.max_shards` – maximum shard count (default `4`, must be positive and
  no greater than that child's `maximum_instances`).

---

## Example

```toml
# CANIC_CONFIG_EXAMPLE_START
controllers = ["aaaaa-aa"]

[app]
name = "example"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale]
kind = "canister"
package = "scale"

[auth.delegated_tokens]
enabled = false
# root_canister_id = "..."
# ic_root_public_key_raw_hex = "..."
build_network = "local"
#
# [auth.delegated_tokens.chain_key_root_proof]
# key_id = "key_1"
# derivation_path_hash_hex = "..."
# derivation_path_hex = ["63616e6963", "64656c65676174696f6e"]
# public_key_hex = "..."
# key_version = 1
# min_accepted_key_version = 1
# min_accepted_proof_epoch = 2
# min_accepted_registry_epoch = 2
# valid_from_ns = 1
# accept_until_ns = 4102444800000000000
# max_revocation_latency_ns = 60000000000
# allow_test_key = true

[standards]
icrc21 = true

[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_specs.users]
component_role = "user_hub"
maximum_instances = 1
topup.threshold = "10T"
topup.amount = "5T"

[component_specs.users.limits]
maximum_children = 1000
maximum_registry_bytes = 2097152

[component_specs.users.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 4

[component_specs.users.children.user_shard]
kind = "shard"
maximum_instances = 4

[component_specs.scaling]
component_role = "scale_hub"
maximum_instances = 1
topup.threshold = "10T"
topup.amount = "5T"

[component_specs.scaling.scaling.pools.scales]
canister_role = "scale"
policy.initial_workers = 1
policy.min_workers = 2
policy.max_workers = 32

[component_specs.scaling.children.scale]
kind = "replica"
maximum_instances = 32
# CANIC_CONFIG_EXAMPLE_END
```

This example defines three flat Component Specs, enables ICRC-21, and
configures one direct shard child under `user_hub` plus one direct replica
child under `scale_hub`. Each occupied Fleet/Subnet root gets one implicit
`wasm_store`; physical Subnet placement and root-local Component admissions
are separate deployment input.

---

## Runtime Release Metadata vs Static Config

`canic.toml` no longer defines wasm-store topology or capacity policy.
It does not enumerate every published template release.

Static config owns:

- user-defined canister roles and policies
- flat Component Specs and bounded Component/child ceilings
- Component roles that a Fleet Subnet Root may create from admitted Specs

Root-authoritative runtime state owns:

- approved manifest records
- logical template release metadata (`template_id`, `version`, `role`, `payload_hash`, `payload_size_bytes`, `chunking_mode`)
- publication binding / store placement state used for install resolution

Template stores own:

- chunk sets
- deterministic chunk metadata
- template-version storage data

This separation is deliberate:

- config defines the user-managed flat Component topology only
- root-approved manifest/runtime state defines what is installable and which implicit store is active
- wasm stores hold the bytes and deterministic chunk-set metadata only
