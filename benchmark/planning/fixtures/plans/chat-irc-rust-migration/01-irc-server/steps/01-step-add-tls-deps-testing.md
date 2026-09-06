# Verification: 01-step-add-tls-deps

## Automated tests

§ 2.1
cargo build --release succeeds and Cargo.toml lists rustls, webpki-roots and ring; cargo tree greps rustls and confirms the deps resolve; clippy is clean.