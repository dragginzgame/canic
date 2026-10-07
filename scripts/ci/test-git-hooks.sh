#!/usr/bin/env bash
set -euo pipefail

# Canic-owned fixture: qualify the application hook, including historical symlinks.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT/ci/tool-versions.env"
[[ "$(cargo sort --version)" == "cargo-sort $SHARED_TOOLING_CARGO_SORT_VERSION" ]] || {
    echo "hook tests require prepared cargo-sort $SHARED_TOOLING_CARGO_SORT_VERSION" >&2
    exit 1
}
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/git-hooks-test.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$FIXTURE"; else printf "Failed hook fixture retained: %s\n" "$FIXTURE" >&2; fi' EXIT
# Reuse an existing source commit read-only, without creating fixture commits.
source_commit="$(git -C "$ROOT" rev-parse HEAD)"
source_objects="$(git -C "$ROOT" rev-parse --git-path objects)"
case "$source_objects" in /*) ;; *) source_objects="$ROOT/$source_objects" ;; esac
mkdir "$FIXTURE/templates"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TEMPLATE_DIR="$FIXTURE/templates"

new_fixture() {
    mkdir -p "$FIXTURE/$1"
    cd "$FIXTURE/$1"
    git init --quiet
    mkdir -p .git/objects/info .githooks scripts/dev scripts/ci
    printf '%s\n' "$source_objects" > .git/objects/info/alternates
    git update-ref HEAD "$source_commit"
    git read-tree HEAD
    git checkout-index --all
    cp "$ROOT/.githooks/pre-commit" .githooks/pre-commit
    cp "$ROOT/scripts/dev/install-git-hooks.sh" scripts/dev/install-git-hooks.sh
    cp "$ROOT/scripts/ci/check-make-execution.sh" scripts/ci/check-make-execution.sh
    cat > Makefile <<'MAKE'
.PHONY: fmt
fmt:
	@bash scripts/fixture-fmt.sh
MAKE
    cat > scripts/fixture-fmt.sh <<'FORMAT'
#!/usr/bin/env bash
set -euo pipefail
[[ -z "${GIT_INDEX_FILE:-}" && -z "${GIT_DIR:-}" ]]
for path in *.rs Cargo.toml README.md; do
    [[ -f "$path" ]] || continue
    sed 's/unformatted/formatted/g' "$path" > "$path.fmt"
    cat "$path.fmt" > "$path"
    rm "$path.fmt"
done
case "${FORMAT_TEST_RACE:-}" in
    index)
        printf 'concurrent staged edit\n' > "$FORMAT_TEST_REAL_ROOT/concurrent.txt"
        git -C "$FORMAT_TEST_REAL_ROOT" add concurrent.txt
        ;;
    worktree) printf 'concurrent working edit\n' > "$FORMAT_TEST_REAL_ROOT/staged.rs" ;;
esac
[[ "${FORMAT_TEST_FAIL:-}" != yes ]]
FORMAT
    printf 'unformatted\n' > Cargo.toml
    git add Makefile scripts/fixture-fmt.sh scripts/ci/check-make-execution.sh Cargo.toml
}
expect_failure() {
    if "$@" > output 2>&1; then
        echo 'git hook test unexpectedly accepted a failing fixture' >&2
        exit 1
    fi
}

new_fixture 'fully staged'
printf 'unformatted\n' > 'staged [1].rs'
newline_path=$'staged\nnewline.rs'
printf 'unformatted\n' > "$newline_path"
printf 'unformatted untracked\n' > unselected.rs
printf 'unformatted unrelated working edit\n' > README.md
git add -- 'staged [1].rs' "$newline_path"
bash .githooks/pre-commit > output
[[ "$(git show ':staged [1].rs')" == formatted && "$(git show ":$newline_path")" == formatted ]]
[[ "$(git show :Cargo.toml)" == formatted && "$(cat README.md)" == 'unformatted unrelated working edit' ]]
[[ "$(git show :README.md)" == "$(git show HEAD:README.md)" ]]
[[ "$(cat unselected.rs)" == 'unformatted untracked' && -z "$(git ls-files -- unselected.rs)" ]]
tree="$(git write-tree)"
bash .githooks/pre-commit > output
[[ "$(git write-tree)" == "$tree" ]]

new_fixture no-selection
git read-tree HEAD
tree="$(git write-tree)"
bash .githooks/pre-commit > output
[[ "$(git write-tree)" == "$tree" && "$(cat Cargo.toml)" == unformatted ]]

new_fixture partial
printf 'unformatted selected\n' > partial.rs
git add partial.rs
printf 'unformatted unstaged\n' >> partial.rs
tree="$(git write-tree)"
cp partial.rs before
expect_failure bash .githooks/pre-commit
[[ "$(git write-tree)" == "$tree" && "$(cat Cargo.toml)" == unformatted ]]
cmp before partial.rs

new_fixture formatter-failure
printf 'unformatted\n' > staged.rs
git add staged.rs
tree="$(git write-tree)"
FORMAT_TEST_FAIL=yes expect_failure bash .githooks/pre-commit
[[ "$(git write-tree)" == "$tree" && "$(cat staged.rs)" == unformatted && "$(cat Cargo.toml)" == unformatted ]]

for race in index worktree; do
    new_fixture "concurrent-$race"
    printf 'unformatted\n' > staged.rs
    git add staged.rs
    tree="$(git write-tree)"
    FORMAT_TEST_RACE="$race" FORMAT_TEST_REAL_ROOT="$PWD" expect_failure bash .githooks/pre-commit
    [[ "$(git show :staged.rs)" == unformatted ]]
    case "$race" in
        index) [[ "$(git show :concurrent.txt)" == 'concurrent staged edit' && "$(cat staged.rs)" == unformatted ]] ;;
        worktree) [[ "$(git write-tree)" == "$tree" && "$(cat staged.rs)" == 'concurrent working edit' ]] ;;
    esac
done

for flags in i n q t v; do
    new_fixture "make-mode-$flags"
    printf 'unformatted\n' > staged.rs
    git add staged.rs
    tree="$(git write-tree)"
    expect_failure env MAKEFLAGS="$flags" bash .githooks/pre-commit
    [[ "$(git write-tree)" == "$tree" && "$(cat staged.rs)" == unformatted ]]
done

new_fixture alternate-index
printf 'unformatted\n' > staged.rs
git add staged.rs
real_tree="$(git write-tree)"
cp .git/index "$FIXTURE/alternate.index"
GIT_DIR="$PWD/.git" GIT_INDEX_FILE="$FIXTURE/alternate.index" bash .githooks/pre-commit > output
[[ "$(git write-tree)" == "$real_tree" && "$(git show :staged.rs)" == unformatted ]]
[[ "$(GIT_INDEX_FILE="$FIXTURE/alternate.index" git show :staged.rs)" == formatted && "$(cat staged.rs)" == formatted ]]

new_fixture symlink
printf 'unformatted\n' > outside.rs
ln -s outside.rs linked.rs
git add linked.rs
tree="$(git write-tree)"
expect_failure bash .githooks/pre-commit
[[ "$(git write-tree)" == "$tree" && "$(cat outside.rs)" == unformatted ]]

new_fixture deleted
git rm --quiet README.md
bash .githooks/pre-commit > output
[[ ! -e README.md && -z "$(git ls-files -- README.md)" ]]

new_fixture installer
bash scripts/dev/install-git-hooks.sh > output
[[ "$(git config --local --get core.hooksPath)" == .githooks ]]
bash scripts/dev/install-git-hooks.sh > output

# Logical aliases such as macOS /var versus /private/var name the same root.
new_fixture installer-path-alias
ln -s "$PWD" "$FIXTURE/installer-alias"
(
    cd "$FIXTURE/installer-alias"
    bash scripts/dev/install-git-hooks.sh > output
)
[[ "$(git config --local --get core.hooksPath)" == .githooks ]]

new_fixture installer-configured
git config --local core.hooksPath custom-hooks
expect_failure bash scripts/dev/install-git-hooks.sh
[[ "$(git config --get core.hooksPath)" == custom-hooks ]]

new_fixture installer-disabled
git config --local core.hooksPath ''
expect_failure bash scripts/dev/install-git-hooks.sh
[[ "$(git config --local --get core.hooksPath)" == '' ]]

new_fixture installer-inherited
printf '[core]\n\thooksPath = inherited-hooks\n' > "$FIXTURE/global-config"
GIT_CONFIG_GLOBAL="$FIXTURE/global-config" expect_failure bash scripts/dev/install-git-hooks.sh
[[ -z "$(git config --local --get core.hooksPath || true)" ]]

new_fixture installer-private-hook
mkdir .git/hooks
printf '#!/bin/sh\nexit 0\n' > .git/hooks/pre-push
chmod +x .git/hooks/pre-push
cp .git/hooks/pre-push before
expect_failure bash scripts/dev/install-git-hooks.sh
cmp before .git/hooks/pre-push
[[ -z "$(git config --get core.hooksPath || true)" ]]

new_fixture installer-non-executable
chmod -x .githooks/pre-commit
expect_failure bash scripts/dev/install-git-hooks.sh
[[ ! -x .githooks/pre-commit && -z "$(git config --get core.hooksPath || true)" ]]

# Exercise real Cargo/rustfmt on both a root and a standalone nested workspace.
# No dependencies, builds, Git commits or network access are needed.
for lockfiles in inherited absent present; do
    new_fixture "cargo-$lockfiles"
    cat > Makefile <<'MAKE'
.PHONY: fmt
fmt:
	cargo sort --workspace
	cargo sort --workspace testing
	cargo fmt --all
	cargo fmt --manifest-path testing/Cargo.toml --all
MAKE
    cat > Cargo.toml <<'CARGO'
[workspace]
[package]
name = "hook-root-fixture"
version = "0.0.0"
edition = "2021"
CARGO
    mkdir -p src testing/src
    cp Cargo.toml testing/Cargo.toml
    printf 'pub fn fixture( ){}\n' > src/lib.rs
    printf 'pub fn fixture( ){}\n' > testing/src/lib.rs
    case "$lockfiles" in
        absent) rm -f Cargo.lock testing/Cargo.lock ;;
        present)
            for workspace in . testing; do
                CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 cargo generate-lockfile --offline --manifest-path "$workspace/Cargo.toml" > output 2>&1
            done
            ;;
    esac
    for workspace in . testing; do
        if [[ -f "$workspace/Cargo.lock" ]]; then cp "$workspace/Cargo.lock" "$workspace/selected-lock"; fi
    done
    git add Makefile Cargo.toml src/lib.rs testing/Cargo.toml testing/src/lib.rs
    CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 bash .githooks/pre-commit > output
    [[ "$(git show :src/lib.rs)" == 'pub fn fixture() {}' && "$(git show :testing/src/lib.rs)" == 'pub fn fixture() {}' ]]
    for workspace in . testing; do
        if [[ -f "$workspace/selected-lock" ]]; then
            cmp "$workspace/selected-lock" "$workspace/Cargo.lock"
        else
            [[ ! -e "$workspace/Cargo.lock" ]]
        fi
    done
    [[ ! -e target && ! -e testing/target ]]
done

# Sorting must cover inherited member tables and separate workspace catalogs,
# preserving the effective metadata and already selected lockfiles.
new_fixture cargo-sort
cat > Makefile <<'MAKE'
.PHONY: fmt fmt-check
fmt:
	cargo sort --workspace
	cargo sort --workspace testing
	cargo fmt --all
	cargo fmt --manifest-path testing/Cargo.toml --all
fmt-check:
	cargo sort --workspace --check
	cargo sort --workspace --check testing
	cargo fmt --all -- --check
	cargo fmt --manifest-path testing/Cargo.toml --all -- --check
MAKE
for workspace in . testing; do
    mkdir -p "$workspace/src" "$workspace/alpha/src" "$workspace/zeta/src" "$workspace/consumer/src"
    cat > "$workspace/Cargo.toml" <<'CARGO'
[workspace]
members = ["zeta", "consumer", "alpha"]
resolver = "2"

[workspace.package]
version = "0.0.0"
edition = "2021"

[workspace.dependencies]
# Preserve the selected source and feature policy.
zeta = { path = "zeta", default-features = false }
alpha = { path = "alpha" }

[package]
name = "sort-root-fixture"
version.workspace = true
edition.workspace = true

[dependencies]
zeta = { workspace = true, features = ["extra"] }
alpha.workspace = true
CARGO
    for member in alpha zeta; do
        printf '[package]\nname = "%s"\nversion.workspace = true\nedition.workspace = true\n\n[features]\nextra = []\n' "$member" > "$workspace/$member/Cargo.toml"
        printf 'pub fn fixture() {}\n' > "$workspace/$member/src/lib.rs"
    done
    cat > "$workspace/consumer/Cargo.toml" <<'CARGO'
[package]
name = "sort-consumer-fixture"
version.workspace = true
edition.workspace = true

[dependencies]
zeta.workspace = true
alpha.workspace = true

[dev-dependencies]
zeta.workspace = true
alpha.workspace = true

[build-dependencies]
zeta.workspace = true
alpha.workspace = true

[target.'cfg(unix)'.dependencies]
zeta.workspace = true
alpha.workspace = true
CARGO
    printf 'pub fn fixture() {}\n' > "$workspace/src/lib.rs"
    printf 'pub fn fixture() {}\n' > "$workspace/consumer/src/lib.rs"
    CARGO_NET_OFFLINE=true cargo generate-lockfile --offline --manifest-path "$workspace/Cargo.toml" > output 2>&1
    cp "$workspace/Cargo.lock" "$workspace/selected-lock"
    CARGO_NET_OFFLINE=true cargo metadata --offline --locked --no-deps --format-version 1 --manifest-path "$workspace/Cargo.toml" |
        jq -S '[.packages[] | .dependencies |= sort_by(.name, .kind, .target)] | sort_by(.name)' > "$workspace/selected-metadata"
done
git add Makefile Cargo.toml src alpha zeta consumer testing
tree="$(git write-tree)"
expect_failure make --no-print-directory fmt-check
[[ "$(git write-tree)" == "$tree" ]]
git --literal-pathspecs diff --quiet -- Cargo.toml consumer/Cargo.toml testing/Cargo.toml testing/consumer/Cargo.toml
CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 bash .githooks/pre-commit > output
make --no-print-directory fmt-check > output
for workspace in . testing; do
    cmp "$workspace/selected-lock" "$workspace/Cargo.lock"
    CARGO_NET_OFFLINE=true cargo metadata --offline --locked --no-deps --format-version 1 --manifest-path "$workspace/Cargo.toml" |
        jq -S '[.packages[] | .dependencies |= sort_by(.name, .kind, .target)] | sort_by(.name)' > "$workspace/sorted-metadata"
    cmp "$workspace/selected-metadata" "$workspace/sorted-metadata"
    grep -q '# Preserve the selected source and feature policy.' "$workspace/Cargo.toml"
    awk '$0 == "[workspace.dependencies]" { catalog=1; next } /^\[/ { catalog=0 } catalog && /^(alpha|zeta) / { sub(/ .*/, ""); print }' "$workspace/Cargo.toml" > observed-order
    printf 'alpha\nzeta\n' > expected-order
    cmp expected-order observed-order
done
tree="$(git write-tree)"
CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 bash .githooks/pre-commit > output
[[ "$(git write-tree)" == "$tree" && ! -e target && ! -e testing/target ]]
echo 'Git hook preservation, installation, Cargo formatting and manifest sorting tests passed'
