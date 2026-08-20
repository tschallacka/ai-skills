#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_require_directory() {
    [ -d "$1" ] || plan_die "Plan directory not found: $1" 66
}
