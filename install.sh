#!/usr/bin/env bash
# MODE: PROD
# GENERATED FILE — do not edit. Assembled from installer/src/*.sh by:
#   installer/build.sh
set -euo pipefail

# Interactive installer for the skills in this repository.
# It is intentionally self-contained so it can be used as:
#   curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh | bash
#
# Staying one file is a hard constraint, not an oversight: the curl|bash form has
# no siblings to source, and download_source() distinguishes a local checkout
# from a download purely by whether ${BASH_SOURCE[0]} is a readable file. So the
# sections below take the place of separate files. Table of contents, in order:
#
#   1.  Configuration and registries
#   2.  CLI-mode argument parsing
#   3.  Interactive input channel
#   4.  Runtime tool verification
#   5.  Agent target detection
#   6.  Terminal capability, splash, and menu rendering
#   6b. Skill picker: state, text metrics, requirement model
#   6c. Skill picker: the sprite and the frame
#   6d. Skill picker: input, the terminal, and the seam
#   7.  Skill and target selection
#   8.  Source acquisition
#   9.  Per-skill file manifest
#   10. CLI-mode handlers
#   11. Interactive install, backup, and merge
#   12. Post-install plan root migration
#   13. Step 2: planning runtime permissions (interactive main path only)
#   14. Main
#
# Usage:
#   install.sh [--all | --skill <name> ...] [--target <path>] [--yes]
#   install.sh --help
#
# Exit codes: 0 success, 1 any error, plus the machine contract of the
# --install-skill CLI mode: 2 = approval declined, 3 = unsafe collision.

# ---------------------------------------------------------------
# 1. Configuration and registries
# ---------------------------------------------------------------
# SKILL_NAMES/SKILL_DESCRIPTIONS and TARGET_NAMES/TARGET_PATHS/TARGET_KINDS are
# index-parallel: usage(), the shop menu, the numeric menu map, and
# agent_kind_for_root() all derive from them, so a new skill or agent is one
# edit here. TARGET_* index order is load-bearing: agent_target_available()
# switches on the index, and index 5 (Cline) owns six alternative detection
# globs. Never reorder or insert; append only.

REPO_URL="${AI_SKILLS_REPO_URL:-https://github.com/tschallacka/ai-skills}"
# The repository's default branch is master; a "main" default 404s on both the
# raw install.sh URL and the archive tarball, which breaks the curl|bash form.
REPO_REF="${AI_SKILLS_REF:-master}"
SOURCE_ROOT=""
TEMP_ROOT=""
SOURCE_VERSION=""
YES=0
SKILL_SELECTION=""
TARGET_SELECTION=""
CLI_MODE=""
CLI_SKILL=""
CLI_FORMAT=""
CLI_RELATIVE=""
CLI_APPROVAL=""

# Filled by verify_runtime_tools() (section 4) and read by main and the summary:
# the installable subset of the selection, and the space-joined names that a
# missing hard requirement removed from it.
RUNTIME_READY_SKILLS=()
RUNTIME_BLOCKED_SKILLS=""
# One formatted line per destination, appended by install_skill() (section 11)
# and printed as the end-of-run summary (section 11b).
SUMMARY_LINES=()
SUMMARY_PRINTED=0

PACKAGE_SELECTION="${PACKAGE_SELECTION:-prod}"

SKILL_NAMES=(planning project-specificies resource-limited-testing brainstorm post-implementation-review todo bug-report chat git-worktrees merge-request-etiquette)
SKILL_DESCRIPTIONS=(
    'Durable, resumable plans with steps and verification.'
    'Records project conventions, quirks, and deviations.'
    'Caps CPU and memory for demanding tool runs.'
    'Shapes an idea into a recorded, agreed picture before planning.'
    'After-the-fact review and proposed fixes for built code.'
    'A nested queue of work in one JSON file, read with jq.'
    'Defects with their reproduction, mechanism and verification, in JSON.'
    'IRC-basis agent chat: channels, deltas, live tails; runtime falls back.'
    'Separate checkouts so parallel work and long verifications cannot collide.'
    'Merge request descriptions in your voice: a one-paragraph TLDR, then the fix.'
)

# The detail pane's body: a summary sentence, then what it actually does. Kept
# here rather than read from each skill's SKILL.md because select_skills() runs
# before download_source() -- under `curl … | bash` no skill directory exists
# yet, so anything the picker shows must be baked in at build time.
#
# Index-parallel with SKILL_NAMES, which test-installer-skill-selection.sh
# enforces: git-worktrees was added to SKILL_NAMES without a description, and
# moving the cursor onto it killed the picker with an unbound variable.
SKILL_DETAILS=(
    'Turns an initiative into a directory of files another agent can resume without reconstructing context.
Goals own one outcome each and hold 2-10 work units; a work unit is one file and one symbol, so every change is separately reviewable.
Steps carry instructions, acceptance criteria and the handoff the next unit relies on.
Validation refuses a plan whose units nothing verifies, and a fresh adversarial reviewer must approve it before execution.
Where a plan touches a UI, browser stories with real interaction are mandatory rather than optional.'
    'Records what a project does differently, so the next session does not rediscover it.
Load the matching note when a convention, environment quirk or tooling deviation could change how you implement, debug or test.
Newly confirmed deviations are written back, which is what keeps the note worth reading.
Not for documentation that follows the ecosystem defaults -- only the surprises.'
    'Runs a command under a CPU and memory cap, so a heavy test, build or analyzer cannot take the machine with it.
Wraps the command rather than the tool: no configuration inside the thing being limited.
On Apple Silicon macOS it uses memlimit, the one documented exception to this repository dependency ceiling, because macOS offers no other way to cap memory.
Without memlimit it says so and still limits CPU through nice and cpulimit, rather than pretending the cap is in force.'
    'A short recorded back-and-forth that settles what to build before anything is planned or written.
Use it when a request could branch in materially different directions and the wrong branch is expensive.
The output is an agreed picture, not a plan: it feeds the planning skill rather than replacing it.
Skip it for small or fully specified changes, where the discussion costs more than the work.'
    'Reviews an implementation after the fact, from three angles that disagree on purpose.
The implementer analyses its own work, an independent agent proposes alternatives, and a critical agent attacks both.
It ends in concrete proposed fixes rather than a verdict.
Not a substitute for the pre-implementation adversarial review, which asks a different question: this one sees the code that exists.'
    'A nested queue of work in one JSON file, read and written with jq.
For work that outlives the conversation and must survive a restart, a handoff or a compaction.
Every task carries its status and its detail, so a cold reader knows what was intended and what is left.
Not for the steps of a task already in progress, and not for defects -- those belong in the bug register.'
    'Defects recorded with the reproduction, the observed output, the mechanism once established, and the verification that closes them.
One defect per entry: if stating it needs the word "and", it is two entries that reference each other.
An entry closes only with a fix and a verification naming the mutation that fails without it, so a closure cannot be a claim.
Written only through its helpers -- an out-of-enum value makes the register unsound and every later write refuses.'
    'An IRC-basis message bus for agents: channels they register, join and leave, with deltas since an id and live tails.
Two agents in different sessions or on different machines exchange messages without either knowing about the other.
The log is the source of truth and the server is optional: local mode appends under a lock and needs no runtime at all.
Reading takes a cursor rather than the whole channel, so a long conversation does not flood a context window.'
    'Gives a task its own checkout, so several agents can work at once without seeing each other half-finished edits.
Also isolates a long verification from later commits, and keeps a risky change off the main checkout.
Covers the concurrency hazard that silently fails a long-running command when two runs share a tree.
And the part that is usually learned late: the order to merge the branches back in, and which conflict classes to expect.'
    'Writes the merge request description in the voice of the person whose name is on it, opening with a one-paragraph TLDR.
The body names the defect, the cause and the change, and stops there: no headings for their own sake, no restating the diff.
The reasoning is derived from git log for the branch, so a description never explains what a commit message should have said.
Chat transcripts never travel, and a collapsible block is allowed only for evidence the commits genuinely cannot carry.'
)
TARGET_NAMES=(
    "Universal Agent Skills"
    "Codex"
    "Claude Code"
    "OpenCode"
    "OpenClaw"
    "Cline"
)
TARGET_PATHS=(
    "$HOME/.agents/skills"
    "$HOME/.codex/skills"
    "$HOME/.claude/skills"
    "$HOME/.config/opencode/skills"
    "$HOME/.openclaw/skills"
    "$HOME/.cline/skills"
)
TARGET_KINDS=(
    universal
    codex
    claude
    opencode
    openclaw
    cline
)
CUSTOM_LOCATIONS_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/custom-locations"

# Detected once by detect_color_mode(); see section 6.
COLOR_MODE=""

# Mascot pixel art: 16 rows of 16 six-hex-digit pixels, one row per line, each
# pixel doubled horizontally by render_art() so the sprite comes out square.
# Rows 8 and 9 are the eyes and are substituted per frame by render_art(). This
# lives here rather than inside show_splash() because render_art() reads it as an
# implicit global, which used to make render_art() unusable before show_splash().
ART=(
    'f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fdfd5e fcfd5f fcf347 fcf347'
    'f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fbfb5d fdfd5e fcf347 fcf347'
    'e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fddc51 fddc51 fdbb37 fdbb37'
    'e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fcdc51 fcdc51 fdbb37 fdbb37'
    'c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18'
    'c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18'
    'd8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18'
    'd8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18'
    'c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 dd8100 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
    'c27f18 c27f18 6c3100 6c3100 c27f18 c27f18 67522d 67522d 67522d 67522d 883300 883300 c27f18 c27f18 6c3100 6c3100'
    '623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000'
    '623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000'
    '280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000'
    '280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000'
    '300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00'
    '300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00'
)

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Interactive by default. Options are useful for automation:
  --all                    Install or update all skills
  --skill <name>           Install or update a skill; repeatable, and the value
                           may be a comma-separated list
  --package prod|dev       prod (default) installs what an end user needs;
                           dev adds the files only a maintainer does
  --target <path>          Install into one skill root without prompting
  --yes                    Answer yes to every prompt, including the planning
                           permission grants; an edited file is still backed up  
  --help                   Show this help

Interactive prompts accept a for "yes to all" (auto-accepts every
remaining confirmation, e.g. replace/backup prompts and permission grants).

Supported skills:
EOF
    printf '  %s\n' "${SKILL_NAMES[@]}"
}

die() {
    echo "Error: $*" >&2
    exit 1
}

# ---------------------------------------------------------------
# 2. CLI-mode argument parsing
# ---------------------------------------------------------------
# The machine-facing modes are consumed FIRST and then `set --` away, because
# the generic flag loop below dies on anything it does not know. Moving this
# case after that loop makes --print-skill-files an "Unknown option".
# `trap cleanup EXIT` must be installed before the first mktemp -d.
case "${1:-}" in
    --print-skill-files)
        [ "$#" -eq 3 ] || die "--print-skill-files needs skill and --format"
        CLI_MODE="print"
        CLI_SKILL="$2"
        [ "$3" = "--format=tsv" ] || die "--print-skill-files requires --format=tsv"
        CLI_FORMAT="tsv"
        set --
        ;;
    --resolve-source)
        [ "$#" -eq 3 ] || die "--resolve-source needs skill and relative path"
        CLI_MODE="resolve"
        CLI_SKILL="$2"
        CLI_RELATIVE="$3"
        set --
        ;;
    --install-skill)
        [ "$#" -eq 6 ] || die "--install-skill needs skill, --target, and --approval"
        CLI_MODE="install"
        CLI_SKILL="$2"
        [ "$3" = "--target" ] || die "--install-skill requires --target"
        TARGET_SELECTION="$4"
        [ "$5" = "--approval" ] || die "--install-skill requires --approval"
        CLI_APPROVAL="$6"
        set --
        ;;
esac

cleanup() {
    # The summary is printed here as well as at the end of main so a run that
    # dies part-way still reports what it wrote; print_install_summary is
    # idempotent, so the normal path prints it exactly once.
    if [ -z "$CLI_MODE" ] \
        && { [ "${#SUMMARY_LINES[@]}" -gt 0 ] || [ -n "$RUNTIME_BLOCKED_SKILLS" ]; }; then
        print_install_summary
    fi
    if [ -n "$TEMP_ROOT" ] && [ -d "$TEMP_ROOT" ]; then
        rm -rf "$TEMP_ROOT"
    fi
}
trap cleanup EXIT

while [ "$#" -gt 0 ]; do
    case "$1" in
        --all)
            SKILL_SELECTION="all"
            shift
            ;;
        --skill)
            [ "$#" -ge 2 ] || die "--skill needs a skill name"
            # Accumulates. `--skill a --skill b` is the form people reach for
            # first, and it used to keep only the last one, installing something
            # other than what was asked with no warning. A comma-joined value is
            # what select_skills already splits, so both spellings meet there.
            if [ -n "$SKILL_SELECTION" ]; then
                SKILL_SELECTION="$SKILL_SELECTION,$2"
            else
                SKILL_SELECTION="$2"
            fi
            shift 2
            ;;
        --package)
            [ "$#" -ge 2 ] || die "--package needs prod or dev"
            case "$2" in
                prod|dev) PACKAGE_SELECTION="$2" ;;
                *) die "--package must be prod or dev, not $2" ;;
            esac
            shift 2
            ;;
        --package=*)
            case "${1#--package=}" in
                prod|dev) PACKAGE_SELECTION="${1#--package=}" ;;
                *) die "--package must be prod or dev, not ${1#--package=}" ;;
            esac
            shift
            ;;
        --target)
            [ "$#" -ge 2 ] || die "--target needs a directory"
            TARGET_SELECTION="$2"
            shift 2
            ;;
        --yes)
            YES=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "Unknown option: $1 (use --help for usage)"
            ;;
    esac
done

# ---------------------------------------------------------------
# 3. Interactive input channel
# ---------------------------------------------------------------
# fd 3 is the one place prompts read from, so it is opened here, before anything
# can call ask/confirm and before show_splash's `[ -t 3 ]` guard. Order-critical:
# keep this block above every consumer.
#
# `ps -p $$ -o tty=` is not a usable tty probe: Linux prints `?` for no tty but
# macOS prints `??`, so a string compare against `?` passed on macOS, /dev/tty
# always exists as a node, and `exec 3</dev/tty` then failed with ENXIO — killing
# the installer under `set -e` instead of falling through to the closed-fd case.
# Just try the open and let it fail quietly. The probe runs in a subshell so a
# failed `exec` redirection cannot take the installer down with it — bash exits a
# non-interactive shell on a redirection error to a special builtin.
if [ -t 0 ]; then
    exec 3<&0
elif ( exec 3</dev/tty ) 2>/dev/null; then
    exec 3</dev/tty
else
    exec 3<&-
fi

ask() {
    local prompt="$1"
    printf '%s' "$prompt" >&2
    # 2>/dev/null: fd 3 is deliberately closed when there is no tty, and bash
    # prints its own "invalid file descriptor" before this can die, so the real
    # message arrived buried under shell noise.
    IFS= read -r -u 3 REPLY 2>/dev/null || die "Interactive input is required"
}

confirm() {
    local prompt="$1"
    # --yes answers every prompt, not only the replacement ones. It is the flag
    # for a headless run, and a headless run that stops on a question is the thing
    # it exists to prevent -- installing planning under --yes died on the plans
    # directory prompt. YES_ALL is the same answer reached interactively with "a".
    if [ "${YES:-0}" -eq 1 ] || [ "${YES_ALL:-0}" -eq 1 ]; then
        return 0
    fi
    ask "$prompt [y/N/a] "
    case "$REPLY" in
        y|Y|yes|YES) return 0 ;;
        a|A|all|ALL)
            YES_ALL=1
            return 0
            ;;
        *) return 1 ;;
    esac
}

contains() {
    local wanted="$1"
    shift
    local item
    for item in "$@"; do
        [ "$item" = "$wanted" ] && return 0
    done
    return 1
}

# ---------------------------------------------------------------
# 4. Runtime tool verification
# ---------------------------------------------------------------
# Some skills need tools on the target system at runtime (not at install time).
# Requirement strength decides what a missing one costs, and it is per skill so
# one unsatisfiable dependency never stops the others:
#   hard — the skill does not work at all, so it is not installed and the run
#          exits non-zero (a partial install must not read as success in CI)
#   soft — the skill degrades honestly, so it is installed with a warning
#          naming the capability that is lost; the exit status is unaffected
#
# A requirement may be platform- and arch-conditional: resource-limited-testing
# only names memlimit on Darwin:arm64, because memlimit does not support Intel
# Macs and demanding it there would refuse a skill whose degraded path works.
#
# A row may also name a GROUP (fifth requires.tsv column): the requirement is
# met when any member resolves, so a chat server's python3->node->perl->socat
# chain warns once when none is present instead of once per missing tool. A
# hard group gates exactly like a hard single tool; strength is per group.
#
# The four tables below are GENERATED by installer/build.sh from each skill's
# requires.tsv and the shared installer/tools.tsv. Edit those files, not this
# block, and re-run installer/build.sh.
# BEGIN GENERATED DEPENDENCY BLOCK
runtime_requirements() {
    local platform
    platform="$(uname -s):$(uname -m)"
    case "$1" in
        brainstorm)
            ;;
        bug-report)
            case "$platform" in *:*) printf '%s\n' jq ;; esac
            ;;
        chat)
            case "$platform" in *:*) printf '%s\n' bash ;; esac
            case "$platform" in *:*) printf '%s\n' @server-runtimes ;; esac
            ;;
        git-worktrees)
            ;;
        merge-request-etiquette)
            case "$platform" in *:*) printf '%s\n' git ;; esac
            ;;
        planning)
            case "$platform" in *:*) printf '%s\n' bash ;; esac
            case "$platform" in *:*) printf '%s\n' jq ;; esac
            case "$platform" in *:*) printf '%s\n' openssl ;; esac
            case "$platform" in *:*) printf '%s\n' @overview-server-runtimes ;; esac
            ;;
        post-implementation-review)
            ;;
        project-specificies)
            ;;
        resource-limited-testing)
            case "$platform" in *:*) printf '%s\n' bash ;; esac
            case "$platform" in Darwin:arm64) printf '%s\n' memlimit ;; esac
            ;;
        todo)
            case "$platform" in *:*) printf '%s\n' jq ;; esac
            ;;
    esac
}

runtime_requirement_strength() {
    local platform
    platform="$(uname -s):$(uname -m)"
    case "$1:$2" in
        bug-report:jq) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
        chat:bash) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
        chat:@server-runtimes) case "$platform" in *:*) printf '%s\n' 'soft' ;; esac ;;
        merge-request-etiquette:git) case "$platform" in *:*) printf '%s\n' 'soft' ;; esac ;;
        planning:bash) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
        planning:jq) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
        planning:openssl) case "$platform" in *:*) printf '%s\n' 'soft' ;; esac ;;
        planning:@overview-server-runtimes) case "$platform" in *:*) printf '%s\n' 'soft' ;; esac ;;
        resource-limited-testing:bash) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
        resource-limited-testing:memlimit) case "$platform" in Darwin:arm64) printf '%s\n' 'soft' ;; esac ;;
        todo:jq) case "$platform" in *:*) printf '%s\n' 'hard' ;; esac ;;
    esac
}

runtime_requirement_why() {
    local platform
    platform="$(uname -s):$(uname -m)"
    case "$1:$2" in
        bug-report:jq) case "$platform" in *:*) printf '%s\n' 'reads and writes BUGS.json; every command in this skill is a jq call, and a register that cannot be read is worse than none' ;; esac ;;
        chat:bash) case "$platform" in *:*) printf '%s\n' 'the server, register and send/read/tail helpers are bash scripts, so without bash none of them run, and the socat rung needs it for the handler' ;; esac ;;
        chat:@server-runtimes) case "$platform" in *:*) printf '%s\n' 'a chat server cannot open a listening socket without one of these runtimes; the server refuses with exit 69 while send/read/tail keep working' ;; esac ;;
        merge-request-etiquette:git) case "$platform" in *:*) printf '%s\n' 'the description is derived from git log for the branch; without git the guidance still reads but its commands cannot run' ;; esac ;;
        planning:bash) case "$platform" in *:*) printf '%s\n' 'every helper this skill ships is a bash script, so without bash none of them run; the guidance in SKILL.md still reads fine' ;; esac ;;
        planning:jq) case "$platform" in *:*) printf '%s\n' 'reads the placeholder and state-change registries and edits agent permission config; validate-plan.sh refuses to run without it' ;; esac ;;
        planning:openssl) case "$platform" in *:*) printf '%s\n' 'derives and verifies fix keys (HMAC-SHA256) for the adversarial-review gate; planning works without it, mint-fix-keys.sh and verify-fix-keys.sh refuse with exit 69' ;; esac ;;
        planning:@overview-server-runtimes) case "$platform" in *:*) printf '%s\n' 'overview-serve.sh cannot open a listening socket without one of these runtimes; serve mode refuses with exit 69 while render-plan-overview.sh still writes the overview to a file' ;; esac ;;
        resource-limited-testing:bash) case "$platform" in *:*) printf '%s\n' 'the wrapper that applies the resource cap is a bash script, so without bash there is nothing to run the capped command' ;; esac ;;
        resource-limited-testing:memlimit) case "$platform" in Darwin:arm64) printf '%s\n' 'enforces the RAM cap on Apple Silicon macOS; without it limited-run.sh caps CPU only' ;; esac ;;
        todo:jq) case "$platform" in *:*) printf '%s\n' 'reads and writes TODO.json; every command in this skill is a jq call, and a queue that cannot be read is worse than no queue' ;; esac ;;
    esac
}

runtime_requirement_members() {
    local platform
    platform="$(uname -s):$(uname -m)"
    case "$1" in
        @server-runtimes) case "$platform" in *:*) printf '%s\n' python3 node perl socat ;; esac ;;
        @overview-server-runtimes) case "$platform" in *:*) printf '%s\n' python3 node perl socat ;; esac ;;
    esac
}

