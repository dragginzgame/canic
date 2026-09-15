#!/usr/bin/env bash
# Disposable, offline ICP build-selection probe; never compiles or installs Canic.
set -euo pipefail

icp_audit_bin="${CANIC_TEST_ICP_BIN:?set CANIC_TEST_ICP_BIN to the native ICP 1.5.0 binary}"
[[ "$icp_audit_bin" = /* ]]
[[ "$("$icp_audit_bin" --version)" = "icp 1.5.0" ]]
audit_root="$(mktemp -d)"
trap 'rm -rf "$audit_root"' EXIT
mkdir -p "$audit_root/input" "$audit_root/runtime"
export ICP_HOME="$audit_root/runtime/icp" TMPDIR="$audit_root/runtime"
export CI=1 ICP_TELEMETRY_DISABLED=1 DO_NOT_TRACK=1
unset ICP_ENVIRONMENT ICP_CLI_ENVIRONMENT ICP_NETWORK ICP_IDENTITY ICP_PROJECT_ROOT

cat > "$audit_root/input/icp.yaml" <<'YAML'
canisters:
  - name: selected
    build:
      steps:
        - type: script
          commands:
            - bash probe-build.sh selected
  - name: excluded
    build:
      steps:
        - type: script
          commands:
            - bash probe-build.sh excluded
environments:
  - name: proof
    network: ic
    canisters: [selected]
  - name: empty
    network: ic
    canisters: []
YAML
cat > "$audit_root/input/probe-build.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\t%s\t%s\n' "$1" "$ICP_CLI_ENVIRONMENT" "${ICP_ENVIRONMENT-unset}" >> observed.tsv
# Minimal empty Wasm module; no Cargo or canister runtime executes.
printf '\000asm\001\000\000\000' > "$ICP_WASM_OUTPUT_PATH"
SH

run_icp() {
    "$icp_audit_bin" --project-root-override "$audit_root/input" "$@"
}
run_icp build -e proof > "$audit_root/runtime/build.log" 2>&1 || {
    cat "$audit_root/runtime/build.log"
    exit 1
}
[[ "$(cat "$audit_root/input/observed.tsv")" = $'selected\tproof\tunset' ]]
printf 'explicit_environment\tselected\tproof\tunset\tPASS\n'

rm "$audit_root/input/observed.tsv"
ICP_ENVIRONMENT=local run_icp build -e proof > "$audit_root/runtime/build.log" 2>&1
[[ "$(cat "$audit_root/input/observed.tsv")" = $'selected\tproof\tlocal' ]]
printf 'conflicting_inherited_default\tselected\tproof\tlocal\tPASS\n'

rm "$audit_root/input/observed.tsv"
run_icp build -e empty > "$audit_root/runtime/build.log" 2>&1
[[ ! -e "$audit_root/input/observed.tsv" ]]
printf 'empty_environment\tno_build\tPASS\n'

if run_icp build -e proof excluded > "$audit_root/runtime/rejection.log" 2>&1; then
    printf 'excluded_canister_was_accepted\tFAIL\n'
    exit 1
fi
[[ ! -e "$audit_root/input/observed.tsv" ]]
printf 'explicit_excluded_canister\trejected_before_build\tPASS\n'

if run_icp build -e missing > "$audit_root/runtime/rejection.log" 2>&1; then
    printf 'unknown_environment_was_accepted\tFAIL\n'
    exit 1
fi
[[ ! -e "$audit_root/input/observed.tsv" ]]
printf 'unknown_environment\trejected_before_build\tPASS\n'

run_icp project bundle -e proof -o "$audit_root/runtime/bundle.tar.gz" > "$audit_root/runtime/bundle.log" 2>&1 || {
    cat "$audit_root/runtime/bundle.log"
    exit 1
}
[[ "$(cat "$audit_root/input/observed.tsv")" = $'selected\tproof\tunset' ]]
tar -tzf "$audit_root/runtime/bundle.tar.gz" > "$audit_root/runtime/archive.txt"
rg -Fx 'icp.yaml' "$audit_root/runtime/archive.txt" > /dev/null
rg -Fx 'canisters/selected.wasm' "$audit_root/runtime/archive.txt" > /dev/null
if rg -F 'excluded' "$audit_root/runtime/archive.txt" > /dev/null; then
    printf 'bundle_contains_excluded_canister\tFAIL\n'
    exit 1
fi
tar -xOzf "$audit_root/runtime/bundle.tar.gz" icp.yaml > "$audit_root/runtime/bundled.yaml"
rg -F 'name: selected' "$audit_root/runtime/bundled.yaml" > /dev/null
if rg -F 'excluded' "$audit_root/runtime/bundled.yaml" > /dev/null; then
    printf 'bundle_manifest_contains_excluded_canister\tFAIL\n'
    exit 1
fi
printf 'environment_bundle\tselected_only_archive_and_manifest\tPASS\n'
