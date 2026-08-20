#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_refuse_existing() {
    [ ! -e "$1" ] || plan_die "Refusing to overwrite an existing artifact: $1" 73
}
