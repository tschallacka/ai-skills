# Verification: 07-step-memory-harness

## Automated tests

§ 2.1
Run cargo test --test memory against W91 and W92 with rustc 1.86.0 and recorded fixture checksums. Reset and inspect the separate allocation and growth counters on RenderBuffer::new and RenderBuffer::write_str in render/shell.rs. Require one allocation and zero growth. Then run cargo test --features test-per-field-buffer --test memory and require non-zero because the production seam is replaced by per-field String allocation.