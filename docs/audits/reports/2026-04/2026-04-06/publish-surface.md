# Publish Surface Audit - 2026-04-06

## Report Preamble

- Scope: workspace root package policy where relevant; published crate manifests under `crates/canic`, `crates/canic-cdk`, `crates/canic-core`, `crates/canic-control-plane`, `crates/canic-memory`, `crates/canic-testkit`, `crates/canic-wasm-store`, `crates/canic-installer`, and `crates/canic-dsl-macros`; package-local README/docs posture and public binary/example surface where present
- Compared baseline report path: same-day earlier retained run at this path before the revised `publish-surface` audit wording
- Code snapshot identifier: `9b3aade1ef65dbb856a9c1a966f8dd63a5b3a6cb`
- Method tag/version: `publish-surface-v2`
- Comparability status: `comparable`
- Exclusions applied: internal crates with `publish = false`, generated docs, packaged artifacts, and non-package-local scripts except where a package README presents them as part of the installed surface
- Notable methodology changes vs baseline: revised `publish-surface` audit wording adopted for this rerun; conclusions still come from direct manifest, README, binary, example, and feature inspection across the published crate set

## 0. Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates reviewed | 9 | 9 | 0 |
| Published crates with thin docs posture | 2 | 2 | 0 |
| Published crates with `readme = false` pressure | 0 | 0 | 0 |
| Publish-surface mismatches | 0 | 0 | 0 |
| Published crates with binary/example posture pressure | 1 | 1 | 0 |
| Alternate-facade ambiguity seams | 2 | 2 | 0 |
| Published crates with default-feature contract pressure | 1 | 1 | 0 |
| Publishable-but-underspecified crates | 0 | 0 | 0 |

Notes:
- Thin docs posture is now mostly concentrated in the intentionally short `canic` facade README; the role-specific `canic-wasm-store` and `canic-installer` READMEs now state intended audience more explicitly.
- No published crate README or binary surface inspected in this run materially implied unsupported internal-only use.

## 1. Manifest Publish Posture

| Crate | Publish Intent | `publish` Posture | README / docs.rs Metadata | Binary / Example Surface | Package Contract Clarity | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | main public facade | `publish = true` | `readme = "README.md"`, docs.rs/homepage/repository all set in [Cargo.toml](/home/adam/projects/canic/crates/canic/Cargo.toml) | one library example (`minimal_root`) plus tests | clear; crate-local README now explains default `metrics` and optional facade features before redirecting to workspace docs | Low |
| `canic-cdk` | standalone support crate | `publish = true` | package metadata set in [Cargo.toml](/home/adam/projects/canic/crates/canic-cdk/Cargo.toml) and README present | no public binaries; README includes one direct example | clear standalone role | Low |
| `canic-core` | lower-level runtime crate | `publish = true` | docs.rs/homepage/repository set and `readme = "README.md"` in [Cargo.toml](/home/adam/projects/canic/crates/canic-core/Cargo.toml) | one benchmark, one small test target | clear lower-level role with explicit “prefer `canic`” guidance in crate-local README | Low |
| `canic-control-plane` | lower-level control-plane support crate | `publish = true` | docs.rs/homepage/repository set and `readme = "README.md"` in [Cargo.toml](/home/adam/projects/canic/crates/canic-control-plane/Cargo.toml) | no public binaries/examples | clear lower-level role after README expansion | Low |
| `canic-memory` | standalone stable-memory support crate | `publish = true` | package metadata set in [Cargo.toml](/home/adam/projects/canic/crates/canic-memory/Cargo.toml); README is substantive | no public binaries; README includes direct usage flows | clear standalone role | Low |
| `canic-testkit` | standalone public test infrastructure | `publish = true` | package metadata set in [Cargo.toml](/home/adam/projects/canic/crates/canic-testkit/Cargo.toml); README is explicit about public/internal boundary | no public binaries | clear standalone role | Low |
| `canic-wasm-store` | canonical published role crate | `publish = true` | package metadata set in [Cargo.toml](/home/adam/projects/canic/crates/canic-wasm-store/Cargo.toml); README now states intended audience and narrow role-specific purpose | no public binaries/examples | clear, role-specific package posture | Low |
| `canic-installer` | installed tooling crate | `publish = true` | package metadata set in [Cargo.toml](/home/adam/projects/canic/crates/canic-installer/Cargo.toml); README now states who should use the crate directly and what it is not | six explicit installed binaries in manifest | clear installed-tooling contract, still broader than the rest of the public set | Low |
| `canic-dsl-macros` | proc-macro support crate | `publish = true` | docs.rs/homepage/repository set and `readme = "README.md"` in [Cargo.toml](/home/adam/projects/canic/crates/canic-dsl-macros/Cargo.toml) | proc-macro library only | clear support role with explicit guidance to prefer `canic` | Low |

## 2. README / docs.rs Alignment

