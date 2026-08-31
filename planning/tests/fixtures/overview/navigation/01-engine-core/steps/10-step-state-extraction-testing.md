# Verification: 10-step-state-extraction

## Automated tests

§ 2.1
Run the production extractor against the fixtures with Bash and jq unavailable, serialize the canonical state, and compare its field set and values to the field-contract fixture. Any field not reproduced is recorded by name; no shell subprocess may be used by the binary.
