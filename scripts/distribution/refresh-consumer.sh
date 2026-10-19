#!/usr/bin/env bash
# Shared companions: scripts/ci/shared-companions.sh
set -euo pipefail

SCRIPT_ROOT="${BASH_SOURCE[0]}"
[[ "$SCRIPT_ROOT" == /* ]] || SCRIPT_ROOT="$PWD/$SCRIPT_ROOT"
SCRIPT_ROOT="$(cd -P "${SCRIPT_ROOT%/*}/../.." && printf '%s/.' "$PWD")"
SCRIPT_ROOT="${SCRIPT_ROOT%/.}"
# shellcheck source=scripts/ci/shared-companions.sh
source "$SCRIPT_ROOT/scripts/ci/shared-companions.sh"
SOURCE_ROOT="$SCRIPT_ROOT"
CONSUMER_ROOT=""
MANIFEST_PATH=".shared-tooling.snapshot"
STAGING_DIR=""
replacement=""
preview=false
requested_files=()
added_files=()
takeover_roots=()
takeover_manifests=()
takeover_paths=()
takeover_states=()
takeover_payload_paths=()
takeover_payload_states=()

usage() {
    cat >&2 <<'USAGE'
usage: refresh-consumer.sh [--preview] [--source <shared-tooling-checkout>] --consumer <repository> [--manifest <relative-path>] [--entrypoint <relative-path>]... [--add-entrypoint <relative-path>]... [--takeover-snapshot <relative-root|.> <relative-manifest>]...

Use --entrypoint to select the complete set of roots, initially or on refresh.
Otherwise refresh reuses the manifest's recorded entrypoints. --add-entrypoint
extends that selection. Required companions and integrity tools are derived.
--preview leaves the consumer untouched and reports additions, updates and
retirements. Refresh automatically removes unchanged, previously owned files
outside the new closure. Select directly used helpers as entrypoints to keep them.
USAGE
}

fail() {
    echo "shared-tooling refresh failed: $1" >&2
    exit 1
}

cleanup() {
    if [[ -n "$replacement" ]]; then rm -f -- "$replacement"; fi
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
    # pipefail carries a failed Git observation even after a complete row. A
    # process substitution hides that status, and Bash 3.2 cannot wait for it.
    git -C "$SOURCE_ROOT" ls-tree -z "$revision" -- "$path" | {
        while IFS= read -r -d '' tree_entry; do
            [[ "${tree_entry#*$'\t'}" == "$path" ]] || continue
            tree_metadata="${tree_entry%%$'\t'*}"
            read -r entry_mode entry_type _ <<<"$tree_metadata"
            [[ "$entry_type" == blob && "$entry_mode" =~ ^100(644|755)$ ]] ||
                fail "source revision does not contain a regular file: $path"
            mode="$entry_mode"
        done
        [[ -n "$mode" ]] || fail "source file is not recorded in revision $revision: $path"
        printf '%s\n' "$mode"
    }
}

check_consumer_unchanged() {
    local path="$1" expected="$2" current
    current="$(consumer_state "$path")" || fail "cannot recheck consumer input: $path"
    [[ "$current" == "$expected" ]] || fail "consumer input changed during refresh: $path"
}

# The manifest alone cannot authorize replacing or retiring consumer bytes.
# Resolve its ownership claim against the exact earlier source for both paths.
export_previous_payload() {
    local index="$1" output="$2" path mode revision_kind executable=-
    path="${previous_sources[$index]}"
    local revision="${previous_revisions[$index]}"
    revision_kind="$(git -C "$SOURCE_ROOT" cat-file -t "$revision")" ||
        fail "cannot inspect previous snapshot revision"
    [[ "$revision_kind" == commit ]] || fail "previous snapshot revision is not an available commit"
    mode="$(committed_mode "$revision" "$path")" || fail "previous snapshot source is unavailable: $path"
    [[ "$mode" != 100755 ]] || executable=x
    [[ "$executable" == "${previous_modes[$index]}" ]] || fail "previous snapshot mode disagrees with its source: $path"
    git -C "$SOURCE_ROOT" cat-file blob "$revision:$path" > "$output" ||
        fail "cannot export previous snapshot source: $path"
    bash "$SCRIPT_ROOT/scripts/ci/verify-file-checksum.sh" sha256 "${previous_digests[$index]}" "$output" ||
        fail "previous snapshot digest disagrees with its source: $path"
    chmod "${mode#100}" "$output"
}

check_index_unchanged() {
    local index="$1" path="$2"
    local recorded="$STAGING_DIR/index-$index" observed="$STAGING_DIR/current-index"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$observed" ||
        fail "cannot inspect consumer index: $path"
    # Reobserve Git on every check. Two readable, regular empty records already
    # prove equality; retain byte comparison for all other index observations.
    if [[ -f "$recorded" && -r "$recorded" && ! -s "$recorded" &&
          -f "$observed" && -r "$observed" && ! -s "$observed" ]]; then
        return 0
    fi
    cmp -s "$recorded" "$observed" ||
        fail "consumer index changed during refresh: $path"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --preview)
        preview=true
        export GIT_OPTIONAL_LOCKS=0
        shift
        ;;
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
    --entrypoint)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        requested_files[${#requested_files[@]}]="$2"
        shift 2
        ;;
    --add-entrypoint)
        [[ $# -ge 2 ]] || { usage; exit 2; }
        added_files[${#added_files[@]}]="$2"
        shift 2
        ;;
    --takeover-snapshot)
        [[ $# -ge 3 ]] || { usage; exit 2; }
        takeover_roots+=("$2")
        takeover_manifests+=("$3")
        shift 3
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
[[ "$source_revision" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || fail "source revision is malformed"
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
if [[ "$preview" == true ]]; then
    preview_tmp="$(resolve_directory "${TMPDIR:-/tmp}")"
    case "$preview_tmp/" in
        "$SOURCE_ROOT/"*|"$CONSUMER_ROOT/"*) fail "preview temporary directory must be outside source and consumer" ;;
    esac
    STAGING_DIR="$(mktemp -d "$preview_tmp/shared-tooling-preview.XXXXXX")"
else
    STAGING_DIR="$(mktemp -d "$CONSUMER_ROOT/.shared-tooling-refresh.XXXXXX")"
fi
manifest_state="$(consumer_state "$MANIFEST_PATH")" || fail "cannot inspect consumer manifest"
git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$MANIFEST_PATH" > "$STAGING_DIR/index-manifest" ||
    fail "cannot inspect consumer manifest index"
files=()
entrypoints=()
previous_files=()
previous_digests=()
previous_modes=()
previous_sources=()
previous_revisions=()

if [[ -e "$manifest" ]]; then
    [[ -f "$manifest" && ! -L "$manifest" ]] || fail "consumer manifest must be a regular file"
    cp -p "$manifest" "$STAGING_DIR/previous-manifest"
    check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"

    # Admit syntax once through our trusted reader. Payload ownership remains
    # independently checked against the earlier committed source below.
    bash "$SCRIPT_ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
        --read-manifest "$STAGING_DIR/previous-manifest" > "$STAGING_DIR/previous-records" ||
        fail "existing manifest is malformed"
    while IFS=$'\t' read -r record first second third; do
        case "$record" in
            '# entrypoint') entrypoints+=("$first") ;;
            source) manifest_source="$first" ;;
            revision) manifest_revision="$first" ;;
            file)
                [[ "$third" != "$MANIFEST_PATH" ]] || fail "manifest may not include itself"
                previous_digests[${#previous_files[@]}]="$first"
                previous_modes[${#previous_files[@]}]="$second"
                previous_files+=("$third")
                previous_sources+=("$third")
                previous_revisions+=("$manifest_revision") ;;
        esac
    done < "$STAGING_DIR/previous-records"
    [[ "$manifest_source" == "$source_remote" ]] ||
        fail "source remote differs from the existing manifest"
else
    [[ ${#added_files[@]} -eq 0 ]] || fail "--add-entrypoint requires an existing snapshot"
fi

for takeover_index in "${!takeover_roots[@]}"; do
    relative_root="${takeover_roots[$takeover_index]}"
    prior_manifest="${takeover_manifests[$takeover_index]}"
    [[ "$relative_root" == . ]] || validate_relative_path "$relative_root"
    validate_relative_path "$prior_manifest"
    prefix="${relative_root}/"
    [[ "$relative_root" != . ]] || prefix=''
    prior_path="$prefix$prior_manifest"
    [[ "$prior_path" != "$MANIFEST_PATH" ]] || fail "cannot take over the destination manifest"
    for existing in ${takeover_paths[@]+"${takeover_paths[@]}"}; do
        [[ "$existing" != "$prior_path" ]] || fail "duplicate takeover manifest"
    done
    prior_state="$(consumer_state "$prior_path")" || fail "cannot inspect takeover manifest"
    [[ -f "$CONSUMER_ROOT/$prior_path" ]] || fail "takeover manifest is missing: $prior_path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs diff --cached --quiet -- "$prior_path" ||
        fail "takeover manifest has staged changes: $prior_path"
    cp -p "$CONSUMER_ROOT/$prior_path" "$STAGING_DIR/takeover-$takeover_index"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$prior_path" > "$STAGING_DIR/index-takeover-$takeover_index"
    takeover_paths+=("$prior_path")
    takeover_states+=("$prior_state")
    bash "$SCRIPT_ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
        --read-manifest "$STAGING_DIR/takeover-$takeover_index" > "$STAGING_DIR/takeover-records" ||
        fail "takeover manifest is malformed"
    prior_revision='' prior_remote=''
    while IFS=$'\t' read -r record first second third; do
        case "$record" in
            source) prior_remote="$first" ;;
            revision) prior_revision="$first" ;;
        esac
    done < "$STAGING_DIR/takeover-records"
    [[ "$prior_remote" == "$source_remote" ]] || fail "takeover source remote differs"
    while IFS=$'\t' read -r record first second third; do
        case "$record" in
            '# entrypoint')
                selected=false
                for existing in ${entrypoints[@]+"${entrypoints[@]}"}; do
                    [[ "$existing" != "$first" ]] || selected=true
                done
                [[ "$selected" == true ]] || entrypoints+=("$first") ;;
            file)
                [[ "$third" != "$prior_manifest" ]] || fail "takeover manifest includes itself"
                physical_path="$prefix$third"
                index=${#previous_files[@]}
                previous_files+=("$physical_path")
                previous_sources+=("$third")
                previous_revisions+=("$prior_revision")
                previous_digests+=("$first")
                previous_modes+=("$second")
                export_previous_payload "$index" "$STAGING_DIR/takeover-payload" || fail "unproved takeover payload"
                takeover_payload_paths+=("$physical_path")
                takeover_payload_states+=("$(consumer_state "$physical_path")")
                git -C "$CONSUMER_ROOT" --literal-pathspecs diff --cached --quiet -- "$physical_path" ||
                    fail "takeover payload has staged changes: $physical_path"
                if [[ -f "$CONSUMER_ROOT/$physical_path" ]]; then
                    cmp -s "$STAGING_DIR/takeover-payload" "$CONSUMER_ROOT/$physical_path" || fail "takeover payload has local changes: $physical_path"
                    if { [[ -x "$STAGING_DIR/takeover-payload" ]] && [[ ! -x "$CONSUMER_ROOT/$physical_path" ]]; } ||
                        { [[ ! -x "$STAGING_DIR/takeover-payload" ]] && [[ -x "$CONSUMER_ROOT/$physical_path" ]]; }; then
                        fail "takeover payload has local mode changes: $physical_path"
                    fi
                fi
                ;;
        esac
    done < "$STAGING_DIR/takeover-records"
done

if [[ ${#requested_files[@]} -gt 0 ]]; then entrypoints=("${requested_files[@]}"); fi
[[ ${#entrypoints[@]} -gt 0 ]] || fail "select the complete --entrypoint set; this manifest has no recorded entrypoints"
for path in ${added_files[@]+"${added_files[@]}"}; do
    validate_relative_path "$path"
    selected=false
    for existing in "${entrypoints[@]}"; do
        [[ "$existing" != "$path" ]] || selected=true
    done
    if [[ "$selected" == false ]]; then entrypoints+=("$path"); fi
done

for index in "${!entrypoints[@]}"; do
    path="${entrypoints[$index]}"
    validate_relative_path "$path"
    for prior_index in "${!entrypoints[@]}"; do
        [[ "$prior_index" -lt "$index" ]] || break
        [[ "${entrypoints[$prior_index]}" != "$path" ]] || fail "duplicate entrypoint: $path"
    done
done
files=("${entrypoints[@]}")
# Integrity verification is required for every selection, independently of roots.
for path in scripts/ci/verify-file-checksum.sh scripts/ci/verify-shared-tooling-snapshot.sh; do
    selected=false
    for existing in "${files[@]}"; do [[ "$existing" != "$path" ]] || selected=true; done
    if [[ "$selected" == false ]]; then files+=("$path"); fi
done
consumer_states=()
selected_count=${#files[@]}
for index in "${!files[@]}"; do
    path="${files[$index]}"
    validate_relative_path "$path"
    consumer_states[index]="$(consumer_state "$path")" || fail "cannot inspect consumer input: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$STAGING_DIR/index-$index" ||
        fail "cannot inspect consumer index: $path"
done

# Follow the selected revision's explicit companion graph, including cycles,
# without executing inspected files. The committed closure is the payload owner.
proposed=("${files[@]}")
source_modes=()
staged_files="$STAGING_DIR/files"
mkdir -p "$staged_files"
cursor=0
while [[ "$cursor" -lt "${#proposed[@]}" ]]; do
    path="${proposed[$cursor]}"
    validate_relative_path "$path"
    # These observations belong to the immutable selected revision. Retain them
    # for export; mutable checkout and consumer admission still run below.
    source_modes[cursor]="$(committed_mode "$source_revision" "$path")" || fail "cannot read committed companion source: $path"
    staged_parent="$staged_files"
    if [[ "$path" == */* ]]; then staged_parent+="/${path%/*}"; fi
    [[ -d "$staged_parent" ]] || mkdir -p "$staged_parent"
    git -C "$SOURCE_ROOT" cat-file blob "$source_revision:$path" > "$staged_files/$path" || fail "cannot inspect companions: $path"
    read_shared_companions "$staged_files/$path" "$path"
    for companion in ${shared_companions[@]+"${shared_companions[@]}"}; do
        validate_relative_path "$companion"
        selected=false
        for existing in "${proposed[@]}"; do
            [[ "$existing" != "$companion" ]] || selected=true
        done
        if [[ "$selected" == false ]]; then
            proposed+=("$companion")
        fi
    done
    cursor=$((cursor + 1))
