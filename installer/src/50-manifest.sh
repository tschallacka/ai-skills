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
# than rjq: rjq is a declared runtime dependency of some skills but the installer
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

# skill_artifact_files <skill> <relative>...
#
# Prints only the per-target artifacts that actually exist. Cross-target
# binaries are CI-delivered and never committed, so a clean checkout has none;
# installer/build-release.sh hard-fails on any listed path it cannot find, and
# a bare list therefore breaks the release on every platform whose artifacts
# this machine did not build.
#
# Takes the skill as its first argument rather than hardcoding one directory:
# it arrived for ai-text-editor, and interactive-shell is the second skill to
# ship CI-built binaries. Copying the function per skill is how the asymmetry
# this exists to remove gets reintroduced.
skill_artifact_files() {
    local skill="$1" relative
    shift
    # --dev-build is a strict opt-in mode: source_file() already checks BOTH
    # the repo-root dev build and the shipped location for a bin/ row, and
    # dies naming --dev-build when neither has it (B108's whole point -- a
    # developer asking for the dev build wants that told loudly, not a
    # skill that quietly installed without its binary). Gating existence
    # here too, against the shipped location alone, would omit the row
    # before source_file() ever got to check the dev-build root or die --
    # exactly what B317 caused it to do (B318, test-installer-dev-build.sh
    # section 3). Skip the gate under --dev-build and let that existing
    # check own it, the way it did before B317; keep gating for the
    # default path, where a missing binary should degrade the skill
    # rather than abort the whole install (B317's own reason for existing).
    if [ "${DEV_BUILD:-0}" -eq 1 ]; then
        printf '%s\n' "$@"
        return 0
    fi
    for relative in "$@"; do
        [ -f "$SOURCE_ROOT/$skill/$relative" ] && printf '%s\n' "$relative"
    done
    return 0
}

