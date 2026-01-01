// workflow/runtime/env.rs (for example)
pub fn init_env_from_view(env_view: EnvView, role: CanisterRole) -> Result<(), Error> {
    // 1. Convert view → snapshot (workflow-level adapter)
    let mut snapshot = env_snapshot_from_view(env_view);

    // 2. Apply contextual overrides (workflow responsibility)
    snapshot.canister_role = Some(role.clone());
    snapshot = ensure_nonroot_env_snapshot(role, snapshot)?;

    // 3. Persist via ops
    EnvOps::import(snapshot)
}