runtime_tool_verify() {
    case "$1" in
        bash) command -v bash >/dev/null 2>&1 ;;
        jq) command -v jq >/dev/null 2>&1 ;;
        memlimit) command -v memlimit >/dev/null 2>&1 ;;
        openssl) command -v openssl >/dev/null 2>&1 ;;
        python3) command -v python3 >/dev/null 2>&1 ;;
        node) command -v node >/dev/null 2>&1 ;;
        perl) command -v perl >/dev/null 2>&1 ;;
        socat) command -v socat >/dev/null 2>&1 ;;
        *) command -v "$1" >/dev/null 2>&1 ;;
    esac
}

runtime_tool_install_hint() {
    local tool="$1" platform
    platform="$(uname -s):$(uname -m)"
    case "$tool" in
        bash)
            case "$platform" in
                Darwin:*)
                    printf '%s\n' '  bash ships with macOS (3.2 at /bin/bash); nothing to install'
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y bash'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y bash'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm bash'
                    fi
                    ;;
                MINGW*|MSYS*|CYGWIN*:*)
                    printf '%s\n' '  install Git for Windows (https://git-scm.com/download/win); its Git Bash provides bash'
                    ;;
                *:*)
                    printf '%s\n' '  install bash from your package manager, or Git for Windows on Windows'
                    ;;
            esac
            ;;
        python3)
            case "$platform" in
                Darwin:*)
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  brew install python3'
                    elif command -v port >/dev/null 2>&1; then
                        printf '%s\n' '  sudo port install python3'
                    else
                        printf '%s\n' '  install Homebrew (https://brew.sh) then: brew install python3'
                    fi
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y python3'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y python3'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm python'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add python3'
                    else
                        printf '%s\n' '  install the python3 package for your distribution'
                    fi
                    ;;
            esac
            ;;
        node)
            case "$platform" in
                Darwin:*)
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  brew install node'
                    elif command -v port >/dev/null 2>&1; then
                        printf '%s\n' '  sudo port install nodejs'
                    else
                        printf '%s\n' '  install Homebrew (https://brew.sh) then: brew install node'
                    fi
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y nodejs'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y nodejs'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm nodejs'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add nodejs'
                    else
                        printf '%s\n' '  install the nodejs package for your distribution'
                    fi
                    ;;
            esac
            ;;
        perl)
            case "$platform" in
                Darwin:*)
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  brew install perl'
                    elif command -v port >/dev/null 2>&1; then
                        printf '%s\n' '  sudo port install perl'
                    else
                        printf '%s\n' '  install Homebrew (https://brew.sh) then: brew install perl'
                    fi
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y perl'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y perl'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm perl'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add perl'
                    else
                        printf '%s\n' '  install the perl package for your distribution'
                    fi
                    ;;
            esac
            ;;
        socat)
            case "$platform" in
                Darwin:*)
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  brew install socat'
                    elif command -v port >/dev/null 2>&1; then
                        printf '%s\n' '  sudo port install socat'
                    else
                        printf '%s\n' '  install Homebrew (https://brew.sh) then: brew install socat'
                    fi
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y socat'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y socat'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm socat'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add socat'
                    else
                        printf '%s\n' '  install the socat package for your distribution'
                    fi
                    ;;
            esac
            ;;
        jq)
            case "$platform" in
                Darwin:*)
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  brew install jq'
                    elif command -v port >/dev/null 2>&1; then
                        printf '%s\n' '  sudo port install jq'
                    else
                        printf '%s\n' '  install Homebrew (https://brew.sh) then: brew install jq'
                    fi
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y jq'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y jq'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm jq'
                    elif command -v zypper >/dev/null 2>&1; then
                        printf '%s\n' '  sudo zypper install -y jq'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add jq'
                    elif command -v snap >/dev/null 2>&1; then
                        printf '%s\n' '  sudo snap install jq'
                    else
                        printf '%s\n' '  download the static jq binary from https://github.com/jqlang/jq/releases'
                    fi
                    ;;
                MINGW*:*|MSYS*:*|CYGWIN*:*)
                    if command -v winget >/dev/null 2>&1; then
                        printf '%s\n' '  winget install jqlang.jq'
                    elif command -v choco >/dev/null 2>&1; then
                        printf '%s\n' '  choco install jq'
                    elif command -v scoop >/dev/null 2>&1; then
                        printf '%s\n' '  scoop install jq'
                    else
                        printf '%s\n' '  download the static jq binary from https://github.com/jqlang/jq/releases'
                    fi
                    ;;
                *:*)
                    printf '%s\n' '  download the static jq binary from https://github.com/jqlang/jq/releases'
                    ;;
            esac
            ;;
        memlimit)
            case "$platform" in
                *:*)
                    printf '%s\n' '  curl -LsSf https://github.com/pingiun/memlimit/releases/latest/download/memlimit-installer.sh | sh'
                    if command -v brew >/dev/null 2>&1; then
                        printf '%s\n' '  or: brew tap pingiun/memlimit https://github.com/pingiun/memlimit && brew install --HEAD memlimit'
                    elif command -v cargo >/dev/null 2>&1; then
                        printf '%s\n' '  or: cargo install --git https://github.com/pingiun/memlimit memlimit'
                    fi
                    printf '%s\n' '  (memlimit is MIT-licensed, by Jelle Besseling; Apple Silicon only)'
                    ;;
            esac
            ;;
        openssl)
            case "$platform" in
                Darwin:*)
                    printf '%s\n' '  macOS ships /usr/bin/openssl (LibreSSL); if it is missing: brew install openssl'
                    ;;
                Linux:*)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apt-get install -y openssl'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '%s\n' '  sudo dnf install -y openssl'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '%s\n' '  sudo pacman -S --noconfirm openssl'
                    elif command -v zypper >/dev/null 2>&1; then
                        printf '%s\n' '  sudo zypper install -y openssl'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '%s\n' '  sudo apk add openssl'
                    else
                        printf '%s\n' '  install the openssl package for your distribution'
                    fi
                    ;;
                MINGW*:*|MSYS*:*|CYGWIN*:*)
                    if command -v winget >/dev/null 2>&1; then
                        printf '%s\n' '  winget install ShiningLight.OpenSSL.Light'
                    elif command -v choco >/dev/null 2>&1; then
                        printf '%s\n' '  choco install openssl'
                    fi
                    ;;
            esac
            ;;
        *)
            printf '  install %s via your system package manager\n' "$tool"
            ;;
    esac
}
# END GENERATED DEPENDENCY BLOCK

# A skill is installable when every hard requirement is met. A missing soft one
# is deliberately not consulted here — it costs a warning, not the install.
skill_runtime_tools_present() {
    local tool
    while IFS= read -r tool; do
        [ -n "$tool" ] || continue
        [ "$(runtime_requirement_strength "$1" "$tool")" = hard ] || continue
        runtime_requirement_met "$tool" || return 1
    done < <(runtime_requirements "$1")
    return 0
}

# The unmet requirements of one skill at one strength, one entry per line: a
# tool id, or an @group standing for its any-of members.
runtime_unmet_tools() {
    local skill="$1" strength="$2" tool
    while IFS= read -r tool; do
        [ -n "$tool" ] || continue
        [ "$(runtime_requirement_strength "$skill" "$tool")" = "$strength" ] || continue
        runtime_requirement_met "$tool" || printf '%s\n' "$tool"
    done < <(runtime_requirements "$skill")
}

# How a report names a requirement: an @group reads as its member list, a
# single tool as itself.
runtime_requirement_label() {
    local entry="$1" member label=""
    case $entry in
        @*)
            while IFS= read -r member; do
                [ -n "$member" ] || continue
                label="${label:+$label, }$member"
            done < <(runtime_requirement_members "$entry")
            printf 'any of %s' "$label"
            ;;
        *) printf '%s' "$entry" ;;
    esac
}

# Whether one requirement entry is satisfied. A group needs any one member; the
# first success wins, because the chain falls through in preference order.
runtime_requirement_met() {
    local entry="$1" member
    case $entry in
        @*)
            while IFS= read -r member; do
                [ -n "$member" ] || continue
                if runtime_tool_verify "$member"; then
                    return 0
                fi
            done < <(runtime_requirement_members "$entry")
            return 1
            ;;
        *) runtime_tool_verify "$entry" ;;
    esac
}

# Install guidance for one requirement entry: a missing group recommends its
# first member, which is the head of the chain and so the preferred install.
runtime_requirement_install_hint() {
    local entry="$1" first=""
    case $entry in
        @*)
            first="$(runtime_requirement_members "$entry" | sed -n '1p')"
            runtime_tool_install_hint "$first"
            ;;
        *) runtime_tool_install_hint "$1" ;;
    esac
}

runtime_report_missing() {
    local skill="$1" entry="$2"
    printf '  - %s (%s requirement of %s)\n' \
        "$(runtime_requirement_label "$entry")" \
        "$(runtime_requirement_strength "$skill" "$entry")" "$skill" >&2
    printf '    lost without it: %s\n' "$(runtime_requirement_why "$skill" "$entry")" >&2
    if [ "${entry#@}" != "$entry" ]; then
        printf '    install one of them with:\n' >&2
    else
        printf '    install it with:\n' >&2
    fi
    runtime_requirement_install_hint "$entry" >&2
}

# Ready-to-paste lines for the skills that are installable now, so a blocked
# --all is never a dead end. The end-of-run summary carries the full replay.
runtime_report_way_forward() {
    local skill ready=""
    for skill in "$@"; do
        skill_runtime_tools_present "$skill" && ready="$ready $skill"
    done
    [ -n "$ready" ] || return 0
    echo >&2
    echo "The skills that are ready now can be installed one at a time:" >&2
    # Unquoted on purpose: $ready is a space-joined list of skill names.
    for skill in $ready; do
        printf '  install.sh --skill %s\n' "$skill" >&2
    done
}

# Partitions "$@" into RUNTIME_READY_SKILLS and RUNTIME_BLOCKED_SKILLS and
# reports every gap on stderr. Callers install the ready list and exit non-zero
# when the blocked list is not empty.
verify_runtime_tools() {
    local skill tool
    RUNTIME_READY_SKILLS=()
    RUNTIME_BLOCKED_SKILLS=""
    for skill in "$@"; do
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            echo >&2
            printf 'Warning: %s installs in a degraded form.\n' "$skill" >&2
            runtime_report_missing "$skill" "$tool"
        done < <(runtime_unmet_tools "$skill" soft)
        if skill_runtime_tools_present "$skill"; then
            RUNTIME_READY_SKILLS+=("$skill")
            continue
        fi
        RUNTIME_BLOCKED_SKILLS="$RUNTIME_BLOCKED_SKILLS $skill"
        echo >&2
        printf 'Not installing %s: a hard requirement is missing.\n' "$skill" >&2
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            runtime_report_missing "$skill" "$tool"
        done < <(runtime_unmet_tools "$skill" hard)
    done
    [ -n "$RUNTIME_BLOCKED_SKILLS" ] || return 0
    runtime_report_way_forward "$@"
}

# ---------------------------------------------------------------
# 5. Agent target detection
# ---------------------------------------------------------------
# Which agents are installed on this machine, plus the user's remembered custom
# skill roots. agent_target_available() switches on the TARGET_* index, so the
# registry order in section 1 must not change.
agent_target_available() {
    local index="$1"
    local path="${TARGET_PATHS[$index]}"

    case "$index" in
        0) return 0 ;; # Universal Agent Skills has no owning application.
        1) command -v codex >/dev/null 2>&1 || [ -d "$HOME/.codex" ] ;;
        2) command -v claude >/dev/null 2>&1 || [ -d "$HOME/.claude" ] ;;
        3) command -v opencode >/dev/null 2>&1 || [ -d "$HOME/.config/opencode" ] ;;
        4) command -v openclaw >/dev/null 2>&1 || [ -d "$HOME/.openclaw" ] ;;
        5)
            [ -d "$path" ] || [ -d "$HOME/.vscode/extensions/saoudrizwan.claude-dev" ] \
                || compgen -G "$HOME/.vscode/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || compgen -G "$HOME/.vscode-server/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || [ -d "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev" ] \
                || compgen -G "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev*" >/dev/null 2>&1
            ;;
        *) return 1 ;;
    esac
}

save_custom_location() {
    local path="$1"
    mkdir -p "$(dirname "$CUSTOM_LOCATIONS_FILE")"
    touch "$CUSTOM_LOCATIONS_FILE"
    if ! grep -Fqx "$path" "$CUSTOM_LOCATIONS_FILE"; then
        printf '%s\n' "$path" >> "$CUSTOM_LOCATIONS_FILE"
    fi
}

load_custom_locations() {
    SAVED_CUSTOM_LOCATIONS=()
    [ -f "$CUSTOM_LOCATIONS_FILE" ] || return 0
    local path
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [[ "$path" = \#* ]] && continue
        [ -d "$path" ] || continue
        # ${arr[@]+…}: expanding an empty array is an unbound-variable abort
        # under `set -u` on bash < 4.4 (macOS ships 3.2).
        contains "$path" ${SAVED_CUSTOM_LOCATIONS[@]+"${SAVED_CUSTOM_LOCATIONS[@]}"} \
            || SAVED_CUSTOM_LOCATIONS+=("$path")
    done < "$CUSTOM_LOCATIONS_FILE"
}

# ---------------------------------------------------------------
# 6. Terminal capability, splash, and menu rendering
# ---------------------------------------------------------------
# Everything here writes absolute cursor positions to stdout and is only reached
# on a real terminal (show_splash returns early unless fd 3 is a tty).
#
# Implicit globals crossing function boundaries in this section — all of them
# deliberate, because bash 3.2 has no way to return a value:
#   COLOR_MODE       set by detect_color_mode, read by fg_sgr
#   FG_SGR           set by fg_sgr, read by its caller on the next line
#   COLOR            set by color_for, read by render_art
#   EYE_ROW          set by eye_row_for, read by render_art and iui_head_line
#   ART              constant (section 1), read by render_art
#   MENU_BOX_WIDTH   set by show_splash (or defaulted by show_shop_menu itself)
#   MENU_COMPACT     set by show_splash, read by show_shop_menu
#   MENU_COL         set by show_shop_menu, read by menu_wrap_text
#   MENU_NEXT_ROW    set by menu_wrap_text, read by show_shop_menu
#   MENU_PROMPT_ROW  set by show_shop_menu, read by select_skills
#   REPLY            set by ask (section 3), read by confirm and every caller
#   YES_ALL          set by confirm on "a", read by every later confirm

# 24-bit SGR is not universal — macOS Terminal.app has never supported it, and
# there the splash used to come out as literal escape residue. Probe once and let
# fg_sgr downgrade to 256-colour, then to the 8 ANSI colours, then to nothing.
detect_color_mode() {
    local colors=0
    if command -v tput >/dev/null 2>&1; then
        colors="$(tput colors 2>/dev/null || echo 0)"
    fi
    case "$colors" in
        ''|*[!0-9]*) colors=0 ;;
    esac
    case "${COLORTERM:-}" in
        truecolor|24bit)
            COLOR_MODE=truecolor
            return
            ;;
    esac
    if [ "$colors" -ge 16777216 ]; then
        COLOR_MODE=truecolor
    elif [ "$colors" -ge 256 ]; then
        COLOR_MODE=256
    elif [ "$colors" -ge 8 ]; then
        COLOR_MODE=8
    else
        COLOR_MODE=none
    fi
}

# Sets FG_SGR to the best foreground escape this terminal understands for the
# "R;G;B" triple in $1, with the optional SGR attribute prefix $2 (e.g. '1;').
# printf -v avoids a subshell: this runs once per art pixel.
fg_sgr() {
    local rgb="$1" attr="${2:-}" r g b rest
    [ -n "$COLOR_MODE" ] || detect_color_mode
    case "$COLOR_MODE" in
        truecolor)
            printf -v FG_SGR '\033[%s38;2;%sm' "$attr" "$rgb"
            return
            ;;
        none)
            FG_SGR=''
            return
            ;;
    esac
    r="${rgb%%;*}"
    rest="${rgb#*;}"
    g="${rest%%;*}"
    b="${rest##*;}"
    if [ "$COLOR_MODE" = "256" ]; then
        # xterm 6x6x6 colour cube, which starts at index 16.
        printf -v FG_SGR '\033[%s38;5;%dm' "$attr" \
            "$(( 16 + 36 * (r * 5 / 255) + 6 * (g * 5 / 255) + (b * 5 / 255) ))"
    else
        printf -v FG_SGR '\033[%s3%dm' "$attr" \
            "$(( (r >= 128) + 2 * (g >= 128) + 4 * (b >= 128) ))"
    fi
}

color_for() {
    if [ "${#1}" -eq 6 ]; then
        COLOR="$((16#${1:0:2}));$((16#${1:2:2}));$((16#${1:4:2}))"
        return
    fi
    case "$1" in
        y) COLOR='224;183;44' ;;
        Y) COLOR='255;216;58' ;;
        o) COLOR='180;116;18' ;;
        O) COLOR='218;151;27' ;;
        d) COLOR='48;26;4' ;;
        b) COLOR='20;9;1' ;;
        s) COLOR='111;93;55' ;;
        H) COLOR='108;94;62' ;;
        h) COLOR='82;69;42' ;;
        W) COLOR='255;255;255' ;;
        P) COLOR='0;150;40' ;;
        G) COLOR='0;186;60' ;;
        r) COLOR='143;47;5' ;;
        *) COLOR='' ;;
    esac
}

# The pixels of sprite rows 8 and 9 for one eye state, into EYE_ROW. Both rows
# carry the same pixels in every state, so one row per state is enough for both.
# Section 6b's iui_head_line() reads this too, so the eye data exists once.
eye_row_for() {
    case "$1" in
        left)
            EYE_ROW='c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
        right)
            EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601'
            ;;
        *)
            EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
    esac
}

