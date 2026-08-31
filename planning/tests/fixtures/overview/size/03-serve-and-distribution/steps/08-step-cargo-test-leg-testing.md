# Verification: 08-step-cargo-test-leg

## Automated tests

§ 2.1
Run the repository suite with a toolchain present and record the crate test count as a number. Then run it three more ways: with no toolchain on PATH and the refuse variable set, requiring a non-zero exit and a failed suite; with no toolchain and the variable unset, requiring the unconfigured report and a shell suite that still runs; and with a deliberately failing crate test, requiring the suite to go red. The first two are the pair that distinguishes a contributor's machine from the gate, and only the strict one protects CI.