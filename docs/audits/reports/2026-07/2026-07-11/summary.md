# Audit Summary - 2026-07-11

## Run Contexts

| Report | Type | Scope | Status |
| --- | --- | --- | --- |
| [codebase-health.md](codebase-health.md) | Broad codebase health audit | workspace architecture, persistence, build boundaries, dependencies, safety, and structural pressure | PARTIAL: targeted validation complete; full test/PocketIC/Wasm matrices intentionally omitted |

## Risk Index Summary

| Report | Risk | Notes |
| --- | ---: | --- |
| `codebase-health.md` | 6 / 10 | One high restore-journal durability issue; medium global-environment and CBOR ownership issues. |

## Method / Comparability Notes

- `Codebase Health V1` is a new combined method and has no comparable prior
  baseline.
- Static scope covers the whole workspace; executable validation remains
  targeted under repository policy.

## Key Findings By Severity

- High: mutating restore execution rewrites its recovery journal in place.
- Medium: two build paths use duplicated unsafe process-global environment
  guards.
- Medium: direct unmaintained `serde_cbor` owns stable-state and IC wire bytes.
- Medium: medic, deploy-plan, and state-manifest modules are new high-churn
  structural hubs.
- Low: external diagnostic classifiers remain distributed by command boundary.

## Verification Readout Rollup

- PASS: layering guard, workspace manifest tests, changelog governance test,
  RustSec vulnerability scan, dependency ownership inspection, stable-memory
  declaration scan, and whitespace validation.
- BLOCKED by scope: full tests, PocketIC, deployment, network operations, and a
  new Wasm build.

## Follow-up Actions

1. `canic-backup`: hard-cut restore journal writes to one durable replacement
   primitive.
2. `canic-host`/`canic-cli`: remove global unsafe build environment mutation.
3. `canic-core`/`canic-host`: design and execute a single CBOR hard cut with
   stable/wire fixtures.
