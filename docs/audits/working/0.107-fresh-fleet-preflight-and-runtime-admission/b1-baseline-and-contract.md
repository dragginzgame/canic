# Canic 0.107 B1 Baseline And Frozen Contract

Date: 2026-08-20

## 1. Source And Tool Identity

| Item | Exact identity |
| --- | --- |
| Direct predecessor | annotated `v0.105.0`, peeled commit `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811` |
| Predecessor parent | `bf6d39ad6d1a06e6c74e2f375475b1765875bec6` |
| Predecessor commit date | `2026-08-20T11:01:29+02:00` |
| Current branch/HEAD | `main` / `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811` |
| Predecessor `Cargo.lock` SHA-256 | `ce8705c5eee0274525f2bb24b73d12faea51a6ce5945a1ef849ca3c90b38ee66` |
| Captured working `Cargo.lock` SHA-256 | `60937280cdae1bdd633d23d7bacb0b6bc5607d9e392858b76760da7382ed8d35` |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)`, host `x86_64-unknown-linux-gnu`, LLVM 22.1.6 |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| `ic-query` | crates.io `0.40.1`, checksum `5aa6101fcf4a014042ee1ad48a8a34cdcc840b26f07660e6575f5364f481f12b` |
| Toko evidence | clean `main`, commit `bf14a5d3d89be4335d3da2601e8a60128fde04df`, 418 tracked files |

The current lockfile differs from the predecessor only for co-delivered
test-only work recorded by its owning evidence. `cargo tree --locked --offline
-p canic-host -i ic-query` resolves exactly `ic-query v0.40.1 -> canic-host
v0.105.0`. The 0.107 production files listed in `source-baseline.tsv` have no
diff from `v0.105.0`.

## 2. Downstream Traceability

The read-only Toko checkout contains no Canic dependency, no `CANIC-011`,
`CANIC-012` or `CANIC-013` identifier, and no current Canic integration
fixture. Its `backend/src/canisters/icu.toml` nevertheless supplies an exact
current sizing input: a compile-time `[whitelist]` containing 175 principals,
file SHA-256
`b398d8c2453d043a019ea8787ff9d90058ed082b5b8ab86326693206b6eb14a6`.
That list is context for the hard maximum, not Canic authority and not final
acceptance evidence.

The B7 downstream rerun therefore has a truthful external boundary: it must
use a newer read-only Toko source that exercises the three feedback items, or
record the absence of such a source as an exact downstream blocker. Canic must
not edit Toko to manufacture the result.

## 3. Current Planner And Install Inventory

### Direct plan leaf

The current direct leaf is incomplete:

- global environment forwarding admits `deploy check` and selected `deploy
  inspect` leaves, but not `deploy plan`;
- the leaf accepts `<fleet>`, `--app`, `--build-profile`, `--config`, `--json`
  and `--out`, plus its hidden environment input;
- it accepts no Fleet input, finalized release-build identity or expected plan
  digest;
- it builds `DeploymentPlanV1` from local config/artifact/catalog observations
  rather than the exact fresh-install authority; and
- its current `plan_id` is contextual text, not the canonical fresh-Fleet
  digest.

The maintained public B1 spelling is:

```text
canic --environment <name> deploy plan <fleet> \
  --app <app> \
  --fleet-input <PATH> \
  [--json] \
  [--out <PATH>] \
  [--profile <debug|fast|release>] \
  [--release-build <ID>]
```

Rules:

- `--environment` remains a top-level global and is forwarded once to the
  direct leaf through the existing hidden environment mechanism.
- `--fleet-input` is required and has the same path and schema meaning as
  install.
- `--profile` is the one common spelling for plan and install; the old
  plan-only `--build-profile` spelling is hard-cut without an alias.
- `--config` is removed from plan. `--app` selects the canonical
  `apps/<app>/canic.toml` authority, matching install.
- `--release-build` is optional and means the same exact finalized build as
  install. Absence selects the current workspace release source.
- `--json` controls stdout rendering only. `--out` delivers the already
  compiled JSON report with the existing create-new/no-follow safety and is
  excluded from plan identity.
- The plan leaf does not accept or forward the ICP executable selector because
  it performs no ICP command or IC effect. Canonical environment/network
  resolution still uses the selected ICP project configuration.
- No alias or compatibility spelling is retained.

### Install leaf

The maintained install spelling remains:

```text
canic --environment <name> [--icp <PATH>] install <app> <fleet> \
  --fleet-input <PATH> \
  [--expected-plan-digest <SHA256>] \
  [--profile <debug|fast|release>] \
  [--release-build <ID>]
