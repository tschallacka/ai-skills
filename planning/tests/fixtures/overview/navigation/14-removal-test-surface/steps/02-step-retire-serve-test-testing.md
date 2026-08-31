# Verification: 02-step-retire-serve-test

## Automated tests

§ 2.1
Run the rewritten serve test and confirm all four replacements pass against the binary in serve mode. Prove the port-printed-before-first-request property by connecting to the printed port immediately after the line is read and recording a served response rather than a refused connection. Fault-inject by printing the port after the listener is bound but before it accepts, and confirm the test fails.