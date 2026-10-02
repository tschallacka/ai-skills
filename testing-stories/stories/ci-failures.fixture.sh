#!/usr/bin/env bash
# Fixture for the ci-failures testing story.
#
# ci-failures needs a real forge with real CI history to query -- no fixture
# can fabricate a GitHub Actions run that actually happened. This seeds the
# one thing that CAN be set up locally: an empty git repo whose origin points
# at the real ai-skills project (public, real GitHub Actions history,
# read-only over HTTPS with no credentials needed to inspect public run
# metadata via `gh`). Given a GH_TOKEN on the host (see run-story.sh), the
# agent can genuinely run this story end to end.
set -euo pipefail

git init -q
git remote add origin https://github.com/tschallacka/ai-skills.git
git fetch -q origin master:refs/remotes/origin/master 2>/dev/null || true
git checkout -q -b master origin/master 2>/dev/null || git checkout -q -b master
