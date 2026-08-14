#!/usr/bin/env bash
# Reserved driver contract template for the benchmark agent runtime.
#
# Scaffold a concrete driver with benchmark/planning/runtime/scaffold-agent.sh
# (which substitutes the __*__ placeholders below), then implement each
# reserved function. This template must source cleanly and expose every
# reserved symbol as a clear failing stub marked TODO, so an un-implemented
# driver fails loudly instead of silently.
#
# Reserved contract:
#   AGENT_NAME, agent_bin, agent_model_env, agent_default_model
#   agent_argv_worker <workspace> <capsule> <prompt>        -> sets AGENT_ARGV
#   agent_argv_reviewer <workspace> <capsule> <prompt> [bin] -> sets AGENT_ARGV
#   agent_argv_analyzer <workspace> <capsule> <prompt>      -> sets AGENT_ARGV
#   agent_session_id <agent_output.jsonl>                   -> prints session id
#   agent_telemetry <session_id>                            -> prints telemetry

set -euo pipefail

# TODO: replace with the CLI/program name the driver invokes on PATH.
AGENT_NAME="__AGENT_NAME__"
agent_bin="__AGENT_CLI__"
# TODO: environment variable that overrides the model (e.g. with CODEX_MODEL).
agent_model_env="__MODEL_ENV__"
# TODO: default model string when agent_model_env is unset.
agent_default_model="__MODEL_DEFAULT__"

_agent_template_fail() {
    echo "TEMPLATE: $AGENT_NAME driver stub not implemented for: $1" >&2
    exit 65
}

agent_argv_worker() {
    # TODO: set AGENT_ARGV to launch this agent for the benchmark worker role.
    #   $1 = workspace cwd, $2 = capsule dir, $3 = prompt file.
    _agent_template_fail "agent_argv_worker"
}

agent_argv_reviewer() {
    # TODO: set AGENT_ARGV to launch this agent for a reviewer role.
    #   $1 = reviewer workspace, $2 = reviewer capsule, $3 = prompt file,
    #   $4 (optional) = binary override for the REVIEWER_COMMAND test seam.
    _agent_template_fail "agent_argv_reviewer"
}

agent_argv_analyzer() {
    # TODO: set AGENT_ARGV to launch this agent for the batch analyzer role.
    #   $1 = analyzer workspace, $2 = analyzer capsule, $3 = prompt file.
    _agent_template_fail "agent_argv_analyzer"
}

agent_session_id() {
    # TODO: print the session id from this agent's output stream ($1), or
    # print nothing / 'unavailable' for honest degrade.
    _agent_template_fail "agent_session_id"
}

agent_telemetry() {
    # TODO: print thread_id/usage_records/total_usage_tokens/
    # telemetry_status (+ optional telemetry_source). Never fabricate token
    # counts; report telemetry_status=unavailable:... when no documented
    # store exists for this agent.
    _agent_template_fail "agent_telemetry"
}