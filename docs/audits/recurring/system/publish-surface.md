# Audit: Publish Surface / Package Contract Discipline

`Cargo.toml` package posture plus published crate docs, examples, and binary surface

## Purpose

Verify that published crate/package posture remains:

* intentional
* documented
* package-safe
* downstream-comprehensible
* aligned with actual supported use

This audit measures **publish surface discipline**, **package-contract clarity**,
and **docs/examples alignment** for crates that are published or plausibly
publishable.

It does **NOT** evaluate:

* correctness
* runtime performance
* internal module visibility
* crate dependency direction except where it changes package posture
* redesign ideas unless package drift is severe enough that the report cannot
  describe the problem accurately without naming the corrective direction

---

## Audit Objective

Determine whether the workspace’s published crates still present a clean,
intentional downstream contract, or whether they have drifted toward:

* crates that are publishable but not actually documented for standalone use
* README/docs/examples that imply unsupported use
* binaries, examples, or features that widen the apparent package contract
* crate roles that are unclear from package metadata and docs posture
* alternate public entry surfaces that are not explained as such
* crates that only package cleanly because the whole workspace is present
* docs.rs or package metadata that underspecify who the crate is for
* public crates whose default package surface is broader than the intended user story

This is a **package contract audit**, not a runtime audit.

---

## Risk Model / Structural Invariant

Primary publish-surface risks:

* published crates may look broader or more general than they really are
* public support crates may silently become alternate facades
* docs/examples may imply contracts the crate does not actually own
* thin crates may ship without enough standalone context for downstream users
* binary/tooling crates may expose installed commands without documenting scope
* default features may widen what users think the ordinary package contract is
* workspace-local assumptions may leak into public package posture

Structural invariant:

> Each published crate should present a package surface that matches its stated
> role, with README/docs/examples/features consistent with what downstream users
> are actually expected to rely on.

---

## Why This Matters

Even when dependency structure is clean, publish posture can still drift into a
shape where:

* docs.rs pages are thin or misleading
* downstream users choose the wrong crate because roles are unclear
* examples imply unsupported standalone use
* public support crates become de facto alternate facades by omission
* package metadata says “publishable” while documentation still assumes
  workspace-local context
* binaries or examples imply workflows the crate does not actually support as a
  standalone package

This audit exists to catch that drift before it becomes release confusion.

---

## Run This Audit After

* adding a new published crate
* changing crate `publish` posture
* adding/removing public binaries or examples
* major README or docs.rs posture changes
* public facade cleanup passes
* default-feature changes in published crates
* publishing/support-crate productization work
* installer/tooling surface changes
* changing package metadata fields that affect downstream discovery

---

## Report Preamble (Required)

Every report generated from this audit must include:

* Scope
* Compared baseline report path
* Code snapshot identifier
* Method tag/version
* Comparability status
* Exclusions applied
* Notable methodology changes vs baseline

Required fields:

* **Scope**: exact published/publishable crates and package artifacts reviewed
* **Compared baseline report path**: prior comparable report path or `N/A`
* **Code snapshot identifier**: commit SHA, tree hash, or equivalent
* **Method tag/version**: e.g. `publish-surface-current`
* **Comparability status**:

  * `comparable`
  * `non-comparable: <reason>`
* **Exclusions applied**: explicit list, or `none`
* **Notable methodology changes vs baseline**: explicit list, or `none`

If package-local docs, examples, or binaries were not inspected for a crate, the
report must either narrow the judgment for that crate or mark it `BLOCKED`.

---

## Audit Rules (Mandatory)

### Evidence Standard

Every non-trivial claim MUST identify:

* crate or manifest
* package field, README/docs/example/binary surface, or feature that shapes the package contract
* whether the concern is metadata posture, docs posture, example posture, feature posture, or binary posture
* downstream exposure impact

Additional rules:

* Medium, High, and Critical findings MUST be supported by direct manifest or README/docs/example inspection.
* Package-surface claims must name the exact field or artifact that creates the concern.
* “Thin docs posture” is valid only when the crate is published/publishable and the package metadata points downstream users at that surface.
* Example findings must distinguish:

  * examples that are intentionally facade-oriented
  * examples that imply unsupported standalone use
