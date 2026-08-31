# Verification: 03-step-npm-packaging

## Automated tests

§ 2.1
Run npm pack, then generate or refresh the W105-owned planning/tests/fixtures/overview/npm-package-baseline.tsv from that tarball using its fixed TSV schema: package_path<TAB>byte_size rows in lexical order followed by one tarball_bytes<TAB>byte_size row. Extract the tarball into a clean directory and invoke bash install.sh from that package root on a supported POSIX host, native Windows through the supported Bash environment, and one unsupported OS or architecture input. W120 must read this exact baseline and must not create a second file list or byte-size record. Record the normalized key, selected filename and execution output or the unavailable notice. The tarball must carry all five artifacts and install.sh.