# skill_unsupported_here <skill>
#
# Prints why this machine cannot run the skill and returns 0 when that is the
# case; returns 1 for a skill this platform supports.
#
# Every other skill is text plus, at most, a binary that exists for all five
# release targets, so "can I install this here" never had to be asked before.
# interactive-shell is the first that cannot exist on a platform we otherwise
# support: its wrapper allocates the PTY through libc (openpty, TIOCSCTTY,
# TIOCSWINSZ), sets up a session and a process group, and kills that group by
# negative pid. binaries.tsv therefore declares no Windows row.
#
# Without this gate skill_files() falls through to its `*)` arm on Git Bash,
# MSYS2 or Cygwin and returns 69. install.sh runs under `set -euo pipefail` and
# assigns that in `files="$(skill_files ...)"`, so the whole installer dies
# mid-loop -- taking every skill that had not been reached yet with it. The
# `*)` arm stays as the backstop for a genuinely unknown platform, which is a
# different answer from "this platform is known and this skill is not for it".
#
# Windows support is wanted, through Cygwin, MSYS2 and native ConPTY; it is
# queued as T84a/T84b/T84c, and when it lands the row here goes away with it.
skill_unsupported_here() {
    case "$1:$(uname -s)" in
        interactive-shell:MINGW*|interactive-shell:MSYS*|interactive-shell:CYGWIN*|interactive-shell:Windows*)
            printf 'no Windows build exists; the PTY wrapper is POSIX-only (see interactive-shell/binaries.tsv)\n'
            return 0 ;;
    esac
    return 1
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
parts/part-1.md
parts/part-2.md
parts/part-3.md
parts/part-4.md
docs/README.md
REVIEWER.md
binaries.tsv
RUST-ARTIFACT-MANIFEST.md
bin/x86_64-unknown-linux-musl/plan-overview
bin/aarch64-unknown-linux-musl/plan-overview
bin/x86_64-apple-darwin/plan-overview
bin/aarch64-apple-darwin/plan-overview
bin/x86_64-pc-windows-msvc/plan-overview.exe
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
scripts/register-read.sh
scripts/register-lib.sh
scripts/resolve-finding.sh
scripts/render-plans-board.sh
scripts/plans-board-lib.sh
scripts/create-ui-story-run-cache.sh
scripts/create-ui-validation.sh
scripts/create-work-unit-inventory.sh
scripts/plan-content.sh
scripts/overview-state.sh
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
scripts/plan-crypt-lib.sh
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
scripts/verify-skill-load.sh
scripts/update-plan-progress.sh
scripts/update-progress.sh
scripts/update-step.sh
scripts/validate-plan.sh
scripts/validate-plan-common-lib.sh
scripts/validate-plan-docs-lib.sh
scripts/validate-plan-placeholders-lib.sh
scripts/validate-plan-stale-lib.sh
scripts/validate-plan-coherence-lib.sh
scripts/validate-plan-stale-wording.awk
scripts/validate-plan-countable-enumeration.awk
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
scripts/add-adversarial-finding
scripts/add-coverage
scripts/add-fix-claim
scripts/add-goal
scripts/add-planning-bug
scripts/add-ui-story
scripts/add-ui-story-links
scripts/add-work-unit
scripts/cleanup-plans
scripts/configure-ui-story-cache
scripts/create-adversarial-review
scripts/create-plan
scripts/create-plan-progress
scripts/create-progress
scripts/create-step-testing
scripts/create-ui-story-run-cache
scripts/create-ui-validation
scripts/create-work-unit-inventory
scripts/generate-reviewer
scripts/mint-fix-keys
scripts/monitor-read
scripts/overview-state
scripts/plan-content
scripts/plan-context
scripts/plan-context-wrapper
scripts/plan-env
scripts/plan-mutate
scripts/plan-root
scripts/rebuild-plan-progress
scripts/register-command
scripts/register-read
scripts/remove-coverage
scripts/remove-plan
scripts/remove-work-unit
scripts/resolve-finding
scripts/role-context
scripts/run-adversary-probe
scripts/supervision-frame
scripts/update-adversarial-review
scripts/update-plan-content
scripts/update-plan-progress
scripts/update-progress
scripts/update-step
scripts/update-ui-story
scripts/update-work-unit
scripts/validate-plan
scripts/verify-fix-keys
scripts/verify-target
EOF
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    printf '%s\n' 'bin/x86_64-unknown-linux-musl/rjq' ;;
                Linux:aarch64|Linux:arm64)
                    printf '%s\n' 'bin/aarch64-unknown-linux-musl/rjq' ;;
                Darwin:x86_64)
                    printf '%s\n' 'bin/x86_64-apple-darwin/rjq' ;;
                Darwin:arm64)
                    printf '%s\n' 'bin/aarch64-apple-darwin/rjq' ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
                    printf '%s\n' 'bin/x86_64-pc-windows-msvc/rjq.exe' ;;
                *)
                    printf 'skill_files: no rjq artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            [ "$package" = dev ] || return 0
            cat <<'EOF'
