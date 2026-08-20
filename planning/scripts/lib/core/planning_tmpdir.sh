#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Scratch directory the planning skill may write temporary capsules and run
# artifacts into. It lives under the system temp dir so it is fresh per boot;
# the agent's existing write access to the temp dir suffices to create it.
planning_tmpdir() {
    printf '%s\n' "${TMPDIR:-/tmp}/planning-agent"
}