```

`--expected-plan-digest` accepts exactly 64 lowercase hexadecimal characters.
It is optional; when present it must equal the freshly recomputed digest or
install fails before any build preparation, workspace mutation or IC update.

### Current ordering defect and accepted order

Current install calls `current_install_build_inputs` before resolving Fleet
input. For a workspace build that function calls
`plan_release_build_for_profile`, which allocates randomness and creates a
durable `.canic/release-builds/.../plan.cbor` record. The actual artifact build
then runs inside `prepare_install_deployment_truth`, and the current immutable
Fleet install plan is compiled only after artifact build and manifest emission.

The accepted 0.107 fresh-install order is:

```text
parse exact CLI identity
    -> resolve workspace/ICP project and canonical App config
    -> resolve canonical environment/network and reject disagreement
    -> load and validate Fleet-input bytes
    -> load exact release-source input without allocating new durable state
    -> collect/validate catalog evidence
    -> derive placement, admissions, role/root/pool counts and funding
    -> resolve operator principal/account and observe balance
    -> invoke the pure shared plan compiler
    -> reject blockers or expected-digest disagreement
    -> print digest and maximum debit
    -> only now allocate/bind a workspace release-build identity or reuse the named finalized build
    -> build/materialize artifacts
    -> persist install session/receipts bound to the plan digest
    -> perform IC updates
