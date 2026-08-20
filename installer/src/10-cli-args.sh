# MODE: DEV
# PACKAGE: DEV
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
            SKILL_SELECTION="$2"
            shift 2
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

