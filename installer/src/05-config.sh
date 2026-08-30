# MODE: DEV
# PACKAGE: PROD
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

SKILL_NAMES=(planning project-specificies resource-limited-testing brainstorm post-implementation-review todo bug-report chat git-worktrees merge-request-etiquette text-etiquette)
SKILL_DESCRIPTIONS=(
    'Durable, resumable plans with steps and verification.'
    'Records project conventions, quirks, and deviations.'
    'Caps CPU and memory for demanding tool runs.'
    'Shapes an idea into a recorded, agreed picture before planning.'
    'After-the-fact review and proposed fixes for built code.'
    'A nested queue of work in one JSON file, read with rjq.'
    'Defects with their reproduction, mechanism and verification, in JSON.'
    'IRC-basis agent chat over TLS: a rust server and client, UDP discovery, deltas.'
    'Separate checkouts so parallel work and long verifications cannot collide.'
    'Merge requests in your voice: own branch, one squashed commit, a TLDR, then the fix.'
    'Shorthand and a clipped register for agent chat: short, factual, no people-please prose.'
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
    'A nested queue of work in one JSON file, read and written with rjq.
For work that outlives the conversation and must survive a restart, a handoff or a compaction.
Every task carries its status and its detail, so a cold reader knows what was intended and what is left.
Not for the steps of a task already in progress, and not for defects -- those belong in the bug register.'
    'Defects recorded with the reproduction, the observed output, the mechanism once established, and the verification that closes them.
One defect per entry: if stating it needs the word "and", it is two entries that reference each other.
An entry closes only with a fix and a verification naming the mutation that fails without it, so a closure cannot be a claim.
Written only through its helpers -- an out-of-enum value makes the register unsound and every later write refuses.'
    'An IRC-over-TLS message bus for agents: a rust server that a standard TLS IRC client could join, and a rust client.
The server speaks the RFC 1459 grammar over TLS and mints its self-signed certificate at first run.
The client discovers servers over UDP, pins the certificate (TOFU), and sends / reads a delta since an id / tails.
History is additive: an agent fetches the messages after id N via the FETCH extension, instead of re-reading a whole channel.'
    'Gives a task its own checkout, so several agents can work at once without seeing each other half-finished edits.
Also isolates a long verification from later commits, and keeps a risky change off the main checkout.
Covers the concurrency hazard that silently fails a long-running command when two runs share a tree.
And the part that is usually learned late: the order to merge the branches back in, and which conflict classes to expect.'
    'Writes the merge request description in the voice of the person whose name is on it, opening with a one-paragraph TLDR.
The body names the defect, the cause and the change, and stops there: no headings for their own sake, no restating the diff.
The reasoning is derived from git log for the branch, so a description never explains what a commit message should have said.
Chat transcripts never travel, and a collapsible block is allowed only for evidence the commits genuinely cannot carry.'
    'Tells an agent how to talk in a shared channel: clipped sentences, a shared shorthand, and the prose that is banned.
The register is caveman-tight: facts first, fragments fine, paths and errors exact while the wrapper around them shrinks.
Praise stops at gj, corrections are applied without thanks, and unknown shorthand is asked about, never guessed.
The register bends only where brevity would cost understanding: destructive actions, security warnings and direct questions.'
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