render_art() {
    local offset_x="$1"
    local offset_y="$2"
    local scale="$3"
    local eye_state="$4"
    local row char x y repeat pixel_width blocks
    local -a pixels

    eye_row_for "$eye_state"

    pixel_width=$((scale * 2))
    blocks=''
    for ((x = 0; x < pixel_width; x++)); do
        blocks="${blocks}█"
    done
    for ((y = 0; y < ${#ART[@]}; y++)); do
        row="${ART[$y]}"
        { [ "$y" -eq 8 ] || [ "$y" -eq 9 ]; } && row="$EYE_ROW"
        IFS=' ' read -r -a pixels <<< "$row"
        for ((repeat = 0; repeat < scale; repeat++)); do
            printf '\033[%d;%dH' "$((offset_y + y * scale + repeat))" "$offset_x"
            for ((x = 0; x < ${#pixels[@]}; x++)); do
                char="${pixels[$x]}"
                color_for "$char"
                if [ -n "$COLOR" ]; then
                    fg_sgr "$COLOR"
                    printf '%s%s\033[0m' "$FG_SGR" "$blocks"
                else
                    printf '%*s' "$pixel_width" ''
                fi
            done
        done
    done
}

menu_wrap_text() {
    local row="$1"
    local text="$2"
    local width="$3"
    local indent="$4"
    local line split

    while [ -n "$text" ]; do
        line="$text"
        if [ "${#line}" -gt "$width" ]; then
            line="${text:0:width}"
            split="${line% *}"
            if [ -n "$split" ] && [ "$split" != "$line" ]; then
                line="$split"
            fi
        fi
        printf '\033[%d;%dH%*s%s' "$row" "${MENU_COL:-1}" "$indent" '' "$line"
        row=$((row + 1))
        text="${text:${#line}}"
        text="${text# }"
    done
    MENU_NEXT_ROW="$row"
}

show_shop_menu() {
    local row=2
    local title="TSCHALLACKA'S SKILL SHOP"
    local box_width
    local inner_width title_padding
    local horizontal=''
    local index label description
    # The registry from section 1, plus the synthetic "everything" entry whose
    # number select_skills() treats as "all".
    local -a labels=("${SKILL_NAMES[@]}" 'all five skills')
    local -a descriptions=("${SKILL_DESCRIPTIONS[@]}" 'Installs or updates the complete skill set.')

    [ -n "$COLOR_MODE" ] || detect_color_mode
    if [ -z "${MENU_BOX_WIDTH:-}" ]; then
        local columns="${COLUMNS:-80}"
        if command -v tput >/dev/null 2>&1; then
            columns="$(tput cols 2>/dev/null || echo "$columns")"
        fi
        MENU_BOX_WIDTH=$((columns - 36))
        [ "$MENU_BOX_WIDTH" -gt 60 ] && MENU_BOX_WIDTH=60
        [ "$MENU_BOX_WIDTH" -lt 28 ] && MENU_BOX_WIDTH=28
    fi
    MENU_COL=${MENU_COL:-1}
    MENU_COMPACT=0
    [ "${LINES:-24}" -lt 20 ] && MENU_COMPACT=1
    box_width="$MENU_BOX_WIDTH"
    inner_width=$((box_width - 2))
    title_padding=$((inner_width - ${#title}))

    for ((index = 0; index < inner_width; index++)); do
        horizontal+='─'
    done
    fg_sgr '255;211;64' '1;'
    printf '\033[%d;%dH%s╭%s╮\033[0m' "$row" "$MENU_COL" "$FG_SGR" "$horizontal"
    printf '\033[%d;%dH%s│%*s%s%*s│\033[0m' \
        "$((row + 1))" "$MENU_COL" "$FG_SGR" "$((title_padding / 2))" '' "$title" \
        "$((title_padding - title_padding / 2))" ''
    fg_sgr '255;211;64'
    printf '\033[%d;%dH%s╰%s╯\033[0m' "$((row + 2))" "$MENU_COL" "$FG_SGR" "$horizontal"

    row=$((row + 4))
    for ((index = 0; index < ${#labels[@]}; index++)); do
        label="$((index + 1))) ${labels[$index]}"
        menu_wrap_text "$row" "$label" "$((inner_width - 2))" 2
        row="$MENU_NEXT_ROW"
        if [ "${MENU_COMPACT:-0}" -eq 0 ]; then
            description="${descriptions[$index]}"
            menu_wrap_text "$row" "$description" "$((inner_width - 7))" 5
            row="$MENU_NEXT_ROW"
        fi
        row=$((row + 1))
    done
    MENU_PROMPT_ROW="$row"
}

pixel_message() {
    local frame="$1" base_x="$2" base_y="$3" row col char glyph line
    local -a chars glyph_rows
    chars=(m h h h - m h h h)
    fg_sgr '255;211;64'
    for ((row = 0; row < 7; row++)); do
        printf '\033[%d;%dH%s' "$((base_y + row))" "$base_x" "$FG_SGR"
        for ((col = 0; col < ${#chars[@]}; col++)); do
            if [ "$col" -ge "$frame" ]; then
                printf '          '
                continue
            fi
            char="${chars[$col]}"
            case "$char" in
                m) glyph='10001|11011|10101|10101|10001|10001|10001' ;;
                h) glyph='10001|10001|10001|11111|10001|10001|10001' ;;
                -) glyph='00000|00000|00000|11111|00000|00000|00000' ;;
            esac
            IFS='|' read -r -a glyph_rows <<< "$glyph"
            line="${glyph_rows[$row]}"
            line="${line//1/██}"
            line="${line//0/  }"
            printf '%s  ' "$line"
        done
        printf '\033[0m'
    done
}

show_splash() {
    # Every early exit says 0 explicitly: a bare return here propagated the
    # tty test's status 1 and, under set -e, killed headless installs before
    # they printed anything at all (B39).
    [ "${AI_SKILLS_NO_SPLASH:-0}" = "1" ] && return 0
    [ -t 3 ] || return 0
    [ -n "$COLOR_MODE" ] || detect_color_mode
    # A terminal with no colour at all gets no pixel mascot; the menu still works.
    [ "$COLOR_MODE" = "none" ] && return 0

    local columns lines scale art_width art_height offset_x offset_y state step anim_scale anim_x anim_y message_y
    columns="${COLUMNS:-80}"
    lines="${LINES:-24}"
    if command -v tput >/dev/null 2>&1; then
        columns="$(tput cols 2>/dev/null || echo "$columns")"
        lines="$(tput lines 2>/dev/null || echo "$lines")"
    fi
    scale=1
    while [ $(( (scale + 1) * 32 )) -le "$columns" ] \
        && [ $(( (scale + 1) * 16 )) -le "$((lines - 5))" ]; do
        scale=$((scale + 1))
    done
    art_width=$((scale * 32))
    art_height=$((scale * 16))
    offset_x=$((columns - art_width - 1))
    offset_y=2
    [ "$offset_x" -lt 1 ] && offset_x=1
    MENU_BOX_WIDTH=$((offset_x - 4))
    [ "$MENU_BOX_WIDTH" -gt 60 ] && MENU_BOX_WIDTH=60
    [ "$MENU_BOX_WIDTH" -lt 28 ] && MENU_BOX_WIDTH=28
    MENU_COMPACT=0
    [ "$lines" -lt 20 ] && MENU_COMPACT=1
    message_y=$((lines - 7))
    [ "$message_y" -lt 1 ] && message_y=1

    printf '\033[?25l\033[2J\033[H'
    for ((anim_scale = 1; anim_scale <= scale; anim_scale++)); do
        printf '\033[2J\033[H'
        anim_x=$(( (columns - anim_scale * 32) / 2 + 1 ))
        anim_y=$(( (lines - anim_scale * 16) / 2 + 1 ))
        [ "$anim_x" -lt 1 ] && anim_x=1
        [ "$anim_y" -lt 1 ] && anim_y=1
        render_art "$anim_x" "$anim_y" "$anim_scale" front
        sleep 0.18
    done

    for state in right left front; do
        printf '\033[2J\033[H'
        render_art "$offset_x" "$offset_y" "$scale" "$state"
        sleep 0.60
    done

    for ((step = 1; step <= 9; step++)); do
        printf '\033[2J\033[H'
        render_art "$offset_x" "$offset_y" 1 front
        pixel_message "$step" 2 "$message_y"
        sleep 0.15
    done

    # Leave the mascot in the right column when the menu opens.
    printf '\033[2J\033[H'
    render_art "$offset_x" "$offset_y" 1 front
    printf '\033[?25h'
}

# ---------------------------------------------------------------
# 6b. Full-screen skill picker: state, text metrics, requirement model
# ---------------------------------------------------------------
# The interactive picker that replaces the numbered menu of section 6 when fd 3
# is a terminal. Its three parts are split by concern and read in order: this one
# owns the state and the arithmetic, 6c draws a frame, 6d reads the keyboard and
# is the seam select_skills() calls.
#
# Everything here is prefixed iui_ and shares section 6's primitives rather than
# repeating them: ART, detect_color_mode, fg_sgr, color_for and eye_row_for are
# defined there, so the sprite and the palette exist once in this file.
#
# Banners in this part, in order:
#   State
#   Palette segments
#   Glyph sets and the plain-text fold
#   The requirement table — the data seam
#   The global, per-tool dependency cache
#   Layout arithmetic

# ─────────────────────────────────────────────────────────────────────────────
# State
# ─────────────────────────────────────────────────────────────────────────────

# PORTABILITY(assoc-array): the skill table and the dependency cache both want a
# map; index-parallel arrays plus a linear lookup are the bash 3.2 shape of one.
IUI_SKILL_NAMES=()
IUI_SKILL_DETAILS=()
IUI_INFO_SCROLL=0
IUI_INFO_ROWS=0
IUI_INFO_MAX_SCROLL=0
IUI_SKILL_DESCS=()
IUI_SKILL_INSTALLED=()
IUI_SKILL_HAVE=()
IUI_SKILL_WANT=()
IUI_SKILL_SEL=()

# The global dependency cache. Keyed by TOOL, never by skill, so reverifying
# `jq` from one skill's info box refreshes every skill that also needs `jq`.
IUI_DEP_TOOLS=()
IUI_DEP_STATES=()

IUI_CURSOR=0
IUI_SCROLL=0
IUI_FOCUS=list
IUI_COLS=80
IUI_ROWS=24
IUI_GLYPHS=blocks
IUI_POSITION=0
IUI_DONE=0
IUI_RC=130
IUI_EYE=front
IUI_HEAD_ON=0
IUI_HEAD_SCALE=0
IUI_HEAD_H=0
IUI_MESSAGE=()
IUI_STTY_SAVED=""
IUI_TERM_ACTIVE=0

# ─────────────────────────────────────────────────────────────────────────────
# Palette segments
# ─────────────────────────────────────────────────────────────────────────────

# Minecraft block palette. iui_seg() wraps text in a role's colour; in the
# `none` colour mode every wrap is the identity, which is what keeps the ASCII
# fallback free of escape bytes.
iui_seg() {
    local role="$1" text="$2" rgb='' attr=''
    [ -n "$COLOR_MODE" ] || detect_color_mode
    case "$role" in
        grass) rgb='91;135;49' ;;
        dirt) rgb='139;90;43' ;;
        gold) rgb='252;238;75'; attr='1;' ;;
        redstone) rgb='217;58;43' ;;
        diamond) rgb='74;237;217' ;;
        obsidian) rgb='21;12;28' ;;
        # Body text. Named explicitly rather than left to the fallback below: at
        # 127;127;127 it measured 4.2:1 against a #1e1e1e terminal, which reads as
        # dark grey and is under the 4.5:1 needed for body text. 205;207;212 is
        # 11:1 on the same ground, with a faint cool bias so it belongs to the
        # palette instead of being pure grey.
        stone) rgb='205;207;212' ;;
        # The focused pane's label. Reverse video rather than another hue: the
        # only previous cue was [BRACKETS] versus spaces, which is invisible at
        # a glance and vanishes entirely in the no-colour mode.
        focus) rgb='252;238;75'; attr='1;7;' ;;
        # Secondary text that should recede without becoming unreadable.
        slate) rgb='150;154;163' ;;
        # Deliberately left as mid grey: a role nobody named should look wrong
        # rather than quietly inheriting body-text contrast.
        *) rgb='127;127;127' ;;
    esac
    if [ "$COLOR_MODE" = "none" ]; then
        IUI_SEG="$text"
        return
    fi
    fg_sgr "$rgb" "$attr"
    IUI_SEG="$FG_SGR$text"$'\033'"[0m"
}

# ─────────────────────────────────────────────────────────────────────────────
# Glyph sets and the plain-text fold
# ─────────────────────────────────────────────────────────────────────────────

# The glyph vocabulary is closed: these nine are the only non-ASCII bytes a frame
# may hold, and iui_plain() folds exactly them back to one byte each.
# ---- quoted: glyph -> ASCII fold ----
# ▀ =    ▄ =    ▌ |
# ▛ +    ▜ +    ▙ +    ▟ +
# ─ -    █ #
# ---- end quoted ----
iui_set_glyphs() {
    IUI_GLYPHS="${1:-blocks}"
    if [ "$IUI_GLYPHS" = "ascii" ]; then
        IUI_G_TOP='='; IUI_G_BOT='='; IUI_G_V='|'
        IUI_G_TL='+'; IUI_G_TR='+'; IUI_G_BL='+'; IUI_G_BR='+'
        IUI_G_TJ='+'; IUI_G_BJ='+'
        IUI_G_RULE='-'; IUI_G_FILL='#'
        return
    fi
    IUI_G_TOP='▀'; IUI_G_BOT='▄'; IUI_G_V='▌'
    IUI_G_TL='▛'; IUI_G_TR='▜'; IUI_G_BL='▙'; IUI_G_BR='▟'
    # Where the pane divider meets the top and bottom borders. Without these the
    # border drew a plain horizontal at the divider column, so the vertical line
    # below it looked disconnected from the frame.
    IUI_G_TJ='▛'; IUI_G_BJ='▙'
    IUI_G_RULE='─'; IUI_G_FILL='█'
}

# Strips SGR sequences and folds the glyph set, leaving one ASCII byte per
# display cell. Pure parameter expansion — no forks, so it is cheap enough to
# call on every rendered line as a self-check.
iui_plain() {
    local s="$1" pre rest
    while [ "${s#*$'\033'\[}" != "$s" ]; do
        pre="${s%%$'\033'\[*}"
        rest="${s#*$'\033'\[}"
        rest="${rest#*m}"
        s="$pre$rest"
    done
    s="${s//▀/=}"; s="${s//▄/=}"; s="${s//▌/|}"
    s="${s//▛/+}"; s="${s//▜/+}"; s="${s//▙/+}"; s="${s//▟/+}"
    s="${s//─/-}"; s="${s//█/#}"
    IUI_PLAIN="$s"
}

iui_repeat() {
    local glyph="$1" count="$2" out='' i
    for ((i = 0; i < count; i++)); do
        out="$out$glyph"
    done
    IUI_REPEAT="$out"
}

# PORTABILITY(bytes-vs-characters): the pad/truncate arithmetic is in display
# cells and the text it measures is ASCII, so ${#t} is both the byte and the
# cell count. Never hand this a glyph — glyphs are placed by count, not measured.
# A 5x7 pixel font, as data rather than a case statement: bash 3.2 has no
# associative arrays, and forty case arms would breach the function-length cap.
# One entry per character, `CHAR:row|row|...` with seven rows of five bits.
#
# The splash already drew mmh-mhh in this shape at two columns per pixel
# (pixel_message). The hints reuse the shape at one column per pixel, which is
# the "tad bit smaller" that keeps a whole hint on one line.
IUI_PIXEL_FONT=(
    '_:00000|00000|00000|00000|00000|00000|00000'
    '-:00000|00000|00000|11111|00000|00000|00000'
    '.:00000|00000|00000|00000|00000|00100|00100'
    '/:00001|00010|00010|00100|01000|01000|10000'
    '0:01110|10001|10011|10101|11001|10001|01110'
    '1:00100|01100|00100|00100|00100|00100|01110'
    '2:01110|10001|00001|00010|00100|01000|11111'
    '3:11110|00001|00001|01110|00001|00001|11110'
    '4:00010|00110|01010|10010|11111|00010|00010'
    '5:11111|10000|11110|00001|00001|10001|01110'
    '6:00110|01000|10000|11110|10001|10001|01110'
    '7:11111|00001|00010|00100|01000|01000|01000'
    '8:01110|10001|10001|01110|10001|10001|01110'
    '9:01110|10001|10001|01111|00001|00010|01100'
    '::00000|00100|00000|00000|00000|00100|00000'
    'A:01110|10001|10001|11111|10001|10001|10001'
    'B:11110|10001|10001|11110|10001|10001|11110'
    'C:01110|10001|10000|10000|10000|10001|01110'
    'D:11110|10001|10001|10001|10001|10001|11110'
    'E:11111|10000|10000|11110|10000|10000|11111'
    'F:11111|10000|10000|11110|10000|10000|10000'
    'G:01110|10001|10000|10111|10001|10001|01110'
    'H:10001|10001|10001|11111|10001|10001|10001'
    'I:11111|00100|00100|00100|00100|00100|11111'
    'J:00111|00010|00010|00010|00010|10010|01100'
    'K:10001|10010|10100|11000|10100|10010|10001'
    'L:10000|10000|10000|10000|10000|10000|11111'
    'M:10001|11011|10101|10101|10001|10001|10001'
    'N:10001|11001|10101|10011|10001|10001|10001'
    'O:01110|10001|10001|10001|10001|10001|01110'
    'P:11110|10001|10001|11110|10000|10000|10000'
    'Q:01110|10001|10001|10001|10101|10011|01111'
    'R:11110|10001|10001|11110|10100|10010|10001'
    'S:01111|10000|10000|01110|00001|00001|11110'
    'T:11111|00100|00100|00100|00100|00100|00100'
    'U:10001|10001|10001|10001|10001|10001|01110'
    'V:10001|10001|10001|10001|10001|01010|00100'
    'W:10001|10001|10001|10101|10101|11011|10001'
    'X:10001|01010|00100|00100|00100|01010|10001'
    'Y:10001|10001|01010|00100|00100|00100|00100'
    'Z:11111|00001|00010|00100|01000|10000|11111'
)

# One pixel row of one character. Unknown characters render blank rather than
# failing: a hint is decoration, and a missing glyph must not take the frame down.
iui_pixel_rows() {
    local want="$1" entry
    [ "$want" != ' ' ] || want='_'
    IUI_PIXEL_ROWS=''
    for entry in "${IUI_PIXEL_FONT[@]}"; do
        if [ "${entry%%:*}" = "$want" ]; then
            IUI_PIXEL_ROWS="${entry#*:}"
            return 0
        fi
    done
    IUI_PIXEL_ROWS='00000|00000|00000|00000|00000|00000|00000'
}

# Row `row` (0-6) of `text` rendered in the pixel font, padded to `cols`.
iui_big_line() {
    local text="$1" row="$2" cols="$3" i char out='' bits
    local -a rows
    for ((i = 0; i < ${#text}; i++)); do
        char="${text:$i:1}"
        iui_pixel_rows "$char"
        IFS='|' read -r -a rows <<< "$IUI_PIXEL_ROWS"
        bits="${rows[$row]}"
        bits="${bits//1/$IUI_G_FILL}"
        bits="${bits// /_}"
        bits="${bits//0/ }"
        out="$out$bits "
    done
    iui_pad "$out" "$cols"
    IUI_BIG_LINE="$IUI_PAD"
}

iui_pad() {
    local t="$1" w="$2"
    [ "$w" -ge 0 ] || w=0
    if [ "${#t}" -gt "$w" ]; then
        if [ "$w" -ge 1 ]; then t="${t:0:$((w - 1))}~"; else t=''; fi
    fi
    printf -v IUI_PAD '%s%*s' "$t" "$((w - ${#t}))" ''
}

# Wraps to whole words where it can and hyphenates where it cannot, so a token
# longer than the pane continues on the next line as `term-` / `inator` rather
# than being cut without a mark. `consume` is tracked separately from the emitted
# line because the hyphen is added by this function and is not part of the source
# text: consuming ${#line} would swallow the character the hyphen replaced.
iui_wrap() {
    local text="$1" width="$2" line split consume
    IUI_WRAP_LINES=()
    [ "$width" -ge 8 ] || width=8
    while [ -n "$text" ]; do
        line="$text"
        consume="${#text}"
        if [ "${#line}" -gt "$width" ]; then
            line="${text:0:width}"
            split="${line% *}"
            if [ -n "$split" ] && [ "$split" != "$line" ]; then
                line="$split"
                consume="${#line}"
            else
                # One unbroken token wider than the pane: break it and say so.
                consume=$((width - 1))
                line="${text:0:consume}-"
            fi
        fi
        IUI_WRAP_LINES+=("$line")
        text="${text:consume}"
        text="${text# }"
    done
    [ "${#IUI_WRAP_LINES[@]}" -gt 0 ] || IUI_WRAP_LINES=('')
}

# ─────────────────────────────────────────────────────────────────────────────
# The requirement table — the data seam
# ─────────────────────────────────────────────────────────────────────────────

# One row per (skill, tool) requirement. iui_req_add() is the only ingest point
# and iui_load_requirements() the only producer, so wiring a generated table in
# means replacing one function; nothing in the renderer reads a skill list.
IUI_REQ_SKILL=()
IUI_REQ_TOOL=()
IUI_REQ_COND=()
IUI_REQ_STRENGTH=()
IUI_REQ_WHY=()

# iui_req_add <skill-index> <tool> <condition> hard|soft <what-is-lost>
# The condition is `*`, an OS glob, or OS:ARCH — memlimit is Darwin:arm64 only
# and must not show up as required on an Intel Mac.
iui_req_add() {
    IUI_REQ_SKILL+=("$1")
    IUI_REQ_TOOL+=("$2")
    IUI_REQ_COND+=("$3")
    IUI_REQ_STRENGTH+=("$4")
    IUI_REQ_WHY+=("$5")
}

iui_req_reset() {
    IUI_REQ_SKILL=(); IUI_REQ_TOOL=(); IUI_REQ_COND=()
    IUI_REQ_STRENGTH=(); IUI_REQ_WHY=()
}

iui_platform() {
    [ -n "${IUI_UNAME_S:-}" ] || IUI_UNAME_S="$(uname -s)"
    [ -n "${IUI_UNAME_M:-}" ] || IUI_UNAME_M="$(uname -m)"
}

iui_cond_applies() {
    local cond="$1" os arch
    [ "$cond" = '*' ] && return 0
    iui_platform
    os="${cond%%:*}"
    # Unquoted patterns on purpose: MINGW*/CYGWIN* are globs.
    # shellcheck disable=SC2254
    case "$IUI_UNAME_S" in $os) : ;; *) return 1 ;; esac
    [ "$cond" = "$os" ] && return 0
    arch="${cond#*:}"
    # shellcheck disable=SC2254
    case "$IUI_UNAME_M" in $arch) return 0 ;; esac
    return 1
}

iui_req_applies() {
    [ "${IUI_REQ_SKILL[$1]}" = "$2" ] || return 1
    iui_cond_applies "${IUI_REQ_COND[$1]}"
}

# The one producer of the table, from section 4's generated tables and not a
# skill's requires.tsv: select_skills() runs before download_source(), so under
# `curl … | bash` no skill directory exists yet. Conditions are applied there.
iui_load_requirements() {
    local i skill tool
    iui_req_reset
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        skill="${IUI_SKILL_NAMES[$i]}"
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            iui_req_add "$i" "$tool" '*' \
                "$(runtime_requirement_strength "$skill" "$tool")" \
                "$(runtime_requirement_why "$skill" "$tool")"
        done < <(runtime_requirements "$skill")
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# The global, per-tool dependency cache
# ─────────────────────────────────────────────────────────────────────────────

# Keyed by tool, never by skill: every read goes through iui_dep_state and every
# write through iui_dep_set, so one record serves every skill needing that tool.
iui_dep_index() {
    local wanted="$1" i
    IUI_DEP_INDEX=-1
    for ((i = 0; i < ${#IUI_DEP_TOOLS[@]}; i++)); do
        if [ "${IUI_DEP_TOOLS[$i]}" = "$wanted" ]; then
            IUI_DEP_INDEX="$i"
            return
        fi
    done
}

iui_dep_set() {
    iui_dep_index "$1"
    if [ "$IUI_DEP_INDEX" -lt 0 ]; then
        IUI_DEP_TOOLS+=("$1")
        IUI_DEP_STATES+=("$2")
        return
    fi
    IUI_DEP_STATES[$IUI_DEP_INDEX]="$2"
}

iui_dep_state() {
    iui_dep_index "$1"
    if [ "$IUI_DEP_INDEX" -lt 0 ]; then
        iui_dep_set "$1" unknown
        IUI_DEP_STATE=unknown
        return
    fi
    IUI_DEP_STATE="${IUI_DEP_STATES[$IUI_DEP_INDEX]}"
}

# The generated runtime_tool_verify() from section 4 is the authority for a
# single tool, and runtime_requirement_met() extends it to any-of groups, so
# the picker and verify_runtime_tools() can never disagree. Overridable so a
# test can inject a probe result without installing tools.
iui_dep_probe() {
    case $1 in
        @*) runtime_requirement_met "$1" ;;
        *) runtime_tool_verify "$1" ;;
    esac
}

iui_dep_verify() {
    if iui_dep_probe "$1"; then
        iui_dep_set "$1" ok
    else
        iui_dep_set "$1" missing
    fi
}

# Reverify every applicable tool of one skill. The writes land in the per-tool
# cache, so the refresh is global by construction.
iui_dep_reverify_skill() {
    local index="$1" i
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_verify "${IUI_REQ_TOOL[$i]}"
    done
}

# blocked beats degraded: an unmet hard requirement stops the install, an unmet
# soft one only costs capability. IUI_SKILL_BLOCKER names the first offender.
iui_skill_state() {
    local index="$1" i
    IUI_SKILL_STATE=ok
    IUI_SKILL_BLOCKER=''
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        if [ "${IUI_REQ_STRENGTH[$i]}" = "hard" ]; then
            IUI_SKILL_STATE=blocked
            IUI_SKILL_BLOCKER="${IUI_REQ_TOOL[$i]}"
            return
        fi
        IUI_SKILL_STATE=degraded
        [ -n "$IUI_SKILL_BLOCKER" ] || IUI_SKILL_BLOCKER="${IUI_REQ_TOOL[$i]}"
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# The published release, for the "latest release" row
# ─────────────────────────────────────────────────────────────────────────────

# Asked once per run and cached: the picker redraws on every keypress, so a
# per-render fetch would be one HTTP request per arrow key.
#
# raw.githubusercontent rather than the GitHub API: raw needs no token and has no
# 60-per-hour unauthenticated rate limit, and the repository is public. The cost
# is a CDN cache of a few minutes, so a version pushed seconds ago can still read
# as the old one -- which is why this row says "latest release" and not "HEAD".
IUI_RELEASE_URL="https://raw.githubusercontent.com/tschallacka/ai-skills/master/package.json"
IUI_RELEASE_VERSION=''
IUI_RELEASE_CHECKED=0
iui_release_version() {
    [ "$IUI_RELEASE_CHECKED" -eq 0 ] || return 0
    IUI_RELEASE_CHECKED=1
    # IUI_NO_NETWORK=1 is how a test pins the offline path.
    [ "${IUI_NO_NETWORK:-0}" -eq 0 ] || return 0
    command -v curl >/dev/null 2>&1 || return 0
    # A picker that hangs on a dead network is worse than one that cannot answer,
    # so the timeout is short and a failure leaves the answer empty rather than
    # reporting a version nobody fetched.
    IUI_RELEASE_VERSION="$(curl -fsSL --max-time 3 "$IUI_RELEASE_URL" 2>/dev/null \
        | awk -F'"' '/"version"/ { print $4; exit }')"
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# Layout arithmetic
# ─────────────────────────────────────────────────────────────────────────────

# Rows: 1 title, 1 top border, IUI_BODY_ROWS body, 1 bottom border, 1 hint.
# Columns: 1 vertical + IUI_LEFT_W + 1 divider + IUI_RIGHT_W + 1 vertical.
# IUI_NARROW is the degrade path: one pane at a time, whichever has focus.
iui_layout() {
    IUI_BODY_ROWS=$((IUI_ROWS - 4))
    [ "$IUI_BODY_ROWS" -ge 1 ] || IUI_BODY_ROWS=1
    IUI_NARROW=0
    if [ "$IUI_COLS" -lt 56 ]; then
        IUI_NARROW=1
        IUI_LEFT_W=$((IUI_COLS - 2))
        [ "$IUI_LEFT_W" -ge 8 ] || IUI_LEFT_W=8
        IUI_RIGHT_W="$IUI_LEFT_W"
        iui_head_geometry
        return
    fi
    iui_list_width
    IUI_RIGHT_W=$((IUI_COLS - IUI_LEFT_W - 3))
    iui_head_geometry
}

# The list is sized to its content, not to a fraction of the terminal: a row is
# cursor(1) + checkbox(3) + space(1) + name + state tag(6), so the longest skill
# name decides the width and nothing truncates. A third of the terminal was the
# earlier rule and it capped at 34 columns, which cut
# post-implementation-review (26 chars, needing 37) on every screen size.
IUI_LIST_MIN_W=22
IUI_LIST_MAX_W=46
IUI_DETAIL_MIN_W=30
iui_list_width() {
    local i longest=0 length
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        length="${#IUI_SKILL_NAMES[$i]}"
        [ "$length" -le "$longest" ] || longest="$length"
    done
    IUI_LEFT_W=$((longest + 11))
    [ "$IUI_LEFT_W" -ge "$IUI_LIST_MIN_W" ] || IUI_LEFT_W="$IUI_LIST_MIN_W"
    [ "$IUI_LEFT_W" -le "$IUI_LIST_MAX_W" ] || IUI_LEFT_W="$IUI_LIST_MAX_W"
    # The detail pane is the reason the picker exists; it wins a narrow screen.
    local ceiling=$((IUI_COLS - 3 - IUI_DETAIL_MIN_W))
    [ "$IUI_LEFT_W" -le "$ceiling" ] || IUI_LEFT_W="$ceiling"
    [ "$IUI_LEFT_W" -ge 8 ] || IUI_LEFT_W=8
}

# The sprite costs scale*32 columns and scale*16 rows, and is dropped rather than
# shrunk when it does not fit: with no colour it would paint blank, and in the
# narrow layout or under IUI_HEAD_MIN_TEXT_ROWS it would starve the info text.
# Where the sprite goes depends on how much room there is, and there are three
# answers rather than two:
#
#   right-top    a tall, wide terminal. The sprite sits at the top of the detail
#                pane and the empty space beside it -- which was blank before --
#                carries the hints.
#   left-bottom  an ordinary terminal. The sprite sits at the bottom of the list
#                pane, in the rows the list does not use, alone and with nothing
#                animating beside it. The detail pane keeps its full height.
#   none         no colour, a narrow layout, or not enough rows to spare.
#
# The deciding measurement is rows: the detail text must keep a real floor after
# the sprite has taken its band, or the hints would be bought with the content a
# reader came for.
IUI_HEAD_MIN_LIST_ROWS=6
IUI_HEAD_DETAIL_FLOOR=30
# The hints are drawn in the pixel font at six columns per character, so the
# space beside the sprite has to be wide enough for the longest of them. 34 was
# the figure for plain text and would have truncated every hint into nonsense.
IUI_HINT_MIN_COLS=108
iui_head_geometry() {
    IUI_HEAD_ON=0
    IUI_HEAD_SCALE=0
    IUI_HEAD_H=0
    IUI_HINT_ROWS=0
    IUI_HEAD_PLACE="none"
    IUI_LIST_ROWS="$IUI_BODY_ROWS"
    [ "$IUI_NARROW" -eq 0 ] || return 0
    [ -n "$COLOR_MODE" ] || detect_color_mode
    [ "$COLOR_MODE" != "none" ] || return 0
    # One size, always: 32x16. The sprite cannot shrink below it, and growing it
    # bought nothing while costing the detail pane its floor -- at 200x60 a
    # doubled sprite pushed the whole layout back into the small-screen case.
    IUI_HEAD_SCALE=1
    IUI_HEAD_H=16
    [ "$IUI_LEFT_W" -ge 32 ] || [ "$IUI_RIGHT_W" -ge $((32 + IUI_HINT_MIN_COLS)) ] || return 0
    IUI_HEAD_ON=1
    if [ $((IUI_BODY_ROWS - IUI_HEAD_H)) -ge "$IUI_HEAD_DETAIL_FLOOR" ] \
        && [ $((IUI_RIGHT_W - IUI_HEAD_SCALE * 32)) -ge "$IUI_HINT_MIN_COLS" ]; then
        IUI_HEAD_PLACE="right-top"
        IUI_HINT_ROWS="$IUI_HEAD_H"
        return 0
    fi
    if [ $((IUI_BODY_ROWS - IUI_HEAD_H - 1)) -ge "$IUI_HEAD_MIN_LIST_ROWS" ]; then
        IUI_HEAD_PLACE="left-bottom"
        IUI_LIST_ROWS=$((IUI_BODY_ROWS - IUI_HEAD_H - 1))
        return 0
    fi
    IUI_HEAD_ON=0
    IUI_HEAD_H=0
    return 0
}

iui_clamp_scroll() {
    local count rows
    count="${#IUI_SKILL_NAMES[@]}"
    rows="$IUI_LIST_ROWS"
    [ "$IUI_CURSOR" -ge 0 ] || IUI_CURSOR=0
    [ "$IUI_CURSOR" -lt "$count" ] || IUI_CURSOR=$((count - 1))
    [ "$IUI_CURSOR" -ge 0 ] || IUI_CURSOR=0
    [ "$IUI_SCROLL" -le "$IUI_CURSOR" ] || IUI_SCROLL="$IUI_CURSOR"
    if [ "$IUI_CURSOR" -ge $((IUI_SCROLL + rows)) ]; then
        IUI_SCROLL=$((IUI_CURSOR - rows + 1))
    fi
    if [ "$count" -le "$rows" ]; then
        IUI_SCROLL=0
    elif [ "$IUI_SCROLL" -gt $((count - rows)) ]; then
        IUI_SCROLL=$((count - rows))
    fi
    [ "$IUI_SCROLL" -ge 0 ] || IUI_SCROLL=0
}

# The info pane scrolls independently of the list. Without this the pane was
# fixed: tabbing to it highlighted it, but Up/Down still moved the skill cursor,
# so anything past the visible rows -- the whole detail block -- was unreachable.
iui_clamp_info_scroll() {
    IUI_INFO_ROWS=$((IUI_BODY_ROWS - IUI_HEAD_H))
    [ "$IUI_INFO_ROWS" -ge 1 ] || IUI_INFO_ROWS=1
    IUI_INFO_MAX_SCROLL=$(( ${#IUI_INFO_TEXT[@]} - IUI_INFO_ROWS ))
    [ "$IUI_INFO_MAX_SCROLL" -ge 0 ] || IUI_INFO_MAX_SCROLL=0
    [ "$IUI_INFO_SCROLL" -le "$IUI_INFO_MAX_SCROLL" ] || IUI_INFO_SCROLL="$IUI_INFO_MAX_SCROLL"
    [ "$IUI_INFO_SCROLL" -ge 0 ] || IUI_INFO_SCROLL=0
}

iui_measure() {
    local columns="${COLUMNS:-80}" lines="${LINES:-24}"
    if command -v tput >/dev/null 2>&1; then
        columns="$(tput cols 2>/dev/null || echo "$columns")"
        lines="$(tput lines 2>/dev/null || echo "$lines")"
    fi
    case "$columns" in ''|*[!0-9]*) columns=80 ;; esac
    case "$lines" in ''|*[!0-9]*) lines=24 ;; esac
    IUI_COLS="$columns"
    IUI_ROWS="$lines"
    [ "$IUI_COLS" -ge 20 ] || IUI_COLS=20
    [ "$IUI_ROWS" -ge 6 ] || IUI_ROWS=6
}
# ---------------------------------------------------------------
# 6c. Full-screen skill picker: the sprite and the frame
# ---------------------------------------------------------------
# Draws one frame of the picker from the state 6b holds. The sprite pixels come
# from section 6's ART and eye_row_for(), so this part carries the geometry of
# the head and not a second copy of the head.
#
# Banners in this part, in order:
#   The animated head
#   Frame rendering

# ─────────────────────────────────────────────────────────────────────────────
# The animated head
# ─────────────────────────────────────────────────────────────────────────────

# One entry per tick, and a tick is one second: iui_read_first_byte's timeout is
# `read -t 1`, and bash 3.2 -- the macOS floor -- rejects a fractional timeout
# outright, so one second is the shortest frame available. That is why the shake
# at the end is two held beats rather than a flutter.
#
# The dwell at the front is deliberate: the sprite sat still for one second and
# then moved, which reads as a glitch rather than as the start of something. Six
# still frames give the movement a beginning.
IUI_EYE_FRAMES=(
    front front front front front front
    right right
    front
    left left
    front
    left right front
)
IUI_EYE_INDEX=0
# Advances by index, not by searching for the current state. The search version
# matched the FIRST entry equal to IUI_EYE, and `front` appears more than once,
# so the sequence collapsed to front/right/front/right and `left` was never
# reached at all -- measured, not suspected.
iui_advance_eye() {
    IUI_EYE_INDEX=$(( (IUI_EYE_INDEX + 1) % ${#IUI_EYE_FRAMES[@]} ))
    IUI_EYE="${IUI_EYE_FRAMES[$IUI_EYE_INDEX]}"
}

# One sprite row as a string of exactly $3 display cells: 16 pixels of
# scale*2 fill glyphs each, then padding out to the pane width.
iui_head_line() {
    local art_y="$1" scale="$2" width="$3" eye="$4"
    local row blocks out='' pad x
    local -a pixels
    row="${ART[$art_y]}"
    if [ "$art_y" -eq 8 ] || [ "$art_y" -eq 9 ]; then
        eye_row_for "$eye"
        row="$EYE_ROW"
    fi
    iui_repeat "$IUI_G_FILL" $((scale * 2))
    blocks="$IUI_REPEAT"
    IFS=' ' read -r -a pixels <<< "$row"
    printf -v pad '%*s' $((scale * 2)) ''
    for ((x = 0; x < ${#pixels[@]}; x++)); do
        color_for "${pixels[$x]}"
        if [ -n "$COLOR" ]; then
            fg_sgr "$COLOR"
            out="$out$FG_SGR$blocks"$'\033'"[0m"
        else
            out="$out$pad"
        fi
    done
    printf -v pad '%*s' $((width - scale * 32)) ''
    IUI_HEAD_LINE="$out$pad"
}

# The eyes are the only thing that changes without user input, so their redraw
# is separate from the full frame: two sprite rows, absolutely positioned, and
# nothing else is touched. That is why iui_head_line() is width-parameterised.
iui_redraw_eyes() {
    [ "$IUI_HEAD_ON" -eq 1 ] || return 0
    [ "$IUI_POSITION" -eq 1 ] || return 0
    local scale="$IUI_HEAD_SCALE" col art_y r row top
    if [ "$IUI_HEAD_PLACE" = right-top ]; then
        col=$((IUI_LEFT_W + 3))
        top=3
    else
        col=2
        top=$((3 + IUI_LIST_ROWS + 1))
    fi
    for art_y in 8 9; do
        iui_head_line "$art_y" "$scale" $((scale * 32)) "$IUI_EYE"
        for ((r = 0; r < scale; r++)); do
            row=$((top + art_y * scale + r))
            printf '\033[%d;%dH%s' "$row" "$col" "$IUI_HEAD_LINE"
        done
    done
    printf '\033[%d;1H' "$IUI_ROWS"
}

# ─────────────────────────────────────────────────────────────────────────────
# Frame rendering
# ─────────────────────────────────────────────────────────────────────────────

# One renderer serves both modes: IUI_POSITION=1 prefixes an absolute cursor
# move (interactive), IUI_POSITION=0 emits plain lines (headless, assertable).
iui_out_line() {
    if [ "$IUI_POSITION" -eq 1 ]; then
        printf '\033[%d;1H\033[K%s' "$1" "$2"
    else
        printf '%s\n' "$2"
    fi
}

# A header segment of exactly $2 cells: one edge glyph, the label, then filler.
# A focused pane's label is bracketed, so the focus is assertable without colour.
iui_header_seg() {
    local label="$1" width="$2" focused="$3" text head
    if [ "$focused" -eq 1 ]; then text="[$label]"; else text=" $label "; fi
    [ "${#text}" -le $((width - 2)) ] || text="${text:0:$((width - 2))}"
    iui_repeat "$IUI_G_TOP" 1
    if [ "$focused" -eq 1 ]; then
        iui_seg focus "$text"
    else
        iui_seg dirt "$text"
    fi
    head="$IUI_REPEAT$IUI_SEG"
    iui_repeat "$IUI_G_TOP" $((width - 1 - ${#text}))
    iui_seg dirt "$IUI_REPEAT"
    IUI_HEADER_SEG="$head$IUI_SEG"
}

# The row carries the requirement verdict as a word as well as a colour, so the
# three states stay apart in 8-colour mode and in the ASCII fallback. The tag is
# dropped below 16 cells, where there is no room for a name beside it.
iui_list_cell() {
    local index="$1" width="$2" role box cursor tag='' tag_w=0
    if [ "${IUI_SKILL_SEL[$index]}" -eq 1 ]; then box="[$IUI_G_FILL]"; else box='[ ]'; fi
    if [ "$index" -eq "$IUI_CURSOR" ]; then cursor='>'; else cursor=' '; fi
    role=stone
    [ "${IUI_SKILL_SEL[$index]}" -eq 1 ] && role=grass
    [ "$index" -eq "$IUI_CURSOR" ] && role=gold
    iui_skill_state "$index"
    if [ "$width" -ge 16 ]; then
        tag_w=6
        case "$IUI_SKILL_STATE" in
            blocked) tag=' block' ;;
            degraded) tag='  warn' ;;
            *) tag='    ok' ;;
        esac
    fi
    iui_pad " ${IUI_SKILL_NAMES[$index]}" $((width - 4 - tag_w))
    iui_seg "$role" "$cursor$box$IUI_PAD"
    IUI_LIST_CELL="$IUI_SEG"
    [ "$tag_w" -eq 0 ] && return
    iui_seg "$(iui_state_role "$IUI_SKILL_STATE")" "$tag"
    IUI_LIST_CELL="$IUI_LIST_CELL$IUI_SEG"
}

# Builds the info text into IUI_INFO_TEXT (padded cells), IUI_INFO_ROLE and
# IUI_INFO_TAG, index-parallel. IUI_INFO_TAG marks the two action lines so a
# mouse click can be mapped back to an action after the frame is drawn.
# The pane's head: name, one-line summary, and the rule above them.
iui_info_head() {
    local width="$1" index="$2" line
    iui_repeat "$IUI_G_RULE" "$width"
    IUI_INFO_TEXT+=("$IUI_REPEAT"); IUI_INFO_ROLE+=(dirt); IUI_INFO_TAG+=(rule)
    iui_info_push gold "${IUI_SKILL_NAMES[$index]}" name "$width"
    iui_wrap "${IUI_SKILL_DESCS[$index]}" "$width"
    for line in "${IUI_WRAP_LINES[@]}"; do
        iui_info_push gold "$line" body "$width"
    done
    iui_info_push stone '' body "$width"
}

# Dependencies and status: the rows a reader decides on, so they come before the
# prose and stay on screen when it scrolls off.
iui_info_status() {
    local width="$1" index="$2" line state up
    iui_info_push diamond 'DEPENDENCIES' body "$width"
    iui_info_requirements "$index" "$width"
    iui_info_push diamond 'STATUS' body "$width"
    iui_skill_state "$index"
    printf -v line '  %-14s %s' 'install' "$(iui_install_verdict)"
    iui_info_push "$(iui_state_role "$IUI_SKILL_STATE")" "$line" body "$width"
    printf -v line '  %-14s %s' 'installed' "${IUI_SKILL_INSTALLED[$index]}"
    iui_info_push stone "$line" body "$width"
    up="$(iui_uptodate_text "$index")"
    printf -v line '  %-14s %s' 'latest release' "$up"
    # Red is for something wrong. An earlier version painted this row redstone
    # whenever the answer was not literally "yes", so a skill that is simply not
    # installed yet read as a warning. An available update is an invitation.
    case "$up" in
        yes) state=diamond ;;
        'no ('*) state=gold ;;
        *) state=stone ;;
    esac
    iui_info_push "$state" "$line" body "$width"
}

# The README-style body: one source paragraph per line, each wrapped. A skill
# with no detail recorded degrades to its summary rather than failing, which is
# what an index-parallel array does when someone extends only one of them.
iui_info_detail() {
    local width="$1" index="$2" paragraph line
    [ "$index" -lt "${#IUI_SKILL_DETAILS[@]}" ] || return 0
    iui_info_push stone '' body "$width"
    while IFS= read -r paragraph; do
        [ -n "$paragraph" ] || continue
        iui_wrap "$paragraph" "$width"
        for line in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push stone "$line" body "$width"
        done
    done <<< "${IUI_SKILL_DETAILS[$index]}"
}

iui_info_lines() {
    local width="$1" index="$IUI_CURSOR"
    IUI_INFO_TEXT=(); IUI_INFO_ROLE=(); IUI_INFO_TAG=()
    iui_info_head "$width" "$index"
    iui_info_status "$width" "$index"
    iui_info_detail "$width" "$index"
    iui_info_actions "$width"
    iui_info_message "$width"
}

# Tool, strength and state on one line; for an unmet requirement the next line
# says what is lost, which is the part that matters for a soft one.
iui_info_requirements() {
    local index="$1" width="$2" i line role any=0 wrapped
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        any=1
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        role=stone
        [ "$IUI_DEP_STATE" = "ok" ] && role=diamond
        if [ "$IUI_DEP_STATE" = "missing" ]; then
            role=gold
            [ "${IUI_REQ_STRENGTH[$i]}" = "hard" ] && role=redstone
        fi
        printf -v line '  %-14s %-5s %s' \
            "$(runtime_requirement_label "${IUI_REQ_TOOL[$i]}")" "${IUI_REQ_STRENGTH[$i]}" "$IUI_DEP_STATE"
        iui_info_push "$role" "$line" body "$width"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        iui_wrap "${IUI_REQ_WHY[$i]}" $((width - 4))
        for wrapped in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push "$role" "    $wrapped" body "$width"
        done
    done
    [ "$any" -eq 1 ] || iui_info_push stone '  (none)' body "$width"
}

iui_state_role() {
    case "$1" in
        blocked) printf 'redstone' ;;
        degraded) printf 'gold' ;;
        *) printf 'diamond' ;;
    esac
}

iui_install_verdict() {
    case "$IUI_SKILL_STATE" in
        blocked) printf 'blocked (%s missing)' "$IUI_SKILL_BLOCKER" ;;
        degraded) printf 'allowed, degraded (%s missing)' "$IUI_SKILL_BLOCKER" ;;
        *) printf 'allowed' ;;
    esac
}

# The wanted version is SOURCE_VERSION, which download_source() only sets after
# the selection is made, so during the picker it is legitimately empty.
# Answers "is a newer release published", by comparing the release this copy was
# installed from against master's package.json. The earlier version compared the
# installed marker against the local checkout's own commit, which under
# `curl … | bash` is not known yet -- hence its "unknown until the source is
# fetched", the commonest answer and the least useful one.
iui_uptodate_text() {
    local index="$1" have
    if [ "${IUI_SKILL_INSTALLED[$index]}" != "yes" ]; then
        printf 'not installed'
        return
    fi
    iui_release_version
    have="${IUI_SKILL_HAVE_PKG[$index]:-}"
    if [ -z "$IUI_RELEASE_VERSION" ]; then
        # Offline, no curl, or a fetch that failed. Never guess a version.
        printf 'unknown (could not reach the release)'
    elif [ -z "$have" ]; then
        printf 'unknown (installed copy records no release)'
    elif [ "$have" = "$IUI_RELEASE_VERSION" ]; then
        printf 'yes'
    else
        printf 'no (%s -> %s)' "$have" "$IUI_RELEASE_VERSION"
    fi
}

iui_info_push() {
    local role="$1" text="$2" tag="$3" width="$4"
    iui_pad "$text" "$width"
    IUI_INFO_TEXT+=("$IUI_PAD"); IUI_INFO_ROLE+=("$role"); IUI_INFO_TAG+=("$tag")
}

# The actions are always listed; they are only *usable* when the info pane has
# focus, and the leading marker says which state they are in.
iui_info_actions() {
    local width="$1" marker role
    if [ "$IUI_FOCUS" = "info" ]; then marker='>'; role=gold; else marker='-'; role=stone; fi
    iui_info_push diamond 'ACTIONS' body "$width"
    iui_info_push "$role" " $marker d  help me install dependencies" act-dep "$width"
    iui_info_push "$role" " $marker r  reverify dependencies" act-verify "$width"
}

iui_info_message() {
    local width="$1" line wrapped
    [ "${#IUI_MESSAGE[@]}" -gt 0 ] || return 0
    for line in "${IUI_MESSAGE[@]}"; do
        iui_wrap "$line" "$width"
        for wrapped in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push gold "$wrapped" body "$width"
        done
    done
}

# What a reader wants from the title is the state of the machine, not the state
# of the checkboxes: how many skills are installed, and how many of those have a
# newer release. "9/9 selected" said only that everything installable was ticked,
# which is the default and so carries no information.
iui_title_bar() {
    local text total="${#IUI_SKILL_NAMES[@]}"
    iui_count_states
    if [ "$IUI_COUNT_UPGRADE" -gt 0 ]; then
        printf -v text ' %s  %d/%d installed  %d upgradeable  %d selected ' \
            'TSCHALLACKA SKILL SHOP' "$IUI_COUNT_INSTALLED" "$total" \
            "$IUI_COUNT_UPGRADE" "$(iui_selected_count)"
    else
        printf -v text ' %s  %d/%d installed  %d selected ' \
            'TSCHALLACKA SKILL SHOP' "$IUI_COUNT_INSTALLED" "$total" \
            "$(iui_selected_count)"
    fi
    iui_pad "$text" "$IUI_COLS"
    iui_seg gold "$IUI_PAD"
    iui_out_line 1 "$IUI_SEG"
}

# Installed, and how many of those are behind the published release. Upgradeable
# is only ever counted when the release is known: offline it stays 0 and the
# title says nothing about upgrades rather than implying everything is current.
iui_count_states() {
    local i
    IUI_COUNT_INSTALLED=0
    IUI_COUNT_UPGRADE=0
    iui_release_version
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        [ "${IUI_SKILL_INSTALLED[$i]}" = yes ] || continue
        IUI_COUNT_INSTALLED=$((IUI_COUNT_INSTALLED + 1))
        [ -n "$IUI_RELEASE_VERSION" ] || continue
        [ -n "${IUI_SKILL_HAVE_PKG[$i]:-}" ] || continue
        [ "${IUI_SKILL_HAVE_PKG[$i]}" != "$IUI_RELEASE_VERSION" ] || continue
        IUI_COUNT_UPGRADE=$((IUI_COUNT_UPGRADE + 1))
    done
}

iui_hint_bar() {
    iui_pad ' Up/Dn move  Enter/Space toggle  click toggle  Tab focus  a all  n none  i install  q quit' "$IUI_COLS"
    iui_seg stone "$IUI_PAD"
    iui_out_line "$IUI_ROWS" "$IUI_SEG"
}

iui_selected_count() {
    local i n=0
    for ((i = 0; i < ${#IUI_SKILL_SEL[@]}; i++)); do
        [ "${IUI_SKILL_SEL[$i]}" -eq 1 ] && n=$((n + 1))
    done
    printf '%d' "$n"
}

iui_render_frame() {
    iui_layout
    iui_clamp_scroll
    IUI_ACTION_ROW_DEP=0
    IUI_ACTION_ROW_VERIFY=0
    [ "$IUI_POSITION" -eq 1 ] && printf '\033[H'
    iui_title_bar
    if [ "$IUI_NARROW" -eq 1 ]; then
        iui_render_narrow
    else
        iui_render_wide
    fi
    iui_hint_bar
    [ "$IUI_POSITION" -eq 1 ] && printf '\033[%d;1H' "$IUI_ROWS"
    return 0
}

iui_top_border() {
    local left right tl td tr focus_list=0 focus_info=0
    [ "$IUI_FOCUS" = "list" ] && focus_list=1
    [ "$IUI_FOCUS" = "info" ] && focus_info=1
    iui_header_seg SKILLS "$IUI_LEFT_W" "$focus_list"
    left="$IUI_HEADER_SEG"
    iui_header_seg DETAILS "$IUI_RIGHT_W" "$focus_info"
    right="$IUI_HEADER_SEG"
    iui_seg dirt "$IUI_G_TL"; tl="$IUI_SEG"
    iui_seg dirt "$IUI_G_TJ"; td="$IUI_SEG"
    iui_seg dirt "$IUI_G_TR"; tr="$IUI_SEG"
    iui_out_line 2 "$tl$left$td$right$tr"
}

# The divider gets a junction here too, so the frame closes on the same column
# the body rows divide at rather than running a plain line under it.
iui_bottom_border() {
    local bl bj br left right
    iui_seg dirt "$IUI_G_BL"; bl="$IUI_SEG"
    iui_repeat "$IUI_G_BOT" "$IUI_LEFT_W"
    iui_seg dirt "$IUI_REPEAT"; left="$IUI_SEG"
    iui_seg dirt "$IUI_G_BJ"; bj="$IUI_SEG"
    iui_repeat "$IUI_G_BOT" "$IUI_RIGHT_W"
    iui_seg dirt "$IUI_REPEAT"; right="$IUI_SEG"
    iui_seg dirt "$IUI_G_BR"; br="$IUI_SEG"
    iui_out_line $((IUI_ROWS - 1)) "$bl$left$bj$right$br"
}

# The right cell is the sprite for its first IUI_HEAD_H rows and info text after
# that, so the head and the text share one row budget and neither can overflow.
iui_right_cell() {
    local body="$1" row="$2"
    if [ "$IUI_HEAD_PLACE" = right-top ] && [ "$body" -lt "$IUI_HEAD_H" ]; then
        iui_head_beside_hint "$body"
        return
    fi
    local offset=0
    [ "$IUI_HEAD_PLACE" != right-top ] || offset="$IUI_HEAD_H"
    iui_info_cell $((body - offset + IUI_INFO_SCROLL)) "$row"
}

# One band row: the sprite, then the hint that belongs beside it. The space to
# the right of the sprite was blank before -- on a wide terminal that is most of
# the pane -- so the hints cost no rows that content was using.
iui_head_beside_hint() {
    local body="$1" cols slot glyph_row at text=''
    iui_head_line $((body / IUI_HEAD_SCALE)) "$IUI_HEAD_SCALE" \
        $((IUI_HEAD_SCALE * 32)) "$IUI_EYE"
    cols=$((IUI_RIGHT_W - IUI_HEAD_SCALE * 32))
    slot=$((body / 8))
    glyph_row=$((body % 8))
    if [ "$glyph_row" -lt 7 ]; then
        at=$(( (IUI_HINT_PAGE * IUI_HINT_PER_PAGE + slot) % ${#IUI_HINTS[@]} ))
        text="${IUI_HINTS[$at]}"
        iui_big_line " $text" "$glyph_row" "$cols"
    else
        iui_pad '' "$cols"
        IUI_BIG_LINE="$IUI_PAD"
    fi
    iui_seg gold "$IUI_BIG_LINE"
    IUI_INFO_CELL="$IUI_HEAD_LINE$IUI_SEG"
}

# Hints for the keys a reader would otherwise have to guess at. Shown only on a
# tall terminal, beside nothing and above the sprite, so a small screen keeps the
# sprite alone in its block. Kept to one line each and phrased as actions.
# Kept inside the pane width on purpose: a hint that truncates with a tilde is
# worse than no hint. Installing over an existing copy is how an update happens,
# so the install line says both rather than implying a separate command.
IUI_HINTS=(
    'PRESS A: ALL'
    'PRESS I: INSTALL'
    'PRESS N: NONE'
    'PRESS D: FIX DEPS'
    'TAB: DETAILS'
    'CLICK: TOGGLE'
)
# Two hints fit beside a 16-row sprite at seven pixel rows each plus a gap, so
# the rest cycle. IUI_HINT_DIRTY tells the event loop that a tick changed more
# than the eyes and the whole frame has to be repainted.
IUI_HINT_PER_PAGE=2
IUI_HINT_PAGE=0
IUI_HINT_TICKS=0
IUI_HINT_TICKS_PER_PAGE=4
IUI_HINT_DIRTY=0
iui_advance_hints() {
    [ "$IUI_HINT_ROWS" -gt 0 ] || return 0
    [ "${#IUI_HINTS[@]}" -gt "$IUI_HINT_PER_PAGE" ] || return 0
    IUI_HINT_TICKS=$((IUI_HINT_TICKS + 1))
    [ "$IUI_HINT_TICKS" -ge "$IUI_HINT_TICKS_PER_PAGE" ] || return 0
    IUI_HINT_TICKS=0
    IUI_HINT_PAGE=$((IUI_HINT_PAGE + 1))
    IUI_HINT_DIRTY=1
}
iui_hint_cell() {
    local at="$1" text=''
    [ "$at" -lt "${#IUI_HINTS[@]}" ] && text=" ${IUI_HINTS[$at]}"
    iui_pad "$text" "$IUI_LEFT_W"
    iui_seg slate "$IUI_PAD"
    IUI_LIST_CELL="$IUI_SEG"
}

# The left pane, top to bottom: the skill list, a separator, optional hint rows,
# then the sprite pinned to the bottom in its own block.
iui_left_cell() {
    local body="$1" index hint_at
    if [ "$body" -lt "$IUI_LIST_ROWS" ]; then
        index=$((IUI_SCROLL + body))
        if [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ]; then
            iui_list_cell "$index" "$IUI_LEFT_W"
        else
            iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        fi
        return
    fi
    if [ "$IUI_HEAD_PLACE" != left-bottom ]; then
        iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        return
    fi
    if [ "$body" -eq "$IUI_LIST_ROWS" ]; then
        iui_repeat "$IUI_G_RULE" "$IUI_LEFT_W"
        iui_seg dirt "$IUI_REPEAT"; IUI_LIST_CELL="$IUI_SEG"
        return
    fi
    hint_at=$((body - IUI_LIST_ROWS - 1))
    if [ "$hint_at" -lt "$IUI_HINT_ROWS" ]; then
        iui_hint_cell "$hint_at"
        return
    fi
    iui_head_line $(( (body - IUI_LIST_ROWS - 1 - IUI_HINT_ROWS) / IUI_HEAD_SCALE )) \
        "$IUI_HEAD_SCALE" "$IUI_LEFT_W" "$IUI_EYE"
    IUI_LIST_CELL="$IUI_HEAD_LINE"
}

iui_render_wide() {
    local row body index v
    iui_top_border
    iui_seg dirt "$IUI_G_V"; v="$IUI_SEG"
    iui_info_lines "$IUI_RIGHT_W"
    iui_clamp_info_scroll
    for ((body = 0; body < IUI_BODY_ROWS; body++)); do
        row=$((body + 3))
        iui_left_cell "$body"
        iui_right_cell "$body" "$row"
        iui_out_line "$row" "$v$IUI_LIST_CELL$v$IUI_INFO_CELL$v"
    done
    iui_bottom_border
}

iui_info_cell() {
    local i="$1" row="$2"
    if [ "$i" -lt 0 ] || [ "$i" -ge "${#IUI_INFO_TEXT[@]}" ]; then
        iui_pad '' "$IUI_RIGHT_W"
        iui_seg stone "$IUI_PAD"
        IUI_INFO_CELL="$IUI_SEG"
        return
    fi
    case "${IUI_INFO_TAG[$i]}" in
        act-dep) IUI_ACTION_ROW_DEP="$row" ;;
        act-verify) IUI_ACTION_ROW_VERIFY="$row" ;;
    esac
    iui_seg "${IUI_INFO_ROLE[$i]}" "${IUI_INFO_TEXT[$i]}"
    IUI_INFO_CELL="$IUI_SEG"
}

# Degrade path for a terminal too narrow for two panes: one pane at a time,
# chosen by focus, in the same row budget, and no sprite. Nothing overflows and
# nothing is half-drawn — a plain box beats a broken one.
iui_render_narrow() {
    local row body index label v
    label=SKILLS
    [ "$IUI_FOCUS" = "info" ] && label=DETAILS
    iui_header_seg "$label" "$IUI_LEFT_W" 1
    iui_seg dirt "$IUI_G_TL"; local tl="$IUI_SEG"
    iui_seg dirt "$IUI_G_TR"; local tr="$IUI_SEG"
    iui_out_line 2 "$tl$IUI_HEADER_SEG$tr"
    iui_seg dirt "$IUI_G_V"; v="$IUI_SEG"
    IUI_RIGHT_W="$IUI_LEFT_W"
    [ "$IUI_FOCUS" = "info" ] && iui_info_lines "$IUI_LEFT_W"
    for ((body = 0; body < IUI_BODY_ROWS; body++)); do
        row=$((body + 3))
        index=$((IUI_SCROLL + body))
        if [ "$IUI_FOCUS" = "info" ]; then
            iui_info_cell "$body" "$row"
            iui_out_line "$row" "$v$IUI_INFO_CELL$v"
            continue
        fi
        if [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ]; then
            iui_list_cell "$index" "$IUI_LEFT_W"
        else
            iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        fi
        iui_out_line "$row" "$v$IUI_LIST_CELL$v"
    done
    iui_repeat "$IUI_G_BOT" "$IUI_LEFT_W"
    iui_seg dirt "$IUI_G_BL$IUI_REPEAT$IUI_G_BR"
    iui_out_line $((IUI_ROWS - 1)) "$IUI_SEG"
}

# ---------------------------------------------------------------
# 6d. Full-screen skill picker: input, the terminal, and the seam
# ---------------------------------------------------------------
# Reads fd 3 — which section 3 opened, so this part must stay below it — runs the
# event loop, and exposes iui_select_skills(), the one function section 7 calls.
#
# Banners in this part, in order:
#   Input — keys and SGR mouse, read from fd 3
#   Terminal enter/leave
#   Event loop
#   The installer seam

# ─────────────────────────────────────────────────────────────────────────────
# Input — keys and SGR mouse, read from fd 3
# ─────────────────────────────────────────────────────────────────────────────

# Keys come from fd 3 (open on /dev/tty, so prompts survive `curl … | bash`) and
# escapes go to stdout. A bare ESC blocks for one more byte; `q` is the quit key.
# The continuation bytes of an escape sequence, with a timeout, because a bare
# Escape has none. Without it `read` blocked forever waiting for a second byte
# that a lone Escape never sends: the picker hung in raw mode on the alternate
# screen with the cursor hidden, so the terminal came back unusable and the only
# way out was to kill the shell.
#
# One second rather than the ~100ms a terminal usually allows: bash 3.2, the
# floor, refuses a fractional `read -t` outright. An arrow key's bytes arrive at
# once and pay nothing; only a lone Escape waits, and it waits once.
iui_read_byte() {
    IUI_BYTE=''
    IFS= read -r -n 1 -t 1 -u 3 IUI_BYTE || return 1
    [ -n "$IUI_BYTE" ] || IUI_BYTE=$'\n'
    return 0
}

# Returns 2 on timeout (an idle animation tick) and 1 on EOF (quit).
# PORTABILITY(read-timeout-integer): 1s is the finest tick available here, so the
# eyes blink on whole seconds and ESC cannot be disambiguated by timing.
iui_read_first_byte() {
    local rc=0
    IUI_BYTE=''
    IFS= read -r -n 1 -t 1 -u 3 IUI_BYTE || rc="$?"
    [ "$rc" -le 128 ] || return 2
    [ "$rc" -eq 0 ] || return 1
    [ -n "$IUI_BYTE" ] || IUI_BYTE=$'\n'
    return 0
}

# ---- quoted: SGR mouse report ----
# \033[<{btn};{col};{row}M   press
# \033[<{btn};{col};{row}m   release
# ---- end quoted ----
# Wheel (btn 64/65) and drag (btn with the 32 bit set) are parsed and then
# ignored — nothing in this UI should answer a wheel or a drag.
iui_read_mouse() {
    local buf='' rest
    IUI_MOUSE_RELEASE=0
    while iui_read_byte; do
        case "$IUI_BYTE" in
            M) break ;;
            m) IUI_MOUSE_RELEASE=1; break ;;
            *) buf="$buf$IUI_BYTE" ;;
        esac
        [ "${#buf}" -lt 24 ] || break
    done
    IUI_MOUSE_BTN="${buf%%;*}"
    rest="${buf#*;}"
    IUI_MOUSE_COL="${rest%%;*}"
    IUI_MOUSE_ROW="${rest##*;}"
    case "$IUI_MOUSE_BTN" in ''|*[!0-9]*) IUI_MOUSE_BTN=99 ;; esac
    case "$IUI_MOUSE_COL" in ''|*[!0-9]*) IUI_MOUSE_COL=0 ;; esac
    case "$IUI_MOUSE_ROW" in ''|*[!0-9]*) IUI_MOUSE_ROW=0 ;; esac
}

iui_read_key() {
    local rc=0
    iui_read_first_byte || rc="$?"
    if [ "$rc" -eq 2 ]; then IUI_KEY=TICK; return 0; fi
    if [ "$rc" -ne 0 ]; then IUI_KEY=EOF; return 0; fi
    case "$IUI_BYTE" in
        $'\033') iui_read_escape ;;
        $'\n'|$'\r') IUI_KEY=ENTER ;;
        ' ') IUI_KEY=SPACE ;;
        $'\003') IUI_KEY=q ;;
        *) IUI_KEY="$IUI_BYTE" ;;
    esac
    return 0
}

