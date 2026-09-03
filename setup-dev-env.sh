#!/usr/bin/env bash
# MODE: DEV
# setup-dev-env.sh — build the crates under src/ into a working local tree.
#
# A fresh clone carries Rust source but almost no binaries: only one artifact is
# committed, and the skills look for compiled helpers that are not there. The
# suite still passes, because every one of them degrades honestly — which is
# exactly what hides the fact that the compiled path is never being exercised.
# This builds each crate for THIS machine into one bin/<target triple> at the
# repository root, which is where the skills look, so a local tree runs the same
# code a target does.
#
# It also builds the generated shell artifacts a clean checkout lacks — the five
# plan-*-lib.sh that planning/scripts/*.sh source, and planning/REVIEWER.md — on
# the same build-if-missing terms, so one run leaves a tree that works rather
# than one whose compiled half works.
#
# Usage:
#   setup-dev-env.sh              # build everything for this host
#   setup-dev-env.sh --list       # print what would be built, and where
#   setup-dev-env.sh --check      # report what is present or missing; build nothing
#   setup-dev-env.sh --help
#
# Only the host's target triple is built. Cross-building the other four is what
# a release does (installer/build-release.sh) and what CI does per runner; a
# development tree needs the one it can actually execute.
#
# Exit codes: 64 = bad usage; 69 = nix is missing (see the message it prints);
# 70 = a crate failed to build.

set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mode=build

usage() {
    sed -n '3,23p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --list) mode=list ;;
        --check) mode=check ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
    shift
done

# ── nix is the toolchain, so its absence is fatal rather than degraded ───────
# Every other missing tool in this repository costs a capability and says so.
# This one cannot: the crates are pinned to one compiler version by the flake,
# and a build from some other cargo is not the artifact CI and a release
# produce. Refusing is honest; guessing at a system rust is not.
require_nix() {
    command -v nix >/dev/null 2>&1 && return 0
    cat >&2 <<'NONIX'
setup-dev-env: nix is required and is not installed.

The crates are built by the toolchain flake.nix pins -- the newest stable rust
the locked nixpkgs offers, with the five house targets. Any other cargo produces
a different artifact from the one CI and a release ship, so this script will not
fall back to one.

Install nix, then re-run this script:

  Determinate Nix (recommended; this is what the repository is developed on)
    curl -fsSL https://install.determinate.systems/nix | sh -s -- install

  Upstream multi-user install
    sh <(curl -L https://nixos.org/nix/install) --daemon

Both need a new shell afterwards so the profile script is sourced. Verify with:

  nix --version

If nix is installed but not on PATH, source its profile:

  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh

NONIX
    exit 69
}

# The host's Rust target triple, using the same five-row house list the skills
# resolve against at runtime (rust-development-guidelines.md section 4). A
# machine outside the list has no row to build and is refused by name.
host_triple() {
    local os arch
    os="$(uname -s 2>/dev/null || printf 'unknown')"
    arch="$(uname -m 2>/dev/null || printf 'unknown')"
    case "$os" in
        Linux)
            case "$arch" in
                x86_64|amd64) printf 'x86_64-unknown-linux-musl\n' ;;
                aarch64|arm64) printf 'aarch64-unknown-linux-musl\n' ;;
                *) return 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) printf 'x86_64-apple-darwin\n' ;;
                arm64|aarch64) printf 'aarch64-apple-darwin\n' ;;
                *) return 1 ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            case "$arch" in
                x86_64|amd64) printf 'x86_64-pc-windows-msvc\n' ;;
                *) return 1 ;;
            esac
            ;;
        *) return 1 ;;
    esac
}

# What to build: <crate> <binary>. Everything lands in ONE bin/<triple> at the
# repository root, not in a bin/ inside each skill. Planning commands also get
# a second, untracked copy beside their shell oracle as scripts/<binary>; that
# is the extensionless command layout users invoke after the migration. rjq alone is a hard
# requirement of planning, todo and bug-report, so a per-skill layout means the
# same binary copied three times -- or, as it was, shipped by one skill and
# missing from the other two, which the installer then refuses to install.
#
# chat-proto is a library the two chat crates depend on and produces no binary,
# so it is absent here and built as a dependency of theirs.
plan() {
    plan_primary
    plan_secondary
}

