# Installing Canic

Install the operator CLI at the same version as the downstream `canic` crate:

```bash
cargo install --locked canic-cli --version <same-version-as-canic>
canic --version
```

From this checkout:

```bash
make install
```

For the complete maintainer toolchain:

```bash
make install-dev
```

The maintainer setup installs the repository-pinned ICP CLI, `ic-wasm`,
Binaryen, Candid tools and `sccache`, and configures the repository pre-commit
formatter. Release artifact builds require the checksum-bound Binaryen 132
`wasm-opt`; they fail rather than emitting unoptimized release bytes when it is
missing or has a different identity or executable SHA-256. Published CLI users
can install the governed optimizer without a Canic checkout:

```bash
canic toolchain install
```

The command prints the admitted path under `~/.local/bin`; place that directory
before any other `wasm-opt` on `PATH`. Explicit
`CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `RUSTC_WRAPPER` values remain
authoritative. Otherwise Make and Canic artifact builds discover `sccache` and
keep deterministic Wasm builds non-incremental.

## ICP CLI

The maintained range is `icp-cli >=1.2.0, <2.0.0`; the maintainer toolchain currently pins `1.4.0`.

```bash
which icp
icp --version
bash scripts/ci/install-icp-cli.sh
```

Custom connected networks must declare their exact root key. Enroll that trust
through Canic before Fleet observation or mutation:

```bash
sha256sum ./root-key.der
canic network enroll <environment> \
  --root-key ./root-key.der \
  --fingerprint <64-lowercase-hex>
```

For password-protected identities, ICP CLI can cache a bounded session:

```bash
icp settings session-length 1h
icp identity reauth <identity-name> --duration 1h
```

## Canister Dependencies

Each Canic-managed canister needs runtime and build dependencies plus exact
package metadata:

```toml
[dependencies]
candid = "<version>"
canic = "<same-version-as-cli>"
ic-cdk = "<version>"

[build-dependencies]
canic = "<same-version-as-cli>"

[package.metadata.canic]
app = "example"
role = "app"
```

The role must exist in the selected App configuration. Root packages use
`role = "root"` and the required control-plane feature.

The build script remains small:

```rust
fn main() {
    canic::build!("../canic.toml");
}
```

An ordinary managed canister uses the maintained lifecycle facade:

```rust
#![expect(clippy::unused_async)]

use canic::prelude::*;

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::finish!();
```

Application endpoints belong between `start!` and `finish!` and use Canic's
endpoint macros. The complete App schema is in [CONFIG.md](CONFIG.md).

## Configure And Build

```bash
canic app create example
canic scaffold canister example app
canic app role attach example app --component-spec example.app
canic build example app --profile release \
  --provenance artifacts/example-app-provenance.json
```

For split Cargo/ICP roots, pass `--workspace`, `--icp-root` and an absolute
`--config` path explicitly.

## Ensure A Fleet

Start the selected local replica when applicable:

```bash
canic replica start --background
```

Write the current desired Fleet contract at `fleets/<fleet>.toml`; its complete
schema and cycle-safety boundary are in
[Fleet ensure](docs/features/operations/fleet-ensure.md).

Plan first:

```bash
canic fleet ensure example-local --desired fleets/example-local.toml
```

Review the returned `plan_sha256`, dispositions, transfers, fees, funding,
maximum debit/burn and conservation equation. Apply only that digest:

```bash
canic fleet ensure example-local \
  --desired fleets/example-local.toml \
  --apply <plan_sha256>
```

Rerun the same apply command after interruption. The current journal reconciles
the live result before retry. After terminal convergence, run plan/apply again
to prove the immediate successor has zero mutation actions.

Historical install, deployment, adoption, retained-repair and recovery-bundle
commands are removed. Do not copy their state into `.canic/fleet-ensure` or
attempt to migrate it. Any old canister that still holds recoverable cycles
must appear explicitly in the current desired document for reuse or safe drain.

## Cycle-Recovery Limitation

The IC does not let a controller pull cycles from an arbitrary canister. A
material canister may be replaced or deleted only when it exposes the exact
configured, idempotent treasury-drain contract. Without it, Canic returns a
typed blocker and leaves the canister untouched. Never bypass that blocker with
a raw stop/delete command.

## Development Validation

Automated coding work runs only targeted package checks. Human maintainers own
the complete release boundary:

```bash
make validate
```

Versioning, tagging, package publication, pushing and live deployment remain
separate human-owned actions governed by
[CI and deployment governance](docs/governance/ci-deployment.md).
