#!/usr/bin/env bash
# Shared companions: scripts/ci/check-cargo-dependency-boundaries.sh scripts/ci/fixture-lifecycle.sh ci/examples/platform-owner-boundaries.json
set -euo pipefail
selection=all
if [[ $# != 0 ]]; then
    [[ $# == 2 && $1 == --case && $2 == native-client ]] || {
        echo 'usage: test-cargo-dependency-boundaries.sh [--case native-client]' >&2; exit 2;
    }
    selection=native-client
fi
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
if [[ $selection == all ]]; then
    for package in consumer ic-platform ic-platform-pocket-ic; do
        perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>$ARGV[1],dependencies=>[{name=>"candid",rename=>"hidden",kind=>"dev",target=>"cfg(windows)"}]}]})' "$FIXTURE" "$package" > "$FIXTURE/metadata.json"
        printf '[{"dependency":"candid","allowed_packages":["consumer","ic-platform","ic-platform-pocket-ic"]}]\n' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
        bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE"
        status=0
        bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/common.log" 2>&1 || status=$?
        if [[ "$package" != ic-platform ]]; then [[ "$status" != 0 ]] || exit 1
        else [[ "$status" == 0 ]] || exit 1; fi
    done
fi
# The maintained PocketIC implementation owns only its four native SDK inputs.
# The common exception never grants build/dev or portable/Wasm declarations.
for dependency in candid ic-certification ic-management-canister-types ic-transport-types ic-agent; do
    for profile in native portable wasm build dev; do
        perl -MJSON::PP -e '
          my ($root,$dependency,$profile)=@ARGV;
          my $target=$profile eq "portable" ? undef : $profile eq "wasm" ? q{cfg(target_family = "wasm")} : q{cfg(not(target_family = "wasm"))};
          my $kind=$profile eq "build" || $profile eq "dev" ? $profile : undef;
          print encode_json({version=>1,workspace_root=>$root,workspace_members=>["member"],packages=>[{id=>"member",name=>"ic-testkit",dependencies=>[{name=>$dependency,rename=>"hidden",kind=>$kind,target=>$target}]}]});
        ' "$FIXTURE" "$dependency" "$profile" > "$FIXTURE/metadata.json"
        printf '[{"dependency":"%s","allowed_packages":["ic-testkit"]}]\n' "$dependency" > "$FIXTURE/ci/cargo-dependency-boundaries.json"
        actual=reject
        if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/pocket-$dependency-$profile.log" 2>&1; then actual=pass; fi
        expected=reject
        if [[ "$profile" == native && "$dependency" != ic-agent ]]; then expected=pass; fi
        [[ "$actual" == "$expected" ]] || { cat "$FIXTURE/pocket-$dependency-$profile.log" >&2; exit 1; }
    done
done

# The retired implementation cannot regain the exception through a local allowlist.
for dependency in candid ic-certification ic-management-canister-types ic-transport-types; do
    perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>"ic-platform-pocket-ic",dependencies=>[{name=>$ARGV[1],kind=>undef,target=>q{cfg(not(target_family = "wasm"))}}]}]})' "$FIXTURE" "$dependency" > "$FIXTURE/metadata.json"
    printf '[{"dependency":"%s","allowed_packages":["ic-platform-pocket-ic"]}]\n' "$dependency" > "$FIXTURE/ci/cargo-dependency-boundaries.json"
    if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/retired-$dependency.log" 2>&1; then exit 1; fi
    grep -Fx "ic-platform-pocket-ic -> $dependency" "$FIXTURE/retired-$dependency.log" >/dev/null
done
# Consumer policy can still restrict the new native owner.
perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>"ic-testkit",dependencies=>[{name=>"candid",kind=>undef,target=>q{cfg(not(target_family = "wasm"))}}]}]})' "$FIXTURE" > "$FIXTURE/metadata.json"
printf '[{"dependency":"candid","allowed_packages":[]}]\n' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/client-tightened.log" 2>&1; then exit 1; fi
grep -Fx 'ic-testkit -> candid' "$FIXTURE/client-tightened.log" >/dev/null
if [[ $selection == native-client ]]; then
    fixture_complete=true
    echo 'Native Testkit boundary admits four SDK inputs; aliases, other profiles, retired owners and tightened policy remain enforced'
    exit 0
fi

