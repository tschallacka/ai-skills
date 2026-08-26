#!/usr/bin/env python3
"""Minimal localhost HTTP server for the plan overview.

# MODE: PROD
Serves / (rendered HTML) and /state.json (extractor output) from the given
plan directory. Runs until killed; prints the bound port on startup.
"""
import http.server
import os
import subprocess
import sys
import tempfile

plan_dir = sys.argv[1]
port = int(sys.argv[2]) if len(sys.argv) > 2 else 0
skill_dir = os.path.dirname(os.path.abspath(__file__))

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
            body = section_of(self._render(), section_id)
            if body is None:
                self._send(404, "text/plain", "Not found\n")
            else:
                self._send(200, "text/html", body)
        else:
            body = self._render()
            self._send(200, "text/html", body)

    def _render(self):
        # The renderer writes a file and prints its path; render to a temp
        # and read it back so the response is exactly the artifact.
        fd, path = tempfile.mkstemp(suffix=".html")
        os.close(fd)
        try:
            subprocess.run(
                ["bash", render_script, "--serve", plan_dir, "--out", path],
                capture_output=True, text=True,
            )
            with open(path, encoding="utf-8") as fh:
                return fh.read()
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


server = http.server.HTTPServer(("127.0.0.1", port), Handler)
# os.write bypasses every io layer: the invoker polls this line to learn the
# port, so no buffering subtlety may delay it.
os.write(1, f"{server.server_address[1]}\n".encode())
server.serve_forever()
