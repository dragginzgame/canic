# Canic Configuration

This guide documents the canonical shape of `canic.toml`, the configuration file consumed by Canic build scripts and runtime helpers.

At a high level the file describes:

- Fleet identity and package-backed roles (`fleet`, `roles`).
- Global cluster settings (`controllers`, `app_index`, `standards`, `app`, `auth`, `log`).
- Subnet-specific behaviour under `subnets.<name>` (including per-subnet pool settings).
- Per-canister policies inside each subnet, with optional scaling, sharding,
  and directory pools.
- The implicit wasm-store behavior used by chunk-store-backed installs.

All fields are validated when `canic::build!` runs, so configuration drift fails
fast at compile time. Every canister crate also declares the fleet and role it
implements in `Cargo.toml`:

```toml
[package.metadata.canic]
fleet = "demo"
role = "app"
```

That package fleet must match `[fleet] name = "..."`, and the package role must
exist in `canic.toml`. `role = "root"` selects the root lifecycle and root
endpoint bundle; every other role selects the ordinary non-root lifecycle and
endpoint bundle.

---

## Runtime Config + Env Lifecycle

Canic treats config/env identity as startup invariants. Missing env data is a fatal error.

- Build time: `CANIC_CONFIG_PATH` is embedded into the Wasm and `ICP_ENVIRONMENT` is baked in (`local` or `ic`), defaulting to `local` when unset.
- Init/post-upgrade: generated lifecycle code loads the embedded TOML and parsed config model; `ConfigOps::current_*` is infallible.
- Root env: `root_init(identity)` sets base env fields directly from `SubnetIdentity` (no registry lookup).
  - `Prime` means root == subnet == prime root.
  - `Standard` carries the `subnet_type` and `prime_root_pid` from the prime subnet.
  - `Manual` is a test/support override that pins the subnet principal.
- Non-root env: children must receive a complete `EnvBootstrapArgs` in `CanisterInitPayload` from root.
  - Missing env fields always trap (no local fallback).

---

## Global Keys

### `[fleet]`

Required operator-facing identity for the configured fleet.

- `name: string` – required; used in role evidence and host install-state paths.

### `[roles.<role>]`

Required package declaration for every role attached through `subnets`. The
`root` declaration is also required whenever topology is present.

- `kind = "root" | "canister"` – package role class. Only `[roles.root]` may
  use `root`.
- `package: string` – non-empty path to the role package, relative to this
  `canic.toml`.

Role declarations own package identity. The matching
`subnets.<name>.canisters.<role>` entry owns topology and placement policy.

### `controllers = ["aaaaa-aa", ...]`

Optional list of controller principals appended to every provisioned canister.

### `app_index = ["role_a", "role_b", ...]`

Global set of canister roles that should appear in the prime root directory export. Every entry must also exist under `subnets.prime.canisters` and have `kind = "service"`.

### `[app]`

Initial application mode applied at canister install.

- `init_mode = "enabled" | "readonly" | "disabled"` – default `enabled`.

### `[app.whitelist]`

Optional allow-list for privileged operations.

- `principals = ["aaaaa-aa", ...]` – principal text strings authorised for whitelist checks.
  - If `[app.whitelist]` or `principals` is omitted, whitelist checks deny all
    principals. An empty table is also deny-all.

### `[subnets.<name>.pool]`

Controls the warm canister pool for a subnet.

- `minimum_size: u8` – minimum number of spare canisters to keep on hand (default `0` when the table is omitted; required when the table is present).
- `import.initial: u16` – number of canisters to import immediately before queuing the rest (defaults to `minimum_size`).
- `import.local = ["aaaaa-aa", ...]` – canister IDs to import when built with `ICP_ENVIRONMENT=local` (also used when unset).
- `import.ic = ["aaaaa-aa", ...]` – canister IDs to import when built with `ICP_ENVIRONMENT=ic`.
  Import is destructive (controllers reset, code uninstalled); failures are logged and skipped.
