#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_warn() {
    printf 'WARN: %s\n' "$*" >&2
}
