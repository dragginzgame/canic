# Chain-Key Batch Maintainability Plan

Date: 2026-06-30

## Purpose

`crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs` is the
current 0.76 root delegation renewal state-machine implementation. It is
correct enough to keep as production code, but it is large enough that future
edits should be planned before extraction.

This was completed as a no-behavior-change cleanup. Do not combine future
follow-up movement with auth feature work, proof format changes, stable record
changes, or endpoint changes.

## Current Shape

Current directory:
`crates/canic-core/src/ops/auth/delegation/chain_key_batch/`

Current file sizes after extraction:

| File | Lines | Responsibility |
| --- | ---: | --- |
| `mod.rs` | 2065 | Public ops boundary, state machine, signing orchestration, builder tests. |
| `batch_id.rs` | 83 | Local deterministic batch-id encoding. |
| `install.rs` | 83 | Signed batch to issuer proof install payloads. |
| `merkle.rs` | 109 | Merkle root, witness, and duplicate issuer helpers. |
| `selection.rs` | 88 | Due template selection, caps, pending quota helpers. |

Completed slices: all planned extraction slices were completed on 2026-06-30
with no behavior changes.

Current public ops boundary:

| Function | Current line | Responsibility |
| --- | ---: | --- |
| `prepare_due_chain_key_root_delegation_batch` | `mod.rs` | Prepare/reuse a canonical batch for due issuers. |
| `sign_next_chain_key_root_delegation_batch` | `mod.rs` | Select next prepared batch and sign it. |
| `sign_chain_key_root_delegation_batch` | `mod.rs` | Idempotently sign one persisted batch. |
| `get_or_create_chain_key_delegation_proof_for_issuer` | `mod.rs` | Lazy repair cache/sign/materialize path. |
| `start_next_chain_key_root_delegation_batch_install` | `mod.rs` | Select next signed batch for install. |
| `start_chain_key_root_delegation_batch_install` | `mod.rs` | Materialize issuer proof install payloads. |
| `record_chain_key_root_delegation_install_success` | `mod.rs` | Persist issuer install success. |
| `record_chain_key_root_delegation_install_failure` | `mod.rs` | Persist issuer install failure/retry state. |

Internal clusters:

| Cluster | Current lines | Notes |
| --- | ---: | --- |
| Proof materialization | `install.rs` | Signed batch to issuer proof/witness payload. |
| Batch state selection | `mod.rs` | Signing/install selection, retry windows, stale batch marking. |
| Due template selection | `selection.rs` | Template due checks, caps, pending quotas. |
| Batch builder | `mod.rs` | Cert leaf construction and shared policy window checks. |
| Merkle builder | `merkle.rs` | Duplicate issuer rejection, tree root, witnesses. |
| Batch id / local encoders | `batch_id.rs` | Private deterministic encoders for local batch id inputs. |
| Tests | `mod.rs` | State-machine, Merkle, signing, install, and lazy repair coverage. |

## Required Test Gate Before Any Move

Run these before the first extraction and after every slice:

```bash
cargo fmt --all -- --check
cargo test --locked -p canic-core chain_key_batch --lib
cargo test --locked -p canic-core chain_key_lazy_repair --lib
cargo test --locked -p canic-core workflow::runtime::auth --lib
cargo test --locked -p canic-core auth --lib
cargo test --locked -p canic --test protocol_surface
git diff --check
```

When a PocketIC-capable environment is available, also run:

```bash
POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic \
  cargo test --locked -p canic-tests --test root_suite auth_076 -- --nocapture --test-threads=1
```

## Extraction Order

### Slice 1: Directory Module Move Only

Completed 2026-06-30. Moved:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch.rs
```

to:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs
```

No submodules were extracted in this slice. Rust's normal module discovery now
resolves the directory module in alignment with `AGENTS.md`.

### Slice 2: Extract Pure Merkle Code

Completed 2026-06-30.

Create:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch/merkle.rs
```

Move only:

- `ChainKeyBatchLeaf`
- `reject_duplicate_chain_key_issuers`
- `merkle_root_and_witnesses`
- `MerkleNode`
- `chain_key_batch_node_hash`

Keep APIs private to `chain_key_batch`. Do not move persisted state access or
signing calls.

### Slice 3: Extract Local Batch-ID Encoding

Completed 2026-06-30.

Create:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch/batch_id.rs
```

Move only:

- `ChainKeyBatchIdInput`
- `chain_key_batch_id`
- private local encoding helpers used only by batch-id input

Do not move canonical auth encoders from `ops/auth/delegated/canonical.rs`.
The local encoders in this file are for batch-id construction only.

### Slice 4: Extract Template Selection

Completed 2026-06-30.

Create:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch/selection.rs
```

Move:

- `DueChainKeyTemplate`
- `due_chain_key_templates`
- `cap_due_chain_key_templates`
- `chain_key_template_due`
- `enabled_template_count`
- pending quota helpers

Keep `AuthStateOps` access inside ops. Do not push storage reads into policy or
workflow.

### Slice 5: Extract Install Materialization

Completed 2026-06-30.

Create:

```text
crates/canic-core/src/ops/auth/delegation/chain_key_batch/install.rs
```

Move:

- `signed_chain_key_delegation_proof_for_issuer`
- `materialize_chain_key_delegation_proof`
- install success/failure record helpers only if tests remain simple

Do not change install retry semantics. Partial issuer install retry must not
re-sign a batch.

## Do Not Change During Extraction

- `RootProof::IcChainKeyBatchSignatureV1` DTO shape.
- Stable records or memory ids.
- Root key policy validation.
- Signature message or encoding.
- Merkle witness format.
- Retry-after and stale-callback behavior.
- Lazy repair caller/auth semantics.
- Public endpoints or CLI commands.

## Stop Conditions

Stop and revert the extraction slice, not prior unrelated work, if any of these
occur:

- `cargo test --locked -p canic-core chain_key_batch --lib` fails.
- A public `pub(crate)` surface has to grow beyond the `delegation` module.
- Workflow starts constructing or mutating stable records directly.
- The diff mixes file movement with behavior changes.
- Test changes are needed for anything other than module paths or helper names.
