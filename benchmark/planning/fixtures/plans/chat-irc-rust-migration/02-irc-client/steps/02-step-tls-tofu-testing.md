# Verification: 02-step-tls-tofu

## Automated tests

§ 2.1
Connect to a goal-01 server: assert a fingerprint is written on first connect and a changed/recreated server cert is rejected unless --insecure is given.