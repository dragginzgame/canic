# Managed-App Qualification

Canic publishes one bounded, host-only test surface for downstream managed
Apps. It constructs the exact version-1 managed init payload, local admission
projection, Component Group Directory and Root-issued runtime configuration
from the same checked-in `canic.toml` that built the App Wasm.

Add one development dependency; product role packages keep their normal
feature selection unchanged:

```toml
[dev-dependencies]
canic = { version = "=0.109.14", features = ["testing"] }
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
