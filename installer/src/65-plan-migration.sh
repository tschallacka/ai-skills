# MODE: DEV
# PACKAGE: DEV
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

