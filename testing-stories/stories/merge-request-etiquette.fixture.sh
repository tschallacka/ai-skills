#!/usr/bin/env bash
# Seeds a real local "origin" (a bare repo, kept outside /workspace so it
# doesn't show up as clutter in the agent's own `git status`) plus a
# working checkout with a finished feature branch carrying two real,
# meaningful commits -- so the merge-request-etiquette story can derive a
# description from actual `git log`/`git diff` output, and so the full
# cut-a-request-branch-and-push workflow SKILL.md describes is genuinely
# available (a real, local `origin` to push to, no network required).
set -euo pipefail

origin="${HOME:-/root}/origin.git"
git init -q --bare "$origin"

git clone -q "$origin" .
git config user.email "dev@example.com"
git config user.name "Dev"

cat > sync.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

attempt=0
until curl -fsS "$1"; do
    attempt=$((attempt + 1))
    sleep 1
done
EOF
chmod +x sync.sh
git add -A
git commit -q -m "Add sync.sh: retries a request until it succeeds"
git branch -M main
git push -q origin main

git checkout -q -b fix/sync-retry-loop

cat > sync.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

attempt=0
delay=1
until curl -fsS "$1"; do
    attempt=$((attempt + 1))
    sleep "$delay"
    delay=$((delay * 2))
done
EOF
git add -A
git commit -q -m "Back off exponentially between sync retries"

cat > sync.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

max_attempts=5
attempt=0
delay=1
until curl -fsS "$1"; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge "$max_attempts" ]; then
        echo "sync.sh: giving up after $max_attempts attempts" >&2
        exit 1
    fi
    sleep "$delay"
    delay=$((delay * 2))
done
EOF
git add -A
git commit -q -m "Cap sync retries at 5 attempts instead of looping forever"

git push -q origin fix/sync-retry-loop
