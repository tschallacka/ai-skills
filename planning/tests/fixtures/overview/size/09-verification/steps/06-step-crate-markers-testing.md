# Verification: 06-step-crate-markers

## Automated tests

§ 2.1
This unit is the verification. List every file under src/plan-overview with the marker the gate reads from it, and run the gate. Then run it twice more under mutation: one source with its marker pair removed, one with its pair moved below the module documentation. Record both failures with the file each names. If either mutation leaves the gate green, the gate is not covering the crate and that is the finding, not a note.