.gitignore
ARCHITECTURE.md
MAINTAINER.md
PACKAGE-MAP.tsv
skill-source.txt
scripts/generate-skill-docs.sh
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
scripts/lib/crypt/plan_crypt_bin.sh
scripts/lib/crypt/plan_crypt_resolve.sh
scripts/lib/crypt/plan_crypt_target_triple.sh
scripts/lib/crypt/plan_fix_key.sh
scripts/lib/crypt/plan_random_hex.sh
scripts/lib/crypt/plan_sha256_chain.sh
scripts/lib/crypt/plan_sha256_hex.sh
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
EOF
            if [ -d "$SOURCE_ROOT/planning/tests/fixtures/overview" ]; then
                (cd "$SOURCE_ROOT/planning/tests/fixtures/overview" && find . -type f -print) \
                    | sed 's#^\./#tests/fixtures/overview/#'
            fi
            cat <<'EOF'
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
tests/test-ci-failures-contract.sh
tests/test-coherence-checks.sh
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
tests/test-handoff-ordering-gate.sh
tests/test-inner-shell-consistency.sh
tests/test-install-ui.sh
tests/test-installer-any-of.sh
tests/test-installer-backups.sh
tests/test-installer-build.sh
tests/test-installer-busy-binary.sh
tests/test-installer-codex-permissions.sh
tests/test-installer-dependencies.sh
tests/test-installer-dev-build.sh
tests/test-installer-editor-steering.sh
tests/test-installer-integration-carryover.sh
tests/test-installer-integration-mode.sh
tests/test-installer-interactive-shell-permission.sh
tests/test-installer-manifest.sh
tests/test-installer-mcp-registration.sh
tests/test-installer-multi-root-refusal.sh
tests/test-installer-noninteractive.sh
tests/test-installer-opencode-permissions.sh
tests/test-installer-skill-selection.sh
tests/test-inventory-helpers.sh
tests/test-lib-core.sh
tests/test-lib-document.sh
tests/test-lib-progress.sh
tests/test-lib-table.sh
tests/test-limited-run-contract.sh
tests/test-man-page-roff.sh
tests/test-mermaid-accuracy.sh
tests/test-obsolete-plan.sh
tests/test-plan-overview.sh
tests/test-plans-board.sh
tests/test-plans-root-parity.sh
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
tests/test-portability-redaction.sh
tests/test-skill-positional-params.sh
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
tests/test-register-read.sh
tests/test-resolve-finding.sh
tests/test-validate-gates.sh
tests/test-skill-provenance.sh
tests/test-skill-file-length.sh
tests/test-skill-docs-generation.sh
tests/test-gate-caps.sh
tests/test-atomicity-flow.sh
tests/test-plan-data-lib.sh
tests/test-writer-hardening.sh
tests/test-overview-state.sh
tests/test-overview-serve.sh
tests/test-platform-selection.sh
tests/test-npm-package.sh
tests/test-overview-fixtures.sh
scripts/register-lib.sh
scripts/register-rebuild.sh
tests/test-plan-crypt.sh
tests/test-plan-freshness.sh
tests/test-roster-cross-reference.sh
tests/test-runtime-dependencies.sh
tests/test-self-hosted-plan.sh
tests/test-sha256-fallbacks.sh
tests/test-stale-sweep.sh
tests/test-step-atomicity-reset.sh
tests/test-step-testing-reminder.sh
tests/test-step-testing-sections.sh
tests/test-supervision-frame.sh
tests/test-target-path-validation.sh
tests/test-target-reachability-gate.sh
tests/test-ui-prohibition-scope.sh
tests/test-validation-readiness-summary.sh
tests/test-verifier-reach-memo.sh
tests/test-voice-artifact-drift.sh
tests/test-workspace-copy-excludes-build-trees.sh
tests/test-worktree-id-collision-warning.sh
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
        git-merge-resolving)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        merge-request-etiquette)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        text-etiquette)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        www)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        todo)
            printf '%s\n' SKILL.md docs/README.md requires.tsv binaries.tsv \
                schema.1.4.2.json schema.2.0.0-alpha.1.json
            # The queue's tools ship as one prebuilt binary per target, so an
            # installed skill can actually write its queue instead of being told
            # to hand-edit JSON. Only the host's row is emitted, existence-gated
            # through skill_artifact_files (B317): a raw printf of the path
            # named a `cp` source that a fresh checkout's own binary had not
            # been built for, and install_skill's copy loop has no guard of its
            # own -- under set -e that one missing file killed every skill still
            # queued behind it, not just this one.
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    skill_artifact_files todo bin/x86_64-unknown-linux-musl/todo ;;
                Linux:aarch64|Linux:arm64)
                    skill_artifact_files todo bin/aarch64-unknown-linux-musl/todo ;;
                Darwin:x86_64)
                    skill_artifact_files todo bin/x86_64-apple-darwin/todo ;;
                Darwin:arm64)
                    skill_artifact_files todo bin/aarch64-apple-darwin/todo ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
                    skill_artifact_files todo bin/x86_64-pc-windows-msvc/todo.exe ;;
                *)
                    printf 'skill_files: no todo artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            ;;
        bug-report)
            printf '%s\n' SKILL.md docs/README.md requires.tsv binaries.tsv \
                schema.1.4.2.json schema.2.0.0-alpha.1.json
            # The register's tools ship as one prebuilt binary per target, so an
            # installed skill can actually write its register instead of being
            # told to hand-edit JSON. Only the host's row is emitted,
            # existence-gated through skill_artifact_files (B317): see todo's
            # arm above for why a raw printf of the path is not safe here.
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    skill_artifact_files bug-report bin/x86_64-unknown-linux-musl/bugs ;;
                Linux:aarch64|Linux:arm64)
                    skill_artifact_files bug-report bin/aarch64-unknown-linux-musl/bugs ;;
                Darwin:x86_64)
                    skill_artifact_files bug-report bin/x86_64-apple-darwin/bugs ;;
                Darwin:arm64)
                    skill_artifact_files bug-report bin/aarch64-apple-darwin/bugs ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
                    skill_artifact_files bug-report bin/x86_64-pc-windows-msvc/bugs.exe ;;
                *)
                    printf 'skill_files: no bugs artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            ;;
        post-implementation-review)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            ;;
        ai-text-editor)
            cat <<'EDITOR_EOF'