```

Invalid App/Fleet/environment/Fleet-input identity rejects before catalog or
balance work where deterministic ordering permits. Missing, stale,
inconsistent or insufficient catalog, placement, funding or balance evidence
is a blocker before durable build preparation. Planning stops after rendering
or explicit report delivery and never executes the final four steps.

## 4. Pure Plan Compiler

One host-owned synchronous function accepts one named input and returns one
`DeploymentPlanV1` or typed blockers. It performs no filesystem/environment
read, cache action, build, random allocation, clock read, IC call or mutation.
Loaders supply all bytes, identities, observations and times explicitly.

The compiler input is one private named structure containing:

- canonical App identity and canonical config digest;
- Fleet name and Fleet binding identity;
- requested environment and resolved canonical network;
- build profile;
- Fleet-input schema version, canonical content and digest;
- release source: either finalized release-build ID/plan hash/manifest digest,
  or the current workspace builder/package/lock/source-snapshot identity;
- sorted expected artifact role/package inventory;
- catalog network, assurance, source endpoints, cache disposition, collection
  time, Registry version, catalog digest and freshness decision;
- operator principal and exact funding account;
- sorted placement/admission decisions for every required role;
- expected role, Root, pool and total Canister counts;
- sorted per-category funding requirements and checked maximum debit;
- observed operator balance, source, observation time and sufficiency result;
- ordered typed blockers, warnings and assumptions; and
- `build_started`, `workspace_mutation_started` and `ic_mutation_started`, all
  required to be false at this boundary.

The workspace release-source identity is a loader-owned canonical digest of
the exact build-input snapshot admitted by the canonical builder. It includes
the builder/package version, `Cargo.lock` bytes, canonical App configuration
and the sorted package/source inputs for every expected artifact. It excludes
`.git`, `target`, `.canic`, report output and other generated artifact roots;
symlinks and non-regular inputs reject. The loader must pass the same snapshot
identity to install and verify it again immediately before building. A changed
input rejects and requires a new plan rather than silently preserving the old
digest.

Collections use canonical semantic ordering, not discovery order. Checked
arithmetic owns every count and debit. Unknown required evidence produces a
blocker, never an assumption that allows install.

## 5. Canonical Plan Digest

The product schema remains version 1. `DeploymentPlanV1` retains its existing
contextual `plan_id` for deployment-truth compatibility and adds a distinct
canonical `plan_digest`.

The digest is:

```text
SHA-256(
    b"canic-deployment-plan:v1\0"
    || compact canonical JSON of DeploymentPlanDigestInputV1
)
```

`DeploymentPlanDigestInputV1` is a private explicit projection of every
decision-bearing field listed in section 4. It has `deny_unknown_fields`,
contains no digest field and serializes struct fields in declared order. Maps
are projected to sorted vectors; set-like vectors are sorted and duplicate
free; integer quantities are JSON integers except cycle quantities, which are
canonical base-10 strings. Principals, IDs and hashes use their canonical text
forms. The published digest is 64 lowercase hexadecimal characters.

Excluded from the digest:

- `plan_id` contextual prose;
- filesystem paths after their canonical content/identity has been captured;
- text-rendering choices and diagnostic prose;
- the `--out` destination;
- elapsed durations; and
- timestamps that do not affect freshness, authority, balance or admission.

Included timestamps are exact integer Unix seconds and are included whenever
they affect a freshness or admission result. Any input or observation capable
of changing install admission, debit, target, release source or artifact
inventory is included.

Plan and install call the same compiler. Install stores the digest in its
session plus completion, rejection and recovery receipts. Every resumed or
retried install validates that digest before another effect. A changed live
observation intentionally yields a different digest.

## 6. Current Subnet-Catalog Error Boundary

The exact dependency is `ic-query 0.40.1`. It already provides useful typed
authority:

- stable host error code and category;
- typed catalog subjects for many validation failures;
- successful `CatalogAuthorityEvidence` with Registry version, digest,
  assurance, endpoints and cache disposition; and
- exact versions and digests for endpoint-agreement mismatches.

It does not provide the complete failure evidence claimed by 0.107:

1. `fetch_mainnet_subnet_catalog_async` learns `registry_version` before the
   subnet-list, routing-table and record reads, but later `RegistryFetchError`
   variants do not retain it.
2. `load_subnet_catalog_with_source_async` knows request network, assurance,
   source selection and which cache/refresh branch it attempted, but failures
   return only `SubnetCatalogHostError`.
3. successful cache disposition is typed, while a failed refresh/load cannot
   report bypassed, absent, rejected, refresh-attempted or post-refresh-load
   stage precisely.
4. retryability is only `Retryable | NotRetryable`; it cannot represent
   `Unknown(reason)`. Current deterministic classification marks every catalog
   validation error non-retryable even when the wrapper lacks enough context
   to justify that operational claim.

The smallest accepted upstream addition is a companion detailed load result;
existing simple methods and source errors may remain:

```text
SubnetCatalogLoadFailure {
    request: { network, source_endpoints, minimum_assurance },
    stage,
    registry_version: Option<u64>,
    cache_disposition,
    subject: Option<typed subject>,
    code,
    category,
    retry: Retryable | NotRetryable | Unknown { reason },
    source: SubnetCatalogHostError,
}
```

The stage/cache vocabulary must distinguish at least cache bypass, cache
absence, cache rejection, refresh attempt, refresh failure and post-refresh
load failure. Registry collection must attach the known version to failures
after `get_latest_version`. Subject remains typed (principal, subnet, routing
range, Registry key/record kind or field) rather than parsed from display text.

Canic B5 consumes only a committed or published upstream surface. It does not
fork the collector, parse error strings, infer missing Registry versions or
label unknown retryability transient. Canic adds its local no-effect facts:
`build_started = false`, `workspace_mutation_started = false` and
`ic_mutation_started = false`.

## 7. Runtime Whitelist Stable Contract

### Reachability and allocation

The capability exists on canonical managed non-root application roles only.
It is absent from Root, Fleet Coordinator, Wasm Store and standalone-local
specialized surfaces. It adds variants below the existing `canic_command` and
`canic_status` methods and adds no method identity.

Memory ID 61 is frozen as the next Canic-core allocation:

```text
StateAllocationKey::CoreRuntimeWhitelist
stable key: canic.core.runtime.whitelist.v1
memory ID: 61
owner: canic-core
schema version: 1
```

The 0.107 boundary is reinstall-only. There is one current decoder, no V2,
fallback, dual reader, compiled-seed merge or cross-release migration.

### Canonical record and hard bounds

```text
RuntimeWhitelistRecord {
    schema_version: 1,
    principals: sorted unique Vec<Principal>,
    revision: u64,
    membership_digest: [u8; 32],
    last_operation: Option<RuntimeWhitelistOperationRecord>,
}
```

`RuntimeWhitelistOperationRecord` retains the one last accepted operation ID,
canonical request hash and exact mutation response. The bounds are:

| Bound | Frozen value | Direct evidence |
| --- | ---: | --- |
| principals per Canister | 256 | Toko's 175-member input leaves 81 entries; maximum fixture remains far below stable/ingress ceilings |
| membership page | 128 | maximum nested status Candid is 4,072 bytes |
| retained operations | 1 | schema owns only the last accepted exact replay/result |
| stable record bytes | 32 KiB | maximum CBOR fixture is 8,417 bytes |
| update ingress | existing 16 KiB | one mutation request is 101 bytes |

Capacity excess, encoding excess and checked-arithmetic failure reject without
mutation. No audit history, unbounded scan or second membership index is
stable. A bounded heap index may be rebuilt synchronously from the canonical
sorted record but is never authority.

### Canonical membership and request hashes

Principals sort by canonical principal bytes. The membership digest is:

```text
SHA-256(
    b"canic.runtime-whitelist.membership.v1\0"
    || member_count as u32 big-endian
    || for each member: byte_length as u8 || principal bytes
)
```

The accepted-operation request hash is:

```text
SHA-256(
    b"canic.runtime-whitelist.operation.v1\0"
    || action byte (0 = add, 1 = remove)
    || principal byte_length as u8 || principal bytes
    || expected_revision as u64 big-endian
)
```

The operation ID is the replay key and is not duplicated inside its request
hash. All-zero operation IDs reject.

### Bootstrap and restore

Fresh synchronous bootstrap validates and parses the compiled seed, sorts by
principal bytes, collapses exact duplicates, enforces 256, sets revision zero,
computes the membership digest and stores no operation. Missing configuration
creates an empty record. This completes before the 0.104 lifecycle participant
and deferred hooks.

Same-release restoration reads only memory ID 61, validates schema, order,
uniqueness, capacity, digest and retained operation consistency, rebuilds any
heap index and fails closed on corruption. It never reseeds from current
compiled configuration.

### Public role variants

The exact managed-role additions are:

```text
CanisterCommand::RuntimeWhitelist(RuntimeWhitelistCommand)
RuntimeWhitelistCommand::Add(RuntimeWhitelistMutationRequest)
RuntimeWhitelistCommand::Remove(RuntimeWhitelistMutationRequest)
CanisterCommandResponse::RuntimeWhitelist(RuntimeWhitelistMutationResponse)

