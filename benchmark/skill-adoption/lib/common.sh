#!/usr/bin/env bash
# Shared helpers for run-cell.sh. Sourced, never executed directly.
#
# Dev tooling (benchmark/**) -- python3 is fine here per CODE-STYLE.md's own
# carve-out; kept to bash + python3 + standard git/coreutils throughout.

# wa_work_base: prints the disk-backed base directory cell workspaces are
# created under. Never /tmp: on this harness's dev host (and most Linux
# desktops) /tmp is a tmpfs ramdisk, and a full concurrent pass holds many
# complete `claude`/`opencode` HOME trees (skill installs, caches, journals)
# at once -- that is RAM, not disk, and a crashed cell's leftovers never get
# reclaimed until the system is rebooted. WA_WORK_BASE overrides.
wa_work_base() {
    printf '%s\n' "${WA_WORK_BASE:-$HOME/.cache/ai-skills-benchmark/skill-adoption}"
}

# wa_new_workspace: creates WORK/project and WORK/home under wa_work_base,
# prints WORK. Each call gets its own mktemp -d, so concurrent cells never
# share a workspace.
wa_new_workspace() {
    local base work
    base="$(wa_work_base)"
    mkdir -p "$base" || return 1
    work="$(mktemp -d "$base/cell.XXXXXXXXXX")" || return 1
    mkdir -p "$work/project" "$work/home"
    printf '%s\n' "$work"
}

# wa_reap_stale_workspaces: removes every leftover cell.* workspace under
# wa_work_base. Safe only when nothing is using that directory yet (e.g. at
# the very start of a fresh full-pass.sh run, before any cell is launched) --
# run-cell.sh's own `trap cleanup EXIT` already removes a cell's workspace on
# any normal exit, including a caught signal; this sweep exists only for the
# cells it cannot cover (SIGKILL, an OOM kill, a crash before the trap runs)
# or for debug litter left over from driving run-cell.sh by hand.
wa_reap_stale_workspaces() {
    local base
    base="$(wa_work_base)"
    [ -d "$base" ] || return 0
    find "$base" -mindepth 1 -maxdepth 1 -name 'cell.*' -print0 2>/dev/null \
        | while IFS= read -r -d '' d; do
            echo "wa_reap_stale_workspaces: removing stale workspace $d" >&2
            rm -rf "$d"
        done
}

# wa_capture_environment REPO_ROOT CLI AGENT_HOME: prints the `environment`
# object (schema/run-record.schema.json) as one JSON line.
wa_capture_environment() {
    local repo_root="$1" cli="$2" home="$3"
    local commit dirty cli_version is_version ishell_version os host
    commit="$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo unknown)"
    if [ -n "$(git -C "$repo_root" status --porcelain 2>/dev/null)" ]; then
        dirty=true
    else
        dirty=false
    fi
    cli_version="$(HOME="$home" "$cli" --version 2>/dev/null | head -n1 || echo unknown)"
    ishell_version="$(HOME="$home" interactive-shell --version 2>/dev/null | head -n1 || echo unknown)"
    os="$(uname -a 2>/dev/null || echo unknown)"
    host="$(hostname 2>/dev/null || echo unknown)"
    python3 -c '
import json, sys
print(json.dumps({
    "ai_skills_commit": sys.argv[1],
    "worktree_dirty": sys.argv[2] == "true",
    "cli_version": sys.argv[3],
    "interactive_shell_version": sys.argv[4],
    "subject_model": "unknown",
    "experimenter_model": "haiku",
    "os": sys.argv[5],
    "host": sys.argv[6],
}))
' "$commit" "$dirty" "$cli_version" "$ishell_version" "$os" "$host"
}

# wa_install_skills REPO_ROOT AGENT_HOME CLI_AGENT_NAME MODE: installs the
# whole skill collection into a disposable HOME, with the four dual-surface
# skills set to MODE (skill|mcp) -- see PROTOCOL.md step 1. Installer
# invocation is UNVERIFIED end-to-end as of this writing; see PROTOCOL.md's
# "Known limitations and unverified assumptions".
wa_install_skills() {
    local repo_root="$1" agent_home="$2" cli_agent="$3" mode="$4"
    local installer_bin
    installer_bin="$(wa_installer_bin "$repo_root")" || return 1
    HOME="$agent_home" "$installer_bin" install \
        --source "$repo_root" --package dev \
        --agent "$cli_agent" --all --yes \
        --integration "decisions=$mode" --integration "chat=$mode" \
        --integration "tailpipe=$mode" --integration "ai-text-editor=$mode" \
        --integration "interactive-shell=skill"
}