SKILL.md
agents/openai.yaml
docs/README.md
ai-text-editor.1
binaries.tsv
schemas/protocol.v1.json
schemas/capabilities.v1.json
requires.tsv
integration.tsv
references/protocol.md
EDITOR_EOF
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    skill_artifact_files ai-text-editor bin/x86_64-unknown-linux-musl/ai-text-editor-server bin/x86_64-unknown-linux-musl/ai-text-editor bin/x86_64-unknown-linux-musl/ai-text-editor-mcp ;;
                Linux:aarch64|Linux:arm64)
                    skill_artifact_files ai-text-editor bin/aarch64-unknown-linux-musl/ai-text-editor-server bin/aarch64-unknown-linux-musl/ai-text-editor bin/aarch64-unknown-linux-musl/ai-text-editor-mcp ;;
                Darwin:x86_64)
                    skill_artifact_files ai-text-editor bin/x86_64-apple-darwin/ai-text-editor-server bin/x86_64-apple-darwin/ai-text-editor bin/x86_64-apple-darwin/ai-text-editor-mcp ;;
                Darwin:arm64)
                    skill_artifact_files ai-text-editor bin/aarch64-apple-darwin/ai-text-editor-server bin/aarch64-apple-darwin/ai-text-editor bin/aarch64-apple-darwin/ai-text-editor-mcp ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
                    skill_artifact_files ai-text-editor bin/x86_64-pc-windows-msvc/ai-text-editor-server.exe bin/x86_64-pc-windows-msvc/ai-text-editor.exe bin/x86_64-pc-windows-msvc/ai-text-editor-mcp.exe ;;
                *)
                    printf 'skill_files: no ai-text-editor artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            ;;
        chat)
            cat <<'CHATEOF'
SKILL.md
docs/README.md
requires.tsv
binaries.tsv
integration.tsv
CHATEOF
            # Existence-gated through skill_artifact_files (B317): a fresh
            # checkout with none of chat's three binaries built used to name
            # them anyway, and install_skill's `cp` of the first one killed the
            # whole install under set -e, taking every skill queued after chat
            # down with it.
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    skill_artifact_files chat bin/x86_64-unknown-linux-musl/chat-server-rs bin/x86_64-unknown-linux-musl/chat-client-rs bin/x86_64-unknown-linux-musl/chat-mcp ;;
                Linux:aarch64|Linux:arm64)
                    skill_artifact_files chat bin/aarch64-unknown-linux-musl/chat-server-rs bin/aarch64-unknown-linux-musl/chat-client-rs bin/aarch64-unknown-linux-musl/chat-mcp ;;
                Darwin:x86_64)
                    skill_artifact_files chat bin/x86_64-apple-darwin/chat-server-rs bin/x86_64-apple-darwin/chat-client-rs bin/x86_64-apple-darwin/chat-mcp ;;
                Darwin:arm64)
                    skill_artifact_files chat bin/aarch64-apple-darwin/chat-server-rs bin/aarch64-apple-darwin/chat-client-rs bin/aarch64-apple-darwin/chat-mcp ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
                    skill_artifact_files chat bin/x86_64-pc-windows-msvc/chat-server-rs.exe bin/x86_64-pc-windows-msvc/chat-client-rs.exe bin/x86_64-pc-windows-msvc/chat-mcp.exe ;;
                *)
                    printf 'skill_files: no chat artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            [ "$package" = dev ] || return 0
            cat <<'CHATEOF'
