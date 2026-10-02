#!/usr/bin/env bash
# Shared entrypoint for every testing-story image (claude/codex/opencode).
# Baked identically into all three; the only per-image difference is which
# CLI got installed and the AGENT env var each Dockerfile sets.
#
# Contract with run-story.sh (the host-side caller):
#   /repo                       a read-only bind mount of the ai-skills
#                               checkout, used as the installer's --source.
#   /opt/ai-skills-installer    the compiled installer binary, read-only.
#                               This image does not compile anything itself.
#   /results      a fresh, empty, read-write bind mount for OUTPUT only:
#                 transcript.jsonl and (after the agent exits) a copy of the
#                 final /workspace tree, so results survive after the
#                 container exits. Never the agent's own cwd -- see below.
#   /story.md     the testing-story prompt, read-only, mounted by the runner.
#   /fixture.sh   optional, read-only. If present, run (as a bash subprocess,
#                 cwd /workspace) BEFORE the agent starts, to seed pre-existing
#                 state
#                 (a git repo, a sample file to edit, ...). Most stories need
#                 none of this -- the point of a testing story is that the
#                 agent builds what it needs using the skill itself.
#   $SKILLS       space-separated skill names to install (usually one).
#   $AGENT        claude | codex | opencode. Set by the Dockerfile; a runner
#                 override is accepted but must match the image it is in.
#   $MODEL        optional model override; each harness has its own default.
#   /opt/codex-auth.json   optional, read-only. codex only, and only when
#                 $OPENAI_API_KEY is unset: a ChatGPT-account auth.json,
#                 copied (never mounted read-write) into this container's own
#                 fresh $HOME/.codex/ so codex can authenticate the way a
#                 host logged in via `codex login` does, without an API key.
#   $INTERACTIVE  codex only. When set, execs the real interactive `codex`
#                 TUI (approval policy on-request) instead of the batch
#                 `codex exec -a never --json` pipeline, and skips the
#                 transcript/final-workspace capture below entirely -- the
#                 caller drives the TUI directly (docker run -it, piped
#                 through the interactive-shell skill) and reads its own
#                 screen state. `-a never` (used otherwise) means exactly
#                 what it says: no approval path exists at all, so any
#                 sandbox-boundary action (writing `.git`, a network bind)
#                 fails outright and is indistinguishable in the transcript
#                 from a genuine hard limit. Verified directly: the *same*
#                 git commit that `-a never exec` refuses ("Read-only file
#                 system") succeeds under `-a on-request` once approved
#                 interactively -- codex can write `.git` exactly like any
#                 other model; `-a never` was hiding that, not exposing a
#                 real constraint.
#
# /workspace is deliberately a plain directory INSIDE the container, not a
# bind mount: codex's own command sandbox (bubblewrap) creates a nested Linux
# user namespace and remaps uids for everything under its target directory,
# which a host bind mount cannot follow -- verified directly (writing into a
# bind-mounted /workspace under bwrap's sandbox failed with "Permission
# denied" / "Can't mkdir", even once the namespace itself could be created).
# An internal directory has no such conflict. Its content does not survive
# the container on its own, so it is copied to /results/final-workspace/
# after the agent exits -- after codex's own sandboxed subprocess has
# finished, from this unsandboxed entrypoint, which can read it freely.
#
# What this buys over running the same CLI on the host: $HOME starts
# completely empty. No CLAUDE.md, no prior skill installs, no MCP config, no
# project-specifics notes, no shell history the agent could stumble onto. The
# ONLY guidance available to the agent is whatever the freshly-installed
# skill's own SKILL.md (and any docs/MCP tools it registers) provides, plus
# the story prompt itself -- which is the entire point of a testing story.

set -euo pipefail

die() {
    echo "testing-story-entrypoint: $*" >&2
    exit 1
}

: "${AGENT:?AGENT must be set (claude|codex|opencode) -- set by the image}"
: "${SKILLS:?SKILLS must be set to one or more space-separated skill names}"

