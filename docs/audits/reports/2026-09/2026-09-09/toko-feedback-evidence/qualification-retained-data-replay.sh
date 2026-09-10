#!/usr/bin/env bash
set -euo pipefail
source /tmp/toko-feedback-fleet/qualification-retained-context.sh
source qualification-retained-environment-after.txt
query() {
    local role="$1" canister="$2" method="$3" output="$4"
    icp canister call --environment "$ENVIRONMENT" --identity "$LOCAL_DEVELOPMENT_IDENTITY" --query \
        --candid "$artifacts/$role/$role.did" "$canister" "$method" '()' --json > "$output.raw"
    /tmp/toko-feedback-decode "$output.raw" > "$output"
}
for variable in CANIC_GAME_SHARD_1 CANIC_GAME_SHARD_2 CANIC_GAME_SHARD_3; do
    output="qualification-retained-empty-${variable}.json"
    query game_shard "${!variable}" get_my_robot "$output"
    jq -e 'has("Ok") and .Ok == null' "$output" >/dev/null
done
for variable in CANIC_USER_SHARD_1 CANIC_USER_SHARD_2 CANIC_USER_SHARD_3; do
    output="qualification-retained-empty-${variable}.json"
    query user_shard "${!variable}" get_my_game_placement "$output"
    jq -e '.Err | has("PrincipalConflict")' "$output" >/dev/null
done
icp canister call --environment "$ENVIRONMENT" --identity "$LOCAL_DEVELOPMENT_IDENTITY" \
    --candid "$artifacts/user_hub/user_hub.did" "$CANIC_USER_HUB" enroll_user \
    '(record { operation_id = "qualification-retained-user" })' --json > qualification-retained-enrol-after.json
/tmp/toko-feedback-decode qualification-retained-enrol-after.json > qualification-retained-user-after.json
jq -e --slurpfile before qualification-retained-user-before.json '.Ok.user_id != $before[0].Ok.user_id' qualification-retained-user-after.json >/dev/null
new_game=$(jq -r .Ok.game_shard qualification-retained-user-after.json)
new_user=$(jq -r .Ok.user_shard qualification-retained-user-after.json)
query game_shard "$new_game" get_my_robot qualification-retained-robot-new.json
jq -e '.Ok.Some' qualification-retained-robot-new.json >/dev/null
query user_shard "$new_user" get_my_game_placement qualification-retained-membership-new.json
export PATH="${ROOT_DIR}/qualification-retained-bin:${PATH}"
[[ "$(prepare_local_reinstall)" == "$release" ]]
converge_local_fleet reinstall
query game_shard "$new_game" get_my_robot qualification-retained-robot-replay.json
query user_shard "$new_user" get_my_game_placement qualification-retained-membership-replay.json
cmp qualification-retained-robot-new.json qualification-retained-robot-replay.json
cmp qualification-retained-membership-new.json qualification-retained-membership-replay.json
[[ $(wc -l < "$RUNTIME_DIR/install-effects.log") == 3 ]]
acknowledge_local_reinstall
[[ ! -f "$RUNTIME_DIR/reinstall-request.json" ]]
printf 'PASS: empty application data, new enrolment, completed replay preserves new data, three exact installs, acknowledgement cleared\n'
