# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 2. CLI-mode argument parsing
# ---------------------------------------------------------------
# The machine-facing modes are consumed FIRST and then `set --` away, because
# the generic flag loop below dies on anything it does not know. Moving this
# case after that loop makes --print-skill-files an "Unknown option".
# `trap cleanup EXIT` must be installed before the first mktemp -d.
case "${1:-}" in
    --print-skill-files)
        [ "$#" -eq 3 ] || die_usage "--print-skill-files needs skill and --format"
        CLI_MODE="print"
        CLI_SKILL="$2"
        [ "$3" = "--format=tsv" ] || die_usage "--print-skill-files requires --format=tsv"
        CLI_FORMAT="tsv"
        set --
        ;;
    --resolve-source)
        [ "$#" -eq 3 ] || die_usage "--resolve-source needs skill and relative path"
        CLI_MODE="resolve"
        CLI_SKILL="$2"
        CLI_RELATIVE="$3"
        set --
        ;;
    --install-skill)
        [ "$#" -eq 6 ] || die_usage "--install-skill needs skill, --target, and --approval"
        CLI_MODE="install"
        CLI_SKILL="$2"
        [ "$3" = "--target" ] || die_usage "--install-skill requires --target"
        TARGET_SELECTION="$4"
        [ "$5" = "--approval" ] || die_usage "--install-skill requires --approval"
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
            [ "$#" -ge 2 ] || die_usage "--skill needs a skill name"
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
            [ "$#" -ge 2 ] || die_usage "--package needs prod or dev"
            case "$2" in
                prod|dev) PACKAGE_SELECTION="$2" ;;
                *) die_usage "--package must be prod or dev, not $2" ;;
            esac
            shift 2
            ;;
        --package=*)
            case "${1#--package=}" in
                prod|dev) PACKAGE_SELECTION="${1#--package=}" ;;
                *) die_usage "--package must be prod or dev, not ${1#--package=}" ;;
            esac
            shift
            ;;
        --integration)
            [ "$#" -ge 2 ] || die_usage "--integration needs a mode, or skill=mode"
            record_integration "$2"
            shift 2
            ;;
        --integration=*)
            record_integration "${1#--integration=}"
            shift
            ;;
        --editor-integration)
            [ "$#" -ge 2 ] || die_usage "--editor-integration needs skill or mcp"
            record_integration "ai-text-editor=$2"
            shift 2
            ;;
        --editor-integration=*)
            record_integration "ai-text-editor=${1#--editor-integration=}"
            shift
            ;;
        --target)
            [ "$#" -ge 2 ] || die_usage "--target needs a directory"
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
            die_usage "Unknown option: $1 (use --help for usage)"
            ;;
    esac
done
