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

SKILL_NAMES=(planning project-specificies resource-limited-testing brainstorm post-implementation-review todo)
SKILL_DESCRIPTIONS=(
    'Durable, resumable plans with steps and verification.'
    'Records project conventions, quirks, and deviations.'
    'Caps CPU and memory for demanding tool runs.'
    'Shapes an idea into a recorded, agreed picture before planning.'
    'After-the-fact review and proposed fixes for built code.'
    'A nested queue of work in one JSON file, read with jq.'
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
  --skill <name>           Install or update one skill
  --target <path>          Install into one skill root without prompting
  --yes                    Accept replacements; an edited file is still backed up  
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

