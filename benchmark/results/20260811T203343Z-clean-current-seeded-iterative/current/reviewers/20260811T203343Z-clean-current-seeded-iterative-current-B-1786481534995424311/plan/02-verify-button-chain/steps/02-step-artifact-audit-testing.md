# Verification: 02-step-artifact-audit

## Command verification

Commands for the future executor:

```bash
find . -maxdepth 2 -type f \\( -name '*.html' -o -name '*.htm' \\) -print | sort
pgrep -af 'chrome|chromium|playwright|webdriver|geckodriver|safaridriver|vite|webpack-dev-server|http-server|python3 -m http.server' || true
```

Pass only if the file audit lists `./button-chain.html` as the sole implementation HTML/HTM artifact for this task and the process audit shows no browser, server, or driver process left running by the verification work.
