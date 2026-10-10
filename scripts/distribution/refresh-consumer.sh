#!/usr/bin/env bash
set -euo pipefail

SCRIPT_ROOT="${BASH_SOURCE[0]}"
[[ "$SCRIPT_ROOT" == /* ]] || SCRIPT_ROOT="$PWD/$SCRIPT_ROOT"
SCRIPT_ROOT="$(cd -P "${SCRIPT_ROOT%/*}/../.." && printf '%s/.' "$PWD")"
SCRIPT_ROOT="${SCRIPT_ROOT%/.}"
SOURCE_ROOT="$SCRIPT_ROOT"
CONSUMER_ROOT=""
MANIFEST_PATH=".shared-tooling.snapshot"
STAGING_DIR=""
requested_files=()
added_files=()

usage() {
    cat >&2 <<'USAGE'
usage: refresh-consumer.sh [--source <shared-tooling-checkout>] --consumer <repository> [--manifest <relative-path>] [--file <relative-path>]... [--add-file <relative-path>]...

Use --file only when creating the initial snapshot. Later refreshes read the
declared file set from the existing consumer manifest.
Use --add-file to explicitly extend an existing selection; repeated additions
are idempotent. Required companions are checked, never added implicitly.
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

# Check before command substitution can trim a pathname, and again after
# resolving aliases. Absolute cd inputs avoid CDPATH lookup and output.
resolve_directory() {
    local path="$1" resolved
    [[ "$path" == /* ]] || path="$PWD/$path"
    [[ "$path" != *$'\n'* && "$path" != *$'\r'* ]] ||
        fail "directory paths must not contain LF or CR characters"
    resolved="$(cd -P "$path" 2>/dev/null && printf '%s/.' "$PWD")" ||
        fail "directory does not exist: $path"
    resolved="${resolved%/.}"
    [[ "$resolved" != *$'\n'* && "$resolved" != *$'\r'* ]] ||
        fail "resolved directory paths must not contain LF or CR characters"
    printf '%s\n' "$resolved"
}

validate_relative_path() {
    local path="$1"

    [[ -n "$path" && "$path" != /* ]] || fail "path must be relative: $path"
    case "/$path/" in
    *'/../'* | *'/./'* | *'//'*) fail "path is not canonical: $path" ;;
    esac
    case "$path" in
    *$'\n'* | *$'\r'* | *$'\t'*) fail "path contains a forbidden control character" ;;
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

# Capture bytes, executable state and path identity independently of Git status.
# A later identical-content replacement is still a different consumer input.
consumer_state() {
    local path="$1" target="$CONSUMER_ROOT/$1"
    check_consumer_parent "$path"
    [[ ! -L "$target" && ( ! -e "$target" || -f "$target" ) ]] ||
        fail "consumer destination is not a regular file: $path"
    if [[ ! -e "$target" ]]; then printf 'absent\n'; return; fi
    perl -e 'my @s = lstat($ARGV[0]); @s or die "cannot stat input: $!\n"; print "$s[0]:$s[1]:$s[2]\n"' "$target" ||
        fail "cannot identify consumer input: $path"
    bash "$SCRIPT_ROOT/scripts/ci/verify-file-checksum.sh" --print sha256 "$target" ||
        fail "cannot hash consumer input: $path"
}

committed_mode() {
    local revision="$1" path="$2" tree_entry tree_metadata entry_mode entry_type mode=""
    while IFS= read -r -d '' tree_entry; do
        [[ "${tree_entry#*$'\t'}" == "$path" ]] || continue
        tree_metadata="${tree_entry%%$'\t'*}"
        read -r entry_mode entry_type _ <<<"$tree_metadata"
        [[ "$entry_type" == blob && "$entry_mode" =~ ^100(644|755)$ ]] ||
            fail "source revision does not contain a regular file: $path"
        mode="$entry_mode"
    done < <(git -C "$SOURCE_ROOT" ls-tree -z "$revision" -- "$path")
    [[ -n "$mode" ]] || fail "source file is not recorded in revision $revision: $path"
    printf '%s\n' "$mode"
}

check_consumer_unchanged() {
    local path="$1" expected="$2" current
    current="$(consumer_state "$path")" || fail "cannot recheck consumer input: $path"
    [[ "$current" == "$expected" ]] || fail "consumer input changed during refresh: $path"
}

check_index_unchanged() {
    local index="$1" path="$2"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$STAGING_DIR/current-index" ||
        fail "cannot inspect consumer index: $path"
    cmp -s "$STAGING_DIR/index-$index" "$STAGING_DIR/current-index" ||
        fail "consumer index changed during refresh: $path"
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
    --add-file)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        added_files[${#added_files[@]}]="$2"
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

SCRIPT_ROOT="$(resolve_directory "$SCRIPT_ROOT")"
SOURCE_ROOT="$(resolve_directory "$SOURCE_ROOT")"
CONSUMER_ROOT="$(resolve_directory "$CONSUMER_ROOT")"
[[ "$CONSUMER_ROOT" != "/" ]] || fail "consumer repository may not be the filesystem root"
[[ "$SOURCE_ROOT" != "$CONSUMER_ROOT" ]] || fail "source and consumer repositories must differ"

source_top="$(git -C "$SOURCE_ROOT" rev-parse --show-toplevel 2>/dev/null && printf '.')" ||
    fail "source is not a Git checkout"
source_top="${source_top%.}"
source_top="$(resolve_directory "${source_top%$'\n'}")"
[[ "$source_top" == "$SOURCE_ROOT" ]] || fail "--source must name the checkout root"

consumer_top="$(git -C "$CONSUMER_ROOT" rev-parse --show-toplevel 2>/dev/null && printf '.')" ||
    fail "consumer is not a Git checkout"
consumer_top="${consumer_top%.}"
consumer_top="$(resolve_directory "${consumer_top%$'\n'}")"
[[ "$consumer_top" == "$CONSUMER_ROOT" ]] || fail "--consumer must name the checkout root"

source_status="$(git -C "$SOURCE_ROOT" status --porcelain --untracked-files=all)"
[[ -z "$source_status" ]] || fail "source checkout must be clean so its revision identifies every copied byte"

source_revision="$(git -C "$SOURCE_ROOT" rev-parse HEAD)"
[[ "$source_revision" =~ ^[0-9a-f]{40,64}$ ]] || fail "source revision is malformed"
# Read the display version from the same immutable source as the payload, never
# the working tree or the consumer's own VERSION. Preserve invalid extra lines.
committed_mode "$source_revision" VERSION >/dev/null || fail "source VERSION must be a committed regular file"
source_version="$(git -C "$SOURCE_ROOT" cat-file blob "$source_revision:VERSION" && printf '.')" ||
    fail "cannot read committed source VERSION"
source_version="${source_version%.}"
source_version="${source_version%$'\n'}"
[[ "$source_version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] ||
    fail "source VERSION must contain a canonical stable version"
source_remote="$(git -C "$SOURCE_ROOT" remote get-url origin)" ||
    fail "source checkout has no origin remote"
[[ -n "$source_remote" ]] || fail "source origin remote is empty"
case "$source_remote" in
*$'\n'* | *$'\t'*) fail "source remote contains a forbidden control character" ;;
esac

manifest="$CONSUMER_ROOT/$MANIFEST_PATH"
check_consumer_parent "$MANIFEST_PATH"
[[ ! -L "$manifest" ]] || fail "consumer manifest may not be a symlink"
STAGING_DIR="$(mktemp -d "$CONSUMER_ROOT/.shared-tooling-refresh.XXXXXX")"
manifest_state="$(consumer_state "$MANIFEST_PATH")" || fail "cannot inspect consumer manifest"
git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$MANIFEST_PATH" > "$STAGING_DIR/index-manifest" ||
    fail "cannot inspect consumer manifest index"
files=()
previous_digests=()
previous_modes=()

if [[ -e "$manifest" ]]; then
    [[ -f "$manifest" && ! -L "$manifest" ]] || fail "consumer manifest must be a regular file"
    [[ ${#requested_files[@]} -eq 0 ]] ||
        fail "--file is only allowed when creating the initial snapshot"
    cp -p "$manifest" "$STAGING_DIR/previous-manifest"
    check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"

    manifest_format_count=0
    manifest_source_count=0
    manifest_revision_count=0
    manifest_version_count=0
    manifest_source=""
    while IFS=$'\t' read -r record first second third extra || [[ -n "$record" ]]; do
        case "$record" in
        '# version')
            [[ "$first" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && -z "$second" && -z "$third" && -z "$extra" ]] ||
                fail "existing manifest has a malformed version annotation"
            manifest_version_count=$((manifest_version_count + 1))
            ;;
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
            manifest_revision="$first"
            ;;
        file)
            [[ "$first" =~ ^[0-9a-f]{64}$ && "$second" =~ ^(-|x)$ && -n "$third" && -z "$extra" ]] ||
                fail "existing manifest has a malformed file record"
            previous_digests[${#files[@]}]="$first"
            previous_modes[${#files[@]}]="$second"
            files[${#files[@]}]="$third"
            ;;
        *) fail "existing manifest contains an unknown record: $record" ;;
        esac
    done <"$STAGING_DIR/previous-manifest"
    [[ "$manifest_format_count" -eq 1 ]] || fail "existing manifest must contain one format record"
    [[ "$manifest_source_count" -eq 1 ]] || fail "existing manifest must contain one source record"
    [[ "$manifest_revision_count" -eq 1 ]] || fail "existing manifest must contain one revision record"
    [[ "$manifest_version_count" -le 1 ]] || fail "existing manifest has duplicate version annotations"
    [[ "$manifest_source" == "$source_remote" ]] ||
        fail "source remote differs from the existing manifest"
else
    [[ ${#added_files[@]} -eq 0 ]] || fail "--add-file requires an existing snapshot"
    [[ ${#requested_files[@]} -gt 0 ]] || fail "the initial snapshot requires at least one --file"
    files=("${requested_files[@]}")
fi

for path in ${added_files[@]+"${added_files[@]}"}; do
    validate_relative_path "$path"
    selected=false
    for existing in "${files[@]}"; do
        [[ "$existing" != "$path" ]] || selected=true
    done
    if [[ "$selected" == false ]]; then files[${#files[@]}]="$path"; fi
done

[[ ${#files[@]} -gt 0 ]] || fail "snapshot file set is empty"

checksum_tool_declared=false
snapshot_verifier_declared=false
source_modes=()
consumer_states=()
for index in "${!files[@]}"; do
    path="${files[$index]}"
    validate_relative_path "$path"
    [[ "$path" != "$MANIFEST_PATH" ]] || fail "manifest may not include itself"
    source_file="$SOURCE_ROOT/$path"
    [[ -f "$source_file" && ! -L "$source_file" ]] ||
        fail "source file is missing or symlinked: $path"
    source_parent="$(resolve_directory "${source_file%/*}")"
    case "$source_parent/" in
    "$SOURCE_ROOT/"*) ;;
    *) fail "source file escapes the source checkout: $path" ;;
    esac
    source_modes[index]="$(committed_mode "$source_revision" "$path")" || fail "cannot read committed mode: $path"
    consumer_states[index]="$(consumer_state "$path")" || fail "cannot inspect consumer input: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$STAGING_DIR/index-$index" ||
        fail "cannot inspect consumer index: $path"
    [[ "$path" != "scripts/ci/verify-file-checksum.sh" ]] || checksum_tool_declared=true
    [[ "$path" != "scripts/ci/verify-shared-tooling-snapshot.sh" ]] || snapshot_verifier_declared=true

    for prior_index in "${!files[@]}"; do
        [[ "$prior_index" -lt "$index" ]] || break
        [[ "${files[$prior_index]}" != "$path" ]] || fail "duplicate snapshot path: $path"
    done
done

[[ "$checksum_tool_declared" == "true" ]] || fail "snapshot must include the checksum verifier"
[[ "$snapshot_verifier_declared" == "true" ]] || fail "snapshot must include the snapshot verifier"

staged_files="$STAGING_DIR/files"
mkdir -p "$staged_files"

for index in "${!files[@]}"; do
    path="${files[$index]}"
    mkdir -p "$staged_files/$(dirname "$path")"
    git -C "$SOURCE_ROOT" cat-file blob "$source_revision:$path" >"$staged_files/$path" ||
        fail "cannot export committed source file: $path"
    chmod "${source_modes[$index]#100}" "$staged_files/$path"
done

# Companions are declared by the selected committed source, not guessed from
# shell syntax or this exporter's revision. Only the exact second-line marker
# is metadata; declarations inside a test's heredoc are ordinary source bytes.
for path in "${files[@]}"; do
    declaration="$(sed -n '2s/^# Shared companions: //p' "$staged_files/$path")"
    companions=()
    read -r -a companions <<< "$declaration"
    for companion in ${companions[@]+"${companions[@]}"}; do
        validate_relative_path "$companion"
        selected=false
        for existing in "${files[@]}"; do
            [[ "$existing" != "$companion" ]] || selected=true
        done
        [[ "$selected" == true ]] || fail "$path requires selected companion: $companion (add it explicitly)"
    done
done

staged_manifest="$STAGING_DIR/manifest"
{
    echo "# Shared Tooling snapshot v1"
    printf '# version\t%s\n' "$source_version"
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
for index in "${!files[@]}"; do
    path="${files[$index]}"
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
    [[ -n "$destination_status" ]] || continue
    # Only a declared, unchanged previous export can advance without a consumer
    # commit. Its manifest is not authority to overwrite arbitrary local edits:
    # prove the recorded bytes/mode against the previous committed source too.
    [[ -n "${previous_digests[$index]:-}" && -f "$destination" ]] ||
        fail "consumer destination has local changes; preserve or reconcile them before refreshing: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs diff --cached --quiet -- "$path" ||
        fail "consumer destination has staged changes; preserve or reconcile them before refreshing: $path"
    previous_mode="$(committed_mode "$manifest_revision" "$path")" || fail "previous snapshot source is unavailable: $path"
    previous_executable=-
    [[ "$previous_mode" != 100755 ]] || previous_executable=x
    [[ "$previous_executable" == "${previous_modes[$index]}" ]] || fail "previous snapshot mode disagrees with its source: $path"
    git -C "$SOURCE_ROOT" cat-file blob "$manifest_revision:$path" > "$STAGING_DIR/previous-file" ||
        fail "cannot export previous snapshot source: $path"
    bash "$SCRIPT_ROOT/scripts/ci/verify-file-checksum.sh" sha256 "${previous_digests[$index]}" "$STAGING_DIR/previous-file" ||
        fail "previous snapshot digest disagrees with its source: $path"
    if ! cmp -s "$STAGING_DIR/previous-file" "$destination" ||
        { [[ "$previous_executable" == x ]] && [[ ! -x "$destination" ]]; } ||
        { [[ "$previous_executable" == - ]] && [[ -x "$destination" ]]; }; then
        fail "consumer destination has local changes; preserve or reconcile them before refreshing: $path"
    fi
    [[ "$(git -C "$SOURCE_ROOT" cat-file -t "$manifest_revision")" == commit ]] ||
        fail "previous snapshot revision is not an available commit"
done

# Detect changes made while committed payloads and companions were prepared,
# before publishing any selected file. Keep unrelated working/index edits intact.
check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"
check_index_unchanged manifest "$MANIFEST_PATH"
for index in "${!files[@]}"; do
    check_consumer_unchanged "${files[$index]}" "${consumer_states[$index]}"
    check_index_unchanged "$index" "${files[$index]}"
done

# Prepare a custom manifest's parent before replacing any consumer files.
mkdir -p "$(dirname "$manifest")" || fail "cannot create consumer manifest directory"

for index in "${!files[@]}"; do
    path="${files[$index]}"
    check_consumer_unchanged "$path" "${consumer_states[$index]}"
    check_index_unchanged "$index" "$path"
    mkdir -p "$CONSUMER_ROOT/$(dirname "$path")"
    cp -p "$staged_files/$path" "$CONSUMER_ROOT/$path"
done
check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"
check_index_unchanged manifest "$MANIFEST_PATH"
mv "$staged_manifest" "$manifest"

bash "$staged_files/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$CONSUMER_ROOT" \
    --manifest "$MANIFEST_PATH"

echo "shared-tooling snapshot refreshed: ${#files[@]} file(s) from $source_version ($source_revision)"