* Build or packaging concerns count only when they materially affect downstream expectations.
* Do not infer actual support boundaries from crate names alone.
* If docs.rs metadata is relevant, name the specific field or omission, such as:

  * `documentation`
  * `readme`
  * `homepage`
  * `repository`
  * docs.rs metadata configuration when present

### Severity Rules

* **Low**: acceptable thinness or mild posture pressure worth monitoring
* **Medium**: avoidable ambiguity or incomplete standalone package posture
* **High**: published package surface clearly implies unsupported use or hides a meaningful contract boundary
* **Critical**: published crate is materially misleading about what is supported, or package posture defeats safe downstream use

Severity calibration rules:

* `readme = false` on a published lower-level crate is usually **pressure**, not automatically a violation.
* A crate may intentionally redirect users to a broader workspace guide; that is pressure only if the standalone role becomes unclear.
* Broad facade crates are not automatically risky; they become risky when docs/examples/features blur ownership.
* Thin lower-level crates are not automatically problematic if the package contract clearly states they are advanced/lower-level support crates.

### Counting + Comparability Rules

Definitions used throughout this audit:

* **Published crate**: crate intended to be consumed outside the local workspace
* **Publishable crate**: crate not marked `publish = false`
* **Package contract**: what a downstream user can reasonably infer from manifest metadata, README/docs.rs posture, examples, binaries, and default features
* **Standalone-ready docs posture**: README/docs posture sufficient for a user to understand the crate’s role without already knowing the workspace
* **Thin docs posture**: package surface exists but relies heavily on external/workspace docs for correct understanding
* **Example posture**: what examples or documented snippets imply about intended usage
* **Binary posture**: what installed binaries imply about supported workflows
* **Publish-surface mismatch**: package metadata/docs/examples imply a broader, different, or unsupported contract than the crate actually owns
* **Role-clarity signal**: whether metadata and docs make clear whether the crate is:

  * primary facade
  * lower-level runtime support
  * role-specific tooling
  * proc-macro support
  * testing infrastructure
* **Workspace-redirection posture**: package docs intentionally pointing users to higher-level workspace documentation while still preserving clear standalone role boundaries

Ignore unless explicitly stated otherwise:

* private/internal crates with `publish = false`, except when they accidentally look publishable
* generated docs or artifacts
* non-package-local scripts unless the README presents them as part of the public installed surface
* test-only examples
* internal examples not shipped or presented as public usage guidance

### Pressure vs Violation Rule

* **Pressure** = thin docs posture, mild ambiguity, or package-surface breadth that is still intentional
* **Violation** = published package surface materially implies the wrong contract or hides a key support boundary

Do not collapse these terms in findings.

Mandatory phrasing rule:

* call incompleteness or mild ambiguity **pressure**
* call misleading or unsupported implied contract **violation**

---

## Canonical Published Crate Map (Mandatory)

Use this map when judging intended downstream contract:

| Crate                        | Intended Package Role                                                     |
| ---------------------------- | ------------------------------------------------------------------------- |
| `crates/canic`               | main public facade and macro entry surface                                |
| `crates/canic-backup`        | backup/restore domain primitives and durable layout contracts             |
| `crates/canic-cdk`           | standalone curated IC CDK facade                                          |
| `crates/canic-cli`           | published operator CLI package exposing the `canic` binary                |
| `crates/canic-core`          | lower-level runtime/support crate, not the primary beginner entry surface |
| `crates/canic-control-plane` | lower-level control-plane support crate                                   |
| `crates/canic-host`          | host-side build/install/fleet/release-set support library                 |
| `crates/canic-macros`        | proc-macro support crate, public but not a general facade                 |
| `crates/canic-memory`        | standalone stable-memory helper/support crate                             |
| `crates/canic-testkit`       | standalone generic PocketIC/test infrastructure crate                     |
| `crates/canic-wasm-store`    | canonical published `wasm_store` canister crate                           |

Rules:

* `canic` is the primary broad facade.
* `canic-memory`, `canic-cdk`, and `canic-testkit` are intended standalone support crates.
* `canic-core`, `canic-control-plane`, and `canic-macros` may remain published while still being lower-level and thinner in documentation posture.
* `canic-backup`, `canic-host`, `canic-cli`, and `canic-wasm-store` are published role-specific crates; their binary/README posture must clearly say so.
* Lower-level published crates are allowed to be thinner than `canic`, but not allowed to be misleading about their role.

