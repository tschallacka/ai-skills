# Verification: 06-step-tls-listener

## Automated tests

§ 2.1
Start the server twice: assert the cert file (server.crt/server.key) is created once and reused, the port file holds bare digits, and a rustls handshake completes.