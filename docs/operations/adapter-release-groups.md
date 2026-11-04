# Independent Canic releases

The maintainer selected this boundary on 2026-11-02: keep one Cargo workspace,
catalog and lockfile, with four independently versioned release groups.

| Group | Primary and public packages | Coupling retained |
| --- | --- | --- |
| `framework` | `canic`; Contracts, Core, Control Plane and Macros | Exact runtime family matching; framework-owned private probes belong here too. |
| `host` | `canic-cli` and `canic-host` | The tool's linked framework version defines exact runtime admission; Host and CLI share one tool version. |
| `icydb` | `canic-icydb` | Its declared framework macro and published Platform/Timers requirements; applications qualify their database composition. |
| `blob-service` | `canic-blob-service` | Its declared Canic and Blob runtime/contracts requirements; dedicated and embedded consumers retain independent locks. |

The [group policy](../../ci/cargo-release-groups.json) assigns every root member
exactly once. Each member declares its group's literal version. The
[root catalog](../../Cargo.toml) lists every member with its current local version
and path. Common metadata and dependency declarations still inherit from the
workspace. There is no additional workspace, version file or release runner.

The adoption preserves existing numeric package versions and published tags.
Each group continues its package history; its first independent release receives
a group-prefixed tag, such as `icydb-vX.Y.Z`. Earlier coordinated notes remain
once in [shared history](../../releases/history.md). The root
[changelog](../../CHANGELOG.md) indexes the four canonical group ledgers.

An authorized source release selects its group explicitly:

```sh
make release-patch RELEASE_GROUP=icydb
make publish RELEASE_GROUP=icydb
```

The same selection is required for version preparation, staging, candidate
checks, fast releases and exact-version resume. Missing, empty or unknown
groups refuse before tool setup, validation or mutation. A saved attempt resumes
with its original group: `make release-resume RELEASE_GROUP=icydb VERSION=X.Y.Z`.
Plans, tags and validation receipts carry that group. Publication observes every
public package in the selected group; a resume suffix cannot skip missing
predecessors. Cargo's normal package verification remains enabled.

Adapter-only releases change their own versions and ledger, the root requirement
for that adapter, and exact affected lock records. Framework and other adapter
versions remain unchanged. Shared consumer locks are checked for every group;
records outside the selected group's exact resolved ownership remain unchanged.
Only Host releases update the installed CLI default. Candidate checks bind
the transaction to its validated source and policy, including unchanged
manifests, other-group identities, consumer locks and CLI declaration.

Independent versions do not isolate Cargo resolution. The release hooks adopt
Shared Tooling's [group validation contract](../release-group-validation.md)
through the [stage roster](../../ci/release-validation.json), reviewed
[scope mapping](../../ci/release-validation-scopes.json) and Canic's
[selector](../../scripts/release/validation-plan.pl). Repository-wide CI and
`make validate` keep their complete graph. An adapter using a new
framework contract must first adopt an available supplying framework release.
For the prepared batch, publish the updated framework before Host and adapters
that require its new contracts or runtime identities. Do not republish an old
package version with a changed dependency declaration. Assess its own public
contract to choose the next authorized version.

## Tool and framework identities

Host and CLI versions identify the released tool. The linked `canic-core::VERSION`
identifies its supported framework. Runtime admission compares each application's
resolved `canic` with that exact framework version and still validates the complete
selected runtime graph. No version ranges, negotiation or fallback are introduced.

Scaffolds select the linked framework for their Canic dependencies. Generated
Fleet package versions and protocol identities come from the selected framework
graph. Local Fleet records bind the same framework identity, configuration, session
and trust bytes. A Host-only patch can retain its supported framework; adopting a
different framework requires explicit dependency adoption and qualification.

Tool help/version output, generator caches, builder versions and the producer's
`canic_version` in evidence/build provenance retain the Host/CLI release identity.
Artifact protocol bindings retain their framework identity. These inputs remain
independent in hashes and review; separating release groups grants no new paid
authority or cross-release installation recovery.

