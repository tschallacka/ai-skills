# Verification: 05-step-memory-ceiling

## Automated tests

§ 2.1
Run the memory verification against the checked-in W91 and W92 fixtures with the pinned Rust toolchain. Capture the toolchain version, fixture checksums, peak resident memory, total bytes allocated, total allocation count and output-buffer allocation or growth count. Pass only when each normal render uses one output-buffer allocation and zero growth, and when the temporary per-field-concatenation mutation fails that assertion. Do not substitute a field-count ratio or an unrecorded absolute RSS limit.
