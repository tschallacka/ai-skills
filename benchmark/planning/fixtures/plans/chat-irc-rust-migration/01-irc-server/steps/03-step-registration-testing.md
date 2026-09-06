# Verification: 03-step-registration

## Automated tests

§ 2.1
Drive the server over TLS with a raw fixture: send NICK nick

§ 2.2
USER u 0 * :name

§ 2.3
and assert a 001/002/003/004/005 block, then MOTD 375/372/376; assert 433 on a duplicate NICK.