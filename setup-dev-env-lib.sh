#!/usr/bin/env bash
# MODE: DEV
# setup-dev-env-lib.sh — the crate/build-target plan and host-triple
# resolution setup-dev-env.sh sources; CODE-STYLE §3 (400-line script cap).
#
# Sourced by setup-dev-env.sh only. Every function here reads repo_root from
# the caller rather than resolving its own copy.

# shellcheck disable=SC2154
# repo_root is set by setup-dev-env.sh before this is sourced; shellcheck
# lints each file alone and cannot see that assignment.
set -euo pipefail
export LC_ALL=C

# B164: the root Cargo.toml is a virtual workspace globbing members = ["src/*"],
# so cargo treats every directory under src/ as a member and a missing
# Cargo.toml is fatal for the WHOLE workspace before a single crate builds --
# even one built by manifest path alone, since cargo still resolves the
# workspace first. A directory holding only gitignored content (a stale
# target/ left from a deleted crate) passes every git check while being
# unbuildable, so name it here rather than let cargo's "failed to read
# .../Cargo.toml" -- true but silent about the real cause -- be the only word.
check_stray_src_dirs() {
    local dir stray=()
    for dir in "$repo_root"/src/*/; do
        [ -d "$dir" ] || continue
        [ -f "${dir}Cargo.toml" ] || stray+=("${dir%/}")
    done
    [ "${#stray[@]}" -eq 0 ] && return 0
    printf 'setup-dev-env: src/ has %d director%s with no Cargo.toml, which breaks the whole workspace build:\n' \
        "${#stray[@]}" "$([ "${#stray[@]}" -eq 1 ] && echo y || echo ies)" >&2
    for dir in "${stray[@]}"; do printf '  %s\n' "${dir#"$repo_root"/}" >&2; done
    printf 'Remove it (a leftover from a deleted or renamed crate) and re-run:\n' >&2
    for dir in "${stray[@]}"; do printf '  rm -rf %q\n' "${dir#"$repo_root"/}" >&2; done
    exit 70
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
# repository root. Planning commands also get a second, untracked copy beside
# their shell oracle as scripts/<binary>; that is the extensionless command
# layout users invoke after the migration. rjq alone was a hard requirement of
# planning, todo and bug-report, so a per-skill layout means the same binary
# copied three times -- or, as it was, shipped by one skill and missing from the
# other two, which the installer then refuses to install. todo and bug-report no
# longer declare rjq at all: their own binaries replaced every rjq call, so only
# planning still needs it.
#
# The register skills are the exception, and not by preference: skill_files()
# promises bin/<triple>/<binary> RELATIVE TO THE SKILL, so the installer looks
# in bug-report/bin/<triple>/ and todo/bin/<triple>/. Those get a per-skill copy
# as well, matching CI's "Place the compiled register rungs" step. T72 replaces
# both paths with one shared bin and this exception goes with it.
#
# chat-proto is a library the two chat crates depend on and produces no binary,
# so it is absent here and built as a dependency of theirs.
plan() {
    plan_primary
    plan_secondary
    cat <<'PLAN'
ai-text-editor	ai-text-editor
ai-text-editor	ai-text-editor-server
ai-text-editor-mcp	ai-text-editor-mcp
interactive-shell	interactive-shell
interactive-shell	interactive-shell-input
PLAN
}

plan_primary_add_and_infra() {
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
chat-mcp	chat-mcp
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
PLAN
}

plan_primary_update_and_verify() {
    cat <<'PLAN'
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

plan_primary() {
    plan_primary_add_and_infra
    plan_primary_update_and_verify
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
bug-report	bugs
todo	todo
PLAN
}
