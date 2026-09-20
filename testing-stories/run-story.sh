#!/usr/bin/env bash
# Runs one testing story against one harness in a fresh Docker container: a
# clean $HOME, exactly the named skill(s) installed via the real installer,
# and nothing else. See testing-stories/README.md for what a testing story
# is and how to read the result afterward.
#
# Usage:
#   testing-stories/run-story.sh <skill> <harness> [--model MODEL]
#                                 [--skills "extra-skill another-skill"]
#                                 [--no-build]
#
#   <skill>     name of the story file under testing-stories/stories/
#               (without .md) -- also the default (only) skill installed.
#   <harness>   claude | codex | opencode
#   --skills    override the space-separated list of skills to install, if
#               the story needs more than just <skill> (rare -- most stories
#               exercise exactly one skill).
#   --model     override the harness's default model.
#   --no-build  skip `docker build` and run whatever image is already tagged
#               ai-skills-testing-story-<harness> (faster iteration once the
#               image is known-good).
#   --interactive   codex only. Runs the real interactive codex TUI
#               (approval policy on-request) instead of the batch
#               `codex exec -a never --json` pipeline. `-a never` means
#               exactly what it says -- no approval path exists, so a
#               sandbox-boundary action (writing `.git`, a network bind)
#               fails outright and looks identical to a real hard limit
#               when it may just be an unapproved request. Pipe this
#               through the interactive-shell skill to drive it (answer the
#               initial trust prompt, type/observe the story, approve or
#               decline each request as it comes up) -- plain `docker run
#               -it` with nothing attached to the tty blocks on the first
#               prompt forever.
#
# Requires: docker, and bin/x86_64-unknown-linux-musl/installer already built
# (run ./setup-dev-env.sh first if it is not).
#
# Credentials are never baked into the image -- pass them through the
# environment this script runs under:
#   ANTHROPIC_API_KEY   required for --harness claude
#   OPENAI_API_KEY       codex, if this host authenticates that way
#   CODEX_AUTH_FILE      codex, alternative to OPENAI_API_KEY: a path to a
#                         codex auth.json (default: $HOME/.codex/auth.json)
#                         for a host logged in via ChatGPT account auth
#                         instead of a raw API key. Only auth.json itself is
#                         copied into the container's own fresh $HOME/.codex
#                         -- never the rest of $HOME/.codex (config.toml,
#                         history, mcp_servers, project trust) -- so no
#                         project-specific config or prior session state
#                         leaks into the fresh-context run.
#   OPENCODE_API_KEY / whatever env var the chosen opencode provider needs --
#     forwarded verbatim if set; opencode's own `auth login` config under a
#     mounted config dir is not wired up here (add it if a story needs a
#     provider that only supports interactive auth).

set -euo pipefail

self_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$self_dir/.." && pwd)"

usage() {
    awk 'NR > 1 && /^set -euo pipefail/ { exit } NR > 1 && /^#/ { sub(/^# ?/, ""); print }' "$0" >&2
    exit 64
}

[ "$#" -ge 2 ] || usage

skill="$1"
harness="$2"
shift 2

skills="$skill"
model=""
do_build=1
interactive=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --skills)
            [ "$#" -ge 2 ] || usage
            skills="$2"
            shift 2
            ;;
        --model)
            [ "$#" -ge 2 ] || usage
            model="$2"
            shift 2
            ;;
        --no-build)
            do_build=0
            shift
            ;;
        --interactive)
            interactive=1
            shift
            ;;
        *)
            echo "run-story.sh: unknown argument: $1" >&2
            usage
            ;;
    esac
done

if [ "$interactive" -eq 1 ] && [ "$harness" != codex ]; then
    echo "run-story.sh: --interactive is codex-only for now" >&2
    exit 64
fi

case "$harness" in
    claude|codex|opencode) ;;
    *)
        echo "run-story.sh: harness must be claude, codex, or opencode (got: $harness)" >&2
        exit 64
        ;;
esac

