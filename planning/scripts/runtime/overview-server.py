#!/usr/bin/env python3
"""Minimal localhost HTTP server for the plan overview.

# MODE: PROD
Serves / (rendered HTML) and /state.json (extractor output) from the given
plan directory. Runs until killed; prints the bound port on startup.
"""
import http.server
import os
import signal
import socketserver
import subprocess
import sys
import tempfile

plan_dir = sys.argv[1]
port = int(sys.argv[2]) if len(sys.argv) > 2 else 0
skill_dir = os.path.dirname(os.path.abspath(__file__))
if sys.platform.startswith("linux") and os.uname().machine in ("x86_64", "amd64"):
    rjq_dir = "x86_64-unknown-linux-musl"
elif sys.platform.startswith("linux"):
    rjq_dir = "aarch64-unknown-linux-musl"
elif sys.platform == "darwin" and os.uname().machine == "x86_64":
    rjq_dir = "x86_64-apple-darwin"
elif sys.platform == "darwin":
    rjq_dir = "aarch64-apple-darwin"
else:
    rjq_dir = "x86_64-pc-windows-msvc"
os.environ["PATH"] = os.path.join(skill_dir, "..", "..", "bin", rjq_dir) + os.pathsep + os.environ.get("PATH", "")

state_script = os.path.join(skill_dir, "..", "overview-state.sh")
render_script = os.path.join(skill_dir, "..", "render-plan-overview.sh")


SECTIONS = ("identity-panel", "step-details", "tests-panel",
            "coverage-panel", "findings-panel", "dep-graph", "narr")

import re


def section_of(html, section_id):
    pat = (r'(<[^>]*id="' + re.escape(section_id) + r'"[^>]*>[\s\S]*?)'
           r'(?=<[^>]*id="(?:' + "|".join(SECTIONS) + r')"|</main>|</body>)')
    m = re.search(pat, html)
    return m.group(1) if m else None


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        path = self.path.split("?")[0]
        if path in ("/state.json", "/state"):
            result = subprocess.run(
                ["bash", state_script, plan_dir], capture_output=True, text=True
            )
            self._send(200, "application/json", result.stdout)
        elif path.startswith("/sections/"):
            section_id = path[len("/sections/"):].replace("/", "")
            if not re.fullmatch(r"[a-z-]+", section_id):
                self._send(404, "text/plain", "Not found\n")
                return
            body = self._render()
            if body is None:
                self._send(500, "text/plain", "render failed\n")
                return
            body = section_of(body, section_id)
            if body is None:
                self._send(404, "text/plain", "Not found\n")
            else:
                self._send(200, "text/html", body)
        else:
            body = self._render()
            if body is None:
                self._send(500, "text/plain", "render failed\n")
            else:
                self._send(200, "text/html", body)

    def _render(self):
        # The renderer writes a file and prints its path; render to a temp
        # and read it back so the response is exactly the artifact. A failed
        # render must surface as 500, never a 200 with an empty body (B51).
        fd, path = tempfile.mkstemp(suffix=".html")
        os.close(fd)
        try:
            result = subprocess.run(
                ["bash", render_script, "--serve", plan_dir, "--out", path],
                capture_output=True, text=True,
            )
            if result.returncode != 0:
                sys.stderr.write("render failed (%d): %s\n"
                                 % (result.returncode, result.stderr[:500]))
                return None
            with open(path, encoding="utf-8") as fh:
                return fh.read() or None
        finally:
            os.unlink(path)

    def _send(self, code, ctype, body):
        payload = body.encode()
        self.send_response(code)
        self.send_header("Content-Type", ctype + "; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


class Server(http.server.HTTPServer):
    # HTTPServer.server_bind resolves socket.getfqdn("127.0.0.1") — a
    # reverse-DNS lookup that mDNSResponder can stall for tens of seconds on
    # macOS, which reads as "never reported a port". The name is only used in
    # error pages; a literal keeps the bind immediate everywhere.
    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        host, sport = self.server_address[:2]
        self.server_name = "localhost"
        self.server_port = sport


server = Server(("127.0.0.1", port), Handler)
# os.write bypasses every io layer: the invoker polls this line to learn the
# port, so no buffering subtlety may delay it.
os.write(1, f"{server.server_address[1]}\n".encode())
# Blocked-in-accept processes can miss a plain TERM on some platforms; make
# the disposition explicit so a clean stop is immediate.
signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
signal.signal(signal.SIGINT, lambda *_: sys.exit(0))
server.serve_forever()
