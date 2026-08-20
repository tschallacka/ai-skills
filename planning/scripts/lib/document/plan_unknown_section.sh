#!/usr/bin/env bash
# Concise, agent-friendly error for an unknown narrative section: list the
# valid ids for the document kind and, when one is close, suggest it.
plan_unknown_section() {
    local kind="$1" section="$2" valid id close="" best=""
    case "$kind" in
        plan) valid="current-state desired-outcome approach approach-decisions scope affected-areas constraints-and-decisions risks-and-open-questions environment-facts" ;;
        goal) valid="current-state-and-prior-goal-handoffs outcome-and-definition-of-done why-this-goal-is-needed scope affected-areas dependencies-and-handoffs implementation-approach-risks-and-edge-cases owned-work-units goal-size-exception" ;;
        step) valid="objective instructions acceptance-criteria handoff" ;;
        testing) valid="automated-tests browser-verification backend-verification manual-verification" ;;
        # A review has no narrative section: Review scope and Verdict hold
        # fields (-f writes them, one label at a time) and Findings is a table
        # (update-adversarial-review.sh writes it). A section rewrite here
        # dropped `- Status:` and left the plan unapprovable.
        review) valid="" ;;
        *) valid="" ;;
    esac
    for id in $valid; do
        if [ "$id" = "$section" ]; then
            printf 'Section is valid: %s\n' "$section"
            return 0
        fi
        # Simple closeness: same prefix or >50% shared prefix length.
        if [[ "$id" == "$section"* ]] || [[ "$section" == "$id"* ]]; then
            [ -z "$close" ] && close="$id"
        fi
        if [ -z "$best" ] || [ "${#id}" -lt "${#best}" ]; then
            # crude nearest: shortest id differing in fewest chars by prefix
            :
        fi
    done
    printf "Section '%s' is not a mutable narrative section for a %s document.\n" "$section" "$kind"
    if [ -n "$close" ]; then
        printf 'Closest match: %s\n' "$close"
    fi
    printf 'Valid %s section ids: %s\n' "$kind" "$valid"
}
