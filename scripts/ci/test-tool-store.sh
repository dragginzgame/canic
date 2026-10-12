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
[[ -d "$first_slot/install.lock" && ! -e "$consumer/.tools/$store_test_kind" &&
   ! -e "$consumer/.tools/.$store_test_kind-tools.lock" &&
   "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
rmdir "$first_slot/install.lock"

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
consumer="$first_consumer"
install --check
printf 'Shared %s storage reuse, isolated selections, corruption, locks and offline refusal passed\n' "$store_test_kind"
