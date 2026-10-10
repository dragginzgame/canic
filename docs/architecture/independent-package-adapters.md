# Independent Package Adapters

Canic coordinates independently usable blob and backup packages. Those packages
must not depend on Canic. This page defines the accepted ownership boundary and
the integration contracts to implement; it does not advertise unfinished adapters
as available APIs.

An extraction qualifies only when a non-Canic application can use the package
directly through a coherent public API and meaningful standalone tests. Moving
a Canic feature into another Cargo package does not meet that requirement.
Technical independence is necessary but insufficient: a credible non-Canic use
case must justify maintaining a separate product.

The Canic-owned blob composition lives in `integrations/blob-service`, as a
main-workspace member sharing the catalog, lockfile and release version. Its two
application consumers retain isolated qualification workspaces. The upstream
service remains independently usable and has no Canic dependency. Thin runtime hooks belong in
Core, operator orchestration in Host, and command parsing/reporting in CLI.
Creating an integration directory does not require moving those owners.

Optional runtime capabilities need not own a separate canister. The blob adapter
supports a host-owned `mount!` plus synchronous lifecycle functions and metric
collection; its dedicated `canister!` convenience API assembles those same parts.
An embedding application keeps one lifecycle, Candid export, memory bootstrap
and application sampler. Each adapter must expose composable parts rather than
silently replacing those owners. Memory grants and endpoint names must remain
disjoint. This is a composition pattern, not a generic plugin registry or a claim
that backup and authentication adapters already implement it. Host-side backup
orchestration retains its separate ownership and authority contracts below.

| Capability | Independent package owns | Canic retains |
| --- | --- | --- |
| Blob service | Storage, upload protocols, accounting, certification and service tests | Fleet endpoint guards, lifecycle participant, memory grants and application-metric sampler |
| Backup | Artifacts, checksums, manifests, journal transitions, bounded capture/restore runners, retention and command custody | Fleet discovery, exact release/controller authority, application consistency and operator integration |

When backup runner adoption is qualified, delete `crates/canic-backup` and its
duplicate mechanisms. Do not retain it as a forwarding crate. Public upstream
APIs must suffice; Canic must not copy an upstream engine into an adapter.

## Backup Adapter Contract

IC Backup supplies artifacts, durable publication, layout/reference locks,
command custody and bounded capture, metadata, download and upload stages under
retained original plans and spending. Canic delegates verified directory
publication to that owner. These individual stages do not supply a complete
Fleet backup/restore runner or Canic's live authority and application consistency.
Local file locks do not freeze a remote Fleet or prove a paid call completed.

Host must supply these observations and decisions to the eventual public runner:

1. **Selection and membership.** Bind the exact App/Fleet, canonical network,
   current release, Coordinator and owning Root. Retained Ensure inventory is a
   discovery seed only. Read current Component partitions and all directory
   pages, pin their revisions/hashes, validate provenance and uniqueness, and
   detect missing pages, newly allocated descendants and changed membership.
   Explicit subtree selection determines its owning Root even in a multi-Root
   Fleet. Authority canisters require an explicit separate capture policy;
   a Coordinator row must not enter an ordinary Root-descendant backup.
2. **Control and read authority.** Resolve the executing principal and prove
   current management controller and snapshot-read authority for each target.
   Root control does not prove direct operator control. Configuration claims
   stay declared until observed evidence satisfies the selected transport.
   Bind installed artifact hashes and the exact release to the capture intent.
3. **Consistency.** Define whether the result is per-canister crash consistency
   or an application-coordinated checkpoint. Root retirement/draining is not a
   reversible backup pause. Application coordination needs a dedicated bounded
   admission/freeze operation, a membership revision and a recoverable release.
   Re-reading a revision detects drift but does not itself prevent an intervening
   allocation or state mutation. Never label that observation a global checkpoint.
4. **Execution and retry.** Review maximum call/debit bounds and persist exact
   intent and command custody before effects. Re-observe current authority and
   consistency evidence on nonterminal resume; a historical
   `preflight_accepted` bit is insufficient. Reconcile uncertain effects before
   retry, preserve completed snapshots and original running/stopped states,
   and release quiescence after success or an interrupted capture. Terminal
   replay verifies retained completion without repeating remote effects.
5. **Restore and retention.** Restore only within the same release, bind explicit
   destination identities/controllers and verify artifact hashes before effects.
   Keep source references through interruption until terminal evidence and child
   process quiescence permit release. Prune cannot infer completion from a
   missing journal or expired local process.

The runner-facing boundary should carry named observation, authority,
consistency, budget and outcome records. Host converts Fleet-specific data in
ops and orchestrates observations in workflow; policy remains pure. Upstream
records must contain opaque adapter bindings rather than Canic DTO dependencies.
TTL expiration alone must not erase unresolved paid intent or permit a new spend.

Qualification must include membership change between pages and between capture
steps, wrong controller/network/release, missing read authority, interrupted
stop/snapshot/start, lost responses, expired preflight on resume and exact
effect-free terminal replay. Simulator tests belong to the relevant owning
package; Canic needs focused evidence for Fleet bindings and coordination.
Until these contracts are implemented, live backup creation remains unavailable.
The existing gap is tracked in [Canic #394](https://github.com/dragginzgame/canic/issues/394).

## Reporting And Consumer Use

Applications opt into the package they use. Blob usage already flows through
Canic's generic application metrics. Backup adapters should likewise expose
bounded aggregate health/counters through existing reporting surfaces, without
public identities, credentials or backup paths.
Neither package adoption nor an adapter build qualifies a downstream deployment.

See the [blob composition guide](../features/blob-storage/README.md),
[backup availability](../features/backup-and-restore/README.md), and
[authentication architecture](authentication.md).
