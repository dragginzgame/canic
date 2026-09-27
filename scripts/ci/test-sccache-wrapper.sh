#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
mkdir -p "$ROOT/.tmp"
fixture="$(mktemp -d "$ROOT/.tmp/sccache-wrapper-test.XXXXXX")"
trap 'rm -rf -- "$fixture"' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin with spaces"
cp "$ROOT/scripts/ci/run-sccache.sh" "$fixture/scripts/ci/"
export CANIC_SCCACHE_BIN="$fixture/cache"
export COMPILER_RECORD="$fixture/compiler-args"
export CACHE_RECORD="$fixture/cache-args"
compiler="$fixture/bin with spaces/rustc"
wrapper="$fixture/scripts/ci/run-sccache.sh"

cat > "$compiler" <<'COMPILER'
#!/usr/bin/env bash
printf '%s\0' "$@" >> "$COMPILER_RECORD"
printf '%s\n' 'compiler stdout'
if [[ "${COMPILER_STATUS:-0}" -ne 0 ]]; then
    echo 'error: genuine compiler failure' >&2
fi
exit "${COMPILER_STATUS:-0}"
COMPILER
cat > "$CANIC_SCCACHE_BIN" <<'CACHE'
#!/usr/bin/env bash
printf '%s\0' "$@" > "$CACHE_RECORD"
case "${CACHE_MODE:-healthy}" in
    healthy) exec "$@" ;;
    unavailable)
        echo 'sccache: error: Operation not permitted (os error 1)' >&2
        exit 2 ;;
    failed)
        echo 'cache failed without an infrastructure diagnostic' >&2
        exit 7 ;;
esac
CACHE
chmod +x "$compiler" "$CANIC_SCCACHE_BIN"
# shellcheck disable=SC2016 # Literal shell syntax must survive unchanged.
printf '%s\0' 'argument with spaces' '' '--cfg=literal=$HOME' > "$fixture/expected"

run_compiler() {
    : > "$COMPILER_RECORD"
    # shellcheck disable=SC2016 # Literal shell syntax must survive unchanged.
    "$wrapper" "$compiler" 'argument with spaces' '' '--cfg=literal=$HOME' \
        > "$fixture/stdout" 2> "$fixture/stderr"
}

# Healthy cache executes exactly once and preserves argument boundaries.
run_compiler
cmp "$fixture/expected" "$COMPILER_RECORD"
[[ "$(cat "$fixture/stdout")" == 'compiler stdout' ]]
[[ ! -s "$fixture/stderr" ]]

# The reported cache connection error falls back to the original compiler argv.
CACHE_MODE=unavailable run_compiler
cmp "$fixture/expected" "$COMPILER_RECORD"
[[ "$(cat "$fixture/stdout")" == 'compiler stdout' ]]
grep -q '^sccache: warning: cache unavailable' "$fixture/stderr"

# Compiler failures, including exit 2, stay failures and are never retried.
for compiler_status in 1 2; do
    status=0
    COMPILER_STATUS="$compiler_status" run_compiler || status=$?
    [[ "$status" -eq "$compiler_status" ]]
    cmp "$fixture/expected" "$COMPILER_RECORD"
    [[ "$(cat "$fixture/stderr")" == 'error: genuine compiler failure' ]]
done

# A failed fallback returns the compiler's failure, not success or the cache code.
status=0
CACHE_MODE=unavailable COMPILER_STATUS=42 run_compiler || status=$?
[[ "$status" -eq 42 ]]
cmp "$fixture/expected" "$COMPILER_RECORD"
grep -q '^error: genuine compiler failure$' "$fixture/stderr"

# Unclassified failures and cache management commands must not invoke a compiler.
status=0
CACHE_MODE=failed run_compiler || status=$?
[[ "$status" -eq 7 && ! -s "$COMPILER_RECORD" ]]
status=0
CACHE_MODE=unavailable "$wrapper" --show-stats > "$fixture/stdout" 2> "$fixture/stderr" || status=$?
[[ "$status" -eq 2 && ! -s "$COMPILER_RECORD" ]]
printf '%s\0' --show-stats > "$fixture/expected-cache"
cmp "$fixture/expected-cache" "$CACHE_RECORD"

[[ -z "$(find "$fixture/.tmp/sccache-runtime/tmp" -name 'client-error.*' -print -quit)" ]]
echo 'sccache wrapper tests passed'