| Crate | README Posture | Standalone-Ready? | Redirect/Thin-Wrapper Signal | Downstream Contract Impact | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | README now explains facade role, default `metrics`, optional features, and ordinary use in [README.md](/home/adam/projects/canic/crates/canic/README.md) | yes | still redirects to workspace docs for fuller setup, but only after stating the crate-local contract | low | none | Low |
| `canic-cdk` | README explains role, re-export shape, and gives a small example in [README.md](/home/adam/projects/canic/crates/canic-cdk/README.md) | yes | mild redirect to workspace install docs only | low | none | Low |
| `canic-memory` | README explains role, standalone use, and public API flows in [README.md](/home/adam/projects/canic/crates/canic-memory/README.md) | yes | no problematic redirect signal | low | none | Low |
| `canic-testkit` | README clearly limits public surface and points internal harness code elsewhere in [README.md](/home/adam/projects/canic/crates/canic-testkit/README.md) | yes | none | low | none | Low |
| `canic-installer` | README explicitly describes installed script/binary surface and who should use the crate directly in [README.md](/home/adam/projects/canic/crates/canic-installer/README.md) | yes | no mismatch; role-specific docs are explicit | low | none | Low |
| `canic-wasm-store` | README now states who should use the crate and that most users should start from `canic` in [README.md](/home/adam/projects/canic/crates/canic-wasm-store/README.md) | yes for its narrow role | thin by design, but no longer underspecified about intended audience | low | none | Low |
| `canic-core`, `canic-control-plane`, `canic-dsl-macros` | package-local README now present and aligned with lower-level role | yes | each README now says to prefer `canic` unless a lower-level crate is specifically needed | low | none | Low |

## 3. Example / Binary Surface

| Crate | Surface Item | Surface Type | What It Implies To Users | Supported / Intended? | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `examples/minimal_root.rs` plus facade-oriented README wording | example + README example posture | ordinary users should start from the facade | intended | none | Low |
| `canic-cdk` | README example using `canic::cdk` | README example posture | most users should still come through the facade path | intended; README says so explicitly | none | Low |
| `canic-installer` | six published binaries in [Cargo.toml](/home/adam/projects/canic/crates/canic-installer/Cargo.toml) and binary inventory in [README.md](/home/adam/projects/canic/crates/canic-installer/README.md) | installed binary surface | installed crate owns real build/staging/install workflows | intended, and the README now scopes that audience more clearly | pressure | Low |
| `canic-wasm-store` | no binaries/examples; README says canonical role-only crate | narrow role README posture | narrow role crate, not a general facade | intended | none | Low |

## 4. Feature / Package Contract Alignment

| Crate | Feature / Package Lever | Default? | What It Widens | Docs / README Alignment | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `metrics` feature in [Cargo.toml](/home/adam/projects/canic/crates/canic/Cargo.toml) | yes | ordinary facade users now get `canic_metrics` by default unless they opt out | package README now explains this directly in [README.md](/home/adam/projects/canic/crates/canic/README.md) | low pressure only | Low |
| `canic` | `control-plane`, `sharding`, `auth-crypto` features | no | widens facade into optional support/runtime roles | acceptable because feature names match real ownership seams | none | Low |
| `canic-wasm-store` | runtime dependency on `canic` with `control-plane` | n/a | role crate pulls in the broad facade plus control-plane support | README makes the role explicit enough that this does not read as a general-purpose surface | pressure | Low |
| `canic-installer` | binary-heavy package posture | n/a | installed binary suite is part of the package contract, not just an internal tool | README aligns well with that contract | none | Low |

## 5. Alternate Facade / Ownership Ambiguity

| Area | Ambiguity Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic` vs `canic-core` | both are published and can look like user entry points from Cargo alone | both package-local READMEs now describe their roles, and [README.md](/home/adam/projects/canic/crates/canic-core/README.md) explicitly says most users should prefer `canic` | low pressure only | Low |
| `canic` vs `canic-memory` | both can serve memory-related users | `canic-memory` now has explicit standalone README/API posture in [README.md](/home/adam/projects/canic/crates/canic-memory/README.md), which reduces confusion materially | pressure, but bounded | Low |
| `canic` vs `canic-cdk` | both can appear as import surfaces for CDK helpers | `canic-cdk` README explicitly says most users should access it via `canic::cdk` in [README.md](/home/adam/projects/canic/crates/canic-cdk/README.md) | low pressure only | Low |

## 6. Publish Surface Risk Index

| Category | Risk Index (1-10, lower is better) | Basis |
| --- | ---: | --- |
| Manifest Publish Discipline | 1 | published crate set is intentional and metadata is broadly present; the earlier lower-level `readme = false` pressure is gone |
| README / Docs Contract Clarity | 2 | main standalone support crates are clear, and the lower-level published crates now have crate-local role guidance |
| Example / Binary Surface Discipline | 2 | examples and binaries inspected in this run matched intended roles, and `canic-installer` now states its intended direct audience more clearly |
| Feature / Default Surface Discipline | 2 | `canic` still has one explicit default-enabled public feature (`metrics`), but the crate-local README now explains that default clearly |
| Facade / Ownership Clarity | 3 | the remaining ambiguity is mostly inherent public overlap between `canic` and its lower-level published support crates, not missing package-local guidance |

## Overall Publish Surface Risk Index

**2 / 10**

Interpretation:
- low package-surface pressure
- no confirmed High/Critical publish-surface violation
- main remaining pressure is just broad intentional public overlap and the intentionally thin `canic` facade README, not misleading package contracts

## Delta Since Baseline

| Delta Type | Crate / Surface | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| audit method | entire audit family | `publish-surface-v1` | `publish-surface-v2` | retained report now explicitly tracks default-feature contract pressure and publishable-but-underspecified crates under the revised wording |

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| published crate manifest inspection | PASS | package metadata, `publish` posture, README/docs.rs fields, binary lists, and feature posture inspected directly in the published crate manifests |
| package-local README inspection | PASS | `canic`, `canic-cdk`, `canic-memory`, `canic-testkit`, `canic-installer`, and `canic-wasm-store` README posture inspected directly |
| public surface alignment judgment | PASS | no README, binary, or feature surface inspected in this run materially implied unsupported internal-only use |
| publish-surface judgment | PASS | no High/Critical publish-surface violation confirmed; only Low/Medium pressure remains |
