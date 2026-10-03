# Builds, Provenance, And Evidence

IC canisters are deployed as **WebAssembly (Wasm)** files. Canic builds those
files and records **provenance**: evidence about the source, configuration,
tools, and dependencies that produced them. Operators and automated checks can
compare that evidence before deciding to deploy anything.

Building, inspecting evidence, and deploying are deliberately separate actions.
An evidence check can report a problem, but it cannot modify a Fleet.

<p align="center">
  <a href="../../../assets/canic-build-deploy.jpg">
    <img src="../../../assets/canic-build-deploy.jpg" alt="Canic turns Rust source and App configuration into qualified build artifacts and evidence before a desired Fleet plan is reviewed and applied" width="650" />
  </a>
</p>

`canic build <app> --json` writes one schema-1 result to stdout; progress and
tool diagnostics stay on stderr. Read `release_build_id` and `release_manifest`
for a complete App build. A cache hit returns the same identity with `reused: true`.
For a selected role, `artifact` contains the Wasm, compressed Wasm and Candid paths,
and the release fields are `null`. `role`, `artifact` and both release fields are
always present. Automation should use these fields instead of parsing build
headings or artifact tables. A failing build returns a nonzero exit status.

Application workspaces use Cargo's resolver selection and defaults. Canic validates
the resolved dependencies and role features without requiring an explicit resolver
declaration or a particular resolver version. Canic's own workspace and generated
Fleet packages select `resolver = "3"`.

## What It Provides

- role-aware Wasm and Candid artifact construction
- deterministic artifact identities and checked configuration inputs
- optional build-provenance output for CI and release pipelines
- generic evidence envelopes and policy manifests
- passive policy gates over previously captured evidence

Evidence envelopes are designed for transport and comparison in CI. They
preserve the underlying report and its input fingerprints without claiming
that a deployment mutation happened.

The host generates thin Cargo packages for Root, Coordinator and Store under
`.canic/generated/<config-file>/` beside the selected configuration. Sibling
configurations have independent manifests and lockfiles. All three bind the exact
Canic dependency and use unpublished build packages. Root selects configured
capabilities, Store compiles its configuration, and Coordinator remains
independent of App configuration. Cargo graphs are validated before artifact
finalization. Canonical Coordinator/Store Candid ships in `canic/candid`; Root
Candid follows its configured capabilities.

Generated locks are derived from the selected workspace's current `Cargo.lock`.
A change to that lock or the generated manifest atomically refreshes the seed;
Cargo then resolves the generated package's graph. Unchanged inputs preserve that
resolved graph. The `lock-seed.json` record identifies the derivation, while cache
and provenance evidence fingerprint the actual resolved lock. Complete-build reuse
prepares these inputs before taking its snapshot.

Application and Root builds select the Wasm named in Cargo's compiler-artifact
messages for the exact package manifest. Custom library names are supported.
Each declaration batch's Candid is extracted, and each runtime batch's bytes are
captured, before another workspace can overwrite a shared Cargo output filename.
An unreported stale file never becomes the selected artifact.

Every final artifact, including a Local build, must fit Canic's supported
10 MiB code-section and 50,000 defined-function ceilings before the Wasm,
Candid and gzip outputs are published. Rejection preserves any previous output
set and reports the offending artifact and measured limit. These are build
admission checks, not a substitute for replica validation of the whole module.

## Boundary

Build provenance is not runtime attestation. Evidence and policy commands do
not install canisters, change controllers, sign artifacts, import registries,
or adopt discovered resources. Ordinary deployment convergence belongs to the
reviewed `canic fleet ensure` workflow; bootstrap, capacity import and other
explicit control operations retain their own reviewed authority.

## Continue From Here

- [Build artifact architecture](../../architecture/build-artifacts.md)
- [Evidence envelopes](../../architecture/evidence-envelopes.md)
- [CI policy gates](../../architecture/ci-policy-gates.md)
- [Managed-App qualification](managed-app-qualification.md)
- [Fixture build artifacts](fixture-artifacts.md)
- [Operator walkthrough](../../architecture/v1-operator-walkthrough.md)
- [Choose another feature](../README.md)
- [Browse all documentation](../../README.md)
