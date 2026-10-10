#!/usr/bin/env bash
# Shared companions: scripts/ci/check-crates-io-version.sh
set -euo pipefail
root="$0"
[[ "$root" == /* ]] || root="$PWD/$root"
root="$(cd -P "${root%/*}/../.." && printf '%s/.' "$PWD")"
root="${root%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/registry-observation.XXXXXX")"
fixture_complete=false
finish() {
    local status=$?
    # Bash 3.2 may report zero after nounset before assertions finish.
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "Registry observation fixtures retained: $fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
mkdir "$fixture/bin"
export REGISTRY_FIXTURE="$fixture"
cat > "$fixture/bin/curl" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" == '--disable --version' ]]; then
    printf '%s\n' "${REGISTRY_CURL_VERSION:-curl 8.4.0 fixture}"
    exit 0
fi
printf '%s\n' "$@" > "$REGISTRY_FIXTURE/args"
printf '%s\n' called >> "$REGISTRY_FIXTURE/calls"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output)
            if [[ "$2" != /dev/null ]]; then
                cp "$REGISTRY_BODY" "$2"
                if [[ "${REGISTRY_BREAK_EVIDENCE:-false}" == true ]]; then mkdir "${2%/*}/http-status"; fi
            fi
            shift 2 ;;
        --stderr) printf 'fixture transport diagnostics\n' > "$2"; shift 2 ;;
        *) shift ;;
    esac
done
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
    [[ "$status" == "$expected" && "$(wc -l < "$fixture/calls")" -eq 1 ]] || exit 1
done
# Even HTTP-looking output from an unsuccessful transport is unknown.
for http in 200 404; do
    for transport in 6 28 35 60; do
        status=0
        REGISTRY_HTTP="$http" REGISTRY_TRANSPORT="$transport" bash "$root/scripts/ci/check-crates-io-version.sh" fixture_crate 0.1.2 \
            > "$fixture/output" 2>&1 || status=$?
        [[ "$status" == 2 ]] || exit 1
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
    [[ "$status" == 2 ]] || exit 1
done
for version in '' 01.1.2 v0.1.2 0.1.2-rc.1 0.1.2+build '0.1.2/other' '0.1.2?x'; do
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" fixture "$version" > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]] || exit 1
done
for count in 0 1 3; do
    args=()
    while [[ "${#args[@]}" -lt "$count" ]]; do args+=(fixture); done
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" ${args[@]+"${args[@]}"} > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]] || exit 1
done
[[ ! -s "$fixture/calls" ]] || exit 1

# Metadata mode retains each response, validates one exact identity, and never
# turns inconclusive transport/JSON into permission to publish.
export REGISTRY_BODY="$fixture/body" REGISTRY_HTTP=200 REGISTRY_TRANSPORT=0
jq -nc '{version:{crate:"fixture_crate",num:"0.1.2",checksum:("a" * 64),yanked:false}}' > "$fixture/valid"
cp "$fixture/valid" "$REGISTRY_BODY"
attempt=0
observe() {
    local expected="$1" status=0
    attempt=$((attempt + 1))
    evidence="$fixture/observation $attempt"$'\n'
    : > "$fixture/calls"
    bash "$root/scripts/ci/check-crates-io-version.sh" --metadata "$evidence" fixture_crate 0.1.2 \
        > "$fixture/stdout" 2> "$fixture/stderr" || status=$?
    [[ "$status" == "$expected" && "$(wc -l < "$fixture/calls")" -eq 1 ]] || exit 1
    cmp "$REGISTRY_BODY" "$evidence/response.json"
    [[ "$(cat "$evidence/http-status")" == "$REGISTRY_HTTP" ]] || exit 1
    [[ "$(cat "$evidence/curl-exit")" == "$REGISTRY_TRANSPORT" ]] || exit 1
    [[ -s "$evidence/curl.stderr" && -s "$evidence/request.url" && -s "$evidence/curl-version.txt" ]] || exit 1
    if [[ "$expected" == 0 ]]; then
        jq -c '.version | {crate, version:.num, checksum, yanked}' "$REGISTRY_BODY" > "$fixture/expected-metadata"
        cmp "$fixture/expected-metadata" "$fixture/stdout"
        cmp "$fixture/stdout" "$evidence/metadata.json"
    else
        [[ ! -s "$fixture/stdout" && ! -s "$evidence/metadata.json" ]] || exit 1
    fi
}
observe 0
grep -Fx -- --max-filesize "$fixture/args"
grep -Fx 1048576 "$fixture/args"
# Existing evidence (including a symlink to it) cannot be overwritten or queried again.
cp "$evidence/response.json" "$fixture/retained"
ln -s "$evidence" "$fixture/evidence-link"
for existing in "$evidence" "$fixture/evidence-link" "$fixture/valid" "$fixture/missing/parent"; do
    : > "$fixture/calls"
    status=0
    bash "$root/scripts/ci/check-crates-io-version.sh" --metadata "$existing" fixture_crate 0.1.2 \
        > "$fixture/stdout" 2> "$fixture/stderr" || status=$?
    [[ "$status" == 2 && ! -s "$fixture/calls" && ! -s "$fixture/stdout" ]] || exit 1
done
cmp "$fixture/retained" "$evidence/response.json"

# Yanked is an observed fact; the consumer owns its acceptance policy.
jq '.version.yanked = true' "$fixture/valid" > "$REGISTRY_BODY"
observe 0
for http in 404 401 429 503 000 malformed; do
    REGISTRY_HTTP="$http" observe "$([[ "$http" == 404 ]] && echo 1 || echo 2)"
done
for http in 200 404; do
    for transport in 6 28 35 60 63; do
        REGISTRY_HTTP="$http" REGISTRY_TRANSPORT="$transport" observe 2
    done
done
for field in crate num checksum yanked; do
    jq --arg field "$field" 'del(.version[$field])' "$fixture/valid" > "$REGISTRY_BODY"
    observe 2
    for invalid in null 0 '[]' '{}' '"wrong"'; do
        jq --arg field "$field" --argjson value "$invalid" '.version[$field] = $value' "$fixture/valid" > "$REGISTRY_BODY"
        observe 2
    done
done
for checksum in '("a" * 63)' '("a" * 65)' '("A" * 64)' '(("a" * 63) + "\n")'; do
    jq ".version.checksum = $checksum" "$fixture/valid" > "$REGISTRY_BODY"
    observe 2
done
for body in '' 'not json' 'null' '[]' '{}' '{"version":[]}' '{"version":false}'; do
    printf '%s' "$body" > "$REGISTRY_BODY"
    observe 2
done
cat "$fixture/valid" "$fixture/valid" > "$REGISTRY_BODY"
observe 2
cat "$fixture/valid" > "$REGISTRY_BODY"
printf '\ninvalid trailing input' >> "$REGISTRY_BODY"
observe 2
# Even a transport substitute ignoring its limit cannot feed oversized JSON
# into the parser. Keep the rejected body as failure evidence.
cp "$fixture/valid" "$REGISTRY_BODY"
perl -e 'print " " x (1048577 - (-s $ARGV[0]))' "$fixture/valid" >> "$REGISTRY_BODY"
observe 2
cp "$fixture/valid" "$REGISTRY_BODY"
status=0
REGISTRY_BREAK_EVIDENCE=true bash "$root/scripts/ci/check-crates-io-version.sh" \
    --metadata "$fixture/write-failure" fixture_crate 0.1.2 > "$fixture/stdout" 2> "$fixture/stderr" || status=$?
[[ "$status" == 2 && ! -s "$fixture/stdout" ]] || exit 1
cmp "$REGISTRY_BODY" "$fixture/write-failure/response.json"
for curl_version in 'curl 7.88.1 fixture' 'curl 8.3.0 fixture' 'not a version'; do
    : > "$fixture/calls"
    status=0
    REGISTRY_CURL_VERSION="$curl_version" bash "$root/scripts/ci/check-crates-io-version.sh" \
        --metadata "$fixture/old-curl" fixture_crate 0.1.2 > "$fixture/stdout" 2> "$fixture/stderr" || status=$?
    [[ "$status" == 2 && ! -e "$fixture/old-curl" && ! -s "$fixture/calls" && ! -s "$fixture/stdout" ]] || exit 1
done
# The older presence-only contract does not gain the metadata prerequisites.
REGISTRY_CURL_VERSION='curl 7.88.1 fixture' REGISTRY_HTTP=200 \
    bash "$root/scripts/ci/check-crates-io-version.sh" fixture_crate 0.1.2
for args in --metadata '--metadata only-directory'; do
    status=0
    # Intentional splitting exercises missing positional inputs.
    # shellcheck disable=SC2086
    bash "$root/scripts/ci/check-crates-io-version.sh" $args > /dev/null 2>&1 || status=$?
    [[ "$status" == 2 ]] || exit 1
done
echo 'Exact registry presence, metadata and retained evidence tests passed (curl substitute)'
fixture_complete=true