command -v docker >/dev/null 2>&1 || {
    echo "run-story.sh: docker is required" >&2
    exit 69
}

# The installer binary is a release-packaging artifact, not one of the
# ordinary skill-command binaries ./setup-dev-env.sh stages into
# bin/x86_64-unknown-linux-musl/ -- so it is looked for there first (in case
# some other flow already staged it), then in cargo's own release output,
# and built on demand as a last resort. Mounted into the container at a
# fixed path so entrypoint.sh does not need to know which of these it was.
installer_bin="$repo_root/bin/x86_64-unknown-linux-musl/installer"
if [ ! -x "$installer_bin" ]; then
    installer_bin="$repo_root/target/x86_64-unknown-linux-musl/release/installer"
fi
if [ ! -x "$installer_bin" ]; then
    command -v cargo >/dev/null 2>&1 || {
        echo "run-story.sh: no installer binary found and cargo is not on PATH -- run this inside 'nix develop .', or build it yourself: cargo build -p installer --release --target x86_64-unknown-linux-musl" >&2
        exit 69
    }
    echo "run-story.sh: building the installer binary (one-time; cargo build -p installer --release --target x86_64-unknown-linux-musl)" >&2
    (cd "$repo_root" && cargo build -p installer --release --target x86_64-unknown-linux-musl) || {
        echo "run-story.sh: failed to build the installer binary" >&2
        exit 70
    }
fi
[ -x "$installer_bin" ] || {
    echo "run-story.sh: installer binary still not found at $installer_bin after building" >&2
    exit 70
}

story_file="$self_dir/stories/$skill.md"
[ -f "$story_file" ] || {
    echo "run-story.sh: no testing story at $story_file" >&2
    exit 66
}

fixture_file="$self_dir/stories/$skill.fixture.sh"

codex_auth_file=""
case "$harness" in
    claude)
        : "${ANTHROPIC_API_KEY:?ANTHROPIC_API_KEY must be set for the claude harness}"
        ;;
    codex)
        if [ -z "${OPENAI_API_KEY:-}" ]; then
            codex_auth_file="${CODEX_AUTH_FILE:-$HOME/.codex/auth.json}"
            [ -f "$codex_auth_file" ] || {
                echo "run-story.sh: codex needs either OPENAI_API_KEY or an existing auth.json (checked $codex_auth_file; override with CODEX_AUTH_FILE)" >&2
                exit 64
            }
        fi
        ;;
    opencode)
        # opencode's provider credential var varies by chosen model/provider;
        # not asserted here. Set whatever the target provider needs before
        # invoking this script.
        ;;
esac

image="ai-skills-testing-story-$harness"

if [ "$do_build" -eq 1 ]; then
    echo "run-story.sh: building $image" >&2
    docker build -f "$self_dir/docker/Dockerfile.$harness" -t "$image" "$self_dir/docker"
fi

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
run_dir="$self_dir/runs/$skill-$harness-$timestamp"
mkdir -p "$run_dir"

{
    printf 'skill=%s\n' "$skill"
    printf 'skills_installed=%s\n' "$skills"
    printf 'harness=%s\n' "$harness"
    printf 'model=%s\n' "${model:-<harness default>}"
    printf 'started_at=%s\n' "$timestamp"
    printf 'story_file=%s\n' "$story_file"
} > "$run_dir/manifest.txt"

docker_args=(
    run --rm
    -v "$repo_root:/repo:ro"
    -v "$installer_bin:/opt/ai-skills-installer:ro"
    -v "$run_dir:/results"
    -v "$story_file:/story.md:ro"
    -e "SKILLS=$skills"
    -e "AGENT=$harness"
)