iui_read_escape() {
    iui_read_byte || { IUI_KEY=ESC; return 0; }
    case "$IUI_BYTE" in
        '[') : ;;
        'O') iui_read_byte || { IUI_KEY=ESC; return 0; }; iui_key_from_final; return 0 ;;
        *) IUI_KEY=ESC; return 0 ;;
    esac
    iui_read_byte || { IUI_KEY=ESC; return 0; }
    case "$IUI_BYTE" in
        '<') iui_read_mouse; IUI_KEY=MOUSE ;;
        [0-9]) iui_read_tilde "$IUI_BYTE" ;;
        *) iui_key_from_final ;;
    esac
    return 0
}

iui_key_from_final() {
    case "$IUI_BYTE" in
        A) IUI_KEY=UP ;;
        B) IUI_KEY=DOWN ;;
        C) IUI_KEY=RIGHT ;;
        D) IUI_KEY=LEFT ;;
        H) IUI_KEY=HOME ;;
        F) IUI_KEY=END ;;
        Z) IUI_KEY=SHIFTTAB ;;
        *) IUI_KEY=UNKNOWN ;;
    esac
}

iui_read_tilde() {
    local digits="$1"
    while iui_read_byte; do
        case "$IUI_BYTE" in
            [0-9]|';') digits="$digits$IUI_BYTE" ;;
            *) break ;;
        esac
        [ "${#digits}" -lt 12 ] || break
    done
    case "$digits" in
        1|7) IUI_KEY=HOME ;;
        4|8) IUI_KEY=END ;;
        5) IUI_KEY=PGUP ;;
        6) IUI_KEY=PGDN ;;
        *) IUI_KEY=UNKNOWN ;;
    esac
}

