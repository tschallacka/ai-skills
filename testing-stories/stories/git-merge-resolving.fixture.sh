#!/usr/bin/env bash
# Seeds a repository with two branches that each insert a different new row
# at the SAME position in the same table -- a guaranteed, unavoidable
# textual conflict whose correct resolution is "keep both," the exact
# "union, not a choice" shape git-merge-resolving/SKILL.md calls out.
set -euo pipefail

git init -q
git config user.email "dev@example.com"
git config user.name "Dev"

cat > commands.md <<'EOF'
# Command registry

| Command | Description |
|---|---|
| list | Lists all records |
| search | Searches records by keyword |
EOF

git add -A
git commit -q -m "Initial command registry: list, search"
git branch -M main

git checkout -q -b feature/add-export
cat > commands.md <<'EOF'
# Command registry

| Command | Description |
|---|---|
| list | Lists all records |
| search | Searches records by keyword |
| export | Exports records to CSV |
EOF
git add -A
git commit -q -m "Add export command to registry"

git checkout -q main
git checkout -q -b feature/add-import
cat > commands.md <<'EOF'
# Command registry

| Command | Description |
|---|---|
| list | Lists all records |
| search | Searches records by keyword |
| import | Imports records from CSV |
EOF
git add -A
git commit -q -m "Add import command to registry"

git checkout -q main