# wa_dev_bin_dir REPO_ROOT NAME: prints the directory holding a compiled
# dev-tree binary called NAME (checking target/release/ then bin/<triple>/,
# same search order as wa_installer_bin), or fails with nothing printed if
# neither has it -- never builds it (unlike wa_installer_bin; NAME is not
# necessarily something bootstrap.sh or a cargo build -p here knows how to
# produce, e.g. interactive-shell comes from setup-dev-env.sh's own build).
# run-cell.sh prepends this onto the experimenter's PATH, mirroring
# AGENTS.md's own `export AI_SKILLS_BIN_ROOT=$PWD/bin/<triple>` convention
# for this exact class of tool (a dev-tree binary with no system install) --
# the experimenter runs under the orchestrator's real HOME, never the
# disposable per-cell one, so it cannot reach a skill's shared bin dir the
# way the subject does (see make-experimenter-brief.py's own PATH prepend)
# and needs this instead for the one binary it genuinely owns itself:
# interactive-shell, the wrapper it drives the subject through.
wa_dev_bin_dir() {
    local repo_root="$1" name="$2"
    local candidate
    for candidate in "$repo_root/target/release/$name" "$(find "$repo_root/bin" -mindepth 2 -maxdepth 2 -name "$name" 2>/dev/null | head -n1)"; do
        if [ -n "$candidate" ] && [ -x "$candidate" ]; then
            dirname "$candidate"
            return 0
        fi
    done
    return 1
}

# wa_installer_bin REPO_ROOT: prints the path to a built installer binary,
# building it (cargo build --release) if it is not already there.
wa_installer_bin() {
    local repo_root="$1"
    local candidate
    for candidate in "$repo_root/target/release/installer" "$(find "$repo_root/bin" -mindepth 2 -maxdepth 2 -name installer 2>/dev/null | head -n1)"; do
        if [ -n "$candidate" ] && [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    echo "common.sh: building installer (cargo build --release -p installer)..." >&2
    (cd "$repo_root" && cargo build --release -p installer) >&2 || return 1
    printf '%s\n' "$repo_root/target/release/installer"
}

# wa_shared_bin AGENT_HOME: prints where the installer put shared binaries
# for this disposable HOME.
wa_shared_bin() {
    local agent_home="$1"
    printf '%s/.config/tsch-ai-skills/bin\n' "$agent_home"
}

# wa_seed_agent_auth CLI AGENT_HOME: makes a disposable subject HOME already
# authenticated, so the subject reaches the task prompt instead of blocking
# on first-run onboarding/login. Mirrors LAB-PROTOCOL.md's own subject, which
# never swaps HOME at all and so stays authenticated by construction -- this
# harness swaps HOME for skill-install isolation, so it must restore auth
# explicitly. Symlinks the real credential file in (never copies or reads
# its bytes here) and, for claude, merges a few non-secret onboarding-skip
# keys from the real ~/.claude.json into the disposable one -- read and
# written entirely inside the python3 subprocess below, never printed to
# this script's own stdout/stderr.
#
# Must run BEFORE wa_install_skills (prepare-cell.sh's own call order), not
# just before the subject launches: `permissions::claude_*_permissions` in
# the installer (src/installer/src/permissions.rs) refuse to write anything
# -- PermissionOutcome::NoConfigFile -- unless `.claude/settings.json`
# already exists, which it does on a real machine (Claude Code itself
# creates it on first run, before a skill is ever installed) but never does
# in a freshly-made disposable HOME. Without this seed the install step
# silently grants the subject none of the Read/Edit/Bash allowlist entries
# a real install produces for planning/worktrees/interactive-shell/
# project-specifics, so the subject faces path-write prompts a real,
# already-onboarded user never would -- not a faithful recreation of what
# gets installed. Seeding an empty `{}` settings.json is enough to make that
# merge fire (`PermissionOutcome::NoConfigFile` is a file-existence gate, not
# a content gate): this does NOT copy the real operator's own accumulated
# `permissions.allow`, which would tie a benchmark subject's baseline to one
# person's unrelated, ever-changing real-machine grants and could mask the
# exact install-time prompt friction a scenario is measuring (including this
# install's own `mcp__<skill>__*` pre-approval, which the real operator may
# already have from unrelated live use).
# wa_seed_claude_auth AGENT_HOME REAL_HOME: the claude-only half of
# wa_seed_agent_auth below, split out to stay under CODE-STYLE.md's 40-line
# function cap. Symlinks the real credential file, merges a few non-secret
# onboarding-skip keys, and seeds an empty settings.json so the installer's
# own permission merge fires -- see wa_seed_agent_auth's own doc comment for
# why each step is there.
wa_seed_claude_auth() {
    local agent_home="$1" real_home="$2"
    if [ -f "$real_home/.claude/.credentials.json" ]; then
        mkdir -p "$agent_home/.claude"
        ln -sf "$real_home/.claude/.credentials.json" "$agent_home/.claude/.credentials.json"
    fi
    if [ -f "$real_home/.claude.json" ]; then
        python3 -c "
import json
real = json.load(open('$real_home/.claude.json'))
disposable_path = '$agent_home/.claude.json'
try:
    disposable = json.load(open(disposable_path))
except FileNotFoundError:
    disposable = {}
for key in ('hasCompletedOnboarding', 'lastOnboardingVersion', 'oauthAccount',
            'hasCompletedClaudeInChromeOnboarding', 'theme'):
    if key in real:
        disposable[key] = real[key]
json.dump(disposable, open(disposable_path, 'w'))
"
    fi
    mkdir -p "$agent_home/.claude"
    python3 -c "
import json
disposable_path = '$agent_home/.claude/settings.json'
try:
    json.load(open(disposable_path))
except (FileNotFoundError, json.JSONDecodeError):
    json.dump({}, open(disposable_path, 'w'))
"
}

wa_seed_agent_auth() {
    local cli="$1" agent_home="$2" real_home="${HOME:-}"
    [ -n "$real_home" ] || { echo "common.sh: \$HOME is unset, cannot seed auth" >&2; return 1; }
    case "$cli" in
    claude)
        wa_seed_claude_auth "$agent_home" "$real_home"
        ;;
    opencode)
        if [ -f "$real_home/.local/share/opencode/auth.json" ]; then
            mkdir -p "$agent_home/.local/share/opencode"
            ln -sf "$real_home/.local/share/opencode/auth.json" "$agent_home/.local/share/opencode/auth.json"
        fi
        ;;
    *)
        echo "common.sh: unknown cli '$cli'" >&2
        return 64
        ;;
    esac
}