done
files=("${proposed[@]}")

checksum_tool_declared=false
snapshot_verifier_declared=false
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
    if [[ "$index" -ge "$selected_count" ]]; then
        consumer_states[index]="$(consumer_state "$path")" || fail "cannot inspect consumer input: $path"
        git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$STAGING_DIR/index-$index" ||
            fail "cannot inspect consumer index: $path"
    fi
    [[ "$path" != "scripts/ci/verify-file-checksum.sh" ]] || checksum_tool_declared=true
    [[ "$path" != "scripts/ci/verify-shared-tooling-snapshot.sh" ]] || snapshot_verifier_declared=true

    for prior_index in "${!files[@]}"; do
        [[ "$prior_index" -lt "$index" ]] || break
        [[ "${files[$prior_index]}" != "$path" ]] || fail "duplicate snapshot path: $path"
    done
done

[[ "$checksum_tool_declared" == "true" ]] || fail "snapshot must include the checksum verifier"
[[ "$snapshot_verifier_declared" == "true" ]] || fail "snapshot must include the snapshot verifier"

for index in "${!files[@]}"; do
    path="${files[$index]}"
    chmod "${source_modes[$index]#100}" "$staged_files/$path"
done

staged_manifest="$STAGING_DIR/manifest"
{
    echo "# Shared Tooling snapshot v1"
    printf '# version\t%s\n' "$source_version"
    printf 'format\t1\n'
    printf 'source\t%s\n' "$source_remote"
    printf 'revision\t%s\n' "$source_revision"
    for path in "${entrypoints[@]}"; do printf '# entrypoint\t%s\n' "$path"; done
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
    previous_index=-1
    for candidate in "${!previous_files[@]}"; do
        [[ "${previous_files[$candidate]}" != "$path" ]] || previous_index="$candidate"
    done
    [[ "$previous_index" -ge 0 && -f "$destination" ]] ||
        fail "consumer destination has local changes; preserve or reconcile them before refreshing: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs diff --cached --quiet -- "$path" ||
        fail "consumer destination has staged changes; preserve or reconcile them before refreshing: $path"
    export_previous_payload "$previous_index" "$STAGING_DIR/previous-file"
    if ! cmp -s "$STAGING_DIR/previous-file" "$destination" ||
        { [[ -x "$STAGING_DIR/previous-file" ]] && [[ ! -x "$destination" ]]; } ||
        { [[ ! -x "$STAGING_DIR/previous-file" ]] && [[ -x "$destination" ]]; }; then
        fail "consumer destination has local changes; preserve or reconcile them before refreshing: $path"
    fi
done

# Retire stale payloads automatically. Prove ownership against
# the previous committed source even when Git considers the consumer clean.
# An absent owned file is a safe interrupted-deletion retry, not authority to
# remove any replacement, parent directory, tool generation or evidence.
obsolete=()
retirement_states=()
for previous_index in "${!previous_files[@]}"; do
    path="${previous_files[$previous_index]}"
    selected=false
    for existing in "${files[@]}"; do [[ "$existing" != "$path" ]] || selected=true; done
    [[ "$selected" == false ]] || continue
    duplicate=false
    for existing in ${obsolete[@]+"${obsolete[@]}"}; do [[ "$existing" != "$path" ]] || duplicate=true; done
    [[ "$duplicate" == false ]] || continue
    index=${#obsolete[@]}
    obsolete+=("$path")
    retirement_states[index]="$(consumer_state "$path")" || fail "cannot inspect retirement: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs ls-files --stage -z -- "$path" > "$STAGING_DIR/index-retire-$index" ||
        fail "cannot inspect retirement index: $path"
    git -C "$CONSUMER_ROOT" --literal-pathspecs diff --cached --quiet -- "$path" ||
        fail "retirement has staged changes: $path"
    export_previous_payload "$previous_index" "$STAGING_DIR/retire-$index"
    destination="$CONSUMER_ROOT/$path"
    if [[ -e "$destination" ]]; then
        cmp -s "$STAGING_DIR/retire-$index" "$destination" || fail "retirement has local changes: $path"
        if { [[ -x "$STAGING_DIR/retire-$index" ]] && [[ ! -x "$destination" ]]; } ||
            { [[ ! -x "$STAGING_DIR/retire-$index" ]] && [[ -x "$destination" ]]; }; then
            fail "retirement has local mode changes: $path"
        fi
    fi
done

for index in "${!takeover_paths[@]}"; do
    path="${takeover_paths[$index]}"
    for existing in "${files[@]}" "${previous_files[@]}"; do
        [[ "$existing" != "$path" ]] || fail "takeover manifest collides with a payload: $path"
    done
    check_consumer_unchanged "$path" "${takeover_states[$index]}"
    check_index_unchanged "takeover-$index" "$path"
done

for index in "${!takeover_payload_paths[@]}"; do
    check_consumer_unchanged "${takeover_payload_paths[$index]}" "${takeover_payload_states[$index]}"
done

# Detect changes made while committed payloads and companions were prepared,
# before publishing any selected file. Keep unrelated working/index edits intact.
check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"
check_index_unchanged manifest "$MANIFEST_PATH"
for index in "${!files[@]}"; do
    check_consumer_unchanged "${files[$index]}" "${consumer_states[$index]}"
    check_index_unchanged "$index" "${files[$index]}"
done
for index in "${!obsolete[@]}"; do
    check_consumer_unchanged "${obsolete[$index]}" "${retirement_states[$index]}"
    check_index_unchanged "retire-$index" "${obsolete[$index]}"
done

if [[ "$preview" == true ]]; then
    printf 'source\t%s\t%s\n' "$source_version" "$source_revision"
    printf 'manifest\t%s\n' "$MANIFEST_PATH"
    for path in "${files[@]}"; do
        action=UPDATE
        destination="$CONSUMER_ROOT/$path"
        if [[ ! -e "$destination" ]]; then action=ADD
        elif cmp -s "$staged_files/$path" "$destination" &&
            { { [[ -x "$destination" ]] && [[ -x "$staged_files/$path" ]]; } ||
              { [[ ! -x "$destination" ]] && [[ ! -x "$staged_files/$path" ]]; }; }; then action=KEEP
        fi
        selection=required
        for entrypoint in "${entrypoints[@]}"; do
            [[ "$path" != "$entrypoint" ]] || selection=selected
        done
        printf '%s\t%s\t%s\n' "$action" "$selection" "$path"
    done
    for path in ${obsolete[@]+"${obsolete[@]}"}; do
        printf 'REMOVE\tobsolete\t%s\n' "$path"
    done
    for path in ${takeover_paths[@]+"${takeover_paths[@]}"}; do
        printf 'REMOVE\ttakeover-manifest\t%s\n' "$path"
    done
    exit 0
fi

# Prepare a custom manifest's parent before replacing any consumer files.
mkdir -p "${manifest%/*}" || fail "cannot create consumer manifest directory"

for index in "${!files[@]}"; do
    path="${files[$index]}"
    check_consumer_unchanged "$path" "${consumer_states[$index]}"
    check_index_unchanged "$index" "$path"
    destination_parent="$CONSUMER_ROOT"
    if [[ "$path" == */* ]]; then destination_parent+="/${path%/*}"; fi
    [[ -d "$destination_parent" ]] || mkdir -p "$destination_parent"
    # Copy beside the destination so even a mounted subtree gets an atomic
    # rename. A failed/interrupted copy must leave the admitted old bytes intact.
    replacement="$(mktemp "$destination_parent/.shared-tooling-payload.XXXXXX")"
    cp -p "$staged_files/$path" "$replacement"
    check_consumer_unchanged "$path" "${consumer_states[$index]}"
    check_index_unchanged "$index" "$path"
    perl -e 'rename($ARGV[0], $ARGV[1]) or die "cannot replace snapshot payload: $!\n"' \
        "$replacement" "$CONSUMER_ROOT/$path"
    replacement=""
done
for index in "${!obsolete[@]}"; do
    path="${obsolete[$index]}"
    check_consumer_unchanged "$path" "${retirement_states[$index]}"
    check_index_unchanged "retire-$index" "$path"
    if [[ -e "$CONSUMER_ROOT/$path" ]]; then rm -- "$CONSUMER_ROOT/$path"; fi
done
for index in "${!takeover_paths[@]}"; do
    check_consumer_unchanged "${takeover_paths[$index]}" "${takeover_states[$index]}"
    check_index_unchanged "takeover-$index" "${takeover_paths[$index]}"
done
check_consumer_unchanged "$MANIFEST_PATH" "$manifest_state"
check_index_unchanged manifest "$MANIFEST_PATH"
mv "$staged_manifest" "$manifest"

bash "$staged_files/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$CONSUMER_ROOT" \
    --manifest "$MANIFEST_PATH"

# Retire receipts only after the new authoritative manifest and bytes verify.
for index in "${!takeover_paths[@]}"; do
    check_consumer_unchanged "${takeover_paths[$index]}" "${takeover_states[$index]}"
    check_index_unchanged "takeover-$index" "${takeover_paths[$index]}"
    rm -- "$CONSUMER_ROOT/${takeover_paths[$index]}"
done

echo "shared-tooling snapshot refreshed: ${#files[@]} file(s) from $source_version ($source_revision)"
