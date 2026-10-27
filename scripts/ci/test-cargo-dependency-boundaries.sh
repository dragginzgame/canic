#!/usr/bin/env bash
# Shared companions: scripts/ci/check-cargo-dependency-boundaries.sh scripts/ci/fixture-lifecycle.sh
set -euo pipefail
root="${BASH_SOURCE[0]}"
[[ "$root" == /* ]] || root="$PWD/$root"
root="$(CDPATH='' cd -P -- "${root%/*}/../.." && printf '%s/.' "$PWD")"
root="${root%/.}"
original_path="$PATH"
real_cargo="$(command -v cargo)"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$root/scripts/ci/fixture-lifecycle.sh"
FIXTURE="$(fixture_create "${TMPDIR:-/tmp}/cargo-boundaries.XXXXXX")"
fixture_complete=false
trap 'fixture_finish "$?" "$FIXTURE" "Boundary fixture retained"' EXIT
mkdir -p "$FIXTURE/ci" "$FIXTURE/bin"
printf '[workspace]\n' > "$FIXTURE/Cargo.toml"
export BOUNDARY_CALLS="$FIXTURE/calls" BOUNDARY_MODE=pass
cat > "$FIXTURE/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'metadata --no-deps --locked --format-version 1' ]]
[[ "$CARGO_NET_OFFLINE" == true && "$RUSTUP_AUTO_INSTALL" == 0 ]]
printf 'metadata\n' >> "$BOUNDARY_CALLS"
cat metadata.json
case "$BOUNDARY_MODE" in failed) exit 29 ;; signal) kill -TERM "$$" ;; esac
MOCK
chmod +x "$FIXTURE/bin/cargo"
export PATH="$FIXTURE/bin:$PATH"
perl -MJSON::PP -e 'print encode_json({version => 1, workspace_root => $ARGV[0], workspace_members => ["consumer"], packages => [{id => "consumer", name => "consumer", dependencies => [{name => "substrate", rename => "hidden", kind => "dev", target => "cfg(windows)"}]}]})' "$FIXTURE" > "$FIXTURE/metadata.json"
cp "$FIXTURE/metadata.json" "$FIXTURE/valid-metadata.json"
# Common rules are opt-in, cover renamed non-production declarations, and
# cannot be relaxed by a consumer allowlist. Platform remains the explicit owner.
for package in consumer ic-platform; do
    perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>$ARGV[1],dependencies=>[{name=>"candid",rename=>"hidden",kind=>"dev",target=>"cfg(windows)"}]}]})' "$FIXTURE" "$package" > "$FIXTURE/metadata.json"
    printf '[{"dependency":"candid","allowed_packages":["consumer","ic-platform"]}]\n' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
    bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE"
    status=0
    bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/common.log" 2>&1 || status=$?
    if [[ "$package" == consumer ]]; then [[ "$status" != 0 ]] || exit 1
    else [[ "$status" == 0 ]] || exit 1; fi
done
rm "$FIXTURE/ci/cargo-dependency-boundaries.json"
bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic
printf '[{"dependency":"candid","allowed_packages":[]}]\n' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/tightened.log" 2>&1; then exit 1; fi
cp "$FIXTURE/valid-metadata.json" "$FIXTURE/metadata.json"
printf '%s\n' '[{"dependency":"substrate","allowed_packages":["consumer"]}]' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE"
for policy in \
    '[{"dependency":"substrate","allowed_packages":[]}]' \
    '[{"dependency":"substrate","allowed_packages":["owner"]}]' \
    '[{"dependency":"substrate","allowed_packages":["consumer"],"typo":true}]' \
    '[{"dependency":"substrate","allowed_packages":"consumer"}]' \
    '[{"dependency":"substrate","allowed_packages":[]},{"dependency":"substrate","allowed_packages":["consumer"]}]' \
    '[{"dependency":"substrate","allowed_packages":["consumer","consumer"]}]' \
    '[]' '{' ; do
    printf '%s\n' "$policy" > "$FIXTURE/ci/cargo-dependency-boundaries.json"
    : > "$BOUNDARY_CALLS"
    if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" > "$FIXTURE/output" 2>&1; then
        echo "incorrectly accepted policy: $policy" >&2
        exit 1
    fi
    case "$policy" in
        '[{"dependency":"substrate","allowed_packages":[]}]'|'[{"dependency":"substrate","allowed_packages":["owner"]}]') ;;
        *) [[ ! -s "$BOUNDARY_CALLS" ]] || exit 1 ;;
    esac
done
printf '%s\n' '[{"dependency":"different","allowed_packages":[]}]' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE"
# Successful-looking JSON from a failed or signalled child is not an observation.
for mode in failed signal; do
    if BOUNDARY_MODE="$mode" bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" > "$FIXTURE/$mode.log" 2>&1; then exit 1; fi
    if grep -q 'boundaries passed' "$FIXTURE/$mode.log"; then exit 1; fi
done
for malformed in missing empty duplicate missing-package bad-package bad-dependency wrong-root; do
    perl -MJSON::PP -0e '
        my $metadata = decode_json(<STDIN>); my $mode = shift @ARGV;
        if ($mode eq "missing") { $metadata = {} }
        elsif ($mode eq "empty") { $metadata->{workspace_members} = [] }
        elsif ($mode eq "duplicate") { push @{$metadata->{workspace_members}}, "consumer" }
        elsif ($mode eq "missing-package") { $metadata->{packages} = [] }
        elsif ($mode eq "bad-package") { $metadata->{packages}[0]{dependencies} = {} }
        elsif ($mode eq "bad-dependency") { $metadata->{packages}[0]{dependencies}[0] = {} }
        elsif ($mode eq "wrong-root") { $metadata->{workspace_root} .= "/elsewhere" }
        print encode_json($metadata);
    ' "$malformed" < "$FIXTURE/valid-metadata.json" > "$FIXTURE/metadata.json"
    if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" > "$FIXTURE/$malformed.log" 2>&1; then exit 1; fi
    if grep -q 'boundaries passed' "$FIXTURE/$malformed.log"; then exit 1; fi
done
cp "$FIXTURE/valid-metadata.json" "$FIXTURE/metadata.json"
mv "$FIXTURE/ci/cargo-dependency-boundaries.json" "$FIXTURE/policy.saved"
ln -s ../policy.saved "$FIXTURE/ci/cargo-dependency-boundaries.json"
: > "$BOUNDARY_CALLS"
if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" > "$FIXTURE/symlink.log" 2>&1; then exit 1; fi
[[ ! -s "$BOUNDARY_CALLS" ]] || exit 1
rm "$FIXTURE/ci/cargo-dependency-boundaries.json"
cp "$FIXTURE/policy.saved" "$FIXTURE/ci/cargo-dependency-boundaries.json"
# Raw and aliased forbidden directories refuse before Cargo or policy reads.
for suffix in $'\n' $'\r'; do
    bad="$FIXTURE/directory$suffix"
    mkdir "$bad"
    ln -s "$bad" "$FIXTURE/alias"
    for selected in "$bad" "$FIXTURE/alias"; do
        if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$selected" > "$FIXTURE/path.log" 2>&1; then exit 1; fi
    done
    rm "$FIXTURE/alias"
done
[[ ! -s "$BOUNDARY_CALLS" ]] || exit 1
# Real Cargo proves canonical renamed/build/dev/target declarations without
# fetching dependencies or compiling. Only dependency-free local fixture crates.
actual="$FIXTURE/actual workspace"
mkdir -p "$actual/ci"
printf '[workspace]\nmembers=["owner","consumer","substrate"]\nresolver="2"\n' > "$actual/Cargo.toml"
for package in owner consumer substrate; do
    mkdir -p "$actual/$package/src"
    printf '[package]\nname="%s"\nversion="0.0.0"\nedition="2021"\n' "$package" > "$actual/$package/Cargo.toml"
    : > "$actual/$package/src/lib.rs"
done
cat >> "$actual/owner/Cargo.toml" <<'TOML'
[dependencies]
normal = {package="substrate",path="../substrate"}
[build-dependencies]
normal = {package="substrate",path="../substrate"}
[target.'cfg(windows)'.dev-dependencies]
normal = {package="substrate",path="../substrate"}
TOML
(cd "$actual"; PATH="$original_path" CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 "$real_cargo" generate-lockfile)
cp "$actual/Cargo.lock" "$FIXTURE/lock.before"
printf '%s\n' '[{"dependency":"substrate","allowed_packages":["owner"]}]' > "$actual/ci/cargo-dependency-boundaries.json"
PATH="$original_path" bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$actual"
printf '%s\n' '[{"dependency":"substrate","allowed_packages":[]}]' > "$actual/ci/cargo-dependency-boundaries.json"
if PATH="$original_path" bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$actual" > "$FIXTURE/actual-denied.log" 2>&1; then exit 1; fi
grep -Fx 'owner -> substrate' "$FIXTURE/actual-denied.log" > /dev/null
cmp "$FIXTURE/lock.before" "$actual/Cargo.lock"
fixture_complete=true
echo 'Dependency boundaries, failed/malformed observations and actual offline Cargo declarations passed'
