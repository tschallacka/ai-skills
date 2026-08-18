#!/usr/bin/env bash
# lib-structural-gate.sh - the required-artifact checks a benchmark plan must
# satisfy, and the five matchers they are built from. Sourced, never executed.
#
# Extracted from the quoted `start-worker.sh` heredoc `setup-benchmark.sh` used
# to emit. In that heredoc the verdict was a mutable global assigned from inside
# a `{ ... } > report` brace group and read after it, which worked ONLY because
# bash does not fork for a redirected group. Here the verdict is a
# function-local and the only channel out is the return code:
#
#   0 - every required artifact is present
#   1 - at least one is missing; the report names each one
#
# so the caller threads it explicitly and correctly whether or not the
# redirection forks - the property the old shared global silently relied on:
#
#   STRUCTURAL_VALIDATION="pass"
#   structural_gate_report "$PLAN_DIR" "$PLAN_NAME" "$PLAN_FOUND" \
#       > "$STRUCTURAL_REPORT" || STRUCTURAL_VALIDATION="fail"
#
# The report goes to stdout: one PASS/FAIL line per check, then a trailing
# `result=<pass|fail>` line that downstream tooling greps for.
#
# The five matchers are defined inside the entry point, but bash scopes function
# definitions globally, so they are prefixed `structural_` to avoid shadowing a
# caller's own `require_file`.

structural_gate_report() {
    local plan_dir="$1" plan_name="$2" plan_found="$3"
    local verdict="pass"
    echo "Structural validation for $plan_name"
    echo
    structural_require_file() {
        local label="$1"
        local path="$2"
        if [ -f "$path" ]; then
            echo "PASS: $label ($path)"
        else
            echo "FAIL: $label ($path)"
            verdict="fail"
        fi
    }
    structural_require_any_file() {
        local label="$1"
        shift
        local path
        for path in "$@"; do
            if [ -f "$path" ]; then
                echo "PASS: $label ($path)"
                return 0
            fi
        done
        echo "FAIL: $label (none of: $*)"
        verdict="fail"
    }
    structural_require_pattern() {
        local label="$1"
        local pattern="$2"
        local match
        match="$(find "$plan_dir" -type f -iname "$pattern" -print -quit 2>/dev/null)"
        if [ -n "$match" ]; then
            echo "PASS: $label ($match)"
        else
            echo "FAIL: $label (pattern $pattern)"
            verdict="fail"
        fi
    }
    structural_require_any_pattern() {
        local label="$1"
        shift
        local pattern match
        for pattern in "$@"; do
            match="$(find "$plan_dir" -type f -iname "$pattern" -print -quit 2>/dev/null)"
            if [ -n "$match" ]; then
                echo "PASS: $label ($match)"
                return 0
            fi
        done
        echo "FAIL: $label (patterns: $*)"
        verdict="fail"
    }
    structural_require_directory_with_files() {
        local label="$1"
        local directory="$2"
        if [ -d "$plan_dir/$directory" ] && find "$plan_dir/$directory" -type f -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: $label ($plan_dir/$directory)"
        else
            echo "FAIL: $label ($plan_dir/$directory)"
            verdict="fail"
        fi
    }

    if [ "$plan_found" -eq 1 ]; then
        structural_require_file "plan description" "$plan_dir/plan-description.md"
        structural_require_file "plan progress tracker" "$plan_dir/progress.md"
        structural_require_any_file "validation report" "$plan_dir/validation.md" "$plan_dir/validation-results.md"
        structural_require_any_file "analysis report" "$plan_dir/analysis-report.md" "$plan_dir/analysis.md"
        structural_require_any_pattern "goal" 'goal.md'
        structural_require_any_pattern "work-unit inventory" '*work-unit*' '*atomic-work-unit*'
        structural_require_any_pattern "UI user story" '*ui*user*stor*' '*ui*stor*'
        if find "$plan_dir" -type f -iname '*run*cache*' -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: UI story run/cache (matching run/cache artifact)"
        else
            structural_require_directory_with_files "UI story run/cache" 'ui-story-runs'
        fi
        structural_require_pattern "adversarial review" '*adversarial*review*'
        structural_require_any_pattern "bug register" '*bug*' '*bugs*'
        if find "$plan_dir" -type f -iname '*context*snap*' -print -quit 2>/dev/null | grep -q .; then
            echo "PASS: context snapshot (matching context snapshot artifact)"
        else
            structural_require_directory_with_files "context snapshot" 'context/snapshots'
        fi
        structural_require_any_pattern "testing companion" '*-testing.md' '*testing*.md'
    else
        echo "FAIL: plan directory missing ($plan_dir)"
        verdict="fail"
    fi
    echo
    echo "result=$verdict"
    [ "$verdict" = "pass" ]
}
