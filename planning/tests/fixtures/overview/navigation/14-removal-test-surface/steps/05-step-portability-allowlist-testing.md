# Verification: 05-step-portability-allowlist

## Automated tests

§ 2.1
Run the portability contract test and confirm it passes with the allowlist arm removed. Prove the arm was load-bearing rather than inert: reintroduce a python3 invocation in a shipped script and confirm the rule now reports it, which is the violation the stale exemption would have hidden.