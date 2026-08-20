#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_document_kind() {
    case "$1" in
        plan) printf '%s\n' plan ;;
        adversarial-review) printf '%s\n' review ;;
        coverage|inventory|stories|bugs|fixes|fix-keys|fixkeys|approval|progress) printf '%s\n' reference ;;
        goal-progress:*) printf '%s\n' reference ;;
        goal:*) printf '%s\n' goal ;;
        step:*)
            # A step id ending in -testing names the step's testing companion,
            # which has its own writable sections (Automated tests, ...).
            case "$1" in
                *-testing) printf '%s\n' testing ;;
                *) printf '%s\n' step ;;
            esac
            ;;
        unit:*) printf '%s\n' step ;;
        *) plan_die "Unknown document ID: $1" ;;
    esac
}
