# Verification: 05-step-history-extension

## Automated tests

§ 2.1
Seed the log with ids 1 and 7, send "FETCH #r 0", and assert both rows followed by a terminating marker; FETCH #r 7 returns only id>=7.