# MODE: DEV
# PACKAGE: PROD
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
    prepend_bundled_rjq
    case "$CLI_MODE" in
        print) cli_print_skill_files ;;
        resolve) cli_resolve_source ;;
        install) ensure_agent_data_dir; cli_install_skill ;;
    esac
    exit $?
fi

if [ -z "$SKILL_SELECTION" ] || [ -z "$TARGET_SELECTION" ]; then
    show_splash
fi
select_skills
download_source
prepend_bundled_rjq
verify_runtime_tools "${SELECTED_SKILLS[@]}"
ensure_agent_data_dir
# PORTABILITY(empty-array-setu): every requested skill can be blocked, and bash
# 3.2 treats the empty expansion as unbound under set -u.
SELECTED_SKILLS=(${RUNTIME_READY_SKILLS[@]+"${RUNTIME_READY_SKILLS[@]}"})
select_targets

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
