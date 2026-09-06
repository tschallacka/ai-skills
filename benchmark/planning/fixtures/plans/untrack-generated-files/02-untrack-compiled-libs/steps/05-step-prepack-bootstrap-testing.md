# Verification: 05-step-prepack-bootstrap

## Automated tests

§ 2.1
Remove the five libs and REVIEWER.md, run npm pack --dry-run: succeeds and both artifacts exist afterwards; validate package.json with jq; with artifacts present the generators still run idempotently.