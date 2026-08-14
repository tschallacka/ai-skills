#!/usr/bin/env bash
# Scaffold a new benchmark agent driver from the reserved-contract template.
#
# Usage:
#   benchmark/planning/runtime/scaffold-agent.sh <agent-name> [cli] [model-env] [model-default]
#
# Copies runtime/TEMPLATE/agent.sh to runtime/<agent-name>/agent.sh, filling
# baseline values. Refuses to overwrite an existing driver (no clobber); a
# second scaffold of the same agent is a no-op refusal and leaves the first
# driver untouched.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    sed -n '2,7p' "$0" >&2
    exit 64
}

[ "$#" -ge 1 ] && [ "$#" -le 4 ] || usage

AGENT_NAME="$1"
AGENT_CLI="${2:-$AGENT_NAME}"
MODEL_ENV="${3:-$(printf '%s_MODEL' "$(printf '%s' "$AGENT_NAME" | tr '[:lower:]' '[:upper:]')")}"
MODEL_DEFAULT="${4:-}"

if [[ ! "$AGENT_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Agent name must start with a letter or number and contain only letters, numbers, '.', '_' or '-'" >&2
    exit 64
fi

TEMPLATE_DIR="$SCRIPT_DIR/TEMPLATE"
TARGET_DIR="$SCRIPT_DIR/$AGENT_NAME"
TARGET="$TARGET_DIR/agent.sh"

if [ ! -f "$TEMPLATE_DIR/agent.sh" ]; then
    echo "Driver template not found: $TEMPLATE_DIR/agent.sh" >&2
    exit 65
fi
if [ -e "$TARGET" ]; then
    echo "Refusing to clobber existing driver: $TARGET" >&2
    exit 73
fi

mkdir -p "$TARGET_DIR"
sed \
    -e "s/__AGENT_NAME__/$(printf '%s' "$AGENT_NAME" | sed -e 's/[\/&]/\\&/g')/g" \
    -e "s/__AGENT_CLI__/$(printf '%s' "$AGENT_CLI" | sed -e 's/[\/&]/\\&/g')/g" \
    -e "s/__MODEL_ENV__/$(printf '%s' "$MODEL_ENV" | sed -e 's/[\/&]/\\&/g')/g" \
    -e "s/__MODEL_DEFAULT__/$(printf '%s' "$MODEL_DEFAULT" | sed -e 's/[\/&]/\\&/g')/g" \
    "$TEMPLATE_DIR/agent.sh" > "$TARGET"
chmod +x "$TARGET"

cat <<EOF
Scaffolded agent driver
  agent:        $AGENT_NAME
  cli:          $AGENT_CLI
  model env:    $MODEL_ENV
  model default: $MODEL_DEFAULT
  driver:       $TARGET
Implement every reserved function in $TARGET before use.
EOF