tests/test-chat.sh
tests/test-chat-resolution.sh
tests/test-chat-broadcast-stall.sh
tests/test-chat-descriptor-leak.sh
tests/test-chat-owner-socket.sh
tests/test-chat-cap-negotiation.sh
tests/test-chat-tail-msgid-cursor.sh
CHATEOF
            ;;
        interactive-shell)
            cat <<'ISHEOF'
SKILL.md
agents/openai.yaml
docs/README.md
requires.tsv
binaries.tsv
ISHEOF
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64)
                    skill_artifact_files interactive-shell bin/x86_64-unknown-linux-musl/interactive-shell bin/x86_64-unknown-linux-musl/interactive-shell-input ;;
                Linux:aarch64|Linux:arm64)
                    skill_artifact_files interactive-shell bin/aarch64-unknown-linux-musl/interactive-shell bin/aarch64-unknown-linux-musl/interactive-shell-input ;;
                Darwin:x86_64)
                    skill_artifact_files interactive-shell bin/x86_64-apple-darwin/interactive-shell bin/x86_64-apple-darwin/interactive-shell-input ;;
                Darwin:arm64)
                    skill_artifact_files interactive-shell bin/aarch64-apple-darwin/interactive-shell bin/aarch64-apple-darwin/interactive-shell-input ;;
                *)
                    printf 'skill_files: no interactive-shell artifact for %s:%s\n' "$(uname -s)" "$(uname -m)" >&2
                    return 69 ;;
            esac
            [ "$package" = dev ] || return 0
            cat <<'ISHEOF'
tests/test-interactive-shell.sh
tests/test-interactive-shell-exploration.sh
TODO.json
ISHEOF
            ;;
        ci-failures)
            printf '%s\n' SKILL.md docs/README.md requires.tsv
            local file
            for file in "$SOURCE_ROOT/ci-failures/scripts/"*.sh; do
                [ -f "$file" ] && printf '%s\n' "scripts/$(basename "$file")"
            done
            ;;
    esac
}

# integration_mode_for() moved to 05-config.sh, beside the INTEGRATION_*
# variables it reads and the writers that set them: the picker calls it too
# (T95), and install-ui.sh does not source this part.

# Which mode's binary is already on disk at DESTINATION, or empty when there
# is none (a first install) -- signal 1 of the two T109 names (the other,
# an agent config already pointing at this skill's mcp binary, is left for a
# follow-up; the destination signal alone is what a headless `--all` update
# needs to stop tearing down a live mode it was never told to leave).
#
# More than one mode's binary present is a half-finished earlier switch --
# remove_stale_integration_binaries only ever cleans up what the CURRENT
# mode's answer says to remove, so it cannot itself have caused this -- and
# it is reported rather than guessed at, on stderr so a caller capturing the
# mode itself is not corrupted by the warning.
integration_installed_mode() { # <skill> <destination> -> mode, or empty
    local skill="$1" destination="$2" path mode found=''
    for path in "$destination"/bin/*/*; do
        [ -f "$path" ] || continue
        mode="$(integration_binary_mode "$skill" "${path##*/}")"
        [ -n "$mode" ] || continue
        if [ -n "$found" ] && [ "$found" != "$mode" ]; then
            printf '%s: %s has binaries for both %s and %s modes; a previous switch may be unfinished. Pass --integration to say which mode to keep.\n' \
                "${0##*/}" "$destination" "$found" "$mode" >&2
            printf ''
            return 0
        fi
        found="$mode"
    done
    printf '%s\n' "$found"
}