Host-only releases preserve framework and adapter versions and ledgers. Framework
releases preserve Host versions and the installed CLI default. Cross-group
dependencies still need available supplying packages. This boundary authorizes no
release, publication or deployment.

## Release validation selection

`release-verify`, `patch`, `minor` and `major` select a source-bound plan and run
it with Shared Tooling's existing `--plan` interface. The full proposed change is
measured from the selected group's current annotated release tag, rather than
the latest commit. The descriptive handoff and selected ledger may accompany a
confined change; neither determines validation authority. Missing first split
tags select the full gate. Unknown groups,
divergent baselines and stage/group roster drift refuse. Shared files, compiler,
CI/release recipes, dependencies/locks/features, generators, public identities,
unknown paths and cross-group changes select every stage.

The initial mapping deliberately keeps workspace warning-denied all-target and
all-feature Clippy, normal workspace checking, formatting, policy/security,
locked root/private-consumer runtime identities and the complete ordinary and
installed PocketIC test graph. It changes neither test membership nor feature
profiles. Its only eligible omissions are:

| Group | Confined inputs | Eligible omissions |
| --- | --- | --- |
| Framework | None: framework work affects its consumers. | None; full roster. |
| Host | The presentation, bounded-read and test roots enumerated in the mapping. | Private Blob fixture Clippy/generated-peer qualification and the Control Plane feature matrix. Generators, installation, deployment, admission and protocol work expand to full. |
| IcyDB | Adapter tests only. | Private Blob fixture Clippy/generated-peer qualification and the Control Plane feature matrix. Production code exports macros and Wire identities, so it expands to full. |
| Blob service | Enumerated metric projection/workflow/metrics/funding roots and tests. | Control Plane feature matrix. Public schemas, mounting/transport, memory/lifecycle composition and supplying dependencies expand to full; Blob consumer checks and installed/recovery tests stay required. |

Each omitted stage carries its concrete reason in the canonical plan. This is
group-specific release qualification, separate from the non-runtime fast patch
lane. Linux and both macOS host obligations remain with the existing workflow,
native fixture checks and required CI; a local plan does not certify hosted runs.

Narrowing is admitted only after an ancestral annotated release receipt proves
the exact mapping against a full gate on a source eligible for that group scope.
The first such group-confined release therefore still executes every stage. Its
actual successful roster also covers the proposed subset on that same source;
subsequent confined changes may select the subset. Recipe/mapping changes require
fresh full qualification. The current baseline receipt must also qualify the
exact maintained Cargo manifests and locks. The canonical version transaction
projects their resulting graph hash into the receipt, carrying the same basis
across ordinary group version bumps. A dependency, feature, supplier or shared
change drops that qualification; an older full tag cannot silently restore it.
Synthetic dispatch fixtures cannot qualify production
recipes: they have different source and mapping identities. The current mixed
dependency/runtime/release-tooling batch selects the full roster and does not
establish a narrower group's qualification.

Each fresh attempt is retained under `target/release-validation/`. Shared retains
the parsed canonical plan/policy, exclusions, input checkpoints, stage events,
timings and raw logs. Canic seals a successful proof only after every required
stage passes and the source stays clean. The `schema: 1`, `gate: "plan"` group
receipt binds the complete source/recipe checkpoint, baseline and qualification
tag objects, mapping, Cargo graph, host/compiler/configuration and executed log hashes. Its
`validation.plan.scope` explicitly distinguishes `workspace` from `group`.
Preparation and candidate/recovery checks re-admit that exact proof and require
its retained logs. Missing, altered, interrupted or failed evidence refuses;
a new failed attempt invalidates its previous source/group success pointer.
Keep the attempt evidence through publication and authorized cleanup.

Earlier complete/fast receipts retain their existing meanings. A group plan is
never a full-workspace fast-patch basis; the fast lane must locate a distinct
complete or `plan-workspace` ancestor. Source/index admission, exact metadata
transactions, unrelated group identities/ledgers, archive checks, supplier-first
publication, registry/provenance and exact-version recovery remain unchanged.
No performance saving is claimed before actual consumer qualification and timing.
