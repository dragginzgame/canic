#!/usr/bin/env bash
set -euo pipefail
root="$0"
[[ "$root" == /* ]] || root="$PWD/$root"
root="$(cd -P "${root%/*}/../.." && printf '%s/.' "$PWD")"
root="${root%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/registry-observation.XXXXXX")"
finish() {
    local status=$?
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "Registry observation fixtures retained: $fixture" >&2; fi
}
trap finish EXIT
mkdir "$fixture/bin"
export REGISTRY_FIXTURE="$fixture"
cat > "$fixture/bin/curl" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "$REGISTRY_FIXTURE/args"
printf '%s\n' called >> "$REGISTRY_FIXTURE/calls"
printf '%s' "$REGISTRY_HTTP"
exit "${REGISTRY_TRANSPORT:-0}"
STUB
chmod +x "$fixture/bin/curl"
export PATH="$fixture/bin:$PATH"
for http in 200 404 401 429 503 000 malformed '200404'; do
    : > "$fixture/calls"
    expected=2
    case "$http" in 200) expected=0 ;; 404) expected=1 ;; esac
    status=0
    REGISTRY_HTTP="$http" bash "$root/scripts/ci/check-crates-io-version.sh" fixture_crate 0.1.2 \
        > "$fixture/output" 2>&1 || status=$?
    [[ "$status" == "$expected" && "$(wc -l < "$fixture/calls")" -eq 1 ]]
done
# Even HTTP-looking output from an unsuccessful transport is unknown.
for http in 200 404; do
    for transport in 6 28 35 60; do
        status=0
        REGISTRY_HTTP="$http" REGISTRY_TRANSPORT="$transport" bash "$root/scripts/ci/check-crates-io-version.sh" fixture_crate 0.1.2 \
            > "$fixture/output" 2>&1 || status=$?
        [[ "$status" == 2 ]]
    done
done
cat > "$fixture/expected" <<'ARGS'
--disable
--silent
--show-error
--location
--proto
=https
--proto-redir
=https
--tlsv1.2
--connect-timeout
10
--max-time
30
--output
/dev/null
--write-out
%{http_code}
--user-agent
dragginzgame-shared-tooling (https://github.com/dragginzgame/shared-tooling)
https://crates.io/api/v1/crates/fixture_crate/0.1.2
ARGS
cmp "$fixture/expected" "$fixture/args"
: > "$fixture/calls"
for crate in '' '-option' 'a/b' 'a?b' 'a b'; do
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" "$crate" 0.1.2 > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]]
done
for version in '' 01.1.2 v0.1.2 0.1.2-rc.1 0.1.2+build '0.1.2/other' '0.1.2?x'; do
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" fixture "$version" > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]]
done
for count in 0 1 3; do
    args=()
    while [[ "${#args[@]}" -lt "$count" ]]; do args+=(fixture); done
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" ${args[@]+"${args[@]}"} > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]]
done
[[ ! -s "$fixture/calls" ]]
echo 'Exact registry observation status and request tests passed (curl substitute)'
