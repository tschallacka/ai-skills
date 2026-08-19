# ---------------------------------------------------------------
# 9. Per-skill file manifest
# ---------------------------------------------------------------
# The planning list is a deliberate second copy of planning/PACKAGE-MANIFEST.txt:
# the two are diffed by planning/tests/test-installer-manifest.sh, which extracts
# this heredoc by regex. Do not restructure skill_files() or that heredoc, and do
# not derive it from the manifest — the duplication IS the cross-check.
version_marker_content() {
    printf 'format=ai-skills-version-1\n'
    printf 'source_version=%s\n' "$SOURCE_VERSION"
    printf 'source_ref=%s\n' "$REPO_REF"
}

skill_files() {
    case "$1" in
        planning)
            cat <<'EOF'
SKILL.md
REVIEWER.md
references/plan-read-contract.md
references/ui-user-story-validation.md
references/comment-discipline-contract.md
telemetry-schema.json
placeholders.json
state-change-registry.json
never-executable-extensions.json
context/brainstorm-limiting-context.md
context/brainstorm-limiting-context-contract.json
context/brainstorm-limiting-context-benchmark.json
context/brainstorm-limiting-context-oracle.json
tests/fixtures/planning-context/case-matrix.tsv
tests/fixtures/planning-context/expected-outcomes.jsonl
tests/fixtures/planning-context/test-signing-key.pub
tests/fixtures/planning-context/platform-inputs.tsv
tests/fixtures/planning-context/runner-targets.discovery.txt
tests/fixtures/planning-context/runner-targets.tsv
tests/fixtures/progress-shape/progress.md
tests/fixtures/progress-shape/01-goal-a/goal.md
tests/fixtures/progress-shape/01-goal-a/progress.md
tests/fixtures/progress-shape/01-goal-a/steps/01-step-a.md
tests/fixtures/progress-shape/02-goal-b/goal.md
tests/fixtures/progress-shape/02-goal-b/progress.md
tests/fixtures/progress-shape/02-goal-b/steps/01-step-b.md
tests/fixtures/progress-shape/02-goal-b/steps/02-step-b2.md
tests/fixtures/adversary-probe/FIXTURE-VERSION
tests/fixtures/adversary-probe/README.md
tests/fixtures/adversary-probe/plan-description.md
tests/fixtures/adversary-probe/progress.md
tests/fixtures/adversary-probe/work-unit-inventory.md
tests/fixtures/adversary-probe/adversarial-review.md
tests/fixtures/adversary-probe/01-health-endpoint/goal.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/01-step-add-handler.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/02-step-add-test.md
tests/test-planning-context-contract.sh
tests/test-installer-manifest.sh
tests/lib-test.sh
tests/test-plan-env.sh
tests/test-plan-snapshot.sh
tests/test-plan-integrity-and-monitor.sh
tests/test-reviewer-projection.sh
tests/test-plan-context-reviewer.sh
tests/test-voice-artifact-drift.sh
tests/test-supervision-frame.sh
tests/test-persona-drift.sh
tests/test-progress-bar-shape.sh
tests/test-stale-sweep.sh
tests/test-fix-keys.sh
tests/test-coverage-gaps.sh
tests/test-flag-coverage.sh
PACKAGE-MANIFEST.txt
requires.tsv
ROLES.md
MAINTAINER-STYLE-CONTRACT.md
roles/planning.md
roles/execution.md
roles/cleanup.md
roles/VOICES.md
scripts/add-coverage.sh
scripts/add-adversarial-finding.sh
scripts/add-goal.sh
scripts/add-ui-story.sh
scripts/add-work-unit.sh
scripts/configure-ui-story-cache.sh
scripts/create-adversarial-review.sh
scripts/create-plan-progress.sh
scripts/create-plan.sh
scripts/create-progress.sh
scripts/create-step-testing.sh
scripts/rebuild-plan-progress.sh
scripts/register-command.sh
scripts/create-ui-story-run-cache.sh
scripts/create-ui-validation.sh
scripts/create-work-unit-inventory.sh
scripts/plan-content.sh
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
scripts/remove-plan.sh
scripts/cleanup-plans.sh
scripts/run-adversary-probe.sh
EOF
            ;;
        project-specificies)
            printf '%s\n' SKILL.md requires.tsv
            ;;
        resource-limited-testing)
            printf '%s\n' SKILL.md requires.tsv
            local file
            for file in "$SOURCE_ROOT/resource-limited-testing/scripts/"*.sh; do
                [ -f "$file" ] && printf '%s\n' "scripts/$(basename "$file")"
            done
            ;;
        brainstorm)
            printf '%s\n' SKILL.md requires.tsv
            ;;
        post-implementation-review)
            printf '%s\n' SKILL.md requires.tsv
            ;;
    esac
}

source_file() {
    local skill="$1"
    local relative="$2"
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$relative"
}

