#!/usr/bin/env bash
# Seeds a small, real repository with a config-loading module the
# git-worktrees testing story asks the agent to rework in isolation.
set -euo pipefail

git init -q
git config user.email "dev@example.com"
git config user.name "Dev"

mkdir -p src

cat > src/config.py <<'EOF'
import json


def load_config(path="config.json"):
    with open(path) as handle:
        return json.load(handle)
EOF

cat > config.json <<'EOF'
{
  "debug": false,
  "retries": 3
}
EOF

cat > README.md <<'EOF'
# demo-project

A tiny demo project used for a git-worktrees testing story.

`src/config.py` loads `config.json` at startup.
EOF

git add -A
git commit -q -m "Initial project: config.json loaded by src/config.py"
git branch -M main