# wa_path_without TARGET_DIR NAME...: builds a symlink farm at TARGET_DIR
# covering every binary reachable on the current $PATH except NAME, and
# prints TARGET_DIR (the caller's responsibility to clean up -- pass a path
# under the cell's own $WORK so run-cell.sh's cleanup trap sweeps it).
#
# Used by a scenario whose premise requires a tool's genuine absence (S09's
# jq, so the subject has no choice but rjq) -- HOME swapping alone does not
# touch PATH. An earlier version removed whole PATH directories containing
# NAME instead; measured live, that also removed bash and env themselves
# (co-located with jq in /usr/bin on this host), making the stripped PATH
# unusable for anything, not just the forbidden binary. A symlink farm
# removes exactly NAME and nothing else.
wa_path_without() {
    local target="$1"
    shift
    local names=("$@")
    mkdir -p "$target"
    local dir f base skip name
    local IFS=:
    read -ra _wa_path_dirs <<< "$PATH"
    for dir in "${_wa_path_dirs[@]}"; do
        [ -d "$dir" ] || continue
        for f in "$dir"/*; do
            [ -e "$f" ] || continue
            base="$(basename "$f")"
            skip=false
            for name in "${names[@]}"; do
                if [ "$base" = "$name" ]; then
                    skip=true
                    break
                fi
            done
            [ "$skip" = true ] && continue
            [ -e "$target/$base" ] && continue
            ln -s "$f" "$target/$base" 2>/dev/null || true
        done
    done
    printf '%s\n' "$target"
}

# wa_subject_argv CLI SUBJECT_UUID AGENT_HOME: prints (one per line) the
# argv to launch the subject with, env-stripping applied per PROTOCOL.md.
# claude's stripping is the confirmed fix from LAB-PROTOCOL.md; opencode's
# is this protocol's unverified hypothesis by analogy -- see PROTOCOL.md's
# "Known limitations".
wa_subject_argv() {
    local cli="$1" uuid="$2" home="$3"
    case "$cli" in
    claude)
        cat <<EOF
env
-u
CLAUDE_CODE_CHILD_SESSION
-u
CLAUDE_CODE_SESSION_ID
-u
CLAUDE_CODE_SESSION_ATTENDED
-u
CLAUDE_CODE_BRIDGE_SESSION_ID
-u
CLAUDE_CODE_MESSAGING_SOCKET
-u
CLAUDE_CODE_MESSAGING_TOKEN
HOME=$home
claude
--session-id
$uuid
--permission-mode
acceptEdits
EOF
        ;;
    opencode)
        cat <<EOF
env
HOME=$home
opencode
EOF
        ;;
    *)
        echo "common.sh: unknown cli '$cli'" >&2
        return 64
        ;;
    esac
}
