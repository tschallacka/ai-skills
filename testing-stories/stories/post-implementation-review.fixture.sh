#!/usr/bin/env bash
# Seeds /workspace with a just-written, only-happy-path-tested module before
# the agent starts, so the story's task ("review what I just wrote") has
# something real and non-trivial to review. Deliberately not documented in
# any README the agent could shortcut to -- the bugs must be found by reading
# the code and its test.
set -euo pipefail

mkdir -p rate_limiter
cat > rate_limiter/rate_limiter.py <<'PY'
"""Per-user sliding-window rate limiter. Written just now; only the happy
path below has been exercised."""

import time


class RateLimiter:
    def __init__(self, max_requests, window_seconds):
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        self.requests = {}  # user_id -> list of request timestamps

    def allow(self, user_id):
        now = time.time()
        history = self.requests.get(user_id, [])
        # Drop timestamps outside the window.
        history = [t for t in history if now - t < self.window_seconds]
        if len(history) < self.max_requests:
            history.append(now)
            self.requests[user_id] = history
            return True
        self.requests[user_id] = history
        return False

    def reset(self, user_id):
        del self.requests[user_id]
PY

cat > rate_limiter/test_happy_path.py <<'PY'
from rate_limiter import RateLimiter


def test_allows_up_to_the_limit():
    limiter = RateLimiter(max_requests=3, window_seconds=60)
    assert limiter.allow("alice") is True
    assert limiter.allow("alice") is True
    assert limiter.allow("alice") is True
    assert limiter.allow("alice") is False


if __name__ == "__main__":
    test_allows_up_to_the_limit()
    print("happy path OK")
PY

echo "fixture: seeded rate_limiter/ (module + one happy-path test) into /workspace" >&2
