# MODE: DEV
# PACKAGE: PROD
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
runtime/server.py
runtime/server.js
runtime/server.pl
runtime/bash-handler.sh
CHATEOF
            [ "$package" = dev ] || return 0
            cat <<'CHATEOF'
tests/test-chat.sh
CHATEOF
            ;;
    esac
}

source_file() {
    local skill="$1"
    local relative="$2"
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$relative"
}
