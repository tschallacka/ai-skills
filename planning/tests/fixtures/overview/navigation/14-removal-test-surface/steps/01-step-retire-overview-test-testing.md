# Verification: 01-step-retire-overview-test

## Automated tests

§ 2.1
Run the rewritten test file on the largest fixture and confirm it passes against the binary. Then prove each replacement assertion bites: for every one of the twelve retired checks, inject the fault it caught (a missing section, an unescaped value, an empty state) into the binary's output path and record the observed failure. A replacement that stays green under its own fault has not replaced anything.