[ -d /repo ] || die "/repo is not mounted"
[ -d /results ] || die "/results is not mounted"
[ -f /story.md ] || die "/story.md is not mounted"

installer_bin="/opt/ai-skills-installer"
[ -x "$installer_bin" ] || die "compiled installer not found at $installer_bin -- run-story.sh should have mounted it"

export HOME=/home/agent
mkdir -p "$HOME" /workspace

echo "testing-story-entrypoint: installing skill(s) [$SKILLS] for agent=$AGENT" >&2
skill_args=()
for skill in $SKILLS; do
    skill_args+=(--skill "$skill")
done
"$installer_bin" install --agent "$AGENT" --source /repo --yes "${skill_args[@]}" \
    || die "installer install failed"

if [ -f /fixture.sh ]; then
    echo "testing-story-entrypoint: running fixture setup" >&2
    (cd /workspace && bash /fixture.sh) || die "fixture setup failed"
fi

cd /workspace

transcript=/results/transcript.jsonl
: > "$transcript"

case "$AGENT" in
    claude)
        model="${MODEL:-${CLAUDE_MODEL:-opus}}"
        : "${ANTHROPIC_API_KEY:?ANTHROPIC_API_KEY must be set to run the claude harness}"
        set -- claude -p --output-format=json --permission-mode acceptEdits \
            --model "$model" "$(cat /story.md)"
        ;;
    codex)
        model="${MODEL:-${CODEX_MODEL:-gpt-5.5}}"
        if [ -z "${OPENAI_API_KEY:-}" ]; then
            [ -f /opt/codex-auth.json ] || die "codex needs OPENAI_API_KEY or a mounted /opt/codex-auth.json"
            mkdir -p "$HOME/.codex"
            # Copied, not symlinked/mounted directly: codex may rewrite this
            # file (token refresh), and that must never touch the host's own
            # real auth.json through the read-only bind mount's source.
            cp /opt/codex-auth.json "$HOME/.codex/auth.json"
        fi
        if [ -n "${INTERACTIVE:-}" ]; then
            echo "testing-story-entrypoint: exec'ing interactive codex TUI (approval: on-request, model=$model)" >&2
            exec codex -a on-request -C /workspace --sandbox workspace-write --model "$model" "$(cat /story.md)"
        fi
        set -- codex -a never exec --model "$model" --json \
            -C /workspace --skip-git-repo-check --sandbox workspace-write \
            "$(cat /story.md)"
        ;;
    opencode)
        model="${MODEL:-${OPENCODE_MODEL:-opencode/big-pickle}}"
        set -- opencode run --format json --dir /workspace --model "$model" --auto \
            "$(cat /story.md)"
        ;;
    *)
        die "unknown AGENT: $AGENT (expected claude, codex, or opencode)"
        ;;
esac

echo "testing-story-entrypoint: running: $*" >&2
# Deliberately no extra system prompt, no capsule, no allowlisted commands --
# the story prompt above is the entire task the agent receives. stdout+stderr
# both land in the transcript (matching the runtime drivers' own convention
# that agent_session_id/agent_telemetry parse a merged JSONL+stderr stream),
# and are also mirrored live to this container's own stdout for `docker run`
# without `-d` to watch.
"$@" 2>&1 | tee "$transcript"
exit_code="${PIPESTATUS[0]}"

# Copied AFTER the agent exits, from this unsandboxed entrypoint -- codex's
# own subprocess sandbox has already released /workspace by this point, so
# no nested-namespace uid mapping is in play here, just a plain recursive
# copy. Opened up afterward: this container runs as root, and /results is a
# host bind mount with no uid remapping, so anything left root-owned needs
# root (or another container) on the host side just to delete it later --
# not worth it for a throwaway results directory.
rm -rf /results/final-workspace
cp -r /workspace /results/final-workspace
chmod -R a+rwX /results

echo "testing-story-entrypoint: agent exited $exit_code; transcript at /results/transcript.jsonl, final workspace at /results/final-workspace" >&2
exit "$exit_code"