# Several skills (planning's verify-skill-load, ci-failures' rjq, and others)
# use the repo's compiled-binary-preference pattern: they exec a real
# compiled helper when one is found, and only fall back to a slower bash
# implementation -- or, for a few (verify-skill-load, rjq) that ship no bash
# fallback at all, refuse outright -- when none exists. A skill install here
# never bundles those binaries (that only happens in a packaged release
# build), and this container has neither `nix` nor `cargo` to build them on
# demand, so without this mount every such skill hits a hard, misleading
# "no compiled binary found" wall that looks like a real capability gap but
# is really just this test harness missing a step. AI_SKILLS_BIN_ROOT is the
# officially-supported override (plan_bin_dir's first and highest-priority
# resolution path -- see planning/scripts/lib/crypt/plan_bin_dir.sh) for
# exactly this: an install stating its own answer rather than being guessed
# at. Mounted read-only and harmless to skills that never look for it.
bin_root="$repo_root/bin/x86_64-unknown-linux-musl"
if [ -d "$bin_root" ]; then
    docker_args+=(-v "$bin_root:/opt/ai-skills-bin:ro" -e "AI_SKILLS_BIN_ROOT=/opt/ai-skills-bin")
fi

[ "$interactive" -eq 1 ] && docker_args+=(-it -e "INTERACTIVE=1")
[ -n "$model" ] && docker_args+=(-e "MODEL=$model")
[ -f "$fixture_file" ] && docker_args+=(-v "$fixture_file:/fixture.sh:ro")

# codex's own command sandbox (bubblewrap) creates a nested Linux user
# namespace and mount namespace for every shell command it runs. Docker's
# default confinement blocks the namespace itself ("bwrap: No permissions to
# create a new namespace"), and even once that's allowed, blocks the mount
# propagation change bwrap needs next ("Failed to make / slave"). Verified
# directly: all three of these are needed together (SYS_ADMIN alone, or
# seccomp=unconfined alone, each get partway and then hit the next wall).
# This is also exactly why entrypoint.sh keeps /workspace internal rather
# than bind-mounted -- a bind mount survives namespace creation but not the
# uid remapping bwrap does inside it.
[ "$harness" = codex ] && docker_args+=(
    --cap-add=SYS_ADMIN
    --security-opt seccomp=unconfined
    --security-opt apparmor=unconfined
)

# Forge tokens, forwarded verbatim when set on the host -- only the
# ci-failures story needs these (gh/glab are installed in every image
# regardless), so this is a harmless no-op for every other story.
[ -n "${GH_TOKEN:-}" ] && docker_args+=(-e "GH_TOKEN=$GH_TOKEN")
[ -n "${GITHUB_TOKEN:-}" ] && docker_args+=(-e "GITHUB_TOKEN=$GITHUB_TOKEN")
[ -n "${GITLAB_TOKEN:-}" ] && docker_args+=(-e "GITLAB_TOKEN=$GITLAB_TOKEN")
[ -n "${GLAB_TOKEN:-}" ] && docker_args+=(-e "GLAB_TOKEN=$GLAB_TOKEN")

case "$harness" in
    claude)
        docker_args+=(-e "ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY")
        ;;
    codex)
        if [ -n "${OPENAI_API_KEY:-}" ]; then
            docker_args+=(-e "OPENAI_API_KEY=$OPENAI_API_KEY")
        else
            docker_args+=(-v "$codex_auth_file:/opt/codex-auth.json:ro")
        fi
        ;;
    opencode)
        [ -n "${OPENCODE_API_KEY:-}" ] && docker_args+=(-e "OPENCODE_API_KEY=$OPENCODE_API_KEY")
        ;;
esac

docker_args+=("$image")

echo "run-story.sh: running skill=$skill harness=$harness -> $run_dir" >&2
run_status=0
docker "${docker_args[@]}" || run_status=$?

if [ "$interactive" -eq 1 ]; then
    echo "run-story.sh: interactive session ended (exit $run_status). No transcript file --" >&2
    echo "run-story.sh: capture what you observed yourself (a screen snapshot, notes) for analysis." >&2
else
    echo "run-story.sh: done (exit $run_status). Transcript: $run_dir/transcript.jsonl" >&2
    echo "run-story.sh: next step -- analyze it against testing-stories/analysis-prompt.md" >&2
fi
exit "$run_status"
