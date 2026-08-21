# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 10. CLI-mode handlers
# ---------------------------------------------------------------
# Machine-facing entry points for the planning skill's own tooling. The exit
# codes of cli_install_skill are a contract: 2 = approval declined (nothing
# written), 3 = a collision that is not a managed-version upgrade, or a symlink.
cli_print_skill_files() {
    [ "$CLI_SKILL" = "planning" ] || die "unsupported CLI skill: $CLI_SKILL"
    [ "$CLI_FORMAT" = "tsv" ] || die "unsupported CLI format: $CLI_FORMAT"
    cat "$SOURCE_ROOT/planning/PACKAGE-MANIFEST.txt"
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

