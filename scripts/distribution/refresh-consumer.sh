#!/usr/bin/env bash
set -euo pipefail

SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SOURCE_ROOT="$SCRIPT_ROOT"
CONSUMER_ROOT=""
MANIFEST_PATH=".shared-tooling.snapshot"
STAGING_DIR=""
requested_files=()

usage() {
    cat >&2 <<'USAGE'
usage: refresh-consumer.sh [--source <shared-tooling-checkout>] --consumer <repository> [--manifest <relative-path>] [--file <relative-path>]...

Use --file only when creating the initial snapshot. Later refreshes read the
declared file set from the existing consumer manifest.
USAGE
}

fail() {
    echo "shared-tooling refresh failed: $1" >&2
    exit 1
}

cleanup() {
    if [[ -n "$STAGING_DIR" && -d "$STAGING_DIR" ]]; then
        rm -rf "$STAGING_DIR"
    fi
}

trap cleanup EXIT

validate_relative_path() {
    local path="$1"

    [[ -n "$path" && "$path" != /* ]] || fail "path must be relative: $path"
    case "/$path/" in
    *'/../'* | *'/./'* | *'//'*) fail "path is not canonical: $path" ;;
    esac
    case "$path" in
    *$'\n'* | *$'\t'*) fail "path contains a forbidden control character" ;;
    esac
}

check_consumer_parent() {
    local relative_path="$1"
    local parent="${relative_path%/*}"
    local cursor="$CONSUMER_ROOT"
    local component

    [[ "$parent" != "$relative_path" ]] || return 0
    while [[ -n "$parent" ]]; do
        component="${parent%%/*}"
        if [[ "$parent" == */* ]]; then
            parent="${parent#*/}"
        else
            parent=""
        fi
        cursor="$cursor/$component"
        [[ ! -L "$cursor" ]] || fail "consumer path traverses a symlink: $relative_path"
    done
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --source)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        SOURCE_ROOT="$2"
        shift 2
        ;;
    --consumer)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        CONSUMER_ROOT="$2"
        shift 2
        ;;
    --manifest)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        MANIFEST_PATH="$2"
        shift 2
        ;;
    --file)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        requested_files[${#requested_files[@]}]="$2"
        shift 2
        ;;
    -h | --help)
        usage
        exit 0
        ;;
    *)
        usage
        exit 2
        ;;
    esac
done

[[ -n "$CONSUMER_ROOT" ]] || { usage; exit 2; }
validate_relative_path "$MANIFEST_PATH"

SOURCE_ROOT="$(cd "$SOURCE_ROOT" 2>/dev/null && pwd -P)" ||
    fail "source checkout does not exist: $SOURCE_ROOT"
CONSUMER_ROOT="$(cd "$CONSUMER_ROOT" 2>/dev/null && pwd -P)" ||
    fail "consumer repository does not exist: $CONSUMER_ROOT"
[[ "$CONSUMER_ROOT" != "/" ]] || fail "consumer repository may not be the filesystem root"
[[ "$SOURCE_ROOT" != "$CONSUMER_ROOT" ]] || fail "source and consumer repositories must differ"

source_top="$(git -C "$SOURCE_ROOT" rev-parse --show-toplevel 2>/dev/null)" ||
    fail "source is not a Git checkout"
source_top="$(cd "$source_top" && pwd -P)"
[[ "$source_top" == "$SOURCE_ROOT" ]] || fail "--source must name the checkout root"

consumer_top="$(git -C "$CONSUMER_ROOT" rev-parse --show-toplevel 2>/dev/null)" ||
    fail "consumer is not a Git checkout"
consumer_top="$(cd "$consumer_top" && pwd -P)"
[[ "$consumer_top" == "$CONSUMER_ROOT" ]] || fail "--consumer must name the checkout root"

source_status="$(git -C "$SOURCE_ROOT" status --porcelain --untracked-files=all)"
[[ -z "$source_status" ]] || fail "source checkout must be clean so its revision identifies every copied byte"