# ─────────────────────────────────────────────────────────────────────────────
# Terminal enter/leave
# ─────────────────────────────────────────────────────────────────────────────

# Install the trap before the first escape byte is written; iui_term_leave() is
# idempotent. IUI_NO_STTY=1 skips the stty calls so a test can drive this without
# owning a tty.
iui_term_enter() {
    IUI_STTY_SAVED=""
    if [ "${IUI_NO_STTY:-0}" -ne 1 ]; then
        IUI_STTY_SAVED="$(stty -g <&3 2>/dev/null || true)"
        stty raw -echo <&3 2>/dev/null || true
    fi
    IUI_TERM_ACTIVE=1
    # ---- quoted: enter sequences ----
    # \033[?1049h  alternate screen
    # \033[?25l    hide cursor
    # \033[?1000h  mouse tracking
    # \033[?1006h  SGR mouse encoding
    # ---- end quoted ----
    printf '\033[?1049h\033[?25l\033[?1000h\033[?1006h\033[2J\033[H'
}

iui_term_leave() {
    [ "$IUI_TERM_ACTIVE" -eq 1 ] || return 0
    IUI_TERM_ACTIVE=0
    # Every enable from iui_term_enter, disabled in reverse order.
    printf '\033[?1006l\033[?1000l\033[?25h\033[?1049l'
    # The drain and the restore are one block on purpose. Split, the drain could
    # leave the terminal non-canonical with `min 0 time 0` while the restore was
    # skipped because no state had been saved -- the shell then came back with an
    # unusable tty, which is worse than the stray mouse bytes being drained.
    if [ -n "$IUI_STTY_SAVED" ] && [ "${IUI_NO_STTY:-0}" -ne 1 ]; then
        iui_drain_input
        stty "$IUI_STTY_SAVED" <&3 2>/dev/null || true
    fi
}