plan_primary() {
    cat <<'PLAN'
add-adversarial-finding	add-adversarial-finding
add-coverage	add-coverage
add-fix-claim	add-fix-claim
add-goal	add-goal
add-planning-bug	add-planning-bug
add-ui-story	add-ui-story
add-ui-story-links	add-ui-story-links
add-work-unit	add-work-unit
chat-client-rs	chat-client-rs
chat-server-rs	chat-server-rs
configure-ui-story-cache	configure-ui-story-cache
create-adversarial-review	create-adversarial-review
create-plan	create-plan
create-plan-progress	create-plan-progress
update-plan-progress	update-plan-progress
rebuild-plan-progress	rebuild-plan-progress
register-read	register-read
register-command	register-command
register-rebuild	register-rebuild
plan-mutate	plan-mutate
todo-add	todo-add
todo-update	todo-update
bug-add	bug-add
bug-update	bug-update
supervision-frame	supervision-frame
generate-reviewer	generate-reviewer
cleanup-plans	cleanup-plans
verify-target	verify-target
update-step	update-step
verify-fix-keys	verify-fix-keys
update-work-unit	update-work-unit
validate-plan	validate-plan
run-adversary-probe	run-adversary-probe
update-plan-content	update-plan-content
monitor-read	monitor-read
create-progress	create-progress
create-step-testing	create-step-testing
create-ui-story-run-cache	create-ui-story-run-cache
create-ui-validation	create-ui-validation
create-work-unit-inventory	create-work-unit-inventory
PLAN
}

plan_secondary() {
    cat <<'PLAN'
mint-fix-keys	mint-fix-keys
plan-crypt	plan-crypt
plan-env	plan-env
plan-content	plan-content
plan-context-wrapper	plan-context-wrapper
plan-context	plan-context
role-context	role-context
plan-overview	plan-overview
plan-overview	overview-state
plan-root	plan-root
remove-coverage	remove-coverage
remove-plan	remove-plan
remove-work-unit	remove-work-unit
resolve-finding	resolve-finding
tony-the-pony	tony-the-pony
update-adversarial-review	update-adversarial-review
update-progress	update-progress
update-ui-story	update-ui-story
rjq	rjq
PLAN
}

triple="$(host_triple)" || {
    printf '%s: no house target covers %s:%s; nothing to build here\n' \
        "${0##*/}" "$(uname -s)" "$(uname -m)" >&2
    exit 69
}
exe=''
case "$triple" in *windows-msvc) exe='.exe' ;; esac

if [ "$mode" = list ] || [ "$mode" = check ]; then
    printf 'host target: %s\n\n' "$triple"
    while IFS="$(printf '\t')" read -r crate binary; do
        [ -n "$crate" ] || continue
        dest="bin/$triple/$binary$exe"
        if [ "$mode" = check ]; then
            state=$([ -x "$repo_root/$dest" ] && echo present || echo MISSING)
            printf '  %-16s -> %-52s %s\n' "$crate" "$dest" "$state"
            if [ -f "$repo_root/planning/scripts/$binary.sh" ]; then
                sibling="planning/scripts/$binary$exe"
                state=$([ -x "$repo_root/$sibling" ] && echo present || echo MISSING)
                printf '  %-16s -> %-52s %s\n' "$crate" "$sibling" "$state"
            fi
        else
            printf '  %-16s -> %s\n' "$crate" "$dest"
            if [ -f "$repo_root/planning/scripts/$binary.sh" ]; then
                printf '  %-16s -> %s\n' "$crate" "planning/scripts/$binary$exe"
            fi
        fi
    done <<EOF
$(plan)
EOF
    exit 0
fi

# One dev shell for the whole run, entered here, rather than one per crate.
#
# Entering it costs about 2.7s and does not vary with what is being built, so
# at 58 crates the old shape spent roughly 157s starting shells before any
# compilation happened. The reason recorded for doing it per crate was that a
# crate which fails to build should report its own name while the rest still
# build — but that is the loop's `if` below, not the shell boundary, and it is
# unchanged here. The only thing a per-crate entry isolated was the dev shell
# itself failing for one crate, which is not a per-crate condition.
#
# --list and --check return above this point, so they still cost no nix at all.
if [ -z "${SETUP_DEV_ENV_IN_NIX:-}" ] && [ -z "${IN_NIX_SHELL:-}" ]; then
    require_nix
    exec nix develop "$repo_root" --command env \
        SETUP_DEV_ENV_IN_NIX=1 "$repo_root/${0##*/}" "$@"
fi

