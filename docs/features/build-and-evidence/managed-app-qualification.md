# Managed-App Qualification

Canic publishes one bounded, host-only test surface for downstream managed
Apps. It constructs the exact version-1 managed init payload, local admission
projection, Component Group Directory and Root-issued runtime configuration
from the same checked-in `canic.toml` that built the App Wasm.

Add one development dependency; product role packages keep their normal
feature selection unchanged. Replace the version placeholder with the exact
Canic release used by the application and CLI:

```toml
[dev-dependencies]
canic = { version = "=<same-version-as-application>", features = ["testing"] }
```

The minimal fixture is:

```rust,no_run
use candid::Principal;
use canic::testing::{ManagedAppQualificationInput, install_managed_app};

let input = ManagedAppQualificationInput::new(
    include_str!("../canic.toml"),
    "app",
    "app",
    env!("QUALIFICATION_RELEASE_BUILD_ID"),
    vec![Principal::from_slice(&[7; 29])],
    managed_app_wasm,
);
let fixture = install_managed_app(input)?;
fixture.configure_and_wait_until_active(30)?;
```

Application tests then use `fixture.pic()` and `fixture.app()` for direct
public, admitted, denied and application-ownership assertions. The fixture can
read the protected admission status, prepare an exact successor projection so
the App becomes fenced, and upgrade the exact same Wasm to prove stable
same-release recovery. `install_standalone_app` covers the corresponding
standalone-local build and its exact same-release upgrade. The module
re-exports the required PocketIC call traits, so the downstream test does not
pin or reconstruct `canic-core` or `ic-testkit` internals.

Enable the `testing` feature through the normal Cargo feature resolver, then
commit the resulting downstream lockfile. The public facade currently inherits
PocketIC's exact `thiserror` selection; Cargo may therefore lower a previously
unconstrained `thiserror 2.x` lock entry when the feature is enabled. That
ordinary lock refresh is supported and is qualified from a packaged consumer;
downstreams must not add a direct transitive-version override or hand-edit the
lockfile.

The input requires one exact Component Group deployment and one Component Spec.
An absent or repeated occurrence, invalid release-build identity, empty
admission set or inconsistent topology fails before canister installation.
Synthetic Principals, operation identities and Directory evidence exist only
inside the fresh caller-owned PocketIC. This support does not add a runtime
storage owner, endpoint, timer, lifecycle participant or Fleet policy
authority.

Canic owns only the infrastructure lifecycle. The downstream remains
responsible for assertions about its public endpoints, caller-derived
application membership and ownership, database readiness, timers and other
framework participants.

## Managed Component trees

`install_managed_component_group` qualifies a complete configured Component
Group when a test must include children created by a Hub. The caller supplies
the exact Wasm and optional application-init bytes for every role; Canic parses
the selected Component Group deployment and derives every top-level
`Component` and descendant `ComponentChild` authority internally.

```rust,no_run
use candid::Principal;
use canic::testing::{
    ManagedComponentGroupQualificationInput, ManagedRoleQualificationArtifact,
    install_managed_component_group,
};

let input = ManagedComponentGroupQualificationInput::new(
    include_str!("../canic.toml"),
    "application",
    env!("QUALIFICATION_RELEASE_BUILD_ID"),
    vec![Principal::from_slice(&[7; 29])],
    vec![
        ManagedRoleQualificationArtifact::new("hub".parse()?, hub_wasm),
        ManagedRoleQualificationArtifact::new("child".parse()?, child_wasm),
    ],
);
let mut fixture = install_managed_component_group(input)?;
```

For application contracts that bind their installation to the actual canister ID,
set `artifact.application_init_args` to
`ManagedApplicationInit::ForCanister(encode_application)`, where the deterministic
encoder has signature `fn(Principal) -> Vec<u8>`. The fixture calls it after
allocation and nests its bytes after the unchanged protected Canic payload.
Do not guess simulator IDs or rewrite Canic authority to construct these inputs.
An encoder panic propagates to the fixture caller. Use
`ManagedApplicationInit::Encoded(bytes)` for fixed application arguments or
`ManagedApplicationInit::None` to omit them. Explicit child arguments supplied by
the Root allocation request take precedence over this per-role fallback.
This is fixture injection, not a production deployment option. Root's current
top-level Component and Component Group installation paths supply no application
arguments; a consumer needing them must separately qualify a production delivery
path before deployment.

Configured initial sharding and scaling children are installed before the
constructor returns. Submit an on-demand indexed, sharded or scaled child request
with `fixture.pic().submit_call(...)`, then call
`fixture.settle_submitted_call(&message_id, maximum_ticks)` before reading the ingress
reply with `fixture.pic().await_call(message_id)`. Allocation replies remain
pending until the fixture installs and activates the exact child and publishes
its allocation identity in the parent's Directory. Settlement also waits for the
submitted request's terminal result, including when application work precedes its
allocation. A synchronous update call cannot drive this host-owned settlement.
`fixture.nodes()` exposes the resulting read-only tree;
`binding`, `runtime_status`, `admission_status`, `upgrade_same_release` and
`prepare_admission_successor` exercise its protected lifecycle without asking
the downstream to construct Canic authority DTOs.

All placement strategies share this boundary: the Hub calls its ordinary
sharding, scaling or index API; the fixture observes the Root's replay-safe
allocation receipt and installs the child with the exact parent, pool/slot
metadata, Component deployment, Root, runtime Directory and Fleet-admission
projection. The fixture does not choose placement or fabricate a child request.

Configured initial children use the same production bootstrap boundary. A Hub
remains unready while the Root is unavailable or the child operation is
nonterminal, and Root membership activation independently checks that readiness
before publishing the Hub as Active. The fixture therefore cannot turn a failed
initial-child bootstrap into a successful top-level lifecycle result.

## Continue From Here

- [Review fixture artifacts](fixture-artifacts.md)
- [Browse build and evidence](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
