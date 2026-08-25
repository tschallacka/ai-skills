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

plan_dir = sys.argv[1]
port = int(sys.argv[2]) if len(sys.argv) > 2 else 0
skill_dir = os.path.dirname(os.path.abspath(__file__))

state_script = os.path.join(skill_dir, "..", "overview-state.sh")
render_script = os.path.join(skill_dir, "..", "render-plan-overview.sh")


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path in ("/state.json", "/state"):
            script = state_script
            args = [plan_dir]
        elif self.path.startswith("/sections"):
            self.send_response(404)
            self.end_headers()
            return
        else:
            script = render_script
            args = ["--serve", plan_dir]
        result = subprocess.run(
            ["bash", script] + args, capture_output=True, text=True
        )
        ctype = "application/json" if "state" in self.path else "text/html"
        self.send_response(200)
        self.send_header("Content-Type", ctype + "; charset=utf-8")
        self.end_headers()
        self.wfile.write(result.stdout.encode())


server = http.server.HTTPServer(("127.0.0.1", port), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