---

## Scope Defaults

Default scope for this audit includes:

* workspace root manifest when it shapes package policy
* all published/publishable crate manifests under `crates/**`
* package-local README/docs posture for those crates
* package-local examples and binaries where present
* package metadata affecting docs.rs or downstream discovery

If a crate’s package-local docs are excluded, package-contract judgment for that
crate must be marked `BLOCKED` or explicitly narrowed.

---

## Audit Checklist

### STEP 0 — Baseline Capture (Mandatory)

Produce:

| Metric                                                  | Previous | Current | Delta |
| ------------------------------------------------------- | -------: | ------: | ----: |
| Published crates reviewed                               |          |         |       |
| Published crates with thin docs posture                 |          |         |       |
| Published crates with `readme = false` pressure         |          |         |       |
| Publish-surface mismatches                              |          |         |       |
| Published crates with binary/example posture pressure   |          |         |       |
| Alternate-facade ambiguity seams                        |          |         |       |
| Published crates with default-feature contract pressure |          |         |       |
| Publishable-but-underspecified crates                   |          |         |       |

Rules:

* deltas must compare only against a comparable baseline
* if no comparable baseline exists, mark `N/A`
* do not claim trend where method or scope changed materially

---

### STEP 1 — Manifest Publish Posture

Inspect for each published/publishable crate:

* `publish`
* `name`
* `description`
* `readme`
* `documentation`
* `repository`
* `homepage`
* `license` / `license-file` when relevant to package clarity
* binary targets and example targets where present
* docs.rs metadata or other package metadata affecting downstream rendering/discovery

Produce:

| Crate | Publish Intent | `publish` Posture | README / docs.rs Metadata | Binary / Example Surface | Package Contract Clarity | Risk |
| ----- | -------------- | ----------------- | ------------------------- | ------------------------ | ------------------------ | ---- |

Rules:

* “Package Contract Clarity” must reflect what a downstream user can infer from the package without prior workspace context.
* A crate may be intentionally publishable but thin; mark that as pressure only if the role is still understandable.
* If metadata is missing but the role is still clear from the README and examples, do not overstate.

---

### STEP 2 — README / docs.rs Alignment

Inspect:

* package-local README
* crate-level docs when used as public entry docs
* docs.rs-facing posture if package metadata points there
* explicit role statements
* installation/use guidance where relevant
* disclaimers or redirect language for lower-level crates

Produce:

| Crate | README Posture | Standalone-Ready? | Redirect/Thin-Wrapper Signal | Downstream Contract Impact | Pressure or Violation | Risk |
| ----- | -------------- | ----------------- | ---------------------------- | -------------------------- | --------------------- | ---- |

Rules:

* “Standalone-Ready” does not require tutorial-level docs; it requires enough clarity for users to understand the crate’s role and intended audience.
* Redirects to higher-level docs are acceptable only if the crate’s own role remains explicit.
* Thin docs posture is pressure; misleading role or unsupported implied use is violation.

---

### STEP 3 — Example / Binary Surface

Inspect:

* examples shipped with the crate
* documented code snippets that function as examples
* public binaries installed by the package
* README commands or workflows implying installed tooling support

Produce:

| Crate | Surface Item | Surface Type | What It Implies To Users | Supported / Intended? | Pressure or Violation | Risk |
| ----- | ------------ | ------------ | ------------------------ | --------------------- | --------------------- | ---- |

Rules:

* distinguish examples intended to teach the main facade from examples that imply standalone use of lower-level crates
* binaries must clearly communicate whether they are:

  * end-user tooling
  * project scaffolding
  * release/install utilities
  * role-specific infra tools
* if an example or binary suggests a broader workflow than the crate actually supports, that is a violation

---

### STEP 4 — Feature / Package Contract Alignment

Inspect:

* default features
* public features that materially widen package surface
* feature-gated binaries/examples when relevant
* README/docs mention of feature-dependent behavior
* whether feature names reveal internal layout rather than user-facing intent

Produce:

| Crate | Feature / Package Lever | Default? | What It Widens | Docs / README Alignment | Pressure or Violation | Risk |
| ----- | ----------------------- | -------- | -------------- | ----------------------- | --------------------- | ---- |

