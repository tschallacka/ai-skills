# Verification: 06-step-binaries-registry

## Automated tests

§ 2.1
Parse the registry with awk alone and confirm every row is read. Recompute each checksum from the built artifact and compare. Confirm the row set matches the artifact set the matrix produces, in both directions.