# Discard input the terminal queued but nobody read -- typically an SGR mouse
# report, which the shell then prints as `[<0;79;20M` at its next prompt.
# Disabling mouse mode does not help: those bytes are already in the tty queue.
#
# Every read here carries a timeout, and that is not belt-and-braces. The first
# version used `stty min 0 time 0` and then an untimed `read -n 256`, reasoning
# that non-canonical mode makes read(2) return immediately. When the stty did not
# take effect the read blocked for 256 bytes that never came -- after the restore
# sequences had already been emitted, so the terminal was left in raw mode with
# no prompt and no way out but killing the shell. A cosmetic tidy-up must never
# be able to do that.
#
# PORTABILITY(read-timeout-floor): the timeout is a whole second because bash 3.2
# refuses a fractional one. One second on exit is acceptable; the loop is gone so
# it is paid at most once.
iui_drain_input() {
    local discard
    [ -n "$IUI_STTY_SAVED" ] || return 0
    stty min 0 time 0 <&3 2>/dev/null || return 0
    IFS= read -r -t 1 -n 256 discard <&3 2>/dev/null || true
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# Event loop
# ─────────────────────────────────────────────────────────────────────────────

# Selecting a blocked skill is refused with the reason rather than allowed and
# then rejected by the installer. Deselecting is always allowed.
iui_toggle() {
    local index="$1"
    [ "$index" -ge 0 ] && [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ] || return 0
    if [ "${IUI_SKILL_SEL[$index]}" -eq 1 ]; then
        IUI_SKILL_SEL[$index]=0
        return 0
    fi
    iui_skill_state "$index"
    if [ "$IUI_SKILL_STATE" = "blocked" ]; then
        IUI_MESSAGE=("cannot select ${IUI_SKILL_NAMES[$index]}: $IUI_SKILL_BLOCKER is required and missing")
        return 0
    fi
    IUI_SKILL_SEL[$index]=1
}

iui_action_dep_hint() {
    local index="$IUI_CURSOR" i any=0 line
    IUI_MESSAGE=('HOW TO INSTALL THE MISSING DEPENDENCIES')
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        any=1
        IUI_MESSAGE+=("$(runtime_requirement_label "${IUI_REQ_TOOL[$i]}") (${IUI_REQ_STRENGTH[$i]}): ${IUI_REQ_WHY[$i]}")
        # Section 4's generated hint table, so the picker offers the same
        # instruction verify_runtime_tools() prints on the non-interactive path.
        while IFS= read -r line; do
            [ -n "$line" ] || continue
            IUI_MESSAGE+=("$line")
        done < <(runtime_requirement_install_hint "${IUI_REQ_TOOL[$i]}")
    done
    [ "$any" -eq 1 ] || IUI_MESSAGE+=('nothing missing for this skill')
}

iui_action_reverify() {
    iui_dep_reverify_skill "$IUI_CURSOR"
    IUI_MESSAGE=('reverified; the per-tool cache is shared by every skill')
}

iui_handle_mouse() {
    [ "$IUI_MOUSE_RELEASE" -eq 0 ] || return 0
    case "$IUI_MOUSE_BTN" in
        64|65) return 0 ;;
    esac
    [ "$IUI_MOUSE_BTN" -lt 32 ] || return 0
    local first=3 last=$((IUI_BODY_ROWS + 2)) index
    [ "$IUI_MOUSE_ROW" -ge "$first" ] && [ "$IUI_MOUSE_ROW" -le "$last" ] || return 0
    if [ "$IUI_NARROW" -eq 0 ] && [ "$IUI_MOUSE_COL" -gt $((IUI_LEFT_W + 1)) ]; then
        IUI_FOCUS=info
        [ "$IUI_MOUSE_ROW" -eq "$IUI_ACTION_ROW_DEP" ] && iui_action_dep_hint
        [ "$IUI_MOUSE_ROW" -eq "$IUI_ACTION_ROW_VERIFY" ] && iui_action_reverify
        return 0
    fi
    [ "$IUI_FOCUS" = "info" ] && return 0
    index=$((IUI_SCROLL + IUI_MOUSE_ROW - first))
    [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ] || return 0
    IUI_CURSOR="$index"
    iui_toggle "$index"
}

# Movement follows focus. Previously Up/Down always moved the skill cursor, so
# tabbing to the detail pane highlighted it and then scrolled the wrong thing --
# the detail text below the fold could not be reached at all.
iui_move() {
    local delta="$1" count="${#IUI_SKILL_NAMES[@]}"
    if [ "$IUI_FOCUS" = "info" ]; then
        IUI_INFO_SCROLL=$((IUI_INFO_SCROLL + delta))
        return 0
    fi
    IUI_CURSOR=$((IUI_CURSOR + delta))
    # A new skill has its own detail of its own length, so the old offset would
    # leave the pane scrolled into blank rows.
    IUI_INFO_SCROLL=0
    [ "$count" -ge 0 ] || return 0
}

iui_handle_key() {
    local count="${#IUI_SKILL_NAMES[@]}" i
    case "$1" in
        UP|k) iui_move -1 ;;
        DOWN|j) iui_move 1 ;;
        PGUP) iui_move -"$IUI_BODY_ROWS" ;;
        PGDN) iui_move "$IUI_BODY_ROWS" ;;
        HOME) if [ "$IUI_FOCUS" = "info" ]; then IUI_INFO_SCROLL=0; else IUI_CURSOR=0; IUI_INFO_SCROLL=0; fi ;;
        END) if [ "$IUI_FOCUS" = "info" ]; then IUI_INFO_SCROLL="$IUI_INFO_MAX_SCROLL"; else IUI_CURSOR=$((count - 1)); IUI_INFO_SCROLL=0; fi ;;
        ENTER|SPACE) iui_toggle "$IUI_CURSOR" ;;
        $'\t') if [ "$IUI_FOCUS" = "list" ]; then IUI_FOCUS=info; else IUI_FOCUS=list; fi
              IUI_INFO_SCROLL=0 ;;
        SHIFTTAB) if [ "$IUI_FOCUS" = "info" ]; then IUI_FOCUS=list; else IUI_FOCUS=info; fi ;;
        a) for ((i = 0; i < count; i++)); do
               IUI_SKILL_SEL[$i]=0
               iui_toggle "$i"
           done ;;
        n) for ((i = 0; i < count; i++)); do IUI_SKILL_SEL[$i]=0; done ;;
        d) [ "$IUI_FOCUS" = "info" ] && iui_action_dep_hint ;;
        r) [ "$IUI_FOCUS" = "info" ] && iui_action_reverify ;;
        i) IUI_DONE=1; IUI_RC=0 ;;
        q|ESC|EOF) IUI_DONE=1; IUI_RC=130 ;;
        MOUSE) iui_handle_mouse ;;
    esac
    return 0
}

# Returns 69 without touching the terminal when fd 3 is not a tty, so the caller
# can fall back to the plain menu. An idle tick redraws only the two eye rows; a
# keypress redraws the whole frame. The caller, not this, installs the traps.
iui_run() {
    [ -t 3 ] || return 69
    [ -n "$COLOR_MODE" ] || detect_color_mode
    iui_set_glyphs "$IUI_GLYPHS"
    iui_term_enter
    IUI_POSITION=1
    IUI_DONE=0
    IUI_RC=130
    local need_full=1
    while [ "$IUI_DONE" -eq 0 ]; do
        if [ "$need_full" -eq 1 ]; then
            iui_measure
            iui_render_frame
            need_full=0
        fi
        iui_read_key
        if [ "$IUI_KEY" = "TICK" ]; then
            iui_advance_eye
            iui_advance_hints
            # A hint page turn changes rows the eye redraw does not touch, so it
            # asks for a full frame instead of leaving half the band stale.
            if [ "$IUI_HINT_DIRTY" -eq 1 ]; then
                IUI_HINT_DIRTY=0
                need_full=1
            else
                iui_redraw_eyes
            fi
            continue
        fi
        iui_handle_key "$IUI_KEY"
        need_full=1
    done
    iui_term_leave
    IUI_POSITION=0
    return "$IUI_RC"
}

iui_selected_names() {
    local i
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        [ "${IUI_SKILL_SEL[$i]}" -eq 1 ] && printf '%s\n' "${IUI_SKILL_NAMES[$i]}"
    done
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# The installer seam
# ─────────────────────────────────────────────────────────────────────────────

# The source_version of an already-installed copy of one skill, into IUI_MARKER,
# or '' when no known root holds it. The target root is chosen after the skills
# are, so every candidate root is searched and the first marker found wins.
iui_installed_marker() {
    local skill="$1" root marker
    IUI_MARKER=''
    IUI_MARKER_PACKAGE=''
    for root in ${TARGET_SELECTION:+"$TARGET_SELECTION"} "${TARGET_PATHS[@]}"; do
        marker="$root/$skill/.version"
        [ -f "$marker" ] || continue
        IUI_MARKER="$(awk '/^source_version=/ {
            sub(/^source_version=/, "")
            print
            exit
        }' "$marker")"
        IUI_MARKER_PACKAGE="$(awk '/^package_version=/ {
            sub(/^package_version=/, "")
            print
            exit
        }' "$marker")"
        [ -n "$IUI_MARKER" ] || IUI_MARKER=unknown
        return 0
    done
}

# Section 1's registry into the picker's tables. Everything installable starts
# selected, which is what the numbered menu's default of "all" does; iui_toggle
# refuses a blocked skill, so the preselection cannot include one.
iui_load_installer_skills() {
    local i
    IUI_SKILL_NAMES=("${SKILL_NAMES[@]}")
    IUI_SKILL_DESCS=("${SKILL_DESCRIPTIONS[@]}")
    IUI_SKILL_DETAILS=("${SKILL_DETAILS[@]}")
    IUI_SKILL_INSTALLED=(); IUI_SKILL_HAVE=(); IUI_SKILL_WANT=(); IUI_SKILL_SEL=()
    IUI_SKILL_HAVE_PKG=()
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        iui_installed_marker "${IUI_SKILL_NAMES[$i]}"
        if [ -n "$IUI_MARKER" ]; then
            IUI_SKILL_INSTALLED+=(yes)
        else
            IUI_SKILL_INSTALLED+=(no)
        fi
        IUI_SKILL_HAVE+=("$IUI_MARKER")
        IUI_SKILL_HAVE_PKG+=("$IUI_MARKER_PACKAGE")
        IUI_SKILL_WANT+=("$SOURCE_VERSION")
        IUI_SKILL_SEL+=(0)
    done
    IUI_DEP_TOOLS=(); IUI_DEP_STATES=()
    iui_load_requirements
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        iui_dep_reverify_skill "$i"
        iui_toggle "$i"
    done
    IUI_CURSOR=0; IUI_SCROLL=0; IUI_FOCUS=list; IUI_MESSAGE=()
}

# Runs the picker and leaves the answer in SELECTED_SKILLS. Returns 69 when fd 3
# is not a tty, which is select_skills()'s signal to draw the numbered menu, and
# the picker's own code otherwise (130 when the user quit).
#
# Two EXIT traps cannot coexist and cleanup() already owns EXIT, so a second one
# would silently drop the temp-directory removal and the summary. EXIT therefore
# chains both for the lifetime of the picker and is put back afterwards.
iui_select_skills() {
    local rc=0 name
    iui_load_installer_skills
    trap 'iui_term_leave; cleanup' EXIT
    trap 'iui_term_leave; exit 130' INT
    trap 'iui_term_leave; exit 143' TERM
    iui_run || rc="$?"
    trap cleanup EXIT
    trap - INT
    trap - TERM
    [ "$rc" -ne 69 ] || return 69
    # show_splash() drew on the normal screen and iui_term_leave() just restored
    # it, so wipe the mascot before the install log starts writing over it.
    printf '\033[2J\033[H'
    [ "$rc" -eq 0 ] || return "$rc"
    SELECTED_SKILLS=()
    while IFS= read -r name; do
        [ -n "$name" ] || continue
        SELECTED_SKILLS+=("$name")
    done < <(iui_selected_names)
    return 0
}
# ---------------------------------------------------------------
# 7. Skill and target selection
# ---------------------------------------------------------------
# Resolves SKILL_SELECTION/TARGET_SELECTION (from the flags) or asks, and leaves
# the answers in SELECTED_SKILLS, SELECTED_TARGET_PATHS and SELECTED_TARGET_NAMES.
# Asking is the picker of sections 6b-6d, with section 6's numbered menu as the
# fallback whenever it declines with 69.
select_skills() {
    SELECTED_SKILLS=()

    if [ -z "$SKILL_SELECTION" ]; then
        # The picker of sections 6b-6d first. 69 is its "fd 3 is not a terminal"
        # answer and the numbered menu below is the fallback, so every non-tty
        # path behaves exactly as it did before the picker existed.
        local ui_rc=0
        iui_select_skills || ui_rc="$?"
        case "$ui_rc" in
            0)
                [ "${#SELECTED_SKILLS[@]}" -gt 0 ] \
                    || { echo "Nothing selected; nothing was installed." >&2; exit 0; }
                return
                ;;
            69) ;;
            *)
                echo "Aborted; nothing was installed." >&2
                exit "$ui_rc"
                ;;
        esac
        show_shop_menu
        printf '\033[%d;1H' "$MENU_PROMPT_ROW"
        ask "Choose 1-6 or enter comma-separated names [6]: "
        SKILL_SELECTION="${REPLY:-6}"
    fi

    if [ "$SKILL_SELECTION" = "all" ] || [ "$SKILL_SELECTION" = "6" ]; then
        SELECTED_SKILLS=("${SKILL_NAMES[@]}")
        return
    fi

    local choice name
    IFS=',' read -r -a choices <<< "$SKILL_SELECTION"
    for choice in "${choices[@]}"; do
        name="$choice"
        # A menu number selects by position in SKILL_NAMES. The patterns exclude
        # a leading zero so `08` cannot reach $(( )) and be read as octal; an
        # out-of-range number falls through to the name check and dies there.
        # `all` anywhere in the selection means every skill, so
        # `--skill all --skill todo` is not a contradiction to be resolved by
        # ordering. The bare "6" spelling stays whole-string only, above: with
        # seven skills, 6 is also a valid position, and reading it as "all" in a
        # list would silently install one thing when a list was asked for.
        case "$name" in
            all) SELECTED_SKILLS=("${SKILL_NAMES[@]}"); return ;;
        esac
        case "$name" in
            [1-9]|[1-9][0-9])
                if [ "$name" -le "${#SKILL_NAMES[@]}" ]; then
                    name="${SKILL_NAMES[$((name - 1))]}"
                fi
                ;;
        esac
        contains "$name" "${SKILL_NAMES[@]}" || die "Unknown skill: $name"
        contains "$name" ${SELECTED_SKILLS[@]+"${SELECTED_SKILLS[@]}"} \
            || SELECTED_SKILLS+=("$name")
    done
}