printf 'setup-dev-env: building for %s\n\n' "$triple"
built=0 failed=''
while IFS="$(printf '\t')" read -r crate binary; do
    [ -n "$crate" ] || continue
    src="$repo_root/src/$crate"
    [ -f "$src/Cargo.toml" ] || { printf '  %-16s no crate at src/%s; skipped\n' "$crate" "$crate"; continue; }
    printf '  %-16s ' "$crate"
    # A crate that fails to build reports its own name and the loop carries on
    # to the next one; only the summary at the end is non-zero. The dev shell
    # was entered once before this loop, so cargo is already on PATH.
    if cargo build --release --manifest-path "$src/Cargo.toml" --target "$triple" \
        >"$repo_root/.setup-dev-env.log" 2>&1; then
        dest_dir="$repo_root/bin/$triple"
        mkdir -p "$dest_dir"
        cp "$repo_root/target/$triple/release/$binary$exe" "$dest_dir/$binary$exe"
        chmod +x "$dest_dir/$binary$exe"
        printf 'ok -> bin/%s/%s%s\n' "$triple" "$binary" "$exe"
        if [ -f "$repo_root/planning/scripts/$binary.sh" ]; then
            cp "$repo_root/target/$triple/release/$binary$exe" \
                "$repo_root/planning/scripts/$binary$exe"
            chmod +x "$repo_root/planning/scripts/$binary$exe"
            printf '   -> planning/scripts/%s%s\n' "$binary" "$exe"
        fi
        built=$((built + 1))
    else
        printf 'FAILED\n'
        sed 's/^/      | /' "$repo_root/.setup-dev-env.log" >&2
        failed="$failed $crate"
    fi
done <<EOF
$(plan)
EOF
rm -f "$repo_root/.setup-dev-env.log"

# The generated shell artifacts, on the same build-if-missing terms as the
# crates above. They are never committed (MAINTAINER.md section 2.16), and a
# fresh clone therefore has none of them — which is the same gap this script
# exists to close: planning/scripts/*.sh `source` the five plan-*-lib.sh files,
# so a freshly cloned tree cannot run a planning helper at all until they are
# built. Doing it here means "I ran setup-dev-env.sh" is enough to have a
# working tree, rather than being enough only for the compiled half.
#
# Staleness is deliberately not detected here, exactly as run-tests.sh's
# bootstrap_generated has it: regenerating unconditionally would let this script
# mask drift the tests are there to find. Missing is built, present is left
# alone.
generated=0
for lib in plan-core-lib.sh plan-crypt-lib.sh plan-document-lib.sh plan-progress-lib.sh plan-table-lib.sh; do
    if [ ! -f "$repo_root/planning/scripts/$lib" ]; then
        if "$repo_root/planning/scripts/build-plan-libs.sh" >/dev/null 2>&1; then
            printf 'setup-dev-env: built the generated shell libraries\n'
            generated=$((generated + 1))
        else
            printf 'setup-dev-env: build-plan-libs.sh failed; planning helpers will not load\n' >&2
        fi
        break
    fi
done
if [ ! -f "$repo_root/planning/REVIEWER.md" ]; then
    if "$repo_root/planning/scripts/generate-reviewer.sh" >/dev/null 2>&1; then
        printf 'setup-dev-env: generated planning/REVIEWER.md\n'
        generated=$((generated + 1))
    else
        printf 'setup-dev-env: generate-reviewer.sh failed; the reviewer contract is missing\n' >&2
    fi
fi
[ "$generated" -eq 0 ] && printf 'setup-dev-env: generated artifacts already present\n'

# Wire the repo's pre-push gate (./pre-push-check.sh) as the pre-push hook.
# Git does not version .git/hooks, so the shim lives in hooks/ and
# core.hooksPath points there; every clone that runs this script gets the
# gate. CI runs the same checks server-side regardless.
if git -C "$repo_root" config core.hooksPath hooks; then
    printf 'setup-dev-env: pre-push gate wired (git config core.hooksPath hooks)\n'
else
    printf 'setup-dev-env: could not set core.hooksPath; the pre-push gate is not wired\n' >&2
fi

printf '\n%s binary/binaries built.\n' "$built"
if [ -n "$failed" ]; then
    printf 'failed:%s\n' "$failed" >&2
    exit 70
fi

# plan_bin_dir finds this directory on its own, so nothing needs configuring for
# the skills themselves. The export line is for a human's own shell.
cat <<PATHNOTE

The helpers put this directory on PATH themselves when they load, so rjq and
plan-crypt resolve here with nothing further to do. For an interactive shell
that wants them too:

  export PATH="$repo_root/bin/$triple:\$PATH"
PATHNOTE
