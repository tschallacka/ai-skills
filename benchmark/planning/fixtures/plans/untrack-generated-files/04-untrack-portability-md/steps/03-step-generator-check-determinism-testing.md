# Verification: 03-step-generator-check-determinism

## Automated tests

§ 2.1
Run generate-portability.sh --check twice on the stable registry: exit 0 and byte-identical temp outputs; confirm --help no longer references a committed file.