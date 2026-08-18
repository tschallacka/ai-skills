#!/usr/bin/env bash
# Reserved driver contract template for the benchmark agent runtime.
#
# Scaffold a concrete driver with runtime/scaffold-agent.sh (it substitutes the
# __*__ placeholders below), then implement each reserved function. The template
# must source cleanly and expose every reserved symbol as a loud failing stub,
# so an un-implemented driver fails at first use instead of silently doing
# nothing.
#
# Reserved contract:
#   AGENT_NAME, agent_bin, agent_model_env, agent_default_model
#   agent_argv_worker <workspace> <capsule> <prompt>         -> sets AGENT_ARGV
#   agent_argv_reviewer <workspace> <capsule> <prompt> [bin] -> sets AGENT_ARGV
#   agent_argv_analyzer <workspace> <capsule> <prompt>       -> sets AGENT_ARGV
#   agent_session_id <agent_output.jsonl>                    -> prints session id
#   agent_telemetry <session_id>                             -> prints telemetry
#
# Each agent_argv_* sets AGENT_ARGV and AGENT_CWD and launches nothing;
# launch_agent owns process control. Take the model from agent_resolve_model.
# Drivers are sourced, so guard arity with `return 64`, never `exit`.

set -euo pipefail

# TODO: replace with the CLI/program name the driver invokes on PATH.
AGENT_NAME="__AGENT_NAME__"
agent_bin="__AGENT_CLI__"
# TODO: environment variable that overrides the model (e.g. with CODEX_MODEL).
agent_model_env="__MODEL_ENV__"
# TODO: default model string when agent_model_env is unset. The scaffolder
# fills a loud placeholder; a real model string is required before first use.
agent_default_model="__MODEL_DEFAULT__"

# Drivers are sourced: return a code, never exit, so the harness can trap it.
_agent_template_fail() {
    echo "TEMPLATE: $AGENT_NAME driver stub not implemented for: $1" >&2
    return 65
}

agent_argv_worker() {
    # TODO: set AGENT_CWD="$1" and AGENT_ARGV to launch this agent for the
    # benchmark worker role, with --model "$(agent_resolve_model)".
    # ---- quoted: agent_argv_worker arguments ----
    #   $1 = workspace cwd   $2 = capsule dir   $3 = prompt file
    # ---- end quoted ----
    [ "$#" -eq 3 ] || return 64
    _agent_template_fail "agent_argv_worker"
}

agent_argv_reviewer() {
    # TODO: set AGENT_CWD="$1" and AGENT_ARGV for a reviewer run. The reviewer
    # capsule is flat with the plan under plan/, so grant the whole capsule.
    # ---- quoted: agent_argv_reviewer arguments ----
    #   $1 = reviewer workspace   $2 = reviewer capsule   $3 = prompt file
    #   $4 = optional REVIEWER_COMMAND seam binary, replacing argv[0] only
    # ---- end quoted ----
    [ "$#" -ge 3 ] && [ "$#" -le 4 ] || return 64
    _agent_template_fail "agent_argv_reviewer"
}

agent_argv_analyzer() {
    # TODO: set AGENT_CWD="$1" and AGENT_ARGV for the batch analyzer role; its
    # capsule holds benchmark-test.md, harness-summary.tsv and results/.
    # ---- quoted: agent_argv_analyzer arguments ----
    #   $1 = analyzer workspace   $2 = analyzer capsule   $3 = prompt file
    # ---- end quoted ----
    [ "$#" -eq 3 ] || return 64
    _agent_template_fail "agent_argv_analyzer"
}

agent_session_id() {
    # TODO: print the session id from this agent's output stream ($1), parsed
    # line by line, or print nothing for an honest degrade. A missing file is a
    # degrade, not an error: `[ -f "$1" ] || return 0`.
    [ "$#" -eq 1 ] || return 64
    _agent_template_fail "agent_session_id"
}

agent_telemetry() {
    # TODO: print these keys, one KEY=VALUE per line at column 0; the harness
    # takes the first match per key. Never fabricate a token count.
    # ---- quoted: telemetry stdout contract ----
    # thread_id=<session id>
    # usage_records=<n>|unavailable
    # total_usage_tokens=<n>|unavailable
    # telemetry_status=available|unavailable:<reason>
    # telemetry_db=<path>          (optional)
    # telemetry_source=<name>      (optional)
    # ---- end quoted ----
    [ "$#" -eq 1 ] || return 64
    _agent_template_fail "agent_telemetry"
}
