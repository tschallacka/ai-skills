# Verification: 02-step-hoist-simple-helpers

## Automated tests

§ 2.1
For each converted helper, run it positionally and with --plan-dir against fresh copies of the fixture plan; diff -r the two trees and compare stdout, stderr and exit status.