If `pool.import.initial` is `0` and the subnet declares service roles, root
bootstrap may create new service canisters before queued imports are ready.

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
- `root_proof_mode: "chain_key_batch"` – required active delegated root proof mode. Other values are rejected.
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
- `min_accepted_proof_epoch: u64` – verifier floor for root proof epochs.
- `min_accepted_registry_epoch: u64` – verifier floor for delegated-auth registry epochs.
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
- `icrc103: bool` – include ICRC-103 metadata (default `false`).

---

## Subnets

Declare each subnet under `[subnets.<name>]`. The name is an arbitrary identifier; `prime` is reserved for the main orchestrator subnet and should always be present.
Canisters are declared as nested subnet canister tables such as
`[subnets.prime.canisters.app]`; Canic does not use a flat `[[canisters]]`
array.

### `[subnets.<name>]`

- `canisters.*` – nested tables describing per-role policies (see below).

Configured `kind = "service"` roles are derived as the stable subnet services.
Root ensures those service roles exist during bootstrap and exposes them through
`canic_subnet_index()`. Singletons, shards, replicas, and instances are created
through their explicit placement flows instead.

### Implicit `wasm_store`

Every subnet always has one mandatory same-subnet `wasm_store`.
It is bootstrapped implicitly and must not be declared in `canic.toml`.

Fixed `0.18` preset:

- canister role: `wasm_store`
- kind: implicit `singleton`
- `max_store_bytes = 40000000`
- `headroom_bytes = 4000000`
- `max_templates = none`
- `max_template_versions_per_template = none`

Rules:

- do not define `subnets.<name>.wasm_stores.*`
- do not define `subnets.<name>.canisters.wasm_store`
- ordinary deployable roles install from published chunked manifests in this store
- inline install is reserved for bootstrapping `wasm_store` itself

### `[subnets.<name>.canisters.<role>]`

Each child table configures a logical canister role within the subnet. The role is derived
from the table key (`subnets.<name>.canisters.<role>`); do not declare `role`, `type`, or
`sharding.role` fields.

- `kind = "root" | "service" | "singleton" | "replica" | "shard" | "instance"` – required; declares how this role attaches in the topology.
  - `root` cannot define placement pools, canister-local authentication roles,
    or canister-local standards.
  - `root` must be unique across all subnets.
  - `subnets.prime.canisters.root` must exist and set `kind = "root"`.
  - `service` is root-created, appears in the subnet index, and may own
    scaling, sharding, or directory pools.
  - `singleton`, `replica`, `shard`, and `instance` cannot own placement pools.
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

#### Parent cycles funding

`cycles_funding` limits cycle requests made by this role to its parent. It is
always active as policy; omitted values use finite defaults.

- `max_per_request = "5T"` – maximum granted by one request.
- `max_per_child = "100T"` – cumulative parent budget for one child.
- `cooldown_secs = 60` – minimum time between grants for the child.

`max_per_request` must not exceed `max_per_child`, and all three values must be
positive.

#### Manual root ICP refill

Only the root role may define `icp_refill`. It enables an operator-triggered
conversion of ICP held by root into cycles. It is manual and has no timer or
automatic threshold.

- `max_refill_e8s_per_call: u64` – required positive per-call spending cap.
- `min_xdr_permyriad_per_icp: u64` – optional positive minimum conversion-rate
  gate.
- `ledger_canister_id` and `cmc_canister_id` – optional system-canister
  overrides for local/test environments.
- `allow_ic_system_canister_overrides: bool` – required opt-in before those
  overrides may be used on the IC (default `false`).

The `wasm_store` role is reserved and implicit.
Do not add it under `canisters.*`.

#### Scaling Pools

Scaling pools model interchangeable replicas with simple bounds on how many to keep alive.

```toml
[subnets.<name>.canisters.<role>.scaling.pools.<pool>]
canister_role = "replica_role"
policy.initial_workers = 1
policy.min_workers = 2
policy.max_workers = 16
```

Fields:

- `canister_role` – canister role that represents replicas in this pool (must exist in the same subnet).
- `policy.initial_workers` – workers to create during canister startup warmup (default `1`).
- `policy.min_workers` – minimum workers to keep alive (default `1`).
- `policy.max_workers` – hard cap on workers (default `32`, set to `0` for no max).

#### Directory Pools

Directory pools place keyed stateful instances.

```toml
[subnets.<name>.canisters.<role>.directory.pools.<pool>]
canister_role = "instance_role"
key_name = "project"
```

- `canister_role` – same-subnet role implementing the instance; it must have
  `kind = "instance"`.
- `key_name` – non-empty logical key name used by directory admission.

#### Sharding Pools

Sharding pools manage stateful shards that own capacity-bounded partitions.

```toml
[subnets.<name>.canisters.<role>.sharding.pools.<pool>]
canister_role = "shard_role"
policy.capacity = 1000
policy.max_shards = 64
```

Fields:

- `canister_role` – canister role that implements the shard (must exist in the same subnet).
- `policy.capacity` – per-shard capacity (default `1000`, must be > 0).
- `policy.initial_shards` – shards created by initial warmup (default `1`; may
  be `0`, but cannot exceed `max_shards`).
- `policy.max_shards` – maximum shard count (default `4`, must be > 0).

---

## Example

```toml
# CANIC_CONFIG_EXAMPLE_START
controllers = ["aaaaa-aa"]
app_index = ["user_hub", "scale_hub"]

[fleet]
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

[roles.minimal]
kind = "canister"
package = "minimal"

[auth.delegated_tokens]
enabled = false
# root_canister_id = "..."
# ic_root_public_key_raw_hex = "..."
build_network = "local"
# root_proof_mode = "chain_key_batch"
#
# [auth.delegated_tokens.chain_key_root_proof]
# key_id = "key_1"
# derivation_path_hash_hex = "..."
# derivation_path_hex = ["63616e6963", "64656c65676174696f6e"]
# public_key_hex = "..."
# key_version = 1
# min_accepted_key_version = 1
# min_accepted_proof_epoch = 1
# min_accepted_registry_epoch = 1
# valid_from_ns = 1
# accept_until_ns = 4102444800000000000
# max_revocation_latency_ns = 60000000000
# allow_test_key = true

[standards]
icrc21 = true

[subnets.prime]
pool.minimum_size = 3
pool.import.initial = 3
pool.import.local = ["aaaaa-aa"]
pool.import.ic = ["aaaaa-aa"]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"

[subnets.prime.canisters.user_hub]
kind = "service"
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 4

[subnets.prime.canisters.scale_hub]
kind = "service"
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale"
policy.initial_workers = 1
policy.min_workers = 2

[subnets.prime.canisters.scale]
kind = "replica"

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.general]

[subnets.general.canisters.minimal]
kind = "replica"
initial_cycles = "3T"
# CANIC_CONFIG_EXAMPLE_END
```

This example defines two subnets (`prime` and `general`), enables the pool, enables ICRC-21, and configures sharding on `user_hub` plus scaling on `scale_hub`. Each subnet also gets one implicit `wasm_store` automatically.

---

## Runtime Release Metadata vs Static Config

`canic.toml` no longer defines wasm-store topology or capacity policy.
It does not enumerate every published template release.

Static config owns:

- user-defined canister roles and policies
- configured service roles that root bootstraps and exposes through the subnet index
- the explicit app index exported by the prime root

Root-authoritative runtime state owns:

- approved manifest records
- logical template release metadata (`template_id`, `version`, `role`, `payload_hash`, `payload_size_bytes`, `chunking_mode`)
- publication binding / store placement state used for install resolution

Template stores own:

- chunk sets
- deterministic chunk metadata
- template-version storage data

This separation is deliberate:

- config defines the user-managed topology only
- root-approved manifest/runtime state defines what is installable and which implicit store is active
- wasm stores hold the bytes and deterministic chunk-set metadata only
