#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
plan_register_temp_file() {
    PLAN_DIE_TEMP_FILES+=("$1")
}