select_targets() {
    SELECTED_TARGET_PATHS=()
    SELECTED_TARGET_NAMES=()

    if [ -n "$TARGET_SELECTION" ]; then
        SELECTED_TARGET_PATHS=("$TARGET_SELECTION")
        SELECTED_TARGET_NAMES=("$TARGET_SELECTION")
        return
    fi

    AVAILABLE_TARGET_PATHS=()
    AVAILABLE_TARGET_NAMES=()
    local index path choice selection custom_choice
    load_custom_locations

    for index in "${!TARGET_PATHS[@]}"; do
        if agent_target_available "$index"; then
            AVAILABLE_TARGET_PATHS+=("${TARGET_PATHS[$index]}")
            AVAILABLE_TARGET_NAMES+=("${TARGET_NAMES[$index]}")
        fi
    done
    for path in ${SAVED_CUSTOM_LOCATIONS[@]+"${SAVED_CUSTOM_LOCATIONS[@]}"}; do
        AVAILABLE_TARGET_PATHS+=("$path")
        AVAILABLE_TARGET_NAMES+=("Custom: $path")
    done

    [ "${#AVAILABLE_TARGET_PATHS[@]}" -gt 0 ] || die "No installed agent roots or saved custom locations were found"

    echo >&2
    echo "Install into which skill root?" >&2
    for index in "${!AVAILABLE_TARGET_PATHS[@]}"; do
        path="${AVAILABLE_TARGET_PATHS[$index]}"
        if [ -d "$path" ]; then
            printf '  %d) %s: %s [exists]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        else
            printf '  %d) %s: %s [will create]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        fi
    done
    custom_choice=$(( ${#AVAILABLE_TARGET_PATHS[@]} + 1 ))
    echo "  $custom_choice) custom directory" >&2
    echo "  a) all listed roots" >&2
    # --yes exists to prevent a headless run stopping on a question; with no
    # interactive channel it takes the menu's own default, the first root.
    if [ "${YES:-0}" -eq 1 ] && ! [ -t 3 ]; then
        printf '%s: no interactive channel; using the first listed root\n' "${0##*/}" >&2
        selection="1"
    else
        ask "Choose 1-$custom_choice, comma-separated numbers, or a [1]: "
        selection="${REPLY:-1}"
    fi

    if [ "$selection" = "a" ] || [ "$selection" = "all" ]; then
        SELECTED_TARGET_PATHS=("${AVAILABLE_TARGET_PATHS[@]}")
        SELECTED_TARGET_NAMES=("${AVAILABLE_TARGET_NAMES[@]}")
        echo "Warning: multiple roots can make the same skill appear more than once." >&2
        return
    fi

    IFS=',' read -r -a choices <<< "$selection"
    for choice in "${choices[@]}"; do
        if [ "$choice" = "$custom_choice" ]; then
            ask "Custom skill root: "
            [ -n "$REPLY" ] || die "A custom directory is required"
            path="${REPLY/#\~/$HOME}"
            case "$path" in
                /*) ;;
                *) die "Custom directory must be an absolute path" ;;
            esac
            if [ ! -d "$path" ]; then
                if ! confirm "$path does not exist. Create it?"; then
                    die "Custom directory does not exist: $path"
                fi
                mkdir -p "$path"
            fi
            save_custom_location "$path"
            SELECTED_TARGET_PATHS+=("$path")
            SELECTED_TARGET_NAMES+=("Custom: $path")
        elif [ "$choice" -ge 1 ] 2>/dev/null && [ "$choice" -le "${#AVAILABLE_TARGET_PATHS[@]}" ]; then
            index=$((choice - 1))
            SELECTED_TARGET_PATHS+=("${AVAILABLE_TARGET_PATHS[$index]}")
            SELECTED_TARGET_NAMES+=("${AVAILABLE_TARGET_NAMES[$index]}")
        else
            die "Unknown target choice: $choice"
        fi
    done
}
# ---------------------------------------------------------------
# 8. Source acquisition
# ---------------------------------------------------------------
# Local checkout or downloaded tarball, decided solely by whether
# ${BASH_SOURCE[0]} is a readable file next to planning/SKILL.md — that is what
# makes the curl|bash form work, so do not replace the probe with $0 or a marker.
download_source() {
    local script_dir ref_prefix archive source_commit source_branch
    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [ -f "$script_dir/planning/SKILL.md" ]; then
            SOURCE_ROOT="$script_dir"
            source_commit="$(git -C "$SOURCE_ROOT" rev-parse --short=12 HEAD 2>/dev/null || true)"
            source_branch="$(git -C "$SOURCE_ROOT" symbolic-ref --short -q HEAD 2>/dev/null || true)"
            SOURCE_VERSION="$(git -C "$SOURCE_ROOT" describe --tags --exact-match HEAD 2>/dev/null || true)"
            if [ -n "$SOURCE_VERSION" ]; then
                SOURCE_VERSION="tag:$SOURCE_VERSION commit:${source_commit:-unknown}"
            else
                SOURCE_VERSION="branch:${source_branch:-detached} commit:${source_commit:-unknown}"
            fi
            return
        fi
    fi

    command -v curl >/dev/null 2>&1 || die "curl is required"
    command -v tar >/dev/null 2>&1 || die "tar is required"
    TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills.XXXXXX")"
    local archive="$TEMP_ROOT/source.tar.gz"
    if [[ "$REPO_REF" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        ref_prefix="refs/tags"
        SOURCE_VERSION="tag:$REPO_REF"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    elif [[ "$REPO_REF" =~ ^[0-9a-fA-F]{7,40}$ ]]; then
        SOURCE_VERSION="commit:$REPO_REF"
        local url="${REPO_URL%/}/archive/${REPO_REF}.tar.gz"
    else
        ref_prefix="refs/heads"
        source_commit=""
        if command -v git >/dev/null 2>&1; then
            source_commit="$(git ls-remote "$REPO_URL" "refs/heads/$REPO_REF" 2>/dev/null | awk 'NR == 1 {print $1}')"
        fi
        SOURCE_VERSION="branch:$REPO_REF commit:${source_commit:-unknown}"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    fi
    echo "Downloading skills from $url" >&2
    curl -fsSL "$url" -o "$archive"
    tar -xzf "$archive" -C "$TEMP_ROOT"
    SOURCE_ROOT="$(find "$TEMP_ROOT" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    [ -n "$SOURCE_ROOT" ] && [ -f "$SOURCE_ROOT/planning/SKILL.md" ] \
        || die "Downloaded archive does not contain the expected skills"
}

# ---------------------------------------------------------------
# 9. Per-skill file manifest
# ---------------------------------------------------------------
# The planning list is a deliberate second copy of planning/PACKAGE-MANIFEST.tsv:
# the two are diffed by planning/tests/test-installer-manifest.sh, which extracts
# this heredoc by regex. Do not restructure skill_files() or that heredoc, and do
# not derive it from the manifest — the duplication IS the cross-check.
# package_version is the released version from package.json, read with sed rather
# than jq: jq is a declared runtime dependency of some skills but the installer
# must run before any of them is installed. A register written by a skill records
# this value, so a reader can compare it against the installed skill and see that
# an upgrade happened.
version_marker_content() {
    local package_version=''
    if [ -f "$SOURCE_ROOT/package.json" ]; then
        package_version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
            "$SOURCE_ROOT/package.json" | head -1)"
    fi
    printf 'format=ai-skills-version-1\n'
    printf 'package_version=%s\n' "${package_version:-unknown}"
    printf 'source_version=%s\n' "$SOURCE_VERSION"
    printf 'source_ref=%s\n' "$REPO_REF"
}

# skill_files <skill> [package]
#
# prod (the default) is what an end user receives: the files whose header marks
# them for production. dev is inclusive -- prod plus the files only a maintainer
# needs, which is what a dev marking means. So the dev arm prints the prod list
# first and then adds to it, rather than repeating it.
#
# No line here may begin with a marker keyword: the generators strip such lines
# out of install.sh, and a comment shaped like a marker reads as one.
#
# Still a hand list, deliberately: the planning arms are a second copy of
# PACKAGE-MANIFEST.tsv and the duplication is the cross-check.
# tests/test-mode-markers.sh compares both arms against the markers in the files.
skill_files() {
    local package="${2:-prod}"
    case "$package" in
        prod|dev) ;;
        *) printf 'skill_files: unknown package: %s\n' "$package" >&2; return 64 ;;
    esac
    case "$1" in
        planning)
            cat <<'EOF'
SKILL.md
docs/README.md
REVIEWER.md
references/plan-read-contract.md
references/ui-user-story-validation.md
references/comment-discipline-contract.md
telemetry-schema.json
placeholders.json
gate-caps.json
state-change-registry.json
never-executable-extensions.json
goal-tables.json
artifact-comparisons.json
document-sections.json
context/brainstorm-limiting-context.md
context/brainstorm-limiting-context-contract.json
context/brainstorm-limiting-context-benchmark.json
context/brainstorm-limiting-context-oracle.json
PACKAGE-MANIFEST.tsv
requires.tsv
ROLES.md
MAINTAINER-STYLE-CONTRACT.md
templates/plan-overview.html.tmpl
roles/planning.md
roles/execution.md
roles/cleanup.md
roles/VOICES.md
scripts/add-coverage.sh
scripts/remove-coverage.sh
scripts/add-adversarial-finding.sh
scripts/add-fix-claim.sh
scripts/add-goal.sh
scripts/add-planning-bug.sh
scripts/add-ui-story.sh
scripts/add-ui-story-links.sh
scripts/update-ui-story.sh
scripts/add-work-unit.sh
scripts/configure-ui-story-cache.sh
scripts/create-adversarial-review.sh
scripts/create-plan-progress.sh
scripts/create-plan.sh
scripts/create-progress.sh
scripts/create-step-testing.sh
scripts/rebuild-plan-progress.sh
scripts/register-command.sh
scripts/register-read.sh
scripts/resolve-finding.sh
scripts/render-plan-overview.sh
scripts/create-ui-story-run-cache.sh
scripts/create-ui-validation.sh
scripts/create-work-unit-inventory.sh
scripts/plan-content.sh
scripts/overview-state.sh
scripts/overview-serve.sh
scripts/runtime/overview-server.py
scripts/runtime/overview-server.js
scripts/runtime/overview-server.pl
scripts/runtime/overview-serve-handler.sh
scripts/plan-content-diff-lib.sh
scripts/plan-context-lib.sh
scripts/plan-context.sh
scripts/plan-context-wrapper.sh
scripts/plan-env.sh
scripts/plan-mutate.sh
scripts/plan-root.sh
scripts/plan-reconcile-lib.sh
scripts/role-context.sh
scripts/monitor-read.sh
scripts/supervision-frame.sh
scripts/update-work-unit.sh
scripts/remove-work-unit.sh
scripts/plan-core-lib.sh
scripts/plan-progress-lib.sh
scripts/plan-table-lib.sh
scripts/plan-document-lib.sh
scripts/plan-map-lib.sh
scripts/plan-inventory-lib.sh
scripts/update-plan-content.sh
scripts/update-adversarial-review.sh
scripts/mint-fix-keys.sh
scripts/verify-fix-keys.sh
scripts/verify-target.sh
scripts/generate-reviewer.sh
scripts/update-plan-progress.sh
scripts/update-progress.sh
scripts/update-step.sh
scripts/validate-plan.sh
scripts/validate-plan-common-lib.sh
scripts/validate-plan-docs-lib.sh
scripts/validate-plan-placeholders-lib.sh
scripts/validate-plan-stale-lib.sh
scripts/validate-plan-inventory-lib.sh
scripts/validate-plan-ui-lib.sh
scripts/validate-plan-goals-lib.sh
scripts/validate-plan-serve-lib.sh
scripts/validate-plan-commands-lib.sh
scripts/validate-plan-propagation-lib.sh
scripts/validate-plan-comparisons-lib.sh
scripts/remove-plan.sh
scripts/cleanup-plans.sh
scripts/run-adversary-probe.sh
EOF
            [ "$package" = dev ] || return 0
            cat <<'EOF'
.gitignore
ARCHITECTURE.md
MAINTAINER.md
PACKAGE-MAP.tsv
scripts/build-plan-libs.sh
scripts/lib/core/00-state.sh
scripts/lib/core/plan_atomic_write.sh
scripts/lib/core/plan_awk_trim.sh
scripts/lib/core/plan_cleanup.sh
scripts/lib/core/plan_decode_escaped_newlines.sh
scripts/lib/core/plan_default_root.sh
scripts/lib/core/plan_die.sh
scripts/lib/core/plan_duplicate_step_numbers.sh
scripts/lib/core/plan_ensure_root_permissions.sh
scripts/lib/core/plan_fail.sh
scripts/lib/core/plan_git_snapshot.sh
scripts/lib/core/plan_hoist_plan_dir.sh
scripts/lib/core/plan_refuse_existing.sh
scripts/lib/core/plan_register_temp_file.sh
scripts/lib/core/plan_require_bash.sh
scripts/lib/core/plan_require_directory.sh
scripts/lib/core/plan_require_file.sh
scripts/lib/core/plan_require_safe_value.sh
scripts/lib/core/plan_resolve_symlink.sh
scripts/lib/core/plan_snapshot_repo.sh
scripts/lib/core/plan_stat_probe.sh
scripts/lib/core/plan_track_tmp.sh
scripts/lib/core/plan_warn.sh
scripts/lib/core/planning_ensure_tmpdir.sh
scripts/lib/core/planning_tmpdir.sh
scripts/lib/document/99-facade.sh
scripts/lib/document/plan_delete_paragraph.sh
scripts/lib/document/plan_document_kind.sh
scripts/lib/document/plan_document_path.sh
scripts/lib/document/plan_insert_paragraph.sh
scripts/lib/document/plan_missing_section_message.sh
scripts/lib/document/plan_refuse_field_section.sh
scripts/lib/document/plan_render_paragraphs.sh
scripts/lib/document/plan_replace_field.sh
scripts/lib/document/plan_replace_paragraph.sh
scripts/lib/document/plan_replace_section.sh
scripts/lib/document/plan_replace_title.sh
scripts/lib/document/plan_section_spec.sh
scripts/lib/document/plan_unknown_section.sh
scripts/lib/progress/plan_emit_step_testing_reminder.sh
scripts/lib/progress/plan_progress_bar.sh
scripts/lib/progress/plan_progress_icon.sh
scripts/lib/progress/plan_progress_percent.sh
scripts/lib/progress/plan_status_label.sh
scripts/lib/progress/plan_step_objective.sh
scripts/lib/table/plan_goal_definition_of_done.sh
scripts/lib/table/plan_render_csv_table.sh
scripts/lib/table/plan_replace_testing_requirement.sh
scripts/lib/table/plan_review_gated_pairs.sh
scripts/lib/table/plan_testing_requirement_for_goal.sh
scripts/lib/table/plan_testing_requirement_row.sh
tests/fixtures/adversary-probe/01-health-endpoint/goal.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/01-step-add-handler.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/02-step-add-test.md
tests/fixtures/adversary-probe/FIXTURE-VERSION
tests/fixtures/adversary-probe/README.md
tests/fixtures/adversary-probe/adversarial-review.md
tests/fixtures/adversary-probe/plan-description.md
tests/fixtures/adversary-probe/progress.md
tests/fixtures/adversary-probe/work-unit-inventory.md
tests/fixtures/context-cache-coupled.md
tests/fixtures/context-cache-medium.md
tests/fixtures/context-cache-small.md
tests/fixtures/planning-context/case-matrix.tsv
tests/fixtures/planning-context/expected-outcomes.jsonl
tests/fixtures/planning-context/platform-inputs.tsv
tests/fixtures/planning-context/runner-targets.discovery.txt
tests/fixtures/planning-context/runner-targets.tsv
tests/fixtures/planning-context/test-signing-key.pub
tests/fixtures/progress-shape-bad/01-goal-bad/goal.md
tests/fixtures/progress-shape-bad/01-goal-bad/progress.md
tests/fixtures/progress-shape-bad/01-goal-bad/steps/01-step-bad.md
tests/fixtures/progress-shape-bad/progress.md
tests/fixtures/progress-shape/01-goal-a/goal.md
tests/fixtures/progress-shape/01-goal-a/progress.md
tests/fixtures/progress-shape/01-goal-a/steps/01-step-a.md
tests/fixtures/progress-shape/02-goal-b/goal.md
tests/fixtures/progress-shape/02-goal-b/progress.md
tests/fixtures/progress-shape/02-goal-b/steps/01-step-b.md
tests/fixtures/progress-shape/02-goal-b/steps/02-step-b2.md
tests/fixtures/progress-shape/progress.md
tests/lib-test.sh
tests/test-add-fix-claim.sh
tests/test-add-planning-bug.sh
tests/test-add-work-unit-staging.sh
tests/test-adversarial-review-cycles.sh
tests/test-adversarial-review-sources.sh
tests/test-adversarial-review-mint-order.sh
tests/test-adversarial-review-preamble.sh
tests/test-adversary-probe-fixture.sh
tests/test-artifact-comparisons.sh
tests/test-blast-radius.sh
tests/test-comment-format.sh
tests/test-context-id-suggestions.sh
tests/test-context-json-control-chars.sh
tests/test-context-summary-excerpt.sh
tests/test-coverage-gaps.sh
tests/test-create-plan-explicit-root.sh
tests/test-csv-table-errors.sh
tests/test-die-temp-file-cleanup.sh
tests/test-discovery-unit-target.sh
tests/test-document-id-parity.sh
tests/test-document-sections.sh
tests/test-duplication-ratchet.sh
tests/test-fix-keys.sh
tests/test-flag-coverage.sh
tests/test-flag-form-equivalence.sh
tests/test-function-length-ratchet.sh
tests/test-goal-testing-row.sh
tests/test-inner-shell-consistency.sh
tests/test-install-ui.sh
tests/test-installer-any-of.sh
tests/test-installer-backups.sh
tests/test-installer-build.sh
tests/test-installer-dependencies.sh
tests/test-installer-manifest.sh
tests/test-installer-noninteractive.sh
tests/test-installer-opencode-permissions.sh
tests/test-installer-skill-selection.sh
tests/test-inventory-helpers.sh
tests/test-lib-core.sh
tests/test-lib-document.sh
tests/test-lib-progress.sh
tests/test-lib-table.sh
tests/test-limited-run-contract.sh
tests/test-mermaid-accuracy.sh
tests/test-obsolete-plan.sh
tests/test-plan-overview.sh
tests/test-persona-drift.sh
tests/test-plan-commands.sh
tests/test-plan-context-arguments.sh
tests/test-plan-context-deferred-boundary.sh
tests/test-plan-context-optional-inventory.sh
tests/test-plan-context-paging.sh
tests/test-plan-context-reviewer.sh
tests/test-plan-context-unit-entry.sh
tests/test-plan-context.sh
tests/test-plan-dir-synonym.sh
tests/test-owned-roster-scaffold.sh
tests/test-plan-env.sh
tests/test-plan-integrity-and-monitor.sh
tests/test-plan-libs-build.sh
tests/test-plan-root.sh
tests/test-plan-snapshot.sh
tests/test-planning-context-contract.sh
tests/test-portability-contract.sh
tests/test-portable-helpers.sh
tests/test-progress-bar-shape.sh
tests/test-progress-derivation.sh
tests/test-progress-entry-ids.sh
tests/test-progress-helpers.sh
tests/test-report17-regressions.sh
tests/test-report18-regressions.sh
tests/test-report20-regressions.sh
tests/test-reviewer-projection.sh
tests/test-register-helpers.sh
tests/test-register-read.sh
tests/test-resolve-finding.sh
tests/test-validate-gates.sh
tests/test-gate-caps.sh
tests/test-atomicity-flow.sh
tests/test-plan-data-lib.sh
tests/test-writer-hardening.sh
tests/test-overview-state.sh
tests/test-overview-serve.sh
scripts/register-lib.sh
scripts/todo-add.sh
scripts/todo-update.sh
scripts/bug-add.sh
scripts/bug-update.sh
scripts/register-rebuild.sh
tests/test-plan-freshness.sh
tests/test-roster-cross-reference.sh
tests/test-runtime-dependencies.sh
tests/test-self-hosted-plan.sh
tests/test-sha256-fallbacks.sh
tests/test-stale-sweep.sh
tests/test-step-atomicity-reset.sh
tests/test-step-testing-reminder.sh
tests/test-step-testing-sections.sh
tests/test-supervision-frame.sh
tests/test-target-path-validation.sh
tests/test-target-reachability-gate.sh
tests/test-ui-prohibition-scope.sh
tests/test-validation-readiness-summary.sh
tests/test-verifier-reach-memo.sh
tests/test-voice-artifact-drift.sh
EOF
            ;;
        project-specificies)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        resource-limited-testing)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            local file
            for file in "$SOURCE_ROOT/resource-limited-testing/scripts/"*.sh; do
                [ -f "$file" ] && printf '%s\n' "scripts/$(basename "$file")"
            done
            ;;
        brainstorm)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        git-worktrees)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        merge-request-etiquette)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        todo)
            printf '%s\n' SKILL.md docs/README.md requires.tsv schema.1.4.2.json
            ;;
        bug-report)
            printf '%s\n' SKILL.md docs/README.md requires.tsv schema.1.4.2.json
            ;;
        post-implementation-review)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        chat)
            cat <<'CHATEOF'
SKILL.md
docs/README.md
requires.tsv
scripts/chat-server.sh
scripts/chat-register.sh
scripts/chat-send.sh
scripts/chat-read.sh
scripts/chat-tail.sh
scripts/chat-watch.sh
scripts/chat-announce.sh
scripts/chat-discover.sh
runtime/server.py
runtime/server.js
runtime/server.pl
runtime/bash-handler.sh
CHATEOF
            [ "$package" = dev ] || return 0
            cat <<'CHATEOF'
tests/test-chat.sh
tests/test-chat-injection.sh
tests/test-chat-watch-cursor.sh
CHATEOF
            ;;
    esac
}

source_file() {
    local skill="$1"
    local relative="$2"
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$relative"
}
# ---------------------------------------------------------------
# 10. CLI-mode handlers
# ---------------------------------------------------------------
# Machine-facing entry points for the planning skill's own tooling. The exit
# codes of cli_install_skill are a contract: 2 = approval declined (nothing
# written), 3 = a collision that is not a managed-version upgrade, or a symlink.
cli_print_skill_files() {
    [ "$CLI_SKILL" = "planning" ] || die "unsupported CLI skill: $CLI_SKILL"
    [ "$CLI_FORMAT" = "tsv" ] || die "unsupported CLI format: $CLI_FORMAT"
    cat "$SOURCE_ROOT/planning/PACKAGE-MANIFEST.tsv"
}

cli_resolve_source() {
    [ "$CLI_SKILL" = "planning" ] || die "unsupported CLI skill: $CLI_SKILL"
    local source
    source="$(source_file "$CLI_SKILL" "$CLI_RELATIVE")"
    [ -f "$source" ] || die "source does not exist: $CLI_RELATIVE"
    printf '%s\n' "$source"
}

cli_install_skill() {
    contains "$CLI_SKILL" "${SKILL_NAMES[@]}" || die "unsupported CLI skill: $CLI_SKILL"
    verify_runtime_tools "$CLI_SKILL"
    [ -z "$RUNTIME_BLOCKED_SKILLS" ] \
        || die "$CLI_SKILL is missing a hard runtime requirement; nothing was written"
    case "$CLI_APPROVAL" in yes|no) ;; *) die "--approval must be yes or no" ;; esac
    local relative source destination_file collision=0 unsafe_collision=0 managed_version_transition=0
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$CLI_SKILL" "$relative")"
        [ -f "$source" ] || die "source does not exist: $relative"
        destination_file="$TARGET_SELECTION/$CLI_SKILL/$relative"
        if [ -e "$destination_file" ] || [ -L "$destination_file" ]; then
            printf 'Collision: %s\n' "$destination_file" >&2
            collision=1
            [ -L "$destination_file" ] && unsafe_collision=1
        fi
    done < <(skill_files "$CLI_SKILL" "$PACKAGE_SELECTION")
    if [ -e "$TARGET_SELECTION/$CLI_SKILL/.version" ] || [ -L "$TARGET_SELECTION/$CLI_SKILL/.version" ]; then
        printf 'Collision: %s\n' "$TARGET_SELECTION/$CLI_SKILL/.version" >&2
        collision=1
        [ -L "$TARGET_SELECTION/$CLI_SKILL/.version" ] && unsafe_collision=1
        if [ -f "$TARGET_SELECTION/$CLI_SKILL/.version" ] && ! cmp -s <(version_marker_content) "$TARGET_SELECTION/$CLI_SKILL/.version"; then
            managed_version_transition=1
        fi
    fi
    if [ "$collision" -ne 0 ] && { [ "$managed_version_transition" -eq 0 ] || [ "$unsafe_collision" -ne 0 ]; }; then
        return 3
    fi
    [ "$CLI_APPROVAL" = "yes" ] || { printf 'Approval declined; no files changed.\n' >&2; return 2; }
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$CLI_SKILL" "$relative")"
        destination_file="$TARGET_SELECTION/$CLI_SKILL/$relative"
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done < <(skill_files "$CLI_SKILL" "$PACKAGE_SELECTION")
    version_marker_content > "$TARGET_SELECTION/$CLI_SKILL/.version"
    printf 'Installed: %s/%s\n' "$TARGET_SELECTION" "$CLI_SKILL"
}

# ---------------------------------------------------------------
# 11. Interactive install, backup, and merge
# ---------------------------------------------------------------
# The interactive counterpart to section 10: compares every managed file against
# the source, then asks once per destination. A .version marker that differs is a
# managed upgrade. Either way a file is backed up unless its recorded digest
# proves we wrote it and nobody has touched it since.
# A change detector for the installed content, recorded per file so the next run
# can tell "we wrote this and nobody touched it" from "the user edited it". The
# whole-skill version marker cannot make that distinction: before this, a version
# transition replaced every file without a backup, including edited ones.
#
# cksum, not sha256: POSIX-mandated so it needs no probe and adds no dependency,
# and it gives CRC-32 plus the byte count. The value never leaves the machine
# that wrote it -- it is compared against the same machine's next install -- so
# cross-platform agreement does not matter, and this is a change detector, not a
# security boundary.
content_digest() {
    cksum "$1" | awk '{print $1 "-" $2}'
}

# Written after a successful copy, so a run that dies part-way leaves the old
# manifest and the next run treats the untouched files as ours (correct) and the
# half-written ones as modified (a needless backup, never a lost edit).
record_digests() {
    local destination="$1" files="$2" relative manifest temporary
    manifest="$(digest_manifest "$destination")"
    temporary="$manifest.tmp.$$"
    : > "$temporary"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        [ -f "$destination/$relative" ] || continue
        printf '%s %s\n' "$(content_digest "$destination/$relative")" "$relative" >> "$temporary"
    done <<RECORD
$files
RECORD
    printf '%s %s\n' "$(content_digest "$destination/.version")" .version >> "$temporary"
    mv -f "$temporary" "$manifest"
}

digest_manifest() {
    printf '%s/.filehashes\n' "$1"
}

# The digest this skill recorded for one relative path, or nothing.
recorded_digest() {
    local destination="$1" relative="$2" manifest
    manifest="$(digest_manifest "$destination")"
    [ -f "$manifest" ] || return 0
    awk -v want="$relative" '$2 == want { print $1; exit }' "$manifest"
}

# True when the file on disk is byte-for-byte what we last installed, so
# replacing it destroys nothing.
unmodified_since_install() {
    local destination="$1" relative="$2" file="$3" recorded
    recorded="$(recorded_digest "$destination" "$relative")"
    [ -n "$recorded" ] || return 1
    [ "$recorded" = "$(content_digest "$file")" ]
}

# A backup only earns its clutter where nothing else can recover the file. Inside
# a git work tree the user already has history, so we say what happened and let
# git be the recovery path; outside one, the .back file IS the only path.
recoverable_from_git() {
    local directory="$1"
    command -v git >/dev/null 2>&1 || return 1
    git -C "$directory" rev-parse --is-inside-work-tree >/dev/null 2>&1
}

# A dotfile beside the original: visible to `ls -a` and to the message below,
# skipped by a plain `grep -r` of the installed tree.
backup_file() {
    local file="$1"
    local directory base stamp backup suffix
    directory="$(dirname "$file")"
    if recoverable_from_git "$directory"; then
        echo "  Replaced (recoverable with git): $file" >&2
        return 0
    fi
    base="$(basename "$file")"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    backup="$directory/.$base.$stamp.back"
    suffix=1
    while [ -e "$backup" ] || [ -L "$backup" ]; do
        backup="$directory/.$base.$stamp.$suffix.back"
        suffix=$((suffix + 1))
    done
    cp -p "$file" "$backup"
    echo "  Backup: $backup" >&2
}

install_skill() {
    local skill="$1"
    local root="$2"
    local destination="$root/$skill"
    local relative source destination_file
    local changed=0
    local missing=0
    local managed_version_transition=0
    local files

    files="$(skill_files "$skill" "$PACKAGE_SELECTION")"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        if [ -L "$destination" ] || [ -L "$destination_file" ]; then
            echo "Skipping $root/$skill: existing symlink requires manual review." >&2
            summary_add "Skipped:   $destination — existing symlink requires manual review"
            return
        fi
        if [ ! -e "$destination_file" ]; then
            missing=1
        elif ! cmp -s "$source" "$destination_file"; then
            changed=1
        fi
    done <<EOF
$files
EOF

    if [ -L "$destination/.version" ]; then
        echo "Skipping $root/$skill: existing .version symlink requires manual review." >&2
        summary_add "Skipped:   $destination — existing .version symlink requires manual review"
        return
    elif [ ! -e "$destination/.version" ]; then
        missing=1
    elif ! cmp -s <(version_marker_content) "$destination/.version"; then
        changed=1
        managed_version_transition=1
    fi

    if [ "$changed" -eq 1 ]; then
        if [ "$YES" -eq 1 ]; then
            if [ "$managed_version_transition" -eq 1 ]; then
                echo "Version transition detected in $destination; replacing, backing up any edited file." >&2
            else
                echo "Changes detected in $destination; replacing after backup." >&2
            fi
        elif [ "$managed_version_transition" -eq 1 ]; then
            if ! confirm "Installed version differs in $destination. Replace it? Any file you have edited is backed up first."; then
                echo "Skipped $destination" >&2
                summary_add "Skipped:   $destination — replacement declined"
                return
            fi
        elif ! confirm "Changes detected in $destination. Replace them and create .bak backups?"; then
            echo "Skipped $destination" >&2
            summary_add "Skipped:   $destination — replacement declined"
            return
        fi
    elif [ "$missing" -eq 0 ]; then
        echo "Up to date: $destination" >&2
        summary_add "Up to date: $destination$(summary_soft_note "$skill")"
        return
    fi

    mkdir -p "$destination"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        # Back up unless we can prove the file is ours and untouched. A version
        # transition no longer suppresses this: the marker says the version
        # changed, not that the user's edits are expendable.
        if [ -e "$destination_file" ] && ! cmp -s "$source" "$destination_file" \
            && ! unmodified_since_install "$destination" "$relative" "$destination_file"; then
            backup_file "$destination_file"
        fi
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done <<EOF
$files
EOF
    # The marker is ours by definition, so it is only worth keeping when it does
    # not match any version we wrote.
    if [ -e "$destination/.version" ] && ! cmp -s <(version_marker_content) "$destination/.version" \
        && ! unmodified_since_install "$destination" .version "$destination/.version"; then
        backup_file "$destination/.version"
    fi
    version_marker_content > "$destination/.version"
    record_digests "$destination" "$files"
    echo "Installed: $destination" >&2
    summary_add "Installed: $destination$(summary_soft_note "$skill")"
}

# ---------------------------------------------------------------
# 11b. End-of-run summary and replay commands
# ---------------------------------------------------------------
# The run's result, as one contiguous block on stdout. It goes to stdout and not
# stderr because it is the answer to "what happened" (CODE-STYLE.md §10), which
# also means `install.sh > run.txt` keeps the outcome and `2>/dev/null` keeps it
# readable; the per-item progress and diagnostics stay on stderr as before.
#
# install_skill() records one line per destination through summary_add(), so the
# summary reports what actually happened rather than what was selected. A blocked
# skill is reported once, not once per root, and carries the commands that finish
# the job — the install step for its missing tool and a replay of this very run.
summary_add() {
    SUMMARY_LINES+=("$1")
}

# The install path as the user invoked it. Under `curl … | bash` there is no
# script file at all ($0 is "bash", BASH_SOURCE is empty), so the documented
# piped form is emitted instead of a path that does not exist.
replay_prefix() {
    local raw
    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        printf '%s' "$0"
        return
    fi
    raw="${REPO_URL%/}"
    raw="https://raw.githubusercontent.com/${raw#https://github.com/}"
    printf 'curl -fsSL %s/%s/install.sh | bash -s --' "$raw" "$REPO_REF"
}

# One replay line per selected root: --target takes a single directory and --skill
# a single selection, so a multi-root run needs one command per root.
replay_commands() {
    local skill="$1" root yes=""
    [ "$YES" -eq 1 ] && yes=" --yes"
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        printf '  %s --skill %s --target %s%s\n' \
            "$(replay_prefix)" "$skill" "$root" "$yes"
    done
}

summary_blocked_block() {
    local skill="$1" tool step=1
    printf 'Skipped:   %s — a hard requirement is missing, nothing was written\n' "$skill"
    printf 'To install %s once its requirements are met:\n' "$skill"
    while IFS= read -r tool; do
        [ -n "$tool" ] || continue
        printf '  %d. install %s:\n' "$step" "$(runtime_requirement_label "$tool")"
        runtime_requirement_install_hint "$tool" | sed 's/^/  /'
        step=$((step + 1))
    done < <(runtime_unmet_tools "$skill" hard)
    printf '  %d. replay this run:\n' "$step"
    replay_commands "$skill"
}

# The suffix appended to an Installed: line when a soft requirement is unmet.
summary_soft_note() {
    local skill="$1" tool note=""
    while IFS= read -r tool; do
        [ -n "$tool" ] || continue
        note="$note   (warning: $(runtime_requirement_label "$tool") missing — $(runtime_requirement_why "$skill" "$tool"))"
    done < <(runtime_unmet_tools "$skill" soft)
    printf '%s' "$note"
}

# Idempotent, because cleanup() calls it too: a run that dies part-way (the
# permission step needs a tty) must still end with the block that says what was
# written, which for a headless run is the whole user-facing story.
print_install_summary() {
    local line skill
    [ "$SUMMARY_PRINTED" -eq 0 ] || return 0
    SUMMARY_PRINTED=1
    printf '\n== Summary ==\n'
    # PORTABILITY(empty-array-setu): nothing is recorded when every selected
    # skill was blocked, and bash 3.2 treats the empty expansion as unbound.
    for line in ${SUMMARY_LINES[@]+"${SUMMARY_LINES[@]}"}; do
        printf '%s\n' "$line"
    done
    # Unquoted on purpose: $RUNTIME_BLOCKED_SKILLS is a space-joined name list.
    for skill in $RUNTIME_BLOCKED_SKILLS; do
        summary_blocked_block "$skill"
    done
}

# ---------------------------------------------------------------
# 12. Post-install plan root migration
# ---------------------------------------------------------------
# Moves plans out of the old per-agent planning/plans directories into the single
# portable plan root. This sources plan-document-lib.sh from the files this very
# run just installed, so it must stay after the install loop, and it reads that
# library from SELECTED_TARGET_PATHS[0] only.
legacy_plan_migration() {
    local root plan_skill_dir plan_root source_dir plan marker destination state_dir
    plan_skill_dir="${SELECTED_TARGET_PATHS[0]}/planning"
    [ -f "$plan_skill_dir/scripts/plan-document-lib.sh" ] || return 0
    # shellcheck source=/dev/null
    source "$plan_skill_dir/scripts/plan-document-lib.sh"
    plan_root="$(plan_ensure_root_permissions "$(plan_default_root)" "$plan_skill_dir/scripts")"
    state_dir="$plan_root/.migration-state"
    mkdir -p "$state_dir"
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        source_dir="$root/planning/plans"
        [ -d "$source_dir" ] || continue
        while IFS= read -r -d '' plan; do
            marker="$state_dir/$(printf '%s' "$plan" | cksum | awk '{print $1}')"
            [ -e "${marker}.complete" ] && continue
            destination="$plan_root/$(basename "$plan")"
            if [ -e "$destination" ] || [ -L "$destination" ]; then
                printf 'Plan migration blocked by collision; human review required: %s -> %s\n' "$plan" "$destination" >&2
                printf '%s\n' "$plan" > "${marker}.blocked"
                continue
            fi
            printf 'Migrating plan: %s -> %s\n' "$plan" "$destination" >&2
            printf '%s\n' "$plan" > "${marker}.moving"
            if mv "$plan" "$destination"; then
                rm -f "${marker}.moving"
                printf '%s\n' "$plan" > "${marker}.complete"
            else
                printf 'Plan migration blocked; rerun after fixing permissions: %s\n' "$plan" >&2
                printf '%s\n' "$plan" > "${marker}.blocked"
            fi
            # Unsorted on purpose: `sort -z` is GNU-only, and on BSD it errored
            # out, leaving the NUL read loop with zero records — every plan was
            # silently skipped while the success line below still printed. Each
            # plan is moved independently and keyed by a cksum of its own path,
            # so sibling order carries no meaning.
        done < <(find "$source_dir" -mindepth 1 -maxdepth 1 -type d -print0)
    done
    printf 'Portable plan root ready: %s\n' "$plan_root" >&2
}

ensure_plan_root_after_install() {
    legacy_plan_migration
}

# ---------------------------------------------------------------
# 13. Step 2: planning runtime permissions (interactive main path only)
# ---------------------------------------------------------------
# Grants the user-chosen agents read/write on the plans root and execution
# access to the copied planning shell scripts. Every config file that is
# modified is first backed up by backup_file from section 60 -- one scheme for
# the whole installer, rather than a second one spelled <file>.bak.<timestamp>
# here. That also means a config file inside a git work tree is replaced without
# a copy, because git is already its recovery path. Additions are idempotent:
# entries already present are never duplicated.

# Index lookup against the registry in section 1; anything not in it is custom.
agent_kind_for_root() {
    local root="${1%/}" index
    for index in "${!TARGET_PATHS[@]}"; do
        if [ "$root" = "${TARGET_PATHS[$index]%/}" ]; then
            printf '%s\n' "${TARGET_KINDS[$index]}"
            return
        fi
    done
    printf '%s\n' custom
}

# Trailing-slash trim. The python implementations these replaced used
# rstrip("/"), which removes every trailing slash, not just one.
strip_trailing_slashes() {
    local value="$1"
    while [ "$value" != "${value%/}" ]; do
        value="${value%/}"
    done
    printf '%s\n' "$value"
}

# Both permission editors are reached only from planning_permission_step, inside
# main's `contains planning "${SELECTED_SKILLS[@]}"` branch, and planning declares
# jq in runtime_requirements() — so verify_runtime_tools has already refused to
# get this far without jq. jq is therefore guaranteed, and the command -v check
# below only turns a hypothetical `set -e` abort into a clear message plus manual
# instructions. It has to precede backup_file, or a failure here leaves
# an orphaned backup behind. python3 is deliberately not used anywhere:
# jq is the only runtime dependency this installer is allowed to add.
claude_permissions() {
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}" scripts="$1" plans="$2" tmp="$3"
    local doc added tmpfile program
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v jq >/dev/null 2>&1; then
        echo "  claude-code: jq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions claude "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    backup_file "$cfg"

    # An unparseable or non-object settings.json is rebuilt from {} rather than
    # edited. `objectify` is the same defensive read at every level.
    doc="$(jq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    program='
def objectify: if type == "object" then . else {} end;
def entries: [
    "Read(\($plans)/**)", "Edit(\($plans)/**)",
    "Bash(\($scripts)/**:*)", "Read(\($scripts)/**)",
    "Bash(bash \($scripts)/**:*)",
    "Read(\($tmp)/**)", "Edit(\($tmp)/**)",
    "Bash(\($tmp)/**:*)"
];
def allowed: objectify | .permissions | objectify | .allow
    | if type == "array" then . else [] end;
'
    added="$(printf '%s' "$doc" | jq -r \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'(entries - allowed)[]')"

    # mktemp in the config's own directory so the rename is atomic, and cp -p to
    # inherit the user's mode before jq truncates it.
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | jq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        objectify
        | (.permissions | objectify) as $perm
        | ($perm.allow | if type == "array" then . else [] end) as $allow
        | .permissions = ($perm | .allow = ($allow + (entries - $allow)))' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "jq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ -n "$added" ]; then
        printf '  claude-code: added to permissions.allow:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  claude-code: permissions already present\n'
    fi
}

# opencode reads ~/.config/opencode/opencode.json or opencode.jsonc -- either
# name, JSON-C syntax allowed in both. Edit whichever exists, .json preferred;
# when neither exists, create opencode.json so the grant below has a home --
# skipping would leave every planning helper behind a permission prompt.
opencode_configfile() {
    local dir="$HOME/.config/opencode"
    if [ -n "${OPENCODE_CONFIGFILE:-}" ]; then
        printf '%s\n' "$OPENCODE_CONFIGFILE"
    elif [ -f "$dir/opencode.json" ] || [ ! -f "$dir/opencode.jsonc" ]; then
        printf '%s\n' "$dir/opencode.json"
    else
        printf '%s\n' "$dir/opencode.jsonc"
    fi
}

opencode_permissions() {
    local cfg scripts="$1" plans="$2" tmp="$3"
    local doc added legacy tmpfile program created=0
    cfg="$(opencode_configfile)"
    if [ ! -f "$cfg" ]; then
        mkdir -p "$(dirname "$cfg")" \
            || { echo "  opencode: cannot create $(dirname "$cfg")/" >&2; print_manual_permissions opencode "$scripts" "$plans" "$tmp"; return 0; }
        printf '{\n  "$schema": "https://opencode.ai/config.json"\n}\n' > "$cfg" \
            || { echo "  opencode: cannot write $cfg" >&2; print_manual_permissions opencode "$scripts" "$plans" "$tmp"; return 0; }
        echo "  opencode: created $cfg" >&2
        created=1
    fi
    if ! command -v jq >/dev/null 2>&1; then
        echo "  opencode: jq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions opencode "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    # A non-empty config that strict jq cannot parse carries JSON-C comments or
    # trailing commas, which a rewrite would strip: print manual instructions
    # instead of rebuilding from {}. Emptiness is decided here -- jq's own exit
    # status for empty input flips between versions.
    if [ "$created" -eq 0 ] && [ -s "$cfg" ] && ! jq -e '.' "$cfg" >/dev/null 2>&1; then
        echo "  opencode: $cfg is not strict JSON; add these by hand:" >&2
        print_manual_permissions opencode "$scripts" "$plans" "$tmp"
        return 0
    fi
    [ "$created" -eq 1 ] || backup_file "$cfg"

    doc="$(jq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    # opencode's permission block is keyed by tool name; each value is either an
    # action string ("ask"/"allow"/"deny") or a {pattern: action} object. A bare
    # action string is preserved as the "*" fallback pattern. A stray
    # Claude-style allow/deny/ask list is not valid here, so `base` migrates it
    # out — that removal is what the legacy notice below reports.
    program='
def objectify: if type == "object" then . else {} end;
def wanted: [
    ["read",               ["\($plans)/**", "\($scripts)/**", "\($tmp)/**"]],
    ["edit",               ["\($plans)/**", "\($tmp)/**"]],
    ["bash",               ["\($scripts)/**", "bash \($scripts)/**", "\($tmp)/**"]],
    ["external_directory", ["\($plans)/**", "\($scripts)/**", "\($tmp)/**"]]
];
def rules: if type == "object" then . elif type == "string" then {"*": .} else {} end;
def base:
    objectify
    | .permission as $p
    | (if ($p | type) == "string"
       then reduce wanted[] as $w ({}; .[$w[0]] = {"*": $p})
       else ($p | objectify) end)
    | del(.allow, .deny, .ask);
'
    legacy="$(printf '%s' "$doc" | jq -r '
        (if type == "object" then . else {} end) | .permission
        | if type == "object" and (.allow | type) == "array" and (.allow | length) > 0
          then "yes" else "no" end')"
    added="$(printf '%s' "$doc" | jq -r \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        [ wanted[] as $w
          | ($w[0]) as $tool
          | (base[$tool] | rules) as $rule
          | $w[1][] as $pattern
          | select($rule[$pattern] != "allow")
          | "\($tool): \($pattern)" ][]')"

    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | jq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        (if type == "object" then . else {} end) as $data
        | (reduce wanted[] as $w (base;
              .[$w[0]] = (reduce $w[1][] as $pattern ((.[$w[0]] | rules); .[$pattern] = "allow"))
          )) as $perm
        | $data | .permission = $perm' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "jq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ "$legacy" = "yes" ]; then
        printf '  opencode: removed invalid claude-style permission.allow list\n'
    fi
    if [ -n "$added" ]; then
        printf '  opencode: allowed in permission:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  opencode: permissions already present\n'
    fi
}

print_manual_permissions() {
    local kind="$1" scripts="$2" plans="$3" tmp="$4"
    echo "  $kind: no safe auto-editable permission file was modified." >&2
    echo "    - grant $kind read/write on $plans" >&2
    echo "    - allow $kind to execute the planning helpers under $scripts" >&2
    echo "    - allow $kind read/write/execute under the planning temp dir $tmp" >&2
    echo "    - example (Claude Code settings.json permissions.allow):" >&2
    echo "        Read($plans/**), Edit($plans/**), Bash($scripts/**:*), Bash(bash $scripts/**:*)" >&2
}

# Fallback because auto-configuration is not always possible or effective
# (e.g. Codex/OpenClaw/Cline have no stable JSON permission key, or an agent
# changed its format). Prints a self-contained prompt the user can paste into
# their AI agent so the agent configures the correct permissions itself.
print_agent_permission_prompt() {
    local plans="$1" tmp="$2"; shift 2
    local root kind
    echo >&2
    echo "If the automatic permission setup above is not effective for your agent," >&2
    echo "paste the following into your AI agent to configure it for the planning" >&2
    echo "skill. It is self-contained and refuses broad permissions:" >&2
    echo >&2
    printf '%s\n' '--- BEGIN AGENT PROMPT (copy from here) ---'
    cat <<PROMPT
Please configure my AI tooling so the "planning" skill can run. Concretely ensure:

1. My plans directory is readable and writable by you and me:
   $plans

2. My planning scratch directory is readable, writable, and executable by you
   (the planning skill writes temporary capsules and run artifacts there):
   $tmp
   Create it if it does not exist (mkdir -p).

3. The planning helper shell scripts under each of the following scripts
   directories may be executed by you; do not block them as unknown or
   suspicious:
PROMPT
    for root in "$@"; do
        kind="$(agent_kind_for_root "$root")"
        printf '   - %s: %s\n' "$kind" "${root%/}/planning/scripts"
    done
    cat <<PROMPT
Find your own permission/settings file (for example: Claude Code
settings.json "permissions.allow", OpenCode opencode.json "permission.allow",
Codex policy/sandbox config, Cline or OpenClaw allowed-tools list) and add
entries that:
  - grant read + write under $plans
  - grant read/write/execute under $tmp
  - allow executing Bash for the planning helper scripts (Read/Edit/Write plus
    Bash rules scoped to those scripts)
Add only entries that are not already present. If you modify a config file,
first copy it to .<basename>.<UTC timestamp>.back beside it before editing --
the same scheme the installer uses -- then tell me the exact path and the
entries you changed. If the file is inside a git working tree, commit or stash
instead: git is its recovery path and a stray backup file only clutters the
tree. Do not change any other permissions and do
not grant broad or all-tools access.
--- END AGENT PROMPT (copy from here) ---
PROMPT
}

planning_permission_step() {
    local plans="$HOME/.plans" agent_tmp="${TMPDIR:-/tmp}/planning-agent" root kind scripts
    echo >&2
    echo "== Step 2: planning runtime permissions ==" >&2
    if confirm "Create $plans as the global plans directory?"; then
        mkdir -p "$plans" && echo "  Created $plans" >&2
    fi
    if confirm "Grant the selected agents read/write on $plans and $agent_tmp, and allow them to execute the planning shell scripts? (Each edited config is backed up beside itself, unless git already tracks it)"; then
        for root in "${SELECTED_TARGET_PATHS[@]}"; do
            kind="$(agent_kind_for_root "$root")"
            scripts="${root%/}/planning/scripts"
            case "$kind" in
                claude)   claude_permissions "$scripts" "$plans" "$agent_tmp" ;;
                opencode) opencode_permissions "$scripts" "$plans" "$agent_tmp" ;;
                *)        print_manual_permissions "$kind" "$scripts" "$plans" "$agent_tmp" ;;
            esac
        done
    fi
    print_agent_permission_prompt "$plans" "$agent_tmp" "${SELECTED_TARGET_PATHS[@]}"
}

# ---------------------------------------------------------------
# 14. Main
# ---------------------------------------------------------------
# Order is load-bearing: show_splash before download_source (the splash is the
# thing that covers the download), selection before verify_runtime_tools, and the
# plan migration and permission step strictly after the install loop.
#
# verify_runtime_tools narrows SELECTED_SKILLS to what is installable and runs
# before select_targets, so the replay commands in the summary — which need the
# chosen roots — are printed at the end of the run rather than there.
if [ -n "$CLI_MODE" ]; then
    download_source
    case "$CLI_MODE" in
        print) cli_print_skill_files ;;
        resolve) cli_resolve_source ;;
        install) cli_install_skill ;;
    esac
    exit $?
fi

if [ -z "$SKILL_SELECTION" ] || [ -z "$TARGET_SELECTION" ]; then
    show_splash
fi
select_skills
verify_runtime_tools "${SELECTED_SKILLS[@]}"
# PORTABILITY(empty-array-setu): every requested skill can be blocked, and bash
# 3.2 treats the empty expansion as unbound under set -u.
SELECTED_SKILLS=(${RUNTIME_READY_SKILLS[@]+"${RUNTIME_READY_SKILLS[@]}"})
select_targets
download_source

if [ "${#SELECTED_SKILLS[@]}" -eq 0 ]; then
    echo >&2
    echo "Nothing was installed: every requested skill is missing a hard requirement." >&2
else
    echo >&2
    echo "Selected skills: ${SELECTED_SKILLS[*]}" >&2
    echo "Selected roots:  ${SELECTED_TARGET_NAMES[*]}" >&2
    echo >&2

    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        for skill in "${SELECTED_SKILLS[@]}"; do
            install_skill "$skill" "$root"
        done
    done

    if contains planning "${SELECTED_SKILLS[@]}"; then
        ensure_plan_root_after_install
        planning_permission_step
    fi

    echo >&2
    echo "Done. Restart the agent CLI if it does not detect the new skills automatically." >&2
fi

print_install_summary

# Non-zero when a requested skill was blocked, so a partial install cannot read
# as success in CI. A soft warning never changes the status.
[ -z "$RUNTIME_BLOCKED_SKILLS" ] || exit 1
