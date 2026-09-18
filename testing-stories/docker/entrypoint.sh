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
#   /workspace    a fresh, empty, read-write bind mount. This becomes the
#                 agent's cwd and the "project" it works in. Also where the
#                 transcript is written, so results survive after the
#                 container exits.
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
[ -d /workspace ] || die "/workspace is not mounted"
[ -f /story.md ] || die "/story.md is not mounted"

installer_bin="/opt/ai-skills-installer"
[ -x "$installer_bin" ] || die "compiled installer not found at $installer_bin -- run-story.sh should have mounted it"

export HOME=/home/agent
mkdir -p "$HOME"

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

transcript=/workspace/transcript.jsonl
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
        : "${OPENAI_API_KEY:?OPENAI_API_KEY must be set to run the codex harness}"
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

echo "testing-story-entrypoint: agent exited $exit_code; transcript at /workspace/transcript.jsonl" >&2
exit "$exit_code"