# Does this file belong in the mode this skill is being installed in?
#
# Only artifacts under bin/ carry a mode; everything else -- SKILL.md, the
# schemas, the manpage -- is mode-free, so an mcp install is still a complete
# skill directory and not a lone binary. A skill that declares no
# integration.tsv has no arm in the generated table, its lookup is empty, and
# every file is allowed: the flag is a no-op for it rather than an error.
#
# `mode` is the answer install_skill() already resolved once, up front, and
# every one of its calls passes it in: integration_mode_for detects from
# whatever is CURRENTLY on disk at the destination, and this same function is
# what remove_stale_integration_binaries uses to decide what to delete FROM
# that disk -- recomputing per call would let the answer change mid-loop as
# soon as the first stale binary is removed. A caller with no resolved mode
# yet (the picker, T95) omits it and gets the old explicit-or-default answer.
integration_file_allowed() {
    local skill="$1" relative="$2" mode="${3:-}" declared
    case "$relative" in
        bin/*) : ;;
        *) return 0 ;;
    esac
    declared="$(integration_binary_mode "$skill" "${relative##*/}")"
    [ -n "$declared" ] || return 0
    [ -n "$mode" ] || mode="$(integration_mode_for "$skill")"
    [ "$declared" = "$mode" ]
}

# A skill switching integration mode (mcp -> skill or back) leaves the
# previous mode's binary on disk unless something removes it: the install
# loop only ever copies files the CURRENT mode allows, so a binary the
# PREVIOUS mode wrote and this one does not declare is simply never revisited.
# `$files` (from skill_files) already lists every bin/ row for every mode this
# platform offers -- the same list the copy loop filters by
# integration_file_allowed -- so reusing it here needs no second, hand-kept
# list of triples or binary names to fall out of date the next platform this
# grows to support.
remove_stale_integration_binaries() {
    local skill="$1" destination="$2" files="$3" mode="${4:-}" relative physical
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        case "$relative" in bin/*) : ;; *) continue ;; esac
        if integration_file_allowed "$skill" "$relative" "$mode"; then
            continue
        fi
        physical="$(platform_relative_path "$skill" "$relative")"
        if [ -e "$destination/$physical" ]; then
            rm -f "$destination/$physical"
        fi
    done <<EOF
$files
EOF
    # The common case is "nothing to remove": the last loop iteration's own
    # exit status must never become this function's return status, or the
    # bare `remove_stale_integration_binaries ...` call at each call site trips
    # `set -e` on exactly the runs that needed no cleanup at all.
    return 0
}

source_file() {
    local skill="$1"
    local relative="$2"
    local physical
    physical="$(platform_relative_path "$skill" "$relative")"
    if [ "$DEV_BUILD" -eq 1 ]; then
        case "$relative" in
            bin/*)
                if [ -f "$SOURCE_ROOT/$physical" ]; then
                    printf '%s/%s\n' "$SOURCE_ROOT" "$physical"
                    return
                fi
                [ -f "$SOURCE_ROOT/$skill/$physical" ] || die \
                    "--dev-build: no build of $skill/$relative in $SOURCE_ROOT/bin/ or $SOURCE_ROOT/$skill/bin/ -- run ./setup-dev-env.sh, or drop --dev-build to use the shipped binary"
                ;;
        esac
    fi
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$physical"
}

# True when --dev-build is set, this row is a binary, and its source resolved
# to the repo-root dev build directory rather than the skill's shipped one --
# so a caller can name which tree an installed binary actually came from
# (T108: two locations silently disagreeing cost an hour to notice).
source_is_dev_build() {
    local skill="$1" relative="$2" physical
    [ "$DEV_BUILD" -eq 1 ] || return 1
    case "$relative" in
        bin/*) ;;
        *) return 1 ;;
    esac
    physical="$(platform_relative_path "$skill" "$relative")"
    [ -f "$SOURCE_ROOT/$physical" ]
}

# Manifest entries keep the command name users invoke, without a platform
# suffix. Windows still needs the executable suffix on disk. Only generated
# planning commands use this logical-name rule; ordinary files and shell
# helpers retain their manifest path exactly.
platform_relative_path() {
    local skill="$1"
    local relative="$2"
    case "$skill:$relative" in
        planning:scripts/*.sh) : ;;
        planning:scripts/*)
            case "$(uname -s)" in
                MINGW*|MSYS*|CYGWIN*|Windows*) relative="$relative.exe" ;;
            esac
            ;;
    esac
    printf '%s\n' "$relative"
}
