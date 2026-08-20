# MODE: DEV
# PACKAGE: DEV
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
        printf '  %d. install %s:\n' "$step" "$tool"
        runtime_tool_install_hint "$tool" | sed 's/^/  /'
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
        note="$note   (warning: $tool missing — $(runtime_requirement_why "$skill" "$tool"))"
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

