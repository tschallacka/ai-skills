#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Print the required heading and its paragraph-number prefix for a mutable
# narrative section. Structured sections are intentionally excluded.
plan_section_spec() {
    local kind="$1" section="$2"
    case "$kind/$section" in
        plan/current-state) printf '%s\t%s\n' '## Current state' 2 ;;
        plan/desired-outcome) printf '%s\t%s\n' '## Desired outcome' 3 ;;
        plan/approach) printf '%s\t%s\n' '## Approach' 4 ;;
        plan/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        plan/affected-areas) printf '%s\t%s\n' '## Affected areas' 6 ;;
        plan/constraints-and-decisions) printf '%s\t%s\n' '## Constraints and decisions' 7 ;;
        plan/risks-and-open-questions) printf '%s\t%s\n' '## Risks and open questions' 8 ;;
        plan/environment-facts) printf '%s\t%s\n' '## Environment facts' 9 ;;
        plan/approach-decisions) printf '%s\t%s\n' '## Approach decisions' 10 ;;
        goal/current-state-and-prior-goal-handoffs) printf '%s\t%s\n' '## Current state and prior-goal handoffs' 2 ;;
        goal/outcome-and-definition-of-done) printf '%s\t%s\n' '## Outcome and definition of done' 3 ;;
        goal/why-this-goal-is-needed) printf '%s\t%s\n' '## Why this goal is needed' 4 ;;
        goal/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        goal/affected-areas) printf '%s\t%s\n' '## Affected files, systems, data, and interfaces' 6 ;;
        goal/dependencies-and-handoffs) printf '%s\t%s\n' '## Dependencies and handoffs' 7 ;;
        goal/implementation-approach-risks-and-edge-cases) printf '%s\t%s\n' '## Implementation approach, risks, and edge cases' 8 ;;
        goal/owned-work-units) printf '%s\t%s\n' '## Owned work units' 9 ;;
        goal/goal-size-exception) printf '%s\t%s\n' '## Goal-size exception' 11 ;;
        step/objective) printf '%s\t%s\n' '## Objective' 4 ;;
        step/instructions) printf '%s\t%s\n' '## Instructions' 5 ;;
        step/acceptance-criteria) printf '%s\t%s\n' '## Acceptance criteria' 6 ;;
        step/handoff) printf '%s\t%s\n' '## Handoff' 7 ;;
        testing/automated-tests) printf '%s\t%s\n' '## Automated tests' 2 ;;
        testing/browser-verification) printf '%s\t%s\n' '## Browser verification' 3 ;;
        testing/backend-verification) printf '%s\t%s\n' '## Backend verification' 4 ;;
        testing/manual-verification) printf '%s\t%s\n' '## Manual verification' 5 ;;
        review/review-scope) printf '%s\t%s\n' '## Review scope' 1 ;;
        review/findings) printf '%s\t%s\n' '## Findings' 2 ;;
        review/rationale) printf '%s\t%s\n' '## Verdict' 3 ;;
        *) plan_die "$(plan_unknown_section "$kind" "$section")" ;;
    esac
}