CanisterStatusRequest::RuntimeWhitelist(PageRequest)
CanisterStatusResponse::RuntimeWhitelist(RuntimeWhitelistStatusResponse)
```

```text
RuntimeWhitelistMutationRequest {
    principal: Principal,
    expected_revision: u64,
    operation_id: [u8; 32],
}

RuntimeWhitelistMutationResponse {
    outcome: Added | AlreadyPresent | Removed | AlreadyAbsent,
    principal: Principal,
    revision: u64,
    membership_digest: [u8; 32],
}

RuntimeWhitelistStatusResponse {
    principals: Page<Principal>,
    revision: u64,
    membership_digest: [u8; 32],
    maximum_principals: u16,
}
```

The status page uses the existing `PageRequest { offset, limit }`. It clamps a
limit above 128 to 128, returns an empty page for a zero limit or an offset at
or beyond the total, and uses saturating/checked conversions so an
unrepresentable offset also returns no entries. It reports only canonical
membership, revision, digest and the hard maximum. It exposes no operation ID,
request hash or replay record.

### Administration and mutation order

Administration is authorized by the actual transport caller being either the
current controller of the managed Canister or its exact stable Root binding.
The endpoint reads `msg_caller` once. It checks controller authority first; a
controller succeeds without requiring the Root binding to load. Otherwise it
loads the exact Root binding: equal Root succeeds, unavailable binding returns
compact authority-unavailable, and any other caller returns compact
unauthorized. Membership never grants administration.

Authorization completes before membership, revision or operation state is
read. The accepted mutation order is:

1. reject zero operation ID;
2. derive the canonical request hash;
3. if the retained operation ID and hash both match, return its exact result;
4. if the ID matches but the hash differs, reject operation conflict;
5. require `expected_revision == current revision`;
6. enforce capacity for a new member;
7. derive the idempotent or effective result;
8. atomically commit members, revision, membership digest and retained result.

`Add` of a present member and `Remove` of an absent member succeed without
advancing revision. They still become the retained accepted operation.
Effective add/remove advances revision exactly once. Rejected operations do
not mutate state or replace the retained result. A retry older than the one
retained operation cannot be applied as a fresh effect at an old revision; it
fails revision or operation admission.

`caller::is_whitelisted()` keeps its public predicate meaning but reads only
the stable runtime record through deterministic ops. It performs no cleanup or
mutation. Compiled config is seed input only and ceases to be an access
authority after bootstrap. Runtime whitelist state and 0.105 application
sessions remain separate authorities.

## 8. B1 Completion And Gates

| Required B1 output | Result |
| --- | --- |
| Toko traceability | Complete, exact read-only commit/hash/count and final-rerun blocker recorded |
| current CLI/install/compiler inventory | Complete, including preflight/build-persistence ordering defect |
| current whitelist inventory | Complete, config-only authority and exact managed role boundary recorded |
| upstream error inventory | Complete for exact `ic-query 0.40.1`; missing fields and smallest additive API frozen |
| exact bounds | Complete and executable test-only measurements pass |
| variant and option spelling | Complete |
| digest contract | Complete |
| production/stable mutation | Not begun; prohibited until B1 acceptance |

No 0.106 B2 effect was performed. No sibling repository was modified. B2-B7
remain pending this B1's explicit maintainer acceptance.