# Stable canonical identity owns the registry Principal conversions. This narrow
# exception cannot grant Candid or another external IC dependency to that owner.
for package in consumer ic-platform ic-platform-types ic-platform-runtime; do
    for dependency in ic_principal candid; do
        perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>$ARGV[1],dependencies=>[{name=>$ARGV[2],rename=>"hidden",kind=>"dev",target=>"cfg(windows)"}]}]})' "$FIXTURE" "$package" "$dependency" > "$FIXTURE/metadata.json"
        printf '[{"dependency":"%s","allowed_packages":["consumer","ic-platform","ic-platform-types","ic-platform-runtime"]}]\n' "$dependency" > "$FIXTURE/ci/cargo-dependency-boundaries.json"
        status=0
        bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/owner-$package-$dependency.log" 2>&1 || status=$?
        if [[ "$package" == ic-platform || ( "$package" == ic-platform-types && "$dependency" == ic_principal ) ]]; then
            [[ "$status" == 0 ]] || exit 1
        else [[ "$status" != 0 ]] || exit 1; fi
    done
done
# A local policy may still tighten the admitted stable-owner case.
perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>"ic-platform-types",dependencies=>[{name=>"ic_principal"}]}]})' "$FIXTURE" > "$FIXTURE/metadata.json"
printf '[{"dependency":"ic_principal","allowed_packages":[]}]\n' > "$FIXTURE/ci/cargo-dependency-boundaries.json"
if bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" --consumer "$FIXTURE" --external-ic > "$FIXTURE/stable-tightened.log" 2>&1; then exit 1; fi
# Restore the facade case used by the original common-owner checks below.
perl -MJSON::PP -e 'print encode_json({version=>1,workspace_root=>$ARGV[0],workspace_members=>["member"],packages=>[{id=>"member",name=>"ic-platform",dependencies=>[{name=>"candid"}]}]})' "$FIXTURE" > "$FIXTURE/metadata.json"
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
# Exercise the documented owner policy through real Cargo declarations. These
# dependency-free stand-ins prove policy wiring, not production SDK behavior.
owners="$FIXTURE/platform owners"
mkdir -p "$owners/ci"
cp "$root/ci/examples/platform-owner-boundaries.json" "$owners/ci/cargo-dependency-boundaries.json"
printf '[workspace]\nmembers=["identity-model","ic0-adapter","crypto-adapter","ic-platform-types","ic-platform-runtime","ic-platform"]\nresolver="2"\n' > "$owners/Cargo.toml"
for package in identity-model ic0-adapter crypto-adapter ic-platform-types ic-platform-runtime ic-platform; do
    mkdir -p "$owners/$package/src"
    printf '[package]\nname="%s"\nversion="0.0.0"\nedition="2021"\n' "$package" > "$owners/$package/Cargo.toml"
    cp "$owners/$package/Cargo.toml" "$owners/$package/base.toml"
    : > "$owners/$package/src/lib.rs"
done
(cd "$owners"; PATH="$original_path" CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 "$real_cargo" generate-lockfile)
for dependency in ic-platform-types ic-platform-runtime ic-platform; do
    case "$dependency" in
        ic-platform-types) allowed=identity-model; denied=ic0-adapter ;;
        ic-platform-runtime) allowed=ic0-adapter; denied=identity-model ;;
        ic-platform) allowed=crypto-adapter; denied=identity-model ;;
    esac
    for profile in normal alias optional build dev target target-optional; do
        for package in "$allowed" "$denied"; do
            cp "$owners/$package/base.toml" "$owners/$package/Cargo.toml"
            table=dependencies; optional=''; key=selected
            case "$profile" in
                normal) key="$dependency" ;;
                optional) optional=', optional=true' ;;
                build) table=build-dependencies ;;
                dev) table=dev-dependencies ;;
                target) table="target.'cfg(windows)'.dependencies" ;;
                target-optional) table="target.'cfg(windows)'.dependencies"; optional=', optional=true' ;;
            esac
            printf '\n[%s]\n%s={package="%s",path="../%s"%s}\n' \
                "$table" "$key" "$dependency" "$dependency" "$optional" >> "$owners/$package/Cargo.toml"
            status=0
            PATH="$original_path" bash "$root/scripts/ci/check-cargo-dependency-boundaries.sh" \
                --consumer "$owners" --external-ic > "$FIXTURE/owner-$dependency-$profile-$package.log" 2>&1 || status=$?
            if [[ "$package" == "$allowed" ]]; then
                [[ "$status" == 0 ]] || { cat "$FIXTURE/owner-$dependency-$profile-$package.log" >&2; exit 1; }
            else
                [[ "$status" != 0 ]] || exit 1
                grep -Fx "$package -> $dependency" "$FIXTURE/owner-$dependency-$profile-$package.log" > /dev/null
            fi
            cp "$owners/$package/base.toml" "$owners/$package/Cargo.toml"
        done
    done
done
fixture_complete=true
echo 'Dependency boundaries, failed/malformed observations and actual offline Cargo declarations passed'
