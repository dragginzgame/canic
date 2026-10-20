# Current handoff

## Published Build adoption contribution

This isolated contribution includes PR #596 head `c378d48e1`, preserving its
shared history and runtime Cargo facts work. Normal sibling checkouts and their
pending changes are excluded. Own package versions remain 0.111.0; the existing
0.112.0 changelog batch includes this adoption. No PR merge, version bump, release,
publication, installed lifecycle operation or normal-checkout mutation is included.

The root catalog now selects published Build 0.23, Host 0.31 and Backup 0.35.
The registry lock selects Build 0.23.0, Host 0.31.2 and Backup 0.35.1. Build's
metadata/tree specifications render commands; Canic's Host owns their execution,
output limits and typed failures. Success is admitted before completeness and
canonical decoding. Feature-safe grouping receives a tree observer using the
same build context/environment. Workspace discovery and embedded fixture inventory
retain locked/offline choices, I/O error kinds, existing budgets and no deadline.

Backup supplies the matching Host checksum/publication error identities. Its
published artifact checksum implementation is unchanged from 0.31.4; existing
Canic custody, framing and error adapters are retained. Host Tools explicitly
selects the Candid parser already required by artifact service checks. No local
compatibility implementation or public re-export is added.

Canic keeps recipes, source/config custody, transformation, release admission,
application protocols and retained-artifact policy. The root application Platform
requirement is unchanged. Upstream packages still select older Host 0.27 and
Platform 0.1, while the selected Host/Backup owners also require Platform 0.4;
these transitive identities do not establish runtime migration. The two private
Blob consumer catalogs and lockfiles are unchanged.

## Validation and remaining acceptance

Host, CLI and Testing Internal all-target compilation and actual Rust 1.91 library
checking pass against the real registry graph. Strict Host/CLI all-target and
Internal library Clippy pass. Testing Internal's default-feature test target emits
the pre-existing unused admission-helper import warning. Nine build/grouping,
13 provenance, nine backup artifact, 22 persistence and four embedded-inventory
cases pass. The explicit real-Wasm reproduction remains ignored. Grouped formatting,
direct/inherited dependency policy and the maintained runtime graph check pass.

The role-package group now passes 36 cases; five failures retain fresh fixture
resolution of both Memory 0.48 and 0.49. The passive-contract check uses the actual
package-selected Cargo tree, avoiding sibling workspace feature unification.
The managed test role selects the verifier required by its existing configuration.
Both corrected regressions pass on the parent and adoption graphs; the test role
also passes actual Rust 1.91 native checking. The comparison uses inherited offline
mode after an initial registry DNS failure. Do not weaken runtime admission
or change storage graphs to make this adoption appear qualified. The existing
[component handoff PR](https://github.com/dragginzgame/canic/pull/595) records the
coordinated Memory/Storage/Blob selection prerequisite.

The parent PR's eight Binaryen controls, compiler-cache wrapper controls, actual
three workflow binding blocks with substituted installer/compiler, Actionlint,
strict Host Clippy and grouped formatting pass. Its cache installer binding,
physical-path assertions and development-toolchain repair are separate from this
adoption. Its pinned-compiler Core/Contracts/CLI failures are repaired without
lint suppressions. Session admission reads the existing subject index with its
other indexes, retaining generation, replay, capacity and pre-write refusal.
Parent all-target/all-feature Core/Contracts/CLI and Host test Clippy pass, as do
18 session, 28 memory, 11 sharding, one pending-intent and one Candid round-trip
case. Actual Rust 1.91 Core/Contracts checks pass natively and for Wasm. New-head
hosted checks remain required. See the retained
[parent handoff](https://github.com/dragginzgame/canic/blob/c378d48e1da2ac0930efde84000d017f085ba2b8/docs/status/current.md).

Direct Cargo Wasm checking is refused by the existing authoritative-build guard;
do not bypass it or claim installed acceptance. Exact-head hosted checks,
authoritative Canic builds and owning managed tests remain necessary. Consumer
acceptance stays with [Canic #530](https://github.com/dragginzgame/canic/issues/530)
and [Build #9](https://github.com/dragginzgame/ic-build/issues/9).

Current evidence/before-images: `/tmp/ic-build-patch1-fixes.LDF1Oa/`.
Earlier adoption evidence: `/tmp/ic-build-adoption-delivery.O20V1y/`.
Leave review and integration to the owner.
