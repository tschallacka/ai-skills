"""Sliding-window rate limiter, keyed per client_id, thread-safe."""

import threading
import time
from collections import deque


class RateLimiter:
    """Allows at most `max_requests` per `window_seconds` for each client_id.

    Uses a sliding log: each client keeps a deque of the timestamps of its
    accepted requests inside the current window. A request is allowed when,
    after dropping timestamps older than the window, fewer than
    `max_requests` remain.
    """

    def __init__(self, max_requests=100, window_seconds=60.0, clock=time.monotonic):
        if max_requests < 1:
            raise ValueError("max_requests must be >= 1")
        if window_seconds <= 0:
            raise ValueError("window_seconds must be > 0")
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        self._clock = clock
        self._lock = threading.Lock()
        self._hits = {}  # client_id -> deque of accepted-request timestamps

    def _prune(self, hits, now):
        cutoff = now - self.window_seconds
        while hits and hits[0] <= cutoff:
            hits.popleft()

    def allow(self, client_id):
        """Record and allow the request if under the limit; return False otherwise."""
        now = self._clock()
        with self._lock:
            hits = self._hits.get(client_id)
            if hits is None:
                hits = self._hits[client_id] = deque()
            self._prune(hits, now)
            if len(hits) >= self.max_requests:
                return False
            hits.append(now)
            return True

    def retry_after(self, client_id):
        """Seconds until client_id may make another request (0 if it may now)."""
        now = self._clock()
        with self._lock:
            hits = self._hits.get(client_id)
            if not hits:
                return 0.0
            self._prune(hits, now)
            if len(hits) < self.max_requests:
                return 0.0
            return max(0.0, hits[0] + self.window_seconds - now)

    def cleanup(self):
        """Drop clients with no requests left in the window, bounding memory."""
        now = self._clock()
        with self._lock:
            for client_id in list(self._hits):
                hits = self._hits[client_id]
                self._prune(hits, now)
                if not hits:
                    del self._hits[client_id]
