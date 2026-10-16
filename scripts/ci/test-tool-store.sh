#!/usr/bin/env bash
# Shared companions: scripts/dev/tool-store.sh
# Sourced by the host/IC fixtures with their authenticated fake payloads and
# install callback. This exercises real store coordination without networking.
# shellcheck disable=SC2154 # Parent fixture owns fixture and store_test_kind.
store_refuse() {
    if "$@" > "$fixture/store-refusal.log" 2>&1; then
        echo 'shared tool store accepted an invalid operation' >&2; exit 1
    fi
}
export HOST_SHARED_STORE="$fixture/shared store" IC_SHARED_STORE="$fixture/shared store"
consumer="$fixture/sharing-a"; mkdir "$consumer"
install
first_consumer="$consumer"
first_bundle="$(cd "$consumer/.tools/$store_test_kind" && pwd -P)"
first_slot="${first_bundle%/*}"
before="$(wc -l < "$fixture/downloads")"
consumer="$fixture/sharing-b"; mkdir "$consumer"
store_refuse install --check
[[ ! -e "$consumer/.tools" ]] || exit 1
install
[[ "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" &&
   "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
# No physical executable payload lives under either checkout.
[[ -z "$(find "$consumer/.tools" "$first_consumer/.tools" -type f)" ]] || exit 1
second_consumer="$consumer"

# Neither checks nor explicit setup may replace an unmanaged checkout path.
for shape in directory symlink; do
    consumer="$fixture/unmanaged-$shape"; mkdir -p "$consumer/.tools"
    if [[ "$shape" == directory ]]; then
        mkdir "$consumer/.tools/$store_test_kind"
        printf 'user-owned bytes\n' > "$consumer/.tools/$store_test_kind/payload"
    else
        ln -s "$first_bundle" "$consumer/.tools/$store_test_kind"
    fi
    for mode in check install; do
        if [[ "$mode" == check ]]; then store_refuse install --check;
        else store_refuse install; fi
        [[ "$(wc -l < "$fixture/downloads")" == "$before" &&
           ! -e "$consumer/.tools/.$store_test_kind-tools.lock" ]] || exit 1
        if [[ "$shape" == directory ]]; then
            [[ "$(cat "$consumer/.tools/$store_test_kind/payload")" == 'user-owned bytes' ]] || exit 1
        else
            [[ "$(readlink "$consumer/.tools/$store_test_kind")" == "$first_bundle" ]] || exit 1
        fi
    done
done
consumer="$second_consumer"

# Explicit adoption replaces only the old active reference; retained physical
# bundles remain byte-for-byte available and are never a check-time fallback.
consumer="$fixture/local-adoption"; mkdir -p "$consumer/.tools"
cp -R "$first_bundle" "$consumer/.tools/$store_test_kind-set.previous"
ln -s "$store_test_kind-set.previous" "$consumer/.tools/$store_test_kind"
store_refuse install --check
install
[[ "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" &&
   -d "$consumer/.tools/$store_test_kind-set.previous" &&
   ! -L "$consumer/.tools/$store_test_kind-set.previous" &&
   "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
diff -r "$first_bundle" "$consumer/.tools/$store_test_kind-set.previous"
consumer="$second_consumer"

# Other platforms and changed versions never select the existing payload.
TEST_OS=Darwin TEST_ARCH=arm64 IC_TOOLS_TEST_OS=Darwin IC_TOOLS_TEST_ARCH=arm64 install
[[ "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" != "$first_bundle" &&
   "$(cd "$first_consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
install
if [[ "$store_test_kind" == host ]]; then
    previous_pins="$pins"
    cp "$pins" "$fixture/store-upgrade.env"
    cp "$fixture/assets/jq-linux-amd64" "$fixture/store-original-asset"
    sed 's/jq-1.8.2/jq-1.8.3/' "$fixture/jq" > "$fixture/assets/jq-linux-amd64"
    digest="$(shasum -a 256 "$fixture/assets/jq-linux-amd64")"
    printf 'export SHARED_TOOLING_JQ_VERSION=1.8.3\nexport SHARED_TOOLING_JQ_SHA256_LINUX_AMD64=%s\n' "${digest%% *}" >> "$fixture/store-upgrade.env"
    pins="$fixture/store-upgrade.env"
else
    previous_pins="$selected_pins"
    cp "$fixture/assets/quill-linux-x86_64" "$fixture/store-original-asset"
    sed 's/quill 0.5.4/quill 0.5.5/' "$fixture/payload/quill" > "$fixture/assets/quill-linux-x86_64"
    digest="$(shasum -a 256 "$fixture/assets/quill-linux-x86_64")"
    awk -F '\t' -v hash="${digest%% *}" 'BEGIN { OFS="\t" } $1=="quill" { $2="0.5.5"; if ($3=="linux-x86_64") $4=hash } { print }' "$pins" > "$fixture/store-upgrade.tsv"
    selected_pins="$fixture/store-upgrade.tsv"
fi
install
[[ "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" != "$first_bundle" &&
   "$(cd "$first_consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
if [[ "$store_test_kind" == host ]]; then
    pins="$previous_pins"; cp "$fixture/store-original-asset" "$fixture/assets/jq-linux-amd64"
else
    selected_pins="$previous_pins"; cp "$fixture/store-original-asset" "$fixture/assets/quill-linux-x86_64"
fi
store_refuse install --check
install
[[ "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
before="$(wc -l < "$fixture/downloads")"

# A third checkout cannot race an owner of the same installation selection.
consumer="$fixture/sharing-locked"; mkdir "$consumer"
mkdir "$first_slot/install.lock"
consumer="$second_consumer"
store_refuse install --check
diagnostic="$fixture/store-refusal.log"
[[ "$store_test_kind" != host ]] || diagnostic="$fixture/install.log"
grep -F "tool store installation locked: $first_slot/install.lock" "$diagnostic"
consumer="$fixture/sharing-locked"
store_refuse install
grep -F "tool installation lock path already exists: $first_slot/install.lock" "$diagnostic" >/dev/null
[[ -d "$first_slot/install.lock" && ! -e "$consumer/.tools/$store_test_kind" &&
   ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
   "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
rmdir "$first_slot/install.lock"

# Existing non-directory lock paths are occupied too, never an acquired lock.
for obstruction in file dangling-link; do
    if [[ "$obstruction" == file ]]; then
        printf 'foreign lock evidence\n' > "$first_slot/install.lock"
    else
        ln -s "$fixture/absent-lock-target" "$first_slot/install.lock"
    fi
    store_refuse install
    grep -F "tool installation lock path already exists: $first_slot/install.lock" "$diagnostic" >/dev/null
    [[ ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
       ! -e "$consumer/.tools/$store_test_kind" &&
       ! -e "$fixture/absent-lock-target" &&
       "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
    if [[ "$obstruction" == file ]]; then
        [[ "$(cat "$first_slot/install.lock")" == 'foreign lock evidence' ]] || exit 1
    else
        [[ -L "$first_slot/install.lock" && "$(readlink "$first_slot/install.lock")" == "$fixture/absent-lock-target" ]] || exit 1
    fi
    rm "$first_slot/install.lock"
done

# Inject mkdir failures independently of UID: chmod-only denial can succeed when
# the fixture runs privileged. The real installers/cleanup still own every effect.
mkdir "$fixture/lock-errors-bin"
export STORE_TEST_REAL_MKDIR STORE_TEST_MKDIR_PATH STORE_TEST_MKDIR_MESSAGE STORE_TEST_MKDIR_STATUS STORE_TEST_MKDIR_CALLS
STORE_TEST_REAL_MKDIR="$(command -v mkdir)"
STORE_TEST_MKDIR_CALLS="$fixture/lock-mkdir-calls"
cat > "$fixture/lock-errors-bin/mkdir" <<'SCRIPT'
#!/usr/bin/env bash
if [[ "${!#}" == "$STORE_TEST_MKDIR_PATH" ]]; then
    printf 'attempt\n' >> "$STORE_TEST_MKDIR_CALLS"
    printf '%s\n' "$STORE_TEST_MKDIR_MESSAGE" >&2
    exit "$STORE_TEST_MKDIR_STATUS"
fi
exec "$STORE_TEST_REAL_MKDIR" "$@"
SCRIPT
chmod +x "$fixture/lock-errors-bin/mkdir"
find "$first_slot" -print | sort > "$fixture/lock-store-before"
for boundary in checkout store slot; do
    for reason in permission read-only; do
        consumer="$fixture/lock-$boundary-$reason"
        mkdir -p "$consumer/.tools"
        printf 'retained consumer evidence\n' > "$consumer/.tools/evidence"
        case "$boundary" in
            checkout) STORE_TEST_MKDIR_PATH="$consumer/.tools/.$store_test_kind-tools.lock" ;;
            store) STORE_TEST_MKDIR_PATH="$first_slot/install.lock" ;;
            slot) STORE_TEST_MKDIR_PATH="$first_slot" ;;
        esac
        STORE_TEST_MKDIR_STATUS=13
        [[ "$reason" != read-only ]] || STORE_TEST_MKDIR_STATUS=30
        STORE_TEST_MKDIR_MESSAGE="mkdir: $reason filesystem refusal at $STORE_TEST_MKDIR_PATH"
        : > "$STORE_TEST_MKDIR_CALLS"
        log_start=0
        [[ ! -f "$fixture/install.log" ]] || log_start="$(wc -l < "$fixture/install.log")"
        diagnostic="$fixture/lock-$boundary-$reason.log"
        status=0
        PATH="$fixture/lock-errors-bin:$PATH" install > "$diagnostic" 2>&1 || status=$?
        if [[ "$store_test_kind" == host ]]; then
            tail -n "+$((log_start + 1))" "$fixture/install.log" >> "$diagnostic"
        fi
        [[ "$status" == "$STORE_TEST_MKDIR_STATUS" && "$(wc -l < "$STORE_TEST_MKDIR_CALLS")" -eq 1 ]] || exit 1
        grep -Fx "$STORE_TEST_MKDIR_MESSAGE" "$diagnostic" >/dev/null
        if grep -E 'already exists|already locked|installation locked' "$diagnostic"; then exit 1; fi
        if [[ "$boundary" != slot ]]; then
            grep -F "unable to acquire tool installation lock: $STORE_TEST_MKDIR_PATH" "$diagnostic" >/dev/null
        fi
        [[ ! -e "$first_slot/install.lock" && ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
           ! -e "$consumer/.tools/$store_test_kind" &&
           "$(cat "$consumer/.tools/evidence")" == 'retained consumer evidence' &&
           "$(cd "$first_consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" &&
           "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
        find "$first_slot" -print | sort > "$fixture/lock-store-after"
        cmp "$fixture/lock-store-before" "$fixture/lock-store-after"
        diff -r "$first_bundle" "$fixture/local-adoption/.tools/$store_test_kind-set.previous"
        # A later explicit attempt succeeds; the failed command never retries.
        install
        [[ ! -e "$first_slot/install.lock" && ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
           "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" &&
           "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
    done
done

# Corruption is rejected before probes. Explicit setup creates a fresh retained
# generation; it never overwrites the one still selected by another checkout.
payload=jq; [[ "$store_test_kind" != ic ]] || payload=quill
cp "$first_bundle/bin/$payload" "$fixture/store-original"
printf '\ncorrupt\n' >> "$first_bundle/bin/$payload"
: > "$fixture/executions"
consumer="$second_consumer"
store_refuse install --check
[[ ! -s "$fixture/executions" ]] || exit 1
consumer="$fixture/sharing-repair"; mkdir "$consumer"
install
replacement="$(cd "$consumer/.tools/$store_test_kind" && pwd -P)"
[[ "$replacement" != "$first_bundle" && -d "$first_bundle" &&
   "$(cd "$first_consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
consumer="$first_consumer"
store_refuse install --check
cp "$fixture/store-original" "$first_bundle/bin/$payload"
install --check

# Missing shared state never triggers a download or repair during an offline check.
mv "$first_bundle" "$first_slot/retained-missing"
before="$(wc -l < "$fixture/downloads")"
store_refuse install --check
[[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
mv "$first_slot/retained-missing" "$first_bundle"

# Managed-root redirects and newline-bearing locations are refused without writes.
ln -s "$HOST_SHARED_STORE" "$fixture/redirected-store"
for bad_store in "$fixture/redirected-store" "$fixture/redirected-store/" "$fixture/store"$'\n' "$fixture/new/./store"; do
    HOST_SHARED_STORE="$bad_store" IC_SHARED_STORE="$bad_store" store_refuse install
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
done
# Refuse a writable-by-others store before creating a selection beneath it.
mkdir "$fixture/unsafe-store"
chmod 0777 "$fixture/unsafe-store"
HOST_SHARED_STORE="$fixture/unsafe-store" IC_SHARED_STORE="$fixture/unsafe-store" store_refuse install
[[ -z "$(ls -A "$fixture/unsafe-store")" ]] || exit 1
install --check

# Checkout cleanup failure must not strand the shared lock or hide an earlier
# download failure. Exercise the real entrypoints, including cached activation.
mkdir "$fixture/lifecycle-bin"
export STORE_TEST_REAL_RMDIR
STORE_TEST_REAL_RMDIR="$(command -v rmdir)"
cat > "$fixture/lifecycle-bin/rmdir" <<'SCRIPT'
#!/usr/bin/env bash
if [[ "$1" == "$STORE_TEST_CHECKOUT_LOCK" ]]; then exit 31; fi
exec "$STORE_TEST_REAL_RMDIR" "$@"
SCRIPT
chmod +x "$fixture/lifecycle-bin/rmdir"
consumer="$fixture/cleanup-failure"; mkdir "$consumer"
export STORE_TEST_CHECKOUT_LOCK="$consumer/.tools/.$store_test_kind-tools.lock"
status=0
PATH="$fixture/lifecycle-bin:$PATH" install || status=$?
[[ "$status" == 31 && ! -e "$first_slot/install.lock" &&
   "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$replacement" &&
   "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
"$STORE_TEST_REAL_RMDIR" "$STORE_TEST_CHECKOUT_LOCK"

cat > "$fixture/lifecycle-bin/curl" <<'SCRIPT'
#!/usr/bin/env bash
exit 23
SCRIPT
chmod +x "$fixture/lifecycle-bin/curl"
consumer="$fixture/download-failure"; mkdir "$consumer"
STORE_TEST_CHECKOUT_LOCK="$consumer/.tools/.$store_test_kind-tools.lock"
status=0
HOST_SHARED_STORE="$fixture/failed-store" IC_SHARED_STORE="$fixture/failed-store" \
    PATH="$fixture/lifecycle-bin:$PATH" install || status=$?
[[ "$status" == 23 && ! -e "$consumer/.tools/$store_test_kind" ]] || exit 1
failed_bundle="$(cd "$consumer/.tools/$store_test_kind-set."* && pwd -P)"
[[ -d "$failed_bundle" && ! -e "${failed_bundle%/*}/install.lock" &&
   "$(cd "$first_consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
"$STORE_TEST_REAL_RMDIR" "$STORE_TEST_CHECKOUT_LOCK"
HOST_SHARED_STORE="$fixture/failed-store" IC_SHARED_STORE="$fixture/failed-store" install
[[ -d "$failed_bundle" && ! -e "${failed_bundle%/*}/install.lock" &&
   ! -e "$STORE_TEST_CHECKOUT_LOCK" &&
   "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" != "$failed_bundle" ]] || exit 1

# Curl owns retries; a final HTTP error must not restart setup, alter the old
# selection or remove its failed candidate. Exercise both real installer callers.
cat > "$fixture/lifecycle-bin/curl" <<'SCRIPT'
#!/usr/bin/env bash
echo called >> "$STORE_TEST_DOWNLOAD_CALLS"
exit 22
SCRIPT
export STORE_TEST_DOWNLOAD_CALLS="$fixture/http-download-calls"
consumer="$fixture/http-failure"; mkdir -p "$consumer/.tools"
ln -s "$first_bundle" "$consumer/.tools/$store_test_kind-set.previous"
ln -s "$store_test_kind-set.previous" "$consumer/.tools/$store_test_kind"
status=0
HOST_SHARED_STORE="$fixture/http-failure-store" IC_SHARED_STORE="$fixture/http-failure-store" \
    PATH="$fixture/lifecycle-bin:$PATH" install || status=$?
[[ "$status" == 22 && "$(wc -l < "$STORE_TEST_DOWNLOAD_CALLS")" -eq 1 &&
   ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
   "$(cd "$consumer/.tools/$store_test_kind" && pwd -P)" == "$first_bundle" ]] || exit 1
retained_candidates=0
for reference in "$consumer/.tools/$store_test_kind-set."*; do
    [[ "$reference" != *-set.previous ]] || continue
    failed_bundle="$(cd "$reference" && pwd -P)"
    [[ -d "$failed_bundle" && ! -e "${failed_bundle%/*}/install.lock" ]] || exit 1
    retained_candidates=$((retained_candidates + 1))
done
[[ "$retained_candidates" == 1 ]] || exit 1
consumer="$first_consumer"
install --check
printf 'Shared %s storage reuse, isolated selections, corruption, locks and offline refusal passed\n' "$store_test_kind"