Flag:

* default features that broaden expected package contract without documentation
* public features that only exist for workspace-local or test-only behavior
* feature names that imply unsupported or ambiguous standalone use
* package levers that make the crate appear broader than its actual supported role

Rules:

* features are a publish-surface concern only when they affect what downstream users reasonably think the crate supports
* do not treat every undocumented feature as a violation; judge based on impact to package contract

---

### STEP 5 — Alternate Facade / Ownership Ambiguity

Inspect whether published crates create public-entry ambiguity, especially:

* `canic` versus lower-level published crates
* role-specific crates whose docs do not say they are role-specific
* proc-macro crates that may look like general-purpose entry surfaces
* support crates whose README/examples make them look like the main public path

Produce:

| Area | Ambiguity Signal | Evidence | Pressure or Violation | Risk |
| ---- | ---------------- | -------- | --------------------- | ---- |

Rules:

* overlap is pressure unless a downstream user could reasonably choose the wrong crate based on package posture alone
* ambiguity must be grounded in docs/metadata/examples, not inferred from the dependency graph alone

---

### STEP 6 — Publish Surface Risk Index

Produce:

| Category                             | Risk Index (1-10, lower is better) | Basis |
| ------------------------------------ | ---------------------------------: | ----- |
| Manifest Publish Discipline          |                                    |       |
| README / Docs Contract Clarity       |                                    |       |
| Example / Binary Surface Discipline  |                                    |       |
| Feature / Default Surface Discipline |                                    |       |
| Facade / Ownership Clarity           |                                    |       |

## Overall Publish Surface Risk Index (1-10, lower is better)

Rules:

* overall score must reflect the worst real package-contract issue, not a polite average
* a confirmed High misleading-contract finding should usually push overall risk to at least `7`
* a Critical downstream-misleading package posture should usually push overall risk to at least `9`

Interpretation:

* `1-3` = low risk / package contract healthy
* `4-6` = moderate risk / manageable package-surface pressure
* `7-8` = high risk / publish posture needs attention
* `9-10` = critical risk / downstream contract instability

---

### Delta Since Baseline

Produce:

| Delta Type | Crate / Surface | Previous | Current | Impact |
| ---------- | --------------- | -------- | ------- | ------ |

Highlight only:

* new published/publishable crates
* changed `publish` posture
* changed README/docs.rs posture
* added/removed binaries or examples
* changed default features affecting package contract
* reduced or increased facade ambiguity

Rules:

* delta must compare only against a comparable baseline
* if baseline is absent or non-comparable, mark `N/A`

---

### Verification Readout (`PASS` / `FAIL` / `BLOCKED`)

Rules:

* `PASS` = no High/Critical publish-surface violations; only Low/Medium pressure
* `FAIL` = any confirmed High/Critical publish-surface violation
* `BLOCKED` = insufficient package/docs visibility for comparable judgment

If `BLOCKED`, state exactly which crate(s) or artifact(s) were not inspectable.

---

## Required Method Discipline

At minimum, the auditor must:

1. inspect the manifest of every published/publishable crate in scope
2. inspect package-local README or equivalent public docs posture for each such crate
3. inspect binaries/examples where present
4. inspect default features and public package-shaping features
5. compare package posture against the canonical crate-role map
6. separate thinness from actual contract mismatch
7. compare only against a comparable baseline

---

## Anti-Shallow Requirement

Do NOT:

* praise packaging effort
* comment on stylistic prose quality
* infer downstream contract from crate name alone
* invent unsupported publish claims from broad dependency graphs alone
* mark a crate risky merely because it is lower-level or thinly documented

Rules:

* inspect README/docs/manifest context for every Medium+ finding
* distinguish thin docs posture from actual contract mismatch
* every claim must identify crate, package field or docs/example surface, and downstream impact
* every ambiguity claim must explain what a reasonable downstream user would likely infer incorrectly

---

## Auditor Output Contract

The report must let a reviewer answer, for every finding:

* which crate is affected
* what package field, docs surface, example, binary, or feature is involved
* what a downstream user would infer from it
* why that is pressure or a violation
* whether it changed since baseline

If the report cannot support those questions, the run should be marked `BLOCKED`
rather than overstated.
