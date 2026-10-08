#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/runner-disk-test.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf "$fixture"; else printf "Disk fixtures retained: %s\n" "$fixture" >&2; fi' EXIT
checker="$ROOT/scripts/ci/check-runner-disk-space.sh"
mkdir -p "$fixture/bin" "$fixture/workspace [one]" "$fixture/-selected"
printf 'retained build/evidence input\n' > "$fixture/workspace [one]/artifact"

# Real host observation: this runs on Linux and both native macOS CI runners.
BLOCK_SIZE=1M BLOCKSIZE=1M DF_BLOCK_SIZE=1M POSIXLY_CORRECT=1 \
    "$BASH" "$checker" --path "$fixture/workspace [one]" --min-free-mib 0 \
    --diagnostic-path "$fixture/workspace [one]" > "$fixture/native.log"

export DISK_FIXTURE="$fixture"
cat > "$fixture/bin/df" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
[[ "$LC_ALL" == C && $# == 2 && "$1" == -Pk ]]
printf '%s\n' "$2" > "$DISK_FIXTURE/df-path"
cat "$DISK_FIXTURE/df-output"
exit "${DISK_DF_STATUS:-0}"
SCRIPT
cat > "$fixture/bin/du" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
[[ "$LC_ALL" == C && $# == 2 && "$1" == -sk ]]
printf '%s\n' "$2" >> "$DISK_FIXTURE/du-paths"
printf '4\t%s\n' "$2"
exit "${DISK_DU_STATUS:-0}"
SCRIPT
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
cd "$fixture"

expect_status() {
    local expected="$1" status=0
    shift
    "$BASH" "$checker" "$@" > "$fixture/output.log" 2>&1 || status=$?
    [[ "$status" == "$expected" ]] || {
        cat "$fixture/output.log" >&2
        printf 'expected disk status %s, got %s\n' "$expected" "$status" >&2
        exit 1
    }
}
capacity() {
    printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n' > "$fixture/df-output"
    printf '/dev/fixture 100000 2000 %s 2%% /mount with spaces\n' "$1" >> "$fixture/df-output"
}

capacity 4096
expect_status 0 --path 'workspace [one]' --min-free-mib 4 --label 'before build'
[[ "$(< "$fixture/df-path")" == './workspace [one]' && ! -e "$fixture/du-paths" ]]
grep -F 'before build: 4 MiB available; 4 MiB required' "$fixture/output.log" > /dev/null
capacity 4095
expect_status 1 --path 'workspace [one]' --min-free-mib 4
capacity -1
expect_status 1 --path . --min-free-mib 0
capacity 0
expect_status 0 --path . --min-free-mib 0
capacity 999999999999999999
expect_status 1 --path . --min-free-mib 999999999999999999
expect_status 0 --path . --min-free-mib 976562499999999

capacity 4096
# du warnings never mask insufficient capacity or reject a valid capacity gate.
export DISK_DU_STATUS=7
expect_status 1 --path -selected --min-free-mib 5 --diagnostic-path -selected
[[ "$(< "$fixture/df-path")" == './-selected' ]]
grep -F 'warning: disk usage unavailable' "$fixture/output.log" > /dev/null
expect_status 0 --path . --min-free-mib 4 --diagnostic-path 'workspace [one]' --diagnostic-path missing
[[ "$(wc -l < "$fixture/du-paths" | tr -d ' ')" == 3 ]]
grep -Fx './workspace [one]' "$fixture/du-paths" > /dev/null
grep -Fx './missing' "$fixture/du-paths" > /dev/null
unset DISK_DU_STATUS

# A successful-looking partial response from a failed command is not admitted.
export DISK_DF_STATUS=23
expect_status 2 --path . --min-free-mib 0
unset DISK_DF_STATUS
for payload in '' '/dev/x 100 10 unknown 10% /' \
    '/dev/x 100 10 1000000000000000000 10% /' \
    $'/dev/x 100 10 90 10% /\n/dev/y 100 10 90 10% /' \
    '/dev/x 100 10 90 invalid /'; do
    printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n%s\n' "$payload" > "$fixture/df-output"
    expect_status 2 --path . --min-free-mib 0
done
printf 'Filesystem 512-blocks Used Available Capacity Mounted on\n/dev/x 100 10 90 10%% /\n' > "$fixture/df-output"
expect_status 2 --path . --min-free-mib 0
: > "$fixture/df-output"
expect_status 2 --path . --min-free-mib 0
capacity 4096
for threshold in -1 1.5 01 1000000000000000000 invalid; do
    expect_status 2 --path . --min-free-mib "$threshold"
done
expect_status 2
expect_status 2 --path .
expect_status 2 --path
expect_status 2 --path missing --min-free-mib 0
expect_status 2 --path . --min-free-mib 0 --label $'two\nlines'
expect_status 2 --path . --min-free-mib 0 --diagnostic-path $'tab\tpath'
expect_status 2 --path . --min-free-mib 0 --unknown
[[ "$(< "$fixture/workspace [one]/artifact")" == 'retained build/evidence input' ]]
echo 'Runner disk capacity checks passed'
