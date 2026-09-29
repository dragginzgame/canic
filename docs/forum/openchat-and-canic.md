# What OpenChat can teach Canic about operating large canister fleets

OpenChat is a strong reference application for Canic's development. It combines many of the problems a canister framework needs to solve: subnet expansion, dynamic allocation, cycle funding, background work, governance, and releases across a large persistent application.

After examining its implementation, my conclusion is that **Canic could eventually remove substantial fleet-management work from OpenChat, but I would not recommend switching OpenChat's existing production fleet to Canic today.**

The largest obstacle is fundamental: OpenChat preserves application state through rolling upgrades, while Canic currently mandates reinstall-only release transitions. That prevents a responsible production replacement regardless of how closely the provisioning APIs match.

This post explores both sides: where Canic could help OpenChat, and what Canic would need to improve to support an application of this kind.

The assessment uses OpenChat commit [`2611b2e5`](https://github.com/open-chat-labs/open-chat/tree/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b), reviewed on September 29, 2026, and the Canic development checkout at that time. The Canic review includes pending `0.110.47` work; development findings should not all be read as published capabilities. This is a source-level assessment, not a live-fleet census, security audit, or measured performance comparison. It does not imply an adoption commitment from OpenChat.

**OpenChat already has substantial fleet infrastructure.**

OpenChat is far beyond an application that simply needs a convenient wrapper around `create_canister`:

| Area | What OpenChat implements | Implication for Canic |
| --- | --- | --- |
| Subnet expansion | A governance-controlled sequence creates a local index, sets controllers, and registers it with funding and application services. | Canic must coordinate activation beyond creating canisters. |
| Provisioning | Local indexes maintain spare canister pools and create/install children. | Ready capacity and allocation recovery are directly relevant. |
| User placement | Dedicated user canisters coexist with shared `multi_user` canisters. | Logical users and physical canisters must remain distinct. |
| Releases | Hash-bound Wasm distribution, upgrade queues, filters, concurrency controls, and progress tracking. | A generic install loop would be a regression. |
| Background work | Destination-grouped batches, retries, idempotency envelopes, and reconstructed timers. | Typed calls and timers alone cannot replace the existing machinery. |
| Product services | Identity, chat, communities, media, notifications, governance, and financial integrations. | Most application semantics should remain OpenChat-owned. |

The relevant implementations include the [subnet-expansion workflow](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/registry/impl/src/timer_job_types.rs), [child creation](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/local_user_index/impl/src/updates/c2c_create_multi_user_canister.rs), [placement selection](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/local_user_index/impl/src/model/local_multi_user_canister_map.rs), and [grouped queues](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/libraries/timer_job_queues/src/grouped_timer_job_queue.rs).

The shared-user work is especially significant. OpenChat's older architecture document describes one canister per user, but current source also implements shared placement and [migration from dedicated canisters](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/local_user_index/impl/src/jobs/start_user_migrations.rs). Canic should be evaluated against that current architecture.

**Where Canic could offer the most value**

The strongest opportunity is reducing the maintenance burden of provisioning and cycle management. OpenChat maintains pool replenishment, create/install handling, controller checks, reserves, top-up throttling, and allocation bookkeeping. Canic could consolidate those mechanics into a maintained implementation with explicit operation identity, durable intent, bounded spending, reconciliation, and recovery evidence.

OpenChat already handles difficult cases. Its multi-user creation path publishes an acknowledged event so registration can complete even if the initiating reply disappears. Its [cycle dispenser](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/cycles_dispenser/impl/src/updates/c2c_request_cycles.rs) enforces authorization, reserve limits, throttling, and in-progress exclusion. The opportunity is to replace duplicated machinery while preserving those guarantees. A source review alone does not establish that Canic's implementation would be safer or cheaper.

A second opportunity is release evidence. OpenChat already builds [governance proposals around Wasm hashes](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/tools/canister_upgrade_proposal_builder/src/lib.rs). Canic could contribute consistent artifact manifests, configuration and dependency fingerprints, size admission, endpoint inventories, and evidence comparisons. A reviewer should be able to trace:

```text
Source + toolchain + configuration
              ↓
          Final Wasm
              ↓
       Proposal payload
              ↓
     Observed deployment
```

This should initially work with OpenChat's existing build pipeline. Requiring a runtime conversion before offering useful build evidence would make the entry cost unnecessarily high. Canic's release-identity binding also means that reproducibility must include the selected identity and complete finalization inputs; matching a source commit alone is insufficient.

A third opportunity is consistent operational diagnostics. Operators need to distinguish responding, ready, funded, waiting for capacity, and recovering an uncertain effect. Canic already makes several of these distinctions. However, its current observatory depends on terminal Canic Fleet inventory and protocol bindings. It is not an existing drop-in monitor for arbitrary OpenChat canisters. Passive external observation would need a dedicated integration.

Finally, both projects could benefit from shared infrastructure qualification. OpenChat already uses PocketIC, including a [test that upgrades production user Wasm while checking retained application data](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/integration_tests/src/upgrade_from_prod_tests.rs). Canic should complement those tests with reusable infrastructure scenarios: lost replies, exhausted reserves, interrupted provisioning, stale observations, and concurrent requests. Product correctness remains OpenChat's responsibility.

**The barriers are also a useful improvement agenda for Canic.**

1. **Production lifecycle support is the decisive long-term requirement.**

   OpenChat's [user restoration code](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/user/impl/src/lifecycle/post_upgrade.rs) handles previous layouts, bounded migration work, and deferred continuation. Its release tests account for production running multiple versions during rollout. These are fundamental requirements for a persistent application.

   Canic currently provides no supported production transition that preserves OpenChat's state across Canic releases. Same-release backup and recovery do not supply that missing contract.

   For an eventual 1.0 adoption decision, Canic would need an explicitly accepted policy for state-preserving releases, application-controlled schema evolution, staged rollout, interrupted-operation ownership, and recovery after partial rollout. This is a future product requirement, not a proposal to introduce compatibility lanes into the current pre-1.0 design. Under today's policy, realistic evaluation is limited to passive tooling and disposable deployments.

2. **Memory, lifecycle, and timer ownership must compose deliberately.**

   OpenChat's [user canister](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/user/impl/src/memory.rs) owns a memory manager with two-page buckets. Its [multi-user canister](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/multi_user/impl/src/memory.rs) uses eight-page buckets and different memory IDs. Canic uses one shared `ic-memory` manager, defaults to sixteen-page buckets, and requires bootstrap admission before storage access. Two independent managers cannot simply own the same physical stable memory.

   OpenChat also reconstructs some timers during deserialization. Its restoration code warns that trying one decoder and then falling back to another can leave timers scheduled by the failed attempt. Canic's shared timer provider and lifecycle ordering need an explicit integration with those semantics.

   Canic should offer an early integration assessment covering memory ownership, allocation geometry, lifecycle exports, timer providers, and restoration side effects. Claims that capabilities are separable should have executable examples demonstrating their actual prerequisites.

3. **Existing wire protocols and payload sizes need explicit support.**

   OpenChat's [endpoint macros](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/libraries/canister_api_macros/src/lib.rs) support Candid, MessagePack, and JSON, with custom encoding and decoding. Its upgrade tests exercise 100,000-byte avatars and 1 MiB profile backgrounds. Canic-managed external updates inherit a 16 KiB encoded argument limit unless explicitly configured otherwise.

   Adding lifecycle and inspection machinery could therefore change previously valid requests before any business code changes. Integration must identify exported names, codecs, guards, argument ceilings, and the different ingress and inter-canister paths. Actual Wasm calls should qualify those boundaries. Canic need not own every serializer, but its admission machinery must compose predictably with application endpoints.

4. **Logical placement and physical infrastructure need separate contracts.**

   A user, group, or channel is not necessarily a canister. OpenChat's shared-user placement considers occupancy, upgrade state, and permanent exhaustion of user indexes. Those are application decisions.

   A plausible future ownership split would be:

   | Responsibility | Owner |
   | --- | --- |
   | Membership, history, moderation, and user routing | OpenChat |
   | Selecting which shared canister receives a user | OpenChat policy using infrastructure observations |
   | Physical allocation, installation, reserves, and cycle accounting | Canic |
   | Application readiness after installation | OpenChat reports; Canic coordinates publication |
   | Subnet capacity and infrastructure inventory | Canic |

   A local user index could remain an application Component with managed descendants. It cannot simply become a Canic Root: the canonical Root is framework-owned and does not host arbitrary application lifecycle hooks. A future integration would need to change provisioning and controller responsibilities explicitly, with one authoritative owner per management effect.

5. **Scale needs measured contracts.**

   Canic currently limits the sum of configured top-level Component maxima to 4,096. Separate default limits allow 20,000 descendants and 16 MiB of registry data per Component. These are admission bounds, not demonstrated capacities. The definitions are in the [Fleet schema](https://github.com/dragginzgame/canic/blob/06d73694d4c15a5d2c80469030c8944281334fbe/crates/canic-core/src/config/schema/mod.rs) and [Component schema](https://github.com/dragginzgame/canic/blob/06d73694d4c15a5d2c80469030c8944281334fbe/crates/canic-core/src/config/schema/component_spec/mod.rs).

   Making every user a top-level Component would be the wrong mapping. Allowing a descendant count does not prove the registry byte budget can accommodate it. A bounded page response does not prove bounded total collection cost. Runtime child growth also needs qualification against operator inventory, observatory, and backup discovery.

   Canic already has useful range-based child traversal. The next evidence should measure allocation throughput, registry growth, Root contention, discovery, and recovery duration. Large synthetic populations can test storage and algorithm costs; smaller real multi-subnet PocketIC deployments can test platform effects. Neither substitutes for the other.

6. **Background operations need backpressure and explicit recovery semantics.**

   OpenChat's [user-upgrade worker](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/local_user_index/impl/src/jobs/upgrade_users.rs) pauses when pending event-store work exceeds 100,000 events and bounds concurrent upgrades. Its grouped queues preserve per-destination sequencing while processing different destinations concurrently.

   Infrastructure work must respond to application pressure. Useful Canic improvements include application-supplied admission signals, per-subnet concurrency budgets, fairness, bounded retry debt, and observable pause reasons. Generic mechanisms should remain separate from chat-specific queue semantics.

   Liveness and idempotency also remain separate. Canic does not promise automatic trap recovery for ordinary application timers. A timer abstraction must not imply durable delivery or exactly-once application effects.

7. **SNS governance must remain the source of authority.**

   OpenChat's [upgrade entrypoints](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/user_index/impl/src/updates/upgrade_user_canister_wasm.rs) enforce governance-principal guards, validate selected hashes, and distribute approved artifacts through its indexes.

   A future Canic integration would need proposal-bound targets, artifacts, allowed effects, and spending limits. Operators could prepare and observe operations without gaining authority to bypass SNS decisions. Canic's reviewed-plan machinery is a promising foundation, but an SNS execution integration was not found in the inspected source. It is a substantive gap.

8. **Per-canister overhead must become an acceptance criterion.**

   Small fixed costs accumulate across a large fleet: stable-memory allocation slack, periodic timers, sampling, module size, installation cost, cycle reserves, receipt retention, and monitoring calls.

   Canic's recent memory work and pending optional-observability changes address this direction. Qualification should cover different role profiles: a mostly idle dedicated user canister differs from a busy shared-user canister or a storage bucket.

   Measure incremental Wasm size, touched stable pages, idle cycles, timer executions, and management calls under identical workloads and selected features. Source-code reduction alone does not establish operating savings.

**An illustrative configuration and subnet hierarchy**

Here is a possible `canic.toml` for the disposable experiment. It uses the current role, Component Spec, child, and spawn-grant syntax. The package paths are placeholders for newly integrated fixture crates; pointing them at unmodified OpenChat crates would not make those crates Canic-compatible. The example covers the core placement and provisioning roles, not OpenChat's complete service inventory.

The capacities below are deliberately small experimental ceilings, not measurements of OpenChat or production recommendations. `maximum_instances` limits the number of top-level Component instances across the Fleet; child grants apply independently to each concrete parent. These declarations permit creation but do not create instances or assign physical subnets. A separate desired Fleet document must select actual subnet Principals, infrastructure identities, Component placements, artifacts, and reviewed funding.

```toml
[app]
name = "openchat_lab"

# Canonical Canic infrastructure. Coordinator and root-local Wasm Stores
# are supplied by the Fleet machinery, not application Component Specs.
[roles.root]
kind = "root"

# Placeholder paths to future fixture crates, relative to this file.
[roles.user_index]
kind = "canister"
package = "canisters/user_index"

[roles.group_index]
kind = "canister"
package = "canisters/group_index"

[roles.storage_index]
kind = "canister"
package = "canisters/storage_index"

[roles.local_user_index]
kind = "canister"
package = "canisters/local_user_index"

[roles.user]
kind = "canister"
package = "canisters/user"

[roles.multi_user]
kind = "canister"
package = "canisters/multi_user"

[roles.group]
kind = "canister"
package = "canisters/group"

[roles.community]
kind = "canister"
package = "canisters/community"

# Proposed application allocator for this experiment; OpenChat does not
# already expose this role under this name.
[roles.storage_allocator]
kind = "canister"
package = "canisters/storage_allocator"

[roles.storage_bucket]
kind = "canister"
package = "canisters/storage_bucket"

# Application authentication remains part of the fixture implementation.
# This switch only disables Canic's optional delegated-token subsystem.
[auth.delegated_tokens]
enabled = false

# Three Fleet-wide application services, each deployed once.
[component_specs.user_directory]
component_role = "user_index"
maximum_instances = 1

[component_specs.chat_directory]
component_role = "group_index"
maximum_instances = 1

[component_specs.media_directory]
component_role = "storage_index"
maximum_instances = 1

# Plan one local_chat Component on each of two subnets.
# The ceiling of two does not itself enforce one per subnet.
[component_specs.local_chat]
component_role = "local_user_index"
maximum_instances = 2

[component_specs.local_chat.limits]
maximum_descendants = 120
maximum_registry_bytes = 16777216

# Flat catalog of physical child canister roles.
# A multi_user canister may hold many logical users; they are not children.
[component_specs.local_chat.children.user]
kind = "instance"

[component_specs.local_chat.children.multi_user]
kind = "instance"

[component_specs.local_chat.children.group]
kind = "instance"

[component_specs.local_chat.children.community]
kind = "instance"

# The application requests children; the bound Root performs allocation.
[component_specs.local_chat.spawn_grants.local_user_index.user]
maximum_instances_per_parent = 64

[component_specs.local_chat.spawn_grants.local_user_index.multi_user]
maximum_instances_per_parent = 8

[component_specs.local_chat.spawn_grants.local_user_index.group]
maximum_instances_per_parent = 32

[component_specs.local_chat.spawn_grants.local_user_index.community]
maximum_instances_per_parent = 16

# Separate media capacity on each occupied subnet. The global storage_index
# would need application logic to select and contact these allocators.
[component_specs.media_capacity]
component_role = "storage_allocator"
maximum_instances = 2

[component_specs.media_capacity.limits]
maximum_descendants = 4
maximum_registry_bytes = 1048576

[component_specs.media_capacity.children.storage_bucket]
kind = "instance"

[component_specs.media_capacity.spawn_grants.storage_allocator.storage_bucket]
maximum_instances_per_parent = 4
```

The five Component Specs allow at most seven top-level instances: three global services, two local chat Components, and two media-capacity Components. Each local chat Component admits up to 120 physical descendants, and each media-capacity Component up to four. Byte limits, Root capacity, funding, and other admission checks can bind before those counts are reached. Automatic top-ups are omitted in this small example; actual funding and reserve policies must be selected for the experiment.

Using `kind = "instance"` for `multi_user` is intentional. The fixture leaves logical user placement with the application. Selecting Canic's sharding pools instead would introduce its assignment policy and capacity semantics, which would need a separate comparison with OpenChat's current selection rules. This configuration also does not register global services with one another or wire their application APIs; the fixture must implement those initialization and readiness steps.

The proposed component inventory is shown below. Counts describe the illustrated placement or configured ceilings, not an observed deployment.

```text
+---------------------+---------------------+--------------------------+
| Role                | Scope / placement   | Responsibility           |
+---------------------+---------------------+--------------------------+
| Fleet Coordinator   | 1 Fleet; subnet A   | Fleet planning / publish |
| Fleet Subnet Root   | 1 per subnet        | Management effects       |
| Wasm Store          | 1 per subnet        | Qualified artifacts      |
+---------------------+---------------------+--------------------------+
| user_index          | 1 global; subnet A  | User directory / routing |
| group_index         | 1 global; subnet A  | Chat directory           |
| storage_index       | 1 global; subnet A  | Media directory / routing|
| local_user_index    | 1 on A; 1 on B      | Local chat parent        |
| user                | <=64 / local parent | Dedicated user state     |
| multi_user          | <=8 / local parent  | Shared user state        |
| group               | <=32 / local parent | Group chat state         |
| community           | <=16 / local parent | Community / channel state|
| storage_allocator * | 1 on A; 1 on B      | Local media parent       |
| storage_bucket      | <=4 / media parent  | Media bytes / serving    |
+---------------------+---------------------+--------------------------+
* Proposed application role for this experiment.
```

For this example, the separate placement plan would put the three global application services and the Coordinator on subnet A, then place one local chat Component and one media-capacity Component on each subnet. The hierarchy below represents infrastructure association and logical application parentage. It is not a controller graph: a local index requests children, while its bound Root retains their management authority. The Coordinator's placement does not make it a child controlled by Root A.

```text
Fleet: openchat_lab
|
+-- Subnet A
|   |
|   +-- Fleet Coordinator [Fleet-wide coordination]
|   |
|   +-- Fleet Subnet Root A
|       |
|       +-- Wasm Store A [implicit infrastructure]
|       |
|       +-- user_index [user_directory Component]
|       +-- group_index [chat_directory Component]
|       +-- storage_index [media_directory Component]
|       |
|       +-- local_user_index A [local_chat Component]
|       |   +-- user canisters         [0..64]
|       |   +-- multi_user canisters   [0..8]
|       |   |   `-- logical users     [application data, not canisters]
|       |   +-- group canisters        [0..32]
|       |   `-- community canisters    [0..16]
|       |       `-- channels          [application data, not canisters]
|       |
|       `-- storage_allocator A [media_capacity Component]
|           `-- storage_bucket canisters [0..4]
|
`-- Subnet B
    |
    `-- Fleet Subnet Root B
        |
        +-- Wasm Store B [implicit infrastructure]
        |
        +-- local_user_index B [local_chat Component]
        |   +-- user canisters         [0..64]
        |   +-- multi_user canisters   [0..8]
        |   |   `-- logical users     [application data, not canisters]
        |   +-- group canisters        [0..32]
        |   `-- community canisters    [0..16]
        |       `-- channels          [application data, not canisters]
        |
        `-- storage_allocator B [media_capacity Component]
            `-- storage_bucket canisters [0..4]

Fleet coordination: Coordinator <--> Root A / Root B
Application calls:  global indexes <--> local indexes / allocators
Artifact access:    each Root uses its own subnet's Wasm Store
```

This deliberately separates OpenChat's application directories from Canic's infrastructure registry. The proposed storage allocator makes the subnet-local management boundary visible; it is additional integration work, not a claim about OpenChat's current architecture. SNS authorization, identity, notifications, event delivery, escrow, and other services are outside this reduced diagram. In particular, placing a Coordinator does not establish an SNS integration or authorize control of an existing OpenChat deployment.

**A practical evaluation sequence**

I would pursue four bounded pieces of work. These are proposals for scope discussion, not commitments in Canic's current release plan.

| Proposed work | Evidence required |
| --- | --- |
| Inspect externally built artifacts and emit passive evidence | Consume an OpenChat-shaped artifact without changing it; bind hashes, tools, exports, and size findings. |
| Qualify difficult runtime composition | A fresh-install fixture with application-owned storage, custom codecs, large payloads, timers, and explicit lifecycle ownership. |
| Establish scale and overhead envelopes | Large registry measurements plus real multi-subnet provisioning, contention, funding, interruption, and replay cases. |
| Evaluate eventual production adoption | An accepted lifecycle and governance contract, followed by an independently qualified implementation. |

The first experimental application should be a disposable OpenChat-shaped workload: global and subnet-local indexes, dedicated and shared user roles, group/community roles, and a storage role. It should exercise registration bursts, pool exhaustion, uncertain replies, funding pressure, and application readiness.

Production migration should remain outside that experiment. Pinning Canic forever or restoring snapshots across releases would not resolve the underlying product contract.

I would also leave OpenChat's media implementation alone initially. Its [storage bucket](https://github.com/open-chat-labs/open-chat/blob/2611b2e5bdb2cc0dbe397bd04a87989e4c05553b/backend/canisters/storage_bucket/impl/src/queries/http_request.rs) already handles HTTP ranges, streaming callbacks, and quarantine checks. Canic's blob feature is not evidence of equivalent behavior. Replacing identity, authorization, or chat queues would similarly expand the exercise far beyond infrastructure reuse.

**The strongest use of OpenChat here is as a demanding qualification target for Canic's modularity, operating cost, and recovery guarantees.** A convincing adoption result would be a narrowly scoped replacement that removes application-owned infrastructure code, survives the same failure cases, respects governance, and has measured acceptable overhead.

For developers operating similar applications, which of these costs dominates today: fleet releases, cycle management, subnet placement, recovery, or the per-canister overhead of shared infrastructure? That would help identify the most useful first integration to qualify.
