# Verification: 11-step-commit-artifacts

## Automated tests

§ 2.1
Recompute each artifact's checksum from the committed file and compare against its binaries.tsv row, recording both values rather than a pass. Record each artifact's byte size and the total the repository gained. Confirm every path skill_files() names resolves to a file present on disk, and confirm each artifact executes on its own platform rather than merely existing.