#!/usr/bin/env bash
# Registered temp files, removed when plan_die exits. Its own file because
# plan_die reads it and a test may source either alone.
PLAN_DIE_TEMP_FILES=()