source_revision="$(git -C "$SOURCE_ROOT" rev-parse HEAD)"
[[ "$source_revision" =~ ^[0-9a-f]{40,64}$ ]] || fail "source revision is malformed"
source_remote="$(git -C "$SOURCE_ROOT" remote get-url origin)" ||
    fail "source checkout has no origin remote"
[[ -n "$source_remote" ]] || fail "source origin remote is empty"
case "$source_remote" in
*$'\n'* | *$'\t'*) fail "source remote contains a forbidden control character" ;;
esac

manifest="$CONSUMER_ROOT/$MANIFEST_PATH"
check_consumer_parent "$MANIFEST_PATH"
[[ ! -L "$manifest" ]] || fail "consumer manifest may not be a symlink"
files=()

if [[ -e "$manifest" ]]; then
    [[ -f "$manifest" && ! -L "$manifest" ]] || fail "consumer manifest must be a regular file"
    [[ ${#requested_files[@]} -eq 0 ]] ||
        fail "--file is only allowed when creating the initial snapshot"

    manifest_format_count=0
    manifest_source_count=0
    manifest_revision_count=0
    manifest_source=""
    while IFS=$'\t' read -r record first second third extra || [[ -n "$record" ]]; do
        case "$record" in
        '' | \#*) continue ;;
        format)
            [[ "$first" == "1" && -z "$second" && -z "$third" && -z "$extra" ]] ||
                fail "existing manifest has an unsupported format"
            manifest_format_count=$((manifest_format_count + 1))
            ;;
        source)
            [[ -n "$first" && -z "$second" && -z "$third" && -z "$extra" ]] ||
                fail "existing manifest has a malformed source record"
            manifest_source="$first"
            manifest_source_count=$((manifest_source_count + 1))
            ;;
        revision)
            [[ "$first" =~ ^[0-9a-f]{40,64}$ && -z "$second" && -z "$third" && -z "$extra" ]] ||
                fail "existing manifest has a malformed revision record"
            manifest_revision_count=$((manifest_revision_count + 1))
            ;;
        file)
            [[ "$first" =~ ^[0-9a-f]{64}$ && "$second" =~ ^(-|x)$ && -n "$third" && -z "$extra" ]] ||
                fail "existing manifest has a malformed file record"
            files[${#files[@]}]="$third"
            ;;
        *) fail "existing manifest contains an unknown record: $record" ;;
        esac
    done <"$manifest"
    [[ "$manifest_format_count" -eq 1 ]] || fail "existing manifest must contain one format record"
    [[ "$manifest_source_count" -eq 1 ]] || fail "existing manifest must contain one source record"
    [[ "$manifest_revision_count" -eq 1 ]] || fail "existing manifest must contain one revision record"
    [[ "$manifest_source" == "$source_remote" ]] ||
        fail "source remote differs from the existing manifest"
else
    [[ ${#requested_files[@]} -gt 0 ]] || fail "the initial snapshot requires at least one --file"
    files=("${requested_files[@]}")
fi

[[ ${#files[@]} -gt 0 ]] || fail "snapshot file set is empty"

checksum_tool_declared=false
snapshot_verifier_declared=false
source_modes=()
for index in "${!files[@]}"; do
    path="${files[$index]}"
    validate_relative_path "$path"
    [[ "$path" != "$MANIFEST_PATH" ]] || fail "manifest may not include itself"
    source_file="$SOURCE_ROOT/$path"
    [[ -f "$source_file" && ! -L "$source_file" ]] ||
        fail "source file is missing or symlinked: $path"
    source_parent="$(cd "$(dirname "$source_file")" && pwd -P)"
    case "$source_parent/" in
    "$SOURCE_ROOT/"*) ;;
    *) fail "source file escapes the source checkout: $path" ;;
    esac
    source_mode=""
    while IFS= read -r -d '' tree_entry; do
        # NUL records preserve literal paths, including spaces and Git quoting.
        [[ "${tree_entry#*$'\t'}" == "$path" ]] || continue
        tree_metadata="${tree_entry%%$'\t'*}"
        read -r entry_mode entry_type _ <<<"$tree_metadata"
        [[ "$entry_type" == "blob" && "$entry_mode" =~ ^100(644|755)$ ]] ||
            fail "source revision does not contain a regular file: $path"
        source_mode="$entry_mode"
    done < <(git -C "$SOURCE_ROOT" ls-tree -z "$source_revision" -- "$path")
    [[ -n "$source_mode" ]] || fail "source file is not recorded in revision $source_revision: $path"
    source_modes[index]="$source_mode"
    check_consumer_parent "$path"
    destination="$CONSUMER_ROOT/$path"
    [[ ! -L "$destination" ]] || fail "consumer destination is a symlink: $path"
    [[ "$path" != "scripts/ci/verify-file-checksum.sh" ]] || checksum_tool_declared=true
    [[ "$path" != "scripts/ci/verify-shared-tooling-snapshot.sh" ]] || snapshot_verifier_declared=true

    for prior_index in "${!files[@]}"; do
        [[ "$prior_index" -lt "$index" ]] || break
        [[ "${files[$prior_index]}" != "$path" ]] || fail "duplicate snapshot path: $path"
    done
done

[[ "$checksum_tool_declared" == "true" ]] || fail "snapshot must include the checksum verifier"
[[ "$snapshot_verifier_declared" == "true" ]] || fail "snapshot must include the snapshot verifier"

STAGING_DIR="$(mktemp -d "$CONSUMER_ROOT/.shared-tooling-refresh.XXXXXX")"
staged_files="$STAGING_DIR/files"
mkdir -p "$staged_files"

for index in "${!files[@]}"; do
    path="${files[$index]}"
    mkdir -p "$staged_files/$(dirname "$path")"
    git -C "$SOURCE_ROOT" cat-file blob "$source_revision:$path" >"$staged_files/$path" ||
        fail "cannot export committed source file: $path"
    chmod "${source_modes[$index]#100}" "$staged_files/$path"
done

staged_manifest="$STAGING_DIR/manifest"
{
    echo "# Shared Tooling snapshot v1"
    printf 'format\t1\n'
    printf 'source\t%s\n' "$source_remote"
    printf 'revision\t%s\n' "$source_revision"
    for path in "${files[@]}"; do
        digest="$(bash "$SCRIPT_ROOT/scripts/ci/verify-file-checksum.sh" --print sha256 "$staged_files/$path")"
        if [[ -x "$staged_files/$path" ]]; then
            executable="x"
        else
            executable="-"
        fi
        printf 'file\t%s\t%s\t%s\n' "$digest" "$executable" "$path"
    done
} >"$staged_manifest"

# Check every destination before replacing any file. A retry may already have
# installed these exact bytes; accepting them preserves content and index state.
for path in "${files[@]}"; do
    destination="$CONSUMER_ROOT/$path"
    [[ ! -e "$destination" || -f "$destination" ]] || fail "consumer destination is not a regular file: $path"
    if [[ -f "$destination" ]] && cmp -s "$staged_files/$path" "$destination" && \
        { { [[ -x "$destination" ]] && [[ -x "$staged_files/$path" ]]; } || \
          { [[ ! -x "$destination" ]] && [[ ! -x "$staged_files/$path" ]]; }; }; then
        continue
    fi
    destination_status="$(git -C "$CONSUMER_ROOT" --literal-pathspecs status \
        --porcelain --untracked-files=all --ignored -- "$path")" ||
        fail "cannot inspect consumer changes: $path"
    [[ -z "$destination_status" ]] || fail "consumer destination has local changes; preserve or reconcile them before refreshing: $path"
done

# Prepare a custom manifest's parent before replacing any consumer files.
mkdir -p "$(dirname "$manifest")" || fail "cannot create consumer manifest directory"

for path in "${files[@]}"; do
    mkdir -p "$CONSUMER_ROOT/$(dirname "$path")"
    cp -p "$staged_files/$path" "$CONSUMER_ROOT/$path"
done
mv "$staged_manifest" "$manifest"

bash "$staged_files/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$CONSUMER_ROOT" \
    --manifest "$MANIFEST_PATH"

echo "shared-tooling snapshot refreshed: ${#files[@]} file(s) from $source_revision"
