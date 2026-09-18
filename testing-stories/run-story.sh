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
#
# Requires: docker, and bin/x86_64-unknown-linux-musl/installer already built
# (run ./setup-dev-env.sh first if it is not).
#
# Credentials are never baked into the image -- pass them through the
# environment this script runs under:
#   ANTHROPIC_API_KEY   required for --harness claude
#   OPENAI_API_KEY       required for --harness codex
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
        *)
            echo "run-story.sh: unknown argument: $1" >&2
            usage
            ;;
    esac
done

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

case "$harness" in
    claude)
        : "${ANTHROPIC_API_KEY:?ANTHROPIC_API_KEY must be set for the claude harness}"
        ;;
    codex)
        : "${OPENAI_API_KEY:?OPENAI_API_KEY must be set for the codex harness}"
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
    -v "$run_dir:/workspace"
    -v "$story_file:/story.md:ro"
    -e "SKILLS=$skills"
    -e "AGENT=$harness"
)

[ -n "$model" ] && docker_args+=(-e "MODEL=$model")
[ -f "$fixture_file" ] && docker_args+=(-v "$fixture_file:/fixture.sh:ro")

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
        docker_args+=(-e "OPENAI_API_KEY=$OPENAI_API_KEY")
        ;;
    opencode)
        [ -n "${OPENCODE_API_KEY:-}" ] && docker_args+=(-e "OPENCODE_API_KEY=$OPENCODE_API_KEY")
        ;;
esac

docker_args+=("$image")

echo "run-story.sh: running skill=$skill harness=$harness -> $run_dir" >&2
run_status=0
docker "${docker_args[@]}" || run_status=$?

echo "run-story.sh: done (exit $run_status). Transcript: $run_dir/transcript.jsonl" >&2
echo "run-story.sh: next step -- analyze it against testing-stories/analysis-prompt.md" >&2
exit "$run_status"
