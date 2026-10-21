"""Minimal JSON API server with per-client rate limiting."""

import json
import math
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from rate_limiter import RateLimiter

rate_limiter = RateLimiter(max_requests=100, window_seconds=60)


def client_id_for(handler):
    """X-Client-ID header if present, else the remote IP."""
    return handler.headers.get("X-Client-ID") or handler.client_address[0]


class RequestHandler(BaseHTTPRequestHandler):
    def _send_json(self, status, body, headers=None):
        payload = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        for name, value in (headers or {}).items():
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(payload)

    def _handle(self):
        client_id = client_id_for(self)
        if not rate_limiter.allow(client_id):
            retry_after = math.ceil(rate_limiter.retry_after(client_id))
            self._send_json(
                429,
                {"error": "rate limit exceeded", "retry_after": retry_after},
                {"Retry-After": str(retry_after)},
            )
            return
        self._send_json(200, {"ok": True, "path": self.path})

    do_GET = _handle
    do_POST = _handle
    do_PUT = _handle
    do_DELETE = _handle


def main(host="127.0.0.1", port=8000):
    server = ThreadingHTTPServer((host, port), RequestHandler)
    print(f"listening on http://{host}:{port}")
    server.serve_forever()


if __name__ == "__main__":
    main()
